# Phase 9 (v3): Scaffolding and Tests

### 9.1 `vir new`

**Why:** Scaffolded projects need a top module selected from the
start, or `vir bitstream` has nothing to synthesize; hardcoding
`"Top"` (the old behavior) no longer works now that the top module is
configurable (Decision 6). Under v3 there is no `[prog] platform`
key: the platform binding lives in the source via the `for` clause
(Decision 2).

Edit `new_project` (`vir.rs:714-767`) so the generated
`Virdant.toml` selects the top module:

```toml
[project]
name = "<project>"

[prog]
top = "Top"
```

### 9.2 `assets/blink.vir`

**Why:** This file is the template `vir new` copies into every
scaffolded project, so it must satisfy the exact-match port rule
against the `IceSugar` platform (Decision 4) or new projects would
fail `vir check` immediately with no changes made.

Rewrite so it compiles against `lib/ice40.vir` under the
exact-match rule: `import ice40`, bind with `for ice40::IceSugar`,
declare all eleven platform ports, mark unused incoming ports with
`unused` statements, and tie off every output.
The LEDs are active-low per the platform docstrings.

The top module must be **`export mod Top`**: non-export mods are
emitted to Verilog under their mangled qualified name
(`\top::Top`), which breaks `synth_ice40 -top Top`.  (This was a
latent bug in the v2 scaffold, whose `vir bitstream` hardcoded
`-top Top` while `assets/blink.vir` had a non-export `mod Top`.)
Also note `Bit` literals are `true`/`false`, not `0`/`1`.

```virdant
import ice40

export mod Top for ice40::IceSugar {
    incoming clock : Clock
    outgoing led_red   : Bit
    outgoing led_blue  : Bit
    outgoing led_green : Bit
    incoming switch0 : Bit
    incoming uart_rx : Bit
    outgoing uart_tx : Bit
    outgoing flash_cs   : Bit
    outgoing flash_clk  : Bit
    outgoing flash_mosi : Bit
    incoming flash_miso : Bit

    reg counter : Word[24] on clock {
        it <= it + 1
    }

    led_red := ~counter[23]
    led_blue := false
    led_green := false
    uart_tx := true
    flash_cs := true
    flash_clk := false
    flash_mosi := false

    unused switch0
    unused uart_rx
    unused flash_miso
}
```

(Verify the exact `unused <path>` statement syntax against
`ModDefStmtUnused` at `lalrpop:516-518` when writing this file.)

### 9.3 Tests

**Why:** Test coverage is intentionally minimal (Decision 11), but
these two tests lock in the feature's core guarantees: that `vir
check` can actually load a lib platform package and resolve
`import ice40` + `for ice40::IceSugar` (Decision 2), and that a
port mismatch is rejected by `vir check` (Decision 5) — no external
toolchain involved.

Two tests only (Decision 11).

**Happy path** — `tests/pass/platform_icesugar/`:

- `Virdant.toml`:
  ```toml
  [project]
  name = "platform_icesugar"

  [prog]
  top = "Top"
  ```
- `src/top.vir`: the same design as `assets/blink.vir` (or the file
  itself copied in).
- `Makefile`, mirroring `tests/pass/annotations`:
  ```makefile
  BASEDIR = $(abspath ../../../..)
  VIR ?= $(BASEDIR)/target/release/vir

  test:
  	$(VIR) check
  ```

This proves `vir check` loads the lib package and resolves the
`for` clause. Note the test relies on `VIR`'s `LIB_DIR` resolving to
`target/lib`, which the top-level `virdant/target/lib` target
(`Makefile:16-18`) populates from `./lib/` — including the new
`ice40.vir`.

**Port-mismatch fail-test** — `tests/errors/platform_mismatch/`
(moved from v2's `tests/pass/`, since the check now lives in
`vir check`):

- Same project layout, but `src/top.vir` declares a port that does
  not exist on the platform (e.g. an extra `outgoing extra : Bit`).
- The `tests/errors/` harness (`tests/errors/Makefile`) runs
  `vir check` on `src/*.vir` projects the same way it runs `-F` on
  single files; if needed, extend the harness loop minimally to
  support directory projects (mirroring `tests/pass/annotations`),
  and use `filecheck` to assert the mismatch message.

This passes on any machine: the exact-match check is part of
`vir check`, so no external tool is ever invoked (Decision 5).

Run `make grammar && make test` at the end of every phase.
