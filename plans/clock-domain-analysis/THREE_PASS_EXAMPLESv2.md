# Three-Pass Algorithm Examples (v2)

This document walks through examples of every situation the domain analysis algorithm encounters, showing the data structures built during each pass.

## Algorithm Overview

**Passes:**
1. **Constraint Generation**: Walk the module's AST to identify all components and expressions, then generate constraints (fixed and equality).
2. **Constraint Solving**: Use union-find to merge equivalence classes according to equality constraints, propagating concrete domains to find a domain assignment that satisfies all constraints.
3. **Signature Extraction**: Build the module's polymorphic domain signature for use at instantiation sites.

**Constraint Types:**
- **Fixed constraint**: `dom(x) = D` - x must be exactly domain D
- **Upper bound constraint**: `D ≤ dom(x)` - x must be at least as specific as D (D is an upper bound on generality)
- **Strict upper bound constraint**: `dom(x) < D` - x must be strictly less specific than D (cannot equal D)
- **Equality constraint**: `dom(x) = dom(y)` - x and y must have the same domain
- **Free variables** - expressions with no constraints that adopt whatever domain is needed

**Key constraint**: ALL components have an implicit constraint `dom(c) < Async` (strictly less than Async) — meaning they cannot be Async unless explicitly declared `async`. This is the "async must be explicit" rule.

**Note on upper bounds**: Since Constant is the bottom element, `Constant ≤ dom(x)` is vacuously true for all x. We don't use upper bound constraints for literals. However, the mechanism exists and is used for binary operators.

**Lattice ordering** `≤` means "at most as specific as" (or "at least as general as"):
```
      Constant (bottom)
        /  |  \
   Sync(C1) Sync(C2) ...
        \  |  /
       Async (top)
       Clock, Reset (special)
```
- `Constant ≤ Sync(c)` for any c
- `Sync(c) ≤ Async` for any c (but not used for automatic subsumption!)
- `d ≤ d` (reflexive)

**Important**: The lattice exists, but we don't use it for automatic subsumption. The "async must be explicit" principle prevents automatic upcasting.

**Key insights**: 
- We're solving a constraint satisfaction problem with fixed, upper bound, strict upper bound, and equality constraints
- Constraints are declarative mathematical predicates
- The union-find algorithm is an implementation detail for finding a solution
- Literals are free variables that adopt whatever domain is needed
- ALL components have implicit `dom(c) < Async` unless declared `async` (the "async must be explicit" rule)

**Critical realization**: Every expression is an AST node that participates in constraint solving. Component references create expression nodes that are constrained to equal the component they reference.

---

## Example 1: Simple Single-Clock Module

```virdant
mod Simple {
    incoming clock : Clock
    incoming data : Word[8]

    reg buffer : Word[8] on clock {
        it <= data
    }

    outgoing out : Word[8] {
        it := buffer
    }
}
```

**What's happening**: A simple buffer with one clock. The `data` port flows into a register, and the register drives `out`.

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `buffer` (reg, type Word[8])
- `out` (outgoing, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming data : Word[8]`
- Component: `data`
- No expressions

Declaration: `reg buffer : Word[8] on clock`
- Component: `buffer`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing out : Word[8]`
- Component: `out`
- No expressions in declaration

Driver: `buffer <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Driver: `out := buffer`
- Expression `#2`: reference to `buffer` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprReference to `buffer`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `data` → `dom(#1) = dom(data)`
- Expression `#2` references `buffer` → `dom(#2) = dom(buffer)`

**Equality constraints from drivers:**

Driver: `buffer <= data`
- LHS: component `buffer`
- RHS: expression `#1`
- Constraint: `dom(buffer) = dom(#1)`

Driver: `out := buffer`
- LHS: component `out`
- RHS: expression `#2`
- Constraint: `dom(out) = dom(#2)`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(buffer) = Sync(ComponentId(clock))

Equality:
  C₃: dom(#0) = dom(clock)
  C₄: dom(#1) = dom(data)
  C₅: dom(#2) = dom(buffer)
  C₆: dom(buffer) = dom(#1)
  C₇: dom(out) = dom(#2)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**

Assign concrete domain `Clock` to clock's equivalence class:
```
{clock: Clock}, {data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

**Processing fixed constraint C₂: `dom(buffer) = Sync(ComponentId(clock))`**

Assign concrete domain `Sync(clock)` to buffer's equivalence class:
```
{clock: Clock}, {data}, {buffer: Sync(clock)}, {out}, {#0}, {#1}, {#2}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {data}, {buffer: Sync(clock)}, {out}, {#1}, {#2}
```

**Processing equality constraint C₄: `dom(#1) = dom(data)`**

Merge expression #1 with data's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer: Sync(clock)}, {out}, {#2}
```

**Processing equality constraint C₅: `dom(#2) = dom(buffer)`**

Merge expression #2 with buffer's equivalence class:
```
{clock, #0: Clock}, {data, #1}, {buffer, #2: Sync(clock)}, {out}
```

**Processing equality constraint C₆: `dom(buffer) = dom(#1)`**

Merge buffer's class with #1's class:
- buffer's class: `{buffer, #2: Sync(clock)}` (has concrete domain)
- #1's class: `{data, #1}` (no concrete domain yet)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {data, #1, buffer, #2: Sync(clock)}, {out}
```

**Processing equality constraint C₇: `dom(out) = dom(#2)`**

Merge out with #2's class:
- #2's class: `{data, #1, buffer, #2: Sync(clock)}`
- out's class: `{out}` (no concrete domain)
- Merged class keeps `Sync(clock)`

```
{clock, #0: Clock}, {data, #1, buffer, #2, out: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, buffer, #2, out} → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(buffer) = Sync(ComponentId(clock))
  dom(out) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(buffer) = Sync(clock)` ✓
- All equality constraints satisfied: ✓
  - All variables in Class 1 have domain `Clock` ✓
  - All variables in Class 2 have domain `Sync(clock)` ✓

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → Clock (concrete, not a parameter)
- `dom(data) = Sync(ComponentId(clock))` → α (the domain parameter)
- `dom(out) = Sync(ComponentId(clock))` → α (the domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
        data: ???,   // α
        out: ???,    // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: Simple,
    component_domains: {
        clock: Clock,
        data: Sync(ComponentId(clock)),
        buffer: Sync(ComponentId(clock)),
        out: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Example 2: Constant Literal

```virdant
mod ConstantExample {
    incoming clock : Clock

    reg counter : Word[8] on clock {
        it <= 0w8
    }
}
```

**What's happening**: A register driven by a constant literal. Literals impose lower bound constraints (at least as specific as Constant), which allows them to unify with any Sync domain.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `counter` (reg, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `reg counter : Word[8] on clock`
- Component: `counter`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Driver: `counter <= 0w8`
- Expression `#1`: the literal `0w8` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: Literal `0w8`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed)

**Fixed constraints from register declarations:**
- `dom(counter) = Sync(ComponentId(clock))` (register with `on clock`)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`

**Note on literal:**
- Expression `#1` is a literal `0w8` with no signal references
- No constraint generated - literals are free variables
- Will be solved through equality with whatever uses them

**Equality constraint from driver:**

Driver: `counter <= 0w8`
- LHS: component `counter`
- RHS: expression `#1`
- Constraint: `dom(counter) = dom(#1)`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(counter) = Sync(ComponentId(clock))

Equality:
  C₃: dom(#0) = dom(clock)
  C₄: dom(counter) = dom(#1)
```

---

### Pass 2: Constraint Solving

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {counter}, {#0}, {#1}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {counter}, {#0}, {#1}
```

**Processing fixed constraint C₂: `dom(counter) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {counter: Sync(clock)}, {#0}, {#1}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**

Merge expression #0 with clock:
```
{clock, #0: Clock}, {counter: Sync(clock)}, {#1}
```

**Processing equality constraint C₄: `dom(counter) = dom(#1)`**

Merge counter's class with #1's class:
- counter's class: `{counter: Sync(clock)}` (has concrete domain)
- #1's class: `{#1}` (no concrete domain)
- Merged class gets the concrete domain `Sync(clock)`

```
{clock, #0: Clock}, {counter, #1: Sync(clock)}
```

**Candidate solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(counter) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
```

**Verification - Check all constraints:**

Fixed constraints:
- `dom(clock) = Clock` ✓
- `dom(counter) = Sync(clock)` ✓

Equality constraints:
- `dom(#0) = dom(clock)` → both have `Clock` ✓
- `dom(counter) = dom(#1)` → both have `Sync(clock)` ✓

**All constraints satisfied! Solution is valid.**

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {counter, #1} → Sync(ComponentId(clock))
```

**Natural language**: The literal `0w8` (expression #1) had no constraints - it's a free variable. The equality constraint `dom(counter) = dom(#1)` required them to have the same domain. Since `dom(counter) = Sync(clock)`, the solution assigns `dom(#1) = Sync(clock)`. This is why literals "just work" in any clock domain - they impose no constraints and simply adopt whatever domain is needed by their context.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → Clock

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
    }
}
```

Note: `counter` is internal (not a port), so it doesn't appear in the signature.

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: ConstantExample,
    component_domains: {
        clock: Clock,
        counter: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Example 3: Binary Operator with Backward Flow

```virdant
mod BinaryOp {
    incoming clock : Clock
    incoming inp : Word[8]
    
    reg r : Word[8] on clock {
        it <= inp + 1
    }
}
```

**What's happening**: A register driven by a binary operation `inp + 1`. The register's domain requirement flows backward through the `+` operator to constrain both operands, including the input port.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `inp` (incoming, type Word[8])
- `r` (reg, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`

Declaration: `incoming inp : Word[8]`
- Component: `inp`

Declaration: `reg r : Word[8] on clock`
- Component: `r`
- Expression `#0`: reference to `clock` (in `on clock` clause)

Driver: `r <= inp + 1`
- Expression `#1`: reference to `inp` (left operand of `+`)
- Expression `#2`: literal `1` (right operand of `+`)
- Expression `#3`: `inp + 1` (the binary operation, RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `inp`
- `#2`: Literal `1`
- `#3`: BinaryOp `+` with operands `#1` and `#2`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(r) = Sync(ComponentId(clock))`

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `inp` → `dom(#1) = dom(inp)`

**Note on literal:**
- Expression `#2` is literal `1` - no constraint (free variable)

**Equality constraints from binary operator:**
- The `+` operator requires all operands to have the same domain as the result
- `dom(#3) = dom(#1)` (result equals left operand)
- `dom(#3) = dom(#2)` (result equals right operand)

**Equality constraint from driver:**
- `dom(r) = dom(#3)` (register driven by the `+` expression)

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(r) = Sync(ComponentId(clock))

Equality:
  C₃: dom(#0) = dom(clock)
  C₄: dom(#1) = dom(inp)
  C₅: dom(#3) = dom(#1)    (+ result = left operand)
  C₆: dom(#3) = dom(#2)    (+ result = right operand)
  C₇: dom(r) = dom(#3)     (driver)
```

---

### Pass 2: Constraint Solving

**Initial state:**
```
{clock}, {inp}, {r}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {inp}, {r}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint C₂: `dom(r) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {inp}, {r: Sync(clock)}, {#0}, {#1}, {#2}, {#3}
```

**Processing equality constraint C₃: `dom(#0) = dom(clock)`**
```
{clock, #0: Clock}, {inp}, {r: Sync(clock)}, {#1}, {#2}, {#3}
```

**Processing equality constraint C₄: `dom(#1) = dom(inp)`**
```
{clock, #0: Clock}, {inp, #1}, {r: Sync(clock)}, {#2}, {#3}
```

**Processing equality constraint C₅: `dom(#3) = dom(#1)`**

Merge #3 with #1's class:
```
{clock, #0: Clock}, {inp, #1, #3}, {r: Sync(clock)}, {#2}
```

**Processing equality constraint C₆: `dom(#3) = dom(#2)`**

Merge #2 with #3's class:
- #3's class: `{inp, #1, #3}` (no concrete domain)
- #2's class: `{#2}` (free variable, no constraint)
```
{clock, #0: Clock}, {inp, #1, #2, #3}, {r: Sync(clock)}
```

**Processing equality constraint C₇: `dom(r) = dom(#3)`**

Merge r's class with #3's class:
- r's class: `{r: Sync(clock)}` (has concrete domain)
- #3's class: `{inp, #1, #2, #3}` (no concrete domain)
- Merged class adopts `Sync(clock)`

```
{clock, #0: Clock}, {inp, #1, #2, #3, r: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {inp, #1, #2, #3, r} → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(inp) = Sync(ComponentId(clock))
  dom(r) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))  (free variable adopted Sync(clock))
  dom(#3) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied ✓
- All equalities satisfied ✓

**Natural language**: The constraint flow was **backward through the expression tree**. The register required `Sync(clock)`, which propagated through the driver constraint to the `+` expression, then through the operator constraints to both operands (`inp` and `1`), finally constraining the input port `inp` to be `Sync(clock)`. The literal `1` started as a free variable and adopted `Sync(clock)` through this chain.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**
- `clock` is Clock-typed → introduces `DomainVar(0)` (α)

**Express port domains in terms of parameters:**
- `dom(clock) = Clock` → Clock
- `dom(inp) = Sync(ComponentId(clock))` → α

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,  // Clock
        inp: ???,    // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: BinaryOp,
    component_domains: {
        clock: Clock,
        inp: Sync(ComponentId(clock)),
        r: Sync(ComponentId(clock)),
    },
    expression_domains: {
        // Expression domains computed during solving but not stored
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Example 4a: Mixing Clocks with Explicit Async (LEGAL)

```virdant
mod AsyncMixing {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    reg r_a : Bit on clk_a {
        it <= false
    }
    
    reg r_b : Bit on clk_b {
        it <= false
    }
    
    wire mixed : Bit async {
        it := r_a | r_b
    }
}
```

**What's happening**: Mixing signals from two different clock domains with explicit `async` target. The unified binary operator rule applies: operands can upcast to the result domain.

---

### Pass 1: Constraint Generation

**Step 1 - Components from ComponentAnalysis:**
- `clk_a` (incoming, type Clock)
- `clk_b` (incoming, type Clock)
- `r_a` (reg, type Bit)
- `r_b` (reg, type Bit)
- `mixed` (wire, type Bit, declared async)

**Step 2 - Walk declarations, identify expressions:**

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

Declaration: `wire mixed : Bit async`
- Component: `mixed` (explicitly async)

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

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clk_a) = Clock`
- `dom(clk_b) = Clock`

**Fixed constraints from register declarations:**
- `dom(r_a) = Sync(ComponentId(clk_a))`
- `dom(r_b) = Sync(ComponentId(clk_b))`

**Fixed constraints from async declarations:**
- `dom(mixed) = Async` (explicitly declared async)

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clk_a) < Async`
- `dom(clk_b) < Async`
- `dom(r_a) < Async`
- `dom(r_b) < Async`
- (mixed is declared async, so NO implicit constraint)

**Equality constraints from component references:**
- `dom(#0) = dom(clk_a)`
- `dom(#2) = dom(clk_b)`
- `dom(#4) = dom(r_a)`
- `dom(#5) = dom(r_b)`

**Equality constraints from drivers (non-async):**
- `dom(r_a) = dom(#1)`
- `dom(r_b) = dom(#3)`

**Upper bound constraints from binary operator (UNIFIED RULE):**
- `dom(#4) ≤ dom(#6)` (left operand can upcast to result)
- `dom(#5) ≤ dom(#6)` (right operand can upcast to result)

**Equality constraint from driver:**
- `dom(mixed) = dom(#6)` (wire driven by the `|` expression)

**Complete constraint system:**
```
Fixed:
  C₁: dom(clk_a) = Clock
  C₂: dom(clk_b) = Clock
  C₃: dom(r_a) = Sync(ComponentId(clk_a))
  C₄: dom(r_b) = Sync(ComponentId(clk_b))
  C₅: dom(mixed) = Async

Strict upper bound (implicit):
  C₆: dom(clk_a) < Async
  C₇: dom(clk_b) < Async
  C₈: dom(r_a) < Async
  C₉: dom(r_b) < Async

Equality:
  C₁₀: dom(#0) = dom(clk_a)
  C₁₁: dom(#2) = dom(clk_b)
  C₁₂: dom(#4) = dom(r_a)
  C₁₃: dom(#5) = dom(r_b)
  C₁₄: dom(r_a) = dom(#1)
  C₁₅: dom(r_b) = dom(#3)

Upper bound (from | operator):
  C₁₆: dom(#4) ≤ dom(#6)
  C₁₇: dom(#5) ≤ dom(#6)

Equality (from driver):
  C₁₈: dom(mixed) = dom(#6)
```

---

### Pass 2: Constraint Solving

**Initial state:**
```
{clk_a}, {clk_b}, {r_a}, {r_b}, {mixed}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing fixed constraints C₁-C₅:**
```
{clk_a: Clock}, {clk_b: Clock}, {r_a: Sync(clk_a)}, {r_b: Sync(clk_b)}, {mixed: Async}, 
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

**Processing equality constraints C₁₀-C₁₅:**

After C₁₀: `{clk_a, #0: Clock}, ...`
After C₁₁: `{clk_b, #2: Clock}, ...`
After C₁₂: `{r_a, #4: Sync(clk_a)}, ...`
After C₁₃: `{r_b, #5: Sync(clk_b)}, ...`
After C₁₄: `{r_a, #1, #4: Sync(clk_a)}, ...`
After C₁₅: `{r_b, #3, #5: Sync(clk_b)}, ...`

Current state:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)}, 
{r_b, #3, #5: Sync(clk_b)}, {mixed: Async}, {#6}
```

**Processing equality constraint C₁₈: `dom(mixed) = dom(#6)`**

Merge #6 with mixed:
```
{clk_a, #0: Clock}, {clk_b, #2: Clock}, {r_a, #1, #4: Sync(clk_a)}, 
{r_b, #3, #5: Sync(clk_b)}, {mixed, #6: Async}
```

**Candidate solution:**
```
Components:
  dom(clk_a) = Clock
  dom(clk_b) = Clock
  dom(r_a) = Sync(ComponentId(clk_a))
  dom(r_b) = Sync(ComponentId(clk_b))
  dom(mixed) = Async

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clk_a))
  dom(#2) = Clock
  dom(#3) = Sync(ComponentId(clk_b))
  dom(#4) = Sync(ComponentId(clk_a))
  dom(#5) = Sync(ComponentId(clk_b))
  dom(#6) = Async
```

**Verification - Check all constraints:**

Fixed constraints: ✓

Strict upper bound constraints:
- `dom(clk_a) < Async` → `Clock < Async` ✓
- `dom(clk_b) < Async` → `Clock < Async` ✓
- `dom(r_a) < Async` → `Sync(clk_a) < Async` ✓
- `dom(r_b) < Async` → `Sync(clk_b) < Async` ✓

Equality constraints: ✓

Upper bound constraints (from | operator):
- `dom(#4) ≤ dom(#6)` → `Sync(clk_a) ≤ Async` ✓ (Sync can upcast to Async in lattice)
- `dom(#5) ≤ dom(#6)` → `Sync(clk_b) ≤ Async` ✓ (Sync can upcast to Async in lattice)

**All constraints satisfied! Solution is valid.**

**Natural language**: The unified binary operator rule generates upper bound constraints `dom(operand) ≤ dom(result)`. Since the result ended up being Async (from the driver), both operands (Sync(clk_a) and Sync(clk_b)) can upcast to Async, satisfying `Sync(clk_a) ≤ Async` and `Sync(clk_b) ≤ Async`. The two different clock domains never needed to unify with each other - they just both upcast to the common Async result.

---

### Pass 3: Signature Extraction

**Identify domain parameters:**
- `clk_a` introduces `DomainVar(0)` (α)
- `clk_b` introduces `DomainVar(1)` (β)

**Express port domains:**
- `dom(clk_a) = Clock` → Clock
- `dom(clk_b) = Clock` → Clock
- `dom(mixed) = Async` → Async

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clk_a: DomainVar(0),  // α
        clk_b: DomainVar(1),  // β
    },
    port_domains: {
        clk_a: ???,   // Clock
        clk_b: ???,   // Clock
        mixed: ???,   // Async
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: AsyncMixing,
    component_domains: {
        clk_a: Clock,
        clk_b: Clock,
        r_a: Sync(ComponentId(clk_a)),
        r_b: Sync(ComponentId(clk_b)),
        mixed: Async,
    },
    expression_domains: {},
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---
