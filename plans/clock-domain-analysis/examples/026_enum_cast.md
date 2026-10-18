# Example 26: Enum cast() Domain Preservation

```virdant
enum type Opcode : Word[4] {
    Add = 0
    Sub = 1
    Mul = 2
    Div = 3
}

mod CastExample {
    incoming clock : Clock
    incoming raw_bits : Word[4]
    
    reg opcode_reg : Opcode on clock {
        it <= cast(raw_bits)  // Word[4] -> Opcode
    }
    
    wire opcode_wire : Opcode {
        it := cast(raw_bits)  // Word[4] -> Opcode
    }
    
    wire back_to_bits : Word[4] {
        it := cast(opcode_wire)  // Opcode -> Word[4]
    }
}
```

**What's happening**: The `cast()` function converts between enum types and their underlying `Word[n]` types. It reinterprets the bit pattern without changing values. As a **unary operation**, `cast()` should preserve the domain of its operand:
- `cast(expr)` has the same domain as `expr`

This example demonstrates:
1. `cast(raw_bits)` converting Word[4] → Opcode preserves `raw_bits`'s domain
2. `cast(opcode_wire)` converting Opcode → Word[4] preserves `opcode_wire`'s domain
3. Domain constraints flow through cast() transparently

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `raw_bits` (incoming, type Word[4])
- `opcode_reg` (reg, type Opcode)
- `opcode_wire` (wire, type Opcode)
- `back_to_bits` (wire, type Word[4])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions

**Declaration: `incoming raw_bits : Word[4]`**
- Component: `raw_bits`
- Type: Word[4]
- No expressions in declaration

**Declaration: `reg opcode_reg : Opcode on clock`**
- Component: `opcode_reg`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `opcode_reg <= cast(raw_bits)`**

The cast() is a unary operation:
- Expression `#1`: reference to `raw_bits` (operand of cast)
- Expression `#2`: `cast(#1)` (the cast operation itself)

**Declaration: `wire opcode_wire : Opcode`**
- Component: `opcode_wire`
- No `async` modifier
- No expressions in declaration

**Driver: `opcode_wire := cast(raw_bits)`**

The cast() is a unary operation:
- Expression `#3`: reference to `raw_bits` (operand of cast)
- Expression `#4`: `cast(#3)` (the cast operation itself)

**Declaration: `wire back_to_bits : Word[4]`**
- Component: `back_to_bits`
- No expressions in declaration

**Driver: `back_to_bits := cast(opcode_wire)`**

The cast() is a unary operation:
- Expression `#5`: reference to `opcode_wire` (operand of cast)
- Expression `#6`: `cast(#5)` (the cast operation itself)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `raw_bits` (cast operand)
- `#2`: ExprCast of `#1` (Word[4] → Opcode)
- `#3`: ExprReference to `raw_bits` (cast operand)
- `#4`: ExprCast of `#3` (Word[4] → Opcode)
- `#5`: ExprReference to `opcode_wire` (cast operand)
- `#6`: ExprCast of `#5` (Opcode → Word[4])

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from register declarations:**
- `dom(opcode_reg) = Sync(ComponentId(clock))`

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#1) = dom(raw_bits)` (reference in first cast)
- `dom(#3) = dom(raw_bits)` (reference in second cast)
- `dom(#5) = dom(opcode_wire)` (reference in third cast)

**Equality constraints from cast() unary operations:**

For `cast()` as a unary operation that preserves domain:
- **Constraint rule**: `dom(cast_result) = dom(operand)`

Applied to our expressions:
- `dom(#2) = dom(#1)` (cast result equals operand)
- `dom(#4) = dom(#3)` (cast result equals operand)
- `dom(#6) = dom(#5)` (cast result equals operand)

**Equality constraints from drivers:**
- `dom(opcode_reg) = dom(#2)` (register driven by cast expression)
- `dom(opcode_wire) = dom(#4)` (wire driven by cast expression)
- `dom(back_to_bits) = dom(#6)` (wire driven by cast expression)

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(raw_bits) < Async`
- `dom(opcode_reg) < Async`
- `dom(opcode_wire) < Async`
- `dom(back_to_bits) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock) = Clock
  F2: dom(opcode_reg) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  S1: dom(clock) < Async
  S2: dom(raw_bits) < Async
  S3: dom(opcode_reg) < Async
  S4: dom(opcode_wire) < Async
  S5: dom(back_to_bits) < Async

Equality:
  E1: dom(#0) = dom(clock)
  E2: dom(#1) = dom(raw_bits)
  E3: dom(#3) = dom(raw_bits)
  E4: dom(#5) = dom(opcode_wire)
  E5: dom(#2) = dom(#1)           ← cast preserves domain
  E6: dom(#4) = dom(#3)           ← cast preserves domain
  E7: dom(#6) = dom(#5)           ← cast preserves domain
  E8: dom(opcode_reg) = dom(#2)
  E9: dom(opcode_wire) = dom(#4)
  E10: dom(back_to_bits) = dom(#6)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock}, {raw_bits}, {opcode_reg}, {opcode_wire}, {back_to_bits},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

### Processing fixed constraints F1-F2:

Assign concrete domains:
```
{clock: Clock}, {raw_bits}, {opcode_reg: Sync(clock)}, {opcode_wire}, {back_to_bits},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

### Processing equality constraint E1: `dom(#0) = dom(clock)`

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {raw_bits}, {opcode_reg: Sync(clock)}, {opcode_wire}, {back_to_bits},
{#1}, {#2}, {#3}, {#4}, {#5}, {#6}
```

### Processing equality constraint E2: `dom(#1) = dom(raw_bits)`

Merge expression #1 with raw_bits's equivalence class:
```
{clock, #0: Clock}, {raw_bits, #1}, {opcode_reg: Sync(clock)}, {opcode_wire}, {back_to_bits},
{#2}, {#3}, {#4}, {#5}, {#6}
```

### Processing equality constraint E3: `dom(#3) = dom(raw_bits)`

Merge expression #3 with raw_bits's equivalence class:
```
{clock, #0: Clock}, {raw_bits, #1, #3}, {opcode_reg: Sync(clock)}, {opcode_wire}, {back_to_bits},
{#2}, {#4}, {#5}, {#6}
```

### Processing equality constraint E4: `dom(#5) = dom(opcode_wire)`

Merge expression #5 with opcode_wire's equivalence class:
```
{clock, #0: Clock}, {raw_bits, #1, #3}, {opcode_reg: Sync(clock)}, {opcode_wire, #5}, {back_to_bits},
{#2}, {#4}, {#6}
```

### Processing equality constraint E5: `dom(#2) = dom(#1)` (cast preserves domain)

Merge #2 with #1's class (raw_bits):
- #1's class: `{raw_bits, #1, #3}` (no concrete domain yet)
- #2's class: `{#2}` (no concrete domain)
- Result: merge into one class

```
{clock, #0: Clock}, {raw_bits, #1, #3, #2}, {opcode_reg: Sync(clock)}, {opcode_wire, #5}, {back_to_bits},
{#4}, {#6}
```

### Processing equality constraint E6: `dom(#4) = dom(#3)` (cast preserves domain)

Merge #4 with #3's class (raw_bits):
- #3's class: `{raw_bits, #1, #3, #2}` (no concrete domain yet)
- #4's class: `{#4}` (no concrete domain)
- Result: merge into one class

```
{clock, #0: Clock}, {raw_bits, #1, #3, #2, #4}, {opcode_reg: Sync(clock)}, {opcode_wire, #5}, {back_to_bits},
{#6}
```

### Processing equality constraint E7: `dom(#6) = dom(#5)` (cast preserves domain)

Merge #6 with #5's class (opcode_wire):
- #5's class: `{opcode_wire, #5}` (no concrete domain yet)
- #6's class: `{#6}` (no concrete domain)
- Result: merge into one class

```
{clock, #0: Clock}, {raw_bits, #1, #3, #2, #4}, {opcode_reg: Sync(clock)}, {opcode_wire, #5, #6}, {back_to_bits}
```

### Processing equality constraint E8: `dom(opcode_reg) = dom(#2)`

Merge opcode_reg with #2's class:
- opcode_reg's class: `{opcode_reg: Sync(clock)}` (has concrete domain)
- #2's class: `{raw_bits, #1, #3, #2, #4}` (no concrete domain)
- Result: merged class inherits `Sync(clock)`

```
{clock, #0: Clock}, {raw_bits, #1, #3, #2, #4, opcode_reg: Sync(clock)}, {opcode_wire, #5, #6}, {back_to_bits}
```

### Processing equality constraint E9: `dom(opcode_wire) = dom(#4)`

Merge opcode_wire with #4's class:
- #4's class: `{raw_bits, #1, #3, #2, #4, opcode_reg: Sync(clock)}` (has concrete domain)
- opcode_wire's class: `{opcode_wire, #5, #6}` (no concrete domain)
- Result: merged class inherits `Sync(clock)`

```
{clock, #0: Clock}, {raw_bits, #1, #3, #2, #4, opcode_reg, opcode_wire, #5, #6: Sync(clock)}, {back_to_bits}
```

### Processing equality constraint E10: `dom(back_to_bits) = dom(#6)`

Merge back_to_bits with #6's class:
- #6's class: `{raw_bits, #1, #3, #2, #4, opcode_reg, opcode_wire, #5, #6: Sync(clock)}`
- back_to_bits's class: `{back_to_bits}` (no concrete domain)
- Result: back_to_bits joins and inherits `Sync(clock)`

```
{clock, #0: Clock}, 
{raw_bits, #1, #3, #2, #4, opcode_reg, opcode_wire, #5, #6, back_to_bits: Sync(clock)}
```

### Final partition:
```
Class 1: {clock, #0} → Clock
Class 2: {raw_bits, #1, #3, #2, #4, opcode_reg, opcode_wire, #5, #6, back_to_bits} → Sync(ComponentId(clock))
```

### Solution (domain assignment):
```
Components:
  dom(clock) = Clock
  dom(raw_bits) = Sync(ComponentId(clock))
  dom(opcode_reg) = Sync(ComponentId(clock))
  dom(opcode_wire) = Sync(ComponentId(clock))
  dom(back_to_bits) = Sync(ComponentId(clock))

Expressions:
  dom(#0) = Clock
  dom(#1) = Sync(ComponentId(clock))  ← raw_bits reference
  dom(#2) = Sync(ComponentId(clock))  ← cast(raw_bits) to Opcode
  dom(#3) = Sync(ComponentId(clock))  ← raw_bits reference
  dom(#4) = Sync(ComponentId(clock))  ← cast(raw_bits) to Opcode
  dom(#5) = Sync(ComponentId(clock))  ← opcode_wire reference
  dom(#6) = Sync(ComponentId(clock))  ← cast(opcode_wire) to Word[4]
```

### Verification:

**All fixed constraints satisfied:** ✓
- `dom(clock) = Clock` ✓
- `dom(opcode_reg) = Sync(clock)` ✓

**All strict upper bound constraints satisfied:** ✓
- `dom(clock) < Async` → `Clock < Async` ✓
- `dom(raw_bits) < Async` → `Sync(clock) < Async` ✓
- `dom(opcode_reg) < Async` → `Sync(clock) < Async` ✓
- `dom(opcode_wire) < Async` → `Sync(clock) < Async` ✓
- `dom(back_to_bits) < Async` → `Sync(clock) < Async` ✓

**All equality constraints satisfied:** ✓
- All variables in Class 1 have domain `Clock` ✓
- All variables in Class 2 have domain `Sync(clock)` ✓
- **Key insight**: All cast() operations preserved their operand domains ✓

**Key domain flow observations:**

1. **Forward flow through cast()**: 
   - `raw_bits` (unconstrained incoming) → `cast(raw_bits)` → `opcode_reg` (fixed to Sync(clock))
   - Domain flows backward: register's fixed domain propagates through cast() to constrain input

2. **Preservation through type conversion**:
   - `opcode_wire` unified with `raw_bits` through the cast chain
   - `back_to_bits` gets the same domain as `opcode_wire` through cast()
   - Cast operations are transparent to domain analysis

3. **Bidirectional casting**:
   - Word[4] → Opcode: domain preserved
   - Opcode → Word[4]: domain preserved
   - Direction of type conversion irrelevant for domains

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock) = Clock` → Clock (concrete, the parameter's binding)
- `dom(raw_bits) = Sync(ComponentId(clock))` → α (the domain parameter)

### Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        raw_bits: Var(DomainVar(0)),  // α
    }
}
```

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: CastExample,
    component_domains: {
        clock: Clock,
        raw_bits: Sync(ComponentId(clock)),
        opcode_reg: Sync(ComponentId(clock)),
        opcode_wire: Sync(ComponentId(clock)),
        back_to_bits: Sync(ComponentId(clock)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Yes, with a small clarification needed.** The algorithm needs a `cast()` case added to `walk_expression()` in ALGORITHM.md, but the logic is straightforward:

```python
# Cast expression (unary operation, preserves domain)
elif expr.kind == Cast:
    expr_id, operand_expr = walk_expression(expr.operand, &mut variables,
                                           &mut equalities, &mut upper_bounds, expr_id)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Cast preserves operand domain
    equalities.append(Equality(result_expr, operand_expr))
    
    return (expr_id, result_expr)
```

This follows the same pattern as field access and array element access: **unary operations that preserve domain**.

### Surprises or edge cases?

**No surprises, perfect alignment with existing patterns:**

1. **Unary operation pattern is consistent**:
   - Field access: `struct.field` → `dom(result) = dom(struct)`
   - Array indexing: `array[idx]` → `dom(result) = dom(array)`
   - Cast: `cast(expr)` → `dom(result) = dom(expr)`
   
   All unary operations that "extract" or "reinterpret" data preserve the source domain.

2. **Type conversion is orthogonal to domain**:
   - Changing the type (Word[4] ↔ Opcode) doesn't affect domain
   - Domain tracking follows data flow, not type flow
   - This is correct: the bits still come from the same clock domain

3. **Cast chains compose naturally**:
   ```virdant
   wire w1 : Word[4] { it := some_signal }
   wire w2 : OpCode { it := cast(w1) }
   wire w3 : Word[4] { it := cast(w2) }
   // All three have the same domain as some_signal
   ```

4. **Cast doesn't introduce upcasting**:
   - Unlike binary operators (which can upcast operands to match result)
   - Cast is strict equality: `dom(result) = dom(operand)`
   - No implicit conversions or flexibility

5. **Error propagation works naturally**:
   ```virdant
   reg a : Opcode on clk_a { ... }
   reg b : Word[4] on clk_b { it <= cast(a) }  // ERROR: Sync(clk_a) ≠ Sync(clk_b)
   ```
   
   The cast doesn't hide the domain mismatch — the equality constraint `dom(b) = dom(cast(a)) = dom(a)` will trigger a unification error.

### Does the constraint system handle this without special cases?

**Yes, perfectly.** The constraint system handles cast() with no special logic beyond adding the case to `walk_expression()`:

1. **No lattice interaction**: Cast generates simple equality constraints, no upper/lower bound logic

2. **No multi-operand complexity**: Single operand means no unification between multiple sources

3. **No control flow**: Cast is a pure expression, no branching or pattern matching

4. **Composes with existing rules**: Works with registers, wires, binary ops, etc. through the same equality constraint mechanism

### Any clarifications needed for ALGORITHM.md or PLANv2.md?

**ALGORITHM.md needs cast() case added:**

Add to `walk_expression()` function in the expression kind enumeration:

```python
# Cast expression (unary type conversion)
elif expr.kind == Cast:
    expr_id, operand_expr = walk_expression(expr.operand, &mut variables,
                                           &mut equalities, &mut upper_bounds, expr_id)
    
    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Cast preserves domain: result has same domain as operand
    equalities.append(Equality(result_expr, operand_expr))
    
    return (expr_id, result_expr)
```

**PLANv2.md clarification (optional but helpful):**

In the section on "Combinational Expressions" (§3.1), add a note:

> **Unary operations**: Operations that transform a single value (like `cast()` for type conversion, field access `struct.field`, or array indexing `array[idx]`) preserve the source domain. The result has the same domain as the operand.

**No algorithm gaps discovered:**

- Cast() fits naturally into the existing framework
- No new constraint types needed
- No special-case logic required
- Error detection works through existing unification mechanisms

### Key implementation notes

1. **AST representation**: `cast()` should be its own expression kind (`ExprCast`) with one child expression

2. **Type checking happens first**: Domain analysis doesn't need to validate that the cast is between compatible types (enum ↔ Word[n]) — that's already checked by the type checker

3. **No ambiguity**: Unlike some operations where domain flow might be bidirectional, cast is simple: `dom(result) = dom(operand)` always

4. **Performance**: Cast constraints are as cheap as field access — just one equality constraint per cast

### Comparison with other unary operations

| Operation | Domain Rule | Notes |
|-----------|-------------|-------|
| `cast(x)` | `dom(result) = dom(x)` | Type conversion |
| `x.field` | `dom(result) = dom(x)` | Field access |
| `x[idx]` | `dom(result) = dom(x)` | Array element (idx also = x) |
| `zext(x)` | `dom(result) = dom(x)` | Zero extend |
| `sext(x)` | `dom(result) = dom(x)` | Sign extend |
| `!x` | `dom(result) = dom(x)` | Logical not |
| `-x` | `dom(result) = dom(x)` | Unary negation |

**Pattern**: All unary data-manipulation operations preserve domain. This is a fundamental principle of the domain system.

### Key takeaway

**`cast()` is the simplest domain preservation case.** It's a pure unary operation with no branching, no lattice complexity, and no multi-operand unification. It demonstrates that the constraint system's equality-based approach handles type conversions transparently — domain tracking is orthogonal to type tracking.

The only work needed is documenting the `cast()` case in ALGORITHM.md's `walk_expression()` function. Everything else (solving, signature extraction, error detection) works automatically through the existing constraint mechanism.
