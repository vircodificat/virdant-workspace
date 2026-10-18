# Example 9: Array Indexing Domain Conflict (ERROR)

```virdant
mod ArrayIndexConflict {
    incoming clock_a : Clock
    incoming clock_b : Clock

    reg array_data : Word[8][4] on clock_a {
        it <= $[0x00, 0x11, 0x22, 0x33]
    }

    reg index : Word[2] on clock_b {
        it <= 0b10
    }

    outgoing result : Word[8] {
        it := array_data[index]  // ERROR: index and array have different domains
    }
}
```

**What's happening**: An array synchronized to `clock_a` is being indexed by a value synchronized to `clock_b`. The array indexing rule requires that both the array and the index have the same domain. The equality constraint `dom(index) = dom(array)` forces `Sync(clock_a) = Sync(clock_b)`, which is a unification error.

**Expected error**: Domain unification error when trying to equate `Sync(ComponentId(clock_a))` with `Sync(ComponentId(clock_b))`.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock_a` (incoming, type Clock)
- `clock_b` (incoming, type Clock)
- `array_data` (reg, type Word[8][4])
- `index` (reg, type Word[2])
- `result` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock_a : Clock`**
- Component: `clock_a`
- Type: Clock
- No expressions

**Declaration: `incoming clock_b : Clock`**
- Component: `clock_b`
- Type: Clock
- No expressions

**Declaration: `reg array_data : Word[8][4] on clock_a`**
- Component: `array_data`
- Has `on clock_a` clause
- Expression `#0`: reference to `clock_a` (in the `on clock_a` clause)

**Driver: `array_data <= $[0x00, 0x11, 0x22, 0x33]`**
- Expression `#1`: array literal `$[0x00, 0x11, 0x22, 0x33]`
  - Expression `#2`: literal `0x00`
  - Expression `#3`: literal `0x11`
  - Expression `#4`: literal `0x22`
  - Expression `#5`: literal `0x33`

**Declaration: `reg index : Word[2] on clock_b`**
- Component: `index`
- Has `on clock_b` clause
- Expression `#6`: reference to `clock_b` (in the `on clock_b` clause)

**Driver: `index <= 0b10`**
- Expression `#7`: literal `0b10`

**Declaration: `outgoing result : Word[8]`**
- Component: `result`
- No expressions in declaration

**Driver: `result := array_data[index]`**
- Expression `#8`: array indexing `array_data[index]`
  - Expression `#9`: reference to `array_data` (array operand)
  - Expression `#10`: reference to `index` (index operand)

**Expression inventory:**
- `#0`: ExprReference to `clock_a`
- `#1`: ExprArrayLiteral (4 elements)
- `#2`: ExprLiteral `0x00`
- `#3`: ExprLiteral `0x11`
- `#4`: ExprLiteral `0x22`
- `#5`: ExprLiteral `0x33`
- `#6`: ExprReference to `clock_b`
- `#7`: ExprLiteral `0b10`
- `#8`: ExprArrayIndex (result of `array_data[index]`)
- `#9`: ExprReference to `array_data`
- `#10`: ExprReference to `index`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock_a) = Clock` (Clock-typed signal)
- `dom(clock_b) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(array_data) = Sync(ComponentId(clock_a))` (register with `on clock_a` clause)
- `dom(index) = Sync(ComponentId(clock_b))` (register with `on clock_b` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock_a` → `dom(#0) = dom(clock_a)`
- Expression `#6` references `clock_b` → `dom(#6) = dom(clock_b)`
- Expression `#9` references `array_data` → `dom(#9) = dom(array_data)`
- Expression `#10` references `index` → `dom(#10) = dom(index)`

**Equality constraints from array literal:**

Array literal `#1` with elements `#2`, `#3`, `#4`, `#5`:
- All elements must equal the array: `dom(#2) = dom(#1)`, `dom(#3) = dom(#1)`, `dom(#4) = dom(#1)`, `dom(#5) = dom(#1)`

**Equality constraints from array indexing:**

Array indexing `#8 = array_data[index]` with array `#9` and index `#10`:
- Result has same domain as array: `dom(#8) = dom(#9)`
- Index has same domain as array: `dom(#10) = dom(#9)` ← **THE PROBLEM**

**Equality constraints from drivers:**

Driver: `array_data <= $[...]`
- LHS: component `array_data`
- RHS: expression `#1`
- Constraint: `dom(array_data) = dom(#1)`

Driver: `index <= 0b10`
- LHS: component `index`
- RHS: expression `#7`
- Constraint: `dom(index) = dom(#7)`

Driver: `result := array_data[index]`
- LHS: component `result`
- RHS: expression `#8`
- Constraint: `dom(result) = dom(#8)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock_a) < Async`
- `dom(clock_b) < Async`
- `dom(array_data) < Async`
- `dom(index) < Async`
- `dom(result) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock_a) = Clock
  F2: dom(clock_b) = Clock
  F3: dom(array_data) = Sync(ComponentId(clock_a))
  F4: dom(index) = Sync(ComponentId(clock_b))

Strict upper bound (implicit):
  S1: dom(clock_a) < Async
  S2: dom(clock_b) < Async
  S3: dom(array_data) < Async
  S4: dom(index) < Async
  S5: dom(result) < Async

Equality:
  E1:  dom(#0) = dom(clock_a)
  E2:  dom(#6) = dom(clock_b)
  E3:  dom(#9) = dom(array_data)
  E4:  dom(#10) = dom(index)
  E5:  dom(#2) = dom(#1)
  E6:  dom(#3) = dom(#1)
  E7:  dom(#4) = dom(#1)
  E8:  dom(#5) = dom(#1)
  E9:  dom(#8) = dom(#9)
  E10: dom(#10) = dom(#9)          ← THE CRITICAL CONSTRAINT
  E11: dom(array_data) = dom(#1)
  E12: dom(index) = dom(#7)
  E13: dom(result) = dom(#8)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment (or detect the error).

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock_a}, {clock_b}, {array_data}, {index}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing fixed constraint F1: `dom(clock_a) = Clock`

Assign concrete domain `Clock` to clock_a's equivalence class:
```
{clock_a: Clock}, {clock_b}, {array_data}, {index}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing fixed constraint F2: `dom(clock_b) = Clock`

Assign concrete domain `Clock` to clock_b's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {array_data}, {index}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing fixed constraint F3: `dom(array_data) = Sync(ComponentId(clock_a))`

Assign concrete domain `Sync(clock_a)` to array_data's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {array_data: Sync(clock_a)}, {index}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing fixed constraint F4: `dom(index) = Sync(ComponentId(clock_b))`

Assign concrete domain `Sync(clock_b)` to index's equivalence class:
```
{clock_a: Clock}, {clock_b: Clock}, {array_data: Sync(clock_a)}, {index: Sync(clock_b)}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing equality constraint E1: `dom(#0) = dom(clock_a)`

Merge expression #0 with clock_a's equivalence class:
```
{clock_a, #0: Clock}, {clock_b: Clock}, {array_data: Sync(clock_a)}, {index: Sync(clock_b)}, {result},
{#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}
```

### Processing equality constraint E2: `dom(#6) = dom(clock_b)`

Merge expression #6 with clock_b's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data: Sync(clock_a)}, {index: Sync(clock_b)}, {result},
{#1}, {#2}, {#3}, {#4}, {#5}, {#7}, {#8}, {#9}, {#10}
```

### Processing equality constraint E3: `dom(#9) = dom(array_data)`

Merge expression #9 with array_data's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data, #9: Sync(clock_a)}, {index: Sync(clock_b)}, {result},
{#1}, {#2}, {#3}, {#4}, {#5}, {#7}, {#8}, {#10}
```

### Processing equality constraint E4: `dom(#10) = dom(index)`

Merge expression #10 with index's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data, #9: Sync(clock_a)}, {index, #10: Sync(clock_b)}, {result},
{#1}, {#2}, {#3}, {#4}, {#5}, {#7}, {#8}
```

### Processing equality constraint E5: `dom(#2) = dom(#1)`

Merge expression #2 with #1's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data, #9: Sync(clock_a)}, {index, #10: Sync(clock_b)}, {result},
{#1, #2}, {#3}, {#4}, {#5}, {#7}, {#8}
```

### Processing equality constraints E6, E7, E8: `dom(#3) = dom(#1)`, `dom(#4) = dom(#1)`, `dom(#5) = dom(#1)`

Merge all literal elements with the array literal:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data, #9: Sync(clock_a)}, {index, #10: Sync(clock_b)}, {result},
{#1, #2, #3, #4, #5}, {#7}, {#8}
```

### Processing equality constraint E9: `dom(#8) = dom(#9)`

Merge expression #8 (array index result) with #9's equivalence class:
```
{clock_a, #0: Clock}, {clock_b, #6: Clock}, {array_data, #9, #8: Sync(clock_a)}, {index, #10: Sync(clock_b)}, {result},
{#1, #2, #3, #4, #5}, {#7}
```

### Processing equality constraint E10: `dom(#10) = dom(#9)` ⚠️ ERROR

Attempt to merge #10's class with #9's class:
- #10's class: `{index, #10: Sync(clock_b)}` (has concrete domain `Sync(clock_b)`)
- #9's class: `{array_data, #9, #8: Sync(clock_a)}` (has concrete domain `Sync(clock_a)`)

Check compatibility: `compatible(Sync(clock_b), Sync(clock_a))`?
- `Sync(clock_b) == Sync(clock_a)`? No (different ComponentIds)
- Both are `Sync` domains? Yes
- Same clock? `clock_b == clock_a`? **No** ❌
- **Result: INCOMPATIBLE** ❌

**ERROR DETECTED**: Cannot unify `Sync(ComponentId(clock_b))` with `Sync(ComponentId(clock_a))`

Error message:
```
Domain unification error in array indexing `array_data[index]`:
  Array `array_data` has domain Sync(clock_a)
  Index `index` has domain Sync(clock_b)

Array indexing requires that both the array and the index expression
have the same clock domain. The array is synchronized to `clock_a`,
but the index is synchronized to `clock_b`.

Help: To use an index from a different clock domain, first synchronize
it to the array's domain:
  wire synced_index : Word[2] { it := Sync(index, clock_a) }
  outgoing result : Word[8] { it := array_data[synced_index] }
```

### Solver terminates with error

The solver stops processing further constraints and returns the error.

**Solve result:**
```python
SolveResult {
    union_find: /* partial state */,
    errors: [
        "Domain unification error: cannot merge Sync(ComponentId(clock_b)) with Sync(ComponentId(clock_a))
         at constraint E10: dom(#10) = dom(#9)
         from array indexing: array_data[index]"
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
    moddef: ArrayIndexConflict,
    component_domains: {
        clock_a: Clock,
        clock_b: Clock,
        array_data: Sync(ComponentId(clock_a)),
        index: Sync(ComponentId(clock_b)),
        result: /* unresolved */,
    },
    signature: /* invalid */,
    diagnostics: [
        Diagnostic {
            severity: Error,
            message: "Domain unification error: array and index have different clock domains",
            location: /* expression: array_data[index] */,
            help: "Use the Sync module to cross clock domains before indexing",
        }
    ],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes, perfectly.** The array indexing constraint `dom(index) = dom(array)` naturally enforces domain compatibility without any special-case logic. The error is detected during the normal union-find merge process when trying to unify two different `Sync` domains.

The constraint generation for array indexing is straightforward:
1. Walk the array expression → get `array_expr`
2. Walk the index expression → get `index_expr`
3. Create result expression
4. Add constraint: `dom(result) = dom(array_expr)`
5. Add constraint: `dom(index_expr) = dom(array_expr)`

This elegantly captures the semantic requirement that "to index an array, you must be in the same clock domain as the array itself."

### Edge cases discovered:

1. **Constant index**: If the index is a literal constant (no clock domain), the constraint becomes `dom(literal) = dom(array)`. Since literals start with no domain (`Constant`), they unify with any `Sync` domain, adopting the array's domain. This is correct behavior.

2. **Async array with sync index (or vice versa)**: If the array is `async` and the index is `Sync(clock)`, the constraint `dom(index) = dom(array)` would try to unify `Sync(clock)` with `Async`, which fails. This is correct — you can't use a clocked index to access an async array, because the indexing operation itself would need to happen in a clock domain.

3. **Both async**: If both array and index are `async`, they unify to `Async`. This is correct — async logic can index async arrays.

4. **Multi-dimensional arrays**: For `array[i][j]`, the constraints naturally cascade:
   - `dom(array[i]) = dom(array)`
   - `dom(i) = dom(array)`
   - `dom(array[i][j]) = dom(array[i])`
   - `dom(j) = dom(array[i])`

   Through transitivity, this ensures that both `i` and `j` have the same domain as the original array. The algorithm handles this automatically without special logic.

### Design questions:

**Q1: Should we allow async arrays to be indexed by sync signals?**

Current design: No. The constraint `dom(index) = dom(array)` forbids it.

Rationale: Indexing is a combinational operation that happens "in" some clock domain (or in async). An async array has no clock domain, so using a clocked index would be mixing domains — the indexing operation would be ambiguous about whether it's synchronous or asynchronous.

**Alternative design**: We could allow `dom(index) ≤ dom(array)` (index can upcast to array's domain). This would permit sync indices for async arrays. However, this seems semantically odd — if the index is changing synchronously, what does it mean to index an async array?

**Recommendation**: Keep the strict equality constraint. If someone needs this pattern, they can explicitly upcast the index to async first.

**Q2: What about array literals in expressions?**

Array literals (like `$[0, 1, 2, 3]`) are treated like struct literals — all elements must have the same domain, and that becomes the array's domain. If the elements are literal constants, they're `Constant` and unify to whatever domain the target needs. This is demonstrated in this example where the array literal `$[0x00, 0x11, 0x22, 0x33]` adopts `Sync(clock_a)` because it's assigned to a register on `clock_a`.

### Clarifications for PLANv2.md or ALGORITHM.md:

1. **Array indexing rule is well-specified**: The ALGORITHM.md already correctly specifies the two equality constraints for array indexing. No changes needed.

2. **Potential addition to PLANv2.md**: Under "Struct and Array Operations", clarify that the index equality constraint means:
   > "The index expression must have the same domain as the array being indexed. There is no implicit upcasting for index expressions — if you need to use an index from a different clock domain, you must explicitly synchronize it first using the Sync module."

3. **Error message quality**: The error message can be made very specific because we know:
   - It's an array indexing operation
   - We have both the array's domain and the index's domain
   - We can suggest the exact fix (synchronize the index to the array's domain)

### Comparison with other cross-domain errors:

This error is structurally similar to example 006 (async driving register):
- Both involve two concrete `Sync` domains or mixing `Sync` with `Async`
- Both are caught during union-find merge when constraint solving
- Both have clear error messages with actionable fixes

The main difference is **intent clarity**:
- In the register case, the error is about a domain crossing at an assignment boundary (driver)
- In the array indexing case, the error is about trying to perform a combinational operation (indexing) with operands from different domains

Both are caught by the same underlying mechanism (incompatible domain unification), but the context allows for different error messages.

### Potential optimization:

For better error reporting, we could track the **provenance** of each expression in the constraint system. This would allow us to generate error messages like:

```
Error: array_data[index] has incompatible domains
  array_data: Sync(clock_a) (from register declaration at line 5)
  index: Sync(clock_b) (from register declaration at line 9)
```

This would require extending the `ConstraintSystem` to store source locations and constraint origins, but the core algorithm would remain unchanged.
