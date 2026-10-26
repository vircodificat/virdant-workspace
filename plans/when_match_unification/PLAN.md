# PLAN: Unify `when` / `match` — one grammar, context-driven style rules

Status: In Progress

## 1. Overview

Merge the expression-form and statement-form productions of `when` and
`match` into single grammar productions. The parser accepts both `=>`
(production) and `{ ... }` (block) arm bodies in both contexts. A new early
semantic pass then validates the *style* of each arm against the context
(expression vs statement) in which the `when`/`match` appears, producing
helpful hard errors for mismatches.

## 2. Grammar changes (`virdant.lalrpop`)

### 2.1 Merge the four productions into two

Current productions:

- `ModDefStmtWhen` — statement-context `when`
- `ExprWhen` — expression-context `When`
- `ModDefStmtMatch` — statement-context `match`
- `ExprMatch` — expression-context `match`

New productions (same name as the payload names, unified):

```lalrpop
When: AstNodeId = {
    "when" "{" <arms:WhenArm*> "}" => ...
};

Match: AstNodeId = {
    "match" <subject:Expr> "{" <arms:MatchArm*> "}" => ...
};
```

### 2.2 Unified arms

```lalrpop
WhenArm: u16 = {
    "case" <guard:Expr> "=>" <body:Expr> => 2,
    "case" <guard:Expr> <block:WhenBlock> => 2,
    "else" "=>" <body: `Expr`> => 1,
    "else" <block:WhenBlock> => 1,
};
```

### 2.3 Arm blocks: statements *or* a single expression (as built)

The original plan called for arm blocks mixing statements and bare
expressions freely. That grammar is **genuinely ambiguous** (not merely
LALR-unfriendly): with no statement terminators, an expression item's
end glues onto the next item's start. `{ a - b }` has two valid parses
(one binary-subtraction item, or two items: `a` then `- b`), and
`{ x := b - c }` is ambiguous between a driver whose RHS is `b - c`
and a driver `x := b` followed by expression item `- c`. It produced
~1000 grammar-wide LR conflicts.

The as-built rule is a provably unambiguous XOR:

```lalrpop
ArmBlock: AstNodeId = {
    "{" <stmts:ModDefStmt*> "}" => ...,      // statements (possibly none)
    "{" <e:ArmBlockExpr> "}" => ...,        // a single expression
};
```

`ArmBlockExpr` mirrors the ordinary expression chain
(logical → additive → … → primary) with one carve-out: the top-level
comparison operator may not be `<=`, because at block-item position
`lhs <= rhs` is a latched driver, not a comparison (a top-level `<=`
comparison must be parenthesized). When/match are excluded from
`ArmBlockExpr` (a bare nested when/match arm body parses through the
statement alternative), and the arm-body forms `"case" Expr When` /
`"case" Pat Match` remain available as before.

New AST payload: `ArmBlock` (one child when expression-form, N children
when statement-form).

### 2.4 Expression rules

`When` and `Match` appear in the `Expr` alternatives:

```lalrpop
Expr :=
    When | Match | ExprStruct | ExprBinOpLogical
```

### 2.5 Statement rules

`When` and `ModDefStmt` alternatives:

```lalrpop
ModDefStmt :=
    ... | When | Match | ...
```

## 3. AST / payload changes (as built)

1. Payloads `ExprWhen`/`ModDefStmtWhen` merged into `When`;
   `ExprMatch`/`ModDefStmtMatch` merged into `Match`; new payload
   `ArmBlock` replaces `ModDefStmtBlock` in arm-body position
   (`ModDefStmtBlock` remains for It-blocks).
2. Context is **derived, not stored**: `AstNode::is_expression_position()`
   in `ast.rs` walks the parent chain (guards and match subjects are
   expression position; bare nested arm bodies inherit the enclosing
   when/match's position; `ModDef`/`It`/`ModDefStmtBlock`/`ArmBlock`
   parents are statement position).
3. `analysis/component.rs`, `analysis/dependency.rs`, `analysis/package.rs`,
   `analysis/drivers.rs` traversals updated: arm bodies are `ArmBlock`;
   the statement-when expr-root walker uses `when_arm_children` (only
   guards are expr roots).
4. `queries/typing.rs`: the separate statement/expression `When`/`Match`
   branches merged; guard-vs-body classification uses `when_arm_children`.
5. `types/match_coverage.rs`, `bin/virlsp.rs`: statement-ness derived via
   `is_expression_position()`.
6. `types/typing/check.rs`: `check_when`/`check_match` handle `ArmBlock`
   arm bodies via a shared `check_arm_block` (single-expression blocks are
   unwrapped and type-checked; statement blocks in expression position
   remain the old "Block with statements not allowed in expression
   position" error).

## 4. New semantic pass: `check_when_match_style` (as built)

Implemented inside `virdant/src/queries/check.rs` (runs before the
driver checks), not as a separate file.

For each `When`/`Match` in every ModDef's AST:
1. Derive context via `is_expression_position()`.
2. For each arm (via `when_arm_children`/`match_arm_children`):
   - `ArmBlock` with a single bare-expression child:
     * expression context → `ArmBlockNeedsArrow` ("use `=>` instead of
       `{ ... }`")
     * statement context → `ArmNeedsDriverStatements`
   - `=>`-arm expression body in statement context →
     `ArmNeedsDriverStatements` ("use `{ ... }` with driver statements")
   - `ArmBlock` with statement children: valid in statement context;
     in expression context the typing pass keeps its existing "Block
     with statements not allowed in expression position" error.
   - bare nested `When`/`Match` bodies are skipped: the AST cannot
     distinguish `case c => when {...}` from `case c when {...}`, and
     both are legal in statement context.

New diagnostics: `ArmBlockNeedsArrow`, `ArmNeedsDriverStatements`
(both errors) in `diagnostics.rs`.

Each error spans the arm body region.

## 5. Verilog conversion / simulation

`verilog/conversion.rs` and `sim/expr.rs` match the unified `When`/
`Match` payloads; expression-position when/match lower exactly as the
old `ExprWhen`/`ExprMatch` did (statement-position when/match lower via
the driver analysis's `Driver::When`, unchanged).

## 6. Tests (as built)

- `tests/errors/when_match_style.vir` — all new error cases (expression-
  context block arms for both when and match, statement-context `=>`
  arms, statement-context bare-expression blocks), via `// CHECK:`
  patterns.
- The existing corpus (`tests/pass`, `tests/errors`, docs tests)
  passes unchanged: `make test` is green.
- `GRAMMAR.txt` regenerated via `make grammar`.

## 7. Execution order

1. Prototype the grammar merge in `virdant.lalrpop` (watch for the
   `@Ident(...)` Annotation-vs-ExprPrimary reduce/reduce conflict).
2. Payload/accessor changes in `payload.rs`/`ast.rs`.
3. Update traversals (`analysis/*`, `queries/typing.rs`,
   `verilog/conversion.rs`, `sim/expr.rs`).
4. Implement `check_when_match_style`.
5. Tests + `make test`.
