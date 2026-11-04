# Plan: sim_revamp

Date:   2026-10-26
Status: Implementing

## Source

`CONCEPT.md` (this directory). Key decisions:

1. **No X/Z.** Uninitialized registers and top-level incoming ports are
   initialized to a value chosen by a configurable `InitStrategy` (default:
   `Random`, the only option for now). X-propagation logic is deleted.
2. **Bit-pattern values.** All values are tracked as plain bit patterns,
   tagged with their (semantic) `Type`. Gate logic decodes the bits on
   demand. Constructors/unions/enums/structs are encoded to bits via the
   type bit-layout (`sim/layout.rs`, shared by eval, state, VCD, and the
   board). Printing goes through the tag: `Bit` "0" prints as `false`; a
   pattern that is not a valid inhabitant of its type prints `<invalid>`.
3. **Clock.** The board runs at the frequency declared in the platform
   (`@period_ns`). The simulation is paced to real time ("FPS lock");
   `--fps-no-cap` runs uncapped.
4. **Delays and events.** No new investment: the existing scheduler
   (`at`/`after`/`on_change`/`on_clock`) stays as-is.
5. **Feature coverage.** The sim supports all language features except
   tristate buffers: `when`, `match` (ctor/word/bit patterns), ctors,
   enumerants, struct literals, indexing (static/dynamic/range),
   `word{}` concat, `zext`/`sext`/`trunc`, `any`/`all`, primitives
   (`mux`, `cast`, `asclock`, ...).

## Work items

### Sim engine (`virdant/virdant/src/sim/`)

- **`bits.rs`** — `BitPat`: arbitrary-width 2-state pattern with
  zext/sext/trunc, slice, concat, shifts, wrapping add/sub/neg (top-limb
  masked!), bitwise ops, unsigned compare, xorshift random.
- **`layout.rs`** — `TypeLayoutIndex`: bit layout per type instantiation
  (tag/payload offsets), ctor encode/decode, validity, typed display.
- **`eval.rs`** — `Value { typ, bits }`; every eval node tags its result
  with its static type. X-poisoning branches deleted; `Struct` literals
  evaluate.
- **`state.rs`** — `InitStrategy` initialization; idempotent `set_val` /
  `transfer` (same-value writes are no-ops so change detection keeps the
  net transition).
- **`sim.rs`** — full-network `flow_all` at construction (no stale random
  values); `on_clock` watchers self-track their last-seen value (immune to
  nested-dispatch `commit_change` poisoning).
- **`vcd.rs`** — raw bit-pattern dumps; scope tree from the elaboration.
- **`builtins.rs`** — `rf::Rf`, `mem::Mem`, `mem::Csrs` ext-module models
  implemented through the public Sim API (NBA-sampled write ports,
  combinational read ports, borrow-safe).

### Elaboration (`analysis/elaboration.rs`)

- Socket channels become elaborated components; `:=:` connections resolve
  to (source → sink) aliases; parent-driven channels and bare-reference
  drivers resolve via explicit `driver_scope`; platform-`default` port
  synthesis; no self-aliases.

### Board (`virt.rs` + `vir run`)

- `vir run` asserts the top module is `for virt::Virt`, then runs the
  Rust board: UART 115200 8N1 (exact rational tick timing), 16 MiB SPI
  flash stepped on every clock edge (like the C++ harness), `exit` port,
  `--flash/--cycles/--fps-no-cap/--trace[=F]` flags, real-time pacing,
  raw-mode terminal, VCD trace.

## Status / verification

- `vir run` on `virdant-riscv`'s echo demo: boots the design from SPI
  flash over the simulated bus, prints `BOOTING...` / `READY`, and
  **echoes** an input byte (end-to-end UART RX → CPU → UART TX).
- **Dhrystone: PASSED end-to-end.** Fixed a `BitPat::not` bug (`and`
  instead of `xor`) that made the ALU's `SUB` (built as `a + ~b + 1`)
  compute `a + b + 1`; division traces matched the Verilator golden
  bit-for-bit, and the full benchmark (10000 runs, ~45M cycles, ~35 min
  wall) now terminates through the design's `exit` port with output
  **identical** to the Verilator golden — including
  `Cycles for one run: 1554` and `DMIPS: 17.57`.
- Echo demo: full UART round-trip (input byte → design → echo out).
- VCD tracing verified (26 MB trace of the boot+echo run).
- All 44 lib tests pass; debug, release, and vir-lsp feature builds
  compile clean.

## Non-goals

- Tristate buffers; VGA (`virt::VirtVga`); delays/events semantics changes;
  the Verilator/Icarus paths.
