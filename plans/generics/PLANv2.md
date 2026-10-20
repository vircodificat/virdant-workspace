# Generics — Implementation Plan (v2)

Status: Accepted (point-by-point review, 2026-11-27)
Supersedes: PLAN.md
Related: plans/pipe_socket/CONCEPT.md, plans/PARAMETERS.md, plans/generics/FLOW_UNIFICATION.md

This plan describes how to get **full generics** in Virdant: type
parameters on socket defs, type defs (structs, enums, unions), and mod
defs, all driven by one substitution mechanism.

## 1. The architecture: analyze once, generically

Each item gets exactly **one** analysis keyed by its `SymbolId` — the
name of the item. There are no per-instantiation copies of analyses.

Generic parameters appear in that analysis **unbound**: each parameter
is recorded in a **`KindContext`** with its kind (`Type` or `Nat`), but
is not bound to a concrete type or value. The analysis of a generic
item describes the *shape* of the family, with `T` still standing for
"some type of kind Type".

Substitution into a concrete form happens only where the arguments are
actually known and a concrete answer is required — at use sites
(component expansion needing widths, Verilog generation, the
simulator, connection checks). The concrete result of that
substitution is a **mono** form of the item:

* **mono socket** — a socket instance with its parameters substituted
  (`client socket pipe of Pipe[Word[8]]` is the mono socket of
  `Pipe` at `Word[8]`).
* **mono type** — analogously, a user-defined type with its arguments
  substituted.
* **mono module** — analogously, a mod def instantiation.

Terminology: a *generic item* is the declaration with unbound
parameters; a *mono* socket/type/module is a concrete instantiation
with parameters substituted.

Re-running an analysis per instantiation is done only when the result
genuinely cannot be expressed generically. Analyses are never
duplicated merely because the arguments differ.

## 2. Type representation

`Type` gains two changes:

```rust
pub enum Type {
    Bit, Clock, Reset,
    Word(Width),
    Usual(SymbolId, Vec<Type>),  // user types now carry type arguments
    Valid(Box<Type>),            // transitional; deleted when Valid is de-hardcoded
    Param(ParamId),              // NEW: a reference to a generic parameter
}
```

**`Param` — a parameter reference inside a type.** Needed because
generic analyses run with parameters unbound: the channel type of
`cosi data : Valid[T]` inside `Pipe`'s analysis is represented as
`Valid(Param(T))`. There is no way to represent "the parameter `T`"
in today's enum; every variant is concrete.

`Param` is **zoned**:

* Legal in generic analyses (typing, checking, structural analysis).
* Forbidden in mono artifacts — concrete components, generated
  Verilog, simulator state, computed widths. Any code producing those
  artifacts substitutes parameters away first. Encountering a `Param`
  there is a compiler bug: a debug assertion turns it into a loud
  panic rather than a silently wrong width.

**`Usual` carries type arguments.** `Usual(SymbolId)` can only say
"this is the type `Option`"; it has nowhere to record
"this is `Valid[Word[8]]`". It becomes `Usual(SymbolId, Vec<Type>)`.
Types are used as values in the compiler (equality for connection
checks, width computation, flattening), so the arguments must travel
with the type. This is the widest mechanical change in the plan: every
site that destructures `Type::Usual` must account for the arguments.

## 3. Kinds

Generic parameters are declared with kind annotations. There are
exactly two kinds:

* **`Type`** — the parameter substitutes as a whole type
  (`T` in `Pipe[T]` becomes `Bit`, `Word[8]`, `Valid[Bit]`, ...).
* **`Nat`** — the parameter substitutes as a numeric literal
  (`n` in `Word[n]`). `Nat` arguments are **numeric literals only** —
  not expressions — so no expression evaluator is needed.

```virdant
builtin type Word[n : Nat]
socket Pipe[T : Type]
struct Pair[A : Type, B : Type]
```

Kind polymorphism, higher kinds, and default parameters are out of
scope.

## 4. Syntax

Parameter *declarations* use the existing `[Ident : Kind]` grammar
(today used only by the builtin `Word[n : Nat]`), extended to
`SocketDef`, `StructDef`, `UnionDef`, `EnumDef`, and `ModDef`.

Type *arguments* appear in two new places:

1. **On uses of items** — the `Ofness` node:
   `of Pipe[Word[8]]`, `of Fifo[Word[8]]`, `reg x : Valid[Bit]`.
2. **On type references inside declarations** — anywhere a `Type`
   appears (channel types, field types, component types).

Arguments **nest**: the argument to `Valid` in `Valid[Word[8]]` is
itself a full type. One unified argument syntax handles both kinds:
the existing `GenericsParams` node is extended from
bare-number-only to number-or-type, so `Word[8]` (existing) and
`Valid[Bit]` (new) parse through the same mechanism.

## 5. Socket channel expansion

When a moddef contains a socket instance, `component.rs` expands the
socketdef's channels into components of the containing moddef. This
mechanism is **unchanged**: same paths, flows, component kinds, same
two expansion sites (direct socket; socket belonging to a submodule).

Component types are recorded **as written**, with parameters unbound:
`Valid[T]` may now appear as a component type (via `Param`). The
instantiation's arguments live on the `Ofness` node where they were
written. Substitution happens only in the consumers that actually need
a concrete type.

A pre-existing inconsistency in the two sites' flow-derivation tables
was found during planning; it is tracked separately in
`plans/generics/FLOW_UNIFICATION.md` and is not part of this plan.

## 6. Connections match on mono items

When `:=:` connects two socket paths, the match is on the **mono
socket** — the socket item together with its type arguments — not
merely the item's name. Two instances of the same socketdef with
different arguments are different mono sockets and do not connect;
`Pipe[Word[8]] :=: Pipe[Word[9]]` is an error whose diagnostic names
both instantiations and points at the connection.

The same rule applies to mono modules at submodule boundaries.

## 7. De-hardcoding `Valid[T]`

`Valid` is currently built into the compiler: a hardcoded variant of
`Type` (`Valid(Box<Type>)`), recognized by name-matching in
`typedef.rs`, with special-case code in type checking,
constructor/pattern handling (`@Valid(x)` / `@Invalid`), and the
simulator.

The goal: **`Valid[T]` becomes an ordinary user-defined generic type**,
spelled out in the `builtin` module as roughly:

```virdant
enum type Valid[T : Type] {
    Valid(T)
    Invalid
}
```

with no compiler support beyond what generic types get. The hardcoded
variant, the name-matching, and the `@Valid`/`@Invalid` special cases
are deleted; `@Valid(x)` and `@Invalid` become ordinary constructor
patterns of an ordinary generic enum.

This is the honesty test for the generic-types machinery: if `Valid`
can be defined as a plain generic type and everything still works —
typing, patterns, coverage, flattening, simulation, VCD display — the
machinery is real rather than special-purpose.

The `Type::Valid(Box<Type>)` variant may survive transitionally (sim
and VCD display reach inside it), but the target is its deletion.

`Word[n]` is **not** de-hardcoded by this plan (see §11).

## 8. Taking user-defined types apart

Member types (struct fields, enum constructor parameters, union
members) are analyzed **once, generically**: for
`struct Pair[A : Type, B : Type] { fst : A, snd : B }`, the field list
is `fst : Param(A)`, `snd : Param(B)`.

Every consumer that needs concrete member types — Verilog flattening
(field widths), pattern matching (payload types), the simulator
(constructing and printing members), `virlsp` hover — takes a
`Usual(symbol, args)` plus the generic member list and substitutes:
`Param(A) := Word[4]`, etc.

Mitigations for the scatter risk (many destructuring sites):

* **One shared `substitute` function** in `src/types/`; no site
  hand-rolls its own. It recurses into nested arguments
  (`Valid[Pair[Word[4], Bit]]` substitutes inside-out).
* The debug assertion from §2 (no `Param` in codegen) turns a missed
  site into a loud failure.
* `type_width` and type-equality helpers recurse through `Usual`
  args; they are part of the same checklist of sites.
* Cycle detection when resolving arguments guards pathological
  self-referential arguments.

## 9. Generic mod defs

```virdant
mod Fifo[T : Type] {
    incoming inp : Valid[T]
    outgoing out : Valid[T]
    ...
}
```

Submodules are elaborated today by **inlining** — `elaborate_module`
recurses once per instance with a fresh path prefix. Each instance
already carries its own type arguments on its own statement
(`mod f1 of Fifo[Word[8]]`). So the change is small:

* When inlining an instance, read its type arguments; for that walk,
  the parameters are known in that context.
* Types mentioning `T` are resolved in that context: substituted where
  a concrete answer is needed (port widths, `it.` references,
  Verilog), left unbound where the generic form suffices.
* The same walk happens per instance, so
  `mod f1 of Fifo[Word[8]]` and `mod f2 of Fifo[Word[16]]` produce
  mono modules with the right port types.

Connections between instances follow §6: item + arguments.

**Out of scope: `ext mod` + generics.** External Verilog modules use a
different parameterization mechanism (see `plans/PARAMETERS.md`).

## 10. Kind checking

Arguments are validated **at the instantiation site itself** — socket
instances, submodule instances, and type references all go through one
argument-checking routine:

* **Arity**: `Pair[Bit]` is an error — `Pair` takes two arguments.
* **Kind**: `Valid[8]` is an error — the parameter has kind `Type`,
  and `8` is a `Nat` literal; `Word[Bit]` is an error — the parameter
  is a `Nat`.

Downstream machinery may assume the arguments are well-formed. This
keeps diagnostics pointed: errors name the args the user wrote at the
site, not a downstream expansion artifact.

## 11. Out of scope

1. **De-hardcoding `Word[n]`.** `Word` is wired into literals,
   slicing, concatenation, bit operations, and width computation on
   every path; making it ordinary requires Nat-generic support for
   built-in *behavior*, not just structure. Deferred until demand
   exists (e.g., a user-written `Vec[N, T]` type).
2. **Kinds beyond `Type` and `Nat`** — no kind polymorphism, no
   higher-kinded parameters.
3. **Constraints** — no where-clauses or trait-style restrictions.
4. **Default parameters.**
5. **Emitting parameterized Verilog** instead of monomorphized copies.
   Deduplicating identical specializations is a possible future
   size/compile-time optimization; it changes nothing semantically.

## 12. Testing strategy

1. **Unit tests on `substitute`** — nested args
   (`Valid[Pair[Word[4], Bit]]`), the same parameter twice
   (`Pair[A, A]` with both args equal), unused parameters, and the
   invariant that a fully-substituted type contains no `Param`.
2. **Snapshot tests per feature** (`tests/pass/`): a generic socket
   (the `Client`/`Server`/`Top` example from
   `plans/pipe_socket/CONCEPT.md`), `Valid[T]` defined as an ordinary
   generic type, a generic struct as a reg type, a generic mod
   instantiated twice with different args in one top module.
3. **Negative tests** (`tests/errors/`): wrong arity; wrong kind
   (`Valid[8]`, `Word[Bit]`); `:=:` between mismatched mono
   sockets/modules; `Param` leaking into codegen (debug assertion).
4. **Golden-Verilog equivalence** — byte-compare the generated
   Verilog of a generic version and a hand-expanded non-generic
   version of the same design, proving substitution is
   semantics-preserving.

## 13. Documentation

* A generic item's doc page shows its parameter list
  (`T : Type`) and members whose types are written in terms of the
  parameters (`data : Valid[T]`).
* Consumers displaying types to the user (e.g. `virlsp` hover) show
  substituted member types for mono types: hovering a reg declared
  `Pair[Word[4], Bit]` shows fields `fst : Word[4]`, `snd : Bit`.

## 14. File-level work items

Core:

- [ ] `GRAMMAR.txt` — parameter declarations on
      Socket/Struct/Union/Enum/ModDef; type args on Ofness and type
      references (nesting, number-or-type)
- [ ] `src/syntax/` — parser + AST payloads for the above
- [ ] `src/types/typ.rs` — `Type::Param(ParamId)`;
      `Usual(SymbolId, Vec<Type>)`
- [ ] `src/types/` — `KindContext`; one shared `substitute` function
- [ ] `src/diagnostics.rs` — kind-checking diagnostics (arity, kind,
      mono-item connection mismatch)
- [ ] Kind checking at every instantiation site

Sockets:

- [ ] `src/analysis/component.rs` — both expansion sites tolerate
      `Param` in component types (no substitution here)
- [ ] `src/queries/` + `src/analysis/drivers.rs` — `:=:` matches on
      the mono socket; mismatched instantiations diagnosed
- [ ] `src/docs/mod.rs` — generic item docs

Types:

- [ ] Substitution applied at every `Type::Usual` consumer:
      `typedef.rs`, `signature.rs`, `typing/check.rs`, `infer.rs`,
      `match_coverage.rs`, `verilog/conversion.rs`, `sim/expr.rs`,
      `sim/vcd.rs`, `analysis/structs.rs`, `bin/virlsp.rs`
- [ ] `type_width` / type-equality recurse through `Usual` args
- [ ] Debug assertion: no `Param` in codegen/sim/width paths
- [ ] De-hardcode `Valid`: remove name-matching in `typedef.rs`, the
      `@Valid`/`@Invalid` special cases in typing/check and sim, and
      (finally) `Type::Valid`

Mod defs:

- [ ] `src/analysis/elaboration.rs` — read type args per instance;
      elaborate with parameters in context
- [ ] `src/analysis/component.rs`, `src/queries/typing.rs`,
      `src/verilog/conversion.rs` — instance port types / `it.`
      resolution with args applied where concrete
- [ ] `:=:` on mono modules at submodule boundaries
