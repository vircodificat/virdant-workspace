# Generics — Full Implementation Plan

Status: In Planning
Date: 2026-11-27

This plan describes what it would take to get **full generics** in
Virdant: type parameters on socket defs, type defs (structs, enums,
unions), and mod defs, all driven by one substitution core.

The guiding constraints come from how the compiler actually works today:

* The query cache (`db_getter!` in `virdant/src/db.rs`) keys every
  analysis by `SymbolId` or `Location` — there is no room for type
  arguments in those keys.
* Submodules are elaborated by *inlining*: `elaborate_module`
  (`src/analysis/elaboration.rs`) recurses once per instance with a
  fresh path prefix.
* Socket defs are expanded *structurally*: each `cosi`/`soci` channel
  becomes a plain `Component` (two expansion sites in
  `src/analysis/component.rs`), and Verilog generation never sees a
  socketdef at all (`src/verilog/conversion.rs` matches channels by
  name-suffix + flow in `collect_bidirectional_assigns`).
* `Type` is a closed enum (`src/types/typ.rs`):
  `Bit | Clock | Reset | Word(n) | Valid(Box<Type>) | Usual(SymbolId)`,
  with `builtin::Word` and `builtin::Valid` resolved by *name matching*
  in `src/types/typedef.rs` (~223–290).

These facts make one implementation strategy clearly better than the
others, and the whole plan hangs off it.

---

## 1. The architectural decision: monomorphize

**D1 — Substitute early (monomorphize), do not thread environments.**

Each *instantiation* of a generic item — `Pipe[Word[8]]`,
`Fifo[Valid[Word[8]]]`, `mod f of Fifo[Word[8]]` — is interned as a
fresh **specialization** early (see D5). Analyses that today are keyed
by `SymbolId` (`get_component_analysis`, `get_typing_context`,
`typecheck`, `get_typedef`, ...) are run against the specialization's
own identity, so **every existing `db_getter!` signature stays
unchanged**.

The rejected alternative — parameterizing every query with a type
environment (`Vec<Type>`) threaded through the cache — would change
every getter signature, every caller, and cache-coherence reasoning.
That is the path where generics become a rewrite.

**D2 — The substitution invariant.**

`Type` gains a variant:

```rust
pub enum Type {
    Bit, Clock, Reset,
    Word(Width),
    Valid(Box<Type>),          // transitional, until Phase 4
    Usual(SymbolId, Vec<Type>), // D4: user types now carry type args
    Param(ParamId),             // NEW: only in pre-specialization types
}
```

Invariant: **no `Type::Param` survives past specialization.** The
specializer substitutes parameters into every type it emits; all
downstream destructuring code (`Type::Usual` matches in
`typing/check.rs`, `typing/infer.rs`, `match_coverage.rs`, `sim/`,
`verilog/conversion.rs`) continues to see only concrete types. This is
what keeps the change *wide-but-shallow* instead of a rewrite.

**D3 — `Type::Usual` carries type arguments.**

User-defined types become `Usual(symbol_id, Vec<Type>)`. Every site
that today destructures `Type::Usual(symbol_id)` must substitute into
member/parameter/field types using the carried args. This is the widest
mechanical change in the plan; a missed site is a silent width bug, so
Phase 3 is done exhaustively with a checklist.

**D4 — Kinds.**

`Generics := "[" Ident ":" Kind "]"` already exists in `GRAMMAR.txt`
(used by `builtin type Word[n : Nat]`). Kinds are `Type` and `Nat`:

* `Type` parameters substitute as types (`T` in `Pipe[T]`).
* `Nat` parameters substitute as width expressions (`n` in `Word[n]`);
  they reuse the constant-expression evaluator already built for enum
  widths (`eval_const_expr` in `src/types/typedef.rs`).

**D5 — Where specializations live.**

New interned identity: a `SpecializationId` (or a `SymbolKind::
SpecializedItem` wrapping `(SymbolId, Vec<Type>)`) plus a
`Specializations` table built as a query over the symbol table +
instantiation sites. Instantiation sites (`mod f of Fifo[Word[8]]`,
`socket pipe of Pipe[Word[8]]`, type references `Valid[Word[8]]`) each
produce one specialization. Symbol resolution matches on the *name*
only (`Pipe`), ignoring type args, then records the args.

---

## 2. Target syntax

```virdant
// Generic socket def (motivating case; plans/pipe_socket/CONCEPT.md)
builtin socket Pipe[T : Type] {
    cosi data : Valid[T]
    soci ready : Bit
}

// Generic enum — this is how Valid stops being builtin
enum type Option[T : Type] {
    None = 0
    Some(T)
}

// Generic struct
struct type Pair[A : Type, B : Type] {
    fst : A
    snd : B
}

// Generic mod def
mod Fifo[T : Type] {
    incoming inp : Valid[T]
    outgoing out : Valid[T]
    ...
}

// Instantiation sites — type args on Ofness everywhere:
socket pipe of Pipe[Word[8]] { ... }
mod fifo of Fifo[Word[8]] { ... }
reg x : Pair[Word[4], Option[Bit]] { ... }

// Nat parameters, same machinery:
type type Vec[N : Nat, T : Type] { ... }   // future; see Phase 6
```

Constructor/pattern use is unchanged in shape:
`@Some(x)` / `case @Some(y) => ...`, with `y` bound at the *substituted*
payload type.

---

## 3. Phases

Each phase is independently landable and leaves the compiler green.

### Phase 1 — Core: parameters, substitution, specialization

The one deep change; everything after is plumbing onto it.

**Grammar / AST** (`GRAMMAR.txt`, `src/syntax/`):

* `SocketDef`, `StructDef`, `UnionDef`, `EnumDef`, `ModDef` accept
  `Generics?` (production already exists for builtin types).
* `Ofness` accepts optional `GenericsParams` (type args at
  instantiation sites): `of Pipe[Word[8]]`. Note `GenericsParams`
  today only holds a `Nat` spelling for `Word[n]`; extend to hold
  either a `Nat` spelling or a `Type` node (it must now nest: the arg
  to `Valid` is itself a full type).
* Type references anywhere (`Type` nodes in component/field/param
  declarations) get the same optional args.

**Type representation** (`src/types/typ.rs`):

* Add `Type::Param(ParamId)` where `ParamId` identifies the declaring
  item + parameter index/name.
* Change `Type::Usual(SymbolId)` → `Type::Usual(SymbolId, Vec<Type>)`.
  This alone touches every `Type::Usual` destructure; do it as part of
  Phase 1 with the Phase-3 checklist applied trivially (args always
  empty until Phase 3 introduces non-empty ones).

**Substitution engine** (`src/types/`, new file, e.g. `subst.rs`):

* `substitute(typ, env) -> typ` where `env: &[(ParamId, Type)]`.
  Recurses through `Valid`, `Usual` args, struct/union/enum field types
  (via `get_struct_fields` / signatures), and `Nat`-typed width
  expressions (evaluate via the const-expr path; see D4).
* `check_type_args(def_generics, given_args) -> Vec<Diagnostic>`:
  arity, kind matching, and no-`Param`-leakage validation.

**Specialization table** (`src/analysis/`, new):

* Query `get_specializations : Specializations() -> Arc<Specializations>`
  walks all instantiation sites and interns `(item, args)` pairs.
* A specialization carries: source item `SymbolId`, resolved args,
  and its own stable id usable as a key for the existing per-item
  queries.

**Verification**: a generic *socket def* parses; instantiating it
yields a specialization; substitution runs; nothing downstream is
generic yet.

### Phase 2 — Generic socket defs (`Pipe[T]`)

The motivating case, and the cheapest consumer of the core.

**`src/analysis/component.rs`** — the two channel-expansion sites
(submodule-socket case ~281–340, direct-socket case ~373–408):

* Resolve the instantiation's type args from the `Ofness` node.
* Resolve channel types with the env (`T := Word[8]`) instead of bare
  `builder.get_type_at(typ_node.location())` on the socketdef's AST
  (where `T` is unbound).
* Flow mapping is unchanged (role × dir), **but** first resolve the
  existing inconsistency: the two sites map `(Client, Cosi)` to
  opposite flows (`Flow::Source` vs `Flow::Sink`). Confirm which
  perspective is intended before building on either.

**Typing of `it.` paths** (`src/queries/typing.rs`): the
`Socket(socket)` ancestor cases (~55, ~339, ~423, ~575) already resolve
`it.data` through the socket instance; once component types are
substituted, `it.data := @Valid(0w8)` typechecks against
`Valid[Word[8]]` with no further change expected — verify, don't
rewrite.

**Connection compatibility** (`src/queries/typecheck.rs`): today
`:=:` matching is by name-suffix + flow only (`collect_bidirectional_
assigns` in `verilog/conversion.rs` ~1901, and driver checks). Add a
typecheck rule: for each matched channel pair, substituted types must
be equal. `Pipe[Word[8]] :=: Pipe[Word[9]]` is a compile error with a
pointed diagnostic (name the mismatched instantiation).

**Docs** (`src/docs/mod.rs`): socketdef doc generation learns to
render generic params and per-instantiation member types.

**Success test**: the full `plans/pipe_socket/CONCEPT.md` example
(`Client` / `Server` / `Top` with `slave.pipe :=: master.pipe`) passes
check, elaborates, and generates correct Verilog. No Verilog-side
changes should be needed — sockets expand to wires regardless of
generics.

### Phase 3 — Generic type defs (structs, enums, unions)

The widest phase; purely mechanical given Phase 1, but exhaustive.

Apply the substitution rule at **every** `Type::Usual` consumer:

| Site | Work |
|---|---|
| `src/types/typedef.rs` | `TypeDef`/`TypeIndex` learn type args; remove the `builtin::Word`/`builtin::Valid` name-matching (after Phase 4) or gate it behind "is generic" |
| `src/types/signature.rs` | ctor signatures become per-specialization: `@Some(x)` on `Option[Word[8]]` has parameter type `Word[8]` |
| `src/types/typing/check.rs` + `infer.rs` | substitute into field/param types everywhere `Type::Usual` is matched (~7 sites) |
| `src/types/match_coverage.rs` | pattern exhaustiveness over substituted enum arms (~3 sites) |
| `src/verilog/conversion.rs` | flattening (structs → packed bits, enums → tag+payload) reads substituted field widths; every `Type::Usual(...) else unreachable!()` (~lines 822–1322) substitutes before calling `type_width` on members |
| `src/sim/expr.rs`, `sim/vcd.rs` | simulator constructors/patterns/prints use substituted types (~9 sites) |
| `src/analysis/structs.rs` | `get_struct_fields` keyed by specialization so field types come back already substituted (preferred) or raw-with-args |
| `src/bin/virlsp.rs` | hover/completion shows substituted member types |

**Decision within this phase**: prefer making downstream queries
(*struct fields, ctor signatures, typedefs*) answer in
*substituted* form when queried through a specialization, so consumers
don't each re-substitute. This concentrates substitution in fewer,
deeper places.

**Success test**: generic `struct Pair[A, B]` usable as a reg type,
driven, matched, flattened, simulated; nested generic args
(`Pair[Option[Word[8]], Bit]`) flatten with correct widths.

### Phase 4 — De-hardcode `Valid[T]` (generic enums with payloads)

`Valid[T]` is a unit-arm + payload-arm enum. Spelling it out is the
proof that Phase 3's enum machinery is real, and it deletes four
special-case sites:

* `src/types/typedef.rs` `builtin::Valid` name-matching.
* `src/types/typing.rs` pattern-ctor case for `Valid`/`Invalid`
  (~723–760).
* `src/types/typing/check.rs` ctor cases (~503, ~667).
* `src/sim/expr.rs` pattern cases (~122, ~469) and the
  `payload::Pat::Valid` variant they produce.

Requirements: generic enums with constructor *parameters* (`Some(T)`)
— a ctor parameter whose type is the enum's own `T` — plus `enum`
flattening (tag + payload) that uses the substituted payload type.
`@Valid(t)` typing falls out of the ordinary ctor path once `Valid` is
an ordinary generic enum.

Keep `Type::Valid(Box<Type>)` as a *transitional* variant only if the
sim/VCD display code still needs it; target deletion by the end of
this phase.

**Do not** de-hardcode `Word[n]` in this plan (see Phase 6).

### Phase 5 — Generic mod defs

Cheaper than Phase 3 because of the inlining elaboration model:
`elaborate_module` already recurses per instance, so a generic moddef
is naturally elaborated once per instantiation.

* `src/analysis/elaboration.rs`: at each `Submodule` instantiation,
  resolve type args; elaborate against the specialization (the
  instance's components get substituted types; drivers inside the body
  typecheck against them via the existing per-moddef
  `DriverAnalysis`/`TypeCheck` queries, which now run per
  specialization).
* `src/analysis/component.rs`: the submodule case resolves instance
  port types from the specialization.
* `it.` path resolution (`src/queries/typing.rs` `Submodule` cases,
  `src/verilog/conversion.rs` `it_context`) uses substituted port
  types.
* Connection compatibility from Phase 2 applies to submodule boundary
  ports too.
* `ext mod` + generics is out of scope (Verilog module parameters are
  a different mechanism; see `plans/PARAMETERS.md`).

**Success test**: generic `Fifo[T]` module instantiated at two
different `T`s in one top module; both elaborate, generate, and
simulate; connecting mismatched instantiations is a type error.

### Phase 6 — Explicitly out of scope / future

* **De-hardcoding `Word[n]`.** `Word` is special-cased everywhere:
  literals, slicing, concatenation, bit ops, `type_width` on every
  path. Needs Nat-generic *builtin-with-behavior* support. Revisit
  only after `Vec[N, T]`-style types create demand.
* Kind polymorphism beyond `Type`/`Nat`.
* Where-clauses / trait-style constraints (e.g. "T is a bounded
  type").
* Higher-kinded or default parameters.
* Emitting parameterized Verilog modules instead of monomorphizing
  (IR-level sharing of identical specializations is a size/compile-time
  optimization, not a semantics change).

---

## 4. File-by-file change checklist

Core (Phase 1):

- [ ] `GRAMMAR.txt` — Generics on Socket/Struct/Union/Enum/ModDef; type args on Ofness and Type refs
- [ ] `src/syntax/` — parser + AST payloads (`payload.rs`, `ast.rs`) for the above
- [ ] `src/types/typ.rs` — `Type::Param(ParamId)`; `Usual(SymbolId, Vec<Type>)`
- [ ] `src/types/subst.rs` (new) — substitution + arity/kind checking
- [ ] `src/analysis/` (new) — specialization table + query in `src/db.rs`
- [ ] `src/diagnostics.rs` — arity/kind/mismatch/Param-leak diagnostics

Sockets (Phase 2):

- [ ] `src/analysis/component.rs` — both channel-expansion sites substitute; settle the `(Client, Cosi)` flow inconsistency first
- [ ] `src/queries/typecheck.rs` — connection type-compatibility rule
- [ ] `src/docs/mod.rs` — generic socketdef docs
- [ ] `src/queries/typing.rs` — verify `it.` paths (expected: no change)

Types (Phase 3):

- [ ] `src/types/typedef.rs`, `signature.rs`
- [ ] `src/types/typing/check.rs`, `infer.rs`
- [ ] `src/types/match_coverage.rs`
- [ ] `src/verilog/conversion.rs` (all `Type::Usual` destructures + flattening widths)
- [ ] `src/sim/expr.rs`, `src/sim/vcd.rs`
- [ ] `src/analysis/structs.rs`
- [ ] `src/bin/virlsp.rs`

Enums-with-payloads / Valid (Phase 4):

- [ ] `src/types/typedef.rs` — drop `builtin::Valid` name-matching
- [ ] `src/types/typing.rs` — drop pattern-ctor Valid case
- [ ] `src/types/typing/check.rs` — drop ctor Valid cases
- [ ] `src/sim/expr.rs` — drop `payload::Pat::Valid` special path
- [ ] `src/types/typ.rs` — delete `Type::Valid` if display code allows

Moddefs (Phase 5):

- [ ] `src/analysis/elaboration.rs` — resolve args at Submodule sites; elaborate per specialization
- [ ] `src/analysis/component.rs` — submodule port types from specialization
- [ ] `src/queries/typing.rs`, `src/verilog/conversion.rs` — `it.` substitution

---

## 5. Risks and mitigations

1. **A missed `Type::Usual` destructure silently miscompiles a width.**
   Mitigation: Phase 3 is checklist-driven (table above); add a debug
   assertion that `Type::Param` never reaches flattening/sim code —
   this turns missed substitutions into loud panics in debug builds.
2. **The `(Client, Cosi)` flow discrepancy in `component.rs`.**
   Mitigation: resolve before Phase 2 lands; add a test pinning the
   intended directionality.
3. **Specialization explosion** (many instantiations → many elaborated
   copies). Mitigation: irrelevant at Virdant's scale for now; note as
   a future optimization (dedup identical specializations).
4. **Recursion in type args** (`T[X]` instantiated with itself).
   Mitigation: detect cycles when interning specializations; the
   const-expr evaluator already needs cycle detection for Nat params.
5. **Error-message quality.** Generic mismatches can produce confusing
   "no matching component" errors if they surface in driver analysis.
   Mitigation: arity/kind checks at the instantiation site
   (`check_type_args`) so most errors point at the args, not the
   expansion.

## 6. Testing strategy

* Unit: `substitute` over nested/negative cases (`Option[Option[Word[8]]]`,
  `Pair[A, A]` with both args equal).
* Snapshot: `tests/` cases per phase —
  generic socket (the `Pipe` example), generic enum with payload ctors,
  generic struct flattening, generic mod instantiated twice with
  different args.
* Negative: wrong arity, wrong kind, `:=:` between mismatched
  instantiations, Param leaking into a non-generic context.
* Golden-Verilog: byte-compare generated Verilog of a generic vs
  hand-expanded non-generic pair to prove monomorphization is
  semantics-preserving.

## 7. Sequencing summary

Phase 1 is the only *deep* work. Phase 2 is the motivating payoff and
validates the core. Phase 3 is wide-but-mechanical. Phase 4 is the
honesty test (delete the `Valid` special cases) and Phase 5 is likely
small once the above lands. If only Phase 1 + 2 ship, Virdant already
has useful generic protocol sockets; the remaining phases convert the
hardcoded `Word`/`Valid` corners into ordinary users of the same
machinery.
