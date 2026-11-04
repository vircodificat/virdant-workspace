# Folded Programs

Decisions from the 2026-11-03 brainstorming session on program shape.
These supersede parts of [`03-architecture.md`](03-architecture.md)
("Program shape") and the notation of [`05-ir.md`](05-ir.md); those
documents have not been rewritten yet. Items marked **leaning** are
not yet decided.

## The change being made

The standing decision was a **fully flattened program**: elaborate the
whole design, one copy of every instance's logic, absolute slot IDs,
"the program is the schedule."

New direction: **each module definition gets its own code** — module
bodies are shared across instances, and cross-module sequencing lives
in an explicit, static, topologically ordered list of invocations.

The motivation is conceptual clarity: folded → elaborated is a
**monotone specialization** (you can always inline more; you can never
recover sharing from a flattened program), and aggressive inlining
later recovers everything flattening bought.

## Two orthogonal axes

| Axis | Alternatives | Decision |
|---|---|---|
| Code sharing | folded (shared bodies, context-relative addressing) vs elaborated (one copy per instance, absolute slot IDs) | **Both supported.** It is a load-time/operand property, not a VM concept. |
| Cross-module control flow | continuation-chained (`call!` tail-calls stitch the whole design) vs schedule-driven (bodies end in `ret!`; the machine walks a static invocation list) | **Schedule-driven, universally.** Folded code *cannot* be continuation-chained (a shared body has no single successor), and elaborated code works fine as a list. |

Consequences:

- Block parameters become **intra-module only** (diamond arms). Module
  entry blocks take no arguments; data crosses module boundaries
  through slots, as it always did.
- The VM executes either form. Instruction operands carry a
  resolution mode — absolute, or context-relative (instance base +
  offset) — specialized at load time, extending the existing
  "handlers width-specialized at load time" mechanism. An elaborated
  program is one where every operand came out absolute.
- The inliner (folded → elaborated) is an **optional** program→program
  pass, not a prerequisite for running anything.

## Terminators and transfer mnemonics

Renamed and reorganized. Convention: **`!` means control never
continues to the next line; no `!` means it returns.**

| Where | Form | Meaning |
|---|---|---|
| block | `goto! ^succ(args)` | unconditional tail-call (was `call!`); control does not continue |
| block | `branch! cond, ^t(args), ^f(args)` | conditional tail-call; unchanged |
| block | `ret!` | leave the block; resume the list at the next line |
| list | `call ^blk in ctx` | run a block to completion; continue with the next list line (was `run`) |
| list | `ret!` | list is over; the machine takes over (settle: done; tick: commit) |

- `yield!` was proposed and then **collapsed back into `ret!`**: the
  same word, one meaning — hand control to the next level up.
- List `call`s are **not** tail-calls; the list is the one place where
  something genuinely returns. That asymmetry is the folded-vs-chained
  distinction in miniature.
- The old fixed-name rule (`^settle()` / `^tick()` as global entry
  blocks) dissolves: those names now belong to the **lists**, not to
  blocks. Every block ends in exactly one of `goto!` / `branch!` /
  `ret!`; every list ends in exactly one `ret!`; fall-through is
  illegal at both levels.
- Commit remains the machine's action after the `^tick` list yields.

## Blocks per module

- A module contributes **one settle entry per point where its comb
  logic must yield to a child, plus one for the tail** — the simple
  case is one entry. Splitting is forced only by comb chains that
  thread through children and back; the split structure is a property
  of the module *definition* (shareable across instances), not of any
  instance.
- **One tick entry per clock the module's registers are on** (v1: at
  most one). No comb logic → no settle entry; no registers → no tick
  entry (the "neither entry required" rule, per module).
- `when`/`match` diamonds live *below* entries as `branch!`-reachable
  blocks; they are never scheduled directly.
- Further splitting (independent cones, finer skip granularity) is
  optional — a lowering knob, not a structural requirement.

## Canonical example

The reference design for this document
(`/tmp/vmtest2/src/top.vir`, verified with current `vir check`):

```virdant
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
```

Rendered as a folded program:

```
.types
    builtin::Clock    1
    builtin::Word[8]  8

.signals
    ;; root context (Top)
    sig top.clock : builtin::Clock
    sig top.inA   : builtin::Word[8]
    sig top.inB   : builtin::Word[8]
    sig top.link  : builtin::Word[8]
    sig top.outA  : builtin::Word[8]
    sig top.outB  : builtin::Word[8]

    ;; Child — declared once, instantiated by the loader
    sig Child.clock : builtin::Clock
    sig Child.a     : builtin::Word[8]
    sig Child.b     : builtin::Word[8]
    sig Child.sum   : builtin::Word[8]
    reg Child.acc   : builtin::Word[8]

.instances
    top.c0 : Child  in top
    top.c1 : Child  in top

;; direct port connections become slot-sharing
.aliases
    top.c0.a = top.inA      top.c0.b = top.inB      top.c0.clock = top.clock
    top.c1.a = top.link     top.c1.b = top.inB      top.c1.clock = top.clock

;; ---------- module bodies ----------

^Child.settle():
    %0 = load a             ;; this invocation's a
    %1 = load b
    %2 = add %0, %1
    %3 = load acc
    %4 = add %2, %3
    store sum, %4
    ret!

^Child.tick():
    %0 = load a
    %1 = load b
    %2 = add %0, %1
    stage acc, %2
    ret!

;; Top, split at the point where c1 must consume link

^Top.settle0():
    %0 = load c0.sum        ;; cross-instance: (c0's base, sum)
    store link, %0
    store outA, %0
    ret!

^Top.settle1():
    %0 = load c1.sum
    store outB, %0
    ret!

;; ---------- the lists ----------

^settle:
    call ^Child.settle in top.c0
    call ^Top.settle0  in top
    call ^Child.settle in top.c1
    call ^Top.settle1  in top
    ret!

^tick:
    call ^Child.tick in top.c0
    call ^Child.tick in top.c1
    ret!
```

Notes:

- **The alias table *is* the wiring.** `c0.a = top.inA` is
  `it.a := inA`; `c1.a = top.link` is `it.a := link`. Both children
  share `top.inB`; their `a`s are different slots.
- **The split is where the chain is.** Top has two entries exactly at
  `c1.a := link`. The list mirrors the comb topology, one `call` per
  link in the chain.
- **Same body, two worlds:** `load a` is `top.inA` in c0's invocation,
  `top.link` in c1's. The loader resolves module-local names per
  context.
- **Slot count:** 8 `sig` slots + 2 register pairs = 12. Aliases
  *reduce* slot count (shared direct connections are not re-allocated).
  The elaborated rendering spells out all `top.c0.*` / `top.c1.*` as
  separate declarations and inlines both bodies into one `^settle()`
  chain — same 12 slots, roughly twice the instructions, no
  `.instances` / `.aliases` / lists.

## Sensitivity-based skipping (the A-vs-B analysis)

The proposal: give entry blocks sensitivity lists (Verilog-style);
the machine watches dirty signals and triggers sensitive entries.

Two designs were compared at length:

- **A — event dispatch (structural):** reader tables, a worklist,
  per-block "already queued" flags, dependency *ranks* (quietly
  re-importing the static order), and a **compare in every `store`**
  (event semantics = trigger on *change*). The machine chases changes
  at runtime.
- **B — walk + skip (annotational):** the static list is walked every
  settle; each list line carries a read-set; one bit per signal means
  "changed since last time"; a line whose read-set is clean is
  skipped. No compare in `store` ("might have changed" is a good
  enough trigger). Group/summary masks reduce the clean-path cost to
  a scan over a small bitmap.

**Leaning: B**, and only as an annotation on the lists, never
structural: the flat run remains the semantic ground truth, the
program is unchanged except for optional read-set annotations, and
the native backend gets straight-line code. The decision to *promote*
sensitivity into the core structure is deferred; the read-sets fall
out of `dependson` for free when wanted.

Dirty bookkeeping under B: pokes dirty the poked slots; running a
list line dirties what it wrote; **commit dirties the `val` slots it
promotes** (that is how downstream settle logic knows to re-run next
cycle); bits are cleared at the end of settle.

## `dependson` (current grammar, verified against source)

The in-language machinery that feeds all of this. Current form is an
**inline clause on the declaration**:

```
outgoing sum : Word[8] dependson a, b
```

Rules discovered by checking against `vir check`:

- Entries must be **the module's own incoming ports** — no regs
  (sequential, excluded), no child paths.
- The checker verifies the declared set covers the **full transitive
  comb dependency** through the instance tree, propagating children's
  contracts upward (`outA := c0.sum` forces `outA dependson inA, inB`).
- **Internal signals may not declare `dependson`** — only `outgoing`
  and `outgoing wire`. The contract lives exactly at module
  boundaries; internal topology is derived.
- For **ext mods**, `dependson` is the trusted, unverifiable contract
  (unchanged from 02-semantics.md).

Role in the VM: each module's boundary declarations are exactly the
data needed to derive (a) each list line's read-set for the
bit-check, and (b) the child-yield split points for multi-entry
modules.

## Open items

- Notation details for the lists (`.schedule` section spelling,
  whether list lines take the bit-check read-set annotations, etc.).
- The elaborated rendering of the canonical example (for the
  round-trip/golden tests).
- Whether hand-written v1 programs default to folded (leaning: yes).
- Ext mod hooks in the folded world (v2; likely another list).
- Integration: rewrite the affected sections of CONCEPT.md,
  03-architecture.md, 05-ir.md, and DECISIONS.md once the notation
  settles.
