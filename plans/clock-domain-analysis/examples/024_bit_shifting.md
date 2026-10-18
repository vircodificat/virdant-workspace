## Example 24: Bit Shifting Patterns with word() and Slicing

```virdant
mod BitShiftingExample {
    incoming clock : Clock
    incoming data : Word[8]

    reg buffer : Word[8] on clock {
        it <= data
    }

    // Left shift by 3: concatenate slice with zeros
    // word(buffer[5..0], 0w3) = buffer[4:0] << 3
    outgoing left_shift_3 : Word[8] {
        it := word(buffer[5..0], 0w3)
    }

    // Right shift by 2: concatenate zeros with slice
    // word(0w2, buffer[8..2]) = buffer[7:2] >> 2
    outgoing right_shift_2 : Word[8] {
        it := word(0w2, buffer[8..2])
    }

    // Shift with extraction: grab middle bits and shift left
    // word(buffer[6..2], 0w4) = buffer[5:2] << 4
    outgoing middle_shift : Word[8] {
        it := word(buffer[6..2], 0w4)
    }

    // Rotate-like pattern: swap halves
    // word(buffer[4..0], buffer[8..4])
    outgoing swap_halves : Word[8] {
        it := word(buffer[4..0], buffer[8..4])
    }
}
```

**What's happening**: This example demonstrates bit shifting patterns that combine slicing and word concatenation. Each shift operation involves:
1. Extracting a bit range from a register (preserves domain)
2. Concatenating with zero literals (Constant unifies with register's domain)
3. The result adopts the register's `Sync(clock)` domain

Key patterns:
- **Left shift**: `word(value[N-S..0], 0wS)` - slice high bits, pad with zeros
- **Right shift**: `word(0wS, value[N..S])` - pad with zeros, slice low bits  
- **Extract and shift**: `word(value[high..low], 0wS)` - slice middle, pad with zeros
- **Rotation pattern**: `word(value[N/2..0], value[N..N/2])` - swap bit ranges

---

### Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

**Step 1 - Components from ComponentAnalysis:**
- `clock` (incoming, type Clock)
- `data` (incoming, type Word[8])
- `buffer` (reg, type Word[8])
- `left_shift_3` (outgoing, type Word[8])
- `right_shift_2` (outgoing, type Word[8])
- `middle_shift` (outgoing, type Word[8])
- `swap_halves` (outgoing, type Word[8])

**Step 2 - Walk declarations, identify expressions:**

Declaration: `incoming clock : Clock`
- Component: `clock`
- No expressions

Declaration: `incoming data : Word[8]`
- Component: `data`
- No expressions

Declaration: `reg buffer : Word[8] on clock`
- Component: `buffer`
- Expression `#0`: reference to `clock` (in the `on clock` clause)

Declaration: `outgoing left_shift_3 : Word[8]`
- Component: `left_shift_3`
- No expressions in declaration

Declaration: `outgoing right_shift_2 : Word[8]`
- Component: `right_shift_2`
- No expressions in declaration

Declaration: `outgoing middle_shift : Word[8]`
- Component: `middle_shift`
- No expressions in declaration

Declaration: `outgoing swap_halves : Word[8]`
- Component: `swap_halves`
- No expressions in declaration

Driver: `buffer <= data`
- Expression `#1`: reference to `data` (RHS of driver)

Driver: `left_shift_3 := word(buffer[5..0], 0w3)`
- Expression `#2`: word concatenation `word(buffer[5..0], 0w3)`
  - Expression `#3`: bit slice `buffer[5..0]`
    - Expression `#4`: reference to `buffer` (source of slice)
  - Expression `#5`: literal `0w3`

Driver: `right_shift_2 := word(0w2, buffer[8..2])`
- Expression `#6`: word concatenation `word(0w2, buffer[8..2])`
  - Expression `#7`: literal `0w2`
  - Expression `#8`: bit slice `buffer[8..2]`
    - Expression `#9`: reference to `buffer` (source of slice)

Driver: `middle_shift := word(buffer[6..2], 0w4)`
- Expression `#10`: word concatenation `word(buffer[6..2], 0w4)`
  - Expression `#11`: bit slice `buffer[6..2]`
    - Expression `#12`: reference to `buffer` (source of slice)
  - Expression `#13`: literal `0w4`

Driver: `swap_halves := word(buffer[4..0], buffer[8..4])`
- Expression `#14`: word concatenation `word(buffer[4..0], buffer[8..4])`
  - Expression `#15`: bit slice `buffer[4..0]`
    - Expression `#16`: reference to `buffer` (source of slice)
  - Expression `#17`: bit slice `buffer[8..4]`
    - Expression `#18`: reference to `buffer` (source of slice)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `data`
- `#2`: ExprWord `word(buffer[5..0], 0w3)`
- `#3`: ExprBitSlice `buffer[5..0]`
- `#4`: ExprReference to `buffer`
- `#5`: ExprLiteral `0w3`
- `#6`: ExprWord `word(0w2, buffer[8..2])`
- `#7`: ExprLiteral `0w2`
- `#8`: ExprBitSlice `buffer[8..2]`
- `#9`: ExprReference to `buffer`
- `#10`: ExprWord `word(buffer[6..2], 0w4)`
- `#11`: ExprBitSlice `buffer[6..2]`
- `#12`: ExprReference to `buffer`
- `#13`: ExprLiteral `0w4`
- `#14`: ExprWord `word(buffer[4..0], buffer[8..4])`
- `#15`: ExprBitSlice `buffer[4..0]`
- `#16`: ExprReference to `buffer`
- `#17`: ExprBitSlice `buffer[8..4]`
- `#18`: ExprReference to `buffer`

**Step 3 - Generate constraints:**

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Fixed constraints from literals:**
- `dom(#5) = Constant` (literal `0w3`)
- `dom(#7) = Constant` (literal `0w2`)
- `dom(#13) = Constant` (literal `0w4`)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `data` → `dom(#1) = dom(data)`
- Expression `#4` references `buffer` → `dom(#4) = dom(buffer)`
- Expression `#9` references `buffer` → `dom(#9) = dom(buffer)`
- Expression `#12` references `buffer` → `dom(#12) = dom(buffer)`
- Expression `#16` references `buffer` → `dom(#16) = dom(buffer)`
- Expression `#18` references `buffer` → `dom(#18) = dom(buffer)`

**Equality constraints from bit slicing (projection rule):**

Bit slice `buffer[5..0]` (expression `#3`):
- Result has same domain as source
- Constraint: `dom(#3) = dom(#4)`

Bit slice `buffer[8..2]` (expression `#8`):
- Result has same domain as source
- Constraint: `dom(#8) = dom(#9)`

Bit slice `buffer[6..2]` (expression `#11`):
- Result has same domain as source
- Constraint: `dom(#11) = dom(#12)`

Bit slice `buffer[4..0]` (expression `#15`):
- Result has same domain as source
- Constraint: `dom(#15) = dom(#16)`

Bit slice `buffer[8..4]` (expression `#17`):
- Result has same domain as source
- Constraint: `dom(#17) = dom(#18)`

**Equality constraints from word() concatenation:**

Word concatenation `word(buffer[5..0], 0w3)` (expression `#2`):
- All operands must unify to the same domain, and result has that domain
- `dom(#3) = dom(#2)` (slice operand equals result)
- `dom(#5) = dom(#2)` (literal operand equals result)

Word concatenation `word(0w2, buffer[8..2])` (expression `#6`):
- All operands must unify to the same domain, and result has that domain
- `dom(#7) = dom(#6)` (literal operand equals result)
- `dom(#8) = dom(#6)` (slice operand equals result)

Word concatenation `word(buffer[6..2], 0w4)` (expression `#10`):
- All operands must unify to the same domain, and result has that domain
- `dom(#11) = dom(#10)` (slice operand equals result)
- `dom(#13) = dom(#10)` (literal operand equals result)

Word concatenation `word(buffer[4..0], buffer[8..4])` (expression `#14`):
- All operands must unify to the same domain, and result has that domain
- `dom(#15) = dom(#14)` (first slice operand equals result)
- `dom(#17) = dom(#14)` (second slice operand equals result)

**Equality constraints from drivers:**

Driver: `buffer <= data`
- LHS: component `buffer`
- RHS: expression `#1`
- Constraint: `dom(buffer) = dom(#1)`

Driver: `left_shift_3 := word(buffer[5..0], 0w3)`
- LHS: component `left_shift_3`
- RHS: expression `#2`
- Constraint: `dom(left_shift_3) = dom(#2)`

Driver: `right_shift_2 := word(0w2, buffer[8..2])`
- LHS: component `right_shift_2`
- RHS: expression `#6`
- Constraint: `dom(right_shift_2) = dom(#6)`

Driver: `middle_shift := word(buffer[6..2], 0w4)`
- LHS: component `middle_shift`
- RHS: expression `#10`
- Constraint: `dom(middle_shift) = dom(#10)`

Driver: `swap_halves := word(buffer[4..0], buffer[8..4])`
- LHS: component `swap_halves`
- RHS: expression `#14`
- Constraint: `dom(swap_halves) = dom(#14)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(data) < Async`
- `dom(buffer) < Async`
- `dom(left_shift_3) < Async`
- `dom(right_shift_2) < Async`
- `dom(middle_shift) < Async`
- `dom(swap_halves) < Async`

**Complete constraint system:**
```
Fixed:
  C₁: dom(clock) = Clock
  C₂: dom(buffer) = Sync(ComponentId(clock))
  C₃: dom(#5) = Constant
  C₄: dom(#7) = Constant
  C₅: dom(#13) = Constant

Strict upper bound (implicit):
  C₆: dom(clock) < Async
  C₇: dom(data) < Async
  C₈: dom(buffer) < Async
  C₉: dom(left_shift_3) < Async
  C₁₀: dom(right_shift_2) < Async
  C₁₁: dom(middle_shift) < Async
  C₁₂: dom(swap_halves) < Async

Equality (component references):
  C₁₃: dom(#0) = dom(clock)
  C₁₄: dom(#1) = dom(data)
  C₁₅: dom(#4) = dom(buffer)
  C₁₆: dom(#9) = dom(buffer)
  C₁₇: dom(#12) = dom(buffer)
  C₁₈: dom(#16) = dom(buffer)
  C₁₉: dom(#18) = dom(buffer)

Equality (bit slicing projection):
  C₂₀: dom(#3) = dom(#4)
  C₂₁: dom(#8) = dom(#9)
  C₂₂: dom(#11) = dom(#12)
  C₂₃: dom(#15) = dom(#16)
  C₂₄: dom(#17) = dom(#18)

Equality (word() unification):
  C₂₅: dom(#3) = dom(#2)
  C₂₆: dom(#5) = dom(#2)
  C₂₇: dom(#7) = dom(#6)
  C₂₈: dom(#8) = dom(#6)
  C₂₉: dom(#11) = dom(#10)
  C₃₀: dom(#13) = dom(#10)
  C₃₁: dom(#15) = dom(#14)
  C₃₂: dom(#17) = dom(#14)

Equality (drivers):
  C₃₃: dom(buffer) = dom(#1)
  C₃₄: dom(left_shift_3) = dom(#2)
  C₃₅: dom(right_shift_2) = dom(#6)
  C₃₆: dom(middle_shift) = dom(#10)
  C₃₇: dom(swap_halves) = dom(#14)
```

---

### Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

**Initial state:**
All components and expressions start in separate equivalence classes:
```
{clock}, {data}, {buffer}, {left_shift_3}, {right_shift_2}, {middle_shift}, {swap_halves},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9},
{#10}, {#11}, {#12}, {#13}, {#14}, {#15}, {#16}, {#17}, {#18}
```

**Step 1: Process fixed constraints (C₁-C₅)**

C₁: `dom(clock) = Clock`
```
{clock: Clock}, ...
```

C₂: `dom(buffer) = Sync(ComponentId(clock))`
```
{clock: Clock}, {data}, {buffer: Sync(clock)}, ...
```

C₃: `dom(#5) = Constant`
```
... {#5: Constant}, ...
```

C₄: `dom(#7) = Constant`
```
... {#7: Constant}, ...
```

C₅: `dom(#13) = Constant`
```
... {#13: Constant}, ...
```

**Step 2: Process equality constraints**

Let me show the key constraint flow for the first shift operation, then summarize the others.

**C₁₃: `dom(#0) = dom(clock)`**
Merge #0 with clock:
```
{clock, #0: Clock}, ...
```

**C₁₄: `dom(#1) = dom(data)`**
Merge #1 with data:
```
{data, #1}, ...
```

**C₁₅: `dom(#4) = dom(buffer)`**
Merge #4 with buffer:
```
{buffer, #4: Sync(clock)}, ...
```

**C₂₀: `dom(#3) = dom(#4)` (bit slice projection)**
Merge #3 with buffer's class:
```
{buffer, #3, #4: Sync(clock)}, ...
```

**C₂₅: `dom(#3) = dom(#2)` (word() unification)**
Merge #2 with buffer's class:
```
{buffer, #2, #3, #4: Sync(clock)}, {#5: Constant}, ...
```

**C₂₆: `dom(#5) = dom(#2)` (word() unification with literal)**
Merge literal #5 (Constant) with buffer's class:
- Class 1: `{buffer, #2, #3, #4: Sync(clock)}`
- Class 2: `{#5: Constant}`
- Unify: `Constant` unifies with `Sync(clock)` → result is `Sync(clock)`
```
{buffer, #2, #3, #4, #5: Sync(clock)}, ...
```

**C₃₃: `dom(buffer) = dom(#1)` (driver)**
Merge buffer's class with data's class:
```
{data, #1, buffer, #2, #3, #4, #5: Sync(clock)}, ...
```

**C₃₄: `dom(left_shift_3) = dom(#2)` (driver)**
Merge left_shift_3 with buffer's class:
```
{data, #1, buffer, #2, #3, #4, #5, left_shift_3: Sync(clock)}, ...
```

**Processing other shift operations:**

For `right_shift_2` (expressions #6, #7, #8, #9):
- C₁₆: Merge #9 with buffer's class
- C₂₁: Merge #8 with buffer's class (via #9)
- C₂₈: Merge #6 with buffer's class (via #8)
- C₂₇: Merge #7 (Constant) with buffer's class → unifies to Sync(clock)
- C₃₅: Merge right_shift_2 with buffer's class

For `middle_shift` (expressions #10, #11, #12, #13):
- C₁₇: Merge #12 with buffer's class
- C₂₂: Merge #11 with buffer's class (via #12)
- C₂₉: Merge #10 with buffer's class (via #11)
- C₃₀: Merge #13 (Constant) with buffer's class → unifies to Sync(clock)
- C₃₆: Merge middle_shift with buffer's class

For `swap_halves` (expressions #14, #15, #16, #17, #18):
- C₁₈: Merge #16 with buffer's class
- C₁₉: Merge #18 with buffer's class
- C₂₃: Merge #15 with buffer's class (via #16)
- C₂₄: Merge #17 with buffer's class (via #18)
- C₃₁: Merge #14 with buffer's class (via #15)
- C₃₂: Already in same class (via #17)
- C₃₇: Merge swap_halves with buffer's class

**Final partition:**
```
Class 1: {clock, #0} → Clock

Class 2: {data, #1, buffer, #2, #3, #4, #5, #6, #7, #8, #9, #10, #11, #12, #13,
          #14, #15, #16, #17, #18, left_shift_3, right_shift_2, middle_shift, swap_halves}
         → Sync(ComponentId(clock))
```

**Solution (domain assignment):**
```
Components:
  dom(clock) = Clock
  dom(data) = Sync(ComponentId(clock))
  dom(buffer) = Sync(ComponentId(clock))
  dom(left_shift_3) = Sync(ComponentId(clock))
  dom(right_shift_2) = Sync(ComponentId(clock))
  dom(middle_shift) = Sync(ComponentId(clock))
  dom(swap_halves) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1-#18) = Sync(ComponentId(clock))
    [including all component references, bit slices, word expressions, and literals]
```

**Verification:**
- All fixed constraints satisfied: ✓
  - `dom(clock) = Clock` ✓
  - `dom(buffer) = Sync(clock)` ✓
  - Literals (#5, #7, #13) started as `Constant`, unified to `Sync(clock)` ✓
- All strict upper bound constraints satisfied: ✓
  - All checked domains are either `Clock` or `Sync(clock)`, both satisfy `< Async` ✓
- All equality constraints satisfied: ✓
  - All variables in large class have domain `Sync(clock)` ✓
  - Bit slicing preserved domains through projection ✓
  - word() unification propagated domains through concatenation ✓
  - Literal `Constant` domains successfully unified with `Sync(clock)` ✓

**Key insight**: The composite operations work through constraint chaining:
1. **Bit slice** → equality constraint links slice to source register
2. **word() with slice and literal** → equality constraints link both to result
3. **Literal Constant** → unifies with Sync(clock) through lattice operation
4. **Result** → inherits Sync(clock) domain from register through the chain

Domain **flows bidirectionally** through the composite expressions:
- Forward: buffer → slice → word() → output
- Backward: output driver constraint → word() → slice → buffer → data input

---

### Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

**Identify domain parameters:**

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

**Express port domains in terms of parameters:**

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)` (clock port itself)
- `dom(data) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(left_shift_3) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(right_shift_2) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(middle_shift) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)
- `dom(swap_halves) = Sync(ComponentId(clock))` → `Var(α)` (tied to domain parameter)

**Signature:**
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        data: Var(α),
        left_shift_3: Var(α),
        right_shift_2: Var(α),
        middle_shift: Var(α),
        swap_halves: Var(α),
    }
}
```

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: BitShiftingExample,
    component_domains: {
        clock: Clock,
        data: Sync(ComponentId(clock)),
        buffer: Sync(ComponentId(clock)),
        left_shift_3: Sync(ComponentId(clock)),
        right_shift_2: Sync(ComponentId(clock)),
        middle_shift: Sync(ComponentId(clock)),
        swap_halves: Sync(ComponentId(clock)),
    },
    signature: DomainSignature {
        domain_params: { clock: DomainVar(0) },
        port_domains: {
            clock: Concrete(Clock),
            data: Var(DomainVar(0)),
            left_shift_3: Var(DomainVar(0)),
            right_shift_2: Var(DomainVar(0)),
            middle_shift: Var(DomainVar(0)),
            swap_halves: Var(DomainVar(0)),
        }
    },
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes, completely naturally**. This example demonstrates that **composite operations** (combining bit slicing and word concatenation) work perfectly through the constraint system with no special handling needed.

The algorithm already handles both pieces:
- **Bit slicing** (from Example 17): Generates equality constraint `dom(slice) = dom(source)`
- **word() concatenation** (from Example 18): Generates equality constraints `dom(operand_i) = dom(result)` for all operands

When combined in patterns like `word(buffer[5..0], 0w3)`:
1. Slice expression creates: `dom(buffer[5..0]) = dom(buffer)`
2. word() creates: `dom(buffer[5..0]) = dom(result)` and `dom(0w3) = dom(result)`
3. Union-find merges all three into one equivalence class
4. Fixed constraint on buffer provides concrete domain `Sync(clock)`
5. Literal's `Constant` domain unifies with `Sync(clock)`
6. Result: all expressions and outputs have `Sync(clock)` domain

**No algorithm modifications needed**. The constraint system naturally composes the two operations.

### Any surprises or edge cases?

**No surprises**, but several **important validations**:

1. **Constant unification works correctly**: Literals (`0w3`, `0w2`, `0w4`) start with domain `Constant` and successfully unify with `Sync(clock)` through the lattice's unify operation. This confirms the lattice design is correct.

2. **Multiple slices from same source unify naturally**: In `swap_halves`, we have `word(buffer[4..0], buffer[8..4])`. Both slices reference the same buffer, so they naturally share the same domain. The constraint system merges them correctly without confusion.

3. **Domain flow is bidirectional**: The register's fixed constraint flows forward through slices and word() to outputs, while driver constraints flow backward from outputs through word() and slices to inputs. Both directions work simultaneously through equality constraints.

4. **Slice indices don't participate in domain analysis**: The slice bounds (`[5..0]`, `[8..2]`, etc.) are constant expressions compiled at elaboration time. They don't generate domain variables or constraints. This is correct behavior.

**Edge case consideration**: What if we had a dynamic slice (hypothetical, not in current Virdant):
```virdant
wire shifted : Word[8] {
    it := word(buffer[index +: 5], 0w3)
}
```
In that case, `index` would need to be treated like an array index operand, requiring `dom(index) = dom(buffer)`. But since Virdant uses **compile-time constant slicing**, this doesn't apply to the current language.

### Does the constraint system handle this without special cases?

**Yes, completely without special cases**. The constraint system handles composite operations through:

1. **Compositional constraint generation**: Each sub-expression generates its own constraints, and they chain together naturally through shared expression IDs.

2. **Union-find transitivity**: Equality constraints are transitive. If A=B and B=C, union-find automatically merges A and C. This allows long constraint chains (register → slice → word() → output) to resolve correctly.

3. **Lattice unification**: When merging `Constant` literals with `Sync(clock)` expressions, the unify() operation correctly returns `Sync(clock)` per the lattice design.

4. **No order dependencies**: The constraint solving order doesn't matter. Whether we process bit slice constraints before word() constraints or vice versa, we get the same final partition.

**Validation of the algorithm's design**: This example proves that the algorithm naturally handles:
- ✅ Multiple operations in sequence (slice then concatenate)
- ✅ Multiple operands from the same source (swap_halves)
- ✅ Mixed Constant and Sync domains (literals with slices)
- ✅ Complex expression trees with shared sub-expressions

### Any clarifications needed for ALGORITHM.md or PLANv2.md?

**No clarifications needed**. This example validates that:

1. **ALGORITHM.md is complete**: The existing `BitSlice` and `Word` cases (which should be added per Examples 17 and 18) are sufficient to handle composite shift patterns.

2. **PLANv2.md domain model is sound**: The lattice with `Constant` at bottom correctly handles literal padding in shift operations.

3. **No special "shift operation" case needed**: Shifts are naturally expressed as compositions of existing operations. No new constraint generation rules needed.

**Documentation note**: It might be helpful to add this example (or one like it) to PLANv2.md to show that common hardware patterns like shifting work naturally through composition. But the algorithm itself needs no changes.

### Relationship to hardware shift operations

This example demonstrates how **Virdant's expression-oriented shift patterns** map to hardware domain constraints:

| Verilog Pattern | Virdant Pattern | Domain Constraint |
|----------------|-----------------|-------------------|
| `x << 3` | `word(x[5..0], 0w3)` | `dom(x) = dom(result)` |
| `x >> 2` | `word(0w2, x[8..2])` | `dom(x) = dom(result)` |
| rotate/swap | `word(x[4..0], x[8..4])` | `dom(x) = dom(result)` |

In all cases:
- The source register's domain propagates through slicing (projection preserves domain)
- Literal zero-padding uses `Constant` domain which unifies with the register's domain
- Result adopts the register's `Sync(clock)` domain

**Key insight**: Shift operations in hardware don't change clock domains. A shifted signal remains synchronized to the same clock as its source. The constraint system correctly captures this invariant.

### Integration with other examples

This example builds on:
- **Example 17** (bit slicing): Uses slice projection rule
- **Example 18** (word concatenation): Uses variadic unification rule
- **Example 2** (constant literals): Uses `Constant` domain and lattice unification

Together they show that Virdant's domain system handles realistic hardware patterns through simple compositional rules. No special cases for common operations like shifting, masking, field extraction, etc.

### Summary

Bit shifting patterns with word() and slicing:
- ✅ Work naturally with the constraint system through composition
- ✅ No special solver logic needed for composite operations
- ✅ Domain preservation through slice → word() → output chains
- ✅ Constant literals correctly unify with synchronous slices
- ✅ Bidirectional domain flow works correctly
- ✅ Multiple references to same source (swap_halves) handled correctly
- ✅ No algorithm gaps revealed
- ✅ Validates that realistic hardware shift patterns work through the existing constraint rules

**Recommendation**: No changes to ALGORITHM.md or PLANv2.md needed. This example validates the algorithm handles composite operations correctly without special cases. The existing bit slicing and word concatenation rules compose naturally to support shift patterns.
