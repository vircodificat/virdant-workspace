//! Color themes, derived from the Virdant palette.
//!
//! Brand anchors (from the Virdant docs CSS):
//!   brand green   #8FC225   hover #7AAD18   light #E8F5C8   dark-accent #9FD42E
//!   dark surfaces #0D1117 / #1c1c1c / #282828
//!   kind-dot accent colors: #4A9EFF #28A745 #D63384 #FD7E14 #6610F2 #DC3545
//!   link greens   #2E7D32 / #7AB87A
//!   burnt orange  #CC5500

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    /// Canvas background.
    pub bg: &'static str,
    /// Subtle row striping.
    pub row_alt: &'static str,
    /// Panel behind signal names.
    pub name_panel: &'static str,
    pub grid_major: &'static str,
    pub grid_minor: &'static str,
    pub text_primary: &'static str,
    pub text_muted: &'static str,
    pub text_faint: &'static str,
    pub group_text: &'static str,
    /// Virdant brand green.
    pub accent: &'static str,
    pub title_text: &'static str,
    /// Trace stroke colors, cycled across signals.
    pub trace_colors: &'static [&'static str],
    pub x_color: &'static str,
    pub z_color: &'static str,
    pub bus_fill: &'static str,
    pub bus_text: &'static str,
    /// Heat strip ramp (low -> high density).
    pub heat_ramp: &'static [&'static str],
    pub cursor_color: &'static str,
    pub font_family: &'static str,
}

pub const DARK: Theme = Theme {
    name: "dark",
    bg: "#0D1117",
    row_alt: "#141B24",
    name_panel: "#11161D",
    grid_major: "#2A3441",
    grid_minor: "#1B2330",
    text_primary: "#E6EDF3",
    text_muted: "#8B949E",
    text_faint: "#4C5866",
    group_text: "#9FD42E",
    accent: "#8FC225",
    title_text: "#E6EDF3",
    trace_colors: &[
        "#8FC225", // virdant green
        "#4A9EFF", // kind-mod blue
        "#FD7E14", // kind-enum orange
        "#D63384", // kind-union pink
        "#6610F2", // kind-fn violet
        "#CC5500", // burnt orange
        "#28A745", // kind-struct green
        "#7AB87A", // link green
        "#DC3545", // kind-socket red
        "#E8F5C8", // brand light
    ],
    x_color: "#E3B341",
    z_color: "#58A6FF",
    bus_fill: "#1A2332",
    bus_text: "#C9D1D9",
    heat_ramp: &["#1B2330", "#2E5B1E", "#57861C", "#7AAD18", "#8FC225", "#B4E44E"],
    cursor_color: "#CC5500",
    font_family: "ui-monospace, 'SF Mono', 'Cascadia Code', 'JetBrains Mono', Menlo, Consolas, monospace",
};

/// Resolve a theme by name. Unknown names fall back to dark.
pub fn by_name(name: &str) -> &'static Theme {
    match name {
        "dark" => &DARK,
        _ => &DARK,
    }
}
