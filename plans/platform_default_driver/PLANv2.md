# Plan v2: Default Drivers (`?=`)

Revised after review discussion. Supersedes `PLAN.md`.

## Semantics (agreed)

```
Path "?=" Expr
```

- `?=` is a **continuous** driver, written inside a `platform` def, giving a
  default value to one of the platform's ports.
- A moddef with a `for` clause is no longer obligated to **drive** — or even
  **declare** — a platform port that carries a default. From the moddef's
  point of view the port "doesn't exist": it cannot be driven, read, or
  referenced (e.g. `a.tx` from an instantiating module is unresolvable).
- The emitted **Verilog still has the port** (it is physically pinned via the
  platform's `@pin`), driven internally by the default expression.
- The default expression **must be a constant expression**. It is not
  evaluated by the compiler — it is typechecked in the platform context and
  compiled to Verilog as-is.
- A moddef that *does* declare and explicitly drive the port overrides the
  default.

## Design decisions (agreed in review)

1. **`?=` parses to a regular `Driver` AST node** — no new node kind. The
   parser payload records `DriverType::Default` so a `?=` is distinguishable
   from a `:=` (both are continuous; without this, a banned `:=` inside a
   platform would be undetectable).
2. **No new `Driver` variant in the analysis-level enum, and no injection into
   the moddef's drivers map.** Defaults live in a side-table on
   `DriverAnalysis`, because a defaulted port that the moddef doesn't declare
   has **no `ComponentId`** to key a driver on. One mechanism covers both
   declared-but-undriven and undeclared ports.
3. **New diagnostic `NonDefaultDriverInPlatform`**: `:=`, `<=`, `:=:` inside a
   platform are hard errors (not silently ignored).
4. **New diagnostic `DefaultDriverNotInPlatform`**: `?=` outside a platform is
   a hard error. Grammatically `?=` is accepted anywhere a driver statement is
   accepted, so this is a semantic error with a good message rather than a
   parse error.
5. Defaults are **outgoing-ports-only** and **platform-top-level-only** (see
   "Restrictions" below).

---

## Lexer (`src/syntax/token.rs`)

Add a `?=` token. It must be registered **before** the existing lone `?` token
so logos matches the longer token first:

```diff
+#[token(b"?=")]  EqQuestion,
 #[token(b"?")]   Question,
```

## Grammar (`src/syntax/virdant.lalrpop`)

- Register `"?=" => Token::EqQuestion` in the `extern` token block.
- `ModDefStmtDriver` gains a fourth alternative:

  ```lalrpop
  <ll:@L> <lhs:Path> "?=" <rhs:Expr> <rr:@R> => {
      let payload = AstNodePayload::Driver(payload::Driver {
          driver_type: DriverType::Default,
      });
      let span = parser.span(ll, rr);
      let num_children = 2;
      parser.add_node(payload, span, num_children)
  },
  ```

- `PlatformStmt` is extended to also accept `ModDefStmtDriver`:

  ```lalrpop
  PlatformStmt: AstNodeId = {
      <component:ModDefStmtComponent> => component,
      <driver:ModDefStmtDriver> => driver,
  }
  ```

  This makes `:=`/`<=`/`:=:` syntactically legal in platforms too; they are
  rejected semantically by `NonDefaultDriverInPlatform` (below).

## AST (`src/syntax/payload.rs`, `src/syntax/ast.rs`)

- `src/common.rs`: add `DriverType::Default` to the enum. This variant exists
  only at the payload/parsing level; the analysis-level driver tree does not
  use it (see Driver analysis below).
- No new `AstNodePayload` variant. The existing `Driver` payload already
  stores `driver_type: DriverType`, so `?=` flows through `target()` /
  `driver()` / `summary()` unchanged.
- `src/diagnostics.rs`: `WrongDriverType` rendering maps `DriverType` to the
  operator string (`:=` / `<=`); add `Default => "?="`.

## Diagnostics (`src/diagnostics.rs`)

Add two payloads and their renderings:

```rust
/// `?=` used outside a `platform` def.
DefaultDriverNotInPlatform(DefaultDriverNotInPlatform),
/// `:=` / `<=` / `:=:` used inside a `platform` def.
NonDefaultDriverInPlatform(NonDefaultDriverInPlatform),
```

```rust
pub struct DefaultDriverNotInPlatform { pub target: BString }
pub struct NonDefaultDriverInPlatform { pub target: BString }
```

Messages: `"?= may only appear in a platform"` and
"`only ?= drivers may appear in a platform`" (with the offending operator
noted).

## Driver analysis (`src/analysis/drivers.rs`)

`build_driver_analysis` runs for moddefs today. Two changes:

### Moddefs: defaults side-table

`DriverAnalysis` gains a field:

```rust
/// Platform default drivers inherited via the `for` clause, for platform
/// ports the moddef did not explicitly drive (whether or not it declares
/// them). Keyed by port path; value is the location of the `?=` RHS.
defaults: IndexMap<BString, Location>,
```

Population: after `collect_block_drivers` returns the moddef's own drivers,
resolve the platform via `resolve_platform_for`; walk the platform item's
top-level `Driver` nodes (`driver_type == Default`); for each target path that
is **not** driven by the moddef, insert into `defaults`. The location recorded
is the location of the `?=`'s RHS expression (in the platform's package), so
diagnostics point at the platform source.

### Platforms: defaults are the platform's own drivers

The platform item's driver analysis treats its `?=` statements as ordinary
continuous drivers in its own `drivers` map (they drive the platform's
`outgoing` ports, which is exactly what a driver on an outgoing port is).
This is what makes "typechecked in the platform just as if it were in a
moddef" work, and what lets `MultipleDrivers` catch two `?=` for the same
port once, at the platform level, instead of per-moddef.

`collect_block_drivers` needs no special casing for `Default` — it flows
through as a continuous driver.

### Dependency graph

Defaults are constant expressions and read no components, so they contribute
**no dependency edges**. Unchanged behavior; nothing to do here.

## Check wiring (`src/queries/check.rs`)

`check.rs` currently **skips platform items entirely** (`if item.kind ==
SymbolKind::Platform { continue; }` — running module checks on them would
falsely flag `incoming` ports as undriven). This needs to be partially
un-skipped:

- Run `typecheck` on platform items so the `?=` RHS expressions are
  typechecked against the port types in the platform context (per the
  CONCEPT).
- Add a **platform-restricted driver check** (`check_drivers_for_platform`, or
  a `check_platform_defaults` function): validates that
  - every `?=` targets a resolved, `outgoing` port (`DriverForSink`-style
    error otherwise — this enforces the outgoing-only rule),
  - default driver type matches (outgoing ports expect Continuous, so a
    `Default` payload driver is fine),
  - no `MultipleDrivers` per port (catches two `?=` on one port),
  - and emits `NonDefaultDriverInPlatform` for any `:=`/`<=`/`:=:` found in
    the platform.
- `DefaultDriverNotInPlatform` is checked for moddefs (and any other item
  kind): walk the item's AST for `Driver` nodes with
  `driver_type == Default` and report each.

## Platform port checks (`src/analysis/platform.rs`)

`check_platform_ports_for` currently reports `PlatformMissingPort` for every
platform port the moddef doesn't declare. Changes:

- Collect the set of platform port paths carrying a `?=` default.
- **Skip `PlatformMissingPort`** for those ports.
- If the moddef *does* declare a defaulted port, keep checking direction
  (`PlatformPortDirMismatch`) and type (`PlatformPortTypeMismatch`) as today —
  declaring it wrong is still an error.

## NoDrivers exemption (`src/queries/check_drivers.rs`)

In the `NoDrivers` branch of `check_drivers`: before reporting, consult
`driver_analysis.defaults()`; if the component's path is covered by a default,
skip the diagnostic. No other changes — `WrongDriverType`, `MultipleDrivers`,
and `DriverForSink` need no exemptions because defaults never enter the
moddef's `drivers` map.

## Verilog conversion (`src/verilog/conversion.rs`)

When converting a moddef:

1. Resolve its platform (via `resolve_platform_for`); if none, nothing changes.
2. For each platform port with a default:
   - If the moddef **does not declare** the port, add it to the emitted
     module's interface (name, width, direction from the platform port — the
     `@pin` constraints come from the platform elsewhere and are unaffected).
   - If the port has no explicit driver in the moddef (explicit driver or
     not declared), emit an `assign` of the default expression. The default is
     a constant expression; convert the RHS AST node (located in the
     platform's package — location resolution already works cross-package)
     directly to a Verilog expression. No evaluation, no elaboration.
3. A defaulted port that the moddef explicitly drives is emitted exactly as
   today.

Since defaults never appear in the moddef's `drivers` map, the main driver
iteration is untouched; this is a separate pass over platform defaults.

## Restriction summary

| Restriction | Enforcement |
|---|---|
| `?=` only in platforms | `DefaultDriverNotInPlatform` (moddef/other items) |
| Only `?=` in platforms | `NonDefaultDriverInPlatform` |
| `?=` targets outgoing ports only | platform driver check (sink check) |
| `?=` at platform top level only | grammar allows nesting inside `when`/`match` blocks in platforms; the platform driver check errors on any `Default` driver found in a nested block (or the grammar is restricted — prefer the semantic check for better messages) |
| Declared defaulted ports still match dir/type | `check_platform_ports_for` unchanged for declared ports |

## Tests

### Pass tests

`tests/pass/platform_default_driver/`:

- Platform with `tx ?= true`; moddef `for` that platform declares `tx` but
  never drives it → compiles; Verilog contains the port driven by the
  constant.
- Same platform; moddef does **not** declare `tx` at all → compiles; Verilog
  interface still contains `tx`, driven by the constant.
- Moddef explicitly drives `tx` → overrides the default; no default assign
  emitted.
- Default of a non-`Bit` type (e.g. `Word[8]` port, `?=` a word literal).

### Error tests

`tests/errors/`:

- `?=` inside a moddef → `DefaultDriverNotInPlatform`.
- `:=` inside a platform → `NonDefaultDriverInPlatform`.
- `?=` targeting an incoming port → sink error.
- Two `?=` for the same port → `MultipleDrivers`.
- `?=` RHS with a non-constant / unresolved reference → typecheck error
  surfacing from the platform context.

Also update `tests/pass/platform_for_clause/` to exercise a default, and add a
simulation testbench under `tests/pass/platform_default_driver/` if the
existing sim harness makes that cheap.

## File map

| Area | Files |
|---|---|
| Lexer | `src/syntax/token.rs` |
| Grammar | `src/syntax/virdant.lalrpop` |
| AST | `src/common.rs` (`DriverType::Default`), `src/diagnostics.rs` (rendering) |
| Diagnostics | `src/diagnostics.rs` |
| Driver analysis | `src/analysis/drivers.rs` (defaults side-table; platform drivers) |
| Platform queries | `src/analysis/platform.rs` (port-check exemption) |
| Checks | `src/queries/check.rs` (un-skip platforms partially), `src/queries/check_drivers.rs` (NoDrivers exemption; platform-restricted checks; `DefaultDriverNotInPlatform`) |
| Db plumbing | `src/db.rs` (query registration for any new queries) |
| Verilog | `src/verilog/conversion.rs` (interface ports + default assigns) |
| Tests | `tests/pass/platform_default_driver/`, `tests/errors/` |

## Open questions

- Exact wording/rendering of the two new diagnostics (follow existing
  diagnostic message conventions).
- Whether `?=` should ever be allowed on non-`outgoing` components in the
  future (e.g. defaulted inputs) — out of scope for now.