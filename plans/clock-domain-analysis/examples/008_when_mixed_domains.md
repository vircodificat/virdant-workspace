# Example 8: When Expression with Mixed Domains (ERROR)

```virdant
mod WhenMixedDomains {
    incoming clk_a : Clock
    incoming clk_b : Clock

    reg flag_a : Bit on clk_a {
        it <= false
    }

    reg flag_b : Bit on clk_b {
        it <= false
    }

    wire result : Bit {
        it := when {
            case flag_a => true
            case flag_b => false
        }
    }
}
```

**What's happening**: A `when` expression uses guards from two different clock domains (`flag_a` on `clk_a`, `flag_b` on `clk_b`), and the result `result` is not declared `async`. According to the `when` expression rules, all guards and all arms must have the same domain as the result. This creates equality constraints forcing `Sync(clk_a) = Sync(clk_b)`, which fails because these are incompatible domains.

**Error condition**: PLANv2.md §8.5 — "In a `when` or `match` expression, if operands cannot unify to a single domain (and the result is not declared `async`), this is an error."

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clk_a` (incoming, type Clock)
- `clk_b` (incoming, type Clock)
- `flag_a` (reg, type Bit)
- `flag_b` (reg, type Bit)
- `result` (wire, type Bit)

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clk_a : Clock`**
- Component: `clk_a`
- Type: Clock
- No expressions

**Declaration: `incoming clk_b : Clock`**
- Component: `clk_b`
- Type: Clock
- No expressions

**Declaration: `reg flag_a : Bit on clk_a`**
- Component: `flag_a`
- Has `on clk_a` clause
- Expression `#0`: reference to `clk_a` (in the `on clk_a` clause)

**Driver: `flag_a <= false`**
- Expression `#1`: literal `false`

**Declaration: `reg flag_b : Bit on clk_b`**
- Component: `flag_b`
- Has `on clk_b` clause
- Expression `#2`: reference to `clk_b` (in the `on clk_b` clause)

**Driver: `flag_b <= false`**
- Expression `#3`: literal `false`

**Declaration: `wire result : Bit`**
- Component: `result`
- No `async` modifier
- No expressions in declaration

**Driver: `result := when { ... }`**
- Expression `#4`: reference to `flag_a` (guard 1)
- Expression `#5`: literal `true` (arm 1)
- Expression `#6`: reference to `flag_b` (guard 2)
- Expression `#7`: literal `false` (arm 2)
- Expression `#8`: the `when` expression result

**Expression inventory:**
- `#0`: ExprReference to `clk_a`
- `#1`: Literal `false`
- `#2`: ExprReference to `clk_b`
- `#3`: Literal `false`
- `#4`: ExprReference to `flag_a` (guard 1)
- `#5`: Literal `true` (arm 1)
- `#6`: ExprReference to `flag_b` (guard 2)
- `#7`: Literal `false` (arm 2)
- `#8`: When expression result

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clk_a) = Clock`
- `dom(clk_b) = Clock`

**Fixed constraints from register declarations:**
- `dom(flag_a) = Sync(ComponentId(clk_a))`
- `dom(flag_b) = Sync(ComponentId(clk_b))`

**Equality constraints from component references:**
- `dom(#0) = dom(clk_a)`
- `dom(#2) = dom(clk_b)`
- `dom(#4) = dom(flag_a)`
- `dom(#6) = dom(flag_b)`

**Equality constraints from register drivers:**
- `dom(flag_a) = dom(#1)`
- `dom(flag_b) = dom(#3)`

**Equality constraints from `when` expression:**

According to ALGORITHM.md, for a `when` expression:
> All guards and all arms must equal result

This generates the following constraints:
- `dom(#4) = dom(#8)` (guard 1 equals result)
- `dom(#5) = dom(#8)` (arm 1 equals result)
- `dom(#6) = dom(#8)` (guard 2 equals result)
- `dom(#7) = dom(#8)` (arm 2 equals result)

**Equality constraint from driver:**
- `dom(result) = dom(#8)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clk_a) < Async`
- `dom(clk_b) < Async`
- `dom(flag_a) < Async`
- `dom(flag_b) < Async`
- `dom(result) < Async` ⚠️ — `result` is NOT declared `async`

### Complete constraint system:
```
Fixed:
  F1: dom(clk_a) = Clock
  F2: dom(clk_b) = Clock
  F3: dom(flag_a) = Sync(ComponentId(clk_a))
  F4: dom(flag_b) = Sync(ComponentId(clk_b))

Strict upper bound (implicit):
  S1: dom(clk_a) < Async
  S2: dom(clk_b) < Async
  S3: dom(flag_a) < Async
  S4: dom(flag_b) < Async
  S5: dom(result) < Async

Equality:
  E1: dom(#0) = dom(clk_a)
  E2: dom(#2) = dom(clk_b)
  E3: dom(#4) = dom(flag_a)
  E4: dom(#6) = dom(flag_b)
  E5: dom(flag_a) = dom(#1)
  E6: dom(flag_b) = dom(#3)
  E7: dom(#4) = dom(#8)          ← when guard 1
  E8: dom(#5) = dom(#8)          ← when arm 1
  E9: dom(#6) = dom(#8)          ← when guard 2
  E10: dom(#7) = dom(#8)         ← when arm 2
  E11: dom(result) = dom(#8)     ← driver
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment (or detect the error).

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clk_a}, {clk_b}, {flag_a}, {flag_b}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}
```

### Processing fixed constraints F1-F4:

Assign concrete domains:
```
{clk_a: Clock}, {clk_b: Clock}, {flag_a: Sync(clk_a)}, {flag_b: Sync(clk_b)},
{result}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}
```

### Processing equality constraint E1: `dom(#0) = dom(clk_a)`

Merge expression #0 with clk_a's equivalence class:
```
{clk_a, #0: Clock}, {clk_b: Clock}, {flag_a: Sync(clk_a)}, {flag_b: Sync(clk_b)},
{result}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}
```

### Processing equality constraint E2: `dom(#2) = dom(clk_b)`

Merge expression #2 with clk_b's equivalence class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a: Sync(clk_a)}, {flag_b: Sync(clk_b)},
{result}, {#1}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}
```

### Processing equality constraint E3: `dom(#4) = dom(flag_a)`

Merge expression #4 with flag_a's equivalence class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #4: Sync(clk_a)},
{flag_b: Sync(clk_b)}, {result}, {#1}, {#3}, {#5}, {#6}, {#7}, {#8}
```

### Processing equality constraint E4: `dom(#6) = dom(flag_b)`

Merge expression #6 with flag_b's equivalence class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #4: Sync(clk_a)},
{flag_b, #6: Sync(clk_b)}, {result}, {#1}, {#3}, {#5}, {#7}, {#8}
```

### Processing equality constraint E5: `dom(flag_a) = dom(#1)`

Merge #1 with flag_a's class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #1, #4: Sync(clk_a)},
{flag_b, #6: Sync(clk_b)}, {result}, {#3}, {#5}, {#7}, {#8}
```

### Processing equality constraint E6: `dom(flag_b) = dom(#3)`

Merge #3 with flag_b's class:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #1, #4: Sync(clk_a)},
{flag_b, #3, #6: Sync(clk_b)}, {result}, {#5}, {#7}, {#8}
```

### Processing equality constraint E7: `dom(#4) = dom(#8)` (when guard 1)

Merge #8 with #4's class (which is flag_a's class):
- #4's class: `{flag_a, #1, #4: Sync(clk_a)}`
- #8's class: `{#8}` (no concrete domain yet)
- Result: #8 joins flag_a's class and inherits `Sync(clk_a)`

```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #1, #4, #8: Sync(clk_a)},
{flag_b, #3, #6: Sync(clk_b)}, {result}, {#5}, {#7}
```

### Processing equality constraint E8: `dom(#5) = dom(#8)` (when arm 1)

Merge #5 with #8's class:
- #8's class: `{flag_a, #1, #4, #8: Sync(clk_a)}` (has concrete domain)
- #5's class: `{#5}` (literal, no concrete domain)
- Result: #5 joins and inherits `Sync(clk_a)`

```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {flag_a, #1, #4, #5, #8: Sync(clk_a)},
{flag_b, #3, #6: Sync(clk_b)}, {result}, {#7}
```

### Processing equality constraint E9: `dom(#6) = dom(#8)` ⚠️ ERROR

Attempt to merge #6's class with #8's class:
- #6's class: `{flag_b, #3, #6: Sync(clk_b)}` (has concrete domain `Sync(clk_b)`)
- #8's class: `{flag_a, #1, #4, #5, #8: Sync(clk_a)}` (has concrete domain `Sync(clk_a)`)

Check compatibility: `compatible(Sync(clk_a), Sync(clk_b))`?
- Both are `Sync` domains
- Are their clocks the same? `clk_a == clk_b`? **No**
- **Result: INCOMPATIBLE** ❌

**ERROR DETECTED**: Cannot unify `Sync(ComponentId(clk_a))` with `Sync(ComponentId(clk_b))`

Error message:
```
Domain unification error in `when` expression:
  Guard `flag_a` has domain Sync(clk_a)
  Guard `flag_b` has domain Sync(clk_b)

A `when` expression requires all guards and arms to have the same domain.
The guards come from incompatible clock domains and cannot be unified.

Location: when expression at result := when { ... }

Help: If you need to mix signals from different clock domains:
  1. Declare the result as `async`:
       wire result : Bit async { ... }
  2. Or synchronize signals to a common clock domain first using the Sync module
```

### Solver terminates with error

The solver stops processing further constraints and returns the error.

**Solve result:**
```python
SolveResult {
    union_find: /* partial state */,
    errors: [
        "Domain unification error: cannot merge Sync(ComponentId(clk_a)) with Sync(ComponentId(clk_b))
         at constraint E9: dom(#6) = dom(#8)
         from when expression: guard flag_b (Sync(clk_b)) cannot unify with result (Sync(clk_a))"
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
    moddef: WhenMixedDomains,
    component_domains: {
        clk_a: Clock,
        clk_b: Clock,
        flag_a: Sync(ComponentId(clk_a)),
        flag_b: Sync(ComponentId(clk_b)),
        result: /* unresolved */,
    },
    signature: /* invalid */,
    diagnostics: [
        Diagnostic {
            severity: Error,
            message: "Domain unification error: when expression guards from incompatible clock domains",
            location: /* when expression in result driver */,
            help: "Declare result as 'async' or synchronize signals to a common clock",
        }
    ],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes, very naturally.** The `when` expression's constraint generation creates equality constraints linking all guards and arms to the result. When the solver processes these equalities, it attempts to merge equivalence classes with incompatible domains (`Sync(clk_a)` vs `Sync(clk_b)`), which fails during the compatibility check. No special-case logic is needed — the standard unification algorithm catches this error.

### Edge cases or design issues discovered:

1. **Order of constraint processing matters for error reporting quality**: In this example, constraint E7 merges the result with `flag_a` first, establishing `Sync(clk_a)` as the tentative domain. Then E9 tries to merge `flag_b` (`Sync(clk_b)`) and fails. The error message naturally points to `flag_b` as the "conflicting" guard. If E9 were processed before E7, the error would point to `flag_a` instead. Both are correct, but the first signal processed becomes the "expected" domain in error messages.

2. **The equality constraint model for `when` is elegant**: By requiring all guards and arms to equal the result, we get:
   - Automatic domain unification across all branches
   - Clear error when branches come from incompatible domains
   - Natural support for upcasting when the result is `async` (all branches can upcast)

3. **Literal arms are domain-free**: The literal `true` and `false` in the arms have no inherent domain (they're `Constant`), so they unify with whatever domain the guards establish. This is correct — literals should adapt to the context, not constrain it.

4. **The fix is clear**: The error message can suggest two solutions:
   - Add `async` to the result (if mixing domains is intended)
   - Synchronize all guards to a common clock first (if single-domain is intended)

### Clarifications needed for PLANv2.md or ALGORITHM.md:

1. **ALGORITHM.md is clear**: The `when` expression constraint generation is well-specified. The example confirms that the algorithm works as documented.

2. **PLANv2.md error condition §8.5** could be more specific: It says "if operands cannot unify to a single domain," but it should clarify that this specifically means guards and arms (not just subject in the case of `match`). The current wording is slightly ambiguous.

   Suggested revision:
   > **8.5** In a `when` expression, if the guard expressions and arm expressions cannot all unify to a single domain (and the result is not declared `async`), this is an error. In a `match` expression, if the subject expression and arm expressions cannot all unify to a single domain (and the result is not declared `async`), this is an error.

3. **The symmetry between `when` and `match`** is worth emphasizing: Both constructs require all participating expressions (guards+arms for `when`, subject+arms for `match`) to unify. This is a general principle for control flow expressions.

4. **No issues discovered**: The algorithm handles this case cleanly and produces a clear diagnostic.
