## Example 5: Mixing Clocks Without Async (ERROR)

```virdant
mod BadMixing {
    incoming clk_a : Clock
    incoming clk_b : Clock

    reg r_a : Bit on clk_a {
        it <= false
    }

    reg r_b : Bit on clk_b {
        it <= false
    }

    wire mixed : Bit {
        it := r_a | r_b
    }
}
```

**What's happening**: Two registers on different clocks are combined via `|` into a target that is NOT declared `async`. The `|` operator's upper bound constraints force the result to be `Async` (the only common ancestor of the two `Sync` domains in the lattice), but the target's implicit strict upper bound `dom(mixed) < Async` forbids it. This is a domain unification error.

**Error condition**: REQUIREMENTS §8.2 — "When operands in a combinational expression have different non-`Constant` domains, and the target is not declared `async`, this is an error."

---

### Pass 1: Constraint Generation

**Step 1 — Components from ComponentAnalysis:**
- `clk_a` (incoming, type Clock)
- `clk_b` (incoming, type Clock)
- `r_a` (reg, type Bit)
- `r_b` (reg, type Bit)
- `mixed` (wire, type Bit)

**Step 2 — Walk declarations, identify expressions:**

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

Declaration: `wire mixed : Bit`
- Component: `mixed` (NOT declared async)
- No expressions in declaration

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

**Step 3 — Generate constraints:**

**Fixed constraints from component types:**
- `dom(clk_a) = Clock`
- `dom(clk_b) = Clock`

**Fixed constraints from register declarations:**
- `dom(r_a) = Sync(ComponentId(clk_a))`
- `dom(r_b) = Sync(ComponentId(clk_b))`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clk_a) < Async`
- `dom(clk_b) < Async`
- `dom(r_a) < Async`
- `dom(r_b) < Async`
- `dom(mixed) < Async`  ⚠ — mixed is NOT declared async

**Equality constraints from component references:**
- `dom(#0) = dom(clk_a)`
- `dom(#2) = dom(clk_b)`
- `dom(#4) = dom(r_a)`
- `dom(#5) = dom(r_b)`

**Equality constraints from drivers (register assignments use `<=`):**
- `dom(r_a) = dom(#1)`
- `dom(r_b) = dom(#3)`

**Upper bound constraints from binary operator (UNIFIED RULE):**
- `dom(#4) ≤ dom(#6)` (left operand can upcast to result)
- `dom(#5) ≤ dom(#6)` (right operand can upcast to result)

**Equality constraint from driver (`mixed := r_a | r_b`):**
- LHS: component `mixed`
- RHS: expression `#6`
- `dom(mixed) = dom(#6)`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clk_a) = Clock
  C₂: dom(clk_b) = Clock
  C₃: dom(r_a) = Sync(ComponentId(clk_a))
  C₄: dom(r_b) = Sync(ComponentId(clk_b))

Strict upper bound (implicit):
  C₅: dom(clk_a) < Async
  C₆: dom(clk_b) < Async
  C₇: dom(r_a) < Async
  C₈: dom(r_b) < Async
  C₉: dom(mixed) < Async    ⚠ Key constraint!

Equality:
  C₁₀: dom(#0) = dom(clk_a)
  C₁₁: dom(#2) = dom(clk_b)
  C₁₂: dom(#4) = dom(r_a)
  C₁₃: dom(#5) = dom(r_b)
  C₁₄: dom(r_a) = dom(#1)
  C₁₅: dom(r_b) = dom(#3)

Upper bound (from `|` operator):
  C₁₆: dom(#4) ≤ dom(#6)
  C₁₇: dom(#5) ≤ dom(#6)

Equality (from driver):
  C₁₈: dom(mixed) = dom(#6)
```

---

### Pass 2: Constraint Solving

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clk_a}, {clk_b}, {r_a}, {r_b}, {mixed}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraints C₁–C₄:**
```
{clk_a: Clock}, {clk_b: Clock}, {r_a: Sync(clk_a)}, {r_b: Sync(clk_b)},
{mixed}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing equality constraints C₁₀–C₁₅:**

C₁₀: `dom(#0) = dom(clk_a)` — merge #0 with clk_a:
```
{clk_a, #0: Clock}, {clk_b: Clock}, {r_a: Sync(clk_a)}, {r_b: Sync(clk_b)},
{mixed}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

C₁₁: `dom(#2) = dom(clk_b)` — merge #2 with clk_b:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a: Sync(clk_a)}, {r_b: Sync(clk_b)},
{mixed}, {#1}, {#3}, {#4}, {#5}, {#6}
```

C₁₂: `dom(#4) = dom(r_a)` — merge #4 with r_a:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #4: Sync(clk_a)},
{r_b: Sync(clk_b)}, {mixed}, {#1}, {#3}, {#5}, {#6}
```

C₁₃: `dom(#5) = dom(r_b)` — merge #5 with r_b:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #4: Sync(clk_a)},
{r_b, #5: Sync(clk_b)}, {mixed}, {#1}, {#3}, {#6}
```

C₁₄: `dom(r_a) = dom(#1)` — merge #1 with r_a's class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)},
{r_b, #5: Sync(clk_b)}, {mixed}, {#3}, {#6}
```

C₁₅: `dom(r_b) = dom(#3)` — merge #3 with r_b's class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)},
{r_b, #3, #5: Sync(clk_b)}, {mixed}, {#6}
```

C₁₈: `dom(mixed) = dom(#6)` — merge #6 with mixed:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)},
{r_b, #3, #5: Sync(clk_b)}, {mixed, #6}
```

**Equality constraints exhausted. Remaining classes:**
```
Class 1: {clk_a, #0} → Clock
Class 2: {clk_b, #2} → Clock
Class 3: {r_a, #1, #4} → Sync(ComponentId(clk_a))
Class 4: {r_b, #3, #5} → Sync(ComponentId(clk_b))
Class 5: {mixed, #6} → (no concrete domain yet)
```

**Now check upper bound constraints C₁₆ and C₁₇:**

C₁₆: `dom(#4) ≤ dom(#6)`
- dom(#4) is in Class 3 → `Sync(clk_a)`
- dom(#6) is in Class 5 → currently unassigned
- We need to find the minimal (most specific) domain for dom(#6) that satisfies all constraints.

C₁₇: `dom(#5) ≤ dom(#6)`
- dom(#5) is in Class 4 → `Sync(clk_b)`
- Same requirement: find a domain X for dom(#6) such that `Sync(clk_a) ≤ X` AND `Sync(clk_b) ≤ X`

**Finding the least upper bound:**

The upper bound constraints require:
- `Sync(clk_a) ≤ dom(#6)`
- `Sync(clk_b) ≤ dom(#6)`

In the lattice:
```
         Async (top)
        /       \
  Sync(clk_a)  Sync(clk_b)
        \       /
      Constant (bottom)
```

The **least upper bound** of `Sync(clk_a)` and `Sync(clk_b)` is `Async`. There is no domain strictly between them that is ≥ both. Constant is below them, and they are incomparable siblings.

Therefore, the only domain for dom(#6) that satisfies both upper bound constraints is `Async`.

**Assign dom(#6) = Async (via least upper bound reasoning):**
```
Class 5: {mixed, #6} → Async (tentative)
```

**Candidate (tentative) solution:**
```
Components:
  dom(clk_a) = Clock
  dom(clk_b) = Clock
  dom(r_a) = Sync(ComponentId(clk_a))
  dom(r_b) = Sync(ComponentId(clk_b))
  dom(mixed) = Async    ⚠ Derived through LUB

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clk_a))
  dom(#2) = Clock
  dom(#3) = Sync(ComponentId(clk_b))
  dom(#4) = Sync(ComponentId(clk_a))
  dom(#5) = Sync(ComponentId(clk_b))
  dom(#6) = Async
```

---

### Error Detection: Strict Upper Bound Violation

**Check C₉: `dom(mixed) < Async`**

From the candidate solution: `dom(mixed) = Async`

`Async < Async`? **No.** Strict inequality `d₁ < d₂` requires `d₁ ≤ d₂` AND `d₁ ≠ d₂`. Since `Async = Async`, this is `Async = Async`, not strictly less. **VIOLATION.**

**Root cause**: The `|` operator forced `dom(#6) = Async` (the only domain ≥ both `Sync(clk_a)` and `Sync(clk_b)`), and since `dom(mixed) = dom(#6)`, the target `mixed` ended up with domain `Async`. But `mixed` was not declared `async`, so the implicit `dom(mixed) < Async` constraint is violated.

**Diagnostic produced:**
```
Error: signal `mixed` is implicitly async but not declared `async`
  ┌─ BadMixing.vir:9:10
  │
 9 │     wire mixed : Bit {
  │          ^^^^^
  │         ╎ wire `mixed` is not declared async
  │         ╎ but is driven by signals from incompatible clock domains
  │
 12 │         it := r_a | r_b
  │               ^^^^^^^^^^^
  │         ╎   r_a is Sync(clk_a)
  │         ╎   r_b is Sync(clk_b)
  │         ╎   these domains cannot be unified without an explicit async declaration
  │
  │   suggestion: add `async` modifier → `wire mixed : Bit async { ... }`
```

**Why this is not a unification error for the operator itself:**
The `|` operator's unified rule (`dom(operand) ≤ dom(result)`) is designed to handle this case gracefully. The operator itself doesn't error — it allows both operands to upcast to `Async`. The error comes from the **implicit strict upper bound** on the target signal, which enforces the "async must be explicit" rule.

---

### Comparison with Legal Case (Example 4)

| Aspect | Example 4 (legal) | Example 5 (error) |
|--------|-------------------|-------------------|
| Target | `wire mixed : Bit async` | `wire mixed : Bit` |
| Mixed's domain | `Async` (explicit) | `Async` (derived, implicit) |
| Has `dom(mixed) < Async`? | No (explicit async removes it) | Yes (implicit, all non-async) |
| Result | All constraints satisfied ✓ | C₉ violated ✗ |
| Fix | — | Add `async` or redesign |

The **only** difference between Example 4 and Example 5 is the `async` keyword on `mixed`. Every other constraint is identical. This demonstrates that the "async must be explicit" rule is enforced through the strict upper bound constraint alone — the rest of the constraint system operates uniformly regardless.

---

### Pass 3: Signature Extraction

**Note**: Since the module has a domain error, a valid `DomainAnalysis` cannot be produced for compilation purposes. However, we can still extract partial information for error reporting.

**Domain parameters:**
- `clk_a` introduces `DomainVar(0)` (α)
- `clk_b` introduces `DomainVar(1)` (β)

**Port domains (if ignoring the error):**
- `dom(clk_a) = Clock` → Clock
- `dom(clk_b) = Clock` → Clock
- `dom(mixed) = Async` → would need `async` declaration

The error prevents forming a valid polymorphic signature. The module cannot be instantiated until the domain conflict is resolved.

---

### Summary

- **Error**: Two signals from incompatible `Sync` domains combined via `|` without an explicit `async` target
- **Constraint mechanism**: Upper bound constraints force `dom(result) = Async` (the LUB), which violates the strict upper bound `dom(target) < Async`
- **Fix**: Either add `async` to the target (if cross-domain mixing is intended) or restructure to avoid mixing incompatible clock domains