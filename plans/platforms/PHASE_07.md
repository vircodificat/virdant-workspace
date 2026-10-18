# Phase 7: Build Module (Constraints + Toolchain)

Create `virdant/src/build/mod.rs`:

```rust
pub mod constraints;
pub mod toolchain;

pub use constraints::*;
pub use toolchain::*;
```

Add `pub mod build;` to `virdant/src/lib.rs`.

### 7.1 `constraints.rs`

**Why:** nextpnr-ice40 needs a PCF file telling it which physical pin
each top-module port is wired to, and this is the one place that
turns the platform's `@pin`/`@period_ns` metadata into that file.

Only PCF is needed for v1 (Decision 8). Under v4 the platform's data
is read with the Phase 4 accessors — there is no `PlatformAnalysis`
type:

```rust
use crate::analysis::platform::{platform_pins, platform_clocks};

/// Emits the PCF for a platform item: one `set_io` line per pinned
/// port, one `set_frequency` line per clock port.  Platforms with
/// several clocks simply get several `set_frequency` lines (the PCF
/// syntax is per-net; Decision 13 forbids cardinality constraints).
/// A platform port with no `@pin` yields an `Unknown` diagnostic
/// naming the port, and no file is produced — the PCF would
/// otherwise be silently incomplete (Decision 13: point-of-use
/// errors).
pub fn emit_pcf(builder: &mut Builder, platform_id: SymbolId)
    -> Result<String, Vec<Diagnostic>>
{
    let pins = platform_pins(builder, platform_id);
    let clocks = platform_clocks(builder, platform_id);

    // Every platform port must have a @pin: compare the platform's
    // full port set (symboltable slots) against `pins`; a port with
    // no pin collects diagnostics::Unknown { message: ... }.

    let mut out = String::new();
    for (port, pin) in &pins {
        let _ = writeln!(out, "set_io {port} {pin}");
    }
    for (_port, pin, period_ns) in &clocks {
        let mhz = (1000.0 / period_ns * 10.0).round() / 10.0;
        let _ = writeln!(out, "set_frequency {pin} {mhz:.1}");
    }
    Ok(out)
}
```

Since every top-module port matches a platform pin (the exact-match
check in Phase 8 runs first), no filtering by top ports is needed
here.

### 7.2 `toolchain.rs`

**Why:** the yosys/nextpnr/icepack/icesprog invocations were
hardcoded inline in `vir.rs`, which made it impossible to support a
second FPGA family later; isolating them behind `toolchain_for` gives
a single, testable dispatch point keyed on `@fpga`.

```rust
#[derive(Debug, Clone, Copy)]
pub enum Toolchain { Ice40 }

pub fn toolchain_for(fpga: &str) -> Option<Toolchain> {
    match fpga {
        "ice40" => Some(Toolchain::Ice40),
        _ => None,
    }
}
```

`run_toolchain` and `flash_bitstream` move the ice40 logic out of
`vir.rs bitstream` (`:811-873`) and `upload` (`:947-972`) nearly
verbatim. Signatures stay data-shaped (v4: no platform struct to
pass): `run_toolchain(&builddir, &project, &top_name, part:
Option<BString>, toolchain: Toolchain)` and
`flash_bitstream(cwd: &Path, project: &str)`. Per invocation:
- Write `script.ys` (`read_verilog -I <builddir> <sv files>;
  synth_ice40 -top <top> -json <project>.json`) from
  `glob_sv_files(&builddir)` (`vir.rs:974-982`).
- Run `yosys -l yosys.log script.ys`.
- Run `nextpnr-ice40 --up5k --package <pkg> --top <top> --json ...
  --pcf ... --asc ...`, where `<pkg>` comes from `platform_part`
  (Phase 4.2; `"up5k-sg48"` -> `"sg48"`; default to `"sg48"` if
  `@part` is absent).
- Run `icepack -s <asc> <bin>`.
- `flash_bitstream` runs `icesprog <project>.bin`.

`toolchain_for` returns `None` for any family other than `ice40`,
which Phase 8 turns into a hard error. This is the *only* family
check in the codebase: platforms are unvalidated data (Decision 13),
so `@fpga("ecp5")` parses and passes `vir check` fine, and simply
fails here when a build is attempted.