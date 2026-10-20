# PLANv2: Reg Initialization (`initial <reg> <= <const>`)

Implements `plans/initial/CONCEPT.md`: regs on FPGAs need a way to declare an
initial value. The Verilog `initial` block already exists — the compiler
currently emits `counter = 0` for every reg. This feature lets the user
override that default with a constant.

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

| # | Rule | Error |
|---|------|-------|
| A | Target must be a register kind (reg / outgoing reg) | `InitialForNonReg` |
| B | At most one `initial` statement per register | `InitialMultiple` |
| C | Target path must resolve to a component in scope | reuse `UnresolvedComponent` |
| D | `initial` does not count as a driver | reuse `NoRegDrivers` (existing) |
| E | `initial` may not appear inside `when` or `match` | `InitialInsideWhenMatch` |
| F | RHS type must be compatible with target reg type | handled by existing typechecker |

---

## Phase 1 — Lexer & Grammar

Files: `virdant/src/syntax/token.rs`, `virdant/src/syntax/virdant.lalrpop`,
`virdant/src/syntax/payload.rs`, `GRAMMAR.txt` (regenerate via `make grammar`).

1. Add `#[token(b"initial")] KwInitial` to `Token` (lexer) and add `"initial"`
   to `KEYWORDS` (it must not be usable as an identifier).
2. Add `"initial" => Token::KwInitial` to the grammar's `extern Token` table.
3. New grammar rule `ModDefStmtInitial`:
   `<ll:@L> "initial" <lhs:Path> "<=" <rhs:Expr> <rr:@R>` producing payload
   `AstNodePayload::InitialDriver` (see payload below). 2 children (path, expr).
4. Add `ModDefStmtInitial` to the `ModDefStmt` alternatives.
5. New payload struct `pub struct InitialDriver;` (target/expr are just the two
   child AST nodes; no extra fields needed).
6. Add `"initial"` to `GRAMMAR.txt` token list via `make grammar`.

## Phase 2 — *(removed)*

No constant-evaluation phase. The expression is lowered to Verilog as-is.

## Phase 3 — Analysis & Diagnostics

Files: `virdant/src/analysis/drivers.rs`, new `virdant/src/queries/check_initial.rs`,
`virdant/src/diagnostics.rs`, `virdant/src/db.rs` (new query), `virdant/src/queries.rs`.

1. In `collect_block_drivers`, skip `InitialDriver` statements when gathering
   normal drivers (so they don't count towards the reg's single driver).
2. New analysis pass: walk each moddef's blocks collecting
   `IndexMap<ComponentId, Vec<(ExprNode, Location)>>` of initial statements.
   Handle the `it_context` path resolution the same way as normal drivers.
3. Validate per the rules in the table above. Emit at the statement's region.
4. Wire the new diagnostics check into `check()` in `queries/check.rs` (a
   per-item pass like `check_drivers`), and add the payload variants + fields
   to `diagnostics.rs`. Find the payload→message renderer and add messages.
5. Typecheck: already handled by existing typechecker. No additional check.

### New diagnostic payloads

Follow the existing pattern in `diagnostics.rs` — each new error is a struct
carrying the component name as `pub target: BString`:

```rust
// diagnostics.rs — new structs

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

The `UnresolvedComponent` case (Rule C) reuses the existing payload and message
— no new variant needed.

### Diagnostic levels

All three new diagnostics are `DiagnosticLevel::Error`.

## Phase 4 — Verilog Emission

File: `virdant/src/verilog/conversion.rs`.

1. Today every reg gets `counter = 0` in the initial block (default-init).
   Change: for regs with a declared initial value, lower the RHS expression
   tree to Verilog instead of emitting `0`. All other regs keep `0`.
2. The existing code builds `init_stmts` from `Element::Reg` entries. Extend
   to look up the reg's initial expression (by component → verilog name) and
   lower it. The emitter has access to the target reg's width from type
   information, so it can format literals correctly (e.g., `3'd7`).
3. `Element::Initial` / `Stmt::AssignBlocking` already exist and are
   pretty-printed by `verilog.rs` — no emitter changes needed outside the
   expression lowering.

## Phase 5 — Simulator

Files: `virdant/src/sim/` (`state.rs`, `sim.rs`).

Investigate how the simulator initializes reg state. If it hardcodes 0, plumb
the declared initial values through so `vir sim` matches Verilog behavior. If
the simulator already reads from the Verilog-style IR, confirm it picks up the
initial expression correctly.

## Phase 6 — Tests

Two `.vir` files only (no Rust unit tests):

- `tests/pass/initial.vir` — all positive cases:
  - Basic `initial counter <= 7` with normal driver coexisting
  - `initial it <= 7` inside the it-block
  - Some regs with explicit init, others defaulting to 0
  - All pass with `// CHECK: Check OK`

- `tests/errors/initial_errors.vir` — all error cases in one file:
  - Non-reg target (wire)
  - Multiple inits for the same reg
  - `initial` inside a `when` block
  - Unresolved target path

## Phase 7 — Docs & Chores

1. Regenerate `GRAMMAR.txt` (`make grammar`).
2. Update `agents/skills/virdant/driver-blocks.md` (agent-facing docs) with the
   `initial` statement.
3. Update `docs/source/` (language reference / driver docs) with the new
   statement and the corrected example from CONCEPT.md.