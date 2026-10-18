## Example 17: Bit Slicing Domain Preservation

```virdant
mod BitSliceExample {
    incoming clock : Clock
    incoming data : Word[8]

    reg buffer : Word[8] on clock {
        it <= data
    }

    outgoing lower : Word[3] {
        it := buffer[4..1]   // Extract bits [3:1] (3 bits)
    }

    outgoing upper : Word[4] {
        it := buffer[8..4]   // Extract bits [7:4] (4 bits)
    }

    outgoing middle : Bit {
        it := buffer[5]      // Extract single bit [4]
    }
}
```

**What's happening**: A register stores 8-bit data on a clock domain. We extract different bit ranges and individual bits from the register. All slicing operations should preserve the register's `Sync(clock)` domain since they're pure projections.

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `buffer` (reg, type Word[8])
- `lower` (outgoing, type Word[3])
- `upper` (outgoing, type Word[4])
- `middle` (outgoing, type Bit)

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

Declaration: `outgoing lower : Word[3]`
- Component: `lower`
- No expressions in declaration

Declaration: `outgoing upper : Word[4]`
- Component: `upper`
- No expressions in declaration

Declaration: `outgoing middle : Bit`
- Component: `middle`
- No expressions in declaration

Driver: `buffer <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Driver: `lower := buffer[4..1]`
- Expression `#2`: bit slice `buffer[4..1]`
  - Expression `#3`: reference to `buffer` (source of slice)

Driver: `upper := buffer[8..4]`
- Expression `#4`: bit slice `buffer[8..4]`
  - Expression `#5`: reference to `buffer` (source of slice)

Driver: `middle := buffer[5]`
- Expression `#6`: bit index `buffer[5]`
  - Expression `#7`: reference to `buffer` (source of index)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprBitSlice of `buffer[4..1]`
- `#3`: ExprReference to `buffer` (within #2)
- `#4`: ExprBitSlice of `buffer[8..4]`
- `#5`: ExprReference to `buffer` (within #4)
- `#6`: ExprBitIndex of `buffer[5]`
- `#7`: ExprReference to `buffer` (within #6)

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `data` → `dom(#1) = dom(data)`
- Expression `#3` references `buffer` → `dom(#3) = dom(buffer)`
- Expression `#5` references `buffer` → `dom(#5) = dom(buffer)`
- Expression `#7` references `buffer` → `dom(#7) = dom(buffer)`

**Equality constraints from bit slicing (projection rule):**

Bit slice `buffer[4..1]` (expression `#2`):
- Result has same domain as source
- Constraint: `dom(#2) = dom(#3)`

Bit slice `buffer[8..4]` (expression `#4`):
- Result has same domain as source
- Constraint: `dom(#4) = dom(#5)`

Bit index `buffer[5]` (expression `#6`):
- Result has same domain as source
- Constraint: `dom(#6) = dom(#7)`

**Equality constraints from drivers:**

Driver: `buffer <= data`
- LHS: component `buffer`
- RHS: expression `#1`
- Constraint: `dom(buffer) = dom(#1)`

Driver: `lower := buffer[4..1]`
- LHS: component `lower`
- RHS: expression `#2`
- Constraint: `dom(lower) = dom(#2)`

Driver: `upper := buffer[8..4]`
- LHS: component `upper`
- RHS: expression `#4`
- Constraint: `dom(upper) = dom(#4)`

Driver: `middle := buffer[5]`
- LHS: component `middle`
- RHS: expression `#6`
- Constraint: `dom(middle) = dom(#6)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(buffer) < Async`
- `dom(lower) < Async`
- `dom(upper) < Async`
- `dom(middle) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(buffer) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  C₃: dom(clock) < Async
  C₄: dom(data) < Async
  C₅: dom(buffer) < Async
  C₆: dom(lower) < Async
  C₇: dom(upper) < Async
  C₈: dom(middle) < Async

Equality (component references):
  C₉: dom(#0) = dom(clock)
  C₁₀: dom(#1) = dom(data)
  C₁₁: dom(#3) = dom(buffer)
  C₁₂: dom(#5) = dom(buffer)
  C₁₃: dom(#7) = dom(buffer)

Equality (bit slicing projection):
  C₁₄: dom(#2) = dom(#3)
  C₁₅: dom(#4) = dom(#5)
  C₁₆: dom(#6) = dom(#7)

Equality (drivers):
  C₁₇: dom(buffer) = dom(#1)
  C₁₈: dom(lower) = dom(#2)
  C₁₉: dom(upper) = dom(#4)
  C₂₀: dom(middle) = dom(#6)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {data}, {buffer}, {lower}, {upper}, {middle},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**

Assign concrete domain `Clock` to clock's equivalence class:
```
{clock: Clock}, {data}, {buffer}, {lower}, {upper}, {middle},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Processing fixed constraint C₂: `dom(buffer) = Sync(ComponentId(clock))`**

Assign concrete domain `Sync(clock)` to buffer's equivalence class:
```
{clock: Clock}, {data}, {buffer: Sync(clock)}, {lower}, {upper}, {middle},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Processing equality constraint C₉: `dom(#0) = dom(clock)`**

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {data}, {buffer: Sync(clock)}, {lower}, {upper}, {middle},
{#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Processing equality constraint C₁₀: `dom(#1) = dom(data)`**

Merge expression #1 with data's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer: Sync(clock)}, {lower}, {upper}, {middle},
{#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Processing equality constraint C₁₁: `dom(#3) = dom(buffer)`**

Merge expression #3 with buffer's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer, #3: Sync(clock)}, {lower}, {upper}, {middle},
{#2}, {#4}, {#5}, {#6}, {#7}
```

**Processing equality constraint C₁₂: `dom(#5) = dom(buffer)`**

Merge expression #5 with buffer's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer, #3, #5: Sync(clock)}, {lower}, {upper}, {middle},
{#2}, {#4}, {#6}, {#7}
```

**Processing equality constraint C₁₃: `dom(#7) = dom(buffer)`**

Merge expression #7 with buffer's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer, #3, #5, #7: Sync(clock)}, {lower}, {upper}, {middle},
{#2}, {#4}, {#6}
```

**Processing equality constraint C₁₄: `dom(#2) = dom(#3)`**

Merge expression #2 with #3's equivalence class (buffer's class):
```
{clock, #0: Clock}, {data, #1}, {buffer, #2, #3, #5, #7: Sync(clock)}, {lower}, {upper}, {middle},
{#4}, {#6}
```

**Processing equality constraint C₁₅: `dom(#4) = dom(#5)`**

Merge expression #4 with #5's equivalence class (buffer's class):
```
{clock, #0: Clock}, {data, #1}, {buffer, #2, #3, #4, #5, #7: Sync(clock)}, {lower}, {upper}, {middle},
{#6}
```

**Processing equality constraint C₁₆: `dom(#6) = dom(#7)`**

Merge expression #6 with #7's equivalence class (buffer's class):
```
{clock, #0: Clock}, {data, #1}, {buffer, #2, #3, #4, #5, #6, #7: Sync(clock)}, {lower}, {upper}, {middle}
```

**Processing equality constraint C₁₇: `dom(buffer) = dom(#1)`**

Merge buffer's class with #1's class:
- buffer's class: `{buffer, #2, #3, #4, #5, #6, #7: Sync(clock)}` (has concrete domain)
- #1's class: `{data, #1}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {data, #1, buffer, #2, #3, #4, #5, #6, #7: Sync(clock)}, {lower}, {upper}, {middle}
```

**Processing equality constraint C₁₈: `dom(lower) = dom(#2)`**

Merge lower with #2's class (buffer's class):
```
{clock, #0: Clock}, {data, #1, buffer, #2, #3, #4, #5, #6, #7, lower: Sync(clock)}, {upper}, {middle}
```

**Processing equality constraint C₁₉: `dom(upper) = dom(#4)`**

Merge upper with #4's class (buffer's class):
```
{clock, #0: Clock}, {data, #1, buffer, #2, #3, #4, #5, #6, #7, lower, upper: Sync(clock)}, {middle}
```

**Processing equality constraint C₂₀: `dom(middle) = dom(#6)`**

Merge middle with #6's class (buffer's class):
```
{clock, #0: Clock}, {data, #1, buffer, #2, #3, #4, #5, #6, #7, lower, upper, middle: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, buffer, #2, #3, #4, #5, #6, #7, lower, upper, middle} → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(buffer) = Sync(ComponentId(clock))
  dom(lower) = Sync(ComponentId(clock))
  dom(upper) = Sync(ComponentId(clock))
  dom(middle) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))  [buffer[4..1]]
  dom(#3) = Sync(ComponentId(clock))  [buffer reference in #2]
  dom(#4) = Sync(ComponentId(clock))  [buffer[8..4]]
  dom(#5) = Sync(ComponentId(clock))  [buffer reference in #4]
  dom(#6) = Sync(ComponentId(clock))  [buffer[5]]
  dom(#7) = Sync(ComponentId(clock))  [buffer reference in #6]
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(buffer) = Sync(clock)` ✓
- All strict upper bound constraints satisfied: ✓
  - All checked domains are either `Clock` or `Sync(clock)`, both of which satisfy `< Async` ✓
- All equality constraints satisfied: ✓
  - All variables in Class 1 have domain `Clock` ✓
  - All variables in Class 2 have domain `Sync(clock)` ✓
- Bit slicing preserved domain: ✓
  - `buffer[4..1]`, `buffer[8..4]`, and `buffer[5]` all have the same domain as `buffer` ✓

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)` (clock port itself)
- `dom(data) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(lower) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(upper) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(middle) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        data: Var(α),
        lower: Var(α),
        upper: Var(α),
        middle: Var(α),
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: BitSliceExample,
    component_domains: {
        clock: Clock,
        data: Sync(ComponentId(clock)),
        buffer: Sync(ComponentId(clock)),
        lower: Sync(ComponentId(clock)),
        upper: Sync(ComponentId(clock)),
        middle: Sync(ComponentId(clock)),
    },
    signature: DomainSignature {
        domain_params: { clock: DomainVar(0) },
        port_domains: {
            clock: Concrete(Clock),
            data: Var(DomainVar(0)),
            lower: Var(DomainVar(0)),
            upper: Var(DomainVar(0)),
            middle: Var(DomainVar(0)),
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes, with one addition**: The algorithm needs to explicitly handle bit slicing and bit indexing expressions in `walk_expression()`. Currently, ALGORITHM.md includes cases for:
- Component references
- Literals
- Binary operators
- Match expressions
- When expressions
- Struct construction
- Array indexing
- Field access

But it doesn't explicitly list **bit slicing** or **bit indexing**. The handling is straightforward and follows the same pattern as field access:

```python
elif expr.kind == BitSlice:
    expr_id, source_expr = walk_expression(expr.source, &mut variables,
                                          &mut equalities, &mut upper_bounds, expr_id)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Result has same domain as source (pure projection)
    equalities.append(Equality(result_expr, source_expr))
    
    return (expr_id, result_expr)

elif expr.kind == BitIndex:
    expr_id, source_expr = walk_expression(expr.source, &mut variables,
                                          &mut equalities, &mut upper_bounds, expr_id)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Result has same domain as source (pure projection)
    equalities.append(Equality(result_expr, source_expr))
    
    return (expr_id, result_expr)
```

Note: Unlike array indexing where both the array and the index must have the same domain, bit slicing uses **constant indices** (`buffer[4..1]`, `buffer[5]`). The slice bounds are literals, not expressions, so they don't participate in domain analysis.

### Any surprises or edge cases?

**No surprises**. Bit slicing behaves exactly like field access from a domain perspective:
- It's a pure projection operation
- The result has the same domain as the source
- The slice indices are compile-time constants, not runtime expressions
- No additional domain constraints are needed

**Edge case consideration**: What if we add dynamic bit slicing in the future (e.g., `buffer[x +: 4]` where `x` is a variable)? In that case, we'd need to ensure `x` has the same domain as `buffer`, similar to array indexing. But for the current language with constant indices, this doesn't apply.

### Does the constraint system handle this without special cases?

**Yes**. Bit slicing is handled by the same equality constraint mechanism used throughout the algorithm:
- Generate an equality constraint: `dom(result) = dom(source)`
- Union-find automatically propagates the domain
- No special-case logic needed in the solver
- Works naturally with registers, ports, and any other signal type

The only "special" aspect is recognizing bit-slicing syntax during Pass 1 (constraint generation), but that's just adding another case to the expression walker, not a fundamental algorithm change.

### Any clarifications needed for ALGORITHM.md or PLANv2.md?

**ALGORITHM.md additions needed:**

1. Add `BitSlice` case to `walk_expression()` in Pass 1, showing the equality constraint generation
2. Add `BitIndex` case to `walk_expression()` in Pass 1 (single-bit indexing)
3. Note that slice bounds are constant expressions that don't participate in domain analysis

**Example addition to ALGORITHM.md**:

```python
# Bit slicing (e.g., x[8..4])
elif expr.kind == BitSlice:
    expr_id, source_expr = walk_expression(expr.source, &mut variables,
                                          &mut equalities, &mut upper_bounds, expr_id)
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Slice bounds are constants, don't generate domain variables
    # Result has same domain as source
    equalities.append(Equality(result_expr, source_expr))
    
    return (expr_id, result_expr)

# Bit indexing (e.g., x[5])
elif expr.kind == BitIndex:
    expr_id, source_expr = walk_expression(expr.source, &mut variables,
                                          &mut equalities, &mut upper_bounds, expr_id)
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Index is a constant, doesn't generate domain variables
    # Result has same domain as source
    equalities.append(Equality(result_expr, source_expr))
    
    return (expr_id, result_expr)
```

**PLANv2.md clarifications:** No changes needed. Bit slicing is a specific syntax case, not a conceptual addition to the domain model.

### Relationship to other projection operations

Bit slicing has the same domain behavior as:
- **Field access** (`struct.field`): Result has same domain as struct
- **Array indexing** (`array[index]`): Element has same domain as array (and index must match)

The key insight: **Projection operations preserve domains**. They extract a subset of the data but don't change the clock domain relationship.

### Summary

Bit slicing is a straightforward addition to the algorithm:
- ✅ Works naturally with the constraint system
- ✅ No special solver logic needed
- ✅ Follows the same pattern as field access
- ✅ Only requires adding expression cases to Pass 1
- ✅ No algorithm gaps or design issues revealed

**Recommendation**: Add the `BitSlice` and `BitIndex` cases to ALGORITHM.md's `walk_expression()` function for completeness.
