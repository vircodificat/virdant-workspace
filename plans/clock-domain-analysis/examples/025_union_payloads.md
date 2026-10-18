# Example 25: Union Type with Payload Domain Constraints

```virdant
union type Packet {
    Empty()
    Single(data : Word[8])
    Dual(first : Word[8], second : Word[8])
}

mod UnionPayloads {
    incoming clock : Clock
    incoming enable : Bit

    reg counter : Word[8] on clock {
        it <= counter + 1w8
    }

    // Legal: all payloads come from the same domain
    wire pkt1 : Packet {
        it := @Single(counter)  // payload is Sync(clock)
    }

    // Legal: both payloads from same domain  
    wire pkt2 : Packet {
        it := @Dual(counter, counter + 1w8)  // both payloads Sync(clock)
    }

    // Legal: literal payloads are Constant
    wire pkt3 : Packet {
        it := @Dual(0w8, 1w8)  // both payloads Constant
    }

    // Pattern matching: subject and all arms same domain
    wire extracted : Word[8] {
        it := match pkt1 {
            case @Empty() => 0w8
            case @Single(d) => d
            case @Dual(f, s) => f
        }
    }

    outgoing out : Word[8] {
        it := extracted
    }
}
```

**What's happening**: Union variants are constructed with payloads from the same clock domain. In `@Single(counter)`, the payload comes from a register on `clock`, so the entire union value has domain `Sync(clock)`. In `@Dual(counter, counter + 1w8)`, both payloads are expressions in `Sync(clock)`, so they unify successfully. The `pkt3` example shows that literal payloads (domain `Constant`) also work. Pattern matching on the union follows the standard match rules: the subject and all arm results must have the same domain.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `enable` (incoming, type Bit)
- `counter` (reg, type Word[8])
- `pkt1` (wire, type Packet)
- `pkt2` (wire, type Packet)
- `pkt3` (wire, type Packet)
- `extracted` (wire, type Word[8])
- `out` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions

**Declaration: `incoming enable : Bit`**
- Component: `enable`
- Type: Bit
- No expressions

**Declaration: `reg counter : Word[8] on clock`**
- Component: `counter`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `counter <= counter + 1w8`**
- Expression `#1`: reference to `counter` (LHS of binary op)
- Expression `#2`: literal `1w8` (RHS of binary op)
- Expression `#3`: binary op `counter + 1w8` (RHS of driver)

**Declaration: `wire pkt1 : Packet`**
- Component: `pkt1`
- No expressions in declaration

**Driver: `pkt1 := @Single(counter)`**
- Expression `#4`: reference to `counter` (payload)
- Expression `#5`: union constructor `@Single(counter)` (RHS of driver)

**Declaration: `wire pkt2 : Packet`**
- Component: `pkt2`
- No expressions in declaration

**Driver: `pkt2 := @Dual(counter, counter + 1w8)`**
- Expression `#6`: reference to `counter` (first payload)
- Expression `#7`: reference to `counter` (LHS of binary op in second payload)
- Expression `#8`: literal `1w8` (RHS of binary op in second payload)
- Expression `#9`: binary op `counter + 1w8` (second payload)
- Expression `#10`: union constructor `@Dual(counter, counter + 1w8)` (RHS of driver)

**Declaration: `wire pkt3 : Packet`**
- Component: `pkt3`
- No expressions in declaration

**Driver: `pkt3 := @Dual(0w8, 1w8)`**
- Expression `#11`: literal `0w8` (first payload)
- Expression `#12`: literal `1w8` (second payload)
- Expression `#13`: union constructor `@Dual(0w8, 1w8)` (RHS of driver)

**Declaration: `wire extracted : Word[8]`**
- Component: `extracted`
- No expressions in declaration

**Driver: `extracted := match pkt1 { ... }`**
- Expression `#14`: reference to `pkt1` (match subject)
- Expression `#15`: literal `0w8` (arm 1 result)
- Expression `#16`: reference to `d` (arm 2 result - but `d` is bound from pattern)
- Expression `#17`: reference to `f` (arm 3 result - but `f` is bound from pattern)
- Expression `#18`: match expression result

**Note on pattern bindings**: In the match arms:
- `case @Single(d) => d`: The pattern binding `d` captures the payload, which must equal the subject's payload field. When we reference `d` in the arm result, it has the same domain as that payload field.
- `case @Dual(f, s) => f`: Similarly, `f` captures the first payload field.

Actually, let me reconsider the expression handling for pattern matching. According to ALGORITHM.md:

```python
elif expr.kind == Match:
    expr_id, subject_expr = walk_expression(expr.subject, ...)
    
    arm_exprs = []
    for arm in expr.arms:
        expr_id, arm_expr = walk_expression(arm.result, ...)
        arm_exprs.append(arm_expr)
    
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # Subject and all arms must equal result
    equalities.append(Equality(subject_expr, result_expr))
    for arm_expr in arm_exprs:
        equalities.append(Equality(arm_expr, result_expr))
```

So pattern bindings introduce new variable names in scope, but when those variables are referenced in the arm expression, they create normal component references. However, these bound variables aren't "components" in ComponentAnalysis - they're pattern-introduced bindings.

Let me think about this more carefully. In the match expression:
```virdant
match pkt1 {
    case @Empty() => 0w8
    case @Single(d) => d
    case @Dual(f, s) => f
}
```

The pattern `@Single(d)` binds `d` to the payload field from `pkt1`. In the implementation, this would be like a field access `pkt1.Single.data` (if we had explicit field access on unions, which we don't in concrete syntax, but conceptually that's what destructuring does).

For the purposes of domain analysis, I think we should treat pattern-bound variables as having the same domain as the variant payload they came from, which in turn has the same domain as the union subject. So:
- `dom(pkt1.Single.data) = dom(pkt1)` (union field has same domain as union)
- `d` is bound to `pkt1.Single.data`
- `dom(d) = dom(pkt1.Single.data) = dom(pkt1)`

But in the match arm `case @Single(d) => d`, when we use `d` as the result expression, we're creating an expression that references this bound variable. The key insight is that `d` represents a part of the subject, so `dom(d) = dom(subject)`.

Actually, I think the cleanest approach is:
1. Pattern matching doesn't create separate expression nodes for pattern bindings
2. When a pattern binding is referenced in the arm result (e.g., `d` in `=> d`), we treat it as implicitly equal to the match subject's domain
3. We can generate an explicit equality constraint: `dom(arm_result) = dom(subject)` automatically handles this

But wait, that's what ALGORITHM.md already does! The match expression rule states:
```python
# Subject and all arms must equal result
equalities.append(Equality(subject_expr, result_expr))
for arm_expr in arm_exprs:
    equalities.append(Equality(arm_expr, result_expr))
```

So:
- `dom(subject) = dom(result)`
- `dom(arm1) = dom(result)`
- `dom(arm2) = dom(result)`
- etc.

By transitivity: `dom(subject) = dom(arm1) = dom(arm2) = ... = dom(result)`

When an arm uses a pattern-bound variable (like `d`), that variable represents a field from the subject, which by the union construction rules had to have the same domain as the subject. So `dom(d) = dom(subject)`, and the constraint `dom(arm) = dom(result) = dom(subject)` is satisfied.

The key is: **union construction already enforced that all payload fields equal the union's domain**. So when we destructure in the match and use those fields, they automatically have the right domain.

Let me simplify the expression list:

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `counter` (in counter + 1w8)
- `#2`: ExprLiteral `1w8`
- `#3`: ExprBinaryOp `counter + 1w8`
- `#4`: ExprReference to `counter` (payload in @Single)
- `#5`: ExprUnionConstructor `@Single(counter)`
- `#6`: ExprReference to `counter` (first payload in @Dual)
- `#7`: ExprReference to `counter` (in counter + 1w8, second payload)
- `#8`: ExprLiteral `1w8`
- `#9`: ExprBinaryOp `counter + 1w8`
- `#10`: ExprUnionConstructor `@Dual(counter, counter + 1w8)`
- `#11`: ExprLiteral `0w8`
- `#12`: ExprLiteral `1w8`
- `#13`: ExprUnionConstructor `@Dual(0w8, 1w8)`
- `#14`: ExprReference to `pkt1` (match subject)
- `#15`: ExprLiteral `0w8` (arm 1 result: @Empty() => 0w8)
- `#16`: ExprReference to bound `d` (arm 2 result: @Single(d) => d)
- `#17`: ExprReference to bound `f` (arm 3 result: @Dual(f,s) => f)
- `#18`: ExprMatch (entire match expression)

For pattern-bound variables `d`, `f`, `s`, I think we should treat them as ephemeral expression nodes that are equal to the subject. Let me reconsider...

Actually, I think for this example, I should keep it simple and not get too deep into the implementation details of pattern binding. The key constraints that matter are:
1. Match subject and all arms equal the result
2. That's sufficient to ensure domain consistency

Let me revise to just focus on the arm result expressions:

**Expression inventory (revised):**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `counter`
- `#2`: ExprLiteral `1w8`
- `#3`: ExprBinaryOp `counter + 1w8`
- `#4`: ExprReference to `counter` (pkt1 payload)
- `#5`: ExprUnionConstructor `@Single(counter)`
- `#6`: ExprReference to `counter` (pkt2 first payload)
- `#7`: ExprReference to `counter`
- `#8`: ExprLiteral `1w8`
- `#9`: ExprBinaryOp `counter + 1w8` (pkt2 second payload)
- `#10`: ExprUnionConstructor `@Dual(counter, counter + 1w8)`
- `#11`: ExprLiteral `0w8`
- `#12`: ExprLiteral `1w8`
- `#13`: ExprUnionConstructor `@Dual(0w8, 1w8)`
- `#14`: ExprReference to `pkt1` (match subject)
- `#15`: ExprLiteral `0w8` (arm 1 result)
- `#16`: Arm 2 result expression (references payload from subject)
- `#17`: Arm 3 result expression (references payload from subject)
- `#18`: ExprMatch (entire match expression)

For expressions #16 and #17, the key is that they refer to parts of the subject. But in the constraint system, what matters is that these arm result expressions will equal the match result, which equals the subject. The internal mechanics of how pattern bindings work don't need to be spelled out in detail - the match rule handles it.

Actually, for clarity in the example, I'll note that pattern-bound variables conceptually have the same domain as the subject (since they're part of it), and references to them create expressions with that domain. Let me proceed with the constraints.

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `F1: dom(clock) = Clock`

**Fixed constraints from literals:**
- `F2: dom(#2) = Constant` (literal 1w8)
- `F3: dom(#8) = Constant` (literal 1w8)
- `F4: dom(#11) = Constant` (literal 0w8)
- `F5: dom(#12) = Constant` (literal 1w8)
- `F6: dom(#15) = Constant` (literal 0w8)

**Fixed constraints from register declarations:**
- `F7: dom(counter) = Sync(ComponentId(clock))`

**Equality constraints from component references:**
- `E1: dom(#0) = dom(clock)` (reference to clock in `on` clause)
- `E2: dom(#1) = dom(counter)` (reference in counter + 1w8)
- `E3: dom(#4) = dom(counter)` (reference in @Single payload)
- `E4: dom(#6) = dom(counter)` (reference in @Dual first payload)
- `E5: dom(#7) = dom(counter)` (reference in counter + 1w8)
- `E6: dom(#14) = dom(pkt1)` (reference to pkt1 in match)

**Equality constraints from binary operators:**

Binary op `#3` (counter + 1w8):
- Operands can upcast to result
- `E7: dom(#1) ≤ dom(#3)` (UpperBound)
- `E8: dom(#2) ≤ dom(#3)` (UpperBound)

Binary op `#9` (counter + 1w8):
- `E9: dom(#7) ≤ dom(#9)` (UpperBound)
- `E10: dom(#8) ≤ dom(#9)` (UpperBound)

**Equality constraints from union construction:**

Union constructor `#5` (@Single(counter)):
- Payload must equal union domain
- `E11: dom(#4) = dom(#5)`

Union constructor `#10` (@Dual(counter, counter + 1w8)):
- All payloads must equal union domain
- `E12: dom(#6) = dom(#10)`
- `E13: dom(#9) = dom(#10)`

Union constructor `#13` (@Dual(0w8, 1w8)):
- All payloads must equal union domain
- `E14: dom(#11) = dom(#13)`
- `E15: dom(#12) = dom(#13)`

**Equality constraints from match expression:**

Match expression `#18`:
- Subject must equal result
- All arms must equal result
- `E16: dom(#14) = dom(#18)` (subject = result)
- `E17: dom(#15) = dom(#18)` (arm 1 = result)
- `E18: dom(#16) = dom(#18)` (arm 2 = result)
- `E19: dom(#17) = dom(#18)` (arm 3 = result)

**Note on pattern-bound expressions #16 and #17:**
- Expression `#16` represents the arm result `d` in `case @Single(d) => d`
- The binding `d` captures the payload from `pkt1`'s `Single` variant
- By union construction rules, that payload has domain `dom(pkt1)`
- So `dom(#16) = dom(pkt1)` conceptually
- Similarly, `dom(#17) = dom(pkt1)` (references first payload of Dual variant)
- These will be represented as equalities to the subject in the actual implementation

For this walkthrough, I'll treat #16 and #17 as expressions that are implicitly tied to the subject:
- `E20: dom(#16) = dom(#14)` (arm 2 result linked to subject field)
- `E21: dom(#17) = dom(#14)` (arm 3 result linked to subject field)

**Equality constraints from drivers:**

Driver: `counter <= counter + 1w8`
- `E22: dom(counter) = dom(#3)`

Driver: `pkt1 := @Single(counter)`
- `E23: dom(pkt1) = dom(#5)`

Driver: `pkt2 := @Dual(counter, counter + 1w8)`
- `E24: dom(pkt2) = dom(#10)`

Driver: `pkt3 := @Dual(0w8, 1w8)`
- `E25: dom(pkt3) = dom(#13)`

Driver: `extracted := match pkt1 { ... }`
- `E26: dom(extracted) = dom(#18)`

Driver: `out := extracted`
- Expression `#19`: reference to `extracted`
- `E27: dom(#19) = dom(extracted)`
- `E28: dom(out) = dom(#19)`

**Implicit strict upper bound constraints (all non-async components):**
- `S1: dom(clock) < Async`
- `S2: dom(enable) < Async`
- `S3: dom(counter) < Async`
- `S4: dom(pkt1) < Async`
- `S5: dom(pkt2) < Async`
- `S6: dom(pkt3) < Async`
- `S7: dom(extracted) < Async`
- `S8: dom(out) < Async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock) = Clock
  F2: dom(#2) = Constant
  F3: dom(#8) = Constant
  F4: dom(#11) = Constant
  F5: dom(#12) = Constant
  F6: dom(#15) = Constant
  F7: dom(counter) = Sync(ComponentId(clock))

Strict upper bounds (implicit):
  S1-S8: all components < Async

Upper bounds (from binary operators):
  E7: dom(#1) ≤ dom(#3)
  E8: dom(#2) ≤ dom(#3)
  E9: dom(#7) ≤ dom(#9)
  E10: dom(#8) ≤ dom(#9)

Equalities:
  E1: dom(#0) = dom(clock)
  E2: dom(#1) = dom(counter)
  E3: dom(#4) = dom(counter)
  E4: dom(#6) = dom(counter)
  E5: dom(#7) = dom(counter)
  E6: dom(#14) = dom(pkt1)
  
  E11: dom(#4) = dom(#5)           [union Single payload]
  E12: dom(#6) = dom(#10)          [union Dual first payload]
  E13: dom(#9) = dom(#10)          [union Dual second payload]
  E14: dom(#11) = dom(#13)         [union Dual first payload]
  E15: dom(#12) = dom(#13)         [union Dual second payload]
  
  E16: dom(#14) = dom(#18)         [match subject = result]
  E17: dom(#15) = dom(#18)         [match arm 1 = result]
  E18: dom(#16) = dom(#18)         [match arm 2 = result]
  E19: dom(#17) = dom(#18)         [match arm 3 = result]
  E20: dom(#16) = dom(#14)         [arm 2 uses subject field]
  E21: dom(#17) = dom(#14)         [arm 3 uses subject field]
  
  E22: dom(counter) = dom(#3)
  E23: dom(pkt1) = dom(#5)
  E24: dom(pkt2) = dom(#10)
  E25: dom(pkt3) = dom(#13)
  E26: dom(extracted) = dom(#18)
  E27: dom(#19) = dom(extracted)
  E28: dom(out) = dom(#19)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment.

### Initial state:
All components and expressions start in separate equivalence classes.

### Processing fixed constraints:

**F1: `dom(clock) = Clock`**
```
{clock: Clock}, {enable}, {counter}, {pkt1}, {pkt2}, {pkt3}, {extracted}, {out},
{#0}, {#1}, {#2: Constant}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8: Constant},
{#9}, {#10}, {#11: Constant}, {#12: Constant}, {#13}, {#14}, {#15: Constant},
{#16}, {#17}, {#18}, {#19}
```

**F2-F6**: Literals become Constant
```
{clock: Clock}, {enable}, {counter}, {pkt1}, {pkt2}, {pkt3}, {extracted}, {out},
{#0}, {#1}, {#2: Constant}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8: Constant},
{#9}, {#10}, {#11: Constant}, {#12: Constant}, {#13}, {#14}, {#15: Constant},
{#16}, {#17}, {#18}, {#19}
```

**F7: `dom(counter) = Sync(ComponentId(clock))`**
```
{clock: Clock}, {enable}, {counter: Sync(clock)}, {pkt1}, {pkt2}, {pkt3},
{extracted}, {out}, {#0}, {#1}, {#2: Constant}, {#3}, {#4}, {#5}, {#6},
{#7}, {#8: Constant}, {#9}, {#10}, {#11: Constant}, {#12: Constant}, {#13},
{#14}, {#15: Constant}, {#16}, {#17}, {#18}, {#19}
```

### Processing equality constraints:

I'll show the key merges that establish the main domains:

**E1: `dom(#0) = dom(clock)`**
Merge #0 with clock:
```
{clock, #0: Clock}, ...
```

**E2: `dom(#1) = dom(counter)`**
Merge #1 with counter:
```
..., {counter, #1: Sync(clock)}, ...
```

**E3: `dom(#4) = dom(counter)`**
Merge #4 with counter's class:
```
..., {counter, #1, #4: Sync(clock)}, ...
```

**E4: `dom(#6) = dom(counter)`**
Merge #6 with counter's class:
```
..., {counter, #1, #4, #6: Sync(clock)}, ...
```

**E5: `dom(#7) = dom(counter)`**
Merge #7 with counter's class:
```
..., {counter, #1, #4, #6, #7: Sync(clock)}, ...
```

**E11: `dom(#4) = dom(#5)`** (union Single payload = union)
#4 is in counter's class (Sync(clock)), #5 is unresolved.
Merge #5 into counter's class:
```
..., {counter, #1, #4, #5, #6, #7: Sync(clock)}, ...
```

**E23: `dom(pkt1) = dom(#5)`** (pkt1 = @Single(...))
#5 is in counter's class (Sync(clock)), pkt1 is unresolved.
Merge pkt1 into counter's class:
```
..., {counter, #1, #4, #5, #6, #7, pkt1: Sync(clock)}, ...
```

**Key insight**: `pkt1` now has domain `Sync(clock)` because its payload came from `counter`, which is `Sync(clock)`.

**E12: `dom(#6) = dom(#10)`** (union Dual first payload)
#6 is in counter's class (Sync(clock)), #10 is unresolved.
Merge #10 into counter's class:
```
..., {counter, ..., #10: Sync(clock)}, ...
```

**E13: `dom(#9) = dom(#10)`** (union Dual second payload)
Need to resolve #9 first. Binary op `#9` is `counter + 1w8`.
Upper bounds: `dom(#7) ≤ dom(#9)` and `dom(#8) ≤ dom(#9)`
- #7 is `Sync(clock)`
- #8 is `Constant`
- LUB of {Sync(clock), Constant} = Sync(clock)
So #9 resolves to `Sync(clock)`.

Now merge #9 with #10's class:
- Both have `Sync(clock)` ✓ Compatible
```
..., {counter, ..., #9, #10: Sync(clock)}, ...
```

**E24: `dom(pkt2) = dom(#10)`**
#10 is `Sync(clock)`, pkt2 is unresolved.
Merge pkt2 into that class:
```
..., {counter, ..., pkt2, ...: Sync(clock)}, ...
```

**Key insight**: `pkt2` also has domain `Sync(clock)` because both its payloads came from `counter` and `counter+1w8`, both in `Sync(clock)`.

**E14: `dom(#11) = dom(#13)`** (union Dual with literals, first payload)
#11 is `Constant`, #13 is unresolved.
Merge #13 into Constant class? Or vice versa. Let's say merge #13 with #11's class:
```
{#11, #13: Constant}, ...
```

**E15: `dom(#12) = dom(#13)`** (second payload)
#12 is `Constant`, #13 is now `Constant`.
- Compatible ✓
Merge:
```
{#11, #12, #13: Constant}, ...
```

**E25: `dom(pkt3) = dom(#13)`**
#13 is `Constant`, pkt3 is unresolved.
Merge pkt3 into Constant class:
```
{#11, #12, #13, pkt3: Constant}, ...
```

**Key insight**: `pkt3` has domain `Constant` because both its payloads are literals.

**E6: `dom(#14) = dom(pkt1)`** (match subject reference)
pkt1 is `Sync(clock)`, #14 is unresolved.
Merge #14 into pkt1's class:
```
..., {counter, ..., pkt1, #14: Sync(clock)}, ...
```

**E16: `dom(#14) = dom(#18)`** (match subject = result)
#14 is `Sync(clock)`, #18 is unresolved.
Merge #18 into that class:
```
..., {counter, ..., pkt1, #14, #18: Sync(clock)}, ...
```

**E17: `dom(#15) = dom(#18)`** (match arm 1 = result)
#15 is `Constant` (literal 0w8), #18 is `Sync(clock)`.
- Check compatibility: `compatible(Constant, Sync(clock))`? **Yes** ✓
- Constant unifies with Sync(clock) → result is Sync(clock)
Merge #15 into #18's class:
```
..., {counter, ..., pkt1, #14, #15, #18: Sync(clock)}, ...
```

**E18: `dom(#16) = dom(#18)`** (match arm 2 = result)
- #16 represents the arm result that uses pattern-bound variable `d`
- #18 is `Sync(clock)`
- By E20, `dom(#16) = dom(#14)`, and #14 is `Sync(clock)`, so #16 should be `Sync(clock)`
- Merge #16 into #18's class:
```
..., {counter, ..., pkt1, #14, #15, #16, #18: Sync(clock)}, ...
```

**E19: `dom(#17) = dom(#18)`** (match arm 3 = result)
- Similarly, #17 is `Sync(clock)` via E21
- Merge:
```
..., {counter, ..., pkt1, #14, #15, #16, #17, #18: Sync(clock)}, ...
```

**E26: `dom(extracted) = dom(#18)`**
#18 is `Sync(clock)`, extracted is unresolved.
Merge extracted:
```
..., {counter, ..., pkt1, #14, #15, #16, #17, #18, extracted: Sync(clock)}, ...
```

**E27: `dom(#19) = dom(extracted)`**
Merge #19:
```
..., {counter, ..., pkt1, #14, #15, #16, #17, #18, #19, extracted: Sync(clock)}, ...
```

**E28: `dom(out) = dom(#19)`**
Merge out:
```
..., {counter, ..., pkt1, #14, #15, #16, #17, #18, #19, extracted, out: Sync(clock)}, ...
```

### Final partition (conceptual):
```
Class 1: {clock, #0} → Clock
Class 2: {counter, #1, #3, #4, #5, #6, #7, #9, #10, pkt1, pkt2, #14, #15, #16, #17, #18, #19, extracted, out} → Sync(ComponentId(clock))
Class 3: {#2, #8, #11, #12, #13, pkt3} → Constant
Class 4: {enable} → unresolved, no constraints
```

### Solution (domain assignment):
```
Components:
  dom(clock) = Clock
  dom(enable) = unresolved (no constraints)
  dom(counter) = Sync(ComponentId(clock))
  dom(pkt1) = Sync(ComponentId(clock))
  dom(pkt2) = Sync(ComponentId(clock))
  dom(pkt3) = Constant
  dom(extracted) = Sync(ComponentId(clock))
  dom(out) = Sync(ComponentId(clock))

Key expressions:
  dom(#5) = Sync(ComponentId(clock))      [@Single(counter)]
  dom(#10) = Sync(ComponentId(clock))     [@Dual(counter, counter+1w8)]
  dom(#13) = Constant                      [@Dual(0w8, 1w8)]
  dom(#18) = Sync(ComponentId(clock))     [match expression]
```

### Verification:

**Fixed constraints:** ✓
- dom(clock) = Clock ✓
- Literals = Constant ✓
- dom(counter) = Sync(clock) ✓

**Strict upper bounds:** ✓
- All resolved components are either Clock, Sync(clock), or Constant
- All are strictly less than Async ✓

**Union construction constraints:** ✓
- @Single(counter): payload Sync(clock) = union Sync(clock) ✓
- @Dual(counter, counter+1w8): both payloads Sync(clock) = union Sync(clock) ✓
- @Dual(0w8, 1w8): both payloads Constant = union Constant ✓

**Match expression constraints:** ✓
- Subject pkt1 (Sync(clock)) = result #18 (Sync(clock)) ✓
- All arms equal result ✓

**All equality constraints satisfied** ✓

---

## Pass 3: Signature Extraction

Build the module's polymorphic domain signature.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(enable)` = unresolved → `Free()`
- `dom(out) = Sync(ComponentId(clock))` → `Var(α)`

### Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        enable: Free(),
        out: Var(α),
    }
}
```

**Interpretation**: 
- The module is parameterized by one clock domain α (corresponding to the `clock` port)
- The `enable` input is unconstrained and can accept any domain
- The `out` port has whatever domain the clock port establishes

**Final DomainAnalysis result:**
```rust
DomainAnalysis {
    moddef: UnionPayloads,
    component_domains: {
        clock: Clock,
        enable: Free(),  // or could be left unresolved
        counter: Sync(ComponentId(clock)),
        pkt1: Sync(ComponentId(clock)),
        pkt2: Sync(ComponentId(clock)),
        pkt3: Constant,
        extracted: Sync(ComponentId(clock)),
        out: Sync(ComponentId(clock)),
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Error Case Example

Now let's show what happens when union payloads have incompatible domains:

```virdant
mod UnionPayloadError {
    incoming clock_a : Clock
    incoming clock_b : Clock

    reg val_a : Word[8] on clock_a {
        it <= 0w8
    }

    reg val_b : Word[8] on clock_b {
        it <= 0w8
    }

    // ERROR: payloads from different clock domains
    wire bad_pkt : Packet {
        it := @Dual(val_a, val_b)  // Sync(clock_a) vs Sync(clock_b)
    }
}
```

### Pass 1: Constraint Generation (Error Case)

Components:
- `clock_a`, `clock_b` (incoming, Clock)
- `val_a` (reg on clock_a)
- `val_b` (reg on clock_b)
- `bad_pkt` (wire, Packet)

Expressions:
- `#0`: ref clock_a
- `#1`: literal 0w8
- `#2`: ref clock_b
- `#3`: literal 0w8
- `#4`: ref val_a (first payload)
- `#5`: ref val_b (second payload)
- `#6`: union constructor @Dual(val_a, val_b)

Constraints:
```
Fixed:
  dom(clock_a) = Clock
  dom(clock_b) = Clock
  dom(#1) = Constant
  dom(#3) = Constant
  dom(val_a) = Sync(ComponentId(clock_a))
  dom(val_b) = Sync(ComponentId(clock_b))

Equalities:
  dom(#0) = dom(clock_a)
  dom(#2) = dom(clock_b)
  dom(#4) = dom(val_a)
  dom(#5) = dom(val_b)
  dom(#4) = dom(#6)    ← first payload = union
  dom(#5) = dom(#6)    ← second payload = union (CONFLICT)
  dom(val_a) = dom(#1)
  dom(val_b) = dom(#3)
  dom(bad_pkt) = dom(#6)
```

### Pass 2: Constraint Solving (Error Case)

After processing fixed constraints and some equalities:
```
{clock_a, #0: Clock}
{clock_b, #2: Clock}
{val_a, #1, #4: Sync(clock_a)}
{val_b, #3, #5: Sync(clock_b)}
{#6}, {bad_pkt}
```

**Processing: `dom(#4) = dom(#6)`**

Merge #6 with val_a's class:
```
{val_a, #1, #4, #6: Sync(clock_a)}
{val_b, #3, #5: Sync(clock_b)}
```

**Processing: `dom(#5) = dom(#6)`** ⚠️ ERROR

Attempt to merge #5 with #6:
- #5 is in class with val_b: `Sync(clock_b)`
- #6 is in class with val_a: `Sync(clock_a)`
- Check: `compatible(Sync(clock_b), Sync(clock_a))`?
- Both are Sync, but clock_b ≠ clock_a → **INCOMPATIBLE** ❌

**ERROR DETECTED**: Cannot unify `Sync(ComponentId(clock_b))` with `Sync(ComponentId(clock_a))`

Error message:
```
Domain unification error in union construction `@Dual(val_a, val_b)`:
  First payload has domain Sync(clock_a)
  Second payload has domain Sync(clock_b)

All payload fields in a union constructor must have the same clock domain.

The payload fields come from:
  - val_a is a register on clock_a
  - val_b is a register on clock_b

Help: If you need to combine values from different clock domains in a union,
you must first synchronize them to the same domain using the Sync module,
or declare the result as async.
```

---

# Notes

## Does this work naturally with the algorithm as-is?

**Yes**, union types with payloads work seamlessly with the existing algorithm:

1. **Union construction mirrors struct construction**: The constraint generation rule "all payload fields must equal the union domain" is identical to the struct rule. The implementation would look almost exactly like the struct case in ALGORITHM.md.

2. **Pattern matching already handled**: The match expression rule (subject and all arms equal result) already handles union destructuring. Pattern bindings extract payload fields that, by construction, have the same domain as the union value, so they automatically satisfy the match constraints.

3. **Error detection is automatic**: When payloads have incompatible domains, the union-find merge detects the conflict during unification, just like with structs.

4. **No special lattice operations**: Union types don't introduce any new domain relationships. The lattice operations (compatible, unify, LUB, strictly_less_than) work unchanged.

## Surprises or edge cases?

**Pattern binding domain flow**: One subtlety is how pattern-bound variables in match arms relate to the subject's domain. The key insight:
- Union construction enforces `dom(payload) = dom(union)`
- When destructuring in a match, bound variables represent those payloads
- So bound variables conceptually have `dom(bound_var) = dom(subject)`
- The match rule `dom(arm_result) = dom(subject)` automatically ensures consistency

**No explicit binding constraints needed**: The algorithm doesn't need to generate explicit constraints for pattern bindings. The match rule's requirement that all arms equal the subject (which equals the result) is sufficient.

**Empty variant payloads**: Union variants with no payloads (like `@Empty()`) don't generate any payload constraints. The union domain is determined entirely by how the union value is used or what drives it.

**Variant choice doesn't affect domains**: The specific variant being constructed doesn't matter for domain analysis. Whether it's `@Single(x)` or `@Dual(x, y)`, the rule is the same: all payloads must share the union's domain.

## Does the constraint system handle this without special cases?

**Absolutely yes**. Union types require no special treatment beyond:

1. **Constraint generation**: When walking a union constructor expression, generate equality constraints from each payload expression to the constructor result. This is a simple 5-line addition to `walk_expression()`:

```python
elif expr.kind == UnionConstructor:
    payload_exprs = []
    for payload in expr.payloads:
        expr_id, payload_expr = walk_expression(payload, ...)
        payload_exprs.append(payload_expr)
    
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)
    
    # All payloads must equal result
    for payload_expr in payload_exprs:
        equalities.append(Equality(payload_expr, result_expr))
    
    return (expr_id, result_expr)
```

This is exactly parallel to struct construction, confirming the orthogonality of the design.

2. **Pattern matching**: Already handled by the existing match expression rule. No changes needed.

3. **Constraint solving**: Union-find handles union payload constraints exactly like struct field constraints. No special logic.

## Any clarifications needed for ALGORITHM.md or PLANv2.md?

**ALGORITHM.md additions**:

Add a section parallel to "Struct construction" showing union construction:

```python
# Union construction
elif expr.kind == UnionConstructor:
    payload_exprs = []
    for payload in expr.payloads:
        expr_id, payload_expr = walk_expression(payload.value, &mut variables,
                                               &mut equalities, &mut upper_bounds, expr_id)
        payload_exprs.append(payload_expr)

    # Result expression
    result_expr = expr_id
    expr_id += 1
    variables.add(result_expr)

    # All payloads must equal result
    for payload_expr in payload_exprs:
        equalities.append(Equality(payload_expr, result_expr))

    return (expr_id, result_expr)
```

**PLANv2.md additions**:

Under "Struct and Array Operations", add a subsection:

> #### Union Construction
>
> When constructing a union value:
> ```virdant
> union type Packet {
>     Empty()
>     Single(data : Word[8])
>     Dual(first : Word[8], second : Word[8])
> }
>
> wire pkt : Packet {
>     it := @Dual(a, b)
> }
> ```
>
> **Rule**: All payload expressions (`a`, `b`) must have the same domain, and the resulting union value has that domain. Variants with no payloads generate no constraints.
>
> **Pattern matching**: When destructuring a union in a `match` expression, pattern-bound variables represent payload fields that have the same domain as the union value (enforced at construction). The standard match rule (subject and all arms equal result) ensures domain consistency.

## Implementation confidence

This example demonstrates that:

1. **Union types are a natural fit** for the constraint-based algorithm. No awkward workarounds or special cases needed.

2. **The lattice design handles composite types elegantly**: Whether it's a struct, union, or (hypothetically) tuple, the rule is the same: all constituent expressions must unify to a single domain.

3. **Pattern matching "just works"**: The match expression rule already anticipates destructuring of aggregate types. Union variants with payloads fit perfectly.

4. **Error messages will be clear**: The constraint system naturally preserves the source location (which union constructor had the conflict) and can identify exactly which payloads are incompatible.

5. **Constant payloads work correctly**: Literals in union payloads have domain `Constant`, which unifies with any `Sync` domain, allowing mixing of literal and computed values naturally.

The only implementation work needed is:
- Add the union constructor case to `walk_expression()` in constraint generation (5-10 lines)
- Ensure the AST representation of union constructors includes the list of payload expressions
- Error reporting should mention "union construction" in addition to the existing "struct construction" messaging

The constraint solving and signature extraction require zero changes.
