//! Layout computation: canvas geometry, row placement, time-axis ticks.
//!
//! Times inside wavir are raw VCD "ticks". User-facing times (ranges, marks)
//! are seconds and are converted here using the VCD timescale.

use crate::cli::Opts;
use crate::theme::Theme;
use crate::types::Row;
use crate::vcd::Vcd;

pub struct TimeAxis {
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone)]
pub struct Tick {
    /// Raw time of the tick.
    pub time: u64,
    /// Label in user units.
    pub label: String,
    pub major: bool,
}

pub struct Layout {
    pub svg_w: u32,
    pub svg_h: u32,
    pub margin_l: u32,
    pub margin_r: u32,
    /// Height of the title band at the top.
    pub title_h: u32,
    /// Height of the top time axis strip.
    pub axis_h: u32,
    /// Height of the bottom time axis strip.
    pub axis_b_h: u32,
    /// Heat strip height (0 if disabled).
    pub heat_h: u32,
    pub name_w: u32,
    pub row_h: u32,
    /// x pixel where waveforms start.
    pub wave_x0: f64,
    /// x pixel where waveforms end.
    pub wave_x1: f64,
    /// y pixel of the top of the first row.
    pub rows_y0: f64,
    pub window: TimeAxis,
    pub ticks: Vec<Tick>,
    /// Heat strip top y (if enabled).
    pub heat_y: f64,
    /// Conversion: raw ticks -> seconds.
    pub tick_secs: f64,
}

/// Minimum width of the waveform area preserved even on very narrow canvases.
const MIN_WAVE_W: u32 = 80;
/// Row-height bounds: readable, but a bad flag value cannot produce an
/// absurd canvas.
const MIN_ROW_H: u32 = 14;
const MAX_ROW_H: u32 = 200;
/// Major ticks are kept at least this many pixels apart.
const MIN_MAJOR_PX: f64 = 90.0;

/// Convert user seconds to raw ticks (negative values clamp to 0).
pub fn secs_to_ticks(secs: f64, tick_secs: f64) -> u64 {
    (secs / tick_secs).round().max(0.0) as u64
}

/// Resolve the rendered time window in raw ticks.
///
/// - No range: the whole trace, but always at least one tick wide.
/// - A range is clamped to `[0, end_time]` (negative starts at 0).
/// - A range entirely after (or before) the data clamps to the data edge.
/// - A window smaller than one tick falls back to a 1-tick span so
///   rendering stays sane.
fn time_window(data_end: u64, range: Option<(f64, f64)>, tick_secs: f64) -> (u64, u64) {
    let data_end = data_end.max(1);
    let Some((a, b)) = range else {
        return (0, data_end);
    };
    // Defensive: cli rejects end <= start, but keep the window well-formed
    // even if compute() is called with a swapped pair.
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let lo_t = secs_to_ticks(lo, tick_secs);
    let hi_t = secs_to_ticks(hi, tick_secs);
    if lo_t > data_end {
        // Entirely after the data: clamp to the last tick of the data.
        return (data_end - 1, data_end);
    }
    if hi_t == 0 {
        // Entirely before the data start (data always begins at tick 0):
        // clamp to the first tick.
        return (0, 1);
    }
    let start = lo_t.min(data_end);
    let end = hi_t.min(data_end);
    if end <= start {
        // Window smaller than one tick: fall back to a 1-tick span.
        if start < data_end {
            (start, start + 1)
        } else {
            (data_end - 1, data_end)
        }
    } else {
        (start, end)
    }
}

/// Pick the smallest "nice" step (1/2/5 x 10^k seconds) that is at least
/// `min_secs`.
fn nice_step(min_secs: f64) -> f64 {
    if !(min_secs > 0.0) {
        return 1.0;
    }
    let base = 10f64.powf(min_secs.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if base * m >= min_secs {
            return base * m;
        }
    }
    base * 10.0
}

impl Layout {
    pub fn compute(vcd: &Vcd, opts: &Opts, rows: &[Row], _theme: &Theme) -> Layout {
        let (num, unit) = vcd.header.timescale;
        let tick_secs = num as f64 * unit.factor_secs();

        // ---- time window ------------------------------------------------
        let (start, end) = time_window(vcd.end_time, opts.range, tick_secs);

        // ---- name column width ------------------------------------------
        let mut longest = 0usize;
        for r in rows {
            longest = longest.max(r.label().chars().count());
        }
        let depth_pad = rows.iter().map(|r| r.depth()).max().unwrap_or(0) as u32 * 14;
        let est = (longest as f64 * 7.2 + depth_pad as f64 + 24.0).ceil() as u32;
        let base_name_w = 220u32.max(est.min(420));

        // ---- geometry ----------------------------------------------------
        let svg_w = opts.width.clamp(400, 16_384);
        let margin_r = 24u32;
        // Never starve the wave area: clamp the name column so at least
        // MIN_WAVE_W pixels of waveform remain on narrow canvases.
        let max_name = svg_w.saturating_sub(margin_r).saturating_sub(MIN_WAVE_W);
        let name_w = base_name_w.min(max_name);
        let wave_x0 = name_w as f64 + 18.0;
        let wave_x1 = (svg_w - margin_r) as f64;
        let row_h = opts.row_height.clamp(MIN_ROW_H, MAX_ROW_H);
        let title_h = 64u32;
        let axis_h = 30u32;
        let axis_b_h = 34u32;
        let heat_h = if opts.heat { 30u32 } else { 0 };
        let rows_y0 = (title_h + axis_h + heat_h + 12) as f64;
        // u64 math: a huge row count saturates instead of wrapping u32.
        let svg_h = (rows_y0 as u64
            + rows.len() as u64 * row_h as u64
            + axis_b_h as u64
            + 8)
        .min(u32::MAX as u64) as u32;

        // ---- ticks -------------------------------------------------------
        let ticks = compute_ticks(start, end, wave_x1 - wave_x0, tick_secs);

        Layout {
            svg_w,
            svg_h,
            margin_l: 16,
            margin_r,
            title_h,
            axis_h,
            axis_b_h,
            heat_h,
            name_w,
            row_h,
            wave_x0,
            wave_x1,
            rows_y0,
            window: TimeAxis { start, end },
            ticks,
            heat_y: (title_h + axis_h + 6) as f64,
            tick_secs,
        }
    }

    pub fn time_to_x(&self, t: u64) -> f64 {
        let span = (self.window.end - self.window.start).max(1) as f64;
        let px = self.wave_x1 - self.wave_x0;
        self.wave_x0 + (t - self.window.start) as f64 * (px / span)
    }
}

/// Compute major time ticks across `[start, end]` (raw ticks).
///
/// Rules:
/// - Major ticks are 1/2/5 x 10^k seconds apart, at least ~90px (and never
///   finer than one raw tick, which would produce duplicate labels).
/// - Narrow windows get fewer, well-spaced majors rather than collisions.
/// - Minor gridlines (drawn by the renderer as 1/5-of-major subdivisions)
///   stay sensible because majors are always at least one raw tick apart.
fn compute_ticks(start: u64, end: u64, px: f64, tick_secs: f64) -> Vec<Tick> {
    if end <= start || px <= 0.0 || !(tick_secs > 0.0) {
        return Vec::new();
    }
    let span_secs = (end - start) as f64 * tick_secs;
    let min_major_secs = MIN_MAJOR_PX / px.max(1.0) * span_secs;
    // Never finer than one raw tick: a finer step would round several
    // candidates onto the same tick time and duplicate labels.
    let min_step = min_major_secs.max(tick_secs);
    let step = nice_step(min_step);

    let start_secs = start as f64 * tick_secs;
    let end_secs = end as f64 * tick_secs;
    // First candidate at or below the window start, as an exact multiple of
    // the step (integer-indexed iteration avoids float drift accumulation).
    let first = start_secs - start_secs % step;
    let mut ticks = Vec::new();
    let mut k = 0i64;
    // The step guarantees >=90px (or >=1 raw tick) spacing, so the loop is
    // short; the cap is a defensive guard against pathological input.
    while k < 512 {
        let t = first + k as f64 * step;
        // Relative epsilon: a candidate that rounds exactly onto the window
        // end tick must not be dropped by float noise (e.g. 0.0101 vs
        // 0.010100000000000002).
        if t > end_secs + step * 1e-9 {
            break;
        }
        let raw = (t / tick_secs).round();
        if raw >= start as f64 && raw <= end as f64 {
            let raw = raw as u64;
            // Defensive: float rounding must never emit a duplicate tick.
            if ticks.last().map_or(true, |prev: &Tick| raw > prev.time) {
                ticks.push(Tick {
                    time: raw,
                    label: format_secs(t, step),
                    major: true,
                });
            }
        }
        k += 1;
    }
    ticks
}

/// SI time units, largest first.
const SEC_UNITS: [(f64, &str); 6] = [
    (1.0, "s"),
    (1e-3, "ms"),
    (1e-6, "µs"),
    (1e-9, "ns"),
    (1e-12, "ps"),
    (1e-15, "fs"),
];

/// Format seconds compactly with SI units: 0, 500µs, 1.5ms, 2.4s ...
///
/// The unit is chosen so the mantissa is always >= 1 (values read naturally:
/// "500µs", never "0.05ms" or "0.5ms"). Enough decimals are emitted that
/// ticks one `step` apart stay distinct, then trailing zeros are trimmed
/// ("2.0ms" -> "2ms").
pub fn format_secs(secs: f64, step: f64) -> String {
    let abs = secs.abs();
    if abs == 0.0 {
        return "0".into();
    }
    // Largest unit whose scaled mantissa is >= 1 (tiny epsilon guards the
    // boundary against float noise, e.g. 0.9999999999ms reads as "1ms").
    let (div, sym) = SEC_UNITS
        .iter()
        .copied()
        .find(|&(d, _)| abs >= d * (1.0 - 1e-9))
        .unwrap_or((1e-15, "fs"));
    let v = secs / div;
    // Decimals needed so consecutive ticks one `step` apart stay distinct.
    let decimals = if step > 0.0 {
        let d = (-(step / div).log10()).ceil();
        d.clamp(0.0, 9.0) as usize
    } else {
        2
    };
    let mut s = format!("{:.*}", decimals, v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s + sym
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Opts;
    use crate::theme;
    use crate::vcd::{Header, TimeUnit, Vcd};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= b.abs().max(1e-30) * 1e-9
    }

    fn make_vcd(timescale: (u64, TimeUnit), end_time: u64) -> Vcd {
        Vcd {
            header: Header {
                date: String::new(),
                version: String::new(),
                timescale,
            },
            scopes: Vec::new(),
            signals: Vec::new(),
            changes: Vec::new(),
            end_time,
        }
    }

    fn layout(ts: (u64, TimeUnit), end: u64, opts: &Opts) -> Layout {
        Layout::compute(&make_vcd(ts, end), opts, &[], theme::by_name("dark"))
    }

    // ---- nice-step selection --------------------------------------------

    #[test]
    fn nice_step_picks_1_2_5() {
        for (min, want) in [
            (0.0082, 0.01),
            (0.0101, 0.02),
            (0.021, 0.05),
            (0.051, 0.1),
            (0.1, 0.1),
            (0.2, 0.2),
            (0.6, 1.0),
            (3e-9, 5e-9),
            (2.5e-6, 5e-6),
            (1e-15, 1e-15),
            (7e-12, 1e-11),
        ] {
            assert!(close(nice_step(min), want), "nice_step({min}) != {want}");
        }
    }

    #[test]
    fn nice_step_always_meets_minimum() {
        let mut m = 1e-18;
        for _ in 0..40 {
            let s = nice_step(m);
            assert!(s >= m, "step {s} < min {m}");
            m *= 3.7;
        }
    }

    // ---- format_secs ------------------------------------------------------

    #[test]
    fn format_secs_zero_is_plain_zero() {
        assert_eq!(format_secs(0.0, 1e-9), "0");
    }

    #[test]
    fn format_secs_across_units() {
        // (value, step, expected)
        for (v, step, want) in [
            (1e-15, 1e-15, "1fs"),
            (5e-13, 1e-13, "500fs"), // mantissa < 1 ps -> drop to fs
            (1e-12, 1e-12, "1ps"),
            (2.5e-9, 0.5e-9, "2.5ns"),
            (5e-7, 1e-7, "500ns"), // 0.5us would violate mantissa >= 1
            (1e-6, 1e-6, "1µs"),
            (2.4e-6, 0.1e-6, "2.4µs"),
            (2.4e-6, 1e-6, "2µs"), // off-grid value rounds to the step grid
            (5e-4, 1e-4, "500µs"), // never "0.5ms"
            (1e-3, 1e-3, "1ms"),
            (2.0e-3, 1e-3, "2ms"), // trailing zeros trimmed
            (0.05, 0.01, "50ms"),  // never "0.05ms" or "0.0500ms"
            (1.5, 0.5, "1.5s"),
            (1000.0, 100.0, "1000s"),
        ] {
            assert_eq!(format_secs(v, step), want, "format_secs({v}, {step})");
        }
    }

    #[test]
    fn format_secs_keeps_narrow_window_labels_distinct() {
        // 100ps step inside an ms-range value: enough decimals to differ.
        let a = format_secs(66.639999e-3, 1e-10);
        let b = format_secs(66.640000e-3, 1e-10);
        assert_ne!(a, b);
        assert_eq!(a, "66.639999ms");
        assert_eq!(b, "66.64ms");
    }

    // ---- secs_to_ticks ----------------------------------------------------

    #[test]
    fn secs_to_ticks_clamps_and_rounds() {
        assert_eq!(secs_to_ticks(0.0, 1e-9), 0);
        assert_eq!(secs_to_ticks(-5e-9, 1e-9), 0);
        assert_eq!(secs_to_ticks(10.4e-9, 1e-9), 10);
        assert_eq!(secs_to_ticks(10.6e-9, 1e-9), 11);
        assert_eq!(secs_to_ticks(10e-3, 1e-12), 10_000_000_000);
    }

    // ---- range / window ---------------------------------------------------

    #[test]
    fn window_default_is_whole_trace() {
        let opts = Opts::default();
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!((l.window.start, l.window.end), (0, 50));
    }

    #[test]
    fn window_range_inside_data() {
        // riscv-like: 1ps ticks, 66.64ms trace, window 10ms..10.1ms.
        let opts = Opts {
            range: Some((0.010, 0.0101)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ps), 66_640_000_000, &opts);
        assert_eq!(l.window.start, 10_000_000_000);
        assert_eq!(l.window.end, 10_100_000_000);
    }

    #[test]
    fn window_range_clamps_to_data_end() {
        let opts = Opts {
            range: Some((0.060, 0.080)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ps), 66_640_000_000, &opts);
        assert_eq!(l.window.start, 60_000_000_000);
        assert_eq!(l.window.end, 66_640_000_000);
    }

    #[test]
    fn window_entirely_after_data_clamps_to_edge() {
        let opts = Opts {
            range: Some((0.100, 0.110)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ps), 66_640_000_000, &opts);
        assert_eq!(l.window.end, 66_640_000_000);
        assert_eq!(l.window.end - l.window.start, 1);
    }

    #[test]
    fn window_entirely_before_data_clamps_to_start() {
        let opts = Opts {
            range: Some((-0.010, -0.005)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!((l.window.start, l.window.end), (0, 1));
    }

    #[test]
    fn window_smaller_than_one_tick_falls_back() {
        // 1ns ticks: a 0..0.4ns range rounds to a zero-tick span -> 1 tick.
        let opts = Opts {
            range: Some((0.0, 0.4e-9)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!((l.window.start, l.window.end), (0, 1));
        // Sub-tick window mid-trace.
        let opts = Opts {
            range: Some((10e-9, 10.4e-9)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!((l.window.start, l.window.end), (10, 11));
    }

    #[test]
    fn window_swapped_range_stays_well_formed() {
        let opts = Opts {
            range: Some((10.4e-9, 10e-9)),
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert!(l.window.end > l.window.start);
    }

    // ---- time-axis ticks --------------------------------------------------

    #[test]
    fn ticks_first_label_is_plain_zero() {
        let ticks = compute_ticks(0, 50, 1018.0, 1e-9);
        assert_eq!(ticks.first().unwrap().label, "0");
        assert_eq!(ticks.first().unwrap().time, 0);
    }

    #[test]
    fn ticks_are_nicely_spaced_and_unique() {
        let (start, end, px, tick_secs) = (0u64, 50u64, 1018.0, 1e-9);
        let ticks = compute_ticks(start, end, px, tick_secs);
        assert!(ticks.len() >= 2);
        let mut labels = std::collections::HashSet::new();
        for w in ticks.windows(2) {
            assert!(w[1].time > w[0].time, "duplicate/overlapping ticks");
            let px_gap = (w[1].time - w[0].time) as f64 / (end - start) as f64 * px;
            assert!(px_gap >= 90.0, "major spacing {px_gap}px < 90px");
            labels.insert(w[0].label.clone());
        }
        labels.insert(ticks.last().unwrap().label.clone());
        assert_eq!(labels.len(), ticks.len(), "duplicate labels");
    }

    #[test]
    fn ticks_labels_natural_units() {
        // 5ns majors on a 1ns trace: no trailing ".0".
        let ticks = compute_ticks(0, 50, 1018.0, 1e-9);
        assert_eq!(ticks[1].label, "5ns");
        assert_eq!(ticks.last().unwrap().label, "50ns");
    }

    #[test]
    fn ticks_one_tick_window_stays_sane() {
        // Window of exactly one raw tick: two majors, distinct labels.
        let ticks = compute_ticks(10, 11, 1000.0, 1e-9);
        assert_eq!(ticks.len(), 2);
        assert_eq!(ticks[0].time, 10);
        assert_eq!(ticks[1].time, 11);
        assert_ne!(ticks[0].label, ticks[1].label);
    }

    #[test]
    fn ticks_narrow_window_does_not_duplicate() {
        // Zoomed window (100us of a 66ms trace, 1ps ticks): labels must be
        // distinct and majors >= 90px apart.
        let (start, end) = (10_000_000_000u64, 10_100_000_000u64);
        let ticks = compute_ticks(start, end, 1100.0, 1e-12);
        assert!(ticks.len() >= 5);
        let mut labels = std::collections::HashSet::new();
        for w in ticks.windows(2) {
            assert!(w[1].time > w[0].time);
            let px_gap =
                (w[1].time - w[0].time) as f64 / (end - start) as f64 * 1100.0;
            assert!(px_gap >= 90.0, "major spacing {px_gap}px < 90px");
            labels.insert(w[0].label.clone());
        }
        labels.insert(ticks.last().unwrap().label.clone());
        assert_eq!(labels.len(), ticks.len(), "duplicate labels");
        assert_eq!(ticks.first().unwrap().label, "10ms");
    }

    #[test]
    fn ticks_include_window_end_tick() {
        // Float noise at the right edge (0.0101 vs 0.01010000...002) must
        // not drop the final major tick.
        let (start, end) = (10_000_000_000u64, 10_100_000_000u64);
        let ticks = compute_ticks(start, end, 818.0, 1e-12);
        assert_eq!(ticks.first().unwrap().time, start);
        assert_eq!(ticks.last().unwrap().time, end);
        assert_eq!(ticks.last().unwrap().label, "10.1ms");
    }

    #[test]
    fn ticks_non_nice_timescale_never_duplicates() {
        // 3ns ticks (not a 1/2/5 unit): step must still exceed one raw tick.
        for px in [300.0, 800.0, 2000.0] {
            let ticks = compute_ticks(0, 40, px, 3e-9);
            for w in ticks.windows(2) {
                assert!(w[1].time > w[0].time, "px={px} duplicate ticks");
            }
            let mut labels = std::collections::HashSet::new();
            for t in &ticks {
                assert!(labels.insert(t.label.clone()), "px={px} duplicate label");
            }
        }
    }

    // ---- sizing -----------------------------------------------------------

    #[test]
    fn sizing_honors_width_and_row_height() {
        let rows: Vec<Row> = (0..3)
            .map(|i| Row::Signal {
                signal: i,
                path: vec![format!("sig{i}")],
                depth: 0,
            })
            .collect();
        let vcd = make_vcd((1, TimeUnit::Ns), 50);
        let opts = Opts {
            width: 800,
            row_height: 40,
            heat: true,
            ..Opts::default()
        };
        let l = Layout::compute(&vcd, &opts, &rows, theme::by_name("dark"));
        assert_eq!(l.svg_w, 800);
        assert_eq!(l.row_h, 40);
        // rows_y0 = 64 + 30 + 30(heat) + 12 = 136; + 3*40 + 34 + 8 = 298.
        assert_eq!(l.svg_h, 298);
        assert!((l.rows_y0 - 136.0).abs() < 1e-9);
    }

    #[test]
    fn sizing_svg_h_consistent_without_heat() {
        let rows: Vec<Row> = (0..3)
            .map(|i| Row::Signal {
                signal: i,
                path: vec![format!("sig{i}")],
                depth: 0,
            })
            .collect();
        let vcd = make_vcd((1, TimeUnit::Ns), 50);
        let opts = Opts {
            width: 800,
            row_height: 40,
            heat: false,
            ..Opts::default()
        };
        let l = Layout::compute(&vcd, &opts, &rows, theme::by_name("dark"));
        assert_eq!(l.heat_h, 0);
        // rows_y0 = 64 + 30 + 0 + 12 = 106; + 3*40 + 34 + 8 = 268.
        assert_eq!(l.svg_h, 268);
        assert!((l.rows_y0 - 106.0).abs() < 1e-9);
    }

    #[test]
    fn sizing_hardens_bounds() {
        let opts = Opts {
            width: 1, // clamps up to 400
            row_height: 1, // clamps up to 14
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!(l.svg_w, 400);
        assert_eq!(l.row_h, 14);
        // Wave area keeps a usable width even on a narrow canvas.
        assert!(l.wave_x1 > l.wave_x0 + MIN_WAVE_W as f64);

        let opts = Opts {
            row_height: 100_000, // clamps down
            ..Opts::default()
        };
        let l = layout((1, TimeUnit::Ns), 50, &opts);
        assert_eq!(l.row_h, MAX_ROW_H);
    }

    #[test]
    fn sizing_empty_rows_keeps_sane_canvas() {
        let l = layout((1, TimeUnit::Ns), 50, &Opts::default());
        assert!(l.svg_h >= l.rows_y0 as u32 + l.axis_b_h);
    }
}
