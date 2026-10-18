# Example 22: mux() Ternary Selector Domain Unification

```virdant
mod MuxExample {
    incoming clock : Clock
    incoming select : Bit
    
    reg a : Word[8] on clock {
        it <= 42w8
    }
    
    reg b : Word[8] on clock {
        it <= 67w8
    }
    
    wire result : Word[8] {
        it := mux(select, a, b)
    }
}
```

**What's happening**: The `mux(select, a, b)` function is a ternary selector that returns `a` when `select` is true, and `b` when `select` is false. According to Virdant semantics, `mux` desugars to a `when` expression:
```virdant
when {
    case select => a
    else => b
}
```

The key domain rule: **all three arguments** (`select`, `a`, and `b`) must unify to the same domain, and the result has that domain. In this example, all components are on the same clock domain `Sync(clock)`, so unification succeeds naturally.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `select` (incoming, type Bit)
- `a` (reg, type Word[8])
- `b` (reg, type Word[8])
- `result` (wire, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions

**Declaration: `incoming select : Bit`**
- Component: `select`
- Type: Bit
- No expressions in declaration

**Declaration: `reg a : Word[8] on clock`**
- Component: `a`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `a <= 42w8`**
- Expression `#1`: literal `42w8`

**Declaration: `reg b : Word[8] on clock`**
- Component: `b`
- Has `on clock` clause
- Expression `#2`: reference to `clock` (in the `on clock` clause)

**Driver: `b <= 67w8`**
- Expression `#3`: literal `67w8`

**Declaration: `wire result : Word[8]`**
- Component: `result`
- No `async` modifier
- No expressions in declaration

**Driver: `result := mux(select, a, b)`**

The `mux(select, a, b)` call desugars to:
```virdant
when {
    case select => a
    else => b
}
```

Generating expressions for the desugared form:
- Expression `#4`: reference to `select` (guard of case 1)
- Expression `#5`: reference to `a` (arm of case 1)
- Expression `#6`: reference to `b` (else arm)
- Expression `#7`: the `when` expression result (the mux result)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: Literal `42w8`
- `#2`: ExprReference to `clock`
- `#3`: Literal `67w8`
- `#4`: ExprReference to `select` (mux condition)
- `#5`: ExprReference to `a` (mux true branch)
- `#6`: ExprReference to `b` (mux false branch)
- `#7`: Mux/When expression result

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(a) = Sync(ComponentId(clock))`
- `dom(b) = Sync(ComponentId(clock))`

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#2) = dom(clock)`
- `dom(#4) = dom(select)` (select reference)
- `dom(#5) = dom(a)` (a reference)
- `dom(#6) = dom(b)` (b reference)

**Equality constraints from register drivers:**
- `dom(a) = dom(#1)`
- `dom(b) = dom(#3)`

**Equality constraints from `mux`/`when` expression:**

Since `mux` desugars to `when`, we apply the `when` expression rule:
> All guards and all arms must equal result

This generates:
- `dom(#4) = dom(#7)` (guard equals result — select unifies with result)
- `dom(#5) = dom(#7)` (arm 1 equals result — a unifies with result)
- `dom(#6) = dom(#7)` (arm 2 equals result — b unifies with result)

**Equality constraint from driver:**
- `dom(result) = dom(#7)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(select) < Async`
- `dom(a) < Async`
- `dom(b) < Async`
- `dom(result) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock) = Clock
  F2: dom(a) = Sync(ComponentId(clock))
  F3: dom(b) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  S1: dom(clock) < Async
  S2: dom(select) < Async
  S3: dom(a) < Async
  S4: dom(b) < Async
  S5: dom(result) < Async

Equality:
  E1: dom(#0) = dom(clock)
  E2: dom(#2) = dom(clock)
  E3: dom(#4) = dom(select)
  E4: dom(#5) = dom(a)
  E5: dom(#6) = dom(b)
  E6: dom(a) = dom(#1)
  E7: dom(b) = dom(#3)
  E8: dom(#4) = dom(#7)    ← mux condition = result
  E9: dom(#5) = dom(#7)    ← mux true branch = result
  E10: dom(#6) = dom(#7)   ← mux false branch = result
  E11: dom(result) = dom(#7) ← driver
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock}, {select}, {a}, {b}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing fixed constraints F1-F3:

Assign concrete domains:
```
{clock: Clock}, {select}, {a: Sync(clock)}, {b: Sync(clock)}, {result},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E1: `dom(#0) = dom(clock)`

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {select}, {a: Sync(clock)}, {b: Sync(clock)}, {result},
{#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E2: `dom(#2) = dom(clock)`

Merge expression #2 with clock's equivalence class:
```
{clock, #0, #2: Clock}, {select}, {a: Sync(clock)}, {b: Sync(clock)}, {result},
{#1}, {#3}, {#4}, {#5}, {#6}, {#7}
```

### Processing equality constraint E3: `dom(#4) = dom(select)`

Merge expression #4 with select's equivalence class:
```
{clock, #0, #2: Clock}, {select, #4}, {a: Sync(clock)}, {b: Sync(clock)}, {result},
{#1}, {#3}, {#5}, {#6}, {#7}
```

### Processing equality constraint E4: `dom(#5) = dom(a)`

Merge expression #5 with a's equivalence class:
```
{clock, #0, #2: Clock}, {select, #4}, {a, #5: Sync(clock)}, {b: Sync(clock)}, {result},
{#1}, {#3}, {#6}, {#7}
```

### Processing equality constraint E5: `dom(#6) = dom(b)`

Merge expression #6 with b's equivalence class:
```
{clock, #0, #2: Clock}, {select, #4}, {a, #5: Sync(clock)}, {b, #6: Sync(clock)}, {result},
{#1}, {#3}, {#7}
```

### Processing equality constraint E6: `dom(a) = dom(#1)`

Merge #1 with a's class:
```
{clock, #0, #2: Clock}, {select, #4}, {a, #1, #5: Sync(clock)}, {b, #6: Sync(clock)}, {result},
{#3}, {#7}
```

### Processing equality constraint E7: `dom(b) = dom(#3)`

Merge #3 with b's class:
```
{clock, #0, #2: Clock}, {select, #4}, {a, #1, #5: Sync(clock)}, {b, #3, #6: Sync(clock)}, {result},
{#7}
```

### Processing equality constraint E8: `dom(#4) = dom(#7)` (mux condition)

Merge #7 with #4's class (select):
- #4's class: `{select, #4}` (no concrete domain yet)
- #7's class: `{#7}` (no concrete domain yet)
- Result: merge into one class, still no concrete domain

```
{clock, #0, #2: Clock}, {select, #4, #7}, {a, #1, #5: Sync(clock)}, 
{b, #3, #6: Sync(clock)}, {result}
```

### Processing equality constraint E9: `dom(#5) = dom(#7)` (mux true branch)

Merge #5's class with #7's class:
- #5's class: `{a, #1, #5: Sync(clock)}` (has concrete domain)
- #7's class: `{select, #4, #7}` (no concrete domain)
- Result: merged class inherits `Sync(clock)`

```
{clock, #0, #2: Clock}, {select, #4, #5, #7, a, #1: Sync(clock)}, 
{b, #3, #6: Sync(clock)}, {result}
```

### Processing equality constraint E10: `dom(#6) = dom(#7)` (mux false branch)

Merge #6's class with #7's class:
- #6's class: `{b, #3, #6: Sync(clock)}` (has concrete domain `Sync(clock)`)
- #7's class: `{select, #4, #5, #7, a, #1: Sync(clock)}` (has concrete domain `Sync(clock)`)
- Check compatibility: `compatible(Sync(clock), Sync(clock))`? **Yes** ✓
- Both have the same clock component
- Result: merge into one class with domain `Sync(clock)`

```
{clock, #0, #2: Clock}, 
{select, #4, #5, #7, a, #1, b, #3, #6: Sync(clock)}, 
{result}
```

### Processing equality constraint E11: `dom(result) = dom(#7)`

Merge result with #7's class:
- #7's class: `{select, #4, #5, #7, a, #1, b, #3, #6: Sync(clock)}`
- result's class: `{result}` (no concrete domain)
- Result: result joins and inherits `Sync(clock)`

```
{clock, #0, #2: Clock}, 
{select, #4, #5, #7, a, #1, b, #3, #6, result: Sync(clock)}
```

### Final partition:
```
Class 1: {clock, #0, #2} → Clock
Class 2: {select, #4, #5, #7, a, #1, b, #3, #6, result} → Sync(ComponentId(clock))
```

### Solution (domain assignment):
```
Components:
  dom(clock) = Clock
  dom(select) = Sync(ComponentId(clock))
  dom(a) = Sync(ComponentId(clock))
  dom(b) = Sync(ComponentId(clock))
  dom(result) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Clock
  dom(#3) = Sync(ComponentId(clock))
  dom(#4) = Sync(ComponentId(clock))  ← mux condition
  dom(#5) = Sync(ComponentId(clock))  ← mux true branch
  dom(#6) = Sync(ComponentId(clock))  ← mux false branch
  dom(#7) = Sync(ComponentId(clock))  ← mux result
```

### Verification:

**All fixed constraints satisfied:** ✓
- `dom(clock) = Clock` ✓
- `dom(a) = Sync(clock)` ✓
- `dom(b) = Sync(clock)` ✓

**All strict upper bound constraints satisfied:** ✓
- `dom(clock) < Async` → `Clock < Async` ✓
- `dom(select) < Async` → `Sync(clock) < Async` ✓
- `dom(a) < Async` → `Sync(clock) < Async` ✓
- `dom(b) < Async` → `Sync(clock) < Async` ✓
- `dom(result) < Async` → `Sync(clock) < Async` ✓

**All equality constraints satisfied:** ✓
- All variables in Class 1 have domain `Clock` ✓
- All variables in Class 2 have domain `Sync(clock)` ✓
- **Key insight**: All three `mux` arguments (`#4`, `#5`, `#6`) unified to `Sync(clock)` ✓

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock) = Clock` → Clock (concrete, the parameter's binding)
- `dom(select) = Sync(ComponentId(clock))` → α (the domain parameter)

### Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        select: Var(DomainVar(0)),  // α
    }
}
```

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: MuxExample,
    component_domains: {
        clock: Clock,
        select: Sync(ComponentId(clock)),
        a: Sync(ComponentId(clock)),
        b: Sync(ComponentId(clock)),
        result: Sync(ComponentId(clock)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes, perfectly.** The `mux(sel, a, b)` function desugars to a `when` expression, and the existing `when` constraint generation rule naturally handles it:

> All guards and all arms must equal result

This creates three equality constraints:
- `dom(sel) = dom(result)`
- `dom(a) = dom(result)`
- `dom(b) = dom(result)`

Through transitivity, this forces all three arguments to unify. The algorithm requires **no special case** for `mux` — it's handled entirely by the existing `when` expression logic.

### Surprises or edge cases?

**No surprises.** The behavior is exactly as expected:

1. **All three arguments must unify**: The constraints `E8`, `E9`, and `E10` force the selector, true branch, and false branch to all have the same domain. This is the intended semantics.

2. **Order doesn't matter**: In our trace, constraint `E9` merged `a` (with concrete domain `Sync(clock)`) into the result, establishing the domain. Then `E10` verified that `b` was compatible. If `E10` had been processed first, `b` would have established the domain and `E9` would verify `a`. Either way works.

3. **Literals adapt**: If we had used literal expressions like `mux(select, 42w8, 67w8)`, the literals would have no fixed domain and would unify with whatever domain the selector has. This is correct behavior — literals should be domain-flexible.

4. **Error case is symmetric with `when`**: If `a` and `b` were on different clocks (like in example 008), the unification would fail with the same error as a `when` expression:
   ```virdant
   reg a : Word[8] on clk_a { ... }
   reg b : Word[8] on clk_b { ... }
   wire result : Word[8] {
       it := mux(select, a, b)  // ERROR: Sync(clk_a) vs Sync(clk_b)
   }
   ```

5. **Async workaround**: If mixing domains is intentional, declaring the result as `async` would allow all three arguments to upcast:
   ```virdant
   wire result : Word[8] async {
       it := mux(select_on_clk_a, data_on_clk_b, data_on_clk_c)  // OK
   }
   ```

### Does the constraint system handle this without special cases?

**Yes, completely.** There is no need for special-case logic for `mux` because:

1. **Desugaring is sufficient**: `mux(sel, a, b)` → `when { case sel => a; else => b }` means `mux` is syntactic sugar that disappears before domain analysis.

2. **`when` constraints are general**: The `when` expression rule of "all guards and arms equal result" is a general principle that naturally enforces the three-way unification for `mux`.

3. **No separate rule needed**: ALGORITHM.md doesn't need a `mux` case in `walk_expression()` — it's handled by the `when` case.

### Clarifications needed for ALGORITHM.md or PLANv2.md?

**Minor documentation note for completeness:**

1. **ALGORITHM.md** doesn't explicitly show `mux` in `walk_expression()`, which is correct (since it desugars before domain analysis). But it might be helpful to add a comment noting this:

   ```python
   # Note: mux(sel, a, b) is desugared to when { case sel => a; else => b }
   # before domain analysis, so no separate case is needed here.
   ```

2. **PLANv2.md** mentions `mux` in the section on combinational expressions (§3.1) but doesn't explicitly state the domain rule. It could add:

   > **mux() ternary selector**: `mux(sel, a, b)` desugars to a `when` expression, so the `when` rule applies: all three arguments must have the same domain, and the result has that domain.

3. **No algorithm gaps discovered**: The existing algorithm handles `mux` perfectly. The constraint system naturally enforces that all three arguments unify through the transitive equality constraints from `when` expression processing.

### Key takeaway

**`mux` is elegantly handled as a special case of `when`.** By having `mux` desugar before domain analysis, and by having a general "all branches equal result" rule for `when`, the algorithm handles ternary selectors with no additional logic. This is good language design — the domain system doesn't need to know about every syntactic construct, just the core expression forms.
