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

/// Convert user seconds to raw ticks.
pub fn secs_to_ticks(secs: f64, tick_secs: f64) -> u64 {
    (secs / tick_secs).round().max(0.0) as u64
}

impl Layout {
    pub fn compute(vcd: &Vcd, opts: &Opts, rows: &[Row], _theme: &Theme) -> Layout {
        let (num, unit) = vcd.header.timescale;
        let tick_secs = num as f64 * unit.factor_secs();

        // ---- time window ------------------------------------------------
        let mut start = 0u64;
        let mut end = vcd.end_time.max(1);
        if let Some((a, b)) = opts.range {
            let a = secs_to_ticks(a, tick_secs);
            let b = secs_to_ticks(b, tick_secs);
            start = a.min(vcd.end_time);
            end = b.min(vcd.end_time).max(start + 1);
        }

        // ---- name column width ------------------------------------------
        let mut name_w: u32 = 220;
        let mut longest = 0usize;
        for r in rows {
            longest = longest.max(r.label().chars().count());
        }
        let depth_pad = rows.iter().map(|r| r.depth()).max().unwrap_or(0) as u32 * 14;
        let est = (longest as f64 * 7.2 + depth_pad as f64 + 24.0).ceil() as u32;
        name_w = name_w.max(est.min(420));

        // ---- geometry ----------------------------------------------------
        let svg_w = opts.width.max(400);
        let margin_r = 24u32;
        let wave_x0 = name_w as f64 + 18.0;
        let wave_x1 = (svg_w - margin_r) as f64;
        let row_h = opts.row_height.max(14);
        let title_h = 64u32;
        let axis_h = 30u32;
        let axis_b_h = 34u32;
        let heat_h = if opts.heat { 30u32 } else { 0 };
        let rows_y0 = (title_h + axis_h + heat_h + 12) as f64;
        let svg_h = rows_y0 as u32 + rows.len() as u32 * row_h + axis_b_h + 8;

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

/// Pick a "nice" tick step in seconds so that ticks are at least ~90px apart
/// and majors fall on 1/2/5 boundaries. Sub-ticks (minor) at 1/5 of major.
fn compute_ticks(start: u64, end: u64, px: f64, tick_secs: f64) -> Vec<Tick> {
    let span_secs = (end - start) as f64 * tick_secs;
    if span_secs <= 0.0 {
        return Vec::new();
    }
    let min_major_secs = 90.0 / px.max(1.0) * span_secs;
    // nice step: 1, 2, 5 x 10^k
    let mut step_secs = 10f64.powf(min_major_secs.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if step_secs * m >= min_major_secs {
            step_secs *= m;
            break;
        }
    }
    let mut ticks = Vec::new();
    let first = (start as f64 * tick_secs / step_secs).floor() * step_secs;
    let last = end as f64 * tick_secs;
    let mut t = first;
    while t <= last {
        let raw = (t / tick_secs).round();
        if raw >= start as f64 && raw <= end as f64 {
            ticks.push(Tick {
                time: raw as u64,
                label: format_secs(t, step_secs),
                major: true,
            });
        }
        t += step_secs;
    }
    ticks
}

/// Format seconds compactly with SI units: 0, 1.2µs, 1.5ms, 2.4s ...
pub fn format_secs(secs: f64, step: f64) -> String {
    let abs = secs.abs();
    if abs == 0.0 {
        return "0".into();
    }
    let (div, sym) = if abs >= 1.0 {
        (1.0, "s")
    } else if abs >= 1e-3 {
        (1e-3, "ms")
    } else if abs >= 1e-6 {
        (1e-6, "µs")
    } else if abs >= 1e-9 {
        (1e-9, "ns")
    } else {
        (1e-12, "ps")
    };
    let v = secs / div;
    let decimals = if step / div >= 10.0 {
        0
    } else if step / div >= 1.0 {
        1
    } else {
        2
    };
    format!("{:.*}{}", decimals, v, sym)
}
