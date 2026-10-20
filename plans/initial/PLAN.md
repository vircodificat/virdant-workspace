# PLAN: Reg Initialization (`initial <reg> <= <const>`)

Implements `plans/initial/CONCEPT.md` (with corrected syntax): regs on FPGAs
need a way to declare an initial value. The Verilog `initial` block already
exists — the compiler currently emits `counter = 0` for every reg. This feature
lets the user override that default with a constant.

## Syntax (final)

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

Note that the `initial` driver creates an `initial` block in Verilog.

## Design Decisions

- `initial ...` is a new statement kind. It is
  **not** a normal driver: a reg with an `initial` statement still needs
  exactly one regular latched driver (`when`/`case`), otherwise `NoRegDrivers`
  fires.
- The RHS must be a **constant** expression that fits the reg's width. If it
  isn't constant or doesn't fit, emit a compile error.
- Only `Word`-typed regs (incl. enums via their numeric values) are supported;
  the initial value is a numeric constant. Enum-ctor init values are out of
  scope for now.
- Allowed only on `reg` / `outgoing reg` components (`ComponentKind::Reg` /
  `OutgoingReg`). Using it on a wire/port is a compile error.

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

## Phase 2 — Constant Evaluation

Files: `virdant/src/types/typedef.rs` (extract helper), new
`virdant/src/queries/check_initial.rs`.

1. `eval_const_expr` in `typedef.rs` currently panics on non-literal nodes.
   Extract a fallible version, e.g.
   `fn try_eval_const_expr(node) -> Result<(WordValue, Option<Width>), ()>`
   into a shared location (e.g. `types/typedef.rs` made `pub(crate)`, or a new
   `util` const-eval module). Keep enum construction on top of it.
   Supported node kinds (minimal): `ExprWordLit`, `ExprNatLit` (bare `7`),
   `ExprParen`, `ExprAs`. Bare `Nat` must be accepted since `7` (no `w`
   suffix) is the natural way to write it.
2. The check (Phase 3) reports `InitialNotConstant` when evaluation fails.

## Phase 3 — Analysis & Diagnostics

Files: `virdant/src/analysis/drivers.rs` (or new `analysis/initials.rs`),
`virdant/src/queries/check_initial.rs`, `virdant/src/diagnostics.rs`,
`virdant/src/db.rs` (new query), `virdant/src/queries.rs`.

1. In `collect_block_drivers`, skip `InitialDriver` statements when gathering
   normal drivers (so they don't count towards the reg's single driver).
2. New analysis: walk each moddef's blocks collecting
   `IndexMap<ComponentId, Vec<(WordValue, Location)>>` of initial statements.
   Handle the `it_context` path resolution the same way as normal drivers
   (`initial it <= 7` inside an `it` block resolves to the reg).
3. Validation per initial statement (emit at the statement's region):
   - `InitialNotConstant`: RHS is not a constant expression.
   - `InitialDoesntFit`: value does not fit the reg's width (like `DoesntFit`).
   - `InitialForNonReg`: target is a wire/port/incoming (not a reg kind).
   - `InitialMultiple`: more than one initial statement for the same reg
     (reuse the `MultipleDrivers` shape: report each).
   - `UnresolvedComponent` when the target path doesn't resolve (reuse existing).
4. Wire the new diagnostics check into `check()` in `queries/check.rs` (a
   per-item pass like `check_drivers`), and add the payload variants + fields
   to `diagnostics.rs`. Find the payload→message renderer (grep for
   `DiagnosticPayload` `match` sites in the CLI/LSP) and add messages.
5. Typecheck: the RHS type must be compatible with the reg's type. If
   `typecheck` would flag it anyway, verify no false positive; otherwise add a
   targeted check in the initial pass.

## Phase 4 — Verilog Emission

Files: `virdant/src/verilog/conversion.rs`.

1. Today every reg gets `counter = 0` in the initial block (default-init).
   Change: for regs with a declared initial value, use that value; all other
   regs keep `0`.
2. The existing code builds `init_stmts` from `Element::Reg` entries (lines
   ~476-492). Extend to look up the reg's initial value from the Phase-3
   analysis (by component → verilog name), keeping the `WordLit { value,
   width, radix: Radix::Dec }` shape.
3. `Element::Initial` / `Stmt::AssignBlocking` already exist and are
   pretty-printed by `verilog.rs` — no emitter changes needed.

## Phase 5 — Simulator

Files: `virdant/src/sim/` (`state.rs`, `sim.rs`).

Check how the simulator initializes reg state; if it hardcodes 0, honor the
declared initial value so `vir sim`/examples match the Verilog behavior. If the
simulator already reads from the Verilog-style IR, confirm; otherwise plumb the
initial values through the same query used by Phase 4.

## Phase 6 — Tests

Files: `tests/pass/`, `tests/errors/`, possibly `tests/virformat/`.

1. `tests/pass/initial.vir` — the CONCEPT example; `// CHECK: Check OK`.
   Also cover: `initial it <= 7` inside the it-block, default 0 still applied
   to other regs, reg with initial + normal driver coexisting.
2. `tests/errors/initial_not_constant.vir` — RHS referencing a component.
3. `tests/errors/initial_doesnt_fit.vir` — `initial counter <= 9` for
   `Word[3]`.
4. `tests/errors/initial_non_reg.vir` — `initial` on a wire.
5. Run `make test` (needs `vir` + `filecheck` built: `cargo build --release`).
6. Add a cargo unit test in `virdant/src/syntax/tests.rs` parsing the CONCEPT
   snippet and asserting the `InitialDriver` payload exists.

## Phase 7 — Docs & Chores

1. Regenerate `GRAMMAR.txt` (`make grammar`).
2. Update `agents/skills/virdant/driver-blocks.md` (agent-facing docs) with the
   `initial` statement.
3. Update `docs/source/` (language reference / driver docs) with the new
   statement and the corrected example from CONCEPT.md.

## Risks / Notes

- `initial` becomes a reserved keyword — any existing code using `initial` as
  an identifier will break. Grep `lib/`, `tests/`, `docs/` for collisions
  before adding the keyword.
- LALRPOP conflict risk: `initial Path <= Expr` vs the existing `Path <= Expr`
  driver rule is unambiguous (leading `initial` keyword), but verify the parser
  builds cleanly.
- The existing default-init (`= 0`) emission means behavior for regs without
  `initial` is unchanged; only the value lookup changes.
- Keep `eval_const_expr` panic behavior for enum definitions intact; add the
  fallible variant alongside rather than changing the existing call sites.
