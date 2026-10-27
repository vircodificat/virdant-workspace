# Tristate and Bidirectional Ports

## Idea
Virdant has `incoming` ports and `outgoing` ports, but it has no notion of bidirectional ports.
Similarly, there is no tristate buffer primitive.
We want to add examples to both.

## Bidirectional Nets
The `net` keyword acts like a Virdant `wire`, but can be multiply driven.
This mirrors Verilog's `net` keyword.

`net`, like `wire`, is a locally-visible signal, but otherwise acts identically to `bus`.

## Bidirectional Ports
Virdant has `incoming` ports and `outgoing` ports.
We wish to add an `bus` port that acts like another type of port.

## X and Z
Verilog uses a "4-value model" for bits, allowing values 0, 1, X, and Z.

Virdant takes a more principled approach to interpreting X and Z.
We do *not* interpret these as values.
Instead, we take X to mean "multiply driven" (also called "contention")
and Z to mean "undriven" (also called "floating").
*THESE ARE NOT VALUES*.
Rather, they are either the presence of multiple values driving the same net or no values, respectively.

What this means is that uninitialized data is not set to X, as is the case for many Verilog simulators,
but instead to some arbitrary value (random, zeroed, error-pattern, etc).

For `incoming` `outgoing` `wire` and `reg`,
there is no possibility of contention or floating by construction.
Consequently, these always carry a single, definite value.

The only place where floating might arise is with `bus` and `net`.
These need not be driven with a value, and when they aren't, they are considered to float.

However, becuase `bus` and `net` do not necessarily have a value.
For this reason, we cannot use them directly as references.
Instead, we must use `tap n` (where `n` is our `bus` or `net`).
But this is an unsafe operation,
and its use indicates the designer is asserting that `n` will always have a value.

To use `n` as a variable reference without `tap` is an error:
"{name} is not guaranteed to have a value. Use `tap` {name} to assert it will.
The designer is responsible for making sure that `v` has a single value
(eg, not floating) when using `tap v`.


## Net drivers
To drive a net, there are three forms.

### Net drivers
Let `n` be a path to a `net` or a `bus` and `e` be an arbitrary expression of the same type.

`n #= e` drives `n` with the value of `e`.

### Attachment
Let `m` be another path to a `net` or `bus` (it's okay to intermix the two):

`n #=# m` is a bidirectional connect operation. (Similar to Chisel's `attach` statement).
Electrically speaking, these have the effect of attaching the two nets with a wire.
Any value driven to `n` is also driven to `m` and vice versa.

### Weak Drivers
`n #- e` pulls the value of `n` to the value `e`.

So `n #- true` is attaching a pullup resistor to `n`,
and similarly,
`n #- false` is adding a pulldown resistor to it.
This is called a "weak" driver.
The `#-` looks like `#=` but with "half" the force (equal sign becomes a minus sign).

Weak drivers may be used for any type of value.


## Drivers and Submodules
The reason for using `#=` is to visually distinguish them,
signaling to the designer that, athough they are looking at a driver `n #= ...`,
there MAY be other drivers as well which could be active at the same time.
(And for `bus`es, these may be driven from the other side of a module boundary).

When connecting to a submodule, you use `#=` and `#=#` just the same:

```virdant
mod Foo {
    bus a : Bit

    mod bar of Bar

    bar.b #=# a
}

mod Bar {
    bus b : Bit
}
```

```virdant
mod Foo {
    bus a : Bit

    mod bar of Bar

    a #= bar.b
}

mod Bar {
    outgoing b : Bit
}
```

## Driver Multiplicity
The whole point of nets is that they may or may not be driven.
This is the major difference between them and `incoming` `outgoing` and `wire`.
For these, there is to be a unique driver (or block of drivers) for the component.
But for `net` and `bus`, this need not be the case.

The # in `#=` `#=#` and `#-` (as in "number") is a visual mnemonic that these may have multiple drivers.

A moddef may drive a `bus` just as with an `outgoing` port,
but at the same time, any module which instantiates it may drive it as if it were an `incoming` port.
That is, both parent and child may drive a `bus`.


## Expressions are always single-valued
Expressions *always* generate a concrete value.
That is to say, *no expression* can ever evaluate to `Z` or `X`.

If you want to "set a `net` `n` to `Z`", you **CANNOT** set `n` to `Z`.
To make `n` float, you must have zero (active) drivers.


## Initialization
Initial values of each port, wire, and reg signal should be set to an arbitrarily value.
It is up to the simulator to determine the method by which to initailize them.
(Suggestions are to use seeded random values, zero initialization, or test-pattern initialization).

NOTE: This only applies to `reg`s (eg, before reset) and top-level `incoming` ports,
since all other owned signals can be traced back combinationally to one of these.
Any values which "would" be floating are considered to collapse to a definition (although arbitrary value).

For shared signals, there should be no initial value unless combinationally driven by a value (or weakly driven with a value).
So `bus`es are left floating as are `net`s, unless driven from the initial conditions of the simulation.

## Contention
Contention is *always* considered an error at simulation time.
In real hardware, contention creates a short circuit which will at best burn energy,
and at worst, destroy your hardware.

It is always the designer's responsibility to ensure that no contention is possible in a design.

In simulation, contention should be grounds to abort the simulation in almost all circumstances.

## `when` and `match` statements

`when` statements are allowed to omit the `else` clause when driving `net`s and `bus`es.
and `match` statements aren't required to have complete case coverage (similar to `reg`).

The semantics of this are obvious.
When there is no `case` for the given situation, no value is driven.
When lowering to Verilog, this means setting the resulting variable to a Z value of the proper bitwidth.

`when` and `match` expressions are unaffected by this propsal.
Both still require complete case coverage
(ie, `when` expressions require an `else` clause
and `match` statements must have full case coverage or an `else` clause).


## Driving nets
If a net is driven by only weak values, and those values are all the same, the value of the net is that value.
If a net is driven by only weak values, and there are at least two such values, the net is in contention.
If a net is driven by at least one strong value, if all the strong values are the same, that is the value of the net
  (overriding any weak values being driven)
If a net is driven by at least one strong value, and there are at least two distinct values,
  the net is in contention.

## Tristate
Using this feature, a tristate module could be defined as follows:

```virdant
  mod Tristate {
      incoming data : Valid[Bit]

      bus out : Bit {
          it #= match {
              case @Valid(b) => b
          }
      }
  }
```

## Notes
- `tap` on a floating value is a simulation error.
- Do not implement this for the simulator `sim` module yet.

## Future Work

### Sockets
Socekts may contain slots marked `bus`, just as they may have `ciso` and `cosi` slots.

How this works should be straightforward: both `client socket` and `server socket` sides of the socket
end up with a bidirectional `bus` port.
