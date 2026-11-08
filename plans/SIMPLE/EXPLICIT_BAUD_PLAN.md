# EXPLICIT_BAUD_PLAN.md — Explicit UART baud for `vir run`

This document specifies the replacement of the `virt` simulator's
auto-baud TX decoder (and its idle-high trust gate) with an explicitly
configured baud rate, supplied by `--baud` on `vir run` or by a
`[virt]` table in `virdant.toml`.

Everything here was decided in design discussion. Nothing in this
document is left open.

---

## 1. Summary

Today the `virt::Virt` board's TX decoder (design → host stdout)
infers the design's UART bit period by measuring edges on the `tx`
line (auto-baud), and refuses to trust the first start bit until the
line has idled high for ~86 cycles (the "gate", `UartTx::new(10 *
clock_hz / BAUD)`). The gate delays — and in the degenerate case
permanently silences — any design that transmits immediately at
power-up (see §2).

Instead:

- The bit period becomes an explicit configuration: **baud**, resolved
  as `--baud` (CLI) > `[virt].baud` (`virdant.toml`) > `115200`
  (default).
- The TX decoder drops all measurement machinery and decodes with the
  fixed period `clock_hz / baud` (exact rational, reusing the existing
  `BaudTiming`).
- The trust gate is deleted outright.
- The RX side (host → design stdin) transmits with the same fixed
  period from the first byte; it no longer waits for the TX side to
  confirm a period.

The contract on designs becomes explicit: **a design targeting
`virt::Virt` must clock its UART bits at `clock_hz / baud` cycles per
bit**, where `clock_hz` is the platform's declared `@period_ns` and
`baud` is the resolved baud. A design using a different divisor will
miscommunicate; this is now a documented contract rather than a
measured property.

## 2. Motivation

### 2.1 The gate

The sim initializes every register at a random value. Before `initial`
drivers settle, `tx` can glitch low for a few cycles. Auto-baud infers
the bit period as the *minimum* distance between consecutive `tx`
edges — a monotone `min()` that is never reset — so a 1–2 cycle
startup glitch would poison the estimate forever. The gate defends
against this: until a frame decodes successfully (`confirmed` is set),
a falling edge is only accepted as a start bit if the line has been
continuously high for `gate_cycles` (10 nominal bit periods at the
platform clock: `10 * 1_000_000 / 115_200 = 86`).

### 2.2 Why the gate is unacceptable

The gate is a one-time hurdle — but it can deadlock: a design that
starts transmitting at ~cycle 3 never satisfies "idled high 86
cycles", and if it then streams bytes back-to-back (9-cycle stop bits),
the line *never* accumulates the idle budget mid-stream either. No
frame ever opens, `confirmed` is never set, and every byte is silently
discarded for the entire run. This is not hypothetical: a tutorial
"Hello, world!" design hits it, and the reference `virt::Hello` module
in `lib/virt.vir` must carry a 256-cycle boot quiet period solely to
work around it.

With a fixed period, none of this machinery is needed: a glitch cannot
poison anything because nothing is learned; a glitch-corrupted frame is
simply decoded, fails its framing check, and is dropped — recovery is
automatic on the next frame.

## 3. Baud resolution

The effective baud is resolved once, at startup, in `vir/src/vir/main.rs`:

1. If `--baud <N>` was passed on the command line, use it.
2. Else, if `virdant.toml` (the project manifest in the resolved
   project root) contains a `[virt]` table with an integer `baud` key,
   use it.
3. Else, use `115_200`.

The value is an unsigned integer, interpreted in bits per second
(8N1 framing is fixed and unchanged).

Validation (hard error, exit code 1, message to stderr):

- `N` must be at least 1.
- The resulting cycles-per-bit must be at least 2: reject
  `baud > clock_hz / 2`, e.g.
  `error: baud 600000 is too fast for a 1000000 Hz clock (need at least 2 cycles per bit)`.
  (`clock_hz` is `1e9 / period_ns` from the platform's declared clock,
  as today.)

CLI/config errors are reported before the simulator starts.

Example `virdant.toml`:

```toml
[project]
name = "foo"

[prog]
top = "top::Top"

[virt]
baud = 115200
```

## 4. CLI changes

`vir/src/vir/cli.rs`:

- `Command::Run` gains `baud: Option<u64>`.
- New clap arg: `.arg(Arg::new("baud").long("baud").num_args(1)
  .value_name("BAUD"))`, parsed with `m.get_one::<u64>("baud")`.
- Help text: `--baud <BAUD>  UART baud rate for the virt board
  (default 115200, overridable via [virt] in virdant.toml)`.

`vir/src/vir/main.rs`:

- `Command::Run { .., baud }` arms the existing `run(...)` call; the
  resolution chain of §3 runs inside `run()` (it already has `cwd` and
  the manifest path).
- A new helper in `vir/src/common.rs`, `read_virt_baud(cwd: &Path) ->
  Option<u64>`, mirrors `read_prog_key` but reads `[virt].baud` as an
  integer (`toml.get("virt").and_then(|v| v.get("baud"))`).
  A non-integer `baud` is a hard error (same style as §3 validation),
  not silently ignored.

`vir/src/vir/virt.rs`:

- `VirtOptions` gains `pub baud: u64` (already resolved, never
  `None`); `run()`'s signature is unchanged (it keeps receiving
  `clock_hz` and now derives `cycles_per_bit` from `clock_hz` and
  `opts.baud`).

## 5. TX decoder redesign (`UartTx`)

### 5.1 Removed

- The gate: `gate_cycles`, `high_since`, and the `trusted` checks in
  `cycle()` and at frame-open sites.
- All auto-baud state: `min_edge`, `frame_min`, `confirmed`, and the
  candidate-period search in `finalize()`.
- `UartTx::new(10 * clock_hz / BAUD)`; `BAUD` is no longer a constant
  in this file (it comes from `VirtOptions`).
- The large auto-baud struct doc comment (replaced by the fixed-period
  contract of §1).

### 5.2 Kept

- Retrospective frame buffering: edges are buffered from each
  start-bit falling edge; `finalize()` decodes the closed frame.
- The frame-close rule: a falling edge at or beyond `10 * bit_period`
  into an open frame closes it and opens the next frame; anything
  sooner is a data-bit transition. (This rule needs the period — now
  always known — and never needed measurement.)
- Framing validation and the drop-on-failure rule: a frame produces a
  byte only if every transition lands on a bit boundary and
  start/stop framing is valid; otherwise it is dropped.
- `flush()` on exit and on the `--cycles` cap, so the final byte still
  reaches stdout.

### 5.3 New shape

```rust
struct UartTx {
    cycles: u64,
    prev_edge: u64,
    /// Exact cycles-per-bit: clock_hz / baud as a reduced rational
    /// p/q (reuses `BaudTiming`); boundaries are compared in units of
    /// q-ticks, so a non-integer period (1 MHz / 115200 = 8.68…) is
    /// handled without drift.
    timing: BaudTiming,
    frame_start: Option<u64>,
    frame_edges: Vec<u64>,
    last: bool,
}
```

- `UartTx::new(timing: BaudTiming)`; `bit_period()` returns
  `Some(timing)` unconditionally (see §6).
- Boundary arithmetic uses the same q-tick accumulation `UartRx`
  already performs via `BaudTiming`; no new timing code is written.

### 5.4 Startup glitch behavior

Under random register init, `tx` may glitch low in the first few
cycles. With a fixed period that glitch opens a frame; the frame fails
framing validation and is dropped. At most the first frame (one byte's
worth of edges, or less) is affected, and only at power-up. This is
accepted, documented behavior. Designs that want a guaranteed-clean
first byte may still idle before transmitting (the `virt::Hello` boot
quiet period remains correct, and remains in `lib/virt.vir`), but it
is no longer *required*.

## 6. RX side (`UartRx`)

Today `UartRx` starts with the nominal `baud_timing(clock_hz, BAUD)`
and re-aims at `uart_tx.bit_period()` once the TX side confirms a
design period (virt.rs:268). Under explicit baud:

- `UartRx` is constructed once with `baud_timing(clock_hz,
  opts.baud)` and never re-aimed: `set_bit_period` and the
  `self.uart_rx.set_bit_period(self.uart_tx.bit_period())` call in
  `PerCycle::cycle` are deleted.
- The "the design's UART divisor is the same on both directions"
  assumption becomes the §1 contract, enforced by configuration
  instead of measurement.

## 7. Test harness

No changes to `vir/src/virtue/discovery.rs` or `exec.rs`: `run` tests
already accept free-form `run_args`, so a baud-specific test declares

```toml
kind = "run"
cycle_limit = 40
run_args = ["--baud=9600"]
```

New tests (under `tests/run/`):

- **immediate-tx**: a `virt::Virt` design that starts transmitting at
  cycle ~3 with no boot delay — passes today only via the workaround;
  after this change it must print its message with the default baud.
  This is the regression test for the gate's removal.
- **explicit-baud-config**: same design, baud set only via `[virt].baud`
  in `virdant.toml` (no `--baud`).
- **baud-cli-overrides-config**: `[virt].baud = 9600` plus
  `run_args = ["--baud=115200"]`; the CLI value must win.
- **baud-too-fast**: `[virt].baud` exceeding `clock_hz / 2`; expect
  the §3 hard error.

## 8. Migration / behavior changes

- `vir run`'s output is unchanged for existing designs that follow the
  `virt::Hello` idiom (boot quiet period + `exit`); their first byte
  now decodes immediately instead of after the gate.
- Designs that transmit immediately and previously printed nothing
  (gate deadlock) now work.
- Designs whose UART divisor differs from `clock_hz / baud` previously
  worked by auto-baud; they now misdecode and must either match the
  resolved baud or run with a matching `--baud`. (The auto-baud
  fallback is intentionally not retained: it is the sole state that
  made the gate necessary.)
- The tutorial (`assets/tutorial/`, docs examples) may drop its boot
  quiet period if it has one; `lib/virt.vir`'s `virt::Hello` keeps its
  boot period (harmless, and it is also the reference idiom for real
  hardware where the host terminal does no auto-baud).
- Exit codes are unchanged: the design's requested code, 124 on the
  `--cycles` cap, 1 on configuration errors.

## 9. Non-goals

- No per-direction baud (`--rx-baud`/`--tx-baud`): a UART shares one
  divisor.
- No auto-baud fallback mode or `--auto-baud` escape hatch.
- No `[virt]` keys other than `baud` in this change (flash, trace, fps
  remain CLI-only).
- No changes to `virt::VirtVga`, the platform definition in
  `lib/virt.vir`, or the `exit`/`--cycles` semantics.
