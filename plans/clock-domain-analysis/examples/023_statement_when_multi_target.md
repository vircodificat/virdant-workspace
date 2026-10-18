# Example 23: Statement-Level when with Multiple Targets

```virdant
mod StatementWhenMulti {
    incoming clock : Clock
    incoming condition : Bit
    incoming async_source : Word[8] async

    reg reg_a : Word[8] on clock {
        when {
            case condition {
                it <= 42w8
            }
            else {
                it <= 67w8
            }
        }
    }

    wire sync_wire : Word[8] {
        it := 0w8
    }

    wire async_wire : Word[8] async {
        it := 0w8
    }

    when {
        case condition {
            sync_wire := 10w8
            async_wire := async_source
        }
        else {
            sync_wire := 20w8
            async_wire := 99w8
        }
    }

    outgoing out_sync : Word[8] {
        it := sync_wire
    }

    outgoing out_async : Word[8] async {
        it := async_wire
    }
}
```

**What's happening**: A statement-level `when` block drives two targets (`sync_wire` and `async_wire`) with different domains. The key insight is that **each target is independently constrained** — `sync_wire` must match its RHS expressions (which are literals, so they unify to `sync_wire`'s domain), and `async_wire` must match its RHS expressions independently. There's no requirement that `sync_wire` and `async_wire` have the same domain.

Contrast this with an **expression-level** `when`:
```virdant
// This would be ERROR: all guards and arms must unify
wire result : Word[8] {
    it := when {
        case condition => sync_value  // Sync(clock)
        else => async_source          // Async
    }
}
```

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `condition` (incoming, type Bit)
- `async_source` (incoming, type Word[8], async)
- `reg_a` (reg, type Word[8])
- `sync_wire` (wire, type Word[8])
- `async_wire` (wire, type Word[8], async)
- `out_sync` (outgoing, type Word[8])
- `out_async` (outgoing, type Word[8], async)

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions

**Declaration: `incoming condition : Bit`**
- Component: `condition`
- Type: Bit
- No expressions

**Declaration: `incoming async_source : Word[8] async`**
- Component: `async_source`
- Type: Word[8]
- Has `async` modifier
- No expressions

**Declaration: `reg reg_a : Word[8] on clock`**
- Component: `reg_a`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver (inside statement-level when in reg_a block):**

This is a **statement-level when** inside a register driver block. Unlike expression-level when, this doesn't create a single when-expression. Instead, it's a control structure that conditionally executes driver statements.

Statement-level when case 1: `case condition`
- Expression `#1`: reference to `condition` (the guard)
- Expression `#2`: literal `42w8` (RHS of `it <= 42w8`)
- Driver: `reg_a <= #2`

Statement-level when case 2: `else`
- Expression `#3`: literal `67w8` (RHS of `it <= 67w8`)
- Driver: `reg_a <= #3`

**Declaration: `wire sync_wire : Word[8]`**
- Component: `sync_wire`
- No `async` modifier (implicitly has `dom(sync_wire) < Async`)
- Driver: `it := 0w8`
  - Expression `#4`: literal `0w8`
  - Driver: `sync_wire := #4`

**Declaration: `wire async_wire : Word[8] async`**
- Component: `async_wire`
- Has `async` modifier
- Driver: `it := 0w8`
  - Expression `#5`: literal `0w8`
  - Driver: `async_wire := #5`

**Statement-level when block (driving multiple targets):**

Statement-level when case 1: `case condition`
- Expression `#6`: reference to `condition` (the guard, reused)
- Expression `#7`: literal `10w8` (RHS of `sync_wire := 10w8`)
- Driver: `sync_wire := #7`
- Expression `#8`: reference to `async_source` (RHS of `async_wire := async_source`)
- Driver: `async_wire := #8`

Statement-level when case 2: `else`
- Expression `#9`: literal `20w8` (RHS of `sync_wire := 20w8`)
- Driver: `sync_wire := #9`
- Expression `#10`: literal `99w8` (RHS of `async_wire := 99w8`)
- Driver: `async_wire := #10`

**Declaration: `outgoing out_sync : Word[8]`**
- Component: `out_sync`
- Driver: `it := sync_wire`
  - Expression `#11`: reference to `sync_wire`
  - Driver: `out_sync := #11`

**Declaration: `outgoing out_async : Word[8] async`**
- Component: `out_async`
- Has `async` modifier
- Driver: `it := async_wire`
  - Expression `#12`: reference to `async_wire`
  - Driver: `out_async := #12`

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `condition` (guard in reg_a when)
- `#2`: Literal `42w8`
- `#3`: Literal `67w8`
- `#4`: Literal `0w8`
- `#5`: Literal `0w8`
- `#6`: ExprReference to `condition` (guard in multi-target when)
- `#7`: Literal `10w8`
- `#8`: ExprReference to `async_source`
- `#9`: Literal `20w8`
- `#10`: Literal `99w8`
- `#11`: ExprReference to `sync_wire`
- `#12`: ExprReference to `async_wire`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock`

**Fixed constraints from async declarations:**
- `dom(async_source) = Async` (explicitly async incoming port)
- `dom(async_wire) = Async` (explicitly async wire)
- `dom(out_async) = Async` (explicitly async outgoing port)

**Fixed constraints from register declarations:**
- `dom(reg_a) = Sync(ComponentId(clock))`

**Fixed constraints from literals (following SUMMARY.md recommendation):**
- `dom(#2) = Constant` (literal `42w8`)
- `dom(#3) = Constant` (literal `67w8`)
- `dom(#4) = Constant` (literal `0w8`)
- `dom(#5) = Constant` (literal `0w8`)
- `dom(#7) = Constant` (literal `10w8`)
- `dom(#9) = Constant` (literal `20w8`)
- `dom(#10) = Constant` (literal `99w8`)

**Equality constraints from component references:**
- `dom(#0) = dom(clock)`
- `dom(#1) = dom(condition)` (guard reference in reg_a when)
- `dom(#6) = dom(condition)` (guard reference in multi-target when)
- `dom(#8) = dom(async_source)`
- `dom(#11) = dom(sync_wire)`
- `dom(#12) = dom(async_wire)`

**Equality constraints from drivers:**

Each driver creates an independent equality constraint:
- `dom(reg_a) = dom(#2)` (from `reg_a <= 42w8` in when case 1)
- `dom(reg_a) = dom(#3)` (from `reg_a <= 67w8` in when else)
- `dom(sync_wire) = dom(#4)` (from initial `sync_wire := 0w8`)
- `dom(async_wire) = dom(#5)` (from initial `async_wire := 0w8`)
- `dom(sync_wire) = dom(#7)` (from `sync_wire := 10w8` in when case 1)
- `dom(async_wire) = dom(#8)` (from `async_wire := async_source` in when case 1)
- `dom(sync_wire) = dom(#9)` (from `sync_wire := 20w8` in when else)
- `dom(async_wire) = dom(#10)` (from `async_wire := 99w8` in when else)
- `dom(out_sync) = dom(#11)`
- `dom(out_async) = dom(#12)`

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(condition) < Async`
- `dom(reg_a) < Async`
- `dom(sync_wire) < Async` ⚠️ important: NOT async
- `dom(out_sync) < Async`

**Note**: No strict upper bound for `async_source`, `async_wire`, and `out_async` because they're explicitly `async`.

### Complete constraint system:
```
Fixed:
  F1:  dom(clock) = Clock
  F2:  dom(async_source) = Async
  F3:  dom(async_wire) = Async
  F4:  dom(out_async) = Async
  F5:  dom(reg_a) = Sync(ComponentId(clock))
  F6:  dom(#2) = Constant
  F7:  dom(#3) = Constant
  F8:  dom(#4) = Constant
  F9:  dom(#5) = Constant
  F10: dom(#7) = Constant
  F11: dom(#9) = Constant
  F12: dom(#10) = Constant

Strict upper bound (implicit):
  S1: dom(clock) < Async
  S2: dom(condition) < Async
  S3: dom(reg_a) < Async
  S4: dom(sync_wire) < Async
  S5: dom(out_sync) < Async

Equality (component references):
  E1: dom(#0) = dom(clock)
  E2: dom(#1) = dom(condition)
  E3: dom(#6) = dom(condition)
  E4: dom(#8) = dom(async_source)
  E5: dom(#11) = dom(sync_wire)
  E6: dom(#12) = dom(async_wire)

Equality (drivers - each target independently):
  E7:  dom(reg_a) = dom(#2)        ← reg_a driven by 42w8
  E8:  dom(reg_a) = dom(#3)        ← reg_a driven by 67w8
  E9:  dom(sync_wire) = dom(#4)    ← sync_wire initial driver
  E10: dom(async_wire) = dom(#5)   ← async_wire initial driver
  E11: dom(sync_wire) = dom(#7)    ← sync_wire driven by 10w8 (when case)
  E12: dom(async_wire) = dom(#8)   ← async_wire driven by async_source (when case)
  E13: dom(sync_wire) = dom(#9)    ← sync_wire driven by 20w8 (else case)
  E14: dom(async_wire) = dom(#10)  ← async_wire driven by 99w8 (else case)
  E15: dom(out_sync) = dom(#11)    ← out_sync driven by sync_wire
  E16: dom(out_async) = dom(#12)   ← out_async driven by async_wire
```

**Key observation**: Notice that there are **no constraints** linking `sync_wire` and `async_wire` together. Each target is constrained only by its own drivers (E9, E11, E13 for `sync_wire`; E10, E12, E14 for `async_wire`). This is the fundamental difference from expression-level when.

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment that satisfies all constraints.

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock}, {condition}, {async_source}, {reg_a}, {sync_wire}, {async_wire}, 
{out_sync}, {out_async},
{#0}, {#1}, {#2}, {#3}, {#4}, {#5}, {#6}, {#7}, {#8}, {#9}, {#10}, {#11}, {#12}
```

### Processing fixed constraints F1-F12:

Assign concrete domains:
```
{clock: Clock},
{condition},
{async_source: Async},
{reg_a: Sync(clock)},
{sync_wire},
{async_wire: Async},
{out_sync},
{out_async: Async},
{#0}, {#1}, {#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#6}, {#7: Constant}, {#8}, {#9: Constant}, {#10: Constant}, {#11}, {#12}
```

### Processing equality constraint E1: `dom(#0) = dom(clock)`

Merge #0 with clock's class:
```
{clock, #0: Clock}, {condition}, {async_source: Async}, {reg_a: Sync(clock)},
{sync_wire}, {async_wire: Async}, {out_sync}, {out_async: Async},
{#1}, {#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#6}, {#7: Constant}, {#8}, {#9: Constant}, {#10: Constant}, {#11}, {#12}
```

### Processing equality constraint E2: `dom(#1) = dom(condition)`

Merge #1 with condition's class:
```
{clock, #0: Clock}, {condition, #1}, {async_source: Async}, {reg_a: Sync(clock)},
{sync_wire}, {async_wire: Async}, {out_sync}, {out_async: Async},
{#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#6}, {#7: Constant}, {#8}, {#9: Constant}, {#10: Constant}, {#11}, {#12}
```

### Processing equality constraint E3: `dom(#6) = dom(condition)`

Merge #6 with condition's class:
```
{clock, #0: Clock}, {condition, #1, #6}, {async_source: Async}, {reg_a: Sync(clock)},
{sync_wire}, {async_wire: Async}, {out_sync}, {out_async: Async},
{#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#7: Constant}, {#8}, {#9: Constant}, {#10: Constant}, {#11}, {#12}
```

### Processing equality constraint E4: `dom(#8) = dom(async_source)`

Merge #8 with async_source's class:
```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, {reg_a: Sync(clock)},
{sync_wire}, {async_wire: Async}, {out_sync}, {out_async: Async},
{#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#7: Constant}, {#9: Constant}, {#10: Constant}, {#11}, {#12}
```

### Processing equality constraint E5: `dom(#11) = dom(sync_wire)`

Merge #11 with sync_wire's class:
```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, {reg_a: Sync(clock)},
{sync_wire, #11}, {async_wire: Async}, {out_sync}, {out_async: Async},
{#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#7: Constant}, {#9: Constant}, {#10: Constant}, {#12}
```

### Processing equality constraint E6: `dom(#12) = dom(async_wire)`

Merge #12 with async_wire's class:
```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, {reg_a: Sync(clock)},
{sync_wire, #11}, {async_wire, #12: Async}, {out_sync}, {out_async: Async},
{#2: Constant}, {#3: Constant}, {#4: Constant}, {#5: Constant},
{#7: Constant}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E7: `dom(reg_a) = dom(#2)`

Merge reg_a's class with #2's class:
- reg_a's class: `{reg_a: Sync(clock)}` (concrete domain)
- #2's class: `{#2: Constant}` (concrete domain)
- Check compatibility: `compatible(Sync(clock), Constant)`? **Yes** ✓
- Constant unifies with any Sync → unified domain is `Sync(clock)`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, 
{reg_a, #2: Sync(clock)}, {sync_wire, #11}, {async_wire, #12: Async}, 
{out_sync}, {out_async: Async}, {#3: Constant}, {#4: Constant}, 
{#5: Constant}, {#7: Constant}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E8: `dom(reg_a) = dom(#3)`

Merge reg_a's class with #3's class:
- reg_a's class: `{reg_a, #2: Sync(clock)}` (concrete domain)
- #3's class: `{#3: Constant}` (concrete domain)
- Check compatibility: `compatible(Sync(clock), Constant)`? **Yes** ✓
- Unified domain: `Sync(clock)`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11}, {async_wire, #12: Async}, 
{out_sync}, {out_async: Async}, {#4: Constant}, {#5: Constant}, 
{#7: Constant}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E9: `dom(sync_wire) = dom(#4)`

Merge sync_wire's class with #4's class:
- sync_wire's class: `{sync_wire, #11}` (no concrete domain yet)
- #4's class: `{#4: Constant}` (concrete domain)
- Result: merged class gets `Constant`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4: Constant}, 
{async_wire, #12: Async}, {out_sync}, {out_async: Async}, 
{#5: Constant}, {#7: Constant}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E10: `dom(async_wire) = dom(#5)`

Merge async_wire's class with #5's class:
- async_wire's class: `{async_wire, #12: Async}` (concrete domain)
- #5's class: `{#5: Constant}` (concrete domain)
- Check compatibility: `compatible(Async, Constant)`? **No** ❌

**Wait** — Let me reconsider. According to the lattice:
- Constant is the bottom
- Constant can unify with any Sync domain (becoming that Sync)
- Can Constant unify with Async?

Looking at ALGORITHM.md's `compatible()` function:
```python
def compatible(d1: Domain, d2: Domain) -> bool:
    if d1 == d2:
        return True
    # Constant unifies with any Sync
    if d1 == Constant and isinstance(d2, Sync):
        return True
    if d2 == Constant and isinstance(d1, Sync):
        return True
    # Different Sync domains are incompatible
    if isinstance(d1, Sync) and isinstance(d2, Sync):
        return d1.clock == d2.clock
    # Async, Clock, Reset don't unify with anything except themselves
    return False
```

The last line says "Async, Clock, Reset don't unify with anything except themselves."

So `compatible(Async, Constant)` returns `False`. This would be an error!

But wait — the component `async_wire` is declared `async`, so it has a **fixed** constraint `dom(async_wire) = Async`. And `#5` is a literal with `dom(#5) = Constant`. The driver `async_wire := 0w8` tries to unify these, which should fail...

Actually, let me reconsider the semantics. When we have:
```virdant
wire async_wire : Word[8] async {
    it := 0w8
}
```

The `async` modifier means the wire is in the Async domain. The RHS `0w8` is a constant. According to PLANv2.md:
> **Constant** is the bottom: unifies with any `Sync(ComponentId)`, resulting in that `Sync`. Can also upcast to `Async`.

Ah! "Can also upcast to `Async`" — this suggests Constant should be compatible with Async. Let me check PLANv2.md more carefully...

Looking at the lattice diagram in PLANv2.md:
```
         Async (top)
        /  |  \
   Sync(C1) Sync(C2) Sync(C3) ...
        \  |  /
      Constant (bottom)
```

Constant is at the bottom of the lattice, and Async is at the top. In a lattice, anything can upcast to the top. So Constant should be able to upcast to Async.

But ALGORITHM.md's `compatible()` function doesn't include this case. This might be a gap in the algorithm specification. Let me check if this is mentioned in SUMMARY.md...

Looking at SUMMARY.md, it recommends:
> **Fix literal constraint generation** (walk_expression):
> ```python
> elif expr.kind == Literal:
>     expr_node = expr_id
>     expr_id += 1
>     variables.add(expr_node)
>     fixed.append(Fixed(expr_node, Constant))  # ← ADD THIS
>     return (expr_id, expr_node)
> ```

And it notes that literals should have domain Constant.

I think the issue is that when an async target is driven by a constant literal, the constant should upcast to Async. This is similar to how binary operators use **upper bound constraints** to allow operands to upcast to the result.

Actually, looking more carefully at ALGORITHM.md, I see that for **drivers**, the constraint generated is:
```python
# Target driven by RHS expression
equalities.append(Equality(target, rhs_expr))
```

This creates a strict equality: `dom(target) = dom(rhs_expr)`. But for upcasting to work (like Constant → Async), we might need an **upper bound** constraint instead: `dom(rhs_expr) ≤ dom(target)`.

Hmm, but looking at the algorithm examples (001, 002, etc.), drivers always use equality constraints. Let me reconsider...

Actually, I think the key insight is that the equality constraint E10: `dom(async_wire) = dom(#5)` tries to unify two fixed domains: `Async` and `Constant`. According to the `compatible()` function, these don't unify.

But according to PLANv2.md, constants should be able to upcast to Async. This suggests the `compatible()` function needs to be updated to allow `Constant` to unify with `Async`.

Let me check if there's a specific rule about this in PLANv2.md...

PLANv2.md states:
> - **Constant** is the bottom: unifies with any `Sync(ComponentId)`, resulting in that `Sync`. Can also upcast to `Async`.

And:
> - **Any domain** can upcast to `Async` when the target is explicitly declared `async`.

So yes, Constant should upcast to Async. The `compatible()` function should include:
```python
if (d1 == Constant and d2 == Async) or (d2 == Constant and d1 == Async):
    return True
```

And the `unify()` function should return:
```python
if d1 == Constant:
    return d2  # Constant upcasts to anything
if d2 == Constant:
    return d1
```

So the algorithm as specified in ALGORITHM.md has a gap — it doesn't handle Constant → Async unification. The SUMMARY.md notes mention fixing literals to be Constant but don't explicitly address this unification gap.

For the purpose of this example, I'll note this as a discovery in the Notes section and proceed with the constraint solving assuming the fix is in place. The corrected `compatible()` would be:

```python
def compatible(d1: Domain, d2: Domain) -> bool:
    if d1 == d2:
        return True
    # Constant unifies with any Sync, Async, Clock, Reset
    if d1 == Constant or d2 == Constant:
        return True
    # Different Sync domains are incompatible
    if isinstance(d1, Sync) and isinstance(d2, Sync):
        return d1.clock == d2.clock
    # Async, Clock, Reset don't unify with each other
    return False
```

Continuing with constraint solving...

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4: Constant}, 
{async_wire, #12, #5: Async}, {out_sync}, {out_async: Async}, 
{#7: Constant}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E11: `dom(sync_wire) = dom(#7)`

Merge sync_wire's class with #7's class:
- sync_wire's class: `{sync_wire, #11, #4: Constant}`
- #7's class: `{#7: Constant}`
- Both have `Constant` → compatible and unified domain is `Constant`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7: Constant}, 
{async_wire, #12, #5: Async}, {out_sync}, {out_async: Async}, 
{#9: Constant}, {#10: Constant}
```

### Processing equality constraint E12: `dom(async_wire) = dom(#8)`

Merge async_wire's class with #8's class:
- async_wire's class: `{async_wire, #12, #5: Async}`
- #8's class: `{async_source, #8: Async}`
- Both have `Async` → compatible and unified domain is `Async`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8, async_wire, #12, #5: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7: Constant}, 
{out_sync}, {out_async: Async}, {#9: Constant}, {#10: Constant}
```

### Processing equality constraint E13: `dom(sync_wire) = dom(#9)`

Merge sync_wire's class with #9's class:
- sync_wire's class: `{sync_wire, #11, #4, #7: Constant}`
- #9's class: `{#9: Constant}`
- Both have `Constant` → compatible and unified domain is `Constant`

```
{clock, #0: Clock}, {condition, #1, #6}, {async_source, #8, async_wire, #12, #5: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7, #9: Constant}, 
{out_sync}, {out_async: Async}, {#10: Constant}
```

### Processing equality constraint E14: `dom(async_wire) = dom(#10)`

Merge async_wire's class with #10's class:
- async_wire's class: `{async_source, #8, async_wire, #12, #5: Async}`
- #10's class: `{#10: Constant}`
- Check compatibility: `compatible(Async, Constant)`? With the fixed `compatible()`, **Yes** ✓
- Unified domain: `Async` (Constant upcasts)

```
{clock, #0: Clock}, {condition, #1, #6}, 
{async_source, #8, async_wire, #12, #5, #10: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7, #9: Constant}, 
{out_sync}, {out_async: Async}
```

### Processing equality constraint E15: `dom(out_sync) = dom(#11)`

Merge out_sync with #11's class:
- #11's class: `{sync_wire, #11, #4, #7, #9: Constant}`
- out_sync's class: `{out_sync}` (no concrete domain)
- Result: out_sync joins and gets `Constant`

```
{clock, #0: Clock}, {condition, #1, #6}, 
{async_source, #8, async_wire, #12, #5, #10: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7, #9, out_sync: Constant}, 
{out_async: Async}
```

### Processing equality constraint E16: `dom(out_async) = dom(#12)`

Merge out_async with #12's class:
- #12's class: `{async_source, #8, async_wire, #12, #5, #10: Async}`
- out_async's class: `{out_async: Async}` (already has Async from fixed constraint)
- Both have `Async` → compatible

```
{clock, #0: Clock}, {condition, #1, #6}, 
{async_source, #8, async_wire, #12, #5, #10, out_async: Async}, 
{reg_a, #2, #3: Sync(clock)}, {sync_wire, #11, #4, #7, #9, out_sync: Constant}
```

### Checking strict upper bound constraints:

**S1: `dom(clock) < Async`**
- `dom(clock) = Clock`
- Is `Clock < Async`? **Yes** ✓

**S2: `dom(condition) < Async`**
- `dom(condition)` is unresolved (no fixed domain or equality with fixed components)
- With no upper bounds forcing it, let's trace through...
  Actually, `condition` is in class `{condition, #1, #6}` with no concrete domain assigned yet.
  
Let me reconsider: After all equality constraints are processed, we have unresolved classes. According to ALGORITHM.md Pass 2 Step 3:
> For unresolved classes, use upper bound constraints

In this example, `condition` has no upper bound constraints, and it's not unified with any fixed component. So it remains a free variable. But it has a strict upper bound constraint `dom(condition) < Async`.

For unresolved classes with no concrete domain and no upper bounds, they remain unresolved. But the strict upper bound `< Async` means that when this module is instantiated, `condition` must be bound to something that is strictly less than Async (i.e., any Sync domain, Clock, Reset, or Constant, but not Async).

Actually, let me think about this differently. The `condition` is an incoming port with type Bit. It has no `async` modifier, so it has the implicit strict upper bound `dom(condition) < Async`. During constraint solving, it's not unified with any component that has a fixed domain, so it remains unresolved.

This is correct! The `condition` port is unconstrained internally (not tied to any register or clock), so it becomes a free variable in the signature. At instantiation time, whatever signal is connected to `condition` will determine its domain, as long as it's not Async.

So after all constraints are processed, the final partition is:
```
Class 1: {clock, #0} → Clock
Class 2: {condition, #1, #6} → (unresolved, will be free in signature)
Class 3: {async_source, #8, async_wire, #12, #5, #10, out_async} → Async
Class 4: {reg_a, #2, #3} → Sync(ComponentId(clock))
Class 5: {sync_wire, #11, #4, #7, #9, out_sync} → Constant
```

### Checking strict upper bound constraints (continued):

**S2: `dom(condition) < Async`**
- `dom(condition)` is unresolved (free variable)
- Strict upper bound still applies at instantiation time ✓

**S3: `dom(reg_a) < Async`**
- `dom(reg_a) = Sync(clock)`
- Is `Sync(clock) < Async`? **Yes** ✓

**S4: `dom(sync_wire) < Async`**
- `dom(sync_wire) = Constant`
- Is `Constant < Async`? **Yes** ✓

**S5: `dom(out_sync) < Async`**
- `dom(out_sync) = Constant`
- Is `Constant < Async`? **Yes** ✓

All constraints satisfied! ✓

### Final solution (domain assignment):
```
Components:
  dom(clock) = Clock
  dom(condition) = (free variable, constrained to < Async)
  dom(async_source) = Async
  dom(reg_a) = Sync(ComponentId(clock))
  dom(sync_wire) = Constant
  dom(async_wire) = Async
  dom(out_sync) = Constant
  dom(out_async) = Async

Expressions:
  dom(#0) = Clock
  dom(#1) = (free variable, same as condition)
  dom(#2) = Sync(ComponentId(clock))
  dom(#3) = Sync(ComponentId(clock))
  dom(#4) = Constant
  dom(#5) = Async
  dom(#6) = (free variable, same as condition)
  dom(#7) = Constant
  dom(#8) = Async
  dom(#9) = Constant
  dom(#10) = Async
  dom(#11) = Constant
  dom(#12) = Async
```

---

## Pass 3: Signature Extraction

Now we build the module's polymorphic domain signature for use at instantiation sites.

### Identify domain parameters:

Clock ports introduce domain parameters:
- `clock` is Clock-typed → introduces `DomainVar(0)` (call it α)

### Express port domains in terms of parameters:

From the solution:
- `dom(clock) = Clock` → `Concrete(Clock)`
- `dom(condition)` = free variable → `Free()` (unconstrained)
- `dom(async_source) = Async` → `Concrete(Async)`
- `dom(out_sync) = Constant` → `Concrete(Constant)`
- `dom(out_async) = Async` → `Concrete(Async)`

### Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        clock: Concrete(Clock),
        condition: Free(),                 // unconstrained
        async_source: Concrete(Async),
        out_sync: Concrete(Constant),
        out_async: Concrete(Async),
    }
}
```

### Final DomainAnalysis result:
```rust
DomainAnalysis {
    moddef: StatementWhenMulti,
    component_domains: {
        clock: Clock,
        condition: /* free variable */,
        async_source: Async,
        reg_a: Sync(ComponentId(clock)),
        sync_wire: Constant,
        async_wire: Async,
        out_sync: Constant,
        out_async: Async,
    },
    signature: /* as above */,
    diagnostics: [],
    unsafe_clocks: false,
}
```

---

## Notes

### Does this work naturally with the algorithm as-is?

**Almost, with one important fix needed.** The constraint generation and solving work naturally for statement-level when blocks. Each driver inside the when block creates independent equality constraints, with no requirement that different targets have related domains. This is exactly the intended behavior.

However, there's an algorithm gap discovered: **Constant domain must be able to unify with Async**. The current `compatible()` function in ALGORITHM.md doesn't allow this, but PLANv2.md states that "Constant... can also upcast to `Async`" and "Any domain can upcast to `Async` when the target is explicitly declared `async`."

**Fix needed for ALGORITHM.md:**

```python
def compatible(d1: Domain, d2: Domain) -> bool:
    if d1 == d2:
        return True
    # Constant unifies with anything (it's the bottom of the lattice)
    if d1 == Constant or d2 == Constant:
        return True
    # Different Sync domains are incompatible
    if isinstance(d1, Sync) and isinstance(d2, Sync):
        return d1.clock == d2.clock
    # Async, Clock, Reset don't unify with each other
    return False

def unify(d1: Domain, d2: Domain) -> Domain:
    if d1 == d2:
        return d1
    # Constant always weakens to the other domain
    if d1 == Constant:
        return d2
    if d2 == Constant:
        return d1
    # Should not reach here if compatible() returned True
    raise Exception(f"Cannot unify {d1} and {d2}")
```

### Surprises or edge cases?

**No surprises with the statement-level when itself.** The key insights are:

1. **Independent constraints per target**: Unlike expression-level when (which creates a single result that all guards and arms must unify with), statement-level when generates separate driver constraints for each target. This naturally allows different targets to have different domains.

2. **Guard expressions are just references**: The guard `condition` is evaluated (creating expression nodes #1 and #6), but these don't constrain the driver targets. The guard is used for control flow, not domain unification.

3. **Multiple drivers to the same target unify**: Both `sync_wire := 10w8` and `sync_wire := 20w8` create equality constraints with `sync_wire`, which transitively unify all the RHS expressions. This is correct — multiple drivers to the same signal must have compatible domains.

4. **Literal upcasting to Async**: As discovered, literals (Constant domain) must be able to upcast to Async when driving an async target. This works through the unification system with the corrected `compatible()` function.

5. **Free variables are common**: The `condition` input port ends up as a free variable because it's not constrained by any register or fixed domain internally. This is correct and expected behavior.

### Does the constraint system handle this without special cases?

**Yes, perfectly.** The constraint system handles statement-level when without any special-case logic:

- Each driver generates `Equality(target, rhs_expr)` independently
- Guards generate expression nodes but don't create cross-target constraints
- Multiple drivers to the same target naturally unify through transitivity
- Different targets remain independent

The only thing that differs from expression-level when is the **absence** of constraints linking all the pieces together. For statement-level when, there's no "when-result" node that everything must equal.

### Clarifications needed for ALGORITHM.md or PLANv2.md?

**Critical updates needed:**

1. **ALGORITHM.md `compatible()` and `unify()` functions** must handle Constant domain correctly:
   - Constant should unify with Async (and Clock, Reset, any Sync)
   - Constant is the bottom of the lattice — it upcasts to anything
   
2. **ALGORITHM.md should explicitly document statement-level when**:
   - Currently, `walk_driver()` and `walk_expression()` are shown, but statement-level control flow isn't explicitly mentioned
   - Add a note that statement-level `when` and `match` desugar to multiple driver calls
   - Each driver inside a statement-level when generates independent constraints
   
   Suggested addition:
   ```python
   # Statement-level when/match blocks
   # 
   # A statement-level when/match is a control structure that conditionally
   # executes driver statements. For domain analysis, it's treated as a
   # sequence of conditional drivers, each generating independent constraints.
   #
   # Example:
   #   when {
   #       case guard { target1 := expr1; target2 := expr2 }
   #       else { target1 := expr3; target3 := expr4 }
   #   }
   #
   # Generates:
   #   - Expression nodes for guard, expr1, expr2, expr3, expr4
   #   - Equality constraints: dom(target1) = dom(expr1), dom(target1) = dom(expr3)
   #   - Equality constraints: dom(target2) = dom(expr2), dom(target3) = dom(expr4)
   #   - Note: No constraint linking target1, target2, target3 together
   ```

3. **PLANv2.md Section 3 (Control Flow Expressions) should distinguish expression vs statement**:
   - Current text describes expression-level when/match
   - Add subsection clearly distinguishing statement-level forms
   - Emphasize that statement-level when has **independent per-target constraints**
   
   Suggested addition:
   > **Statement-level vs Expression-level Control Flow**
   > 
   > Virdant has two forms of `when` and `match`:
   > 
   > **Expression-level** (RHS of a driver, produces a value):
   > ```virdant
   > wire result := when { case guard => expr1; else => expr2 }
   > ```
   > Rule: All guards and arms must unify to a single domain.
   > 
   > **Statement-level** (control structure, executes driver statements):
   > ```virdant
   > when {
   >     case guard { target1 := expr1; target2 := expr2 }
   >     else { target1 := expr3 }
   > }
   > ```
   > Rule: Each target is independently constrained by its drivers. No requirement that target1 and target2 have related domains.

### Algorithm gaps?

**One gap found and fixed:**

The `compatible()` function in ALGORITHM.md needs to handle Constant domain unifying with all other domains (since Constant is the bottom of the lattice). Without this, driving an async signal with a literal would incorrectly fail.

With this fix applied, the algorithm handles statement-level when with multiple targets perfectly. No other gaps or special cases needed.
