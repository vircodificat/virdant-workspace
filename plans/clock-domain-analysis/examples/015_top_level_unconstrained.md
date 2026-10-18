# Example 15: Top-Level Unconstrained Ports

```virdant
mod TopLevel {
    incoming a : Word[8]
    incoming b : Word[8]

    wire sum : Word[8] {
        it := a + b
    }

    outgoing result : Word[8] {
        it := sum
    }
}
```

**What's happening**: A top-level module with no explicit clock declarations. All ports and signals flow through combinational logic without any clock constraints. The domain variables remain free (unconstrained) in the final signature.

This is completely legal at the top level — the module is polymorphic over clock domains. When instantiated, the parent's clock domain will flow through via port connections.

**Expected result**: All components end up in a single equivalence class with no concrete domain assigned. Signature has free domain variables.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `a` (incoming, type Word[8])
- `b` (incoming, type Word[8])
- `sum` (wire, type Word[8])
- `result` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming a : Word[8]`**
- Component: `a`
- No expressions in declaration

**Declaration: `incoming b : Word[8]`**
- Component: `b`
- No expressions in declaration

**Declaration: `wire sum : Word[8]`**
- Component: `sum`
- No expressions in declaration

**Driver: `sum := a + b`**
- Expression `#0`: reference to `a` (left operand)
- Expression `#1`: reference to `b` (right operand)
- Expression `#2`: binary operator `+` (result of `a + b`)

**Declaration: `outgoing result : Word[8]`**
- Component: `result`
- No expressions in declaration

**Driver: `result := sum`**
- Expression `#3`: reference to `sum` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `a`
- `#1`: ExprReference to `b`
- `#2`: ExprBinaryOp `+` with operands `#0` and `#1`
- `#3`: ExprReference to `sum`

### Step 3 - Generate constraints:

**No fixed constraints:**
- No Clock-typed signals
- No registers with `on` clauses
- No async modifiers

**Equality constraints from component references:**
- Expression `#0` references `a` → `dom(#0) = dom(a)`
- Expression `#1` references `b` → `dom(#1) = dom(b)`
- Expression `#3` references `sum` → `dom(#3) = dom(sum)`

**Upper bound constraints from binary operator:**

For binary operator `a + b` (expression `#2`):
- Left operand `#0` can upcast to result: `dom(#0) ≤ dom(#2)`
- Right operand `#1` can upcast to result: `dom(#1) ≤ dom(#2)`

**Equality constraints from drivers:**

Driver: `sum := a + b`
- LHS: component `sum`
- RHS: expression `#2`
- Constraint: `dom(sum) = dom(#2)`

Driver: `result := sum`
- LHS: component `result`
- RHS: expression `#3`
- Constraint: `dom(result) = dom(#3)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(a) < Async`
- `dom(b) < Async`
- `dom(sum) < Async`
- `dom(result) < Async`

### Complete constraint system:
```
Fixed: (none)

Strict upper bound (implicit):
  S1: dom(a) < Async
  S2: dom(b) < Async
  S3: dom(sum) < Async
  S4: dom(result) < Async

Upper bound (from operators):
  U1: dom(#0) ≤ dom(#2)
  U2: dom(#1) ≤ dom(#2)

Equality:
  E1: dom(#0) = dom(a)
  E2: dom(#1) = dom(b)
  E3: dom(#3) = dom(sum)
  E4: dom(sum) = dom(#2)
  E5: dom(result) = dom(#3)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{a}, {b}, {sum}, {result}, {#0}, {#1}, {#2}, {#3}
```

### Processing fixed constraints:
No fixed constraints to process.

### Processing equality constraint E1: `dom(#0) = dom(a)`

Merge expression #0 with component a's equivalence class:
```
{a, #0}, {b}, {sum}, {result}, {#1}, {#2}, {#3}
```

### Processing equality constraint E2: `dom(#1) = dom(b)`

Merge expression #1 with component b's equivalence class:
```
{a, #0}, {b, #1}, {sum}, {result}, {#2}, {#3}
```

### Processing equality constraint E3: `dom(#3) = dom(sum)`

Merge expression #3 with component sum's equivalence class:
```
{a, #0}, {b, #1}, {sum, #3}, {result}, {#2}
```

### Processing equality constraint E4: `dom(sum) = dom(#2)`

Merge sum's class with expression #2's equivalence class:
```
{a, #0}, {b, #1}, {sum, #3, #2}, {result}
```

### Processing equality constraint E5: `dom(result) = dom(#3)`

Merge result with #3's class:
```
{a, #0}, {b, #1}, {sum, #3, #2, result}
```

### Processing upper bound constraint U1: `dom(#0) ≤ dom(#2)`

Find representatives:
- `find(#0)` → class `{a, #0}` (no concrete domain)
- `find(#2)` → class `{sum, #3, #2, result}` (no concrete domain)

Since both classes are unresolved, we can't determine a least upper bound yet. The constraint indicates that if these classes remain separate, `#0`'s domain must be at most as specific as `#2`'s domain.

However, note: We need to check if there's a path to unify them. Let's trace:
- `#0` is in class with `a`
- `#2` is in class with `sum`, `#3`, `result`

These are separate classes. The upper bound constraint means if we assign concrete domains, we need `dom(a) ≤ dom(sum)`.

### Processing upper bound constraint U2: `dom(#1) ≤ dom(#2)`

Find representatives:
- `find(#1)` → class `{b, #1}` (no concrete domain)
- `find(#2)` → class `{sum, #3, #2, result}` (no concrete domain)

Same situation: separate classes with no concrete domains.

### Resolving with upper bounds:

Since we have upper bound constraints but no fixed constraints, we cannot determine a concrete domain. The algorithm looks for least upper bounds (LUB) to assign to unresolved classes.

For class `{a, #0}`:
- Upper bounds: `#0 ≤ #2`, and `#2` is in class `{sum, #3, #2, result}`
- But that class also has no concrete domain!
- Without a concrete domain to serve as an upper bound, the class remains unresolved

For class `{b, #1}`:
- Upper bounds: `#1 ≤ #2`, same situation
- Class remains unresolved

For class `{sum, #3, #2, result}`:
- Has two incoming upper bounds from `#0` and `#1`
- But no concrete domain as target
- Class remains unresolved

**Key insight**: The upper bound constraints create relationships between classes, but without any fixed constraint to anchor them, all classes remain free variables.

### Checking strict upper bounds:

All components have `dom(component) < Async`:
- Class `{a, #0}`: domain = None → `None < Async` ✓ (unresolved satisfies strict upper bound)
- Class `{b, #1}`: domain = None → `None < Async` ✓
- Class `{sum, #3, #2, result}`: domain = None → `None < Async` ✓

All strict upper bound constraints are satisfied because unresolved domains are implicitly compatible (they haven't been fixed to `Async` or anything incompatible).

### Final partition:
```
Class 1: {a, #0} → None (unresolved)
Class 2: {b, #1} → None (unresolved)
Class 3: {sum, #3, #2, result} → None (unresolved)
```

### Solution (domain assignment):
```
Components:
  dom(a) = ⊥ (free variable)
  dom(b) = ⊥ (free variable)
  dom(sum) = ⊥ (free variable)
  dom(result) = ⊥ (free variable)

Expressions:
  dom(#0) = ⊥ (free variable)
  dom(#1) = ⊥ (free variable)
  dom(#2) = ⊥ (free variable)
  dom(#3) = ⊥ (free variable)
```

**Verification:**
- All fixed constraints satisfied: ✓ (no fixed constraints)
- All strict upper bound constraints satisfied: ✓ (free variables satisfy `< Async`)
- All equality constraints satisfied: ✓ (variables in same class)
- All upper bound constraints satisfied: ✓ (no concrete domains to violate bounds)

**No errors** — the constraint system is satisfiable with all variables remaining free.

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- No Clock-typed ports in this module

Domain parameters: (none)

### Express port domains in terms of parameters:

From the solution, all ports have unresolved domains:
- `dom(a)` = None → free variable
- `dom(b)` = None → free variable
- `dom(result)` = None → free variable

Since there are no clock ports to introduce domain variables, and all ports are unconstrained, each port gets a free domain variable in the signature.

### Signature:
```rust
DomainSignature {
    domain_params: {},  // No clock ports
    port_domains: {
        a: SignatureDomain.Free(),
        b: SignatureDomain.Free(),
        result: SignatureDomain.Free(),
    }
}
```

### Interpretation:

This signature means:
- `TopLevel` is polymorphic over clock domains
- All three ports have independent free domain variables
- When instantiated, the parent's concrete domains will flow through connections

However, note the **constraint relationships** that must be maintained:
- Due to the binary operator, `a` and `b` can upcast to `sum`
- Due to equality, `sum` = `result`

So while all ports are "free," they're not completely independent. A more precise signature might track these relationships, but the current algorithm treats them as separate free variables.

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: TopLevel,
    component_domains: {
        a: None,        // Free variable
        b: None,        // Free variable
        sum: None,      // Free variable
        result: None,   // Free variable
    },
    signature: DomainSignature {
        domain_params: {},
        port_domains: {
            a: Free(),
            b: Free(),
            result: Free(),
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Instantiation Example

When this module is instantiated, parent signals flow through:

```virdant
mod Parent {
    incoming clock : Clock
    reg x : Word[8] on clock { ... }
    reg y : Word[8] on clock { ... }

    // Instantiate TopLevel
    mod top of TopLevel {
        it.a := x       // x is Sync(clock), binds a's free variable
        it.b := y       // y is Sync(clock), binds b's free variable
        // result inherits Sync(clock)
    }

    reg output : Word[8] on clock {
        it <= top.result  // result is Sync(clock), matches register
    }
}
```

At the instantiation site:
1. Connection `it.a := x` unifies `Free()` with `Sync(clock)` → `a` gets `Sync(clock)`
2. Connection `it.b := y` unifies `Free()` with `Sync(clock)` → `b` gets `Sync(clock)`
3. Due to internal constraints, `sum` and `result` also get `Sync(clock)`
4. Connection `it <= top.result` checks `Sync(clock)` = `Sync(clock)` ✓

---

## Notes

### Does this work naturally with the algorithm?

**Yes, very naturally.** The algorithm handles unconstrained ports elegantly:
1. No fixed constraints → all variables start free
2. Equality constraints group variables but don't fix them
3. Upper bound constraints establish relationships but don't force concrete domains
4. Free variables pass all strict upper bound checks (implicitly `< Async`)
5. Signature extraction marks them as `SignatureDomain.Free()`

### Edge cases discovered:

1. **Separate free variables**: In this example, `a` and `b` end up in different equivalence classes from `sum/result`. The signature shows all three ports as independent free variables. This is conservative — the internal constraints `a ≤ sum`, `b ≤ sum` aren't captured in the signature.

2. **Upper bound constraints without anchors**: When all classes are unresolved, upper bound constraints `dom(x) ≤ dom(y)` create a partial order but no concrete assignment. The algorithm correctly leaves them all as None.

3. **Is this too permissive?** Consider:
   ```virdant
   mod top of TopLevel {
       it.a := sig_on_clk_A
       it.b := sig_on_clk_B
       // What domain is result?
   }
   ```

   With `a` on clock A, `b` on clock B, the binary operator `a + b` would try to unify different clocks. But at the instantiation site, the constraint `dom(a) = dom(#0)`, `dom(b) = dom(#1)`, `dom(#0) ≤ dom(#2)`, `dom(#1) ≤ dom(#2)` would flow through.

   Since `#2` (the operator result) is in the same class as `sum` and `result`, the LUB of `Sync(A)` and `Sync(B)` would be... `Async`? But `result` has constraint `< Async`, so this would error.

   **This is correct behavior!** The module's internal constraint `result < Async` (implicit on all non-async components) prevents mixing clock domains unless the result is declared async.

4. **Alternative: Single free variable for all?** Could we unify all three ports into one free variable earlier? Let's trace:
   - `E1: a = #0`, `E2: b = #1`, so `a` and `b` start separate
   - `U1: #0 ≤ #2`, `U2: #1 ≤ #2`, but upper bounds don't force equality
   - Only if we had `E: a = b` would they merge

   The algorithm correctly keeps them separate because the binary operator allows upcasting (async target scenario), not just equality.

### Design issues:

1. **Upper bound without LUB**: The current algorithm only computes LUB when there are concrete domains in the upper bound set. With all free variables, it leaves everything unresolved. This is correct but means the signature loses information about relationships.

2. **Should signatures track constraints?** The signature currently just says "three free ports." It doesn't encode "result depends on a and b." This is probably fine — instantiation will re-solve constraints in the parent context.

3. **Free vs. parametric**: `SignatureDomain.Free()` means "unconstrained, will be determined at instantiation." This is different from `SignatureDomain.Var(DomainVar)` which means "parametric over this clock." The distinction matters:
   - `Var(α)` → multiple ports share the same variable, must all be on the same clock
   - `Free()` → independent variables, can be different (but constrained by internal logic)

### Clarifications for PLANv2.md or ALGORITHM.md:

1. **Clarify**: "Free variables at top level are legal" is specified in PLANv2, but ALGORITHM.md could be more explicit about:
   - When a class remains unresolved (None domain)
   - That unresolved domains satisfy strict upper bounds
   - The distinction between Free() and Var() in signatures

2. **Add**: A note in ALGORITHM.md about upper bound LUB computation:
   ```
   For unresolved classes with upper bounds on other unresolved classes,
   no LUB can be computed. The class remains unresolved (None).
   This is correct: free variables can unify with anything later.
   ```

3. **Consider**: Should the signature include constraint metadata? For example:
   ```rust
   DomainSignature {
       domain_params: {},
       port_domains: {
           a: Free(0),      // different free var ID
           b: Free(1),      // different free var ID
           result: Free(2), // different free var ID
       },
       constraints: [
           UpperBound(Free(0), Free(2)),
           UpperBound(Free(1), Free(2)),
       ]
   }
   ```

   This would make instantiation checking more precise. But it adds complexity, and re-solving in parent context works too.

4. **Test case**: This example should definitely be in the test suite — it's a common pattern (pure combinational modules with no explicit clocking).

### Relationship to existing tests:

This complements:
- Example 001 (single clock) — shows the other extreme, fully constrained
- Example 006 (error case) — shows constraint violation
- This example (015) — shows the "anything goes" case with no constraints

Together they cover the spectrum: fully constrained, unconstrained, and invalid.
