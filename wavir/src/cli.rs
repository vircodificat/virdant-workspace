//! Command-line options for wavir.
//!
//! All feature flags are declared here; feature modules read the fields they
//! own. Parsing is dependency-free (std only).

#[derive(Debug, Clone)]
pub struct Opts {
    /// Input .vcd file.
    pub input: String,
    /// Output file (default: input with .svg / .html extension).
    pub output: Option<String>,

    // --- feature: sizing -------------------------------------------------
    /// Total SVG width in pixels.
    pub width: u32,
    /// Height of one waveform row in pixels.
    pub row_height: u32,

    // --- feature: range --------------------------------------------------
    /// Time window "start..end" in timescale units, e.g. "1.2us..1.4us".
    pub range: Option<(f64, f64)>,

    // --- feature: signals ------------------------------------------------
    /// Include only signals matching any of these globs (full or leaf name).
    pub signals: Vec<String>,
    /// Exclude signals matching any of these globs.
    pub exclude: Vec<String>,

    // --- feature: group --------------------------------------------------
    /// Render the scope hierarchy with group headers and indentation.
    pub group: bool,

    // --- feature: bus-labels (default on) --------------------------------
    /// Annotate bus segments with their values where space allows.
    pub bus_labels: bool,

    // --- feature: theme --------------------------------------------------
    pub theme: String,

    // --- feature: heat ---------------------------------------------------
    /// Transition-density heat strip (default on).
    pub heat: bool,

    // --- feature: mark ---------------------------------------------------
    /// Vertical time cursors, comma-separated list in timescale units.
    pub marks: Vec<f64>,

    // --- feature: html ---------------------------------------------------
    /// Emit an interactive HTML viewer instead of a static SVG.
    pub html: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            input: String::new(),
            output: None,
            width: 1280,
            row_height: 26,
            range: None,
            signals: Vec::new(),
            exclude: Vec::new(),
            group: false,
            bus_labels: true,
            theme: "dark".into(),
            heat: true,
            marks: Vec::new(),
            html: false,
        }
    }
}

#[derive(Debug)]
pub struct OptsError(pub String);

impl std::fmt::Display for OptsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn parse_args(args: &[String]) -> Result<Opts, OptsError> {
    let mut o = Opts::default();
    let mut i = 0;
    let mut positional: Vec<String> = Vec::new();

    let need_val = |flag: &str, i: &mut usize| -> Result<String, OptsError> {
        *i += 1;
        args.get(*i)
            .cloned()
            .ok_or_else(|| OptsError(format!("{} requires a value", flag)))
    };

    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-o" | "--output" => o.output = Some(need_val("--output", &mut i)?),
            "--width" => {
                let v = need_val("--width", &mut i)?;
                o.width = v
                    .parse()
                    .map_err(|_| OptsError(format!("bad --width: {}", v)))?;
            }
            "--row-height" => {
                let v = need_val("--row-height", &mut i)?;
                o.row_height = v
                    .parse()
                    .map_err(|_| OptsError(format!("bad --row-height: {}", v)))?;
            }
            "--range" => {
                let v = need_val("--range", &mut i)?;
                o.range = Some(parse_range(&v)?);
            }
            "--signals" => {
                let v = need_val("--signals", &mut i)?;
                o.signals = v.split(',').map(|s| s.trim().to_string()).collect();
            }
            "--exclude" => {
                let v = need_val("--exclude", &mut i)?;
                o.exclude = v.split(',').map(|s| s.trim().to_string()).collect();
            }
            "--group" => o.group = true,
            "--no-group" => o.group = false,
            "--bus-labels" => o.bus_labels = true,
            "--no-bus-labels" => o.bus_labels = false,
            "--theme" => o.theme = need_val("--theme", &mut i)?,
            "--heat" => o.heat = true,
            "--no-heat" => o.heat = false,
            "--mark" => {
                let v = need_val("--mark", &mut i)?;
                for tok in v.split(',') {
                    let t = tok.trim();
                    if t.is_empty() {
                        continue;
                    }
                    o.marks.push(parse_time(t).map_err(|_| {
                        OptsError(format!("bad --mark value: {}", t))
                    })?);
                }
            }
            "--html" => o.html = true,
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                if a.starts_with("--") || (a.starts_with('-') && a.len() > 1) {
                    return Err(OptsError(format!("unknown option: {}", a)));
                }
                positional.push(a.clone());
            }
        }
        i += 1;
    }

    if positional.len() != 1 {
        print_usage();
        return Err(OptsError("expected exactly one input file".into()));
    }
    o.input = positional.into_iter().next().unwrap();
    Ok(o)
}

/// Parse a time like "1.2us", "1500ns", "3ms" into timescale ticks.
/// Needs the timescale, so returns unit-seconds; conversion happens in layout.
pub fn parse_time(s: &str) -> Result<f64, ()> {
    let bytes = s.as_bytes();
    let mut num_end = 0;
    while num_end < bytes.len()
        && (bytes[num_end].is_ascii_digit() || bytes[num_end] == b'.' || bytes[num_end] == b'-')
    {
        num_end += 1;
    }
    if num_end == 0 {
        return Err(());
    }
    let num: f64 = s[..num_end].parse().map_err(|_| ())?;
    let unit = crate::vcd::TimeUnit::parse(&s[num_end..]).ok_or(())?;
    Ok(num * unit.factor_secs())
}

fn parse_range(s: &str) -> Result<(f64, f64), OptsError> {
    let (a, b) = s
        .split_once("..")
        .ok_or_else(|| OptsError(format!("--range expects start..end, got: {}", s)))?;
    let a = parse_time(a.trim()).map_err(|_| OptsError(format!("bad range start: {}", a)))?;
    let b = parse_time(b.trim()).map_err(|_| OptsError(format!("bad range end: {}", b)))?;
    if b <= a {
        return Err(OptsError("range end must be after start".into()));
    }
    Ok((a, b))
}

fn print_usage() {
    println!(
        "wavir — render a beautiful SVG waveform from a VCD dump

USAGE:
    wavir [OPTIONS] <trace.vcd>

OPTIONS:
    -o, --output <file>     Output file (default: <input>.svg)
    --width <px>            SVG width in pixels (default 1280)
    --row-height <px>       Row height in pixels (default 26)
    --range <start..end>    Time window, e.g. --range 1.2us..1.4us
    --signals <globs>       Show only matching signals (comma-separated globs)
    --exclude <globs>       Hide matching signals (comma-separated globs)
    --group                 Group signals hierarchically by scope
    --no-bus-labels         Disable bus value annotations
    --theme <name>          dark | light | terminal (default dark)
    --no-heat               Disable the transition-density heat strip
    --mark <t1,t2,...>      Draw labeled time cursors
    --html                  Emit an interactive HTML viewer instead of SVG
    -h, --help              Show this help"
    );
}
