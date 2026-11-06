# Concept: memories

## Goal
Add a behavioral sublanguage to Virdant that allows for
very flexible definition of memories.

They should simulate efficiently.

When synthesizing, they should always either produce infer to a reasonable set of memory primitives for the platform, or else fail.
They should NEVER synthesize to a bunch of flops.

The sublanguage should be simple and be comprehensible to someone who can read either C or Python.

`memory` should be top level item declarations, which result in a module-like item you can instantiate.

- They should support any number of read and write ports.
- They should allow internal registering of inputs and outputs.
- They should allow you to express any kind of read-over-write/write-over-read/etc behavior,
- implicitly through the behavior of the code.
- They should be restricted to memories whose addresses are a power of 2 and whose data widths are a power of 2.
- Comb and clocked reads are both supported.
