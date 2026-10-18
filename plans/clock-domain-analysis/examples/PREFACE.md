# Three-Pass Algorithm Examples

This directory contains examples of every situation the domain analysis algorithm encounters, showing the data structures built during each pass.

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
- `Sync(c) ≤ Async` for any c (used for upcasting in operators)
- `d ≤ d` (reflexive)

**Important**: The lattice exists and is used for constraint checking (upper bounds). The "async must be explicit" principle is enforced through the implicit `dom(c) < Async` constraint on all non-async components.

**Key insights**:
- We're solving a constraint satisfaction problem with fixed, upper bound, strict upper bound, and equality constraints
- Constraints are declarative mathematical predicates
- The union-find algorithm is an implementation detail for finding a solution
- Literals are free variables that adopt whatever domain is needed
- ALL components have implicit `dom(c) < Async` unless declared `async` (the "async must be explicit" rule)
- Binary operators use a unified rule: operand domains can upcast to result domain (`dom(operand) ≤ dom(result)`)

**Critical realization**: Every expression is an AST node that participates in constraint solving. Component references create expression nodes that are constrained to equal the component they reference.

## Examples

- [001_simple_single_clock.md](001_simple_single_clock.md) - Basic single-clock module
- [002_constant_literal.md](002_constant_literal.md) - Literals as free variables
- [003_binary_operator_backward_flow.md](003_binary_operator_backward_flow.md) - Binary operators and backward constraint flow
- [004_async_mixing_legal.md](004_async_mixing_legal.md) - Mixing clocks with explicit async (legal)
