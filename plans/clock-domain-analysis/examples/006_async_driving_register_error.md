# Example 6: Async Driving Register (ERROR)

```virdant
mod AsyncRegisterError {
    incoming clock : Clock
    incoming async_data : Word[8] async

    reg buffer : Word[8] on clock {
        it <= async_data  // ERROR: async signal drives register
    }

    outgoing out : Word[8] {
        it := buffer
    }
}
```

**What's happening**: An `async` signal tries to drive a register directly. This violates the fundamental rule that registers must be driven by signals on the same clock domain. The equality constraint forces `Sync(clock) = Async`, which is a unification error.

**Expected error**: Domain unification error when trying to equate `Sync(ComponentId(clock))` with `Async`.

---

## Pass 1: Constraint Generation

We walk the module's AST to generate domain constraints.

### Step 1 - Components from ComponentAnalysis:
- `clock` (incoming, type Clock)
- `async_data` (incoming, type Word[8], async modifier)
- `buffer` (reg, type Word[8])
- `out` (outgoing, type Word[8])

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions

**Declaration: `incoming async_data : Word[8] async`**
- Component: `async_data`
- Has `async` modifier
- No expressions in declaration

**Declaration: `reg buffer : Word[8] on clock`**
- Component: `buffer`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Declaration: `outgoing out : Word[8]`**
- Component: `out`
- No expressions in declaration

**Driver: `buffer <= async_data`**
- Expression `#1`: reference to `async_data` (RHS of driver)

**Driver: `out := buffer`**
- Expression `#2`: reference to `buffer` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `async_data`
- `#2`: ExprReference to `buffer`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from async modifier:**
- `dom(async_data) = Async` (explicitly declared async)

**Fixed constraints from register declarations:**
- `dom(buffer) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `async_data` → `dom(#1) = dom(async_data)`
- Expression `#2` references `buffer` → `dom(#2) = dom(buffer)`

**Equality constraints from drivers:**

Driver: `buffer <= async_data`
- LHS: component `buffer`
- RHS: expression `#1`
- Constraint: `dom(buffer) = dom(#1)`

Driver: `out := buffer`
- LHS: component `out`
- RHS: expression `#2`
- Constraint: `dom(out) = dom(#2)`

**Implicit strict upper bound constraints (all non-async components):**
- `dom(clock) < Async`
- `dom(buffer) < Async`
- `dom(out) < Async`
- **NOTE:** `async_data` does NOT get this constraint because it's declared `async`

### Complete constraint system:
```
Fixed:
  F1: dom(clock) = Clock
  F2: dom(async_data) = Async
  F3: dom(buffer) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  S1: dom(clock) < Async
  S2: dom(buffer) < Async
  S3: dom(out) < Async

Equality:
  E1: dom(#0) = dom(clock)
  E2: dom(#1) = dom(async_data)
  E3: dom(#2) = dom(buffer)
  E4: dom(buffer) = dom(#1)          ← THE PROBLEM
  E5: dom(out) = dom(#2)
```

---

## Pass 2: Constraint Solving

We solve the constraint system to find a domain assignment (or detect the error).

### Initial state:
All components and expressions start in separate equivalence classes:
```
{clock}, {async_data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

### Processing fixed constraint F1: `dom(clock) = Clock`

Assign concrete domain `Clock` to clock's equivalence class:
```
{clock: Clock}, {async_data}, {buffer}, {out}, {#0}, {#1}, {#2}
```

### Processing fixed constraint F2: `dom(async_data) = Async`

Assign concrete domain `Async` to async_data's equivalence class:
```
{clock: Clock}, {async_data: Async}, {buffer}, {out}, {#0}, {#1}, {#2}
```

### Processing fixed constraint F3: `dom(buffer) = Sync(ComponentId(clock))`

Assign concrete domain `Sync(clock)` to buffer's equivalence class:
```
{clock: Clock}, {async_data: Async}, {buffer: Sync(clock)}, {out}, {#0}, {#1}, {#2}
```

### Processing equality constraint E1: `dom(#0) = dom(clock)`

Merge expression #0 with clock's equivalence class:
```
{clock, #0: Clock}, {async_data: Async}, {buffer: Sync(clock)}, {out}, {#1}, {#2}
```

### Processing equality constraint E2: `dom(#1) = dom(async_data)`

Merge expression #1 with async_data's equivalence class:
```
{clock, #0: Clock}, {async_data, #1: Async}, {buffer: Sync(clock)}, {out}, {#2}
```

### Processing equality constraint E3: `dom(#2) = dom(buffer)`

Merge expression #2 with buffer's equivalence class:
```
{clock, #0: Clock}, {async_data, #1: Async}, {buffer, #2: Sync(clock)}, {out}
```

### Processing equality constraint E4: `dom(buffer) = dom(#1)` ⚠️ ERROR

Attempt to merge buffer's class with #1's class:
- buffer's class: `{buffer, #2: Sync(clock)}` (has concrete domain `Sync(clock)`)
- #1's class: `{async_data, #1: Async}` (has concrete domain `Async`)

Check compatibility: `compatible(Sync(clock), Async)`?
- `Sync(clock) == Async`? No
- `Sync(clock) == Constant` or `Async == Constant`? No
- Both are `Sync` with same clock? No (one is `Async`)
- **Result: INCOMPATIBLE** ❌

**ERROR DETECTED**: Cannot unify `Sync(ComponentId(clock))` with `Async`

Error message:
```
Domain unification error at driver `buffer <= async_data`:
  Left-hand side `buffer` has domain Sync(clock)
  Right-hand side `async_data` has domain Async

The register `buffer` is on clock domain `clock`, but is being driven
by an async signal `async_data`. Registers must be driven by signals
on the same clock domain.

Help: If you need to cross clock domains, use the Sync module:
  wire synced : Word[8] { it := Sync(async_data, clock) }
  reg buffer : Word[8] on clock { it <= synced }
```

### Solver terminates with error

The solver stops processing further constraints and returns the error.

**Solve result:**
```python
SolveResult {
    union_find: /* partial state */,
    errors: [
        "Domain unification error: cannot merge Sync(ComponentId(clock)) with Async
         at constraint E4: dom(buffer) = dom(#1)
         from driver: buffer <= async_data"
    ]
}
```

---

## Pass 3: Signature Extraction (SKIPPED)

Since domain analysis found errors, signature extraction is skipped. The module is invalid.

---

## Final DomainAnalysis Result

```rust
DomainAnalysis {
    moddef: AsyncRegisterError,
    component_domains: {
        clock: Clock,
        async_data: Async,
        buffer: Sync(ComponentId(clock)),  // partial result
        out: /* unresolved */,
    },
    signature: /* invalid */,
    diagnostics: [
        Diagnostic {
            severity: Error,
            message: "Domain unification error: async signal cannot drive register",
            location: /* driver: buffer <= async_data */,
            help: "Use the Sync module to cross clock domains",
        }
    ],
    unsafe_clocks: false,
}
```

---

## Key Insights

1. **The lattice naturally catches this error**: No special-case logic needed. The constraint system forces `Sync(clock) = Async` which violates `compatible()`.

2. **Error is detected during union-find merges**: When processing equality constraints, attempting to merge two classes with incompatible concrete domains fails immediately.

3. **Fixed constraints establish the conflict**:
   - Register declaration fixes `dom(buffer) = Sync(clock)`
   - Async modifier fixes `dom(async_data) = Async`
   - Driver equality tries to merge them: INCOMPATIBLE

4. **The error message can be domain-specific**: Since we know one side is a register and the other is async, we can provide targeted help (use the `Sync` buffer module).

5. **Partial results are still meaningful**: Even though solving failed, we collected partial domain information that can be used for error reporting and IDE tooling.
