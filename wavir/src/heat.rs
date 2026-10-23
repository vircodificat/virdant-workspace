//! Feature: transition-density heat strip.
//!
//! Drawn above the waveform rows: one pixel column of color per time step
//! position, showing where the design is busiest. Density counts level
//! transitions across ALL displayed signal rows, projected into pixel
//! columns, normalized with an outlier-resistant cap (99th percentile of
//! non-zero columns) and log-scaled into the theme heat ramp.

use crate::layout::Layout;
use crate::svg::Svg;
use crate::theme::Theme;
use crate::types::{build_series, Row, Series};
use crate::vcd::Vcd;

/// Height of the colored strip inside the `heat_h` band (leaves room for
/// the frame lines and axis spacing).
const STRIP_H: f64 = 18.0;

pub struct Ctx<'a> {
    pub layout: &'a Layout,
    pub theme: &'a Theme,
    pub opts: &'a crate::cli::Opts,
    pub n_rows: usize,
}

pub fn svg_defs(_theme: &Theme) -> String {
    String::new()
}

pub fn draw(svg: &mut Svg, ctx: &Ctx, vcd: &Vcd, rows: &[Row]) {
    if !ctx.opts.heat || ctx.layout.heat_h == 0 {
        return;
    }
    let layout = ctx.layout;
    let x0 = layout.wave_x0;
    let x1 = layout.wave_x1;
    let n_cols = ((x1 - x0).ceil() as usize).max(1);
    let counts = density_columns(vcd, rows, layout, n_cols);
    let cap = outlier_cap(&counts);
    let ramp = ctx.theme.heat_ramp;
    let indices: Vec<usize> = counts
        .iter()
        .map(|&c| ramp_index(c, cap, ramp.len()))
        .collect();

    svg.open(
        "g",
        &[
            ("shape-rendering", "crispEdges".into()),
            ("stroke-width", "1".into()),
        ],
    );

    // One colored rect per run of same-ramp-index columns.
    for (start, len, idx) in merge_runs(&indices) {
        svg.leaf(
            "rect",
            &[
                ("x", crate::svg::fmt(x0 + start as f64)),
                ("y", crate::svg::fmt(layout.heat_y)),
                ("width", (len as f64).to_string()),
                ("height", crate::svg::fmt(STRIP_H)),
                ("fill", ramp[idx].into()),
            ],
        );
    }

    // Subtle top/bottom frame lines around the strip.
    for y in [layout.heat_y, layout.heat_y + STRIP_H] {
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(x0)),
                ("y1", crate::svg::fmt(y)),
                ("x2", crate::svg::fmt(x1)),
                ("y2", crate::svg::fmt(y)),
                ("stroke", ctx.theme.grid_major.into()),
            ],
        );
    }
    svg.close("g");

    // Small caption (drawn last so it sits over the strip edge).
    svg.text(
        12.0,
        layout.heat_y + STRIP_H * 0.5,
        "activity",
        9.0,
        ctx.theme.text_faint,
        "start",
    );
}

/// Per-pixel-column transition counts across all displayed signal rows.
fn density_columns(vcd: &Vcd, rows: &[Row], layout: &Layout, n_cols: usize) -> Vec<u64> {
    let mut counts = vec![0u64; n_cols];
    for row in rows {
        let Row::Signal { signal, .. } = row else {
            continue;
        };
        let series = build_series(vcd, *signal);
        for t in transition_times(&series) {
            if t < layout.window.start || t > layout.window.end {
                continue;
            }
            let x = layout
                .time_to_x(t)
                .clamp(layout.wave_x0, layout.wave_x1 - 1.0);
            let col = ((x - layout.wave_x0) as usize).min(n_cols - 1);
            counts[col] += 1;
        }
    }
    counts
}

/// Times (in raw ticks) at which a signal changes level: kind change, or a
/// bus value change between two digital (b'v') events.
fn transition_times(s: &Series) -> Vec<u64> {
    let mut out = Vec::new();
    for i in 1..s.events.len() {
        let prev = &s.events[i - 1];
        let cur = &s.events[i];
        let changed = if prev.kind != cur.kind {
            true
        } else if cur.kind == b'v' && !s.values.is_empty() {
            s.values[i] != s.values[i - 1]
        } else {
            false
        };
        if changed {
            out.push(cur.time);
        }
    }
    out
}

/// Outlier-resistant normalization cap: the 99th percentile of non-zero
/// column counts (so a single hot column does not flatten the whole strip),
/// minimum 1. Columns above the cap saturate to the ramp top.
fn outlier_cap(counts: &[u64]) -> u64 {
    let mut nz: Vec<u64> = counts.iter().copied().filter(|&c| c > 0).collect();
    if nz.is_empty() {
        return 1;
    }
    nz.sort_unstable();
    let idx = ((nz.len() - 1) as f64 * 0.99).round() as usize;
    nz[idx].max(1)
}

/// Map a column count to a heat ramp index: 0 stays 0; non-zero counts are
/// log-scaled against `cap` so low-activity regions stay visible.
pub fn ramp_index(count: u64, cap: u64, ramp_len: usize) -> usize {
    if ramp_len <= 1 {
        return 0;
    }
    if count == 0 {
        return 0;
    }
    let cap = cap.max(1) as f64;
    let c = (count as f64).min(cap);
    let norm = (1.0 + c).ln() / (1.0 + cap).ln();
    let idx = (norm * (ramp_len - 1) as f64).round() as i64;
    idx.clamp(0, ramp_len as i64 - 1) as usize
}

/// Merge contiguous equal ramp indices into runs: `(start_col, len, index)`.
fn merge_runs(indices: &[usize]) -> Vec<(usize, usize, usize)> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < indices.len() {
        let mut j = i;
        while j < indices.len() && indices[j] == indices[i] {
            j += 1;
        }
        runs.push((i, j - i, indices[i]));
        i = j;
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Opts;
    use crate::theme::by_name;
    use crate::vcd::{Change, Header, Signal, TimeUnit, Value};

    fn minimal_vcd() -> Vcd {
        Vcd {
            header: Header {
                date: String::new(),
                version: String::new(),
                timescale: (1, TimeUnit::Ns),
            },
            scopes: Vec::new(),
            signals: vec![Signal {
                id: b"!".to_vec(),
                path: vec!["TOP".to_string(), "clk".to_string()],
                width: 1,
                range: None,
                kind: "wire".to_string(),
            }],
            changes: vec![vec![
                Change {
                    time: 0,
                    value: Value::Bit(b'0'),
                },
                Change {
                    time: 10,
                    value: Value::Bit(b'1'),
                },
                Change {
                    time: 20,
                    value: Value::Bit(b'0'),
                },
            ]],
            end_time: 30,
        }
    }

    fn layout_for(opts: &Opts, rows: &[Row], vcd: &Vcd) -> Layout {
        Layout::compute(vcd, opts, rows, by_name("dark"))
    }

    #[test]
    fn ramp_index_maps_log_scale() {
        assert_eq!(ramp_index(0, 100, 6), 0, "zero count stays at ramp base");
        assert_eq!(ramp_index(100, 100, 6), 5, "cap saturates at ramp top");
        assert_eq!(ramp_index(1_000_000, 100, 6), 5, "outliers clamp to ramp top");
        assert_eq!(ramp_index(5, 100, 1), 0, "single-slot ramp");
        assert_eq!(ramp_index(5, 100, 0), 0, "empty ramp is safe");
        // log scale is monotonic
        let seq: Vec<usize> = (1..=100u64)
            .map(|c| ramp_index(c, 100, 6))
            .collect();
        let mut sorted = seq.clone();
        sorted.sort_unstable();
        assert_eq!(seq, sorted);
    }

    #[test]
    fn outlier_cap_ignores_single_hot_column() {
        let mut counts = vec![0u64; 1000];
        counts[..999].fill(3);
        counts[999] = 1_000_000;
        // 99th percentile of non-zero columns is 3, so the hot column does
        // not become the cap.
        assert_eq!(outlier_cap(&counts), 3);
        assert_eq!(outlier_cap(&[0, 0, 0]), 1, "all-empty data");
        assert_eq!(outlier_cap(&[]), 1, "no columns");
    }

    #[test]
    fn empty_data_is_a_no_op() {
        let vcd = minimal_vcd();
        let mut opts = Opts::default();
        opts.heat = false;
        let rows: Vec<Row> = Vec::new();
        let layout = layout_for(&opts, &rows, &vcd);
        assert_eq!(layout.heat_h, 0);
        let ctx = Ctx {
            layout: &layout,
            theme: by_name("dark"),
            opts: &opts,
            n_rows: 0,
        };
        let mut svg = Svg::new("monospace");
        draw(&mut svg, &ctx, &vcd, &rows);
        let out = svg.finish(400, 200, "t", "");
        // heat disabled: nothing drawn at all
        assert!(!out.contains("activity"));
        assert!(!out.contains("rect"));

        // density with no rows: all-zero columns
        let counts = density_columns(&vcd, &rows, &layout, 100);
        assert!(counts.iter().all(|&c| c == 0));
    }

    #[test]
    fn transitions_project_into_columns() {
        let vcd = minimal_vcd();
        let rows = vec![Row::Signal {
            signal: 0,
            path: vec!["TOP".to_string(), "clk".to_string()],
            depth: 0,
        }];
        let opts = Opts::default();
        let layout = layout_for(&opts, &rows, &vcd);
        let n_cols = (layout.wave_x1 - layout.wave_x0) as usize;
        let counts = density_columns(&vcd, &rows, &layout, n_cols);
        let total: u64 = counts.iter().sum();
        assert_eq!(total, 2, "two transitions (at t=10 and t=20)");
        // transitions at 1/3 and 2/3 across the window
        let cols: Vec<usize> = counts
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c > 0)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(cols.len(), 2);
        assert!(cols[0] < n_cols / 3 + 1, "t=10 near a third of the window");
        assert!(cols[1] > 2 * n_cols / 3 - 1, "t=20 near two thirds");
    }

    #[test]
    fn runs_merge_contiguous_columns() {
        assert_eq!(
            merge_runs(&[0, 0, 2, 2, 2, 5]),
            vec![(0, 2, 0), (2, 3, 2), (5, 1, 5)]
        );
        assert!(merge_runs(&[]).is_empty());
    }
}
