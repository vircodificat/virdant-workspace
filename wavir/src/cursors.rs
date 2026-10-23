//! Feature: labeled time cursors (--mark t1,t2,...).
//!
//! Each mark is a user time in seconds. It is converted to raw VCD ticks via
//! `layout.tick_secs`, clipped to the visible window, and drawn as a dashed
//! vertical line spanning the waveform rows with a labeled chip near the top.
//! Chips are staggered vertically when marks land close together and are kept
//! inside the canvas horizontally.

use crate::svg::Svg;
use crate::theme::Theme;

pub struct Ctx<'a> {
    pub layout: &'a crate::layout::Layout,
    pub theme: &'a Theme,
    pub opts: &'a crate::cli::Opts,
}

/// Marks closer together than this (px) get staggered chip rows.
const NEAR_PX: f64 = 70.0;
/// Vertical step between staggered chip rows.
const CHIP_STEP_Y: f64 = 16.0;
/// Chip height in px.
const CHIP_H: f64 = 14.0;
/// Chip horizontal padding around the text.
const CHIP_PAD: f64 = 5.0;
/// Approximate px per character at the 10px chip font.
const PX_PER_CHAR: f64 = 6.6;

pub fn svg_defs(_theme: &Theme) -> String {
    String::new()
}

/// A mark resolved into pixel space.
struct Mark {
    /// Raw tick of the mark (within the window).
    _tick: u64,
    /// x pixel of the cursor line.
    x: f64,
    /// Original user time in seconds.
    secs: f64,
}

pub fn draw(svg: &mut Svg, ctx: &Ctx) {
    let l = ctx.layout;
    if ctx.opts.marks.is_empty() {
        return;
    }

    // Resolve marks to pixels, dropping ones outside the window.
    let mut marks: Vec<Mark> = Vec::new();
    for &secs in &ctx.opts.marks {
        let tick = crate::layout::secs_to_ticks(secs, l.tick_secs);
        if tick < l.window.start || tick > l.window.end {
            continue;
        }
        marks.push(Mark {
            _tick: tick,
            x: l.time_to_x(tick),
            secs,
        });
    }
    if marks.is_empty() {
        return;
    }

    // Number-format step: the smallest gap between consecutive marks, so
    // labels share digits and stay comparable; a lone mark uses the window
    // span. Window span in seconds.
    let span_secs = (l.window.end - l.window.start) as f64 * l.tick_secs;
    let mut sorted_secs: Vec<f64> = marks.iter().map(|m| m.secs).collect();
    sorted_secs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let step = format_step(&sorted_secs, span_secs);
    let labels: Vec<String> = marks
        .iter()
        .map(|m| crate::layout::format_secs(m.secs, step))
        .collect();

    // Vertical line span: from the top of the first row down to just above
    // the bottom axis strip.
    let y0 = l.rows_y0;
    let y1 = l.svg_h as f64 - l.axis_b_h as f64 - 8.0;

    // Pass 1: cursor lines (behind chips).
    svg.open("g", &[("shape-rendering", "geometricPrecision".into())]);
    for m in &marks {
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(m.x)),
                ("y1", crate::svg::fmt(y0)),
                ("x2", crate::svg::fmt(m.x)),
                ("y2", crate::svg::fmt(y1)),
                ("stroke", ctx.theme.cursor_color.into()),
                ("stroke-width", "1.5".into()),
                ("stroke-dasharray", "4,4".into()),
            ],
        );
    }
    svg.close("g");

    // Pass 2: chips (staggered, canvas-clamped).
    let widths: Vec<f64> = labels
        .iter()
        .map(|s| s.chars().count() as f64 * PX_PER_CHAR + 2.0 * CHIP_PAD)
        .collect();
    let xs: Vec<f64> = marks.iter().map(|m| m.x).collect();
    let chips = chip_layout(&xs, &widths, NEAR_PX, CHIP_STEP_Y, l.svg_w as f64);
    for (i, _m) in marks.iter().enumerate() {
        let (cx, level) = chips[i];
        let cy = y0 + level + CHIP_H / 2.0 + 2.0;
        svg.leaf(
            "rect",
            &[
                ("x", crate::svg::fmt(cx)),
                ("y", crate::svg::fmt(cy - CHIP_H / 2.0)),
                ("width", crate::svg::fmt(widths[i])),
                ("height", crate::svg::fmt(CHIP_H)),
                ("rx", "3".into()),
                ("fill", ctx.theme.bg.into()),
                ("stroke", ctx.theme.cursor_color.into()),
                ("stroke-width", "1".into()),
            ],
        );
        svg.text(
            cx + widths[i] / 2.0,
            cy,
            &labels[i],
            10.0,
            ctx.theme.cursor_color,
            "middle",
        );
    }
}

/// Pick the decimal-control step for chip labels: the smallest positive gap
/// between sorted marks, falling back to the window span.
fn format_step(secs: &[f64], span_secs: f64) -> f64 {
    let mut best = f64::INFINITY;
    for i in 1..secs.len() {
        let d = secs[i] - secs[i - 1];
        if d > 0.0 && d < best {
            best = d;
        }
    }
    if best.is_finite() {
        best
    } else {
        span_secs.max(1e-15)
    }
}

/// Chip placement: returns `(chip_left_x, y_offset)` per mark.
///
/// A mark whose x is within `near` px of the previous mark's x is staggered
/// one `step_y` further down; otherwise the offset resets to 0. Chips are
/// clamped horizontally so they stay fully inside `[0, canvas_w]`.
fn chip_layout(
    xs: &[f64],
    widths: &[f64],
    near: f64,
    step_y: f64,
    canvas_w: f64,
) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(xs.len());
    let mut level = 0.0f64;
    let mut prev_x = f64::NAN;
    for (i, &x) in xs.iter().enumerate() {
        if i > 0 && (x - prev_x).abs() < near {
            level += step_y;
        } else {
            level = 0.0;
        }
        prev_x = x;
        let w = widths[i];
        let left = (x - w / 2.0).max(2.0).min((canvas_w - w - 2.0).max(2.0));
        out.push((left, level));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stagger_accumulates_for_nearby_marks_and_resets() {
        // 10px apart -> levels 0, 16, 32; then far away -> reset to 0.
        let chips = chip_layout(&[100.0, 110.0, 120.0, 400.0], &[60.0; 4], 70.0, 16.0, 640.0);
        assert_eq!(chips[0].1, 0.0);
        assert_eq!(chips[1].1, 16.0);
        assert_eq!(chips[2].1, 32.0);
        assert_eq!(chips[3].1, 0.0);
    }

    #[test]
    fn distant_marks_share_one_row() {
        let chips = chip_layout(&[50.0, 200.0, 600.0], &[60.0; 3], 70.0, 16.0, 640.0);
        assert!(chips.iter().all(|c| c.1 == 0.0));
    }

    #[test]
    fn chips_are_clamped_inside_the_canvas() {
        // Left-edge mark and right-edge mark with a wide chip.
        let chips = chip_layout(&[5.0, 635.0], &[60.0; 2], 70.0, 16.0, 640.0);
        assert_eq!(chips[0].0, 2.0);
        assert!((chips[1].0 - (640.0 - 60.0 - 2.0)).abs() < 1e-9);
    }

    #[test]
    fn format_step_uses_smallest_gap_then_span() {
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(format_step(&[5e-3, 3.0e-2, 6.0e-2], 6.6e-2), 2.5e-2));
        assert!(close(format_step(&[1e-9], 5e-8), 5e-8));
        assert!(close(format_step(&[], 1e-6), 1e-6));
    }
}
