# Example 14: The @unsafe_clocks Sync Module

```virdant
@unsafe_clocks
mod Sync {
    incoming inp : Bit async
    incoming clock : Clock
    outgoing reg out : Bit on clock {
        it <= inp
    }
}
```

**What's happening**: The `Sync` module is the primitive for crossing clock domains. It has an explicitly `async` input port, a clock input, and a synchronized output register. The body contains an assignment `out <= inp` that would normally be a domain error (async signal driving a register), but the `@unsafe_clocks` annotation suppresses internal domain checking.

**Expected behavior**: The `@unsafe_clocks` annotation causes the domain analysis algorithm to skip analyzing the module body and instead synthesize a polymorphic signature directly from the port declarations. When `Sync` is instantiated in another module, the compiler still performs full domain checking using this synthesized signature.

---

## Pass 1: Constraint Generation (Hypothetical)

Let's first examine what would happen if we tried to analyze the body of `Sync` like a normal module (ignoring `@unsafe_clocks` for now).

### Step 1 - Components from ComponentAnalysis:
- `inp` (incoming, type Bit, async modifier)
- `clock` (incoming, type Clock)
- `out` (outgoing reg, type Bit)

### Step 2 - Walk declarations, identify expressions:

**Declaration: `incoming inp : Bit async`**
- Component: `inp`
- Has `async` modifier
- No expressions in declaration

**Declaration: `incoming clock : Clock`**
- Component: `clock`
- Type: Clock
- No expressions in declaration

**Declaration: `outgoing reg out : Bit on clock`**
- Component: `out`
- Has `on clock` clause
- Expression `#0`: reference to `clock` (in the `on clock` clause)

**Driver: `out <= inp`**
- Expression `#1`: reference to `inp` (RHS of driver)

**Expression inventory:**
- `#0`: ExprReference to `clock`
- `#1`: ExprReference to `inp`

### Step 3 - Generate constraints:

**Fixed constraints from component types:**
- `dom(clock) = Clock` (Clock-typed signal)

**Fixed constraints from async modifier:**
- `dom(inp) = Async` (explicitly declared async)

**Fixed constraints from register declarations:**
- `dom(out) = Sync(ComponentId(clock))` (register with `on clock` clause)

**Equality constraints from component references:**
- Expression `#0` references `clock` → `dom(#0) = dom(clock)`
- Expression `#1` references `inp` → `dom(#1) = dom(inp)`

**Equality constraints from drivers:**

Driver: `out <= inp`
- LHS: component `out`
- RHS: expression `#1`
- Constraint: `dom(out) = dom(#1)`  ← **THE PROBLEM**

**Implicit strict upper bound constraints:**
- `dom(clock) < Async`
- `dom(out) < Async`
- **NOTE:** `inp` does NOT get this constraint because it's declared `async`

### Complete constraint system (hypothetical):
```
Fixed:
  F1: dom(clock) = Clock
  F2: dom(inp) = Async
  F3: dom(out) = Sync(ComponentId(clock))

Strict upper bound (implicit):
  S1: dom(clock) < Async
  S2: dom(out) < Async

Equality:
  E1: dom(#0) = dom(clock)
  E2: dom(#1) = dom(inp)
  E3: dom(out) = dom(#1)  ← This would force Sync(clock) = Async
```

---

## Pass 2: The Conflict (and @unsafe_clocks Escape Hatch)

If we proceeded with constraint solving on the hypothetical constraint system above:

**The inevitable error:**
- Constraint E3 requires: `dom(out) = dom(#1)`
- But E2 forces: `dom(#1) = dom(inp) = Async`
- And F3 fixes: `dom(out) = Sync(ComponentId(clock))`
- Therefore: `Sync(ComponentId(clock)) = Async`

**This is a unification error** — `Sync(clock)` and `Async` are incompatible domains in the lattice.

### The @unsafe_clocks Annotation

This is exactly the situation `@unsafe_clocks` exists to handle. The `Sync` module performs a clock domain crossing at the hardware level (using a D flip-flop primitive), which is outside the domain safety system. The assignment `out <= inp` is semantically valid hardware (an async signal feeding a flip-flop's D input), but it violates the high-level domain calculus.

**Effect of @unsafe_clocks:**

From ALGORITHM.md:
```python
def analyze_module_domains(
    module: Module,
    child_analyses: Dict[SymbolId, DomainAnalysis],
) -> DomainAnalysis:
    # Special case: @unsafe_clocks modules
    if module.has_annotation("unsafe_clocks"):
        return unsafe_clocks_signature(module)

    # Normal three-pass analysis...
```

When `@unsafe_clocks` is present:
1. **Skip Pass 1 (Constraint Generation)** — do not walk the module body
2. **Skip Pass 2 (Constraint Solving)** — no constraints to solve
3. **Synthesize signature from port declarations only** — trust the module author

---

## Pass 3: Signature Synthesis (Special Handling)

For `@unsafe_clocks` modules, we synthesize the domain signature directly from port declarations without analyzing the body.

### Step 1: Identify domain parameters from port declarations

Clock ports introduce domain parameters:
- `clock` is Clock-typed incoming port → introduces `DomainVar(0)` (call it α)

### Step 2: Synthesize port domains from declarations

**Port: `inp`**
- Declared type: `Bit`
- Has `async` modifier
- Domain: `Async` (concrete)

**Port: `clock`**
- Declared type: `Clock`
- Domain: `Clock` (concrete, not a domain parameter itself)
- Introduces domain parameter α for signals synchronized to it

**Port: `out`**
- Declared as: `outgoing reg out : Bit on clock`
- The `on clock` clause means `out` is synchronized to `clock`
- Domain: `Sync(ComponentId(clock))` internally
- In the signature: `α` (the domain parameter introduced by `clock`)

### Synthesized Signature:
```rust
DomainSignature {
    domain_params: {
        clock: DomainVar(0)  // α
    },
    port_domains: {
        inp: Concrete(Async),
        clock: Concrete(Clock),
        out: Var(DomainVar(0))  // α
    }
}
```

**Interpretation:**
- `inp` is always `Async`
- `clock` is always `Clock`
- `out` is synchronized to whatever clock is connected to the `clock` port (polymorphic)

---

## Instantiation Example

Now let's see how this signature works when `Sync` is instantiated in another module:

```virdant
mod User {
    incoming sys_clk : Clock
    incoming async_signal : Word[8] async

    // Instantiate Sync to cross from async to sys_clk domain
    mod sync of Sync {
        it.inp := async_signal
        it.clock := sys_clk
    }

    // Now sync.out is on sys_clk domain
    reg buffer : Word[8] on sys_clk {
        it <= sync.out
    }
}
```

### Domain Substitution at Instantiation

**Step 1: Establish bindings from clock connections**

Connection: `it.clock := sys_clk`
- Child port `clock` introduces domain parameter `α`
- Parent signal `sys_clk` has domain `Clock` (but see note below)
- More precisely: `sys_clk` defines the concrete domain `Sync(ComponentId(sys_clk))`
- **Binding**: `α → Sync(ComponentId(sys_clk))`

**Step 2: Check data port connections with substitution**

Connection: `it.inp := async_signal`
- Child port `inp` has domain `Concrete(Async)` in signature
- Parent signal `async_signal` has domain `Async`
- Substitution: `Concrete(Async)` → `Async` (no variables to substitute)
- Check: `compatible(Async, Async)` ✓

Connection: `sync.out` read in parent
- Child port `out` has domain `Var(α)` in signature
- Substitute: `Var(α)[α → Sync(ComponentId(sys_clk))] = Sync(ComponentId(sys_clk))`
- Therefore: `sync.out` has domain `Sync(ComponentId(sys_clk))` in the parent

**Step 3: Check downstream usage**

Driver: `buffer <= sync.out`
- `buffer` is declared `reg buffer : Word[8] on sys_clk`
- `dom(buffer) = Sync(ComponentId(sys_clk))`
- `dom(sync.out) = Sync(ComponentId(sys_clk))` (after substitution)
- Check: `compatible(Sync(sys_clk), Sync(sys_clk))` ✓

**Result:** All domain constraints satisfied. The `Sync` instantiation successfully crosses from the async domain to the `sys_clk` domain.

---

## Key Mechanics Demonstrated

### 1. Signature Synthesis Without Body Analysis

The `@unsafe_clocks` annotation allows trusted modules to escape domain checking while still participating in the polymorphic signature system. The signature is synthesized from:
- Port types (Clock ports introduce domain parameters)
- Async modifiers (mark ports as `Async`)
- `on clock` clauses in register declarations (tie ports to domain parameters)

### 2. Polymorphic Domain Parameters

The signature `out : α` means "the output is synchronized to whatever clock you connect to the `clock` port". This is instantiation-time polymorphism:
- One module definition
- Reusable with different clocks
- Type-safe domain checking at each instantiation site

### 3. Domain Variable Binding

Clock port connections establish bindings:
```
it.clock := parent_clock  ⟹  α := Sync(ComponentId(parent_clock))
```

All other ports referencing α are then checked with this substitution applied.

### 4. The Upcast Property of Async Ports

Notice that connecting `async_signal` to `inp` works regardless of `async_signal`'s original domain:
```virdant
incoming signal_a : Bit async      // Async
reg signal_b : Bit on clk { ... }  // Sync(clk)

mod sync1 of Sync {
    it.inp := signal_a    // Async → Async ✓
    it.clock := clk
}

mod sync2 of Sync {
    it.inp := signal_b    // Sync(clk) → Async ✓ (upcast)
    it.clock := clk
}
```

Wait, actually this needs more thought. Let me reconsider...

Actually, based on the lattice rules, `Sync(clk)` does NOT upcast to `Async` automatically. The rules say:
- "Any domain can upcast to Async **when the target is explicitly declared async**"

So when connecting to `Sync.inp` (which is declared `async`), the parent signal can be anything and it will upcast. This is the key feature that makes `Sync` work as a domain crossing primitive.

**Corrected understanding:**
```virdant
mod sync_from_clocked of Sync {
    it.inp := signal_b    // Sync(clk) upcasts to Async because inp is declared async
    it.clock := dest_clk
}
```

The upcast rule specifically allows this connection without error.

---

## Notes

### 1. Does this work naturally with the algorithm?

**Yes, with the special case.** The algorithm needs explicit handling for `@unsafe_clocks`:
- Check for the annotation before starting Pass 1
- If present, skip to signature synthesis
- The synthesis reads port declarations and follows simple rules

The synthesis process is straightforward:
- Clock ports → domain parameters
- `async` modifier → `Concrete(Async)`
- `on clock` clause → `Var(α)` where α comes from that clock port
- No modifier and not a register → could be `Free()` or inferred from other constraints if we wanted, but for Sync specifically everything is fixed

### 2. Edge cases and design issues

**Trustworthiness:** `@unsafe_clocks` is an escape hatch. There's no verification that the module actually implements safe clock domain crossing. This is similar to Rust's `unsafe` — you're promising the compiler "trust me, this is safe" without proof.

**Future: builtin mod:** The plan mentions that the morally correct solution is `builtin mod` — modules whose implementation is primitive hardware (like D flip-flops) rather than Virdant code. This would be cleaner than an annotation, but requires more language machinery.

**Signature synthesis limitations:** The current synthesis approach works for simple cases like `Sync`, but what if an `@unsafe_clocks` module has more complex internal structure? The synthesis currently uses a simple rule-based approach that might not capture all possible signatures. For example:
- What if a port's domain depends on a computation over multiple clocks?
- What if there are internal combinational relationships?

For now, `@unsafe_clocks` is specifically for the `Sync` primitive where the signature is obvious from the declarations.

**Module completeness:** Does `Sync` need a reset? Probably not — the behavioral model is just a D flip-flop with clock enable. The first cycle after clock connection, `out` will capture whatever is on `inp` (or some X state). This is hardware-level semantics.

**Type polymorphism vs domain polymorphism:** Notice that the example shows `Sync` working with `Bit`, but the plan's examples show `Word[8]`. The domain signature is orthogonal to the type signature. `Sync` should probably be generic in the data type as well:
```virdant
@unsafe_clocks
mod Sync[T] {
    incoming inp : T async
    incoming clock : Clock
    outgoing reg out : T on clock { it <= inp }
}
```

But this example focuses on domain polymorphism, not type polymorphism.

### 3. Clarifications for PLANv2.md and ALGORITHM.md

**Clarification 1: Upcast rule at connection sites**

PLANv2.md says:
> "**Exception**: When the target is explicitly declared `async`, operands can be anything — they automatically upcast."

This needs to be precise about "target". At connection sites:
```virdant
mod sync of Sync {
    it.inp := some_signal    // some_signal is the source, inp is the target
}
```

The target (`inp`) is the child port. If the child port is declared `async`, the source can be any domain and will upcast.

**Clarification 2: Signature synthesis rules**

ALGORITHM.md should specify the synthesis rules more explicitly:
```python
def unsafe_clocks_signature(module: Module) -> DomainAnalysis:
    """
    For @unsafe_clocks modules, synthesize signature from port declarations
    without analyzing the body.
    """
    domain_params = {}
    port_domains = {}
    domain_var_counter = 0

    # Pass 1: Clock ports introduce domain parameters
    for port in module.ports:
        if port.type == Clock:
            var = DomainVar(domain_var_counter)
            domain_var_counter += 1
            domain_params[port.name] = var

    # Pass 2: Determine each port's domain from its declaration
    for port in module.ports:
        if port.type == Clock:
            port_domains[port.name] = SignatureDomain.Concrete(Clock)

        elif port.type == Reset:
            port_domains[port.name] = SignatureDomain.Concrete(Reset)

        elif port.async_modifier:
            port_domains[port.name] = SignatureDomain.Concrete(Async)

        elif port.kind == Register and port.on_clause:
            # Register synchronized to a clock port
            clock_component = port.on_clause
            if clock_component in domain_params:
                var = domain_params[clock_component]
                port_domains[port.name] = SignatureDomain.Var(var)
            else:
                # Error: register references non-port clock
                # (shouldn't happen in well-formed @unsafe_clocks module)
                pass

        else:
            # Unconstrained port
            port_domains[port.name] = SignatureDomain.Free()

    return DomainAnalysis(
        moddef=module.symbol_id,
        component_domains={},  # Empty for @unsafe_clocks
        signature=DomainSignature(domain_params, port_domains),
        diagnostics=[],
        unsafe_clocks=True
    )
```

**Clarification 3: Domain parameter binding details**

When a clock port introduces a domain parameter α, what exactly does α represent? It's not just `Clock` — it represents the domain defined by that clock, i.e., `Sync(ComponentId)` where the `ComponentId` is filled in at instantiation time.

The binding `α → Sync(ComponentId(parent_clock))` happens when you connect a parent clock signal to the child's clock port.

### 4. Testing implications

To test `@unsafe_clocks`, we need:
1. Verify that `Sync` module's body is NOT analyzed (no error despite illegal assignment)
2. Verify that signature is correctly synthesized
3. Verify that instantiation checking works correctly with the synthesized signature
4. Verify that illegal instantiations (wrong domains) are still caught

Example test cases:
```virdant
// Should succeed: crossing from async to clock
mod test1 {
    incoming clk : Clock
    incoming sig : Bit async
    mod s of Sync {
        it.inp := sig
        it.clock := clk
    }
    // s.out has domain Sync(clk)
}

// Should succeed: crossing from one clock to another
mod test2 {
    incoming clk_a : Clock
    incoming clk_b : Clock
    incoming sig : Bit
    wire sig_sync : Bit { it := sig }  // inherits some domain
    mod s of Sync {
        it.inp := sig_sync  // upcasts to async
        it.clock := clk_b
    }
}

// Should succeed: composing Sync modules
mod test3 {
    incoming clk_a : Clock
    incoming clk_b : Clock
    incoming sig : Bit async

    mod s1 of Sync {
        it.inp := sig
        it.clock := clk_a
    }

    mod s2 of Sync {
        it.inp := s1.out   // Sync(clk_a) upcasts to Async
        it.clock := clk_b
    }
    // s2.out has domain Sync(clk_b)
}

// Should fail: output domain mismatch
mod test4 {
    incoming clk_a : Clock
    incoming clk_b : Clock
    incoming sig : Bit async

    mod s of Sync {
        it.inp := sig
        it.clock := clk_a
    }

    reg buffer : Bit on clk_b {
        it <= s.out  // ERROR: s.out is Sync(clk_a), buffer is Sync(clk_b)
    }
}
```
