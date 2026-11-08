# Plan: virdant_skill

Date:   2026-11-06
Status: In Planning

## Goal

Rewrite the Virdant skill (`virdant/agents/skills/virdant/`) so it serves
**coding agents** as the primary consumer. The language moved enormously in
the last month (`let`, `pub`, generics, `dependson`, tristate, platforms,
clock inference, one-hot enums, payload enumerants, `+.`/`-.`, `dyn`, holes,
diagnostics…) and the skill covers almost none of it. The skill must become
self-contained and current, so an agent never needs to leave it.

## Decisions (from interview)

- **Audience**: coding agents. Token-efficient, rule-first, copyable examples.
- **Self-contained**: all load-bearing content lives in the skill; virdant.org
  links become "further reading," never the primary source.
- **Structure**: `SKILL.md` carries the always-needed core *inline* (agents
  reliably read SKILL.md, often skip linked files), with topic files for depth.
- **Frontmatter triggers**: "virdant" / "Virdant" / "vir", plus FPGA / HDL /
  Verilog / hardware contexts.
- **`tools.md`**: public-facing `vir` subcommands only (exactly what `vir -h`
  shows). No test-runner internals, no hidden flags.
- **`diagnostics.md`** (not "errors-and-warnings.md").
- **`idioms.md`**: canonical patterns; the "Virdant vs Verilog differences"
  table folds into it.
- **No snippet-testing suite.** Correctness comes from careful grounding in
  the reference docs and the test corpus, not CI.
- **Repo bugs found while writing** are summarized in `MISTAKES.md` at the
  workspace root (not fixed in this plan).

## Ground rules for every file (style contract)

1. Rules as bullets; prose only when a rule needs justification.
2. Every example must compile against **v0.4.0 idiom**: implicit `on` when the
   module has exactly one `Clock` (redundant `on clock` is now a *warning*),
   `pub` on cross-package items, `//>` docstrings, snake_case filenames,
   4-space indent, no tabs, no trailing spaces.
3. Cross-reference sibling files by relative path (e.g. see `./let-bindings.md`).
4. Ground every claim in `virdant/docs/source/reference/*.rst` and, where docs
   and tests disagree, **the tests win** — and the disagreement is reported
   back for `MISTAKES.md`.
5. Known existing bug to fix in passing: SKILL.md references
   `./module-system.md`; the file is `modules.md`.

## Work packages

| # | File | Action | Grounding sources |
|---|------|--------|-------------------|
| 1 | `SKILL.md` | **Rewrite** (core inline + router) | all others; `reference/index.rst` |
| 2 | `coding-conventions.md` | Update | `grammar.rst`, `packages.rst`, tests/errors/style, tests/errors/syntax |
| 3 | `driver-blocks.md` | Update (clock inference, `initial`, chaining, culling) | `statements.rst`, tests/warnings/{redundant_driver,redundant_on_clause,no_reg_drivers,not_it_initial} |
| 4 | `control-flow.md` | Update (arm forms, guards, let arms) | `statements.rst`, `expressions.rst`, tests/errors/style/if_match_style |
| 5 | `types-and-patterns.md` | Update (one-hot enums, payload enumerants, `dyn`, `Valid[T]` one-hot width) | `types.rst`, `patterns.rst`, tests/check/{enum_onehot,index_dyn} |
| 6 | `literals-and-builtins.md` | Update (`+.`, `-.`, `all`, `trunc`, `?` holes, literal truncation) | `expressions.rst`, `grammar.rst`, tests/run/{add_carry,literal_*}, tests/sim/literals |
| 7 | `modules.md` | Update (`pub`, import/visibility rules, `ext mod` + `@parameter`, forwarding block) | `modules.rst`, `packages.rst`, `annotations.rst`, tests/check/pub_kinds |
| 8 | `sockets.md` | Update (generic sockets, mono-socket `:=:`, channel `dependson`) | `sockets.rst`, tests/check/dependson_sockets |
| 9 | `let-bindings.md` | **New** (statement + expression, terminator style, shadowing, `CantInfer` anchoring) | `statements.rst`, `expressions.rst`, tests/{run,check,sim}/let*, tests/errors/{style,naming}/let*,undefined_let |
| 10 | `generics.md` | **New** (Type/Nat kinds, generic types/mods/sockets, mono modules, ext-mod & top restrictions, error cases) | `types.rst`, `modules.rst`, `sockets.rst`, tests/check/generics_*, tests/golden/generics, tests/errors/platforms/top_generics |
| 11 | `dependson.md` | **New** (interface contract, error vs unused-warning, socket obligations, cycle detection) | `modules.rst` (dependson + cycles), tests/errors/{cycles,dependson}, tests/check/dependson_* |
| 12 | `tristate.md` | **New** (`bus`/`net`, `#=`, `#-`, `#=#`, `tap`, float/strong/weak, I²C open-drain) | `modules.rst`, `statements.rst`, `expressions.rst`, tests/sim/i2c_open_drain |
| 13 | `platforms.md` | **New** (`platform` items, `for` clause, port matching, `default`, `@noinitial`, shipped `ice40`, flows) | `platforms.rst`, `annotations.rst`, `lib/ice40.vir`, tests/errors/platforms |
| 14 | `tools.md` | **New** (`vir -h` surface only: new, check, run, poke, doc, format, ls, completion, build, synth, pnr, bitstream, upload, version, tutor) | `vir -h` output, docs "Tools" section, tests/poke, tests/cli |
| 15 | `diagnostics.md` | **New** (error families + warnings cheat sheet, organized like tests/errors/* and tests/warnings/*) | tests/errors/**, tests/warnings/** |
| 16 | `idioms.md` | **New** (FSM via enum-state + match-on-reg-selfref; handshake/`Pipe[T]`; debouncer/edge-detect; shift/barrel-shifter; Virdant-vs-Verilog differences table; reserved keywords) | tests/run/**, tests/sim/**, `grammar.rst` keywords |
| 17 | `MISTAKES.md` (workspace root) | **New** — consolidates repo bugs found during writing | children's reports |

## Orchestration

- Nine parallel subagent children, grouped by shared grounding:
  - A: `let-bindings.md` + `control-flow.md`
  - B: `generics.md` + `sockets.md`
  - C: `dependson.md` + `modules.md`
  - D: `tristate.md` + `types-and-patterns.md`
  - E: `platforms.md`
  - F: `tools.md`
  - G: `diagnostics.md`
  - H: `idioms.md`
  - I: `coding-conventions.md` + `driver-blocks.md` + `literals-and-builtins.md`
- Children **report** repo bugs in their output; the parent consolidates them
  into `MISTAKES.md` (no concurrent writes to shared files).
- Parent writes `SKILL.md` **last**, after all topic files exist, so the
  inline core and the "read this when…" index match reality exactly.

## Acceptance

- `SKILL.md` index lists exactly the files that exist; no `module-system.md`
  references anywhere.
- No stale examples: no redundant `on clock` in single-clock modules, no
  hyphenated `.vir` filenames, all examples use `pub`/`//>` idiom.
- Every feature shipped this month is covered by at least one file (per the
  work-package table).
- `MISTAKES.md` exists at the workspace root with the reference-doc bugs found.
