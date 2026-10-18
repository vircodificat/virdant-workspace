# Phase 4 (v4): `check_platform_ports` and Platform Accessors

> Revised after an implementation review of v3's `PlatformAnalysis`.
> There is **no `PlatformAnalysis`** and **no `CheckPlatform` query**.
> The platform item's AST *is* the data: platform facts are read on
> demand through small, un-memoized accessor functions, and the exact
> port match is a plain helper called from `Check()`, like
> `check_mod_cycles` and `check_duplicate_enum_values`. `db.rs` is
> untouched by this phase — no new queries, no dispatch entries, no
> getters.

Platforms are unvalidated data (OVERVIEW Decision 13): this phase
adds no `@fpga`/`@pin`/`@period_ns` validation and no cardinality
rules (any number of clock ports is fine; a platform may even have
none). Point-of-use consumers (Phases 7/8) report bad platform data
with `DiagnosticPayload::Unknown { message }` or CLI errors.

Where things live:

| Concern | Home |
|---------|------|
| Generic annotation lookup | `syntax/ast.rs` (next to `item_children`) |
| Platform accessors, `for`-clause resolution, per-module port match | `analysis/platform.rs` (free functions, no queries) |
| Aggregation over all items | `queries/check.rs` (`check_platform_ports`) |

### 4.1 Annotation accessors (`ast.rs`)

**Why:** Annotations are fiddly to read: the values sit in a nested
`Annotations` wrapper (`child(0)`), values come in nat and str forms,
and `Str` values keep their source quotes (`@fpga("ice40")` interns
`"ice40"` *with* the quotes — the `Str` rule interns the raw span,
and nothing unquotes it). One accessor, written once, serves the
platform accessors, Phase 5's `@cell` lookup, and any future
annotation consumer.

Add to `virdant/src/syntax/ast.rs`:

```rust
pub enum AnnotationValue {
    Nat(u64),
    Str(BString),  // unquoted
}

/// All values of `@<name>(...)` annotations on this node, in source
/// order.  Bare `@name` annotations (no value) are skipped.
pub fn annotation_values(
    &self,
    parsing: &Parsing,
    name: &str,
) -> Vec<AnnotationValue>
```

Walk `self.annotations()` (the existing helper over the `Annotations`
wrapper), match `annotation_name()` against `name` (through
`parsing.string(...)`), and map each value: the nat form via
`annotation_natural()`, the str form via `annotation_string()` with
the surrounding double quotes stripped. `Str` comparison and lookup
must go through `parsing.string(...)` (`InternedString` is
per-package).

### 4.2 Platform accessors (`analysis/platform.rs`)

**Why:** Phases 7/8 need the platform's pins, clock periods, and
`@fpga`/`@part` as data. These are plain functions over the
platform's AST node and its `Parsing` — no struct, no memoization,
no query registration. No cardinality assumptions: pins and clocks
are *lists*.

Create `virdant/src/analysis/platform.rs` (add `pub mod platform;` to
`analysis.rs`) with:

```rust
pub fn platform_fpga(builder: &mut Builder, symbol_id: SymbolId) -> Option<BString>;
pub fn platform_part(builder: &mut Builder, symbol_id: SymbolId) -> Option<BString>;
/// (port name, pin string) for every port carrying a `@pin`.
pub fn platform_pins(builder: &mut Builder, symbol_id: SymbolId) -> Vec<(BString, BString)>;
/// (port name, pin string, period_ns) for every Clock-typed incoming
/// port carrying a `@period_ns`.
pub fn platform_clocks(builder: &mut Builder, symbol_id: SymbolId)
    -> Vec<(BString, BString, f64)>;
```

Each walks the platform item's node via `item_children` and reads
annotations with `annotation_values` (4.1). Pins stringify either
form (`35` or `"A1"` -> `35`/`A1`); periods parse to `f64` (nat or
str). Clock detection: an `incoming` port whose `Type` child's
`Ofness` names `Clock` (`component.rs`'s `stmt.typ()` gives the type
node; the `Ofness` name check through the parsing is sufficient for
accessor purposes — no type resolution needed).

### 4.3 `for`-clause resolution (`analysis/platform.rs`)

**Why:** Both the port match (`vir check`) and `vir bitstream`
(Phase 8) must resolve a module's `for` clause identically; one
shared helper keeps them in sync.

```rust
pub fn resolve_platform_for(
    builder: &mut Builder,
    moddef_symbol_id: SymbolId,
) -> Result<Option<SymbolId>, Diagnostic>
```

- `Ok(None)`: the module has no `for` clause (`platform_for()`
  returns `None`), or is an `ext` mod (ext mods have no ports to
  match and no meaningful binding — same skip as v3's check).
- `Err(diagnostic)`: the clause names an unresolvable item
  (`UnresolvedItem`, at the `Ofness` node's region) or a resolved
  item that is not a platform (`ExpectedPlatform`, same region).
- `Ok(Some(platform_symbol_id))`: the clause resolved to a platform.

Resolution follows the codebase's standard pattern: qualified names
(`ice40::IceSugar`) go through `packages.id(...)` +
`resolve_item`; bare names try the defining package first, then each
package in `package_analysis.imports()` (`component.rs:167-172`
shows the import-set pattern). No platform-specific machinery: the
platform item is just an item in the symbol table.

### 4.4 The port-match helper

`analysis/platform.rs` holds the per-module core; `queries/check.rs`
holds the aggregation. Same semantics as v3's design — only the
mechanism changed (plain helper, not a query).

```rust
// analysis/platform.rs
pub fn check_platform_ports_for(
    builder: &mut Builder,
    moddef_symbol_id: SymbolId,
    diagnostics: &mut Vec<Diagnostic>,
)
```

1. `resolve_platform_for(...)`: `Ok(None)` -> return; `Err(d)` ->
   push and return.
2. `let module_ports = builder.get_ports_of(moddef_symbol_id);`
   `let platform_ports = builder.get_ports_of(platform_id);`
3. Every module port must have a same-name platform port, else
   `PlatformExtraPort { platform, port }` at the module port's region
   (the module declares a port the platform does not have).
4. Every platform port must have a same-name module port, else
   `PlatformMissingPort { platform, port }` at the `ModDef` node's
   region (the module is missing a port the platform requires).
5. Same-name pairs must match direction (`Incoming`<->`Incoming`,
   `Outgoing`<->`Outgoing`; platforms never declare `reg`/`wire`),
   else `PlatformPortDirMismatch { port, expected, actual }` at the
   module port's region; and have equal types
   (`Port.typ == Port.typ`; `Type::Usual` wraps a `SymbolId`, so
   equality holds exactly when both resolve to the same declaration,
   e.g. both `builtin::Clock`), else
   `PlatformPortTypeMismatch { port, expected, actual }` at the same
   region.

```rust
// queries/check.rs
fn check_platform_ports(builder: &mut Builder, diagnostics: &mut Vec<Diagnostic>) {
    let symboltable = builder.get_symboltable();
    for item in symboltable.items() {
        if item.kind == SymbolKind::ModDef {
            crate::analysis::platform::check_platform_ports_for(
                builder, item.id(), diagnostics);
        }
    }
}
```

Called from `check()` next to `check_mod_cycles` /
`check_duplicate_enum_values`.

**`ports.rs` guard:** `build_ports_of` (`ports.rs:30`) only accepts
`SymbolKind::ModDef`, so `get_ports_of(platform_symbol_id)` would
return empty. Widen the guard to
`matches!(kind, SymbolKind::ModDef | SymbolKind::Platform)`.
Platform ports are `Incoming`/`Outgoing` components, so the existing
port-classification logic handles them unchanged.

### 4.5 Diagnostics

Add to `diagnostics.rs` (struct + `DiagnosticPayload` variant +
`message()` arm + `from_diagnostic!` entry; the default Error level
is correct for all of them):

- `ExpectedPlatform { item: BString }`
- `PlatformMissingPort { platform: BString, port: BString }`
- `PlatformExtraPort { platform: BString, port: BString }`
- `PlatformPortDirMismatch { port: BString, expected: PortDir, actual: PortDir }`
- `PlatformPortTypeMismatch { port: BString, expected: Type, actual: Type }`

Note: `diagnostics.rs` declares `pub type Type = BString`; use the
full path `crate::types::Type` for the mismatch payload's fields.

Reuse `UnresolvedItem` (exists). Deliberately absent:
`DuplicatePlatform` (never existed in v3), `PlatformMissing`
(Decision 7), and *all* platform-file validation diagnostics
(`PlatformUnknownFpga`, `PlatformMissingPin`, `PlatformMultiplePins`,
`PlatformMissingPeriodNs`, `PlatformMissingClock`,
`PlatformMultipleClocks` — Decision 13).

### 4.6 Test update

`tests/pass/mod_for_clause.vir` (Phase 2) declares `for FPGA`,
`for vendor::SpecificFPGA`, etc. — unresolved names, which were
silently ignored before this phase. They are now `UnresolvedItem`
errors. Rewrite the test so every non-`ext` module's `for` clause
resolves to a `platform` item defined in the same file. Platforms
are unvalidated data (Decision 13), so they can be exactly as small
as the modules require — a platform needs nothing for `vir check` to
pass. Keep the `ext` modules' deliberately unresolvable clauses
(ext mods are skipped by the port match), and drop the
package-qualified case (`for vendor::SpecificFPGA`): it cannot
resolve until a second package exists; Phase 9's happy path
(`for ice40::IceSugar`) covers the qualified form.

Run `make grammar && make test` (no grammar changes in this phase,
but run it anyway per the plan convention).
