# Example 12: Struct Construction with Mixed Domains (ERROR)

```virdant
struct type Pair {
    x : Word[8]
    y : Word[8]
}

mod StructMixedError {
    incoming clock_a : Clock
    incoming clock_b : Clock

    reg val_a : Word[8] on clock_a {
        it <= $w8(42)
    }

    reg val_b : Word[8] on clock_b {
        it <= $w8(84)
    }

    wire result : Pair {
        it := ${ x = val_a, y = val_b }  // ERROR: fields from different domains
    }

    outgoing out : Pair {
        it := result
    }
}
```

**What's happening**: A struct is constructed with fields from two different clock domains. The register `val_a` is on `clock_a` domain, and `val_b` is on `clock_b` domain. The struct construction rule requires all fields to have the same domain, so trying to unify `Sync(clock_a)` with `Sync(clock_b)` causes an error.

**Expected error**: Domain unification error when trying to equate `Sync(ComponentId(clock_a))` with `Sync(ComponentId(clock_b))` through the struct construction constraint.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock_a` (incoming, type Clock)
- `clock_b` (incoming, type Clock)
- `val_a` (reg, type Word[8])
- `val_b` (reg, type Word[8])
- `result` (wire, type Pair)
- `out` (outgoing, type Pair)

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock_a : Clock`**
- Component: `clock_a`
- Type: Clock
- No expressions

**Declaration: `incoming clock_b : Clock`**
- Component: `clock_b`
- Type: Clock
- No expressions

**Declaration: `reg val_a : Word[8] on clock_a`**
- Component: `val_a`
- Has `on clock_a` clause
- Expression `#0`: reference to `clock_a` (in the `on clock_a` clause)

**Driver: `val_a <= $w8(42)`**
- Expression `#1`: literal `$w8(42)` (RHS of driver)

**Declaration: `reg val_b : Word[8] on clock_b`**
- Component: `val_b`
- Has `on clock_b` clause
- Expression `#2`: reference to `clock_b` (in the `on clock_b` clause)

**Driver: `val_b <= $w8(84)`**
- Expression `#3`: literal `$w8(84)` (RHS of driver)

**Declaration: `wire result : Pair`**
- Component: `result`
- No expressions in declaration

**Driver: `result := ${ x = val_a, y = val_b }`**
- Expression `#4`: reference to `val_a` (field `x`)
- Expression `#5`: reference to `val_b` (field `y`)
- Expression `#6`: struct literal `${ x = val_a, y = val_b }` (RHS of driver)

**Declaration: `outgoing out : Pair`**
- Component: `out`
- No expressions in declaration

**Driver: `out := result`**
- Expression `#7`: reference to `result` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock_a`
- `#1`: ExprLiteral `$w8(42)`
- `#2`: ExprReference to `clock_b`
- `#3`: ExprLiteral `$w8(84)`
- `#4`: ExprReference to `val_a` (field x value)
- `#5`: ExprReference to `val_b` (field y value)
- `#6`: ExprStructLiteral with fields #4 and #5
- `#7`: ExprReference to `result`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock_a) = Clock` (Clock-typed signal)
- `dom(clock_b) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(val_a) = Sync(ComponentId(clock_a))` (register with `on clock_a` clause)
- `dom(val_b) = Sync(ComponentId(clock_b))` (register with `on clock_b` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock_a` → `dom(#0) = dom(clock_a)`
- Expression `#2` references `clock_b` → `dom(#2) = dom(clock_b)`
- Expression `#4` references `val_a` → `dom(#4) = dom(val_a)`
- Expression `#5` references `val_b` → `dom(#5) = dom(val_b)`
- Expression `#7` references `result` → `dom(#7) = dom(result)`

**Equality constraints from struct construction:**

Struct literal expression `#6` has fields `#4` and `#5`:
- All fields must equal the struct result: `dom(#4) = dom(#6)`
- All fields must equal the struct result: `dom(#5) = dom(#6)` ← THE PROBLEM

**Equality constraints from drivers:**

Driver: `val_a <= $w8(42)`
- LHS: component `val_a`
- RHS: expression `#1` (literal)
- Constraint: `dom(val_a) = dom(#1)`

Driver: `val_b <= $w8(84)`
- LHS: component `val_b`
- RHS: expression `#3` (literal)
- Constraint: `dom(val_b) = dom(#3)`

Driver: `result := ${ x = val_a, y = val_b }`
- LHS: component `result`
- RHS: expression `#6` (struct literal)
- Constraint: `dom(result) = dom(#6)`

Driver: `out := result`
- LHS: component `out`
- RHS: expression `#7`
- Constraint: `dom(out) = dom(#7)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock_a) < Async`
- `dom(clock_b) < Async`
- `dom(val_a) < Async`
- `dom(val_b) < Async`
- `dom(result) < Async`
- `dom(out) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock_a) = Clock
  F2: dom(clock_b) = Clock
  F3: dom(val_a) = Sync(ComponentId(clock_a))
  F4: dom(val_b) = Sync(ComponentId(clock_b))

Strict upper bound (implicit):
  S1: dom(clock_a) < Async
  S2: dom(clock_b) < Async
  S3: dom(val_a) < Async
  S4: dom(val_b) < Async
  S5: dom(result) < Async
  S6: dom(out) < Async

Equality:
  E1: dom(#0) = dom(clock_a)
  E2: dom(#2) = dom(clock_b)
  E3: dom(#4) = dom(val_a)
  E4: dom(#5) = dom(val_b)
  E5: dom(#7) = dom(result)
  E6: dom(#4) = dom(#6)           ← struct field constraint
  E7: dom(#5) = dom(#6)           ← struct field constraint (PROBLEM)
  E8: dom(val_a) = dom(#1)
  E9: dom(val_b) = dom(#3)
  E10: dom(result) = dom(#6)
  E11: dom(out) = dom(#7)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment (or detect the error).

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock_a}, {clock_b}, {val_a}, {val_b}, {result}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing fixed constraint F1: `dom(clock_a) = Clock`

Assign concrete domain `Clock` to clock_a's equivalence class:
```
{clock_a: Clock}, {clock_b}, {val_a}, {val_b}, {result}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing fixed constraint F2: `dom(clock_b) = Clock`

Assign concrete domain `Clock` to clock_b's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {val_a}, {val_b}, {result}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing fixed constraint F3: `dom(val_a) = Sync(ComponentId(clock_a))`

Assign concrete domain `Sync(clock_a)` to val_a's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {val_a: Sync(clock_a)}, {val_b}, {result}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing fixed constraint F4: `dom(val_b) = Sync(ComponentId(clock_b))`

Assign concrete domain `Sync(clock_b)` to val_b's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {val_a: Sync(clock_a)}, {val_b: Sync(clock_b)},
{result}, {out}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E1: `dom(#0) = dom(clock_a)`

Merge expression #0 with clock_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b: Clock}, {val_a: Sync(clock_a)}, {val_b: Sync(clock_b)},
{result}, {out}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E2: `dom(#2) = dom(clock_b)`

Merge expression #2 with clock_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #2: Clock}, {val_a: Sync(clock_a)}, {val_b: Sync(clock_b)},
{result}, {out}, {#1}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E3: `dom(#4) = dom(val_a)`

Merge expression #4 with val_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #2: Clock}, {val_a, #4: Sync(clock_a)}, {val_b: Sync(clock_b)},
{result}, {out}, {#1}, {#3}, {#5}, {#6}, {#7}
```

### Processing equality constraint E4: `dom(#5) = dom(val_b)`

Merge expression #5 with val_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #2: Clock}, {val_a, #4: Sync(clock_a)},
{val_b, #5: Sync(clock_b)}, {result}, {out}, {#1}, {#3}, {#6}, {#7}
```

### Processing equality constraint E5: `dom(#7) = dom(result)`

Merge expression #7 with result's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #2: Clock}, {val_a, #4: Sync(clock_a)},
{val_b, #5: Sync(clock_b)}, {result, #7}, {out}, {#1}, {#3}, {#6}
```

### Processing equality constraint E6: `dom(#4) = dom(#6)`

Merge struct literal #6 with field #4's class:
- #4's class: `{val_a, #4: Sync(clock_a)}` (has concrete domain `Sync(clock_a)`)
- #6's class: `{#6}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock_a)`

```
{clock_a, #0: Clock}, {clock_b, #2: Clock}, {val_a, #4, #6: Sync(clock_a)},
{val_b, #5: Sync(clock_b)}, {result, #7}, {out}, {#1}, {#3}
```

### Processing equality constraint E7: `dom(#5) = dom(#6)` ⚠️ ERROR

Attempt to merge field #5's class with struct #6's class:
- #5's class: `{val_b, #5: Sync(clock_b)}` (has concrete domain `Sync(clock_b)`)
- #6's class: `{val_a, #4, #6: Sync(clock_a)}` (has concrete domain `Sync(clock_a)`)

Check compatibility: `compatible(Sync(clock_b), Sync(clock_a))`?
- Both are `Sync` domains
- Are they the same clock? `clock_b == clock_a`? **No** ❌
- **Result: INCOMPATIBLE** ❌

**ERROR DETECTED**: Cannot unify `Sync(ComponentId(clock_b))` with `Sync(ComponentId(clock_a))`

Error message:
```
Domain unification error in struct construction `${ x = val_a, y = val_b }`:
  Field `x` has domain Sync(clock_a)
  Field `y` has domain Sync(clock_b)

All fields in a struct literal must have the same clock domain.

The fields come from:
  - val_a is a register on clock_a
  - val_b is a register on clock_b

Help: If you need to combine values from different clock domains, you must
first synchronize them to the same domain using the Sync module, or declare
the result as async if it doesn't need to be registered.
```

### Solver terminates with error

The solver stops processing further constraints and returns the error.

**Solve result:**
```python
SolveResult {
    union_find: /* partial state */,
    errors: [
        "Domain unification error: cannot merge Sync(ComponentId(clock_a)) with Sync(ComponentId(clock_b))
         at constraint E7: dom(#5) = dom(#6)
         from struct construction: ${ x = val_a, y = val_b }"
    ]
}
```

---

## Pass 3: Signature Extraction (SKIPPED)

Since domain analysis found errors, signature extraction is skipped. The module is invalid.

---

## Final DomainAnalysis Result

```rust
DomainAnalysis {
    moddef: StructMixedError,
    component_domains: {
        clock_a: Clock,
        clock_b: Clock,
        val_a: Sync(ComponentId(clock_a)),
        val_b: Sync(ComponentId(clock_b)),
        result: /* unresolved */,
        out: /* unresolved */,
    },
    signature: /* invalid */,
    diagnostics: [
        Diagnostic {
            severity: Error,
            message: "Domain unification error: struct fields from incompatible clock domains",
            location: /* struct literal: ${ x = val_a, y = val_b } */,
            help: "Use the Sync module to synchronize signals to the same clock domain",
        }
    ],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes**, this works very naturally with the constraint-based algorithm:

1. **Struct construction generates simple equality constraints**: The rule "all fields must have the same domain as the struct" translates directly to equality constraints `dom(field1) = dom(struct)`, `dom(field2) = dom(struct)`.

2. **The error emerges from transitivity**: Through the equality constraints, we get `dom(field1) = dom(struct) = dom(field2)`, which forces `dom(field1) = dom(field2)`. When the fields have different `Sync` domains, this unification fails naturally.

3. **No special case logic needed**: The struct-specific logic is just "generate equality constraints for all fields." The domain incompatibility is caught by the standard union-find merge logic in Pass 2.

4. **Clear error location**: The constraint system preserves the source location (the struct literal expression), making it easy to generate a precise error message.

### Edge Cases Discovered

1. **Struct with literal fields**: If a struct is constructed entirely from literals (e.g., `${ x = 0, y = 1 }`), all fields start with domain `Constant` and will unify successfully. The struct's domain then propagates from its usage context (what drives it or what uses it).

2. **Struct with one field**: Even single-field structs generate the constraint correctly — the field domain unifies with the struct domain.

3. **Nested structs**: If a struct contains another struct as a field, the inner struct has already been resolved to a single domain, so it acts like any other field. The equality constraints chain through: `dom(inner_field_a) = dom(inner_struct) = dom(outer_field) = dom(outer_struct)`.

4. **Empty struct**: A struct type with no fields would generate no field constraints. The struct's domain would be completely determined by external constraints (where it's used). This is probably pathological and wouldn't appear in real code.

5. **Mixing Constant with Sync**: If some fields are literals (`Constant`) and some are from a clock domain (`Sync(clock)`), they unify successfully to `Sync(clock)` because `Constant` is the lattice bottom and unifies with any `Sync`.

### Design Issues

**None major**. The algorithm handles struct construction cleanly. Some observations:

1. **Struct field access is the dual operation**: When accessing a field (`struct.field`), the field expression gets the same domain as the struct. This is also a simple equality constraint and works symmetrically.

2. **Type system integration**: The type checker already ensures all fields match the struct type. Domain checking is orthogonal — it only cares that all field *values* (which have correct types) come from the same domain.

3. **Error message quality**: The struct construction site naturally captures all the relevant information for a good error message:
   - Which fields are involved
   - Where each field value comes from
   - What domains they're on
   - Suggestions for fixing (use `Sync` module or declare result `async`)

### Clarifications for PLANv2.md and ALGORITHM.md

**No clarifications needed.** The existing algorithm documents exactly specify this case:

From ALGORITHM.md:
```python
# Struct construction
elif expr.kind == StructLiteral:
    field_exprs = []
    for field in expr.fields:
        expr_id, field_expr = walk_expression(field.value, ...)
        field_exprs.append(field_expr)

    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)

    # All fields must equal result
    for field_expr in field_exprs:
        equalities.append(Equality(field_expr, result_expr))

    return (expr_id, result_expr)
```

This is exactly what we implemented in the example, and it correctly catches the error.

From PLANv2.md:
> **Struct Construction**
>
> When constructing a struct:
> ```virdant
> wire result : Pair {
>     it := ${ x = a, y = b }
> }
> ```
>
> **Rule**: All field expressions (`a`, `b`) must have the same domain, and the resulting struct has that domain.

This rule is correctly enforced by the equality constraints. The example demonstrates that the algorithm works as specified.

### Implementation Confidence

This example gives high confidence that the constraint-based approach handles composite data structures (structs) correctly:

- **Compositionality**: Struct construction is just another expression that generates constraints. No special solver logic needed.
- **Precision**: The error pinpoints exactly which fields are problematic and why.
- **Natural flow**: The constraint generation in Pass 1 is straightforward. Pass 2's union-find naturally detects the conflict.
- **Extensibility**: Array literals would work the same way (all elements must have the same domain). Tuple construction would be identical to struct construction.
