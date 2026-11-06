# Plan: memories

Date:   2026-11-05
Status: In Planning

## Source

`CONCEPT.md` (this directory). Design decisions below were made in an
interview with the user.

## Goal

A behavioral sublanguage for defining memories in Virdant. `memory` is a
new top-level item, instantiable like a module. Its body is a set of
concurrent processes (some clocked, some combinational) describing the
behavior of the memory's ports. Semantics are flexible enough to express
any kind of read-over-write / write-over-read behavior, implicitly
through the behavior of the code.

Hard guarantees:

- Memories simulate **efficiently** (dense-array backends, never a mux
  network of flops).
- Synthesis **infers real memory primitives or fails**. It **NEVER**
  synthesizes to a bunch of flops. If a memory cannot map to the
  target platform's primitives, it is rejected for that platform.
- Conflicts (e.g. two writes racing, a read racing a write in a way the
  hardware primitive cannot honor) are **detected and reported as
  errors before simulation or synthesis**.
- Addresses and data widths are restricted to powers of 2.

## Design decisions (from interview)

1. **Syntax: new top-level `memory` item.** Exact syntax is TBD, but the
   rough sketch is C/Python-flavored and should match Virdant wherever
   possible:

   ```
   memory mymem<ADDR: u8, DATA: u16> {
       in clk: bit
       in we: bit
       in waddr: ADDR
       in wdata: DATA
       out rdata: DATA

       storage data[256]: DATA

       on clk {
           if we { data[waddr] = wdata }
       }
       out rdata = data[raddr]   // comb process
   }
   ```

   It does *not* need to be parsed by the same grammar machinery as
   modules, but reusing the existing expression/statement syntax and
   type system is a strong preference.

2. **Execution model: concurrent processes.** The body is a set of
   blocks (like Verilog `always` blocks), each a process. Some clocked
   (`on clk`), some combinational. Statement order within a block
   defines intra-block sequencing; inter-block behavior is defined by
   the declared collision semantics of each access (see 4).

3. **Reads are not inherently combinational — but clocks are optional
   for reads.** The semantics must be flexible enough to express comb
   reads, sync reads, and every read-over-write variant. This emerges
   from the process structure and the collision menu, not from port
   annotations.

4. **Conflicts are compile-time errors.** A conflict (two writes to the
   same address in the same cycle; a read/write race whose outcome the
   code does not declare) is detected and reported as an error before
   simulation or synthesis can run.
   - The prover is deliberately **light**: conflicts are pairwise over
     port accesses (N ports → few pairs; each access has exactly one
     address expression per cycle). Analysis = syntactic aliasing,
     constant folding, range analysis through guards
     (`if waddr < 64` ⇒ write confined to that range), and
     mutual-exclusion of guards (e.g. `match`/`case` splitting the
     address space). Interval arithmetic, not an SMT solver.
   - Reframing that makes the checker tractable: hardware primitives do
     not *forbid* collisions — they support a tiny menu of collision
     outcomes (old-data, undefined, or registered-output behavior). The
     checker's job is to prove that **every conflict that can occur is
     one whose outcome the code declares and the primitive supports**.
     Anything the checker cannot prove maps to no hardware primitive
     anyway, so it is rejected.
   - Escape hatch: an explicit user assertion that two ports "never
     conflict" (trusted annotation) for patterns the prover cannot see
     through.

5. **No replication inference.** A memory that cannot map to a single
   primitive (or a supported composition) for the target platform is
   simply **rejected for that platform**. E.g. 4+ read ports cannot map
   to a single BRAM ⇒ rejected. No automatic store-replication or
   time-multiplexing.

6. **No reset.** Storage cannot be reset (real BRAMs cannot be). Reads
   of not-yet-written storage are governed by initialization (see 7).

7. **Initialization is supported**, but is platform-checked and may fail
   on platforms without bitstream memory initialization. In simulation,
   uninitialized values use the existing `InitStrategy` (default:
   `Random`) — the same as any other Virdant value. **No X's.**

8. **Per-port clocks; registering is implicit.** Every port declares its
   own clock. Whether an output is registered falls out of the code
   structure: assigned inside a clocked process ⇒ registered; assigned
   from a comb process or a direct storage read ⇒ combinational. No
   separate "register this output" annotation. Multi-clock memories
   fall out naturally, and cross-clock port pairs constrain which
   primitives they can map to (true dual-clock BRAMs are a specific
   primitive).

## V1 scope

- **Language core:** `memory` top-level item; port declarations with
  per-port clocks; `storage` declaration (one storage per memory);
  clocked and combinational processes.
- **Checking:** power-of-2 checks (address space and data width);
  conflict detection with the light prover + escape hatch.
- **Simulation:** memories simulate efficiently.
- **Synthesis:** mapping to one platform's memory primitives
  (single-port and true dual-port RAM) via the Verilog output, with
  clean rejection diagnostics when a memory does not map.
- **Deferred:** initialization, multiple storages per memory, richer
  read-over-write menus, exotic primitives (URAM/LRAM), fancier prover
  reasoning.

## Phases

### Phase 0 — Grammar and AST

- Extend `virdant.lalrpop` with the `memory` item, `storage`
  declarations, and process blocks (clocked `on <expr>` and comb).
  Reuse the existing expression grammar wholesale.
- New AST payloads under `syntax/`; item kind `memory` in
  `common::ComponentKind` / item resolution so names cross-reference
  like any top-level item.
- Ends with `make grammar`.

### Phase 1 — Checking

- New query module (alongside `queries/check.rs`): validate a memory
  item — power-of-2 constraints, storage width/depth, port clocks
  resolve to Clock-typed expressions, every port driven by exactly one
  process, every storage access well-formed.
- Conflict checker: enumerate accesses per cycle per storage, pairwise
  address-overlap reasoning (aliasing, constant folding, guard ranges,
  guard mutual exclusion), compare against the declared collision menu
  of the mapped primitive. Errors carry spans of both conflicting
  accesses.
- `vir check` reports memory errors like any other diagnostic; a failed
  memory check blocks downstream phases.

### Phase 2 — Simulation

- Decide between (documented in the phase, both sketched here):
  1. **Native sim node**: a first-class simulator node backed by a
     dense array (`Vec`/bit-pattern buffer), ports as scheduled
     watchers. Reference implementations already exist:
     `sim/builtins.rs` (`builtin_mem`, `builtin_csrs` are
     `Rc<RefCell<Vec<u64>>>`-backed array models with edge-triggered
     write watchers and comb read watchers). This phase reuses that
     machinery/patterns.
  2. **Behavioral Verilog lowering**: emit `reg [W-1:0] mem [0:D-1]`
     plus always blocks for Verilog-backend simulation — efficient in
     external simulators, but does not help the native Virdant sim.
- Either way: initialization via `InitStrategy` (Random), no X's, and
  read-during-write outcomes exactly as declared/checked.

### Phase 3 — Synthesis mapping

- Verilog output: lower the memory to the appropriate behavioral Verilog
  RAM idiom per mapped primitive (correct coding style so vendor tools
  infer BRAMs, e.g. registered read outputs, plain `always @(posedge
  clk)` writes, no reset on the storage).
- Platform mapping rules: which (port config, clock config, collision
  menu, depth/width) tuples each platform's primitives accept. Start
  with one platform (single-port and true dual-port; read-first /
  write-first collision modes as the primitive supports).
- **Rejection diagnostics**: when a memory does not map for the bound
  platform, a precise error naming the offending property (too many
  read ports, unsupported collision mode, cross-clock pair, non-power-
  of-2, uninitialized platform without init support).
- Tests under `tests/pass` / `tests/errors`.

### Phase 4 — Platform integration

- Declare the memory primitives in the platform packages
  (`lib/xilinx.vir`, `lib/ice40.vir`) following the in-language
  platform binding from the `platforms` plan (`mod Top for ...`).
- End-to-end: a `memory` instantiable in a `for`-bound top module maps,
  checks, simulates, and produces a bitstream for a real board.

## Open questions

- Exact syntax of port declarations and process blocks (sketch above is
  provisional).
- How initialization syntax looks (initializer list? file-based?) and
  how it interacts with the platform check.
- Whether the escape-hatch "never conflicts" annotation is per-port
  pair or a general assertion attribute.
- Whether `memory` items can be generic/parameterized beyond type
  parameters (e.g. depth as a const parameter).
- Native sim node vs behavioral-Verilog-only for Phase 2 (decide inside
  the phase).
