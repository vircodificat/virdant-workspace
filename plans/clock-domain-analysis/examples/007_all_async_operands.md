# Example 7: All-Async Operands with Non-Async Target (ERROR)

```virdant
mod AllAsyncOperands {
    incoming a : Bit async
    incoming b : Bit async

    wire result : Bit {
        it := a | b  // ERROR: async operands but target not declared async
    }

    outgoing out : Bit {
        it := result
    }
}
```

**What's happening**: Two signals both declared `async` are combined with a binary operator (`|`) into a target `result` that is NOT declared `async`. The binary operator creates upper bound constraints allowing operands to upcast to the result domain, but since both operands are fixed to `Async`, the result must also be `Async`. This conflicts with the target's implicit `dom(result) < Async` constraint.

**Expected error**: Strict upper bound violation when the result is inferred to be `Async` but has an implicit constraint `dom(result) < Async`.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `a` (incoming, type Bit, async modifier)
- `b` (incoming, type Bit, async modifier)
- `result` (wire, type Bit, no async modifier)
- `out` (outgoing, type Bit)

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming a : Bit async`**
- Component: `a`
- Has `async` modifier
- No expressions in declaration

**Declaration: `incoming b : Bit async`**
- Component: `b`
- Has `async` modifier
- No expressions in declaration

**Declaration: `wire result : Bit`**
- Component: `result`
- No `async` modifier (this is key!)
- No expressions in declaration

**Declaration: `outgoing out : Bit`**
- Component: `out`
- No expressions in declaration

**Driver: `result := a | b`**
- Expression `#0`: reference to `a` (left operand)
- Expression `#1`: reference to `b` (right operand)
- Expression `#2`: binary operator `|` with operands #0 and #1

**Driver: `out := result`**
- Expression `#3`: reference to `result` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `a`
- `#1`: ExprReference to `b`
- `#2`: ExprBinaryOp (a | b)
- `#3`: ExprReference to `result`

### Step 3 - Generate constraints:

**Fixed constraints from async modifier:**
- `dom(a) = Async` (explicitly declared async)
- `dom(b) = Async` (explicitly declared async)

**Equality constraints from component references:**
- Expression `#0` references `a` → `dom(#0) = dom(a)`
- Expression `#1` references `b` → `dom(#1) = dom(b)`
- Expression `#3` references `result` → `dom(#3) = dom(result)`

**Upper bound constraints from binary operator:**

The binary operator creates upper bound constraints allowing operands to upcast to the result:
- `dom(#0) ≤ dom(#2)` (left operand can upcast to result)
- `dom(#1) ≤ dom(#2)` (right operand can upcast to result)

**Equality constraints from drivers:**

Driver: `result := a | b`
- LHS: component `result`
- RHS: expression `#2` (the binary operator)
- Constraint: `dom(result) = dom(#2)`

Driver: `out := result`
- LHS: component `out`
- RHS: expression `#3`
- Constraint: `dom(out) = dom(#3)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(result) < Async` ← THE KEY CONSTRAINT
- `dom(out) < Async`
- **NOTE:** `a` and `b` do NOT get this constraint because they're declared `async`

### Complete constraint system:
```
Fixed:
  F1: dom(a) = Async
  F2: dom(b) = Async

Strict upper bound (implicit):
  S1: dom(result) < Async         ← THE CONSTRAINT THAT WILL BE VIOLATED
  S2: dom(out) < Async

Upper bound (from binary operator):
  U1: dom(#0) ≤ dom(#2)
  U2: dom(#1) ≤ dom(#2)

Equality:
  E1: dom(#0) = dom(a)
  E2: dom(#1) = dom(b)
  E3: dom(#3) = dom(result)
  E4: dom(result) = dom(#2)       ← CONNECTS TARGET TO OPERATOR
  E5: dom(out) = dom(#3)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment (or detect the error).

### Initial state:
All components and expressions start in separate equivalence classes:
```
{a}, {b}, {result}, {out}, {#0}, {#1}, {#2}, {#3}
```

### Processing fixed constraint F1: `dom(a) = Async`

Assign concrete domain `Async` to a's equivalence class:
```
{a: Async}, {b}, {result}, {out}, {#0}, {#1}, {#2}, {#3}
```

### Processing fixed constraint F2: `dom(b) = Async`

Assign concrete domain `Async` to b's equivalence class:
```
{a: Async}, {b: Async}, {result}, {out}, {#0}, {#1}, {#2}, {#3}
```

### Processing equality constraint E1: `dom(#0) = dom(a)`

Merge expression #0 with a's equivalence class:
```
{a, #0: Async}, {b: Async}, {result}, {out}, {#1}, {#2}, {#3}
```

### Processing equality constraint E2: `dom(#1) = dom(b)`

Merge expression #1 with b's equivalence class:
```
{a, #0: Async}, {b, #1: Async}, {result}, {out}, {#2}, {#3}
```

### Processing equality constraint E3: `dom(#3) = dom(result)`

Merge expression #3 with result's equivalence class:
```
{a, #0: Async}, {b, #1: Async}, {result, #3}, {out}, {#2}
```

### Processing equality constraint E4: `dom(result) = dom(#2)`

Merge result's class with #2's equivalence class:
```
{a, #0: Async}, {b, #1: Async}, {result, #3, #2}, {out}
```

### Processing equality constraint E5: `dom(out) = dom(#3)`

Merge out with #3's class (which includes result and #2):
```
{a, #0: Async}, {b, #1: Async}, {result, #3, #2, out}
```

**State after equality constraints:**
```
Class 1: {a, #0} → Async
Class 2: {b, #1} → Async
Class 3: {result, #3, #2, out} → (no domain yet)
```

### Processing upper bound constraints:

Now we need to resolve domains for unresolved classes that have upper bound constraints.

**Upper bound U1: `dom(#0) ≤ dom(#2)`**
- `#0` is in Class 1 with domain `Async`
- `#2` is in Class 3 with no domain yet
- This means: `Async ≤ dom(#2)`
- Since `Async` is the top of the lattice (excluding fixed-only domains like Clock/Reset), this means `dom(#2) = Async`

**Upper bound U2: `dom(#1) ≤ dom(#2)`**
- `#1` is in Class 2 with domain `Async`
- `#2` is in Class 3 with no domain yet
- This means: `Async ≤ dom(#2)`
- This also requires `dom(#2) = Async`

**Computing least upper bound for Class 3:**
- Class 3 contains: `{result, #3, #2, out}`
- Upper bounds on this class: both U1 and U2 require the domain to be at least `Async`
- Since `Async` is the top of the lattice, the only domain that satisfies `Async ≤ d` is `Async` itself
- Assign: `dom(Class 3) = Async`

**State after resolving upper bounds:**
```
Class 1: {a, #0} → Async
Class 2: {b, #1} → Async
Class 3: {result, #3, #2, out} → Async
```

### Checking strict upper bound constraints:

Now check the implicit strict upper bound constraints.

**Strict upper bound S1: `dom(result) < Async`** ⚠️ ERROR

- `result` is in Class 3 with domain `Async`
- Check: `strictly_less_than(Async, Async)`?
  - `Async == Async`? Yes
  - Return `False` (not strictly less than)
- **Result: VIOLATION** ❌

**ERROR DETECTED**: Strict upper bound violation

Error message:
```
Strict upper bound violation for component `result`:
  Inferred domain: Async
  Required: dom(result) < Async

The wire `result` is computed from the expression `a | b`, where both
operands have domain Async. Since all operands are async, the result
must also be Async. However, `result` is not declared with the `async`
modifier.

Help: To allow async operands, declare the target as async:
  wire result : Bit async { it := a | b }
```

**Strict upper bound S2: `dom(out) < Async`** ⚠️ ERROR

- `out` is also in Class 3 with domain `Async`
- Check: `strictly_less_than(Async, Async)`?
  - Return `False`
- **Result: VIOLATION** ❌

Error message:
```
Strict upper bound violation for component `out`:
  Inferred domain: Async
  Required: dom(out) < Async

The outgoing port `out` is assigned from `result`, which has domain
Async. Since `out` is not declared with the `async` modifier, it
cannot have an async domain.

Help: Declare `out` as async:
  outgoing out : Bit async { ... }
```

### Solver terminates with errors

The solver completes the union-find process but detects violations when checking strict upper bounds.

**Solve result:**
```python
SolveResult {
    union_find: {
        Class 1: {a, #0} → Async,
        Class 2: {b, #1} → Async,
        Class 3: {result, #3, #2, out} → Async,
    },
    errors: [
        "Strict upper bound violation: result has Async, required < Async",
        "Strict upper bound violation: out has Async, required < Async"
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
    moddef: AllAsyncOperands,
    component_domains: {
        a: Async,
        b: Async,
        result: Async,  // inferred but violates constraint
        out: Async,     // inferred but violates constraint
    },
    signature: /* invalid */,
    diagnostics: [
        Diagnostic {
            severity: Error,
            message: "Strict upper bound violation: async operands require async target",
            location: /* driver: result := a | b */,
            help: "Declare `result` with the `async` modifier: wire result : Bit async",
        },
        Diagnostic {
            severity: Error,
            message: "Strict upper bound violation: out has async domain but not declared async",
            location: /* declaration: outgoing out : Bit */,
            help: "Declare `out` with the `async` modifier: outgoing out : Bit async",
        }
    ],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes, extremely naturally.** This is one of the cleanest error patterns:

1. **Fixed constraints establish operand domains**: Both `a` and `b` are fixed to `Async` by their `async` modifiers.

2. **Upper bound constraints propagate through the operator**: The binary operator creates `dom(operand) ≤ dom(result)`, which forces the result to be at least as general as the operands.

3. **Least upper bound resolution**: When computing the domain for the unresolved result class, the algorithm finds that it must be `≥ Async` from both operands. Since `Async` is the top of the lattice, the result must be `Async`.

4. **Strict upper bound check catches the violation**: After solving, the algorithm checks the implicit `dom(result) < Async` constraint and finds `Async < Async` is false, triggering the error.

**The lattice structure makes this automatic**: The upper bound constraints from the binary operator naturally propagate `Async` upward. No special case logic needed.

### Edge cases or design issues discovered?

**Cascading errors**: Notice that both `result` and `out` are flagged with violations. This makes sense:
- `result` is the primary error (async operands, non-async target)
- `out` inherits the async domain through equality constraints
- Both violations are real and should be reported

However, **error message quality matters**:
- The primary error is at `result := a | b` (the operator combining async operands)
- The secondary error at `out` is a consequence
- The compiler might want to prioritize or group these errors in the diagnostic output

**Alternative fix strategies**:
1. Declare `result` as async (preferred, makes the intent explicit)
2. Declare both `result` and `out` as async (if the module's interface should be async)
3. Use synchronizers to bring `a` and `b` into a clock domain first (if they need to be synchronized)

### Clarifications needed for PLANv2.md or ALGORITHM.md?

**1. Upper bound constraint resolution is well-defined**: The algorithm correctly handles the case where multiple upper bounds force a domain upward to `Async`. The least upper bound of `{Async, Async}` is `Async`.

**2. Order of error checking matters**: The algorithm must:
   - First process fixed and equality constraints
   - Then resolve upper bounds
   - Finally check strict upper bounds

   This order ensures that domains are fully resolved before checking violations.

**3. The "async must be explicit" rule is enforced by strict upper bounds**: Every component without the `async` modifier gets an implicit `dom(c) < Async` constraint. This is the mechanism that prevents implicit async inference.

**4. Error message improvement suggestion**: The diagnostics could benefit from tracking the *reason* a domain was assigned during solving. For example:
   ```
   result has domain Async because:
     - a | b creates upper bound constraints
     - both operands (a, b) are Async
     - least upper bound of {Async, Async} is Async
   ```

**5. The binary operator rule is crucial**: The upper bound constraints `dom(operand) ≤ dom(result)` are what allow upcasting. Without these, the example would generate equality constraints instead, causing a unification error (like example 006) rather than a strict upper bound violation.

**Proposed clarification for ALGORITHM.md**:

Add to the "Strict upper bound checking" section:
```
Note: Strict upper bound violations occur after full constraint resolution.
This means a component can be inferred to have domain Async through upper
bound propagation, and the violation is detected when checking the implicit
constraint, not during unification. This is distinct from unification errors
which occur when merging incompatible domains during equality constraint
processing.
```

**Overall assessment**: The algorithm handles this case perfectly. The three-pass structure cleanly separates:
1. Constraint generation (captures the operator semantics and implicit restrictions)
2. Solving (propagates domains through upper bounds)
3. Validation (detects the conflict between inferred and required domains)

This is exactly the kind of error the system was designed to catch, and it does so elegantly.
