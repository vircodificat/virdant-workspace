//! Feature: labeled time cursors (--mark t1,t2,...).

use crate::svg::Svg;
use crate::theme::Theme;

pub struct Ctx<'a> {
    pub layout: &'a crate::layout::Layout,
    pub theme: &'a Theme,
    pub opts: &'a crate::cli::Opts,
}

pub fn svg_defs(_theme: &Theme) -> String {
    String::new()
}

pub fn draw(_svg: &mut Svg, _ctx: &Ctx) {
    // Implemented by the cursors feature lane.
}
