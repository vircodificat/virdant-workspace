# The Embedder API

The sim library's public surface.
Shaped as an RWLock-style discipline; the Rust type system enforces the
cycle rhythm.

## Core surface (v1)

Three capabilities are core and shape the machine:
**drive-and-step**, **waveform capture**, and **ext mod hosting**.

```rust
// Write path: staging scope
let mut w = sim.lock_mut();          // &mut via guard; no tick possible inside
w.poke(w.id_of("top.reset"), val);   // stage a poke (by ID)
w.peek(w.id_of("top.led"));          // pre-settled view — documented, intentional
drop(w);                             // ← drop triggers SETTLE

// Read path: observation
let r = sim.lock();                  // only acquirable when no writer is held
r.peek(r.id_of("top.led"));          // settled view, by construction

// The edge
sim.tick(clk);                       // requires &mut Sim; excluded while any
                                     // lock is held — the borrow checker IS the rule
```

### The lock discipline (decided)

- **The lock is the discipline.**
  There is no unguarded way to poke or peek; therefore there is no moment
  where state is stale *without someone knowing why*.
- **Inside a writer scope the world is paused.**
  Pokes are staged; peeks deliberately show the pre-settled world.
  This is the documented meaning of peeking inside a staging scope, not a
  trap.
- **Drop the writer lock → settle once.**
  All staged pokes take effect; the comb network re-evaluates; dirty clears.
- **Reader locks and `tick` require an unsettled world to be impossible:**
  `lock()` is exclusive against writers, and `tick` needs `&mut Sim`, which
  any held lock excludes.
  So reads and edges always see a settled world — no runtime rules needed.

### Names and IDs

- Name→ID resolution is a **method on the session**:
  `id_of("top.cpu0.mem0.RADDR") -> PortId`.
- IDs are **small opaque session-scoped integers** — copyable, storable,
  meaningless outside their session
  (see [`04-machine.md`](04-machine.md)).
- ID→name resolution exists for diagnostics, tracing, and dumps.

### `tick(clk)` and clocks

- `tick` takes a **clock handle** — v1 has one clock, but the API is
  multi-clock-shaped from day one.
- Clocks are inert signals; `tick` keys off signal identity.
  Registers and ext mod ports are tagged with their clock at compile time;
  `tick(clk)` runs that clock's NEXT sub-section, the ext mod `tick()` hooks for
  ports bound to it, and the commit.

## Tracing

**The sim library owns tracing.**

- A session is constructed with an optional tracer (e.g., a VCD writer).
- The session's step function invokes the tracer at the **phase boundaries**:
  after settle, after commit.
  This yields correct VCD semantics and a choice of recording post-settle,
  post-edge, or both rows per cycle.
- The VM program never knows tracing exists; the hooks are no-op calls when
  no tracer is attached.
- The tracer reads the slot file through the side table —
  the same mechanism probing uses.
- v1 default: **record all signals** (everything is probeable), with
  selection filters as a later refinement.
- **Name emission is configurable**: the tracer is configured with which
  names to emit.
  Convenience form: emit **all** — in which case alias names (multiple
  names for the same slot) each appear in the trace, every name shown for
  every signal.

The phase-boundary hook seam is general: future observers (watchpoints,
instrumentation counters, checkpointing) attach to the same mechanism rather
than spawning bespoke machinery.

## Explicitly not in v1

- **Internal probing beyond the uniform side table** — today every net is
  probeable by construction, so nothing extra is needed; the future change is
  a *restriction* that enables dead-code elimination
  (see [`10-deferred.md`](10-deferred.md)).
- **Watchpoints, mid-run pokes, forced partial re-evaluation.**
  Noted: a future poke machinery needs partial re-execution, so the step
  function stays structured as "settle → commit" with clean entry points,
  never fused into one opaque blob.
- **Instrumentation counters.** Zero instrumentation in v1; retrofittable by
  documented constraint (program structure survives to the artifact; encoding
  leaves opcode room).
- **Save/restore of a running session.** Eventual, not v1.
  The slot-file side is trivially snapshotable (it is bytes);
  ext mod `save`/`load` hooks are defined in the trait from day one so the ABI
  never breaks (see [`08-extmods.md`](08-extmods.md)).

## Error handling

**Poisoned-state pattern, panic-free** (decided; mechanics in
[`08-extmods.md`](08-extmods.md)):

- An `Err` from an ext mod (or, later, an anomaly) is *stored*; the sim enters a
  poisoned state in which every subsequent operation immediately reports the
  same error and no further evaluation occurs.
- `Drop`-triggered settle works naturally: there is nothing to propagate —
  the error is already recorded.
- Errors surface as `Result`s at the API boundaries the embedder calls.
- Diagnostics carry full context: ext mod path, port ID (resolved to a name),
  cycle number.
- The one-line contract: **after any error, the session is dead; every
  operation reports the same error; the only way forward is a new session.**
- Anomaly termination (when checks land) rides this exact path.
