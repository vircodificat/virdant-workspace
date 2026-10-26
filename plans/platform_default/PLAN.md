# Plan: Platform Defaults

Date:   2026-10-22
Status: In Planning

## Summary

Allow an optional `default <EXPR>` clause on `outgoing` port declarations in a
`platform` definition. When a module bound to the platform via a `for` clause
does not declare a platform port:

- A missing `outgoing` port **with** a `default` is synthesized into the module
  (port + driver driving the default expression).
- A missing `incoming` port is synthesized as an input port that suppresses the
  unused-signal warning (an internal flag, not a real `unused` statement).
- A missing `outgoing` port **without** a `default` still reports
  `PlatformMissingPort`.

Synthesis happens **early**: as far as every analysis is concerned, the module
*does* declare the port. Synthesized ports are indistinguishable from
hand-written ports everywhere — analyses, dumps, and documentation.

## Requirements (from interview)

1. Synthesis happens early in the pipeline. The top is *considered to have*
   the port for every analysis, and it appears in documentation.
2. The synthesized `incoming` port suppresses the unused warning via an
   internal flag — we do **not** inject a real `unused` statement.
3. The grammar change is on `ModDefStmtComponent` (there is no separate
   `OutgoingStmt` rule; `PlatformStmt` reuses `ModDefStmtComponent`).
   `DefaultClause?` is added to **all three** outgoing arms
   (`outgoing`, `outgoing wire`, `outgoing reg`), not to `incoming`.
4. `default` is grammatically accepted in plain mod definitions (shared rule)
   but **semantically rejected** there (`DefaultOutsidePlatformPort`).
5. The compiler does not evaluate the default expression; it is lowered
   verbatim to Verilog.
6. Deliverables include language-reference doc updates and `GRAMMAR.txt`.
7. Tests are `tests/` directory-style golden tests only. **No `#[test]` at all.**
8. Synthesized ports appear in documentation **identical** to hand-written
   ports (no marking).
9. Pullup interaction: out of scope, ignore entirely.

## Design

### 1. Grammar (`virdant.lalrpop`, `token.rs`, `GRAMMAR.txt`)

New rule, following the `ItBlock` pattern (payload node wrapping children):

```
DefaultClause: AstNodeId = {
    <ll:@L> "default" <expr:Expr> <rr:@R> => { … AstNodePayload::DefaultClause … },
}
```

Extend the three outgoing arms of `ModDefStmtComponent`:

```
"outgoing"      <ident:Ident> ":" <typ:Type> <default:DefaultClause?> <on:OnClause?> <it:ItBlock?>
"outgoing" "wire" <ident:Ident> ":" <typ:Type> <default:DefaultClause?> <on:OnClause?> <it:ItBlock?>
"outgoing" "reg"  <ident:Ident> ":" <typ:Type> <default:DefaultClause?> <on:OnClause?> <it:ItBlock?>
```

- New keyword: `default => Token::KwDefault` in `token.rs`
  (note: any existing user identifier named `default` becomes a keyword —
  acceptable; mention in docs release notes if any).
- New AST payload: `AstNodePayload::DefaultClause` with a single `Expr` child,
  mirroring how `AstNodePayload::It` wraps a `ModDefStmtBlock`.
- Update `GRAMMAR.txt` to match.

Note: since platforms only permit plain `outgoing` ports (see *Reminders*),
`DefaultClause` on the wire/reg arms is only reachable in mod definitions,
where it is semantically rejected anyway. It is included because the arms
share the rule shape and for grammar uniformity.

### 2. Semantic check: `DefaultOutsidePlatformPort`

New diagnostic in `diagnostics.rs`:

- Message: `` `default` is only allowed on `outgoing` ports in a platform definition. ``
- Emitted when a **mod definition** declares an outgoing component
  (any of the three arms) carrying a `DefaultClause`.
- Emitted from the moddef checking path (alongside the other per-item checks,
  cf. `queries/check.rs` / the item-check walk).
- Platform items are unaffected: `default` on a platform `outgoing` is legal.

### 3. Platform defaults accessor (`analysis/platform.rs`)

New accessor, following the existing plain-function style:

```rust
/// (port name, type, default-expr location) for every outgoing port of the
/// platform that carries a `default` clause.
pub fn platform_outgoing_defaults(builder: &mut Builder, platform_id: SymbolId)
    -> Vec<(BString, Option<Type>, Location)>
```

Implemented by walking the platform item's AST children and matching
`Component` nodes with kind `Outgoing` and a `DefaultClause` child.
The `Location` is the location of the **default expression node** (in the
platform's package/parsing), so downstream consumers can lower/typecheck the
expression verbatim without copying it across packages.

### 4. Synthesis in component analysis (`analysis/component.rs`)

The earliest hook where the platform is resolvable is component analysis:
`build_component_analysis` is the central registry from which ports,
documentation, drivers, and most analyses flow. Modify
`build_component_analysis` for `ModDef` symbols (non-`ext`):

1. Resolve the `for` clause via `resolve_platform_for`.
   - `Ok(None)` → no synthesis.
   - `Err(diagnostic)` → ignore here (do **not** push); the existing
     `check_platform_ports_for` already reports `UnresolvedItem` /
     `ExpectedPlatform`, and duplicating it would double-report.
2. Fetch `platform_outgoing_defaults` for the resolved platform.
3. For every platform `outgoing` port whose name is **not** among the
   module's components:
   - **Has default** → append a synthetic `Outgoing` component to the
     component analysis. Its `Location` points at the platform port's
     declaration AST node (so `build_ports_of`'s payload check still works —
     the payload is `Component`, not `Submodule`).
   - **No default** → do nothing here; `check_platform_ports_for` continues to
     emit `PlatformMissingPort` exactly as today (the port is still missing
     from the module's component list, so no changes are needed there).
4. For every platform `incoming` port whose name is **not** among the
   module's components: append a synthetic `Incoming` component, flagged
   `synthetic: true` (new field on the component struct).

Because synthesis lives in component analysis, `build_ports_of` picks the
synthesized ports up with **no changes** — so `vir ports`, LSP, docs, and
`check_platform_ports_for` (direction/type matching for *declared* ports) all
see the port as if hand-written.

`Component` struct change (`analysis/component.rs`):
```rust
pub struct Component {
    …
    /// True for ports synthesized from platform defaults / platform
    /// incoming ports the module does not declare. Used to suppress the
    /// unused-signal warning; synthesized ports are otherwise identical
    /// to hand-written ones.
    pub synthetic: bool,
}
```
(All construction sites default `synthetic: false`.)

### 5. Synthetic drivers (`analysis/drivers.rs`)

In `build_driver_analysis`, after `collect_block_drivers`:

- For every synthetic `Outgoing` component with a default, insert
  `Driver::Expr(DriverType::Continuous, <default-expr Location>)` into
  `drivers` keyed by that component's `ComponentId`.
- The driver's `Location` points into the platform's parsing, matching the
  existing location-based driver model (`Driver::Expr(DriverType, Location)`),
  so no new representation is needed.

Consequences (all desirable):

- Driver-counting checks treat the port as driven exactly once.
- Lowering emits the default expression verbatim as the port's continuous
  assignment — the compiler never evaluates it.
- A type mismatch between the default expression and the port type is
  reported by the normal driver-typechecking path, at the platform's
  expression region.

**Verification item during implementation:** confirm the driver-typechecking
and Verilog-lowering paths resolve expressions by `Location` (package +
ast node) rather than assuming the driver lives in the module's own package.
`Driver::Expr` is already location-based, so this is expected to work; if a
path assumes `parsing == module parsing`, generalize it to fetch the parsing
for the driver's location.

### 6. Suppressing the unused warning for synthesized `incoming` ports

The "Unused signal" warning is emitted in `types/typing.rs` while iterating
`component_analysis.components()` for components that `can_source()` and are
never read. Add the synthetic flag to the condition:

```rust
&& !component.synthetic
```

No `unused` statement is injected; the suppression is purely the flag.

### 7. Documentation

- Synthesized ports flow into virdoc through component analysis automatically
  and are rendered **identically** to hand-written ports (requirement 8).
  No renderer changes expected; verified via `tests/virdoc` golden tests.
- Hand-written reference updates:
  - `docs/source/reference/platforms.rst` — document `default <EXPR>` on
    `outgoing` ports and the synthesized-port behavior.
  - `docs/source/reference/modules.rst` / `statements.rst` — mention that
    `default` is not allowed in mod definitions (error).
  - `GRAMMAR.txt` — new `DefaultClause` rule.

## Diagnostics summary

| Diagnostic | When | Status |
|---|---|---|
| `PlatformMissingPort` | platform outgoing port missing, **no** default | unchanged |
| `DefaultOutsidePlatformPort` *(new)* | `default` on outgoing component in a mod definition | new |
| Driver type mismatch | default expr type ≠ port type | existing machinery, reported at the platform's expression |
| Unused-signal warning | synthesized incoming port | suppressed via flag |

## Test plan (`tests/` golden style only — no `#[test]`)

- `tests/pass/platform_default/`
  - Platform with several defaulted outgoing ports; module omits some of
    them. Golden Verilog shows the synthesized continuous assignments
    (expression lowered verbatim), and the remaining ports behave as usual.
  - Platform with an incoming port the module omits; expect no unused
    warning.
  - Module that *does* declare a defaulted port itself (override) — compiles,
    platform match still passes.
  - Default expressions of several shapes: literals, `@Valid(n)`, struct
    literal.
- `tests/errors/platform_default/`
  - `default` on an outgoing port in a plain mod definition →
    `DefaultOutsidePlatformPort`.
  - Missing platform outgoing port **without** default → still
    `PlatformMissingPort`.
  - Default expression type mismatch → type error pointing at the platform.
- `tests/warnings/platform_default/`
  - Module omitting a defaulted outgoing port and an incoming port produces
    no unused warnings.
- `tests/virdoc/platform_default/`
  - Module docs list the synthesized ports indistinguishably from
    hand-written ones.

## Out of scope / non-goals

- No `default` on `incoming` ports.
- The compiler does not evaluate or constant-fold default expressions.
- Pullup interaction: ignored entirely (per decision).
- No changes to the LSP quick-fix: ports with defaults no longer produce
  `PlatformMissingPort`, so the quick-fix naturally only offers the
  non-defaulted missing ports.

## Reminders (for after this plan is written)

- **`outgoing wire` / `outgoing reg` are not allowed in platforms.** There is
  currently no check enforcing this; consider whether this plan (or a
  separate one) should add a semantic check rejecting wire/reg components in
  platform definitions.
