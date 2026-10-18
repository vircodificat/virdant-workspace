## Example 19: Extension Functions (zext/sext) Domain Preservation

```virdant
mod ExtensionTest {
    incoming clock : Clock
    incoming narrow_data : Word[4]

    reg result_zext : Word[8] on clock {
        it <= zext(narrow_data)
    }

    outgoing result_sext : Word[8] {
        it := sext(narrow_data)
    }

    wire const_extended : Word[16] {
        it := zext(0xaaw8)
    }
}
```

**What's happening**: This module extends narrow signals to wider widths using `zext()` and `sext()`. The key question is whether these unary extension operations preserve the source signal's domain.

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `narrow_data` (incoming, type Word[4])
- `result_zext` (reg, type Word[8])
- `result_sext` (outgoing, type Word[8])
- `const_extended` (wire, type Word[16])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming narrow_data : Word[4]`
- Component: `narrow_data`
- No expressions

Declaration: `reg result_zext : Word[8] on clock`
- Component: `result_zext`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing result_sext : Word[8]`
- Component: `result_sext`
- No expressions in declaration

Declaration: `wire const_extended : Word[16]`
- Component: `const_extended`
- No expressions in declaration

Driver: `result_zext <= zext(narrow_data)`
- Expression `#1`: `zext(narrow_data)`
  - This is a unary operation (zext)
  - Expression `#2`: reference to `narrow_data` (subject of zext)

Driver: `result_sext := sext(narrow_data)`
- Expression `#3`: `sext(narrow_data)`
  - This is a unary operation (sext)
  - Expression `#4`: reference to `narrow_data` (subject of sext)

Driver: `const_extended := zext(0xaaw8)`
- Expression `#5`: `zext(0xaaw8)`
  - This is a unary operation (zext)
  - Expression `#6`: literal `0xaaw8`

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprZext with subject `#2`
- `#2`: ExprReference to `narrow_data`
- `#3`: ExprSext with subject `#4`
- `#4`: ExprReference to `narrow_data`
- `#5`: ExprZext with subject `#6`
- `#6`: ExprLiteral `0xaaw8`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(result_zext) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Fixed constraints from literal expressions:**
- `dom(#6) = Constant` (literal value)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#2` references `narrow_data` → `dom(#2) = dom(narrow_data)`
- Expression `#4` references `narrow_data` → `dom(#4) = dom(narrow_data)`

**Equality constraints from unary operations (zext/sext):**

For unary operations like zext/sext, the result has the same domain as the operand:

- Expression `#1` is `zext(#2)` → `dom(#1) = dom(#2)`
- Expression `#3` is `sext(#4)` → `dom(#3) = dom(#4)`
- Expression `#5` is `zext(#6)` → `dom(#5) = dom(#6)`

**Equality constraints from drivers:**

Driver: `result_zext <= zext(narrow_data)`
- LHS: component `result_zext`
- RHS: expression `#1`
- Constraint: `dom(result_zext) = dom(#1)`

Driver: `result_sext := sext(narrow_data)`
- LHS: component `result_sext`
- RHS: expression `#3`
- Constraint: `dom(result_sext) = dom(#3)`

Driver: `const_extended := zext(0xaaw8)`
- LHS: component `const_extended`
- RHS: expression `#5`
- Constraint: `dom(const_extended) = dom(#5)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(narrow_data) < Async`
- `dom(result_zext) < Async`
- `dom(result_sext) < Async`
- `dom(const_extended) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(result_zext) = Sync(ComponentId(clock))
  C₃: dom(#6) = Constant

Strict upper bound (implicit):
  C₄: dom(clock) < Async
  C₅: dom(narrow_data) < Async
  C₆: dom(result_zext) < Async
  C₇: dom(result_sext) < Async
  C₈: dom(const_extended) < Async

Equality (from references):
  C₉: dom(#0) = dom(clock)
  C₁₀: dom(#2) = dom(narrow_data)
  C₁₁: dom(#4) = dom(narrow_data)

Equality (from unary operations):
  C₁₂: dom(#1) = dom(#2)
  C₁₃: dom(#3) = dom(#4)
  C₁₄: dom(#5) = dom(#6)

Equality (from drivers):
  C₁₅: dom(result_zext) = dom(#1)
  C₁₆: dom(result_sext) = dom(#3)
  C₁₇: dom(const_extended) = dom(#5)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {narrow_data}, {result_zext}, {result_sext}, {const_extended},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**

Assign concrete domain `Clock` to clock's equivalence class:
```
{clock: Clock}, {narrow_data}, {result_zext}, {result_sext}, {const_extended},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraint C₂: `dom(result_zext) = Sync(ComponentId(clock))`**

Assign concrete domain `Sync(clock)` to result_zext's equivalence class:
```
{clock: Clock}, {narrow_data}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraint C₃: `dom(#6) = Constant`**

Assign concrete domain `Constant` to #6's equivalence class:
```
{clock: Clock}, {narrow_data}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6: Constant}
```

**Processing equality constraint C₉: `dom(#0) = dom(clock)`**

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {narrow_data}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#1}, {#2}, {#3}, {#4}, {#5}, {#6: Constant}
```

**Processing equality constraint C₁₀: `dom(#2) = dom(narrow_data)`**

Merge expression #2 with narrow_data's equivalence class:
```
{clock, #0: Clock}, {narrow_data, #2}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#1}, {#3}, {#4}, {#5}, {#6: Constant}
```

**Processing equality constraint C₁₁: `dom(#4) = dom(narrow_data)`**

Merge expression #4 with narrow_data's equivalence class:
```
{clock, #0: Clock}, {narrow_data, #2, #4}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#1}, {#3}, {#5}, {#6: Constant}
```

**Processing equality constraint C₁₂: `dom(#1) = dom(#2)`**

Merge expression #1 with #2's equivalence class (narrow_data's class):
```
{clock, #0: Clock}, {narrow_data, #1, #2, #4}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#3}, {#5}, {#6: Constant}
```

**Processing equality constraint C₁₃: `dom(#3) = dom(#4)`**

Merge expression #3 with #4's equivalence class (narrow_data's class):
```
{clock, #0: Clock}, {narrow_data, #1, #2, #3, #4}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#5}, {#6: Constant}
```

**Processing equality constraint C₁₄: `dom(#5) = dom(#6)`**

Merge expression #5 with #6's equivalence class:
```
{clock, #0: Clock}, {narrow_data, #1, #2, #3, #4}, {result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#5, #6: Constant}
```

**Processing equality constraint C₁₅: `dom(result_zext) = dom(#1)`**

Merge result_zext's class with #1's class:
- result_zext's class: `{result_zext: Sync(clock)}` (has concrete domain)
- #1's class: `{narrow_data, #1, #2, #3, #4}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {narrow_data, #1, #2, #3, #4, result_zext: Sync(clock)}, {result_sext}, {const_extended},
{#5, #6: Constant}
```

**Processing equality constraint C₁₆: `dom(result_sext) = dom(#3)`**

Merge result_sext with #3's class:
- #3's class: `{narrow_data, #1, #2, #3, #4, result_zext: Sync(clock)}`
- result_sext's class: `{result_sext}` (no concrete domain)
- Merged class keeps `Sync(clock)`

```
{clock, #0: Clock}, {narrow_data, #1, #2, #3, #4, result_zext, result_sext: Sync(clock)}, {const_extended},
{#5, #6: Constant}
```

**Processing equality constraint C₁₇: `dom(const_extended) = dom(#5)`**

Merge const_extended with #5's class:
- #5's class: `{#5, #6: Constant}`
- const_extended's class: `{const_extended}` (no concrete domain)
- Merged class keeps `Constant`

```
{clock, #0: Clock}, {narrow_data, #1, #2, #3, #4, result_zext, result_sext: Sync(clock)},
{const_extended, #5, #6: Constant}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {narrow_data, #1, #2, #3, #4, result_zext, result_sext} → Sync(ComponentId(clock))
Class 3: {const_extended, #5, #6} → Constant
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(narrow_data) = Sync(ComponentId(clock))
  dom(result_zext) = Sync(ComponentId(clock))
  dom(result_sext) = Sync(ComponentId(clock))
  dom(const_extended) = Constant

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))  // zext preserves Sync domain
  dom(#2) = Sync(ComponentId(clock))
  dom(#3) = Sync(ComponentId(clock))  // sext preserves Sync domain
  dom(#4) = Sync(ComponentId(clock))
  dom(#5) = Constant                   // zext preserves Constant domain
  dom(#6) = Constant
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(result_zext) = Sync(clock)` ✓
  - `dom(#6) = Constant` ✓
- All strict upper bound constraints satisfied: ✓
  - `dom(clock) < Async` → `Clock < Async` ✓
  - `dom(narrow_data) < Async` → `Sync(clock) < Async` ✓
  - `dom(result_zext) < Async` → `Sync(clock) < Async` ✓
  - `dom(result_sext) < Async` → `Sync(clock) < Async` ✓
  - `dom(const_extended) < Async` → `Constant < Async` ✓
- All equality constraints satisfied: ✓
  - All variables in Class 1 have domain `Clock` ✓
  - All variables in Class 2 have domain `Sync(clock)` ✓
  - All variables in Class 3 have domain `Constant` ✓

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → Clock (concrete, not a parameter)
- `dom(narrow_data) = Sync(ComponentId(clock))` → α (the domain parameter)
- `dom(result_sext) = Sync(ComponentId(clock))` → α (the domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        narrow_data: Var(DomainVar(0)),      // α
        result_sext: Var(DomainVar(0)),      // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: ExtensionTest,
    component_domains: {
        clock: Clock,
        narrow_data: Sync(ComponentId(clock)),
        result_zext: Sync(ComponentId(clock)),
        result_sext: Sync(ComponentId(clock)),
        const_extended: Constant,
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

# Notes

## Does this work naturally with the algorithm as-is?

**Yes, perfectly.** Extension operations (`zext`, `sext`) are handled as unary operations with a single subject expression. The constraint generation naturally produces:

```
dom(result) = dom(subject)
```

This is exactly the correct constraint: extension operations preserve the domain of their operand. There's no need for any special-case handling.

## Comparison with other unary operations

Extension operations behave identically to field access and array indexing from a domain perspective:

- **Field access**: `struct.field` → `dom(result) = dom(struct)`
- **Array indexing**: `array[index]` → `dom(result) = dom(array)` (and `dom(index) = dom(array)`)
- **Extension**: `zext(subject)` → `dom(result) = dom(subject)`

All unary "projection" operations preserve domain.

## Edge cases handled correctly

1. **Clocked signal extended** → Result preserves `Sync(clock)` domain ✓
   - `zext(narrow_data)` has domain `Sync(clock)` when `narrow_data` is `Sync(clock)`

2. **Constant extended** → Result preserves `Constant` domain ✓
   - `zext(0xaaw8)` has domain `Constant` (pure combinational)

3. **Extension result used in different contexts** → Unification works naturally ✓
   - Extended value can be assigned to register (must match clock)
   - Extended value can be assigned to wire (follows backward propagation)

## Domain flow through extension chains

If you chain extensions: `zext(zext(x))`, each level preserves domain:
- `dom(inner_zext) = dom(x)`
- `dom(outer_zext) = dom(inner_zext) = dom(x)`

The constraint system handles this naturally with transitive equality.

## ALGORITHM.md requirements

**Constraint generation for unary operations:**

The algorithm should handle `zext` and `sext` analogously to `FieldAccess`:

```python
elif expr.kind == Zext or expr.kind == Sext:
    expr_id, subject_expr = walk_expression(expr.subject, &mut variables,
                                           &mut equalities, &mut upper_bounds, expr_id)

    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)

    # Result has same domain as subject
    equalities.append(Equality(result_expr, subject_expr))

    return (expr_id, result_expr)
```

This is the **same pattern** as field access and array element access. The algorithm already has this structure; extension operations just need to be added to the cases.

## Does the constraint system handle this without special cases?

**Yes, completely.** Extension operations are pure domain-preserving unary projections. They:

- Don't introduce new clocks (unlike registers)
- Don't require upcasting rules (unlike async targets)
- Don't have multiple operands to unify (unlike binary operators)
- Don't have control flow (unlike `match`/`when`)

The single equality constraint `dom(result) = dom(subject)` is all that's needed.

## Clarifications needed for ALGORITHM.md?

**Add extension operations to `walk_expression`:**

The current ALGORITHM.md doesn't list `zext`/`sext` as expression kinds. They should be added alongside `FieldAccess` as unary domain-preserving operations:

```python
# Unary domain-preserving operations (field access, extension, etc.)
elif expr.kind in [FieldAccess, Zext, Sext]:
    expr_id, subject_expr = walk_expression(expr.subject, &mut variables,
                                           &mut equalities, &mut upper_bounds, expr_id)
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    equalities.append(Equality(result_expr, subject_expr))
    return (expr_id, result_expr)
```

**Natural category:** These belong in the same category as field access and array indexing—operations that project or transform a value while preserving its clocking/synchronization domain.

## Surprises or edge cases?

**None.** Extension operations are remarkably straightforward for domain analysis:

- They're pure unary operations
- They preserve domain perfectly
- They compose naturally with other operations
- No special cases or corner cases discovered

This is exactly what you'd expect: changing the width of a signal doesn't change when it's valid or what clock it's synchronized to.

## Implementation note

Extension operations also don't affect the lattice operations. Unlike binary operators (which may need LUB computation for mismatched operands), extensions simply propagate their single operand's domain with no transformation.

This makes them one of the simplest expression kinds for domain analysis, on par with component references.
