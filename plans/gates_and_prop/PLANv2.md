# Plan: gates_and_prop (v2) — the VCD→VCD Delay Transform

**v2 supersedes `PLAN.md` (v1).** Per workspace rules, the highest version
suffix is canonical, and `CONCEPT.md` still takes precedence over any plan
where they diverge.

**v2 abandons v1's architecture entirely.** The simulator no longer knows
propagation delays exist. Instead, a standalone **VCD→VCD transform**
re-timestamps a finished, zero-delay trace after the fact, using a static
per-signal cost model. Every piece of v1 implementation inside the
simulator is deleted (see *Removals* below).

This document is self-contained: it does not assume the reader has seen
v1.

## Why the architecture changed

The delays exist for education: waveforms where outputs visibly lag the
logic that produced them teach that gates are not instantaneous, that
ripple-carry adders are deep, and that registered outputs sample *before*
the logic settles. They are not a timing analysis — real timing questions
belong to STA with real library data, and a uniform per-gate model with no
process, voltage, or routing information must never be mistaken for one.

Given that purpose, live stamping inside the simulator was the wrong
home, for reasons the v1 implementation demonstrated concretely:

* **The engine paid for the cosmetics.** Stamping as changes happen means
  events arrive out of order (a shallow signal's delayed change can be
  overtaken by a later-settling deeper one), so the VCD writer needed a
  reorder buffer whose flush rule was tied to the scheduler's next wake —
  the single most intricate piece of the whole feature, existing only to
  restore a property (monotonic timestamps) that is trivial when all
  events are known at once.
* **The sim carried model plumbing** (a flag, unit state, clock
  bookkeeping, eager panic checks) that has nothing to do with simulation.
* **Calibration churn was expensive.** Every visibility tweak (scale the
  table, re-weight wiring, change the unit) touched simulator code and
  re-simulation. In a transform, re-tuning is a re-run of a millisecond
  post-pass on an existing trace.
* **The transform is reusable** on traces from any simulator, including
  external tools (iverilog), since its input is a plain VCD plus a
  sidecar of per-signal costs.

## Plan split

The original idea bundled three models. They are now separate plans:

| Plan | Contents | Depends on |
|---|---|---|
| `plans/prop_cost/` | per-node delay tables, source rules, per-signal pole computation, `Db` query + JSON export | — |
| `plans/gate_count/` | per-node gate/flop counts, aggregation, `vir gates` report | — |
| this plan | the VCD→VCD transform | `prop_cost` |

This document specifies only the transform; the cost model's tables and
rules live in `plans/prop_cost/PLAN.md` and are referenced here as an
interface.

## Removals (v1 implementation)

The current tree contains the discarded live-stamping implementation.
This plan removes:

* `virdant/virdant/src/sim/delay.rs` (deleted; its table *content* is
  restated in `plans/prop_cost/PLAN.md` and reimplemented there as a
  `Db` query, not simulator code).
* The `poles` / `builtin_ext_mods` fields and computations on
  `sim::circuit::Circuit`.
* On `sim::sim::Sim`: `propagation`, `clock_period_ps`,
  `prop_unit_override_ps`, `with_propagation`, `with_propagation_unit_ps`,
  `propagation`, `prop_offset_ps`, `prop_unit_ps`,
  `check_pole_overflow`, and the propagation-gated marker firing in the
  run loop.
* `sim::scheduler`: revert `pop_after_delay` to the original
  `next_after_delay` shape.
* `sim::vcd`: restore the simple immediate writer (no `Emitter`, no
  buffering, no `Drop` tail flush, no per-change propagation branch).
* `vir/src/poke`: remove the `prop` command (ast variant, lexer token,
  grammar rules, command arm, help text, completion entry) and the
  `prop on` line from the docs' `run.poke` scripts.
* `virdant/tests/prop_delay.rs`: deleted; its *behavioral* assertions are
  re-homed — pole expectations to `prop_cost`'s tests, VCD stamping
  expectations to the transform's tests.
* The docs guide figures' `:tick: 50000` option (added for the old
  stamping) — kept only if the transformed traces still need it (they
  will, for the same rendering-granularity reason; see *Docs pipeline*).

## Architecture

```
   elaboration ──► prop_cost query (per-signal pole, in units)
                        │
   zero-delay sim ──► plain VCD ──► vir vcd-delay ──► delayed VCD ──► figures
```

* The simulator is exactly what it was before v1: zero-delay settlement,
  a simple VCD writer, no flags. A trace it produces is an honest
  record of *what* changed and *when it settled*.
* The transform answers *when it would have become visible*: each
  signal's change at time `t` is re-stamped to `t + pole(signal) · unit`,
  where `pole` comes from the cost model and `unit` converts delay units
  to picoseconds.
* Nothing about the simulation result changes; only the trace's
  timestamps move. The transform never runs during simulation, so it
  costs the engine nothing in any mode.

## CLI

```
vir vcd-delay <in.vcd> <out.vcd> [options]

  --period-ps P     the clock period the trace's timescale represents
                    (default 1,000,000 — the virt platform's 1 MHz)
  --unit-ps U       explicit delay-unit size in ps; overrides --period-ps
  --clock PATH      path of the clock signal (default: the design's
                    unique Clock-typed top-level port)
  --poles FILE      standalone mode: read per-signal poles from JSON
                    instead of resolving the design from the project
  --strict          timing-check violations are a hard error instead of
                    a warning
```

Implemented as a companion binary `vir-vcd-delay` (same
companion-binary mechanism as `vir-poke`), living in
`vir/src/vcd_delay/`, dispatched as `vir vcd-delay` from
`vir/src/vir/cli.rs`.

## Inputs

### The VCD

A reader for the subset of VCD that this project (and iverilog) emits:

* header: `$timescale`, `$scope module`, `$var wire`, `$upscope`,
  `$enddefinitions`;
* `$dumpvars … $end` initial snapshot;
* timestamps `#<ps>`;
* value changes: 1-bit scalar (`0`/`1`, pass `x`/`z` through verbatim)
  and vector (`b<bits> <id>`, `r` too);
* anything unrecognized is skipped with a warning, never fatal — the
  reader must tolerate foreign writers' extra constructs (`$date`,
  `$version`, `$dumpall`, `$vcdclose`…).

The reader lives in `vir/src/vcd_delay/vcd.rs` (tool-local; the
`virdant` crate only writes VCDs today, and parsing is this tool's
concern).

### The poles

Two modes:

1. **Project mode (default):** the tool runs inside a Virdant project
   (like `vir poke`), loads the `Db` for the project, and maps every VCD
   `$var` to an elaborated signal by joining its scope segments and name
   into a dotted path (`top` + scope + name), then resolves it. The
   signal's pole comes from `db.get_prop_cost(top)`. Var width is
   checked against the signal's width; mismatches warn and the var is
   treated as unknown.
2. **Standalone mode (`--poles FILE`):** a JSON sidecar
   `{"signals": {"top.inp": 1, …}, "max_pole": N}` produced by
   `PropCost::to_json`. This decorates traces from *any* simulator —
   e.g. an iverilog run of compiled Virdant output — without a project
   at hand.

Signals with no pole (not in the design / sidecar) get offset 0 and are
listed once in a warning; their changes pass through unchanged.

The pole model's cone-stopping semantics are the same as the existing
`vir cones` analysis (see `plans/prop_cost/PLAN.md`, "Relationship to the
existing `analysis::cones`") — a user comparing `vir cones` output with a
delayed trace must never see the two disagree about what feeds what.

## Algorithm

1. **Parse the header.** Build `id → (dotted path, width)`. Record the
   `$timescale` for verbatim passthrough.
2. **Resolve poles.** Project or standalone mode as above.
3. **Parse the body** into an event list in file order, assigning each
   event a global sequence number:
   * `Change { time, seq, id, value }`
   * `Marker { time, seq }` — a `#t` with no changes (pure time
     advancement; it exists so trace renderers don't truncate the tail).
4. **Re-stamp:** every change to signal `s` becomes `t' = t +
   pole(s) · unit_ps`. Per-signal order is preserved automatically
   (adding a constant is monotone per signal); cross-signal ordering is
   fixed by the global sort below. `Marker`s keep their original time.
5. **Timing check** (if a clock is identified — `--clock`, else the
   unique `Clock`-typed top-level port in project mode, else none in
   standalone mode): walk the clock's changes to find its rising edges
   (0→1). For every re-stamped change with `t' ≥ next_rise(t)`, record a
   violation (signal, original time, stamped time). Default: one warning
   summarizing the count and the first violation — a quarter-scale
   educational model overflowing a cycle means the *visibility knobs*
   are mis-set, not that the design fails timing. `--strict`: exit with
   an error instead.
6. **Sort** all events (changes and markers) by `(t', seq)` — stable, so
   same-stamp events keep file order, which preserves per-signal
   settlement order.
7. **Emit:**
   * header verbatim, plus one `$comment` recording provenance:
     `delayed by vir vcd-delay (unit_ps=50000, model=prop_cost)`;
   * `#0` + `$dumpvars` snapshot verbatim (the snapshot is the state at
     trace start; it is not delayed);
   * the sorted events: for each new timestamp not equal to the last
     written one, `#t'`; then the change lines verbatim (same id codes,
     same bit strings — only timestamps move).

Determinism: the output is a pure function of the inputs; no randomness
anywhere.

## Scale defaults

Delay units are abstract; the transform picks the ps scale. The
educational default is **`unit_ps = period_ps / 20`** (with
`period_ps = 1,000,000` → 50,000 ps): a direct connection visibly lags
by 5% of a cycle, a register fanout (clock-to-Q + wire = 2 units) by
10%, an 8-bit adder by 1.6 cycles — dramatic enough to teach, and
`--unit-ps`/`--period-ps` retune it without re-simulating. (A
"realistic" scale would be closer to period/400; the educational default
is deliberately louder.) The timing check above catches the case where
loudness pushes changes past a clock edge.

## Edge cases

* **Same signal, same timestamp, several changes** (two pokes settling
  in one tick): the sort's `seq` tiebreaker keeps their settlement
  order, so the final value wins in file order.
* **A delayed change crossing a marker:** markers sort at their original
  time. A change originally at 1.4M re-stamped to 1.9M now appears after
  a 1.5M marker — correct: the trace simply says time reached 1.5M with
  nothing changing, then the change at 1.9M. Monotonicity is guaranteed
  by the global sort.
* **Changes at the trace's end** re-stamped beyond the last marker:
  they simply extend the timeline; the final `#t'` is the last emitted
  timestamp.
* **`$dumpvars` values vs delayed changes:** the snapshot shows
  settle-time state at `#0`; a signal whose first change re-stamps to
  `t > 0` will re-announce its (possibly identical) value later. This is
  harmless and honest — the delayed transition is the *visible* one.
* **Zero-pole signals** (clocks, constants): unchanged timestamps.
* **Empty traces / header-only files:** pass through with a warning.

## Implementation sketch

```
vir/src/vcd_delay/
  main.rs        CLI + companion-binary entry, project loading
  vcd.rs         the VCD subset reader
  transform.rs   resolve poles → re-stamp → check → sort → emit
vir/src/vir/cli.rs    dispatch `vcd-delay`
```

`PropCost::to_json` (from `plans/prop_cost/`) supplies the standalone
mode; the query itself supplies project mode.

## Tests

* **Reader unit tests:** round-trip this project's own writer output;
  parse a hand-written iverilog-style VCD (with `$date`, `$dumpall`,
  `r`-values) and assert tolerance.
* **Transform golden tests** (`kind = "golden"`): small hand-written
  traces + tiny designs with known poles → exact expected delayed VCDs.
  Cases: register fanout (clk-to-Q + wire), pure combinational cone,
  clock passthrough, unmatched signals, marker crossing, same-tick
  multiple changes.
* **Monotonicity property:** for every fixture, assert the output's `#`
  sequence is non-decreasing and per-signal changes preserve order.
* **Timing check tests:** a fixture whose re-stamped changes cross a
  clock edge — warns by default, errors under `--strict`.
* **Standalone mode test:** a foreign VCD + a hand-written poles JSON.

## Docs pipeline

* The guide examples' `run.poke` scripts revert to plain traces (drop
  `prop on`).
* A small checked-in step (one line in the docs example regeneration
  flow) runs `vir vcd-delay trace.vcd trace-delayed.vcd` after the poke
  session; the guide `.rst` figures point at the delayed trace.
* The `:tick: 50000` rendering option stays on those two figures: the
  transformed offsets (50k, 100k ps) are invisible on the default
  500k-ps render grid, exactly as before.

## Future work

* **Per-bit poles** — the cost model's shape leaves room; the transform
  would then re-stamp individual bits of a vector. Not planned yet.
* **Other cost models behind the same interface** (e.g. technology-flavored
  numbers) — the transform only consumes "per-signal offset", so swapping
  models is a flag, not a rewrite.
* **Live-view integration** — if a real-time waveform viewer is ever
  attached to a running sim, it can invoke the same transform
  incrementally; deliberately deferred.

## Open questions

* Command name: `vir vcd-delay` (proposed) vs `vir delay-vcd` vs making it
  a flag on an existing command. Leaning `vcd-delay` for verb-noun
  consistency with `vcd`-adjacent tooling.
* Whether the timing check should also consider *falling* edges for
  multi-phase clocks (virt has none today).
* Whether `vir vcd-delay` should also accept `-` stdin/stdout for
  pipeability. Cheap to add; unspecified until wanted.
