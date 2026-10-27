# Tristate and Bidirectional Ports (v4)

**v4 supersedes `PLANv3.md`.** Per workspace rules, the highest version suffix is
canonical. `CONCEPT.md` takes precedence over any plan.

**The charter for v4 is different from v1–v3: this plan mirrors `CONCEPT.md`
section-by-section and invents nothing.** v1 invented `inout`/`tri`/`unsafe_read`;
v2 invented `outgoing bus` mirror spellings and a `pullup(v)` read-crossing; v3
kept the pull-crossing, folded attachment into `#=`, restricted weak drives to
`Bit`, and promoted sockets from Future Work into core scope. Every one of those
divergences is reverted here. Where CONCEPT is silent, v4 marks the point as an
**Interpretation** or **Open Question** rather than silently legislating.

## Deltas from v3

| Topic               | PLAN v3                                        | PLAN v4 (this document)                              |
|---------------------|------------------------------------------------|------------------------------------------------------|
| Reading shared nets | `pullup(v)` / `pulldown(v)` read-with-default  | **`tap n`** — CONCEPT's unsafe assert-drivenness op    |
| Attachment          | `#= with a bare shared ref as RHS` (implicit join) | **`n #=# m`** — CONCEPT's dedicated attach operator  |
| Weak drives         | `b #= pullup()` / `b #= pulldown()`, `Bit` only | **`n #- e`** — CONCEPT's weak driver, any type       |
| Resolution          | per-bit strong/weak rules (v3 §Net Identity)   | CONCEPT's four resolution rules, verbatim (§Driving nets below) |
| Sockets `bus` slots | Core scope                                     | **Future Work** (CONCEPT's own placement)            |
| Sim rework          | Deferred ("§8 not implemented")                | **Still deferred** — language plan only; sim is a separate follow-up plan |
| Initialization      | `InitPolicy` replaces blanket-X (approved v2)  | Carried (consistent with CONCEPT's Initialization)   |

The implementation restarts from `master`; the `tristate` branch (v3's
implementation) is treated as a source of mechanical knowledge only (grammar
shapes, backend seams, test scaffolding), not as a design to preserve.

---

## 1. Bidirectional Nets

The `net` keyword acts like a Virdant `wire`, but can be multiply driven.
This mirrors Verilog's `net` keyword.

`net`, like `wire`, is a locally-visible signal, but otherwise acts identically
to `bus`.

```virdant
net line : Word[8]
```

* Locally visible within its moddef (like `wire`); not a port.
* May be driven any number of times (or not at all).
* No `on <clock>` clause, no `default` clause (parse errors — same rationale
  as v3: shared signals are combinational; `default` is platform machinery).

## 2. Bidirectional Ports

Virdant has `incoming` ports and `outgoing` ports. We add a `bus` port as a
third port kind:

```virdant
bus out : Bit
```

* One keyword, one declaration, no direction, no parent-side mirror spelling
  (the v3 decision, retained — it was correct and matches CONCEPT's syntax
  exactly).
* May be driven by the moddef (like an `outgoing` port) *and* by any
  instantiating parent (like an `incoming` port); both sides may also read it
  (through `tap`).
* Any type is allowed.
* An it-block driver block may be attached directly:
  `bus out : Bit { it #= ... }` (CONCEPT's own Tristate example does this).

**Driver multiplicity** (CONCEPT §Driver Multiplicity, quoted in substance):
the whole point of nets is that they may or may not be driven. For
`incoming`, `outgoing`, `wire`, and `reg` there is a unique driver (or one
block of drivers). For `net` and `bus` this need not be the case. The `#` in
`#=`, `#=#`, and `#-` ("number") is the visual mnemonic that there may be
multiple drivers.

A moddef may drive a `bus` just as with an `outgoing` port, and at the same
time any module which instantiates it may drive it as if it were an
`incoming` port. **Both parent and child may drive a `bus`.**

## 3. X and Z Are Not Values

Verilog uses a 4-value model (0, 1, X, Z). Virdant does **not** interpret X
and Z as values:

* **X** means *multiply driven* (contention) — the presence of multiple
  values driving the same net.
* **Z** means *undriven* (floating) — the absence of any value driving the
  net.

They are properties of a **net**, not values; they do not inhabit any type
and never appear in the type system.

Uninitialized data is **not** X: owned signals initialize to an arbitrary
value (see §7 Initialization).

For `incoming`, `outgoing`, `wire`, and `reg`, contention and floating are
impossible **by construction**; these always carry a single, definite value.

The only places floating can arise are `bus` and `net`. Because a shared
signal does not necessarily have a value, **it cannot be used directly as a
reference**. Reads of shared signals go through `tap` (§5).

## 4. Expressions Are Always Single-Valued

Expressions *always* generate a concrete value. No expression can ever
evaluate to Z or X. There is no way to "set a net to Z": to make a net float,
you must have zero active drivers (e.g. via a `when`/`match` statement arm
that drives nothing — §8).

This principle is why `tap` exists, and why every "Z as a value" proposal is
rejected (see Rejected Designs).

## 5. `tap` — Reading a Shared Signal

Because `bus` and `net` do not necessarily have a value, they cannot be used
directly as references. Instead the designer writes:

```virdant
wire b : Bit { it := tap line }
```

`tap n` (where `n` is a `net` or `bus` path) is an **unsafe operation**: its
use asserts that `n` will always have a value. It is the *only* way to read a
shared signal into an expression.

* Using `n` as a variable reference without `tap` is a compile error:

  > `n` is not guaranteed to have a value. Use `tap n` to assert it will.

* The designer is responsible for ensuring `n` actually has a single value
  (not floating) wherever `tap n` is evaluated. **Interpretation:** a `tap`
  of a floating net at simulation time is therefore an error (the designer's
  assertion was false), just as contention is — see Open Question 1.
* Contended nets abort the simulation regardless of how they are observed
  (§9).
* `tap` is a hardware no-op: it emits the plain read of the net. It is a
  compile-time marker of an assertion, not a value transformation.
* `tap` accepts any type: `tap n : T` for whatever type `n` is declared.
* There is no read-with-default, no pull-primarily read, no "safe read".
  CONCEPT defines exactly one crossing and v4 restores exactly one crossing.

**Fencing.** A path referring to a `net` or `bus` component is legal in
exactly these positions:

1. As the target of `#=`, `#=#` (either side), or `#-`.
2. As the operand of `tap`.
3. As the RHS of `#= *when the RHS expression otherwise typechecks* — note
   that even here the reference must still be spelled `tap n`, because
   expressions are always single-valued (§4). A bare shared reference is
   never a legal expression.

Everywhere else — binop operands, `when`/`match` conditions and subjects,
function arguments, `:=`/`<=` RHS, struct fields, slice subjects — a bare
shared reference is a compile error (`UnsafeSharedRead`).

## 6. Net Drivers

To drive a net there are three forms. Throughout, let `n` be a path to a
`net` or a `bus` and `e` be an arbitrary expression of the same type.

### 6.1 Strong driver — `n #= e`

Drives `n` with the value of `e`. Multiple `#=` drivers on the same shared
signal are legal — including drivers spread across separate `when`/`match`
blocks (that is the point; see §2, Driver multiplicity).

The `#=` operator exists to visually distinguish shared driving from owned
driving: when you see `n #= ...`, there **may** be other drivers active at
the same time (and for `bus`es, possibly from the other side of a module
boundary).

`#=` is continuous; there is no latched form (`#<=` does not exist).

`#=` targets shared signals only. `:=` and `<=` target owned signals only.
Crossing them is a compile error in both directions.

### 6.2 Attachment — `n #=# m`

Let `m` be another path to a `net` or a `bus` (intermixing the two is fine):

```virdant
n #=# m
```

This is a **bidirectional connect** operation (similar to Chisel's `attach`).
Electrically, it attaches the two nets with a wire: any value driven to `n`
is also driven to `m`, and vice versa. `#=#` declares net *identity* — the
two paths become one equivalence class — not a value copy.

* Both operands must be shared signals (`net`/`bus`, including an instance's
  `bus` port). Attaching an owned signal is a compile error.
* Types must match (ordinary type equality).
* Joins chain and merge transitively: `a #=# b`, `b #=# c` → one net of
  three.

**Drivers and submodules** (CONCEPT's two examples, both legal):

```virdant
// Joining a child's bus port with a parent net:
mod Foo {
    bus a : Bit
    mod bar of Bar
    bar.b #=# a
}
mod Bar {
    bus b : Bit
}
```

```virdant
// Driving a parent bus from a child's outgoing port:
mod Foo {
    bus a : Bit
    mod bar of Bar
    a #= bar.b
}
mod Bar {
    outgoing b : Bit
}
```

In the second example `bar.b` is an *owned* outgoing port — reading it is
legal without `tap` — so `a #= bar.b` is an ordinary strong driver
contribution from the child onto `a`'s net.

Driving a child's `bus` port from the parent is likewise ordinary:

```virdant
bar.b #= e          // parent contributes a strong driver to the child's bus
bar.b #=# parentnet // parent attaches the child's bus to its own net
```

### 6.3 Weak drivers — `n #- e`

`n #- e` **pulls** the value of `n` toward the value `e`.

* `n #- true` attaches a pull-up resistor to `n`.
* `n #- false` attaches a pull-down resistor to it.
* The `#-` operator looks like `#=` with "half" the force (the equals sign
  becomes a minus sign).
* **Weak drivers may be used for any type of value** — not just `Bit`. A
  `Word[8]` net may be weakly driven with a `Word[8]` expression.
* Weak drivers may be guarded (appear inside `when`/`match` arms) just like
  strong drivers.
* Weak drivers target shared signals only, for the same reason `#=` does.

See §10 Driving nets for how strong and weak drivers resolve.

## 7. Initialization

Initial values of each port, wire, and reg signal are set to an **arbitrary
value**. It is up to the simulator to determine the method (seeded random,
zeroing, test-pattern initialization — all acceptable).

This only applies to `reg`s (before reset) and top-level `incoming` ports,
since all other owned signals trace back combinationally to one of these.
Any value which "would" be floating collapses to a definition (albeit an
arbitrary one).

Shared signals have **no initial value**: `bus`es and `net`s are left
floating unless combinationally driven (or weakly driven) from the initial
conditions of the simulation.

(The `InitPolicy` mechanism approved in v2/v3 — `Zero` default for
deterministic tests, `Random(seed)` — is retained as the implementation of
"arbitrary value"; it is consistent with CONCEPT, which deliberately leaves
the method to the simulator.)

## 8. `when` and `match` Statements

`when` statements are allowed to omit the `else` clause when driving `net`s
and `bus`es, and `match` statements are not required to have complete case
coverage (similar to `reg`). The semantics are obvious: when there is no
case for the given situation, **no value is driven**.

When lowering to Verilog, this means setting the resulting variable to a Z
value of the proper bitwidth.

`when` and `match` **expressions** are unaffected: both still require
complete case coverage (`when` expressions require an `else`; `match`
expressions require full coverage or an `else`).

This is the entire tristate mechanism. There is no `tri(enable, data)`
builtin and no `z` literal (see Rejected Designs).

## 9. Contention

Contention is **always** an error at simulation time. In real hardware,
contention creates a short circuit which will at best burn energy and at
worst destroy hardware. It is always the designer's responsibility to ensure
no contention is possible in a design.

In simulation, contention is grounds to **abort** the simulation in almost
all circumstances.

There are **no compile-time multi-driver warnings** (v2 item 13, retained):
multiple `#=` drivers on a shared net are legal by design; static heuristics
cannot prove contention and would warn on intentional complementary-guard
patterns. Contention is detected at simulation time only.

**Per-bit analysis (Interpretation).** CONCEPT phrases the resolution rules
in terms of "values". For multi-bit types, v4 interprets contention and
floating **per bit**: if any single bit of a net is multiply driven with
disagreeing values, that bit is in contention — and any contended bit is
grounds to abort (an electrical short on any bit is a short). Floating is
likewise per bit. This matches v2's approved item 11 and is the only
electrically sensible reading; flag it for review.

## 10. Driving Nets (Resolution Rules)

The rules for resolving a net's value, from CONCEPT verbatim:

1. If a net is driven by **only weak** values, and those values are all the
   same, the value of the net is that value.
2. If a net is driven by **only weak** values, and there are at least two
   such values (that disagree), the net is in contention.
3. If a net is driven by **at least one strong** value, and all the strong
   values are the same, that is the value of the net — **overriding** any
   weak values being driven.
4. If a net is driven by **at least one strong** value, and there are at
   least two distinct strong values, the net is in contention.

And the implicit fifth case, from §3: a net driven by **nothing** is
floating.

Notes:

* "Values" means per-bit values for vector types (§9 Interpretation).
* A *partial* driver (one whose `when`/`match` guard excludes the current
  situation, §8) contributes nothing under those conditions; it is not
  counted by the rules above while it contributes nothing.
* Identical weak drivers "all the same" collapse; this makes redundant
  pull-ups harmless and is what makes open-drain patterns (I²C-style:
  everyone either drives low strongly or pulls / releases) expressible.

## 11. Tristate

Using this feature, a tristate module (CONCEPT's example, verbatim):

```virdant
mod Tristate {
    incoming data : Valid[Bit]

    bus out : Bit {
        it #= match {
            case @Valid(b) => b
        }
    }
}
```

The `match` is intentionally incomplete: when `data` is `@Invalid`, `out` is
not driven at all — it floats. Lowered to Verilog this becomes
`assign out = <valid> ? <bit> : 1'bz;`.

---

## Semantic Model (implementation-facing)

These are the mechanics needed to realize CONCEPT's rules; they impose no
new language surface.

### Net identity

A **net** is an equivalence class of shared signals, computed at elaboration
by union-find over attach edges (`#=#`, parent connects of `bus` ports,
socket bulk connects when sockets arrive). Every shared signal belongs to
exactly one net. **Interpretation:** `#= ` never merges nets — it is a
one-way driver contribution. Only `#=#` (and structural port connects of
`bus` ports) create net identity. (v3 conflated these; see Deltas.)

Scope rule for attach resolution: an attach written in the parent
(`bar.b #=# a`) resolves `bar.b` in the child's scope and `a` in the
parent's scope; an attach written inside a moddef resolves both operands in
that moddef's scope.

### Drivers

Each driver is a *(strength, contribution)* pair attached to a signal —
and therefore to its whole net:

* `Strength ∈ { Strong, Weak }`: `#=` → Strong; `#-` → Weak.
* Each contribution may be *partial*: `Driver::When/Match` with missing
  arms, and `Driver::Absent` filling non-covered situations, evaluate to
  "no contribution" under some conditions (existing machinery; see §8).
* Owned signals keep exactly one strong contribution, by construction.

### Resolution

Per net, per bit, evaluate every driver contribution to `Option<Value>`
(`None` = partial-absent), then apply §10's rules plus floating. Cache
per-driver contributions so a change to one driver only re-evaluates that
driver and the net's dependents.

**Consumers of a resolved net.** Because bare shared references are fenced
out of expressions (§5), ordinary expression evaluation never reads a shared
net. The consumers of resolution results are exactly: `tap` expressions, and
the VCD writer. This keeps the existing owned-value evaluation pipeline
intact.

---

## Implementation TODO

Ordered by dependency. Files reference the `~/projects/virdant_tristate`
layout (v3's mapping was verified accurate and is reused; the v3 branch's
diffs remain useful as a mechanical reference even where its design is
overturned).

1. **Grammar and AST** (`virdant/src/syntax/virdant.lalrpop`,
   `token.rs`, `payload.rs`, `common.rs`; regenerate `GRAMMAR.txt` with
   `make grammar`)
   - Keywords `bus`, `net` (`KwBus`/`KwNet`); component declarations with
     `ComponentKind::{Bus, Net}` (no `on`/`default` clauses; optional
     it-block).
   - Tokens/operators: `OpStrongDriver` (`#=`), `OpAttach` (`#=#`),
     `OpWeakDriver` (`#-`). Lexer ordering care: `#=#` must be matched
     before `#=`, both before the `#<ident>` annotation pattern.
   - `payload::Driver` gains a `strength: Strength` field
     (`Strong`/`Weak`) rather than v3's `shared: bool` — owned drivers are
     always Strong, and the strength travels with the driver into the sim.
   - Socketdef bus slots are **not** implemented (Future Work).
2. **Component analysis** (`analysis/component.rs`)
   - `Bus`/`Net` → `Flow::Duplex`; child `bus` ports do not flip
     (neither side owns them); submodule bus-port registration; ports list
     maps `Bus => PortDir::Bidir` (`Net` is not a port).
   - `can_sink`/`can_source` already handle `Duplex`.
3. **Driver rules** (`analysis/drivers.rs`, `queries/check_drivers.rs`)
   - Operator/kind cross-checks: `#=`/`#-`/`#=#` on owned →
     `SharedDriverOnOwned` (message distinguishes the op); `:=`/`<=` on
     shared → `OwnedDriverOnShared`.
   - Shared targets: skip `NoDrivers`/`NoRegDrivers`/`MultipleDrivers`;
     drivers are `Continuous`.
4. **Typing and fencing** (`types/typing/*`)
   - `tap n`: new builtin. `tap` of a `net`/`bus` path yields the net's
     type; `tap` of anything else → `TapNotOnShared`. This is the *only*
     expression that may mention a shared component; thread an expression
     context (as v3 did with `ExprCtx`) so any other appearance of a shared
     path produces:
     > `"{name}" is not guaranteed to have a value. Use "tap {name}" to
     > assert it will.` (`UnsafeSharedRead`)
   - `#=#` type equality on both operands (`JoinTypeMismatch`/`WrongType`).
   - `#-` RHS must have the target's type (ordinary typing; any type
     allowed — do **not** restrict to `Bit`, that was v3's divergence).
5. **Match/when coverage** (`types/match_coverage.rs`)
   - Statement-level `when`/`match` whose arms drive only shared targets
     are exempt from exhaustiveness (absence-as-Z). Owned-target behavior
     is unchanged.
6. **Elaboration changes needed by the frontend** (`analysis/elaboration.rs`)
   - Only what the frontend itself requires: `ElaboratedComponent.drivers:
     Vec<Driver>` (owned: ≤ 1; shared: any) so driver-count and fencing
     checks see every driver, plus union-find net classes over attach edges
     so `#=#` validation (type equality, sharedness of both operands) works
     across the attach graph. Attach statements are removed from driver
     lists and become union edges.
   - **The simulator rework is NOT in this plan.** The `sim/*` side of the
     semantic model (per-net multi-driver resolution, strengths,
     `NetState`, contention abort, `tap` evaluation, `InitPolicy`, VCD
     dumping) is deliberately deferred — same rationale as v3: it is a
     several-hundred-line rework of the resolution core, and this plan's
     test suite observes semantics through iverilog on the generated
     Verilog, not through the Rust sim. See Future Work.
7. **Verilog backend** (`verilog/*`)
   - `bus` ports → `inout`; `net` → `wire` declaration.
   - Shared-target driver blocks: missing-arm default lowers to `1'bz` at
     proper width (`ZLit`); owned targets keep `1'bx`/existing behavior.
   - `#=#` internal joins → `tran (a, b);` per bit (v3 found iverilog
     rejects vector `tran` arrays — reuse its per-bit scalar lowering);
     parent connects of child bus ports → direct port wiring (`.b(n)`),
     which is itself an attach, not a copy.
   - Weak drives: unguarded scalar `n #- true/false` → Verilog
     `pullup(n);` / `pulldown(n);` primitives. Unguarded `#-` with a
     non-literal or vector RHS, and guarded `#-`: Verilog's
     `assign (weak1, weak0)` drive-strength syntax is the candidate;
     **verify iverilog support during implementation** and fall back to a
     documented strong-conditional approximation for what it cannot
     express, exactly as v3 documented its guarded-pull gap. Resolution
     semantics are iverilog's (Verilog models contention as `x` and
     multiple-assign tristate nets natively); the Verilog gap per case is
     documented.
   - `tap n` lowers to the plain reference of `n` (no-op marker).
8. **Ancillary**: formatter (new operator alignments), virdoc (bus/net
   docs, `Bidir` badge), LSP hover (`Bidir` direction), platform check
   rejects `bus` ports (platforms declare no bus ports; backend support is
   Future Work).
9. **Tests**: carry over v3's test matrix (errors / warnings / pass /
   format / virdoc / project-style iverilog sims), adjusted:
   - Replace all `pullup(v)`/`pulldown(v)` read fixtures with `tap`
     fixtures (including the CONCEPT-specified error message).
   - Replace `#= pullup()` drive fixtures with `#-` fixtures, adding a
     vector-typed weak drive case (`net w : Word[8] { w #- 0xFF }`).
   - `#=#` fixtures replace the bare-ref-join ones; add the two CONCEPT
     submodule examples verbatim as pass fixtures.
   - Resolution-rule coverage (weak-collapse, weak-conflict,
     strong-overrides-weak, contention) is asserted through the iverilog
     project-style tests on the generated Verilog, matching v3's approach.

## Open Questions

1. **`tap` of a floating net at simulation time: error or undefined?**
   CONCEPT says the designer is responsible for ensuring a single value
   when using `tap`, and that contention is *always* an error. v4 leans
   **error** (a broken assertion is an error, symmetric with contention),
   but CONCEPT stops short of saying so explicitly. Confirm before
   implementing §6.
2. **Per-bit vs whole-value contention for vector types.** v4 adopts
   per-bit (§9 Interpretation). CONCEPT's rules speak of whole "values";
   whole-value semantics would make a single contended bit poison the
   whole net's reads. Per-bit is the v2-approved reading and the electrically
   sensible one; confirm.
3. **`tap` inside the moddef that owns the net's drivers.** A moddef that
   only ever drives a `net` unconditionally might "obviously" always have a
   value, yet `tap` is still demanded. CONCEPT gives no static escape hatch,
   so v4 provides none (uniform fencing). Recording in case ergonomics
   complain later; no lint proposed.

## Rejected Designs

### WONTDO: `tri(enable, data)` builtin (v1)

Reifies absence as an expression. CONCEPT expresses tristate via incomplete
`when`/`match` statements; absence-as-Z needs no builtin. (v2 rejected; v4
concurs.)

### WONTDO: `z` expression marker

`it := mux(enable, tx_byte, z)` — a builtin that typechecks at any type but
means "no contribution" reifies Z as a value, violating §4 directly.
**WONTDO.**

### WONTDO: `pullup(v)` / `pulldown(v)` as the shared→owned read crossing (v2, v3)

CONCEPT defines exactly one crossing — `tap` — and its error message
verbatim. A pull-based read-with-default *changes the semantics* (floating
silently resolves to the pull value instead of surfacing the designer's
broken assertion) and restricts reads to `Bit`. Weak driving belongs in the
driver forms (`#-`), not in expression position. **Overturned; WONTDO.**

### WONTDO: `#= with a bare shared ref` as implicit attachment (v3)

CONCEPT has a dedicated attach operator (`#=#`) and forbids bare shared
references outright (§5). Making `a #= b` a join requires a bare-reference
exception to the fencing rule and makes driver-vs-join invisible at the use
site. `#=#` is loud, unambiguous, and matches Chisel's precedent. **Overturned.**

### WONTDO: `outgoing bus` / `incoming bus` mirror spellings (v2)

Overturned in v3, upheld in v4: one `bus` keyword, no direction, exactly as
CONCEPT spells it.

### WONTDO: compile-time multi-driver warnings (v2 item 13, retained)

Static heuristics cannot prove contention and would fire on intentional
complementary-guard patterns. Contention is a simulation-time error only.

## Future Work (per CONCEPT §Future Work)

* **Sockets**: socketdefs may contain slots marked `bus`, just as they have
  `cosi` and `soci` slots. Both the `client socket` and `server socket`
  sides end up with a bidirectional `bus` port. Bulk connect joins the bus
  members into one net. Deferred with the rest of this plan, as CONCEPT
  places it in Future Work.
* **Platform/top-level `bus` ports**: real tristate I/O pins, IOBUF
  inference, internal shared-net lowering strategy for FPGA backends
  (carried from v2 item 14; not in CONCEPT, deferred regardless).
* **`unsafe_read`-style ergonomics**: none planned — `tap` *is* the unsafe
  crossing. If a read-with-default need arises, design it as a *separate
  explicit primitive* in a future plan; do not fold it into `tap`.
* **Simulator rework** (the §Semantic Model's resolution side): union-find
  net classes feeding the sim, `Vec<(Strength, Expr)>` drivers,
  per-bit `NetState` resolution implementing §10, contention abort,
  `tap` evaluation against net state, `InitPolicy`, VCD net dumping. Its
  own follow-up plan; the Open Questions (especially 1) resolve there.

## Status

Draft for review. **Scope: language surface, checks, and Verilog backend
only** — the simulator rework is a separate follow-up plan (see Future
Work). Core surface is fully determined by CONCEPT.md; the three Open
Questions above are interpretations where CONCEPT is silent, not
inventions. Implementation starts from `master`; the `tristate` branch is a
mechanical reference only.
