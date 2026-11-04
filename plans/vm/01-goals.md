# Goals

## Purpose

Big-design throughput.
The VM exists to simulate designs that are large by FPGA standards, fast
enough to be useful for real workloads.

## Target numbers

- **Reference design size:** a design that would occupy ~8K FPGA logic cells
  (roughly tens of thousands of evaluated elements after elaboration).
- **Comfortable target:** **1M cycles/sec** on a medium design.
- **Stretch goal:** real-time simulation of a 20MHz design, i.e.
  **20M cycles/sec**.
  This is acknowledged as likely requiring the native-code backend
  (see below) and is not a v1 gate.

## Throughput arithmetic (the honest version)

An 8K-LUT design is roughly 15–40k operations per cycle at the word level.
At 1M cycles/sec that is 10–40 billion element-evaluations per second.

A tight bytecode interpreter dispatches on the order of 100–500M
instructions/sec/core.
A generated-code backend reaches 1–5G ops/sec/core.

**Conclusion:** the interpreter alone cannot reach 1M cycles/sec on a design
of this size.
The native-code backend is not an optional extra — it is the endgame.
The bytecode exists as the first deliverable, the debugging/inspection
artifact, and the common representation both backends lower from.

## Scope constraints

- The VM must be respectable on **any legal FPGA-timed design**.
  No assumption of low toggle activity may be load-bearing.
  In particular, event-driven/incremental evaluation is *not* the backbone;
  flat evaluation of the whole schedule is.
  Activity-based cleverness may be layered on later as an optimization,
  never as a semantic dependency.
- The simulator is a **library**.
  The compiler produces programs; the sim library loads and executes them.
  Embedders are Rust programs that drive the simulation through its API and
  supply behavior the VM cannot compute (ext mods).

## Standing principles

1. **Mirror Verilator** in overall trajectory:
   hierarchical typed IR, context-sensitive optimization, flatten late,
   eventually native code generation.
2. **Optimize on the IR, never on the emitted program.**
   The flat program is dumb by design; its dumbness is a feature.
3. **Trust, but validate loudly at the boundary.**
   Language-internal contracts are compiler-checked; embedder-supplied
   contracts (ext mod declarations) are trusted, and violating them is UB.
4. **Simulation must never lie.**
   When semantics cannot be honored, the session terminates with a loud,
   well-located diagnostic — never silent garbage.
5. **The interpreter is simple; the compiler is smart.**
   All cleverness lives at compile time.
