# Tristate and Bidirectional Ports

## Motivation

Virdant has `incoming` and `outgoing` ports, but no notion of bidirectional
ports, and no way to produce a high-impedance value. This plan adds:

* `inout` ports — ports that both the moddef and the parent may drive and read.
* `tri(enable, data)` — a native builtin expression for tristate driving.
* `unsafe_read(x)` — a marker for reads that may observe a floating or
  contended net.
* `:=:` bidirectional connect — joining two `inout` nets into one.
* `PartialityCheck` — a new analysis owning the safe/unsafe boundary.

See `CONCEPT.md` for the original idea writeup.

## Core Model: The Safe World and the Unsafe World

### X and Z are net properties, not values

The central design principle. In Verilog, `z` and `x` are values that infect
expressions and simulation. In Virdant, they are **evidence about driver
multiplicity on a net**:

* **Z** = the net has **0 drivers** ("no value for this net").
* **X** = the net has **2+ drivers** ("contended values").

They are not values, do not inhabit any type, and never appear in the type
system. Resolution is a property of the **net**, not of any driver or
expression.

### The safe world

`incoming`, `outgoing`, `wire`, and `reg` all carry driver rules that
statically guarantee **exactly one driver** and **never floating**:

* `incoming` must be driven by the parent, and may not be driven inside the
  moddef.
* `outgoing` must be driven inside the moddef, and may not be driven by the
  parent.
* A moddef may assign to a safe net multiple times only under
  syntactically-obvious `when`/`match` guards that statically prevent
  contention.

The compiler therefore guarantees: neither contention (X) nor floating (Z)
are possible within the safe world (barring them bleeding in from the top
level, which is out of scope for this plan).

### The unsafe world

`inout` is the sanctioned weakening of these rules:

* Both the moddef and the parent may drive an `inout` net.
* Both sides may read it.
* Contention is **the designer's guarantee to make, not the compiler's**.
  Multiple drivers in the same moddef are legal.

Because the compiler no longer guarantees drivenness or non-contention, the
boundary between the two worlds must be **loudly marked**:

* Driving "nothing" is only possible via `tri(enable, data)`.
* Reading a net that might be Z or X requires `unsafe_read(x)`.

The surface of the entire unsafe world is four constructs:

| Construct        | Role                                   |
|------------------|----------------------------------------|
| `inout`          | declaration — opens the boundary        |
| `tri(enable, data)` | partial driver — "I drive, or I don't" |
| `unsafe_read(x)` | risky read — "I might see Z or X"       |
| `:=:`            | join — two `inout` nets become one net  |

## The `inout` Port

```virdant
inout bus : Word[8]
```

* Electrically a wire. The rules are the loosest of the three port kinds:

| Port kind  | Driven by parent | Driven inside moddef |
|------------|:----------------:|:--------------------:|
| `incoming` | must             | forbidden            |
| `outgoing` | forbidden        | must                 |
| `inout`    | allowed          | allowed              |

* Both sides may read it.
* **Any arbitrary type** is allowed. The typing is fully general:
  `inout x : A` for any `A`. "The net had 0 drivers" is meaningful
  regardless of type, since Z/X are net properties, not values.
* A driver block may be attached directly (`inout bus : Word[8] { ... }`)
  or written as separate assignments, like other nets.
* `Clock` and `Reset` are technically legal inout types under the
  general rule. If this ever proves to be a hazard rather than a feature,
  address it with a lint later.

## `tri(enable, data)`

A native builtin expression. Argument order: **enable first**.

```virdant
bus := tri(wen, tx_byte)
```

### Typing

* `tri(enable, data) : A` when `enable : Bit` and `data : A`, for any `A`.
* The typing is deliberately simple. All the "this can fail to drive"
  reasoning lives in `PartialityCheck` (below), not in the type system.

### Semantics

`tri(enable, data)` behaves **exactly** like:

```virdant
when {
    case enable => data
    // no `else` case
}
```

A matched arm with no assignment means "not driving". `tri` does **not
produce a value** — it is a driver contribution, not an expression result.
Since Virdant has no simulator, there is no runtime cost to model: `tri`
lowers directly to Verilog.

### Verilog lowering

```virdant
bus := tri(enable, data)
```

lowers to:

```verilog
assign bus = enable ? data : 1'bz;
```

Contention between multiple drivers is Verilog's own X-resolution problem;
Virdant emits nothing special for it.

### The bridge to absence-as-Z

When absence-as-Z (Future Work) lands, the raw missing-`else` form and
`tri(enable, data)` have identical semantics and may unify, with `tri`
becoming sugar. Designing `tri` now as "sugar for a matched arm with no
assignment" is what makes that migration a syntax change, not a semantic
one.

## `unsafe_read(x)`

```virdant
wire byte : Word[8] {
    it := unsafe_read(bus)
}
```

* **Hardware no-op.** It emits exactly the same hardware as a bare read.
* It is a **compile-time marker**: it signals to the compiler and to the
  reader that this is a place where one might accidentally read from a
  Z or X net.
* Exact rules for when `unsafe_read` is required versus optional are an
  **open question** (see Open Questions).

## `:=:` Bidirectional Connect

`:=:` gains a second use: connecting two `inout` nets makes them **one
net** — "these are the same net", not "assign".

```virdant
// Each participant drives its own inout port internally:
mod Agent {
    inout bus : Word[8]
    bus := tri(wen, tx_byte)
}

// The parent joins the participants into one net:
mod Top {
    inout bus : Word[8]
    a of Agent { ... }
    b of Agent { ... }
    bus :=: a.bus
    bus :=: b.bus
}
```

This is the canonical way to build a multi-participant bus across the
hierarchy. (Today `:=:` is socket bulk-connect; inout-to-inout becomes a
second, generalized use.)

## `PartialityCheck`

A new check, **separate from TypeCheck**, in the spirit of
`CombinationalCycleCheck`: typing governs values; this check governs how
many values a net may receive, and when.

### Why a separate check

* Typing stays trivially honest: `tri(...) : A` needs no pseudo-type, and
  the type checker never learns that Z exists.
* Cross-cutting net-level properties already live in standalone checks.
* One check owns the whole unsafe-world story coherently: fencing, target
  restrictions, and the multiplicity warning — instead of smearing them
  across TypeCheck, which embodies the safe world.

### Framing: partial drivers, not `tri()`

Define: a driver RHS is **partial** if there exist circumstances under
which it contributes nothing; otherwise it is **total**. Today, `tri(...)`
is the only way to write a partial RHS. When absence-as-Z lands, a missing
`else` arm becomes the second way — and the check does not change. The
check's contract ("which drivers are partial; what may a partial driver
drive") is the stable part across the migration.

### Fencing (partiality)

* A partial RHS is legal only where no-value-ness can **propagate to the
  driver root**: the whole RHS, or arms of `when`/`match`/`mux` (where
  sibling arms may be total or partial).
* A partial subexpression inside a context that consumes its operand and
  cannot mean anything is an error: arithmetic (`tri(a, e) + 1`),
  comparisons, bit-slicing, concat, function calls, and condition
  positions.
* A partial driver may target **only an `inout` net**:

```virdant
wire w : Bit { it := tri(x, e) }   // ERROR: partial driver on safe net
```

### Multiplicity

* Count drivers per net across all sources: moddef driver blocks, instance
  `inout` ports joined by `:=:`, and the parent's connection.
* Safe-world nets: the existing exactly-one/guarded rule stands; partial
  drivers are rejected.
* `inout` nets: any driver count is legal. **2+ drivers produce a warning**
  — accident detection, not an error. Contention is the designer's
  responsibility; the warning signals that there is still potentially an
  error.
* Open detail: whether `:=:` joins of N inout ports also trigger the
  multiplicity warning (they create 2+ participants too), or whether
  `:=:` joins are exempt as the canonical hierarchical pattern.

## Summary of Rules

1. `inout x : A` — any type; both sides may drive and read.
2. `tri(enable, data) : A` — native builtin; enable first; enable : Bit,
   data : A; does not produce a value; semantics = matched `when` arm with
   no else.
3. `unsafe_read(x)` — hardware no-op; compile-time marker; exact
   requirements TBD.
4. `:=:` between two `inout` nets joins them into one net.
5. `PartialityCheck`: partial drivers only on `inout` nets; partial RHSs
   only in root/when/match/mux positions; 2+ drivers on an inout net is a
   warning.
6. X/Z are never values anywhere in the language surface.
7. Weak driving, pull-ups, and other electrical effects are blackbox/
   primitive territory, outside the language's net model. Virdant models
   only strong drivers and no-drivers.

## Rejected Designs

### WONTDO: `z` expression marker

```virdant
it := mux(enable, tx_byte, z)   // REJECTED
```

A builtin that typechecks at any type but means "no contribution" reifies Z
as a value. This violates the core principle directly: the checker would
have to maintain a pseudo-value inhabiting every type, fence it against
reads/arithmetic/conditions, and key its legality on context. **WONTDO.**

### WONTDO (for now): explicit `tristate` element

A `tristate` declaration kind alongside `wire`/`reg` was considered and
dropped: the native builtin expression achieves the same fencing with less
surface area, and a bodyless builtin needs no in-language "not driving"
mechanism at all. (An explicit element remains conceivable as Future Work
only if `tri` proves insufficient.)

### Rejected: language-level pull-up primitives

Pull-up/pull-down behavior cannot be placed in an FPGA fabric anyway; it is
implemented as logic. Modeling it would either require Z-detection
primitives with special net-state privileges or a driver-strength dimension
on nets — both heavier than the feature is worth. **Out of scope; not
modeled in Virdant.**

## Future Work

* **Absence-as-Z** (long-term): a missing `else` arm in a driver block
  means "not driving" directly, without `tri`. Correct long-term design;
  deferred because it removes the compile-time missing-driver error on
  tristate nets (a forgotten driver silently becomes Z). Mitigation ideas
  (warning/lint on unterminated driver paths) to be designed then. `tri`
  is the bridge: same semantics, different syntax.
* **Sockets `bus` keyword**: a third socket member kind alongside
  `cosi`/`soci` — a bidirectional channel between client and server roles;
  both roles may drive via `tri` and read via `unsafe_read`; `:=:` bulk
  connect joins bus members rather than direction-mapping them.
* **Platform/top-level `inout`**: platform definitions declare `inout`
  ports (bidirectional Pmod pins, I²C SDA/SCL off-chip, memory buses);
  pin mapping and IOBUF inference in the FPGA backends. The language layer
  of this plan is deliberately separable from vendor-shaped backend work.
* **`unsafe_read` requirements**: decide when it is required versus
  optional (see Open Questions).
* **Tests and examples**: bidirectional bus, I²C-style open-drain line,
  error fixtures (partial driver on safe net), warning fixture
  (2+ drivers). Deferred for now.

## Open Questions

1. When is `unsafe_read` required versus optional? Candidates: (a)
   required on every read of an `inout` net; (b) required only when the
   net can float or contend (needs hierarchical analysis); (c) advisory
   only. Lean: (a) — the compiler does not prove drivenness, it demands
   acknowledgment, matching the "contention is the designer's guarantee"
   philosophy.
2. Does a `:=:` join of N inout ports trigger the 2+-drivers warning?
3. Should ordinary (non-`tri`, total) drivers on `inout` nets be legal?
   The current reading of "contention is the designer's matter" implies
   yes — an always-strong participant — but this is not yet pinned down.

## Implementation TODO

Ordered roughly by dependency. Grammar first, then typing, then the check,
then the backend.

1. **Grammar and AST**
   - `inout` keyword: token, `KEYWORDS` registration, port-declaration
     grammar (`inout <name> : <type> <driver-block?>`).
   - `tri(enable, data)` and `unsafe_read(x)`: builtin expression parsing
     (likely handled in the expression parser as builtins; verify whether
     existing builtins like `mux` need keyword registration or are parsed
     as calls).
   - `:=:` inout-to-inout connect: parse alongside existing bulk connect.
2. **Typing (TypeCheck / TypingContext)**
   - `inout` port declarations: component analysis for the new port kind.
   - `tri(enable, data) : A` typing rule (enable : Bit).
   - `unsafe_read(x) : A` typing rule (x : A).
   - Driver/block assignment rules: allow parent/child assignment to inout
     paths; allow driver blocks on inout nets.
3. **`PartialityCheck`** (new query)
   - Partial-RHS detection and fencing errors (target-net checks,
     propagation-position checks).
   - Per-net driver multiplicity collection (moddef drivers, `:=:` joins,
     parent connections) and the 2+-drivers warning on inout nets.
4. **Verilog backend**
   - Emit `inout` ports as Verilog `inout` for modules and instances.
   - Lower `tri(enable, data)` to `assign net = enable ? data : 1'bz;`.
   - `unsafe_read(x)` emits as a bare read (no-op).
   - `:=:` inout join emits as net aliasing / direct net connection.
5. **Follow-ups (out of this plan's scope)**
   - Absence-as-Z, sockets `bus` keyword, platform `inout`, tests/examples
     (see Future Work).