# The Machine

## The slot file

The machine's entire world is one flat array of **slots**.
Slots hold untyped bit patterns; the fast path stores one `u64` per slot.
Every named value in the design — inputs, outputs, registers, wires, aliases,
cell ports — is a slot addressed by an **ID**.
(Here and throughout: "cell ports" are the ports of ext mod instances.)

- **Temporaries** (`%n`) are *not* slots.
  They are SSA values: written once, never reassigned, no memory location.
  They live in the interpreter's virtual-register array and map directly onto
  machine registers in the future native backend.
- **Named slots** are memory: touched only by explicit `load` / `store` /
  `stage`.
  This is the MLIR `memref`/SSA distinction applied to our machine.

### Which values are named

**All user-visible nets are named**: top-level ports, wires, registers,
aliases, ext mod ports.
This follows from the probing decision — everything is probeable in v1, and
probing requires a named location plus a side-table entry.
Demoting a wire to a temporary is forbidden until the future
probe-ability restriction lands
(see [`10-deferred.md`](10-deferred.md)).

### IDs and the side table

- IDs are **small opaque session-scoped integers** (u32-ish newtypes),
  copyable, storable in ext mod structs, meaningless outside their session.
- The **side table** is keyed by ID and records, for each named value:
  its **fully-qualified instance path** (rooted at `top`), its declared type
  (opaque FQN), its width, its kind
  (sig / reg / ext mod port), and register pairing
  (cur ↔ next).
  Aliases are multiple names for the same slot(s) — one ID per name, all
  resolving to the same storage.
- The side table is the *tooling* surface: probing, waveform capture,
  diagnostics, anomaly checks, and name↔ID resolution all read it.
  The VM's hot loop never touches it.
- Compiler may order slots however it likes; determinism of *layout* is not an
  ABI promise.
  Names are the stable currency.

### Register representation

Every register is **two slots**: the committed value (`cur`) and the staged
next value (`next`).

- Everything that reads a register during a cycle reads `cur`.
- Only the register's driver writes `next`, via the `stage` instruction.
- Staged values are **invisible to loads** until commit.

This is what makes atomic-update semantics automatic:
two registers swapping values in one edge, or any intra-cycle feedback, just
work — nothing observes a staged value until all of them are promoted.

### Commit

**Commit = bulk copy** of the register region from `next` to `cur`,
performed by the machine, not by instructions.

Chosen for v1 because it is simple and predictable.
Cost: O(total register bits) per cycle — for the target design, an estimated
2–8 KB, i.e. ~50–200ns, roughly 5–20% of the 1µs cycle budget.
Accepted for v1.

Ruled out: pointer-role swap (O(1) commit, but forces parity-indexed program
variants or indirect bases — complexity for marginal gain).
Possible later upgrade: **minimal snapshot** — snapshot only the registers
that are read pre-edge by another register's driver (a small compiler-analyzed
set), then write in place.
This drops in later without changing the program structure, and is the
performance-optimal endpoint.
The schedule format is designed so this upgrade is local to the commit step.

### The dirty flag

A **global dirty flag** guards settle: pokes set it, settle clears it.
Settle on a clean sim is a single flag check.
Per-input dirty bits (incremental settle) are a future optimization.

## The cycle, mechanically

```
POKE    : embedder writes input slots (under the write lock)   → set dirty
SETTLE  : run COMB section of the program                       (skip if clean)
PEEK    : embedder reads output/probe slots
TICK    : run ext mod tick() hooks for ports on the ticked clock,
          run NEXT section (writes staged slots),
          commit: memcpy next→cur, clear dirty
```

Two semantic notes:

1. **Ext mod `tick()` hooks and the NEXT section are order-independent.**
   Both read only settled (committed) values and both write only staged
   values, which are invisible until commit.
   The VM fixes an order (ext mods first) but nothing semantic depends on it.
2. **Between ticks, nothing moves.**
   Settle is idempotent; observing repeatedly is free.

## Sections

The program is divided into sections by compile-time boundaries:

- **COMB section** — entry block `^settle`.
  Recomputes all combinational values from inputs and committed register
  values.
  Executed at settle.
- **NEXT section** — entry block `^tick()` (fixed names; single clock in
  v1, one settle/tick per clock domain later).
- **Neither entry is required**; invoking a section whose entry block the
  program does not define panics.
  Computes every register's next value from settled comb values and committed
  register values.
  Executed at tick, before commit.
- For the multi-clock future, NEXT is partitioned into per-clock sub-sections;
  `tick(clk)` runs only the sub-section for that clock.

A settle that changes nothing produces no observable difference;
a tick with no state change still performs the (cheap) commit.
