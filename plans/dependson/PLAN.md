# Plan: dependson

Date:   2026-11-01
Status: In Planning

# Summary

`dependson` becomes a modifier on component declarations: the module's
declaration of which `incoming` ports its `outgoing` (and `outgoing wire`)
ports *may* combinationally depend on. It is part of the module's public
interface contract, enabling combinational analysis across module boundaries
without looking inside child modules.

Core semantic points:

* `dependson` **does not create** a combinational dependency. It declares
  *potential* ("may depend").
* The combinational **cycle check operates on potentiality**: a cycle in the
  `dependson` relation is an error, even if no actual comb path exists today.
  This is intentional: a designer must be free to make any declared dependency
  actual at any time without breaking clients.
* The declared set must be **exhaustive**: every actual (and potential, via
  composition) combinational dependency of a port must be declared.
  Declaring more than actual is a warning (stale declaration).
* Declarations compose across submodule boundaries and socket links, so a
  cycle check is sound while remaining local to each module.

# Resolved Design Decisions

1. **Submodule contract.** The declared `dependson` set is the boundary
   contract. Parents analyze children using declarations only — never by
   looking inside a child. Each module verifies its own internals against its
   own declarations, which keeps declarations honest and boundary edges
   trustworthy.
2. **Cycle check = potentiality.** The cycle check runs over the declared
   relation (local declared edges + local actual comb edges + edges imported
   from children's declarations). A cycle is an error regardless of whether
   the actual comb path exists. Separate from this, the *actual* dependency
   graph (from drivers) drives the staleness warning inside the declaring
   module.
3. **`outgoing reg`: always an error.** A registered port can never change
   same-cycle, so a declared comb dependency on it would be permanently
   false. `dependson` on `outgoing reg` is rejected.
4. **Legal carriers (regular components).** Only `outgoing` and
   `outgoing wire` may carry `dependson`. `incoming`, `outgoing reg`, `bus`,
   `net`, `wire`, `reg` are errors, each with a kind-specific message. The
   grammar accepts the clause on any component so diagnostics can be precise.
5. **`bus`: banned, semantics specified for later.** A bus's observable value
   is the resolution of all attached drivers, so the naive declaration is
   ill-defined. The proper future semantics is *contribution sensitivity*:
   the module's drive (value + enable) on the bus may depend on the listed
   incomings; attach (`#=#`) becomes a bidirectional potential edge in the
   cycle graph. Deferred to the tristate plan; error for now.
6. **Exhaustiveness enforced as an error.** For every `outgoing` /
   `outgoing wire`, actual dependencies ⊆ declared set. An actual comb
   dependency on an undeclared incoming is an error at the declaring module.
   Over-declaration (stale entry) is a warning. Absence of a clause is an
   *empty declaration*: any non-trivial `outgoing` effectively requires a
   clause.
7. **Migration:** existing code in the repo is fixed up as part of this plan.
8. **Socket channels.** `dependson` on a `cosi` channel applies when the
   socket is instantiated as a `client` (cosi = outgoing there); on a `soci`
   channel, as a `server`. Entries must be channels of the opposite direction
   word in the same socketdef. In the role where the channel is incoming, the
   clause is **inert**. Socket-derived obligations are exempt from the
   staleness warning (Wishbone/TL legitimately vary their latency) but not
   from exhaustiveness.
9. **Statement form removed.** `lhs dependson rhs` as a `ModDefStmt` goes
   away; the clause form replaces it.

# Language Spec

## Syntax

`dependson` is a clause on a component declaration, between the type and the
`default`/`on` clauses:

```top
pub mod Top {
    incoming inp : Bit
    incoming inp_0 : Bit
    incoming inp_1 : Bit

    outgoing out : Bit dependson inp {
        it := inp
    }

    outgoing out_1 : Bit dependson inp_0, inp_1 {
        out_1 := inp_0 && inp_1
    }
}
```

* Comma-separated list of paths.
* Grammar accepts the clause on **any** component kind (for better error
  messages); the checker restricts it.
* Socket channels take the clause in socket definitions:

```top
socket Wishbone {
    cosi cyc : Bit
    cosi stb : Bit
    soci ack : Bit dependson cyc, stb
}
```

## Meaning

* `outgoing p : T dependson x, y` declares: the value of `p` **may**
  combinationally depend on `x` and `y`. It does not assert that it does.
* Entries must resolve to `incoming` ports of the same module. Anything else
  (outgoing ports, regs, internal signals, paths into instances) is an error.
* On socket channels: a `cosi` channel's entries must be `soci` channels of
  the same socketdef, and vice versa.

## Verification (per module)

Define the module's **potentiality graph**:

* *Local actual edges*: combinational edges from the module's own drivers
  (existing `DependencyGraph` `Combinational` edges, including through
  instance channels and socket bulk connects).
* *Local declared edges*: from the module's own `dependson` clauses (on
  outgoing/outgoing-wire components, and on its own socket *ports* — see
  below).
* *Imported declared edges*: for each submodule instance `inst of C`, for
  each outgoing port `p` of `C` with declared entries `{q1..qn}` (all
  incoming ports of `C`), the edges `inst.p -> inst.qi`. Likewise for socket
  obligations at `client`/`server socket` instances: for a `client` instance,
  a `cosi` channel `c` with declared soci entries `{e1..en}` yields
  `inst.c -> inst.ei`; for a `server` instance, a `soci` channel `s` with
  cosi entries yields `inst.s -> inst.ei`.
* Sequential (`reg`) edges are never traversed: a clock edge breaks
  potentiality.

Three checks run over this graph:

1. **Exhaustiveness (error).** For each `outgoing`/`outgoing wire` port `p`,
   everything *reachable* from `p` must be declared: the set of `incoming`
   ports reachable from `p` (traversing local actual edges, local declared
   edges, and imported declared edges) must be ⊆ `declared(p)`.
   Rationale: reachability through a child's *declared* edge counts, because
   the child reserves the right to make it actual.
2. **Staleness (warning).** `declared(p)` minus the *justified* set of `p`
   is nonempty → warning. The justified set of `p` is reachability from `p`
   over local actual edges, imported declared edges, and declared edges of
   components **other than `p`** (a declaration is justified by another
   port's declared potential flowing through it; a port's own declaration
   cannot justify itself).
3. **Cycles (error).** Any cycle in the potentiality graph (all three edge
   classes, including a port's own declared edges) is an error. Reported at
   the module whose local edge closes the cycle (same policy as today).

Socket-derived obligations participate in (1) and (3) fully, but are exempt
from (2) — a containing module that does not currently exercise a declared
socket dependency gets no staleness warning.

## Invalid placements (all errors, checker-level)

| Placement | Message |
|---|---|
| `outgoing reg` | "`dependson` is not allowed on `outgoing reg` (a registered output cannot change in the same cycle)" |
| `incoming` | "`dependson` is not allowed on `incoming`" |
| `bus` | "`dependson` is not supported on `bus` (tristate components are not yet covered)" |
| `net`, `wire`, `reg` | "`dependson` is not allowed on internal signals" |
| entry resolves to non-`incoming` | "`{entry}` is not an `incoming` port of this module" |
| socket entry of same direction | "a `cosi` channel may only `dependson` `soci` channels of the same socket" |

# Implementation

## Phase 1 — Grammar and AST

Files: `virdant/src/syntax/virdant.lalrpop`, `virdant/src/syntax/payload.rs`
(or wherever `AstNodePayload` lives), `virdant/src/syntax/token.rs` (token
already exists), `virdant/src/syntax/tests.rs`.

* Add a `DependsonClause` grammar rule: `"dependson" <paths: Comma<Path>>`,
  producing `AstNodePayload::DependsonClause` with the paths as children.
* Extend `ModDefStmtComponent` for **all eight component forms** to accept an
  optional `DependsonClause` (after the type, before `DefaultClause`).
  Update `num_children` arithmetic accordingly.
* Extend `SocketDefStmt` `cosi`/`soci` rules with the same optional clause.
* Add `dependson: Option<AstNodeId>` to `payload::Component` and
  `payload::Channel`.
* **Remove** `ModDefStmtDependsOn` and `AstNodePayload::ModDefStmtDependsOn`.
* Regenerate `GRAMMAR.txt` (`make grammar`).
* Rewrite syntax tests: remove statement-form tests (`test_parse_dependson`,
  `test_parse_dependson_standalone`), add clause-form tests (single entry,
  multiple entries, on socket channels, on each component kind, error
  recovery cases).

## Phase 2 — Component analysis

Files: `virdant/src/analysis/component.rs`, new
`virdant/src/queries/dependson.rs`.

* Extend `Component` (or `ComponentAnalysis`) to carry, per component:
  * `declared_dependson: Vec<ComponentId>` — resolved entries; resolution
    failures recorded as diagnostics, not silently dropped.
  * For socket channels: the raw declared entries plus enough info to
    translate obligations at instantiation sites.
* New query `get_dependson_analysis(symbol_id) -> Arc<DependsonAnalysis>`
  computing, per module:
  * validated carrier kinds (kind-specific errors from the table above),
  * validated entries (must resolve to `incoming` ports of the same module;
    socket direction rule for socketdef channels),
  * the module's local declared edges (components + socket-port obligations),
  * the imported declared edges (submodule instances' port declarations,
    socket instances' channel obligations),
  * the staleness justification sets and exhaustiveness closure per outgoing
    port, using the potentiality graph defined above.

## Phase 3 — Diagnostics

Files: `virdant/src/diagnostics.rs`, `virdant/src/queries/check.rs` (wire the
new query into the check pipeline).

New diagnostics:

* `DependsonBadCarrier(kind)` — error, kind-specific messages.
* `DependsonBadEntry(path, reason)` — error (not an incoming / wrong socket
  direction / unresolved).
* `DependsonUndeclared { port, missing }` — error: "`{port}` could
  combinationally depend on `{missing}` but does not declare it; add it to
  `{port}`'s `dependson`".
* `DependsonStale { port, entry }` — warning: "`{port}` declares it may
  depend on `{entry}` but does not" (suppressed for socket-derived
  obligations).
* `DependsonCycle { cycle }` — error: "cycle in the `dependson` relation".

## Phase 4 — Cycle check rewrite

Files: `virdant/src/queries/combinational_cycles.rs`,
`virdant/src/analysis/dependency.rs`.

* Remove `collect_dependson_edges` (the statement-form walker) from
  `dependency.rs`. Driver-derived comb/seq edges stay.
* Rewrite `combinational_cycles.rs`: today it stitches each submodule's full
  recursive graph into the parent. Replace with the local potentiality graph:
  local actual edges + local declared edges + imported declared edges only.
  No recursion into children's internals. Cycle detection stays per-module,
  bottom-up in inclusion order is no longer required for correctness (the
  imported edges come from declarations, not from children's graphs), but the
  reporting policy "error at the module whose local edge closes the cycle"
  is kept.
* Socket link cycles (client declares `req dependson resp`, server declares
  `resp dependson req`) surface naturally when the relation composes at the
  containing module.

## Phase 5 — Sweep existing code

* `tests/`, `tutorial/`, `lib/*.vir`: add `dependson` clauses to every
  `outgoing`/`outgoing wire` whose drivers (transitively) read an incoming;
  verify no stale declarations (or accept the warnings where illustrative —
  prefer clean).
* Update filecheck expectations if any output ordering shifts (none
  expected: `dependson` produces no Verilog).
* This sweep doubles as an audit: every example must state what its outputs
  could depend on.

## Phase 6 — Tests

* Syntax: clause parsing on all component kinds, socket channels, multiple
  entries, statement form now a parse error.
* Checker: each diagnostic fires in a minimal module; exhaustiveness with
  transitive (through-wire, through-instance) dependencies; staleness
  justification via another port's declaration.
* Cycles: local declared self-cycle; two-module cycle through instance
  boundary; three-module; socket-link cycle (client + server both declare);
  non-cycles that today's actual-only check would miss but potentiality
  catches; legitimate feedback through regs must NOT error (sequential edges
  break potentiality).
* Sockets: obligation transfer for client/server; inert-in-wrong-role;
  socket-derived staleness suppressed; exhaustiveness still enforced.

# Out of Scope / Future Work

* `bus`/`net` `dependson` via contribution semantics (design recorded in
  Decision 5; lands with the tristate plan, including attach edges in the
  potentiality graph).
* `tree-sitter-virdant`, `virdant.nvim` highlighting, LSP hover/completion.
* Tutorial prose; workspace-level docs.
* Any timing-analysis use of `dependson` (D-pin/enable declarations on regs).