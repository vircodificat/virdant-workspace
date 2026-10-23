//! Shared display types used by layout, feature modules, and the renderer.

use crate::vcd::Vcd;
use std::rc::Rc;

/// One drawn row: either a real signal trace or a scope group header.
#[derive(Debug, Clone)]
pub enum Row {
    Signal {
        /// Index into Vcd::signals.
        signal: usize,
        /// Full hierarchical path.
        path: Vec<String>,
        /// Header rows the path walks through when grouping is on.
        depth: usize,
    },
    Group {
        /// Scope path segments (excluding root).
        path: Vec<String>,
        depth: usize,
    },
}

impl Row {
    pub fn label(&self) -> String {
        match self {
            Row::Signal { path, .. } => path.join("."),
            Row::Group { path, .. } => path.join("."),
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Row::Signal { depth, .. } | Row::Group { depth, .. } => *depth,
        }
    }
}

/// A value change flattened to what the renderer needs.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub time: u64,
    /// b'0', b'1', b'x', b'z' (vector: 0/1/x/z per-msb classification).
    pub kind: u8,
}

/// A flattened, per-row time series ready to draw.
#[derive(Debug, Clone)]
pub struct Series {
    pub events: Vec<Event>,
    /// For buses: raw bit payloads aligned with `events` (empty for scalars).
    pub values: Vec<std::rc::Rc<[u8]>>,
    pub width: u32,
    pub is_bus: bool,
    /// Presentation basis for numeric labels (e.g. 15:0 => msb 15).
    pub range: Option<(i32, i32)>,
}

pub fn build_series(vcd: &Vcd, signal_idx: usize) -> Series {
    let sig = &vcd.signals[signal_idx];
    let changes = &vcd.changes[signal_idx];
    let is_bus = sig.width > 1;
    let mut events = Vec::with_capacity(changes.len());
    for c in changes {
        let kind = match &c.value {
            crate::vcd::Value::Bit(b) => classify_bit(*b),
            crate::vcd::Value::Bits(bits) => {
                // Represent a vector change by its "shape": if all bits are
                // z -> z; if any x -> x; else a digital value marker ('v')
                // handled by bus label logic via event kinds.
                if bits.iter().all(|&b| b == b'z' || b == b'Z') {
                    b'z'
                } else if bits.iter().any(|&b| b == b'x' || b == b'X') {
                    b'x'
                } else {
                    b'v'
                }
            }
            crate::vcd::Value::Real(_) => b'v',
        };
        events.push(Event {
            time: c.time,
            kind,
        });
    }
    let values = if is_bus {
        changes
            .iter()
            .map(|c| match &c.value {
                crate::vcd::Value::Bits(b) => b.clone(),
                _ => Rc::from(Vec::new()),
            })
            .collect()
    } else {
        Vec::new()
    };
    Series {
        events,
        values,
        width: sig.width,
        is_bus,
        range: sig.range,
    }
}

fn classify_bit(b: u8) -> u8 {
    match b {
        b'0' => b'0',
        b'1' => b'1',
        b'z' | b'Z' => b'z',
        _ => b'x',
    }
}
