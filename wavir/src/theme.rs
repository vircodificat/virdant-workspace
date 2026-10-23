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

/// Paper white: light surfaces, darkened high-contrast traces.
pub const LIGHT: Theme = Theme {
    name: "light",
    bg: "#FAFAFA",
    row_alt: "#EFEFEF",
    name_panel: "#F0F0F0",
    grid_major: "#E2E2E2",
    grid_minor: "#ECECEC",
    text_primary: "#1A1A1A",
    text_muted: "#666666",
    text_faint: "#AAAAAA",
    group_text: "#2E7D32",
    accent: "#7AAD18",
    title_text: "#1A1A1A",
    trace_colors: &[
        "#2E7D32", // link green, darkened
        "#1565C0", // kind-mod blue, darkened
        "#E65100", // kind-enum orange, darkened
        "#AD1457", // kind-union pink, darkened
        "#6A1B9A", // kind-fn violet, darkened
        "#00695C", // teal
        "#C62828", // kind-socket red, darkened
        "#5D4037", // brown
    ],
    x_color: "#B26A00",
    z_color: "#0B5CAD",
    bus_fill: "#E8ECEF",
    bus_text: "#333333",
    heat_ramp: &["#ECECEC", "#CDE3A2", "#A8CB6B", "#7AAD18", "#5C8A10", "#3F6208"],
    cursor_color: "#CC5500",
    font_family: "ui-monospace, 'SF Mono', 'Cascadia Code', 'JetBrains Mono', Menlo, Consolas, monospace",
};

/// Phosphor CRT: black background, green/amber/cyan traces.
pub const TERMINAL: Theme = Theme {
    name: "terminal",
    bg: "#050505",
    row_alt: "#071008",
    name_panel: "#08120A",
    grid_major: "#0F2A12",
    grid_minor: "#08180B",
    text_primary: "#33FF33",
    text_muted: "#22B322",
    text_faint: "#155515",
    group_text: "#88FF44",
    accent: "#00FF66",
    title_text: "#33FF33",
    trace_colors: &[
        "#00FF66", // green phosphor
        "#FFB000", // amber phosphor
        "#33DDFF", // cyan phosphor
        "#88FF44", // lime phosphor
        "#FF6633", // ember phosphor
    ],
    x_color: "#FFB000",
    z_color: "#33DDFF",
    bus_fill: "#0A1F0E",
    bus_text: "#7CFF9A",
    heat_ramp: &["#08180B", "#0E3A14", "#1E6B22", "#2FBF35", "#00FF66", "#88FF44"],
    cursor_color: "#FF3355",
    font_family: "ui-monospace, 'SF Mono', 'Cascadia Code', 'JetBrains Mono', Menlo, Consolas, monospace",
};

/// Resolve a theme by name. Unknown names fall back to dark.
pub fn by_name(name: &str) -> &'static Theme {
    match name {
        "dark" => &DARK,
        "light" => &LIGHT,
        "terminal" => &TERMINAL,
        _ => &DARK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_name_resolves_all_three_presets() {
        assert_eq!(by_name("dark").name, "dark");
        assert_eq!(by_name("light").name, "light");
        assert_eq!(by_name("terminal").name, "terminal");
    }

    #[test]
    fn by_name_unknown_falls_back_to_dark() {
        // compare through by_name so const-inlining cannot fake a mismatch
        assert!(std::ptr::eq(by_name("nonsense"), by_name("dark")));
        assert!(std::ptr::eq(by_name(""), by_name("dark")));
        assert_eq!(by_name("nonsense").name, "dark");
    }

    #[test]
    fn presets_have_all_palette_slots() {
        for t in [&DARK, &LIGHT, &TERMINAL] {
            assert!(!t.trace_colors.is_empty());
            assert!(t.heat_ramp.len() >= 2);
            // every palette slot is a well-formed 6-digit hex color
            let slots = [
                t.bg,
                t.row_alt,
                t.name_panel,
                t.grid_major,
                t.grid_minor,
                t.text_primary,
                t.text_muted,
                t.text_faint,
                t.group_text,
                t.accent,
                t.title_text,
                t.x_color,
                t.z_color,
                t.bus_fill,
                t.bus_text,
                t.cursor_color,
            ];
            for s in slots.iter().copied().chain(t.trace_colors.iter().copied()).chain(t.heat_ramp.iter().copied()) {
                assert_eq!(s.len(), 7, "bad color {:?} in theme {}", s, t.name);
                assert!(s.starts_with('#'), "bad color {:?} in theme {}", s, t.name);
                assert!(s[1..].chars().all(|c| c.is_ascii_hexdigit()), "bad color {:?} in theme {}", s, t.name);
            }
        }
    }

    #[test]
    fn dark_preset_unchanged() {
        assert_eq!(DARK.name, "dark");
        assert_eq!(DARK.bg, "#0D1117");
        assert_eq!(DARK.accent, "#8FC225");
        assert_eq!(DARK.heat_ramp.last().copied(), Some("#B4E44E"));
    }
}
