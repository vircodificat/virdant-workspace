## Example 2: Constant Literal

```virdant
mod ConstantExample {
    incoming clock : Clock

    reg counter : Word[8] on clock {
        it <= 0w8
    }
}
```

**What's happening**: A register driven by a constant literal. Literals impose lower bound constraints (at least as specific as Constant), which allows them to unify with any Sync domain.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `counter` (reg, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `reg counter : Word[8] on clock`
- Component: `counter`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Driver: `counter <= 0w8`
- Expression `#1`: the literal `0w8` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: Literal `0w8`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed)

**Fixed constraints from register declarations:**
- `dom(counter) = Sync(ComponentId(clock))` (register with `on clock`)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`

**Note on literal:**
- Expression `#1` is a literal `0w8` with no signal references
- No constraint generated - literals are free variables
- Will be solved through equality with whatever uses them

**Equality constraint from driver:**

Driver: `counter <= 0w8`
- LHS: component `counter`
- RHS: expression `#1`
- Constraint: `dom(counter) = dom(#1)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(counter) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(counter) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  C₃: dom(clock) < Async
  C₄: dom(counter) < Async

Equality:
  C₅: dom(#0) = dom(clock)
  C₆: dom(counter) = dom(#1)
```

---

### Pass 2: Constraint Solving

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {counter}, {#0}, {#1}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {counter}, {#0}, {#1}
```

**Processing fixed constraint C₂: `dom(counter) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {counter: Sync(clock)}, {#0}, {#1}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**

Merge expression #0 with clock:
```
{clock, #0: Clock}, {counter: Sync(clock)}, {#1}
```

**Processing equality constraint C₄: `dom(counter) = dom(#1)`**

Merge counter's class with #1's class:
- counter's class: `{counter: Sync(clock)}` (has concrete domain)
- #1's class: `{#1}` (no concrete domain)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {counter, #1: Sync(clock)}
```

**Candidate solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(counter) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
```

**Verification - Check all constraints:**

Fixed constraints:
- `dom(clock) = Clock` ✓
- `dom(counter) = Sync(clock)` ✓

Strict upper bound constraints:
- `dom(clock) < Async` → `Clock < Async` ✓
- `dom(counter) < Async` → `Sync(clock) < Async` ✓

Equality constraints:
- `dom(#0) = dom(clock)` → both have `Clock` ✓
- `dom(counter) = dom(#1)` → both have `Sync(clock)` ✓

**All constraints satisfied! Solution is valid.**

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {counter, #1} → Sync(ComponentId(clock))
```

**Natural language**: The literal `0w8` (expression #1) had no constraints - it's a free variable. The equality constraint `dom(counter) = dom(#1)` required them to have the same domain. Since `dom(counter) = Sync(clock)`, the solution assigns `dom(#1) = Sync(clock)`. This is why literals "just work" in any clock domain - they impose no constraints and simply adopt whatever domain is needed by their context.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → Clock

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
    }
}
```

Note: `counter` is internal (not a port), so it doesn't appear in the signature.

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: ConstantExample,
    component_domains: {
        clock: Clock,
        counter: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```
