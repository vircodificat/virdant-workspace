# VM Build Plan

The implementation plan for the clean-slate VM described by the design
documents in this directory.
The design decisions are in [`DECISIONS.md`](DECISIONS.md); the rationale
lives in `01`–`10`.
This file is the ordering: what gets built when, what gates each step, and
what is explicitly not built yet.

## Phase 0 — Delete the old simulator

**"Delete first, then build."** ([`03-architecture.md`](03-architecture.md))

1. Remove the event-driven simulator from `virdant/src/sim/`:
   the scheduler, the picosecond time queue, callback attachment,
   `state.rs`, `eval.rs`, `expr.rs`, `sim.rs`.
2. Move the reusables into their new homes before/while deleting:
   - `bits.rs` — bit-pattern machinery (survives nearly as-is),
   - `layout.rs` — type-layout machinery (survives nearly as-is),
   - `vcd.rs` — VCD writer core (rewire to phase-boundary hooks),
   - `builtins.rs` — becomes the first `ExtMod` implementations (v2, but
     keep the code for then).
3. Accept the temporarily red tree: `vir run`, the virt board, and the
   `kind = "run"` suite do not work until Phase 4.

**Gate:** the tree compiles without the old sim; nothing references it.

## Phase 1 — Program structure + assembly language

The interpreter's fuel. ([`05-ir.md`](05-ir.md))

1. Define the `Program` structure — **serialization-shaped from day one**:
   flat data, indices not pointers, no host closures, no cycles, a version
   field.
2. Write the LALRPOP grammar for the assembly notation:
   `.types` (FQN → explicit width, `builtin::`-qualified),
   `.signals` (`sig` / `reg`, multi-name aliasing),
   blocks (`^name(args):`), terminators (`branch!` / `call!` / `ret!`),
   instructions (one per line, destinations left, literals
   `0w8` / `0b101w8` / `0x2Aw8`, slices `k..r`).
3. Load-time validation (eager, assembler-side):
   duplicate signal names, undeclared references, `stage` to a `sig`,
   literal too wide, width > 64.
4. The serializer: pretty-print a `Program` back to assembly text.
   Golden round-trip tests (parse → print → byte-identical) come with it.

**Gate:** parse/print round-trip tests pass; malformed programs produce
eager, well-located errors.

## Phase 2 — The interpreter

The v1 deliverable. ([`04-machine.md`](04-machine.md),
[`06-instruction-set.md`](06-instruction-set.md))

1. Machine core: slot state, register cur/next pairs, staging, atomic
   commit after `^tick()`.
2. Execution: `^settle()` and `^tick()` sections (neither required; running
   a missing entry panics), the global dirty flag, forward tail-calls only.
3. Instruction handlers, **width-specialized at load time** (one opcode
   set; fast-path slots take single-`u64` handlers): data movement,
   state access, bitwise, arithmetic (`add`/`sub` — no `mul`),
   shifts (out-of-range hits `todo!()`), comparisons, `mux`,
   `slice` (read-only) / n-ary `concat`, `tblread`.
4. The embedder API ([`07-api.md`](07-api.md)), exactly as recorded:
   `lock_mut()` staging scope (`poke` staged, `peek` shows the
   pre-settled world by design, **drop of the writer lock settles once**),
   reader `lock()` for the settled view, `tick(clk)` excluded while any
   lock is held (the borrow checker is the rule), IDs via
   `id_of(name)`, the side table, tracing at phase boundaries, and the
   poisoned-state error model.
   No `settle()` or `probe()` methods exist — settle is lock-drop-triggered
   and observation is `peek`.

**Gate:** the worked-example counter program
([`09-worked-example.md`](09-worked-example.md)) runs, driven by an
embedder, with correct cycle-level behavior.

## Phase 3 — Hand-compiled examples: counter + gcd

**"Do both"** — the feel-for-it evaluation. ([`03-architecture.md`](03-architecture.md))

1. **Counter** (already drafted in [`09-worked-example.md`](09-worked-example.md)):
   straight-line settle, a two-arm `branch!` diamond, slice/load/stage.
2. **GCD** (from `tests/run/gcd`): a three-state FSM via `match`, multi-arm
   `branch!` with block parameters, `mux` chains, comparisons, a 16-bit
   temporary-heavy datapath.
3. Hand-compile both by hand (with AI assistance), run them, and evaluate
   the assembly approach honestly: can a human/AI write correct programs?
   Is the notation readable? Do the fusion opportunities show up?

**Gate:** both programs run with correct behavior; the evaluation notes
feed back into the notation and the future lowering.

## Phase 4 — Bring the suite back

**The sole gate: 197 `kind = "run"` tests exit 0.** ([`03-architecture.md`](03-architecture.md))

1. Build the compiler lowering (flatten → schedule → program) against the
   same `Program` structure the assembler fills.
2. Wire `vir run` and the virt board onto the new VM.
3. Run the suite to green.

**Gate:** `tests/run/` fully passing again — the new VM is the simulator.

## Phase 5 and beyond (not yet planned in detail)

- Ext mods — the `ExtMod` trait, `exteval`, the `.ext` section (**VM v2**;
  [`10-deferred.md`](10-deferred.md)).
- The program artifact file format (serialization-shaped already, so this
  is a writer+reader when it comes).
- Fusion pass tuning; the probe-ability/keep-list policy change.
- Save/restore; watchpoints; multi-clock entries.
- Native-code generation — the endgame for 1M–20M cycles/s
  ([`01-goals.md`](01-goals.md)).

## Sequencing rule

Each phase builds on the last; nothing in a later phase is needed to make
an earlier phase's gate meaningful.
The interpreter (Phases 1–2) is useful and testable without any compiler;
the suite gate (Phase 4) is the first moment the whole system is exercised
together.
