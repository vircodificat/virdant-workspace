# Tristate and Bidirectional Ports (v3)

**v3 supersedes `PLANv2.md`.** Per workspace rules, the highest version suffix is
canonical. `CONCEPT.md` takes precedence over any plan.

## Deltas from v2

The headline change: **there is exactly one `bus` keyword.** v2's "resolved" note
inventing `outgoing bus` / `incoming bus` mirror spellings is **reverted** — that
would have split one bidirectional concept into two views, defeating the point
of the feature. `incoming`/`outgoing` exist because those ports have a
direction; a `bus` port does not. One declaration, one keyword:

```virdant
bus   name : Type      // the third port kind (CONCEPT's exact syntax)
net   name : Type      // local shared signal
```

| Topic              | PLAN v2                                    | PLAN v3 (this document)                                |
|--------------------|--------------------------------------------|--------------------------------------------------------|
| Port naming        | `outgoing bus` (+ `incoming bus` mirror)    | **`bus`** — one keyword, no direction, no mirror       |
| Parent drive/join  | (unspecified precisely)                     | `it.b #= x` (drive), `n #= it.b` (join) — see §Net Join |
| Weak drive form    | "driver form left to implementation"        | `b #= pullup()` / `b #= pulldown()` (zero-arg builtin)  |
| Read-with-default  | `pullup(v)` expression                      | unchanged: `pullup(v)` / `pulldown(v)`                 |
| Net join lowering  | (unspecified)                               | `tran (a, b)` for internal joins; direct port wiring for parent connects |
| Net identity model | implicit                                    | explicit equivalence classes computed in elaboration    |

Everything else carries over from v2 with its v2 approvals intact: X/Z as net
properties (never values), `#=`, absence-as-Z, contention-always-an-error,
`pullup`/`pulldown` as `Bit`-only crossings, initialization rules, per-bit
analysis, sockets `bus` slots [approved], no multi-driver warnings [approved],
`unsafe_read` deferred [deferred], platform/IOBUF work deferred [deferred].

---

## Language Surface

### Declarations

```virdant
mod Tristate {
    incoming data   : Bit
    incoming enable : Bit

    bus out : Bit {
        when {
            case enable {
                it #= data
            }
            // No else on purpose: when `enable` is low, `out` is not driven.
        }
    }
}

mod UsesIt {
    net  line : Bit          // local shared signal
    // `line` is its own net; `t.out` joins it when connected.
}
```

* `bus name : Type` — third port kind. May be driven by the moddef (`out #= x`)
  *and* by any parent (`it.out #= x`), and read by both (through the crossing
  primitives, or as the source of another `#=`). Same declaration at top level
  and inside any moddef; there is no "parent-side spelling".
* `net name : Type` — local shared signal, visible only within its moddef
  (like `wire`). Multiply-drivable, may float.
* `Type` may be any type (`Bit`, `Word[N]`, structs, enums, unions).
* Grammar-level restrictions (parse errors, no new diagnostics needed):
  * No `on <clock>` clause on `bus`/`net` (shared signals are combinational).
  * No `default <EXPR>` clause (that mechanism is for platform-synthesized
    `outgoing` ports).

### Driving

* `target #= expr` — the shared-driver operator. Continuous only; there is no
  latched `#<=`. Targets must be shared: a `bus`, a `net`, or an instance's
  `bus` port (`it.b`). Multiple `#=` drivers on the same shared signal are
  legal — including drivers in separate `when`/`match` blocks.
* `target := expr` / `target <= expr` — unchanged, owned targets only
  (`incoming` from parent, `outgoing`, `wire`, `reg`).
* **Weak drives:** `b #= pullup()` and `b #= pulldown()` weakly drive the
  shared `Bit` net toward `true` / `false`. The zero-arg form is legal only as
  the RHS of a `#=` whose target is a shared `Bit`. Guarded weak drives (inside
  `when`/`match` arms) are allowed.
* `pullup(v)` / `pulldown(v)` — the **read-with-default crossing**: an
  expression of type `Bit` usable anywhere an expression is. `v` must be a
  bare reference to a shared `Bit` signal. Floating resolves to the pull value
  (`pullup` → `true`, `pulldown` → `false`); contention aborts the simulation.

These are the same primitive in two arities: `pullup()` contributes a weak
drive; `pullup(v)` reads the net `v` resolving floating to `true`. Both parse
as `ExprFn` (the existing `Ofness "(" args ")"` rule) and are resolved as
builtins by name+arity — no new keywords.

### Joining nets

`#= with a bare reference to a shared signal as RHS` **joins** the two nets
into one equivalence class (not a value copy):

```virdant
net  a : Word[8]
net  b : Word[8]
a #= b           // a and b are now one net: drivers of either drive both
```

Joins chain and merge transitively (`a #= b`, `b #= c` → one class of three).
A join's two sides must have equal types (checked like any other type
equality). Since only shared signals may be RHS sources of `#=`, every `#=`
RHS is either a join target (shared ref) or a plain expression (driver
contribution on owned signals only — those RHSes can't reference shared
signals except through the pull crossing).

Parent connection of a child's `bus` port is an ordinary named connection
written with `#=`:

```virdant
mod Top {
    net   line : Word[8]
    incoming sel : Bit
    t := Tristate
    t.out #= line        // join: t.out and `line` are one net
    line  #= sel         // parent drives the joined net
    w := pullup(t.out)   // parent reads it (floating → true)
}
```

### Reading (fencing)

Shared → owned is the dangerous edge. In this plan there are exactly two
crossings (v2's table, unchanged):

| Primitive       | Type restriction | Floating resolves to | Contended  |
|-----------------|------------------|----------------------|------------|
| `pullup(v)`     | `Bit` only       | `true`               | sim abort  |
| `pulldown(v)`   | `Bit` only       | `false`              | sim abort  |

Every other read of a shared signal is a compile error. Concretely, a reference
to a shared signal is legal only:

1. As the RHS of a `#=` driver (where a bare shared ref is a *join*, not a read).
2. As the argument of `pullup(...)` / `pulldown(...)`.

It is illegal as a binop operand, a `when`/`match` condition or subject, a fn
argument (other than the pull crossing), a match arm value, a struct-field
assignment, an `:=`/`<=` RHS, etc. `unsafe_read` stays deferred (Future Work),
so `Word[N]`-typed shared nets cannot be sampled into owned logic in this plan.

### Absence-as-Z

When driving a shared signal with `#=`, `when` blocks may omit `else` and
`match` blocks may be incomplete: the missing case drives nothing. Lowering to
Verilog emits `1'bz` at the proper width; the simulator models those bits as
"not driving". `when`/`match` *expressions* are untouched (complete coverage
still required — `MatchNotExhaustive` and friends unchanged).

Note on current behavior: driver-block `when`/`match` statements *already*
permit missing `else` for owned targets (`Driver::When { else_clause: None }`
lowers to `XLit` in combinational contexts). v3 changes the lowering only for
**shared** targets (XLit → ZLit); owned-target behavior is deliberately
unchanged so this plan does not disturb existing designs.

### Contention, initialization, per-bit analysis

Carried from v2 verbatim (items 1, 7, 9, 10, 11): contention is always a
simulation-time error (abort); owned signals initialize arbitrarily
(`reg`s pre-reset, top-level `incoming`), shared signals initialize floating;
contention and floating are analyzed per bit.

### Sockets

`socketdef` slots may be `bus name : Type` alongside `cosi`/`soci`. A bus slot
has no direction: both the client and server instantiations get a shared
`Bit`/`T` member. Bulk `socket1 :=: socket2` connect **joins** bus members into
one net (rather than direction-mapping cosi/soci members); each side may then
`#=`-drive and pull-read the member.

---

## Net Identity Model (new, v3)

Virdant's owned world is a DAG of single-driver signals. The shared world needs
an explicit notion of *net*:

* **Net** = equivalence class of shared signals, computed at elaboration by
  union-find over join edges. Every shared signal belongs to exactly one net;
  joins, parent connections, socket bulk-connects, and `bus`-from-`bus` drives
  all add union edges.
* **Drivers** attach to signals (and therefore to nets): each `#=` driver is a
  strong contribution; each `pullup()`/`pulldown()` statement is a weak
  contribution; each driver may be *partial* (absence → no contribution under
  some conditions).
* **Resolution** happens per net, per bit:
  * All strong contributions for a bit agree → that bit is driven with the
    agreeing value.
  * Strong contributions disagree on a bit → **contention** on that bit.
  * No strong contributions → identical weak contributions collapse to one
    (redundancy); conflicting weak contributions → contention on that bit.
  * No contributions at all → the bit is **floating**.
  * Weak contributions are ignored on bits that have any strong contribution.
* Any contended bit anywhere is a short: the simulation aborts. There are no
  multi-driver warnings at compile time (v2 item 13 stands): static heuristics
  cannot prove contention, and multiple `#=` drivers are legal by design.

Because expressions can never *read* a shared net (fencing above), shared nets
never appear inside ordinary expression evaluation — the only consumers of a
resolved net are pull-crossing expressions and the VCD writer. This keeps the
simulator's owned-value pipeline (`sim/eval.rs`, `Value`-based) intact.

---

## Implementation Plan

### 1. Grammar and AST

Files: `virdant/src/syntax/virdant.lalrpop`, `virdant/src/syntax/token.rs`,
`virdant/src/syntax/payload.rs`, `virdant/src/common.rs`.

1.1 **Keywords** (keyword map at `virdant.lalrpop:1161` and `token.rs`):
`"bus" => Token::KwBus`, `"net" => Token::KwNet`.

1.2 **Operator** `#=`: new token `OpSharedDriver`. The lexer must match `#=`
before the `#<ident>` pattern-prefix rule (cf. `#Red` / `@Ctor` handling in
`token.rs`), and it must not conflict with `:=` / `:=:` / `<=`.

1.3 **Component declarations** — add two `ModDefStmtComponent` alternatives
following the `wire`/`reg` shapes (no `DefaultClause`, no `OnClause`):

```lalrpop
<docstring:DocString?> <annotations:Annotations> <ll:@L> "bus" <ident:Ident> ":" <typ:Type> <it:ItBlock?> <rr:@R>
    => kind: ComponentKind::Bus,
<docstring:DocString?> <annotations:Annotations> <ll:@L> "net" <ident:Ident> ":" <typ:Type> <it:ItBlock?> <rr:@R>
    => kind: ComponentKind::Net,
```

(An `ItBlock` is permitted so a bus/net can carry its own driver block — `bus
out : Bit { when { ... } }`. Child-count bookkeeping must follow the existing
`1 + 1 + optional-it` pattern exactly; see the `wire` alternative for the
template.)

1.4 **Shared driver statement** — extend (do not duplicate) `ModDefStmtDriver`:

```lalrpop
<ll:@L> <lhs:Path> "#=" <rhs:Expr> <rr:@R> => {
    AstNodePayload::Driver(payload::Driver { driver_type: DriverType::Continuous, shared: true })
}
```

`payload::Driver` gains a `shared: bool` field (default `false` for `:=`/`<=`).
Adding a field to an existing payload is preferred over a new payload variant:
every consumer already matches on `AstNodePayload::Driver`, and the shared bit
threads through `collect_block_drivers` naturally. There is deliberately no
`#<=` production.

1.5 **Weak drive / pull read** — both are `ExprFn` (the existing `Ofness "("
ArgList? ")"` rule), resolved later as builtins by name and arity:

* `pullup()` / `pulldown()` — zero args, drive form.
* `pullup(v)` / `pulldown(v)` — one arg, read form.

No grammar change at all. The zero-arg form parses today as an `ExprFn` with
empty `ArgList` (`<ofness:Ofness> "(" <args:ArgList?> ")"` already accepts zero
args). Do **not** spell the weak drive as `pullup(v)` in statement position: a
statement-vs-expression ambiguity (`ArmBlock` vs `ArmBlockExpr`, both starting
with an `Ident`-led `ExprFn`) would create a LALR reduce/reduce conflict; the
`v #= pullup()` driver form avoids it entirely.

1.6 **Socketdef bus slots** — add to `SocketDefStmt`:

```lalrpop
<docstring:DocString?> <ll:@L> "bus" <name:Ident> ":" Type <rr:@R>
    => AstNodePayload::Channel(payload::Channel { name, dir: ChannelDir::Bus, doc_string })
```

`ChannelDir` (in `common.rs`) gains a `Bus` variant. The `Channel` payload
already carries `dir`, so no payload change is needed.

1.7 After editing the grammar: `make grammar` to regenerate `GRAMMAR.txt`.

### 2. Common enums (`virdant/src/common.rs`)

* `ComponentKind` gains `Bus` and `Net`.
* `ChannelDir` gains `Bus`.
* `PortDir` (used by `analysis/ports.rs` and the Verilog backend) gains
  `Bidir`.
* `DriverType` stays `{ Continuous, Latched }` — shared-ness is carried by
  `payload::Driver.shared`, not by driver type. A future `#<=` would need its
  own decision; not in this plan.

### 3. Component analysis (`virdant/src/analysis/component.rs`)

3.1 **Flow mapping** (the two `match component.kind` blocks at ~286 and ~377):

* Moddef-local: `Bus => Flow::Duplex`, `Net => Flow::Duplex`.
* Instance scope: a child's `bus` port keeps `Flow::Duplex` and kind `Bus` —
  it does **not** flip like `Incoming`↔`Outgoing`, because neither side owns
  it. This asymmetry is the whole point of the single keyword.
* Submodule port registration (~330): extend the kind filter
  (`Incoming | Outgoing | OutgoingWire | OutgoingReg`) with `Bus` so `bus`
  ports are registered as `instance.port` components.
* Socket channels (~430): for `ChannelDir::Bus`, both roles get
  `Flow::Duplex` + `ComponentKind::Bus` (extend `drives_channel` or handle
  `Bus` before it — cosi/soci logic does not apply).

3.2 `can_sink()` / `can_source()` already return `true` for `Flow::Duplex`, so
a `Bus`/`Net` component can be both driven and read at the flow level; the
shared-read fencing lives in typing (§6), and the driver-count checks are
relaxed in §5.

3.3 Platform synthesis (`for` clause): platforms declare only
`incoming`/`outgoing` ports in this plan. A `bus`/`net` declaration in a
platform-bound moddef is untouched by synthesis (platforms never have `bus`
ports — see §9), so the synthetic-port code needs no changes beyond not
crashing on the new kinds.

### 4. Ports list (`virdant/src/analysis/ports.rs`)

`build_ports_of`: `ComponentKind::Bus` is a port. Map `Bus => PortDir::Bidir`
(add a `Bidir` arm to the `dir` match; note the existing code's comment that
`OutgoingWire/OutgoingReg` are `Duplex`-internally but `Output` externally —
`Bus` is `Duplex` both internally and externally). `Net` is not a port (like
`Wire`/`Reg`). Any consumer of `PortDir` that assumes two values (e.g. Verilog
emission, VCD direction columns, LSP hover) needs a `Bidir` arm — grep for
`PortDir::` across the tree when implementing.

### 5. Driver rules (`virdant/src/analysis/drivers.rs`, `virdant/src/queries/check_drivers.rs`)

5.1 `collect_block_drivers` threads the existing machinery unchanged:
`Driver::When`/`Driver::Match` with `else_clause: None` already model absence;
`Driver::Absent` already fills clause positions where a component is not
driven. The only change: `Driver::Expr` produced from a `shared: true` driver
statement must record shared-ness (add `shared: bool` to `Driver::Expr`, or a
`Driver::Shared{...}` variant — prefer extending `Expr` with a flag for the
same reason as §1.4).

5.2 **Weak drivers**: `pullup()`/`pulldown()` zero-arg `ExprFn`s in `#=`
position are collected as ordinary `Driver::Expr`s; strength is derived later
(§8) from the expression's resolved builtin tag (`Primitive::Pull`), not at
collection time.

5.3 `check_drivers.rs` relaxations (shared targets only — keyed off the
component's kind being `Bus`/`Net`):

* Skip `NoDrivers`/`NoRegDrivers` — zero drivers is a legal floating net.
* Skip `MultipleDrivers` — multiplicity is legal by design (v2 item 13).
* Keep `WrongDriverType` semantics: shared drivers are always `Continuous`;
  a (nonexistent) latched shared driver would flag. `DriverForSink` keeps
  working for free (`Duplex` can_sink).
* `check_wrong_driver_types`: `ComponentKind::Bus`/`Net` expect
  `DriverType::Continuous`.

5.4 Also in `check_drivers.rs` (or a sibling query — §7): the operator/kind
cross-checks that typing alone cannot express:

* `#= (shared) targeting a component whose kind is not `Bus`/`Net` →
  `SharedDriverOnOwned`.
* `:=` / `<=` targeting `Bus`/`Net` → `OwnedDriverOnShared`.

### 6. Typing and fencing (`virdant/src/types/typing/check.rs`, `infer.rs`, `typing.rs`)

6.1 **Pull builtins.** Add `Primitive::Pull` (or two variants `Pullup`,
`Pulldown`) to the `Primitive` enum in `typing.rs`. Resolve them where the
other builtins are matched by name (`b"mux"`, `b"any"`, `b"all"`, `b"word"`,
`b"sext"`, `b"as_clock"` in `check.rs`/`infer.rs`):

* `pullup(v)` / `pulldown(v)` (1 arg): type is `Bit`; the argument must be an
  `ExprReference` to a component with kind `Bus` or `Net` (instance bus ports
  included) whose type is `Bit`. Otherwise `PullNotOnShared`.
* `pullup()` / `pulldown()` (0 args): type is `Bit`; legal only as the direct
  RHS of a `#=`. The typing walker knows it is in that position (see 6.2);
  elsewhere → `PullDriveOutsideSharedDriver`.
* The zero-arg call must type as `Bit` without evaluating anything — it is a
  marker, not a value-producing expression (§8).

6.2 **Shared-read fencing.** The expression checker needs a notion of the
current context. The `Typing` walker (`check.rs`) already tracks
`scheduler.in_sequential`-style position state in the Verilog backend; add an
equivalent `read_shared_ok: bool` (or a small enum `ExprCtx::Owned |
ExprCtx::SharedDriverRhs | ExprCtx::PullArg`) threaded through expression
checking:

* A component reference resolves via `TypingContext`/`Referent` to a
  component; if the component's kind is `Bus`/`Net`:
  * `SharedDriverRhs` position → legal (this is a join or a driver source).
  * `PullArg` position (argument of a 1-arg pull builtin) → legal (crossing).
  * Any other position → `UnsafeSharedRead`.
* Instance bus ports (`it.b`) resolve the same way — the kind travels with the
  component through instance registration (§3.1), so `it.b` is fenced
  identically to a local `net`.

6.3 **Join type equality.** A `#= whose RHS is a bare shared reference` must
type-check its target and RHS types for equality (`WrongType` otherwise). This
is ordinary typechecking of the driver statement — the driver RHS is typed as
an expression regardless of join-ness.

6.4 `match` subjects/conditions referencing shared signals are caught by 6.2
with no special casing. `it` inside a bus's own driver block resolves to the
bus component (kind `Bus`) — the existing `NotIt`-style machinery needs no
change.

### 7. New/changed diagnostics (`virdant/src/diagnostics.rs`)

New payloads (message wording to be finalized during implementation; shown
here as the catalog):

| Payload                  | Fields                       | Fires when                                                            |
|--------------------------|------------------------------|-----------------------------------------------------------------------|
| `SharedDriverOnOwned`    | `target`, `op`               | `#=` targets an owned component                                        |
| `OwnedDriverOnShared`    | `target`, `op`               | `:=`/`<=` targets a `bus`/`net`                                        |
| `UnsafeSharedRead`       | `path`                       | shared signal referenced outside the two legal contexts (§6.2)          |
| `PullNotOnShared`        | `primitive`, `arg`           | `pullup`/`pulldown` argument is not a bare shared `Bit` reference       |
| `PullDriveOutsideSharedDriver` | `primitive`            | zero-arg `pullup()`/`pulldown()` outside a `#=` RHS                     |
| `JoinTypeMismatch`       | `lhs`, `rhs`                 | join sides have different types (may reuse `WrongType`)                 |
| `PlatformBusUnsupported` | `port`                       | a `bus` port appears in a design bound to a platform (§9, item 14)      |

Existing payloads deliberately **not** emitted for shared signals:
`NoDrivers`, `NoRegDrivers`, `MultipleDrivers` (§5.3).

### 8. Simulator (`virdant/src/sim/*`)

8.1 **Elaboration changes** (`analysis/elaboration.rs`):

* `ElaboratedComponent` currently carries one `driver: Option<Driver>`.
  Replace with `drivers: Vec<Driver>` (owned: length ≤ 1; shared: any count,
  union of the moddef's own drivers and the parent's — the `ParentCtx` lookup
  for `Incoming` ports generalizes: for a `Bus` port, append the drivers the
  parent recorded for `"{instance}.{port}"`). Keep `resolve_alias`-style
  bare-ref detection but generalize it (8.2).
* **Net classes**: add `net_of: IndexMap<SignalId, SignalId>` (union-find
  representative) to `Elaboration`. Build it from every shared-driver
  statement whose RHS is a bare `ExprReference` to a shared signal — those are
  joins, not drivers: the RHS edge is removed from the driver list and unioned
  instead. Parent connections (`it.b #= n`) and socket bulk connects union the
  child port, the parent net, and (for sockets) the counterpart member.
* Scope rule for join resolution: a driver written in the parent targeting
  `it.b` with RHS `n` resolves `n` in the parent's scope; a driver inside a
  moddef targeting its own `bus` with RHS `m` resolves `m` in that moddef's
  scope. The existing `resolve_alias` prefix-stripping logic
  (`Incoming` → drop two segments; others → drop one) is the template; `Bus`
  follows the "driven from parent" rule when the driver's owning scope is the
  parent, and the "in-module" rule otherwise — resolution is per-driver by the
  module that wrote the statement, which is recoverable from the driver's
  `Location`.

8.2 **Circuit** (`sim/circuit.rs`):

* `exprs: IndexMap<SignalId, Arc<Expr>>` becomes
  `drivers: IndexMap<SignalId, Vec<(Strength, Arc<Expr>)>>` where
  `Strength ∈ { Strong, Weak }`. Strength: an expression tagged
  `Primitive::Pull` (zero-arg form) is `Weak`; everything else `Strong`.
  Owned signals keep exactly one `Strong` entry.
* `driver_to_expr` (`sim/expr.rs`) gains:
  * `Expr::NoDrive` — a driver whose value is "absent" under the current
    conditions (produced from `Driver::Absent` / missing-else arms), evaluated
    to `None`.
  * `Expr::PullDrive(bool)` — the zero-arg pull marker.
  * `Expr::PullRead { net: SignalId, up: bool }` — the 1-arg pull read.
* `resolved_referents` stays per-driver (each driver entry gets its own).
* Deps/sensitivities: a `PullRead { net }` expression depends on *every
  driver* of `net`'s class — add edges from each driver's owning signal to the
  pull reader. (Joining nets transitively merges their driver sets, so readers
  see the full class.)

8.3 **Net resolution and state** (`sim/state.rs`, `sim/sim.rs`):

* `State` gains `net_vals: IndexMap<SignalId, NetState>` keyed by net
  representative, where:

  ```rust
  enum NetState {
      Floating,                    // no contributions at all
      Driven(Value),               // all driven bits agree; value is clean
      Contended { first_bit: u16 } // at least one bit has 2+ disagreeing strongs
                                   // or conflicting weaks
  }
  ```

  (Per-bit detail lives inside resolution; `NetState` is the *result*. A
  partially-driven multi-bit net is representable only when every driven bit
  agrees and every undriven bit is floating — since reads are fenced to
  `Bit`-only pulls in this plan, partial multi-bit resolution only matters via
  the eventual `unsafe_read` (Future Work), which will extend `NetState` with
  a per-bit layout. Note this explicitly so the type has room to grow.)
* `Value::Z(Type)` already exists in `sim/eval.rs` — fully-floating owned
  projections keep using it; shared nets use `NetState::Floating`.
* **Resolution pass**: for each net, evaluate every driver contribution
  (`Option<Value>`; `Expr::NoDrive` → `None`), then apply the per-bit rules
  from §Net Identity. Cache per-driver contributions
  (`IndexMap<(SignalId, usize), Option<Value>>`) so a change to one driver
  re-evaluates only that driver plus the net's pull readers.
* **Contention → abort**: resolution finding `Contended` returns an error from
  `flow()`. `SimError` (currently `pub enum SimError {}` — empty) gains
  `Contention { net: BString, location: Option<Location> }`; `Sim::run`
  propagates it. The CLI (`bin/vir.rs`) reports it as a simulation failure.
* **Initialization** (`Sim::new` / `apply_initials`): shared nets initialize
  `Floating`; owned signals initialize to an *arbitrary* value instead of
  `Value::X` — add an init policy to `Sim::new`
  (`InitPolicy::Zero` default for deterministic tests, `InitPolicy::Random(u64)`
  for seeded random), replacing today's blanket `Value::X` fill in
  `State::new`. (`reg`s pre-reset and top-level `incoming` are the only places
  this is observable; everything else traces back to those.)
* **Weak/strong interplay**: weak contributions are dropped on bits with any
  strong contribution; identical weaks collapse; conflicting weaks →
  contention.
* **Pull read evaluation**: `Expr::PullRead { net, up }` evaluates by looking
  up `net`'s `NetState`: `Floating` → `Bit(up)`; `Driven(Bit(b))` → `Bit(b)`;
  `Contended` → sim panic/abort (per v2 item 8 — undefined behavior,
  recommended to panic).

8.4 **VCD** (`sim/vcd.rs`): dump shared nets with their `NetState` (z / 0 / 1 /
x per bit). VCD direction for `bus` ports: `inout` (the VCD `$var` direction
field gains `<>`... verify what `vcd.rs` currently emits for directions and add
`Bidir` there too).

### 9. Verilog backend (`virdant/src/verilog/conversion.rs`, `verilog.rs`, `stmt.rs`, `expr.rs`)

9.1 **Ports**: `PortDir::Bidir` → `inout` in the module header (`verilog::Port`
gains no new fields; the direction rendering adds `Bidir => "inout"`). Bus
ports are always `PortKind::Wire` (never `Reg`).

9.2 **Declarations**: `ComponentKind::Net` emits an ordinary `Wire` element
(identical to `Wire` today). `Bus` ports need no internal declaration beyond
the port header.

9.3 **Shared drivers**: each `#= driver` lowers to `assign net = <expr>;` where
the expression is the existing ternary-chain lowering with one change: the
`else`/missing-arm default becomes `ZLit { width }` (new `verilog::Expr`
variant, e.g. `1'bz` / `8'bzz...`) instead of `XLit`. The lowering needs the
target's kind (`Bus`/`Net` → ZLit default; everything else keeps `XLit`), so
`convert_driver_when_to_expr` / `convert_driver_match_to_expr` take the target
kind. `Driver::Absent` likewise lowers to `ZLit` for shared targets, `XLit`
for owned (unchanged).

Multiple `assign`s to the same Verilog net model tristate resolution natively:
agreeing drivers resolve; conflicting ones produce `x` in the downstream
simulator — matching Virdant's "contention is an error" as closely as Verilog
can.

9.4 **Weak drives**: an unguarded `b #= pullup()` / `b #= pulldown()` at
moddef top level lowers to the Verilog pull primitives: `pullup (b);` /
`pulldown (b);` (new `verilog::Element::Pull { name, up }`). **Guarded weak
drives** (inside `when`/`match` arms) have no faithful Verilog primitive
(Verilog pulls are static). Lower them as a strong conditional drive with
release: `assign b = <guard> ? 1'bz : 1'b1;` — a documented approximation
(strong rather than weak). The simulator implements guarded weak drives
exactly; the Verilog gap is noted in this plan and accepted for v3 (an
alternative — diagnosing guarded pulls in the backend — is listed as an
open follow-up, not a blocker).

9.5 **Joins**:

* Parent connect of a child bus port: `it.b #= n` (bare-ref RHS) lowers by
  connecting the instance port directly to the parent net — i.e. the
  instantiation emits `.b(n)` instead of `.b(inst_b_wire)`, and no `assign`
  is emitted. (The existing instance-port machinery declares a
  `Wire` per `instance.port`; for `Bus` ports joined to a parent net, skip the
  temp wire and wire the port straight to the join target. If the parent also
  *drives* the same bus with an expression (`it.b #= data`), emit
  `assign inst_b_wire = data;` on the declared wire and connect `.b(inst_b_wire)`
  — multiple parents-side contributions on one net are ordinary tristate
  assigns.)
* Internal joins (`a #= b` between two local shared signals): lower to
  `tran (a, b);` (new `verilog::Element::Tran { a, b }`). `tran` is a
  bidirectional primitive supported by iverilog; a plain `assign a = b;` would
  be a *copy*, not a join (drivers on `a` would not reach `b`).
* Socket bulk connect of `bus` slots: the `collect_bidirectional_assigns`
  pass currently emits per-member `assign lhs.member = rhs.member;`. For
  `ChannelDir::Bus` members emit `tran (lhs.member, rhs.member);` instead —
  a join, not a direction-mapped copy.

9.6 **Pull read**: `pullup(v)` in an expression lowers to `v`'s Verilog name
directly — in Verilog, reading a tristate net yields z when floating, and the
`pullup(v)`-default-for-floating behavior is provided by the emitted
`pullup (v);` primitive in the same module. The pull primitive must be emitted
in every module that contains a pull read or pull drive of that net. (This is
why §9.5's joins use `tran`: it keeps the pull semantics well-defined across
the join.)

### 10. Sockets (`analysis/component.rs`, `queries/check.rs`, elaboration, conversion)

* `ChannelDir::Bus` channels: both `client socket` and `server socket`
  instantiations register the channel as a `Flow::Duplex`, `ComponentKind::Bus`
  component (§3.1) — no `drives_channel` role logic applies.
* Bulk `:=:` connect (`check_socket_connections`): for bus members, verify the
  two sides' member types match, then register a join edge (§8.1) instead of a
  driver. No new socketdef-generic machinery is needed — bus slots
  monomorphize exactly like cosi/soci channels.
* Reading/driving a socket bus member: `it.sock.member` — same fencing rules
  as any shared signal (§6.2).

### 11. Platform policy (item 14, deferred)

Platforms (`lib/ice40.vir`, `lib/virt.vir`, `lib/xilinx.vir`, `lib/builtin.vir`)
gain no bus support in this plan. A design bound to a platform whose top module
has `bus` ports gets `PlatformBusUnsupported`. The platform/IOBUF/backend story
(top-level tristate pins, IOBUF inference, internal shared-net mux-tree
conversion) is a separate future plan.

### 12. Ancillary surfaces

* **virdoc** (`tests/virdoc`, doc extraction): `//>` docstrings on `bus`/`net`
  declarations flow through the existing `Component` payload (`doc_string`) —
  no changes expected; one sanity test (below).
* **`vir format`** (`bin/format.rs`, `tests/format`): the formatter must
  indent and align `bus`/`net` declarations, `#=` drivers, and pull
  statements. New golden tests below.
* **LSP** (`bin/virlsp.rs`): hover/completion over `bus`/`net` components
  should work automatically via component analysis; verify `PortDir::Bidir`
  doesn't break direction rendering in hover text.
* **Docs** (`docs/source/reference/*.rst`): add `bus`/`net`/`#=` to the
  modules/statements/expressions reference pages (follow-up, not blocking
  `make test`).

---

## Test Plan

All FileCheck-style tests follow the existing conventions:
`tests/errors/` must fail `vir -F f.vir check` with the listed `CHECK:` strings;
`tests/warnings/` must pass with the listed warnings;
`tests/pass/` must pass `check` cleanly (`CHECK-NOT: Check failed`).
Simulator tests are project-style directories (Virdant.toml + Makefile +
testbench) exercising the generated Verilog under iverilog, matching the
`tests/pass/fifo` / `tests/pass/gcd` layout. All simulator-side semantics are
tested through the iverilog projects and compile-time fixtures — this plan
deliberately contains no Rust unit tests.

### tests/errors/ — compile-time failures

| File | Scenario | Expected diagnostics |
|------|----------|----------------------|
| `bus_shared_driver_on_owned.vir` | `out #= x` on `outgoing`, `wire`, `reg`, inside an it-block | `SharedDriverOnOwned` per site |
| `bus_owned_driver_on_shared.vir` | `:=`/`<=` on `net`, `bus`, and inside an it-block | `OwnedDriverOnShared` per site |
| `bus_unsafe_shared_read.vir` | reads of shared signals in every illegal context: binop, when-condition, match subject/arm, match-arm body, direct `:=` RHS, struct-field projection, own bus port | `UnsafeSharedRead` per site |
| `bus_word_pull_attempt.vir` | exhaustive negative: reading a `Word[N]` shared net into owned logic by any route (reg driver, bus port read) | `UnsafeSharedRead` per route |
| `bus_pull_args.vir` | pullup/pulldown restrictions: read form on owned signal, on shared `Word`, zero-arg drive form outside `#=` | `PullNotOnShared` / `PullDriveOutsideSharedDriver` per site |
| `bus_join_type_mismatch.vir` | join sides of different types: `net`/`net` and parent/child (`c.b : Word[8]` to `n : Bit`) | `JoinTypeMismatch` / `WrongType` per site |
| `bus_syntax_rejects.vir` | `on <clock>` and `default` clauses on `bus`/`net` declarations | syntax error (no such productions) |
| `bus_socket_member_owned_connect.vir` | `ch.line := x` on a socket `bus` slot | `OwnedDriverOnShared` |
| `bus_parent_connect_errors.vir` | parent-side errors on a child bus: `t.out := w`, `w := t.out` (read), `w #= t.out` (join into owned) | `OwnedDriverOnShared` / `UnsafeSharedRead` / `SharedDriverOnOwned` |
| `bus_platform_unsupported/` (project) | platform-bound top module with a `bus` port, wired into `tests/errors/Makefile`'s `VIR_PROJS` | platform extra-port error on `bus line` |

### tests/warnings/ — passes with warnings

(Kept minimal by design — no multi-driver warnings exist, item 13.)

| File | Scenario | Expected warnings |
|------|----------|-------------------|
| `bus_unused.vir` | driven-but-unread and completely undriven `net`/`bus` | no unused-signal warnings (shared signals are exempt from `UnusedSource`), no `NoDrivers` |
| `bus_empty_driver_block.vir` | `bus b : Bit { }` / `net n : Bit { }` with empty it-blocks | `Empty driver block` (matching `tests/warnings/empty_driver_block.vir`) |

### tests/pass/ — compile-clean fixtures

| File | Scenario |
|------|----------|
| `bus_basic.vir` | moddef with `bus` port: total `#=` drive, partial `when` (no else), partial `match` (missing cases), `net` join, pull read |
| `bus_partial_when.vir` | shared driver block with omitted `else`, multi-clause `when`, `Driver::Absent` interplay |
| `bus_partial_match.vir` | incomplete `match` over enum subject in a `#=` block |
| `bus_join_chain.vir` | `a #= b`, `b #= c` — transitively one net; each side drives |
| `bus_parent_join.vir` | parent joins child `bus` to parent `net` (`it.b #= n`); child and parent both drive |
| `bus_parent_drive.vir` | parent drives child bus with an expression (`it.b #= data`) alongside child drives |
| `bus_multiple_drivers.vir` | many `#=` drivers on one net, complementarily guarded (must compile with **no** warnings — proves item 13) |
| `bus_pull_read.vir` | `w := pullup(b)` and `w := pulldown(b)` in expressions, conditions, and match guards |
| `bus_pull_drive.vir` | `b #= pullup()` / `b #= pulldown()`, unguarded and guarded, on `net` and `bus` |
| `bus_redundant_pulls.vir` | two identical `pullup()` drives on one net (must compile clean; sim treats as one) |
| `bus_bit_only.vir` | `Bit`-typed nets and buses throughout (the only pull-able type) |
| `bus_word.vir` | `Word[8]` shared nets driven/joined/partially driven (compile only — no reads) |
| `bus_socket_slot.vir` | socketdef with `bus` slot; client and server sides drive and pull-read; bulk `:=:` connect |
| `bus_socket_slot_child.vir` | socket bus slot through a nested submodule instance (`leaf.ch :=: ch`) |
| `bus_extmod_untouched.vir` | ext moddefs and non-bus designs still compile (regression guard) |
| `bus_generic.vir` | generic moddef with `bus` port parameterized by type argument (monomorphization interplay) |
| `bus_virdoc.vir` | `//>` docstrings on `bus`/`net` declarations (pairs with the tests/virdoc extraction, below) |

The project-style simulator directories below follow the existing convention
(`fifo`, `extmod`, `sockets`): they carry their own `Makefile` running `$(VIR)
build` + `iverilog` and are **not** wired into `tests/pass/Makefile`'s
`VIR_PROJS` — run them directly with `make -C tests/pass/<dir> VIR=...` |

### Project-style simulator tests (iverilog on generated Verilog)

Directory layout like `tests/pass/fifo/` (Virdant.toml, Makefile running
`$(VIR) build` + `iverilog build/*.sv tb.sv`, testbench asserting outcomes).

| Directory | Scenario / testbench assertions |
|-----------|--------------------------------|
| `tests/pass/bus_tristate_sim` | `Tristate` buffer: enable high drives the bus; enable low releases it; two complementary tristate buffers mux a value through one shared bus; testbench reads resolved values. |
| `tests/pass/bus_floating_sim` | bus with no drivers at all floats (z); Verilog-side `pullup` primitive resolves it to 1; `pulldown` to 0. |
| `tests/pass/bus_join_sim` | parent net joined to child bus (port connection); child drives and the parent reads; then the parent drives and the child's pull read sees the value — proves bidirectional join. |
| `tests/pass/bus_socket_sim` | socketdef `bus` slot bulk-connected between client/server; both sides drive in turns (open-drain style), testbench asserts the resolved value at each phase. |
| `tests/pass/bus_i2c_style_sim` | open-drain pattern: two modules `#=`-driving a shared `Bit` net, each releasing (z) or pulling low; asserts wired-AND-like resolution and no contention in the driven phases. |
| `tests/pass/bus_pull_guards_sim` | guarded `#=` pullups under `when`; asserts the documented Verilog approximation (strong drive / release) behaves sanely in iverilog. |
| `tests/pass/bus_word_sim` | `Word[8]` shared net with two complementary drivers; asserts full-word resolution and the all-z floating phase on the export module's own bus port. |

### tests/format/ — formatter goldens

| Pair | Covers |
|------|--------|
| `bus_declarations.vir` / `.golden.vir` | `bus`/`net` declarations aligned like `incoming`/`outgoing` columns |
| `bus_shared_drivers.vir` / `.golden.vir` | `#=` alignment (analogous to existing `driver_align` golden), inside and outside `when` blocks |
| `bus_pulls.vir` / `.golden.vir` | `b #= pullup()` statement lines and `pullup(b)` in expressions |

### tests/virdoc/ — doc extraction

| File | Covers |
|------|--------|
| `tests/virdoc/src/top.vir` (extended in place) with `bus line` / `net tap` docstrings, and Makefile greps added | `//>` docstrings on `bus`/`net` declarations appear in the generated module page |

---

## Future Work

* **`unsafe_read(v)`** — the general shared → owned crossing (any type, no
  default), with simulator panic on floating reads and undefined behavior on
  contended reads. Requires the per-bit `NetState` layout sketched in §8.3.
* **Platform `bus` ports / IOBUF / internal tristate** (v2 item 14) — backend
  plan.
* Guarded weak-drive Verilog lowering beyond the documented approximation.
* ext-moddef `bus` ports (see the error fixture above; decide when a use case
  appears).

## Status

All v2 items carried forward; the v2 "outgoing bus / incoming bus" naming
decision is **overturned** — there is one `bus` kind, declared `bus name :
Type`, exactly as `CONCEPT.md` specifies. New decisions made in v3 (subject to
review): weak drives are spelled `b #= pullup()` / `b #= pulldown()`; joins
are `#= with a bare shared reference as RHS` (lowered with `tran` in Verilog);
guarded weak drives lower to a documented strong-drive approximation; sim
`InitPolicy` replaces blanket-X initialization. Ready for implementation.
