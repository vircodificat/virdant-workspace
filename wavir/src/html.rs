//! Feature: interactive HTML viewer export (--html).
//!
//! Single-file, dependency-free viewer: the static SVG is embedded inline and
//! augmented with JavaScript for wheel zoom, drag pan, and a hover crosshair
//! with a per-signal value readout. The value data is precomputed in Rust
//! (values already formatted with `bus::format_bits`) and embedded as JSON in
//! a `<script type="application/json">` tag; JavaScript binary-searches the
//! value at the hovered time.
//!
//! Event data is capped per row (first/last kept, thinned uniformly) and all
//! `<`/`>` characters in the JSON are escaped as U+003C / U+003E so the payload can
//! never terminate the script tag.

use crate::cli::Opts;
use crate::theme::Theme;
use crate::vcd::{Value, Vcd};

/// Maximum embedded events per signal row (downsampled uniformly).
const EVENT_CAP: usize = 3000;

/// The never-changing viewer scaffold; theme colors and content are spliced
/// in via `__TOKEN__` replacements so no brace escaping is needed.
const PAGE: &str = r##"<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>wavir — __TITLE__</title>
<style>
html,body{margin:0;height:100%;background:__BG__;overflow:hidden}
#wrap{position:fixed;inset:0}
#wrap svg{display:block;width:100%;height:100%;cursor:crosshair}
#panel{position:fixed;top:10px;right:10px;background:__BG__;border:1px solid __GRID__;border-radius:6px;padding:8px 10px;font-family:__FONT__;font-size:11px;color:__TEXT__;max-height:80vh;overflow:auto;min-width:230px;display:none;z-index:2;box-shadow:0 4px 16px rgba(0,0,0,.45)}
#panel .t{color:__CURSOR__;font-weight:700;margin-bottom:6px;white-space:nowrap}
#panel table{border-collapse:collapse}
#panel td{padding:1px 6px 1px 0;white-space:nowrap}
#panel td.n{color:__MUTED__;max-width:240px;overflow:hidden;text-overflow:ellipsis}
#panel td.v{color:__ACCENT__;font-weight:700}
#panel tr.hov td{background:__ROWALT__}
#hint{position:fixed;bottom:8px;left:10px;font-family:__FONT__;font-size:10px;color:__MUTED__;opacity:.75;z-index:2}
</style></head>
<body>
<div id="wrap">__SVG__</div>
<div id="panel"><div class="t"></div><table></table></div>
<div id="hint">wheel: zoom &middot; drag: pan &middot; double-click: reset &middot; hover: values at time</div>
<script type="application/json" id="wavir-data">__DATA__</script>
<script>
(function(){
"use strict";
var svg=document.getElementById("wavir-svg");
var D=JSON.parse(document.getElementById("wavir-data").textContent);
var M=D.meta;
var vb={x:0,y:0,w:M.w,h:M.h};
function apply(){svg.setAttribute("viewBox",vb.x+" "+vb.y+" "+vb.w+" "+vb.h);}
apply();
function svgPt(ev){
  var m=svg.getScreenCTM();
  var p=new DOMPoint(ev.clientX,ev.clientY);
  return p.matrixTransform(m.inverse());
}
function clampVb(){
  vb.x=Math.min(Math.max(vb.x,0),Math.max(0,M.w-vb.w));
  vb.y=Math.min(Math.max(vb.y,0),Math.max(0,M.h-vb.h));
}
/* ---- wheel zoom around the pointer ---- */
svg.addEventListener("wheel",function(e){
  e.preventDefault();
  var k=Math.pow(1.0015,e.deltaY);
  var nw=Math.min(M.w,Math.max(M.w/200,vb.w*k));
  var s=nw/vb.w;
  var p=svgPt(e);
  vb.x=p.x-(p.x-vb.x)*s;
  vb.y=p.y-(p.y-vb.y)*s;
  vb.w=nw;vb.h=vb.h*s;
  clampVb();apply();update(e);
},{passive:false});
/* ---- drag pan ---- */
var drag=null;
svg.addEventListener("mousedown",function(e){
  var p=svgPt(e);
  drag={px:p.x,py:p.y,vx:vb.x,vy:vb.y};
  e.preventDefault();
});
window.addEventListener("mousemove",function(e){
  if(drag){
    var p=svgPt(e);
    vb.x=drag.vx+(drag.px-p.x);
    vb.y=drag.vy+(drag.py-p.y);
    clampVb();apply();
  }
  update(e);
});
window.addEventListener("mouseup",function(){drag=null;});
svg.addEventListener("dblclick",function(){vb={x:0,y:0,w:M.w,h:M.h};apply();update(null);});
/* ---- crosshair + readout ---- */
var NS="http://www.w3.org/2000/svg";
var cross=document.createElementNS(NS,"line");
cross.setAttribute("stroke",M.cursor);
cross.setAttribute("stroke-width","1");
cross.setAttribute("stroke-dasharray","3,3");
cross.setAttribute("visibility","hidden");
var band=document.createElementNS(NS,"rect");
band.setAttribute("fill",M.cursor);
band.setAttribute("opacity","0.08");
band.setAttribute("visibility","hidden");
var label=document.createElementNS(NS,"text");
label.setAttribute("font-size","10");
label.setAttribute("fill",M.cursor);
label.setAttribute("text-anchor","middle");
label.setAttribute("dominant-baseline","middle");
label.setAttribute("visibility","hidden");
svg.appendChild(band);
svg.appendChild(cross);
svg.appendChild(label);
var panel=document.getElementById("panel");
var panelTitle=panel.querySelector(".t");
var panelTable=panel.querySelector("table");
function fmtSecs(s){
  var a=Math.abs(s);
  if(a===0)return"0";
  var u=[[1,"s"],[1e-3,"ms"],[1e-6,"µs"],[1e-9,"ns"],[1e-12,"ps"]];
  for(var i=0;i<u.length;i++){
    if(a>=u[i][0]){
      var v=s/u[i][0];
      return(Math.round(v*100)/100)+u[i][1];
    }
  }
  return String(s);
}
function valAt(evts,t){
  if(!evts)return null;
  if(!evts.length)return null;
  if(t<evts[0][0])return evts[0][1];
  var lo=0,hi=evts.length-1;
  while(lo<hi){
    var mid=(lo+hi+1)>>1;
    if(evts[mid][0]<=t)lo=mid;else hi=mid-1;
  }
  return evts[lo][1];
}
function update(ev){
  if(!ev){cross.setAttribute("visibility","hidden");band.setAttribute("visibility","hidden");label.setAttribute("visibility","hidden");panel.style.display="none";return;}
  var p=svgPt(ev);
  var inX=p.x>=M.waveX0&&p.x<=M.waveX1;
  var yBot=M.rowsY0+M.nRows*M.rowH;
  if(!inX||p.y<M.rowsY0-30||p.y>yBot+30){
    cross.setAttribute("visibility","hidden");band.setAttribute("visibility","hidden");
    label.setAttribute("visibility","hidden");panel.style.display="none";
    return;
  }
  var t=M.t0+(p.x-M.waveX0)/(M.waveX1-M.waveX0)*(M.t1-M.t0);
  cross.setAttribute("x1",p.x);cross.setAttribute("x2",p.x);
  cross.setAttribute("y1",M.rowsY0);cross.setAttribute("y2",M.svgH-M.axisBH);
  cross.setAttribute("visibility","visible");
  label.setAttribute("x",p.x);label.setAttribute("y",M.rowsY0-8);
  label.textContent=fmtSecs(t);
  label.setAttribute("visibility","visible");
  /* hovered row band */
  var ri=Math.floor((p.y-M.rowsY0)/M.rowH);
  if(ri>=0&&ri<M.nRows){
    band.setAttribute("x",0);band.setAttribute("y",M.rowsY0+ri*M.rowH);
    band.setAttribute("width",M.w);band.setAttribute("height",M.rowH);
    band.setAttribute("visibility","visible");
  }else{band.setAttribute("visibility","hidden");}
  /* readout */
  var esc=function(s){return String(s).replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/\"/g,"&quot;");};
  var html="<tr><td>time</td><td class=v>"+fmtSecs(t)+"</td></tr>";
  for(var i=0;i<D.rows.length;i++){
    var r=D.rows[i];
    if(r.k==="group")continue;
    var v=valAt(D.series[r.s],t);
    html+="<tr"+(i===ri?" class=hov":"")+"><td class=n>"+esc(r.n)+"</td><td class=v>"+
      (v===null||v===undefined?"—":esc(v))+"</td></tr>";
  }
  panelTitle.textContent="t = "+fmtSecs(t);
  panelTable.innerHTML=html;
  panel.style.display="block";
}
})();
</script>
<!-- wavir: per-row event data capped at __CAP__ entries (uniformly thinned, first/last kept) -->
</body></html>
"##;

pub fn export(vcd: &Vcd, opts: &Opts, theme: &Theme) -> Result<String, String> {
    let rows = crate::display::build(vcd, opts);
    let layout = crate::layout::Layout::compute(vcd, opts, &rows, theme);
    let svg = crate::render::render(vcd, opts, theme);
    if !svg.starts_with("<svg") {
        return Err("internal: renderer did not produce an SVG document".into());
    }
    // Tag the root so the viewer script can find it.
    let svg = svg.replacen("<svg xmlns", "<svg id=\"wavir-svg\" xmlns", 1);

    let data = build_data(vcd, opts, &rows, &layout, theme);
    let name = opts.input.rsplit('/').next().unwrap_or(&opts.input);
    let page = PAGE
        .replace("__TITLE__", &crate::svg::escape(name))
        .replace("__BG__", theme.bg)
        .replace("__ROWALT__", theme.row_alt)
        .replace("__GRID__", theme.grid_major)
        .replace("__TEXT__", theme.text_primary)
        .replace("__MUTED__", theme.text_muted)
        .replace("__ACCENT__", theme.accent)
        .replace("__CURSOR__", theme.cursor_color)
        .replace("__FONT__", theme.font_family)
        .replace("__CAP__", &EVENT_CAP.to_string())
        .replace("__SVG__", &svg)
        .replace("__DATA__", &data);
    Ok(page)
}

/// Build the embedded JSON: shared series (deduped per signal index) plus the
/// row table referencing them.
fn build_data(
    vcd: &Vcd,
    _opts: &Opts,
    rows: &[crate::types::Row],
    layout: &crate::layout::Layout,
    theme: &Theme,
) -> String {
    let tick_secs = layout.tick_secs;
    // Dedupe series: many rows can reference the same signal (shared id).
    let mut series_idx: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut series_json: Vec<String> = Vec::new();
    let mut rows_json: Vec<String> = Vec::new();

    for (ri, row) in rows.iter().enumerate() {
        let y = layout.rows_y0 + ri as f64 * layout.row_h as f64;
        match row {
            crate::types::Row::Group { .. } => {
                rows_json.push(format!(
                    "{{\"n\":\"{}\",\"k\":\"group\",\"y\":{}}}",
                    json_escape(&row.label()),
                    crate::svg::fmt(y)
                ));
            }
            crate::types::Row::Signal { signal, .. } => {
                let slot = match series_idx.get(signal) {
                    Some(&s) => s,
                    None => {
                        let s = series_json.len();
                        series_idx.insert(*signal, s);
                        series_json.push(series_events_json(
                            vcd,
                            *signal,
                            tick_secs,
                            layout.window.start,
                        ));
                        s
                    }
                };
                let kind = if vcd.signals[*signal].width > 1 { "bus" } else { "bit" };
                rows_json.push(format!(
                    "{{\"n\":\"{}\",\"k\":\"{}\",\"y\":{},\"s\":{}}}",
                    json_escape(&row.label()),
                    kind,
                    crate::svg::fmt(y),
                    slot
                ));
            }
        }
    }

    let t0 = layout.window.start as f64 * tick_secs;
    let t1 = layout.window.end as f64 * tick_secs;
    format!(
        "{{\"meta\":{{\"w\":{},\"h\":{},\"svgH\":{},\"waveX0\":{},\"waveX1\":{},\"rowsY0\":{},\"rowH\":{},\"nRows\":{},\"axisBH\":{},\"t0\":{},\"t1\":{},\"cursor\":\"{}\"}},\"series\":[{}],\"rows\":[{}]}}",
        layout.svg_w,
        layout.svg_h,
        layout.svg_h,
        crate::svg::fmt(layout.wave_x0),
        crate::svg::fmt(layout.wave_x1),
        crate::svg::fmt(layout.rows_y0),
        layout.row_h,
        rows.len(),
        layout.axis_b_h,
        t0,
        t1,
        theme.cursor_color,
        series_json.join(","),
        rows_json.join(",")
    )
}

/// Events for one signal as `[[time_seconds, value_string], ...]`, capped.
///
/// If the signal's first change is after the window start, an initial "x"
/// event is prepended at the window start (VCD semantics: undriven before the
/// first change; matches the renderer's pre-first-event x presentation).
fn series_events_json(vcd: &Vcd, signal: usize, tick_secs: f64, start_tick: u64) -> String {
    let changes = &vcd.changes[signal];
    let sig = &vcd.signals[signal];
    let keep = downsample_indices(changes.len(), EVENT_CAP);
    let mut prefix = String::new();
    if !keep.is_empty() && changes[keep[0]].time > start_tick {
        let t0 = start_tick as f64 * tick_secs;
        prefix = format!("[{},\"x\"],", t0);
    } 
    let mut parts = Vec::with_capacity(keep.len());
    for &i in &keep {
        let c = &changes[i];
        let t = c.time as f64 * tick_secs;
        let v = match &c.value {
            Value::Bit(b) => {
                let lo = b.to_ascii_lowercase();
                String::from_utf8_lossy(&[lo]).into_owned()
            }
            Value::Real(r) => format!("{}", r),
            Value::Bits(bits) => crate::bus::format_bits(bits, sig.range),
        };
        parts.push(format!("[{},\"{}\"]", t, json_escape(&v)));
    }
    format!("[{}{}]", prefix, parts.join(","))
}

/// Indices to keep when capping `n` entries to `cap` (first and last always
/// kept, remainder thinned uniformly).
fn downsample_indices(n: usize, cap: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    if cap < 2 {
        // degenerate cap: keep only the first entry so the cap holds
        return vec![0];
    }
    if n <= cap {
        return (0..n).collect();
    }
    let last = n - 1;
    let step = ((last as f64) / (cap as f64 - 1.0)).ceil() as usize;
    let mut idx = Vec::with_capacity(cap);
    let mut i = 0;
    while i < last {
        idx.push(i);
        i += step;
    }
    if *idx.last().unwrap() != last {
        idx.push(last);
    }
    idx
}

/// JSON string escaping; `<`/`>` are escaped as U+003C / U+003E so a payload can
/// never contain `</script>` or `<!--`.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_keeps_first_and_last_and_respects_cap() {
        assert_eq!(downsample_indices(5, 3000), vec![0, 1, 2, 3, 4]);
        let idx = downsample_indices(10_000, 3000);
        assert_eq!(idx.first(), Some(&0));
        assert_eq!(idx.last(), Some(&9_999));
        assert!(idx.len() <= 3000);
        assert!(idx.windows(2).all(|w| w[1] > w[0]));
        assert_eq!(downsample_indices(0, 3000), Vec::<usize>::new());
        assert_eq!(downsample_indices(1, 3000), vec![0]);
    }

    #[test]
    fn json_escape_neutralizes_script_terminators() {
        assert_eq!(json_escape("a</script>b"), "a\\u003c/script\\u003eb");
        assert_eq!(json_escape("q\"x\\y"), "q\\\"x\\\\y");
        assert_eq!(json_escape("l\nb"), "l\\nb");
    }
}
