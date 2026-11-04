# Concept: vm

# Syntax


```grammar
VasmFile := TypeDecl SignalDecl BlockDecl*

TypeDecl := "types" "{" Type* "}"

Type := Fqn Width

SignalDecl := ".signals" Signal*

Signal := SignalKind Path+ ":" Fqn

SignalKind :=
      "sig"
    | "reg"

Path := Ident ("." Ident)*

BlockDecl := "^" Path "(" BlockArgs? ")" "{" BlockInstr* "}"

BlockArgs :=
      Temp
    | BlockArgs "," Temp

Instr :=
      InstrTerminal
    | InstrSlot
    | InstrOp


InstrTerminal :=
      "branch!" Temp "," BlockRef "," BlockRef
    | "call!" BlockRef
    | "ret!"

InstrSlot :=
      "const" WordLit
    | "load" Path
    | "store" Temp "," Temp
    | "stage" Temp "," Temp

InstrOp :=
      "and" Temp "," Temp "," Temp
    | "or" Temp "," Temp "," Temp
    | "xor" Temp "," Temp "," Temp
    | "not" Temp "," Temp
    | "add" Temp "," Temp "," Temp
    | "sub" Temp "," Temp "," Temp
    | "eq" Temp "," Temp "," Temp
    | "ne" Temp "," Temp "," Temp
    | "lt" Temp "," Temp "," Temp
    | "ltu" Temp "," Temp "," Temp
    | "le" Temp "," Temp "," Temp
    | "leu" Temp "," Temp "," Temp
    | "mux" Temp "," Temp "," Temp "," Temp
    | "slice" Temp "," Temp "," SliceRange
    | "concat" TempList

BlockRef := "^" Path "(" BlockArgs? ")"

Temp := "%" Nat

SliceRange := Nat ".." Nat

TempList :=
      Temp
    | TempList "," Temp
```

Fqn is a string with no whitespace or newlines
Comments are `//` style

# The Machine
## Types
Types are mainly used to specify bit width.
However, the VM preserves the distinctions in the source code as FQNs for debugging.
Each signal is annotated with a type, but outside of bit width, it is semantically meaningless.

## Signals
A signal is an observable value which evoles over time.

## Temporaries
A tempmorary is a transient value which is not observable.
It is an SSA (single-static assignment) variable.

## Slots
The machine's primary store of data is a single, flat array of **slots**.

Each `sig` signal has one dedicated slot.
Each `reg` signal has two dedicated slots, referred to as its "set" and "val" slots.
The `set` is always the lower numbered slot.

Each slot can correspond to one or more names in the design,
designated by a dotted path beginning with `top`.

## Note
- **Temporaries** (`%n`) are *not* slots.
  They are SSA values: written once, never reassigned, no memory location.
  They live in the interpreter's virtual-register array and map directly onto
  machine registers in the future native backend.
- **Named slots** are memory: touched only by explicit `load` / `store` /
  `stage`.
  This is the MLIR `memref`/SSA distinction applied to our machine.

Slots and temporaries hold untyped bit patterns.
Every named value in the design — inputs, outputs, registers, wires, aliases, cell ports.
Internally, the slots are named by their position index.


### Register representation

Every register is **two slots**: the current value (`val`) represents the Q pin(s)
and the staged value (`set`) represents the D pin(s).

- Everything that reads a register during a cycle reads `val`.
- Only the register's driver writes `set`, via the `stage` instruction.
- Staged values are **invisible to loads** until commit.

This is what makes atomic-update semantics automatic:
two registers swapping values in one edge, or any intra-cycle feedback, just work —
nothing observes a staged value until all of them are promoted.

### Commit

**Commit = bulk copy** of the register region from `set` to `val`,
performed by the machine, not by instructions.
The commit happens at the end of a `sim.tick()` call.


### The dirty flag

A **global dirty flag** guards settle: pokes set it, settle clears it.
Settle on a clean sim is a single flag check.
Per-input dirty bits (incremental settle) are a future optimization.

## Module Frames
The slot space is organized as nested module frames inside the flat slot array.

Each module instance gives rise to a contiguous region of slots, known as the module frame.

The module frame contains its ports, its internal signals, and (nested inside it) the frames of its subinstances.
Every slot op is base + offset:
- the base is the current module instance's frame
- the offset locates the component within it

A module's names resolve only within its own frame.
Thus, a module can access its own internals,
and because child frames are nested inside, its children's ports.
Trying to access slots outside the module frame is an error, and is UB.

Direct connections are realized as aliased slots.
For instance, if a `wire w` from the parent module is connected to an `incoming inp` to a child,
the slot for `w` is given the same slot as `inp` inside of the child module.
One consequence of this is that the child 'owns' the alias in some sense.


## Fragments
A module's logic cannot always run as one block: because it may chain through a child and back, or out to a parent and back.

We will use this example:

```virdant
pub mod Top {
    incoming clock : Clock
    incoming inA : Word[8]
    incoming inB : Word[8]
    outgoing outA : Word[8] dependson inA, inB
    outgoing outB : Word[8] dependson inA, inB

    wire link : Word[8]

    mod c0 of Child {
        it.clock := clock
        it.a := inA
        it.b := inB
    }

    mod c1 of Child {
        it.clock := clock
        it.a := link
        it.b := inB
        outB := it.sum
    }

    link := c0.sum
    outA := link
}

mod Child {
    incoming clock : Clock
    incoming a : Word[8]
    incoming b : Word[8]
    outgoing sum : Word[8] dependson a, b

    reg acc : Word[8] on clock {
        it <= a + b
    }

    sum := a + b + acc
}
```

To settle the parent after a poke to one of its `incoming` ports,
we first need to settle `c0`, since we need it to compute `link` and `outA`.
Next, we settle `c1` after that.

We do `c0` before `c1` because `c1` requires `link` which requires `c0.sum`.

Finally, we settle the remainder of `top`.

In bytecode:

```vasm
  ^settle.0:  call! ^Child.settle(#top.c0)     ;; c0 produces c0.sum
  ^settle.1:  call! ^Top.settle0(#top)         ;; fragment: link, outA
  ^settle.2:  call! ^Child.settle(#top.c1)     ;; c1 consumes link, produces c1.sum
  ^settle.3:  call! ^Top.settle1(#top)         ;; fragment: outB
```

## How to partition

1. Build the module's `dependson` graph
2. Insert children as boundary nodes. Each child instance is a node whose inputs and outputs cross the frame
   boundary; a parent signal feeding a child's input, and a child output feeding parent logic, are edges to/from
   that node.
3. Cut at child crossings. A yield point is a position where the topological order must hand control to a child.
   The module's comb DAG is then split at every yield point into fragments: maximal segments of module-local
   computation containing no mandatory child crossing. Formally: contract all module-internal edges; what
   remains between child-boundary edges are the fragments. (Because the language statically rejects
   combinational cycles — dependson edges included — this graph is acyclic and a total order always exists.)
4. One block per fragment. Each fragment becomes ^Mod.settleK (single entry, ret! at end), with its read-set
   derived from the fragment's actual loads — this read-set is exactly the skip-check annotation.
5. Schedule. The phase chain interleaves fragments and child entries in the topological order of the contracted
   graph.

Properties worth recording.

- Fragments are per module definition, not per instance. The split depends only on the module's own wiring and
  children, so every instance shares the same fragments — folding is untouched.
- Yield points are mandatory; further splits are optional. Only child crossings must cut; splitting independent
  cones finer (for better skip granularity) is a lowering knob.
- Fragmentation has a tick-side contrast worth remembering: NEXT logic needs no fragments — every register's
  staged value is computed purely from pre-edge val slots, writes never read, so tick logic has no intra-module
  ordering constraints at all. Fragmentation is purely a comb (settle-side) phenomenon.
- Conceptual rhyme: a fragment is the compiled form of a Verilog always @* block — but derived and verified by
  the compiler from checked dependency data, hand-declarable only at ext mod boundaries (where dependson is a
  trusted contract).

────────────────────────────────────────────────────────────────────────────────

One naming thought to settle later: if "fragment" is adopted, the -0/-1 suffixes (^Top.settle0) could become
^Top.settle[0]-style or dot-separated ^Top.settle.0, matching the schedule-block names already in use.




---
```
^Child.settle(#self):
    %0 = load #self.a
    %1 = load #self.b
    %2 = add %0, %1
    %3 = load #self.acc
    %4 = add %2, %3
    store #self.sum, %4
    ret!

^Child.tick(#self):
    %0 = load #self.a
    %1 = load #self.b
    %2 = add %0, %1
    stage #self.acc, %2
    ret!

^Top.settle0(#self):                  ;; part of Top's settle: produce link
    %0 = load #self.c0.sum
    store #self.link, %0
    store #self.outA, %0
    ret!

^Top.settle1(#self):                  ;; the tail of Top's settle
    %0 = load #self.c1.sum
    store #self.outB, %0
    ret!

;; schedules — block chains; each call! arms the next block as continuation

^settle.0:  call! ^Child.settle(#top.c0)
^settle.1:  call! ^Top.settle0(#top)
^settle.2:  call! ^Child.settle(#top.c1)
^settle.3:  call! ^Top.settle1(#top)
^settle.4:  ret!

^tick.0:    call! ^Child.tick(#top.c0)
^tick.1:    call! ^Child.tick(#top.c1)
^tick.2:    ret!
```



When using slot instructions (`load` `store` `stage`), we use an offset-based addressing.
In the textual form, we write it as `load %t, top::Foo::path.to.component`,
the execution is done by taking


Consequences. Layout is static and computable at load time; the instance table disappears into offsets.
Elaboration = resolving bases to constants (monotone, one transform). Runtime cost is one base register, updated
per call. Ports sit at the frame edges (incoming at the start, outgoing at the end) by convention; the
load-bearing property is identical port offsets across all instances of a module, which is what makes shared
code possible. Commit is unchanged: a flat, context-free copy over register pairs, wherever they sit.



Frames as SSA values. The module instance (frame/context) becomes an explicit SSA-like value with its own sigil,
#, statically typed by its module: #foo : inst Foo. Blocks take frames as parameters — call! ^settle0(#foo) —
and slot ops name their target explicitly: load %1, #foo, .bar. There is no implicit per-block context to
preserve or worry about: contexts are just values passed around like any other SSA argument (though not signal
values — they are never stored in slots). This makes the code free to mangle under optimization: blocks can be
moved, merged, or fused across contexts because every operand is self-describing, and the "one block, one
context" invariant disappears. Module bodies become typed methods over instances (^Child.settle(#self)), the
same body serving every instance by parameterization. The runtime's base register becomes an ordinary value read
from the frame operand; commit and the machine's end-of-phase actions are unchanged.




## Blocks
The VM uses basic blocks which have a name, parameter list, and a set of instructions.

The overall design mirrors MLIR.
The parameters are are temporaries which are brought into scope when the block is entered.
Blocks must end in a terminal instruction. These are designated by a ! at the end of their names.

The name is a path. This allows a code block to be prefixed with a dotted path to the module definition it represents.

The temporaries are scoped to the block.
There is no equivalent of a PHI instruction.

- Blocks are denoted `^name`, MLIR-style, both in declarations and references.
- **Forward tail-calls only.**
  Blocks form a DAG; no block is ever re-entered.
  No loops, no recursion — hardware has neither, so the per-cycle evaluation
  is statically bounded and every block runs at most once per cycle.

  This is an assumption that can be made my the VM and by optimizations.

### Terminators

Every block ends in exactly one **terminator**, designated by `!` as part of the mnemonic.
Fall-through is not allowed: a block ending in a non-terminator is malformed
and may panic if attempting to execute instructions beyond the last.

| Terminator | Form | Meaning |
|---|---|---|
| `branch!` | `branch! cond, ^t(args...), ^f(args...)` | conditional tail-call: select successor and bind its arguments atomically; `cond` is a 1-bit SSA value |
| `call!` | `call! ^succ(args...)` | unconditional tail-call |
| `ret!` | `ret!` | leave the section; control returns to the VM, which performs the section's end-of-phase action |

**Branch arguments are SSA temporaries.**
Each successor's argument list must be exactly the right number of SSA names — no literals, no expressions.
Any value an arm needs beyond its incoming parameters is computed into a temporary before the branch.

---

### Entry points


## Sections

Each section is a program-with-an-entry-block.
Section entry blocks are named for the phase that runs them:
`^settle()` for COMB (run at settle), `^tick()` for NEXT (run at the edge).
The names are fixed; v1 presumes a single clock.
Future work: one settle and one tick entry per clock domain, expanding the
fixed-name rule.
**Neither entry is required** — a program may define only `^settle()`, only
`^tick()`, or neither.
If the machine is asked to run an entry the program does not provide, it
panics.

```
;; ---- COMB section (runs at settle) ----
^settle():
    ...
    ret!

;; ---- NEXT section (runs at tick, before commit) ----
^tick():
    ...
    ret!
```

Commit is not part of the program; it is the machine's action after
`^tick()` returns.

## Load-time validation (hand-written assembly)

The assembler rejects, eagerly:

- duplicate signal names in `.signals`;
- references to undeclared names in instructions;
- `stage` to a name declared `sig` (only `reg` has a next-slot);
- literals too wide for the destination's declared width.

Type FQNs are **not** validated against any registry — the VM is generic
and carries the FQN as opaque metadata; only the explicit width matters.

## Instruction syntax

- One instruction per line; `//` comments.
- Destinations on the left: `%2 = not %1`, `store top.led, %2`,
  `stage top.count, %7`.
- Widths and literals use Virdant notation: `0w8` is an 8-bit zero.
  Literal widths are **always explicit**; the assembler checks each literal
  fits its destination (see load-time validation below).
  Digits may be decimal (`1w8`), binary (`0b101w8`), or hex (`0x2Aw8`).
- **Slices use Virdant's own convention**: `k..r` denotes bits `k-1 … r`,
  a total of `k-r` bits.
  Extracting bit 0 of a word is `1..0`.

## Lowering policy: blocks and fusion

- **Lowering is faithful:** every `when`/`match` in the source becomes a
  block diamond — the program structure mirrors source control flow
  one-to-one.
  The un-fused program is the *debug mode*: stepping through it walks the
  design's own `when`/`match` structure.
- **A fusion pass** (block fusion / if-conversion) demotes small diamonds to
  `mux` chains: when a block conditionally tail-calls into two successors that
  rejoin at a common continuation, compute both arms and `mux` each outgoing
  parameter over the branch condition.
  Repeat until no diamond is below a size threshold.
- The threshold is deliberately dumb in v1 and re-runnable with different
  policies later; the pass is **backend-ownable** (the native backend may tune
  it differently — a real CPU branch on unpredictable FSM state is often worse
  than if-conversion, but not always).
- Fusion is a pass over the schedule, never a lowering change:
  the structured form is always recoverable.
