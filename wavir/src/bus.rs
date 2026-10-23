//! Bus helpers: X/Z segment styling and bus value formatting/placement.
//!
//! Display contract for `format_bits`:
//!   - width 1            -> the single bit ("0"/"1"/"x"/"z")
//!   - width <= 4         -> binary; x/z digits are shown in place
//!   - width > 4          -> uppercase hex, zero-padded to ceil(width/4)
//!                           digits, no "0x" prefix (8-bit 0x0A -> "0A")
//!   - x/z in wide values -> collapse to a single "x" (any x dominates)
//!                           or "z" token
//! The display width is the declared signal width taken from `range`
//! (e.g. [7:0] -> 8) when the dumped value string is shorter: per VCD
//! semantics short strings are left-extended (x stays x, z stays z,
//! otherwise zero-extended), so an 8-bit `b1010` renders as "0A".
//! Hex is assembled from 4-bit chunks, so arbitrarily wide buses (256+ bits)
//! never accumulate into an integer.
//!
//! X/Z bus segments are filled with diagonal-hatch SVG patterns defined by
//! `svg_defs` ("wavir-x-hatch" amber, "wavir-z-hatch" blue) and keep dashed
//! borders (the `dash` field drives the border dash in `render.rs`).

use crate::svg::Svg;
use crate::types::Series;

/// How to draw one waveform segment.
#[derive(Debug, Clone, Copy)]
pub struct SegStyle {
    /// Fill override: color hex or `url(#pattern-id)`.
    pub fill_override: Option<&'static str>,
    /// Stroke dash pattern override.
    pub dash: Option<&'static str>,
    /// Opacity for the fill (1.0 = opaque).
    pub opacity: f64,
    /// Vertical position of the segment line within the row, as a fraction
    /// from `hi` (0.0) to `lo` (1.0). 0.5 = midline (used for z / x).
    pub level_frac: f64,
}

impl SegStyle {
    pub fn normal() -> SegStyle {
        SegStyle {
            fill_override: None,
            dash: None,
            opacity: 1.0,
            level_frac: 0.0,
        }
    }
}

/// Level fractions for known kinds.
pub const FRAC_HI: f64 = 0.0;
pub const FRAC_LO: f64 = 1.0;
pub const FRAC_MID: f64 = 0.5;

/// Defs ids of the X/Z hatch patterns (see `svg_defs`).
pub const X_PATTERN_ID: &str = "wavir-x-hatch";
pub const Z_PATTERN_ID: &str = "wavir-z-hatch";

/// Styling for a segment starting with the given event kind.
///
/// Used for both scalar level runs and bus value bands: the renderer reads
/// `level_frac`/`dash` for scalars and additionally `fill_override`/
/// `opacity` for bus bands. X/Z set a hatch-pattern fill (`svg_defs`) and
/// keep dashed borders; scalar x/z are dashed midlines in the x/z tone.
pub fn style_for(kind: u8) -> SegStyle {
    match kind {
        b'1' => SegStyle { level_frac: FRAC_HI, ..SegStyle::normal() },
        b'0' => SegStyle { level_frac: FRAC_LO, ..SegStyle::normal() },
        b'x' => SegStyle {
            fill_override: Some("url(#wavir-x-hatch)"),
            dash: Some("5,3"),
            opacity: 1.0,
            level_frac: FRAC_MID,
        },
        b'z' => SegStyle {
            fill_override: Some("url(#wavir-z-hatch)"),
            dash: Some("2,3"),
            opacity: 1.0,
            level_frac: FRAC_MID,
        },
        _ => SegStyle::normal(),
    }
}

/// True if the bit vector contains any unknown (x/X) digit.
pub fn has_x(bits: &[u8]) -> bool {
    bits.iter().any(|&b| b == b'x' || b == b'X')
}

/// True if the bit vector contains any high-impedance (z/Z) digit.
pub fn has_z(bits: &[u8]) -> bool {
    bits.iter().any(|&b| b == b'z' || b == b'Z')
}

/// Hex digit value of a 4-bit (or fewer) chunk of ASCII bit chars.
fn nibble(bits: &[u8]) -> char {
    let mut v: u32 = 0;
    for &b in bits {
        v = (v << 1) | (b == b'1') as u32;
    }
    char::from_digit(v, 16).unwrap_or('0').to_ascii_uppercase()
}

/// Render a vector value as a label (see the display contract in the
/// module docs). `range` carries the declared bit range, e.g. [7:0].
pub fn format_bits(bits: &[u8], range: Option<(i32, i32)>) -> String {
    if bits.is_empty() {
        return String::new();
    }
    // Declared width from the bit range, when the dumped value string is
    // shorter (VCD permits "b1010" for an 8-bit vector).
    let declared = range.map(|(a, b)| a.abs_diff(b) as usize + 1).unwrap_or(0);
    let width = declared.max(bits.len());
    if width == bits.len() {
        return format_bits_exact(bits);
    }
    // Left-extend per VCD semantics: x stays x, z stays z, else zero.
    let pad = match bits[0] {
        b'x' | b'X' => b'x',
        b'z' | b'Z' => b'z',
        _ => b'0',
    };
    let mut padded = vec![pad; width - bits.len()];
    padded.extend_from_slice(bits);
    format_bits_exact(&padded)
}

fn format_bits_exact(bits: &[u8]) -> String {
    if bits.is_empty() {
        return String::new();
    }
    if bits.len() == 1 {
        return String::from_utf8_lossy(&[bits[0].to_ascii_lowercase()]).into_owned();
    }
    let width = bits.len();
    if has_x(bits) || has_z(bits) {
        // Short vectors show x/z digits in place; wider values collapse.
        if width <= 4 {
            return String::from_utf8_lossy(bits).to_ascii_lowercase();
        }
        return if has_x(bits) { "x".into() } else { "z".into() };
    }
    if width <= 4 {
        return String::from_utf8_lossy(bits).into_owned();
    }
    // width > 4: uppercase hex, zero-padded to ceil(width/4) digits.
    let digits = (width + 3) / 4;
    let lead = width % 4;
    let mut hex = String::with_capacity(digits);
    if lead != 0 {
        hex.push(nibble(&bits[..lead]));
    }
    for chunk in bits[lead..].chunks(4) {
        hex.push(nibble(chunk));
    }
    while hex.len() < digits {
        hex.insert(0, '0');
    }
    hex
}

/// Extra SVG <defs> content: diagonal-hatch patterns for X/Z bus fills.
///
/// Each pattern tiles a faint tone wash plus a stronger stripe, rotated 45
/// degrees, in the theme's x/z color. Sizes are in user-space units so the
/// hatch scale stays constant regardless of the viewBox.
pub fn svg_defs(theme: &crate::theme::Theme) -> String {
    let mut out = String::with_capacity(512);
    for (id, color) in [(X_PATTERN_ID, theme.x_color), (Z_PATTERN_ID, theme.z_color)] {
        out.push_str(&format!(
            "<pattern id=\"{}\" patternUnits=\"userSpaceOnUse\" width=\"7\" height=\"7\" patternTransform=\"rotate(45)\">\
<rect width=\"7\" height=\"7\" fill=\"{}\" fill-opacity=\"0.10\"/>\
<line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"7\" stroke=\"{}\" stroke-width=\"2.2\" stroke-opacity=\"0.38\"/>\
</pattern>\n",
            id, color, color
        ));
    }
    out
}

/// Context for drawing bus value labels over already-drawn segments.
pub struct Annotate<'a> {
    /// (x0, x1, event index into Series::events, kind) per visible segment.
    pub segments: &'a [(f64, f64, Option<usize>, u8)],
    pub series: &'a Series,
    pub theme: &'a crate::theme::Theme,
    pub opts: &'a crate::cli::Opts,
    /// Vertical center of the row.
    pub cy: f64,
    pub wave_x0: f64,
    pub wave_x1: f64,
}

/// Approximate advance width of one label character at the 10.5px monospace
/// label font (same estimate as the name column).
const CHAR_W: f64 = 6.6;
/// X-transition markers extend ~3px inward from a band edge; keep this
/// gap between label text and any marked edge.
const MARKER_MARGIN: f64 = 4.0;

/// Draw bus value labels.
///
/// A label is drawn centered in its band only if the text fits between the
/// band edges minus the marker margin on every edge that carries an
/// X-transition marker (edges coinciding with the window boundary are not
/// marked, so text may use the full band there). Labels are never
/// truncated: bands narrower than the text are left unlabeled.
pub fn annotate(svg: &mut Svg, a: &Annotate) {
    if !a.opts.bus_labels {
        return;
    }
    for (x0, x1, ev_idx, kind) in a.segments {
        let label = match *kind {
            b'0' => Some("0".to_string()),
            b'1' => Some("1".to_string()),
            b'x' | b'z' | b'v' => ev_idx
                .and_then(|i| a.series.values.get(i))
                .filter(|v| !v.is_empty())
                .map(|bits| format_bits(bits, a.series.range))
                .or_else(|| {
                    if *kind == b'x' || *kind == b'z' {
                        Some(if *kind == b'x' { "x".into() } else { "z".into() })
                    } else {
                        None
                    }
                }),
            _ => None,
        };
        let label = match label {
            Some(l) if !l.is_empty() => l,
            _ => continue,
        };
        let cx = (x0 + x1) / 2.0;
        if cx <= a.wave_x0 || cx >= a.wave_x1 {
            continue;
        }
        let text_w = label.chars().count() as f64 * CHAR_W;
        // Edges on the window boundary carry no transition marker.
        let left_need = if *x0 > a.wave_x0 + 1.0 { MARKER_MARGIN } else { 0.0 };
        let right_need = if *x1 < a.wave_x1 - 1.0 { MARKER_MARGIN } else { 0.0 };
        if cx - text_w / 2.0 < x0 + left_need || cx + text_w / 2.0 > x1 - right_need {
            continue;
        }
        svg.text(cx, a.cy, &label, 10.5, a.theme.bus_text, "middle");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Opts;
    use crate::types::Event;

    fn bits(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn single_bit_labels() {
        assert_eq!(format_bits(&bits("1"), None), "1");
        assert_eq!(format_bits(&bits("0"), None), "0");
        assert_eq!(format_bits(&bits("x"), None), "x");
        assert_eq!(format_bits(&bits("z"), None), "z");
        assert_eq!(format_bits(&bits(""), None), "");
    }

    #[test]
    fn short_widths_are_binary() {
        assert_eq!(format_bits(&bits("1010"), None), "1010");
        assert_eq!(format_bits(&bits("01"), None), "01");
        assert_eq!(format_bits(&bits("1111"), None), "1111");
    }

    #[test]
    fn hex_padded_to_ceil_width_over_4() {
        assert_eq!(format_bits(&bits("00001010"), None), "0A"); // 8-bit
        assert_eq!(format_bits(&bits("11111111"), None), "FF");
        assert_eq!(format_bits(&bits("1011111011101111"), None), "BEEF"); // 16-bit
        assert_eq!(format_bits(&bits("0000000011111111"), None), "00FF"); // padded
        assert_eq!(format_bits(&bits("00001"), None), "01"); // 5-bit
        assert_eq!(format_bits(&bits("11111"), None), "1F");
        assert_eq!(
            format_bits(
                &bits("11011110101011011011111011101111"),
                None
            ),
            "DEADBEEF"
        ); // 32-bit
    }

    #[test]
    fn very_wide_bus_no_numeric_overflow() {
        // 256-bit value: top and bottom bits set, everything else zero.
        let mut v = vec![b'0'; 256];
        v[0] = b'1';
        v[255] = b'1';
        let s = format_bits(&v, None);
        assert_eq!(s.len(), 64);
        assert!(s.starts_with("80"));
        assert!(s.ends_with("01"));
        // all-ones 256-bit
        let ones = vec![b'1'; 256];
        assert_eq!(format_bits(&ones, None), "F".repeat(64));
    }

    #[test]
    fn xz_detection_and_collapse() {
        assert!(has_x(&bits("000x0000")));
        assert!(!has_x(&bits("00000000")));
        assert!(has_z(&bits("zzzzzzzz")));
        // wide vectors collapse to a single token; x dominates z
        assert_eq!(format_bits(&bits("xxxxxxxx"), None), "x");
        assert_eq!(format_bits(&bits("zzzzzzzz"), None), "z");
        assert_eq!(format_bits(&bits("1010x101"), None), "x");
        assert_eq!(format_bits(&bits("1010z101"), None), "z");
        assert_eq!(format_bits(&bits("1010xz01"), None), "x");
        // short vectors keep the raw pattern
        assert_eq!(format_bits(&bits("10xz"), None), "10xz");
        assert_eq!(format_bits(&bits("zzzz"), None), "zzzz");
        assert_eq!(format_bits(&bits("xz"), None), "xz");
    }

    #[test]
    fn declared_range_width_extends_short_values() {
        // 8-bit signal dumped as "b1010": left-extended to "00001010"
        assert_eq!(format_bits(&bits("1010"), Some((7, 0))), "0A");
        assert_eq!(format_bits(&bits("1"), Some((7, 0))), "01");
        // x/z extend with their own digit per VCD semantics
        assert_eq!(format_bits(&bits("x"), Some((7, 0))), "x");
        assert_eq!(format_bits(&bits("z"), Some((3, 0))), "zzzz");
        assert_eq!(format_bits(&bits("10xz"), Some((7, 0))), "x"); // wide -> collapse
        // ascending ranges count too
        assert_eq!(format_bits(&bits("1010"), Some((0, 7))), "0A");
        // a range narrower than the dumped string: trust the string
        assert_eq!(format_bits(&bits("1010"), Some((2, 0))), "1010");
    }

    #[test]
    fn hatch_pattern_ids_present_in_defs() {
        let t = crate::theme::by_name("dark");
        let defs = svg_defs(t);
        assert!(defs.contains("id=\"wavir-x-hatch\""), "{}", defs);
        assert!(defs.contains("id=\"wavir-z-hatch\""), "{}", defs);
        assert!(defs.contains("patternUnits=\"userSpaceOnUse\""));
        assert!(defs.contains(t.x_color));
        assert!(defs.contains(t.z_color));
        assert!(defs.contains("patternTransform=\"rotate(45)\""));
    }

    #[test]
    fn style_for_xz_uses_patterns_and_keeps_dash() {
        assert_eq!(style_for(b'x').fill_override, Some("url(#wavir-x-hatch)"));
        assert_eq!(style_for(b'z').fill_override, Some("url(#wavir-z-hatch)"));
        assert!(style_for(b'x').dash.is_some());
        assert!(style_for(b'z').dash.is_some());
        assert_eq!(style_for(b'x').level_frac, FRAC_MID);
        assert_eq!(style_for(b'z').level_frac, FRAC_MID);
        // value segments stay clean
        assert_eq!(style_for(b'v').fill_override, None);
        assert_eq!(style_for(b'v').dash, None);
        assert_eq!(style_for(b'1').level_frac, FRAC_HI);
        assert_eq!(style_for(b'0').level_frac, FRAC_LO);
    }

    fn series_of(values: &[&str]) -> Series {
        Series {
            events: values
                .iter()
                .map(|_| Event { time: 0, kind: b'v' })
                .collect(),
            values: values
                .iter()
                .map(|v| std::rc::Rc::from(v.as_bytes().to_vec()))
                .collect(),
            width: 8,
            is_bus: true,
            range: None,
        }
    }

    fn annotate_out(
        segments: Vec<(f64, f64, Option<usize>, u8)>,
        series: &Series,
        bus_labels: bool,
    ) -> String {
        annotate_in(segments, series, bus_labels, 0.0, 1000.0)
    }

    fn annotate_in(
        segments: Vec<(f64, f64, Option<usize>, u8)>,
        series: &Series,
        bus_labels: bool,
        wave_x0: f64,
        wave_x1: f64,
    ) -> String {
        let mut svg = Svg::new("monospace");
        let opts = Opts { bus_labels, ..Opts::default() };
        annotate(
            &mut svg,
            &Annotate {
                segments: &segments,
                series,
                theme: crate::theme::by_name("dark"),
                opts: &opts,
                cy: 10.0,
                wave_x0,
                wave_x1,
            },
        );
        svg.finish(1000, 20, "t", "")
    }

    /// 32-bit value whose label is "DEADBEEF" (8 chars, text width 52.8).
    fn deadbeef() -> Series {
        series_of(&["11011110101011011011111011101111"])
    }

    #[test]
    fn label_drawn_centered_when_it_fits() {
        let s = series_of(&["00001010"]); // "0A", text width 52.8
        let out = annotate_out(vec![(10.0, 110.0, Some(0), b'v')], &s, true);
        assert!(out.contains(">0A<"), "{}", out);
        assert!(out.contains("x=\"60\""), "{}", out); // centered at (10+110)/2
    }

    #[test]
    fn narrow_band_elides_label() {
        let s = series_of(&["00001010"]);
        // band width 10 < text width 52.8
        let out = annotate_out(vec![(10.0, 20.0, Some(0), b'v')], &s, true);
        assert!(!out.contains(">0A<"), "{}", out);
    }

    #[test]
    fn label_skips_transition_markers_at_band_edges() {
        let s = deadbeef(); // label "DEADBEEF", text width 52.8
        // wide enough for the text but inside the marker margin: skipped
        let out = annotate_out(vec![(10.0, 10.0 + 52.8 + 4.0, Some(0), b'v')], &s, true);
        assert!(!out.contains(">DEADBEEF<"), "{}", out);
        // clears both edge markers: drawn
        let out = annotate_out(vec![(10.0, 10.0 + 52.8 + 8.0 + 0.5, Some(0), b'v')], &s, true);
        assert!(out.contains(">DEADBEEF<"), "{}", out);
    }

    #[test]
    fn unmarked_window_edge_band_only_needs_text_width() {
        let s = deadbeef(); // text width 52.8
        // The band spans the whole window, so neither edge carries a
        // transition marker: 60px fits the text (52.8) though it is
        // narrower than text + 2*4px marker margin.
        let out = annotate_in(vec![(0.0, 60.0, Some(0), b'v')], &s, true, 0.0, 60.0);
        assert!(out.contains(">DEADBEEF<"), "{}", out);
        // the same band with a marked left edge (interior band) is skipped
        let out = annotate_in(vec![(0.0, 60.0, Some(0), b'v')], &s, true, 0.0, 1000.0);
        assert!(!out.contains(">DEADBEEF<"), "{}", out);
    }

    #[test]
    fn xz_bus_labels_use_the_value_payload() {
        let s = series_of(&["0000x1z1"]); // wide vector with x -> collapses
        let out = annotate_out(vec![(10.0, 110.0, Some(0), b'x')], &s, true);
        assert!(out.contains(">x<"), "{}", out);
        // missing payload falls back to the kind token
        let empty = series_of(&[]);
        let out = annotate_out(vec![(10.0, 110.0, Some(0), b'z')], &empty, true);
        assert!(out.contains(">z<"), "{}", out);
    }

    #[test]
    fn no_bus_labels_flag_disables_annotation() {
        let s = series_of(&["00001010"]);
        let out = annotate_out(vec![(10.0, 110.0, Some(0), b'v')], &s, false);
        assert!(!out.contains(">0A<"), "{}", out);
        assert!(!out.contains("<text"), "{}", out);
    }
}
