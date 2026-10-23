//! The SVG renderer: turns a parsed VCD + layout into a beautiful waveform.
//!
//! Feature hooks (each owned by its own module, so lanes can work in
//! parallel without touching this file):
//!   - `bus::svg_defs()`      — pattern/gradient defs (X/Z bus styling)
//!   - `heat::draw()`         — transition-density strip above the rows
//!   - `cursors::draw()`      — --mark time cursors over everything

use crate::bus;
use crate::cli::Opts;
use crate::layout::Layout;
use crate::svg::Svg;
use crate::theme::Theme;
use crate::types::{build_series, Row, Series};
use crate::vcd::Vcd;

struct Ctx<'a> {
    layout: &'a Layout,
    theme: &'a Theme,
    opts: &'a Opts,
    n_rows: usize,
}

pub fn render(vcd: &Vcd, opts: &Opts, theme: &Theme) -> String {
    let rows = crate::display::build(vcd, opts);
    let layout = Layout::compute(vcd, opts, &rows, theme);
    let ctx = Ctx {
        layout: &layout,
        theme,
        opts,
        n_rows: rows.len(),
    };

    let mut svg = Svg::new(theme.font_family);

    // full-canvas background, first of all
    svg.leaf(
        "rect",
        &[
            ("x", "0".into()),
            ("y", "0".into()),
            ("width", layout.svg_w.to_string()),
            ("height", layout.svg_h.to_string()),
            ("fill", theme.bg.into()),
        ],
    );

    let n_signals = rows
        .iter()
        .filter(|r| matches!(r, Row::Signal { .. }))
        .count();
    let title = format!(
        "{} — {} signals, {} .. {} (timescale {}{})",
        opts.input,
        n_signals,
        crate::layout::format_secs(layout.window.start as f64 * layout.tick_secs, layout.tick_secs),
        crate::layout::format_secs(layout.window.end as f64 * layout.tick_secs, layout.tick_secs),
        vcd.header.timescale.0,
        vcd.header.timescale.1.symbol()
    );

    // ---- grid (behind everything else in the wave area) ------------------
    draw_grid(&mut svg, &ctx);

    // ---- top axis ---------------------------------------------------------
    draw_axis(&mut svg, &ctx, layout.title_h as f64 + 4.0);

    // ---- waveform rows ----------------------------------------------------
    let mut y = layout.rows_y0;
    for (ri, row) in rows.iter().enumerate() {
        draw_row(&mut svg, &ctx, row, ri, y, vcd);
        y += layout.row_h as f64;
    }

    // ---- bottom axis ------------------------------------------------------
    let axis_b_y = layout.rows_y0 + rows.len() as f64 * layout.row_h as f64 + 4.0;
    draw_axis(&mut svg, &ctx, axis_b_y);

    // ---- name panel (drawn after waves so labels stay readable) -----------
    svg.leaf(
        "rect",
        &[
            ("x", "0".into()),
            ("y", "0".into()),
            ("width", (layout.name_w + 10).to_string()),
            ("height", layout.svg_h.to_string()),
            ("fill", theme.name_panel.into()),
        ],
    );
    let mut y = layout.rows_y0;
    for row in rows.iter() {
        draw_name(&mut svg, &ctx, row, y);
        y += layout.row_h as f64;
    }
    svg.leaf(
        "rect",
        &[
            ("x", (layout.name_w + 10).to_string()),
            ("y", "0".into()),
            ("width", "2".into()),
            ("height", layout.svg_h.to_string()),
            ("fill", theme.grid_major.into()),
        ],
    );

    // ---- title band (drawn after panels so nothing clips it) -------------
    svg.leaf(
        "rect",
        &[
            ("x", "0".into()),
            ("y", "0".into()),
            ("width", layout.svg_w.to_string()),
            ("height", layout.title_h.to_string()),
            ("fill", theme.name_panel.into()),
        ],
    );
    svg.leaf(
        "rect",
        &[
            ("x", "0".into()),
            ("y", "0".into()),
            ("width", "6".into()),
            ("height", layout.title_h.to_string()),
            ("fill", theme.accent.into()),
        ],
    );
    let name = opts.input.rsplit('/').next().unwrap_or(&opts.input);
    svg.text(18.0, layout.title_h as f64 * 0.42, name, 17.0, theme.title_text, "start");
    let sub = format!(
        "{} signals · {} .. {} · timescale {}{}",
        n_signals,
        crate::layout::format_secs(layout.window.start as f64 * layout.tick_secs, layout.tick_secs),
        crate::layout::format_secs(layout.window.end as f64 * layout.tick_secs, layout.tick_secs),
        vcd.header.timescale.0,
        vcd.header.timescale.1.symbol()
    );
    svg.text(18.0, layout.title_h as f64 * 0.74, &sub, 11.5, theme.text_muted, "start");

    // ---- heat strip (drawn after the name panel so its caption shows) -----
    crate::heat::draw(
        &mut svg,
        &crate::heat::Ctx {
            layout: &layout,
            theme,
            opts,
            n_rows: rows.len(),
        },
        vcd,
        &rows,
    );

    // ---- cursors (feature hook, topmost) ---------------------------------
    crate::cursors::draw(
        &mut svg,
        &crate::cursors::Ctx {
            layout: &layout,
            theme,
            opts,
        },
    );

    let defs = format!(
        "{}{}{}",
        bus::svg_defs(theme),
        crate::heat::svg_defs(theme),
        crate::cursors::svg_defs(theme)
    );
    svg.finish(layout.svg_w, layout.svg_h, &title, &defs)
}

fn draw_grid(svg: &mut Svg, ctx: &Ctx) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    let y_top = layout.rows_y0;
    let y_bot = layout.rows_y0 + ctx.n_rows as f64 * layout.row_h as f64;
    svg.open(
        "g",
        &[("stroke-width", "1".into()), ("shape-rendering", "crispEdges".into())],
    );
    // minor ticks: 4 subdivisions between majors
    for w in layout.ticks.windows(2) {
        let dt = (w[1].time as i128 - w[0].time as i128) / 5;
        if dt <= 0 {
            continue;
        }
        for k in 1..5i128 {
            let m = w[0].time as i128 + dt * k;
            if m < layout.window.start as i128 || m > layout.window.end as i128 {
                continue;
            }
            let x = layout.time_to_x(m as u64);
            svg.leaf(
                "line",
                &[
                    ("x1", crate::svg::fmt(x)),
                    ("y1", crate::svg::fmt(y_top)),
                    ("x2", crate::svg::fmt(x)),
                    ("y2", crate::svg::fmt(y_bot)),
                    ("stroke", theme.grid_minor.into()),
                ],
            );
        }
    }
    for t in &layout.ticks {
        let x = layout.time_to_x(t.time);
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(x)),
                ("y1", crate::svg::fmt(y_top)),
                ("x2", crate::svg::fmt(x)),
                ("y2", crate::svg::fmt(y_bot)),
                ("stroke", theme.grid_major.into()),
            ],
        );
    }
    svg.close("g");
}

fn draw_axis(svg: &mut Svg, ctx: &Ctx, y: f64) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    for t in &layout.ticks {
        let x = layout.time_to_x(t.time);
        if x < layout.wave_x0 - 1.0 || x > layout.wave_x1 + 1.0 {
            continue;
        }
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(x)),
                ("y1", crate::svg::fmt(y)),
                ("x2", crate::svg::fmt(x)),
                ("y2", crate::svg::fmt(y + 8.0)),
                ("stroke", theme.text_faint.into()),
                ("stroke-width", "1".into()),
            ],
        );
        // keep right-edge labels inside the canvas
        if x + 44.0 > layout.wave_x1 {
            svg.text(x - 4.0, y + 14.0, &t.label, 10.5, theme.text_muted, "end");
        } else {
            svg.text(x + 4.0, y + 14.0, &t.label, 10.5, theme.text_muted, "start");
        }
    }
}

fn draw_row(svg: &mut Svg, ctx: &Ctx, row: &Row, ri: usize, y: f64, vcd: &Vcd) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    match row {
        Row::Group { .. } => {
            // group header: subtle tint across the wave area
            svg.leaf(
                "rect",
                &[
                    ("x", (layout.name_w + 12).to_string()),
                    ("y", crate::svg::fmt(y)),
                    (
                        "width",
                        crate::svg::fmt(layout.wave_x1 - layout.name_w as f64 - 12.0),
                    ),
                    ("height", layout.row_h.to_string()),
                    ("fill", theme.row_alt.into()),
                    ("fill-opacity", "0.6".into()),
                ],
            );
        }
        Row::Signal { signal, .. } => {
            if ri % 2 == 1 {
                svg.leaf(
                    "rect",
                    &[
                        ("x", (layout.name_w + 12).to_string()),
                        ("y", crate::svg::fmt(y)),
                        (
                            "width",
                            crate::svg::fmt(layout.wave_x1 - layout.name_w as f64 - 12.0),
                        ),
                        ("height", layout.row_h.to_string()),
                        ("fill", theme.row_alt.into()),
                    ],
                );
            }
            let series = build_series(vcd, *signal);
            let color = trace_color(theme, &row.label());
            draw_wave(svg, ctx, &series, y, color);
        }
    }
}

/// Stable FNV-1a hash -> palette cycle.
pub fn trace_color(theme: &Theme, label: &str) -> &'static str {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in label.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    theme.trace_colors[(h as usize) % theme.trace_colors.len()]
}

/// Per-pixel-column projection of an event list: O(pixels) rendering.
struct Columns {
    first_kind: Vec<Option<u8>>,
    last_kind: Vec<Option<u8>>,
    first_idx: Vec<Option<usize>>,
    last_idx: Vec<Option<usize>>,
    count: Vec<u32>,
}

fn project_columns(layout: &Layout, series: &Series, ncols: usize) -> Columns {
    let mut c = Columns {
        first_kind: vec![None; ncols],
        last_kind: vec![None; ncols],
        first_idx: vec![None; ncols],
        last_idx: vec![None; ncols],
        count: vec![0; ncols],
    };
    let s = layout.window.start;
    let e = layout.window.end;
    let pos = series
        .events
        .iter()
        .position(|ev| ev.time >= s)
        .unwrap_or(series.events.len());
    let endpos = series.events[pos..]
        .iter()
        .position(|ev| ev.time > e)
        .map(|p| pos + p)
        .unwrap_or(series.events.len());
    let col_of = |t: u64| -> usize {
        let x = layout.time_to_x(t);
        ((x - layout.wave_x0).max(0.0) as usize).min(ncols - 1)
    };
    for i in pos..endpos {
        let ev = &series.events[i];
        let col = col_of(ev.time);
        if c.first_kind[col].is_none() {
            c.first_kind[col] = Some(ev.kind);
            c.first_idx[col] = Some(i);
        }
        c.last_kind[col] = Some(ev.kind);
        c.last_idx[col] = Some(i);
        c.count[col] += 1;
    }
    c
}

fn draw_wave(svg: &mut Svg, ctx: &Ctx, series: &Series, y: f64, color: &str) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    if series.events.is_empty() {
        return;
    }
    if series.is_bus {
        draw_bus(svg, ctx, series, y, color);
        return;
    }

    let cy = y + layout.row_h as f64 / 2.0;
    let hi = cy - layout.row_h as f64 * 0.30;
    let lo = cy + layout.row_h as f64 * 0.30;

    // Level in effect at the window start.
    let initial = {
        let pos = series
            .events
            .iter()
            .position(|ev| ev.time >= layout.window.start)
            .unwrap_or(series.events.len());
        if pos > 0 {
            series.events[pos - 1].kind
        } else {
            series.events[0].kind
        }
    };

    let ncols = (layout.wave_x1 - layout.wave_x0).ceil() as usize + 1;
    let cols = project_columns(layout, series, ncols);

    let flush = |svg: &mut Svg, run_start: f64, run_end: f64, level: u8| {
        if run_end <= run_start {
            return;
        }
        let st = bus::style_for(level);
        let level_y = cy + (st.level_frac - 0.5) * (lo - hi);
        let stroke = match level {
            b'x' => theme.x_color,
            b'z' => theme.z_color,
            _ => color,
        };
        let mut attrs = vec![
            ("x1", crate::svg::fmt(run_start)),
            ("y1", crate::svg::fmt(level_y)),
            ("x2", crate::svg::fmt(run_end)),
            ("y2", crate::svg::fmt(level_y)),
            ("stroke", stroke.into()),
            ("stroke-width", "1.6".into()),
        ];
        if let Some(d) = st.dash {
            attrs.push(("stroke-dasharray", d.into()));
        }
        svg.leaf("line", &attrs);
    };

    svg.open("g", &[("stroke-linecap", "square".into())]);
    let mut level = initial;
    let mut run_start = layout.wave_x0;
    let mut dense: Vec<(f64, f64)> = Vec::new();
    for c in 0..ncols {
        let x0 = layout.wave_x0 + c as f64;
        let x1 = x0 + 1.0;
        match (cols.first_kind[c], cols.last_kind[c]) {
            (None, _) => { /* empty column: run continues */ }
            (Some(f), Some(l)) => {
                let cnt = cols.count[c];
                if cnt >= 3 {
                    // busy column: shade it, suppress edges
                    flush(svg, run_start, x0, level);
                    match dense.last_mut() {
                        Some(last) if (last.1 - x0).abs() < 1.5 => last.1 = x1,
                        _ => dense.push((x0, x1)),
                    }
                    level = l;
                    run_start = x1;
                } else {
                    if f != level {
                        flush(svg, run_start, x0, level);
                        edge(svg, x0, level, f, hi, lo, color, theme);
                        level = f;
                        run_start = x0;
                    }
                    if l != level {
                        flush(svg, run_start, x1, level);
                        edge(svg, x1, level, l, hi, lo, color, theme);
                        level = l;
                        run_start = x1;
                    }
                }
            }
            _ => {}
        }
    }
    flush(svg, run_start, layout.wave_x1, level);
    svg.close("g");

    if !dense.is_empty() {
        svg.open("g", &[("fill", color.into()), ("fill-opacity", "0.28".into())]);
        for (x0, x1) in dense {
            svg.leaf(
                "rect",
                &[
                    ("x", crate::svg::fmt(x0)),
                    ("y", crate::svg::fmt(hi)),
                    ("width", crate::svg::fmt(x1 - x0)),
                    ("height", crate::svg::fmt(lo - hi)),
                ],
            );
        }
        svg.close("g");
    }
}

fn edge(svg: &mut Svg, x: f64, from: u8, to: u8, hi: f64, lo: f64, color: &str, theme: &Theme) {
    let sf = bus::style_for(from);
    let st = bus::style_for(to);
    let ly = cy_of(hi, lo, sf.level_frac);
    let ny = cy_of(hi, lo, st.level_frac);
    if (ly - ny).abs() < 0.05 {
        return;
    }
    // endpoints in the x/z tone keep undriven edges readable
    let stroke = match to {
        b'z' => theme.z_color,
        b'x' => theme.x_color,
        _ => color,
    };
    svg.leaf(
        "line",
        &[
            ("x1", crate::svg::fmt(x)),
            ("y1", crate::svg::fmt(ly)),
            ("x2", crate::svg::fmt(x)),
            ("y2", crate::svg::fmt(ny)),
            ("stroke", stroke.into()),
            ("stroke-width", "1.6".into()),
        ],
    );
}

fn cy_of(hi: f64, lo: f64, frac: f64) -> f64 {
    hi + (lo - hi) * frac
}

fn draw_bus(svg: &mut Svg, ctx: &Ctx, series: &Series, y: f64, color: &str) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    if series.events.is_empty() {
        return;
    }
    let cy = y + layout.row_h as f64 / 2.0;
    let top = cy - layout.row_h as f64 * 0.34;
    let bot = cy + layout.row_h as f64 * 0.34;
    let h = bot - top;

    // Event index in effect at the window start.
    let pos0 = series
        .events
        .iter()
        .position(|e| e.time >= layout.window.start)
        .unwrap_or(series.events.len());
    let init_idx: Option<usize> = if pos0 == 0 { None } else { Some(pos0 - 1) };

    let ncols = (layout.wave_x1 - layout.wave_x0).ceil() as usize + 1;
    let cols = project_columns(layout, series, ncols);

    let value_at = |idx: Option<usize>| -> Option<u8> {
        idx.map(|i| series.events[i].kind)
    };
    // Two event indices carry the same value iff kind matches AND the raw
    // payload matches (kinds alone cannot distinguish different vectors).
    let same_value = |a: Option<usize>, b: Option<usize>| -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(i), Some(j)) => {
                if series.events[i].kind != series.events[j].kind {
                    return false;
                }
                match (series.values.get(i), series.values.get(j)) {
                    (Some(x), Some(y)) if !x.is_empty() && !y.is_empty() => x == y,
                    _ => true,
                }
            }
            _ => false,
        }
    };

    let mut drawn: Vec<(f64, f64, Option<usize>, u8)> = Vec::new();
    let mut draw_band = |svg: &mut Svg, x0: f64, x1: f64, idx: Option<usize>, kind: u8| {
        if x1 <= x0 {
            return;
        }
        drawn.push((x0, x1, idx, kind));
        let style = bus::style_for(kind);
        let fill = style.fill_override.unwrap_or(theme.bus_fill);
        // x/z bands collapse toward their level line
        let bh = band_h(kind, h, style.level_frac);
        let band_top = top + band_top_offset(kind, h, style.level_frac);
        svg.leaf(
            "rect",
            &[
                ("x", crate::svg::fmt(x0)),
                ("y", crate::svg::fmt(band_top)),
                ("width", crate::svg::fmt(x1 - x0)),
                ("height", crate::svg::fmt(bh)),
                ("fill", fill.into()),
                ("fill-opacity", crate::svg::fmt(style.opacity)),
                ("rx", "2.5".into()),
                ("stroke", color.into()),
                ("stroke-width", "1.2".into()),
            ],
        );
        if let Some(dash) = style.dash {
            svg.leaf(
                "rect",
                &[
                    ("x", crate::svg::fmt(x0)),
                    ("y", crate::svg::fmt(top)),
                    ("width", crate::svg::fmt(x1 - x0)),
                    ("height", crate::svg::fmt(h)),
                    ("fill", "none".into()),
                    (
                        "stroke",
                        if kind == b'z' { theme.z_color } else { theme.x_color }.into(),
                    ),
                    ("stroke-width", "1".into()),
                    ("stroke-dasharray", dash.into()),
                    ("rx", "2.5".into()),
                ],
            );
        }
    };
    let draw_xmark = |svg: &mut Svg, x: f64| {
        if x < layout.wave_x0 || x > layout.wave_x1 {
            return;
        }
        svg.open(
            "g",
            &[
                ("stroke", color.into()),
                ("stroke-width", "1.2".into()),
                ("stroke-linecap", "round".into()),
            ],
        );
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(x - 3.0)),
                ("y1", crate::svg::fmt(top - 2.0)),
                ("x2", crate::svg::fmt(x + 3.0)),
                ("y2", crate::svg::fmt(bot + 2.0)),
            ],
        );
        svg.leaf(
            "line",
            &[
                ("x1", crate::svg::fmt(x - 3.0)),
                ("y1", crate::svg::fmt(bot + 2.0)),
                ("x2", crate::svg::fmt(x + 3.0)),
                ("y2", crate::svg::fmt(top - 2.0)),
            ],
        );
        svg.close("g");
    };

    // Walk pixel columns, merging runs of constant value.
    let mut cur_idx = init_idx;
    let mut band_start = layout.wave_x0;
    for c in 0..ncols {
        let x0 = layout.wave_x0 + c as f64;
        let x1 = x0 + 1.0;
        match (cols.first_idx[c], cols.last_idx[c]) {
            (None, _) => { /* band continues */ }
            (Some(f), Some(l)) => {
                let vf = value_at(Some(f));
                let vcur = value_at(cur_idx);
                let cnt = cols.count[c];
                if cnt >= 3 {
                    // dense: close band, shade, continue with last value
                    draw_band(svg, band_start, x0, cur_idx, vcur.unwrap_or(b'x'));
                    svg.open("g", &[("fill", color.into()), ("fill-opacity", "0.35".into())]);
                    svg.leaf(
                        "rect",
                        &[
                            ("x", crate::svg::fmt(x0)),
                            ("y", crate::svg::fmt(top)),
                            ("width", "1".into()),
                            ("height", crate::svg::fmt(h)),
                        ],
                    );
                    svg.close("g");
                    cur_idx = Some(l);
                    band_start = x1;
                } else {
                    if !same_value(cur_idx, Some(f)) {
                        draw_band(svg, band_start, x0, cur_idx, vcur.unwrap_or(b'x'));
                        draw_xmark(svg, x0);
                        cur_idx = Some(f);
                        band_start = x0;
                    }
                    if !same_value(Some(f), Some(l)) {
                        draw_band(svg, band_start, x1, cur_idx, vf.unwrap_or(b'x'));
                        draw_xmark(svg, x1);
                        cur_idx = Some(l);
                        band_start = x1;
                    }
                }
            }
            _ => {}
        }
    }
    draw_band(
        svg,
        band_start,
        layout.wave_x1,
        cur_idx,
        value_at(cur_idx).unwrap_or(b'x'),
    );

    bus::annotate(
        svg,
        &bus::Annotate {
            segments: &drawn,
            series,
            theme,
            opts: ctx.opts,
            cy,
            wave_x0: layout.wave_x0,
            wave_x1: layout.wave_x1,
        },
    );
}

fn draw_name(svg: &mut Svg, ctx: &Ctx, row: &Row, y: f64) {
    let layout = ctx.layout;
    let theme = ctx.theme;
    let cy = y + layout.row_h as f64 / 2.0;
    match row {
        Row::Group { path, depth } => {
            let x = 12.0 + *depth as f64 * 14.0;
            svg.text(x, cy, &path.join("."), 11.5, theme.group_text, "start");
        }
        Row::Signal { signal: _, path, depth } => {
            let x = 12.0 + *depth as f64 * 14.0;
            let leaf = path.last().cloned().unwrap_or_default();
            svg.text(x, cy, &leaf, 11.0, theme.text_primary, "start");
            let parents = path[..path.len().saturating_sub(1)].join(".");
            if !parents.is_empty() {
                let off = leaf.chars().count() as f64 * 6.6 + 8.0;
                svg.text(x + off, cy, &format!("·{}", parents), 9.0, theme.text_faint, "start");
            }
        }
    }
}

/// Band height for a kind: normal bands are full height; x/z collapse to a
/// thin band around their level line.
fn band_h(kind: u8, h: f64, _level_frac: f64) -> f64 {
    // x/z bands collapse toward their level line; value bands stay full height
    if matches!(kind, b'x' | b'z') {
        h * 0.35
    } else {
        h
    }
}

/// Vertical offset of a band's top edge from `top`, keeping x/z bands
/// centered on their level line and full bands anchored at the top.
fn band_top_offset(kind: u8, h: f64, level_frac: f64) -> f64 {
    if matches!(kind, b'x' | b'z') {
        (level_frac * h - band_h(kind, h, level_frac) / 2.0)
            .max(0.0)
            .min(h - band_h(kind, h, level_frac))
    } else {
        0.0
    }
}
