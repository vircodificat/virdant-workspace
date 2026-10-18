## Example 021: dontcare Constant Domain

```virdant
mod DontcareExample {
    incoming clock : Clock
    incoming enable : Bit

    reg state : Word[8] on clock {
        it <= when {
            case enable => state + 1
            else => dontcare
        }
    }

    outgoing out : Word[8] {
        it := state
    }
}
```

**What's happening**: A register that increments when enabled, but uses `dontcare` for the disabled case. The `dontcare` expression should have domain `Constant`, and it should unify with the register's `Sync(clock)` domain (since Constant is at the bottom of the lattice and can weaken to any Sync domain).

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `enable` (incoming, type Bit)
- `state` (reg, type Word[8])
- `out` (outgoing, type Word[8])

**Step 2 - Walk declarations and drivers, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming enable : Bit`
- Component: `enable`
- No expressions

Declaration: `reg state : Word[8] on clock`
- Component: `state`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing out : Word[8]`
- Component: `out`
- No expressions in declaration

Driver: `state <= when { case enable => state + 1; else => dontcare }`
- Expression `#1`: `when` expression (RHS of driver)
  - Guard `#2`: reference to `enable`
  - Guard result `#3`: binary operation `state + 1`
    - Left operand `#4`: reference to `state`
    - Right operand `#5`: literal `1`
  - Else result `#6`: `dontcare`

Driver: `out := state`
- Expression `#7`: reference to `state` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprWhen (the when expression)
- `#2`: ExprReference to `enable` (guard)
- `#3`: ExprBinaryOp `+` (first arm result)
- `#4`: ExprReference to `state` (left operand of `+`)
- `#5`: ExprLiteral `1` (right operand of `+`)
- `#6`: ExprDontcare (else arm result)
- `#7`: ExprReference to `state` (out driver)

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(state) = Sync(ComponentId(clock))`

**Fixed constraints from literals:**
- `dom(#5) = Constant` (literal `1`)

**Fixed constraints from dontcare:**
- `dom(#6) = Constant` (dontcare expression)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#2` references `enable` → `dom(#2) = dom(enable)`
- Expression `#4` references `state` → `dom(#4) = dom(state)`
- Expression `#7` references `state` → `dom(#7) = dom(state)`

**Constraints from binary operation (`state + 1`):**
- Result expression: `#3`
- Left operand: `#4`, Right operand: `#5`
- Unified operator rule: operands can upcast to result
  - `dom(#4) ≤ dom(#3)` (upper bound)
  - `dom(#5) ≤ dom(#3)` (upper bound)

**Constraints from when expression:**
- When expression `#1` has one guard (`#2`) and two result arms (`#3`, `#6`)
- All guards and all arms must equal the result:
  - `dom(#2) = dom(#1)` (guard equals result)
  - `dom(#3) = dom(#1)` (first arm result equals when result)
  - `dom(#6) = dom(#1)` (else arm result equals when result)

**Equality constraints from drivers:**

Driver: `state <= when { ... }`
- LHS: component `state`
- RHS: expression `#1`
- Constraint: `dom(state) = dom(#1)`

Driver: `out := state`
- LHS: component `out`
- RHS: expression `#7`
- Constraint: `dom(out) = dom(#7)`

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(enable) < Async`
- `dom(state) < Async`
- `dom(out) < Async`

**Complete constraint system:**
```
Fixed:
  F₁: dom(clock) = Clock
  F₂: dom(state) = Sync(ComponentId(clock))
  F₃: dom(#5) = Constant    [literal 1]
  F₄: dom(#6) = Constant    [dontcare]

Strict upper bounds:
  S₁: dom(clock) < Async
  S₂: dom(enable) < Async
  S₃: dom(state) < Async
  S₄: dom(out) < Async

Upper bounds:
  U₁: dom(#4) ≤ dom(#3)     [+ left operand]
  U₂: dom(#5) ≤ dom(#3)     [+ right operand]

Equalities:
  E₁: dom(#0) = dom(clock)
  E₂: dom(#2) = dom(enable)
  E₃: dom(#4) = dom(state)
  E₄: dom(#7) = dom(state)
  E₅: dom(#2) = dom(#1)     [when guard]
  E₆: dom(#3) = dom(#1)     [when arm 1]
  E₇: dom(#6) = dom(#1)     [when arm 2 - dontcare]
  E₈: dom(state) = dom(#1)  [state driver]
  E₉: dom(out) = dom(#7)    [out driver]
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment.

**Initial state:**
```
All variables in separate equivalence classes:
{clock}, {enable}, {state}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

**Step 1: Process fixed constraints**

F₁: `dom(clock) = Clock`
```
{clock: Clock}, {enable}, {state}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

F₂: `dom(state) = Sync(ComponentId(clock))`
```
{clock: Clock}, {enable}, {state: Sync(clock)}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

F₃: `dom(#5) = Constant` (literal `1`)
```
{clock: Clock}, {enable}, {state: Sync(clock)}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5: Constant}, {#6}, {#7}
```

F₄: `dom(#6) = Constant` (dontcare)
```
{clock: Clock}, {enable}, {state: Sync(clock)}, {out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5: Constant}, {#6: Constant}, {#7}
```

**Step 2: Process equality constraints**

E₁: `dom(#0) = dom(clock)` — Merge `#0` with `clock`
```
{clock, #0: Clock}, {enable}, {state: Sync(clock)}, {out},
{#1}, {#2}, {#3}, {#4}, {#5: Constant}, {#6: Constant}, {#7}
```

E₂: `dom(#2) = dom(enable)` — Merge `#2` with `enable`
```
{clock, #0: Clock}, {enable, #2}, {state: Sync(clock)}, {out},
{#1}, {#3}, {#4}, {#5: Constant}, {#6: Constant}, {#7}
```

E₃: `dom(#4) = dom(state)` — Merge `#4` with `state`
```
{clock, #0: Clock}, {enable, #2}, {state, #4: Sync(clock)}, {out},
{#1}, {#3}, {#5: Constant}, {#6: Constant}, {#7}
```

E₄: `dom(#7) = dom(state)` — Merge `#7` with `state`
```
{clock, #0: Clock}, {enable, #2}, {state, #4, #7: Sync(clock)}, {out},
{#1}, {#3}, {#5: Constant}, {#6: Constant}
```

E₅: `dom(#2) = dom(#1)` — Merge `#2` (enable) with `#1` (when expression)
```
{clock, #0: Clock}, {enable, #2, #1}, {state, #4, #7: Sync(clock)}, {out},
{#3}, {#5: Constant}, {#6: Constant}
```

E₆: `dom(#3) = dom(#1)` — Merge `#3` (binary op result) with `#1`
```
{clock, #0: Clock}, {enable, #2, #1, #3}, {state, #4, #7: Sync(clock)}, {out},
{#5: Constant}, {#6: Constant}
```

E₇: `dom(#6) = dom(#1)` — Merge `#6` (dontcare) with `#1`
- `#6` class: `{#6: Constant}` (has concrete domain Constant)
- `#1` class: `{enable, #2, #1, #3}` (no concrete domain)
- **Can Constant unify with an unknown domain?** YES! Constant is at the bottom of the lattice and unifies with any Sync domain.
- Merged class gets domain: Constant (for now, it will be resolved later)

```
{clock, #0: Clock}, {enable, #2, #1, #3, #6: Constant},
{state, #4, #7: Sync(clock)}, {out}, {#5: Constant}
```

E₈: `dom(state) = dom(#1)` — Merge `state` with `#1`
- `state` class: `{state, #4, #7: Sync(clock)}` (has concrete domain Sync(clock))
- `#1` class: `{enable, #2, #1, #3, #6: Constant}` (has concrete domain Constant)
- **Can Constant unify with Sync(clock)?** YES! According to the `unify()` function, when one domain is Constant and the other is Sync, Constant weakens to Sync.
- Merged class gets domain: `Sync(clock)` (Constant weakens to Sync)

```
{clock, #0: Clock},
{enable, #2, #1, #3, #6, state, #4, #7: Sync(clock)},
{out}, {#5: Constant}
```

E₉: `dom(out) = dom(#7)` — Merge `out` with `#7`
- `#7` is already in the `state` class with domain `Sync(clock)`
- Merge `out` into that class

```
{clock, #0: Clock},
{enable, #2, #1, #3, #6, state, #4, #7, out: Sync(clock)},
{#5: Constant}
```

**Step 3: Resolve upper bounds**

U₁: `dom(#4) ≤ dom(#3)` — Both in same class with `Sync(clock)`, satisfied ✓
U₂: `dom(#5) ≤ dom(#3)` — `#5` has `Constant`, `#3` has `Sync(clock)`
- Constant ≤ Sync(clock)? YES! Constant is at the bottom, so it's ≤ any Sync domain ✓

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {enable, #2, #1, #3, #6, state, #4, #7, out} → Sync(ComponentId(clock))
Class 3: {#5} → Constant
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(enable) = Sync(ComponentId(clock))
  dom(state) = Sync(ComponentId(clock))
  dom(out) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock               [clock reference]
  dom(#1) = Sync(clock)         [when expression]
  dom(#2) = Sync(clock)         [enable reference]
  dom(#3) = Sync(clock)         [state + 1]
  dom(#4) = Sync(clock)         [state reference in +]
  dom(#5) = Constant            [literal 1]
  dom(#6) = Sync(clock)         [dontcare - unified to Sync!]
  dom(#7) = Sync(clock)         [state reference in out driver]
```

**Step 4: Check strict upper bounds**

S₁: `dom(clock) < Async` → `Clock < Async` ✓
S₂: `dom(enable) < Async` → `Sync(clock) < Async` ✓
S₃: `dom(state) < Async` → `Sync(clock) < Async` ✓
S₄: `dom(out) < Async` → `Sync(clock) < Async` ✓

**All constraints satisfied! ✓**

**Key observation**: The `dontcare` expression started with domain `Constant` (fixed constraint), then unified with the `when` expression result through equality constraint E₇. When the `when` expression was later unified with `state` (which has domain `Sync(clock)`), the Constant domain weakened to `Sync(clock)`. This is the expected behavior according to the lattice design where Constant is at the bottom and can unify with any Sync domain.

---

### Pass 3: Signature Extraction

Build the module's polymorphic domain signature.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(enable) = Sync(ComponentId(clock))` → `Var(α)`
- `dom(out) = Sync(ComponentId(clock))` → `Var(α)`

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        enable: Var(DomainVar(0)),   // α
        out: Var(DomainVar(0)),      // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: DontcareExample,
    component_domains: {
        clock: Clock,
        enable: Sync(ComponentId(clock)),
        state: Sync(ComponentId(clock)),
        out: Sync(ComponentId(clock)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

# Notes

## Does this work naturally with the algorithm as-is?

**YES!** The `dontcare` expression works perfectly with the algorithm, requiring only one addition:

**Required change**: Add a fixed constraint during constraint generation (Pass 1):
```python
elif expr.kind == Dontcare:
    expr_node = expr_id
    expr_id += 1
    variables.add(expr_node)
    fixed.append(Fixed(expr_node, Constant))  # ← ADD THIS
    return (expr_id, expr_node)
```

This is exactly parallel to how literals should be handled (as discovered in Example 016 and documented in SUMMARY.md).

## Any surprises or edge cases?

**No surprises!** The behavior is exactly what we'd expect:

1. **Dontcare starts as Constant**: The fixed constraint establishes `dom(dontcare) = Constant`.

2. **Constant weakens through unification**: When `dontcare` is used in a context that requires a `Sync` domain (like driving a register), the unification rules naturally handle the weakening from `Constant` to `Sync`.

3. **Lattice properties handle everything**: Because Constant is at the bottom of the lattice:
   - `unify(Constant, Sync(c))` → `Sync(c)` ✓
   - `Constant ≤ Sync(c)` in upper bounds ✓
   - No special-case logic needed

4. **Error detection still works**: If `dontcare` were inappropriately mixed with incompatible domains (e.g., trying to unify `Sync(c1)` and `Sync(c2)` through `dontcare`), the algorithm would still catch it because the unification happens *after* `dontcare`'s Constant domain has been resolved.

## Does the constraint system handle this without special cases?

**Absolutely!** No special cases needed:

- **Constraint generation**: Just add one fixed constraint (same pattern as literals)
- **Constraint solving**: Existing unification and lattice operations handle everything
- **Signature extraction**: `dontcare` expressions don't appear in signatures (they're internal expressions)

The key insight is that `dontcare` is simply another expression with domain `Constant`. The lattice-based unification system automatically handles the weakening semantics.

## Any clarifications needed for ALGORITHM.md or PLANv2.md?

### For ALGORITHM.md:

Add the `dontcare` case to `walk_expression()` in Pass 1:

```python
# Dontcare expression (fixed to Constant)
elif expr.kind == Dontcare:
    expr_node = expr_id
    expr_id += 1
    variables.add(expr_node)
    fixed.append(Fixed(expr_node, Constant))  # Like literals, dontcare is Constant
    return (expr_id, expr_node)
```

This should be added right after the `Literal` case, since they have identical constraint generation behavior.

### For PLANv2.md:

Add a note in the **Domain Categories** section clarifying that `dontcare` expressions have domain `Constant`:

> **`dontcare` expressions**: The `dontcare` keyword represents an intentionally unspecified value. Like numeric literals, `dontcare` has domain `Constant` and can unify with any `Sync` domain or upcast to `Async`.

### Optional enhancement:

Consider adding an example to ALGORITHM.md showing how literals and `dontcare` are treated identically with respect to domain analysis. This reinforces that they're both "pure" values with no temporal dependencies.

## Algorithm gaps revealed?

**None!** This example actually *validates* the algorithm design:

1. The lattice structure with Constant at the bottom is the right choice
2. The unification rules handle weakening correctly
3. No special-casing needed for different expression types that are fundamentally "constant"
4. The equality constraint propagation naturally handles the flow from `dontcare` → when expression → register

The algorithm is complete and handles `dontcare` elegantly as a natural consequence of the lattice design.

## Design Validation

This example demonstrates several important properties:

1. **Compositionality**: `dontcare` can appear anywhere a value is expected, and the domain system correctly propagates constraints.

2. **No implicit promotions**: The domain system doesn't silently promote `dontcare` to arbitrary domains. It follows the explicit unification rules through equality constraints.

3. **Deterministic behavior**: The final domain of a `dontcare` expression is determined by the context it's used in, but this determination follows the same constraint-solving path as any other expression.

4. **Error detection preserved**: If `dontcare` were used in a context that would create a domain conflict, the algorithm would still detect it (e.g., if two different clocks were unified through a shared `dontcare` value).

## Comparison with Literals

Both `dontcare` and numeric literals:
- Start with fixed domain `Constant`
- Can unify with any `Sync` domain (weakening)
- Can upcast to `Async` via upper bound constraints
- Require no special-case logic in constraint solving

The only difference is semantic intent:
- **Literals** represent specific compile-time values
- **Dontcare** represents "any value is acceptable here"

But from the domain analysis perspective, they're identical.
