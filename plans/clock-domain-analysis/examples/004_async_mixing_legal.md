## Example 4: Mixing Clocks with Explicit Async (LEGAL)

```virdant
mod AsyncMixing {
    incoming clk_a : Clock
    incoming clk_b : Clock

    reg r_a : Bit on clk_a {
        it <= false
    }

    reg r_b : Bit on clk_b {
        it <= false
    }

    wire mixed : Bit async {
        it := r_a | r_b
    }
}
```

**What's happening**: Mixing signals from two different clock domains with explicit `async` target. The unified binary operator rule applies: operands can upcast to the result domain.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clk_a` (incoming, type Clock)
- `clk_b` (incoming, type Clock)
- `r_a` (reg, type Bit)
- `r_b` (reg, type Bit)
- `mixed` (wire, type Bit, declared async)

**Step 2 - Walk declarations, identify expressions:**

Declaration: `reg r_a : Bit on clk_a`
- Component: `r_a`
- Expression `#0`: reference to `clk_a` (in `on clk_a`)

Driver: `r_a <= false`
- Expression `#1`: literal `false`

Declaration: `reg r_b : Bit on clk_b`
- Component: `r_b`
- Expression `#2`: reference to `clk_b` (in `on clk_b`)

Driver: `r_b <= false`
- Expression `#3`: literal `false`

Declaration: `wire mixed : Bit async`
- Component: `mixed` (explicitly async)

Driver: `mixed := r_a | r_b`
- Expression `#4`: reference to `r_a` (left operand)
- Expression `#5`: reference to `r_b` (right operand)
- Expression `#6`: `r_a | r_b` (the binary operation)

**Expression inventory:**
- `#0`: ExprReference to `clk_a`
- `#1`: Literal `false`
- `#2`: ExprReference to `clk_b`
- `#3`: Literal `false`
- `#4`: ExprReference to `r_a`
- `#5`: ExprReference to `r_b`
- `#6`: BinaryOp `|`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clk_a) = Clock`
- `dom(clk_b) = Clock`

**Fixed constraints from register declarations:**
- `dom(r_a) = Sync(ComponentId(clk_a))`
- `dom(r_b) = Sync(ComponentId(clk_b))`

**Fixed constraints from async declarations:**
- `dom(mixed) = Async` (explicitly declared async)

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clk_a) < Async`
- `dom(clk_b) < Async`
- `dom(r_a) < Async`
- `dom(r_b) < Async`
- (mixed is declared async, so NO implicit constraint)

**Equality constraints from component references:**
- `dom(#0) = dom(clk_a)`
- `dom(#2) = dom(clk_b)`
- `dom(#4) = dom(r_a)`
- `dom(#5) = dom(r_b)`

**Equality constraints from drivers (non-async):**
- `dom(r_a) = dom(#1)`
- `dom(r_b) = dom(#3)`

**Upper bound constraints from binary operator (UNIFIED RULE):**
- `dom(#4) ≤ dom(#6)` (left operand can upcast to result)
- `dom(#5) ≤ dom(#6)` (right operand can upcast to result)

**Equality constraint from driver:**
- `dom(mixed) = dom(#6)` (wire driven by the `|` expression)

**Complete constraint system:**
```
Fixed:
  C₁: dom(clk_a) = Clock
  C₂: dom(clk_b) = Clock
  C₃: dom(r_a) = Sync(ComponentId(clk_a))
  C₄: dom(r_b) = Sync(ComponentId(clk_b))
  C₅: dom(mixed) = Async

Strict upper bound (implicit):
  C₆: dom(clk_a) < Async
  C₇: dom(clk_b) < Async
  C₈: dom(r_a) < Async
  C₉: dom(r_b) < Async

Equality:
  C₁₀: dom(#0) = dom(clk_a)
  C₁₁: dom(#2) = dom(clk_b)
  C₁₂: dom(#4) = dom(r_a)
  C₁₃: dom(#5) = dom(r_b)
  C₁₄: dom(r_a) = dom(#1)
  C₁₅: dom(r_b) = dom(#3)

Upper bound (from | operator):
  C₁₆: dom(#4) ≤ dom(#6)
  C₁₇: dom(#5) ≤ dom(#6)

Equality (from driver):
  C₁₈: dom(mixed) = dom(#6)
```

---

### Pass 2: Constraint Solving

**Initial state:**
```
{clk_a}, {clk_b}, {r_a}, {r_b}, {mixed}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraints C₁-C₅:**
```
{clk_a: Clock}, {clk_b: Clock}, {r_a: Sync(clk_a)}, {r_b: Sync(clk_b)}, {mixed: Async},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing equality constraints C₁₀-C₁₅:**

After C₁₀: `{clk_a, #0: Clock}, ...`
After C₁₁: `{clk_b, #2: Clock}, ...`
After C₁₂: `{r_a, #4: Sync(clk_a)}, ...`
After C₁₃: `{r_b, #5: Sync(clk_b)}, ...`
After C₁₄: `{r_a, #1, #4: Sync(clk_a)}, ...`
After C₁₅: `{r_b, #3, #5: Sync(clk_b)}, ...`

Current state:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)},
{r_b, #3, #5: Sync(clk_b)}, {mixed: Async}, {#6}
```

**Processing equality constraint C₁₈: `dom(mixed) = dom(#6)`**

Merge #6 with mixed:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)},
{r_b, #3, #5: Sync(clk_b)}, {mixed, #6: Async}
```

**Candidate solution:**
```
Components:
  dom(clk_a) = Clock
  dom(clk_b) = Clock
  dom(r_a) = Sync(ComponentId(clk_a))
  dom(r_b) = Sync(ComponentId(clk_b))
  dom(mixed) = Async

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clk_a))
  dom(#2) = Clock
  dom(#3) = Sync(ComponentId(clk_b))
  dom(#4) = Sync(ComponentId(clk_a))
  dom(#5) = Sync(ComponentId(clk_b))
  dom(#6) = Async
```

**Verification - Check all constraints:**

Fixed constraints: ✓

Strict upper bound constraints:
- `dom(clk_a) < Async` → `Clock < Async` ✓
- `dom(clk_b) < Async` → `Clock < Async` ✓
- `dom(r_a) < Async` → `Sync(clk_a) < Async` ✓
- `dom(r_b) < Async` → `Sync(clk_b) < Async` ✓

Equality constraints: ✓

Upper bound constraints (from | operator):
- `dom(#4) ≤ dom(#6)` → `Sync(clk_a) ≤ Async` ✓ (Sync can upcast to Async in lattice)
- `dom(#5) ≤ dom(#6)` → `Sync(clk_b) ≤ Async` ✓ (Sync can upcast to Async in lattice)

**All constraints satisfied! Solution is valid.**

**Natural language**: The unified binary operator rule generates upper bound constraints `dom(operand) ≤ dom(result)`. Since the result ended up being Async (from the driver), both operands (Sync(clk_a) and Sync(clk_b)) can upcast to Async, satisfying `Sync(clk_a) ≤ Async` and `Sync(clk_b) ≤ Async`. The two different clock domains never needed to unify with each other - they just both upcast to the common Async result.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**
- `clk_a` introduces `DomainVar(0)` (α)
- `clk_b` introduces `DomainVar(1)` (β)

**Express port domains:**
- `dom(clk_a) = Clock` → Clock
- `dom(clk_b) = Clock` → Clock
- `dom(mixed) = Async` → Async

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clk_a: DomainVar(0),  // α
        clk_b: DomainVar(1),  // β
    },
    port_domains: {
        clk_a: ???,   // Clock
        clk_b: ???,   // Clock
        mixed: ???,   // Async
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: AsyncMixing,
    component_domains: {
        clk_a: Clock,
        clk_b: Clock,
        r_a: Sync(ComponentId(clk_a)),
        r_b: Sync(ComponentId(clk_b)),
        mixed: Async,
    },
    expression_domains: {},
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```
