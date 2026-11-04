# The Instruction Set (draft)

## Machine model recap

- Flat, statically-ordered program over named slots and SSA temporaries.
- **Words are ambiguously signed; the operation carries the sign.**
  Bit patterns have no inherent sign: `add`/`sub` behave identically for
  either interpretation (wrap = mask on write), while ops where it matters
  (`lt` vs `ltu`, `shr` vs `sra`) exist in both flavors.
- Every arithmetic write masks to the destination's width — for the fast
  path, a single `and` with a compile-time constant mask.
- **One opcode set; wide-ness is a slot/operand property, not a variant.**
  Widths are static and known at load time, so the interpreter
  specializes each instruction's handler to its actual widths at load:
  fast-path slots take the single-`u64` handler, wide slots take the
  limb-loop handler — no per-cycle width check, no `.wide` mnemonic
  variants, and stepping/debugging stays width-uniform (one vocabulary
  for tooling; a debugger reads the same instruction stream and gathers
  limbs only when probing a wide slot).
  Slow-path needs (limb representation, wider operand space in the
  encoding) are internal mechanics — the second instruction word for wide
  operands remains the plan.
- **v1 restriction: widths > 64 are rejected by the assembler**
  ("wide values not yet supported"), with the unified-opcode design above
  kept intact so wide slots drop in without changing the ISA or program
  text.
- Block-based, forward-only control (see [`05-ir.md`](05-ir.md)).
- Designed to lower cleanly to machine code.

## Instruction classes

| Class | Instructions | Notes |
|---|---|---|
| data movement | `const` · `mov` | both mask to the destination's width |
| state access | `load` · `store` · `stage` | named values only; `stage` writes a register's next-value image (invisible until commit) |
| bitwise | `and` · `or` · `xor` · `not` | |
| arithmetic | `add` · `sub` | wrap = mask on write; no `mul` (removed — signedness belongs to the operation, and words are ambiguously signed) |
| shifts | `shl` · `shr` · `sra` | shift amount from slot or immediate; out-of-range behavior must be defined (open item) |
| comparison | `eq` · `ne` · `lt` · `ltu` · `le` · `leu` | result is a 1-bit SSA value |
| select | `mux` | `mux d, c, t, f` — the conditional; `when`/`case` lower to chains of these |
| bit ops | `slice` · `concat` | the fiddly 10%; slice uses Virdant's `k..r` convention and is **read-only** (no destination slice); `concat` is n-ary |
| dynamic index | `tblread` | read element `k` of a contiguous slot region; O(1); always in bounds (see below) |
| ext mods | *deferred to VM v2* | `exteval` and ext mod declarations are deferred; see [`10-deferred.md`](10-deferred.md) |
| terminators | `branch!` · `call!` · `ret!` | see [`05-ir.md`](05-ir.md) |

### `concat` — n-ary

`concat %d, %a, %b, %c, ...` — operands assembled left-to-right,
high-to-low (first operand = most significant bits).
The destination width is **derived** from the operands: the sum of the
operands' widths.
No restriction on zero-width operands (degenerate cases are legal; an
all-zero-operand concat is just width 0).

### `slice` — read-only, no destination form

`slice` only extracts: `%d = slice %src, k..r`.
There is **no destination-slice** form (no write-through-a-window
addressing); a partial-width write is expressed by reassembling the whole
word with `concat` — the compiler does this, so the ISA never needs
read-modify-write bit tricks.

### `tblread` — dynamic indexing

Virdant's `foo[dyn k]` is **read-only** and requires the subject's size to be
2^N where N is the index width — so the access is *always* in bounds.
A whole array occupies a contiguous slot region; `tblread` selects within it
in O(1).
No mux tree, no bounds check, no loops.

Dynamic-index *writes* do not exist in the language today; if they ever do,
they need a register-file-style write op and interact with the commit model —
a future language decision, not a v1 concern.

### Ext mods in the instruction stream — deferred to VM v2

**Ext mods are out of scope for VM v1** (which covers the hand-written-
assembly phase and the first suite bring-up).
When they arrive, the plan is:

- **`exteval`** appears in the COMB section at the point where the ext mod's
  declared `dependson` inputs are settled.
  It invokes the ext mod's `eval()` hook, which writes the ext mod's
  combinational outputs to their named slots.
  Emitted only for ext mods that declare comb outputs.
- **Ext mod `tick()` hooks are not instructions.**
  They run at the tick boundary, for each ext mod port bound to the ticked
  clock, before the NEXT section and commit
  (order-independent with the NEXT section; see [`04-machine.md`](04-machine.md)).
- A `.ext` declaration section for hand-written assembly (instance name,
  ports, `dependson` clauses, clock-port bindings) — flat vs. scoped port
  syntax was still undecided when deferred.

## Deliberately absent

- **Branches beyond the block terminators.** No jump tables, no computed
  jumps. Control is the block DAG.
- **Calls, stack, recursion.** The program is flattened; blocks tail-call
  forward only.
- **Memory operations.** v1 memories are ext mods
  (`SB_RAM40_4K` et al.); memory *inference* is an IR-level decision made
  pre-flatten, for a later layer.
- **X/Z machinery of any kind.** Two-state machine, total functions.
- **Tracing/instrumentation instructions.** Tracing is a session-level
  component; instrumentation is retrofittable by constraint
  (see [`10-deferred.md`](10-deferred.md)).

## Why branch-free-at-the-core matters

The program is a pure dataflow schedule with structured splices.
That makes it: trivially schedulable, trivially verifiable (static bounds,
every block runs at most once), and mechanically lowerable to machine code.
Every instruction executes on the taken path only — blocks *do* skip work at
runtime (an untaken arm never executes), which is exactly what the fusion
pass trades against dispatch overhead.

## Encoding (sketch — open mechanical layer)

- Fixed-width instructions on the fast path (target: 32 bits:
  opcode + operand fields sized for slot indices in the tens of thousands —
  17–24 bits per operand).
- Slow-path ops get a second instruction word (wide operands, limb counts).
- The interpreter's decode should be field extraction, nothing more.
- **Constraint carried forward:** leave opcode room for retrofitted
  instrumentation and future ops.

Open mechanical layers, to be decided when implementation begins:
exact encoding widths, `Word[65+]` limb representation and op variants,
slot assignment/locality heuristics, shift-amount semantics at out-of-range.
