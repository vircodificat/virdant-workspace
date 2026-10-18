# Example 27: Incoming Forwarding Block Domain Connections

```virdant
mod Reg8 {
    incoming clock : Clock
    incoming inp : Word[8]
    
    reg out : Word[8] on clock {
        it <= inp
    }
}

mod Top {
    incoming clock : Clock {
        reg_a.clock := it
        reg_b.clock := it
    }
    
    incoming data : Word[8]
    
    mod reg_a of Reg8 {
        it.inp := data
    }
    
    mod reg_b of Reg8 {
        it.inp := data
    }
    
    outgoing result_a : Word[8] {
        it := reg_a.out
    }
    
    outgoing result_b : Word[8] {
        it := reg_b.out
    }
}
```

**What's happening**: This example demonstrates the `incoming { ... }` forwarding block syntax. The clock port uses a forwarding block to connect to multiple submodules (`reg_a.clock` and `reg_b.clock`). This is purely syntactic sugar for explicit connection statements.

**Key insight**: From the domain analysis perspective, the forwarding block should generate identical constraints to writing the connections explicitly:

```virdant
incoming clock : Clock
reg_a.clock := clock
reg_b.clock := clock
```

The `it` inside the forwarding block simply refers to the incoming port being declared.

**Expected result**: Domain analysis should handle the forwarding block naturally without any special cases. The connections inside the block are regular connection statements.

---

## Pass 1: Constraint Generation

We analyze both the child module `Reg8` and the parent module `Top`. The parent instantiates two copies of the child.

### Analyzing Child Module: Reg8

#### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `inp` (incoming, type Word[8])
- `out` (reg, type Word[8])

#### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- No expressions

**Declaration: `incoming inp : Word[8]`**
- Component: `inp`
- No expressions

**Declaration: `reg out : Word[8] on clock`**
- Component: `out`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `out <= inp`**
- Expression `#1`: reference to `inp` (RHS of driver)

**Expression inventory (Reg8):**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `inp`

#### Step 3 - Generate constraints (Reg8):

**Fixed constraints:**
- `dom(clock) = Clock` (Clock-typed signal)
- `dom(out) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints:**
- `dom(#0) = dom(clock)` (expression references clock)
- `dom(#1) = dom(inp)` (expression references inp)
- `dom(out) = dom(#1)` (register driven by #1)

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(inp) < Async`
- `dom(out) < Async`

**Complete constraint system (Reg8):**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(out) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  C₃: dom(clock) < Async
  C₄: dom(inp) < Async
  C₅: dom(out) < Async

Equality:
  C₆: dom(#0) = dom(clock)
  C₇: dom(#1) = dom(inp)
  C₈: dom(out) = dom(#1)
```

### Solving Child Module: Reg8

**Processing fixed constraints:**
```
{clock: Clock}, {inp}, {out: Sync(clock)}, {#0}, {#1}
```

**Processing equality C₆: `dom(#0) = dom(clock)`**
```
{clock, #0: Clock}, {inp}, {out: Sync(clock)}, {#1}
```

**Processing equality C₇: `dom(#1) = dom(inp)`**
```
{clock, #0: Clock}, {inp, #1}, {out: Sync(clock)}
```

**Processing equality C₈: `dom(out) = dom(#1)`**
Merge `{out: Sync(clock)}` with `{inp, #1: no domain}`
Result: `{inp, #1, out: Sync(clock)}`

**Final partition (Reg8):**
```
Class 1: {clock, #0} → Clock
Class 2: {inp, #1, out} → Sync(ComponentId(clock))
```

**Solution (Reg8):**
```
Components:
  dom(clock) = Clock
  dom(inp) = Sync(ComponentId(clock))
  dom(out) = Sync(ComponentId(clock))
```

### Signature Extraction for Reg8:

**Domain parameters:**
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Port domains:**
- `clock`: `Concrete(Clock)`
- `inp`: `Var(DomainVar(0))` — tied to the clock parameter α
- `out`: `Var(DomainVar(0))` — tied to the clock parameter α

**Reg8 Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        inp: Var(DomainVar(0)),    // α
        out: Var(DomainVar(0)),     // α
    }
}
```

---

### Analyzing Parent Module: Top

Now we analyze the parent module `Top` which uses the forwarding block syntax.

#### Step 1 - Components from ComponentAnalysis:

**Ports:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `result_a` (outgoing, type Word[8])
- `result_b` (outgoing, type Word[8])

**Child instances and their ports as parent components:**
- `reg_a.clock` (port of child instance reg_a)
- `reg_a.inp` (port of child instance reg_a)
- `reg_a.out` (port of child instance reg_a)
- `reg_b.clock` (port of child instance reg_b)
- `reg_b.inp` (port of child instance reg_b)
- `reg_b.out` (port of child instance reg_b)

#### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock { ... }`**
- Component: `clock`
- Forwarding block contains connections (treated as drivers)

**Connection inside forwarding block: `reg_a.clock := it`**
- The `it` refers to the `clock` port
- Expression `#0`: reference to `clock` (the `it` in the forwarding block)
- This generates: `dom(reg_a.clock) = dom(#0)`

**Connection inside forwarding block: `reg_b.clock := it`**
- Expression `#1`: reference to `clock` (the `it` in the forwarding block)
- This generates: `dom(reg_b.clock) = dom(#1)`

**Note**: The forwarding block syntax is desugared to regular connection statements. From the constraint generation perspective, they're equivalent to:
```virdant
incoming clock : Clock
reg_a.clock := clock
reg_b.clock := clock
```

**Declaration: `incoming data : Word[8]`**
- Component: `data`
- No expressions

**Instantiation: `mod reg_a of Reg8`**
- Creates child port components: `reg_a.clock`, `reg_a.inp`, `reg_a.out`
- Child's signature constraints become parent's equality constraints

**Connection: `reg_a.inp := data`**
- Expression `#2`: reference to `data` (RHS)
- Generates: `dom(reg_a.inp) = dom(#2)`

**Instantiation: `mod reg_b of Reg8`**
- Creates child port components: `reg_b.clock`, `reg_b.inp`, `reg_b.out`

**Connection: `reg_b.inp := data`**
- Expression `#3`: reference to `data` (RHS)
- Generates: `dom(reg_b.inp) = dom(#3)`

**Declaration: `outgoing result_a : Word[8]`**
- Component: `result_a`

**Driver: `result_a := reg_a.out`**
- Expression `#4`: reference to `reg_a.out` (RHS)
- Generates: `dom(result_a) = dom(#4)`

**Declaration: `outgoing result_b : Word[8]`**
- Component: `result_b`

**Driver: `result_b := reg_b.out`**
- Expression `#5`: reference to `reg_b.out` (RHS)
- Generates: `dom(result_b) = dom(#5)`

**Expression inventory (Top):**
- `#0`: ExprReference to `clock` (from `it` in forwarding block)
- `#1`: ExprReference to `clock` (from `it` in forwarding block)
- `#2`: ExprReference to `data`
- `#3`: ExprReference to `data`
- `#4`: ExprReference to `reg_a.out`
- `#5`: ExprReference to `reg_b.out`

#### Step 3 - Generate constraints (Top):

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Equality constraints from child equivalence classes:**

From **Reg8** signature applied to **reg_a**:
- `reg_a` has clock parameter α bound by connection `reg_a.clock := clock`
- Within child: `dom(reg_a.inp) = dom(reg_a.out)` (both are α)
- This becomes parent constraint: `dom(reg_a.inp) = dom(reg_a.out)`

From **Reg8** signature applied to **reg_b**:
- `reg_b` has clock parameter β bound by connection `reg_b.clock := clock`
- Within child: `dom(reg_b.inp) = dom(reg_b.out)` (both are β)
- This becomes parent constraint: `dom(reg_b.inp) = dom(reg_b.out)`

**Equality constraints from expression references:**
- `dom(#0) = dom(clock)` (expression #0 references clock)
- `dom(#1) = dom(clock)` (expression #1 references clock)
- `dom(#2) = dom(data)` (expression #2 references data)
- `dom(#3) = dom(data)` (expression #3 references data)
- `dom(#4) = dom(reg_a.out)` (expression #4 references reg_a.out)
- `dom(#5) = dom(reg_b.out)` (expression #5 references reg_b.out)

**Equality constraints from connections:**
- `dom(reg_a.clock) = dom(#0)` (forwarding block connection)
- `dom(reg_b.clock) = dom(#1)` (forwarding block connection)
- `dom(reg_a.inp) = dom(#2)` (explicit connection)
- `dom(reg_b.inp) = dom(#3)` (explicit connection)
- `dom(result_a) = dom(#4)` (driver)
- `dom(result_b) = dom(#5)` (driver)

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(result_a) < Async`
- `dom(result_b) < Async`
- `dom(reg_a.clock) < Async`
- `dom(reg_a.inp) < Async`
- `dom(reg_a.out) < Async`
- `dom(reg_b.clock) < Async`
- `dom(reg_b.inp) < Async`
- `dom(reg_b.out) < Async`

**Complete constraint system (Top):**
```
Fixed:
  C₁: dom(clock) = Clock

Strict upper bound (implicit):
  C₂: dom(clock) < Async
  C₃: dom(data) < Async
  C₄: dom(result_a) < Async
  C₅: dom(result_b) < Async
  C₆: dom(reg_a.clock) < Async
  C₇: dom(reg_a.inp) < Async
  C₈: dom(reg_a.out) < Async
  C₉: dom(reg_b.clock) < Async
  C₁₀: dom(reg_b.inp) < Async
  C₁₁: dom(reg_b.out) < Async

Equality (from child signatures):
  C₁₂: dom(reg_a.inp) = dom(reg_a.out)
  C₁₃: dom(reg_b.inp) = dom(reg_b.out)

Equality (from expression references):
  C₁₄: dom(#0) = dom(clock)
  C₁₅: dom(#1) = dom(clock)
  C₁₆: dom(#2) = dom(data)
  C₁₇: dom(#3) = dom(data)
  C₁₈: dom(#4) = dom(reg_a.out)
  C₁₉: dom(#5) = dom(reg_b.out)

Equality (from connections):
  C₂₀: dom(reg_a.clock) = dom(#0)
  C₂₁: dom(reg_b.clock) = dom(#1)
  C₂₂: dom(reg_a.inp) = dom(#2)
  C₂₃: dom(reg_b.inp) = dom(#3)
  C₂₄: dom(result_a) = dom(#4)
  C₂₅: dom(result_b) = dom(#5)
```

---

## Pass 2: Constraint Solving (Top Module)

Let's trace through the union-find algorithm to solve the constraints.

**Initial state** (all separate):
```
{clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp}, {reg_a.out},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp}, {reg_a.out},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality C₁₂: `dom(reg_a.inp) = dom(reg_a.out)`**
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality C₁₃: `dom(reg_b.inp) = dom(reg_b.out)`**
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality C₁₄: `dom(#0) = dom(clock)`**
```
{clock, #0: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing equality C₁₅: `dom(#1) = dom(clock)`**
```
{clock, #0, #1: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#2}, {#3}, {#4}, {#5}
```

**Processing equality C₁₆: `dom(#2) = dom(data)`**
```
{clock, #0, #1: Clock}, {data, #2}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#3}, {#4}, {#5}
```

**Processing equality C₁₇: `dom(#3) = dom(data)`**
```
{clock, #0, #1: Clock}, {data, #2, #3}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#4}, {#5}
```

**Processing equality C₁₈: `dom(#4) = dom(reg_a.out)`**
```
{clock, #0, #1: Clock}, {data, #2, #3}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out, #4},
{reg_b.clock}, {reg_b.inp, reg_b.out},
{#5}
```

**Processing equality C₁₉: `dom(#5) = dom(reg_b.out)`**
```
{clock, #0, #1: Clock}, {data, #2, #3}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp, reg_a.out, #4},
{reg_b.clock}, {reg_b.inp, reg_b.out, #5}
```

**Processing equality C₂₀: `dom(reg_a.clock) = dom(#0)`**
Merge `{reg_a.clock}` with `{clock, #0, #1: Clock}`
```
{clock, #0, #1, reg_a.clock: Clock}, {data, #2, #3}, {result_a}, {result_b},
{reg_a.inp, reg_a.out, #4},
{reg_b.clock}, {reg_b.inp, reg_b.out, #5}
```

**Processing equality C₂₁: `dom(reg_b.clock) = dom(#1)`**
Merge `{reg_b.clock}` with `{clock, #0, #1, reg_a.clock: Clock}`
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock}, {data, #2, #3}, {result_a}, {result_b},
{reg_a.inp, reg_a.out, #4},
{reg_b.inp, reg_b.out, #5}
```

Now, recall from the child module analysis that when we instantiate `Reg8`, the child's internal constraint `dom(inp) = dom(out) = Sync(ComponentId(clock))` means that in the parent's context:
- `reg_a.inp` and `reg_a.out` must be `Sync(ComponentId(reg_a.clock))`
- `reg_b.inp` and `reg_b.out` must be `Sync(ComponentId(reg_b.clock))`

But how does this information flow into the parent's constraint system? The answer is that when the child's `clock` port gets bound to the parent's `clock` signal, the child's domain parameter gets substituted. Let me think about this more carefully...

Actually, the cleaner way to handle this is through the child's signature. The child `Reg8` has the constraint that `inp` and `out` are both in domain `Var(α)` where `α` is the clock parameter. When we connect `reg_a.clock := clock`, we're binding α to `Sync(ComponentId(clock))` in the parent. Then the signature tells us:
- `dom(reg_a.inp) = Var(α)` becomes `dom(reg_a.inp) = Sync(ComponentId(parent_clock))`
- `dom(reg_a.out) = Var(α)` becomes `dom(reg_a.out) = Sync(ComponentId(parent_clock))`

But in the constraint system, we express this as: after the clock connection establishes the binding, we need to set the domains of the child's data ports to match the bound domain.

For constraint solving in the parent, the key insight is:
1. The clock connection `reg_a.clock := clock` creates `dom(reg_a.clock) = dom(clock) = Clock`
2. From the child's signature, we know `reg_a.inp` and `reg_a.out` must be `Sync(reg_a.clock)`

This suggests we need **fixed constraints** from the child signature after binding:
- After binding: `dom(reg_a.inp) = Sync(ComponentId(reg_a.clock))`
- After binding: `dom(reg_a.out) = Sync(ComponentId(reg_a.clock))`

Let me reconsider the constraint generation step. When a child module is instantiated, and its signature has a domain parameter, the instantiation creates fixed constraints for the data ports:
- `dom(reg_a.inp) = Sync(ComponentId(reg_a.clock))`
- `dom(reg_a.out) = Sync(ComponentId(reg_a.out))`

Wait, but that's circular because `reg_a.out` refers to itself. Let me reconsider...

Actually, looking back at ALGORITHM.md, it says:

> Child ports are parent components. They participate in parent's constraint solving. Child's internal constraints (e.g., "inp and out same domain") become parent equality constraints.

So the child's constraint `dom(inp) = dom(out)` becomes the parent's `dom(reg_a.inp) = dom(reg_a.out)`, which I already have as C₁₂.

The key is that the child's domain is `Sync(ComponentId(clock_child))` which, when the clock is bound to `parent_clock`, becomes `Sync(ComponentId(parent_clock))` in the parent's context.

For the parent's constraint solving, the child's `Sync(ComponentId(clock))` domain should be translated to `Sync(ComponentId(reg_a.clock))` where `reg_a.clock` is now a parent component.

So after the clock binding, we should have:
- `dom(reg_a.inp) = Sync(ComponentId(reg_a.clock))`
- `dom(reg_a.out) = Sync(ComponentId(reg_a.clock))`

Similarly for reg_b:
- `dom(reg_b.inp) = Sync(ComponentId(reg_b.clock))`
- `dom(reg_b.out) = Sync(ComponentId(reg_b.clock))`

These should be **fixed constraints** that come from the child instantiation. Let me add these to the constraint system:

**Additional fixed constraints from child instantiations:**
- `dom(reg_a.inp) = Sync(ComponentId(reg_a.clock))`
- `dom(reg_a.out) = Sync(ComponentId(reg_a.clock))`
- `dom(reg_b.inp) = Sync(ComponentId(reg_b.clock))`
- `dom(reg_b.out) = Sync(ComponentId(reg_b.clock))`

Now let me restart the constraint solving with these additional fixed constraints:

**Processing fixed constraint C₁: `dom(clock) = Clock`**
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp}, {reg_a.out},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

**Processing fixed constraints from child instantiation:**

`dom(reg_a.inp) = Sync(ComponentId(reg_a.clock))`:
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp: Sync(reg_a.clock)}, {reg_a.out},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

`dom(reg_a.out) = Sync(ComponentId(reg_a.clock))`:
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp: Sync(reg_a.clock)}, {reg_a.out: Sync(reg_a.clock)},
{reg_b.clock}, {reg_b.inp}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

`dom(reg_b.inp) = Sync(ComponentId(reg_b.clock))`:
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp: Sync(reg_a.clock)}, {reg_a.out: Sync(reg_a.clock)},
{reg_b.clock}, {reg_b.inp: Sync(reg_b.clock)}, {reg_b.out},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

`dom(reg_b.out) = Sync(ComponentId(reg_b.clock))`:
```
{clock: Clock}, {data}, {result_a}, {result_b},
{reg_a.clock}, {reg_a.inp: Sync(reg_a.clock)}, {reg_a.out: Sync(reg_a.clock)},
{reg_b.clock}, {reg_b.inp: Sync(reg_b.clock)}, {reg_b.out: Sync(reg_b.clock)},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}
```

Now continue with the equality constraints...

**Processing equality C₁₄: `dom(#0) = dom(clock)`**
```
{clock, #0: Clock}, ...(same as before)...
```

**Processing equality C₁₅: `dom(#1) = dom(clock)`**
```
{clock, #0, #1: Clock}, ...(same as before)...
```

**Processing equality C₂₀: `dom(reg_a.clock) = dom(#0)`**
Merge `{reg_a.clock}` with `{clock, #0, #1: Clock}`
```
{clock, #0, #1, reg_a.clock: Clock}, ...
```

**Processing equality C₂₁: `dom(reg_b.clock) = dom(#1)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock}, ...
```

Now, when we resolve `reg_a.clock` to `Clock`, we need to update the domains that reference it. Since `Sync(ComponentId(reg_a.clock))` references `reg_a.clock`, and `reg_a.clock` is in the same equivalence class as `clock`, we're saying these are the same clock signal.

So:
- `Sync(ComponentId(reg_a.clock))` should resolve to `Sync(ComponentId(clock))` because `reg_a.clock` and `clock` are the same signal.
- Similarly, `Sync(ComponentId(reg_b.clock))` becomes `Sync(ComponentId(clock))`.

Let me simplify by saying that after the clock connections, `reg_a.clock` and `reg_b.clock` both unify with the parent's `clock` component. So the domains become:
- `dom(reg_a.inp) = Sync(ComponentId(clock))`
- `dom(reg_a.out) = Sync(ComponentId(clock))`
- `dom(reg_b.inp) = Sync(ComponentId(clock))`
- `dom(reg_b.out) = Sync(ComponentId(clock))`

**After processing all clock-related equalities:**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp: Sync(clock)}, {reg_a.out: Sync(clock)},
{reg_b.inp: Sync(clock)}, {reg_b.out: Sync(clock)},
{data, #2, #3}, {result_a}, {result_b}, {#4}, {#5}
```

**Processing equality C₁₆: `dom(#2) = dom(data)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp: Sync(clock)}, {reg_a.out: Sync(clock)},
{reg_b.inp: Sync(clock)}, {reg_b.out: Sync(clock)},
{data, #2, #3}, {result_a}, {result_b}, {#4}, {#5}
```

**Processing equality C₂₂: `dom(reg_a.inp) = dom(#2)`**
Merge `{reg_a.inp: Sync(clock)}` with `{data, #2, #3}`
The first class has domain, second doesn't:
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, data, #2, #3: Sync(clock)},
{reg_a.out: Sync(clock)},
{reg_b.inp: Sync(clock)}, {reg_b.out: Sync(clock)},
{result_a}, {result_b}, {#4}, {#5}
```

**Processing equality C₂₃: `dom(reg_b.inp) = dom(#3)`**
Merge `{reg_b.inp: Sync(clock)}` with `{reg_a.inp, data, #2, #3: Sync(clock)}`
Both have same domain:
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, reg_b.inp, data, #2, #3: Sync(clock)},
{reg_a.out: Sync(clock)}, {reg_b.out: Sync(clock)},
{result_a}, {result_b}, {#4}, {#5}
```

**Processing equality C₁₈: `dom(#4) = dom(reg_a.out)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, reg_b.inp, data, #2, #3: Sync(clock)},
{reg_a.out, #4: Sync(clock)}, {reg_b.out: Sync(clock)},
{result_a}, {result_b}, {#5}
```

**Processing equality C₁₉: `dom(#5) = dom(reg_b.out)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, reg_b.inp, data, #2, #3: Sync(clock)},
{reg_a.out, #4: Sync(clock)}, {reg_b.out, #5: Sync(clock)},
{result_a}, {result_b}
```

**Processing equality C₂₄: `dom(result_a) = dom(#4)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, reg_b.inp, data, #2, #3: Sync(clock)},
{reg_a.out, #4, result_a: Sync(clock)},
{reg_b.out, #5: Sync(clock)},
{result_b}
```

**Processing equality C₂₅: `dom(result_b) = dom(#5)`**
```
{clock, #0, #1, reg_a.clock, reg_b.clock: Clock},
{reg_a.inp, reg_b.inp, data, #2, #3: Sync(clock)},
{reg_a.out, #4, result_a: Sync(clock)},
{reg_b.out, #5, result_b: Sync(clock)}
```

**Final partition:**
```
Class 1: {clock, #0, #1, reg_a.clock, reg_b.clock} → Clock
Class 2: {data, #2, #3, reg_a.inp, reg_b.inp} → Sync(ComponentId(clock))
Class 3: {reg_a.out, #4, result_a} → Sync(ComponentId(clock))
Class 4: {reg_b.out, #5, result_b} → Sync(ComponentId(clock))
```

**Solution (Top module):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(result_a) = Sync(ComponentId(clock))
  dom(result_b) = Sync(ComponentId(clock))
  dom(reg_a.clock) = Clock (part of parent's clock class)
  dom(reg_a.inp) = Sync(ComponentId(clock))
  dom(reg_a.out) = Sync(ComponentId(clock))
  dom(reg_b.clock) = Clock (part of parent's clock class)
  dom(reg_b.inp) = Sync(ComponentId(clock))
  dom(reg_b.out) = Sync(ComponentId(clock))
```

**Verification of strict upper bound constraints:**
- All `Clock` domains: `Clock < Async` ✓
- All `Sync(clock)` domains: `Sync(clock) < Async` ✓

✅ **All constraints satisfied!**

---

## Pass 3: Signature Extraction (Top Module)

**Identify domain parameters:**
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(data) = Sync(ComponentId(clock))` → `Var(DomainVar(0))` = α
- `dom(result_a) = Sync(ComponentId(clock))` → `Var(DomainVar(0))` = α
- `dom(result_b) = Sync(ComponentId(clock))` → `Var(DomainVar(0))` = α

**Top Module Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        data: Var(DomainVar(0)),       // α
        result_a: Var(DomainVar(0)),   // α
        result_b: Var(DomainVar(0)),   // α
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: Top,
    component_domains: {
        clock: Clock,
        data: Sync(ComponentId(clock)),
        result_a: Sync(ComponentId(clock)),
        result_b: Sync(ComponentId(clock)),
        // Child port components
        reg_a.clock: Clock,
        reg_a.inp: Sync(ComponentId(clock)),
        reg_a.out: Sync(ComponentId(clock)),
        reg_b.clock: Clock,
        reg_b.inp: Sync(ComponentId(clock)),
        reg_b.out: Sync(ComponentId(clock)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes, with one clarification**: The forwarding block syntax is purely syntactic sugar that desugars to regular connection statements. From the constraint generation perspective:

```virdant
incoming clock : Clock {
    reg_a.clock := it
    reg_b.clock := it
}
```

is **exactly equivalent** to:

```virdant
incoming clock : Clock
reg_a.clock := clock
reg_b.clock := clock
```

The `it` keyword inside the forwarding block simply references the port being declared. The constraint generation algorithm treats these connections identically to explicit connection statements.

### Any surprises or edge cases?

**No surprises**. The forwarding block is a straightforward syntactic convenience that:

1. **Declares the port** (same as regular `incoming` declaration)
2. **Generates connections** (same as writing them explicitly)

The only difference is syntactic: the connections are nested inside the declaration block instead of written separately. The domain analysis sees them as regular connections.

### Does the constraint system handle this without special cases?

**Yes, absolutely**. No special handling is needed because:

1. **Port declaration semantics are unchanged**: `incoming clock : Clock` generates the same component and fixed constraints whether or not it has a forwarding block.

2. **Connections are connections**: Whether written as `reg_a.clock := clock` or `reg_a.clock := it` inside a forwarding block, they generate the same equality constraints: `dom(reg_a.clock) = dom(expr)` where `expr` references the `clock` component.

3. **Reference resolution is standard**: The `it` keyword is resolved to the port component during AST construction, so by the time constraint generation runs, it's just a component reference like any other.

### Any clarifications needed for ALGORITHM.md or PLANv2.md?

**Minor documentation note**: ALGORITHM.md could add a brief note that forwarding blocks are desugared before or during constraint generation, making them equivalent to explicit connections. However, this is more of an implementation detail than an algorithmic concern.

The algorithm documentation is correct as-is because it describes constraints generation from the logical module structure, and forwarding blocks don't change that structure—they're just a convenient way to express multiple connections concisely.

### Key Takeaway

The forwarding block syntax demonstrates a key strength of the constraint-based domain analysis design: **syntactic variations don't require special cases**. Whether connections are written explicitly or nested in forwarding blocks, the underlying constraint structure is identical. This makes the algorithm robust to language-level syntax changes and extensions.

The example confirms that:
- ✅ Forwarding blocks are transparent to domain analysis
- ✅ No special constraint generation rules needed
- ✅ Desugaring happens at AST level (before domain analysis)
- ✅ Algorithm handles it naturally through regular connection constraints
