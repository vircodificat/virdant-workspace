## Example 020: asclock() Type Conversion Domain Assignment

```virdant
mod AsClockExample {
    incoming data : Bit
    
    wire clock_signal : Clock {
        it := asclock(data)
    }
    
    reg buffer : Bit on clock_signal {
        it <= !it
    }
    
    outgoing out : Bit {
        it := buffer
    }
}
```

**What's happening**: This example explores the `asclock()` primitive, which converts a `Bit` to `Clock` type. The key question: what happens to the domain during type conversion? The input `data` could have any domain, but `clock_signal` is Clock-typed, which according to the algorithm should fix its domain to `Clock`.

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `data` (incoming, type Bit)
- `clock_signal` (wire, type Clock)
- `buffer` (reg, type Bit)
- `out` (outgoing, type Bit)

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming data : Bit`
- Component: `data`
- No expressions

Declaration: `wire clock_signal : Clock`
- Component: `clock_signal`
- No expressions in declaration itself
- (Driver will have expression)

Declaration: `reg buffer : Bit on clock_signal`
- Component: `buffer`
- Expression `#0`: reference to `clock_signal` (in the `on` clause)

Declaration: `outgoing out : Bit`
- Component: `out`
- No expressions in declaration

Driver: `clock_signal := asclock(data)`
- Expression `#1`: `asclock(data)` expression
  - This is a primitive function call
  - Expression `#2`: reference to `data` (argument to asclock)

Driver: `buffer <= !it`
- Expression `#3`: unary negation of `it` (self-reference)
- Expression `#4`: reference to `buffer` (the `it` inside the block)

Driver: `out := buffer`
- Expression `#5`: reference to `buffer`

**Expression inventory:**
- `#0`: ExprReference to `clock_signal`
- `#1`: ExprPrimitive `asclock(#2)`
- `#2`: ExprReference to `data`
- `#3`: ExprUnaryOp `!` of `#4`
- `#4`: ExprReference to `buffer` (self-reference)
- `#5`: ExprReference to `buffer`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock_signal) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock_signal))` (register with `on clock_signal` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock_signal` → `dom(#0) = dom(clock_signal)`
- Expression `#2` references `data` → `dom(#2) = dom(data)`
- Expression `#4` references `buffer` → `dom(#4) = dom(buffer)`
- Expression `#5` references `buffer` → `dom(#5) = dom(buffer)`

**Constraints from asclock() primitive:**

The `asclock()` primitive is a type conversion operation. The key question is: what domain constraints does it generate?

According to the algorithm, when we walk an expression:
- The subject `#2` (reference to `data`) has already constrained through `dom(#2) = dom(data)`
- The result `#1` (the asclock call itself) needs its domain determined

**Critical insight**: The result of `asclock()` is Clock-typed, and Clock-typed expressions should have domain `Clock`. This is similar to how a Clock-typed component gets `Fixed(component, Clock)`.

Therefore, we generate:
- `dom(#1) = Clock` (the asclock result is Clock domain)

Note: This is a **fixed constraint**, not an equality constraint with the input! The domain transformation is:
- Input: `data` has whatever domain it gets (to be inferred)
- Output: `asclock(data)` has domain `Clock` (fixed)

**Equality constraints from unary operator:**
- Result `#3` has same domain as operand `#4`: `dom(#3) = dom(#4)`

**Equality constraints from drivers:**

Driver: `clock_signal := asclock(data)`
- LHS: component `clock_signal`
- RHS: expression `#1`
- Constraint: `dom(clock_signal) = dom(#1)`

Driver: `buffer <= !it`
- LHS: component `buffer`
- RHS: expression `#3`
- Constraint: `dom(buffer) = dom(#3)`

Driver: `out := buffer`
- LHS: component `out`
- RHS: expression `#5`
- Constraint: `dom(out) = dom(#5)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(data) < Async`
- `dom(clock_signal) < Async`
- `dom(buffer) < Async`
- `dom(out) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock_signal) = Clock
  C₂: dom(buffer) = Sync(ComponentId(clock_signal))
  C₃: dom(#1) = Clock  [asclock result]

Strict upper bound (implicit):
  C₄: dom(data) < Async
  C₅: dom(clock_signal) < Async
  C₆: dom(buffer) < Async
  C₇: dom(out) < Async

Equality:
  C₈: dom(#0) = dom(clock_signal)
  C₉: dom(#2) = dom(data)
  C₁₀: dom(#3) = dom(#4)
  C₁₁: dom(#4) = dom(buffer)
  C₁₂: dom(#5) = dom(buffer)
  C₁₃: dom(clock_signal) = dom(#1)
  C₁₄: dom(buffer) = dom(#3)
  C₁₅: dom(out) = dom(#5)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{data}, {clock_signal}, {buffer}, {out}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing fixed constraint C₁: `dom(clock_signal) = Clock`**

Assign concrete domain `Clock` to clock_signal's equivalence class:
```
{data}, {clock_signal: Clock}, {buffer}, {out}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing fixed constraint C₂: `dom(buffer) = Sync(ComponentId(clock_signal))`**

Assign concrete domain `Sync(clock_signal)` to buffer's equivalence class:
```
{data}, {clock_signal: Clock}, {buffer: Sync(clock_signal)}, {out}, {#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing fixed constraint C₃: `dom(#1) = Clock`**

Assign concrete domain `Clock` to expression #1's equivalence class:
```
{data}, {clock_signal: Clock}, {buffer: Sync(clock_signal)}, {out}, {#0}, {#1: Clock}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality constraint C₈: `dom(#0) = dom(clock_signal)`**

Merge expression #0 with clock_signal's equivalence class:
```
{data}, {clock_signal, #0: Clock}, {buffer: Sync(clock_signal)}, {out}, {#1: Clock}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality constraint C₉: `dom(#2) = dom(data)`**

Merge expression #2 with data's equivalence class:
```
{data, #2}, {clock_signal, #0: Clock}, {buffer: Sync(clock_signal)}, {out}, {#1: Clock}, {#3}, {#4}, {#5}
```

**Processing equality constraint C₁₀: `dom(#3) = dom(#4)`**

Merge expressions #3 and #4:
```
{data, #2}, {clock_signal, #0: Clock}, {buffer: Sync(clock_signal)}, {out}, {#1: Clock}, {#3, #4}, {#5}
```

**Processing equality constraint C₁₁: `dom(#4) = dom(buffer)`**

Merge #4's class with buffer's class:
- #4's class: `{#3, #4}` (no concrete domain)
- buffer's class: `{buffer: Sync(clock_signal)}` (has concrete domain)
- Merged class gets domain `Sync(clock_signal)`

```
{data, #2}, {clock_signal, #0: Clock}, {buffer, #3, #4: Sync(clock_signal)}, {out}, {#1: Clock}, {#5}
```

**Processing equality constraint C₁₂: `dom(#5) = dom(buffer)`**

Merge #5 with buffer's class:
```
{data, #2}, {clock_signal, #0: Clock}, {buffer, #3, #4, #5: Sync(clock_signal)}, {out}, {#1: Clock}
```

**Processing equality constraint C₁₃: `dom(clock_signal) = dom(#1)`**

Merge clock_signal's class with #1's class:
- clock_signal's class: `{clock_signal, #0: Clock}`
- #1's class: `{#1: Clock}`
- Both have domain `Clock`, so they're compatible

```
{data, #2}, {clock_signal, #0, #1: Clock}, {buffer, #3, #4, #5: Sync(clock_signal)}, {out}
```

**Processing equality constraint C₁₄: `dom(buffer) = dom(#3)`**

Already satisfied - both in same class.

**Processing equality constraint C₁₅: `dom(out) = dom(#5)`**

Merge out with #5's class (buffer's class):
```
{data, #2}, {clock_signal, #0, #1: Clock}, {buffer, #3, #4, #5, out: Sync(clock_signal)}
```

**Final partition:**
```
Class 1: {data, #2} → [no domain assigned yet]
Class 2: {clock_signal, #0, #1} → Clock
Class 3: {buffer, #3, #4, #5, out} → Sync(ComponentId(clock_signal))
```

**Resolve unassigned classes:**

Class 1 (`{data, #2}`) has no fixed domain. Check for upper bound constraints:
- No upper bound constraints reference `data` or `#2`
- No other constraints force a domain

This class remains unresolved. Looking at strict upper bounds:
- C₄: `dom(data) < Async` means data can be `Constant`, `Sync(...)`, `Clock`, or `Reset`

Since `data` is an incoming port with no other constraints, it remains as a **free variable** that will be constrained at instantiation time.

**Solution (domain assignment):**
```
Components:
  dom(data) = [free variable - will be determined at instantiation]
  dom(clock_signal) = Clock
  dom(buffer) = Sync(ComponentId(clock_signal))
  dom(out) = Sync(ComponentId(clock_signal))

Expressions:
  dom(#0) = Clock
  dom(#1) = Clock [asclock result]
  dom(#2) = [free - same as data]
  dom(#3) = Sync(ComponentId(clock_signal))
  dom(#4) = Sync(ComponentId(clock_signal))
  dom(#5) = Sync(ComponentId(clock_signal))
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock_signal) = Clock` ✓
  - `dom(buffer) = Sync(clock_signal)` ✓
  - `dom(#1) = Clock` ✓
- All strict upper bound constraints satisfied: ✓
  - `dom(data) < Async` → unconstrained, but not `Async` ✓
  - `dom(clock_signal) < Async` → `Clock < Async` ✓
  - `dom(buffer) < Async` → `Sync(clock_signal) < Async` ✓
  - `dom(out) < Async` → `Sync(clock_signal) < Async` ✓
- All equality constraints satisfied: ✓
  - All variables in each class have the same domain ✓

**Key observation**: The `asclock()` conversion **breaks domain propagation** - the input's domain does NOT flow to the output. Instead, the output is fixed to domain `Clock` regardless of the input.

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock_signal` is Clock-typed but is NOT an incoming port (it's a wire)
- No incoming Clock ports exist
- Therefore: **no domain parameters** (empty set)

Wait, this is interesting! The module has a Clock-typed signal (`clock_signal`), but it's **internal**, not a port. So it doesn't introduce a domain parameter.

**Express port domains in terms of parameters:**

From the solution:
- `dom(data)` = unconstrained → `Free()`
- `dom(out)` = `Sync(ComponentId(clock_signal))` → but `clock_signal` is internal!

Here's the key insight: `out` is synchronized to an **internal clock**, not a port. So its signature domain is:
- `Concrete(Sync(internal_component))` - concrete, not polymorphic

However, looking at the lattice perspective: from outside the module, `out` behaves as if it's on "some clock domain" that's defined internally. The signature cannot reference internal ComponentIds.

**Actually**: Since `clock_signal` is internal and not a port, the module's signature should show:
- `data`: `Free()` - unconstrained input
- `out`: `Concrete(Sync(...))` where the clock is internal

But wait - in the signature extraction algorithm from ALGORITHM.md:

```python
elif isinstance(assigned_domain, Sync):
    # Map to domain parameter
    clock_component = assigned_domain.clock
    if clock_component in clock_to_var:
        var = clock_to_var[clock_component]
        port_domains[port.name] = SignatureDomain.Var(var)
    else:
        # Clock is internal, not a port → concrete Sync
        port_domains[port.name] = SignatureDomain.Concrete(assigned_domain)
```

So the signature is:

**Signature:**
```rust
DomainSignature {
    domain_params: {
        // Empty - no Clock-typed incoming ports
    },
    port_domains: {
        data: Free(),  // unconstrained
        out: Concrete(Sync(ComponentId(clock_signal)))  // internal clock
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: AsClockExample,
    component_domains: {
        data: None,  // free variable
        clock_signal: Clock,
        buffer: Sync(ComponentId(clock_signal)),
        out: Sync(ComponentId(clock_signal)),
    },
    signature: DomainSignature {
        domain_params: {},
        port_domains: {
            data: Free(),
            out: Concrete(Sync(ComponentId(clock_signal)))
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

# Notes

## Does this work naturally with the algorithm?

**Mostly yes, with one clarification needed:**

The algorithm handles `asclock()` naturally **if** we treat the primitive as generating a fixed constraint:
- `dom(asclock_result) = Clock`

This is analogous to how Clock-typed components get `Fixed(component, Clock)` constraints. The type conversion fixes the domain to match the result type.

**However**, ALGORITHM.md doesn't explicitly cover primitive function calls. The `walk_expression()` function would need a case for primitives:

```python
elif expr.kind == Primitive:
    # Process arguments
    arg_exprs = []
    for arg in expr.args:
        expr_id, arg_expr = walk_expression(arg, &mut variables, 
                                           &mut equalities, &mut upper_bounds, expr_id)
        arg_exprs.append(arg_expr)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Type-based domain constraints
    if expr.primitive == AsClock:
        # asclock() result is always Clock domain
        fixed.append(Fixed(result_expr, Clock))
    elif expr.primitive in [Width, Zext, Sext, ...]:
        # Other primitives: result depends on arguments
        # (specific rules per primitive)
        ...
    
    return (expr_id, result_expr)
```

## Surprises and edge cases

1. **Domain transformation, not propagation**: Unlike most expressions where domains flow through (equality constraints), `asclock()` performs a domain transformation. The input domain is **irrelevant** to the output domain.

2. **Input can be anything**: The `data` port has no constraints from the `asclock()` call. It could be:
   - `Sync(some_other_clock)` - converting a clocked signal to a Clock
   - `Async` - converting an async signal to a Clock (though this would need explicit `async` annotation on `data`)
   - `Constant` - converting a constant to a Clock (e.g., `asclock(1'b0)` for a stopped clock)
   - Any domain at all!

3. **Internal clock creates concrete signature**: Since `clock_signal` is internal, the module's signature shows `out` as having a concrete `Sync` domain, not a polymorphic one. The module is "self-clocking" from an external perspective.

4. **Type conversion fixes domain**: This is the first example where **type** directly determines **domain**. Clock-typed expressions get domain `Clock`, regardless of their construction.

## Does the constraint system handle this without special cases?

**Almost, but needs primitive-specific handling:**

The constraint system handles the **propagation** naturally once we generate the right constraints. But **constraint generation** needs to know:
- `asclock()` produces a Clock-typed expression
- Clock-typed expressions get fixed domain `Clock`

This requires adding primitive-function-specific logic to `walk_expression()` in Pass 1. The logic is straightforward:
- Check the primitive type
- For `asclock`, add `Fixed(result_expr, Clock)`
- For other primitives, add appropriate constraints (most would just use equality with their arguments)

## Clarifications needed for ALGORITHM.md

1. **Add primitive function case to walk_expression()**:
   - Document how to handle `ExprPrimitive` nodes
   - Show examples: `asclock()`, `width()`, `zext()`, etc.
   - Specify domain constraints for each primitive

2. **Type-to-domain mapping principle**:
   - Clarify that Clock-typed expressions **always** have domain `Clock`
   - This applies to:
     - Clock-typed components (`incoming clock : Clock`)
     - Clock-typed expression results (`asclock(bit)`)
     - Any expression with inferred type `Clock`

3. **Free variables in signatures**:
   - Document that unconstrained ports become `Free()` in signatures
   - Explain when a port is unconstrained (no fixed domain, no equalities forcing a domain)

4. **Internal clocks in signatures**:
   - Clarify that `Concrete(Sync(ComponentId))` in a signature means "internal clock"
   - This is distinct from `Var(DomainVar)` which references a port clock
   - At instantiation, internal clocks don't participate in substitution

## Implications for other primitives

Other type-conversion primitives may have similar domain implications:

- **`cast()`**: Converting between enum and Word[N]
  - Both are data types, so domain should flow through: `dom(result) = dom(argument)`
  - No domain transformation

- **`width()`**: Query operation, returns compile-time constant
  - Result has domain `Constant` - fixed, not dependent on argument
  - Similar to `asclock()` in that it breaks propagation

- **`zext()`, `sext()`**: Width extension
  - Domain flows through: `dom(result) = dom(argument)`
  - These are data transformations, not type conversions

- **`word()`, `bits()`**: Array/struct construction
  - Domain equality with all operands (same as struct literal)

The key distinction: **Type-changing primitives that change the semantic interpretation** (like `asclock()` turning data into a clock) may involve domain transformation. **Bit-manipulation primitives** preserve domain.

## Is this a gap in the algorithm?

**Minor gap - needs documentation, not algorithmic change:**

The three-pass structure handles this correctly:
1. Pass 1 generates `Fixed(asclock_result, Clock)` constraint ✓
2. Pass 2 solves constraints, enforcing the fixed domain ✓
3. Pass 3 extracts signature, showing `Free()` input and `Concrete(Sync)` output ✓

What's missing is explicit documentation in ALGORITHM.md for:
- How to walk primitive function calls in Pass 1
- Which primitives generate fixed constraints vs. equality constraints
- The general principle: "type determines domain for Clock/Reset types"

**Recommendation**: Add a section to ALGORITHM.md:

### Special Expression Types

#### Primitive Functions

Primitive functions like `asclock()`, `width()`, etc. have specific domain constraint rules:

```python
elif expr.kind == Primitive:
    # Process arguments first
    arg_exprs = []
    for arg in expr.args:
        expr_id, arg_expr = walk_expression(arg, ...)
        arg_exprs.append(arg_expr)
    
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Constraint based on primitive type
    if expr.primitive == AsClock:
        # Type conversion to Clock → fixed domain
        fixed.append(Fixed(result_expr, Clock))
    else:
        # Most primitives: domain flows from arguments
        for arg_expr in arg_exprs:
            equalities.append(Equality(result_expr, arg_expr))
    
    return (expr_id, result_expr)
```

**Principle**: Clock-typed and Reset-typed expressions have fixed domains (`Clock` and `Reset` respectively), regardless of how they're constructed. Data-typed expressions propagate domains from their operands.
