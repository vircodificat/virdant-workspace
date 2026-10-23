//! Building the display list from the VCD signal table.
//!
//! Base behavior: flat list of every (deduplicated) signal in declaration
//! order, labeled by full hierarchical path. The `group` feature replaces
//! this with hierarchical grouping + glob filtering.

use crate::cli::Opts;
use crate::types::Row;
use crate::vcd::Vcd;

pub fn build(vcd: &Vcd, _opts: &Opts) -> Vec<Row> {
    vcd.signals
        .iter()
        .enumerate()
        .map(|(i, s)| Row::Signal {
            signal: i,
            path: s.path.clone(),
            depth: 0,
        })
        .collect()
}
