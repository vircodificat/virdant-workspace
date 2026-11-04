# Architecture

## The three-layer pipeline

The compiler is organized as three layers.
Optimization lives in layer 1; layers 2 and 3 are (mostly) mechanical.

```
(1) Hierarchical typed IR
    modules, instances, ports, types — structure is visible here
    ↓  context-sensitive instantiation analysis, structure-directed decisions
(2) Scheduled representation
    flattened, topologically ordered; slot assignments; the state map
    ↓  lower to blocks + instructions
(3) Emitted program
    the bytecode program (v1) or native code (eventually)
```

### Layer 1: hierarchical typed IR

All optimization happens here:

- ordinary per-module passes (constant folding, dead-subexpression
  elimination, CSE),
- **context-sensitive instantiation analysis**: for each instantiation site,
  propagate the actual connections (constant ports, shared signals) into a
  per-instance specialization *at the IR level*.
  This is where flattening's biggest win — constant-driven death of whole
  subtrees — is realized without emitting flattened code,
- **structure-directed lowering decisions**, made *pre-flatten*:
  which arrays become memories vs. registers vs. ext mods, etc.
  These are recorded in the IR; the flat program cannot express them.

### Layer 2: scheduled representation

The flattening step.
Mostly mechanical: produce the topological order, assign slots, lay out the
state image, split the schedule into sections.

### Layer 3: emitted program

Two backends lower from the same scheduled representation:

- **bytecode interpreter** — the v1 deliverable; also the debugging and
  inspection artifact, and the golden-file test surface,
- **native code generation** — the eventual fast path; the endgame for the
  throughput target.

The instruction set is designed to **lower cleanly to machine code**, not
merely to interpret nicely.
That single constraint shapes many downstream decisions
(block structure, SSA temporaries, branch-free-ness, fixed-width encoding).

## Program shape

The emitted program is **fully flattened**: one program for the entire
elaborated design, no calls, no stack, no recursion.
Every instance's logic is inlined; a module instantiated eight times
contributes eight copies.
The program *is* the schedule.

This is a *backend choice*, not a structural commitment:
since structure lives in the IR, a per-module-programs backend (for modules
instantiated many times) could be added later without redoing the VM.
Verilator itself hedges the same way.

### Flattening: trade-offs accepted

| Axis | Accepted cost / gain |
|---|---|
| Code size | grows with instance count; fits icache at target scale; revisit for much larger designs |
| State layout | fully static, compile-time-decided; dense contiguous slot space; trivial addressing |
| Compile time | re-lower every instance on any change; acceptable for v1 |
| Incremental recompile | foreclosed for now; re-visit only with a real need |
| Debugging/probing | "which instance owns this slot?" is a compile-time side table (instance-path→slot-range map), not a runtime structure |
| Multi-clock readiness | easy: the schedule is one list; per-domain schedules are compile-time partitions of it |

### Disqualifying constructs — confirmed absent

A static total order would be impossible under any of:

1. dynamic instantiation (instance existence depending on runtime data),
2. runtime-indexed module instances (accessing instance N by a runtime index),
3. required intra-cycle interleaving between instances finer than
   whole-instance granularity.

**None of these exist in Virdant, and no escape hatch will be designed for
them.**
If a future language feature wants one, the language must change first,
deliberately.

## The artifact question

**v1: in-memory only.**
The compiler lowers to a program in the same process that simulates;
nothing is serialized yet.

Constraint imposed to defuse the known risk: the in-memory `Program` is
**serialization-shaped** from day one —

- flat data, indices not pointers,
- no host closures in the schedule,
- no cycles,
- a version field, for free.

A file format later is then a writer+reader, not a redesign.
When it arrives it enables: a dependency-light sim runtime usable without the
compiler crate, cross-language embedders, compilation caching, and a second
test surface (compiled-program golden files).

**Decided for v1: the VM lives inside the `virdant` crate.**
The new simulator replaces `src/sim/` in place; no separate sim crate or
`vir-vm` binary is created.
The "sim library without the compiler crate" aspiration above stays a
future goal, unlocked by the artifact format — nothing in v1 pays for the
crate boundary.

## Build order

**Decided: interpreter first, fed by hand-written assembly.**
The textual program notation of [`05-ir.md`](05-ir.md) is not a debug dump —
it is a real, parsed language: a LALRPOP grammar reads hand-written program
text into the `Program` structure, and the interpreter runs it.
Until the lowering exists, programs are written by hand (with AI assistance)
and simple examples are hand-compiled to evaluate how well the approach
works.
Compiler lowering (flatten → schedule → program) is built afterward, against
the same `Program` structure the assembler already fills.

**Decided: the first hand-compiled exercises are both the worked-example
counter and the `tests/run/gcd` design.**
The counter ([`09-worked-example.md`](09-worked-example.md)) exercises the
straight-line case — slices, load/stage, a two-arm `branch!` diamond.
The gcd design exercises everything the counter does not: a three-state FSM
via `match`, multi-arm `branch!` with block parameters, `mux` chains,
comparisons, and a 16-bit temporary-heavy datapath.
Hand-compiling both is the “get a feel for it” evaluation of whether the
assembly approach works before any compiler lowering is attempted.

## v1 scope (minimum runnable artifact)

Decided for VM v1:

1. **Textual serialize/deserialize** — the assembly notation of
   [`05-ir.md`](05-ir.md) parses into `Program` (deserialize) and prints
   back out (serialize); golden round-trip tests come with it.
2. **Validate** — the eager load-time checks of [`05-ir.md`](05-ir.md).
3. **Execute** — the interpreter: `^settle()` / `^tick()` with staging and
   commit, the global dirty flag, the v1 restrictions (widths ≤ 64,
   no ext mods).
4. **An example demonstrating the API** — a simple embedder use: one poke
   at the start, then loop `tick()` until an exit condition is met (after
   the manner of `lib/virt.vir`'s `exit` port), then print the value and
   exit with it.
   The embedder knows the `Valid[Word[8]]` layout (`builtin.vir`): a
   `Word[9]` whose bit 0 is the enum tag — `1` when `@Valid` — and whose
   bits 9..1 are the payload code. The check is `exit[1..0]` (the 1-bit
   tag; slices are half-open, `k..n` = bits `k` through `n-1`) and
   `exit[9..1]` (the payload).

Deferred from v1: the CLI and runner integration (decided separately),
ext mods (v2), and the compiler lowering (built after v1, against the same
`Program` structure).

## Migration sequencing

**Decided: delete first, then build — reusing whatever can be reused.**
The old simulator (`virdant/src/sim/`) is removed up front rather than kept
compiling during the build-out; the new VM is developed in the vacated
space.
This accepts a temporarily red tree (`vir run`, the virt board, and the
`kind = "run"` suite do not work until the new VM catches up) in exchange
for never maintaining dead code.
The sole gate is unchanged: the suite passes again, then keeps passing.

Reusables — code and structures in `src/sim/` whose logic survives the
rewrite — are moved into the new modules deliberately, not copied by
momentum:
candidate parts are the bit-pattern machinery, type layouts, the VCD
writer, and the ext mod builtin logic (which becomes the first `ExtMod`
implementations).
Everything the new VM's design replaces (the event scheduler, the
picosecond time queue, callback attachment, the old evaluator itself) is
discarded.

## Verification strategy

**The gate: the sim test suite must pass** (`tests/run/`, 197 `kind = "run"`
tests driven by `vir run --cycles=N`).
The suite provides cycle-level expected behavior for the corpus
(gcd, uart, i2c, fifo, pipelines, statemachines, ...).

**Decided: the old simulator is deleted outright — no differential oracle.**
The old `virdant::sim` evaluator will not be kept alive as a comparison
backend; the new VM is its replacement, in the same crate, and the sim
test suite is the sole arbiter of correctness.
The suite is self-checking (each design computes its own verdict through
the `virt::Virt` socket), so the new VM's first public obligation is:
197 designs exit 0.

Compiled-program golden files become a second test surface once the artifact
format exists.
