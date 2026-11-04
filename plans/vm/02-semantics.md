# Simulation Semantics

The semantics the VM must preserve.
These are commitments about the *language-level meaning* of a simulation run.

## Time model

### Clocks (v1: one global clock)

- v1 simulates a **single global clock** with atomic edge semantics.
- The machine's structure nonetheless reserves multi-clock:
  every register is tagged (compile time) with the clock signal it is `on`,
  and the NEXT section is partitioned into per-clock sub-sections.
  `tick(clk)` runs only the sub-section for that clock's identity plus its
  commit.
- **Clocks are inert signals.** The `Clock` type has no operations; a `Bit`
  may be promoted with `asclock`.
  `tick(clk)` therefore keys off *signal identity*, not a distinguished clock
  port.
- The eventual goal is **disciplined multi-clock** (a few domains, clean CDC,
  explicit synchronizers).
  A sketch for first-class domains exists but is unimplemented;
  the reg→clock map reserved above is the only VM-level prerequisite.
- Register update semantics are the classic synchronous contract:
  all registers see pre-edge values; all commit atomically at the edge.
  There are no delta cycles, no blocking/nonblocking distinction, no
  `#delay` machinery.

### The cycle

The canonical cycle, from the embedder's point of view:

```
(1) inputs are driven            (embedder stages pokes under a write lock)
(2) settle                        (COMB section recomputes all comb values)
(3) observation point             (outputs reflect the new inputs; regs unchanged)
(4) tick                          (NEXT section computes staged next-values;
                                   commit promotes them atomically)
```

This is the Verilog testbench rhythm: drive, observe, then edge.
It falls out of the commit machinery naturally (next-values must be computed
before they are applied) and is exposed in the API
(see [`07-api.md`](07-api.md)).

### Initialization and the first cycle

- **Initialization is a strategy, selected by flags.**
  v1 default: **random fill** (seeded, deterministic) for all registers and
  top-level incoming ports.
  Filled values need not conform to their types.
- **`initial` statements override the strategy.**
  `initial` is per-register, constant-valued, at most one per register, not a
  driver, and may not appear inside `when`/`match`.
- There is **no init program**: initial state is data, not code — a per-register
  table mapping each register to either its declared constant or
  "strategy-filled."
- **Ordering at time zero:** initial values exist, the comb network settles,
  outputs are observable, *then* the first clock edge occurs.
  This matches Verilog's power-up feel and gives the embedder a defined moment
  to observe power-on outputs.

## Values

### Word-level, two-state

- The VM operates at the **word level**, mirroring the language's own types.
  A `Word[8]` add is one instruction, not a ripple of full-adders.
- **Two-state world.** There are no X or Z values on standard wires.
- A value that would be X ⇒ **simulation terminates**
  (an anomaly; see below).
- A floating value ⇒ **collapses to a random value**
  (not necessarily type-conforming).
  Floating values are mostly prevented by the language outside the `tap`
  construct.

### Widths

- **Fast path: widths ≤ 64 bits.**
  These occupy one machine slot and every operation on them lowers to one
  machine instruction with a compile-time constant mask.
  The machine is designed around this path.
- **`Word[65+]` is legal** and must be *correct*, but lives on a slow path
  (multi-slot values, wider-op variants).
  Nothing on the fast path pays for it.

### Contention, tristate, dontcare

- Contention (multiple drivers) is mostly prevented by the language and is
  safe to cause termination.
- Tri-state components (`net` and `bus`) exist in the language but their
  simulation is **deferred** — v1 does not model them.
- **`dontcare` is not an X value.**
  It is an elaboration-time hint: the compiler may freely insert any value
  conforming to the target type.
  v1 resolves it eagerly (0-fill is acceptable, with a documented caveat that
  a 0 fill need not conform — see [`10-deferred.md`](10-deferred.md)).

## Dependency and ordering

### Combinational cycles are language errors

Virdant rejects any combinational loop at compile/elaboration time.
Consequently the VM's per-cycle evaluation is a **static total order** —
the compiled program *is* the topological order.
There is no fixed-point iteration, no event propagation, no runtime
discovery of ordering.

### `dependson` is authoritative in-language

- Inside the language, `dependson` declarations are a **verified contract**:
  the compiler checks them against actual expression structure, and
  under-declaring is an error.
  The VM may trust them blindly.
- For **ext mods**, `dependson` clauses on outputs declare which input
  ports the output may combinationally depend on; outputs with no such
  declaration are assumed registered.
  These declarations are *outside* the language checker and cannot be
  verified by Virdant — they are **assumed correct; violating them is UB**.
- Compiler-derived truth and declared truth coincide for language constructs;
  for ext mods, the declaration is the only source of truth and the API
  documents it as a contract.

## Anomalies

A **weak contract** governs value/type conformance:
once a net conforms to its type, it remains conformant until an *anomaly*
occurs.

- Default behavior: **terminate whenever an anomaly is detected.**
- Anomaly checking is **not required for the first draft** and its exact
  taxonomy and check placement are deferred
  (see [`10-deferred.md`](10-deferred.md)).
- When checks exist, they are expected to ride the same error path as ext
  mod errors: a stored, poisoned-state failure with a full diagnostic
  (see [`08-extmods.md`](08-extmods.md)).
