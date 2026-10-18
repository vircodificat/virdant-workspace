# Phase 6 (v3): Lib Loading

Under v3, platform packages are not selected by config; every shipped
lib package is loaded into every project db, and `import` gates
visibility (Decision 10). Loading is not scoping, so loading all of
`lib/` is observably equivalent to on-demand loading — without any
import pre-scanning.

### 6.1 `load_vir_dir` loads every `lib/*.vir`

**Why:** `lib/ice40.vir` must be present in the `PackageTable` for
`import ice40` to resolve. `load_vir_dir` (`util.rs:34-50`) currently
loads exactly `builtin.vir` plus the source dir; extending it to
sweep the whole lib dir makes every shipped platform package
available uniformly, through the same code path as `builtin`.

Edit `virdant/src/util.rs`, `load_vir_dir`:

```rust
fn load_vir_dir<P: Into<std::path::PathBuf>>(source_dir: P) -> Vec<(BString, BString)> {
    let mut files = vec![];
    // Load every *.vir in LIB_DIR (builtin.vir, ice40.vir, ...).
    for filepath in std::fs::read_dir(crate::LIB_DIR)
        .expect("Could not open lib directory")
    {
        let filepath = match filepath {
            Ok(filepath) => filepath.path(),
            Err(_) => continue,
        };
        match filepath.extension() {
            Some(ext) if ext.to_string_lossy() == "vir" => (),
            _ => continue,
        }
        files.push(Source::load_file(filepath));
    }
    let source_dir: std::path::PathBuf = source_dir.into();
    for filepath in std::fs::read_dir(&source_dir).expect(&format!("Could not open directory: {source_dir:?}")) {
        // ... unchanged: push each source-dir *.vir ...
    }
    files
}
```

Notes:
- The db-building helpers (`db_from_dir`, `db_from_file`,
  `db_from_files`, and their `_with_lib` variants at `util.rs:53-109`)
  all funnel through `load_vir_dir` or duplicate its builtin-only
  logic; route them all through the updated `load_vir_dir` so every
  db construction path sees the lib packages. (`db_from_files_with_lib`
  keeps its explicit `lib_dir` parameter and sweeps *that* directory
  the same way, so tests can still use a controlled lib dir.)
- There is deliberately no `db_from_dir_with_platform` — v2's
  config-selected loading is gone with `[prog] platform`.
- Package-name collisions between lib files or between a lib file and
  a project file are a user error; `PackageTable::new` panics on
  duplicates today, which is acceptable for shipped content.

### 6.2 `resolve_cwd` and `read_prog_key` helpers

**Why:** Phase 8's `bitstream` needs `[prog] top`, and the four
inlined cwd-resolution blocks in `vir.rs` are duplicated exactly the
way v2 described; the helpers are still wanted even though
`read_prog_key("platform")` is gone.

Edit `virdant/src/bin/vir.rs`. Add two small helpers:

```rust
fn resolve_cwd(args: &Args) -> PathBuf {
    // Canonicalize args.cwd if present, else env::current_dir().
    // Extract from the four inlined copies (vir.rs:139, :715, :770, :950).
}

fn read_prog_key(cwd: &Path, key: &str) -> Option<String> {
    // Read Virdant.toml, return [prog].<key> as a String if present.
}
```

`read_prog_key` must tolerate a missing file or missing keys
(returning `None`). In v3 the only key read is `top` (Phase 8);
`scaffold`'s existing toml keys are untouched.

`project_db` (`vir.rs:124-169`) needs no platform-specific wiring in
v3: it already ends in `db_from_dir(src_dir)` (`:168`), which now
loads the lib packages via 6.1. The `-F` single-file path of
`project_db` (`:125-137`) is unchanged — single-file mode does not
load lib packages, matching today's behavior for `builtin`-only
deps. (`tests/` drive `-F` mode with `builtin` types only, so the
two new tests in Phase 9 use project mode.)
