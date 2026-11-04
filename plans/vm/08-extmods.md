# Ext Mods (Primitive Behavior)

Ext mods are the VM's black boxes: behavior the embedder supplies, invoked by
the machine at defined points.
Memories, UARTs, VGA controllers, USB — anything the VM cannot compute as
plain logic.

## Declaration (in Virdant)

Ext mods are declared as `ext mod`s.
See `lib/ice40.vir` for the real corpus: `SB_LUT4`, `SB_CARRY`, the `SB_DFF*`
family, `SB_RAM40_4K`, and friends.

- The **interface** is declared in Virdant: typed ports, checkable like any
  module.
- The **design instantiates it like any module** — the author neither knows
  nor cares that it has no Virdant implementation.
- The **implementation is supplied at simulation time** by the embedder.
- Elaboration treats it as an **opaque leaf**: the compiler never looks
  inside; it knows only the ports.
- Port names match what yosys/netlists expect for the corresponding hard
  block.
- **Each output port must declare `dependson`** — the input ports it may
  combinationally depend on.
  An output with no such declaration is assumed **registered**.
  (The current library declarations are missing these clauses; they are
  required by this design.)
  These declarations are outside the language checker: assumed correct,
  violating them is UB.

### The `@cell` annotation is not a simulation concept

The `@cell("NAME")` annotation on an ext mod is **synthesis metadata only**:
it tells Yosys which hard-block implementation to map the ext mod to in the
netlist.
It plays no role in simulation.
The VM binds and invokes ext mods by their **ext mod definition name**
(e.g. `mem::Mem`, `SB_RAM40_4K`), never by the annotation.

## Binding and construction

- **Binding is by ext mod definition name, globally.**
  The embedder registers one implementation per ext mod name; the runtime
  constructs a fresh implementation instance per design instance.
  Per-instance overrides are a possible future refinement
  (the binding table is the only thing that changes).
- **Construction contract:**

  ```rust
  fn new(path: String, params: HashMap<String, ParameterValue>) -> Result<Self, Error>
  ```

  - `path`: the instance's fully-qualified path (`"top.cpu0.mem0"`).
  - `params`: `HashMap<String, ParameterValue>` — names are strings, values
    are a thin typed enum covering the allowed literal types (enumerated
    from `GRAMMAR.txt`).
  - Parameters are **immutable after construction** — they are
    elaboration-time facts, like generics.
  - Constructed **eagerly**, one per instance, at session setup, before
    cycle 0.
  - Construction failure **aborts the session** — an ext mod that cannot
    build has nothing meaningful to contribute; silently substituting a
    broken one is the "simulation lies to you" failure mode.

## The trait

```rust
trait ExtMod {
    /// Constructed eagerly at session setup, one per instance.
    fn new(path: String, params: HashMap<String, ParameterValue>) -> Result<Self, Error>;

    /// Combinational hook. Called during settle, at the point in the schedule
    /// where the ext mod's declared `dependson` inputs are settled.
    /// Default: no-op. Only scheduled at all if comb outputs are declared.
    fn eval(&mut self, ctx: &mut ExtModCtx) -> Result<(), Error> { Ok(()) }

    /// Clocked hook. Called once per bound clock-port per `tick` of that clock.
    /// Default: no-op (e.g. SB_LUT4 implements neither hook).
    fn tick(&mut self, port: PortId, ctx: &mut ExtModCtx) -> Result<(), Error> { Ok(()) }

    /// Defined now, used later (save/restore is an eventual requirement).
    /// v1 ext mods ignore these; the ABI never breaks.
    fn init(&mut self, ctx: &mut ExtModCtx) -> Result<(), Error> { Ok(()) }
    fn save(&self) -> Result<Vec<u8>, Error> { Err(Error::Unsupported) }
    fn load(&mut self, data: &[u8]) -> Result<(), Error> { Err(Error::Unsupported) }
}
```

Notes:

- **`tick` is per clock port, not per instance.**
  `SB_RAM40_4K` has two clock ports (`RCLK`, `WCLK`); ticking `RCLK` calls
  `tick(RCLK, ctx)` only.
  An ext mod whose ports bind to the same clock signal gets one call per
  port, in declared port order.
- **There is no third "every-cycle" hook.**
  Everything an ext mod does per cycle is either combinational (`eval`) or
  edge-triggered (`tick`).
  A model wanting per-cycle behavior hangs it off a clock port — the same
  discipline hardware itself follows.
- **`tick()` mirrors `sim.tick(clk)`** — one mental model all the way down.
- **Ext mod `tick()` hooks and register staging are order-independent:**
  both read only settled values and write only staged values, which are
  invisible until commit.
  The VM fixes an order (ext mods first) but nothing semantic depends on it.

## The `ExtModCtx`

- `ctx.get(id) -> Value` / `ctx.set(id, val)` — access **by abstract ID**,
  the same ID space as `peek`/`poke` at the session level
  (one unified address space; no special ext-mod API).
- `Value` is a plain **bit pattern + width** — no type identity in the
  value itself; every slot knows its own width, and a value wider than the
  slot panics.
- Writes are **masked to the declared port width** — the VM's own write
  discipline, applied at the boundary.
- Port IDs are bound at registration; the ctx's name→slot mapping is paid
  once, not per call.
- The ctx exposes **only the ext mod's declared ports**.
  The slot file, other ext mods, the rest of the design: invisible.
  (A deliberate escape hatch — raw slot access for advanced embedders — is
  designed but possibly unimplemented; restrict by default, widen
  deliberately.)
- The same interface object carries the port map and the `dependson`
  declarations, so they cannot drift apart.

## Errors

Ext mod hooks return `Result`.
An `Err` triggers the **poisoned-state pattern**:

- the error is stored; the session is dead;
- every subsequent operation reports the same error; no further evaluation
  occurs;
- diagnostics carry the ext mod path, port ID (resolved to a name), and
  cycle.

See [`07-api.md`](07-api.md).

## Precedent: the old simulator's builtins

The old simulator already models a subset of ext mods (`rf::Rf`, `mem::Mem`,
`mem::Csrs`) through public-API callbacks; unregistered ext mod instances
are left undriven.
The new VM generalizes this into the trait above: **every** ext mod
instance gets an implementation from the binding table, and an ext mod name
with no registered implementation is a session-setup error (there is no
"leave it undriven" — silence would be the simulation lying).

## The hot-ext-mod problem — out of scope

A netlist-style design could be *thousands* of trivial ext mods
(`SB_LUT4`), and one Rust callback per instance per cycle is a performance
catastrophe at the throughput target.
**This is explicitly out of scope**: the trait mechanism stands as the only
ext mod path for now.
Do not design for it; if a future need arises (native implementations for
standard ext mod families), it is a compiler-side inlining decision, not a
VM redesign.
