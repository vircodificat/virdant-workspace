# Phase 2 (v3): The `for` Clause

This phase replaces v2's Phase 2. v2 added a single-platform
`PackageAnalysis.platform` field and a `DuplicatePlatform` diagnostic;
under v3 Decision 3 (multiple platforms in scope is fine) that work is
removed, and the module-side binding is added instead: an optional
`for` clause on `ModDef`.

### 2.1 Remove the v2 single-platform work

**Why:** `PackageAnalysis.platform: Option<AstNodeId>` assumed exactly
one platform per package and `DuplicatePlatform` fired on a second
one. Both contradict Decision 3: a package may declare several
platform items, duplicate *names* among items are already reported by
the generic `DuplicateItem` check in `add_items`
(`package.rs:116-126`), and nothing downstream needs the field anymore
(Phase 4 finds platform items via the symbol table, like every other
item).

Edit `virdant/src/analysis/package.rs`:

- Delete the `platform: Option<AstNodeId>` field from
  `PackageAnalysis`, its `platform: None` initializer, the
  `platform()` accessor, and the `AstNodePayload::Platform(_)`
  detection block in `add_items`.
- Edit `virdant/src/diagnostics.rs`: delete the
  `DuplicatePlatform(DuplicatePlatform)` variant, the
  `DuplicatePlatform` struct, its `message()` arm, and its
  `from_diagnostic!` entry.

(All of this was implemented for v2 but is uncommitted; removing it
restores `package.rs`/`diagnostics.rs` to their pre-platform state.)

### 2.2 `KwFor` token + keyword

**Why:** The lexer can't recognize `for` until it has a token, and it
must be in `KEYWORDS` so `for` is reserved and can't be used as a
package or identifier name.

Edit `virdant/src/syntax/token.rs`:

- Add `#[token(b"for")] KwFor,` after `KwPlatform` (`:126`), keeping
  the enum ordering stable.
- Add `"for"` to `KEYWORDS`.

### 2.3 Grammar: optional `for` clause on `ModDef`

**Why:** The clause is parsed into the existing `Ofness` node shape
(`package: Option, name`), which already supports both bare `Ident`
and `Ident "::" Ident` forms (`lalrpop:928-949`) — the same shape the
`mod shifter of Shifter` clause uses. No new payload struct, no new
grammar rule beyond extending `ModDef`.

Edit the `ModDef` production (`virdant.lalrpop:152`) and add a
`ForClause` rule:

```lalrpop
ModDef: AstNodeId = {
    <docstring:DocString?> <annotations:Annotations> <ll:@L>
    <ext:"ext"?> <export:"export"?> "mod" <ident:Ident>
    <for_clause:ForClause?>
    "{"
        <stmts:ModDefStmt*>
    "}" <rr:@R> => {
        let payload = AstNodePayload::ModDef(payload::ModDef {
            name: ident,
            is_ext: ext.is_some(),
            is_export: export.is_some(),
            doc_string: docstring,
        });
        let span = parser.span(ll, rr);
        let num_children: u16 = 1u16
            + if for_clause.is_none() { 0u16 } else { 1u16 }
            + stmts.len() as u16;

        parser.add_node(payload, span, num_children)
    },
}

ForClause: AstNodeId = {
    "for" <ofness:Ofness> => ofness,
}
```

(The clause is `"for"` followed by an `Ofness`; the rule returns the
`Ofness` node itself, so the child of `ModDef` is the platform
reference node with the `for` keyword excluded from its span.
Note that a bare `<for_clause:Ofness?>` is wrong — `for` is a
keyword token, not an `Ident`, so `Ofness` alone cannot consume it.
Verified the hard way.)

Notes:
- The clause sits after the name and before `{`, so
  `export mod Top for ice40::IceSugar { ... }` parses.
- `ext mod` declarations are not semantically checked for `for`
  clauses (they have no ports to match); the grammar stays permissive
  and the analysis in Phase 4 simply ignores ext mods.

Add to the `extern { ... }` block:

```lalrpop
"for" => Token::KwFor,
```

### 2.4 `platform_for()` accessor

**Why:** Phase 4's check and Phase 8's `vir bitstream` both need to
read a module's `for` clause. Because the `Ofness` child is optional
and positional, callers should not hard-code child indices; a single
accessor in `ast.rs` keeps the layout knowledge in one place.

Edit `virdant/src/syntax/ast.rs`:

```rust
/// The `for`-clause node (`Ofness`) of a `ModDef`, if present.
pub fn platform_for(&self) -> Option<AstNode<'_>> {
    if !matches!(self.payload(), AstNodePayload::ModDef(_)) {
        return None;
    }
    self.children().into_iter().find(|c|
        matches!(c.payload(), AstNodePayload::Ofness(_)))
}
```

(The filter-based form is robust against the child's position; the
`Annotations` wrapper is always `child(0)`, and the `Ofness` child,
when present, is the only `Ofness` among the ModDef's direct children
since `PlatformStmt`-style content never appears in a `ModDef`.)

### 2.5 `package.rs` match arm

**Why:** `add_item_expr_roots` (`package.rs:142-237`) matches
exhaustively over ModDef children and ends in
`_ => unreachable!(...)` (`package.rs:221`). Once `ModDef` can carry
an `Ofness` child, parsing any `for` clause would panic there.

Edit `virdant/src/analysis/package.rs`, in the
`AstNodePayload::ModDef` loop of `add_item_expr_roots`, add:

```rust
AstNodePayload::Ofness(_) => (),
```

(All other ModDef-children walkers use `_ => continue` and are
unaffected — verified: `symbols.rs:365-380` is the only other
substantive one.)

Run `make grammar && make test`.
Existing tests must still pass; the new clause is unused by analysis
at this point, but `vir parse` on
`mod Top for ice40::IceSugar { ... }` must show the `Ofness` child.
