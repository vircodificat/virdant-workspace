# Phase 8 (v4): Build Flow Rewrite (`vir bitstream` / `vir upload`)

Under v3 there is no `[prog] platform`. The platform comes from the
top module's `for` clause, resolved through the same shared helper
the port match uses (`resolve_platform_for`, Phase 4.3). The port
match itself was already enforced by `vir check` (via
`check_platform_ports`); `bitstream` re-derives the platform and
re-verifies via the same per-module helper
(`check_platform_ports_for`) defensively before invoking any
toolchain. Platform data (`@fpga`, `@part`, pins, clocks) is read
with the Phase 4 accessors.

### 8.1 `bitstream`

**Why:** The old `bitstream` command hardcoded a single platform and a
single top module name, so it can't work for arbitrary projects; this
rewrite makes it read `[prog] top` from `Virdant.toml`, resolve the
top module's `for` clause, and only then emit Verilog and invoke the
toolchain.

Replace the body of `bitstream` (`vir.rs:769-874`):

```rust
fn bitstream(args: &Args) {
    let cwd = resolve_cwd(args);
    if !cwd.join("Virdant.toml").exists() {
        eprintln!("No Virdant.toml found");
        std::process::exit(1);
    }

    let project = read_project_name(&cwd);
    let Some(top_name) = read_prog_key(&cwd, "top") else {
        eprintln!("Virdant.toml is missing [prog] top");
        std::process::exit(1);
    };

    let builddir = cwd.join("build");
    let source_dir = cwd.join("src");

    let db = virdant::util::db_from_dir(source_dir);
    dump_diagnostics(&db);
    if check_db(&db).is_err() {
        eprintln!("Build failed");
        std::process::exit(1);
    }

    // Resolve the top module named by [prog] top.
    let symboltable = db.get_symboltable();
    let top_symbol = symboltable.items().into_iter()
        .find(|sym| sym.name.as_bytes() == top_name.as_bytes()
            && sym.kind == SymbolKind::ModDef)
        .unwrap_or_else(|| {
            eprintln!("Top module '{top_name}' not found");
            std::process::exit(1);
        });

    // The top module's `for` clause selects the platform.
    let platform_id = match resolve_platform_for(&mut builder, top_symbol.id()) {
        Ok(Some(id)) => id,
        Ok(None) => {
            eprintln!("Top module '{top_name}' does not implement a platform (no `for` clause)");
            std::process::exit(1);
        }
        Err(diagnostic) => {
            eprintln!("{diagnostic}");
            std::process::exit(1);
        }
    };

    // Defensive re-check: ports must exactly match the platform.
    // (Normally already enforced by `vir check` via check_platform_ports.)
    let mut diagnostics = vec![];
    check_platform_ports_for(&mut builder, top_symbol.id(), &mut diagnostics);
    if !diagnostics.is_empty() {
        for diagnostic in &diagnostics {
            eprintln!("{diagnostic}");
        }
        eprintln!("Build failed");
        std::process::exit(1);
    }

    // Platform data via the Phase 4 accessors.
    let fpga = platform_fpga(&mut builder, platform_id).unwrap_or_else(|| {
        eprintln!("Platform is missing @fpga");
        std::process::exit(1);
    });
    let part = platform_part(&mut builder, platform_id);

    std::fs::create_dir_all(&builddir).unwrap();
    let verilog = virdant::verilog::conversion::convert_db_to_verilog(&db);
    verilog.write_in_dir(&builddir).unwrap();

    let Some(toolchain) = virdant::build::toolchain_for(&fpga) else {
        eprintln!("FPGA family '{fpga}' is not supported");
        std::process::exit(1);
    };

    let pcf = builddir.join(format!("{}.pcf", project));
    match virdant::build::emit_pcf(&mut builder, platform_id) {
        Ok(pcf_text) => std::fs::write(&pcf, pcf_text).unwrap(),
        Err(diagnostics) => {
            // e.g. a platform port with no @pin (Unknown diagnostics).
            for diagnostic in &diagnostics {
                eprintln!("{diagnostic}");
            }
            eprintln!("Build failed");
            std::process::exit(1);
        }
    }

    virdant::build::run_toolchain(&builddir, &project, &top_name, &part, toolchain)
        .unwrap_or_else(|e| {
            eprintln!("Toolchain error: {e}");
            std::process::exit(1);
        });
}
```

There are no `vir.rs`-local platform helpers: platform resolution,
the port re-check, and all data reads are the Phase 4 helpers
(`resolve_platform_for`, `check_platform_ports_for`, `platform_fpga`,
`platform_part`, `platform_pins`, `platform_clocks`, all in
`analysis/platform.rs`), called directly. Point-of-use errors
(missing `@fpga`, unknown family, unresolvable `for` clause) are
plain CLI errors; missing platform `@pin`s surface as `Unknown`
diagnostics from `emit_pcf` (Phase 7.1) before any tool runs.

### 8.2 `upload`

**Why:** `upload` should not re-implement or second-guess the
validation that `bitstream` already performs; delegating to it keeps
the two commands consistent and avoids duplicating platform/port
checks in two places.

Keep the shape of `upload` (`vir.rs:947-972`): call `bitstream(args)`,
then re-read the project name and run `icesprog <project>.bin`
via `virdant::build::flash_bitstream`.
It must no longer assume anything about the platform beyond what
`bitstream` already validated.

### 8.3 Remove old code

**Why:** The old code hardcoded the "icesugar" platform and a "Top"
module name directly in `vir.rs`, which is now redundant and
contradicts the generic, config-driven flow added above; leaving it
in place would be dead code that could silently drift out of sync
with the new logic.

Delete from `vir.rs`:
- `assert_eq!(platform, "icesugar", ...)` (`:794`)
- the inline yosys/nextpnr/icepack sequence in `bitstream`
  (`:811-873`)
- `create_pcf_file` and `ICESGUAR_PORTS` (`:876-945`)
- the hardcoded `"Top"` in the yosys script and nextpnr args
  (`:818`, `:846`)
