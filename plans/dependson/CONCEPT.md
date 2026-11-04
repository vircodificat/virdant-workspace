# Concept: dependson
`dependson` is underdeveloped in the current version of Virdant.

# Syntax
## Syntax Change
Rather than a statement, `dependson` should be a modifier on an `outgoing`:


```top
pub mod Top {
    incoming inp : Bit
    outgoing out : Bit dependson inp {
        it := inp
    }
}
```

## Multiple `dependson` clauses
Looks like this, comma separated.

```top
pub mod Top {
    incoming inp_0 : Bit
    incoming inp_1 : Bit

    outgoing out : Bit dependson inp_0, inp_1 {
        out := inp_0 && inp_1
    }
}
```

## Other Components
While only allowed on `outgoing`,
the user-facing grammar should support `dependson` on any component
to facilitate better error messages.

A `dependson` on an `outgoing reg` should always be flagged as an error.

A `dependson` on an `outgoing wire` should always be treated the same as a `dependson` on an `outgoing`.

## Tristate
It is banned for `net`.

TODO I'm not sure how `dependson` should work for `bus`.


# Behavior
## Comb Dependency
The `dependson` on clause declares the `outgoing` port may have a combinational dependency.
This is part of the `mod`'s "public interface", and so combinational analysis can be done
without looking "inside" the child module.

The `dependson` clause is not strict.
It must list the set of combinational dependencies the `outgoing` port
has on `incoming` ports.

It is an error to have any component listed in the `dependson` clause which *isn't* an `incoming` port.

An `outgoing` port which declares an `incoming` port as a `dependson`
but which *doesn't* depend on it combinationally generates a warning.

## Dependency analyisis
In light of this, do your best to simplify the comb dependency analysis.
TODO What exactly is the dependency analysis contract for submodules then?

## Sockets
Socket channels are allowed to have `dependson` clauses as well.
When instantiated as the appropriate `socket` (ie, `client` for `cosi` and `server` for `soci`),
this places the same obligation on the module containing the socket as if the `dependson`
were declared on the resulting port.

However, when a `socket` channel doesn't depend combinationally on a port,
it does *not* generate the warning.

The use cases for sockets are:
* Wishbone and TL *support* same-cycle responses, which would mandate a `dependson`.
* But they *also* support arbitrary delays, so no warning should be generated.


# Notes
## Dependson vs Comb Dependency
It should be an error to have a loop in the `dependson` relation.
It's part of the formal contract, so if a module declares that `x` `dependson y`,
a designer is *free to change `x` so that it **DOES** depend on `y` at any time*
and the result should not break anything.

This brings up one point that's worth distinguishing:
- For the purposese of cycle detection, it should error
- However, this does not mean that `x` has a combinational dependency on `y`, since it's only the potential.

In other words, the cycle check is based on *potentiality* not the final design.
And this is intentional.

Said simply one last time:
- `dependson` does *not* create a combinational dependency,
- however, the cycle check should be based on whether there's a cycle in the `dependson` relation.

