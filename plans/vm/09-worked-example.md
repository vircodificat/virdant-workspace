# Worked Example: Blink

The smallest design that exercises the interesting machinery, compiled end to
end. Notation per [`05-ir.md`](05-ir.md).

## The design

```virdant
mod Blink {
    incoming clock : Clock      // an input — driven from outside
    incoming reset : Bit        // an input — driven from outside

    reg count : Word[8] on clock {   // a register — 8 bits of state
        initial it <= 0
        when {
            case reset { it <= 0 }
            else { it <= it + 1 }
        }
    }

    outgoing led : Bit {             // an output wire — combinational
        it := !count[0]
    }
}
```

## The side table

| ID | Path | Kind | Type |
|---|---|---|---|
| 0 | `top.clock` | input | Clock |
| 1 | `top.reset` | input | Bit |
| 2 | `top.count` | reg, cur | Word[8] |
| 3 | `top.count` | reg, next | Word[8] |
| 4 | `top.led` | comb output | Bit |

`initial it <= 0` appears **nowhere** in the program — it is one entry in the
state-image table (`top.count` starts at 0, or random-filled if the strategy
says so).
Initialization is data, not code.

## The program

```
// ---- COMB section (runs at settle) ----
^settle():
    %0 = load top.count
    %1 = slice %0, 1..0          // %1 = count[1..0] — bit 0 of the register, as a Bit
    %2 = not %1
    store top.led, %2
    ret!

// ---- NEXT section (runs at tick, before commit) ----
^tick():
    %3 = load top.reset
    %4 = const 0w8
    branch! %3, ^zero(%4), ^incr()

^zero(%v):
    stage top.count, %v        // v = 0w8, chosen by the branch
    ret!

^incr():
    %5 = load top.count
    %6 = const 1w8
    %7 = add %5, %6
    stage top.count, %7
    ret!
```

## Walking it

### COMB section

`led` is a wire: no clock, no state, re-evaluated every settle.

- `%0 = load top.count` — the register's committed value.
- `%1 = slice %0, 1..0` — bits `1..0` of the word: bit 0, as a `Bit`.
  (`k..r` = bits `k-1 … r`; `0..0` would be nonsense.)
- `%2 = not %1` — invert.
- `store top.led, %2` — write the named wire's slot.

Two instructions of work.
This section never mentions `clock` or `reset` — `led` depends only on
`count`, which changes only at commit.
Between ticks, settle is dirty-guarded and therefore free.

### NEXT section

The driver says: *at the edge, `count`'s next value is* `0` if `reset`, else
`count + 1`.

1. `%3 = load top.reset` — the condition, as sampled at the edge.
2. `branch! %3, ^zero(%4), ^incr()` — the conditional tail-call; the chosen
   value travels as a **block parameter**, not a phi.
3. `^zero(%v)` stages the parameter — `it <= 0` lowered to "stage the incoming
   value."
4. `^incr()` re-loads `top.count` (the *pre-edge* value — same slot as `%0`
   but a different program point, so a different SSA value), adds `1w8`
   (the `add` masks to `Word[8]`: 255 wraps to 0 for free), and stages it.
5. Both arms end by staging `top.count` — the merge expressed by *effect*,
   not control flow: exactly one stage happens per execution, whichever path
   ran; the language already guarantees each driver assigns its target exactly
   once per path.
6. The program ends; the machine commits: `cur ← next`, atomically.
   `it <= it + 1` worked because `^incr()` read the pre-edge value; the staged
   value does not exist until commit.

### Blocks, entry blocks, fusion

- Every instruction lives in a block; blocks are the universal unit.
- `^settle()` is the COMB section's entry block — where `settle` begins.
  `^tick()` is the NEXT section's entry block — where the edge begins.
  Entry blocks are named for the phase that runs them.
- `^settle()` looks like ceremony here (a straight-line block), but it exists
  because a section must have a well-defined start; with top-level
  conditional logic it would end in a real `branch!`.
- **This diamond is tiny, so the fusion pass demotes it to a mux:**
  `^tick()` collapses into a single block —
  `stage top.count, mux(reset, 0w8, count + 1)` — same semantics, three
  blocks gone, no `branch!` left.
  The un-fused form remains available as the debug build.

## One cycle from the embedder's chair

```
// t = 0: session built. State image applied:
//   top.count := 0        (from `initial it <= 0`)
//   top.reset := random   (strategy default; embedder hasn't poked yet)

let mut w = sim.lock_mut();
w.poke(w.id_of("top.reset"), bit(0));   // stage reset = 0
drop(w);                                 // ← SETTLE runs; top.led becomes 1

let r = sim.lock();
r.peek(r.id_of("top.led"))               // → 1 (settled by construction)
drop(r);

sim.tick(clk);
//   ^tick() runs on settled values: reset == 0 ⇒ ^incr() path
//   stage top.count, 1w8 → commit: count is now 1
```

Three facts that make the machinery *correct*, not merely plausible:

1. **The NEXT section only ever reads settled values** — it runs after
   settle, under `&mut` exclusivity, so the edge samples exactly what the
   wires hold.
2. **Staging makes the edge atomic for free** — nothing observes a staged
   value until commit promotes them all at once.
3. **Between ticks, nothing moves** — settle is idempotent and dirty-guarded;
   observing is a flag check.

## What this example does not show

It is the degenerate workload — ~6 instructions per cycle, an 8-bit commit.
It proves the machine's *correctness story*; it says nothing about the
*throughput story*, which is where the flat schedule, fast-path widths,
fusion policy, and the native-code backend earn their keep.
