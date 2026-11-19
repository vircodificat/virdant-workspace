# TODO: clock-coloring

High-level tasks for implementing clock domains as described in `CONCEPT.md`.
Order reflects dependency, not strict schedule.

## 1. Domain model
- [x] Define the `Domain` type: `Sync(Ident)`, `Async`, `Constant`, with the
      lattice/upcasting relation (`Constant ≤ Sync(c) ≤ Async`).
      (Implemented as `analysis::domain::Domain` in
      `virdant/src/analysis/domain.rs`: `Domain::Sync(ComponentId)` —
      the clock named by component id, per CONCEPT — with `upcasts_to`,
      `lub`, `glb`, and a lattice-ordered `PartialOrd`.)
- [x] Add a `ClockColor` (or similar) field to component metadata so each
      component carries its domain alongside its type/typing info.
      (`Component::domain: Option<Domain>` in `analysis/component.rs`; `None`
      when undetermined — submodule-owned ports/channels (item 7) and
      components of zero/multi-clock modules. Populated from clock
      typing → `Async`, `async` → `Async`, explicit `on` → `Sync(c)`
      (path resolved to the clock's `ComponentId` in the post-pass), and
      unique-clock inference → `Sync(<unique clock>)` for every
      non-submodule-owned component left unset, including own socket
      channels and synthesized platform ports.)

## 2. Component declarations: `on` / `async`
- [x] Wire up parsing of `on` clauses and `async` keywords on `incoming`,
      `outgoing`, `wire`, `reg`, `memory`.
      (Grammar unified: `ModDefStmtComponent` now takes a `ComponentKind`
      nonterminal, with clause order `Type OnClause? DependsonList?
      DefaultClause? ItBlock?` — the `on`/`async` clause now precedes
      `dependson`.  `OnClause` is `"on" Expr | "async"`, so the bare
      `async` modifier is the domain clause on every kind, including
      `reg`/`outgoing reg`/`memory` and `bus`/`net`; existing sources
      written `dependson X on Y` were migrated to the new order.
      `payload::Component.is_async` is set from the clause; the
      `on_clause()` accessor and expr-root/typing wiring are
      order-agnostic and unchanged.)
- [x] Enforce that a register can never be `async` — now a dedicated domain
      error: the temporary `NotYetImplemented` gate on the `async` modifier
      was lifted with the clock checker (item 4), and `reg`/`outgoing reg`/
      `memory` declared `async` report `AsyncRegister` from the clock
      checker (`tests/errors/clocks/async_registers`); non-clocked kinds
      (`incoming`, `outgoing`, `wire`) are `async` legally.
      `MissingOnClause` correctly does not fire for async components (the
      `async` clause *is* the domain clause).  The driver-side half (a
      register's driver must be `Sync(register's clock)`) lands with item 6.
- [x] Enforce that `on` clauses only name clock-typed expressions (the clock
      port, or an inward socket channel).  (The typing pass types the clause
      expression against `Clock` for every component kind —
      `queries/typing.rs::build_expected_type` — and unresolved names are
      reported by reference resolution; locked in by
      `tests/errors/typing/on_clause`.)

## 3. Unique-clock inference
- [x] Count incoming clock ports (including inward channels on sockets) per
      module: zero, one, or many.
      (`analysis::component::incoming_clock_ports` classifies the
      registered components after the walk: non-submodule-owned,
      non-synthetic, `kind == Incoming`, typed `Clock` — covering the
      module's own `incoming` components and the inward (receiving)
      channels of its own sockets. Clock-typed non-incoming components
      (PLL outputs, `Bufg.O`, ...) and submodule-owned ports/subsockets
      do not count; the latter via the new `Component::submodule_owned`
      flag. `resolve_unique_clock` returns the one port when there is
      exactly one, and `build_component_analysis` memoizes it as
      `implicit_clock`.)
- [x] Unique clock: implicitly mark every component `on <clock>` unless it is
      explicitly `async`.
      (Already in place for the module's own components; extended to the
      channels of the module's own sockets. Clock-typed channels are
      fixed `Async`; submodule-instance ports/subsockets stay unset for
      item 7. Locked in by `tests/check/socket_clock`, and the
      socket-channel clock-count cases in
      `tests/errors/drivers/missing_on_clause`.)
- [x] Zero clocks: no valid `on` clause; every component must be explicitly
      `async` for the module to check (there is no clock candidate to be
      `Sync` on, so nothing may be left unset — `MissingDomainClause`,
      reason "the module has no clocks"; Clock-typed components are exempt
      since they are always `Async`, and regs/memories keep their
      `MissingOnClause`).  The zero-clock coloring itself is still skipped.
      (`tests/check/zero_clock`, and the zero-clock case in
      `tests/errors/clocks/async_registers`.)
- [x] Multiple clocks: no unique clock; every component must have an explicit
      `on` (or `async`) clause — missing clause is a clock-check error.
      (Regs/clock-needing memories keep their `MissingOnClause` from the
      component analysis; every other kind is diagnosed by the clock checker
      as `MissingDomainClause` — see item 4; locked in by
      `tests/errors/clocks/missing_domain_clause`.)

## 4. Clock checker pass
- [x] Add a new query (e.g. `clockcheck.rs`) that runs after typechecking and
      assigns a domain to every component and every subexpression.
      (`queries/clockcheck.rs` — the `ClockCheck(symbol_id)` query memoizes a
      `ClockAnalysis`: diagnostics plus a per-AST-node `Domain` map for later
      consumers (item 6).  Domains are context-free (bottom-up only), so
      results are memoized per node — a subtree shared between an `if` guard
      and its own expr root is inferred and diagnosed exactly once.)  NOTE:
      moddefs containing submodule instances skip the clockcheck entirely for
      now (submodule clock binding is item 7), with an Info-level `Todo` note
      at the module; the note is suppressed for standard-library packages so
      it does not pollute user projects.
- [x] Implement skip conditions:
      - [x] Unique clock + no `async` components → everything `Sync(clock)`, skip.
      - [x] Zero clocks → everything `Async`, skip.
- [x] Implement the `@unsafe_clocks` annotation to disable clockchecking for a
      module (all components still require explicit `on`/`async`).
      (Added to `KNOWN_ANNOTATIONS`; Clock-typed components are exempt from
      the clause requirement — they are always `Async`.)
- [x] Hook the pass into the top-level check aggregator (`check.rs`), with
      diagnostics sorted by region like the other checks.  (The check runs
      per `ModDef` right after the typecheck loop; `check()`'s global region
      sort orders the diagnostics.)

## 5. Expression domain rules
Implement the rules from CONCEPT.md in the checker:
- [x] Combinational operators: all operands (incl. `when` guards and mux
      selectors) must share one domain; result takes that domain.
      (Joining follows the lattice: `Constant` upcasts to the other
      operands' domain and `Sync(c)` upcasts to `Async`, but two operands on
      *distinct* clocks cannot be combined — reported as `DomainMismatch`
      once per expression.  Covers binops, unops, `word`, `mux`/`any`/`all`
      calls, casts, and `ExprZext`/`ExprSext`.)
- [x] Constant folding respected: `1 + 2` stays `Constant`.
- [x] `match`: subject and all arm results share one domain; patterns exempt.
- [x] `when`/`if`: all guards and results share one domain.  (In statement
      position the guards join among themselves; joining guards with the
      driven components is item 6.)
- [x] Array indexing: `array[index]` — array and index must share a domain;
      result takes the array's domain.  (Constant bit-index `x[3]` and
      slices `x[3..1]` carry the index in the payload, so only the subject
      is an expression; dynamic indices (`mem[dyn addr]`) join the index
      with the memory's domain.)
- [x] Struct field access: `struct->field` inherits the struct's domain.
- [x] Struct/ctor construction: all field expressions share one domain; result
      takes that domain.
- [x] Literals, enumerants, and unions of constants are `Constant`.
      (Holes `?`/dontcares also act as `Constant`: they adopt whatever
      domain the surrounding join produces.)

## 6. Connections and drivers
- [ ] Every operand of a connection joins to one domain.
- [ ] When driving a signal, the target's declared domain must equal the
      driving expression's domain (upcasting allowed, downcasting is an error).
- [ ] Reg drivers must be `Sync(reg's clock)`.

## 7. Submodule clock binding
- [ ] Each incoming clock on a submodule instantiation (port or socket)
      creates a free variable in the child's definition.
- [ ] Driving the submodule's clock port instantiates that variable, mapping
      child `Sync(c)` domains onto a parent clock domain.
- [ ] Reject connecting a non-clock (or wrong-clocked) signal to a clock port.

## 8. `builtin::Sync`
- [ ] Annotate the `Sync` primitive with `@unsafe_clocks` and add explicit
      `on`/`async` clauses per the CONCEPT.md example.
- [ ] Audit the rest of `lib/` (`ice40`, `xilinx`, `virt`) for modules that
      need explicit `on` clauses or `async` inputs.  (Started: the checker
      itself now forces the audit — `virt::CharRom` needed `async` on both
      components and was fixed.)

## 9. Diagnostics
- [x] Define error diagnostics for: domain mismatch (`DomainMismatch`),
      missing `on` in a multi-clock module and a missing `async` in a
      zero-clock module (`MissingDomainClause`, for the kinds
      `MissingOnClause` does not cover), `async` register
      (`AsyncRegister`); plus `ClockCheckSkipped`-style TODO note on skipped
      submodule modules (Info-level `Todo`).  An `on` clause in a
      zero-clock module: no dedicated diagnostic yet (the clause cannot
      resolve to a clock and fails in typing/reference resolution).
      Driving a clock with a non-clock: still open, item 7.
- [x] Add error-test cases under `tests/errors/` for each.
      (`tests/errors/clocks/{domain_mismatch,missing_domain_clause,
      unsafe_clocks_missing_clause,async_registers}` — the async test moved
      from `tests/errors/not_yet_implemented/` after the gate lift.)

## 10. Tests and docs
- [x] Golden/pass tests covering: unique clock (skipped check), multi-clock
      modules, async modules, `Constant` propagation, `@unsafe_clocks`.
      (`tests/check/socket_clock` and the socket-channel cases in
      `tests/errors/drivers/missing_on_clause` for unique-clock via
      sockets; `tests/check/multi_clock` for multi-clock modules with
      explicit `on` clauses — outgoing components now need clauses too;
      `tests/check/clock_color` for the full checker on mixed modules
      (Constant propagation, joins, guards/arms/mux/structs, zero-clock
      async, `@unsafe_clocks`); `tests/check/clockcheck_submodule_skip`
      for the submodule skip + TODO note.)
- [ ] Regression tests for the vestigial-clock pattern (`unused it`).
- [ ] Update `docs/` and the tutorial with the clock-coloring model.
- [ ] Update `GRAMMAR.txt` if grammar changes are needed.
