# Phase 0: Grammar, Payload, and AST Accessors for `platform`

> **Status: Done (committed).**
> Amendment (v3): the `for` keyword and the module-side `for` clause
> are added in Phase 2 (v3), not here. The v3 revision also removes
> the v2 Phase 2 work (`PackageAnalysis.platform`,
> `DuplicatePlatform`); see PHASE_02.md (v3). Nothing else in this
> phase changes.

### 0.1 Token + keyword

**Why:** The lexer can't recognize `platform` as syntax until it has a token for it, and it must be registered in `KEYWORDS` so that `platform` is reserved and can't be used as a package or identifier name.

Edit `virdant/src/syntax/token.rs`:

- Add `#[token(b"platform")] KwPlatform,` after `KwDependsOn`
  (`:125`), keeping the enum ordering stable.
- Add `"platform"` to `KEYWORDS` (`:137-146`).

### 0.2 Grammar rules

**Why:** A token alone doesn't let the parser build an AST node; the grammar rules tell lalrpop how to parse a `platform { ... }` block into a `Platform` payload, and reusing `ModDefStmtComponent` for ports means platform ports get the same parsing (types, `@pin`, docstrings) as ordinary module components for free.

Edit `virdant/src/syntax/virdant.lalrpop`.

Add `PlatformDef` to the `Item` production list (`:142-149`):

```lalrpop
Item: AstNodeId = {
    <moddef:ModDef> => moddef,
    <structdef:StructDef> => structdef,
    <uniondef:UnionDef> => uniondef,
    <enumdef:EnumDef> => enumdef,
    <builtindef:BuiltinDef> => builtindef,
    <socketdef:SocketDef> => socketdef,
    <platformdef:PlatformDef> => platformdef,
}
```

Add the rules (modeled on `ModDef` at `:152`):

```lalrpop
PlatformDef: AstNodeId = {
    <docstring:DocString?> <annotations:Annotations> <ll:@L>
    "platform" <name:Ident> "{"
        <stmts:PlatformStmt*>
    "}" <rr:@R> => {
        let payload = AstNodePayload::Platform(payload::Platform {
            name,
            doc_string: docstring,
        });
        let span = parser.span(ll, rr);
        let num_children: u16 = 1u16 + stmts.len() as u16;

        parser.add_node(payload, span, num_children)
    },
}

PlatformStmt: AstNodeId = {
    <component:ModDefStmtComponent> => component,
}
```

Notes:
- `PlatformStmt` is only `incoming`/`outgoing` components
  (reusing `ModDefStmtComponent`, `lalrpop:336`), so `@pin`,
  `@period_ns`, docstrings, and type parsing behave identically to
  user modules.
- `reg`/`wire`/`submod` inside a platform are not rejected by the
  grammar; the analysis in Phase 4 simply ignores any component whose
  kind is not `Incoming` or `Outgoing`.

### 0.3 Extern block

**Why:** lalrpop grammar rules refer to the literal string `"platform"`, and the `extern` block is what maps that literal to the `Token::KwPlatform` variant produced by the lexer, so without this entry the grammar rules in 0.2 would fail to compile.

Add to the `extern { ... }` block (near `:1091`):

```lalrpop
"platform" => Token::KwPlatform,
```

### 0.4 Payload variant

**Why:** The parser in 0.2 constructs an `AstNodePayload::Platform` value, so that variant and its backing `Platform` struct (holding the name and docstring) must exist before the grammar code can compile, and the `kind()` arm lets generic AST tooling report what kind of node it's looking at.

Edit `virdant/src/syntax/payload.rs`:

- Add `Platform(Platform)` to `AstNodePayload` (after `SocketDef`,
  `:22`).
- Add the `kind()` arm: `AstNodePayload::Platform(_) => "Platform"`.
- Add the struct:

```rust
#[derive(Clone, Debug)]
pub struct Platform {
    pub name: InternedString,
    pub doc_string: Option<InternedString>,
}
```

### 0.5 ast.rs accessors

**Why:** Later phases (symbol table, `PackageAnalysis`, diagnostics) treat all top-level items generically through these shared `ast.rs` helpers, so without a `Platform` arm in `summary()`, `is_item()`, `doc_string()`, and `name()`, a platform item would be invisible to or mishandled by that generic item-processing code.

Edit `virdant/src/syntax/ast.rs`:

- `summary()` (`:143`): arm returning
  `format!("Platform {}", parsing.string(p.name.clone()))`.
- `is_item()` (`:226`): add `AstNodePayload::Platform(_)`.
- `doc_string()` (`:238`): add the `Platform` arm.
- `name()` (`:260`):
  `AstNodePayload::Platform(p) => Some(p.name.clone())`.

Run `make grammar && make test`.
Existing tests must still pass; the new keyword is unused.
