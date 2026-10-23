//! Streaming VCD (Value Change Dump) parser.
//!
//! Handles the full header ($date, $version, $timescale, $scope/$var tree,
//! $upscope, $enddefinitions) and the value-change section (#time lines,
//! scalar `1x`, vector `b1010 id`, real `r1.5 id`).
//!
//! Duplicate ids (the same net dumped under several scopes) are deduplicated:
//! the first declaration wins, all aliases recorded.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Fs,
    Ps,
    Ns,
    Us,
    Ms,
    S,
}

impl TimeUnit {
    pub fn factor_secs(self) -> f64 {
        match self {
            TimeUnit::Fs => 1e-15,
            TimeUnit::Ps => 1e-12,
            TimeUnit::Ns => 1e-9,
            TimeUnit::Us => 1e-6,
            TimeUnit::Ms => 1e-3,
            TimeUnit::S => 1.0,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            TimeUnit::Fs => "fs",
            TimeUnit::Ps => "ps",
            TimeUnit::Ns => "ns",
            TimeUnit::Us => "µs",
            TimeUnit::Ms => "ms",
            TimeUnit::S => "s",
        }
    }

    pub fn parse(s: &str) -> Option<TimeUnit> {
        match s.to_ascii_lowercase().as_str() {
            "fs" => Some(TimeUnit::Fs),
            "ps" => Some(TimeUnit::Ps),
            "ns" => Some(TimeUnit::Ns),
            "us" | "µs" | "μs" => Some(TimeUnit::Us),
            "ms" => Some(TimeUnit::Ms),
            "s" => Some(TimeUnit::S),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub date: String,
    pub version: String,
    /// e.g. (1, Ps) meaning 1 tick = 1 ps.
    pub timescale: (u64, TimeUnit),
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    pub children: Vec<Scope>,
}

#[derive(Debug, Clone)]
pub struct Signal {
    /// VCD id code (raw bytes, may be multi-character).
    pub id: Vec<u8>,
    /// Full hierarchical path, e.g. ["TOP", "soc", "cycle"].
    pub path: Vec<String>,
    /// Bit width; 1 for scalars.
    pub width: u32,
    /// VCD range [msb:lsb] if declared with one.
    pub range: Option<(i32, i32)>,
    /// Net kind as declared (wire, reg, ...).
    pub kind: String,
}

/// A stored value. Vector payloads are interned so identical strings share.
#[derive(Debug, Clone)]
pub enum Value {
    /// Single-bit value: b'0', b'1', b'x', b'z' ...
    Bit(u8),
    /// Vector value, one char per bit, MSB first.
    Bits(Rc<[u8]>),
    Real(f64),
}

impl Value {
    pub fn is_x(&self) -> bool {
        match self {
            Value::Bit(b) => *b == b'x' || *b == b'X',
            Value::Bits(bits) => bits.iter().any(|&b| b == b'x' || b == b'X'),
            Value::Real(_) => false,
        }
    }

    pub fn is_z(&self) -> bool {
        match self {
            Value::Bit(b) => *b == b'z' || *b == b'Z',
            Value::Bits(bits) => bits.iter().all(|&b| b == b'z' || b == b'Z'),
            Value::Real(_) => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Change {
    pub time: u64,
    pub value: Value,
}

#[derive(Debug)]
pub struct Vcd {
    pub header: Header,
    pub scopes: Vec<Scope>,
    /// Deduplicated signals (one entry per unique id).
    pub signals: Vec<Signal>,
    /// Changes per signal index, sorted by time.
    pub changes: Vec<Vec<Change>>,
    pub end_time: u64,
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ParseError {}

struct ParserState {
    id_to_idx: HashMap<Vec<u8>, usize>,
    /// Interning table for vector payload strings.
    interner: HashMap<Rc<[u8]>, Rc<[u8]>>,
    current_time: u64,
    line_no: usize,
}

pub fn parse(path: &str) -> Result<Vcd, ParseError> {
    let file = File::open(path).map_err(|e| ParseError {
        line: 0,
        message: format!("cannot open {}: {}", path, e),
    })?;
    let mut reader = BufReader::with_capacity(1 << 20, file);
    let mut st = ParserState {
        id_to_idx: HashMap::new(),
        interner: HashMap::new(),
        current_time: 0,
        line_no: 0,
    };

    let mut header = Header {
        date: String::new(),
        version: String::new(),
        timescale: (1, TimeUnit::Ps),
    };
    // Scope tree arena: node 0 is the (unnamed) root.
    struct Node {
        name: String,
        children: Vec<usize>,
    }
    let mut nodes: Vec<Node> = vec![Node {
        name: String::new(),
        children: Vec::new(),
    }];
    let mut stack: Vec<usize> = vec![0];
    let mut scope_path: Vec<String> = Vec::new();
    let mut signals: Vec<Signal> = Vec::new();
    let mut changes: Vec<Vec<Change>> = Vec::new();
    let mut end_time: u64 = 0;
    let mut in_defs = true;

    let mut line: Vec<u8> = Vec::with_capacity(256);
    loop {
        line.clear();
        let n = read_line(&mut reader, &mut line);
        if n == 0 && line.is_empty() {
            break;
        }
        st.line_no += 1;
        let mut bytes: &[u8] = &line;
        trim(&mut bytes);
        if bytes.is_empty() {
            continue;
        }

        if in_defs {
            if bytes[0] == b'$' {
                // $keyword ... $end
                let (kw, rest, ended_inline) = split_kw(bytes);
                match kw {
                    b"$date" => {
                        header.date = if rest.is_empty() {
                            read_until_end(&mut reader, &mut st)
                        } else {
                            String::from_utf8_lossy(rest).into_owned()
                        };
                    }
                    b"$version" => {
                        header.version = if rest.is_empty() {
                            read_until_end(&mut reader, &mut st)
                        } else {
                            String::from_utf8_lossy(rest).into_owned()
                        };
                    }
                    b"$timescale" => {
                        header.timescale = parse_timescale(rest.trim_ascii()).ok_or_else(|| {
                            ParseError {
                                line: st.line_no,
                                message: "bad $timescale".into(),
                            }
                        })?;
                    }
                    b"$scope" => {
                        // $scope module <name> $end
                        let name = std::str::from_utf8(rest)
                            .unwrap_or("")
                            .split_ascii_whitespace()
                            .nth(1)
                            .unwrap_or("top")
                            .trim()
                            .to_string();
                        let id = nodes.len();
                        nodes.push(Node {
                            name: name.clone(),
                            children: Vec::new(),
                        });
                        let parent = *stack.last().unwrap();
                        nodes[parent].children.push(id);
                        stack.push(id);
                        scope_path.push(name);
                    }
                    b"$upscope" => {
                        // tolerate malformed dumps with unbalanced $upscope
                        if stack.len() > 1 {
                            stack.pop();
                            scope_path.pop();
                        }
                    }
                    b"$var" => {
                        let sig = parse_var(rest, &scope_path);
                        if let Some(sig) = sig {
                            let idx = match st.id_to_idx.get(&sig.id) {
                                Some(&i) => i,
                                None => {
                                    let i = signals.len();
                                    signals.push(sig.clone());
                                    changes.push(Vec::new());
                                    st.id_to_idx.insert(sig.id.clone(), i);
                                    i
                                }
                            };
                            // keep first declared path
                            let _ = idx;
                        }
                    }
                    b"$enddefinitions" => in_defs = false,
                    _ => {} // ignore $comment etc. (multi-line comments skipped below)
                }
                if kw == b"$comment" && !ended_inline {
                    // multi-line comment: skip until $end on its own
                    loop {
                        line.clear();
                        if read_line(&mut reader, &mut line) == 0 {
                            break;
                        }
                        st.line_no += 1;
                        let mut b2: &[u8] = &line;
                        trim(&mut b2);
                        if b2 == b"$end" {
                            break;
                        }
                    }
                }
            }
        } else {
            // Value change section
            match bytes[0] {
                b'#' => {
                    let s = std::str::from_utf8(&bytes[1..]).unwrap_or("0");
                    st.current_time = s.trim().parse().unwrap_or(st.current_time);
                    if st.current_time > end_time {
                        end_time = st.current_time;
                    }
                }
                b'b' | b'B' => {
                    // b10101010 id   (uppercase B: space-separated id)
                    let (val, id) = split_value_id(&bytes[1..], bytes[0] == b'B');
                    if !val.is_empty() {
                        if let Some(&idx) = st.id_to_idx.get(id) {
                            let bits = intern(&mut st.interner, val);
                            changes[idx].push(Change {
                                time: st.current_time,
                                value: Value::Bits(bits),
                            });
                        }
                    }
                }
                b'r' | b'R' => {
                    let (val, id) = split_value_id(&bytes[1..], bytes[0] == b'R');
                    if let Some(&idx) = st.id_to_idx.get(id) {
                        if let Ok(f) = std::str::from_utf8(val)
                            .map_err(|_| ())
                            .and_then(|s| s.trim().parse::<f64>().map_err(|_| ()))
                        {
                            changes[idx].push(Change {
                                time: st.current_time,
                                value: Value::Real(f),
                            });
                        }
                    }
                }
                _ => {
                    // scalar: <value><id> e.g. 1*
                    if bytes.len() >= 2 {
                        let v = bytes[0];
                        if let Some(&idx) = st.id_to_idx.get(&bytes[1..]) {
                            changes[idx].push(Change {
                                time: st.current_time,
                                value: Value::Bit(v),
                            });
                        }
                    }
                }
            }
        }
    }

    // Materialize the recursive scope tree from the arena.
    fn build_tree(nodes: &[Node], id: usize) -> Scope {
        Scope {
            name: nodes[id].name.clone(),
            children: nodes[id]
                .children
                .iter()
                .map(|&c| build_tree(nodes, c))
                .collect(),
        }
    }
    let scopes: Vec<Scope> = nodes[0]
        .children
        .iter()
        .map(|&c| build_tree(&nodes, c))
        .collect();

    Ok(Vcd {
        header,
        scopes,
        signals,
        changes,
        end_time,
    })
}

fn read_line(r: &mut impl BufRead, out: &mut Vec<u8>) -> usize {
    let mut n = 0usize;
    loop {
        let mut byte = [0u8; 1];
        match r.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                n += 1;
                if byte[0] == b'\n' {
                    break;
                }
                out.push(byte[0]);
            }
            Err(_) => break,
        }
    }
    n
}

fn trim(b: &mut &[u8]) {
    while let Some(&c) = b.first() {
        if c == b' ' || c == b'\t' || c == b'\r' {
            let (_, rest) = b.split_first().unwrap();
            *b = rest;
        } else {
            break;
        }
    }
    while let Some(&c) = b.last() {
        if c == b' ' || c == b'\t' || c == b'\r' {
            let (_, rest) = b.split_last().unwrap();
            *b = rest;
        } else {
            break;
        }
    }
}

/// Split `$keyword rest...$end` -> (keyword, rest-without-$end, ended_inline)
/// where `ended_inline` is true when a trailing `$end` was stripped.
fn split_kw(b: &[u8]) -> (&[u8], &[u8], bool) {
    let end = b.iter().position(|&c| c == b' ').unwrap_or(b.len());
    let kw = &b[..end];
    let mut rest = &b[end..];
    trim(&mut rest);
    // strip trailing $end if present on same line
    let mut ended_inline = false;
    if rest.ends_with(b"$end") {
        rest = &rest[..rest.len() - 4];
        trim(&mut rest);
        ended_inline = true;
    }
    (kw, rest, ended_inline)
}

/// For multi-line keywords ($date, $version): read lines until $end.
fn read_until_end(r: &mut impl BufRead, st: &mut ParserState) -> String {
    let mut acc = String::new();
    let mut line: Vec<u8> = Vec::new();
    loop {
        line.clear();
        if read_line(r, &mut line) == 0 {
            break;
        }
        st.line_no += 1;
        let mut b: &[u8] = &line;
        trim(&mut b);
        if b == b"$end" {
            break;
        }
        if !acc.is_empty() {
            acc.push(' ');
        }
        acc.push_str(&String::from_utf8_lossy(b));
    }
    acc
}

fn parse_timescale(b: &[u8]) -> Option<(u64, TimeUnit)> {
    let s = std::str::from_utf8(b).ok()?;
    let s = s.trim();
    // forms: "1ps", "10 ns", "1 fs"
    let mut num_end = 0;
    let bytes = s.as_bytes();
    while num_end < bytes.len() && (bytes[num_end].is_ascii_digit() || bytes[num_end] == b'.') {
        num_end += 1;
    }
    let num: u64 = s[..num_end].parse().ok()?;
    let unit = s[num_end..].trim();
    Some((num, TimeUnit::parse(unit)?))
}

/// Parse `$var wire 16 s reset_counter [15:0] $end`
/// `rest` already has leading "$var" removed.
fn parse_var(rest: &[u8], scope_path: &[String]) -> Option<Signal> {
    let s = std::str::from_utf8(rest).ok()?;
    let mut it = s.split_ascii_whitespace();
    let kind = it.next()?.to_string(); // wire, reg, ...
    let width: u32 = it.next()?.parse().ok()?;
    let id = it.next()?.as_bytes().to_vec();
    let mut name = it.next()?.to_string();
    let mut range = None;
    // remaining tokens: optional [msb:lsb] then "$end"
    for tok in it {
        if tok == "$end" {
            break;
        }
        if tok.starts_with('[') {
            if let Some(r) = parse_range(tok) {
                range = Some(r);
            }
        } else {
            name.push(' ');
            name.push_str(tok);
        }
    }
    let mut path: Vec<String> = scope_path.to_vec();
    path.push(name);
    Some(Signal {
        id,
        path,
        width,
        range,
        kind,
    })
}

fn parse_range(tok: &str) -> Option<(i32, i32)> {
    let inner = tok.trim_start_matches('[').trim_end_matches(']');
    let (a, b) = inner.split_once(':')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

/// Split `10101010 *` or `10101010*` -> (value bytes, id bytes)
/// `spaced` = uppercase B/R form where a space separates id.
fn split_value_id(b: &[u8], spaced: bool) -> (&[u8], &[u8]) {
    if spaced {
        match b.iter().position(|&c| c == b' ') {
            Some(p) => (&b[..p], &b[p + 1..]),
            None => (b, &[]),
        }
    } else {
        // lowercase b: value then id, id may start anywhere after whitespace-free value
        // VCD vector: value chars are 01xXzZ; id starts at first char outside that set.
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if matches!(c, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z') {
                i += 1;
            } else {
                break;
            }
        }
        let val = &b[..i];
        let mut id = &b[i..];
        trim(&mut id);
        (val, id)
    }
}

fn intern(map: &mut HashMap<Rc<[u8]>, Rc<[u8]>>, val: &[u8]) -> Rc<[u8]> {
    if let Some(rc) = map.get(val) {
        return rc.clone();
    }
    let rc: Rc<[u8]> = val.to_vec().into();
    map.insert(rc.clone(), rc.clone());
    rc
}
