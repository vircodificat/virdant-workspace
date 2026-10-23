//! Building the display list from the VCD signal table.
//!
//! Base behavior: flat list of every (deduplicated) signal in declaration
//! order, labeled by full hierarchical path.
//!
//! Feature: glob filtering (`--signals` / `--exclude`). A glob pattern
//! (`*` = any sequence, `?` = one char, case-sensitive) is matched against
//! BOTH the full dot-joined path ("TOP.soc.clock") AND the leaf name
//! ("clock"); if either matches, the signal matches. `--signals` keeps only
//! matches, `--exclude` removes matches (exclude wins), an empty include
//! list means all signals.
//!
//! Feature: hierarchical grouping (`--group`). Group header rows are emitted
//! for every scope whose subtree still contains at least one visible signal
//! (scopes with none are omitted), in DFS pre-order: header first, then the
//! scope's directly-declared signals, then child scopes in declaration
//! order. Group rows carry a single-segment path so the name column shows
//! the bare scope name. Signal rows are trimmed to their leaf name and get
//! `depth = path.len() - 1` (their enclosing-scope count), so a signal sits
//! one indent level below its scope header; a signal declared outside any
//! scope (path length 1) gets depth 0 and no header.

use crate::cli::Opts;
use crate::types::Row;
use crate::vcd::{Signal, Scope, Vcd};
use std::collections::HashMap;

pub fn build(vcd: &Vcd, opts: &Opts) -> Vec<Row> {
    let visible: Vec<usize> = vcd
        .signals
        .iter()
        .enumerate()
        .filter(|(_, s)| is_visible(s, opts))
        .map(|(i, _)| i)
        .collect();

    if opts.group {
        grouped_rows(vcd, &visible)
    } else {
        visible
            .iter()
            .map(|&i| Row::Signal {
                signal: i,
                path: vcd.signals[i].path.clone(),
                depth: 0,
            })
            .collect()
    }
}

/// Filtering rule: excluded globs win over included globs; an empty include
/// list keeps everything.
fn is_visible(sig: &Signal, opts: &Opts) -> bool {
    if matches_any(&sig.path, &opts.exclude) {
        return false;
    }
    opts.signals.is_empty() || matches_any(&sig.path, &opts.signals)
}

/// A signal matches a pattern list if any pattern matches the full
/// dot-joined path or the leaf name.
fn matches_any(path: &[String], patterns: &[String]) -> bool {
    let full = path.join(".");
    let leaf = path.last().map(String::as_str).unwrap_or("");
    patterns
        .iter()
        .any(|p| !p.is_empty() && (glob_match(p, &full) || glob_match(p, leaf)))
}

/// Classic wildcard match with `*` (any sequence, including empty) and
/// `?` (exactly one char), case-sensitive, single-pass with backtracking.
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star_p = None;
    let mut star_t = 0usize;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_p = Some(pi);
            star_t = ti;
            pi += 1;
        } else if let Some(s) = star_p {
            // Backtrack: let the last '*' consume one more char.
            pi = s + 1;
            star_t += 1;
            ti = star_t;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Build grouped rows: DFS over the scope tree, emitting a Group header for
/// every scope whose subtree holds >=1 visible signal, then that scope's
/// direct signals, then its children.
fn grouped_rows(vcd: &Vcd, visible: &[usize]) -> Vec<Row> {
    // Bucket visible signal indices by their enclosing scope path so the
    // tree walk is O(signals) to build and O(1) lookup per scope.
    let mut by_scope: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    let mut unscoped: Vec<usize> = Vec::new();
    for &i in visible {
        let path = &vcd.signals[i].path;
        if path.len() <= 1 {
            unscoped.push(i);
        } else {
            by_scope
                .entry(path[..path.len() - 1].to_vec())
                .or_default()
                .push(i);
        }
    }

    let mut rows = Vec::new();
    // Signals declared outside any scope have no header; they keep depth 0
    // and lead the list (their scope "path" is empty).
    for &i in &unscoped {
        rows.push(signal_row(vcd, i));
    }
    for scope in &vcd.scopes {
        walk_scope(vcd, scope, &[], &by_scope, &mut rows);
    }
    rows
}

fn walk_scope(
    vcd: &Vcd,
    scope: &Scope,
    prefix: &[String],
    by_scope: &HashMap<Vec<String>, Vec<usize>>,
    rows: &mut Vec<Row>,
) {
    let mut scope_path = prefix.to_vec();
    scope_path.push(scope.name.clone());

    // The scope survives if any descendant bucket starts with this scope
    // path (i.e. the subtree still holds at least one visible signal).
    let subtree_visible = by_scope
        .keys()
        .any(|k| k.len() >= scope_path.len() && k[..scope_path.len()] == scope_path[..]);
    if !subtree_visible {
        return;
    }

    rows.push(Row::Group {
        path: vec![scope.name.clone()],
        depth: scope_path.len() - 1,
    });
    if let Some(indices) = by_scope.get(&scope_path) {
        for &i in indices {
            rows.push(signal_row(vcd, i));
        }
    }
    for child in &scope.children {
        walk_scope(vcd, child, &scope_path, by_scope, rows);
    }
}

fn signal_row(vcd: &Vcd, i: usize) -> Row {
    let sig = &vcd.signals[i];
    let leaf = sig.path.last().cloned().unwrap_or_default();
    Row::Signal {
        signal: i,
        path: vec![leaf],
        depth: sig.path.len().saturating_sub(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Opts;
    use crate::vcd::{Header, Signal, Scope, TimeUnit, Vcd};

    fn sig(path: &[&str]) -> Signal {
        Signal {
            id: path.join(".").into_bytes(),
            path: path.iter().map(|s| s.to_string()).collect(),
            width: 1,
            range: None,
            kind: "wire".into(),
        }
    }

    /// TOP
    ///  ├─ clk, data (direct)
    ///  └─ soc
    ///      ├─ cycle (direct)
    ///      └─ cpu
    ///          └─ pc
    fn test_vcd() -> Vcd {
        Vcd {
            header: Header {
                date: String::new(),
                version: String::new(),
                timescale: (1, TimeUnit::Ns),
            },
            scopes: vec![Scope {
                name: "TOP".into(),
                children: vec![Scope {
                    name: "soc".into(),
                    children: vec![Scope {
                        name: "cpu".into(),
                        children: vec![],
                    }],
                }],
            }],
            signals: vec![
                sig(&["TOP", "clk"]),
                sig(&["TOP", "data"]),
                sig(&["TOP", "soc", "cycle"]),
                sig(&["TOP", "soc", "cpu", "pc"]),
            ],
            changes: vec![vec![], vec![], vec![], vec![]],
            end_time: 0,
        }
    }

    // ---- glob matcher -----------------------------------------------------

    #[test]
    fn glob_literal_exact_match() {
        assert!(glob_match("clock", "clock"));
        assert!(!glob_match("clock", "clocks"));
        assert!(!glob_match("clocks", "clock"));
        assert!(!glob_match("Clock", "clock")); // case-sensitive
    }

    #[test]
    fn glob_star_matches_any_sequence() {
        assert!(glob_match("*", "anything at all"));
        assert!(glob_match("*", ""));
        assert!(glob_match("clk*", "clk"));
        assert!(glob_match("clk*", "clk_en"));
        assert!(glob_match("*clk", "spi_clk"));
        assert!(!glob_match("*clk", "clko"));
        assert!(glob_match("*spi*", "TOP.soc.spi_mosi"));
        assert!(!glob_match("*spi*", "TOP.soc.mosi"));
    }

    #[test]
    fn glob_question_matches_one_char() {
        assert!(glob_match("cl?", "clk"));
        assert!(!glob_match("cl?", "clkk"));
        assert!(!glob_match("cl?", "cl"));
        assert!(glob_match("TOP.soc.cl?ck", "TOP.soc.clock"));
        assert!(!glob_match("TOP.soc.cl?ck", "TOP.soc.clck2"));
    }

    #[test]
    fn glob_multiple_stars_backtrack() {
        assert!(glob_match("*a*b*", "xxaxxbyy"));
        assert!(glob_match("*a*b*", "ab"));
        assert!(glob_match("a*b*c", "aXbYc"));
        assert!(!glob_match("a*b*c", "aXbYd"));
        assert!(glob_match("*TOP*clock*", "TOP.Top.soc.clock"));
    }

    // ---- filtering ----------------------------------------------------------

    #[test]
    fn filter_matches_full_path_or_leaf() {
        let vcd = test_vcd();
        let mut opts = Opts::default();

        // Leaf-name pattern hits signals in every scope.
        opts.signals = vec!["clk".into()];
        let rows = build(&vcd, &opts);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label(), "TOP.clk");

        // Full-path pattern.
        opts.signals = vec!["TOP.soc.cycle".into()];
        let rows = build(&vcd, &opts);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label(), "TOP.soc.cycle");

        // Glob hits either way.
        opts.signals = vec!["*cpu*".into()];
        let rows = build(&vcd, &opts);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label(), "TOP.soc.cpu.pc");
    }

    #[test]
    fn exclude_wins_over_include() {
        let vcd = test_vcd();
        let mut opts = Opts::default();
        opts.signals = vec!["*".into()];
        opts.exclude = vec!["*clk*".into(), "*pc*".into()];
        let rows = build(&vcd, &opts);
        let labels: Vec<String> = rows.iter().map(|r| r.label()).collect();
        assert_eq!(labels, vec!["TOP.data", "TOP.soc.cycle"]);
    }

    #[test]
    fn empty_include_keeps_all_and_flat_stays_base() {
        let vcd = test_vcd();
        let opts = Opts::default();
        let rows = build(&vcd, &opts);
        assert_eq!(rows.len(), 4);
        for row in &rows {
            match row {
                Row::Signal { path, depth, .. } => {
                    assert_eq!(*depth, 0);
                    assert!(path.len() > 1); // full hierarchical path
                }
                Row::Group { .. } => panic!("flat mode must not emit groups"),
            }
        }
    }

    // ---- grouping -----------------------------------------------------------

    #[test]
    fn group_dfs_order_headers_before_members() {
        let vcd = test_vcd();
        let mut opts = Opts::default();
        opts.group = true;
        let rows = build(&vcd, &opts);

        // Expected DFS pre-order: TOP header, direct signals, then child
        // scopes (soc: header, direct signal, child scope cpu).
        let expected = [
            ("TOP", 0, true),
            ("clk", 1, false),
            ("data", 1, false),
            ("soc", 1, true),
            ("cycle", 2, false),
            ("cpu", 2, true),
            ("pc", 3, false),
        ];
        assert_eq!(rows.len(), expected.len());
        for (row, (label, depth, is_group)) in rows.iter().zip(expected) {
            assert_eq!(row.label(), label);
            assert_eq!(row.depth(), depth);
            assert_eq!(matches!(row, Row::Group { .. }), is_group);
        }
    }

    #[test]
    fn group_suppresses_scopes_without_visible_signals() {
        let vcd = test_vcd();
        let mut opts = Opts::default();
        opts.group = true;
        // cpu only holds pc; excluding pc empties cpu's subtree -> both the
        // cpu header and its member vanish, soc stays (cycle visible).
        opts.exclude = vec!["*pc*".into()];
        let rows = build(&vcd, &opts);
        let labels: Vec<String> = rows.iter().map(|r| r.label()).collect();
        assert_eq!(labels, vec!["TOP", "clk", "data", "soc", "cycle"]);
        assert_eq!(rows.iter().map(|r| r.depth()).collect::<Vec<_>>(), vec![0, 1, 1, 1, 2]);
    }

    #[test]
    fn group_respects_filtering() {
        let vcd = test_vcd();
        let mut opts = Opts::default();
        opts.group = true;
        // "*spi*" has no hit in this fixture; "*clock*" matches TOP.clk.
        opts.signals = vec!["*clk*".into()];
        let rows = build(&vcd, &opts);
        let labels: Vec<String> = rows.iter().map(|r| r.label()).collect();
        assert_eq!(labels, vec!["TOP", "clk"]);
    }

    #[test]
    fn unscoped_signal_gets_depth0_and_no_header() {
        let mut vcd = test_vcd();
        vcd.signals.insert(0, sig(&["rootless"]));
        vcd.changes.insert(0, vec![]);
        let mut opts = Opts::default();
        opts.group = true;
        let rows = build(&vcd, &opts);
        match &rows[0] {
            Row::Signal { path, depth, .. } => {
                assert_eq!(path.len(), 1);
                assert_eq!(*depth, 0);
            }
            Row::Group { .. } => panic!("no header expected for a scope-less signal"),
        }
        assert_eq!(rows[0].label(), "rootless");
    }
}
