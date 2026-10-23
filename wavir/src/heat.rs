//! Feature: transition-density heat strip.
//!
//! Drawn above the waveform rows: one pixel column of color per time step
//! position, showing where the design is busiest. Stub renders the frame
//! only; the feature lane implements the density mapping.

use crate::svg::Svg;
use crate::theme::Theme;
use crate::types::Row;
use crate::vcd::Vcd;

pub struct Ctx<'a> {
    pub layout: &'a crate::layout::Layout,
    pub theme: &'a Theme,
    pub opts: &'a crate::cli::Opts,
    pub n_rows: usize,
}

pub fn svg_defs(_theme: &Theme) -> String {
    String::new()
}

pub fn draw(_svg: &mut Svg, _ctx: &Ctx, _vcd: &Vcd, _rows: &[Row]) {
    // Implemented by the heat-strip feature lane.
}
