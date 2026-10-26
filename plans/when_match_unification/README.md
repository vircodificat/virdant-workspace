# when/match unification
Date: 2026-10-22
Status: Completed

Unify the expression-form and statement-form productions of `when` and
`match` into single grammar productions, so that "wrong style" programs
parse into an AST and are reported by an early semantic pass with a
helpful message ("use `=>`" / "use `{ ... }`") instead of a cryptic
syntax error.

## Files

* `CONCEPT.md` — the problem, the core idea, and the non-goals.
* `PLAN.md` — the technical plan and as-built design notes.

## Status

Date: 2026-10-22
Status: Complete

Implemented (see PLAN.md for the as-built deviations, in particular the
arm-block XOR restriction required to keep the grammar LR(1)-parseable).
