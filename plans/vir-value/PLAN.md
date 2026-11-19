# Plan: `vir value` — standalone value encode/decode

Date:   2026-11-09
Status: Draft

A CLI tool that, given a Virdant project, encodes values to bit
patterns and decodes bit patterns to values, for any type in the
project (pub or non-pub) — without elaborating a design.

```
$ vir value encode @Valid(0) : Valid[Word[8]]
0b0000000010
$ vir value encode @Valid(0w8)
0b0000000010
$ vir value encode @Valid(0)
ERROR: Can't infer type
$ vir value decode 0b0000000010 Valid[Word[8]]
@Valid(0)
```

(The examples' bit patterns follow the actual `sim` layout rules —
enums are one-hot, so `Valid`'s tag is `10` in the low 2 bits: 10
bits total for `Valid[Word[8]]`.)

## 1. Decisions (from planning interview)

- **Standalone, no elaboration.** Works from the project's type
  definitions only. No `Sim`, no top module, no program context.
- **Unify the layout.** A new standalone layout query computes type
  layouts from typedefs; `sim`'s layout machinery converges onto it so
  the bit-layout rules live in exactly one place.
- **New parser, no reuse of `syntax`.** A dedicated lalrpop grammar for
  the value-expression subset lives in `vir/src/value/`.
- **Value grammar is a sturdy subset**: literals (all forms), ctors and
  enumerants, struct literals, parens, ascription. No operators, no
  `if`/`match`/`let`, no paths/variable references, no indexing, no
  `->`, no `tap`, no `Str`, **no `dontcare` and no `?`**.
- **Type names are fully-qualified** (`pkg::Name`), except `builtin`
  package names, which are bare (`Bit`, `Word[8]`, `Valid[Word[8]]`).
- **`decode` requires exact width** — the bit pattern's length must
  equal the type's layout width. No implicit zero-extension.

## 2. CLI

New companion binary `vir-value` (in `vir/Cargo.toml` beside
`vir-poke`), exec'd by `vir value` via the existing dynamic companion
discovery in `vir/src/vir/cli.rs`. Project resolution via
`vircommon::add_project_args` (same as `vir poke`).

```
vir value encode <ValueExpr> [":" <Type>]
vir value decode <BitPattern> <Type>
```

- `<Type>` syntax: `Ident[", ...]` for builtin names, `pkg::Ident[...]`
  otherwise. Resolved through `SymbolTable::resolve_item_fqn` (which
  already handles the `::` qualification), so **non-pub types work**.
- On `encode`, the expected type comes from the command-line `: <Type>`
  ascription or an in-expression ascription. Every `Nat` literal's
  width must be determined by one of these; otherwise
  `ERROR: Can't infer type`.
- `<BitPattern>` accepts the same literal syntax as values
  (`0b1010_0101`, `0xDEAD_BEEF`, `_` separators, `w<N>` suffixes);
  its effective bit width after expansion must equal the type's layout
  width.

## 3. The value-expression grammar

A strict subset of Virdant's `Expr` (see `GRAMMAR.txt`), defined in a
new lalrpop grammar with its own lexer in `vir/src/value/` (mirroring
the `poke` setup: `lexer.rs`, `grammar.lalrpop`, vanilla lalrpop crate,
wired into `vir/build.rs`).

```
ValueExpr :=
      WordLit                            // 123, 0b1010_0101, 0xDEAD_BEEF,
                                         // 100w32, 0b1010w8, 0xDEADw16
    | BitLit                             // true | false
    | "@" Ident                          // payload-less ctor/enumerant: @Invalid
    | "@" Ident "(" ValueArgList? ")"    // ctor application: @Valid(0w8)
    | "$" "{" ValueAssignList "}"        // struct literal: ${Pair(a = 1, b = 2)}
    | "(" ValueExpr ")"
    | ValueExpr ":" Type                 // ascription

ValueArgList    := (ValueExpr ",")+ ValueExpr ","?
ValueAssignList := (ValueAssign ",")+ ValueAssign ","?
ValueAssign     := Ident "=" ValueExpr
```

The `#` constructor prefix (the `poke` script language's spelling)
is accepted as an alias for `@`; decode output always prints `@`.

`Type` in ascriptions: `Ident (GenericsParams)?` for builtin names,
`Ident "::" Ident (GenericsParams)?` otherwise — same resolution rules
as the CLI type argument.

## 4. Semantics

Type-directed, driven by the layout:

1. Resolve the expected type; type-check the value expression against
   it (ctor names, enumerant names, struct field names, arity, widths).
2. `Nat` literals take their width from the expected type. `Word`
   literals (`w<N>` suffix) must match the expected width exactly.
3. Encode walks the layout (rules below) and prints the plain bit
   pattern (`0b...`, full width).
4. Decode: tag lookup + padding check; a pattern that is not a valid
   inhabitant reports a precise error.
5. Decode output: constructors and enumerants ALWAYS print with
   parens (`@Name(...)`, `@Name()` even with no payload); `Word[N]`
   values print as plain decimals (the `wN` suffix is stripped for
   now — width comes from context); structs as `${Name(f = ...)}`.

Layout rules (must match `sim/layout.rs` documentation exactly):

- `Bit`/`Clock`/`Reset`: 1 bit.
- `Word[w]`: `w` bits.
- Struct: fields concatenated, **first field in the lowest bits**
  (matching the Verilog backend's `ExprField` bit-offset arithmetic
  and `sim`'s `eval_struct`), filling the width exactly.
- Union: one-hot tag in the low `len(slots)` bits, selected slot's
  payload above (first parameter at the MSB end of the payload area,
  zero padding up to the widest slot).
- Enum: enumerant's value in the low tag bits, payload as for unions.
- Validity: tag must be a known one-hot/enumerant value; padding above
  the payload must be zero.

## 5. The standalone layout query (virdant crate)

`sim/layout.rs`'s `build_type_layout` and friends depend on `db` +
`symboltable` only; the `Elaboration` is used just to *seed* the set of
types. Plan:

- New module `virdant/src/typelayout/` (or `types/layout/`) exposing a
  query that builds a `TypeLayoutIndex` seeded from **all type defs in
  all packages** (structs, unions, enums, builtins), with the same
  `Slot`/`TypeLayoutKind` representation and the same build rules.
- Move/copy the layout-construction core there; refactor
  `sim/layout.rs` to call into it (seeding from `elab.components()` as
  it does today), so the rules live in one place. `sim` behavior and
  tests must be unchanged.
- The `vir-value` binary uses the standalone query directly.

## 6. Files

| Path | Change |
|------|--------|
| `virdant/src/typelayout/` (new) | standalone layout query + shared layout rules |
| `virdant/src/sim/layout.rs` | refactor to consume the shared rules |
| `vir/src/value/lexer.rs` (new) | value-expression lexer (literal regexes as in `syntax/token.rs`) |
| `vir/src/value/grammar.lalrpop` (new) | the subset grammar |
| `vir/src/value/encode.rs` (new) | type-directed encode |
| `vir/src/value/decode.rs` (new) | type-directed decode + round-trippable printing |
| `vir/src/value/main.rs` (new) | `vir-value` CLI (encode/decode subcommands) |
| `vir/build.rs` | process the new grammar |
| `vir/Cargo.toml` | add `vir-value` `[[bin]]` |

## 7. Tests

- Unit tests for the layout query vs `sim`'s (same types → same
  layouts).
- Grammar round-trip tests: decode(encode(v)) == v for sample values of
  each type shape (bit, word, enum, union, struct, nested generics).
- CLI golden tests mirroring the examples in §0, including the
  `Can't infer type` error and non-builtin fully-qualified types
  (`mypkg::SomeUnion[...]`), non-pub types included.
- Decode error cases: wrong width, unknown tag, non-zero padding.

## 8. Implementation order

Work proceeds in dependency order; each phase ends with a working,
testable state.

**Phase 1 — shared layout core (virdant crate).**
1. Move `build_type_layout` and its helpers from `sim/layout.rs` into
   the new standalone module; expose a query that takes `db` + a seed
   list of types and returns the layout map.
2. Add the all-type-defs seeding (iterate packages → type defs →
   seed their instantiated types).
3. Refactor `sim/layout.rs` to call the shared core (seeding from
   `elab.components()` unchanged); run the full `sim` test suite —
   behavior must be identical.
4. Unit tests: for a sample project, standalone query and `sim`-seeded
   query produce identical layouts.

**Phase 2 — value grammar (`vir/src/value/`).**
1. `lexer.rs`: literal regexes copied from `syntax/token.rs`
   (decimal/`0b`/`0x`, `_` separators, `w<N>` suffixes), plus `@`, `$`,
   `{}`, `()`, `:`, `::`, `[`, `]`, `,`, `=`, idents, `true`/`false`.
2. `grammar.lalrpop`: the §3 productions; wire into `vir/build.rs`.
3. `ast.rs`: small AST (`WordLit{value, width?}`, `BitLit`, `Ctor{...}`,
   `StructLit{...}`, `Ascribe{...}`).
4. Parser round-trip tests on the §3 grammar.

**Phase 3 — encode/decode semantics.**
1. Type resolution: CLI type syntax + ascription `Type` syntax →
   `resolve_item_fqn`, generic args substituted (`types::substitute`).
2. Type-directed encode over the Phase-1 layouts: check ctor/enumerant/
   struct-field names, arity, widths; `Nat` borrows expected width,
   `Word` must match exactly; emit the bit pattern.
3. Decode: exact-width check, tag lookup, zero-padding check,
   recursive payload decode; round-trippable printing.
4. Error cases: `Can't infer type`, unknown name, wrong width,
   unknown tag, non-zero padding — each with a precise message.

**Phase 4 — CLI (`vir-value`).**
1. `main.rs` with clap subcommands `encode`/`decode`, project args via
   `vircommon::add_project_args`.
2. Register `[[bin]]` in `vir/Cargo.toml`; verify `vir value` discovers
   and execs it (companion-binary mechanism).
3. CLI golden tests (§7), including non-pub and non-builtin types.

## 9. Open questions

- Should `encode` also accept `0x`-style output for words wider than
  some threshold? (Currently: always `0b...`.)
- Struct output syntax `${...}` is display-only in `poke` today; if
  struct literals land in the script grammar later, keep the two
  spellings aligned.
