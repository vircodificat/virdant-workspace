# Example Generation Summary

**Date**: October 15, 2024  
**Method**: Parallel subagent workflow  
**Total Examples**: 16 complete worked examples  
**Total Lines**: ~6,500 lines of detailed algorithm walkthroughs

---

## Examples Created

### Error Cases (5 examples)
- **005**: Mixing clocks without async (unification error)
- **006**: Async driving register (constraint violation) 
- **007**: All-async operands with non-async target (strict upper bound violation)
- **008**: When expression with mixed domains (unification error)
- **009**: Array indexing domain conflict (unification error)
- **012**: Struct construction with mixed domains (unification error)

### Legal Cases (6 examples)
- **001**: Simple single-clock module
- **002**: Constant literals
- **003**: Binary operator backward flow
- **004**: Mixing clocks with explicit async
- **011**: Multi-clock module with domain parameters
- **015**: Top-level unconstrained ports

### Advanced Scenarios (3 examples)
- **010**: Cross-module instantiation with domain substitution
- **013**: Unconstrained ports propagating to parent
- **016**: Pure constant module (no clocks at all)

### Special Cases (2 examples)
- **014**: The @unsafe_clocks Sync module
- **015**: Top-level unconstrained ports (also validates free variables)

---

## Key Design Issues Discovered

### 1. **CRITICAL: Literals should be fixed to Constant domain**

**Found in**: Example 016

**Issue**: ALGORITHM.md currently shows literals with "No constraints generated", making them free variables. But according to the lattice design where `Constant` is the bottom element, literals should have:

```python
fixed.append(Fixed(expr_node, Constant))
```

**Impact**: This affects how pure combinational expressions work. With literals as `Constant`:
- `wire x : Word[8] { it := 42 }` → domain `Constant` ✓
- `wire x : Word[8] { it := 2 + 3 }` → domain `Constant` (both operands constant, LUB is constant) ✓
- Pure expression evaluations don't require clock dependencies

**Action**: Update ALGORITHM.md walk_expression() literal case

---

### 2. **Child Port Components in Parent Constraint System**

**Found in**: Examples 010, 013

**Issue**: ALGORITHM.md doesn't explicitly state that when a child module is instantiated, each child port becomes a component in the parent's component set.

**Current behavior** (implicit):
- `inc.clock`, `inc.data`, `inc.result` are parent components
- They participate in parent's constraint solving
- Connection `it.data := input` generates `dom(inc.data) = dom(input)`
- Child's internal constraints (e.g., "inp and out same domain") become parent equality constraints

**Needed clarification**: Document that:
1. Child ports are parent components
2. Child equivalence classes become parent equality constraints
3. This enables natural polymorphism and free variable binding

---

### 3. **ComponentId Mapping Across Module Boundaries**

**Found in**: Example 010

**Issue**: When a child has `Sync(ComponentId(clock_child))` and parent binds it to `Sync(ComponentId(clock_parent))`, how is the mapping handled?

**Current approach** (from ALGORITHM.md):
```python
def substitute(sig_domain, bindings):
    if Var(domain_var) in bindings:
        return bindings[domain_var]
```

**Needed clarification**:
- Domain variables map clock ports to their `Sync` domains
- At instantiation, clock connection establishes binding
- Child's `Var(α)` becomes parent's `Sync(ComponentId(parent_clock))`
- ComponentIds are never directly compared across modules

---

### 4. **SignatureDomain Type Confirmed**

**Found in**: Examples 010, 013, 015, 016

**Resolution**: The `???` in PLANv2.md should be:

```rust
pub enum SignatureDomain {
    Concrete(Domain),  // Clock, Async, Reset, Constant
    Var(DomainVar),    // Reference to domain parameter
    Free,              // Unconstrained (will unify with anything at instantiation)
}
```

**Usage**:
- `Concrete(Clock)`: Clock ports
- `Concrete(Async)`: Explicitly async ports
- `Concrete(Constant)`: Ports proven to be constant (rare)
- `Var(DomainVar(n))`: Ports tied to clock parameter n
- `Free`: Unconstrained ports (PassThrough example)

---

### 5. **Upper Bound Constraint Resolution**

**Found in**: Example 007

**Issue**: Order of operations for constraint solving needs clarification.

**Confirmed algorithm**:
1. Process fixed constraints first
2. Process equality constraints (union-find merges)
3. Resolve upper bounds (compute LUB for unresolved classes)
4. Check strict upper bounds (verify `< Async` constraints)

**Key insight**: Upper bound violations are detected *after* full resolution, not during unification. This distinguishes them from unification errors.

---

### 6. **Free Variables vs. Constant Domain**

**Found in**: Examples 015, 016

**Distinction**:
- **Free variable**: Unconstrained port, can unify with anything at instantiation
  - Example: `PassThrough { inp, out }` where both are unconstrained
  - Signature: `Free()`
  
- **Constant domain**: Explicitly has no clock dependency
  - Example: `wire x { it := 42 }` with literal
  - Signature: `Concrete(Constant)`

**Important**: "Unconstrained" ≠ "Constant". Free variables are polymorphic; constant domain is concrete.

---

### 7. **Empty domain_params is Legal**

**Found in**: Examples 015, 016

**Clarification**: A module with no Clock-typed ports has:
```rust
DomainSignature {
    domain_params: {},  // Empty map is legal
    port_domains: { ... }
}
```

This represents modules that:
- Have no domain parameters (not polymorphic via clocks)
- May still be polymorphic via `Free()` ports
- May have all `Constant` domains (pure combinational)

---

### 8. **Bidirectional Domain Flow**

**Found in**: Example 013

**Insight**: Domains flow both directions through module boundaries:
- Parent → Child (input connections bind child domains)
- Child → Parent (output usage propagates child domains)

The constraint system handles this naturally through equality constraints. No special-case logic needed.

---

### 9. **Strict Upper Bounds and Free Variables**

**Found in**: Example 013

**Important property**: Free variables satisfy strict upper bounds.

- Constraint: `dom(port) < Async`
- At instantiation, free variables can be bound to: `Sync(...)`, `Clock`, `Reset`, `Constant`
- They cannot be bound to `Async` (which requires explicit annotation)
- This maintains the "async must be explicit" invariant

---

## Recommended Updates

### To ALGORITHM.md:

1. **Fix literal constraint generation** (walk_expression):
   ```python
   elif expr.kind == Literal:
       expr_node = expr_id
       expr_id += 1
       variables.add(expr_node)
       fixed.append(Fixed(expr_node, Constant))  # ← ADD THIS
       return (expr_id, expr_node)
   ```

2. **Document child port components**:
   Add section explaining that instantiated child ports become parent components.

3. **Clarify constraint solving order**:
   Explicitly document the four-step order shown in #5 above.

4. **Add note on empty domain_params**:
   State that empty `domain_params` map is legal for modules with no Clock ports.

### To PLANv2.md:

1. **Replace `???` with concrete SignatureDomain enum** (see #4 above)

2. **Add Constant domain example**:
   ```virdant
   wire x : Word[8] { it := 42 }  // domain Constant
   ```

3. **Clarify Free vs. Constant distinction** (see #6 above)

4. **Document ComponentId scoping**:
   State that ComponentIds are module-local and never compared across boundaries.

---

## Algorithm Validation

### Strengths Confirmed:

1. **Compositionality**: Module boundaries are clean. Child signatures compose naturally into parent constraints.

2. **No special cases**: Error detection, unconstrained ports, cross-module instantiation all work through the same core constraint mechanism.

3. **Lattice design**: The partial order with Constant at bottom and Async at top handles upcasting naturally.

4. **Union-find elegance**: Single data structure handles both equality constraints and domain propagation.

### Edge Cases Handled:

- ✅ All-async operands → non-async target (strict upper bound)
- ✅ Mixed clock domains → error detection (unification)
- ✅ Unconstrained ports → free variables (polymorphism)
- ✅ Pure combinational → Constant domain
- ✅ Cross-module → substitution and binding
- ✅ Multi-clock → multiple domain parameters
- ✅ @unsafe_clocks → signature synthesis

### Design is Ready for Implementation

All 16 examples validate that the three-pass algorithm:
1. Generates correct constraints for all language constructs
2. Solves constraints consistently
3. Extracts meaningful signatures
4. Catches errors at the right phase
5. Provides actionable error messages

The only fix needed is making literals have domain `Constant` instead of being unconstrained.

---

## Next Steps

1. Apply the literal constraint fix to ALGORITHM.md
2. Update PLANv2.md with SignatureDomain enum
3. Add clarifications from design issues #2, #3, #5, #7
4. Review all examples for any final questions
5. Begin implementation!
