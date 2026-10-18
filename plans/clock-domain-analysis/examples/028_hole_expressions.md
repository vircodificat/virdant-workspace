# Example 28: Hole Expressions (`?`) Domain

```virdant
mod HoleExample {
    incoming clock : Clock
    incoming sync_enable : Bit
    incoming async_enable : Bit async
    incoming async_input : Bit async
    
    reg counter : Word[8] on clock {
        it <= when {
            case sync_enable => counter + 1
            else => ?  // Hole: will be filled in later by user
        }
    }
    
    wire async_result : Bit async {
        it := when {
            case async_enable => async_input
            else => ?  // Hole in async context - will adopt Async domain
        }
    }
    
    outgoing result : Word[8] {
        it := when {
            case sync_enable => counter
            else => ?  // Another hole
        }
    }
    
    wire unconstrained : Word[8] {
        it := ?  // Hole with no constraints - stays free
    }
}
```

**What's happening**: Holes (`?`) are unfilled expression placeholders used during development to represent code the user hasn't written yet. They are **free variables** (unconstrained) that adopt whatever domain their context requires through unification. Unlike literals or `Constant` values, holes can unify with any domain - including `Async` - because they're placeholders for arbitrary future expressions.

**Key semantic difference from `dontcare`**:
- **Hole (`?`)**: "I will write code here later" - generates `UnfilledHole` diagnostic
- **dontcare**: "Compiler, choose an appropriate value" - no diagnostic, just a flexible placeholder

Both are **free variables** that unify with their context, but holes warn the user about incomplete code.

**Expected behavior**: 
- Domain analysis succeeds
- Holes unify with their context (Sync or Async as needed)
- Separate diagnostics warn about unfilled holes
- Unconstrained holes remain free (polymorphic)

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `sync_enable` (incoming, type Bit)
- `async_enable` (incoming, type Bit, async modifier)
- `async_input` (incoming, type Bit, async modifier)
- `counter` (reg, type Word[8])
- `async_result` (wire, type Bit, async modifier)
- `result` (outgoing, type Word[8])
- `unconstrained` (wire, type Word[8])

### Step 2 - Walk declarations and drivers, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- No expressions

**Declaration: `incoming sync_enable : Bit`**
- Component: `sync_enable`
- No expressions

**Declaration: `incoming async_enable : Bit async`**
- Component: `async_enable`
- Has `async` modifier
- No expressions

**Declaration: `incoming async_input : Bit async`**
- Component: `async_input`
- Has `async` modifier
- No expressions

**Declaration: `reg counter : Word[8] on clock`**
- Component: `counter`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `counter <= when { case sync_enable => counter + 1; else => ? }`**
- Expression `#1`: `when` expression (RHS of driver)
  - Guard `#2`: reference to `sync_enable` (case guard)
  - Guard result `#3`: binary operation `counter + 1`
    - Left operand `#4`: reference to `counter`
    - Right operand `#5`: literal `1`
  - Else result `#6`: **hole** `?`

**Declaration: `wire async_result : Bit async`**
- Component: `async_result`
- Has `async` modifier
- No expressions in declaration

**Driver: `async_result := when { case async_enable => async_input; else => ? }`**
- Expression `#7`: `when` expression (RHS of driver)
  - Guard `#8`: reference to `async_enable`
  - Guard result `#9`: reference to `async_input`
  - Else result `#10`: **hole** `?`

**Declaration: `outgoing result : Word[8]`**
- Component: `result`
- No expressions in declaration

**Driver: `result := when { case sync_enable => counter; else => ? }`**
- Expression `#11`: `when` expression (RHS of driver)
  - Guard `#12`: reference to `sync_enable`
  - Guard result `#13`: reference to `counter`
  - Else result `#14`: **hole** `?`

**Declaration: `wire unconstrained : Word[8]`**
- Component: `unconstrained`
- No expressions in declaration

**Driver: `unconstrained := ?`**
- Expression `#15`: **hole** `?`

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprWhen (first when expression)
- `#2`: ExprReference to `sync_enable` (first when guard)
- `#3`: ExprBinaryOp `+` (first when arm)
- `#4`: ExprReference to `counter` (left operand)
- `#5`: ExprLiteral `1` (right operand)
- `#6`: **ExprHole** `?` (first when else arm)
- `#7`: ExprWhen (second when expression - async context)
- `#8`: ExprReference to `async_enable` (second when guard)
- `#9`: ExprReference to `async_input`
- `#10`: **ExprHole** `?` (second when else arm - async context!)
- `#11`: ExprWhen (third when expression)
- `#12`: ExprReference to `sync_enable` (third when guard)
- `#13`: ExprReference to `counter`
- `#14`: **ExprHole** `?` (third when else arm)
- `#15`: **ExprHole** `?` (unconstrained driver)

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from async modifier:**
- `dom(async_enable) = Async` (explicitly declared async input)
- `dom(async_input) = Async` (explicitly declared async input)
- `dom(async_result) = Async` (explicitly declared async wire)

**Fixed constraints from register declarations:**
- `dom(counter) = Sync(ComponentId(clock))`

**Fixed constraints from literals:**
- `dom(#5) = Constant` (literal `1`)

**Hole constraints - FREE VARIABLES:**
- **NO Fixed constraint for #6** - hole is a free variable
- **NO Fixed constraint for #10** - hole is a free variable
- **NO Fixed constraint for #14** - hole is a free variable
- **NO Fixed constraint for #15** - hole is a free variable

**Key insight**: Holes are **free variables** (unconstrained). They will adopt whatever domain their context requires through equality constraints. This makes them placeholders for arbitrary future expressions, which could have any domain.

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#2) = dom(sync_enable)`
- `dom(#4) = dom(counter)`
- `dom(#8) = dom(async_enable)`
- `dom(#9) = dom(async_input)`
- `dom(#12) = dom(sync_enable)`
- `dom(#13) = dom(counter)`

**Constraints from binary operation (`counter + 1`):**
- Result expression: `#3`
- Left operand: `#4`, Right operand: `#5`
- Unified operator rule: operands can upcast to result
  - `dom(#4) ≤ dom(#3)` (upper bound)
  - `dom(#5) ≤ dom(#3)` (upper bound)

**Constraints from first when expression:**
- When expression `#1` has one guard (`#2`) and two result arms (`#3`, `#6`)
- All guards and all arms must equal the result:
  - `dom(#2) = dom(#1)` (guard equals result)
  - `dom(#3) = dom(#1)` (first arm result equals when result)
  - `dom(#6) = dom(#1)` (else arm **hole** equals when result) ← Hole will adopt #1's domain

**Constraints from second when expression (async context):**
- When expression `#7` has one guard (`#8`) and two result arms (`#9`, `#10`)
- All guards and all arms must equal the result:
  - `dom(#8) = dom(#7)` (guard equals result)
  - `dom(#9) = dom(#7)` (first arm result equals when result)
  - `dom(#10) = dom(#7)` (else arm **hole** equals when result) ← Hole will adopt #7's domain

**Constraints from third when expression:**
- When expression `#11` has one guard (`#12`) and two result arms (`#13`, `#14`)
- All guards and all arms must equal the result:
  - `dom(#12) = dom(#11)` (guard equals result)
  - `dom(#13) = dom(#11)` (first arm result equals when result)
  - `dom(#14) = dom(#11)` (else arm **hole** equals when result) ← Hole will adopt #11's domain

**Equality constraints from drivers:**

Driver: `counter <= when { ... }`
- LHS: component `counter`
- RHS: expression `#1`
- Constraint: `dom(counter) = dom(#1)`

Driver: `async_result := when { ... }`
- LHS: component `async_result`
- RHS: expression `#7`
- Constraint: `dom(async_result) = dom(#7)`

Driver: `result := when { ... }`
- LHS: component `result`
- RHS: expression `#11`
- Constraint: `dom(result) = dom(#11)`

Driver: `unconstrained := ?`
- LHS: component `unconstrained`
- RHS: expression `#15`
- Constraint: `dom(unconstrained) = dom(#15)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(sync_enable) < Async`
- `dom(counter) < Async`
- `dom(result) < Async`
- `dom(unconstrained) < Async`
- **NOTE:** `async_enable`, `async_input`, and `async_result` do NOT get this constraint

### Complete constraint system:
```
Fixed:
  F₁: dom(clock) = Clock
  F₂: dom(async_enable) = Async
  F₃: dom(async_input) = Async
  F₄: dom(async_result) = Async
  F₅: dom(counter) = Sync(ComponentId(clock))
  F₆: dom(#5) = Constant             [literal 1]

Strict upper bounds:
  S₁: dom(clock) < Async
  S₂: dom(sync_enable) < Async
  S₃: dom(counter) < Async
  S₄: dom(result) < Async
  S₅: dom(unconstrained) < Async

Upper bounds:
  U₁: dom(#4) ≤ dom(#3)
  U₂: dom(#5) ≤ dom(#3)

Equality:
  E₁: dom(#0) = dom(clock)
  E₂: dom(#2) = dom(sync_enable)
  E₃: dom(#4) = dom(counter)
  E₄: dom(#8) = dom(async_enable)
  E₅: dom(#9) = dom(async_input)
  E₆: dom(#12) = dom(sync_enable)
  E₇: dom(#13) = dom(counter)
  
  [First when expression]
  E₈: dom(#2) = dom(#1)
  E₉: dom(#3) = dom(#1)
  E₁₀: dom(#6) = dom(#1)             [hole #6 → adopt #1's domain]
  
  [Second when expression - async context]
  E₁₁: dom(#8) = dom(#7)
  E₁₂: dom(#9) = dom(#7)
  E₁₃: dom(#10) = dom(#7)            [hole #10 → adopt #7's domain]
  
  [Third when expression]
  E₁₄: dom(#12) = dom(#11)
  E₁₅: dom(#13) = dom(#11)
  E₁₆: dom(#14) = dom(#11)           [hole #14 → adopt #11's domain]
  
  [Drivers]
  E₁₇: dom(counter) = dom(#1)
  E₁₈: dom(async_result) = dom(#7)
  E₁₉: dom(result) = dom(#11)
  E₂₀: dom(unconstrained) = dom(#15) [hole #15 → adopt unconstrained's domain]
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment.

### Initial state:
All components and expressions start in separate equivalence classes. Holes start as **free variables** (no domain assigned):
```
{clock}, {sync_enable}, {async_enable}, {async_input}, {counter}, {async_result}, {result}, {unconstrained},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6: free}, {#7}, {#8}, {#9}, 
{#10: free}, {#11}, {#12}, {#13}, {#14: free}, {#15: free}
```

### Processing fixed constraints:

**F₁: `dom(clock) = Clock`**
```
{clock: Clock}, ...
```

**F₂: `dom(async_enable) = Async`**
```
{clock: Clock}, {async_enable: Async}, ...
```

**F₃: `dom(async_input) = Async`**
```
{clock: Clock}, {async_enable: Async}, {async_input: Async}, ...
```

**F₄: `dom(async_result) = Async`**
```
{clock: Clock}, {async_enable: Async}, {async_input: Async}, {async_result: Async}, ...
```

**F₅: `dom(counter) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {async_enable: Async}, {async_input: Async}, {async_result: Async}, {counter: Sync(clock)}, ...
```

**F₆: `dom(#5) = Constant`** (literal 1)
```
..., {#5: Constant}, ...
```

### Processing equality constraints:

**Building equivalence classes through merges:**

**E₁: Merge #0 with clock**
```
{clock, #0: Clock}, ...
```

**E₃: Merge #4 with counter**
```
..., {counter, #4: Sync(clock)}, ...
```

**First when expression chain (E₈, E₉, E₁₀, E₁₇):**
- E₂: Merge #2 with sync_enable
- E₈: Merge #2 with #1 (when result)
- E₉: Merge #3 with #1
- E₁₀: **Merge #6 (hole - free variable) with #1**
- E₁₇: Merge counter with #1
- (Also U₁, U₂ upper bounds connect #4, #5 to #3)

After these merges, first when class contains: sync_enable, #2, #1, #3, counter, #4, #6 (hole)
Since counter has fixed domain Sync(clock), the entire class adopts Sync(clock).
**Hole #6 (free variable) adopts domain Sync(clock)** through unification!

```
{sync_enable, #2, #1, #3, counter, #4, #6: Sync(clock)}, ...
```

**Second when expression chain (E₁₁, E₁₂, E₁₃, E₁₈) - async context:**
- E₄: Merge #8 with async_enable (Async)
- E₁₁: Merge #8 with #7 (when result)
- E₅: Merge #9 with async_input (Async)
- E₁₂: Merge #9 with #7
- E₁₃: **Merge #10 (hole - free variable) with #7**
- E₁₈: Merge async_result (Async) with #7

After these merges, second when class contains: async_enable (Async), #8, #7, async_input (Async), #9, #10 (hole), async_result (Async)
Multiple components with fixed domain Async - all compatible!
**Hole #10 (free variable) adopts domain Async** through unification!

```
{async_enable, #8, #7, async_input, #9, #10, async_result: Async}, ...
```

**Third when expression chain (E₁₄, E₁₅, E₁₆, E₁₉):**
- E₆: Merge #12 with sync_enable (already in Sync(clock) class)
- E₁₄: Merge #12 with #11 (when result)
- E₇: Merge #13 with counter (already in Sync(clock) class)
- E₁₅: Merge #13 with #11
- E₁₆: **Merge #14 (hole - free variable) with #11**
- E₁₉: Merge result with #11

After these merges, third when class contains: sync_enable, #12, #11, counter, #13, #14 (hole), result
Since sync_enable and counter are already Sync(clock), the entire class adopts Sync(clock).
**Hole #14 (free variable) adopts domain Sync(clock)** through unification!

```
{sync_enable, #2, #1, #3, counter, #4, #6, #12, #11, #13, #14, result: Sync(clock)}, ...
```

**Fourth driver (E₂₀):**
- E₂₀: Merge unconstrained with #15 (hole - free variable)

Both unconstrained and #15 are free variables (no fixed domains yet).
The merged class remains free.
**Hole #15 (free variable) stays free** - no constraints to unify with!

```
{unconstrained, #15: none (free)}, ...
```

### Processing upper bound constraints:

**U₁: `dom(#4) ≤ dom(#3)`**
- Both #4 and #3 are in the Sync(clock) class
- Constraint satisfied: Sync(clock) ≤ Sync(clock) ✓

**U₂: `dom(#5) ≤ dom(#3)`**
- #5 has domain Constant
- #3 has domain Sync(clock)
- Constraint satisfied: Constant ≤ Sync(clock) ✓ (Constant is bottom)

### Final partition:
```
Class 1: {clock, #0} → Clock
Class 2: {sync_enable, #2, #1, #3, counter, #4, #6, #12, #11, #13, #14, result, #5} → Sync(ComponentId(clock))
Class 3: {async_enable, #8, #7, async_input, #9, #10, async_result} → Async
Class 4: {unconstrained, #15} → (no domain - free variable)
```

Note: #5 (literal Constant) gets pulled into Class 2 through the upper bound constraint and unifies to Sync(clock).

### Solution (domain assignment):
```
Components:
  dom(clock) = Clock
  dom(sync_enable) = Sync(ComponentId(clock))
  dom(async_enable) = Async
  dom(async_input) = Async
  dom(counter) = Sync(ComponentId(clock))
  dom(async_result) = Async
  dom(result) = Sync(ComponentId(clock))
  dom(unconstrained) = (no domain - free variable)

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))  [first when]
  dom(#2) = Sync(ComponentId(clock))  [sync_enable reference]
  dom(#3) = Sync(ComponentId(clock))  [counter + 1]
  dom(#4) = Sync(ComponentId(clock))  [counter reference]
  dom(#5) = Sync(ComponentId(clock))  [literal 1 - unified from Constant]
  dom(#6) = Sync(ComponentId(clock))  [hole - unified from free!]
  dom(#7) = Async                      [second when]
  dom(#8) = Async                      [async_enable reference]
  dom(#9) = Async                      [async_input reference]
  dom(#10) = Async                     [hole - unified from free!]
  dom(#11) = Sync(ComponentId(clock)) [third when]
  dom(#12) = Sync(ComponentId(clock)) [sync_enable reference]
  dom(#13) = Sync(ComponentId(clock)) [counter reference]
  dom(#14) = Sync(ComponentId(clock)) [hole - unified from free!]
  dom(#15) = (no domain - free)       [hole - stays free!]
```

### Verification:

**All fixed constraints satisfied:** ✓
- Clock, async signals, counter, literal all have correct fixed domains

**All strict upper bound constraints satisfied:** ✓
- All non-async components satisfy `< Async`
- unconstrained has no domain yet, but when assigned it must satisfy `< Async`

**All equality constraints satisfied:** ✓
- All merged classes have consistent domains
- Holes #6, #10, #14 successfully unified from free variables to Sync/Async
- Hole #15 remains free (no constraints)

**All upper bound constraints satisfied:** ✓

**Key observations:**
1. **Holes as free variables work perfectly**: They start unconstrained and adopt whatever domain their context requires
2. **Hole #6, #10, #14 unified successfully**: From free → Sync(clock) or Async depending on context
3. **Hole #15 stays free**: No constraints means it remains polymorphic (could be filled with any expression later)

---

## Pass 3: Signature Extraction

Build the module's polymorphic domain signature.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(sync_enable) = Sync(ComponentId(clock))` → `Var(α)` (tied to clock parameter)
- `dom(async_enable) = Async` → `Concrete(Async)`

### Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        sync_enable: Var(DomainVar(0)),  // α
        async_enable: Concrete(Async),
        async_input: Concrete(Async),
    }
}
```

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: HoleExample,
    component_domains: {
        clock: Clock,
        sync_enable: Sync(ComponentId(clock)),
        async_enable: Async,
        async_input: Async,
        counter: Sync(ComponentId(clock)),
        async_result: Async,
        result: Sync(ComponentId(clock)),
        unconstrained: None,  // Free variable
    },
    signature: /* as above */,
    diagnostics: [
        // Domain analysis succeeded, but separate diagnostics warn about holes:
        UnfilledHole { location: "counter <= when { ... else => ? }" },
        UnfilledHole { location: "async_result := when { ... else => ? }" },
        UnfilledHole { location: "result := when { ... else => ? }" },
        UnfilledHole { location: "unconstrained := ?" },
    ],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm?

**Yes, perfectly!** Holes work as **free variables** (unconstrained) that adopt their context's domain:

1. **No Fixed constraint**: Unlike literals (which are Fixed to Constant), holes start with no domain constraint
2. **Unification works naturally**: Free variables unify with any domain through equality constraints
3. **Context determines domain**: 
   - Hole in Sync(clock) context → becomes Sync(clock)
   - Hole in Async context → becomes Async
   - Hole with no context → stays free (polymorphic)
4. **No special cases**: The constraint system handles holes through standard unification
5. **Orthogonal to type checking**: Domain analysis succeeds even with holes; a separate diagnostic warns about unfilled holes

### Critical distinction: Holes vs dontcare vs Literals

| Expression | Domain Constraint | Semantics | Can be Async? |
|------------|-------------------|-----------|---------------|
| **Literal** (`42`, `true`) | `Fixed(Constant)` | Explicit constant value | No (fixed to Constant) |
| **dontcare** | None (free) | "Compiler, choose a value" | Yes (unifies with context) |
| **Hole** (`?`) | None (free) | "I'll write code here later" | Yes (unifies with context) |

**Key insight**: Both `dontcare` and holes are **free variables**, but:
- `dontcare`: No diagnostic (intentional flexibility)
- Holes: Generate `UnfilledHole` diagnostic (incomplete code warning)

### Holes enable flexible development

1. **Sketch module structure first**: Use `?` as placeholders while designing
2. **Domain correctness preserved**: Even incomplete code has correct domain semantics
3. **Polymorphic holes**: Unconstrained holes stay free, accepting any domain when filled in
4. **Context-aware holes**: Holes in specific contexts immediately adopt the required domain

### Examples from this analysis:

**Hole #6 in Sync(clock) context:**
```virdant
reg counter : Word[8] on clock {
    it <= when {
        case sync_enable => counter + 1
        else => ?  // Free variable → unified to Sync(clock)
    }
}
```
When the user fills this in, they must provide a `Sync(clock)` expression.

**Hole #10 in Async context:**
```virdant
wire async_result : Bit async {
    it := when {
        case async_enable => async_input
        else => ?  // Free variable → unified to Async
    }
}
```
When the user fills this in, they must provide an `Async` expression.

**Hole #15 unconstrained:**
```virdant
wire unconstrained : Word[8] {
    it := ?  // Free variable → stays free
}
```
When the user fills this in, they can provide **any** expression (any domain except Async due to the wire's implicit `< Async` constraint).

### Interaction with dontcare

The exact same analysis applies to `dontcare`:

```virdant
reg r : Word[8] on clk {
    it <= when {
        case enable => r + 1
        else => dontcare  // Free variable → Sync(clk)
    }
}

wire async_w : Bit async {
    it := dontcare  // Free variable → could be any domain, including Async
}
```

**Both hole and dontcare**:
- Start as free variables
- Unify with their context
- Can become Async if context requires it
- Difference is only in diagnostics

### Algorithm coverage:

The algorithm handles holes correctly through:
- **Pass 1**: Create variable for hole, **generate NO Fixed constraint** (free variable)
- **Pass 2**: Hole unifies with context through equality constraints (or stays free if unconstrained)
- **Pass 3**: Free holes don't affect signature (they're internal placeholders)

### Clarifications needed for ALGORITHM.md or PLANv2.md?

**ALGORITHM.md update needed**:

Add to `walk_expression()`:

```python
elif expr.kind == Hole:
    expr_node = expr_id
    expr_id += 1
    variables.add(expr_node)
    # NO Fixed constraint - holes are free variables
    # They will unify with context through equality constraints
    return (expr_id, expr_node)
```

**Contrast with literal**:
```python
elif expr.kind == Literal:
    expr_node = expr_id
    expr_id += 1
    variables.add(expr_node)
    fixed.append(Fixed(expr_node, Constant))  # Literals ARE fixed
    return (expr_id, expr_node)
```

**PLANv2.md clarification**:

Add section on "Free Variables and Placeholders":

> ### Free Variables: Holes and dontcare
>
> Two expression forms create **free variables** (unconstrained domain variables):
>
> **Holes (`?`)**: Placeholders for code to be written later
> ```virdant
> wire w : Bit {
>     it := ?  // Free variable, will unify with any context
> }
> ```
>
> **dontcare**: Compiler-chosen values
> ```virdant
> wire w : Bit {
>     it := dontcare  // Free variable, compiler chooses
> }
> ```
>
> **Rule**: Free variables have no Fixed domain constraint. They adopt whatever domain their context requires through equality constraints. If unconstrained, they remain free (polymorphic).
>
> **Key property**: Free variables can unify with **any** domain, including `Async`, making them maximally flexible placeholders.
>
> **Difference**: Holes generate `UnfilledHole` diagnostics; dontcare does not.

### Example 021 (dontcare) should be corrected

Based on this understanding, Example 021 should be updated:
- Change `Fixed(dontcare, Constant)` to no Fixed constraint (free variable)
- Show dontcare unifying with Sync(clock) through context

### Design strength: Maximum flexibility

This example showcases how free variables enable flexible, incomplete code:

1. **Development workflow**: Use `?` to sketch structure before filling in details
2. **Domain correctness from the start**: Even sketches have correct domain semantics
3. **Context propagation**: Holes automatically adopt required domains
4. **Polymorphic placeholders**: Unconstrained holes remain flexible
5. **No special-case logic**: Free variables participate in constraint solving naturally

### Summary

**Holes (`?`):**
- ✅ Free variables (no Fixed constraint, unlike literals)
- ✅ Unify naturally with context through equality constraints
- ✅ Can adopt any domain including Async
- ✅ Unconstrained holes stay free (polymorphic)
- ✅ No algorithm changes needed (just add the case to walk_expression with no Fixed constraint)
- ✅ Separate `UnfilledHole` diagnostics warn about incomplete code
- ✅ Enable incremental development with correct domain semantics

**dontcare:**
- ✅ Same domain behavior as holes (free variables)
- ✅ No diagnostic (intentional compiler choice, not incomplete code)

**Recommendation**: 
1. Add hole expression handling to ALGORITHM.md (no Fixed constraint)
2. Update Example 021 (dontcare) to reflect free variable semantics
3. Add "Free Variables" section to PLANv2.md
