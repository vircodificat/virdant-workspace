# Summary

Virdant has no notion of clock domains. We want to add this as a feature.

There is an existing plan under development: plans/clock-domain-analysis/
This is a different spin on that plan. But one which takes a lot of inspiration from it.

# Code examples

For normal Virdant modules, the clock and reset become implicit.
The `on` clause is not required for `reg`.
Everything works as you would expect.

```virdant
mod Foo {
    incoming inp : Bit
    outgoing out : Bit {
        it := r
    }

    reg r : Bit {
        it <= inp
    }
}
```

A new `raw` keyword which can be used on `mod` definitions.
This means that the domains must be explicitly declared.
This is done with the new `with domain` keyword construct.
By convention, the "standard" domain is called `sys`.
In `raw` `mod` definitions, the `on` keyword becomes mandatory on each component declaration.

The equivalent `raw` version of the `Foo` example above is this:

```virdant
raw mod Foo {
    with domain sys

    incoming inp : Bit on sys
    outgoing out : Bit on sys {
        it := r
    }

    reg r : Bit on sys {
        it <= inp
    }
}
```

## Purpose for `raw`
The `raw` keyword makes it so that the "happy path" `mod` definitions are streamlined.
They remove the need to include the explicit clock and reset `incoming` ports,
as well as the need to mark all registers with `on`.
This way, all synchronous designs require much less bookkeeping.

However, with the `raw` keyword, we can always drop down into a mode
where we can introduce modules which have multiple clock domains.

## Submodules
When instantiating submodules you must assign parent domains to child domains.
This is done with the following syntax:

```virdant
raw mod Foo {
    with domain a
    with domain b

    mod bar of Bar {
        it.sys <~ a
    }
}
```

You may also use this syntax in normal synchronous `mod`s.
A synchronous mod may have exactly ONE `with domain`, or else it is an error.
Still, no `on` statements are required.
(Naturally, there's only one domain in a synchromous `mod`, so they must be set to the same thing):

```virdant
mod Baz {
    with domain sys

    mod qux of Qux {
        it.a <~ sys
        it.b <~ sys
    }
}
```


## Async
`raw mod`s also support `async` in a way almost identical to plans/clock-domain-analysis/.

```virdant
raw mod Foo {
    with domain sys

    incoming inp : Bit async

    mod inp_sync of builtin::Sync {
        it.inp := inp
    }

    outgoing out : Bit on sys {
        it := r
    }

    reg r : Bit on sys {
        it <= inp_sync.out
    }
}

// New version of Sync:
//> `Sync` synchronizes a `Bit` input on one clock domain
//> on another clock domain `clock`
@unsafe_clocks
raw mod Sync {
    with domain sys

    //> An input `Bit` on another clock domain
    incoming inp : Bit async

    reg meta : Bit on sys {
        it <= inp
    }

    //> The clock of the output
    outgoing reg out: Bit on sys {
        it <= meta
    }
}
```

## Domains provide a Clock and a Reset
A domain provides an implicit clock and reset signal.
When lowering to Verilog, each domain turns into a signal.
If the domain is called `sys`, these signals are: `sys.clock` and `sys.reset`.

## Removal of `Clock` and `Reset`
In this proposal, clocks and resets become implicitly threaded to registers as necessary.
This means that both the `Clock` type and the `Reset` type become superfluous types.
They are both slated for removal.

## Creation of a new domain
Creation of a new domain is done by assigning to the domain's `clock` and `reset`.
This allows us to treat domains analogously to a `socket`, where it introduces new components into the moddef.
The domain is then available if we want to place `reg`s on it in the current module, or to pass the domain to any submodules:

```virdant
@unsafe_clocks
raw mod Top {
    incoming clock : Bit
    incoming reset : Bit

    domain sys {
        it.clock := clock
        it.reset := reset
    }
}
```

TODO: I'm not sure I like this notation.

# Notes
## Name
The name `raw` is analogous to Chisel's `RawModule`. It has precedence.
It's not the absolute perfect name, but it is short and inoffensive and not especially common.
Alternatives are hard to find:
- `explicit mod` makes you wonder "what is explicit?"
- `strict mod` is misleading
- `domain mod` suggests that it is `mod` *for* domains, rather than an explicit domain `mod`.

- `multi mod` might be a worthy consideration. However, you can technically have 0 domains defined.

## Explicit `on` clauses
The requirement is that, in a `raw mod`, that all components must have *explicit* domains.
This is in contrast to the plans/clock-domain-analysis/ plan, which used a constraint solver.
This should hopefully allow the analysis to be syntax-directed, rather than requiring a solver,
simplifying the domain analysis, since the solution is known up front (it's annotated),
and all that is left is to verify it.

The explicit `on` clauses also aid in readability for the necessarily more complicated multi-domain examples.


## Async and Constants
Async still works nearly identically to the identical to plans/clock-domain-analysis/.
Any signal may be upcast to `async` "forgetting" its domain.

For example, using `Sync`, the `inp` signal is `async` and when connecting to it, you may feed it any signal at all,
regardless of the domain.

Similarly, any literal or constant exprssion is considered `const`.
It may be upcast to any other domain or to `async` as per the lattice.

## Keywords
- `raw` -- used as an optional prefix to `mod` at module definition sites
- `with` -- used to indicate name a domain that must be provided by the parent.
- `domain` -- used to declare a new domain inside a `raw mod`. This is *not* a component, but rather a resource.
  However, it is still a "slot" and still introduces a name, and so the name must be distinct from other slots in the moddef.
- `const` -- just reserve this keyword for now. No functionality.
- `reset` -- used for `case` and `match` statements.
- `sys` -- (considering)


# TODO
- Instantiating a synced `mod` from a `raw` mod.
- Do I want to make `sys` a keyword?
- Reset case
- Generation of a new domain
- Error messages
