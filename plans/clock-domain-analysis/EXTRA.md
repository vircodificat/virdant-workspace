# Extra Context and Design Notes

## Backward Constraint Flow Through Expression Trees

A critical insight: constraints flow **bidirectionally** through expression trees, not just bottom-up.

### Example Demonstrating Backward Flow

```virdant
mod Foo {
    incoming clock : Clock
    outgoing out : Word[8]
    incoming inp : Word[8]

    reg r : Word[8] on clock {
        it <= inp + 1
    }
}
```

**Constraint flow:**
1. `r on clock` (fixed from `reg r on clock` declaration)
2. Driver constraint: `(inp + 1) ≡ r`, so `(inp + 1) on clock`
3. Operator constraint: For `(a + b) on clock`, we need `a on clock` AND `b on clock`
4. Therefore: `inp on clock` AND `1 on clock`
5. But `1` is a constant: `1 const`
6. Constant weakening: `1 const` can weaken to `1 on clock` (Constant is bottom of lattice)
7. Final constraint on port: `inp on clock`

The key: the register's domain requirement **propagated backward** through the `+` operator to constrain the incoming port `inp`. We couldn't solve the input port's domain without analyzing the expression tree.

## Every Subexpression Needs a Domain During Solving

Because constraints flow bidirectionally:
- We can't skip intermediate expression nodes
- Each subexpression participates in ≡ constraints
- The expression tree structure matters

**However**: After solving, we only need to keep **component domains** in the final `DomainAnalysis`. Expression domains are intermediate computation, discarded once constraints are solved.

## Predicate Notation

The notation `e on clock` as a predicate is clearer than `dom(e) = Sync(ComponentId(clock))`.

**Predicates:**
- `e on c` — expression e has domain Sync(ComponentId(c))
- `e const` — expression e is a constant (domain Constant)
- `e async` — expression e has domain Async
- `e clock` — expression e has domain Clock
- `e reset` — expression e has domain Reset

This reads more naturally and maps cleanly to type-theory style inference rules.

## Type-Theory Style Derivation Trees

Domain constraints can be presented as inference rules:

```
inp on clock    1 const [weakens to: 1 on clock]
------------------------------------------------- [+]
           (inp + 1) on clock

           (inp + 1) on clock
         ---------------------- [driver]
              r on clock
```

This presentation makes the constraint flow explicit and shows how each construct contributes to domain inference.

## Implications for Implementation

1. **During constraint generation**: Walk expression trees recursively, creating domain variables for each node (including intermediate operator nodes)

2. **Component references**: A reference expression `x` doesn't create a new domain variable - it uses the component `x`'s domain directly

3. **Complex expressions**: Operators, struct construction, array indexing, etc. create intermediate domain variables that participate in constraints

4. **After solving**: The union-find has resolved all equivalence classes. We extract component domains for storage, discarding expression domain variables

5. **Error reporting**: When a constraint fails, we need to trace back through the expression tree to explain where the incompatibility arose. The intermediate expression domains (computed during solving but discarded from final result) are crucial for good error messages.

## Critical Realization: Every Expression Participates

**Initial misconception**: Component reference expressions (like `data` in `buffer <= data`) don't need domain variables - they just "directly use" the component's domain.

**Reality**: Every expression, including component references, is an AST node that gets a domain variable and participates in constraint solving.

### Why This Matters

```virdant
reg buffer : Word[8] on clock {
    it <= data
}
```

There are TWO distinct entities here:
1. **Component `data`**: An incoming port declared in ComponentAnalysis
2. **Expression `data`**: An ExprReference AST node in the driver's RHS

During constraint generation:
- Component `data` gets domain variable `dom(data)`
- Expression at AST node (call it `#1`) references component `data`
- Constraint generated: `#1 ≡ data` (expression tied to component)
- Driver constraint: `buffer ≡ #1` (LHS tied to RHS expression)

During constraint solving:
- Union-find operates on: `{clock}, {data}, {buffer}, {out}, {#0}, {#1}, {#2}`
- Expression nodes `#0`, `#1`, `#2` participate in merges
- Final equivalence classes contain both components AND expressions

After solving:
- We extract only component domains for storage in `DomainAnalysis`
- Expression domains were necessary for solving but are discarded

### The General Pattern

For ANY expression tree:
1. Each AST node (leaf or internal) gets a domain variable
2. Constraints tie nodes together (parent-child, operand-result, reference-component)
3. Union-find solves over ALL nodes (components + expressions)
4. Final result extracts only component domains

**Key insight**: The AST structure drives constraint generation. We don't skip intermediate nodes - every expression, no matter how simple, participates in the constraint system during solving.

## Critical Error: Mixing Declarative Constraints with Imperative Updates

### The Problem

Initial exposition mixed two paradigms:
1. **Declarative constraints** - mathematical predicates that must hold in a solution
2. **Imperative algorithm** - union-find that "updates" or "changes" values during solving

This led to notational contradictions. For example, with a literal `0w8`:
```
C₄: dom(#1) = Constant      (stated as constraint)
C₅: counter ≡ #1            (equality constraint)
C₂: dom(counter) = Sync(clock)  (fixed constraint)
```

During solving, we said "Constant updates to Sync(clock)" - but this treats constraints as mutable variables, not mathematical predicates. The result was:
```
dom(counter) = dom(#1) = Constant = Sync(clock)
```

This is a contradiction if these are all equalities!

### The Resolution: Fixed Constraints vs Lower Bounds

We need to distinguish:

**1. Fixed constraints: `dom(x) = D`**
- The solution MUST assign exactly domain D to x
- Examples: registers (`dom(counter) = Sync(clock)`), clock-typed signals (`dom(clock) = Clock`)
- Non-negotiable

**2. Upper bound constraints: `D ≤ dom(x)`**
- The solution must assign a domain at least as specific as D (D is an upper bound on generality)
- D can be satisfied by D itself or anything more specific in the lattice
- **Not used for literals**: Since Constant is bottom, `Constant ≤ dom(x)` is vacuously true for all x
- The mechanism exists for potential future use

**3. Equality constraints: `dom(x) = dom(y)`**
- x and y must have the same domain
- Generated from dataflow (drivers, operands, etc.)

**4. Free variables**
- Some expressions (like literals) have no constraints
- They start unconstrained and get solved through equalities with other variables
- Example: a literal `0w8` is a free variable that adopts whatever domain is needed by its context

### The Lattice and What `≤` Means

```
      Constant (bottom)
        /  |  \
   Sync(C1) Sync(C2) ...
        \  |  /
       Async (top)
```

Partial order `d₁ ≤ d₂` means "d₁ is at most as specific as d₂" (or "at least as general as"):
- `Constant ≤ Sync(c)` for any c (Constant is less specific/more general)
- `Sync(c) ≤ Async` for any c (Async is top)
- `Constant ≤ Async` (transitivity)
- `d ≤ d` (reflexive)

**Important**: `Sync(c) ≤ Async` exists in the lattice, but we do NOT use it for automatic subsumption (that would violate "async must be explicit"). Upper bound constraints `D ≤ dom(x)` are part of the constraint framework, even though we don't use them for literals (since `Constant ≤ dom(x)` is vacuous).

### How Literals Work

A literal like `0w8` has NO constraints. It's a free variable.

When solving:
```
E: dom(counter) = dom(0w8)
F: dom(counter) = Sync(clock)
```

From E and F: solution must have `dom(0w8) = Sync(clock)`

The literal started unconstrained and adopted `Sync(clock)` because it was constrained to equal `counter`. This is why literals "just work" in any clock domain - they're free variables that adopt whatever domain their context requires.

**Note on Constant**: "Constant" is metadata describing that an expression has no signal dependencies. It's not a constraint in the constraint system. A literal is eligible to take on any domain, which is represented by having no constraints, not by having a vacuous `Constant ≤ dom(literal)` inequality.

### Declarative vs Algorithmic

**Declarative view (constraints)**:
- Input: A set of constraints (fixed, lower bounds, equalities)
- Output: A domain assignment that satisfies all constraints (or report unsatisfiable)
- Constraints are mathematical predicates - they don't "execute" or "update"

**Algorithmic view (union-find)**:
- Implementation detail for finding a solution
- Union-find incrementally builds equivalence classes and propagates concrete domains
- The algorithm mutates internal state, but this is just a solving technique
- The constraints themselves are immutable mathematical statements

### Revised Notation

**In constraint generation:**
- `dom(x) = D` - fixed constraint
- `D ≤ dom(x)` - upper bound constraint (not used for literals)
- `dom(x) = dom(y)` - equality constraint
- Free variables - no constraint (expression can take any domain)

**In constraint solving:**
- Union-find merges equivalence classes
- When merging classes with concrete domains, check lattice compatibility
- Report error if incompatible (e.g., `Sync(C1)` vs `Sync(C2)` where C1 ≠ C2)

**In final solution:**
- `dom(x) = D` - the domain assigned to x in the solution
- All fixed constraints satisfied: if constraint was `dom(x) = D`, solution has `dom(x) = D`
- All upper bounds satisfied: if constraint was `D ≤ dom(x)`, solution has `D ≤ dom(x)`
- All equalities satisfied: if constraint was `dom(x) = dom(y)`, solution assigns the same domain to both
- Free variables assigned domains that satisfy all constraints they're involved in

### Key Takeaway

Literals are free variables with no constraints. They don't have "fixed domains" that get "updated", nor do they impose inequality constraints. The constraint system finds a domain assignment where free variables adopt whatever domains are needed to satisfy equality constraints.
