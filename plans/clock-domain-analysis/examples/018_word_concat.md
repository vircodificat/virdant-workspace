## Example 18: word() Concatenation with Domain Unification

```virdant
mod WordConcatExample {
    incoming clock : Clock
    incoming a : Bit
    incoming b : Bit
    incoming c : Word[2]

    reg r_a : Bit on clock {
        it <= a
    }

    reg r_b : Bit on clock {
        it <= b
    }

    reg r_c : Word[2] on clock {
        it <= c
    }

    // Concatenate registered values - all same domain
    outgoing concat_regs : Word[4] {
        it := word(r_a, r_b, r_c)   // word(Bit, Bit, Word[2]) -> Word[4]
    }

    // Concatenate with literal - Constant unifies to Sync(clock)
    outgoing concat_with_literal : Word[5] {
        it := word(true, r_a, r_b, r_c)
    }

    // Multiple literals with a register
    outgoing mixed : Word[6] {
        it := word(true, false, r_a, r_b, r_c)
    }
}
```

**What's happening**: This example demonstrates `word()` concatenation where all operands must unify to the same domain. We concatenate:
1. Multiple registered bits/words (all `Sync(clock)`)
2. Literals (`Constant`) with registered values (literals unify to `Sync(clock)`)
3. Multiple literals with registered values (all unify to `Sync(clock)`)

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `a` (incoming, type Bit)
- `b` (incoming, type Bit)
- `c` (incoming, type Word[2])
- `r_a` (reg, type Bit)
- `r_b` (reg, type Bit)
- `r_c` (reg, type Word[2])
- `concat_regs` (outgoing, type Word[4])
- `concat_with_literal` (outgoing, type Word[5])
- `mixed` (outgoing, type Word[6])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming a : Bit`
- Component: `a`
- No expressions

Declaration: `incoming b : Bit`
- Component: `b`
- No expressions

Declaration: `incoming c : Word[2]`
- Component: `c`
- No expressions

Declaration: `reg r_a : Bit on clock`
- Component: `r_a`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `reg r_b : Bit on clock`
- Component: `r_b`
- Expression `#1`: reference to `clock` (in the `on clock` clause)

Declaration: `reg r_c : Word[2] on clock`
- Component: `r_c`
- Expression `#2`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing concat_regs : Word[4]`
- Component: `concat_regs`
- No expressions in declaration

Declaration: `outgoing concat_with_literal : Word[5]`
- Component: `concat_with_literal`
- No expressions in declaration

Declaration: `outgoing mixed : Word[6]`
- Component: `mixed`
- No expressions in declaration

Driver: `r_a <= a`
- Expression `#3`: reference to `a` (RHS of driver)

Driver: `r_b <= b`
- Expression `#4`: reference to `b` (RHS of driver)

Driver: `r_c <= c`
- Expression `#5`: reference to `c` (RHS of driver)

Driver: `concat_regs := word(r_a, r_b, r_c)`
- Expression `#6`: word concatenation `word(r_a, r_b, r_c)`
  - Expression `#7`: reference to `r_a` (first operand)
  - Expression `#8`: reference to `r_b` (second operand)
  - Expression `#9`: reference to `r_c` (third operand)

Driver: `concat_with_literal := word(true, r_a, r_b, r_c)`
- Expression `#10`: word concatenation `word(true, r_a, r_b, r_c)`
  - Expression `#11`: literal `true`
  - Expression `#12`: reference to `r_a`
  - Expression `#13`: reference to `r_b`
  - Expression `#14`: reference to `r_c`

Driver: `mixed := word(true, false, r_a, r_b, r_c)`
- Expression `#15`: word concatenation `word(true, false, r_a, r_b, r_c)`
  - Expression `#16`: literal `true`
  - Expression `#17`: literal `false`
  - Expression `#18`: reference to `r_a`
  - Expression `#19`: reference to `r_b`
  - Expression `#20`: reference to `r_c`

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `clock`
- `#2`: ExprReference to `clock`
- `#3`: ExprReference to `a`
- `#4`: ExprReference to `b`
- `#5`: ExprReference to `c`
- `#6`: ExprWord `word(r_a, r_b, r_c)`
- `#7`: ExprReference to `r_a`
- `#8`: ExprReference to `r_b`
- `#9`: ExprReference to `r_c`
- `#10`: ExprWord `word(true, r_a, r_b, r_c)`
- `#11`: ExprLiteral `true`
- `#12`: ExprReference to `r_a`
- `#13`: ExprReference to `r_b`
- `#14`: ExprReference to `r_c`
- `#15`: ExprWord `word(true, false, r_a, r_b, r_c)`
- `#16`: ExprLiteral `true`
- `#17`: ExprLiteral `false`
- `#18`: ExprReference to `r_a`
- `#19`: ExprReference to `r_b`
- `#20`: ExprReference to `r_c`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(r_a) = Sync(ComponentId(clock))` (register with `on clock` clause)
- `dom(r_b) = Sync(ComponentId(clock))` (register with `on clock` clause)
- `dom(r_c) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Fixed constraints from literals:**
- `dom(#11) = Constant` (literal `true`)
- `dom(#16) = Constant` (literal `true`)
- `dom(#17) = Constant` (literal `false`)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `clock` → `dom(#1) = dom(clock)`
- Expression `#2` references `clock` → `dom(#2) = dom(clock)`
- Expression `#3` references `a` → `dom(#3) = dom(a)`
- Expression `#4` references `b` → `dom(#4) = dom(b)`
- Expression `#5` references `c` → `dom(#5) = dom(c)`
- Expression `#7` references `r_a` → `dom(#7) = dom(r_a)`
- Expression `#8` references `r_b` → `dom(#8) = dom(r_b)`
- Expression `#9` references `r_c` → `dom(#9) = dom(r_c)`
- Expression `#12` references `r_a` → `dom(#12) = dom(r_a)`
- Expression `#13` references `r_b` → `dom(#13) = dom(r_b)`
- Expression `#14` references `r_c` → `dom(#14) = dom(r_c)`
- Expression `#18` references `r_a` → `dom(#18) = dom(r_a)`
- Expression `#19` references `r_b` → `dom(#19) = dom(r_b)`
- Expression `#20` references `r_c` → `dom(#20) = dom(r_c)`

**Equality constraints from word() concatenation:**

Word concatenation `word(r_a, r_b, r_c)` (expression `#6`):
- All operands must unify to the same domain, and result has that domain
- `dom(#7) = dom(#6)` (operand r_a equals result)
- `dom(#8) = dom(#6)` (operand r_b equals result)
- `dom(#9) = dom(#6)` (operand r_c equals result)

Word concatenation `word(true, r_a, r_b, r_c)` (expression `#10`):
- All operands must unify to the same domain, and result has that domain
- `dom(#11) = dom(#10)` (literal operand equals result)
- `dom(#12) = dom(#10)` (operand r_a equals result)
- `dom(#13) = dom(#10)` (operand r_b equals result)
- `dom(#14) = dom(#10)` (operand r_c equals result)

Word concatenation `word(true, false, r_a, r_b, r_c)` (expression `#15`):
- All operands must unify to the same domain, and result has that domain
- `dom(#16) = dom(#15)` (literal true equals result)
- `dom(#17) = dom(#15)` (literal false equals result)
- `dom(#18) = dom(#15)` (operand r_a equals result)
- `dom(#19) = dom(#15)` (operand r_b equals result)
- `dom(#20) = dom(#15)` (operand r_c equals result)

**Equality constraints from drivers:**

Driver: `r_a <= a`
- LHS: component `r_a`
- RHS: expression `#3`
- Constraint: `dom(r_a) = dom(#3)`

Driver: `r_b <= b`
- LHS: component `r_b`
- RHS: expression `#4`
- Constraint: `dom(r_b) = dom(#4)`

Driver: `r_c <= c`
- LHS: component `r_c`
- RHS: expression `#5`
- Constraint: `dom(r_c) = dom(#5)`

Driver: `concat_regs := word(r_a, r_b, r_c)`
- LHS: component `concat_regs`
- RHS: expression `#6`
- Constraint: `dom(concat_regs) = dom(#6)`

Driver: `concat_with_literal := word(true, r_a, r_b, r_c)`
- LHS: component `concat_with_literal`
- RHS: expression `#10`
- Constraint: `dom(concat_with_literal) = dom(#10)`

Driver: `mixed := word(true, false, r_a, r_b, r_c)`
- LHS: component `mixed`
- RHS: expression `#15`
- Constraint: `dom(mixed) = dom(#15)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(a) < Async`
- `dom(b) < Async`
- `dom(c) < Async`
- `dom(r_a) < Async`
- `dom(r_b) < Async`
- `dom(r_c) < Async`
- `dom(concat_regs) < Async`
- `dom(concat_with_literal) < Async`
- `dom(mixed) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(r_a) = Sync(ComponentId(clock))
  C₃: dom(r_b) = Sync(ComponentId(clock))
  C₄: dom(r_c) = Sync(ComponentId(clock))
  C₅: dom(#11) = Constant
  C₆: dom(#16) = Constant
  C₇: dom(#17) = Constant

Strict upper bound (implicit):
  C₈: dom(clock) < Async
  C₉: dom(a) < Async
  C₁₀: dom(b) < Async
  C₁₁: dom(c) < Async
  C₁₂: dom(r_a) < Async
  C₁₃: dom(r_b) < Async
  C₁₄: dom(r_c) < Async
  C₁₅: dom(concat_regs) < Async
  C₁₆: dom(concat_with_literal) < Async
  C₁₇: dom(mixed) < Async

Equality (component references):
  C₁₈: dom(#0) = dom(clock)
  C₁₉: dom(#1) = dom(clock)
  C₂₀: dom(#2) = dom(clock)
  C₂₁: dom(#3) = dom(a)
  C₂₂: dom(#4) = dom(b)
  C₂₃: dom(#5) = dom(c)
  C₂₄: dom(#7) = dom(r_a)
  C₂₅: dom(#8) = dom(r_b)
  C₂₆: dom(#9) = dom(r_c)
  C₂₇: dom(#12) = dom(r_a)
  C₂₈: dom(#13) = dom(r_b)
  C₂₉: dom(#14) = dom(r_c)
  C₃₀: dom(#18) = dom(r_a)
  C₃₁: dom(#19) = dom(r_b)
  C₃₂: dom(#20) = dom(r_c)

Equality (word() unification):
  C₃₃: dom(#7) = dom(#6)
  C₃₄: dom(#8) = dom(#6)
  C₃₅: dom(#9) = dom(#6)
  C₃₆: dom(#11) = dom(#10)
  C₃₇: dom(#12) = dom(#10)
  C₃₈: dom(#13) = dom(#10)
  C₃₉: dom(#14) = dom(#10)
  C₄₀: dom(#16) = dom(#15)
  C₄₁: dom(#17) = dom(#15)
  C₄₂: dom(#18) = dom(#15)
  C₄₃: dom(#19) = dom(#15)
  C₄₄: dom(#20) = dom(#15)

Equality (drivers):
  C₄₅: dom(r_a) = dom(#3)
  C₄₆: dom(r_b) = dom(#4)
  C₄₇: dom(r_c) = dom(#5)
  C₄₈: dom(concat_regs) = dom(#6)
  C₄₉: dom(concat_with_literal) = dom(#10)
  C₅₀: dom(mixed) = dom(#15)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {a}, {b}, {c}, {r_a}, {r_b}, {r_c}, {concat_regs}, {concat_with_literal}, {mixed},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9},
{#10}, {#11}, {#12}, {#13}, {#14}, {#15}, {#16}, {#17}, {#18}, {#19}, {#20}
```

**Step 1: Process fixed constraints (C₁-C₇)**

C₁: `dom(clock) = Clock`
```
{clock: Clock}, ...
```

C₂: `dom(r_a) = Sync(ComponentId(clock))`
```
{clock: Clock}, {a}, {b}, {c}, {r_a: Sync(clock)}, {r_b}, {r_c}, ...
```

C₃: `dom(r_b) = Sync(ComponentId(clock))`
```
{clock: Clock}, {a}, {b}, {c}, {r_a: Sync(clock)}, {r_b: Sync(clock)}, {r_c}, ...
```

C₄: `dom(r_c) = Sync(ComponentId(clock))`
```
{clock: Clock}, {a}, {b}, {c}, {r_a: Sync(clock)}, {r_b: Sync(clock)}, {r_c: Sync(clock)}, ...
```

C₅: `dom(#11) = Constant`
```
... {#11: Constant}, ...
```

C₆: `dom(#16) = Constant`
```
... {#16: Constant}, ...
```

C₇: `dom(#17) = Constant`
```
... {#17: Constant}, ...
```

**Step 2: Process equality constraints**

I'll show key merges:

**C₁₈: `dom(#0) = dom(clock)`**
Merge #0 with clock:
```
{clock, #0: Clock}, ...
```

**C₂₁: `dom(#3) = dom(a)`**
Merge #3 with a:
```
{a, #3}, ...
```

**C₂₄: `dom(#7) = dom(r_a)`**
Merge #7 with r_a:
```
{r_a, #7: Sync(clock)}, ...
```

**C₂₅: `dom(#8) = dom(r_b)`**
Merge #8 with r_b:
```
{r_b, #8: Sync(clock)}, ...
```

**C₂₆: `dom(#9) = dom(r_c)`**
Merge #9 with r_c:
```
{r_c, #9: Sync(clock)}, ...
```

**C₃₃: `dom(#7) = dom(#6)` (word() unification)**
Merge #6 with r_a's class (which includes #7):
```
{r_a, #6, #7: Sync(clock)}, ...
```

**C₃₄: `dom(#8) = dom(#6)` (word() unification)**
Merge r_b's class with r_a's class:
- r_b's class: `{r_b, #8: Sync(clock)}`
- r_a's class: `{r_a, #6, #7: Sync(clock)}`
- Both have `Sync(clock)` → compatible → merge
```
{r_a, r_b, #6, #7, #8: Sync(clock)}, ...
```

**C₃₅: `dom(#9) = dom(#6)` (word() unification)**
Merge r_c's class with the growing class:
```
{r_a, r_b, r_c, #6, #7, #8, #9: Sync(clock)}, ...
```

**C₄₅: `dom(r_a) = dom(#3)` (driver)**
Merge the large class with a's class:
- Large class: `{r_a, r_b, r_c, #6, #7, #8, #9: Sync(clock)}`
- a's class: `{a, #3}` (no domain yet)
- Result: a gets `Sync(clock)`
```
{a, #3, r_a, r_b, r_c, #6, #7, #8, #9: Sync(clock)}, ...
```

**C₄₆: `dom(r_b) = dom(#4)` (driver)**
Merge b's class with the large class:
```
{a, b, #3, #4, r_a, r_b, r_c, #6, #7, #8, #9: Sync(clock)}, ...
```

**C₄₇: `dom(r_c) = dom(#5)` (driver)**
Merge c's class with the large class:
```
{a, b, c, #3, #4, #5, r_a, r_b, r_c, #6, #7, #8, #9: Sync(clock)}, ...
```

**C₄₈: `dom(concat_regs) = dom(#6)` (driver)**
Merge concat_regs with the large class:
```
{a, b, c, #3, #4, #5, r_a, r_b, r_c, #6, #7, #8, #9, concat_regs: Sync(clock)}, ...
```

Now let's handle the word() with literal (expression #10):

**C₂₇: `dom(#12) = dom(r_a)`**
Merge #12 with the large class:
```
{a, b, c, #3, #4, #5, r_a, r_b, r_c, #6, #7, #8, #9, #12, concat_regs: Sync(clock)}, ...
```

**C₂₈: `dom(#13) = dom(r_b)`**
Merge #13 with the large class:
```
{a, b, c, #3, #4, #5, r_a, r_b, r_c, #6, #7, #8, #9, #12, #13, concat_regs: Sync(clock)}, ...
```

**C₂₉: `dom(#14) = dom(r_c)`**
Merge #14 with the large class:
```
{a, b, c, #3, #4, #5, r_a, r_b, r_c, #6, #7, #8, #9, #12, #13, #14, concat_regs: Sync(clock)}, ...
```

**C₃₆: `dom(#11) = dom(#10)` (word() with literal)**
Merge #11 (Constant) with #10 (no domain yet):
```
{#10, #11: Constant}, ...
```

**C₃₇: `dom(#12) = dom(#10)` (word() with literal)**
Merge the large Sync(clock) class with #10's Constant class:
- Class 1: `{a, b, c, ..., #14, concat_regs: Sync(clock)}`
- Class 2: `{#10, #11: Constant}`
- Unify: `Constant` unifies with `Sync(clock)` → result is `Sync(clock)`
```
{a, b, c, ..., #14, #10, #11, concat_regs: Sync(clock)}, ...
```

**C₃₈, C₃₉: Already in the same class**

**C₄₉: `dom(concat_with_literal) = dom(#10)`**
Merge concat_with_literal with the large class:
```
{a, b, c, ..., #14, #10, #11, concat_regs, concat_with_literal: Sync(clock)}, ...
```

Similarly for expression #15 (word with two literals):

**C₃₀-C₃₂, C₄₀-C₄₄:** All merge into the large class
**C₅₀: `dom(mixed) = dom(#15)`**
Merge mixed with the large class.

**Final partition (simplified view):**
```
Class 1: {clock, #0, #1, #2} → Clock
Class 2: {a, b, c, r_a, r_b, r_c, concat_regs, concat_with_literal, mixed,
          #3, #4, #5, #6, #7, #8, #9, #10, #11, #12, #13, #14, #15, #16, #17, #18, #19, #20}
         → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(a) = Sync(ComponentId(clock))
  dom(b) = Sync(ComponentId(clock))
  dom(c) = Sync(ComponentId(clock))
  dom(r_a) = Sync(ComponentId(clock))
  dom(r_b) = Sync(ComponentId(clock))
  dom(r_c) = Sync(ComponentId(clock))
  dom(concat_regs) = Sync(ComponentId(clock))
  dom(concat_with_literal) = Sync(ComponentId(clock))
  dom(mixed) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Clock
  dom(#2) = Clock
  dom(#3-#20) = Sync(ComponentId(clock))
    [including all component references, word expressions, and literals]
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(r_a) = Sync(clock)` ✓
  - `dom(r_b) = Sync(clock)` ✓
  - `dom(r_c) = Sync(clock)` ✓
  - `dom(#11) = Constant` initially, unified to `Sync(clock)` ✓
  - `dom(#16) = Constant` initially, unified to `Sync(clock)` ✓
  - `dom(#17) = Constant` initially, unified to `Sync(clock)` ✓
- All strict upper bound constraints satisfied: ✓
  - All checked domains are either `Clock` or `Sync(clock)`, both satisfy `< Async` ✓
- All equality constraints satisfied: ✓
  - All variables in large class have domain `Sync(clock)` ✓
  - `Constant` literals successfully unified with `Sync(clock)` signals ✓

**Key insight**: The literals (#11, #16, #17) start with domain `Constant`, but through the word() unification constraints, they unify with the `Sync(clock)` operands. Since `Constant` is the bottom of the lattice and unifies with any `Sync` domain, the final result is `Sync(clock)`.

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)` (clock port itself)
- `dom(a) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(b) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(c) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(concat_regs) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(concat_with_literal) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(mixed) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        a: Var(α),
        b: Var(α),
        c: Var(α),
        concat_regs: Var(α),
        concat_with_literal: Var(α),
        mixed: Var(α),
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: WordConcatExample,
    component_domains: {
        clock: Clock,
        a: Sync(ComponentId(clock)),
        b: Sync(ComponentId(clock)),
        c: Sync(ComponentId(clock)),
        r_a: Sync(ComponentId(clock)),
        r_b: Sync(ComponentId(clock)),
        r_c: Sync(ComponentId(clock)),
        concat_regs: Sync(ComponentId(clock)),
        concat_with_literal: Sync(ComponentId(clock)),
        mixed: Sync(ComponentId(clock)),
    },
    signature: DomainSignature {
        domain_params: { clock: DomainVar(0) },
        port_domains: {
            clock: Concrete(Clock),
            a: Var(DomainVar(0)),
            b: Var(DomainVar(0)),
            c: Var(DomainVar(0)),
            concat_regs: Var(DomainVar(0)),
            concat_with_literal: Var(DomainVar(0)),
            mixed: Var(DomainVar(0)),
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes**, with one addition: The algorithm needs to explicitly handle `word()` concatenation expressions in `walk_expression()`. Currently, ALGORITHM.md doesn't list the `word()` case.

The handling follows the same pattern as binary operators, except with N operands instead of 2:

```python
# Word concatenation
elif expr.kind == Word:
    operand_exprs = []
    
    # Process all operands
    for operand in expr.operands:
        expr_id, operand_expr = walk_expression(operand, &mut variables,
                                               &mut equalities, &mut upper_bounds, expr_id)
        operand_exprs.append(operand_expr)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # All operands must equal result (variadic unification)
    for operand_expr in operand_exprs:
        equalities.append(Equality(operand_expr, result_expr))
    
    return (expr_id, result_expr)
```

**Key insight**: `word()` is essentially a variadic version of binary operators. Instead of generating two equality constraints (left = result, right = result), it generates N equality constraints (operand₁ = result, operand₂ = result, ..., operandₙ = result).

### Any surprises or edge cases?

**No surprises**. The `word()` concatenation behaves as expected:

1. **All operands must unify**: The equality constraints force all operands and the result into the same equivalence class during solving.

2. **Literals unify naturally**: `Constant` domain literals unify with `Sync(clock)` operands through the lattice's unify() operation, resulting in `Sync(clock)`.

3. **Same behavior as binary operators**: The constraint pattern is identical to binary operators, just with more operands. This validates the "variadic binary operator" description in the task.

4. **Error detection works the same**: If operands from different clocks were provided (e.g., `word(sync_a, sync_b)` where sync_a is on clock1 and sync_b is on clock2), the union-find merge would detect the incompatibility during equality constraint processing.

**Example of what would fail**:
```virdant
reg r_a : Bit on clock1 { ... }
reg r_b : Bit on clock2 { ... }
wire w : Word[2] {
    it := word(r_a, r_b)  // ❌ Error: Sync(clock1) vs Sync(clock2)
}
```

During solving, when processing `dom(r_a) = dom(#result)` and then `dom(r_b) = dom(#result)`, we'd try to merge classes with incompatible domains (`Sync(clock1)` and `Sync(clock2)`), triggering a unification error.

### Does the constraint system handle this without special cases?

**Yes**, completely naturally:

1. **Constraint generation**: Just generate N equality constraints for N operands
2. **Constraint solving**: Standard union-find merging handles variadic unification
3. **Domain propagation**: Works bidirectionally through all operands
4. **Error detection**: Unification errors caught during merges
5. **Literal handling**: `Constant` domain unifies with any `Sync` through the existing unify() function

No special-case logic needed anywhere. The existing constraint system handles 2-operand, 3-operand, 5-operand, or any N-operand concatenation identically.

### Any clarifications needed for ALGORITHM.md or PLANv2.md?

**ALGORITHM.md additions needed:**

Add the `Word` case to `walk_expression()` in Pass 1:

```python
# Word concatenation (e.g., word(a, b, c))
elif expr.kind == Word:
    operand_exprs = []
    
    # Recurse on each operand
    for operand in expr.operands:
        expr_id, operand_expr = walk_expression(operand, &mut variables,
                                               &mut equalities, &mut upper_bounds, expr_id)
        operand_exprs.append(operand_expr)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Variadic unification rule: all operands must equal result
    for operand_expr in operand_exprs:
        equalities.append(Equality(operand_expr, result_expr))
    
    return (expr_id, result_expr)
```

**PLANv2.md clarifications:**

Add `word()` to the "Combinational Expressions" section, noting it follows the same rule as binary operators:

```markdown
### Combinational Expressions

For combinational operations (`a | b`, `a + b`, `word(a, b, c)`, etc.):

**Rule**: All operands must unify to the same domain, and the result has that domain.

...

Examples:
```virdant
// word() with same-clock registers
reg a : Bit on clk { ... }
reg b : Bit on clk { ... }
wire w : Word[2] {
    it := word(a, b)  // ✓ both Sync(clk)
}

// word() with literals (Constant unifies to Sync)
wire w : Word[3] {
    it := word(true, a, b)  // ✓ Constant unifies with Sync(clk)
}

// word() with different clocks → ERROR
reg a : Bit on clk_a { ... }
reg b : Bit on clk_b { ... }
wire w : Word[2] {
    it := word(a, b)  // ✗ ERROR: incompatible domains
}
```
```

### Relationship to binary operators

`word()` concatenation is the **variadic generalization** of binary operators:

| Construct | Operands | Constraint Pattern |
|-----------|----------|-------------------|
| Binary op `a + b` | 2 | `dom(a) = dom(result)`, `dom(b) = dom(result)` |
| word(a, b) | 2 | `dom(a) = dom(result)`, `dom(b) = dom(result)` |
| word(a, b, c) | 3 | `dom(a) = dom(result)`, `dom(b) = dom(result)`, `dom(c) = dom(result)` |
| word(a, b, c, d) | 4 | `dom(a) = dom(result)`, ..., `dom(d) = dom(result)` |

The constraint generation pattern is identical; `word()` just applies it to N operands instead of exactly 2.

### Interaction with the lattice

This example demonstrates how the domain lattice's unify() operation handles mixed operands:

1. **All Sync(clock)**: Unify to `Sync(clock)` ✓
2. **Sync(clock) + Constant**: Constant is bottom, unifies to `Sync(clock)` ✓
3. **Multiple Constants**: All bottom, would unify to the first non-Constant operand, or stay `Constant` if all operands are Constant
4. **Different Sync domains**: Incompatible, unification error ❌

This validates the lattice design where `Constant` is the bottom element that can unify with any `Sync` domain.

### Summary

`word()` concatenation:
- ✅ Works naturally with the constraint system
- ✅ No special solver logic needed
- ✅ Follows the exact pattern as binary operators, just with N operands
- ✅ Properly demonstrates Constant/Sync unification
- ✅ Error detection works identically to binary operators
- ✅ Only requires adding the `Word` case to `walk_expression()`

**Recommendation**: Add the `Word` case to ALGORITHM.md's `walk_expression()` function, and add a brief note in PLANv2.md that `word()` follows the same domain rule as binary operators.
