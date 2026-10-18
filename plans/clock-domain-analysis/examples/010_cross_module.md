# Example 10: Cross-Module Instantiation with Domain Substitution

```virdant
// Child module with polymorphic domain signature
mod Incrementer {
    incoming clock : Clock
    incoming data : Word[8]

    reg buffered : Word[8] on clock {
        it <= data
    }

    outgoing result : Word[8] {
        it := buffered + 1
    }
}

// Parent module that instantiates the child
mod System {
    incoming sys_clk : Clock
    incoming input : Word[8]

    // Child module instantiation
    mod inc of Incrementer {
        it.clock := sys_clk     // Clock connection binds domain variable
        it.data := input        // Data connection with substitution check
    }

    outgoing output : Word[8] {
        it := inc.result        // Use child's output
    }
}
```

**What's happening**: The `Incrementer` module has a polymorphic domain signature parameterized by its `clock` port. When instantiated in `System`, the clock connection `it.clock := sys_clk` binds the child's domain variable to the parent's concrete domain `Sync(sys_clk)`. The data port connections are then checked with this substitution applied. This demonstrates the core mechanism of domain polymorphism and instantiation.

---

## Part 1: Analyzing the Child Module (`Incrementer`)

We analyze the child module first, as it's a dependency. This produces a polymorphic `DomainSignature`.

### Pass 1: Constraint Generation (Child)

**Step 1 — Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `buffered` (reg, type Word[8])
- `result` (outgoing, type Word[8])

**Step 2 — Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- Type: Clock
- No expressions

Declaration: `incoming data : Word[8]`
- Component: `data`
- No expressions in declaration

Declaration: `reg buffered : Word[8] on clock`
- Component: `buffered`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Driver: `buffered <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Declaration: `outgoing result : Word[8]`
- Component: `result`
- No expressions in declaration

Driver: `result := buffered + 1`
- Expression `#2`: reference to `buffered` (left operand)
- Expression `#3`: literal `1` (right operand)
- Expression `#4`: `buffered + 1` (binary operation)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprReference to `buffered`
- `#3`: Literal `1`
- `#4`: BinaryOp `+`

**Step 3 — Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(buffered) = Sync(ComponentId(clock))`

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(buffered) < Async`
- `dom(result) < Async`

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#1) = dom(data)`
- `dom(#2) = dom(buffered)`

**Equality constraints from drivers:**
- Driver `buffered <= data`: `dom(buffered) = dom(#1)`
- Driver `result := ...`: `dom(result) = dom(#4)`

**Upper bound constraints from binary operator:**
- `dom(#2) ≤ dom(#4)` (left operand can upcast to result)
- `dom(#3) ≤ dom(#4)` (right operand can upcast to result)

**Complete constraint system for Incrementer:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(buffered) = Sync(ComponentId(clock))

Strict upper bound:
  C₃: dom(clock) < Async
  C₄: dom(data) < Async
  C₅: dom(buffered) < Async
  C₆: dom(result) < Async

Equality:
  C₇: dom(#0) = dom(clock)
  C₈: dom(#1) = dom(data)
  C₉: dom(#2) = dom(buffered)
  C₁₀: dom(buffered) = dom(#1)
  C₁₁: dom(result) = dom(#4)

Upper bound:
  C₁₂: dom(#2) ≤ dom(#4)
  C₁₃: dom(#3) ≤ dom(#4)
```

---

### Pass 2: Constraint Solving (Child)

**Initial state:**
```
{clock}, {data}, {buffered}, {result}, {#0}, {#1}, {#2}, {#3}, {#4}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {data}, {buffered}, {result}, {#0}, {#1}, {#2}, {#3}, {#4}
```

**Processing fixed constraint C₂: `dom(buffered) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {data}, {buffered: Sync(clock)}, {result}, {#0}, {#1}, {#2}, {#3}, {#4}
```

**Processing equality constraints C₇–C₁₁:**

C₇: `dom(#0) = dom(clock)` — merge #0 with clock:
```
{clock, #0: Clock}, {data}, {buffered: Sync(clock)}, {result}, {#1}, {#2}, {#3}, {#4}
```

C₈: `dom(#1) = dom(data)` — merge #1 with data:
```
{clock, #0: Clock}, {data, #1}, {buffered: Sync(clock)}, {result}, {#2}, {#3}, {#4}
```

C₉: `dom(#2) = dom(buffered)` — merge #2 with buffered:
```
{clock, #0: Clock}, {data, #1}, {buffered, #2: Sync(clock)}, {result}, {#3}, {#4}
```

C₁₀: `dom(buffered) = dom(#1)` — merge buffered's class with data's class:
- buffered's class: `{buffered, #2: Sync(clock)}` (has concrete domain)
- data's class: `{data, #1}` (no concrete domain)
- Result: `{data, #1, buffered, #2: Sync(clock)}`
```
{clock, #0: Clock}, {data, #1, buffered, #2: Sync(clock)}, {result}, {#3}, {#4}
```

C₁₁: `dom(result) = dom(#4)` — merge result with #4:
```
{clock, #0: Clock}, {data, #1, buffered, #2: Sync(clock)}, {result, #4}, {#3}
```

**Current partition after equality constraints:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, buffered, #2} → Sync(ComponentId(clock))
Class 3: {result, #4} → (no concrete domain yet)
Class 4: {#3} → (no concrete domain yet)
```

**Processing upper bound constraints C₁₂ and C₁₃:**

C₁₂: `dom(#2) ≤ dom(#4)`
- #2 is in Class 2 → `Sync(clock)`
- #4 is in Class 3 → need to assign a domain ≥ `Sync(clock)`

C₁₃: `dom(#3) ≤ dom(#4)`
- #3 is in Class 4 → unresolved (literal)
- #4 is in Class 3 → need to assign a domain ≥ the domain of #3

Since #3 is a literal with no constraints, it defaults to `Constant` (bottom of lattice).
For Class 3, we need: `Sync(clock) ≤ dom(#4)` and `Constant ≤ dom(#4)`

The least upper bound of `Sync(clock)` and `Constant` is `Sync(clock)` (since `Constant` unifies with any `Sync`).

**Assign Class 3 → `Sync(clock)`:**
```
Class 3: {result, #4} → Sync(ComponentId(clock))
```

**Assign Class 4 (literal, unconstrained) → `Constant`:**
```
Class 4: {#3} → Constant
```

**Final partition for Incrementer:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, buffered, #2} → Sync(ComponentId(clock))
Class 3: {result, #4} → Sync(ComponentId(clock))
Class 4: {#3} → Constant
```

**Solution (domain assignment for Incrementer):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(buffered) = Sync(ComponentId(clock))
  dom(result) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))
  dom(#3) = Constant
  dom(#4) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied ✓
- All strict upper bound constraints satisfied ✓
- All equality constraints satisfied ✓
- All upper bound constraints satisfied ✓

---

### Pass 3: Signature Extraction (Child)

Build the polymorphic domain signature for the `Incrementer` module.

**Step 1: Identify domain parameters**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Step 2: Express port domains in terms of parameters**

From the solution:
- `dom(clock) = Clock` → Concrete(Clock) (not a variable reference)
- `dom(data) = Sync(ComponentId(clock))` → Var(α) (references the domain parameter)
- `dom(result) = Sync(ComponentId(clock))` → Var(α) (references the domain parameter)

**Incrementer's Domain Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        data: Var(DomainVar(0)),      // α
        result: Var(DomainVar(0)),    // α
    }
}
```

**Interpretation**: The `Incrementer` module is polymorphic over one domain parameter α, introduced by the `clock` port. Both data ports (`data` and `result`) are on domain α. When instantiated, the clock connection will bind α to a concrete domain, and all data ports will be checked against that binding.

---

## Part 2: Analyzing the Parent Module (`System`)

Now we analyze the parent module, which instantiates `Incrementer`.

### Pass 1: Constraint Generation (Parent)

**Step 1 — Components from ComponentAnalysis:**
- `sys_clk` (incoming, type Clock)
- `input` (incoming, type Word[8])
- `inc` (module instance of `Incrementer`)
  - `inc.clock` (child port reference)
  - `inc.data` (child port reference)
  - `inc.result` (child port reference)
- `output` (outgoing, type Word[8])

**Note on module instance components**: When a child module is instantiated, each of its ports becomes a component in the parent's component analysis. These port components are used in connections.

**Step 2 — Walk declarations, identify expressions:**

Declaration: `incoming sys_clk : Clock`
- Component: `sys_clk`
- No expressions

Declaration: `incoming input : Word[8]`
- Component: `input`
- No expressions

Module instantiation: `mod inc of Incrementer { ... }`
- Connection `it.clock := sys_clk`:
  - Expression `#0`: reference to `sys_clk` (RHS)
  - This is a **clock connection** (special handling for domain binding)
- Connection `it.data := input`:
  - Expression `#1`: reference to `input` (RHS)
  - This is a **data connection** (subject to substitution checking)

Declaration: `outgoing output : Word[8]`
- Component: `output`
- No expressions in declaration

Driver: `output := inc.result`
- Expression `#2`: reference to `inc.result` (RHS)

**Expression inventory:**
- `#0`: ExprReference to `sys_clk` (for clock connection)
- `#1`: ExprReference to `input` (for data connection)
- `#2`: ExprReference to `inc.result` (for output driver)

**Step 3 — Generate constraints:**

**Fixed constraints from component types:**
- `dom(sys_clk) = Clock`

**Implicit strict upper bound constraints:**
- `dom(sys_clk) < Async`
- `dom(input) < Async`
- `dom(output) < Async`

**Equality constraints from component references:**
- `dom(#0) = dom(sys_clk)`
- `dom(#1) = dom(input)`
- `dom(#2) = dom(inc.result)`

**Constraints from module instantiation connections:**

For clock connection `it.clock := sys_clk`:
- Child's clock port has domain `Clock` (from child's signature)
- Connection constraint: `dom(inc.clock) = dom(#0)`

For data connection `it.data := input`:
- Child's data port has domain `Var(α)` (from child's signature)
- Connection constraint: `dom(inc.data) = dom(#1)`

For child's result port usage:
- `inc.result` is a component reference to the child's output port
- Its domain will be resolved through instantiation checking

**Equality constraints from driver:**
- Driver `output := inc.result`: `dom(output) = dom(#2)`

**Complete constraint system for System:**
```
Fixed:
  C₁: dom(sys_clk) = Clock

Strict upper bound:
  C₂: dom(sys_clk) < Async
  C₃: dom(input) < Async
  C₄: dom(output) < Async

Equality:
  C₅: dom(#0) = dom(sys_clk)
  C₆: dom(#1) = dom(input)
  C₇: dom(#2) = dom(inc.result)
  C₈: dom(inc.clock) = dom(#0)     [clock connection]
  C₉: dom(inc.data) = dom(#1)      [data connection]
  C₁₀: dom(output) = dom(#2)
```

**Note**: Constraints from child instantiation are generated based on the child's `DomainSignature`. The actual validation happens during instantiation checking (shown after solving).

---

### Pass 2: Constraint Solving (Parent)

**Initial state:**
```
{sys_clk}, {input}, {inc.clock}, {inc.data}, {inc.result}, {output}, {#0}, {#1}, {#2}
```

**Processing fixed constraint C₁: `dom(sys_clk) = Clock`**
```
{sys_clk: Clock}, {input}, {inc.clock}, {inc.data}, {inc.result}, {output}, {#0}, {#1}, {#2}
```

**Processing equality constraints C₅–C₁₀:**

C₅: `dom(#0) = dom(sys_clk)` — merge #0 with sys_clk:
```
{sys_clk, #0: Clock}, {input}, {inc.clock}, {inc.data}, {inc.result}, {output}, {#1}, {#2}
```

C₆: `dom(#1) = dom(input)` — merge #1 with input:
```
{sys_clk, #0: Clock}, {input, #1}, {inc.clock}, {inc.data}, {inc.result}, {output}, {#2}
```

C₇: `dom(#2) = dom(inc.result)` — merge #2 with inc.result:
```
{sys_clk, #0: Clock}, {input, #1}, {inc.clock}, {inc.data}, {inc.result, #2}, {output}
```

C₈: `dom(inc.clock) = dom(#0)` — merge inc.clock with #0's class:
- #0's class: `{sys_clk, #0: Clock}`
- inc.clock's class: `{inc.clock}`
- Result: `{sys_clk, #0, inc.clock: Clock}`
```
{sys_clk, #0, inc.clock: Clock}, {input, #1}, {inc.data}, {inc.result, #2}, {output}
```

C₉: `dom(inc.data) = dom(#1)` — merge inc.data with #1's class:
- #1's class: `{input, #1}`
- inc.data's class: `{inc.data}`
- Result: `{input, #1, inc.data}` (no concrete domain yet)
```
{sys_clk, #0, inc.clock: Clock}, {input, #1, inc.data}, {inc.result, #2}, {output}
```

C₁₀: `dom(output) = dom(#2)` — merge output with #2's class:
- #2's class: `{inc.result, #2}`
- output's class: `{output}`
- Result: `{inc.result, #2, output}` (no concrete domain yet)
```
{sys_clk, #0, inc.clock: Clock}, {input, #1, inc.data}, {inc.result, #2, output}
```

**Current partition after equality constraints:**
```
Class 1: {sys_clk, #0, inc.clock} → Clock
Class 2: {input, #1, inc.data} → (no concrete domain yet)
Class 3: {inc.result, #2, output} → (no concrete domain yet)
```

**No upper bound constraints in parent module** (all connections are equalities).

**However, we must apply instantiation constraints** from the child's signature.

---

### Instantiation Checking

Now we validate the instantiation of `Incrementer` in `System` using the child's domain signature.

**Child's signature (from Part 1):**
```
domain_params: { clock: α }
port_domains: {
    clock: Concrete(Clock),
    data: Var(α),
    result: Var(α)
}
```

**Step 1: Establish domain variable bindings from clock connections**

Connection: `it.clock := sys_clk`
- Child port: `clock` (introduces domain parameter α)
- Parent signal: `sys_clk`
- Parent signal's domain: `Clock`
- **However**: `clock` port has domain `Concrete(Clock)`, not `Var(α)`

Actually, looking at the signature more carefully, the clock port itself has domain `Clock`, not a variable. The domain **parameter** α is introduced **by** the clock port but represents the domain that the clock **defines**, not the domain of the clock signal itself.

Let me reconsider the signature extraction. When we extract the signature:
- Clock ports are Clock-typed and have domain `Clock`
- But they **introduce** a domain parameter representing the `Sync` domain they create
- Data ports synchronized to that clock have domain `Var(α)` where α refers to this parameter

So the binding happens like this:
- `clock` port connection establishes what concrete clock component is being used
- From this, we derive the `Sync(component)` domain that corresponds to α
- Data ports with domain `Var(α)` get checked against `Sync(component)`

**Corrected understanding:**

When `it.clock := sys_clk` is encountered:
- Child's `clock` parameter introduces `DomainVar(α)`
- Parent's `sys_clk` component has `ComponentId(sys_clk)`
- **Binding**: α → `Sync(ComponentId(sys_clk))`

**Step 2: Check data port connections with substitution**

Connection: `it.data := input`
- Child port `data` has domain `Var(α)` (from signature)
- After substitution: α → `Sync(sys_clk)`, so `data` expects `Sync(sys_clk)`
- Parent signal `input` is in Class 2: `{input, #1, inc.data}`
- From the connection constraint C₉, `dom(inc.data) = dom(input)`
- This forces `dom(input) = Sync(sys_clk)`

Connection: `inc.result` usage in `output := inc.result`
- Child port `result` has domain `Var(α)` (from signature)
- After substitution: α → `Sync(sys_clk)`, so `result` has domain `Sync(sys_clk)`
- Parent's `inc.result` is in Class 3: `{inc.result, #2, output}`
- From the connection, `dom(inc.result) = Sync(sys_clk)`
- This forces `dom(output) = Sync(sys_clk)`

**Apply instantiation-derived assignments:**

After instantiation checking, we know:
- α binds to `Sync(ComponentId(sys_clk))`
- All child ports with domain `Var(α)` get substituted to `Sync(sys_clk)`
- Parent signals connected to those ports must unify with `Sync(sys_clk)`

**Final partition for System (with instantiation constraints applied):**
```
Class 1: {sys_clk, #0, inc.clock} → Clock
Class 2: {input, #1, inc.data} → Sync(ComponentId(sys_clk))
Class 3: {inc.result, #2, output} → Sync(ComponentId(sys_clk))
```

**Solution (domain assignment for System):**
```
Components:
  dom(sys_clk) = Clock
  dom(input) = Sync(ComponentId(sys_clk))
  dom(inc.clock) = Clock
  dom(inc.data) = Sync(ComponentId(sys_clk))
  dom(inc.result) = Sync(ComponentId(sys_clk))
  dom(output) = Sync(ComponentId(sys_clk))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(sys_clk))
  dom(#2) = Sync(ComponentId(sys_clk))
```

**Verification:**
- All fixed constraints satisfied ✓
- All strict upper bound constraints satisfied ✓
- All equality constraints satisfied ✓
- Instantiation constraints from child signature satisfied ✓

---

### Pass 3: Signature Extraction (Parent)

**Domain parameters:**
- `sys_clk` is Clock-typed → introduces `DomainVar(0)` (call it β)

**Port domains:**
- `dom(sys_clk) = Clock` → Concrete(Clock)
- `dom(input) = Sync(ComponentId(sys_clk))` → Var(β)
- `dom(output) = Sync(ComponentId(sys_clk))` → Var(β)

**System's Domain Signature:**
```rust
DomainSignature {
    domain_params: {
        sys_clk: DomainVar(0)  // β
    },
    port_domains: {
        sys_clk: Concrete(Clock),
        input: Var(DomainVar(0)),     // β
        output: Var(DomainVar(0)),    // β
    }
}
```

The `System` module is also polymorphic, mirroring the structure of its child.

---

## Summary of Instantiation

The instantiation of `Incrementer` in `System` demonstrates:

1. **Clock connection establishes binding**: `it.clock := sys_clk` binds the child's domain parameter α to `Sync(ComponentId(sys_clk))`

2. **Data port domains use substitution**: Child ports with domain `Var(α)` are checked against parent signals after substituting α → `Sync(sys_clk)`

3. **Signature validation ensures compatibility**: The child's polymorphic signature is validated against concrete parent domains

4. **Propagation through module boundary**: The domain `Sync(sys_clk)` flows from the clock connection through the child's signature to all connected data ports

5. **Module composition preserves domain safety**: The parent module's analysis incorporates the child's domain constraints, ensuring end-to-end domain correctness

---

## Notes

### Does this work naturally with the algorithm?

**Yes, with some clarifications needed**. The constraint generation and solving algorithm handles the child module naturally. For the parent module, the key insight is that **module instantiation connections generate equality constraints**, and the child's domain signature determines what those equality constraints must satisfy.

The instantiation checking step (from ALGORITHM.md) fits naturally after constraint solving: once we know the parent's component domains, we can validate against the child's signature with substitution.

### Edge cases and design issues discovered

1. **Clock port domain vs. domain parameter**: The algorithm must distinguish between:
   - The domain of the clock signal itself (`Clock`)
   - The domain parameter representing what that clock synchronizes (`Sync(clock_component)`)

   In the signature, clock ports have domain `Concrete(Clock)`, but they **introduce** a domain variable representing their `Sync` domain.

2. **ComponentId scope**: `ComponentId(clock)` in the child refers to the child's local component. When bound at instantiation, it's replaced by the parent's `ComponentId(sys_clk)`. The algorithm must handle this mapping.

3. **Child port components in parent**: When a child is instantiated, its ports become components in the parent's component analysis (e.g., `inc.clock`, `inc.data`, `inc.result`). These components participate in the parent's constraint system and must unify with their connected parent signals.

4. **Instantiation constraints are bidirectional**: The connection `it.data := input` creates both:
   - An equality constraint in the parent's system: `dom(inc.data) = dom(input)`
   - A signature validation requirement: parent's domain must satisfy child's expected domain (after substitution)

### Clarifications needed for PLANv2.md or ALGORITHM.md

1. **ComponentId mapping at instantiation**: ALGORITHM.md should explicitly describe how child `ComponentId`s are mapped or compared during instantiation checking. The current `substitute()` function signature suggests substitution returns `Option[Domain]`, but we need clear semantics for when child `Sync(ComponentId_child)` is compared with parent `Sync(ComponentId_parent)`.

2. **Child port components**: PLANv2.md should clarify that instantiated child ports become first-class components in the parent's component analysis, with their own domain variables that participate in constraint solving.

3. **Signature domain representation**: The `???` placeholders in ALGORITHM.md's `DomainSignature.port_domains` should be replaced with a concrete type like:
   ```rust
   enum SignatureDomain {
       Concrete(Domain),
       Var(DomainVar),
       Free,
   }
   ```

4. **Step ordering**: ALGORITHM.md shows instantiation checking as a separate section, but it's unclear when this happens relative to constraint generation and solving. The algorithm should specify:
   - Parent's constraint generation includes child port components and connection equalities
   - Parent's constraint solving proceeds normally
   - After solving, instantiation validation checks parent domains against child signature
   - If validation fails, it's a domain error (even if parent's constraints were satisfied)

5. **Multiple instantiations of same child**: If a parent instantiates the same child module multiple times with different clocks, each instantiation binds the child's domain parameters independently. The algorithm should clarify that each instantiation is checked separately with its own bindings.

### Additional consideration: Unconstrained ports

If the child had a port that was unconstrained (not tied to any clock, marked `async`, or driven by a `Constant`), that port's domain in the signature would be `Free()`. At instantiation, a `Free()` port can unify with any parent domain — it imposes no constraints. This flexibility is important for maximally polymorphic child modules.

In this example, all child ports were constrained to the same domain (α), demonstrating a fully clock-synchronized module. Future examples could show partially constrained children with multiple domain parameters or free variables.
