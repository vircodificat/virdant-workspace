# Virdant Simulator VM — Clean-Slate Design

This directory is the design record for a from-scratch redesign of the Virdant
simulator VM.
It is the product of a structured design interview; every substantive decision
below was made explicitly, with rationale recorded.

The intent is a **clean-slate replacement**:
the existing implementation will be thrown away, and this document is the
specification the new implementation is built from.

## Reading order

1. [`01-goals.md`](01-goals.md) — what this VM is for, target numbers, standing principles.
2. [`02-semantics.md`](02-semantics.md) — the simulation semantics the VM must preserve.
3. [`03-architecture.md`](03-architecture.md) — the compiler pipeline, the role of the
   bytecode, and the verification strategy.
4. [`04-machine.md`](04-machine.md) — the machine model: slots, state, the cycle, commit.
5. [`05-ir.md`](05-ir.md) — the program notation: blocks, temporaries, terminators.
6. [`06-instruction-set.md`](06-instruction-set.md) — the draft instruction set.
7. [`07-api.md`](07-api.md) — the embedder-facing API: locks, peek/poke, tick, IDs, tracing.
8. [`08-extmods.md`](08-extmods.md) — ext mods: declaration, binding, lifecycle, ABI.
9. [`09-worked-example.md`](09-worked-example.md) — one small design, compiled end to end.
10. [`10-deferred.md`](10-deferred.md) — everything explicitly deferred or out of scope.

## The design in one paragraph

The simulator compiles an elaborated Virdant design into a **flat, statically
ordered program** for a tiny word-level machine.
The program is divided into two sections: a **COMB section**, executed at
*settle* (recomputing all combinational values), and a **NEXT section**,
executed at *tick* (computing every register's next value).
Register updates are *staged* into shadow slots and applied atomically by the
machine at commit.
Control flow uses MLIR-style **blocks with parameters and forward tail-calls
only** — no loops, no phis, no stack.
Values are untyped bit-massaging in the machine; all typing, naming, and
probing live in a compile-time side table keyed by **opaque session-scoped
IDs**.
Behavior the VM cannot compute (memories, UARTs, USB) is delegated to
**ext mods**: embedder-supplied implementations bound by ext mod name,
invoked through a two-hook trait (`eval` / `tick`).
The embedder drives the simulation through an RWLock-style API
(`lock_mut` / `lock` / `tick(clk)`), and the sim library owns tracing via
phase-boundary hooks.

## Status

All structural decisions are made and recorded.
The remaining open items are mechanical (instruction encoding, wide-value
slow paths, slot layout) and are listed in [`10-deferred.md`](10-deferred.md).
