# Verilog Module Parameters via Annotations

## Summary

`ext mod`s in `lib/ice40.vir` wrap Verilog/SiliconBlue hard cells, many of
which are configured through Verilog `parameter`s (e.g. `SB_PLL40_PAD`'s
`DIVR`/`DIVF`/`DIVQ`, `SB_IO`'s `PIN_TYPE`). Virdant has no way to set
these today, so every instantiation uses the cell's hardcoded Verilog
defaults.

This plan threads parameter values through to the generated Verilog
`#(...)` instantiation block using the `@parameter(...)` annotation,
reusing the multi-argument annotation grammar already implemented
(commit `5183ad2`, "Annotations now accept multiple arguments, boolean
literals, and empty parens"). No grammar changes are required.

Consistent with how annotations are treated everywhere else in Virdant
(`@cell`, `@pin`, `@part`, `@period_ns`; see
`docs/source/reference/annotations.rst` and `plans/platforms/OVERVIEW.md`
Decision 13), this is **unvalidated data**: the compiler does not check
parameter names, value kinds, counts, or applicability. Mistakes surface
only when a downstream tool (Yosys, a Verilog simulator) rejects the
generated text.


## Syntax

Two sites, both already parse under the current grammar:

```virdant
// Declaration: on an `ext mod`, one @parameter per parameter.
// A bare name has no default; a (name, value) pair declares a default.
@parameter("DIVR")
@parameter("DIVF", 0)
@parameter("FEEDBACK_PATH", "SIMPLE")
ext mod SbPll40Pad {
    // ports...
}

// Supply: on a submodule instantiation, (name, value) overrides the
// declared default (or supplies a value for a parameter with no default).
@parameter("DIVR", 4)
@parameter("FEEDBACK_PATH", "PHASE_AND_DELAY")
mod pll of SbPll40Pad {
    // it.* connections...
}
```

- `NAME` is a `Str` literal (e.g. `"DIVR"`), parsed as
  `AstNodePayload::ExprStrLit` and unquoted exactly like any other `Str`
  annotation argument — not a bare identifier. This means parameter-name
  extraction needs no special-casing at all: both arguments of
  `@parameter(...)` are read with the exact same literal-matching logic
  already used by `annotation_values`/`annotation_arg_value`.
- `DEFAULT` / `VALUE` is a literal expression:
  - `Nat`/`Word` (e.g. `4`, `0xF`, `4w8`) → emitted as a plain decimal
    integer; width information is discarded (consistent with how
    `@pin`/`@period_ns` already treat `AnnotationValue::Nat`).
  - `Str` (e.g. `"SIMPLE"`) → emitted as a quoted Verilog string literal.
  - `true`/`false` → emitted as `1'b1`/`1'b0`.
- Any other shape (zero args at the instance site, a non-`Str` first
  argument, a non-literal second argument, more than two arguments) is
  silently skipped by the extraction helper. No diagnostic is ever
  produced for `@parameter`.


## Semantics (decided)

1. **Not restricted to `ext mod` targets.** The mechanism is applied to
   *any* submodule instantiation uniformly, with no `is_ext` check and no
   dedicated error handling. Attaching `@parameter` to an instance of an
   ordinary (non-`ext`) Virdant mod still produces a `#(...)` block in the
   generated Verilog; that mod's generated module declares no matching
   `parameter`, so the emitted Verilog will fail at elaboration. This is
   accepted as out of scope rather than guarded against, per explicit
   direction: ordinary mods have no way to make use of parameters, and no
   annotation error-handling is being added at this time.
2. **Defaults always apply.** If the target `ext mod` declares
   `@parameter(NAME, DEFAULT)`, every instantiation of it emits
   `.NAME(DEFAULT)` unless the instance supplies its own
   `@parameter(NAME, VALUE)` — including instantiations with zero
   `@parameter` annotations of their own. Ordinary default-argument
   semantics.
3. **No validation at all.** Not of parameter names (an instance may
   supply a name the `ext mod` never declared — it is still threaded
   through), not of value kinds (a `Nat` default can be silently
   "overridden" by a `Str` value), not of cardinality (repeating
   `@parameter("DIVR", ...)` twice at the same site is not deduplicated
   by any dedicated logic — ordered-map insertion means the last one
   wins, as a side effect of the data structure, not a designed rule).
4. **`lib/ice40.vir` is not migrated by this plan.** No cell in that file
   gains `@parameter` annotations as part of this work.
5. **`WordValue` is not widened.** See "Known limitation" below; tracked
   separately in `NOTES.md`, not a phase of this plan.


## Known limitation: 64-bit values only

`AnnotationValue::Nat` and the literal parsing behind it
(`util::parse_nat_literal`, called transitively from
`AstNode::annotation_arg_value`) resolve into `WordValue = u64`
(`common.rs:10`). A parameter value wider than 64 bits — the only
concrete case found in `lib/ice40.vir` is `SB_RAM40_4K`'s
(and its `*NR`/`*NW`/`*NRNW` variants') 256-bit `INIT_0`..`INIT_F`
fields — cannot be expressed:

- As a `Nat`/`Word` literal, parsing would panic (`u64::from_str_radix`
  called with `.unwrap()` on a value that overflows `u64`).
- As a `Str` literal instead, the planned renderer would wrap it in
  Verilog quotes (`."INIT_0"("256'h000...")`), which is semantically
  wrong — Str is for genuinely string-typed parameters like
  `FEEDBACK_PATH`, not a way to smuggle an unquoted numeric literal
  through.

This is a pre-existing limitation of `WordValue` as the representation
for every `Word[N]` constant in the compiler and simulator — not
something introduced by, or fixable within, this feature. It is
queued as a separate, foundational future project (see `NOTES.md`).


## Implementation phases

### Phase 1 — AST helper

File: `virdant/src/syntax/ast.rs`.

Add, alongside `annotation_values`:

```rust
/// (name, optional value) pairs for every `@parameter("NAME"[, VALUE])`
/// annotation on this node, in source order.  The first argument must
/// be a `Str` literal (the parameter name); annotations whose first
/// argument is not a `Str` literal are skipped entirely.  The second
/// argument, if present, is read the same way `annotation_values` reads
/// a literal; any other expression shape yields `None` (no
/// default/value, not an error).
pub fn parameter_annotations(&self, parsing: &Parsing) -> Vec<(BString, Option<AnnotationValue>)>
```

Implementation notes:
- Iterate `self.annotations()`, filter to ones whose `annotation_name()`
  is `"parameter"`.
- For each, call `annotation_args()` (the raw, unfiltered child list —
  *not* `annotation_values()`, which already discards non-literal
  arguments and would desync the name/value positions).
- First arg → reuse the private `annotation_arg_value` helper (promote
  its visibility to `pub(crate)`); only an `AnnotationValue::Str` result
  counts as a name, so `@parameter(DIVR)` (bare identifier, no quotes)
  and `@parameter(4)` are both skipped, silently. Skip the whole
  annotation if this isn't a `Str`.
- Second arg (if present) → the same `annotation_arg_value` helper, used
  as-is (`Nat`, `Str`, or `Bool`), to get `Option<AnnotationValue>`.
- This one helper is used identically at both call sites (Phase 3): a
  `ModDef` node's own annotations (declarations) and a `Submodule`
  node's own annotations (supplied values) — the two forms are
  structurally identical, only their *meaning* (default vs. override)
  differs, and that meaning is supplied by the caller, not the helper.

### Phase 2 — Verilog IR

File: `virdant/src/verilog.rs`.

- Add a field to `Submodule`:
  ```rust
  pub struct Submodule {
      pub name: String,
      pub submodule_name: String,
      pub ports: Vec<String>,
      pub parameters: Vec<(String, String)>, // (param name, rendered Verilog literal text)
  }
  ```
- Extend `Submodule::write()`: when `parameters` is non-empty, emit a
  `#( .NAME(value), ... )` block between `submodule_name` and `name`,
  reusing the alignment style already used for the port list:
  ```verilog
  SB_PLL40_PAD #(
      .DIVR(4),
      .DIVF(0)
  ) pll (
      .PACKAGEPIN(...),
      ...
  );
  ```
  When `parameters` is empty, emit exactly what is emitted today (no
  behavior change for existing designs).

### Phase 3 — Conversion wiring

File: `virdant/src/verilog/conversion.rs`.

- Add to `Converter`:
  ```rust
  /// "package::Module" -> [(param name, declared default, in @parameter order)]
  module_declared_params: IndexMap<String, Vec<(String, Option<AnnotationValue>)>>,
  ```
  populated in `build_module_info()` in the same loop that already
  builds `module_ports`/`emitted_module_names`, by calling
  `item_ast.parameter_annotations(&parsing)` on each `ModDef` and
  converting the `BString` name to `String`.
- Add:
  ```rust
  fn render_param_value(value: &AnnotationValue) -> String
  ```
  mapping `Nat(n)` → `n.to_string()`, `Str(s)` → a double-quoted Verilog
  string (escaping not attempted — unvalidated data), `Bool(b)` →
  `"1'b1"`/`"1'b0"`.
- In the `AstNodePayload::Submodule(module)` arm of the statement loop
  that already builds `verilog::Submodule { name, submodule_name, ports }`
  (`conversion.rs` ~lines 275-299):
  1. Read the instance's own supplied values:
     `stmt.parameter_annotations(&parsing)` (the `stmt` node here *is*
     the `Submodule` AST node — same node whose `Ofness` child is read
     a few lines above for `mod_path`).
  2. Look up `self.module_declared_params.get(&mod_path)` (empty if the
     target has none, e.g. it isn't an `ext mod` or declares nothing).
  3. Merge into one ordered list: walk the declared list first,
     emitting `(name, rendered)` using the instance's supplied value if
     present for that name, else the declared default if present, else
     skipping the name entirely (no default and no supplied value: emit
     nothing, same as today). Then append any names the instance
     supplied that were *not* in the declared list, in the instance's
     own annotation order (per Decision 3: not declared ≠ not threaded
     through).
  4. Attach the result as `parameters` on the constructed
     `verilog::Submodule`.

### Phase 4 — Tests

- A unit test in `conversion.rs`'s existing `#[cfg(test)] mod tests`
  block, next to `cell_annotation_names_ext_mod_instantiation`: define a
  minimal inline `ext mod` with `@parameter(NAME, DEFAULT)` declarations,
  instantiate it twice (once overriding one parameter and leaving
  another at its default, once with no `@parameter` at all), and assert
  the resulting `verilog::Submodule.parameters` contents directly for
  both instances.
- A new `tests/pass/parameters/` project directory, following the
  minimal style of `tests/pass/annotations/` (a `src/top.vir` plus a
  `Makefile` that just runs `vir check`), proving `@parameter` on both
  an `ext mod` declaration and an instantiation parses and checks
  cleanly end to end. Added to `VIR_PROJS` in `tests/pass/Makefile`.


## Acceptance criteria

- `make test` passes, including the new unit test and
  `tests/pass/parameters/`.
- An `ext mod` instantiation with `@parameter` overrides and/or declared
  defaults emits a `#(...)` block in the generated Verilog containing
  exactly the merged (override-or-default) set, in the order described
  in Phase 3.
- An instantiation with no `@parameter` annotations, of an `ext mod`
  with no `@parameter` declarations, emits no `#(...)` block — byte-for-
  byte identical output to today, for every existing passing test and
  for `lib/ice40.vir` as it stands unmigrated.
- No new diagnostics are introduced; malformed `@parameter` usage (wrong
  argument count/shape, undeclared names, mismatched value kinds) is
  silently accepted everywhere, including `lib/ice40.vir`'s own
  `tests/pass/platform_*` fixtures.


## Out of scope / future work

- Migrating `lib/ice40.vir`'s cells (`SB_IO`, `SB_GB_IO`, the five
  `SB_PLL40_*` variants, `SB_RGBA_DRV`, `SB_RGB_DRV`, `SB_HFOSC`,
  `SB_MAC16`, and `SB_RAM40_4K`'s `WRITE_MODE`/`READ_MODE`) to use
  `@parameter` declarations. Every parameter in that list fits within a
  64-bit `WordValue`; this is purely deferred, not blocked.
- `SB_RAM40_4K` (and `*NR`/`*NW`/`*NRNW`)'s `INIT_0`..`INIT_F`
  parameters (256 bits each) — blocked on widening `WordValue`
  (see `NOTES.md`).
- Any validation of `@parameter` (unknown names, value-kind mismatches,
  duplicates) — would need a dedicated opt-in design, since the general
  annotation philosophy in this codebase is unvalidated data.
- Restricting `@parameter`'s effect to `ext mod` targets only, if the
  silent-garbage-on-ordinary-mods behavior turns out to be a problem in
  practice.
