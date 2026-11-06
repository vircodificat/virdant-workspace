# Plan: `async` Component Modifier

Date: 2026-11-05
Status: In Planning

## Goal

Support the following (see `led/src/top.vir` for a real example):

```virdant
pub mod Top {
    incoming clock : Clock
    outgoing led : Bit {
        it := counter > 0
    }

    incoming button : Bit async

    reg counter : Word[22] {
        initial it <= 0
        if {
            when sync.out {
                it <= 0x100000w22
            }
            when counter > 0 {
                it <= it - 1
            }
        }
    }

    mod sync of Sync {
        it.inp := button
        it.clock := clock
    }
}
```

Every signal gets a **domain**: `Sync(clock)`, `Async`, or `Constant`.
The checker prevents crossing domain boundaries, except where explicitly
permitted (`async` sinks, `@unsafe_clocks` mods).

## Domain Rules (Mod-Local)

A mod's components get domains by these syntax-directed rules:

| Mod declares            | Component domain, unless annotated       |
|-------------------------|------------------------------------------|
| No `Clock` port         | Free variable α (polymorphic; defaults to `Async`) |
| Exactly one `Clock` port| Implicitly `Sync(that clock)`            |
| Two or more `Clock` ports | **Every component must declare** `on <clock>` or `async` |

- `async` and `on` together on one component = error.
- Component kinds accepting `async`: `incoming`, `outgoing`, `wire` (and
  `net`, where applicable). `reg` with `async` = error (registers are
  clocked by definition).

## Domain Lattice

- `Constant` (pure literals/param expressions) unifies with anything.
- `Sync(c)` + `Sync(c)` → `Sync(c)`
- `Sync(c1)` + `Sync(c2)`, `c1 ≠ c2` → **error**
- `Async` + `Async` → `Async`
- `Async` cannot combinationally combine with `Sync(_)` → **error**
- Taint-free: an expression's domain is the join of its operands' domains,
  including `when` guards and `mux` selectors. Guarded assignments must
  domain-unify with the register's domain.
- Sinks: regs and non-async sinks accept `Sync(own clock)` or `Constant`.
  `async` sinks accept anything. `Async` expressions drive only `async`
  sinks (or `@unsafe_clocks` bodies).

## Escape Hatch

- `@unsafe_clocks` mod-level annotation: suppresses domain checking within
  that mod's body. Used by `builtin::Sync` and raw metal primitives.

## Instantiation (Substitution)

- Child's domains are keyed by its clock components.
- `it.clock := clock` in the parent rewrites the child's
  `Sync(child_clock)` to the parent's `Sync(source domain)`.
- Error if a connected source is not `Sync(_)`/`Constant` where the child
  expects a clocked domain.
- Clockless child α bound by its connection sites; conflicting α bindings
  across connections = error at the second connection.

## Implementation Steps

1. **Lexer** — `token.rs`: add `KwAsync`.
2. **Parser** — `virdant.lalrpop`: allow optional `async` on `incoming`,
   `outgoing`, `outgoing wire`, `net` component rules. Reject on `reg`.
3. **AST** — `payload.rs`/`ast.rs`: store the `async` flag on `Component`.
4. **DomainAnalysis (new)** — `virdant/src/analysis/domain.rs`:
   - Per-mod query: domain per component + port signature
     (`Async` / `Sync(clock)` / free α).
   - Enforce explicit annotations in multi-clock mods.
   - Expression domain inference and join rules per the lattice above.
5. **Instantiation checking** — in the parent's analysis: substitution
   mechanics per "Instantiation" above.
6. **Annotations** — reuse the existing `Annotations` mechanism for
   `@unsafe_clocks`; skip domain checks inside annotated mods.
7. **`lib/builtin.vir`** — mark `Sync.inp` as `Bit async`, annotate `Sync`
   with `@unsafe_clocks`.
8. **Example** — update `led/src/top.vir` to use `async` (already drafted).
9. **Error messages** — domain-mismatch diagnostics at expression join
   failures, driver mismatches, instantiation conflicts.

## Out of Scope (Deferred)

- `raw mod` / explicit `with domain` declarations (scrapped from CONCEPT).
- `<~` domain binding operator (scrapped).
- Domains as first-class resources; domain creation syntax.
- Removal of `Clock`/`Reset` types.
- General polymorphic-signature machinery beyond the clockless-mod α case.
