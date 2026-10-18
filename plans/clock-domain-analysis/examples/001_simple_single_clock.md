## Example 1: Simple Single-Clock Module

```virdant
mod Simple {
    incoming clock : Clock
    incoming data : Word[8]

    reg buffer : Word[8] on clock {
        it <= data
    }

    outgoing out : Word[8] {
        it := buffer
    }
}
```

**What's happening**: A simple buffer with one clock. The `data` port flows into a register, and the register drives `out`.

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `buffer` (reg, type Word[8])
- `out` (outgoing, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming data : Word[8]`
- Component: `data`
- No expressions

Declaration: `reg buffer : Word[8] on clock`
- Component: `buffer`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing out : Word[8]`
- Component: `out`
- No expressions in declaration

Driver: `buffer <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Driver: `out := buffer`
- Expression `#2`: reference to `buffer` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprReference to `buffer`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `data` → `dom(#1) = dom(data)`
- Expression `#2` references `buffer` → `dom(#2) = dom(buffer)`

**Equality constraints from drivers:**

Driver: `buffer <= data`
- LHS: component `buffer`
- RHS: expression `#1`
- Constraint: `dom(buffer) = dom(#1)`

Driver: `out := buffer`
- LHS: component `out`
- RHS: expression `#2`
- Constraint: `dom(out) = dom(#2)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(buffer) < Async`
- `dom(out) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(buffer) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  C₃: dom(clock) < Async
  C₄: dom(data) < Async
  C₅: dom(buffer) < Async
  C₆: dom(out) < Async

Equality:
  C₇: dom(#0) = dom(clock)
  C₈: dom(#1) = dom(data)
  C₉: dom(#2) = dom(buffer)
  C₁₀: dom(buffer) = dom(#1)
  C₁₁: dom(out) = dom(#2)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**

Assign concrete domain `Clock` to clock's equivalence class:
```
{clock: Clock}, {data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

**Processing fixed constraint C₂: `dom(buffer) = Sync(ComponentId(clock))`**

Assign concrete domain `Sync(clock)` to buffer's equivalence class:
```
{clock: Clock}, {data}, {buffer: Sync(clock)}, {out}, {#0}, {#1}, {#2}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {data}, {buffer: Sync(clock)}, {out}, {#1}, {#2}
```

**Processing equality constraint C₄: `dom(#1) = dom(data)`**

Merge expression #1 with data's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer: Sync(clock)}, {out}, {#2}
```

**Processing equality constraint C₅: `dom(#2) = dom(buffer)`**

Merge expression #2 with buffer's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer, #2: Sync(clock)}, {out}
```

**Processing equality constraint C₆: `dom(buffer) = dom(#1)`**

Merge buffer's class with #1's class:
- buffer's class: `{buffer, #2: Sync(clock)}` (has concrete domain)
- #1's class: `{data, #1}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {data, #1, buffer, #2: Sync(clock)}, {out}
```

**Processing equality constraint C₇: `dom(out) = dom(#2)`**

Merge out with #2's class:
- #2's class: `{data, #1, buffer, #2: Sync(clock)}`
- out's class: `{out}` (no concrete domain)
- Merged class keeps `Sync(clock)`

```
{clock, #0: Clock}, {data, #1, buffer, #2, out: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, buffer, #2, out} → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(buffer) = Sync(ComponentId(clock))
  dom(out) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(buffer) = Sync(clock)` ✓
- All strict upper bound constraints satisfied: ✓
  - `dom(clock) < Async` → `Clock < Async` ✓
  - `dom(data) < Async` → `Sync(clock) < Async` ✓
  - `dom(buffer) < Async` → `Sync(clock) < Async` ✓
  - `dom(out) < Async` → `Sync(clock) < Async` ✓
- All equality constraints satisfied: ✓
  - All variables in Class 1 have domain `Clock` ✓
  - All variables in Class 2 have domain `Sync(clock)` ✓

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → Clock (concrete, not a parameter)
- `dom(data) = Sync(ComponentId(clock))` → α (the domain parameter)
- `dom(out) = Sync(ComponentId(clock))` → α (the domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
        data: ???,   // α
        out: ???,    // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: Simple,
    component_domains: {
        clock: Clock,
        data: Sync(ComponentId(clock)),
        buffer: Sync(ComponentId(clock)),
        out: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```
