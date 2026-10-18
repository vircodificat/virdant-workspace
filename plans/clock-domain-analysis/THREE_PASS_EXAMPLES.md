# Three-Pass Algorithm Examples

This document walks through examples of every situation the domain analysis algorithm encounters, showing the data structures built during each pass.

**Passes:**
1. **Constraint Generation**: Generate domain constraints over components and expressions
2. **Constraint Solving**: Find minimal domain assignment satisfying all constraints
3. **Validation**: Check structural properties (register drivers, etc.)

**Notation:**
- Components and expressions are the variables (e.g., `buffer`, `clock`, `0w8`)
- `dom(thing)` is the domain assigned to that component/expression
- **Constraints:**
  - Fixed: `dom(clock) = Domain::Clock` (clock must be Clock domain)
  - Equality: `buffer ≡ data` (must have same domain, i.e., `dom(buffer) = dom(data)`)
- `≡` is an equivalence relation (reflexive, symmetric, transitive) - union-find maintains equivalence classes

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

**What's happening**: A simple buffer with one clock. The `data` port flows into a register, and the register drives `out`. Everything should be on the same clock domain.

### Pass 1: Constraint Generation

**Variables:** clock (component), data (component), buffer (component), out (component)

**Constraints generated:**
```
1. dom(clock) = Domain::Clock              (clock-typed signal)
2. dom(buffer) = Sync(ComponentId(clock))  (register with 'on clock' clause)
3. buffer ≡ data                           (driver: buffer <= data)
4. out ≡ buffer                            (driver: out := buffer)
```

**Natural language**: The clock port gets the Clock domain. The register `buffer` is anchored to `Sync(clock)`. The constraint `buffer ≡ data` means they must have the same domain, and `out ≡ buffer` creates another equality constraint. These will propagate during solving.

### Pass 2: Constraint Solving

**Initial state**: Each variable starts in its own equivalence class
```
{clock}, {data}, {buffer}, {out}
```

**Processing constraints:**

1. `dom(clock) = Domain::Clock` → Assign Clock to clock's class: `{clock: Clock}`
2. `dom(buffer) = Sync(clock)` → Assign to buffer's class: `{buffer: Sync(clock)}`
3. `buffer ≡ data` → Merge classes. buffer's class has concrete domain Sync(clock), so merged class adopts it:
   - Result: `{data, buffer: Sync(clock)}`
4. `out ≡ buffer` → Merge out with buffer's class:
   - Result: `{data, buffer, out: Sync(clock)}`

**Final solution:**
```
dom(clock)  = Clock
dom(data)   = Sync(ComponentId(clock))
dom(buffer) = Sync(ComponentId(clock))
dom(out)    = Sync(ComponentId(clock))
```

### Pass 3: Validation

**Check register drivers:**
- `buffer <= data`: data has domain Sync(clock), buffer requires Sync(clock) ✓
- No async signals driving registers ✓

**Module signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // introduces α
    },
    port_domains: {
        clock: ???,  // Clock
        data: ???,   // Var(0) = α
        out: ???,    // Var(0) = α
    }
}
```

**Natural language**: All three ports (`clock`, `data`, `out`) are related: `data` and `out` are both on whatever clock is connected to the `clock` port. The signature captures this polymorphism: α is introduced by the clock port, and both data ports use α.

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

**What's happening**: A register driven by a constant literal. Constants should unify with any Sync domain.

### Pass 1: Constraint Generation

**Variables:** clock (component), counter (component), 0w8 (expression)

**Constraints:**
```
1. dom(clock) = Domain::Clock
2. dom(counter) = Sync(ComponentId(clock))
3. dom(0w8) = Domain::Constant         (literal with no signal references)
4. counter ≡ 0w8                       (driver: counter <= 0w8)
```

### Pass 2: Constraint Solving

**Processing:**
1. `dom(clock) = Clock` → `{clock: Clock}`
2. `dom(counter) = Sync(clock)` → `{counter: Sync(clock)}`
3. `dom(0w8) = Constant` → `{0w8: Constant}`
4. `counter ≡ 0w8` → Merge equivalence classes.
   - counter's class has Sync(clock), 0w8's class has Constant
   - **Constant is bottom of lattice**, so merged class adopts the more specific domain
   - Result: `{counter, 0w8: Sync(clock)}`

**Final solution:**
```
dom(clock)   = Clock
dom(counter) = Sync(ComponentId(clock))
dom(0w8)     = Sync(ComponentId(clock))  (promoted from Constant)
```

**Natural language**: The constant `0w8` starts as Domain::Constant (bottom of the lattice, maximally general). When merged with the register's equivalence class (which has Sync(clock)), it adopts that more specific domain. This is why literals "just work" in any clock domain without explicit annotation - Constant unifies with any Sync domain.

### Pass 3: Validation

All checks pass. The literal, now resolved to Sync(clock), is a valid driver for the register.

---

## Example 3: Explicit Async Signal

```virdant
mod AsyncExample {
    incoming clock : Clock
    incoming button : Bit async
    
    wire synced : Bit on clock {
        it := sync(button)
    }
}
```

**What's happening**: An async input (button press from outside world) is synchronized to the clock domain using the `sync()` builtin.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock:  δ₀
button: δ₁
synced: δ₂
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Async              (explicitly marked async)
3. δ₂ = Sync(ComponentId(clock))   (wire with 'on clock')
4. (special) sync(button) produces Sync(clock)  // handled by builtin
```

**Natural language**: The `button` port is explicitly declared `async`, so it gets Domain::Async (top of the lattice). The `sync()` builtin is special-cased: it takes an async input and produces a synchronized output on the specified clock. This is NOT expressed as a simple equality constraint.

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock → `{δ₀: Clock}`
2. δ₁ = Async → `{δ₁: Async}`
3. δ₂ = Sync(clock) → `{δ₂: Sync(clock)}`
4. No constraint between δ₁ and δ₂ (sync() is the boundary)

**Final domains:**
```
clock:  Clock
button: Async
synced: Sync(ComponentId(clock))
```

### Pass 3: Validation

**Check async boundaries**: The `sync()` builtin correctly bridges Async → Sync(clock). No async signal directly drives a register. ✓

**Module signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: ???,   // Clock
        button: ???,  // Async (concrete)
    }
}
```

---

## Example 4: Multi-Clock with Separation

```virdant
mod Dual {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    incoming data_a : Bit
    incoming data_b : Bit
    
    reg r_a : Bit on clk_a {
        it <= data_a
    }
    
    reg r_b : Bit on clk_b {
        it <= data_b
    }
}
```

**What's happening**: Two independent clock domains. `data_a` flows into a register on `clk_a`, `data_b` into a register on `clk_b`. No mixing, so this should be legal.

### Pass 1: Constraint Generation

**Domain variables:**
```
clk_a:  δ₀
clk_b:  δ₁
data_a: δ₂
data_b: δ₃
r_a:    δ₄
r_b:    δ₅
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Clock
3. δ₄ = Sync(ComponentId(clk_a))
4. δ₅ = Sync(ComponentId(clk_b))
5. δ₄ = δ₂  (r_a <= data_a)
6. δ₅ = δ₃  (r_b <= data_b)
```

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock → `{δ₀: Clock}`
2. δ₁ = Clock → `{δ₁: Clock}`
3. δ₄ = Sync(clk_a) → `{δ₄: Sync(clk_a)}`
4. δ₅ = Sync(clk_b) → `{δ₅: Sync(clk_b)}`
5. δ₄ = δ₂ → `{δ₂, δ₄: Sync(clk_a)}`
6. δ₅ = δ₃ → `{δ₃, δ₅: Sync(clk_b)}`

**Final domains:**
```
clk_a:  Clock
clk_b:  Clock
data_a: Sync(ComponentId(clk_a))
data_b: Sync(ComponentId(clk_b))
r_a:    Sync(ComponentId(clk_a))
r_b:    Sync(ComponentId(clk_b))
```

**Natural language**: The two clock domains remain completely separate. ComponentId(clk_a) ≠ ComponentId(clk_b), so their Sync domains are distinct. No unification happens across domains.

### Pass 3: Validation

All checks pass. Each register is driven by a signal on its own clock domain. ✓

**Module signature:**
```rust
DomainSignature {
    domain_params: {
        clk_a: DomainVar(0),  // α
        clk_b: DomainVar(1),  // β
    },
    port_domains: {
        clk_a: ???,   // Clock
        clk_b: ???,   // Clock
        data_a: ???,  // Var(0) = α
        data_b: ???,  // Var(1) = β
    }
}
```

**Natural language**: This module is polymorphic in TWO domain variables (α and β). `data_a` is on α, `data_b` is on β. At instantiation, these can be bound to the same parent clock or different ones — the module doesn't care.

---

## Example 5: ERROR — Mixing Incompatible Clocks

```virdant
mod BadMix {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    reg r_a : Bit on clk_a { ... }
    reg r_b : Bit on clk_b { ... }
    
    wire mixed : Bit {
        it := r_a & r_b  // ERROR!
    }
}
```

**What's happening**: Attempting to combine signals from two different clock domains in a combinational expression without declaring the result `async`. This should error.

### Pass 1: Constraint Generation

**Domain variables:**
```
clk_a: δ₀
clk_b: δ₁
r_a:   δ₂
r_b:   δ₃
mixed: δ₄
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Clock
3. δ₂ = Sync(ComponentId(clk_a))
4. δ₃ = Sync(ComponentId(clk_b))
5. δ₄ = δ₂  (mixed := r_a & r_b, operand 1)
6. δ₄ = δ₃  (mixed := r_a & r_b, operand 2)
```

**Natural language**: The `&` operator requires all operands to have the same domain. This generates constraints that `mixed` must equal both `r_a`'s domain and `r_b`'s domain.

### Pass 2: Unification

**Processing:**
1-4. (as before, establish concrete domains)
5. δ₄ = δ₂ → `{δ₂, δ₄: Sync(clk_a)}`
6. δ₄ = δ₃ → Try to unify {δ₂, δ₄: Sync(clk_a)} with {δ₃: Sync(clk_b)}
   - **UNIFICATION ERROR**: Sync(clk_a) ≠ Sync(clk_b)
   - ComponentId(clk_a) ≠ ComponentId(clk_b)

**Error reported:**
```
Cannot unify Sync(clk_a) with Sync(clk_b)
Location: wire mixed at 'r_a & r_b' expression
Suggestion: Declare 'mixed' as 'async' if mixing domains is intentional
```

### Pass 3: Validation

(Not reached due to unification error in Pass 2)

**Natural language**: The algorithm detects the conflict during unification. The union-find maintains the invariant that each equivalence class has at most one concrete domain. When trying to merge two sets with incompatible concrete domains, it emits an error.

---

## Example 6: Mixing Clocks with Explicit Async Target

```virdant
mod GoodMix {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    reg r_a : Bit on clk_a { ... }
    reg r_b : Bit on clk_b { ... }
    
    wire mixed : Bit async {
        it := r_a & r_b  // OK - target is async
    }
}
```

**What's happening**: Same as Example 5, but the target `mixed` is declared `async`, which is the escape hatch for mixing domains.

### Pass 1: Constraint Generation

**Domain variables:**
```
(same as Example 5)
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Clock
3. δ₂ = Sync(ComponentId(clk_a))
4. δ₃ = Sync(ComponentId(clk_b))
5. δ₄ = Domain::Async  (explicitly declared async)
6. (NO equality constraints for operands — async target allows anything)
```

**Natural language**: When the target is explicitly `async`, we do NOT generate equality constraints between operands and the result. Instead, the operands are implicitly upcast to Async. This is the "target is async" exception from the domain rules.

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock → `{δ₀: Clock}`
2. δ₁ = Clock → `{δ₁: Clock}`
3. δ₂ = Sync(clk_a) → `{δ₂: Sync(clk_a)}`
4. δ₃ = Sync(clk_b) → `{δ₃: Sync(clk_b)}`
5. δ₄ = Async → `{δ₄: Async}`
6. No further unification (domains remain separate)

**Final domains:**
```
clk_a: Clock
clk_b: Clock
r_a:   Sync(ComponentId(clk_a))
r_b:   Sync(ComponentId(clk_b))
mixed: Async
```

**Natural language**: All domains coexist peacefully. The `async` target is the top of the lattice, so it accepts inputs from any domain via implicit upcast. The domains of `r_a` and `r_b` are not unified.

### Pass 3: Validation

Check that `mixed` (async) doesn't drive any registers. ✓

---

## Example 7: ERROR — Async Driving Register

```virdant
mod BadAsync {
    incoming clock : Clock
    incoming data : Bit async
    
    reg buffer : Bit on clock {
        it <= data  // ERROR!
    }
}
```

**What's happening**: Attempting to drive a register directly with an async signal. This should error.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock:  δ₀
data:   δ₁
buffer: δ₂
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Async
3. δ₂ = Sync(ComponentId(clock))
4. δ₂ = δ₁  (buffer <= data)
```

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock → `{δ₀: Clock}`
2. δ₁ = Async → `{δ₁: Async}`
3. δ₂ = Sync(clock) → `{δ₂: Sync(clock)}`
4. δ₂ = δ₁ → Try to unify Sync(clock) with Async
   - **UNIFICATION ERROR**: Async does not unify with Sync

**Error reported:**
```
Cannot unify Sync(clock) with Async
Location: register buffer driver 'data'
Suggestion: Use Sync module to safely cross clock domains
```

**Natural language**: Async is at the top of the lattice — things upcast TO it, but it doesn't unify with Sync domains. The unification fails, preventing the metastability hazard.

### Pass 3: Validation

(Not reached)

---

## Example 8: `when` Expression Domain Unification

```virdant
mod WhenExample {
    incoming clock : Clock
    incoming enable : Bit
    incoming data : Bit
    
    reg r1 : Bit on clock { ... }
    reg r2 : Bit on clock { ... }
    
    wire result : Bit {
        it := when {
            case enable => r1
            else => r2
        }
    }
}
```

**What's happening**: A `when` expression (mux) combines signals. All guards and results must have the same domain.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock:  δ₀
enable: δ₁
data:   δ₂
r1:     δ₃
r2:     δ₄
result: δ₅
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₃ = Sync(ComponentId(clock))
3. δ₄ = Sync(ComponentId(clock))
4. δ₅ = δ₁  (guard: enable)
5. δ₅ = δ₃  (result: r1)
6. δ₅ = δ₄  (result: r2)
```

**Natural language**: The `when` expression requires its guard (`enable`) and all result branches (`r1`, `r2`) to have the same domain as the overall result. This is because it compiles to a combinational mux — everything flows through the same logic.

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock
2-3. δ₃, δ₄ = Sync(clock)
4. δ₅ = δ₁ → `{δ₁, δ₅}`
5. δ₅ = δ₃ → Merge with Sync(clock) → `{δ₁, δ₃, δ₅: Sync(clock)}`
6. δ₅ = δ₄ → Already same set

**Final domains:**
```
clock:  Clock
enable: Sync(ComponentId(clock))
data:   (unconstrained, remains as δ₂)
r1:     Sync(ComponentId(clock))
r2:     Sync(ComponentId(clock))
result: Sync(ComponentId(clock))
```

**Natural language**: Everything involved in the `when` expression unified to Sync(clock). The `enable` signal, initially unconstrained, gets pulled into the clock domain by the unification. The unused `data` port remains unconstrained.

### Pass 3: Validation

All checks pass. ✓

---

## Example 9: Unconstrained Port (Free Variable)

```virdant
mod Unused {
    incoming clock : Clock
    incoming data : Bit    // never used
    
    reg counter : Word[8] on clock {
        it <= 0w8
    }
}
```

**What's happening**: The `data` port is never connected to anything, so it has no constraints. It should remain a free domain variable in the signature.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock:   δ₀
data:    δ₁
counter: δ₂
literal: δ₃
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₂ = Sync(ComponentId(clock))
3. δ₃ = Domain::Constant
4. δ₂ = δ₃  (counter <= 0w8)
```

**Natural language**: No constraints mention δ₁ (`data`). It's completely unconstrained.

### Pass 2: Unification

**Processing:**
1. δ₀ = Clock
2. δ₂ = Sync(clock)
3. δ₃ = Constant
4. Merge δ₂ and δ₃ → `{δ₂, δ₃: Sync(clock)}`
5. δ₁ remains in its own set, unresolved

**Final domains:**
```
clock:   Clock
data:    δ₁ (free variable)
counter: Sync(ComponentId(clock))
literal: Sync(ComponentId(clock))
```

### Pass 3: Validation

**Check unconstrained ports**: `data` is unconstrained. At the top level, this is legal. In a child module, it becomes a free variable in the signature.

**Module signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0),  // α
    },
    port_domains: {
        clock: ???,  // Clock
        data: ???,   // Var(1) — a fresh free variable β, not tied to any clock
    }
}
```

**Natural language**: The signature has TWO domain variables: α (from the clock) and β (free, from the unconstrained `data` port). When this module is instantiated, β can be bound to any domain based on what the parent connects.

---

## Example 10: Cross-Module Instantiation (Simple)

```virdant
mod Child {
    incoming clk : Clock
    incoming data : Bit
    outgoing result : Bit {
        it := data
    }
}

mod Parent {
    incoming parent_clk : Clock
    reg r : Bit on parent_clk { ... }
    
    mod child of Child {
        it.clk := parent_clk
        it.data := r
    }
}
```

**What's happening**: Parent instantiates Child, connecting its clock and data. The domain analysis must substitute Child's polymorphic signature with Parent's concrete domains.

### Child's Domain Analysis (already computed)

**Final domains:**
```
clk:    Clock
data:   Sync(ComponentId(clk_child))
result: Sync(ComponentId(clk_child))
```

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clk: DomainVar(0),  // α
    },
    port_domains: {
        clk: ???,     // Clock
        data: ???,    // Var(0) = α
        result: ???,  // Var(0) = α
    }
}
```

### Parent's Domain Analysis — Pass 1: Constraint Generation

**Parent's domain variables:**
```
parent_clk: δ₀
r:          δ₁
child:      (submodule, not a signal)
```

**Constraints from Parent's body:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Sync(ComponentId(parent_clk))
```

**Cross-module constraints:**

When processing `mod child of Child { ... }`:

1. Get Child's signature
2. **Clock binding**: `it.clk := parent_clk`
   - Child's `clk` introduces DomainVar(0) = α
   - Bind α → Sync(ComponentId(parent_clk)) in Parent's context
3. **Data connection**: `it.data := r`
   - Child's `data` has domain Var(0) = α (from signature)
   - Substitute: α → Sync(ComponentId(parent_clk))
   - Generate constraint: `domain(r) = Sync(ComponentId(parent_clk))`
   - This is: `δ₁ = Sync(ComponentId(parent_clk))` (already established)

**Natural language**: The instantiation creates a substitution mapping from Child's domain variables to Parent's concrete domains. Child's α is bound to Parent's Sync(parent_clk). Each data port connection applies this substitution and checks that the parent signal matches.

### Parent's Pass 2: Unification

All constraints already consistent. No additional work needed.

### Parent's Pass 3: Validation

All checks pass. ✓

---

## Example 11: Cross-Module with Domain Mismatch ERROR

```virdant
mod Child {
    incoming clk : Clock
    incoming data : Bit
}

mod Parent {
    incoming clk_a : Clock
    incoming clk_b : Clock
    
    reg r_a : Bit on clk_a { ... }
    
    mod child of Child {
        it.clk := clk_b        // binds α to clk_b
        it.data := r_a         // ERROR: r_a is on clk_a, not clk_b
    }
}
```

**What's happening**: Parent connects a signal on `clk_a` to a child port that (via substitution) expects `clk_b`. This should error.

### Parent's Pass 1: Constraint Generation

**Domain variables:**
```
clk_a: δ₀
clk_b: δ₁
r_a:   δ₂
```

**Constraints from Parent's body:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Domain::Clock
3. δ₂ = Sync(ComponentId(clk_a))
```

**Cross-module constraints:**

1. **Clock binding**: `it.clk := clk_b`
   - Bind Child's α → Sync(ComponentId(clk_b))
2. **Data connection**: `it.data := r_a`
   - Child's `data` has domain Var(0) = α
   - Substitute: α → Sync(ComponentId(clk_b))
   - Generate constraint: `δ₂ = Sync(ComponentId(clk_b))`
   - But δ₂ is already Sync(ComponentId(clk_a))

**Constraint list includes:**
```
3. δ₂ = Sync(ComponentId(clk_a))
4. δ₂ = Sync(ComponentId(clk_b))  (from substitution)
```

### Parent's Pass 2: Unification

**Processing:**
- δ₂ = Sync(clk_a) → `{δ₂: Sync(clk_a)}`
- δ₂ = Sync(clk_b) → Try to unify with Sync(clk_a)
  - **UNIFICATION ERROR**: Sync(clk_a) ≠ Sync(clk_b)

**Error reported:**
```
Cannot unify Sync(clk_a) with Sync(clk_b)
Location: instantiation of Child, connection 'it.data := r_a'
Note: Child's 'data' port expects domain tied to 'clk' (bound to clk_b),
      but 'r_a' is on clk_a
```

**Natural language**: The substitution mechanism converts the child's polymorphic constraint into a concrete constraint in the parent's domain space. The unification failure pinpoints the exact connection that violates domain compatibility.

---

## Example 12: Struct Construction

```virdant
struct type Pair {
    x : Word[8]
    y : Word[8]
}

mod StructExample {
    incoming clock : Clock
    reg r1 : Word[8] on clock { ... }
    reg r2 : Word[8] on clock { ... }
    
    wire pair : Pair {
        it := ${ x = r1, y = r2 }
    }
}
```

**What's happening**: A struct is constructed from two fields. All field expressions must have the same domain.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock: δ₀
r1:    δ₁
r2:    δ₂
pair:  δ₃
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Sync(ComponentId(clock))
3. δ₂ = Sync(ComponentId(clock))
4. δ₃ = δ₁  (struct field x = r1)
5. δ₃ = δ₂  (struct field y = r2)
```

**Natural language**: The struct construction `${ x = r1, y = r2 }` requires all fields to have the same domain, which becomes the struct's domain.

### Pass 2: Unification

All δ₁, δ₂, δ₃ unify to Sync(clock).

**Final domains:**
```
clock: Clock
r1:    Sync(ComponentId(clock))
r2:    Sync(ComponentId(clock))
pair:  Sync(ComponentId(clock))
```

### Pass 3: Validation

All checks pass. The entire struct (and transitively all its fields) has domain Sync(clock). ✓

---

## Example 13: Array Indexing

```virdant
mod ArrayExample {
    incoming clock : Clock
    incoming arr : Word[8][4]  // array of 4 words
    incoming idx : Word[2]
    
    reg r : Word[8] on clock {
        it <= arr[idx]
    }
}
```

**What's happening**: Array indexing. The element has the same domain as the array, and the index must also have that domain.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock: δ₀
arr:   δ₁
idx:   δ₂
r:     δ₃
elem:  δ₄ (the expression arr[idx])
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₃ = Sync(ComponentId(clock))
3. δ₄ = δ₁  (array element has array's domain)
4. δ₂ = δ₁  (index has array's domain)
5. δ₃ = δ₄  (r <= arr[idx])
```

**Natural language**: Array indexing creates two constraints: the element domain equals the array domain, and the index domain also equals the array domain. This ensures everything involved in the indexing operation is synchronized.

### Pass 2: Unification

**Processing:**
- δ₃ = Sync(clock)
- δ₄ = δ₁ → `{δ₁, δ₄}`
- δ₂ = δ₁ → `{δ₁, δ₂, δ₄}`
- δ₃ = δ₄ → `{δ₁, δ₂, δ₃, δ₄: Sync(clock)}`

**Final domains:**
```
clock: Clock
arr:   Sync(ComponentId(clock))
idx:   Sync(ComponentId(clock))
r:     Sync(ComponentId(clock))
elem:  Sync(ComponentId(clock))
```

**Natural language**: Everything unified to the same clock domain. Both the array and the index are pulled into Sync(clock) by the register constraint.

### Pass 3: Validation

All checks pass. ✓

---

## Example 14: @unsafe_clocks Module (Sync)

```virdant
@unsafe_clocks
mod Sync {
    incoming inp : Bit async
    incoming clock : Clock
    outgoing reg out : Bit on clock {
        it <= inp  // Would normally be ERROR, but suppressed
    }
}
```

**What's happening**: The Sync module uses `@unsafe_clocks` to bypass normal domain checking. Its signature is synthesized from port declarations, but the body is not analyzed.

### Domain Analysis for Sync

**Pass 1: Constraint Generation**

Check for `@unsafe_clocks` annotation → **SKIP constraint generation**

### Pass 2: Unification

**SKIP** (no constraints)

### Pass 3: Validation

**SKIP** (no validation for @unsafe_clocks modules)

### Synthesized Signature

```rust
DomainAnalysis {
    moddef: Sync,
    unsafe_clocks: true,
    domains: {},  // empty
    diagnostics: [],  // no errors reported
    signature: DomainSignature {
        domain_params: {
            clock: DomainVar(0),  // α introduced by clock port
        },
        port_domains: {
            inp: ???,    // Async (from 'async' modifier)
            clock: ???,  // Clock
            out: ???,    // Var(0) = α (from 'on clock')
        }
    }
}
```

**Natural language**: The signature is extracted by inspecting the port AST:
- `inp : Bit async` → concrete domain Async
- `clock : Clock` → introduces domain variable α
- `out : reg Bit on clock` → domain is α (tied to the clock port)

The body driver `out <= inp` is NOT checked, even though it would normally error (async driving a register). The `@unsafe_clocks` annotation grants carte blanche.

### Using Sync in a Parent Module

When a parent instantiates Sync:

```virdant
mod Parent {
    incoming clk : Clock
    incoming button : Bit async
    
    mod sync of Sync {
        it.clock := clk           // binds α to Sync(ComponentId(clk))
        it.inp := button          // check: Async = Async ✓
    }
    
    wire synced : Bit {
        it := sync.out            // domain(sync.out) = α = Sync(clk)
    }
}
```

**Constraint generation in Parent:**
1. `it.clock := clk` → Bind α to Sync(ComponentId(clk))
2. `it.inp := button` → Check domain(button) = Async. ✓ (both Async)
3. `sync.out` has domain α[substituted] = Sync(ComponentId(clk))
4. `it := sync.out` → domain(synced) = Sync(ComponentId(clk))

**Natural language**: Even though Sync's body is unchecked, its signature provides enough information for parents to perform correct domain analysis. The signature accurately describes the external interface: takes Async in, produces the clock's domain out.

---

## Example 15: match Expression

```virdant
enum type State {
    Idle = 0
    Active = 1
}

mod MatchExample {
    incoming clock : Clock
    reg state : State on clock { ... }
    
    wire is_active : Bit {
        it := match state {
            case #Idle => false
            case #Active => true
        }
    }
}
```

**What's happening**: A `match` expression destructures an enum. The subject and all result expressions must have the same domain.

### Pass 1: Constraint Generation

**Domain variables:**
```
clock:     δ₀
state:     δ₁
is_active: δ₂
literal_f: δ₃ (false)
literal_t: δ₄ (true)
```

**Constraints:**
```
1. δ₀ = Domain::Clock
2. δ₁ = Sync(ComponentId(clock))
3. δ₃ = Domain::Constant
4. δ₄ = Domain::Constant
5. δ₂ = δ₁  (subject: state)
6. δ₂ = δ₃  (result: false)
7. δ₂ = δ₄  (result: true)
```

**Natural language**: The `match` requires the subject (`state`) and all case results (`false`, `true`) to have the same domain as the match expression's result.

### Pass 2: Unification

**Processing:**
- δ₁ = Sync(clock)
- δ₃, δ₄ = Constant
- Unify all: Constants adopt Sync(clock)

**Final domains:**
```
clock:     Clock
state:     Sync(ComponentId(clock))
is_active: Sync(ComponentId(clock))
literal_f: Sync(ComponentId(clock))
literal_t: Sync(ComponentId(clock))
```

### Pass 3: Validation

All checks pass. ✓

---

## Summary Table of Scenarios

| Example | Scenario | Key Constraint | Pass 2 Outcome | Pass 3 Action |
|---------|----------|----------------|----------------|---------------|
| 1 | Simple single-clock | All signals unify to one Sync | Success | Validate register drivers |
| 2 | Constant literal | Constant unifies with Sync | Success (constant promoted) | Pass |
| 3 | Explicit async | Async separate from Sync | Success (no unification needed) | Validate sync() boundary |
| 4 | Multi-clock separation | Two independent Sync domains | Success (no mixing) | Pass |
| 5 | ERROR: Mix clocks | Sync(A) = Sync(B) constraint | **Unification error** | Not reached |
| 6 | Mix clocks with async target | No equality constraints (async target) | Success (domains separate) | Pass |
| 7 | ERROR: Async→register | Async = Sync constraint | **Unification error** | Not reached |
| 8 | when expression | All guards/results unify | Success | Pass |
| 9 | Unconstrained port | No constraints on port | Success (free variable) | Mark as free in signature |
| 10 | Cross-module instantiation | Substitution creates constraints | Success | Pass |
| 11 | ERROR: Cross-module mismatch | Substituted constraint conflicts | **Unification error** | Not reached |
| 12 | Struct construction | All fields unify | Success | Pass |
| 13 | Array indexing | Array, index, element unify | Success | Pass |
| 14 | @unsafe_clocks (Sync) | No constraints generated | Signature synthesized | Skip validation |
| 15 | match expression | Subject and results unify | Success | Pass |

---

## Notes on Algorithm Mechanics

### Constraint Generation Details

**Domain variable initialization:**
- Every component gets a fresh domain variable δᵢ
- Clock-typed signals are immediately constrained to Domain::Clock
- Reset-typed signals to Domain::Reset
- Explicitly `async` signals to Domain::Async
- Literals/constants to Domain::Constant
- Registers with `on clock` immediately constrained to Sync(ComponentId(clock))

**Equality constraint sources:**
- Wire/reg drivers: `domain(lhs) = domain(rhs)`
- Operators: `domain(result) = domain(operand₁) = domain(operand₂) = ...`
- Struct construction: `domain(struct) = domain(field₁) = domain(field₂) = ...`
- Array indexing: `domain(element) = domain(array)` and `domain(index) = domain(array)`
- `when`/`match`: `domain(result) = domain(guard₁) = ... = domain(result₁) = ...`

**Exception — async targets:**
When the target is explicitly `async`, do NOT generate equality constraints. The operands implicitly upcast.

### Unification via Union-Find

**Data structure:**
```rust
struct UnionFind {
    parent: Vec<usize>,           // parent pointers
    domain: Vec<Option<Domain>>,  // concrete domain if known
}
```

**Union operation:**
- Find representatives of both sets
- If both have concrete domains, check compatibility:
  - Constant + Sync(C) → Sync(C)
  - Sync(C) + Sync(C) → Sync(C)
  - Sync(C₁) + Sync(C₂) where C₁ ≠ C₂ → ERROR
  - Async + Sync → ERROR
  - Clock/Reset + anything besides itself → ERROR
- Merge sets, keeping the concrete domain

**Error messages:**
Track which constraint caused each union. On conflict, report the source constraint's location.

### Validation Pass

**Checks:**
1. For each register with `on clock`, verify driver domain = Sync(ComponentId(clock))
2. No Async domain drives any register
3. (optional) Warn about unconstrained ports at top level
