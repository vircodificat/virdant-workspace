# Plan: prop_cost

Date:   2026-11-04
Status: In Planning

Static per-signal propagation-cost model for elaborated Virdant designs:
a gate-delay table plus a longest-path ("pole") computation, exposed as a
`Db` query. Spawned out of `plans/gates_and_prop` (see `PLANv2.md` there
for the consumer — a VCD→VCD delay transform). Source idea:
`plans/gates_and_prop/CONCEPT.md`.

## Motivation

The original gates_and_prop plan stamped propagation delays into
waveforms live, inside the simulator. That coupled a purely *analytical*
model (how deep is the logic behind this signal?) to the *simulation
engine*, and forced complex machinery (out-of-order VCD buffering) that
exists only because events arrive one at a time.

This plan splits the analytical half out: a pure, static, per-signal cost
model computed once over an elaborated design, with no simulator
involvement at all. It is educational tooling: it answers "how much logic
stands between the inputs/clock and this signal?", not "does this meet
timing?" (that is STA's job, with real library data; this model is
deliberately crude).

## Model

All costs are in abstract **delay units**. Consumers convert units to
picoseconds with their own scale (the VCD transform in
`plans/gates_and_prop/PLANv2.md` does; a future report tool might choose
another). Keeping units abstract here keeps the model reusable.

### Per-node gate delays

`w` = operand width in bits, `n` = match arm count, `a` = dynamic-index
address width. Bit-parallel operations do not deepen with width; carry
chains and reduction trees do.

**Binary operators**

| Op | Hardware picture | Delay |
|---|---|---|
| `And`, `Or`, `Xor` | `w` parallel 1-bit gates | `10` |
| `LogicalAnd`, `LogicalOr`, `LogicalXor` | one gate on the (1-bit) operands | `10` |
| `Eq`, `Neq` | per-bit XNOR + AND reduction tree | `10 · (⌈log₂ w⌉ + 1)` |
| `Lt`, `Lte`, `Gt`, `Gte` | ripple comparator | `20w` |
| `Add`, `Sub` | ripple-carry / borrow adder | `20w` |
| `AddCarry`, `SubBorrow` | ripple adder + carry-out | `10 · (2w + 1)` |

**Unary operators**

| Op | Hardware picture | Delay |
|---|---|---|
| `Not`, `Inv` | `w` parallel inverters / one gate | `10` |
| `Neg` | invert + add 1 | `10 · (2w + 1)` |

**Other expression constructs**

| Construct | Hardware picture | Delay |
|---|---|---|
| `If` (`when`, `mux`) | 2:1 mux | `20` |
| `Match` | n-way mux tree | `20 · ⌈log₂ n⌉` |
| `Any`, `All` | reduction tree | `10 · ⌈log₂ w⌉` |
| `IndexDyn` | decoder + mux tree | `20 · a` |
| `Index`, `IndexRange`, `Word`, `Zext`, `Sext`, `Trunc`, `Cast`, `As`, `AsClock` | wiring / bit manipulation | `1` |
| `Ctor`, `Struct`, `Enumerant` | aggregate wiring | `1` |
| `Reference`, `Paren` | direct connection | `1` |
| Literals, `Hole`, `Dontcare` | constants | `1` |
| `Method`, `Fn` | cannot appear in elaborated drivers today; wired through | `1` |

Wiring is never free (every connection costs at least 1 unit); the ×10
scaling of gate delays relative to wiring is a *visibility* calibration
for educational waveforms, not a hardware claim.

### Sources (base delays)

The backwards cone of a signal stops at sources, which contribute a fixed
base delay of their own:

| Source class | Base delay | Rationale |
|---|---|---|
| Register Q output | `1` (clock-to-Q) | a register's output changes one unit after the clock edge that latches it; the delay propagates into downstream cones |
| Top-level incoming port | `1` (user write) | ports are driven by user code (pokes, testbench writes); changes cost one unit and propagate downstream |
| `Clock`-typed port | `0` | the clock is the timing reference its own edges are measured from; delaying it would shift every measurement with it |
| Constant (`initial`, literal-driven) | `0` | constants never change, so their delay is unobservable |
| Submodule incoming port | via its driver | parent connections are ordinary driven signals (alias or expression) |

### Pole definition

The **pole** of a signal is the maximum delay from any source to that
signal — the longest path through its backwards cone:

```
pole(source)          = base(source)            (table above)
pole(alias sink)      = pole(peer) + 1          (a connection hop)
pole(driven signal)   = node_delay(driver expr)
                        + max(pole(input) for each referenced signal)
```

`node_delay(expr)` is the construct's own delay plus the largest child
pole; pattern-bound variables in `match` arms inherit
`pole(subject) + match_delay` (the bound value emerges after the mux
tree). The dataflow graph is a DAG; evaluation is memoized in topological
order, with a loud panic (naming the signal) if a combinational loop is
ever encountered — that would be a compiler bug, not a user error.

Example (`plans/gates_and_prop/CONCEPT.md`'s buffer): `buffer` is a
register → pole 1; `out := buffer` is a direct connection off a register
→ pole 2. An 8-bit `s := a + b` → `20·8 + max(1, 1)` = 161.

## Relationship to the existing `analysis::cones`

`analysis::cones` (`vir cones`) already computes combinational cones with
the canonical stopping semantics: registers and design-boundary ports are
endpoints; wires, ports, and socket channels trace through. It answers a
*different* question — cone **membership** (which endpoints feed a
signal), keyed by registers and boundary ports — while the pole needs
**depth** (the longest weighted path through the cone's interior), keyed
by every driven signal. The pole computation therefore cannot be built on
`Cones` as it exists, but it must agree with it:

* The source-stopping rules in this plan (registers are endpoints;
  boundary `incoming` ports terminate backward cones; socket channels
  trace through) are **defined by reference to `Cones`' semantics**, not
  restated independently.
* A cross-check test pins the two together: for a corpus of designs, every
  endpoint of a signal's `Cones` backward cone must be one of the sources
  contributing to its pole, and vice versa. If the two analyses ever
  disagree about where a cone stops (e.g. how `bus`/`net` signals from the
  tristate plan trace), this test fails loudly instead of producing poles
  that contradict `vir cones` output.

## Implementation

A new cached `Db` query:

```rust
// src/analysis/prop_cost.rs
pub struct PropCost {
    poles: Vec<u64>,                 // densely indexed by SignalId::index()
    max_pole: u64,
    max_pole_signal: Option<SignalId>,
}
impl PropCost {
    pub fn pole(&self, signal: SignalId) -> u64;
    pub fn max_pole(&self) -> u64;
    pub fn to_json(&self, elaboration: &Elaboration) -> String;  // path-keyed
}

#[query]
pub fn get_prop_cost(db: &Db, top: SymbolId) -> Arc<PropCost>;
```

Construction mirrors what the simulator's `Circuit::new` does today for
driver expressions, minus the simulator:

* iterate `get_elaboration(top)` components;
* for each driven component, convert its `analysis::drivers::Driver` to a
  typed expression via the existing `sim::expr::driver_to_expr` (reused as
  a plain conversion — nothing simulation-specific about it);
* resolve references to `SignalId`s exactly as
  `compute_resolved_referents` does (the conversion and resolution helpers
  are salvaged as-is from the current tree; only their *callers* in the
  simulator go away);
* classify sources: `is_reg()` → clock-to-Q; no driver and no alias →
  user write (or 0 for `Clock`-typed); alias sinks → peer + 1;
* walk each expression with the memoized pole rule above.

### Hardening requirements (lessons from the discarded implementation)

The expression walker must visit children in exactly the same DFS order
as referent resolution, or poles silently misalign. The prior
implementation enforced this by mirroring a second walk by hand. This
plan requires:

1. **Exhaustion assertion** — after walking a component's expression, the
   referent iterator must be empty; assert it. Catches under-consumption
   (over-consumption already panics).
2. **Exhaustive `match`** over expression payloads (no wildcard arm), so
   adding a construct forces both walks to be updated at compile time.
3. **A `#[cfg(test)]` cross-check** that walks a corpus of test designs
   and asserts consumed-referent count equals resolved-referent count per
   component.

## Tests

* Table unit tests: every row above, including width scaling and the
  `w = 1` degenerate cases (`⌈log₂ 1⌉ = 0` floors at the wiring minimum
  where applicable — e.g. `Eq` on 1 bit is `10`, `Any` on 1 bit is `1`).
* Pole tests on small hand-written moddefs: register fanout, adder
  chains, mux trees, match arms, socket connections (`:=:`), bare
  reference drivers, submodule boundaries.
* A whole-project test (e.g. the docs' `adder` example) asserting
  `max_pole` and a handful of spot-check poles.
* Combinational-loop panic test (constructed via a deliberately broken
  analysis, if reachable; otherwise documented as unreachable).

## Out of scope

* Picosecond conversion and default scales — consumers decide (the VCD
  transform fixes the educational default).
* Gate *counts* — that is the sibling plan `plans/gate_count/`.
* Any simulator or VCD involvement — the sim stays zero-delay and
  untouched (see `plans/gates_and_prop/PLANv2.md`).
* Per-bit poles — recorded as future work; the query's shape leaves room
  (a `poles: Vec<u64>` per signal instead of a scalar).
