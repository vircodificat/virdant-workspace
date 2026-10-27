# Tristate and Bidirectional Ports (v2)

**v2 supersedes `PLAN.md` (v1).** Per workspace rules, the highest version suffix is
canonical. `CONCEPT.md` takes precedence over any plan; where v1 diverged from
CONCEPT, v2 realigns with CONCEPT.

## Key deltas from v1

| Topic            | PLAN v1                                   | PLAN v2 (this document)                  |
|------------------|-------------------------------------------|------------------------------------------|
| Port naming      | `inout`                                   | `bus` (per CONCEPT), spelled `outgoing bus` |
| Local shared net | not included                              | `net` declaration (per CONCEPT)          |
| Driver operator  | `:=` for everything                       | `#=` for shared signals; `:=` for owned  |
| Tristate         | `tri(enable, data)` builtin               | absent-as-Z: missing `else` case in a `#=` driver block; **no `tri` builtin** |
| Absence-as-Z     | Future Work                               | Core feature                             |
| `pullup`/`pulldown` | Rejected as out of scope               | In scope (per CONCEPT)                   |
| `unsafe_read`     | Core shared→owned crossing                | **Deferred to Future Work**              |
| Sockets `bus` slots | Future Work                            | In scope (approved)                      |

Items below marked **[approved]** were explicitly approved during the v2 interview.
Items marked **[deferred]** or **[rejected]** record decisions made during the
interview. There are no unresolved items; see the Status section.

## Core Model: The Safe World and the Unsafe World

### 1. X and Z are net properties, not values [approved]

In Verilog, `x` and `z` are values that infect expressions and simulation.
In Virdant, they are **evidence about driver multiplicity on a net**:

* **Z** = the net has **0 drivers** ("floating").
* **X** = the net has **2+ drivers** ("contention").

They are not values, do not inhabit any type, and never appear in the type
system. Resolution is a property of the **net**, not of any driver or
expression.

### 2. Owned and shared signals [approved]

Two signal classes:

* **Owned** — `incoming`, `outgoing`, `wire`, `reg`. Exactly one driver (or one
  guarded driver block) by construction. The driver (informally, the module)
  *owns* the signal. Contention and floating are impossible by construction.
* **Shared** — the new class this feature introduces. Any number of drivers,
  including zero. Shared signals may float or contend.

### 3. The shared surface: `bus` ports and `net` declarations [approved]

* **`bus` port** — a third port kind alongside `incoming`/`outgoing`.
  A moddef may drive it like an `outgoing` port; the parent may drive it like
  an `incoming` port; both sides may read it.
* **`net` declaration** — a locally-visible signal, like `wire`, but shared:
  multiply-drivable, may float.

```virdant
inout bus : Word[8]      // NO — v2 uses:
bus   bus : Word[8]      // (see Open Questions for exact syntax)
net   tmp : Word[8]
```

### 4. Driving shared signals: `#=` [approved]

Shared signals are driven with a **distinct operator, `#=`**, so that shared
driving is visually loud at the use site. `:=` remains reserved for owned
signals.

* Multiple `#=` drivers on the same `bus`/`net` are legal — including drivers
  spread across `when`/`match` arms.
* A shared signal may be driven from any signal you can read: `incoming`,
  `wire`, `reg`, `net`, or `bus`. Driving a bus from another bus joins the
  two nets into one.
* A `bus` may be driven by the moddef (like an `outgoing`) and by the parent
  (like an `incoming`).
* **Parent connection** is exactly like other ports: instance named
  connection, connecting a child `bus` port to a parent `bus`/`net`
  associates the two into one net (like an `outgoing` connecting to a
  `wire`). No separate join operator exists.

### 5. Absence-as-Z: missing `else` means "not driving" [approved]

When **driving a shared signal** (via `#=`), a `when` block may omit the
`else` clause, and a `match` may have incomplete case coverage. The missing
case drives **nothing** — the net is simply not driven under that
circumstance. When lowering to Verilog, this emits a `1'bz` of the proper
bitwidth.

`when`/`match` **expressions** are unaffected: they still require complete
case coverage (an `else` / full cases).

This is the entire tristate mechanism. There is **no `tri(enable, data)`
builtin** in v2 (see Rejected Designs).

```virdant
mod Tristate {
    incoming data : Bit
    incoming enable : Bit

    bus out : Bit {
        when {
            case enable {
                it #= data
            }
            // No `else` clause on purpose: when `enable` is low,
            // `out` is not driven at all.
        }
    }
}
```

### 6. Reading from shared signals: `unsafe_read(v)` [deferred]

Values must propagate between the two classes, and the dangerous edge is
**shared → owned**: driving an `outgoing`/`wire`/`reg` from a `bus`/`net`
could float or contend the owned signal, which proper operation disallows.

`unsafe_read(v)` — the primitive for reading a shared signal into an owned
context — is **deferred to Future Work**, like clock crossings. The crossing
is inherently unsafe: there is no proof the compiler can or should provide,
just as with clock-domain crossings or working with C arrays. The designer
"just has to know what they're doing" — but v2 deliberately ships no language
support for it rather than shipping a half-built primitive.

* In this plan, shared signals may be **read** only where the consuming
  context is itself shared (e.g., as sources of `#=` drivers to `bus`/`net`).
* A bare read of a shared signal into an owned context is **not supported**
  in this plan; when `unsafe_read` lands in future work, it becomes the
  required crossing (see Future Work).

```virdant
wire byte : Word[8] {
    it := unsafe_read(bus)   // FUTURE WORK — not in this plan
}
```

**Design note on this plan's fencing.** With `unsafe_read` deferred, the only
shared → owned reads are `pullup`/`pulldown` (item 10). This deliberately
leaves a hole: there is as yet no way to read a `Word[8]`-typed shared bus
into owned logic (e.g., to build an Address Decoder that samples a bus).
That hole is exactly what `unsafe_read` fills in Future Work; v2 prefers an
obviously-incomplete feature over a quietly-unsafe one.

### 7. Contention is always an error at simulation time [approved]

Contention (two+ drivers on the same net — an electrical short) is *always*
an error, even on shared signals. It is always the designer's responsibility
to ensure no contention is possible. In simulation, contention is grounds to
**abort** in almost all circumstances. (When `unsafe_read` lands in future
work, reading a contended value will likewise have no defined semantics.)

### 8. `unsafe_read` of a floating value → simulator panic [deferred]

Deferred with `unsafe_read` (item 6). When it lands: `unsafe_read(v)` when
`v` is floating (Z) is technically undefined behavior; the simulator should
**panic** rather than return garbage.

### 9. Initialization [approved]

* **Owned** signals initialize to an *arbitrary* value; the method is up to
  the simulator (seeded random, zeroed, test patterns). This applies only
  where it matters: `reg`s (before reset) and top-level `incoming` ports —
  everything else traces back combinationally to one of those. Any
  "would-be-floating" owned value collapses to an arbitrary definition.
* **Shared** signals have no initial value — `bus`es and `net`s are floating
  unless driven from the simulation's initial conditions.

### 10. `pullup` / `pulldown` [approved]

For **`Bit`**-typed shared signals only:

* `pullup(v)` and `pulldown(v)` weakly drive the shared net toward `true` and
  `false`, respectively.
* **They are also shared → owned crossings** — the only ones in this plan
  (`unsafe_read` is Future Work): reading through a pull resolves a
  *floating* net to the pulled value (`pullup` → `true`, `pulldown` →
  `false`), yielding an ordinary owned `Bit` value. Non-`Bit` shared signals
  cannot be read this way.
* Dual role (Open Question 6): in this plan, `pullup(v)`/`pulldown(v)` used
  **as a driver target** (`v #= pullup; v #= pulldown;` — or however the
  driver form lands) weak-drives the net, and used **in an expression**
  performs the read-with-default. Both roles coexist; the driver form and
  its exact syntax are part of the Implementation TODO.
* Redundant identical weak drives collapse (act as one). Conflicting weak
  drives (one pulls up, one pulls down) act as contention.

These matter for open-drain patterns such as I²C.

### 11. Multi-bit values are analyzed bitwise [approved]

For shared signals of multi-bit types (e.g., `Word[2]`):

* **Contention is determined per bit.** If any single bit is multiply driven,
  that bit is in contention — and any contended bit is sufficient grounds to
  terminate simulation (an electrical short on any bit is a short).
* **Floating is determined per bit.** A hypothetical `Word[2]` value
  `2'b0z`: indexing `v[1]` yields a clean `0`, safely readable; `v[0]` is
  floating. (Per-bit reads of shared signals become observable once
  `unsafe_read` lands in future work.)

## Shared → Owned crossings (summary)

In this plan, there are exactly **two** ways a shared value becomes an owned
value:

| Primitive       | Type restriction | Floating resolves to | Contended        |
|-----------------|------------------|----------------------|------------------|
| `pullup(v)`     | `Bit` only       | `true`               | contention error |
| `pulldown(v)`   | `Bit` only       | `false`              | contention error |

`unsafe_read(v)` (any type, no default) is **Future Work**. Anything else
that drives an owned signal from a shared signal is an error.

## Remaining items (decided during the interview)

### 12. Sockets `bus` slots [approved]

Socketdefs may contain slots marked `bus`, alongside `cosi`/`soci`. Both the
client and server side of the socket get a bidirectional `bus` port. Bus
members are joined by `#=` (sharing the net) rather than direction-mapping
like cosi/soci members.

### 13. Same-moddef contention check [rejected]

CONCEPT's TODO suggested a common-sense error/warning when two drivers
obviously appear in the same moddef. **Rejected in v2:** no multi-driver
warnings at all. Multiple `#=` drivers on a shared net are legal by design;
whether they contend is the designer's guarantee to make, and only simulation
can prove actual contention. Static heuristics would warn on intentional
patterns (e.g., complementary guards across separate when-blocks) without
being able to prove real contention.

### 14. FPGA support [deferred]

How common are tristates in FPGAs? Most FPGA fabrics only support tristate at
I/O pins, not on internal routing. **Deferred in v2 to a follow-up plan:**
this plan is language-only (surface, typing, checks, simulator). The
platform/IOBUF story — platform `bus` ports as real tristate pins, IOBUF
inference, internal shared-net lowering (mux-tree conversion or rejection) —
is deliberately left to a backend-focused plan. Platform definitions can
already reject unsupported constructs per-platform, so early backends may
simply diagnose `bus`/`net` as unsupported until the follow-up lands.

## Open Questions

1. ~~Exact `bus` declaration syntax.~~ **Resolved:** `bus` follows the same
   keyword+kind pattern as `outgoing wire`/`outgoing reg`: the declaration
   keyword is `outgoing bus` (see note below). `bus out : Bit` would be
   redundant the way `outgoing out : Bit` is; CONCEPT's `bus out : Bit`
   shorthand is dropped.
2. ~~When is `unsafe_read` required versus optional?~~ **Deferred with
   `unsafe_read` itself** (Future Work). Previous lean recorded for that
   future discussion: (a) required on every read of a shared signal — the
   compiler does not prove drivenness, it demands acknowledgment.
3. ~~How are `bus` ports connected at the parent?~~ **Resolved:** `bus` ports
   connect to the parent just like `incoming`/`outgoing` (instance named
   connection; connecting associates the nets). Joins between bus nets use
   `#=` (a bus may be driven from another bus), not a separate `:=:` join
   operator — there is no `:=:` join in v2.
4. ~~Does a parent-side or `:=:`-style join of N bus ports trigger the 2+-
   drivers warning?~~ **Moot:** no multi-driver warnings exist (item 13
   rejected).
5. ~~Do ordinary (total) `#=` drivers on shared nets trigger the multiplicity
   warning?~~ **Moot:** same reason.
6. ~~Do `pullup`/`pulldown` also drive when used purely as a read?~~
   **Resolved:** both roles coexist. As a *driver contribution* they
   weak-drive the net; in an *expression* they are the read-with-default
   crossing. The exact driver syntax is left to implementation (see item
   10).

**Note on `outgoing bus` naming.** Every existing port keyword begins with
`outgoing` (a submodule *outgoing* port: `outgoing`, `outgoing wire`,
`outgoing reg`); `incoming` is the parent-side mirror of the same port seen
from the parent's view. Since `incoming` and `outgoing` are the two views of
one port kind, and v2 adds a third kind, `outgoing bus` names the shared
port; `incoming bus` is its parent-side mirror (permitted at top level and
in platform definitions, like `incoming`/`outgoing` today).

## Rejected Designs

### WONTDO: `tri(enable, data)` builtin (v1's core mechanism)

v1 made a `tri(enable, data)` native builtin the only way to write a partial
driver. v2 rejects it: CONCEPT's absence-as-Z achieves the same fencing with
less surface area, keeps `tri` semantics identical to a missing-`else` arm
(making the eventual "sugar" unification a non-event), and avoids adding a
construct whose only purpose is to spell out what absence already means. If
ergonomics later demand it, `tri(e, d)` can be added as pure sugar for a
one-arm `when` — a syntax change, not a semantic one.

### WONTDO: `z` expression marker

```virdant
it := mux(enable, tx_byte, z)   // REJECTED
```

A builtin that typechecks at any type but means "no contribution" reifies Z
as a value. This violates core principle 1 directly. **WONTDO.**

### Overturned: "language-level pull-up primitives rejected"

v1 rejected pull-up/pull-down behavior as unmodelable. v2 includes
`pullup`/`pulldown` per CONCEPT: they are `Bit`-only weak drivers /
read-with-default crossings, lowering to Verilog's `pullup`/`pulldown`
primitives (or a weak-assign fallback), and require no Z-detection
primitives or driver-strength dimension in the language's net model beyond
the weak/strong distinction these two primitives embody.

## Status

All fourteen items resolved: 1–5, 9–12 approved; 6, 8 deferred with
`unsafe_read`; 13 rejected; 14 deferred. All six Open Questions are resolved
(1, 3, 6 decided during the v2 interview; 2, 4, 5 moot/deferred). The plan is
ready for implementation review.

## Summary of Rules

1. X/Z are net properties (0 drivers / 2+ drivers), never values.
2. Owned signals (`incoming`, `outgoing`, `wire`, `reg`): exactly one driver
   by construction; never float, never contend.
3. Shared signals (`bus` ports, `net` declarations): any driver count
   including zero; may float or contend; driven with `#=`.
4. Missing `else` / incomplete `match` in a `#=` driver block means "not
   driving" (absence-as-Z). Expressions still require complete coverage.
5. Shared → owned reads: only via `pullup`/`pulldown` (`Bit` only, floating
   resolves to the pull). `unsafe_read` (any type) is Future Work; other
   shared → owned reads are errors in this plan.
6. Contention is always an error at simulation time.
7. Owned signals initialize arbitrarily (regs pre-reset, top-level incoming);
   shared signals initialize floating.
8. Contention and floating are analyzed per bit for multi-bit types.
9. Socketdefs may contain `bus` slots; both socket roles get a bidirectional
   `bus` port, joined by bulk connect.

## Implementation TODO

Ordered roughly by dependency.

1. **Grammar and AST**
   - `outgoing bus` port: follows the `outgoing wire`/`outgoing reg` pattern
     in `virdant.lalrpop` (new `ComponentKind::OutgoingBus`, keyword
     registration alongside `KwWire`/`KwReg`). Parent-side mirror
     `incoming bus` for top-level/platform use. `net` declaration keyword
     for local shared signals.
   - `#=` driver operator token and parsing, distinct from `:=`.
   - `pullup(x)`, `pulldown(x)`: builtin expression parsing
     (verify whether existing builtins are parsed as calls or need keyword
     registration).
   - Driver-block absence rules: when a driver block targets a shared signal,
     permit missing `else` / incomplete `match`; when targeting an owned
     signal, keep existing completeness errors.
2. **Typing (TypeCheck / TypingContext)**
   - `bus`/`net` declarations as a new component kind; parent/child drive and
     read rules for `bus`.
   - `pullup(x)`, `pulldown(x) : Bit` when `x : Bit` (shared only).
   - Driver rules: `#=` may target shared signals only; `:=` may target owned
     signals only; reads of shared signals are legal only in shared contexts
     (e.g., `#=` sources) — shared → owned reads require a crossing
     primitive, which is Future Work (`unsafe_read`).
3. **New check: `PartialityCheck`** (or `SharedNetCheck`)
   - Absence analysis: which `#=` driver blocks are partial; per-bit partiality
     for multi-bit types where applicable.
   - Fencing: reads of shared signals outside the two crossing primitives
     (`pullup`/`pulldown`) are errors; `#=` on an owned signal is an error;
     partial driver blocks on owned signals are errors.
   - No multi-driver warnings: multiple `#=` drivers on a shared net are
     legal by design (see item 13).
4. **Simulator**
   - Per-bit net states: driven-0, driven-1, floating, contended.
   - Initialization: arbitrary values for regs pre-reset / top-level incoming;
     shared nets floating.
   - Contention → abort; `pullup`/`pulldown` reads resolve floating to the
     pulled value.
   - Redundant identical weak drives collapse; conflicting weak drives →
     contention.
5. **Verilog backend**
   - Emit `outgoing bus`/`incoming bus` ports as Verilog `inout`; `net` as
     Verilog `wire`.
   - Missing-else driver blocks lower with `1'bz` of proper bitwidth.
   - `pullup`/`pulldown` lower to Verilog pull primitives or equivalent.
6. **Sockets `bus` slots**
   - `bus` slot kind in socketdefs (alongside `cosi`/`soci`), generic per
     existing socketdef generics.
   - Client and server sides each get a bidirectional `bus` port.
   - Bulk-connect joins `bus` members rather than direction-mapping them.
## Future Work

* **`unsafe_read(v)`**: the shared → owned crossing for any type. Inherently
  unsafe, like clock crossings — the designer's guarantee, not the
  compiler's. When it lands, decide when it is required versus optional
  (lean: required on every read of a shared signal), and implement the
  simulator panic on floating reads.
* Platform `bus` ports, IOBUF inference, and internal shared-net lowering
  (deferred backend plan, see item 14).
* Tests and examples: bidirectional bus, I²C open-drain, error fixtures.
