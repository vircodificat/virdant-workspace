# Concept: pub

- pub becomes the visibility keyword on all top-level item kinds; export stays reserved in the lexer but leaves the grammar.
- Referencing a non-pub item from another package errors at the reference site; import itself never visibility-errors.
- No type leakage: every pub item's public surface (mod ports, struct/union fields, enum variants, socket cosi/soci) must be transitively
  pub; kind params count as pub.
- Within a package, all items are visible regardless of pub.
- Tops leave the language: [prog] top in Virdant.toml + tooling is the sole top designation; the non-generics rule re-anchors to top
  resolution, not pub (pub mods may be generic).
- lib/builtin.vir gets pub on every item; no builtin special-casing in the compiler.
- Migration: all export mod → pub mod, docs rewritten (packages.rst already claimed export-as-visibility), GRAMMAR.txt updated.
- Open: D3 — Verilog naming after export dies; recommended is always-qualified names with escaped FQN -top (avoids collisions once builtin's
  pub Sync coexists with user mods); D1/D2 minor.
- Future work: re-exports, per-item imports, stale-import lint.

## Visibility rules (reference-site)

**V1. `pub` marks top-level items only.** Valid on `mod`, `struct type`, `union type`, `enum type`, `builtin type`, `socket`, `platform`. Items inside a `mod` body (wires, regs, submodules, instances) are never `pub` — they're not items.

**V2. Same package: no gate.** All items in the *defining* package are visible to all code in that package, `pub` or not. `pub` only matters at package boundaries.

**V3. Cross-package reference requires `pub`.** Any reference of the form `pkg::Name` where `pkg` ≠ the current package resolves successfully only if `Name` is `pub` in `pkg`. Otherwise: error at the reference span, naming the item and package.

**V4. Where cross-package references occur (all gated):**
- Type positions: `: Type` (ports, wires, regs, fields, params), `of` clauses on submodules
- Expression positions: constructors, enum variants, references to items (e.g. builtin types)
- Platform `for` clauses resolving through imports

**V5. The `import` statement is not gated.** `import buffer` is legal regardless of what (or whether) `buffer` exports. Visibility errors fire only when a name is actually *used*. Duplicate-import and unresolved-package errors are unchanged.

**V6. The builtin import behaves like any import.** It's seeded implicitly (not written in source) but applies V3/V4 identically — builtins pass only because `builtin.vir` marks everything `pub`. No compiler exemption.

**V7. Import chains don't leak visibility.** `A` importing `B`, and `B` importing `C`, does not let `A` see `C`'s items unless `A` also imports `C` (and they're `pub`). Visibility is strictly per-import. (Re-exports — `A` re-exposing `B`'s items under `A::` — are explicitly future work; today nothing is re-exported.)

**V8. Circular imports remain forbidden** — unchanged by `pub`.

**V9. Generics are not gated by `pub`.** A `pub mod` may be generic. Kind parameters (`T` in `Word[T]`) are treated as `pub` wherever they appear in a type, because they're bound at the instantiation site.

## Transitive no-leakage rules (pub-surface check)

**T1. The principle:** if an item is `pub`, everything nameable in its public surface must be reachable through `pub` items. A `pub` item may not expose a non-`pub` item to other packages, directly or through a chain of `pub` types.

**T2. What counts as a public surface, per item kind:**
- `pub mod`: the type of every port — `incoming`, `outgoing` (plain/wire/reg), buses
- `pub struct`: the type of every field
- `pub union`: the type of every variant's parameters
- `pub enum`: the type of every variant's parameters (and variant payload types)
- `pub socket`: the type of every `cosi` and `soci`
- `pub platform`: surfaces per its component statements (mod-level; components are mods, checked as mods)

**T3. The reachability rule for a type `T` appearing in a pub surface:**
- If `T`'s head is a **kind parameter** of the enclosing item → OK (V9).
- If `T`'s head is defined in the **same package** → OK (V2).
- If `T`'s head is `pub` (whatever package) → OK, and **recurse** into its public surface (T4).
- Otherwise (non-`pub`, cross-package) → error at the span of the offending type, naming the non-pub item.

**T4. Recursion.** When a pub surface type is itself a `pub` struct/union/enum, its fields/variants are examined by the same rule, transitively, with a visited-set to handle recursive types (e.g. a struct referencing itself through `Word[n]` indirection). Cycle = allowed (visited set), not an error — the language already has its own cycle rules for mods.

**T5. Concrete examples of what trips T3:**
- `pub mod Foo { incoming x : PrivStruct }` → error: `PrivStruct` is not pub
- `pub mod Foo { incoming c : Clock }` → OK (`Clock` is a pub builtin)
- `pub mod Foo { incoming x : Word[8] }` → OK (`Word` pub, `8` is a nat)
- `pub mod Foo[T : Type] { incoming x : T }` → OK (kind param)
- `pub struct Pair { a : Inner, b : Word[4] }` where `Inner` is non-pub → error
- `pub struct Pair { a : PubInner }`, `pub struct PubInner { x : Priv }` → error (leak through two pub layers)
- `pub socket Chan[T : Type] { cosi client : T }` → OK (kind param); `cosi client : Priv` → error

**T6. Type arguments are checked, not just heads.** If a pub item is generic over `Type` and a pub surface instantiates it with a non-pub type argument (`Valid[Priv]`), the type argument is part of the surface and the head check applies to it too: the argument type must itself pass T3.

**T7. Non-pub items are unconstrained internally.** A non-`pub` mod/struct/socket may reference anything in its own package (V2) and any `pub` item of imported packages (V3). The leakage check runs *only* on items marked `pub`.

**T8. Diagnostics point at the leak site, not the pub item.** The error span is the offending type reference within the pub item's declaration, naming the non-`pub` item that must be marked `pub` (or the surface rewritten). This is D1's shape question, but the *location* rule stands: at the type, not at the `pub` keyword.

**T9. `pub` on `builtin type` is checked like anything else** — builtin types have no fields/ports, so their surface is trivially empty; the rule is uniform, the check just terminates immediately.

**T10. Order of checks.** Visibility (V3) and leakage (T3) are independent passes over the same item data; a leakage error and a visibility error can fire in the same compile, and fixing one never requires the other to pass first.
