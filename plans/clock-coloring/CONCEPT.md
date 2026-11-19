# Concept: clock-coloring

This is a lighter weight alternative to the two existing clock domain plans.

Instead, we make heavy use of the `on` and `async` keywords and track which clock
each signal is associated with.

Domain Lattice
--------------
Instead, each component and each expression (including subexpressions) is associated
with `Sync(component_id)` `Async` or `Constant`.

These are called domains.

These form a lattice, and the domains can be safely upcast
(eg, a `Sync(c1)` used in `Async` context,
or a `Constant` used in a `Sync(c2)` context).

```
          Async
         /  |  \
   Sync(c1) Sync(c2) ...
         \  |  /
        Constant
```

What can be on which domains?
-----------------------------
- A register's driver must be `Sync(register's clock)`
  A register can never be declared `async`.
- A constant expression
  (a word literal, an enumerant, a union element made up of only constants, etc)
  is inferred as `Constant`
- Components may be marked as `async` rather than `on *clock*`,
  and these are put on the `Async` domain.
- Clock themselves are always `Async`.


Connection Rules
----------------
Within a module body: every operand of a combinational expression
(including when guards and mux selectors)
must join to one domain, and that domain is the expression's domain.

Submodules
----------
Each incoming `clock` (whether on a port or a socket) induce a free variable
in a module's definition.
By driving a submodule's clock port, you instantiate that free variable with the concrete clock,
creating an association between the parent's clock domains and the child's.

Drivers
-------
When a signal is driven by an expression, the component's declared domain must equal the expression's domain.


`builtin::Sync`
---------------
For instance, `Sync` in `builtin` becomes:

```virdant
pub mod Sync {
    incoming inp : Bit async

    //> The clock of the output
    incoming clock: Clock

    reg meta : Bit {
        it <= inp
    }

    //> The clock of the output
    outgoing reg out: Bit {
        it <= meta
    }
}
```

Zero, One, or Multiple Clocks
-----------------------------
Modules can have zero, one, or multiple clocks.
This is determined simply by how many incoming clock ports there are,
including `incoming` and inward channels on a socket.

When there is one clock, it is called the unique clock for a module.
When a module has a unique clock, suppose it is named `clock`,
all components are implicitly marked as `on clock` (unless explicitly marked as `async`).

A module with zero clocks naturally has no clocks to provide to any `on` clause.
It will naturally come about that such moddefs require their inputs to be declared `async`.


.. note::

    If a module would combinationally on clocked signals,
    the designer must supply a vestigal clock port and `unused it`.

.. note::

    This design leaves no room for modules which are "polymorphic" over the clock.
    I don't think should should cause problems in practice though.


For modules with multiple clocks, there is no unique clock.
All components must be explicitly annotated with an `on` clause.

Clockchecking
-------------
Expressions are colored using a bidirectional checker similar to the typecheck algorithm.
This is a separate pass from typechecking and performed afterwards.
It is called the clock checker.

If a module has a unique clock and has no components marked `async`,
this check can be skipped entirely.
Every component and subexpression is simply marked as `Sync(clock)`
where `clock` is the unique clock.

Similarly, if a module has no clock incoming ports,
everything is `async` and this check can be skipped.
Every component and subexpression is simply marked as `Async`.

However, clockchecking is required when mixing `async` and synched signals (those with `on` cluases)
or for module definitions with more than one incoming clock port.

The `@unsafe_clocks` annotation be used on a module to completely disable clockchecking for a module.
This is how the `Sync` primitive is handled.
However, it does require that all components are annotated with `on` or `async`.

Rules for Clockchecking
-----------------------
- All operands to a combinational operator must have the same domain. The result has that domain.
  This includes cases such as `1 + 2` which is a constnat `+` another constant, to remain constant.
  It also allows async signals to combine `x_wild + y_wild` if the `*_wild` signals are Async,
  then the result is async.

- In a `match` expression:
```virdant
match subject {
    with pattern1 => result1
    with pattern2 => result2
}
```
The subject expression and all result expressions must have the same domain.
Patterns are not expressions and do not participate in domain analysis.

- In a `when` expression:
```virdant
if {
    when guard1 => result1
    when guard2 => result2
}
```

- All `when` expressions and all `if` and `match` arm result expressions must have the same domain.


- Array indexing: `array[index]` has the same domain as `array`. The `index` expression must also have the same domain as `array`.

- Struct field access: `struct->field` has the same domain as `struct`.

- Enumerants are all constant.

- Struct construction: `${ field1: expr1, field2: expr2, ... }`
  all field expressions must have the same domain, and the resulting struct has that domain.

- Ctors work the same as struct construction.



Open Questions
--------------
- There's an assumption about not using `asclock` for single-clock modules.
  SOLUTION: Replace `asclock` with a primitive `builtin mod` which makes the signal into a component

