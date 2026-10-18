# Example 13: Unconstrained Ports Propagating to Parent

```virdant
// Child module with unconstrained ports
mod PassThrough {
    incoming inp : Bit
    outgoing out : Bit {
        it := inp
    }
}

// Parent module that instantiates the child
mod Parent {
    incoming clock : Clock
    incoming data : Bit
    
    reg r : Bit on clock {
        it <= data
    }
    
    // Instantiate PassThrough, connecting clocked signal
    mod pass of PassThrough {
        it.inp := r
    }
    
    outgoing result : Bit {
        it := pass.out
    }
}
```

**What's happening**: The `PassThrough` module has ports (`inp` and `out`) that have no domain constraints from the module body — they're not tied to any clock, not marked async, and only connected through pure combinational logic (equality). When instantiated in `Parent`, the parent's concrete domain `Sync(clock)` propagates through the child's unconstrained signature, binding the free domain variables. This demonstrates how unconstrained child ports naturally inherit domains from their instantiation context.

---

## Part 1: Analyzing the Child Module (`PassThrough`)

We analyze the child module first, producing a polymorphic `DomainSignature` with free variables.

### Pass 1: Constraint Generation (Child)

**Step 1 — Components from ComponentAnalysis:**
- `inp` (incoming, type Bit)
- `out` (outgoing, type Bit)

**Step 2 — Walk declarations, identify expressions:**

Declaration: `incoming inp : Bit`
- Component: `inp`
- No expressions in declaration

Declaration: `outgoing out : Bit`
- Component: `out`
- No expressions in declaration

Driver: `out := inp`
- Expression `#0`: reference to `inp` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `inp`

**Step 3 — Generate constraints:**

**Fixed constraints from component types:**
- None (no Clock/Reset types, no registers, no async modifiers)

**Implicit strict upper bound constraints (all non-async components):**
- `dom(inp) < Async`
- `dom(out) < Async`

**Equality constraints from component references:**
- `dom(#0) = dom(inp)`

**Equality constraints from drivers:**
- Driver `out := inp`: `dom(out) = dom(#0)`

**Complete constraint system for PassThrough:**
```
Fixed:
  (none)

Strict upper bound:
  C₁: dom(inp) < Async
  C₂: dom(out) < Async

Equality:
  C₃: dom(#0) = dom(inp)
  C₄: dom(out) = dom(#0)
```

**Key observation**: No fixed constraints! Both ports are completely unconstrained except for the implicit "not async" requirement and the equality relationship between them.

---

### Pass 2: Constraint Solving (Child)

**Initial state:**
```
{inp}, {out}, {#0}
```

All equivalence classes start with no concrete domain assigned.

**Processing equality constraint C₃: `dom(#0) = dom(inp)`**

Merge #0 with inp's equivalence class:
```
{inp, #0}, {out}
```

**Processing equality constraint C₄: `dom(out) = dom(#0)`**

Merge out with #0's class (which now includes inp):
```
{inp, #0, out}
```

**Final partition after equality constraints:**
```
Class 1: {inp, #0, out} → (no concrete domain assigned)
```

**No fixed constraints to assign concrete domains**.

**No upper bound constraints to process**.

All three variables are in the same equivalence class, but **no concrete domain has been assigned**. This is not an error — it means the domain is **unconstrained** (a free variable).

**Final state for PassThrough:**
```
Class 1: {inp, #0, out} → None (free variable)
```

**Solution (domain assignment for PassThrough):**
```
Components:
  dom(inp) = <free>
  dom(out) = <free>

Expressions:
  dom(#0) = <free>

Constraint: dom(inp) = dom(out) (they must be equal)
```

**Verification of strict upper bounds:**
- `dom(inp) < Async` → `<free> < Async` ✓ (free variables satisfy strict upper bounds)
- `dom(out) < Async` → `<free> < Async` ✓

The strict upper bound constraints are satisfied because a free variable can be instantiated to any domain except `Async` (which must be explicit).

---

### Pass 3: Signature Extraction (Child)

Build the polymorphic domain signature for the `PassThrough` module.

**Step 1: Identify domain parameters**

Clock ports introduce domain parameters:
- No Clock-typed ports → no domain parameters introduced

**Step 2: Express port domains in terms of parameters**

From the solution:
- `dom(inp) = <free>` → `Free()` (unconstrained)
- `dom(out) = <free>` → `Free()` (unconstrained)

**However**, we know from the constraint system that `dom(inp) = dom(out)`. They're in the same equivalence class. So they must have the **same** free variable.

Actually, let me reconsider the signature representation. Since both ports are in the same equivalence class and unconstrained, we should introduce a fresh domain variable for that equivalence class.

**Corrected Signature Extraction:**

Since the equivalence class `{inp, out}` has no concrete domain, we introduce a fresh domain variable for it:
- Equivalence class → `DomainVar(0)` (call it α)

**PassThrough's Domain Signature:**
```rust
DomainSignature {
    domain_params: {},  // No clock ports, but we have a free variable
    port_domains: {
        inp: Free(DomainVar(0)),   // α (free variable, will be bound at instantiation)
        out: Free(DomainVar(0)),   // α (same free variable)
    }
}
```

Wait, I need to check the signature representation from ALGORITHM.md again. Looking back:

```rust
SignatureDomain = Concrete(Domain) | Var(DomainVar) | Free()
```

And from the algorithm:
```python
if assigned_domain is None:
    # Unconstrained port → free domain variable
    port_domains[port.name] = SignatureDomain.Free()
```

So it looks like `Free()` doesn't carry a variable identifier. But then how do we express that two ports must have the same domain?

Looking at example 010 more carefully, I see that domain variables come from clock ports. But in our case, we have no clock ports, just two data ports that must have the same domain.

I think the correct interpretation is:
- When there are no domain parameters (no clock ports), free variables in the signature are truly independent
- But in our case, both ports are in the same equivalence class, so they're **not** independent
- We need to express that they're the same

Actually, re-reading Pass 3 in ALGORITHM.md more carefully:

```python
# Step 1: Identify domain parameters from clock ports
for port in module.ports:
    if port.type == Clock:
        var = DomainVar(domain_var_counter)
        domain_var_counter += 1
        domain_params[port.name] = var
        clock_to_var[port.id] = var
```

Domain parameters are **only** introduced by clock ports. So in our `PassThrough` module with no clock ports, `domain_params` is empty.

But then for signature domains:
```python
if assigned_domain is None:
    # Unconstrained port → free domain variable
    port_domains[port.name] = SignatureDomain.Free()
```

Both ports get `Free()`. But then we lose the constraint that they must be equal!

I think there might be a gap in ALGORITHM.md here. For unconstrained ports that are in the same equivalence class but not tied to a clock, we need a way to express their relationship.

Let me think about what happens at instantiation time. If both ports have `Free()` in the signature:
- When `inp` is connected to a parent signal with domain `Sync(clock)`, we bind `inp` → `Sync(clock)`
- When `out` is connected to a parent signal, what domain does it get?
- If the signature just says `Free()` for both, without recording they're in the same class, we've lost the constraint!

I think the answer is that the signature extraction needs to handle this case. Looking at ALGORITHM.md again:

Actually, I think the issue is that `Free()` is meant to represent "any domain is acceptable" rather than "unconstrained variable". At instantiation time, the connection to a parent signal determines the domain.

Let me reconsider: In a module with no clocks, if two ports are in the same equivalence class, they must have the same domain when instantiated. The signature needs to express this.

One approach: Introduce domain variables for unconstrained equivalence classes, not just clock ports.

**Revised Signature Extraction for PassThrough:**

Since both ports are in the same equivalence class `{inp, out}` with no concrete domain, we introduce a domain variable for this class:
- Fresh variable: `DomainVar(0)` (α)

Both ports reference this variable:

```rust
DomainSignature {
    domain_params: {
        // No clock ports, but we have an implicit parameter for the unconstrained class
    },
    port_domains: {
        inp: Var(DomainVar(0)),    // α
        out: Var(DomainVar(0)),    // α
    }
}
```

But wait, if we use `Var(DomainVar(0))`, what binds this variable? In example 010, domain variables are bound by clock connections. Here, we have no clock.

I think the answer is: **the first data connection establishes the binding**. When `inp` is connected to a parent signal, that establishes the binding for `DomainVar(0)`, and then `out` must match that binding.

Alternatively, maybe the signature should use a different representation for "these ports must have the same domain, but that domain is free". Let me use `Var(DomainVar(0))` for both and document that the binding is established by data connections.

Actually, I think I'm overcomplicating this. Let me look at the constraint system again. At instantiation time:
- Parent generates constraint: `dom(pass.inp) = dom(r)` from connection `it.inp := r`
- Parent generates constraint: `dom(result) = dom(pass.out)` from connection `it := pass.out`
- From child's solving, we know: `dom(pass.inp) = dom(pass.out)`
- These constraints together force: `dom(r) = dom(pass.inp) = dom(pass.out) = dom(result)`

So the child's internal constraint (both ports in same equivalence class) carries over to the parent's constraint system!

The signature doesn't need to explicitly encode the equality — it's handled by the parent's constraint generation which includes child port components.

Given this, I think `Free()` is actually correct for the signature, and the equality constraint is maintained through the parent's constraint system.

Let me simplify and use `Free()`:

**PassThrough's Domain Signature (simplified):**
```rust
DomainSignature {
    domain_params: {},
    port_domains: {
        inp: Free(),
        out: Free(),
    }
}
```

At instantiation time:
- Both `pass.inp` and `pass.out` become components in the parent
- Parent's constraint system maintains `dom(pass.inp) = dom(pass.out)` (from child's equivalence class)
- Parent's connections add: `dom(pass.inp) = dom(r)` and `dom(result) = dom(pass.out)`
- Solving these together gives: all four have the same domain

Yes, I think this is correct. The child's equivalence classes are preserved when child ports become parent components.

---

## Part 2: Analyzing the Parent Module (`Parent`)

Now we analyze the parent module, which instantiates `PassThrough`.

### Pass 1: Constraint Generation (Parent)

**Step 1 — Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Bit)
- `r` (reg, type Bit)
- `pass` (module instance of `PassThrough`)
  - `pass.inp` (child port reference)
  - `pass.out` (child port reference)
- `result` (outgoing, type Bit)

**Step 2 — Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- Type: Clock
- No expressions

Declaration: `incoming data : Bit`
- Component: `data`
- No expressions

Declaration: `reg r : Bit on clock`
- Component: `r`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Driver: `r <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Module instantiation: `mod pass of PassThrough { ... }`
- Connection `it.inp := r`:
  - Expression `#2`: reference to `r` (RHS)
  - This is a data connection to child port

Declaration: `outgoing result : Bit`
- Component: `result`
- No expressions in declaration

Driver: `result := pass.out`
- Expression `#3`: reference to `pass.out` (RHS)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprReference to `r` (for child connection)
- `#3`: ExprReference to `pass.out`

**Step 3 — Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(r) = Sync(ComponentId(clock))`

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(r) < Async`
- `dom(result) < Async`
- `dom(pass.inp) < Async` (child port component)
- `dom(pass.out) < Async` (child port component)

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#1) = dom(data)`
- `dom(#2) = dom(r)`
- `dom(#3) = dom(pass.out)`

**Equality constraints from drivers:**
- Driver `r <= data`: `dom(r) = dom(#1)`
- Driver `result := pass.out`: `dom(result) = dom(#3)`

**Constraints from module instantiation:**
- Connection `it.inp := r`: `dom(pass.inp) = dom(#2)`

**Constraints from child's equivalence classes:**
- From child's analysis, we know `dom(pass.inp) = dom(pass.out)` (they were in the same equivalence class)
- This constraint is added: `dom(pass.inp) = dom(pass.out)`

**Complete constraint system for Parent:**
```
Fixed:
  F₁: dom(clock) = Clock
  F₂: dom(r) = Sync(ComponentId(clock))

Strict upper bound:
  S₁: dom(clock) < Async
  S₂: dom(data) < Async
  S₃: dom(r) < Async
  S₄: dom(result) < Async
  S₅: dom(pass.inp) < Async
  S₆: dom(pass.out) < Async

Equality:
  E₁: dom(#0) = dom(clock)
  E₂: dom(#1) = dom(data)
  E₃: dom(#2) = dom(r)
  E₄: dom(#3) = dom(pass.out)
  E₅: dom(r) = dom(#1)
  E₆: dom(result) = dom(#3)
  E₇: dom(pass.inp) = dom(#2)        [from connection]
  E₈: dom(pass.inp) = dom(pass.out)  [from child's equivalence class]
```

---

### Pass 2: Constraint Solving (Parent)

**Initial state:**
```
{clock}, {data}, {r}, {pass.inp}, {pass.out}, {result}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint F₁: `dom(clock) = Clock`**
```
{clock: Clock}, {data}, {r}, {pass.inp}, {pass.out}, {result}, {#0}, {#1}, {#2}, {#3}
```

**Processing fixed constraint F₂: `dom(r) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {data}, {r: Sync(clock)}, {pass.inp}, {pass.out}, {result}, {#0}, {#1}, {#2}, {#3}
```

**Processing equality constraints E₁–E₈:**

E₁: `dom(#0) = dom(clock)` — merge #0 with clock:
```
{clock, #0: Clock}, {data}, {r: Sync(clock)}, {pass.inp}, {pass.out}, {result}, {#1}, {#2}, {#3}
```

E₂: `dom(#1) = dom(data)` — merge #1 with data:
```
{clock, #0: Clock}, {data, #1}, {r: Sync(clock)}, {pass.inp}, {pass.out}, {result}, {#2}, {#3}
```

E₃: `dom(#2) = dom(r)` — merge #2 with r:
```
{clock, #0: Clock}, {data, #1}, {r, #2: Sync(clock)}, {pass.inp}, {pass.out}, {result}, {#3}
```

E₄: `dom(#3) = dom(pass.out)` — merge #3 with pass.out:
```
{clock, #0: Clock}, {data, #1}, {r, #2: Sync(clock)}, {pass.inp}, {pass.out, #3}, {result}
```

E₅: `dom(r) = dom(#1)` — merge r's class with data's class:
- r's class: `{r, #2: Sync(clock)}` (has concrete domain)
- data's class: `{data, #1}` (no concrete domain)
- Result: `{data, #1, r, #2: Sync(clock)}`
```
{clock, #0: Clock}, {data, #1, r, #2: Sync(clock)}, {pass.inp}, {pass.out, #3}, {result}
```

E₆: `dom(result) = dom(#3)` — merge result with #3's class:
```
{clock, #0: Clock}, {data, #1, r, #2: Sync(clock)}, {pass.inp}, {pass.out, #3, result}
```

E₇: `dom(pass.inp) = dom(#2)` — merge pass.inp with #2's class:
- #2's class: `{data, #1, r, #2: Sync(clock)}`
- pass.inp's class: `{pass.inp}`
- Result: `{data, #1, r, #2, pass.inp: Sync(clock)}`
```
{clock, #0: Clock}, {data, #1, r, #2, pass.inp: Sync(clock)}, {pass.out, #3, result}
```

E₈: `dom(pass.inp) = dom(pass.out)` — merge pass.inp's class with pass.out's class:
- pass.inp's class: `{data, #1, r, #2, pass.inp: Sync(clock)}`
- pass.out's class: `{pass.out, #3, result}`
- Result: All merge into one class with domain `Sync(clock)`
```
{clock, #0: Clock}, {data, #1, r, #2, pass.inp, pass.out, #3, result: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0} → Clock
Class 2: {data, #1, r, #2, pass.inp, pass.out, #3, result} → Sync(ComponentId(clock))
```

**Solution (domain assignment for Parent):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(r) = Sync(ComponentId(clock))
  dom(pass.inp) = Sync(ComponentId(clock))
  dom(pass.out) = Sync(ComponentId(clock))
  dom(result) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))
  dom(#2) = Sync(ComponentId(clock))
  dom(#3) = Sync(ComponentId(clock))
```

**Verification:**
- All fixed constraints satisfied ✓
- All strict upper bound constraints satisfied ✓
- All equality constraints satisfied ✓

**Key observation**: The child's unconstrained domain variable was bound to `Sync(ComponentId(clock))` through the connection `it.inp := r`. This domain then propagated through the child's internal constraint (`inp = out`) to the child's output port, and finally to the parent's `result` component. The free variable naturally adopted the concrete domain from the instantiation context!

---

### Pass 3: Signature Extraction (Parent)

**Domain parameters:**
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it β)

**Port domains:**
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(data) = Sync(ComponentId(clock))` → `Var(β)`
- `dom(result) = Sync(ComponentId(clock))` → `Var(β)`

**Parent's Domain Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // β
    },
    port_domains: {
        clock: Concrete(Clock),
        data: Var(DomainVar(0)),    // β
        result: Var(DomainVar(0)),  // β
    }
}
```

The `Parent` module is polymorphic over one domain parameter β, and all data ports are synchronous to that domain.

---

## Summary

This example demonstrates **unconstrained port propagation**:

1. **Child module has free variables**: The `PassThrough` module has no internal domain constraints (no clocks, no registers, no async). Both ports are in the same equivalence class but have no assigned domain.

2. **Child signature expresses freedom**: The child's signature has `Free()` for both ports, indicating they accept any domain (but must be the same domain due to the internal equality constraint).

3. **Parent connection binds free variables**: When the parent connects `pass.inp := r` where `r` has domain `Sync(clock)`, the child port `pass.inp` inherits that concrete domain.

4. **Internal constraints propagate binding**: The child's internal constraint `dom(inp) = dom(out)` (represented as an equality in the parent's constraint system) propagates the binding from `pass.inp` to `pass.out`.

5. **Output propagates to parent**: The parent's use of `pass.out` in `result := pass.out` propagates the domain to the parent's output port.

The unconstrained domain flows: `r (Sync(clock))` → `pass.inp (bound to Sync(clock))` → `pass.out (same as pass.inp)` → `result (Sync(clock))`.

---

## Notes

### Does this work naturally with the algorithm?

**Yes, remarkably well!** The algorithm handles unconstrained ports without any special cases:

1. **Constraint generation**: Child module generates only equality constraints, no fixed constraints. Ports remain unconstrained.

2. **Constraint solving**: Child's solving produces an equivalence class with no assigned domain. This is not an error — it's a free variable.

3. **Signature extraction**: Child signature marks ports as `Free()`, indicating they're unconstrained.

4. **Parent instantiation**: Parent's constraint system includes child port components. The connection `it.inp := r` generates an equality constraint in the parent, which binds the child port to the parent's concrete domain.

5. **Propagation**: The child's internal constraint (both ports in same equivalence class) is preserved in the parent's constraint system, ensuring consistent domain propagation.

The key insight is that **child port components participate in the parent's constraint system**, so child-internal constraints become parent-level constraints. This allows free variables to be bound at instantiation time without special handling.

### Edge cases and design issues discovered

1. **Free() vs. Var() in signatures**: The signature representation uses `Free()` for unconstrained ports. This is simpler than introducing domain variables for non-clock-parameterized constraints. However, it relies on child port components maintaining the child's internal equivalence constraints in the parent's system.

2. **Preserving child equivalence classes**: When a child is instantiated, the parent's constraint system must include constraints from the child's equivalence classes. The algorithm does this by treating child ports as regular components that participate in parent's unification.

3. **No domain parameters without clocks**: The `PassThrough` module has no domain parameters in its signature (empty `domain_params` map) because it has no Clock-typed ports. Domain parameters are only introduced by clocks. Unconstrained data ports just get `Free()`.

4. **Strict upper bound satisfaction for free variables**: Free variables satisfy strict upper bound constraints (`< Async`) because at instantiation, they can be bound to any domain except `Async` (which requires explicit annotation).

5. **Bidirectional propagation**: In this example, domain flows from parent to child (input connection) and back from child to parent (output usage). The constraint system naturally handles this bidirectional flow through equality constraints.

### Clarifications needed for PLANv2.md or ALGORITHM.md

1. **Child port components in parent**: ALGORITHM.md should explicitly state that when a child module is instantiated, each child port becomes a component in the parent's component set, participating in the parent's constraint system.

2. **Preserving child equivalence classes**: When child ports become parent components, the child's equivalence class relationships must be added as equality constraints in the parent's system. This ensures constraints like "inp and out have the same domain" are preserved across the module boundary.

3. **Free() semantics**: The `SignatureDomain::Free()` variant represents "this port is unconstrained and will adopt whatever domain it's connected to at instantiation." The documentation should clarify that `Free()` ports still must satisfy strict upper bound constraints (i.e., they can't become `Async` unless explicitly marked).

4. **Multiple free variables**: If a child has multiple disjoint unconstrained equivalence classes (e.g., two separate unconstrained ports), each would get `Free()` in the signature. They could be bound to different domains in the parent. The current algorithm handles this correctly because each becomes a separate component in the parent.

5. **Instantiation without clocks**: This example shows that modules without Clock-typed ports can still be polymorphic (via free variables). PLANv2.md focuses on clock-based polymorphism, but this unconstrained polymorphism is equally important and naturally supported.

### Design strength: Compositionality

This example showcases the **compositional nature** of the domain analysis:

- Child analyzed independently, producing a signature
- Parent analyzed with child signature as input
- No special-case logic for unconstrained ports
- Constraint system unifies child and parent constraints seamlessly

The algorithm treats module instantiation as a natural extension of constraint solving, where child signatures contribute constraints to the parent's system. This compositionality is a major design strength.
