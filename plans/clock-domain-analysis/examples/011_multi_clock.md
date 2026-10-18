# Example 11: Multi-Clock Module with Domain Parameters

```virdant
mod MultiClock {
    incoming clock_a : Clock
    incoming clock_b : Clock

    incoming data_a : Word[8]
    incoming data_b : Word[8]

    reg buffer_a : Word[8] on clock_a {
        it <= data_a
    }

    reg buffer_b : Word[8] on clock_b {
        it <= data_b
    }

    outgoing out_a : Word[8] {
        it := buffer_a
    }

    outgoing out_b : Word[8] {
        it := buffer_b
    }
}
```

**What's happening**: A module with two independent clock domains. Each clock defines a distinct domain parameter. Data flows through separate pipelines, each synchronized to its own clock. This demonstrates how the signature captures multiple independent domain axes.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock_a` (incoming, type Clock)
- `clock_b` (incoming, type Clock)
- `data_a` (incoming, type Word[8])
- `data_b` (incoming, type Word[8])
- `buffer_a` (reg, type Word[8])
- `buffer_b` (reg, type Word[8])
- `out_a` (outgoing, type Word[8])
- `out_b` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock_a : Clock`**
- Component: `clock_a`
- Type: Clock
- No expressions

**Declaration: `incoming clock_b : Clock`**
- Component: `clock_b`
- Type: Clock
- No expressions

**Declaration: `incoming data_a : Word[8]`**
- Component: `data_a`
- No expressions

**Declaration: `incoming data_b : Word[8]`**
- Component: `data_b`
- No expressions

**Declaration: `reg buffer_a : Word[8] on clock_a`**
- Component: `buffer_a`
- Has `on clock_a` clause
- Expression `#0`: reference to `clock_a` (in the `on clock_a` clause)

**Declaration: `reg buffer_b : Word[8] on clock_b`**
- Component: `buffer_b`
- Has `on clock_b` clause
- Expression `#1`: reference to `clock_b` (in the `on clock_b` clause)

**Declaration: `outgoing out_a : Word[8]`**
- Component: `out_a`
- No expressions in declaration

**Declaration: `outgoing out_b : Word[8]`**
- Component: `out_b`
- No expressions in declaration

**Driver: `buffer_a <= data_a`**
- Expression `#2`: reference to `data_a` (RHS of driver)

**Driver: `buffer_b <= data_b`**
- Expression `#3`: reference to `data_b` (RHS of driver)

**Driver: `out_a := buffer_a`**
- Expression `#4`: reference to `buffer_a` (RHS of driver)

**Driver: `out_b := buffer_b`**
- Expression `#5`: reference to `buffer_b` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock_a`
- `#1`: ExprReference to `clock_b`
- `#2`: ExprReference to `data_a`
- `#3`: ExprReference to `data_b`
- `#4`: ExprReference to `buffer_a`
- `#5`: ExprReference to `buffer_b`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock_a) = Clock` (Clock-typed signal)
- `dom(clock_b) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer_a) = Sync(ComponentId(clock_a))` (register with `on clock_a` clause)
- `dom(buffer_b) = Sync(ComponentId(clock_b))` (register with `on clock_b` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock_a` → `dom(#0) = dom(clock_a)`
- Expression `#1` references `clock_b` → `dom(#1) = dom(clock_b)`
- Expression `#2` references `data_a` → `dom(#2) = dom(data_a)`
- Expression `#3` references `data_b` → `dom(#3) = dom(data_b)`
- Expression `#4` references `buffer_a` → `dom(#4) = dom(buffer_a)`
- Expression `#5` references `buffer_b` → `dom(#5) = dom(buffer_b)`

**Equality constraints from drivers:**

Driver: `buffer_a <= data_a`
- LHS: component `buffer_a`
- RHS: expression `#2`
- Constraint: `dom(buffer_a) = dom(#2)`

Driver: `buffer_b <= data_b`
- LHS: component `buffer_b`
- RHS: expression `#3`
- Constraint: `dom(buffer_b) = dom(#3)`

Driver: `out_a := buffer_a`
- LHS: component `out_a`
- RHS: expression `#4`
- Constraint: `dom(out_a) = dom(#4)`

Driver: `out_b := buffer_b`
- LHS: component `out_b`
- RHS: expression `#5`
- Constraint: `dom(out_b) = dom(#5)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock_a) < Async`
- `dom(clock_b) < Async`
- `dom(data_a) < Async`
- `dom(data_b) < Async`
- `dom(buffer_a) < Async`
- `dom(buffer_b) < Async`
- `dom(out_a) < Async`
- `dom(out_b) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock_a) = Clock
  F2: dom(clock_b) = Clock
  F3: dom(buffer_a) = Sync(ComponentId(clock_a))
  F4: dom(buffer_b) = Sync(ComponentId(clock_b))

Strict upper bound (implicit):
  S1: dom(clock_a) < Async
  S2: dom(clock_b) < Async
  S3: dom(data_a) < Async
  S4: dom(data_b) < Async
  S5: dom(buffer_a) < Async
  S6: dom(buffer_b) < Async
  S7: dom(out_a) < Async
  S8: dom(out_b) < Async

Equality:
  E1: dom(#0) = dom(clock_a)
  E2: dom(#1) = dom(clock_b)
  E3: dom(#2) = dom(data_a)
  E4: dom(#3) = dom(data_b)
  E5: dom(#4) = dom(buffer_a)
  E6: dom(#5) = dom(buffer_b)
  E7: dom(buffer_a) = dom(#2)
  E8: dom(buffer_b) = dom(#3)
  E9: dom(out_a) = dom(#4)
  E10: dom(out_b) = dom(#5)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock_a}, {clock_b}, {data_a}, {data_b}, {buffer_a}, {buffer_b}, {out_a}, {out_b},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F1: `dom(clock_a) = Clock`

Assign concrete domain `Clock` to clock_a's equivalence class:
```
{clock_a: Clock}, {clock_b}, {data_a}, {data_b}, {buffer_a}, {buffer_b}, {out_a}, {out_b},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F2: `dom(clock_b) = Clock`

Assign concrete domain `Clock` to clock_b's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {data_a}, {data_b}, {buffer_a}, {buffer_b}, {out_a}, {out_b},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F3: `dom(buffer_a) = Sync(ComponentId(clock_a))`

Assign concrete domain `Sync(clock_a)` to buffer_a's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {data_a}, {data_b}, {buffer_a: Sync(clock_a)},
{buffer_b}, {out_a}, {out_b}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F4: `dom(buffer_b) = Sync(ComponentId(clock_b))`

Assign concrete domain `Sync(clock_b)` to buffer_b's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {data_a}, {data_b}, {buffer_a: Sync(clock_a)},
{buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing equality constraint E1: `dom(#0) = dom(clock_a)`

Merge expression #0 with clock_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b: Clock}, {data_a}, {data_b}, {buffer_a: Sync(clock_a)},
{buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing equality constraint E2: `dom(#1) = dom(clock_b)`

Merge expression #1 with clock_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a}, {data_b}, {buffer_a: Sync(clock_a)},
{buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#2}, {#3}, {#4}, {#5}
```

### Processing equality constraint E3: `dom(#2) = dom(data_a)`

Merge expression #2 with data_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2}, {data_b}, {buffer_a: Sync(clock_a)},
{buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#3}, {#4}, {#5}
```

### Processing equality constraint E4: `dom(#3) = dom(data_b)`

Merge expression #3 with data_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2}, {data_b, #3}, {buffer_a: Sync(clock_a)},
{buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#4}, {#5}
```

### Processing equality constraint E5: `dom(#4) = dom(buffer_a)`

Merge expression #4 with buffer_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2}, {data_b, #3},
{buffer_a, #4: Sync(clock_a)}, {buffer_b: Sync(clock_b)}, {out_a}, {out_b}, {#5}
```

### Processing equality constraint E6: `dom(#5) = dom(buffer_b)`

Merge expression #5 with buffer_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2}, {data_b, #3},
{buffer_a, #4: Sync(clock_a)}, {buffer_b, #5: Sync(clock_b)}, {out_a}, {out_b}
```

### Processing equality constraint E7: `dom(buffer_a) = dom(#2)`

Merge buffer_a's class with #2's class:
- buffer_a's class: `{buffer_a, #4: Sync(clock_a)}` (has concrete domain)
- #2's class: `{data_a, #2}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock_a)`

```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2, buffer_a, #4: Sync(clock_a)},
{data_b, #3}, {buffer_b, #5: Sync(clock_b)}, {out_a}, {out_b}
```

### Processing equality constraint E8: `dom(buffer_b) = dom(#3)`

Merge buffer_b's class with #3's class:
- buffer_b's class: `{buffer_b, #5: Sync(clock_b)}` (has concrete domain)
- #3's class: `{data_b, #3}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock_b)`

```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2, buffer_a, #4: Sync(clock_a)},
{data_b, #3, buffer_b, #5: Sync(clock_b)}, {out_a}, {out_b}
```

### Processing equality constraint E9: `dom(out_a) = dom(#4)`

Merge out_a with #4's class:
- #4's class: `{data_a, #2, buffer_a, #4: Sync(clock_a)}`
- out_a's class: `{out_a}` (no concrete domain)
- Merged class keeps `Sync(clock_a)`

```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2, buffer_a, #4, out_a: Sync(clock_a)},
{data_b, #3, buffer_b, #5: Sync(clock_b)}, {out_b}
```

### Processing equality constraint E10: `dom(out_b) = dom(#5)`

Merge out_b with #5's class:
- #5's class: `{data_b, #3, buffer_b, #5: Sync(clock_b)}`
- out_b's class: `{out_b}` (no concrete domain)
- Merged class keeps `Sync(clock_b)`

```
{clock_a, #0: Clock}, {clock_b, #1: Clock}, {data_a, #2, buffer_a, #4, out_a: Sync(clock_a)},
{data_b, #3, buffer_b, #5, out_b: Sync(clock_b)}
```

### Final partition:
```
Class 1: {clock_a, #0} → Clock
Class 2: {clock_b, #1} → Clock
Class 3: {data_a, #2, buffer_a, #4, out_a} → Sync(ComponentId(clock_a))
Class 4: {data_b, #3, buffer_b, #5, out_b} → Sync(ComponentId(clock_b))
```

### Solution (domain assignment):
```
Components:
  dom(clock_a) = Clock
  dom(clock_b) = Clock
  dom(data_a) = Sync(ComponentId(clock_a))
  dom(data_b) = Sync(ComponentId(clock_b))
  dom(buffer_a) = Sync(ComponentId(clock_a))
  dom(buffer_b) = Sync(ComponentId(clock_b))
  dom(out_a) = Sync(ComponentId(clock_a))
  dom(out_b) = Sync(ComponentId(clock_b))

Expressions:
  dom(#0) = Clock
  dom(#1) = Clock
  dom(#2) = Sync(ComponentId(clock_a))
  dom(#3) = Sync(ComponentId(clock_b))
  dom(#4) = Sync(ComponentId(clock_a))
  dom(#5) = Sync(ComponentId(clock_b))
```

### Verification:

**All fixed constraints satisfied:** ✓
- `dom(clock_a) = Clock` ✓
- `dom(clock_b) = Clock` ✓
- `dom(buffer_a) = Sync(clock_a)` ✓
- `dom(buffer_b) = Sync(clock_b)` ✓

**All strict upper bound constraints satisfied:** ✓
- `dom(clock_a) < Async` → `Clock < Async` ✓
- `dom(clock_b) < Async` → `Clock < Async` ✓
- `dom(data_a) < Async` → `Sync(clock_a) < Async` ✓
- `dom(data_b) < Async` → `Sync(clock_b) < Async` ✓
- `dom(buffer_a) < Async` → `Sync(clock_a) < Async` ✓
- `dom(buffer_b) < Async` → `Sync(clock_b) < Async` ✓
- `dom(out_a) < Async` → `Sync(clock_a) < Async` ✓
- `dom(out_b) < Async` → `Sync(clock_b) < Async` ✓

**All equality constraints satisfied:** ✓
- All variables in Class 1 have domain `Clock` ✓
- All variables in Class 2 have domain `Clock` ✓
- All variables in Class 3 have domain `Sync(clock_a)` ✓
- All variables in Class 4 have domain `Sync(clock_b)` ✓

**Key observation:** The two clock domains (`Sync(clock_a)` and `Sync(clock_b)`) remain completely separate throughout solving. They never try to unify with each other because the module carefully keeps the two data paths isolated. This is the correct behavior for a multi-clock design.

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock_a` is Clock-typed → introduces `DomainVar(0)` (call it α)
- `clock_b` is Clock-typed → introduces `DomainVar(1)` (call it β)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock_a) = Clock` → concrete `Clock` (not a domain parameter itself)
- `dom(clock_b) = Clock` → concrete `Clock` (not a domain parameter itself)
- `dom(data_a) = Sync(ComponentId(clock_a))` → α (domain defined by clock_a)
- `dom(data_b) = Sync(ComponentId(clock_b))` → β (domain defined by clock_b)
- `dom(out_a) = Sync(ComponentId(clock_a))` → α (domain defined by clock_a)
- `dom(out_b) = Sync(ComponentId(clock_b))` → β (domain defined by clock_b)

### Signature:
```
DomainSignature {
    domain_params: {
        clock_a: DomainVar(0),  // α
        clock_b: DomainVar(1),  // β
    },
    port_domains: {
        clock_a: Concrete(Clock),
        clock_b: Concrete(Clock),
        data_a: Var(α),
        data_b: Var(β),
        out_a: Var(α),
        out_b: Var(β),
    }
}
```

### Interpretation at instantiation:

When this module is instantiated:
```virdant
mod Parent {
    incoming parent_clk_x : Clock
    incoming parent_clk_y : Clock
    incoming some_data_x : Word[8]
    incoming some_data_y : Word[8]

    mod multi of MultiClock {
        it.clock_a := parent_clk_x    // binds α → Sync(parent_clk_x)
        it.clock_b := parent_clk_y    // binds β → Sync(parent_clk_y)
        it.data_a := some_data_x      // requires: dom(some_data_x) = α = Sync(parent_clk_x)
        it.data_b := some_data_y      // requires: dom(some_data_b) = β = Sync(parent_clk_y)
    }
}
```

The instantiation checker:
1. Establishes bindings: α → `Sync(parent_clk_x)`, β → `Sync(parent_clk_y)`
2. Checks data connections:
   - `data_a` expects α = `Sync(parent_clk_x)` → `some_data_x` must be on this domain ✓
   - `data_b` expects β = `Sync(parent_clk_y)` → `some_data_y` must be on this domain ✓

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: MultiClock,
    component_domains: {
        clock_a: Clock,
        clock_b: Clock,
        data_a: Sync(ComponentId(clock_a)),
        data_b: Sync(ComponentId(clock_b)),
        buffer_a: Sync(ComponentId(clock_a)),
        buffer_b: Sync(ComponentId(clock_b)),
        out_a: Sync(ComponentId(clock_a)),
        out_b: Sync(ComponentId(clock_b)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes, remarkably well.** The algorithm handles multiple clock domains with no special cases:

1. **Each clock port creates a distinct domain**: The fixed constraints establish `Sync(clock_a)` and `Sync(clock_b)` as distinct, non-unifiable domains. ComponentId distinguishes them.

2. **Separate equivalence classes naturally emerge**: During constraint solving, the algorithm partitions components into four distinct classes (two for clocks, two for data paths). The union-find naturally keeps the two clock domains separate because there are no equality constraints crossing between them.

3. **Multiple domain parameters in the signature**: The signature extraction step naturally generates two domain variables (α and β), one for each clock port. No special machinery needed.

4. **Domain substitution at instantiation scales**: The binding mechanism works identically for 1, 2, or N domain parameters. Each clock connection establishes a binding, and data port checking applies all bindings uniformly.

### Any edge cases or design issues discovered?

1. **Clock ComponentIds are module-local**: Critical that ComponentId distinguishes clocks *within a module*. Two modules each with a `clock_a` will have different ComponentIds, preventing accidental domain confusion. This is already implicit in ComponentAnalysis but worth documenting explicitly.

2. **Clock-typed ports are not themselves on the domain they define**: Subtle but important: `dom(clock_a) = Clock`, not `Sync(clock_a)`. The clock *defines* the domain but isn't *on* the domain. This is correctly handled by the fixed constraint rule for Clock-typed components.

3. **Zero clock modules**: A module with no clocks (all async or constant) would have zero domain parameters. The algorithm handles this: no clock ports → no domain variables → all ports are either Async, Constant, or free variables. This is correct.

4. **Single-clock vs multi-clock**: No algorithmic difference. A single-clock module is just the N=1 case. The algorithm is naturally polymorphic over the number of clocks.

5. **Intentional domain mixing would be caught**: If someone tried to write `wire w : Bit { it := data_a | data_b }`, this would generate an equality `Sync(clock_a) = Sync(clock_b)`, which would fail during solving. The error message could be improved to say "mixing different clock domains" rather than just "unification error".

### Any clarifications needed for PLANv2.md or ALGORITHM.md?

**For PLANv2.md:**

1. **Make explicit**: "Each Clock-typed port in a module defines a distinct domain parameter, even if those clocks might be connected to the same signal at instantiation." This is implied but worth stating clearly.

2. **Example needed**: PLANv2.md describes the mechanism but doesn't show a multi-clock example. This example (011) would be a good addition to the plan document.

**For ALGORITHM.md:**

1. **Domain variables are per clock port**: Should explicitly state that the number of domain variables equals the number of Clock-typed ports. This is clear from the signature extraction pseudocode but not stated upfront.

2. **ComponentId scoping**: Worth mentioning that ComponentId distinguishes clocks within a module, and that `Sync(ComponentId(5))` in module A is a different domain from `Sync(ComponentId(5))` in module B (they're in different namespaces).

3. **Error messaging opportunities**: When unification fails between two `Sync` domains with different ComponentIds, the error could be specialized to "cannot mix clock domains" rather than generic "unification error". The algorithm supports this because it can inspect the Domain values.

4. **Zero-clock case**: Worth explicitly noting that modules with no Clock ports have signature with empty domain_params. This is a valid case (all-async or all-constant modules).

### Implementation confidence

This example demonstrates that **the constraint-based algorithm naturally scales to multiple clock domains** without any special cases. The union-find partitioning, combined with ComponentId-distinguished Sync domains, provides exactly the right behavior:

- Independent clock domains stay separate
- Attempts to mix them are caught as unification errors
- Signature extraction naturally produces N domain parameters for N clocks
- Instantiation checking scales uniformly

The algorithm is well-designed for multi-clock hardware.
