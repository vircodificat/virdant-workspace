//! Bus helpers: X/Z segment styling and bus value formatting/placement.
//!
//! Base implementation: simple but presentable. The bus-label and X/Z
//! feature modules refine these functions.


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

/// Styling for a segment starting with the given event kind.
/// Scalar trace color is applied by the renderer; `fill_override`/`dash`
/// refine x/z presentation.
pub fn style_for(kind: u8) -> SegStyle {
    match kind {
        b'1' => SegStyle { level_frac: FRAC_HI, ..SegStyle::normal() },
        b'0' => SegStyle { level_frac: FRAC_LO, ..SegStyle::normal() },
        b'x' => SegStyle {
            dash: Some("5,3"),
            opacity: 0.55,
            level_frac: FRAC_MID,
            ..SegStyle::normal()
        },
        b'z' => SegStyle {
            dash: Some("2,3"),
            opacity: 0.35,
            level_frac: FRAC_MID,
            ..SegStyle::normal()
        },
        _ => SegStyle::normal(),
    }
}

/// Render a vector value as hex (or as 0/1/x/z for width-1 buses).
pub fn format_bits(bits: &[u8], range: Option<(i32, i32)>) -> String {
    if bits.len() == 1 {
        return String::from_utf8_lossy(&[bits[0].to_ascii_lowercase()]).into_owned();
    }
    // If any digit is x/z, show the raw pattern for short vectors, else 0x??
    if bits.iter().any(|&b| matches!(b, b'x' | b'X' | b'z' | b'Z')) {
        if bits.len() <= 8 {
            return String::from_utf8_lossy(bits).into_owned();
        }
        let x = bits.iter().any(|&b| matches!(b, b'x' | b'X'));
        return if x { "xxxx".into() } else { "zzzz".into() };
    }
    let mut hex = String::with_capacity((bits.len() + 3) / 4);
    let mut acc: u32 = 0;
    let mut n = 0;
    let prefix = if bits.len() > 8 {
        format!("0x{:X}", hex_val_prefix(bits, range))
    } else {
        String::new()
    };
    let _ = prefix;
    for &b in bits {
        acc = (acc << 1) | (b == b'1') as u32;
        n += 1;
        if n == 4 {
            hex.push_str(&format!("{:X}", acc & 0xF));
            acc = 0;
            n = 0;
        }
    }
    if n > 0 {
        hex.push_str(&format!("{:X}", acc));
    }
    // strip leading zero nibbles but keep at least one digit
    let trimmed = hex.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.to_string()
    }
}

fn hex_val_prefix(_bits: &[u8], _range: Option<(i32, i32)>) -> u32 {
    0
}

/// Extra SVG <defs> content (e.g. hatch patterns for X/Z fills).
pub fn svg_defs(_theme: &crate::theme::Theme) -> String {
    String::new()
}
