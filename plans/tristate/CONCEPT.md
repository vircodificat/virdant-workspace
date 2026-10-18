# Tristate and Bidirectional Ports

## Idea
Virdant has `incoming` ports and `outgoing` ports, but it has no notion of bidirectional ports.
Similarly, there is no tristate buffer primitive.
We want to add examples to both.

## Bidirectional Nets
The `net` keyword acts like a Virdant `wire`, but can be multiply driven.
This mirrors Verilog's `net` keyword.

## Bidirectional Ports
Virdant has `incoming` ports and `outgoing` ports.
We wish to add an `bus` port that acts like another type of port.

## Signals, nets, and ownership
We prefer the word 'signal' to refer to a value that is tracked across connected components.

For instance, a parent might connect a `wire` to a submodule's `outgoing` port,
and the result is that the two will get associated with the same signal.
Moreover, the child **owns** the signal, since it is required that the child drive the `outgoing` port.

Ownership is the standard way to work with signals in Virdant.
There may only be one driver (or disjoint block of drivers) which gives the signal its value.
The driver (or informally, the module) is the "owner" of that signal.

In the tristate proposal, we are adding a new kind of signal which is not uniquely owned.
This is called a **shared** signal.
And they are created from `bus` ports and `net`s.
These can have any number of drivers, not just one.
They may have zero drivers, or conditioned drivers with missing `else` clasuses
(meaning that they are not driven under all conditions).
Often, we use a `bus`, which is a shared port,
for modeling a bidirectional bus (such as with a 6502 read-write memory port or an I^2C bus).

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

For owned signals (such as with `incoming` `outgoing` `wire` and `reg`),
there is no possibility of contention or floating by construction.

The one place where floating or contention might arise for an owned signal
is if we use the `unsafe_read(v)` primitive when `v` is floating or in contention.
The designer is responsible for making sure that `v` has a single value
(not in contention, not floating) when using `unsafe_read(v)`.

For shared signals, such as on `bus` and `net`,
these may end up floating (if no one drives them) or in contension (if they are multiply-driven).

## Initialization
Initial values of each owned signal should be set to an arbitrarily value.
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

For this reason, no semantics are given to `unsafe_read(v)` when `v` is in contention.

## `when` and `match` statements

`when` statements are allowed to omit the `else` clause when driving shared components,
and `match` statements aren't required to have complete case coverage.

The semantics of this are obvious.
When there is no `case` for the given situation, no value is driven.
When lowering to Verilog, this means setting the resulting variable to a Z value of the proper bitwidth.

`when` and `match` expressions are unaffected by this propsal.
Both still require complete case coverage
(ie, `when` expressions require an `else` clause
and `match` statements must have full case coverage or an `else` clause).

## Drivers
We use the usual `:=` driver to connect to `bus`es and `net`s.

The difference is that there is no longer the usual restrictions (as with `incoming` `outgoing` and `wire`)
that there be a unique driver (or block of drivers) for the component.

A moddef may drive a `bus` just as with an `outgoing` port,
but at the same time, any module which instantiates it may drive it as if it were an `incoming` port.
That is, both parent and child may drive a `bus`.

`net`, like `wire`, is a locally-visible signal, but otherwise acts identically to `bus`.

## Reading from shared signals
This features introduces a strong distinction between owned and shared signals.
But values must nevertheless propagate between them.

A shared signal (`net` or `bus`) may be safely driven with any other signal you can read from (`incoming` `wire` `reg` `net` `bus`).

However, care must be taken when driving an owned signal with a shared signal.
For example, driving an `outgoing` port from a `net` might create problems when the `net` is floating,
since this leads the `outgoing` port to float as well, which is disallowed.
In proper operation, an owned signal should *never* float and should *never* be in contension.

However, driving from a shared signal is potentially unsafe,
since the signal may be floating.
(NOTE: Contention is always unsafe, even for shared signals, since it creates an electrical short).

However, if the designer takes responsibility for guaranteeing the value is always driven in a certain conditional context
(eg, nested inside a `when` block), he may use `unsafe_read(shared_value)` to read from a shared signal `shared_value`.

This is considered undefined behavior for a A simulator.
However, a simulator is recommended to panic whenever reading from a floating value this way.

For the special cases of `Bit` type shared signals,
we also introduce the primitives `pullup(bit_shared_value)` and `pulldown(bit_shared_value)`,
which weakly drive the signal to `true` and `false` respectively.

There is no way to weakly drive non-`Bit` values.

NOTE: The following situations are technically possible, but not of real interest.
Consider the case that more than one value is weakly driven:
-  if they are the same, the signal acts as if it were weakly driven by just one. (It's redundant).
- if they are different, the signal acts as if it were in contention.

## Tristate
Using this feature, a tristate module could be defined as follows:

```virdant
  //> A tristate buffer
  //>
  //> When `enable` is high, the buffer drives `data` onto the shared
  //> bus `out`. When `enable` is low, the buffer does not participate:
  //> the bus is released, and its value is determined by the other
  //> participants joined to it, or floats if there are none.
  //>
  //> The parent may also drive `out` directly, and must use
  //> `unsafe_read(out)` to read it.
  mod Tristate {
      //> The value driven onto the bus when `enable` is high.
      incoming data : Bit

      //> When true, the buffer drives `out`; when false, it releases it.
      incoming enable : Bit

      //> The shared bus this buffer participates in.
      bus out : Bit {
          when {
              case enable {
                  it := data
              }
              // No `else` clause on purpose, and in this csae, it doesn't drive a value
          }
      }
  }
```



## Sockets
Socekts may contain slots marked `bus`, just as they may have `ciso` and `cosi` slots.

How this works should be straightforward: both `client socket` and `server socket` sides of the socket
end up with a bidirectional `bus` port.

## TODO
- Common sense error/warning when two drivers obviously appear in the same moddef
- How uncommon are tristates in FPGAs? How do we address this?

## Notes
(none)

## Considerations
The following are not yet set in stone, but are points of consideration:

## Multi-bit values
When working with multi-bit values, such as `Word[2]`, we perform our analysis in a bitwise manner.

Contension is determined *per bit*.
However, as any single bit being multiply driven causes a short,
any bit in contention is sufficient to terminate simulation.

Similarly, floating is *per bit*.
If a `Word[2]` value `v` (somehow through a future feature) were to obtain a hypothetical value `2'b0z`,
and then you were to index on it to get `v[1]`, it would be a valid value of `0`.
You could then safely read this with `unsafe_read(v[1])`.
