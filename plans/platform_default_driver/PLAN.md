# Plan: Default Drivers (`?=`)

## Summary

Introduce a new `?=` operator (default driver) that can appear inside `platform`
defs to assign constant default values to ports.  A moddef whose `for` clause
matches that platform is not required to drive ports that carry a default;
instead it inherits the default for any port it does not explicitly drive.

---

## Files to change

| # | File | Change |
|---|------|--------|
| 1 | `src/syntax/token.rs` | Add `?=` token |
| 2 | `src/syntax/virdant.lalrpop` | Grammar: `?=` production, `PlatformStmt` extension |
| 3 | `src/syntax/payload.rs` | Add `DefaultDriver` AST payload variant |
| 4 | `src/syntax/ast.rs` | Handle `DefaultDriver` in accessors and `summary` |
| 5 | `src/common.rs` | Add `DriverType::Default` variant (optional – see notes) |
| 6 | `src/diagnostics.rs` | Add `DefaultDriverNotInPlatform` diagnostic |
| 7 | `src/analysis/drivers.rs` | Collect default drivers separately from regular drivers |
| 8 | `src/queries/check_drivers.rs` | Check `DefaultDriverNotInPlatform`; skip `NoDrivers` for platform ports that carry a default |
| 9 | `src/analysis/platform.rs` | New query: `platform_default_drivers()` that returns default driver mappings for a platform |
| 10 | `src/verilog/conversion.rs` | Emit default-driver assignments for ports the module leaves undriven |
| 11 | `tests/` | Add pass and error test cases |

---

## Phase 1 – Lexer

**`src/syntax/token.rs`**

Add the `?=` token to the `Token` enum:

```diff
 #[token(b"?")]   Question,
+#[token(b"?=")]  EqQuestion,
```

(This must be placed **before** the lone `?` entry so that `logos` matches the
longer string first.)

---

## Phase 2 – Grammar (LALRPOP)

**`src/syntax/virdant.lalrpop`**

### 2a. External token mapping

Add the new token to the `extern` block:

```diff
+        "?="  => Token::EqQuestion,
```

### 2b. `ModDefStmtDriver` – add `?=` alternative

Add a third alternative to `ModDefStmtDriver`:

```lalrpop
+    <ll:@L> <lhs:Path> "?=" <rhs:Expr> <rr:@R> => {
+        let payload = AstNodePayload::DefaultDriver;
+        let span = parser.span(ll, rr);
+        let num_children = 2;
+        parser.add_node(payload, span, num_children)
+    },
```

### 2c. Extend `PlatformStmt`

Currently `PlatformStmt` only accepts `ModDefStmtComponent`.  Add
`ModDefStmtDriver` (which now carries the `?=` alternative) so that
default drivers can be written inside platform bodies:

```lalrpop
 PlatformStmt: AstNodeId = {
     <component:ModDefStmtComponent> => component,
+    <driver:ModDefStmtDriver> => driver,
 }
```

> **Note**: This technically allows `:=`, `<=`, and `:=:` inside platform defs
> as well, but those will be caught later by semantic checks (drivers in
> platforms are meaningless and should emit an appropriate diagnostic – see
> Phase 6).

---

## Phase 3 – AST Payload

**`src/syntax/payload.rs`**

Add a new `DefaultDriver` variant to the `AstNodePayload` enum:

```diff
 pub enum AstNodePayload {
     …
     Driver(Driver),
+    DefaultDriver,
     BidirectionalDriver,
     InitialDriver,
     …
 }
```

No payload struct is needed – the `DefaultDriver` variant carries no extra
data beyond the AST node itself; the children (path, expr) are sufficient.

Add the corresponding `kind()` match arm:

```diff
 AstNodePayload::Driver(_driver) => "Driver",
+AstNodePayload::DefaultDriver => "DefaultDriver",
 AstNodePayload::BidirectionalDriver => "BidirectionalDriver",
```

---

## Phase 4 – AST Accessors

**`src/syntax/ast.rs`**

### 4a. `summary()` – display the node kind

```diff
 AstNodePayload::Driver(_driver) => format!("Driver"),
+AstNodePayload::DefaultDriver => format!("DefaultDriver"),
```

### 4b. `target()` – extract the driven path

The first child of `DefaultDriver` is the `Path` node, same as for `Driver`.

```diff
-    match &self.payload {
-        AstNodePayload::Driver(_) | AstNodePayload::InitialDriver => {
+    match &self.payload {
+        AstNodePayload::Driver(_) | AstNodePayload::DefaultDriver | AstNodePayload::InitialDriver => {
             let target_node = self.child(0);
             target_node.path()
         }
```

### 4c. `driver()` – extract the RHS expression

```diff
-    match &self.payload {
-        AstNodePayload::Driver(_) | AstNodePayload::InitialDriver => Some(self.child(1)),
-        _ => None
-    }
+    match &self.payload {
+        AstNodePayload::Driver(_) | AstNodePayload::DefaultDriver | AstNodePayload::InitialDriver => Some(self.child(1)),
+        _ => None
+    }
```

---

## Phase 5 – `DriverType::Default` (optional alternative)

**`src/common.rs`**

There are two approaches:

- **A)** Reuse `DriverType::Continuous` for `?=` (the CONCEPT says "It is
  continuous, like `:=`") and store the semantic difference only in the
  payload type (`DefaultDriver` vs `Driver`).
- **B)** Add `DriverType::Default { Default }` to the `DriverType` enum for
  extra clarity in pattern-matching code.

**Recommendation: Approach A**.  The `Driver` struct + `DriverType` are used
inside `DriverAnalysis` to build the `Driver` enum tree (Expr/When/Match).
Default drivers are handled separately from the start, so they never need
to become `Driver::Expr(Default, _)`.  Keeping the payload-level distinction
(`DefaultDriver` in the AST) is sufficient.

If approach B is preferred later, it's a trivial refactor.

---

## Phase 6 – Diagnostics

**`src/diagnostics.rs`**

Add a new diagnostic payload:

```rust
 /// `?=` used outside a `platform` definition.
 DefaultDriverNotInPlatform(DefaultDriverNotInPlatform),
```

And the struct:

```rust
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct DefaultDriverNotInPlatform {
     pub target: BString,
 }
```

Add rendering in the `match` block that maps payloads to human-readable
messages (around line 400+):

```rust
 DiagnosticPayload::DefaultDriverNotInPlatform(d) => {
     format!("{}default driver `?=` may only appear inside a platform definition", location_fmt)
 }
```

---

## Phase 7 – Driver Analysis

**`src/analysis/drivers.rs`**

### 7a. `collect_block_drivers` – skip `DefaultDriver`

The existing `collect_block_drivers` function handles `AstNodePayload::Driver`
cases.  Add an arm for `AstNodePayload::DefaultDriver` that **skips** the
statement entirely for the purposes of regular driver collection (but still
validates the path resolves, for early error reporting).

Pseudo‑code:

```rust
AstNodePayload::DefaultDriver => {
    // Resolve the target path for early error reporting (unresolved component).
    // Do NOT push a Driver::Expr into the result – default drivers are
    // not counted for the "every port must be driven" rule.
    let Some(target) = stmt.target() else { continue };
    let target_str = stmt.parsing.string(target).to_owned();
    // Resolve for diagnostic purposes only.
    let _ = component_analysis.resolve(target_str.as_bstr()) else {
        diagnostics.push(Diagnostic::new(
            stmt.region(),
            diagnostics::UnresolvedComponent { path: target_str },
        ));
        continue;
    };
    // Default drivers are also typechecked in the platform context
    // (Phase 9), so we don't do that here.
}
```

### 7b. New function: `collect_default_drivers`

A standalone function that walks a **platform** AST and returns a
`IndexMap<ComponentId, Location>` or `Vec<(BString, Location)>` of default
driver assignments.  This is called from the platform‑level code (Phase 9).

```rust
pub(crate) fn collect_default_drivers(
    stmts: Vec<AstNode<'_>>,
    component_analysis: &ComponentAnalysis,
) -> IndexMap<ComponentId, Location> {
    let mut result: IndexMap<ComponentId, Location> = IndexMap::new();
    for stmt in stmts {
        if matches!(stmt.payload(), AstNodePayload::DefaultDriver) {
            let Some(target) = stmt.target() else { continue };
            let target_str = stmt.parsing.string(target).to_owned();
            if let Some(component) = component_analysis.resolve(target_str.as_bstr()) {
                result.insert(component.id(), stmt.driver().unwrap().location());
            }
        }
    }
    result
}
```

(This may live in a new `src/analysis/default_driver.rs` helper instead, if
the file grows too large.)

---

## Phase 8 – Check Drivers

**`src/queries/check_drivers.rs`**

### 8a. `DefaultDriverNotInPlatform` semantic check

Add a new query (or augment the existing `check_drivers`) that scans a
module's AST for `DefaultDriver` nodes and emits
`DefaultDriverNotInPlatform` if found outside a platform:

```rust
pub(crate) fn check_default_driver_in_platform_only(
    builder: &mut Builder,
    symbol_id: SymbolId,
) -> Arc<Vec<Diagnostic>> {
    let mut diagnostics = vec![];
    let symboltable = builder.get_symboltable();
    let symbol = symboltable.symbol(symbol_id);

    // Skip if the symbol is a platform (allowed there).
    if symbol.kind() == SymbolKind::Platform {
        return Arc::new(diagnostics);
    }

    // For moddefs (or other items), walk the AST for DefaultDriver nodes.
    let parsing = builder.get_parsing(symbol.package());
    let node = parsing.ast_node(symbol.location().ast_node_id());
    let mut stack: Vec<AstNodeId> = vec![node.id()];
    while let Some(node_id) = stack.pop() {
        let current = parsing.ast_node(node_id);
        if matches!(current.payload(), AstNodePayload::DefaultDriver) {
            if let Some(target) = current.target() {
                let target_str = parsing.string(target).to_owned();
                diagnostics.push(Diagnostic::new(
                    current.region(),
                    diagnostics::DefaultDriverNotInPlatform { target: target_str },
                ));
            }
        }
        for child in current.children() {
            stack.push(child.id());
        }
    }

    Arc::new(diagnostics)
}
```

This should be called from the top‑level check harness (likely in
`src/analysis/checker.rs` or similar) for every item.

### 8b. Exempt default‑driven ports from `NoDrivers`

When checking `NoDrivers` for a moddef that has a `for` clause, ports that
carry a default driver in the platform should be exempted.

In `check_drivers`, after detecting a missing driver for a component,
look up the platform's default drivers:

```rust
// Inside the "NoDrivers" branch:
if component.can_sink() {
    let drivers = driver_analysis.drivers().get(&component.id());
    if drivers.is_none() || drivers.is_empty() {
        // Check if this port has a default driver in the platform.
        if let Some(platform_id) = resolve_platform_for(builder, symbol_id) {
            let platform_defaults = builder.get_platform_default_drivers(platform_id);
            // ^ new method on Builder
            if platform_defaults.contains_key(&component.id()) {
                continue; // exempt – default driver covers it
            }
        }
        // … existing NoDrivers / NoRegDrivers diagnostics …
    }
}
```

---

## Phase 9 – Platform Default Drivers Query

**`src/analysis/platform.rs`**

Add a new function `platform_default_drivers` that returns the default
driver mappings for a platform:

```rust
/// Returns a map from component (port) path to the location of the default
/// driver expression, for every `?=` statement in the platform.
pub fn platform_default_drivers(
    builder: &mut Builder,
    platform_symbol_id: SymbolId,
) -> IndexMap<ComponentId, Location> {
    let component_analysis = builder.get_component_analysis(platform_symbol_id);
    let symboltable = builder.get_symboltable();
    let symbol = symboltable.symbol(platform_symbol_id);
    let parsing = builder.get_parsing(symbol.package());
    let node = parsing.ast_node(symbol.location().ast_node_id());

    // Walk top-level children for DefaultDriver nodes.
    let children = item_children(&node);
    collect_default_drivers(children, &component_analysis)
}
```

Add a `get_platform_default_drivers` method to `Builder` (or `Db`) so that
other passes can query this.

---

## Phase 10 – Verilog Conversion

**`src/verilog/conversion.rs`**

When converting a moddef that has a `for` clause, after processing all
explicit drivers, collect the platform's default drivers.  For any
component that has a default driver but **no explicit driver** in the
module, emit the default expression as the driver.

Key location: the function that converts module statements to Verilog
elements (around the main `convert_module` function).  After building
the per‑component driver map, inject the platform defaults for
components that have no explicit driver.

Pseudo‑code:

```rust
// After collecting all module-level drivers:
if let Some(platform_id) = resolve_platform_for(builder, moddef_symbol_id) {
    let platform_defaults = builder.get_platform_default_drivers(platform_id);
    for (comp_id, default_loc) in platform_defaults {
        if !module_drivers.contains_key(comp_id) {
            // No explicit driver – use the default.
            // Convert the default expression to Verilog.
            let expr_ast = /* get the expression node from default_loc */;
            let v_expr = convert_expr_to_verilog(expr_ast);
            module_drivers.insert(comp_id, Driver::Expr(DriverType::Continuous, v_expr));
        }
    }
}
```

> **Note:** The CONCEPT says "Do not evaluate it – just compile it to
> Verilog."  So the default expression is treated as a constant and
> emitted directly as the right‑hand side of the corresponding Verilog
> assignment (inside an `always` block, exactly like a `:=` driver).

---

## Phase 11 – Expression must be constant

The CONCEPT states: "The expression MUST be a constant expression.  Do
not evaluate it – just compile it to Verilog."

The simplest implementation is to rely on the existing type‑checking
machinery: the expression is typechecked in the platform context just
as if it were in a moddef.  If the expression refers to external paths
(e.g., non‑port identifiers), the type‑check pass will emit an
`UnresolvedComponent` or similar error, which is sufficient.

If a stronger check is desired later, a dedicated `DefaultDriverNonConst`
diagnostic can be added.  For the initial implementation, the existing
type‑checking infra is enough.

---

## Phase 12 – Builder / Db plumbing

**`src/db.rs`** (+ `src/db/guts.rs`)

Add a method to `Builder`:

```rust
pub fn get_platform_default_drivers(
    &mut self,
    platform_id: SymbolId,
) -> IndexMap<ComponentId, Location> {
    self.builder.get_or_compute(
        format!("platform-default-drivers:{:?}", platform_id),
        || platform_default_drivers(self, platform_id),
    )
}
```

---

## Phase 13 – Tests

### 13a. Passing tests

| Test | File | What it checks |
|------|------|----------------|
| Default driver in platform | `tests/pass/platform_default_driver/src/top.vir` | Platform with `tx ?= true`; moddef `for` that platform that does NOT drive `tx`; top module instantiates and verifies the default flows through. |
| Default driver overridden | Same file, second moddef | Platform with `tx ?= true`; moddef declares `incoming tx : Bit` and drives it explicitly with `tx := something`. |

### 13b. Error tests

| Test | File | What it checks |
|------|------|----------------|
| `?=` in moddef | `tests/errors/default_driver_in_moddef.vir` | A plain moddef (not inside a platform) uses `?=`.  Expects `DefaultDriverNotInPlatform`. |

### 13c. Existing test updates

- `tests/pass/platform_for_clause/` – update to exercise the `?=` default
  driver in one of the platforms.

---

## Implementation order

1. Lexer + grammar + regenerate parser
2. AST payload and accessors
3. Diagnostics
4. Driver analysis: skip `DefaultDriver` in `collect_block_drivers`; write `collect_default_drivers`
5. Platform query: `platform_default_drivers`
6. Check drivers: `DefaultDriverNotInPlatform` check; exempt default‑driven ports from `NoDrivers`
7. Db / Builder plumbing
8. Verilog conversion: inject default drivers for undriven ports
9. Tests

---

## Open questions / future work

- **`:=` / `<=` / `:=:` inside platforms**: Should these be allowed or
  rejected?  This plan leaves them syntactically valid (via the extended
  `PlatformStmt`) but semantically unsupported (they are not collected by
  `collect_default_drivers` and will be silently ignored or cause errors
  elsewhere).  A follow‑up could add a
  `NonDefaultDriverInPlatform` diagnostic.

- **Non‑constant expression in default driver**: The CONCEPT defers
  const‑checking to the type system.  If tighter checks are desired, a
  dedicated `DefaultDriverNonConst` diagnostic can be added later.

- **Platform‑level `when`/`match` as default drivers**: Not required by
  the CONCEPT, but could be a future extension.