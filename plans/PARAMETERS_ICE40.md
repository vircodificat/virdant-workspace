# Migrating lib/ice40.vir to Declare Real Cell Parameters

## Summary

This is a companion to `PARAMETERS.md` (the `@parameter("NAME"[, VALUE])`
mechanism design). That plan explicitly left `lib/ice40.vir` unmigrated.
This plan is the migration itself: adding `@parameter("NAME", DEFAULT)`
declarations to every `ext mod` in `lib/ice40.vir` whose underlying
SiliconBlue primitive has real Verilog parameters, with the default set
to the primitive's own stock default (so an un-annotated instantiation
keeps behaving exactly as it does today).

Every value here is taken directly from the actual Yosys simulation
models for these cells, not from memory or documentation summaries:

```
$ yosys --version
Yosys 0.56 (git sha1 9c447ad9d, ...)
$ grep -n '^module\|parameter' ~/.local/yosyshq/yosys/share/yosys/ice40/cells_sim.v
```

Every parameter name, bit width, and default literal below was read
from that file. Cross-reference `cells_sim.v` directly during
implementation in case a newer/older Yosys disagrees with this
snapshot (0.56).


## Methodology

For each `ext mod` already declared in `lib/ice40.vir`, its `@cell(...)`
name was looked up in `cells_sim.v`, and every `parameter` statement in
that Verilog module was recorded verbatim (name, declared bit range if
any, default literal).

**Type assignment rule:** if the cell declares the parameter's default
as a quoted Verilog string (`parameter X = "...";`), it becomes a `Str`
`@parameter`. This includes several parameters whose string *looks*
like a binary literal (e.g. `RGB0_CURRENT = "0b000000"`) — that is a
genuine SiliconBlue library quirk, not a transcription error: these
are real Verilog string parameters, not sized constants, and must be
supplied as quoted strings (`@parameter("RGB0_CURRENT", "0b000000")`),
not as `Nat` (`@parameter("RGB0_CURRENT", 0)` would be wrong — it would
render as the bare integer `0`, not the string `"0b000000"`). Every
other parameter becomes a `Nat` `@parameter`, rendered as a plain
decimal integer (matching `PARAMETERS.md`'s renderer, which discards
any bit-width annotation).

**Excluded:** any parameter wider than 64 bits, per `PARAMETERS.md`'s
known limitation (`WordValue = u64`). The only cells affected are the
`SB_RAM40_4K` family's `INIT_0`..`INIT_F` (256 bits each). Their
`INIT_FILE` parameter (a `Str`, pointing to an external `.hex`/`.mem`
init file) is unaffected and is included below.


## Per-cell parameter tables

Each block below is ready to paste directly above the corresponding
`ext mod` line in `lib/ice40.vir`, replacing any "Virdant cannot set
this parameter yet" doc-comment text that references it.

### SB_LUT4 → `SbLut4`

| Parameter | Kind | Default |
|---|---|---|
| `LUT_INIT` | `Nat` | `0` (16-bit truth table) |

```virdant
@parameter("LUT_INIT", 0)
@cell("SB_LUT4")
ext mod SbLut4 { ... }
```

### SB_CARRY, SB_GB, SB_SPRAM256KA, SB_LFOSC, SB_LED_DRV_CUR, SB_LEDDA_IP, SB_WARMBOOT, and all 20 SB_DFF* variants

No parameters in `cells_sim.v`. No change: `SbCarry`, `SbGb`,
`SbSpram256ka`, `SbLfosc`, `SbLedDrvCur`, `SbLeddaIp`, `SbWarmboot`,
`SbDff`, `SbDffe`, `SbDffsr`, `SbDffr`, `SbDffss`, `SbDffs`,
`SbDffesr`, `SbDffer`, `SbDffess`, `SbDffes`, `SbDffn`, `SbDffne`,
`SbDffnsr`, `SbDffnr`, `SbDffnss`, `SbDffns`, `SbDffnesr`, `SbDffner`,
`SbDffness`, `SbDffnes`.

### SB_IO → `SbIo`, SB_GB_IO → `SbGbIo`

Identical 4-parameter set on both cells (`SB_GB_IO` is a thin wrapper
around `SB_IO` in `cells_sim.v`):

| Parameter | Kind | Default | Meaning |
|---|---|---|---|
| `PIN_TYPE` | `Nat` | `0` (6-bit; `6'b000000`) | Pin behavior: input latch/DDR mode (bits `[1:0]`), output mode/DDR (bits `[3:2]`), driver tristate mode (bits `[5:4]`) |
| `PULLUP` | `Nat` | `0` | Enables the pad's weak pull-up |
| `NEG_TRIGGER` | `Nat` | `0` | Inverts the sense of all registered I/O clocking (negative-edge instead of positive-edge) |
| `IO_STANDARD` | `Str` | `"SB_LVCMOS"` | I/O electrical standard |

```virdant
@parameter("PIN_TYPE", 0)
@parameter("PULLUP", 0)
@parameter("NEG_TRIGGER", 0)
@parameter("IO_STANDARD", "SB_LVCMOS")
@cell("SB_IO")
ext mod SbIo { ... }
```

Same four lines (substituting `@cell("SB_GB_IO")`) on `SbGbIo`.

### SB_RAM40_4K → `SbRam40_4k`, SB_RAM40_4KNR → `SbRam40_4knr`, SB_RAM40_4KNW → `SbRam40_4knw`, SB_RAM40_4KNRNW → `SbRam40_4knrnw`

Identical parameter set on all four variants:

| Parameter | Kind | Default |
|---|---|---|
| `WRITE_MODE` | `Nat` | `0` (one of 4 write-port aspect-ratio modes) |
| `READ_MODE` | `Nat` | `0` (one of 4 read-port aspect-ratio modes) |
| `INIT_FILE` | `Str` | `""` (path to a memory-init file; empty = no file) |
| ~~`INIT_0`..`INIT_F`~~ | — | **excluded** — 256 bits each, exceeds `WordValue`'s 64-bit limit (see `PARAMETERS.md`) |

```virdant
@parameter("WRITE_MODE", 0)
@parameter("READ_MODE", 0)
@parameter("INIT_FILE", "")
@cell("SB_RAM40_4K")
ext mod SbRam40_4k { ... }
```

Same three lines (substituting the matching `@cell(...)`) on
`SbRam40_4knr`, `SbRam40_4knw`, `SbRam40_4knrnw`.

### SB_PLL40_CORE → `SbPll40Core`, SB_PLL40_PAD → `SbPll40Pad`

Identical 14-parameter set on both:

| Parameter | Kind | Default |
|---|---|---|
| `FEEDBACK_PATH` | `Str` | `"SIMPLE"` |
| `DELAY_ADJUSTMENT_MODE_FEEDBACK` | `Str` | `"FIXED"` |
| `DELAY_ADJUSTMENT_MODE_RELATIVE` | `Str` | `"FIXED"` |
| `SHIFTREG_DIV_MODE` | `Nat` | `0` |
| `FDA_FEEDBACK` | `Nat` | `0` (4-bit) |
| `FDA_RELATIVE` | `Nat` | `0` (4-bit) |
| `PLLOUT_SELECT` | `Str` | `"GENCLK"` |
| `DIVR` | `Nat` | `0` (4-bit) |
| `DIVF` | `Nat` | `0` (7-bit) |
| `DIVQ` | `Nat` | `0` (3-bit) |
| `FILTER_RANGE` | `Nat` | `0` (3-bit) |
| `ENABLE_ICEGATE` | `Nat` | `0` |
| `TEST_MODE` | `Nat` | `0` |
| `EXTERNAL_DIVIDE_FACTOR` | `Nat` | `1` |

```virdant
@parameter("FEEDBACK_PATH", "SIMPLE")
@parameter("DELAY_ADJUSTMENT_MODE_FEEDBACK", "FIXED")
@parameter("DELAY_ADJUSTMENT_MODE_RELATIVE", "FIXED")
@parameter("SHIFTREG_DIV_MODE", 0)
@parameter("FDA_FEEDBACK", 0)
@parameter("FDA_RELATIVE", 0)
@parameter("PLLOUT_SELECT", "GENCLK")
@parameter("DIVR", 0)
@parameter("DIVF", 0)
@parameter("DIVQ", 0)
@parameter("FILTER_RANGE", 0)
@parameter("ENABLE_ICEGATE", 0)
@parameter("TEST_MODE", 0)
@parameter("EXTERNAL_DIVIDE_FACTOR", 1)
@cell("SB_PLL40_CORE")
ext mod SbPll40Core { ... }
```

Same 14 lines (substituting `@cell("SB_PLL40_PAD")`) on `SbPll40Pad`.

### SB_PLL40_2_PAD → `SbPll40_2Pad`

15 parameters. **Note the asymmetry below is verbatim from
`cells_sim.v`, not an error in this plan**: unlike every other
dual-output PLL variant, this one declares `PLLOUT_SELECT_PORTB` but
*no* `PLLOUT_SELECT_PORTA` at all — port A's output mux has no
user-settable parameter on this specific cell.

| Parameter | Kind | Default |
|---|---|---|
| `FEEDBACK_PATH` | `Str` | `"SIMPLE"` |
| `DELAY_ADJUSTMENT_MODE_FEEDBACK` | `Str` | `"FIXED"` |
| `DELAY_ADJUSTMENT_MODE_RELATIVE` | `Str` | `"FIXED"` |
| `SHIFTREG_DIV_MODE` | `Nat` | `0` |
| `FDA_FEEDBACK` | `Nat` | `0` (4-bit) |
| `FDA_RELATIVE` | `Nat` | `0` (4-bit) |
| `PLLOUT_SELECT_PORTB` | `Str` | `"GENCLK"` |
| `DIVR` | `Nat` | `0` (4-bit) |
| `DIVF` | `Nat` | `0` (7-bit) |
| `DIVQ` | `Nat` | `0` (3-bit) |
| `FILTER_RANGE` | `Nat` | `0` (3-bit) |
| `ENABLE_ICEGATE_PORTA` | `Nat` | `0` |
| `ENABLE_ICEGATE_PORTB` | `Nat` | `0` |
| `TEST_MODE` | `Nat` | `0` |
| `EXTERNAL_DIVIDE_FACTOR` | `Nat` | `1` |

```virdant
@parameter("FEEDBACK_PATH", "SIMPLE")
@parameter("DELAY_ADJUSTMENT_MODE_FEEDBACK", "FIXED")
@parameter("DELAY_ADJUSTMENT_MODE_RELATIVE", "FIXED")
@parameter("SHIFTREG_DIV_MODE", 0)
@parameter("FDA_FEEDBACK", 0)
@parameter("FDA_RELATIVE", 0)
@parameter("PLLOUT_SELECT_PORTB", "GENCLK")
@parameter("DIVR", 0)
@parameter("DIVF", 0)
@parameter("DIVQ", 0)
@parameter("FILTER_RANGE", 0)
@parameter("ENABLE_ICEGATE_PORTA", 0)
@parameter("ENABLE_ICEGATE_PORTB", 0)
@parameter("TEST_MODE", 0)
@parameter("EXTERNAL_DIVIDE_FACTOR", 1)
@cell("SB_PLL40_2_PAD")
ext mod SbPll40_2Pad { ... }
```

### SB_PLL40_2F_CORE → `SbPll40_2fCore`, SB_PLL40_2F_PAD → `SbPll40_2fPad`

16 parameters (same as the `_2_PAD` set above, plus
`PLLOUT_SELECT_PORTA`):

| Parameter | Kind | Default |
|---|---|---|
| `FEEDBACK_PATH` | `Str` | `"SIMPLE"` |
| `DELAY_ADJUSTMENT_MODE_FEEDBACK` | `Str` | `"FIXED"` |
| `DELAY_ADJUSTMENT_MODE_RELATIVE` | `Str` | `"FIXED"` |
| `SHIFTREG_DIV_MODE` | `Nat` | `0` on `_2F_CORE`; **`0` on `_2F_PAD` too** — `cells_sim.v` declares `_2F_PAD`'s default as `2'b00` vs. `_2F_CORE`'s `1'b0`, a width-only difference in the source; the rendered decimal value (`0`) is identical either way |
| `FDA_FEEDBACK` | `Nat` | `0` (4-bit) |
| `FDA_RELATIVE` | `Nat` | `0` (4-bit) |
| `PLLOUT_SELECT_PORTA` | `Str` | `"GENCLK"` |
| `PLLOUT_SELECT_PORTB` | `Str` | `"GENCLK"` |
| `DIVR` | `Nat` | `0` (4-bit) |
| `DIVF` | `Nat` | `0` (7-bit) |
| `DIVQ` | `Nat` | `0` (3-bit) |
| `FILTER_RANGE` | `Nat` | `0` (3-bit) |
| `ENABLE_ICEGATE_PORTA` | `Nat` | `0` |
| `ENABLE_ICEGATE_PORTB` | `Nat` | `0` |
| `TEST_MODE` | `Nat` | `0` |
| `EXTERNAL_DIVIDE_FACTOR` | `Nat` | `1` |

```virdant
@parameter("FEEDBACK_PATH", "SIMPLE")
@parameter("DELAY_ADJUSTMENT_MODE_FEEDBACK", "FIXED")
@parameter("DELAY_ADJUSTMENT_MODE_RELATIVE", "FIXED")
@parameter("SHIFTREG_DIV_MODE", 0)
@parameter("FDA_FEEDBACK", 0)
@parameter("FDA_RELATIVE", 0)
@parameter("PLLOUT_SELECT_PORTA", "GENCLK")
@parameter("PLLOUT_SELECT_PORTB", "GENCLK")
@parameter("DIVR", 0)
@parameter("DIVF", 0)
@parameter("DIVQ", 0)
@parameter("FILTER_RANGE", 0)
@parameter("ENABLE_ICEGATE_PORTA", 0)
@parameter("ENABLE_ICEGATE_PORTB", 0)
@parameter("TEST_MODE", 0)
@parameter("EXTERNAL_DIVIDE_FACTOR", 1)
@cell("SB_PLL40_2F_CORE")
ext mod SbPll40_2fCore { ... }
```

Same 16 lines (substituting `@cell("SB_PLL40_2F_PAD")`) on
`SbPll40_2fPad`.

### SB_HFOSC → `SbHfosc`

| Parameter | Kind | Default | Meaning |
|---|---|---|---|
| `TRIM_EN` | `Str` | `"0b0"` | Enables the `TRIM0`..`TRIM9` input pins overriding the factory trim |
| `CLKHF_DIV` | `Str` | `"0b00"` | Output divider: `"0b00"`=48MHz, `"0b01"`=24MHz, `"0b10"`=12MHz, `"0b11"`=6MHz |

```virdant
@parameter("TRIM_EN", "0b0")
@parameter("CLKHF_DIV", "0b00")
@cell("SB_HFOSC")
ext mod SbHfosc { ... }
```

### SB_RGBA_DRV → `SbRgbaDrv`, SB_RGB_DRV → `SbRgbDrv`

Identical 4-parameter set on both:

| Parameter | Kind | Default |
|---|---|---|
| `CURRENT_MODE` | `Str` | `"0b0"` (`"0b0"` = half-current mode available on `0b1`) |
| `RGB0_CURRENT` | `Str` | `"0b000000"` (6-bit current-level code) |
| `RGB1_CURRENT` | `Str` | `"0b000000"` |
| `RGB2_CURRENT` | `Str` | `"0b000000"` |

```virdant
@parameter("CURRENT_MODE", "0b0")
@parameter("RGB0_CURRENT", "0b000000")
@parameter("RGB1_CURRENT", "0b000000")
@parameter("RGB2_CURRENT", "0b000000")
@cell("SB_RGBA_DRV")
ext mod SbRgbaDrv { ... }
```

Same four lines (substituting `@cell("SB_RGB_DRV")`) on `SbRgbDrv`.

### SB_I2C → `SbI2c`

| Parameter | Kind | Default | Meaning |
|---|---|---|---|
| `I2C_SLAVE_INIT_ADDR` | `Str` | `"0b1111100001"` | 10-bit initial I2C slave address |
| `BUS_ADDR74` | `Str` | `"0b0001"` | 4-bit register-file block-select prefix (distinguishes this hardened block's address range from the chip's other I2C/SPI block) |

```virdant
@parameter("I2C_SLAVE_INIT_ADDR", "0b1111100001")
@parameter("BUS_ADDR74", "0b0001")
@cell("SB_I2C")
ext mod SbI2c { ... }
```

### SB_SPI → `SbSpi`

| Parameter | Kind | Default | Meaning |
|---|---|---|---|
| `BUS_ADDR74` | `Str` | `"0b0000"` | 4-bit register-file block-select prefix |

```virdant
@parameter("BUS_ADDR74", "0b0000")
@cell("SB_SPI")
ext mod SbSpi { ... }
```

### SB_MAC16 → `SbMac16`

20 parameters, all single- or 2-bit, all `Nat`, all defaulting to `0`:

| Parameter | Kind | Default |
|---|---|---|
| `NEG_TRIGGER` | `Nat` | `0` |
| `C_REG` | `Nat` | `0` |
| `A_REG` | `Nat` | `0` |
| `B_REG` | `Nat` | `0` |
| `D_REG` | `Nat` | `0` |
| `TOP_8x8_MULT_REG` | `Nat` | `0` |
| `BOT_8x8_MULT_REG` | `Nat` | `0` |
| `PIPELINE_16x16_MULT_REG1` | `Nat` | `0` |
| `PIPELINE_16x16_MULT_REG2` | `Nat` | `0` |
| `TOPOUTPUT_SELECT` | `Nat` | `0` (2-bit) |
| `TOPADDSUB_LOWERINPUT` | `Nat` | `0` (2-bit) |
| `TOPADDSUB_UPPERINPUT` | `Nat` | `0` |
| `TOPADDSUB_CARRYSELECT` | `Nat` | `0` (2-bit) |
| `BOTOUTPUT_SELECT` | `Nat` | `0` (2-bit) |
| `BOTADDSUB_LOWERINPUT` | `Nat` | `0` (2-bit) |
| `BOTADDSUB_UPPERINPUT` | `Nat` | `0` |
| `BOTADDSUB_CARRYSELECT` | `Nat` | `0` (2-bit) |
| `MODE_8x8` | `Nat` | `0` |
| `A_SIGNED` | `Nat` | `0` |
| `B_SIGNED` | `Nat` | `0` |

```virdant
@parameter("NEG_TRIGGER", 0)
@parameter("C_REG", 0)
@parameter("A_REG", 0)
@parameter("B_REG", 0)
@parameter("D_REG", 0)
@parameter("TOP_8x8_MULT_REG", 0)
@parameter("BOT_8x8_MULT_REG", 0)
@parameter("PIPELINE_16x16_MULT_REG1", 0)
@parameter("PIPELINE_16x16_MULT_REG2", 0)
@parameter("TOPOUTPUT_SELECT", 0)
@parameter("TOPADDSUB_LOWERINPUT", 0)
@parameter("TOPADDSUB_UPPERINPUT", 0)
@parameter("TOPADDSUB_CARRYSELECT", 0)
@parameter("BOTOUTPUT_SELECT", 0)
@parameter("BOTADDSUB_LOWERINPUT", 0)
@parameter("BOTADDSUB_UPPERINPUT", 0)
@parameter("BOTADDSUB_CARRYSELECT", 0)
@parameter("MODE_8x8", 0)
@parameter("A_SIGNED", 0)
@parameter("B_SIGNED", 0)
@cell("SB_MAC16")
ext mod SbMac16 { ... }
```


## Doc-comment cleanup

Several existing doc comments explicitly say Virdant cannot set a
parameter; each becomes stale once that cell is migrated and should be
reworded or removed as part of the same change:

- File header (`lib/ice40.vir:7-9`): "Virdant cannot set Verilog cell
  parameters yet, so instantiations use each cell's default
  parameters." — remove or rephrase to describe `@parameter` instead.
- `SbIo`'s doc comment (`:456`): "Set the pin's behaviour ... through
  the `PIN_TYPE` parameter, which Virdant cannot set yet." → describe
  `@parameter("PIN_TYPE", ...)` instead.
- `SbPll40Pad`/`SbPll40Core`/etc.'s doc comments referencing
  `DIVR`/`DIVF`/`DIVQ` "which Virdant cannot set yet" (`:671-672`,
  `:706-707`, `:740-741`, `:778-779`, `:816-818`) — same treatment.
- `SbRgbaDrv`/`SbRgbDrv`'s doc comments referencing `RGB*_CURRENT`
  "which Virdant cannot set yet" (`:901-902`, `:924-925`).
- `SbMac16`'s doc comment referencing the `*_REG`/`*_SELECT`
  parameters "which Virdant cannot set yet" (`:1205-1207`).


## Interaction with PARAMETERS.md's "defaults always apply" rule

`PARAMETERS.md` decided that an ext mod's declared default is emitted
for *every* instantiation, including ones with zero `@parameter`
annotations of their own. Once this migration lands, every
instantiation of a migrated cell — even a completely bare one — will
emit an explicit `#(...)` block listing that cell's stock defaults,
where today it emits none at all.

This is a *textual* change only, not a behavioral one: every default
value above was copied verbatim from the cell's own built-in Verilog
default, so Yosys sees the identical configuration either way. But it
means any test or tool that does exact-text comparison on generated
`.sv` output (as opposed to inspecting the `Submodule` IR directly, as
`cell_annotation_names_ext_mod_instantiation` does) would need
re-baselining.

**Blast radius check:** as of this writing, no `.vir` file anywhere in
the repository (`tests/`, `examples/`, `arty_demo/`, `fomu_demo/`)
instantiates any of these cells — confirmed by grepping for
`of ice40::` and every `Sb*` ext mod name. The only existing
instantiation at all is the inline Rust string in
`cell_annotation_names_ext_mod_instantiation`
(`verilog/conversion.rs`), which only asserts on `submodule_name`/
`name`/`is_ext`, not exact parameter text, so it is unaffected. There
is currently nothing to re-baseline.


## Implementation

This is a single, mechanical phase: apply the `@parameter(...)` blocks
above to `lib/ice40.vir`, in place, above each affected `ext mod`'s
existing `@cell(...)` annotation, and perform the doc-comment cleanup
listed above. No Rust code changes beyond whatever `PARAMETERS.md`
already implements (this plan produces no new mechanism, only data).

Suggested order, so the diff stays easy to review:
1. `SbLut4`.
2. `SbIo`, `SbGbIo`.
3. The four `SbRam40_4k*` variants.
4. `SbPll40Core`, `SbPll40Pad`, `SbPll40_2Pad`, `SbPll40_2fCore`,
   `SbPll40_2fPad`.
5. `SbHfosc`.
6. `SbRgbaDrv`, `SbRgbDrv`.
7. `SbI2c`, `SbSpi`.
8. `SbMac16`.
9. File-header and per-cell doc-comment cleanup.


## Acceptance criteria

- `make test` passes unchanged (per the blast-radius check above, no
  existing test should need modification).
- A new unit test (alongside Phase 4 of `PARAMETERS.md`) instantiating
  one migrated cell (e.g. `SbPll40Pad`) with no `@parameter`
  annotations at the call site asserts that the emitted
  `verilog::Submodule.parameters` contains exactly that cell's 14
  declared defaults, in declaration order.
- A second case instantiating the same cell overriding one parameter
  (e.g. `@parameter("DIVR", 4)`) asserts the merged list has `DIVR`
  overridden and the other 13 at their declared defaults.
- Every cell's parameter count/names/kinds in `lib/ice40.vir` matches
  this document, which matches the installed Yosys's
  `share/yosys/ice40/cells_sim.v` at the time of writing (0.56) —
  re-verify against whatever Yosys version is current at
  implementation time.


## Out of scope

- `SB_RAM40_4K*`'s `INIT_0`..`INIT_F` (256 bits each) — blocked on
  widening `WordValue` (tracked in `NOTES.md`).
- `SB_IO_I3C`, `SB_IO_OD`, `SB_FILTER_50NS` — present in `cells_sim.v`
  but lib/ice40.vir has no `ext mod` for them at all yet; adding those
  wrappers (with their parameters) is a separate, pre-existing gap
  unrelated to this plan.
- `ICESTORM_LC`, `ICESTORM_RAM` — Yosys-internal post-techmap
  primitives, never instantiated directly by user designs.
