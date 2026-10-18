# Clock Domain Analysis and Checking

## Motivation

Virdant currently has clock-related machinery (the `Clock` type, `reg ... on clock` declarations, platform clock extraction), but no clock-domain analysis or cross-domain checking. This plan defines a clock domain inference and checking system to flag invalid clock-domain crossings as compile-time errors.

## Core Model

Every signal has an associated **domain**. A domain is one of:
- `Sync(ComponentId)` — synchronized to a specific clock component
- `Async` — not synchronized to any clock
- `Clock` — the signal is a clock itself
- `Reset` — the signal is a reset itself  
- `Constant` — a pure expression with no signal references

The domain system has these primitives:

1. **Clock components define domains**: A clock component with `ComponentId(5)` defines domain `Sync(ComponentId(5))`
2. **`async` is explicit**: Signals with domain `Async` must be declared `async`
3. **Registers** output signals on domain `Sync(clock)` where `clock` is from their `on` clause
4. **Registers must be driven by signals with matching domain**
5. **`Sync` module** is the primitive for crossing domains or bringing `async` signals into a clock domain

Key principle: `async` is **declaration-site explicit**, never inferred. This ensures clock boundaries are visible in the code.

## Inference Rules

### Question 1: Domain propagation for combinational expressions

**Q:** When a combinational signal depends on multiple other signals, how does the clock domain propagate?

Example:
```virdant
reg a : Bit on clock { ... }
reg b : Bit on clock { ... }

wire w : Bit {
    it := a | b
}
```

And both `reg_a` and `reg_b` are on `clock`, does `w` inherit `clock`?

**A:** Yes. In this example, `w` is inferred to be on `clock`.

### Clarification: `async` behavior

**Correction:** `async` is the absence of a synchronized domain. Any signal may be freely upcast to `async`. Any two `async` signals can be combinatorially combined to create an `async` output. No `async` signal can ever be used as input to a register.

### Question 2: Mixing clocked and async operands

**Q:** If `a` is on `clk_a` and `b` is `async`, what domain does `w` infer?

```virdant
reg a : Bit on clk_a { ... }
wire b : Bit async { ... }

wire w : Bit {
    it := a | b
}
```

Does `w` infer as `clk_a` (async operands don't constrain the result), or is this an error?

**A:** This is an error, since mixing `async` with `clk_a` would lead to metastability.

### Question 3: Explicit async target allows mixing

**Q:** What about when the output wire is explicitly declared `async`?

```virdant
reg a : Bit on clk_a { ... }
wire b : Bit async { ... }

wire w : Bit async {
    it := a | b
}
```

Is this legal (because `w` is declared `async`, so the metastability is acknowledged/allowed), or is it still an error?

**A:** This is legal. The `a` reference automatically upcasts.

### Question 4: Mixing two different clocks

**Q:** What about mixing signals from two different `Sync` domains?

```virdant
reg a : Bit on clk_a { ... }  // domain Sync(ComponentId(clk_a))
reg b : Bit on clk_b { ... }  // domain Sync(ComponentId(clk_b))

wire w : Bit {
    it := a | b
}
```

Is this an error, or legal if you explicitly declare `wire w : Bit async`?

**A:** This is obviously illegal unless you explicitly declare `w` async.

### Question 5: All-async operands with non-async target

**Q:** If all operands are `async`, but the target is NOT declared `async`, what happens?

```virdant
wire a : Bit async { ... }
wire b : Bit async { ... }

wire w : Bit {
    it := a | b
}
```

Is this an error (must declare `wire w : Bit async`), or legal and `w` infers as `async`?

**A:** This should be an error. The reason is that `async` signals need to be explicit. (This should help with readability.)

## Summary of Rules

- **Target is `async`**: all operands upcast, anything goes
- **Target is not `async`**: all operands must share the same domain (after unification)
- **Operands all `async`** but target not declared `async` → error
- **`async` is always explicit, never inferred**
- **`Constant` domain** unifies with any `Sync` domain

## The `Sync` Module

**Q:** How does `Sync` fit into this checker?

The `Sync` module has:
- `inp: Bit` (takes async or foreign-domain input)
- `clock: Clock` (destination domain)
- `out: reg Bit on clock` (synchronized output)

Is `Sync.out` treated as being on whatever clock you connected to `Sync.clock`? And does connecting to `Sync.inp` require the input to be explicitly `async`, or can you connect a clocked signal directly?

**A:** Yes. `Sync.out` is clocked with `Sync.clock` and `Sync.inp` is `async` explicitly.

When you write:
```virdant
mod sync of Sync {
    it.inp := some_clocked_signal  // upcasts automatically
    it.clock := clock
}
```

The upcast happens implicitly at the assignment because `Sync.inp` is declared `async`.

## Port Domain Inference

### Question 6: Port domain handling

**Q:** How do `incoming` and `outgoing` ports get their clock domains?

Example:
```virdant
mod Top {
    incoming clock : Clock
    incoming data  : Word[8]
    outgoing result : Word[8]
    
    // ...
}
```

Is `data` implicitly `async` (since it's coming from outside)? Or does it have no domain until explicitly declared? And what about `result` - does it need an explicit clock declaration, or does it infer from what drives it?

**A:** They should be inferred unless explicitly marked async: `incoming rx : Bit async`. If a port cannot be solved for (locally), it's flagged as an error.

### Question 7: Cross-module domain inference

**Q:** How does clock domain inference work across module instantiations?

Example:
```virdant
mod SubModule {
    incoming data : Word[8]  // inferred
    outgoing result : Word[8]  // inferred
    // ... implementation
}

mod Top {
    incoming clock : Clock
    reg input_reg : Word[8] on clock { ... }
    
    mod sub of SubModule {
        it.data := input_reg  // connects clk-domain signal to inferred port
    }
}
```

Does the connection `it.data := input_reg` constrain `SubModule.data` to be on `clock`? And then does that constraint propagate through `SubModule`'s body to constrain its internal signals and `result` port?

Are clock domains inferred:
- Per-module in isolation?
- Whole-program?
- Something hybrid?

**A:** It should be per-module. You will need to know the domain analysis of all submodules, obviously.

### Question 8: Per-module domain signatures

**Q:** When analyzing a module in isolation, how are port domains determined?

Example:
```virdant
mod SubModule {
    incoming a : Bit
    incoming b : Bit
    outgoing c : Bit
    
    wire internal : Bit {
        it := a
    }
    
    outgoing c : Bit {
        it := internal
    }
}
```

Does this analysis conclude:
- `a` and `c` must be on the same clock (due to the dataflow), but `b` is unconstrained?
- All three ports get assigned domain variables like `'clk_a`, `'clk_b`, `'clk_c` with constraint `'clk_a = 'clk_c`?

And when instantiating `SubModule`, what happens if you only constrain some of its ports?

**A:** In this example, `dom(a) = dom(c)` and `dom(b)` is unconstrained. When instantiating a submodule, if you leave things unconstrained, that unconstraint will propagate up in the obvious way.

### Polymorphic Domains

Domains are **polymorphic parameters** at module interfaces. When a module is analyzed in isolation, it produces a signature with domain variables (e.g., `α`, `β`) for each clock. At instantiation time, connecting clocks performs domain substitution/unification.

Example:
```virdant
mod DualClock {
    incoming clk_a : Clock       // defines domain variable α
    incoming clk_b : Clock       // defines domain variable β
    incoming data_a : Bit        // domain α = Sync(clk_a)
    incoming data_b : Bit        // domain β = Sync(clk_b)
    outgoing mixed : Bit async {
        it := data_a | data_b    // legal, both upcast to async
    }
}

mod Top {
    incoming clk : Clock         // ComponentId = 0 in Top
    reg r1 : Bit on clk { ... }  // domain Sync(ComponentId(0))
    reg r2 : Bit on clk { ... }  // domain Sync(ComponentId(0))
    
    mod dual of DualClock {
        it.clk_a := clk          // binds α → Sync(ComponentId(0)) from Top
        it.clk_b := clk          // binds β → Sync(ComponentId(0)) from Top
        it.data_a := r1          // check: domain(r1) = α[substituted] = Sync(ComponentId(0)) ✓
        it.data_b := r2          // check: domain(r2) = β[substituted] = Sync(ComponentId(0)) ✓
    }
}
```

Note that `α` and `β` are distinct during `DualClock`'s analysis (they reference different clock ports), but at instantiation they both get bound to `Sync(ComponentId(0))` when both are connected to the same parent clock.

Unconstrained ports get free domain variables that propagate to the parent's constraints.

### Question 9: Top-level unconstrained ports

**Q:** What happens to unconstrained ports at the top-level module?

Example:
```virdant
mod Top {
    incoming a : Bit
    incoming b : Bit
    outgoing c : Bit {
        it := a
    }
    // b is never used
}
```

Here `a` and `c` are constrained to be the same domain, but `b` is unconstrained. Is this an error, or is `b` implicitly `async`?

**A:** Initially suggested error, but relaxed: free domain variables are allowed at the top level.

Unconstrained ports at the top level retain their free domain variables. This is legal.

## Clock and Reset Signal Domains

**Q:** How should `Clock` and `Reset` typed signals themselves be handled in the domain system?

Example:
```virdant
mod Top {
    incoming clock : Clock
    incoming reset : Reset
    
    reg counter : Word[8] on clock { ... }
}
```

Should `clock` and `reset` signals:
- Have no clock domain (they're "outside" the domain system)?
- Be implicitly `async`?
- Be treated specially in some other way?

And can you do combinational logic on them (like `clk1 & clk2`)?

**A:** Give them a special designation (separate from regular domains and `async`), but functionally equivalent to `async` in every way for now.

### Domain Categories Summary

The five domain categories:
1. **`Sync(ComponentId)`** — synchronized to a specific clock component
2. **`Async`** — absence of a synchronized domain
3. **`Clock`** — clock signals (special category, functionally like `async`)
4. **`Reset`** — reset signals (special category, functionally like `async`)  
5. **`Constant`** — pure expressions (can unify with any `Sync` domain)

Rationale for `Clock`/`Reset` as special:
- `Clock` signals define domains, they're not *in* domains (avoiding circularity)
- `Reset` signals are control signals that commonly cross domains
- Keeping them separate from `async` leaves room for future refinement
- The type system already distinguishes them (`Clock`, `Reset` vs. `Bit`/`Word`)

### Clock Components Define Distinct Domains

Each incoming `Clock` port creates a distinct domain identified by its `ComponentId`. For example:

```virdant
mod Top {
    incoming clk_a : Clock            // ComponentId(0) in Top
    incoming clk_b : Clock            // ComponentId(1) in Top
    
    reg r1 : Bit on clk_a { ... }    // domain = Sync(ComponentId(0))
    reg r2 : Bit on clk_b { ... }    // domain = Sync(ComponentId(1))
}
```

Two different `Clock` ports always create different domains, even if they might be connected to the same physical clock signal.

## Control Flow Expressions

### Match Expressions

`match` expressions destructure a subject using patterns:

```virdant
match subject {
    case pattern1 => result1
    case pattern2 => result2
}
```

**Domain rule**: The subject expression and all result expressions must have the same domain. Patterns are not expressions and do not participate in domain analysis.

Example:
```virdant
incoming sel : Word[2]
reg r1 : Bit on clk { ... }  // domain Sync(ComponentId(clk))
reg r2 : Bit on clk { ... }  // domain Sync(ComponentId(clk))

wire result : Bit {  // infers domain Sync(ComponentId(clk))
    it := match sel {
        case 2'd0 => r1
        case 2'd1 => r2
        else => 1'b0  // constant unifies with Sync(ComponentId(clk))
    }
}
```

### When Expressions

`when` expressions are essentially mux chains with guard conditions:

```virdant
when {
    case guard1 => result1
    case guard2 => result2
    else => default_result
}
```

**Domain rule**: All guard expressions AND all result expressions must have the same domain. Since this compiles to a combinational mux chain, every signal involved flows through the same logic and must be synchronized.

Example:
```virdant
reg enable : Bit on clk { ... }   // domain Sync(ComponentId(clk))
reg data : Bit on clk { ... }     // domain Sync(ComponentId(clk))

wire output : Bit {  // infers domain Sync(ComponentId(clk))
    it := when {
        case enable => data   // both guard and result must be on clk
        else => 1'b0          // constant unifies with Sync(ComponentId(clk))
    }
}
```

## Implementation Notes

This should be implemented as a **separate clock-domain inference pass** at the component analysis level (per-module, before elaboration):

1. Type checking establishes value types
2. Component analysis builds per-module component graphs
3. **Clock domain inference** (new pass, per-module):
   - Assigns domain variables to all signals
   - Generates equality constraints from dataflow
   - Solves constraints per-module
   - Checks that all register drivers match the register's clock
   - Produces polymorphic domain signatures for module ports
4. Elaboration uses the domain information (potentially)

## Algorithm: Two-Phase HM-style Inference

Follows Hindley-Milner style constraint generation and solving, chosen for **clarity and maintainability**.

### Phase 1: Constraint Generation

Walk the module body and generate domain constraints:

1. **Initialize domain variables**:
   - Each signal gets a fresh domain variable
   - Constants (literals, pure expressions with no signal refs) get `Domain::Constant`
   - `Clock`-typed signals get `Domain::Clock`
   - `Reset`-typed signals get `Domain::Reset`
   - Signals explicitly marked `async` get `Domain::Async`

2. **Generate equality constraints** from:
   - **Wire drivers**: `domain(wire) = domain(expression)`
   - **Register drivers**: `domain(register) = domain(expression)` (register domain comes from its `on clock` clause)
   - **Combinational operators**: `domain(result) = domain(operand₁) = domain(operand₂) = ...`
   - **Struct construction**: `domain(struct) = domain(field₁) = domain(field₂) = ...` (all fields must match)
   - **Array indexing**: `domain(element) = domain(array)`
   - **Match expressions**: `domain(result) = domain(subject) = domain(result₁) = domain(result₂) = ...` (patterns are not expressions, don't participate)
   - **When expressions**: `domain(result) = domain(guard₁) = domain(guard₂) = ... = domain(result₁) = domain(result₂) = ...` (all guards and results must match)
   - **Port connections** (during instantiation): `domain(parent_signal) = domain(child_port)[substituted]`

3. **Handle async upcasts**:
   - When target is explicitly `async`, don't generate equality constraints
   - Just verify source expression is well-formed (no internal domain mixing)

### Phase 2: Constraint Solving via Unification

Solve constraints using union-find:

1. **Initialize union-find** with one set per domain variable

2. **Process each equality constraint** `domain(X) = domain(Y)`:
   - Find representatives: `repr_x = find(X)`, `repr_y = find(Y)`
   - If `repr_x == repr_y`, already unified, continue
   - Otherwise, attempt merge:
     - `Constant + Constant → Constant`
     - `Constant + Sync(D) → Sync(D)` (constant is bottom)
     - `Sync(D) + Constant → Sync(D)` (constant is bottom)
     - `Sync(D₁) + Sync(D₁) → Sync(D₁)` (same domain)
     - `Sync(D₁) + Sync(D₂)` where `D₁ ≠ D₂` → **ERROR: incompatible domains**
     - Mixing `Clock`, `Reset`, `Async`, or `Sync` inappropriately → **ERROR**

3. **Resolve domain variables** to representatives

4. **Check register constraints**: For each `reg x on clock`, verify `domain(x) = Sync(ComponentId(clock))`

5. **Extract module signature**: For each port, its domain (may be a free variable for polymorphism)

### Cross-Module Instantiation

When instantiating a child module, domain substitution maps the child's polymorphic domains to concrete parent domains.

**Domain substitution process:**

1. **Clock connections establish bindings**: Each connection `child.clock_port := parent_clock` creates a binding from the child's domain variable to the parent's concrete domain.
   - If the child module has `incoming clk : Clock` with local `ComponentId(n)`, this defines domain variable `α = Sync(ComponentId(n))` in the child's signature
   - When the parent connects `it.clk := parent_clock` where `parent_clock` has `ComponentId(m)`, this creates binding `α → Sync(ComponentId(m))`

2. **Data port connections apply substitution**: For each connection `child.port := parent_signal`:
   - Look up the child port's domain from the module signature (may contain domain variables like `α`)
   - Apply all clock bindings: replace `α` with `Sync(ComponentId(m))`, etc.
   - Unify `domain(parent_signal)` with the substituted child port domain
   - If unification fails, report an error

3. **Free variables propagate**: Child ports with unconstrained domains (not tied to any clock) create fresh domain variables in the parent's constraint system

**Example:**
```virdant
mod Child {
    incoming clk : Clock           // ComponentId(0) in Child, domain variable α
    incoming data : Word[8]        // signature: domain α
    outgoing result : Word[8] {    // signature: domain α (from constraint solving)
        it := data
    }
}

mod Parent {
    incoming parent_clk : Clock    // ComponentId(5) in Parent
    reg input : Word[8] on parent_clk { ... }  // domain Sync(ComponentId(5))
    
    mod child of Child {
        it.clk := parent_clk       // binds α → Sync(ComponentId(5))
        it.data := input           // check: domain(input) = α[substituted]
                                   //        Sync(ComponentId(5)) = Sync(ComponentId(5)) ✓
    }
}
```

### Domain Representation

```rust
enum Domain {
    Async,                // top of the lattice - anything can upcast here
    Sync(ComponentId),    // synchronized to a specific clock component
    Clock,                // Clock-typed signals (special, like async)
    Reset,                // Reset-typed signals (special, like async)
    Constant,             // bottom of the lattice - pure expressions, no signal refs
}
```

**Domain Lattice:**
```
         Async (top)
        /  |  \
   Sync(C1) Sync(C2) Sync(C3) ...
        \  |  /
      Constant (bottom)
```

- **Constant** can unify with any `Sync(ComponentId)` (literals, pure expressions)
- Any domain can **upcast** to `Async` (when target is explicitly `async`)
- `Clock` and `Reset` are special categories, functionally like `Async`

**Scope:**
- **Per-module scope**: `ComponentId` is local to each module's component analysis
- **Clock components define domains**: An `incoming clk : Clock` with `ComponentId(5)` defines `Sync(ComponentId(5))`
- **Cross-module**: At instantiation, child `ComponentId`s are mapped to parent domains through clock port connections
