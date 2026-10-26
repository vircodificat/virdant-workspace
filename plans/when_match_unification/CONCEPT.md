# CONCEPT: Unify `when` / `match` — one grammar, context-driven style rules

## Problem

Virdant currently has two separate grammar productions for `when` (and `match`):

- **Expression form** (`ExprWhen`, `ExprMatch`) — used in value position, e.g. `it := when { ... }`.
- **Statement form** (`ModDefStmtWhen`, `ModDefStmtMatch`) — used at statement level.

They are structurally near-identical (same arm shapes, same block-arm rule,
`ModDefStmtBlock` used in both), yet they are separate AST payloads, separate
productions in `virdant.lalrpop`, and separately handled in
`typing.rs`, `component.rs`, `dependency.rs`, etc.

Worse: the current grammar *rejects* natural code like:

```virdant
it := when {
    case counter < 0xff {
        @Invalid()
    }
    else {
        @Valid(0)
    }
}
```

because inside a `ModDefStmtBlock`, `@Invalid()` parses as an *Annotation*,
not an expression — producing the cryptic error
`Unexpected token: "}". Expected: "@" "mod" "incoming" ...`.

## Core idea

1. **One grammar.** Merge the expression and statement variants of `when`
   and `match` into single productions. An arm body may be either a bare
   expression (`case cond => expr`) or a block (`case cond { ... }`) where
   the block may contain a mix of statements and bare expressions.

2. **Parse always succeeds.** The parser produces an AST for all of the
   above; no grammar-level failure for "wrong style".

3. **Context is canonical.** Every `when`/`match` is either acting as an
   *expression* (in value position) or as a *statement*. That context
   determines which arm form is idiomatic:
   - Expression context ⇒ the canonical arm form is `case ... => expr`.
   - Statement context ⇒ the canonical arm form is `case ... { stmts }`.

4. **Early semantic check.** A new pass, run very early after parsing, walks
   every `when`/`match`, determines its context, and flags arm bodies whose
   *form* contradicts the context — as a **hard error** with a helpful
   message:
   - Block arm containing only bare expressions, in expression context ⇒
     "Use `=>` instead of `{ ... }`".
   - Bare `=>` arm (or expr-only block) in statement context ⇒ "Use `{ ... }`"
     with a detailed message.
   - Mixed bodies (statements + expressions) are allowed; only the
     *style-vs-context mismatch* is flagged.

## Example

```virdant
it <= when {
    case is_ok {
        0        // ERROR: value in statement-position block → "Use => instead"
    }
    else {
        1
    }
}
```

becomes the idiomatic:

```virdant
it <= when {
    case is_ok => 0
    else => 1
}
```

## Non-goals

- Changing driver semantics or the typing rules of `when`/`match` bodies.
- Allowing statement content in `=>` arms.
- Warning-severity diagnostics; mismatches are hard errors.
