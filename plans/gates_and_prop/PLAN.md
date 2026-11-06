# Plan: gates_and_prop

Date:   2026-11-04
Status: In Planning

See `CONCEPT.md` for the original idea writeup.

## Motivation

Virdant's sim currently settles combinationally at zero delay: `flow_all`
propagates everything in delta cycles, and the VCD dumper stamps every
value change at the simulated time at which it settled. Waveforms therefore
show combinational logic as instantaneous.

This plan adds a **propagation delay model**: every signal is assigned a
static delay (computed from the longest path — the longest pole — through
its backwards cone, using a per-operator gate-delay table), and the VCD
dump stamps each signal's value changes at `settlement_time + pole_delay`.

**Delays are cosmetic only** [approved]. The simulation semantics are
unchanged: settlement remains zero-delay and instantaneous; only the
*timestamps written to the waveform* are offset. Simulation results, watcher
firing, and callback scheduling are identical with the feature on or off.

The feature is opt-in via `sim.with_propagation(true)` [approved].

## Decisions (from planning interview)

| # | Topic | Decision |
|---|-------|----------|
| 1 | Semantics | Waveform presentation only; zero-delay settlement unchanged [approved] |
| 2 | Backwards cone | Stops at registers, top-level inputs, and constants (all delay 0); cone walks combinational components only [approved] |
| 3 | Pole overflow | Panic if a stamped change would land at or past the next clock edge; delays should be orders of magnitude below the clock in practice [approved] |
| 4 | Time unit | 1 gate-delay unit = first attached clock period / 100 [approved] |
| 5 | Multiple clocks | First attached clock defines the unit; with propagation on, a second clock with a different period panics [approved] |
| 6 | Flag | `sim.with_propagation(true)` [approved] |
| 7 | No clock attached | Fallback: 1 unit = 1 ps until a clock is attached [approved] |
| 8 | Operator delays | Per the table below [approved] |
| 9 | Overflow check | Eager static check (max pole vs clock period), not per-change [approved] |
| 10 | User writes | `SimLock::set` writes are sources: stamped at `t + 0`; downstream cone delays apply from there [approved] |
| 11 | Builtin components | Panic for now if `rf::Rf` / `mem::Mem` / `mem::Csrs` are present with propagation on [approved] |
| 12 | VCD monotonicity | Buffer change events and emit them in timestamp order [approved] |
| 13 | Scope | Core only: model + checks + VCD reorder + tests. No introspection API, no separate doc [approved] |

## The Delay Model

### Units

Gate delays are counted in abstract **units**, converted to picoseconds at
stamp time:

```
unit_ps = first_clock.period_ps / 100      // once a clock is attached
unit_ps = 1                                // fallback before any clock
```

With the fallback in force, the eager overflow check (below) runs against
1-ps units at `run()` start.

### Sources

Sources are where the backwards cone stops — they carry fixed delays
(revised after first implementation; originally all 0):

* **Register Q outputs** — a register's output changes at its clock edge and
  is the root of a new combinational epoch. (`c <= d` stages the D pin, so
  the *D-side* expression carries cone delay, but Q is a source.)  Revised:
  Q carries a **clock-to-Q delay of 1 unit** after the edge, and that delay
  propagates into downstream cones (a path from a clock edge is clk-to-Q
  plus downstream logic).  A user poke of a register charges the same pin.
* **Top-level incoming ports** — driven only by user code; revised: user
  writes cost **1 unit** (`USER_WRITE`), so pokes no longer appear
  instantaneous and the delay propagates downstream.
* **Clock-typed sources** — exempt: a `Clock` port stamps at its true time,
  because it is the timing reference its own edges are measured from
  (charging it would shift the rendered edge by the same amount as the
  clk-to-Q delay it anchors, hiding both).
* **Constants** (literals, `initial` values) — 0; they never change.
* **Socket aliases** (`:=:`) and bare-reference drivers — a copy of the
  peer's value; revised: the wire join costs **1 unit** on top of the
  peer's pole (wiring is never free).

### Operator gate delays

All delays in units; `w` = operand width in bits, `n` = arm/element count.
Bit-parallel operations do not deepen with width; carry chains and
reduction trees do.

**Binary operators:**

| Op | Hardware picture | Delay |
|---|---|---|
| `And`, `Or`, `Xor` | `w` parallel 1-bit gates | `1` |
| `LogicalAnd`, `LogicalOr`, `LogicalXor` | one gate on the (1-bit) operands | `1` (revised: eval reads bit 0 only — a single gate, not a reduction) |
| `Eq`, `Neq` | per-bit XNOR + AND reduction tree | `⌈log₂ w⌉ + 1` |
| `Lt`, `Lte`, `Gt`, `Gte` | ripple comparator | `2w` |
| `Add`, `Sub` | ripple-carry / borrow adder | `2w` |
| `AddCarry`, `SubBorrow` | ripple adder + carry-out | `2w + 1` |

**Unary operators:**

| Op | Hardware picture | Delay |
|---|---|---|
| `Not`, `Inv` | `w` parallel inverters | `1` |
| `Neg` | invert + add 1 | `2w + 1` |

**Other expression payloads:**

| Construct | Hardware picture | Delay |
|---|---|---|
| `If` (`when`) | 2:1 mux | `2` |
| `Match` | n-way mux tree | `2 · ⌈log₂ n⌉` |
| `Any`, `All` | reduction tree | `⌈log₂ w⌉` |
| `Index` (constant) | wire | `0` |
| `IndexRange` (slice) | wire | `0` |
| `IndexDyn` | decoder + mux tree | `2 · w_addr` |
| `Word`, `Zext`, `Sext`, `Trunc`, `Cast`, `As`, `AsClock` | wiring | `0` |
| `Ctor`, `Struct`, `Enumerant` | wiring | `0` |
| Literals, `Hole`, `Dontcare` | constants | `0` |
| `Reference` | wire | `0` (its pole comes from whatever drives it) |
| `Fn` (user functions) | inlined expression trees | composed automatically by the cone walk; no separate rule |

### Pole computation

For every driven component, compute the **pole delay** statically at
`Circuit`-build time:

```
pole(expr) = op_delay(expr) + max(pole(child) for child references)
```

* A `Reference` to a **source** contributes 0.
* A `Reference` to another combinational component contributes that
  component's already-computed pole (memoized; the dataflow graph is a DAG,
  so evaluate in topological order — `sensitivities` gives the edges).
* The component's pole is the pole of its driver expression. Components
  without a driver expression (registers with Q-as-source role, ports) are
  sources with pole 0.
* The maximum pole over all components is retained for the eager overflow
  check.

The result is one `pole: Vec<u64>` (units), densely indexed by
`SignalId::index()`, stored on `Circuit` alongside the other precomputed
dataflow tables.

### Universal ÷4 (final revision)

After the clk-to-Q and user-write revisions, the whole model was
divided by 4 — implemented at the unit level, not the table, so the
integer unit granularity (wiring = 1) and all relative proportions
survive: the clocked unit is **period/400** (nominal table scale is
period/100), and the poke override is **TIMESTEP/20** (50,000 ps).
Every signal's stamped delay is exactly a quarter of the pre-revision
value, and the eager-overflow budget loosens 4× accordingly.

### Scaling (revision after first implementation)

Every gate delay above is **10× the original model**, and every
previously-zero wiring delay (direct connections, bit manipulation,
`word()`, padding, literals) is **1 unit** — wiring is never free.  In
`delay.rs` this is one wrapper: `node_delay = scale(raw)`, where
`scale(0) = 1` and `scale(d) = 10·d`.  Direct connections that the
elaboration resolves to *aliases* (no expression tree, e.g. bare
reference drivers and socket `:=:` copies) get the same +1 in the pole
walk, so no connection is free.

### Fallback-unit override (clockless drivers)

In clockless drivers (the poke tester) the unit would fall back to
1 ps, making even a scaled wire delay invisible against ticks ~10⁶ ps
wide.  Multiplying the *table* further cannot fix this without
shrinking the clocked timing budget by the same factor (the eager
check panics at `pole × period/100 ≥ period`).  Instead the fallback
unit is overridable — `Sim::with_propagation_unit_ps(unit_ps)` takes
precedence over the clock-derived / 1 ps fallback — and `vir poke`'s
`prop on` sets it to **a quarter of the poke tick timestep**
(250,000 ps): every wire costs a clearly visible quarter-cycle step,
gates and adders scale up proportionally, and poke (clockless) never
trips the overflow check.  Clocked sims are unaffected.

(Rendering note: the docs' ``vcd`` directive quantizes traces into
fixed ticks — default 500,000 ps, half a clock cycle, which puts every
edge on a tick boundary and hides any sub-cycle delay entirely.  The
guide figures that demonstrate propagation delay pass ``:tick:
50000`` for a 10× finer grid.)

## Flag Semantics and Panic Conditions

`sim.with_propagation(true)` [approved]:

1. **Builtin check** — panics immediately if the elaborated design contains
   `rf::Rf`, `mem::Mem`, or `mem::Csrs` components [approved]. Supporting
   them is future work.
2. **Clock rules** — the first attached clock's `period_ps` defines
   `unit_ps = period_ps / 100`. Attaching a second clock with a *different*
   period panics [approved]. With no clock attached, `unit_ps = 1` (ps)
   [approved].
3. **Eager overflow check** [approved] — when a clock is attached (or
   `with_propagation(true)` is called after one): if

   ```
   max_pole_units × unit_ps ≥ period_ps
   ```

   panic, naming the offending signal and its pole. Rationale: a stamped
   change at or past the next clock edge indicates the delay model is off
   by orders of magnitude; it should never happen in practice, so fail
   loudly rather than emit misleading waveforms [approved].

   (Implementation note: with no clock attached the check is skipped —
   there are no clock edges to overflow.)

With propagation off (default), none of this machinery engages: the VCD
stamps exactly as today.

## VCD Timestamp Reordering

VCD requires monotonically non-decreasing `#` timestamps. With per-signal
delays, signal A (pole 50) changing at t=100 stamps `#150` while signal B
(pole 2) changing at t=110 stamps `#112` — an out-of-order hazard the
current single-`last_time` dumper cannot express.

Design: **buffer and reorder** [approved].

* The dumper holds pending change events in a min-heap keyed by stamp time
  `(base_time + pole × unit_ps)`.
* Per-signal change watchers push events instead of writing directly.
* **Flush rule:** all future value changes have base time ≥ the next
  scheduled wake time `W` (next scheduler queue entry, or next attached
  clock edge — the scheduler exposes its next wake). Therefore an event
  with stamp ≤ `W` can be emitted safely. The dumper flushes such events
  before time advances past `W`.
* End-of-simulation (`flush()`) drains the remainder.
* **Time markers** (`advance_time` callbacks that stamp pure time
  advancement) must route through the same buffer so the `#` sequence stays
  monotonic.
* Two events for *different* signals sharing a stamp may be emitted in
  either order. Two events for the *same* signal: the newer value must be
  emitted after the older (natural with stable sort by `(stamp, seq)`).

When propagation is off, the dumper behaves exactly as today (no buffering).

## Implementation Sketch

* `virdant/virdant/src/sim/delay.rs` (new) — the delay table:
  `op_delay(expr_payload, ...) -> u64`, pole computation over `Circuit`'s
  driver expressions (topological, memoized), and the `pole` table.
* `virdant/virdant/src/sim/circuit.rs` — build and store
  `pole: Vec<u64>` + `max_pole: u64` at construction time.
* `virdant/virdant/src/sim/sim.rs` —
  * `with_propagation(bool)` flag + stored `unit_ps` state;
  * panic checks in `with_propagation`, `attach_clock`, and `run()`;
  * stamped-time plumbing for the VCD watcher path (watchers need the
    signal's pole and the current unit).
* `virdant/virdant/src/sim/scheduler.rs` — expose next-wake time
  (`next_wake_ps()`) for the dumper's flush rule.
* `virdant/virdant/src/sim/vcd.rs` — buffered, reordered emission
  (min-heap + flush rule + buffered time markers); unchanged behavior when
  propagation is off.

## Tests

* **Delay table unit tests** — each operator row above, including width
  scaling (`2w`, `⌈log₂ w⌉`, `2·⌈log₂ n⌉`, `2·w_addr`).
* **Pole computation** — hand-built moddefs exercising chained cones:
  reg → adder → gate → reg; verify per-signal poles and `max_pole`.
* **VCD stamping** — with propagation on and a clock attached, assert the
  VCD stamps at `t + pole × period/100`.
* **VCD ordering** — a design with mixed-depth cones producing the
  A-then-B out-of-order hazard; assert the emitted `#` sequence is
  monotonic and values land at the right stamps.
* **User writes** — callback `set` stamps at `t + 0`; downstream signals
  stamp at `t + pole`.
* **Panic tests**:
  * second clock with different period while propagation is on;
  * design whose max pole ≥ clock period (use the 1-ps fallback or a tiny
    clock to force it);
  * builtin components present with propagation on.
* **Off-by-default** — identical VCD output with propagation off vs. today.

## Future Work

* Builtin component delay models (`rf::Rf`, `mem::Mem`, `mem::Csrs`) —
  currently panics [approved].
* Introspection API (`sim.prop_delay_ps(sig)`) and a written delay-model
  doc [deferred, per scope decision].
* Finer-grained (bit-level) poles if component granularity proves too
  coarse in practice.
