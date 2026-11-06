# Plan: `vir poke` — peek/poke CLI tester

Date:   2026-11-04
Status: Draft

A REPL-driven peek/poke tester for Virdant built on the `sim` library,
with a small custom scripting language whose expressions use Virdant
syntax. Scripts double as assertion tests.

## 1. Summary

`vir poke` opens an interactive REPL that holds a live, elaborated
`Sim` for one top module. Sessions can preload a script; scripts can
also run standalone as assertion tests (exit code reflects pass/fail).

```
vir poke                          # REPL against project top (from virdant.toml discovery)
vir poke --top Top                # REPL against an explicit top (may be non-pub)
vir poke test.pk                  # REPL, script preloaded
vir poke test.pk --script         # standalone script run (alias: vir poke --script test.pk)
```

The project is discovered via `virdant.toml` in the cwd or an ancestor
(the existing `ProjectLocation` resolution in `vir/src/common.rs`).

## 2. Command set (v1)

### Navigation

| Command        | Effect                                                                 |
|----------------|------------------------------------------------------------------------|
| `cd <path>`    | Drill into a submodule (one or more levels: `cd foo.bar`)               |
| `cd ..`        | Go up one level                                                        |
| `cd /`         | Return to top (proposed spelling; see open question §9)                 |

The session has a **current module**. Signal paths in commands are
resolved relative to it.

### Signals

| Command             | Effect                                                              |
|---------------------|---------------------------------------------------------------------|
| `peek <path>`       | Print value of a signal, round-trippable syntax (§4)                 |
| `poke <path> <expr>`| Drive a signal (restrictions below)                                  |
| `watch <path>`      | Print the signal whenever it changes (`Sim::on_change`)              |

**Poke restrictions (semantics, not just policy):** `poke` is only
meaningful on **top-level incoming ports** and **registers (any depth)**.
Virdant settles combinationally — poking anything else is immediately
overridden by the next settle, so the tool rejects it with a clear
diagnostic rather than silently doing something misleading.

`peek` and `assert` may read **any** signal reachable from the top,
including submodule wires and regs that are not normally visible from
outside — the tester is a debugging tool, and visibility rules do not
apply to it.

### Clocks & time

| Command          | Effect                                                             |
|------------------|--------------------------------------------------------------------|
| `tick [n] [<path>]` | Apply `n` clock edges (default `n=1`). `path` defaults to the current module's default clock; if the default is not unique (or absent), it is an error and a path must be given. |

### Assertions & output

| Command                | Effect                                                           |
|------------------------|------------------------------------------------------------------|
| `assert <expr>`        | Evaluate expression; failure = stopping event (§5)                |
| `print <expr>`         | Evaluate and print an expression                                  |
| `restart`              | Re-initialize the simulation (fresh `InitStrategy`, re-run `initial` statements) |

### Session

| Command  | Effect                                     |
|----------|--------------------------------------------|
| `exit`   | Leave the REPL (also Ctrl+D, Ctrl+C)        |

Paths in `assert`/`print` expressions are relative to the **current
module**; `top.` prefixes reach absolute paths. `peek`/`poke` paths
follow the same rule.

## 3. Scripting language

A new custom line-based language (extension: **`.pk`**). One command
per line; `#` comments. The REPL accepts the same language one line at
a time — one parser shared by both modes.

**Expressions: core subset of Virdant's `Expr` grammar:**

- Paths (`foo.bar`, `top.foo.bar`, `it` for the previously peeked value)
- Literals: `Nat`/word literals (`42`, `0b1010`), bit literals (`true`, `false`)
- Unary: `-`, `~`, `!`
- Additive/bitwise: `+`, `-`, `&`, `|`, `^`
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `&&`, `||`, `^^`
- Indexing/slicing: `x[3]`, `x[7..4]`

**Deliberately out of v1, but the parser is structured to add them:**
`when`/`match`/`let` blocks, struct expressions (`${...}`), enum
constructors (`#Some(...)`), union values, method calls (`->ident`),
`tap`, ascription (`expr: Type`).

**Special types (structs, enums, unions):** room is reserved in both
directions:

- *Input:* expression evaluation should be designed so literal
  constructors for structs/enums/unions can be added without rework
  (an `Expr` AST distinct from the parser's token stream).
- *Display:* values of struct/enum/union type render in
  **round-trippable script syntax** (§4), so the same literal grammar
  serves both input and output.

Implementation note: values come from `Sim::get()` as `Value`s; the
script interpreter is a small evaluator over those, not a reuse of the
compile-time expression machinery — but type information (needed for
bit widths and struct/enum layouts) comes from the elaborated circuit.

## 4. Value display

`peek`, `print`, and assertion diagnostics print values in the same
literal syntax the script language accepts (round-trippable):

- Words with their width, e.g. `0b1010` — wide enough to copy into a
  `poke` or `assert` unchanged.
- Bits as `true`/`false`.
- Structs/enum/unions (when implemented) as their constructor syntax,
  recursively.

## 5. Assertion semantics

- **Standalone script:** first failed `assert` stops the script with a
  nonzero exit code. Passing scripts exit 0 (CI/`make check` friendly).
- **Preloaded script (REPL):** first failed `assert` pauses the script
  and returns control to the interactive REPL, so the failing state can
  be inspected. (A command to continue/abort the paused script can be
  considered; not required for v1.)
- **Direct REPL:** a failed `assert` prints a message and continues.
- **Failure output:** the assertion's source line plus both sides of
  the comparison with concrete (round-trippable) values.

## 6. CLI surface

Handled by the existing `vir` CLI dispatch (`vir/src/vir/cli.rs` +
`main.rs`), with `ProjectLocation` resolution inherited from
`vir/src/common.rs`.

- `vir poke` — REPL, top module from project default.
- `--top <Top>` — explicit top module; **may be non-pub**. This needs a
  corresponding bypass in elaboration (the pub-check on top selection
  must be relaxable when requested by the tester).
- `<SCRIPT>` positional — preload into REPL.
- `<SCRIPT> --script` / `--script <SCRIPT>` — standalone run.

## 7. Implementation sketch

- New subcommand crate/module: `vir/src/poke/`
  - `main.rs` — CLI entry, project location, elaboration of top, `Sim::new`
  - `repl.rs` — line loop, prompt, history; preloaded-script state machine
  - `script.rs` — `.pk` tokenizer/parser, one parser shared with REPL
  - `expr.rs` — expression AST, parser (core subset §3), evaluator over `Value`
  - `display.rs` — round-trippable value rendering
  - `commands.rs` — command dispatch: peek/poke/watch/cd/tick/assert/print/restart/exit
- Sim wiring: `Sim::try_resolve` for paths (relative resolution via the
  current module's prefix), `SimLock::set` for pokes (deferred settle),
  `Sim::tick` / `attach_clock` for clocks, `Sim::on_change` for watch,
  `Sim::now` for time reporting.
- Poke legality check: component kind + depth of resolved signal
  (top-level input port or register) — reject otherwise with a
  diagnostic naming the nearest legal target if apparent.
- Default-clock resolution for bare `tick`: inspect the current module's
  clock declarations; error if ambiguous.
- Non-pub tops: plumb an explicit flag through elaboration query used by
  the tester.

## 8. Testing

- Unit tests for the `.pk` parser and expression evaluator.
- Integration tests: `.pk` scripts run standalone against small
  test designs; failure exits nonzero, success exits 0.
- REPL smoke test via piped stdin.

## 9. Open questions

- Spelling for "jump back to top" in `cd` (`cd /`? `cd top`?).
- Whether a `continue` command should resume a paused preloaded script.
- Exact rendering of wide words (grouping? width prefix style?) —
  decided during implementation against real waveforms.
