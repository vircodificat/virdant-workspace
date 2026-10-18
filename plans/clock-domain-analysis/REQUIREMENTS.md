# Clock Domain Analysis Requirements

## 1. Domain Assignment

**1.1** Every signal has exactly one domain.

**1.2** A domain is one of:
- `Sync(ComponentId)` — synchronized to a specific clock component
- `Async` — not synchronized to any clock
- `Clock` — the signal is a clock itself
- `Reset` — the signal is a reset itself
- `Constant` — a pure expression with no signal references

**1.3** A signal's domain applies to the entire signal. For struct types, all fields share the signal's domain. For array types, all elements share the signal's domain.

## 2. Domain Sources

**2.1** A register's output domain is `Sync(ComponentId)` where `ComponentId` identifies the clock specified in the register's `on` clause.

**2.2** A signal explicitly declared `async` has domain `Async`.

**2.3** A signal of type `Clock` has domain `Clock`.

**2.4** A signal of type `Reset` has domain `Reset`.

**2.5** A literal or pure combinational expression containing no signal references has domain `Constant`.

**2.6** Other signals' domains are inferred from their drivers and usage.

## 3. Domain Constraints

**3.1** All operands to a combinational operator must have the same domain. The result has that domain.

**3.2** In a `match` expression:
```virdant
match subject {
    case pattern1 => result1
    case pattern2 => result2
}
```
The subject expression and all result expressions must have the same domain. Patterns are not expressions and do not participate in domain analysis.

**3.3** In a `when` expression:
```virdant
when {
    case guard1 => result1
    case guard2 => result2
}
```
All guard expressions and all result expressions must have the same domain.

**3.4** When a signal is driven by an expression, the signal's domain must equal the expression's domain.

**3.5** Array indexing: `array[index]` has the same domain as `array`. The `index` expression must also have the same domain as `array`.

**3.6** Struct field access: `struct.field` has the same domain as `struct`.

**3.7** Struct construction: `StructType${ field1: expr1, field2: expr2, ... }` — all field expressions must have the same domain, and the resulting struct has that domain.

## 4. Domain Unification

**4.1** Domain `Constant` unifies with any `Sync(ComponentId)`. The result is that `Sync(ComponentId)`.

**4.2** Two domains `Sync(C1)` and `Sync(C2)` unify only if `C1` and `C2` identify the same clock component. The result is that `Sync` domain.

**4.3** Domains `Async`, `Clock`, and `Reset` do not unify with each other or with any `Sync` domain.

**4.4** Domain `Constant` unifies with domain `Constant`. The result is `Constant`.

**4.5** When a signal is explicitly declared `async`, constraint generation is bypassed for its driver expression. Any well-formed expression may drive it (automatic upcast to `Async`).

## 5. Register Constraints

**5.1** A register declared `reg name : Type on clock` must have domain `Sync(ComponentId)` where `ComponentId` identifies `clock`.

**5.2** A register's driver expression domain must equal the register's domain.

**5.3** No signal with domain `Async` may drive a register.

## 6. Module Instantiation and Domain Substitution

**6.1** When a module is analyzed in isolation, its ports may have polymorphic domains (domain variables).

**6.2** When instantiating a submodule:
```virdant
mod sub of SubModule {
    it.clk := parent_clock
    it.data := parent_signal
}
```

**6.2.1** Each clock port connection (`it.clk := parent_clock`) establishes a domain binding. If `SubModule`'s port `clk` has domain variable `α` in its signature, and `parent_clock` has domain `Sync(C)`, then the binding `α → Sync(C)` is recorded.

**6.2.2** For each data port connection (`it.data := parent_signal`), the checker:
1. Looks up the child port's domain from the module signature
2. Substitutes any domain variables using the bindings from **6.2.1**
3. Unifies the parent signal's domain with the substituted child port domain

**6.2.3** If a child port's domain contains unbound domain variables after all clock connections are processed, that variable becomes a constraint on the parent signal.

**6.3** Unconstrained ports in a child module propagate as unconstrained to the parent module.

**6.4** At the top-level module, ports may remain unconstrained (free domain variables).

## 7. Explicit Async Requirement

**7.1** A signal with domain `Async` must be explicitly declared `async`.

**7.2** Domain `Async` is never inferred for a signal not declared `async`.

**7.3** If an expression would require all operands to have domain `Async` (because they come from incompatible `Sync` domains), but the target signal is not declared `async`, this is an error.

## 8. Error Conditions

**8.1** Attempting to unify incompatible domains is an error:
- `Sync(C1)` with `Sync(C2)` where `C1 ≠ C2`
- `Async` with `Sync`, `Clock`, or `Reset`
- `Clock` with `Reset`, `Sync`, or `Async`
- `Reset` with `Clock`, `Sync`, or `Async`

**8.2** When operands in a combinational expression have different non-`Constant` domains, and the target is not declared `async`, this is an error.

**8.3** Driving a register with a signal of domain `Async` is an error.

**8.4** A register's inferred domain not matching its `on` clause is an error.

**8.5** In a `when` or `match` expression, if operands cannot unify to a single domain (and the result is not declared `async`), this is an error.
