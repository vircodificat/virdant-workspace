//! Minimal SVG writer with pretty-printing.

pub struct Svg {
    buf: String,
    indent: usize,
    font: String,
}

impl Svg {
    pub fn new(font: &str) -> Svg {
        Svg {
            buf: String::with_capacity(1 << 20),
            indent: 0,
            font: font.to_string(),
        }
    }

    pub fn font(&self) -> &str {
        &self.font
    }

    pub fn open(&mut self, tag: &str, attrs: &[(&str, String)]) {
        self.push_indent();
        self.buf.push('<');
        self.buf.push_str(tag);
        self.write_attrs(attrs);
        self.buf.push_str(">\n");
        self.indent += 1;
    }

    pub fn close(&mut self, tag: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.push_indent();
        self.buf.push_str("</");
        self.buf.push_str(tag);
        self.buf.push_str(">\n");
    }

    pub fn leaf(&mut self, tag: &str, attrs: &[(&str, String)]) {
        self.push_indent();
        self.buf.push('<');
        self.buf.push_str(tag);
        self.write_attrs(attrs);
        self.buf.push_str("/>\n");
    }

    pub fn text(&mut self, x: f64, y: f64, s: &str, size: f64, fill: &str, anchor: &str) {
        let esc = escape(s);
        self.leaf_text(x, y, &esc, size, fill, anchor);
    }

    fn leaf_text(&mut self, x: f64, y: f64, esc: &str, size: f64, fill: &str, anchor: &str) {
        self.push_indent();
        self.buf.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" text-anchor=\"{}\" dominant-baseline=\"middle\">{}</text>\n",
            fmt(x),
            fmt(y),
            fmt(size),
            fill,
            anchor,
            esc
        ));
    }

    pub fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn write_attrs(&mut self, attrs: &[(&str, String)]) {
        for (k, v) in attrs {
            self.buf.push_str(&format!(" {}=\"{}\"", k, v));
        }
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
    }

    /// Wrap into a complete standalone SVG document.
    pub fn finish(self, width: u32, height: u32, title: &str, defs: &str) -> String {
        let mut out = String::with_capacity(self.buf.len() + 4096);
        out.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" font-family=\"{}\">\n",
            width, height, width, height, self.font
        ));
        out.push_str(&format!("<title>{}</title>\n", escape(title)));
        if !defs.is_empty() {
            out.push_str(&format!("<defs>\n{}</defs>\n", defs));
        }
        out.push_str(&self.buf);
        out.push_str("</svg>\n");
        out
    }
}

pub fn fmt(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.2}", v)
    }
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
