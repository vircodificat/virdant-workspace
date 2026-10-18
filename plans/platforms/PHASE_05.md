# Phase 5 (v3): `@cell` Consumption and the Shipped Platform File

### 5.1 `lib/ice40.vir`

**Why:** Without a concrete shipped platform file, `platform`,
`@fpga`/`@part`/`@pin`/`@period_ns`, and `@cell` have nothing real to
validate against, and later phases (loading, port matching, blink.vir)
have no board to target.

Under v3 Decision 10, lib files are family-per-file: the package is
named after the FPGA family (`ice40`), and the file may hold several
`platform` items (one per board; only `IceSugar` ships in v1). Users
write `import ice40` and `for ice40::IceSugar` (or a bare `for
IceSugar`, which resolves through the import set like any item).

Create the file at the repository root, next to `lib/builtin.vir`.
It ships automatically with every existing copy step
(`Makefile:16-18`, `:40`, `:53`).

```virdant
//! Platform definitions for the Lattice iCE40 family.

//> iCESugar board.
//> Lattice iCE40-UP5K, SG48 package, programmed over USB.
@fpga("ice40")
@part("up5k-sg48")
platform IceSugar {
    //> Primary 12 MHz clock on pin 35.
    @period_ns("83.33")
    @pin(35)
    incoming clock : Clock

    //> RGB LEDs (active low).
    @pin(39) outgoing led_red   : Bit
    @pin(40) outgoing led_blue  : Bit
    @pin(41) outgoing led_green : Bit

    //> User switch.
    @pin(18) incoming switch0 : Bit

    //> UART (USB CDC).
    @pin(4) incoming  uart_rx : Bit
    @pin(6) outgoing  uart_tx : Bit

    //> SPI flash.
    @pin(46) outgoing flash_cs   : Bit
    @pin(48) outgoing flash_clk  : Bit
    @pin(45) outgoing flash_mosi : Bit
    @pin(47) incoming  flash_miso : Bit
}

//> iCE40 Block RAM primitive.
@cell("SB_RAM40_4K")
ext mod SbRam40_4k {
    incoming clock : Clock
    incoming addr  : Word[14]
    incoming wdata : Word[8]
    incoming wmask : Bit
    outgoing rdata : Word[8]
}
```

The exact port list above is the contract that `blink.vir`
(Phase 9) must match exactly.

Note the package name must be `ice40` (the file name minus `.vir`,
which is how `db_from_name_text_pairs` (`util.rs:20-31`) names
packages), so `import ice40` and `for ice40::IceSugar` resolve.

### 5.2 Verilog emission uses the `@cell` name

**Why:** Yosys only recognizes iCE40 primitives like block RAM by their fixed native names (e.g. `SB_RAM40_4K`); emitting the mangled Virdant name instead would make synthesis fail to map the instantiation to the hardware primitive.

Edit `build_module_info` in `virdant/src/verilog/conversion.rs:115`.
When a `ModDef` is `is_ext` and carries an `@cell("...")` annotation,
use that string as the emitted Verilog name instead of
`valid_verilog_name(...)`:

```rust
let emitted_name = match cell_annotation(&parsing, item_ast) {
    Some(cell) => cell,  // e.g. "SB_RAM40_4K"
    None => if moddef.is_export {
        valid_verilog_name(&moddef_name)
    } else {
        valid_verilog_name(&module_path)
    },
};
```

Where `cell_annotation` reads the first value of
`item_ast.annotation_values(&parsing, "cell")` (Phase 4.1), giving
the unquoted string (e.g. `SB_RAM40_4K`).

This is the only change needed:
- Instantiations of submodules already go through
  `emitted_module_names` (`conversion.rs:281-283`).
- Ext mod definitions are already skipped at emission
  (`verilog.rs:238-247`), so yosys matches the instantiation name
  against its built-in `SB_RAM40_4K` cell.
- `get_ports_of` already works for ext mods (they are
  `SymbolKind::ModDef`, `ports.rs:30`), so instantiation port wiring
  is unchanged.

Unit-testable: `convert_db_to_verilog` on a design instantiating
`SbRam40_4k` must emit `SB_RAM40_4k <inst>(...)`.  (Note: exact
string; write the assertion against `SB_RAM40_4K`.)
