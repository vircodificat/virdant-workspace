# Generics — Types Only (First Slice)

Status: Proposed
Date: 2026-11-27
Derived from: plans/generics/PLANv2.md (accepted architecture)
Related: plans/pipe_socket/CONCEPT.md, plans/PARAMETERS.md,
plans/generics/FLOW_UNIFICATION.md

This is the **types-only first slice** of the generics plan: generic
type defs (struct, enum, union), type arguments on type references,
and the de-hardcoding of `Valid[T]`. It deliberately front-loads all
grammar and AST work — including syntax for features whose *semantics*
are deferred — so that later slices are purely semantic additions with
no grammar churn.

Guiding principle (from PLANv2): **analyze once, generically.** Each
item gets one analysis keyed by its `SymbolId`; generic parameters
appear unbound in a `KindContext`; substitution to a mono type happens
only where a concrete answer is needed.

## 1. Scope

**In scope (semantics implemented):**

* Generic `struct type`, `enum type`, `union type` declarations with
  `[Ident : Kind]` parameters (`Type`, `Nat`).
* Type arguments on **type references**: `reg x : Pair[Word[4], Bit]`,
  `incoming inp : Valid[Word[8]]`, struct fields, enum ctor params.
* The shared substitution machinery, kind checking at type-reference
  sites, and the `Param`/`Usual(..., args)` type representation.
* `Valid[T]` de-hardcoded into an ordinary generic type in `builtin`.

**Out of scope (syntax accepted, semantics deferred with guards):**

* Generic **socket defs** (`builtin socket Pipe[T : Type]`) and their
  instantiation (`of Pipe[Word[8]]`).
* Generic **mod defs** (`mod Fifo[T : Type]`) and their instantiation.
* The mono-socket / mono-module `:=:` matching rule.
* `ext mod` + generics (per PLANv2, permanently out of scope).

## 2. Low-hanging fruit: grammar and AST (all front-loaded)

All parsing support lands in this slice, even for deferred features:

- [ ] `GRAMMAR.txt`:
  * Parameter declarations `[Ident : Kind]` on `StructDef`, `UnionDef`,
    `EnumDef` (implemented), and on `SocketDef`, `ModDef` (parsed,
    guarded — see §3).
  * Type arguments on `Ofness` (`of Pipe[Word[8]]`) — parsed, guarded.
  * Type arguments on `Type` nodes everywhere a type appears —
    implemented.
  * One unified argument syntax: extend the existing `GenericsParams`
    node from bare-number-only to **number-or-type**, with **nesting**
    (the argument to `Valid` in `Valid[Word[8]]` is itself a full
    type). `Word[8]` keeps parsing exactly as today.
- [ ] `src/syntax/` (`payload.rs`, `ast.rs`): AST payloads for
  parameter declarations and argument lists; parser updates; parsing
  round-trip tests for all of the above.
- [ ] `src/diagnostics.rs`: the `NotYetImplemented` diagnostic (§3)
  plus kind-checking diagnostics (§5).

Rationale: grammar and AST are additive and low-risk; landing them now
means the socket/mod slice later changes no syntax and rewrites no
parser code — it flips guards on and implements semantics.

## 3. Not-yet-implemented guards

Features whose semantics are deferred must fail **loudly and clearly**,
never silently or with a confusing downstream error:

* A generic socket/mod **declaration** resolves to an item whose
  instantiation is attempted (e.g., any reference to
  `SocketDef`/`ModDef` carrying parameters) →
  `NotYetImplemented: generic socket defs / mod defs are not yet
  supported` pointing at the parameter list.
* Type arguments on an `Ofness` node referencing a socket or moddef
  (`client socket pipe of Pipe[Word[8]]`, `mod f of Fifo[Word[8]]`) →
  `NotYetImplemented: type arguments on socket/mod instantiation are
  not yet supported` pointing at the argument list.
* Bare instantiation of a parameterized socket/moddef (args omitted) →
  same guard.

Guards live at the instantiation/resolution sites (symbol resolution
and `component.rs`/`elaboration.rs` entry points), so ordinary
non-generic sockets and mods behave exactly as today.

Guard tests (negative, `tests/errors/`): each guarded site produces
its diagnostic, and no downstream expansion happens.

## 4. Type representation

From PLANv2, unchanged:

```rust
pub enum Type {
    Bit, Clock, Reset,
    Word(Width),
    Usual(SymbolId, Vec<Type>),  // user types now carry type arguments
    Valid(Box<Type>),            // transitional; deleted in §6
    Param(ParamId),              // reference to a generic parameter
}
```

* `Param` is legal in generic analyses, forbidden in mono artifacts
  (codegen, sim, widths) — debug assertion panics otherwise.
* `Usual(SymbolId, Vec<Type>)` is the widest mechanical change; every
  destructuring site must account for args (checklist in §7).
* `Nat` arguments are numeric literals only — no evaluator.

Note: in this slice a `Param` can only originate inside a generic
*type def* — kind checking rejects parameter names outside their
declaring def, and sockets/mods cannot be generic yet. This keeps the
`Param` zoning surface small and the debug assertion easy to satisfy.

## 5. Substitution and kind checking

- [ ] `src/types/` — `KindContext` (parameter → kind) attached to
  generic type defs' analyses; one shared `substitute(typ, env)`
  function recursing through `Valid`, `Usual` args, struct/union/enum
  member types. No site hand-rolls substitution.
- [ ] Kind checking at **type-reference instantiation sites**:
  * Arity: `Pair[Bit]` → error, `Pair` takes two arguments.
  * Kind: `Valid[8]` → error (`Type`-kind parameter given a `Nat`
    literal); `Word[Bit]` → error (`Nat` parameter given a type).
  * Unbound parameter names outside their declaring def → error.
- [ ] Cycle detection when resolving arguments (pathological
  self-referential args).
- [ ] `type_width` and type-equality helpers recurse through `Usual`
  args into member types.

## 6. De-hardcode `Valid[T]`

`Valid` becomes an ordinary generic type in `builtin`, roughly:

```virdant
enum type Valid[T : Type] {
    Valid(T)
    Invalid
}
```

with **no compiler support beyond what generic types get**. This
requires generic enums with payload constructor parameters — a ctor
parameter whose type is the enum's own `T`.

- [ ] `src/types/typedef.rs` — remove the `builtin::Valid`
  name-matching (the `builtin::Word` matching stays; `Word` remains
  builtin, per §8).
- [ ] `src/types/typing.rs` — drop the pattern-ctor `Valid`/`Invalid`
  special case; `@Valid(x)` / `@Invalid` become ordinary ctor patterns
  typed through the generic machinery.
- [ ] `src/types/typing/check.rs` — drop the ctor `Valid` cases.
- [ ] `src/types/match_coverage.rs` — exhaustiveness over the
  substituted enum arms.
- [ ] `src/sim/expr.rs` — drop the `payload::Pat::Valid` special path.
- [ ] `src/types/typ.rs` — delete `Type::Valid(Box<Type>)` once sim
  and VCD display go through the ordinary enum machinery
  (transitional retention allowed; target is deletion).

This is the honesty test of the slice: if `Valid` works as a plain
generic type through typing, patterns, coverage, flattening,
simulation, and VCD display, the machinery is real.

## 7. Consumer checklist (substitution at point of use)

Every site that destructures `Type::Usual` must substitute with the
args the type carries, before using member types:

- [ ] `src/types/typedef.rs` — `TypeDef`/`TypeIndex` learn type args;
  generic defs resolved once with members as written
- [ ] `src/types/signature.rs` — ctor signatures per mono type:
  `@Valid(x)` on `Valid[Word[8]]` has parameter type `Word[8]`
- [ ] `src/types/typing/check.rs`, `infer.rs` — substitute into
  field/param types at every `Type::Usual` match
- [ ] `src/types/match_coverage.rs` — patterns over substituted arms
- [ ] `src/verilog/conversion.rs` — flattening (structs → packed bits,
  enums → tag+payload) uses substituted member widths; every
  `Type::Usual(...)` destructure substitutes before `type_width`
- [ ] `src/sim/expr.rs`, `src/sim/vcd.rs` — constructors, patterns,
  printing on substituted types
- [ ] `src/analysis/structs.rs` — `get_struct_fields` returns members
  as written (generic); consumers substitute
- [ ] `src/bin/virlsp.rs` — hover shows substituted member types for
  mono types (`fst : Word[4]`, not `fst : A`)
- [ ] `src/docs/mod.rs` — generic type-def docs render the parameter
  list and members written in terms of parameters

## 8. Deferred to later slices (explicitly)

Carried over from PLANv2, unchanged in intent:

* Generic socket defs and mod defs: syntax lands now (§2), semantics
  later. On landing, they reuse this slice's `substitute`, kind
  checking, and AST unchanged; `component.rs` and `elaboration.rs`
  then tolerate `Param`/apply args per PLANv2 §5 and §9.
* `:=:` matching on the mono socket/module. **Interim behavior**: with
  types-only generics, a client and server socket may still disagree
  (`Valid[Word[8]]` vs `Valid[Word[9]]`); existing type-equality checks
  may catch some cases, but the pointed mono-item diagnostic waits for
  the socket slice. Document this gap in the interim.
* De-hardcoding `Word[n]` — needs Nat-generic built-in behavior;
  deferred until demand exists.
* Kinds beyond `Type`/`Nat`; constraints; default parameters;
  parameterized Verilog emission.

## 9. Testing strategy

1. **Unit tests on `substitute`** — nested args
   (`Valid[Pair[Word[4], Bit]]`), the same parameter twice
   (`Pair[A, A]`), unused parameters, no-`Param` invariant of a fully
   substituted type.
2. **Snapshot tests** (`tests/pass/`): generic struct as a reg type
   (driven, matched, flattened, simulated); `Valid[T]` defined as an
   ordinary generic type used everywhere `Valid` is used today;
   nested args `Valid[Pair[Word[4], Bit]]`; a generic enum
   instantiated at two different args in one module.
3. **Negative tests** (`tests/errors/`): wrong arity; wrong kind
   (`Valid[8]`, `Word[Bit]`); unbound parameter; every §3 guard site;
   `Param` leaking into codegen (debug assertion).
4. **Golden-Verilog equivalence** — byte-compare a generic version and
   a hand-expanded non-generic version of the same design.
5. **Parsing round-trip** — every new grammar production parses to the
   expected AST payload, including the guarded forms (they parse; the
   error comes later, from the guard).
