# Plan: Guide Redo — Lesson Sequence

Date: 2026-11-04
Source: brainstorm session (see session transcript)

## Goal

Completely redo the website language guide (`virdant/docs/source/guide/`) from
scratch, organized as **short sequences of lessons** with a light theme or goal
that each sequence builds up to. The current lessons feel too jumpy — too many
variations crammed into single lessons (see the `adder` split into
halfadder/fulladder/adder as the model).

## Decisions (from interview)

1. **Example-driven.** Sequences are built around example circuits that arise
   naturally; features are taught as they're needed. Coverage is audited at the
   end, not used as the skeleton.
2. **Nearly all features covered.** Deprioritized (may appear in Advanced
   Topics only, or not at all): tristates (`drive-strong`/`drive-weak`,
   `net`/`bus`/`tap`, `net-attach`, `bidi-connect`), one-hot encoding,
   `outgoing-reg`/`outgoing-wire`, `dontcare`, `unused`.
3. **Audience: hobbyist / software person.** No prior digital-logic knowledge.
   The guide teaches digital logic *and* Virdant simultaneously — early lessons
   pace concepts (what is a signal, what is a clock) as carefully as syntax.
4. **The capstone is a reward, not the destination.** A simple RISC-V core is
   the final sequence. Readers can stop after any sequence and still leave with
   something working. Foundational sequences are the priority.
5. **Behavior shown with VCD / gtkw screenshots.** Waveforms from the Virdant
   simulator, rendered as screenshots, embedded in lessons (especially from
   Clocks onward).
6. **No exercises.** Pure read-through lessons. (The adder's rhetorical
   "I wonder what `carry_in` should be?" style of prompting is fine to keep as
   prose, but no formal exercises or solutions.)
7. **Advanced topics** are a separate section at the end of the guide, not
   sidebars within sequences.

## Lesson format

- Sequences live in their own directories: `guide/<seq>/index.rst` with a
  toctree of 2–3 short lessons (following `guide/adder/`).
- Each lesson introduces **one batch of concepts/syntax**, motivated by one
  step of the example circuit. If a lesson needs to explain "variations",
  split the lesson instead.
- Truth tables, waveforms, and complete code listings; code lives under
  `docs/source/examples/` and is `literalinclude`d (as today).
- Cross-check every lesson against `junk/lesson_order.md` (topological feature
  order) so no lesson uses a feature before it's taught.

## The ladder

### Part I — Foundations

1. **First circuits** — *Hello, hardware.*
   Lessons: passthrough; a wire that copies.
   Features: `mod`, ports, `Bit`, `:=`, docstrings, comments, build/simulate.
   Concepts: what a signal is; nothing changes over time yet.

2. **Logic gates** — *Decisions in hardware.*
   Lessons: NOT → AND → XOR (truth-table driven).
   Features: unary ops, logical ops (`&&`, `||`, `!`, `^^`).

3. **Words** — *More than one bit.*
   Lessons: nibble-swap or byte-splitter.
   Features: `Word[n]`, literals (binary, hex, width suffixes), indexing.

4. **Slicing & joining** — *Cutting and gluing bits.*
   Lessons: extract a byte from a word; pack fields into a layout.
   Features: slicing, concatenation (`word(...)`), comparison ops.

5. **Choosing** — *Deciding between values.*
   Lessons: 1-bit mux → word mux → a selector.
   Features: `when` expressions, `else` arms. (When-vs-match contrast begins.)

### Part II — Time and state (the conceptual leap)

6. **Clocks** — *Things that change.*
   Lessons: blinker. Kept tiny on purpose — registers are the hardest idea
   for a software person.
   Features: `Clock`, `reg`, `on clock`, `<=` vs `:=`.
   First VCD/gtkw screenshots here.

7. **Counting** — *Numbers that grow.*
   Lessons: counter with wraparound → LED chaser.
   Features: arith ops on regs, width overflow, `dyn` indexing
   (the chaser motivates it: `leds[dyn counter]`).

### Part III — Structure and reuse

8. **State machines** — *Remembering where you are.*
   Lessons: button debouncer → traffic light.
   Features: `enum`, `match` statements, brace-arm driver blocks,
   exhaustiveness.

9. **Submodules** — *Building blocks.*
   Lessons: half adder → full adder → 4-bit adder (existing adder sequence).
   Features: mod instances, `it` blocks.

10. **Generics** — *One module, many widths.*
    Lessons: parameterize the mux, then the adder.
    Features: generics, `for` clauses.

### Part IV — Rich data

11. **Structs** — *Grouping related signals.*
    Lessons: a register bank or bus-packet type.
    Features: `struct`, construction, field projection, type ascription.

12. **Optionals & variants** — *Maybe it isn't there.*
    Lessons: a FIFO with `Valid[T]`; a result-or-error type.
    Features: `Valid[T]`, `@Valid`/`@Invalid` patterns, match expressions,
    payload enumerants, `let`, `default`.

### Part V — Talking to the world

13. **System wiring** — *Modules talking to modules.*
    Lessons: UART echo.
    Features: sockets (`cosi`/`soci`, `:=:`), packages, imports, `pub`.

14. **Platforms** — *Targeting real hardware.*
    Lessons: add an FPGA target to the UART project.
    Features: platform defs, ext-mods, annotations, `initial`.

### Part VI — The reward

15. **Capstone: RISC-V core** — *Your first processor.*
    Lessons: fetch → decode (unions/structs shine here) → ALU → register
    file → run a program.
    Decomposition borrowed from `virdant-riscv` (alu, idecode, control, rf,
    mem) but rebuilt small for the guide. Assumes everything before.

### Part VII — Advanced topics

16. Short standalone lessons with honest "when would you need this" framing:
    - Tristates & buses (`drive-strong`/`drive-weak`, `net`/`bus`/`tap`,
      `net-attach`, `bidi-connect`)
    - One-hot encoding
    - `outgoing-reg` / `outgoing-wire`
    - `dontcare`, `unused`, `holes`
    - `paths`, `dependson`

## Example circuit inventory (to author)

| Sequence | Example(s) |
|---|---|
| 1 | passthrough |
| 2 | gates (NOT/AND/XOR) |
| 3 | nibble-swap / byte-splitter |
| 4 | byte extractor; field packer |
| 5 | mux1b → muxN → selector |
| 6 | blinker |
| 7 | counter; LED chaser |
| 8 | debouncer; traffic light |
| 9 | half/full/4-bit adder (exists) |
| 10 | generic mux; generic adder |
| 11 | register bank |
| 12 | FIFO (`Valid[T]`) |
| 13 | UART echo |
| 14 | UART + platform target |
| 15 | RISC-V (fetch/decode/ALU/RF/mem) |
| 16 | small per-topic snippets |

## Authoring workflow

- Examples under `docs/source/examples/<seq>/` with testbenches; simulate with
  the Virdant simulator, dump VCD, screenshot with gtkwave (`sim.gtkw` setups
  exist in `virdant-riscv/` as reference).
- Screenshots must be regenerated when examples change; note this in the docs
  Makefile or a `docs` README.
- Each sequence's `index.rst` gets a one-line theme blurb so the overall
  toctree reads as a ladder.

## Open questions / TODO

- [ ] Sequence 5 vs 8: `when` expression vs `match` — confirm the contrast is
      introduced in "Choosing" and deepened in "State machines".
- [ ] Where do `let` and `default` best land (12 vs 13)?
- [ ] Confirm RISC-V capstone scope: which RV32I subset? (No pipelines,
      no interrupts.)
- [ ] Decide whether `holes` / `dependson` / `paths` end up in Advanced Topics
      or are dropped entirely.
- [ ] Existing guide files (passthrough/buffer/mux/arbiter/adder/spi) — retire
      or fold into the new sequences? (buffer and arbiter themes are currently
      unassigned.)
