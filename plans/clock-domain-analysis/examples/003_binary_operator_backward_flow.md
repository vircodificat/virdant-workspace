## Example 3: Binary Operator with Backward Flow

```virdant
mod BinaryOp {
    incoming clock : Clock
    incoming inp : Word[8]

    reg r : Word[8] on clock {
        it <= inp + 1
    }
}
```

**What's happening**: A register driven by a binary operation `inp + 1`. The register's domain requirement flows backward through the `+` operator to constrain both operands, including the input port.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `inp` (incoming, type Word[8])
- `r` (reg, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`

Declaration: `incoming inp : Word[8]`
- Component: `inp`

Declaration: `reg r : Word[8] on clock`
- Component: `r`
- Expression `#0`: reference to `clock` (in `on clock` clause)

Driver: `r <= inp + 1`
- Expression `#1`: reference to `inp` (left operand of `+`)
- Expression `#2`: literal `1` (right operand of `+`)
- Expression `#3`: `inp + 1` (the binary operation, RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `inp`
- `#2`: Literal `1`
- `#3`: BinaryOp `+` with operands `#1` and `#2`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(r) = Sync(ComponentId(clock))`

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `inp` → `dom(#1) = dom(inp)`

**Note on literal:**
- Expression `#2` is literal `1` - no constraint (free variable)

**Equality constraints from binary operator:**
- The `+` operator requires all operands to have the same domain as the result
- `dom(#3) = dom(#1)` (result equals left operand)
- `dom(#3) = dom(#2)` (result equals right operand)

**Equality constraint from driver:**
- `dom(r) = dom(#3)` (register driven by the `+` expression)

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(r) = Sync(ComponentId(clock))

Equality:
  C₃: dom(#0) = dom(clock)
  C₄: dom(#1) = dom(inp)
  C₅: dom(#3) = dom(#1)    (+ result = left operand)
  C₆: dom(#3) = dom(#2)    (+ result = right operand)
  C₇: dom(r) = dom(#3)     (driver)
```

---

### Pass 2: Constraint Solving

**Initial state:**
```
{clock}, {inp}, {r}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {inp}, {r}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint C₂: `dom(r) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {inp}, {r: Sync(clock)}, {#0}, {#1}, {#2}, {#3}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**
```
{clock, #0: Clock}, {inp}, {r: Sync(clock)}, {#1}, {#2}, {#3}
```

**Processing equality constraint C₄: `dom(#1) = dom(inp)`**
```
{clock, #0: Clock}, {inp, #1}, {r: Sync(clock)}, {#2}, {#3}
```

**Processing equality constraint C₅: `dom(#3) = dom(#1)`**

Merge #3 with #1's class:
```
{clock, #0: Clock}, {inp, #1, #3}, {r: Sync(clock)}, {#2}
```

**Processing equality constraint C₆: `dom(#3) = dom(#2)`**

Merge #2 with #3's class:
- #3's class: `{inp, #1, #3}` (no concrete domain)
- #2's class: `{#2}` (free variable, no constraint)
```
{clock, #0: Clock}, {inp, #1, #2, #3}, {r: Sync(clock)}
```

**Processing equality constraint C₇: `dom(r) = dom(#3)`**

Merge r's class with #3's class:
- r's class: `{r: Sync(clock)}` (has concrete domain)
- #3's class: `{inp, #1, #2, #3}` (no concrete domain)
- Merged class adopts `Sync(clock)`

```
{clock, #0: Clock}, {inp, #1, #2, #3, r: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {inp, #1, #2, #3, r} → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(inp) = Sync(ComponentId(clock))
  dom(r) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))  (free variable adopted Sync(clock))
  dom(#3) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied ✓
- All equalities satisfied ✓

**Natural language**: The constraint flow was **backward through the expression tree**. The register required `Sync(clock)`, which propagated through the driver constraint to the `+` expression, then through the operator constraints to both operands (`inp` and `1`), finally constraining the input port `inp` to be `Sync(clock)`. The literal `1` started as a free variable and adopted `Sync(clock)` through this chain.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**
- `clock` is Clock-typed → introduces `DomainVar(0)` (α)

**Express port domains in terms of parameters:**
- `dom(clock) = Clock` → Clock
- `dom(inp) = Sync(ComponentId(clock))` → α

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
        inp: ???,    // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: BinaryOp,
    component_domains: {
        clock: Clock,
        inp: Sync(ComponentId(clock)),
        r: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```
