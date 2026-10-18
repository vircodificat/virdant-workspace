# Clock Domain Analysis and Checking — PLANv2

## Motivation

Virdant currently has clock-related machinery (the `Clock` type, `reg ... on clock` declarations, platform clock extraction), but no clock-domain analysis or cross-domain checking. This plan defines a clock domain inference and checking system to flag invalid clock-domain crossings as compile-time errors.

## Core Model

Every signal has an associated **domain**. A domain is one of:
- `Sync(ComponentId)` — synchronized to a specific clock component
- `Async` — not synchronized to any clock
- `Clock` — the signal is a clock itself
- `Reset` — the signal is a reset itself
- `Constant` — a pure expression with no signal references

### Domain Lattice

```
         Async (top)
        /  |  \
   Sync(C1) Sync(C2) Sync(C3) ...
        \  |  /
      Constant (bottom)
```

- **Constant** is the bottom: unifies with any `Sync(ComponentId)`, resulting in that `Sync`. Can also upcast to `Async`.
- **Any domain** can upcast to `Async` when the target is explicitly declared `async`.
- **`Sync(C1)` + `Sync(C2)`** where `C1 ≠ C2` → ERROR (incompatible domains)
- **`Clock`, `Reset`, `Async`** do not unify with each other or with any `Sync` domain.

### Key Principles

1. **Clock components define domains**: A clock component with `ComponentId(5)` defines domain `Sync(ComponentId(5))`
2. **`async` is declaration-site explicit, never inferred**: Signals with domain `Async` must be declared `async`. If all operands are `async` but the target is not declared `async`, that's an error.
3. **Registers** output signals on domain `Sync(clock)` where `clock` is from their `on` clause
4. **Registers must be driven by signals with matching domain**: No `async` signal may drive a register
5. **Each clock port creates a distinct domain**: Two different `Clock` ports in a module define different domains, even if they might be connected to the same physical clock signal at instantiation time

## Domain Rules

### Combinational Expressions

For combinational operations (`a | b`, `a + b`, etc.):

**Rule**: All operands must unify to the same domain, and the result has that domain.

**Exception**: When the target is explicitly declared `async`, operands can be anything — they automatically upcast.

Examples:
```virdant
// Both Sync(clk) → result is Sync(clk)
reg a : Bit on clk { ... }
reg b : Bit on clk { ... }
wire w : Bit {
    it := a | b  // ✓ both Sync(clk)
}

// Different clocks → ERROR
reg a : Bit on clk_a { ... }
reg b : Bit on clk_b { ... }
wire w : Bit {
    it := a | b  // ✗ ERROR: incompatible domains
}

// Different clocks but target is async → OK
wire w : Bit async {
    it := a | b  // ✓ both upcast to Async
}
```

### Control Flow Expressions

#### `match` Expressions

In a `match` expression:
```virdant
match subject {
    case pattern1 => result1
    case pattern2 => result2
}
```

**Rule**: The subject expression and all result expressions must have the same domain. Patterns are not expressions and do not participate in domain analysis.

#### `when` Expressions

In a `when` expression:
```virdant
when {
    case guard1 => result1
    case guard2 => result2
}
```

**Rule**: All guard expressions AND all result expressions must have the same domain (everything flows through the same combinational logic).

### Struct and Array Operations

#### Struct Construction

When constructing a struct:
```virdant
struct type Pair {
    x : Word[8]
    y : Word[8]
}

wire result : Pair {
    it := ${ x = a, y = b }
}
```

**Rule**: All field expressions (`a`, `b`) must have the same domain, and the resulting struct has that domain.

#### Struct Field Access

`struct.field` has the same domain as `struct`.

#### Array Indexing

For `array[index]`:
- The element has the same domain as `array`
- The `index` expression must also have that same domain

### Register Constraints

1. A register declared `reg name : Type on clock` has domain `Sync(ComponentId)` where `ComponentId` identifies `clock`
2. A register's driver expression domain must equal the register's domain
3. **No signal with domain `Async` may drive a register**

## Module-Level Domain Analysis

### Per-Module Analysis with Polymorphic Signatures

Domain analysis is **per-module**, not whole-program.

When analyzing a module in isolation:
1. Each port gets a **domain variable** (fresh for each clock port)
2. Constraints are generated from the module body
3. Solving produces a **polymorphic signature** for the module's interface

### Domain Substitution at Instantiation

At instantiation time:
```virdant
mod child of SomeModule {
    it.clk := parent_clock       // binds child's domain variable α → Sync(parent_ComponentId)
    it.data := parent_signal     // domain(parent_signal) must = α[substituted]
}
```

Domain substitution maps the child's polymorphic variables to concrete parent domains:
1. **Clock connections establish bindings**: `it.clock := parent_clock` creates a binding from the child's domain variable to the parent's concrete domain
2. **Data port connections apply substitution**: The child port's domain (after substitution) must unify with the parent signal's domain
3. **Free variables propagate**: Unconstrained child ports create constraints on parent signals

### Top-Level Modules

**Unconstrained ports at the top level are legal** — they retain free domain variables.

## The `Sync` Module (Clock Domain Crossing Primitive)

The `Sync` module is the primitive for crossing clock domains:

```virdant
@unsafe_clocks
mod Sync {
    incoming inp : Bit async         // explicitly async input
    incoming clock : Clock            // defines the destination domain
    outgoing reg out : Bit on clock   // synchronized output
}
```

Usage:
```virdant
mod sync of Sync {
    it.inp := some_clocked_signal    // auto-upcasts because inp is async
    it.clock := destination_clock
}
```

Key properties:
- `Sync.inp` is declared `async` → connecting any signal to it auto-upcasts (no error)
- `Sync.out` is a register clocked by `Sync.clock` → its domain is that clock
- This is the **only** legal way to drive a register from a signal not already on that clock

### Implementation Notes

**Current approach**: Mark `Sync` with `@unsafe_clocks` annotation to suppress domain checking inside the module body. The internal driver `out <= inp` would normally be an error (async driving a register), but `@unsafe_clocks` permits it.

**When instantiating Sync**: The compiler still performs full domain analysis using Sync's polymorphic signature extracted from its port declarations. This requires special handling in the domain analysis algorithm (see Algorithm section).

**Future work**: The morally correct solution is a `builtin mod` language construct — a module primitive whose internal implementation lives outside Virdant's domain system (analogous to how hardware primitives like D flip-flops exist in real HDLs).

## Domain Categories

The five domain categories:

| Domain | Meaning |
|--------|---------|
| `Sync(ComponentId)` | Synchronized to a specific clock |
| `Async` | Not synchronized to any clock |
| `Clock` | The signal *is* a clock |
| `Reset` | The signal *is* a reset |
| `Constant` | A pure expression with no signal references |

**`Clock` and `Reset` rationale**: Kept separate from `Async` for future refinement (e.g., tracking assert/deassert patterns on resets). For now, they behave like `Async` for unification purposes: they don't unify with `Sync` domains or drive registers.

**Scope**: `ComponentId` is local to each module's component analysis. At instantiation, child `ComponentId`s are mapped to parent domains through clock port connections.

## Domain Representation

```rust
enum Domain {
    Async,                // top of the lattice
    Sync(ComponentId),    // synchronized to a specific clock component
    Clock,                // Clock-typed signals (special, like async)
    Reset,                // Reset-typed signals (special, like async)
    Constant,             // bottom of the lattice - pure expressions
}
```

## Implementation Notes

### Query System Integration

Domain analysis should be implemented as a **memoized query** in the `Builder` database, following the pattern of existing analyses:

```rust
DomainAnalysis(symbol_id: SymbolId) -> Arc<DomainAnalysis>
```

Each module gets a `DomainAnalysis` computed from its AST, `ComponentAnalysis`, and child modules' `DomainAnalysis` (for substitution).

### Evaluation Order

Domain analysis runs **after** typechecking (to know which signals are `Clock`/`Reset` typed) and driver analysis (which establishes component graph structure):

```
Parsing → ComponentAnalysis → DriverAnalysis → TypeCheck → DomainAnalysis → Check()
```

### Per-Module Scope

Like `ComponentAnalysis`, domain analysis is per-module (keyed by `SymbolId`). The memoized query system handles recursive dependencies: requesting a parent's `DomainAnalysis` automatically triggers building children's first.

## Data Structures

### DomainAnalysis (Query Result)

```rust
pub struct DomainAnalysis {
    /// The module this analysis is for
    moddef: SymbolId,
    
    /// Resolved domain for each component (all components, not just ports)
    domains: IndexMap<ComponentId, Domain>,
    
    /// The module's external interface for instantiation
    signature: DomainSignature,
    
    /// Diagnostics from domain inference/checking
    diagnostics: Vec<Diagnostic>,
    
    /// Whether this module is marked @unsafe_clocks
    /// If true: domains and diagnostics are empty, signature is synthesized
    unsafe_clocks: bool,
}
```

### DomainSignature (External Interface)

```rust
pub struct DomainSignature {
    /// Clock ports that introduce domain variables
    /// Maps clock port SymbolId → the domain variable it introduces
    domain_params: IndexMap<SymbolId, DomainVar>,
    
    /// For each port, its domain (may reference domain variables)
    /// TODO: unclear what type the value should be
    port_domains: IndexMap<SymbolId, ???>,
}
```

### Domain (Domain Categories)

```rust
pub enum Domain {
    Async,
    Sync(ComponentId),
    Clock,
    Reset,
    Constant,
}
```

### DomainVar (Domain Variable for Polymorphism)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomainVar(u32);
```

Domain variables can be traced back to their originating clock via `DomainSignature.domain_params`.

## Algorithm Design

### Constraint Propagation Characteristics

Domain inference is fundamentally a **bidirectional equality constraint system**, not a directional dataflow analysis:

- **Registers establish concrete domains** (sink-constrained): `reg r : Bit on clk` fixes `domain(r) = Sync(ComponentId(clk))`. The driver must match, creating a backward constraint from sink to source.

- **Combinational logic creates equality constraints** (bidirectional): Operations like `a | b` require `domain(a) = domain(b) = domain(result)`, with no inherent direction.

- **Ports are initially unconstrained**, resolved through their connections and usage within the module body.

- **Registers with `on clock` clauses are the anchors** that establish concrete `Sync` domains. Everything else unifies through equality constraints.

This bidirectional nature motivates a constraint-generation + unification approach (analogous to Hindley-Milner type inference) rather than a directional dataflow analysis.

### Algorithm Structure (To Be Determined)

Key considerations:

- Constraint generation and solving (unified vs separate passes)
- Union-find for unification
- Special handling of `@unsafe_clocks` modules (Sync)
- Domain variable management for polymorphic signatures
- Cross-module substitution mechanics

## Syntax Extensions Required

Domain analysis requires adding the `async` keyword to Virdant:

1. **Component declaration modifier**:
   ```virdant
   incoming inp : Bit async
   wire result : Bit async { ... }
   ```

2. **Mutually exclusive with `on` clause**: `async on clk` is a parse error

3. **Token and grammar changes**:
   - Add `async` to `Token` enum and `KEYWORDS` list
   - Add `AsyncClause?` or `AsyncModifier?` to `ModDefStmtComponent` grammar production
   - Add `AsyncAnnotation` to `AstNodePayload` enum

## Error Conditions

**8.1** Attempting to unify incompatible domains:
- `Sync(C1)` with `Sync(C2)` where `C1 ≠ C2`
- `Async` with `Sync`, `Clock`, or `Reset`
- `Clock` with `Reset`, `Sync`, or `Async`
- `Reset` with `Clock`, `Sync`, or `Async`

**8.2** Operands in a combinational expression have different non-`Constant` domains, and the target is not declared `async`

**8.3** Driving a register with a signal of domain `Async`

**8.4** A register's inferred domain not matching its `on` clause

**8.5** In a `when` or `match` expression, if operands cannot unify to a single domain (and the result is not declared `async`)


## TODO
- What about sockets?
- Should `const` modifier act analogous to `async`, removing an implicit Constant > dom(p) constraint that lives on everything unless removed?
