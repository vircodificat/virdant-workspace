# Clock Domain Analysis - Handoff Document

## Current State

This plan defines a complete clock domain inference and checking system for Virdant. The design phase is **substantially complete**, with data structures, constraint system, and algorithm approach defined. Implementation is the next phase.

## What's Been Completed

### 1. Core Requirements (REQUIREMENTS.md)
- ✅ Defined domain assignment rules for all signals
- ✅ Specified domain sources (registers, async declarations, literals)
- ✅ Established domain constraints (fixed, equality, upper bounds)
- ✅ Defined unification rules and error conditions
- ✅ Module instantiation and domain substitution rules

### 2. High-Level Plan (PLANv2.md)
- ✅ Core domain model with 5 categories (Sync, Async, Clock, Reset, Constant)
- ✅ Domain rules for all language constructs (operators, match, when, struct, array)
- ✅ Per-module analysis with polymorphic signatures
- ✅ The `Sync` module design with `@unsafe_clocks` annotation
- ✅ Query system integration approach
- ✅ Syntax extensions required (`async` keyword)

### 3. Data Structures (PLANv2.md - Data Structures section)
```rust
// Main query result - APPROVED
pub struct DomainAnalysis {
    moddef: SymbolId,
    component_domains: IndexMap<ComponentId, Domain>,
    expression_domains: {},  // Computed during solving, not stored
    signature: DomainSignature,
    diagnostics: Vec<Diagnostic>,
    unsafe_clocks: bool,
}

// Module's external interface - APPROVED
pub struct DomainSignature {
    domain_params: IndexMap<SymbolId, DomainVar>,
    port_domains: IndexMap<SymbolId, ???>,  // TODO: determine value type
}

// Domain categories - APPROVED
pub enum Domain {
    Async,
    Sync(ComponentId),
    Clock,
    Reset,
    Constant,
}

// Domain variable for polymorphism - APPROVED
pub struct DomainVar(u32);
```

### 4. Algorithm Design (examples/PREFACE.md + EXTRA.md)

**Three-pass algorithm:**
1. **Constraint Generation**: Walk AST, create domain variables for all components and expressions, generate constraints
2. **Constraint Solving**: Union-find over equivalence classes, propagate concrete domains
3. **Signature Extraction**: Build polymorphic domain signature for module interface

**Constraint types:**
- Fixed: `dom(x) = D` (must be exactly D)
- Upper bound: `D ≤ dom(x)` (D is upper bound on generality)
- Strict upper bound: `dom(x) < D` (strictly less specific than D)
- Equality: `dom(x) = dom(y)` (must have same domain)
- Free variables: no constraints (e.g., literals)

**Key constraint:** ALL components have implicit `dom(c) < Async` unless declared `async` (the "async must be explicit" rule)

**Unified binary operator rule:** For `target := e1 op e2`:
- Always generate `dom(e1) ≤ dom(result)` and `dom(e2) ≤ dom(result)`
- No special-casing for async targets - the lattice handles it naturally

### 5. Worked Examples (examples/ directory)
- ✅ 001: Simple single-clock module
- ✅ 002: Constant literals (free variables)
- ✅ 003: Binary operators with backward constraint flow
- ✅ 004: Mixing clocks with explicit async (legal case)

Each example shows all three passes in detail with constraint systems, union-find states, and final results.

### 6. Design Insights Documented (EXTRA.md)
- ✅ Backward constraint flow through expression trees
- ✅ Every expression participates (no skipping intermediate nodes)
- ✅ Critical error: mixing declarative constraints with imperative updates (resolved)
- ✅ Literals are free variables, not `Constant ≤ dom(literal)` (that's vacuous)
- ✅ Upper bound constraints exist but aren't used for literals
- ✅ The implicit `dom(c) < Async` constraint unifies the operator rules

## Key TODOs

### 1. DomainSignature Value Type (CRITICAL)
**File**: PLANv2.md - Data Structures section

The `port_domains` value type is still marked `???`. Need to determine what representation works for:
- Concrete domains (Clock, Async, Reset)
- Domain variables (references to domain parameters)
- Potentially: domains with structure (though currently we don't need this)

**Current thinking**: Likely a simple enum like:
```rust
pub enum SignatureDomain {
    Concrete(Domain),  // Clock, Async, Reset, Constant
    Var(DomainVar),    // Reference to domain parameter
}
```

But this needs validation against cross-module instantiation scenarios.

### 2. Algorithm Details

**What exists:**
- High-level three-pass structure
- Constraint types and their meanings
- Union-find approach for solving
- Worked examples showing the process

**What's missing:**
- Precise union-find merge rules (when merging classes with different concrete domains)
- Handling of upper bound constraint checking (when and how)
- Strict upper bound checking (`<` vs `≤`)
- Error message generation strategy
- Cross-module substitution mechanics (concrete algorithm)

**Recommendation**: Walk through more complex examples (cross-module instantiation, error cases) to flesh out edge cases.

### 4. Syntax Extensions

**Defined:**
- `async` as a keyword/modifier on component declarations
- Mutually exclusive with `on` clause
- Grammar changes needed (add to Token enum, update ModDefStmtComponent)

**TODO:**
- Exact grammar production for `async` modifier
- AST node representation (`AsyncAnnotation` payload)
- Update GRAMMAR.txt after implementing
- Parser tests

### 5. Implementation Plan

**Not yet defined:**
- Build order (what depends on what)
- Integration points with existing analysis passes
- Testing strategy
- Migration path (can be introduced incrementally?)

## Important Files

### Design Documents
- **REQUIREMENTS.md**: Formal requirements (domain assignment, constraints, errors)
- **PLANv2.md**: High-level plan, approved design decisions, data structures
- **EXTRA.md**: Design evolution, insights, error corrections, detailed notes

### Obsolete/Historical
- **PLAN.md**: Original plan (pre-approval, Q&A format) - kept for reference
- **THREE_PASS_EXAMPLES.md**: Original examples file - superseded by split examples
- **THREE_PASS_EXAMPLESv2.md**: Second iteration - superseded by examples/ directory

## Next Steps

### Immediate (Before Implementation)

1. **Resolve DomainSignature value type**
   - Design the signature domain representation
   - Validate against cross-module scenarios
   - Update PLANv2.md data structures section

2. **Write error case examples**
   - Async driving register (constraint violation)
   - All-async operands, non-async target (explicit async requirement)
   - when/match with mixed domains

3. **Write cross-module instantiation example**
   - Shows domain substitution in practice
   - Child module with polymorphic signature
   - Parent connecting and substituting domain variables
   - Validates the signature representation

4. **Finalize algorithm details**
   - Union-find merge rules precisely defined
   - Upper bound constraint checking algorithm
   - Error message generation strategy
   - Verification pass details

### Before Coding

5. **Write implementation plan**
   - File structure (where does code go?)
   - Build order (queries, analyses, checks)
   - Integration points with existing codebase
   - Testing strategy

6. **Grammar changes specification**
   - Exact lalrpop productions
   - Token changes
   - AST payload changes

### During Implementation

7. **Incremental approach**
   - Start with single-module analysis
   - Add cross-module later
   - Test with examples at each step
