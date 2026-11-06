# Concept: gates_and_prop

I want to create a simple gate-count and propagation delay model for Virdant.

The aim is to add to the waveform dump in Sim a delay like how Verilog allows #1 #2, etc.

However, the number for each signal should be calcualted based on the propagation delay model.


## Delay Model
If two signals are driven:

```virdant
a := b
c <= d
```

then the propagation delay is #0.
(Although for `<=`, not that it's *staging* the value, or in other words, setting the D pins, not the Q pins).

For simple gates, such as `&&` or `||` or `^^` or `!` or `~`, I want a 1-unit gate delay.

For adders, I want a realistic gate delay based on the number of bits, assuming a ripple-carry.

For `word()` or bit manip, it should be 0 (they are just wires).

For `any()` and `all()` it should be... I think log2 the bit length or something close to that?


## Propagation Delay
The total propagation delay should be based on the backwards cone of the component.
It should be the longest pole from a source (according to the model).


