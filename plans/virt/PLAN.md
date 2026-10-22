# Plan: virt — a Virtual FPGA Platform

Date: 2026-10-20
Derived from: `CONCEPT.md` (interview, 2026-10-20)
Related: `virdant-riscv/` (Verilator harness precedent), `virdant/lib/ice40.vir` (platform syntax)

## Summary

Introduce a `virt` platform — a "virtual FPGA" — in `virdant/lib/virt.vir`.
Unlike the iCE40/Xilinx platforms, `virt` is not flashed to hardware: a
design written `for Virt` is compiled to Verilog by `vir build` as usual,
then a standalone tool (`virt build`, living in a new `virdant-virt/` repo)
wraps the generated `.sv` files in a Verilator-based simulator that emulates
the board's UART and SPI flash. `vir upload` has **no** relevance to `virt`.

## Decisions (from interview)

| # | Decision |
|---|----------|
| D1 | Platform is `virt::Virt` (capital V). Includes a `clock` incoming port at 48 MHz (`@period_ns("20.833")`), plus (added later) an `exit : Valid[Word[8]]` outgoing port: `@Valid(n)` on a clock tick ends the simulation with code n. |
| D2 | Port names follow the iCE40 convention, named by signal role from the FPGA's perspective: `spi_ck`, `spi_cs`, `spi_mosi` (outgoing), `spi_miso` (incoming). CONCEPT's `spi_do`/`spi_di` map to `spi_mosi`/`spi_miso`. |
| D3 | `vir upload` is fully decoupled from `virt`. The workflow is `vir build` → `virt build`. |
| D4 | The simulator lives in a new repo `virdant-virt/`, a sibling of `virdant/` and `virdant-riscv/`. |
| D5 | `virt` CLI is written in Rust. |
| D6 | `virt build --top=<TOP>` slurps `build/*.sv` and produces the simulator binary at `build/${TOP}` (in the design project's `build/` directory). Top name passed via `--top` (Virdant.toml integration deferred). |
| D7 | Fixed 115200 8N1 UART; baud tick counts derived from the 48 MHz clock (exact fractional accumulator: 48 MHz / 115200 = 416.67 cycles/bit). |
| D8 | Emulated UART is wired to stdin/stdout. |
| D9 | SPI flash is a **C++ model** in the harness (not a Verilog module). |
| D10 | SPI flash behavior: initialized to all `0xFF`; `--flash=<FILE>` preloads up to 2^24 bytes; supports `0x03` (READ) with **continuous**, auto-incrementing address until `spi_cs` deasserts (which resets the device state). |
| D11 | Harness is a **fixed `sim.cpp`** shipped with `virdant-virt`, compiled with Verilator `--prefix Vvirt` so the generated C++ class is always `Vvirt` regardless of the top module's name. `virt build` does no C++ codegen. |
| D12 | The harness relies on the fixed `virt::Virt` port contract (7 ports, below). Designs not conforming to it are unsupported by the generic sim. |

## Port contract (`virt::Virt`)

| Port | Dir | Type | Meaning |
|------|-----|------|---------|
| `clock` | incoming | `Clock` | 48 MHz primary clock |
| `tx` | outgoing | `Bit` | UART transmit → host stdout |
| `rx` | incoming | `Bit` | UART receive ← host stdin |
| `spi_ck` | outgoing | `Bit` | SPI flash clock |
| `spi_cs` | outgoing | `Bit` | SPI flash chip select (active low) |
| `spi_mosi` | outgoing | `Bit` | SPI data out of the FPGA |
| `spi_miso` | incoming | `Bit` | SPI data into the FPGA |

No `@pin` annotations — there are no physical pins.

## Discrepancies from CONCEPT.md

Where this plan deviates from `CONCEPT.md` (interview decisions supersede the
concept; `CONCEPT.md` itself was not modified):

| # | CONCEPT.md says | Plan says | Why |
|---|-----------------|-----------|-----|
| 1 | Ports `spi_di` / `spi_do` (FPGA perspective) | Renamed to `spi_mosi` (outgoing) / `spi_miso` (incoming), matching the iCE40 platform convention (`flash_mosi`/`flash_miso`) | Interview Q3: "spi_do should be mosi, etc." |
| 2 | No clock port listed | Added `incoming clock : Clock` at 48 MHz (`@period_ns("20.833")`) | Interview: sequential logic needs a clock; frequency set to 48 MHz |
| 3 | Platform named `virt` (lowercase) | Platform is `virt::Virt` (capital V) | Interview clarification |
| 4 | "`vir upload` should actually create a binary in build rather than flash it" | `vir upload` is fully decoupled from `virt`. The workflow is `vir build` (Verilog into `build/`) → `virt build` (slurps `build/*.sv`, produces the simulator) | Interview: "Scrap vir upload having any relevance to virt" |
| 5 | Binary produced by a Verilator/C++ build, implicitly by Virdant | Simulator tooling lives in a **separate repo** `virdant-virt/` (Rust CLI `virt` + fixed `sim.cpp`), not in Virdant | Interview: standalone `virt build` that slurps `.sv` files from `build/` |
| 6 | Binary named `build/${TOP}` | Kept: binary is `build/${TOP}` in the design project, with `${TOP}` supplied via `--top` (auto-detection deferred) | Interview: "named after top", "Let's just add --top for now" |
| 7 | — (no UART details) | UART fixed at 115200 8N1, stdin/stdout | Interview Q4 |
| 8 | "it will begin sending back bytes from the given address" | Confirmed **continuous** read: address auto-increments until `spi_cs` deasserts (standard flash `0x03` behavior) | Interview: "Continuously read" |
| 9 | — (flash model location unspecified) | SPI flash is a C++ model in the harness (not a Verilog module) | Interview: "C++ model" |
| 10 | — (harness structure unspecified) | Fixed `sim.cpp` shipped with `virdant-virt`, compiled with Verilator `--prefix Vvirt` so the class name never changes; no per-build C++ codegen | Interview: "Option 1" |
| 11 | — (CLI stack unspecified) | `virt` CLI is Rust | Interview |

## Files to change

| # | File | Change |
|---|------|--------|
| 1 | `virdant/lib/virt.vir` | **New.** Platform definition `Virt` |
| 2 | `virdant-virt/Cargo.toml` | **New.** Rust crate for the `virt` CLI |
| 3 | `virdant-virt/src/main.rs` | **New.** `virt build` implementation |
| 4 | `virdant-virt/sim.cpp` | **New.** Fixed generic Verilator harness (UART + SPI flash emulation) |
| 5 | `virdant-virt/README.md` | **New.** Usage docs |
| 6 | `virdant-virt/.gitignore` | **New.** `target/`, `build/` |

Pre-existing change (applied during planning, per D11): `virdant-riscv/sim.cpp`
was renamed `VTop` → `Vvirt` and its Makefile now passes `--prefix Vvirt`,
demonstrating the fixed-prefix technique.

## Phase 1 — `virdant/lib/virt.vir`

```vir
//! Platform definition for the virtual FPGA (`virt`).
//!
//! `virt` is not a physical board: designs written `for Virt` are
//! simulated with Verilator via the `virt` tool (repo `virdant-virt/`).
//! The simulated board emulates a UART (115200 8N1, stdin/stdout) and a
//! 16 MiB SPI flash device (initially all 0xFF, preloadable with
//! `--flash=<FILE>`).

//> Virtual FPGA, simulated by Verilator via the `virt` tool.
@fpga("virt")
@part("virt")
platform Virt {
    //> 48 MHz primary clock.
    @period_ns("20.833")
    incoming clock : Clock

    //> UART transmit data to the host (stdout).
    outgoing tx : Bit
    //> UART receive data from the host (stdin).
    incoming rx : Bit

    //> SPI flash clock.
    outgoing spi_ck : Bit
    //> SPI flash chip select (active low).
    outgoing spi_cs : Bit
    //> SPI flash controller-out data.
    outgoing spi_mosi : Bit
    //> SPI flash controller-in data.
    incoming spi_miso : Bit
}
```

## Phase 2 — `virdant-virt/` skeleton

```
virdant-virt/
├── Cargo.toml
├── README.md
├── .gitignore
├── sim.cpp
└── src/
    └── main.rs
```

**`Cargo.toml`** — std-only (no external crates; hand-rolled arg parsing):

```toml
[package]
name = "virt"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "virt"
path = "src/main.rs"
```

## Phase 3 — `virt build` (`src/main.rs`)

Usage: `virt build --top=<TOP>` (run inside the design project directory).

Steps:

1. Parse `--top=<TOP>` (required for now; error otherwise).
2. Collect `build/*.sv` from the current directory (error if none).
3. Run Verilator:
   ```
   verilator --sv --cc build/*.sv --Mdir build \
       --prefix Vvirt --top-module ${TOP} --build \
       -Wno-UNOPTFLAT
   ```
   `--prefix Vvirt` guarantees the generated class is `Vvirt` regardless
   of the top's name (D11).
4. Locate `sim.cpp`: `$VIRT_SIM_CPP`, else relative to the crate source
   (`env!("CARGO_MANIFEST_DIR")/sim.cpp`, baked at build time).
5. Link:
   ```
   g++ -O2 -Ibuild -I$VERILATOR_ROOT/include -I$VERILATOR_ROOT/include/vltstd \
       sim.cpp build/Vvirt__ALL.a -o build/${TOP}
   ```
   (`VERILATOR_ROOT` from `verilator -V`, same technique as the
   virdant-riscv Makefile. If the toolchain produces loose `.o` files
   instead of `Vvirt__ALL.a`, fall back to linking
   `build/verilated.o build/verilated_threads.o build/Vvirt__ALL.o`.)

`virt build` stops after producing `build/${TOP}`; the user runs the
binary themselves (`build/${TOP} [--flash=<FILE>]`).

## Phase 4 — `sim.cpp` (the virtual board)

Fixed harness, class `Vvirt`. Port members used: `clock`, `tx`, `rx`,
`spi_ck`, `spi_cs`, `spi_mosi`, `spi_miso`.

### 4a. Main loop

Two `eval()` phases per emulated clock cycle (low phase, then high
phase). Between evals:

- **UART TX decode** — sample `top->tx` once per cycle; detect the
  start-bit falling edge, then sample the 8 data bits at 1.5, 2.5, …
  bit-times using a fractional baud accumulator
  (`acc += 115200; tick when acc >= 48_000_000`, exact at 48 MHz).
  Deliver completed bytes to stdout.
- **UART RX drive** — poll stdin (non-blocking, every ~half bit-time);
  when a byte is pending and the line is idle, shift it out on `rx`:
  start bit, 8 data bits LSB-first, stop bit, all timed by the same
  fractional accumulator. Hold `rx` high when idle.
- **SPI flash model** — a C++ `SpiFlash` struct tracking previous values
  of `spi_ck`/`spi_cs` to detect edges (polled after each `eval()`):
  - `spi_cs` falling edge → reset state machine, expect command byte.
  - Rising edges of `spi_ck` (while selected): sample `spi_mosi`; first
    8 bits = command (`0x03` = READ; anything else → ignore until CS
    releases), next 24 bits = address.
  - Falling edges of `spi_ck`: shift out the next data bit MSB-first on
    `spi_miso`; every 8 bits advance the address (wrapping at 2^24) and
    load the next byte. This matches SPI mode 0: the device updates
    MISO on the falling edge, the controller samples on the rising edge.
  - `spi_cs` rising edge → go idle, `spi_miso` low.
- **Terminal raw mode** — when stdin is a TTY, put it in raw mode (so
  keystrokes go straight to the simulated UART), restoring on exit and
  on SIGINT (same pattern as virdant-riscv's `sim.cpp`).

### 4b. Flash memory

`std::vector<uint8_t>` of 2^24 bytes, filled with `0xFF`. With
`--flash=<FILE>`, the file's contents are copied to offset 0 (up to
2^24 bytes).

### 4c. CLI flags of the produced binary

| Flag | Meaning |
|------|---------|
| `--flash=<FILE>` | Preload the emulated flash from FILE |
| `--cycles=<N>` | Optional safety limit for testing (otherwise runs until Ctrl+C) |

## Phase 5 — Bring-up

1. `cargo build` in `virdant-virt/`.
2. Write a smoke-test design `for Virt` (e.g. echo bytes received on
   `rx` back out on `tx`; bit-bang a `0x03` READ of address 0 and echo
   the first flash byte over UART).
3. `vir build` in the test project, then `virt build --top=<TOP>`,
   then run `build/${TOP} --flash=<test image>` and verify stdin/stdout
   behavior.

## Future work (from CONCEPT)

- New ports: RGB LED (`led_red`, `led_green`, `led_blue`) and 4 buttons
  (`button_up`, `button_down`, `button_left`, `button_right`), with
  corresponding harness emulation (LED state printed/rendered, buttons
  driven by keyboard keys).
- `Virdant.toml` integration for `--top` discovery.
- Optional VCD tracing flag.
