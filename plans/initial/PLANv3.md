# PLANv3: Reg Initialization (`initial <reg> <= <const>`)

Implements `plans/initial/CONCEPT.md`: regs on FPGAs need a way to declare an
initial value. The Verilog `initial` block already exists — the compiler
currently emits `counter = 0` for every reg. This feature lets the user
override that default with a constant.

PLANv2's implementation strategy was deficient in several ways; this version
corrects them. The headline changes from PLANv2:

- **No second analysis pass.** Initial statements are collected by the
  *existing* `collect_block_drivers` walk in `analysis/drivers.rs`, which
  already owns it-context resolution, It-block recursion, and when/match
  recursion. A separate pass would duplicate ~200 lines of that machinery.
- **Resolve to `ComponentId`, don't rewrite strings.** The target path is
  resolved to a `ComponentId` at one site; everything downstream (drivers map,
  initials map, dependency graph, verilog emitter, sim) is already
  ComponentId-native. No `rewrite_it_path` helpers.
- **The `initials` map lives on `DriverAnalysis`**, not on a new query result.
- **Sim reality check:** the simulator initializes regs to `Value::X`, not `0`
  (`sim/state.rs::State::new`). Pluming initial values through is real work,
  not a confirmation exercise.
- **OutgoingReg fix:** PLANv2 built Verilog init stmts from `Element::Reg`
  entries, which misses `OutgoingReg` (emitted as `output reg` port, no
  `Element::Reg`). The init block is built from the initials map instead.
- **Panic fix:** `analysis/package.rs::add_item_expr_roots` ends with
  `_ => unreachable!(...)`. Once the grammar produces `InitialDriver` nodes,
  this panics at runtime unless the new payloads are handled there.

## Syntax

```virdant
mod Top {
    incoming clock : Clock

    reg counter : Word[3] on clock {
        initial counter <= 7

        when {
            case it > 0 {
                it <= it - 1
            }
        }
    }
}
```

The statement grammar is: `"initial" <PATH> "<=" <EXPR>`.

Expected Verilog (approximate):

```verilog
module Top;
    input clock;

    reg [2:0] counter;

    initial begin
        counter = 3'd7;
    end

    always @(posedge clock) begin
        counter <= (counter + 3'd1);
    end
endmodule
```

## Accepted Design Decisions

- `initial ...` is a new statement kind. It is **not** a normal driver: a reg
  with an `initial` statement still needs exactly one regular latched driver
  (`when`/`case`), otherwise `NoRegDrivers` fires.
- The `initial` statement may appear anywhere a normal statement may appear — it
  does not *need* to be inside a reg's driver block, but it typically will be.
- Inside an `it`-context block, `initial it <= 7` works (path resolves to the
  enclosing reg).
- **Placement:** `initial` may not appear inside a `when` or `match` block.
  That is an error.
- The RHS must be a constant expression (no component references). Any type
  compatible with the target reg is allowed — not restricted to `Word`.
- Allowed only on `reg` / `outgoing reg` components (`ComponentKind::Reg` /
  `OutgoingReg`). Using it on a wire, port, or incoming component is a compile
  error.
- There is **no compile-time constant evaluation**. The expression tree is
  lowered to Verilog as-is. The existing typechecker handles type/width
  compatibility; the Verilog emitter uses the target reg's width to format
  literals (e.g., `3'd7`). No `InitialNotConstant` or `InitialDoesntFit`
  diagnostics.

## Validation Rules (Phase 3)

| # | Rule | Error | Handled where |
|---|------|-------|---------------|
| A | Target must be a register kind (reg / outgoing reg) | `InitialForNonReg` | `check_initial` |
| B | At most one `initial` statement per register | `InitialMultiple` | `check_initial` |
| C | Target path must resolve to a component in scope | reuse `UnresolvedComponent` | collection walk |
| D | `initial` does not count as a driver | reuse `NoRegDrivers` (existing) | collection walk (initials never enter `drivers`) |
| E | `initial` may not appear inside `when` or `match` | `InitialInsideWhenMatch` | collection walk |
| F | RHS type must be compatible with target reg type | handled by existing typechecker | expr roots + `build_expected_type` |

---

## Phase 1 — Lexer & Grammar *(done, staged)*

Files: `virdant/src/syntax/token.rs`, `virdant/src/syntax/virdant.lalrpop`,
`virdant/src/syntax/payload.rs`, `virdant/src/syntax/ast.rs` (display arm),
`GRAMMAR.txt` (regenerate via `make grammar`).

1. `#[token(b"initial")] KwInitial` on `Token`; `"initial"` added to
   `KEYWORDS`. ✅
2. `"initial" => Token::KwInitial` in the grammar's `extern Token` table. ✅
3. Grammar rule `ModDefStmtInitial`:
   `<ll:@L> "initial" <lhs:Path> "<=" <rhs:Expr> <rr:@R>` producing payload
   `AstNodePayload::InitialDriver` with 2 children (path, expr). ✅
4. `ModDefStmtInitial` added to the `ModDefStmt` alternatives. ✅
5. Display arm in `ast.rs` (`"InitialDriver"`). ✅

## Phase 1.5 — AST accessors

File: `virdant/src/syntax/ast.rs`.

Extend the existing accessors to match the new payload so downstream code can
treat an initial statement like a driver statement:

- `target()`: also match `AstNodePayload::InitialDriver` → `child(0).path()`.
- `driver()`: also match `AstNodePayload::InitialDriver` → `child(1)` (the RHS
  expr node).

No new accessor names, no path-rewriting helpers.

## Phase 2 — *(removed)*

No constant-evaluation phase. The expression is lowered to Verilog as-is.

## Phase 3 — Collection & Diagnostics

Files: `virdant/src/analysis/drivers.rs`, new `virdant/src/queries/check_initial.rs`,
`virdant/src/diagnostics.rs`, `virdant/src/db.rs`.

### Data structures

```rust
// analysis/drivers.rs — DriverAnalysis gains one field
pub struct DriverAnalysis {
    drivers: IndexMap<ComponentId, Vec<Driver>>,    // unchanged
    initials: IndexMap<ComponentId, Vec<Location>>, // NEW: RHS expr Location per component
    diagnostics: Vec<Diagnostic>,                   // unchanged
}
```

- Keyed by resolved `ComponentId` (through the it-context machinery already
  threaded into `collect_block_drivers`).
- Values are `Location`s of the RHS expr nodes — the codebase's established
  pattern (`Driver::Expr(dt, Location)`); downstream re-fetches via
  `db.get_parsing(loc.package()).ast_node(loc.ast_node_id())`.
- `Vec<Location>` (not `Option`) so Rule B can report *each* offending
  statement.
- Initial statements never enter the `drivers` map, so Rule D is automatic:
  `NoRegDrivers` / `MultipleDrivers` logic is untouched.

### Collection (one new arm)

In `collect_block_drivers`, add:

```rust
AstNodePayload::InitialDriver => { ... }
```

- Resolve the target path to a `ComponentId` using the `it_context` already
  threaded into the function (equivalent to the ancestor-walk used by
  `typing.rs::build_expected_type` and `sim/circuit.rs::rewrite_it_in_name`):
  `it` → the context component; `it.x` → context path + `.x` composed once;
  anything else resolved directly. On failure emit `UnresolvedComponent` with
  the user's original path text (Rule C).
- If inside a `when`/`match` arm (the recursion into `ModDefStmtWhen` /
  `ModDefStmtMatch` bodies must carry a conditional-context flag, or the arm
  handles it at the recursion site), emit `InitialInsideWhenMatch` instead of
  collecting (Rule E). The ancestor-walk formulation resolves `it` correctly
  even through nested conditionals.
- Otherwise push the RHS `Location` into `initials[component.id()]`.
- `build_driver_analysis` threads `initials` into the struct.

### Validation (Rules A & B) — new per-item pass

New `virdant/src/queries/check_initial.rs`, modeled on `check_drivers`:

```rust
pub(crate) fn check_initial(builder: &mut Builder, symbol_id: SymbolId) -> Arc<Vec<Diagnostic>>
```

For each `(comp_id, locs)` in `driver_analysis.initials()`:

- component kind is not `Reg | OutgoingReg` → `InitialForNonReg` per
  statement (Rule A);
- `locs.len() > 1` → `InitialMultiple` for each statement after the first,
  at its region (Rule B).

Regions via `builder.get_location_region(...)` on the statement/RHS location,
as in `check_wrong_driver_types`.

Register in `db.rs`:

- `CheckInitial(symbol_id: SymbolId) -> Arc<Vec<Diagnostic>>` in `queries!{}`
  and the `dispatch_build!` table;
- a `db_getter!` (`check_initial`);
- a loop in `check()` (`queries/check.rs`) alongside the `check_drivers` loop,
  skipped for `SymbolKind::Platform`.

### Diagnostic payloads

Follow the existing pattern in `diagnostics.rs` — each new error is a struct
carrying the component name as `pub target: BString`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialForNonReg {
    pub target: BString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialMultiple {
    pub target: BString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialInsideWhenMatch {
    pub target: BString,
}
```

Add to `DiagnosticPayload`:

```rust
pub enum DiagnosticPayload {
    // ... existing variants ...
    InitialForNonReg(InitialForNonReg),
    InitialMultiple(InitialMultiple),
    InitialInsideWhenMatch(InitialInsideWhenMatch),
}
```

### Error messages

Match the existing terse style (see `fn message()` in `diagnostics.rs`):

```rust
DiagnosticPayload::InitialForNonReg(d) => {
    format!("initial targets non-register component {}", d.target).into()
}
DiagnosticPayload::InitialMultiple(d) => {
    format!("multiple initial statements for register {}", d.target).into()
}
DiagnosticPayload::InitialInsideWhenMatch(d) => {
    format!(
        "initial statement for {} must not appear inside a when/match block",
        d.target,
    ).into()
}
```

Rule C reuses the existing `UnresolvedComponent` payload and message. All
three new diagnostics are `DiagnosticLevel::Error`.

### Typecheck (Rule F)

Files: `virdant/src/analysis/package.rs`, `virdant/src/queries/typing.rs`.

1. `package.rs::add_item_expr_roots`: handle `ModDefStmtInitial` — push the
   RHS expr node as an expr root. **This is mandatory, not optional: the
   function currently ends in `_ => unreachable!(...)` and will panic once the
   grammar emits `InitialDriver` nodes.**
2. `package.rs::add_moddefstmt_block_expr_roots`: same arm for statements
   inside blocks (incl. It-blocks).
3. `typing.rs::build_expected_type`: new arm for a parent of
   `AstNodePayload::InitialDriver` — expected type is the target component's
   type, resolved the same way as the existing `Driver(_)` arm (ancestor walk
   to the item, compose `it`, `component_analysis.type_of(path)`). Extract the
   shared ancestor-walk into a helper used by both arms rather than
   duplicating it.

## Phase 4 — Verilog Emission

File: `virdant/src/verilog/conversion.rs`.

Today every reg gets `counter = 0` in the initial block (default-init),
built from the `Element::Reg` entries. Change to build it from
`driver_analysis.initials()` keyed by `ComponentId`:

- Map `ComponentId → (valid_verilog_name(component.path()), type_width(typ))`.
- Regs **with** an initial: fetch the RHS AST node from its `Location` and
  lower it with `self.convert_expr(...)` (it is typed by the normal pipeline,
  so literals like `3'd7` format correctly) → `Stmt::AssignBlocking`.
- Regs **without** an initial: keep the existing `WordLit { value: 0, width }`
  default, discovered from the `Element::Reg` entries as today.
- Emit one combined `Element::Initial`. Output for init-less regs is
  unchanged (snapshot tests must not break).
- Keying by ComponentId (not `Element::Reg`) is what makes `OutgoingReg`
  work: it is emitted as an `output reg` port with **no** `Element::Reg`, so
  PLANv2's approach silently dropped its initial value.

## Phase 5 — Simulator

Files: `virdant/src/analysis/elaboration.rs`, `virdant/src/sim/expr.rs`,
`virdant/src/sim/sim.rs` (and possibly `state.rs`).

The sim works purely from `Elaboration` — it never sees `DriverAnalysis` —
and `State::new` initializes every component (incl. regs and their `set_vals`)
to `Value::X`, not `0`. So:

1. `analysis/elaboration.rs`: `ElaboratedComponent` gains

   ```rust
   pub initial_expr: Option<Location>,
   ```

   populated in `elaborate_module` from
   `builder.get_driver_analysis(symbol_id).initials()`.

2. `sim/expr.rs`: reuse the `Driver::Expr → Expr` conversion path
   (`driver_to_expr`) on the initial's `Location` — thin wrapper
   `initial_to_expr(db, loc)`.

3. `sim/sim.rs`: in `Sim::new`, after `flow_constants()`, for each elaborated
   component with `initial_expr`: build a `Context` via
   `Context::context_for_component`, eval the expr, and `state.set_val(...)`.
   `State::new` stays as-is; the override happens at `Sim` level where the
   `Circuit`/`Context` exist.

## Phase 6 — Tests

Two `.vir` files only (no Rust unit tests):

- `tests/pass/initial.vir` — all positive cases:
  - Basic `initial counter <= 7` with normal driver coexisting
  - `initial it <= 7` inside the it-block
  - `initial` on an `outgoing reg`
  - Some regs with explicit init, others defaulting to 0
  - All pass with `// CHECK: Check OK`

- `tests/errors/initial_errors.vir` — all error cases in one file:
  - Non-reg target (wire)
  - Multiple inits for the same reg
  - `initial` inside a `when` block
  - Unresolved target path

## Phase 7 — Docs & Chores

1. Regenerate `GRAMMAR.txt` (`make grammar`).
2. `bin/format.rs`: the line classifier detects `<=` and will mis-classify
   `initial counter <= 7` as a `LineKind::Driver` line (`before =
   "initial counter"`), corrupting op-column alignment. Guard: lines starting
   with the `initial` keyword are not Driver lines.
3. Confirm `analysis/dependency.rs` and `analysis/component.rs` treat
   `InitialDriver` as a no-op (no dependency edges — the RHS is a constant;
   no reference collection). Their wildcards should already cover this;
   verify, don't assume.
4. Update `agents/skills/virdant/driver-blocks.md` (agent-facing docs) with
   the `initial` statement.
5. Update `docs/source/` (language reference / driver docs) with the new
   statement and the corrected example from CONCEPT.md.

## Execution order

1. Phase 1.5 (ast.rs accessors — mechanical, unblocks everything)
2. Phase 3 collection arm + `DriverAnalysis.initials`
3. Expr-root arms (`package.rs`) + `build_expected_type` arm (typecheck works)
4. `check_initial.rs` + `diagnostics.rs` (Rules A/B surface)
5. Phase 4 verilog conversion (golden tests)
6. Phase 5 sim plumbing
7. Phase 6 tests, Phase 7 chores
