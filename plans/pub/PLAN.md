# PLAN: `pub` — package-level visibility

Date:   2026-10-25
Status: In Planning

## Summary

Introduce `pub` as the visibility keyword on top-level items. A `pub` item may
be referenced from other packages that import it; referencing a non-`pub`
item across a package boundary is an error. `pub` replaces the *visibility*
role that the docs had (inconsistently) ascribed to `export`; `export` itself
is retired from the grammar but stays reserved in the lexer. Top-module
designation moves fully out of the language: it is already driven by
`[prog] top` in `Virdant.toml` and the toolchain.

## Settled decisions (from interview)

1. **Keyword:** `pub` is the visibility keyword. `export` remains a lexer
   keyword but becomes unused in the grammar.
2. **Tops:** tops are controlled by `Virdant.toml` (`[prog] top`) and the
   tooling, *not* by a source keyword. A top module must be non-generic, but
   that is a property of top-ness, not of `pub`: `pub mod`s may be generic.
3. **No type leakage (uniform rule):** every `pub` item's public surface must
   be transitively `pub`:
   - `pub mod`: all port types (`incoming`/`outgoing`/`wire`/`reg`/`bus`)
   - `pub struct` / `pub union`: all field/variant types
   - `pub enum`: all variant types
   - `pub socket`: all `cosi`/`soci` types
4. **Item kinds:** `pub` is allowed on *all* top-level item kinds — `mod`,
   `struct type`, `union type`, `enum type`, `builtin type`, `socket`,
   `platform`.
5. **Error site:** cross-package references to non-`pub` items error at the
   reference site. The `import` statement itself stays package-granular and
   never errors for visibility reasons (an import of a package with zero pub
   items is legal).
6. **Within-package:** `pub` gates only cross-package references. Within a
   package, every item is visible regardless of `pub`.
7. **Builtins:** no compiler special-casing. `lib/builtin.vir` is updated so
   every builtin item is marked `pub` like any other source file.
8. **Generics:** kind params (`T` in `x : Word[T]`) count as `pub` in the
   transitive check — they are bound at instantiation, not leakage.
9. **Future work (explicitly out of scope):** re-exports; per-item import
   syntax (`import buffer::Buffer`).

## Current state (key locations)

- Lexer: `pub` already tokenizes as `KwPub` (`virdant/src/syntax/token.rs:95`);
  `pub`/`export` both in `KEYWORDS` (line ~148). No parser use.
- Grammar: `virdant.lalrpop:132` — ModDef has `<ext:"ext"?> <export:"export"?>`;
  no `pub`, and no modifier slot on Struct/Union/Enum/Builtin/Socket/Platform.
- Payload: `ModDef.is_export` (`payload.rs:189`); other item payloads have no
  visibility flag.
- Checker: `check_export_mod_generics` (`queries/check.rs:722-741`) — the
  non-generics rule, currently anchored to `export mod`.
- Verilog naming: `conversion.rs:160-180` (`build_module_info`) — `export`
  mods emit under their simple name; everything else under the qualified
  `package::Name`. `vir.rs:1108-1140` (`verilog_top_name`) mirrors this so
  yosys `-top` matches the emitted name.
- Tops: already tool-driven. `vir.rs:947` reads `[prog] top` (FQN, e.g.
  `"top::Top"`), used for manifest, synthesis, and platform `for` resolution.
  `Virdant.toml` files exist across `tests/pass/*` (mostly `[project]` only).
- Docs: `packages.rst` and `items.rst` already *describe* export-as-visibility
  (docs/implementation discrepancy this plan resolves); `grammar.rst:118`
  keyword list; root `GRAMMAR.txt`.
- Builtins: `lib/builtin.vir` — no `pub` markers today.

## Design

### 1. Syntax & AST

- `virdant.lalrpop`: replace `<export:"export"?>` with `<pub:"pub"?>` on
  ModDef (same slot: after `ext`); add `<pub:"pub"?>` in the same position to
  StructDef, UnionDef, EnumDef, BuiltinDef, SocketDef, PlatformDef.
- Payload: add `is_pub: bool` to every top-level item payload
  (`ModDef`, `StructDef`, `UnionDef`, `EnumDef`, `BuiltinDef`, `SocketDef`,
  `PlatformDef`). Remove `is_export` (rename, not coexist).
- `KwExport` stays in the lexer and `KEYWORDS` (reserved), so `export` still
  produces a keyword-ish parse error rather than being treated as an
  identifier; remove `KwExport` productions from the grammar.
- AST display (`ast.rs:158`): print `pub ` instead of `export `; extend to the
  other item kinds.

### 2. Visibility check (reference site)

- In cross-package name resolution (symbol resolution of `Ofness` /
  `pkg::Name` references: type positions after `of`/`:` and expression
  roots), when the referenced item lives in a *different* package and the
  item's payload lacks `is_pub`, emit a diagnostic, e.g.
  `` `"Name" is not pub in package "pkg"` ``.
- The implicit builtin import participates like any import; builtins pass
  because `builtin.vir` marks everything `pub` (see §5).
- Same-package references are never gated.

### 3. Transitive no-leakage check

New query (e.g. `queries/check.rs` or a sibling module): for each `pub` item,
walk its public surface — mod port types, struct/union field types, enum
variant types, socket cosi/soci types. For each type:

- Resolve the head item. It is acceptable iff: it lives in the same package,
  it is `pub`, or it is a kind parameter of the enclosing item.
- If the head is a pub struct/union/enum, recurse into its fields/variants.
- Guard against cycles with a visited set.
- Emit a diagnostic at the offending type's span, naming the non-pub item.

### 4. Tops & export retirement

- `check_export_mod_generics` is re-anchored: `export` no longer exists, so
  the *non-generics rule for tops* moves to build time — when `vir.rs`
  resolves `[prog] top`, verify the target is a `mod` (not another item kind)
  and is non-generic; error otherwise.
- `verilog/conversion.rs`: drop the `is_export` branch (naming — see
  Decision D3 below). No `is_export` special-casing remains anywhere
  (`vir.rs::verilog_top_name` likewise).
- `[prog] top` continues to be the single source of truth for tops.

### 5. `lib/builtin.vir`

Add `pub` to every item (builtin types, `Valid`, `Sync`, `Pipe`, …). No code
changes to how the builtin import is seeded (`analysis/package.rs` stays as
is).

### 6. Migration

- All `export mod` occurrences in `tests/pass/*`, `tests/`, examples,
  `verilog/tests.rs`, and the workspace root (`decoder.vir`, `virdant-riscv`,
  `virdant-virt`, `wavir`) become `pub mod` where the module is a top, and
  gain `pub` where they are library items referenced cross-package.
- Non-pub items that today are referenced cross-package (currently unguarded)
  must either be marked `pub` or their references fixed — this migration will
  surface them; treat each as intended-public unless clearly internal.
- Docs: rewrite `packages.rst` ("Exporting Declarations" → `pub`),
  `items.rst:63`, `grammar.rst` keyword list, `GRAMMAR.txt`, and any tutorial
  mentions of `export`.

## Decision points (open)

- **D1 — Diagnostic shape for reference-site errors:** plain error naming the
  item and defining package (recommended), or add a "did you mean to mark it
  pub?" hint.
- **D2 — Naming of the payload flag:** `is_pub` (recommended) vs
  `visibility: Visibility` enum (overkill today, but leaves room for
  `pub(crate)`-style scopes later).
- **D3 — Verilog module naming after `export` retirement.** Today: `export`
  mods get simple Verilog names, others get qualified `pkg::Name`.
  - (D3a, recommended) Emit *every* mod under its qualified name; tops are
    passed to yosys/nextpnr as their escaped FQN. Uniform, collision-free
    (important once builtin's `pub mod Sync` and user packages coexist), and
    `verilog_top_name` collapses to "escaped FQN". Cost: Verilog snapshots
    and any `-top` expectations in tests change.
  - (D3b) Attach simple-name emission to `pub`: minimal migration (existing
    `export mod Top` → `pub mod Top` keeps emitted names), but re-conflates
    visibility with naming and introduces cross-package collision risk for
    same-named pub mods.

## Work items

1. Grammar/parser: `pub` slots on all item kinds; retire `export` productions;
   payloads get `is_pub`; AST display update.
2. Visibility query + diagnostics (reference site).
3. Transitive pub-surface query + diagnostics.
4. Re-anchor top non-generics check to `[prog] top` resolution in `vir.rs`;
   remove `is_export` from conversion and `verilog_top_name` (per D3).
5. `lib/builtin.vir`: mark all items `pub`.
6. Migration sweep: workspace sources, `tests/pass/*`, verilog tests,
   examples, docs, `GRAMMAR.txt`.
7. New tests: parser round-trip for `pub` on each item kind; non-pub
   cross-package reference error; transitive leakage error (mod ports,
   struct fields, socket types); pub-with-generics accepted; same-package
   non-pub reference accepted; top-with-generics rejected at build time.

## Future work (not this plan)

- Re-exports.
- Per-item imports (`import buffer::Buffer`) and import-time visibility
  errors.
- Stale-import lint (importing a package that exposes nothing pub).
- LSP/doc-tooling surfacing of pub-ness (hover, `virdant doc` filtering).