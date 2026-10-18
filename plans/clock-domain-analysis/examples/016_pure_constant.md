# Example 16: Module with Only Constant Domains

```virdant
mod PureConstant {
    outgoing zero : Word[8] {
        it := 0w8
    }

    outgoing one : Word[8] {
        it := 1w8
    }

    wire sum : Word[8] {
        it := zero + one
    }

    outgoing result : Word[8] {
        it := sum
    }
}
```

**What's happening**: A module with no clock ports, no registers, and only literal expressions. All signals represent constant values with no clock domain dependencies. This is the edge case where no domain parameters exist in the signature because there are no clocks.

**Expected result**: All components should have domain `Constant` since they only depend on literal values.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `zero` (outgoing, type Word[8])
- `one` (outgoing, type Word[8])
- `sum` (wire, type Word[8])
- `result` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `outgoing zero : Word[8]`**
- Component: `zero`
- No expressions in declaration

**Driver: `zero := 0w8`**
- Expression `#0`: the literal `0w8` (RHS of driver)

**Declaration: `outgoing one : Word[8]`**
- Component: `one`
- No expressions in declaration

**Driver: `one := 1w8`**
- Expression `#1`: the literal `1w8` (RHS of driver)

**Declaration: `wire sum : Word[8]`**
- Component: `sum`
- No expressions in declaration

**Driver: `sum := zero + one`**
- Expression `#2`: reference to `zero` (left operand)
- Expression `#3`: reference to `one` (right operand)
- Expression `#4`: binary operator `+` (result of `zero + one`)

**Declaration: `outgoing result : Word[8]`**
- Component: `result`
- No expressions in declaration

**Driver: `result := sum`**
- Expression `#5`: reference to `sum` (RHS of driver)

**Expression inventory:**
- `#0`: Literal `0w8`
- `#1`: Literal `1w8`
- `#2`: ExprReference to `zero`
- `#3`: ExprReference to `one`
- `#4`: ExprBinaryOp `+` with operands `#2` and `#3`
- `#5`: ExprReference to `sum`

### Step 3 - Generate constraints:

**No fixed constraints yet:**
- No Clock-typed signals
- No registers with `on` clauses
- No async modifiers

**Note on literals**: According to the domain model, literals represent pure expressions with no signal references and should have domain `Constant`. However, the current algorithm in ALGORITHM.md generates no constraints for literals. For this example, we'll **add** Fixed constraints for literals:

**Fixed constraints for literals (algorithmic clarification):**
- `dom(#0) = Constant` (literal `0w8`)
- `dom(#1) = Constant` (literal `1w8`)

*(Note: This may need to be added to ALGORITHM.md as a special case)*

**Equality constraints from component references:**
- Expression `#2` references `zero` → `dom(#2) = dom(zero)`
- Expression `#3` references `one` → `dom(#3) = dom(one)`
- Expression `#5` references `sum` → `dom(#5) = dom(sum)`

**Upper bound constraints from binary operator:**

For binary operator `zero + one` (expression `#4`):
- Left operand `#2` can upcast to result: `dom(#2) ≤ dom(#4)`
- Right operand `#3` can upcast to result: `dom(#3) ≤ dom(#4)`

**Equality constraints from drivers:**

Driver: `zero := 0w8`
- LHS: component `zero`
- RHS: expression `#0`
- Constraint: `dom(zero) = dom(#0)`

Driver: `one := 1w8`
- LHS: component `one`
- RHS: expression `#1`
- Constraint: `dom(one) = dom(#1)`

Driver: `sum := zero + one`
- LHS: component `sum`
- RHS: expression `#4`
- Constraint: `dom(sum) = dom(#4)`

Driver: `result := sum`
- LHS: component `result`
- RHS: expression `#5`
- Constraint: `dom(result) = dom(#5)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(zero) < Async`
- `dom(one) < Async`
- `dom(sum) < Async`
- `dom(result) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(#0) = Constant   (literal 0w8)
  F2: dom(#1) = Constant   (literal 1w8)

Strict upper bound (implicit):
  S1: dom(zero) < Async
  S2: dom(one) < Async
  S3: dom(sum) < Async
  S4: dom(result) < Async

Upper bound (from operators):
  U1: dom(#2) ≤ dom(#4)
  U2: dom(#3) ≤ dom(#4)

Equality:
  E1: dom(#2) = dom(zero)
  E2: dom(#3) = dom(one)
  E3: dom(#5) = dom(sum)
  E4: dom(zero) = dom(#0)
  E5: dom(one) = dom(#1)
  E6: dom(sum) = dom(#4)
  E7: dom(result) = dom(#5)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{zero}, {one}, {sum}, {result}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F1: `dom(#0) = Constant`

Assign concrete domain `Constant` to expression #0's equivalence class:
```
{zero}, {one}, {sum}, {result}, {#0: Constant}, {#1}, {#2}, {#3}, {#4}, {#5}
```

### Processing fixed constraint F2: `dom(#1) = Constant`

Assign concrete domain `Constant` to expression #1's equivalence class:
```
{zero}, {one}, {sum}, {result}, {#0: Constant}, {#1: Constant}, {#2}, {#3}, {#4}, {#5}
```

### Processing equality constraint E1: `dom(#2) = dom(zero)`

Merge expression #2 with component zero's equivalence class:
```
{zero, #2}, {one}, {sum}, {result}, {#0: Constant}, {#1: Constant}, {#3}, {#4}, {#5}
```

### Processing equality constraint E2: `dom(#3) = dom(one)`

Merge expression #3 with component one's equivalence class:
```
{zero, #2}, {one, #3}, {sum}, {result}, {#0: Constant}, {#1: Constant}, {#4}, {#5}
```

### Processing equality constraint E3: `dom(#5) = dom(sum)`

Merge expression #5 with component sum's equivalence class:
```
{zero, #2}, {one, #3}, {sum, #5}, {result}, {#0: Constant}, {#1: Constant}, {#4}
```

### Processing equality constraint E4: `dom(zero) = dom(#0)`

Merge zero's class with expression #0's class:
- zero's class: `{zero, #2}` (no concrete domain yet)
- #0's class: `{#0: Constant}` (has concrete domain `Constant`)
- Merged class gets the concrete domain `Constant`

```
{zero, #2, #0: Constant}, {one, #3}, {sum, #5}, {result}, {#1: Constant}, {#4}
```

### Processing equality constraint E5: `dom(one) = dom(#1)`

Merge one's class with expression #1's class:
- one's class: `{one, #3}` (no concrete domain yet)
- #1's class: `{#1: Constant}` (has concrete domain `Constant`)
- Merged class gets the concrete domain `Constant`

```
{zero, #2, #0: Constant}, {one, #3, #1: Constant}, {sum, #5}, {result}, {#4}
```

### Processing equality constraint E6: `dom(sum) = dom(#4)`

Merge sum's class with expression #4's equivalence class:
```
{zero, #2, #0: Constant}, {one, #3, #1: Constant}, {sum, #5, #4}, {result}
```

### Processing equality constraint E7: `dom(result) = dom(#5)`

Merge result with #5's class (which is sum's class):
```
{zero, #2, #0: Constant}, {one, #3, #1: Constant}, {sum, #5, #4, result}
```

### Processing upper bound constraint U1: `dom(#2) ≤ dom(#4)`

Find representatives:
- `find(#2)` → class `{zero, #2, #0: Constant}` with domain `Constant`
- `find(#4)` → class `{sum, #5, #4}` with domain `None` (unresolved)

Upper bound states: `Constant ≤ dom(#4)`, meaning #4's domain must be at least as general as `Constant`. Since `Constant` is the bottom of the lattice, any domain satisfies this constraint.

Record that class `{sum, #5, #4}` has an upper bound from class `{zero, #2, #0}` with domain `Constant`.

### Processing upper bound constraint U2: `dom(#3) ≤ dom(#4)`

Find representatives:
- `find(#3)` → class `{one, #3, #1: Constant}` with domain `Constant`
- `find(#4)` → class `{sum, #5, #4}` with domain `None` (unresolved)

Same as U1: `Constant ≤ dom(#4)`.

Record that class `{sum, #5, #4}` has a second upper bound from class `{one, #3, #1}` with domain `Constant`.

### Resolving unresolved classes with upper bounds:

For class `{sum, #5, #4, result}` (currently unresolved):
- Collect upper bound constraints on this class:
  - U1: from class `{zero, #2, #0: Constant}`
  - U2: from class `{one, #3, #1: Constant}`
- Both upper bound sources have domain `Constant`
- Compute least upper bound: `LUB([Constant, Constant]) = Constant`
- Assign domain `Constant` to this class

```
{zero, #2, #0: Constant}, {one, #3, #1: Constant}, {sum, #5, #4, result: Constant}
```

### Final partition:
```
Class 1: {zero, #2, #0} → Constant
Class 2: {one, #3, #1} → Constant
Class 3: {sum, #5, #4, result} → Constant
```

### Solution (domain assignment):
```
Components:
  dom(zero) = Constant
  dom(one) = Constant
  dom(sum) = Constant
  dom(result) = Constant

Expressions:
  dom(#0) = Constant
  dom(#1) = Constant
  dom(#2) = Constant
  dom(#3) = Constant
  dom(#4) = Constant
  dom(#5) = Constant
```

### Verification:

**Fixed constraints:**
- `dom(#0) = Constant` ✓
- `dom(#1) = Constant` ✓

**Strict upper bound constraints:**
- `dom(zero) < Async` → `Constant < Async` ✓
- `dom(one) < Async` → `Constant < Async` ✓
- `dom(sum) < Async` → `Constant < Async` ✓
- `dom(result) < Async` → `Constant < Async` ✓

**Upper bound constraints:**
- `dom(#2) ≤ dom(#4)` → `Constant ≤ Constant` ✓
- `dom(#3) ≤ dom(#4)` → `Constant ≤ Constant` ✓

**Equality constraints:**
- All variables in Class 1 have domain `Constant` ✓
- All variables in Class 2 have domain `Constant` ✓
- All variables in Class 3 have domain `Constant` ✓

**All constraints satisfied! Solution is valid.**

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- No Clock-typed ports in this module

**Domain parameters: (none)**

This is the key edge case: a module with no clock ports has no domain parameters in its signature.

### Express port domains in terms of parameters:

From the solution, all ports have domain `Constant`:
- `dom(zero) = Constant` → concrete `Constant`
- `dom(one) = Constant` → concrete `Constant`
- `dom(result) = Constant` → concrete `Constant`

### Signature:
```rust
DomainSignature {
    domain_params: {},  // No clock ports → no domain parameters!
    port_domains: {
        zero: SignatureDomain.Concrete(Constant),
        one: SignatureDomain.Concrete(Constant),
        result: SignatureDomain.Concrete(Constant),
    }
}
```

### Interpretation:

This signature means:
- `PureConstant` has **no domain parameters** (no clock ports)
- All three ports have concrete domain `Constant`
- When instantiated, all ports must connect to signals with domain `Constant` (or signals that can unify with `Constant`)
- This module can be instantiated anywhere — `Constant` unifies with any `Sync` domain

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: PureConstant,
    component_domains: {
        zero: Constant,
        one: Constant,
        sum: Constant,
        result: Constant,
    },
    signature: DomainSignature {
        domain_params: {},
        port_domains: {
            zero: Concrete(Constant),
            one: Concrete(Constant),
            result: Concrete(Constant),
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Instantiation Example

When this module is instantiated, its `Constant` ports can connect to signals in any domain:

```virdant
mod Parent {
    incoming clock : Clock

    // Instantiate PureConstant
    mod constants of PureConstant {
        // Ports are outputs, so we read from them
    }

    // Use in a register (Sync domain)
    reg data : Word[8] on clock {
        it <= constants.result  // Constant unifies with Sync(clock)
    }

    // Use in an async wire
    wire async_data : Word[8] async {
        it := constants.zero    // Constant upcasts to Async
    }
}
```

At the instantiation site:
1. `PureConstant` has no clock ports → no domain bindings needed
2. Reading `constants.result` (domain `Constant`) into a `Sync(clock)` register:
   - Check: `compatible(Constant, Sync(clock))` → Yes ✓
   - Unify: `unify(Constant, Sync(clock))` → `Sync(clock)` (Constant weakens)
3. Reading `constants.zero` (domain `Constant`) into an `Async` wire:
   - Check: `compatible(Constant, Async)` → Yes ✓
   - Unify: `unify(Constant, Async)` → `Async` (Constant upcasts)

**Key insight**: `Constant` as the lattice bottom means constant signals work everywhere — they're the most flexible domain.

---

## Notes

### Does this work naturally with the algorithm?

**Mostly yes, with one clarification needed:**

1. **The algorithm needs to explicitly handle literals**: The current ALGORITHM.md says literals generate no constraints (free variables). However, for the domain model to make sense, literals should be assigned domain `Constant`. This example adds:
   ```python
   Fixed(literal_expr, Constant)
   ```
   for each literal expression.

2. **Upper bound LUB computation works perfectly**: When `sum` has two upper bounds from `Constant` domains, computing `LUB([Constant, Constant]) = Constant` naturally propagates the constant domain through the expression tree.

3. **No domain parameters in signature**: This cleanly demonstrates the edge case where a module has no Clock-typed ports, resulting in an empty `domain_params` map.

4. **Constant unification is elegant**: The lattice rules make `Constant` useful anywhere without special cases.

### Edge cases discovered:

1. **Literal domain assignment**: The algorithm should explicitly state that literal expressions get domain `Constant`. Currently, the pseudocode says "no constraints generated for literals," but that leaves them as free variables rather than `Constant`. The fix is simple:
   ```python
   elif expr.kind == Literal:
       expr_node = expr_id
       expr_id += 1
       variables.add(expr_node)
       fixed.append(Fixed(expr_node, Constant))  # ← ADD THIS
       return (expr_id, expr_node)
   ```

2. **Pure constant modules are valid**: A module with no clock dependencies at all is perfectly legal. Its signature has no domain parameters, and all ports have concrete `Constant` domain. This is useful for:
   - Constant generation modules
   - Lookup tables
   - Pure combinational functions
   - Test harnesses

3. **Reading from constant ports**: When a parent reads from a constant output port, the constant domain unifies with the parent's signal domain. This is different from writing to an input port (which would constrain the parent signal).

4. **Constant ≠ compile-time evaluation**: Domain `Constant` means "no clock dependency," not "known at compile time." A wire assigned a literal is still a runtime signal; its domain is `Constant` because evaluation timing doesn't depend on any clock edge.

### Design issues:

1. **Should literals be fixed to Constant?** Based on the lattice design where `Constant` is the bottom element and the description "pure expression with no signal references," literals should definitely have domain `Constant`. The algorithm needs this fix.

2. **What about other pure expressions?** If we have `wire x : Word[8] { it := 2 + 3 }`, should `2 + 3` be `Constant`? With the fixed literal rule, both operands are `Constant`, so the result is `Constant` via LUB. This is correct!

3. **Interaction with type system**: A signal with type `Word[8]` and domain `Constant` is still a runtime signal that carries constant values. The domain system is orthogonal to types.

### Clarifications needed for PLANv2.md or ALGORITHM.md:

1. **ALGORITHM.md Section "Walk Expression" - Literal case**: Change from "No constraints generated" to:
   ```python
   elif expr.kind == Literal:
       expr_node = expr_id
       expr_id += 1
       variables.add(expr_node)
       # Literals have domain Constant (bottom of lattice)
       fixed.append(Fixed(expr_node, Constant))
       return (expr_id, expr_node)
   ```

2. **PLANv2.md Clarification**: Add example showing that `Constant` domain is for clock-independent signals:
   ```virdant
   wire x : Word[8] { it := 42 }  // domain Constant
   ```
   And note that this is runtime constant value, not compile-time evaluation.

3. **ALGORITHM.md Note on empty domain_params**: Add a note in signature extraction:
   ```
   If a module has no Clock-typed ports, domain_params will be empty.
   This is legal and represents a module with no domain parameters.
   All internal domains will be concrete (Constant, Async, or error).
   ```

4. **Test coverage**: This example should be in the test suite to ensure:
   - Literals get domain `Constant`
   - Empty `domain_params` is handled correctly
   - `Constant` unifies properly with other domains at instantiation

### Comparison with Example 15:

- **Example 15**: No clocks, but has port references → ports are free variables
- **Example 16**: No clocks, only literals → everything is `Constant`
- **Key difference**: References to ports create dependencies, literals do not

Together these examples show:
- Free variables (unconstrained, polymorphic)
- Constant domain (no clock dependency, explicit)
- The difference between "unconstrained" and "constant"
