# Concept: sim_revamp

The `sim` module in virdant's source code has been half-working for a long time.

# X and Z values
Remove `X` and `Z` values.
Uninitialized registers and top-level incoming ports are initialized to a value chosen by a confiurable strategy (default: random, the only option for now).

# Track all vaules as bit patterns
Track all vaules as bit patterns, not as "object" types.
They are still "tagged" with their type.
On demand, cast them to the appropriate type to perform gate logic on them.
Print them according to their object type (so `Bit` "0" prints as `false`).
However, invalid values can be simply printed as `<invalid>`.

# Clock
Assume the clock has the frequency specified in the platform. Lock the FPS. User can pass `--fps-no-cap` to run at full speed.

# Delays and events
Do not worry about delays and events.
I may end up removing those.
Do whatever minimal work is necessary to get things to compile and assume those won't be invoked.
(But keep the functionality where it's easy to do so).

## Tasks
- Bring it up to date with all features *except* tristate buffers.
- For `vir run`, the new behavior should be to assert that the top module (as given in `Virdant.toml`) is `for` the `virt::Virt` platform. If it is, run a simulation on it.
- The simulation should work like the virdant-virt implementation of the `Virt` platform, except written in Rust and using the `sim` module.
- If `--trace $FILENAME` is given (default `$FILENAME` to `trace.vcd`), write a VCD file out.
