//! Feature: interactive HTML viewer export (--html).
//!
//! Stub: wraps the static SVG in a minimal HTML page. The feature lane
//! replaces this with a full interactive viewer (pan/zoom, hover tooltips,
//! signal value readout at the cursor).

pub fn export(vcd: &crate::vcd::Vcd, opts: &crate::cli::Opts, theme: &crate::theme::Theme) -> Result<String, String> {
    let svg = crate::render::render(vcd, opts, theme);
    Ok(format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>wavir — {}</title>\n<style>body{{margin:0;background:{};display:flex;justify-content:center;}}svg{{max-width:100%;height:auto;}}</style>\n</head><body>\n{}\n</body></html>\n",
        crate::svg::escape(&opts.input),
        theme.bg,
        svg
    ))
}
