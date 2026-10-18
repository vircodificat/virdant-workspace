# Platforms: Final Implementation Plan (v4) — Overview

This document supersedes v2 and v3. It was revised after a design
discussion which replaced the config-driven binding (`[prog]
platform`) with an in-language binding: a module declares which
platform it implements with a `for` clause (`mod Top for
ice40::IceSugar`).

A v4 revision (after an implementation review of the first attempt at
Phases 3/4) removed the planned `PlatformAnalysis` query and the
`CheckPlatform` per-symbol query: the port match is a plain helper in
`Check()`, platform data is read through small un-memoized accessor
functions, and platform files are not validated by `vir check`.

A platform package is shipped content, loaded like `builtin`.
Annotations on the platform item and its ports carry the metadata
(FPGA family, part, pins, clock period).
A user design binds to a platform by naming it in its top module's
`for` clause; the ports of that module must exactly match the
platform's ports.

Each phase that touches `virdant.lalrpop` ends with `make grammar`,
and every phase ends with `make test` passing.

## Design Decisions

These decisions were made during the v2 design review and the v3
revision. The v3 decisions override v2 where they conflict.

1. **The `platform` keyword is the heart of the feature.**
   A platform package is just a package (like `builtin`) that ships
   with the language.

2. **A module binds to a platform with a `for` clause.**
   `mod Top for ice40::IceSugar { ... }`. The clause is optional and
   uses the existing `Ofness` node shape (`package: Option, name`),
   exactly like the `mod shifter of Shifter` clause. The name
   resolves with the normal item-resolution rules: bare or
   `package::`-qualified, through the module's imports. If you write
   `for ice40::IceSugar` you had better have `import ice40`.

3. **More than one platform in scope is fine.**
   A package may declare multiple `platform` items (e.g. one
   `lib/ice40.vir` shipping several boards), and several platform
   packages can be loaded and imported at once. There is no
   `DuplicatePlatform` diagnostic; duplicate platform *names* are
   already reported by `DuplicateItem`.

4. **Top-module ports must exactly match the platform's ports.**
   For any module with a `for` clause, the set of its ports must
   equal the platform's port set, with names verbatim, directions
   matching (`Incoming`↔`Incoming`, `Outgoing`↔`Outgoing`), and types
   matching. Usage within the module is enforced exactly as for any
   module.

5. **Exact match is enforced in `vir check` by a plain helper.**
   `check_platform_ports(builder, &mut diagnostics)` in
   `queries/check.rs` is called from `Check()` exactly like
   `check_mod_cycles`/`check_duplicate_enum_values` — it is not a
   per-symbol query. `vir bitstream` re-verifies via the same
   per-module helper before any toolchain invocation (cheap, and
   defensive), but `vir check` is the primary enforcement point.

6. **`[prog] top = "..."` selects the synthesis top module.**
   `[prog] platform` is gone: `vir bitstream` derives the platform
   from the top module's `for` clause. This replaces the current
   hardcoded `"Top"` in `vir bitstream`.

7. **Having no platform is fine.**
   Modules without a `for` clause are unconstrained, and packages
   without any `platform` item are analyzed as usual with no
   diagnostics. There is no `PlatformMissing` diagnostic.

8. **`@cell` is consumed from day one.**
   Verilog emission uses the `@cell("...")` name for `ext mod`
   instantiations. The shipped `lib/ice40.vir` includes
   `ext mod SbRam40_4k` with `@cell("SB_RAM40_4K")`.

9. **Only the ice40 toolchain is implemented.**
   The flow is yosys, nextpnr-ice40, icepack, and icesprog.
   Other FPGA families are rejected with a clear error.
   Only the ice40 `@fpga("ice40")` value dispatches in v1; an unknown
   family errors in the build flow (toolchain dispatch), not in
   `vir check`.

10. **Lib is family-per-file, and all of `lib/` loads.**
    Shipped platform packages live at `<libdir>/<package>.vir`
    alongside `builtin.vir` (e.g. `lib/ice40.vir`), one package per
    FPGA family, possibly containing several `platform` items. Every
    project db loads all `lib/*.vir` files (the Makefile needs no
    changes). Loading is not scoping: only `import` puts a package's
    items in scope, so loading all of `lib/` is observably equivalent
    to loading on demand, with no import pre-scanning.

11. **Keep tests minimal.**
    Two tests only: a happy-path project test (`vir check` passes,
    proving `import ice40` and `for ice40::IceSugar` resolve) and a
    port-mismatch fail-test (`vir check` fails with a port-mismatch
    error). Everything else is listed under "Test Cases Not Covered".

12. **No docs phase.**  A `docs/source/reference/platforms.rst` page is
    deferred to Future Work.

13. **Platforms are unvalidated data.**
    `vir check` performs no validation of platform files beyond the
    symbol table's structural checks (duplicate slots) and the
    module-side port match. There are no cardinality rules (a
    platform may declare any number of clock ports), and `@fpga`,
    `@part`, `@pin`, `@period_ns` values are never validated by the
    compiler. Platform data is read on demand through small,
    un-memoized accessor functions (`analysis/platform.rs`), never
    through a query. Mistakes surface at the point of use in the
    build flow (Phases 7/8) as `DiagnosticPayload::Unknown
    { message }` diagnostics or CLI errors.

## Codebase Facts the Implementer Must Respect

Verified against the tree as of October 2026 (v2 facts still standing
where not superseded here).

1. **`PackageId`, not `PackageFqn`.**
   The codebase uses `crate::package::{PackageId, PackageTable}`
   everywhere. All queries key on `PackageId` (`db.rs:50`) or on
   `SymbolId` (`db.rs:51-77`).

2. **There is no `IsDiagnostic` trait.**
   Diagnostics follow this pattern in `diagnostics.rs`:
   (a) a plain struct (e.g. `:301`),
   (b) a `DiagnosticPayload` enum variant (`:24-125`),
   (c) a `message()` arm in `Diagnostic::message()` (`:364-523`),
   (d) optionally a `level()` arm (`:525-537`, default is Error),
   (e) an entry in the `from_diagnostic!` list (`:563-573`).
   Diagnostics are constructed with
   `Diagnostic::new(region, payload)` (`diagnostics.rs:356`).

3. **Bare-name item resolution walks the import set.**
   `SymbolTable::resolve_item` (`symbols.rs:123-131`) only consults
   `builtin` or the *given* package for bare names; it does not know
   about imports. Callers that must honor imports iterate
   `package_analysis.imports()` and try
   `resolve_item_in_package` per import (see `component.rs:167-172`
   and `check.rs:208-218` for the qualified-vs-bare pattern via
   `try_split_qualification`). The `for`-clause resolution must
   follow this same pattern.

4. **`Ofness` is the existing "item reference" node.**
   `Ofness { package: Option<InternedString>, name }` (`payload.rs`
   `:389`) is produced by the `Ofness` grammar rule
   (`virdant.lalrpop:928-949`), which already supports both bare
   `Ident` and `Ident "::" Ident` forms. The `of` clause of submods
   stores it as a child node. The `for` clause reuses it verbatim;
   no new payload struct is needed.

5. **The only exhaustive match over ModDef children is in
   `package.rs`.**
   `add_item_expr_roots` (`package.rs:142-237`) ends with
   `_ => unreachable!(...)` at `package.rs:221`. Adding the optional
   `Ofness` child to `ModDef` requires an
   `AstNodePayload::Ofness(_) => (),` arm there. All other walkers
   over ModDef children use `_ => continue` patterns and are
   unaffected (`symbols.rs:365-380`).

6. **`Check()` aggregates per-symbol checks in `check.rs`.**
   `check` (`queries/check.rs:59-116`) iterates `symboltable.items()`
   and extends diagnostics from per-symbol queries
   (`check_drivers`, `get_match_coverage`, `typecheck`). The platform
   port match is *not* one of these: `check_platform_ports` is a
   plain helper following the `check_duplicate_enum_values` pattern
   (a function taking `builder` and `&mut Vec<Diagnostic>`), called
   from `check()` next to `check_mod_cycles` (`check.rs:176-259`),
   whose for-clause resolution follows the same import-set pattern.

7. **`resolve_cwd` does not exist.**
   Cwd resolution is inlined four times in `vir.rs`
   (`:139-155`, `:715-719`, `:770-774`, `:950-954`).
   The shared `project_db()` helper at `vir.rs:124` is the wiring
   point for lib loading.

8. **`lib/` files other than `builtin.vir` are not loaded today.**
   `load_vir_dir` (`util.rs:34-50`) loads exactly
   `builtin.vir` + the source dir. Phase 6 extends it to load every
   `lib/*.vir`. `assets/blink.vir` needs a real rewrite: it currently
   declares only `clock` and a single `led` output, has no `import`,
   and no `export`.

9. **Line-number drift from v1.**
   `vir.rs`: the icesugar assert is at `:794`, `ICESGUAR_PORTS` at
   `:877`, `new_project`'s toml write at `:731`, `bitstream` starts
   at `:769`. `symbols.rs`: the `unreachable!()` in
   `node_to_symbol_kind` is at `:496`.

10. **Token insertion point.**
    The keyword section of `token.rs` ends with `KwDontcare` (`:123`),
    `KwDyn` (`:124`), `KwDependsOn` (`:125`), `KwPlatform` (`:126`,
    added in Phase 0). Add `KwFor` after `KwPlatform`, and add
    `"for"` to `KEYWORDS` (`token.rs:137-147`) so package and item
    names cannot collide with it.

11. **`PortDir` is `{ Input, Output }`** (`common.rs:39-43`).
    `ComponentKind` is `{ Incoming, Outgoing, OutgoingWire,
    OutgoingReg, Reg, Wire }` (`common.rs:76-84`).
    Platform ports reuse `Incoming`/`Outgoing`, and the port match
    compares `ComponentKind::Incoming`↔`Incoming` and
    `Outgoing`↔`Outgoing` only (platforms never declare
    `reg`/`wire`/`outgoing reg`; see Phase 0 notes).

12. **`PortsOf` already gives typed ports.**
    `crate::analysis::ports::Port` is
    `{ path: BString, dir: PortDir, typ: Option<Type> }`
    (`ports.rs:16-21`), and `db.get_ports_of(symbol_id)` works for any
    `ModDef`-kind symbol. This is what the exact-match check uses for
    both sides.

13. **Ext mods are already half-wired.**
    `build_module_info` in `verilog/conversion.rs:115` iterates ALL
    `ModDef` payloads (including `is_ext`), and `VerilogFile::
    write_to_file` already skips emitting definitions for `is_ext`
    modules (`verilog.rs:238-247`). Phase 5 only changes the
    *emitted name* for `@cell`-annotated ext mods.

## What Is Already Implemented

| Item | Location | Notes |
|------|----------|-------|
| `@` token, `At` | `virdant/src/syntax/token.rs:63` | present |
| `Annotations` wrapper rule | `virdant/src/syntax/virdant.lalrpop:94` | wraps `Annotation*` |
| `Annotation` rule | `virdant/src/syntax/virdant.lalrpop:104` | three forms: `@(Nat)`, `@(Str)`, bare `@name` |
| `Annotations` prefix on item/slot productions | `virdant/src/syntax/virdant.lalrpop` | e.g. `ModDef:152` |
| `AstNodePayload::Annotations` / `Annotation` | `virdant/src/syntax/payload.rs:35-36` | |
| `AnnotationPayload { name, str_value, nat_value }` | `virdant/src/syntax/payload.rs:170-174` | no `value` field; do not invent an enum |
| Annotation accessors | `virdant/src/syntax/ast.rs:397-441` | `annotations_node()`, `annotations()`, `annotation_name()`, `annotation_string()`, `annotation_natural()` |
| `tests/pass/annotations/` project test | `tests/pass/annotations/src/top.vir` | exercises `@fpga`, `@part`, `@pin`, `@period_ns`, `@keep`, `@deprecated`, etc. (parsed but ignored) |
| `platform` keyword, item grammar, payload, AST accessors | Phase 0 (committed) | `KwPlatform`, `PlatformDef`/`PlatformStmt` rules, `Platform` payload, `ast.rs` arms |
| `item_children` helper | `virdant/src/syntax/ast.rs:529` | Phase 1 (committed); filters `Annotations`/`Annotation` children |
| `for` clause | Phase 2 (committed) | `KwFor` token/keyword, `ForClause` grammar rule producing an optional `Ofness` child on `ModDef`, `platform_for()` accessor, `package.rs` walk arm |

### Superseded v2 work (to be removed, not built upon)

The v2 plan's Phase 2 added `PackageAnalysis.platform:
Option<AstNodeId>`, its accessor, platform detection in `add_items`,
and a `DuplicatePlatform` diagnostic. Under Decision 3 these are
wrong: Phase 2 (v3) removed them. Phases 0-2 are committed; nothing
of the v2 work ever shipped.

### What is NOT implemented (the work of this plan)

- `SymbolKind::Platform`, symbol-table support (Phase 3; a first
  implementation was reverted with the Phase 4 review — re-apply it
  unchanged)
- The `check_platform_ports` helper and the five port-match
  diagnostics (Phase 4)
- Platform accessor functions (`annotation_values`, `platform_fpga`,
  `platform_part`, `platform_pins`, `platform_clocks`,
  `resolve_platform_for`) (Phase 4)
- `lib/ice40.vir`
- `@cell` name mapping in Verilog emission
- `lib/*.vir` loading in `load_vir_dir` / `project_db`
- `build` module (constraints + toolchain)
- `vir bitstream` / `vir upload` rewrite
- `[prog] top` support and scaffold updates
- Tests

---

## Phases

The implementation is split into the following phase files in this directory:

| Phase | File                    | Summary                                                                                                                                            |
|-------|-------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| 0     | [PHASE_00.md](PHASE_00.md) | Grammar, payload, and AST accessors for `platform`: the `KwPlatform` token/keyword, `PlatformDef`/`PlatformStmt` grammar rules, the `Platform` payload struct, and the `ast.rs` accessors. **Done.** |
| 1     | [PHASE_01.md](PHASE_01.md) | The `item_children` helper in `ast.rs` that filters out the `Annotations` wrapper and `Annotation` children. **Done.**                              |
| 2     | [PHASE_02.md](PHASE_02.md) | The `for` clause: remove the v2 single-platform `PackageAnalysis` work, then add `KwFor`, the optional `Ofness` child on `ModDef`, the `platform_for()` accessor, and the `package.rs` match arm. **Done.** |
| 3     | [PHASE_03.md](PHASE_03.md) | Symbol table support: `SymbolKind::Platform` and `build_symboltable_platform_slot` so platform ports get `Component` symbols. (First implementation was reverted; re-apply unchanged.) |
| 4     | [PHASE_04.md](PHASE_04.md) | v4: the `check_platform_ports` helper (a plain helper in `Check()`, not a query), platform accessor functions, the port-match diagnostics, and the `ports.rs` guard. No `PlatformAnalysis`; no db changes. |
| 5     | [PHASE_05.md](PHASE_05.md) | `@cell` consumption and the shipped platform file: create `lib/ice40.vir` and make Verilog emission use the `@cell("...")` name for ext mods.        |
| 6     | [PHASE_06.md](PHASE_06.md) | Lib loading: `load_vir_dir` loads every `lib/*.vir`, plus `resolve_cwd`/`read_prog_key` helpers in `vir.rs`.                                        |
| 7     | [PHASE_07.md](PHASE_07.md) | New `build` module: PCF constraint emission and the ice40 toolchain runner/flasher, moved out of `vir.rs`.                                          |
| 8     | [PHASE_08.md](PHASE_08.md) | Build flow rewrite of `vir bitstream` / `vir upload`: `[prog] top` selection, platform from the top module's `for` clause, toolchain dispatch, and deletion of the old inline ice40 code. |
| 9     | [PHASE_09.md](PHASE_09.md) | Scaffolding and tests: `vir new` writes `[prog] top`, rewrite `assets/blink.vir` with `for ice40::IceSugar`, and add the happy-path and port-mismatch project tests. |

---

## To Add a New Platform

Because platforms are data (a `.vir` file) rather than code, the work
depends on whether the FPGA *family* is already supported.

**A new board in a supported family (today: ice40):**

1. Open `lib/<family>.vir` (e.g. `lib/ice40.vir`) and add one
   `platform` item with:
   - `@fpga("<family>")` and `@part("<part>")` annotations on the item,
   - one port per physical pin, each with a `@pin(...)` annotation
     (numeric or named),
   - a `Clock`-typed port with a `@period_ns(...)` annotation.
2. Nothing else.
   Constraint generation and toolchain dispatch are keyed off the
   `@fpga` family, and the whole `lib/` directory ships as-is.
   Multiple `platform` items in one file are fine (Decision 3).

**A new FPGA family:**

1. Everything above, in a new `lib/<family>.vir`, plus code touch
   points in the build module, keyed by the `@fpga` family string:
   - extend `toolchain_for` and add a runner plus a flasher in
     `build/toolchain.rs`, and a constraint emitter in
     `build/constraints.rs`.
   There is no separate family whitelist to extend: `toolchain_for`
   returning `None` *is* the rejection, at the point of use
   (Decision 13).
2. Update the tests that assert which families are supported.

**Custom boards (not shipped):**
Future Work; see below.

---

## Test Cases Not Covered

Coverage was deliberately kept minimal for v1.
Candidates for later:

- Platform data errors at the build flow's point of use: a port
  missing `@pin`, an unknown `@fpga` family, no clock port (these
  surface as `Unknown` diagnostics or CLI errors in Phases 7/8, not
  as `vir check` diagnostics — Decision 13).
- Duplicate port names in a platform item (`DuplicateSlot`).
- `for` clause naming a non-platform item (`ExpectedPlatform`).
- `for` clause with an unresolved name (`UnresolvedItem`).
- Type-only mismatch (right name, wrong type) between module and
  platform.
- Direction mismatch between module and platform.
- Missing `[prog] top` at `vir bitstream`.
- `[prog] top` naming a module that does not exist or is not a mod.
- `@cell` name emission in generated Verilog (currently only
  exercised by the unit test in Phase 5).
- Non-PCF constraint formats (Future Work).
- End-to-end bitstream generation and flashing (requires
  yosys/nextpnr-ice40/icepack/icesprog).
- ECP5 / Nexus / Gowin / Vivado rejections.
- Custom project-local platform files.

## Future Work

- **Script-driven toolchains.**
  The long-term goal is that adding a platform requires no compiler
  changes at all: a platform should be able to ship a set of scripts
  that `vir` invokes for synthesis, place-and-route, and upload.
  The split between platform *files* and toolchain *dispatch* is
  designed so this migration is natural.
- **`@constraints` escape hatch.**
  Let a platform point at an external constraints file used verbatim
  instead of generating one from `@pin`s.
- **More shipped platforms.**
  Additional ice40 boards in `lib/ice40.vir`; a `nexys4.vir`
  (Artix-7) draft is parked in Future Work of v2.
- **More FPGA families.**
  ECP5, Nexus, Gowin, and Xilinx flows, with their constraint
  formats (LPF, CST, XDC).
- **Custom project-local platforms.**
  Let a project provide `<project>/platform/<name>.vir` shadowing
  the shipped platforms.
- **Documentation.**
  A `docs/source/reference/platforms.rst` page.
- **Expanded test coverage** per the section above.

## Acceptance Criteria

At the end of Phase 9:

- `make grammar` succeeds, and `make test` passes including the two
  new tests.
- `vir new foo` scaffolds a project whose `Virdant.toml` sets
  `[prog] top = "Top"`, and whose `src/blink.vir` says
  `import ice40` and `mod Top for ice40::IceSugar`; `vir check`
  passes on it (the lib package loads, the `for` clause resolves).
- `vir check` on a project without any `for` clause or platform
  import behaves exactly as before (no platform diagnostics).
- A module whose ports do not exactly match its `for`-clause platform
  fails `vir check` with a clear port-mismatch error.
- `vir bitstream` on a project whose top module has no `for` clause
  (or no `[prog] top`) fails with a clear message before any external
  tool is invoked.
- `@cell`-annotated ext mods are emitted in Verilog under their cell
  name (e.g. `SB_RAM40_4K`), and ext mod definitions are not emitted.
- With `yosys`, `nextpnr-ice40`, and `icepack` on PATH,
  `vir bitstream` on the scaffolded blink project produces
  `build/<project>.bin` (manual verification).
- With `icesprog` on PATH, `vir upload` flashes it (manual
  verification).
- `vir check` passes on any parseable platform file (platforms are
  unvalidated data, Decision 13); invalid platform *data* (a port
  missing `@pin`, an unknown `@fpga`) produces a clear error when
  the build flow reads it, via `Unknown` diagnostics or CLI errors.
