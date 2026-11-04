# Decision Log

Every decision made during the design interviews, in one place.
Each entry is a one-line commitment plus a pointer to the section that
elaborates it.
This file is an index — the canonical wording lives in the pointed-to
document; when they disagree, the pointed-to document wins.

## Purpose & principles

| Decision | Where |
|---|---|
| Purpose: big-design throughput; reference design ~8K FPGA logic cells; comfortable target 1M cycles/s; stretch 20M cycles/s | [`01-goals.md`](01-goals.md) |
| The interpreter alone cannot reach the target; native-code generation is the endgame, not an optional extra | [`01-goals.md`](01-goals.md) |
| Flat evaluation of the whole schedule is the backbone; event-driven cleverness is layered on later, never load-bearing | [`01-goals.md`](01-goals.md) |
| The simulator is a library inside the `virdant` crate; embedders are Rust programs | [`01-goals.md`](01-goals.md), [`03-architecture.md`](03-architecture.md) |
| Standing principles: mirror Verilator; optimize on the IR only; trust-but-validate-loudly; simulation never lies; interpreter simple, compiler smart | [`01-goals.md`](01-goals.md) |

## Terminology

| Decision | Where |
|---|---|
| The VM's black-box mechanism is for **ext mods** (not "cells"); `@cell` is synthesis-tooling metadata only, not a simulation concept | [`08-extmods.md`](08-extmods.md) |
| Signals are declared `sig` / `reg` (not `in`/`out`/`wire` — those are annotations, not machine meaning) | [`05-ir.md`](05-ir.md) |

## Semantics

| Decision | Where |
|---|---|
| v1: one global clock; multi-clock reserved in the structure (reg→clock tagging, per-clock NEXT sub-sections) | [`02-semantics.md`](02-semantics.md) |
| Canonical cycle: drive → settle → observe → tick | [`02-semantics.md`](02-semantics.md), [`07-api.md`](07-api.md) |
| Initialization is a strategy (v1: seeded random fill); `initial` is data, not code; no init program | [`02-semantics.md`](02-semantics.md) |
| Two-state world; X ⇒ terminate; floating ⇒ random | [`02-semantics.md`](02-semantics.md) |
| Combinational cycles are language errors, so the program is a static total order — no runtime ordering discovery | [`02-semantics.md`](02-semantics.md) |
| `dependson` on ext mods is a trusted contract; violating it is UB | [`02-semantics.md`](02-semantics.md), [`08-extmods.md`](08-extmods.md) |
| Anomaly checking deferred; default terminate-on-detection | [`02-semantics.md`](02-semantics.md), [`10-deferred.md`](10-deferred.md) |

## Architecture

| Decision | Where |
|---|---|
| Three-layer pipeline; ALL optimization in layer 1 (including context-sensitive instantiation analysis and structure-directed lowering decisions) | [`03-architecture.md`](03-architecture.md) |
| Layer 2 (flatten/schedule/slots) is mechanical | [`03-architecture.md`](03-architecture.md) |
| Two backends lower from the same scheduled representation: bytecode interpreter (v1) and native code (endgame) | [`03-architecture.md`](03-architecture.md) |
| The ISA is designed to lower cleanly to machine code, not merely to interpret nicely | [`03-architecture.md`](03-architecture.md), [`06-instruction-set.md`](06-instruction-set.md) |
| Fully flattened program: no calls, no stack, no recursion; one program for the whole design | [`03-architecture.md`](03-architecture.md) |
| Disqualifying constructs stay absent: dynamic instantiation, runtime-indexed module instances, sub-instance-granularity interleaving | [`03-architecture.md`](03-architecture.md) |
| v1 artifact is in-memory only, but serialization-shaped from day one (flat data, indices, no closures, no cycles, version field) | [`03-architecture.md`](03-architecture.md) |
| The VM lives in the `virdant` crate, replacing `src/sim/` in place; no separate sim crate, no `vir-vm` binary | [`03-architecture.md`](03-architecture.md) |
| Build order: interpreter first, fed by hand-written assembly; compiler lowering afterward against the same `Program` structure | [`03-architecture.md`](03-architecture.md) |
| First hand-compiled exercises: **both** the worked-example counter and the `tests/run/gcd` design | [`03-architecture.md`](03-architecture.md) |
| v1 scope: textual serialize/deserialize, load-time validation, execute, and an API example (poke → tick loop → exit check → print) | [`03-architecture.md`](03-architecture.md) |
| Migration: delete first, then build — reusing bit-pattern machinery, type layouts, VCD writer, ext mod builtin logic; discarding the event scheduler, time queue, callbacks, old evaluator | [`03-architecture.md`](03-architecture.md) |
| Verification: the sim suite (197 `kind = "run"` tests) is the sole gate; no differential oracle — the old simulator is deleted outright | [`03-architecture.md`](03-architecture.md) |
| CLI and runner integration are decided separately, not part of v1's minimum artifact | [`03-architecture.md`](03-architecture.md) |

## Machine model

| Decision | Where |
|---|---|
| Global flat value space; every named value is a slot addressed by ID | [`04-machine.md`](04-machine.md) |
| Side table carries FQN path, opaque type FQN, width, kind (`sig`/`reg`/ext mod port), and cur↔next pairing | [`04-machine.md`](04-machine.md) |
| Aliases are multiple names for the same slot(s); one ID per name | [`04-machine.md`](04-machine.md) |
| Entry blocks have fixed names: `^settle()` (COMB) and `^tick()` (NEXT); v1 presumes one clock; per-clock-domain entries later | [`04-machine.md`](04-machine.md), [`05-ir.md`](05-ir.md) |
| Neither entry is required; running an entry the program does not provide panics | [`05-ir.md`](05-ir.md) |
| Commit is the machine's action after `^tick()` returns, applied atomically | [`04-machine.md`](04-machine.md) |
| Ext mod `tick()` hooks are order-independent with the NEXT section; machine fixes ext mods first | [`04-machine.md`](04-machine.md) |

## Program notation & assembly

| Decision | Where |
|---|---|
| The assembly notation is the v1 **input format**, parsed by a LALRPOP grammar — not a debug dump | [`05-ir.md`](05-ir.md), [`03-architecture.md`](03-architecture.md) |
| `.types` section: FQN → explicit width; `builtin::`-qualified builtins; widths always explicit; FQNs are opaque metadata, never validated | [`05-ir.md`](05-ir.md) |
| `.signals` section: `sig` and `reg` only; `reg` is first-class (allocates a cur/next pair) | [`05-ir.md`](05-ir.md) |
| Declarations may carry multiple names; aliasing is symmetric for `reg` too | [`05-ir.md`](05-ir.md) |
| Load-time validation: duplicate names, undeclared references, `stage` to a `sig`, literal too wide | [`05-ir.md`](05-ir.md) |
| Blocks are `^name` with always-declared argument lists; block arguments are temporaries (`%`); forward tail-calls only; no phis | [`05-ir.md`](05-ir.md) |
| Terminators: `branch!` / `call!` / `ret!`; every block ends in exactly one | [`05-ir.md`](05-ir.md) |
| Branch arguments are SSA temporaries only — no literals, no expressions | [`05-ir.md`](05-ir.md) |
| Literal widths always explicit; decimal, binary, and hex digit forms | [`05-ir.md`](05-ir.md) |
| Slices use Virdant's `k..r` convention (`k-r` bits) | [`05-ir.md`](05-ir.md) |
| Lowering policy: faithful lowering first (program mirrors source control flow); fusion (if-conversion) as a re-runnable pass; threshold deliberately dumb in v1 | [`05-ir.md`](05-ir.md) |

## Instruction set

| Decision | Where |
|---|---|
| Words are ambiguously signed; the operation carries the sign (`lt`/`ltu`, `shr`/`sra` both exist) | [`06-instruction-set.md`](06-instruction-set.md) |
| No `mul` — removed | [`06-instruction-set.md`](06-instruction-set.md) |
| `concat` is n-ary; destination width derived from operands; zero-width operands legal | [`06-instruction-set.md`](06-instruction-set.md) |
| `slice` is read-only — **no destination slice**; partial writes reassemble the word with `concat` | [`06-instruction-set.md`](06-instruction-set.md) |
| One opcode set; wide-ness is a slot/operand property; handlers width-specialized at load time; no `.wide` mnemonic variants | [`06-instruction-set.md`](06-instruction-set.md) |
| v1 restriction: widths > 64 are rejected by the assembler | [`06-instruction-set.md`](06-instruction-set.md) |
| Out-of-range shifts are undefined until decided — interpreter hits `todo!()` | [`10-deferred.md`](10-deferred.md) |
| `tblread` is read-only, O(1), always in bounds (subject size must be 2^N) | [`06-instruction-set.md`](06-instruction-set.md) |
| Deliberately absent: computed jumps, calls/stack, memory ops, X/Z machinery, tracing instructions | [`06-instruction-set.md`](06-instruction-set.md) |
| Ext mods in the instruction stream (`exteval`, `.ext` section) deferred to VM v2 | [`06-instruction-set.md`](06-instruction-set.md), [`10-deferred.md`](10-deferred.md) |
| Encoding target: 32-bit fixed instructions on the fast path; second word for slow path; decode is field extraction only | [`06-instruction-set.md`](06-instruction-set.md) |

## API & tooling

| Decision | Where |
|---|---|
| RWLock-style API: staged pokes under a write lock; dropping the lock settles; `tick()` requires `&mut` | [`07-api.md`](07-api.md) |
| IDs are opaque, session-scoped values; copyable, storable, meaningless outside their session | [`04-machine.md`](04-machine.md), [`07-api.md`](07-api.md) |
| Pre-settled peek (under a write lock) is documented, intentional behavior | [`07-api.md`](07-api.md) |
| Tracing rides phase-boundary hooks; VCD emission default-on; name emission configurable, with an emit-all mode showing every alias name | [`07-api.md`](07-api.md) |
| Error model: stored, poisoned-state failures with full context; after any error the session is dead | [`07-api.md`](07-api.md) |

## Deferred (decided to defer, on purpose)

| Decision | Where |
|---|---|
| Ext mods (trait, `exteval`, `.ext` section) → VM v2 | [`10-deferred.md`](10-deferred.md) |
| Anomaly taxonomy and check placement | [`10-deferred.md`](10-deferred.md) |
| `dontcare` fill conformance caveat (0-fill, documented) | [`10-deferred.md`](10-deferred.md) |
| Tri-state (`net`/`bus`) modeling | [`10-deferred.md`](10-deferred.md) |
| Multi-clock scheduling details; first-class domains | [`10-deferred.md`](10-deferred.md) |
| Probe-ability restriction / keep-lists | [`10-deferred.md`](10-deferred.md) |
| Watchpoints, mid-run pokes, partial re-evaluation | [`10-deferred.md`](10-deferred.md) |
| Save/restore of a running session (trait hooks reserved now) | [`10-deferred.md`](10-deferred.md) |
| Per-instance ext mod implementation overrides | [`10-deferred.md`](10-deferred.md) |
| Dynamic-index writes | [`10-deferred.md`](10-deferred.md) |
| The program artifact file format | [`10-deferred.md`](10-deferred.md) |
| Commit upgrade to minimal-snapshot | [`10-deferred.md`](10-deferred.md) |
| Out-of-range shift semantics (`todo!()` until decided) | [`10-deferred.md`](10-deferred.md) |
| `Word[65+]` limb mechanics (design intent recorded; unified-opcode kept) | [`10-deferred.md`](10-deferred.md) |
| Exact instruction encoding widths and field layout | [`10-deferred.md`](10-deferred.md) |
