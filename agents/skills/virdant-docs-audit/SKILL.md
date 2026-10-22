---
name: virdant-docs-audit
description: Audit the virdant/docs Sphinx documentation (the website) for accuracy against the Virdant compiler source code. Use when the user wants to check that the docs are up to date, find mistakes, or discover undocumented features.
cwd: /home/vircodificat/projects/virdant-workspace
---

# Virdant Docs Audit

When invoked, systematically verify that the Sphinx documentation in `virdant/docs/` (the website source) accurately reflects the Virdant compiler and language. The primary goal is to find **mistakes**—things that are wrong in the docs. Undocumented features should simply be listed at the end.

## Execution Strategy

**Use subagents to perform this audit.** The audit has multiple distinct areas that can be parallelized:

- Tutorial examples validation (can be distributed across tutorials)
- Reference documentation accuracy (grammar, types, modules, etc.)
- Example code compilation and correctness
- Grammar cross-reference against GRAMMAR.txt
- Final synthesis and undocumented features list

Consider using a workflow to coordinate multiple review subagents, each responsible for auditing a subset of the documentation, then aggregate their findings. The audit is exhaustive and systematic—delegation will make it faster and more thorough.

## Procedure

Follow these steps in order. For each step, report ALL findings — every discrepancy, every omission, every inaccuracy. Be exhaustive.

### Step 1: Tutorial examples validation

For each tutorial RST file in `virdant/docs/source/tutorial/`:

1. **Extract code examples** — find all `.. code-block:: virdant` or `.. literalinclude::` directives
2. **Verify referenced files** — if using `literalinclude`, check that the referenced file in `virdant/docs/source/examples/` exists
3. **Compile examples** — run `virdant/target/release/vir check` on each example file or inline code block (save to temp file first)
4. **Check explanations** — verify that the tutorial text accurately describes what the code does

**Report:**
- **BUG** — tutorial claims code does X but it actually does Y
- **BUG** — tutorial shows syntax that doesn't compile
- **BUG** — tutorial references a file that doesn't exist
- **STALE** — tutorial uses old syntax that no longer works
- **GAP** — tutorial explanation is incomplete or misleading

### Step 2: Grammar documentation accuracy

Compare `virdant/docs/source/reference/grammar.rst` against `virdant/GRAMMAR.txt`:

1. **Token definitions** — verify keywords, operators, literals match
2. **Syntax productions** — check that documented syntax rules match the grammar
3. **Examples in grammar doc** — ensure all example code compiles
4. **Missing productions** — find grammar rules in GRAMMAR.txt that aren't documented

**Report:**
- **BUG** — docs show syntax that doesn't match GRAMMAR.txt
- **BUG** — docs show invalid examples
- **STALE** — docs describe old syntax removed from GRAMMAR.txt
- **GAP** — grammar production exists but isn't documented

### Step 3: Reference documentation accuracy

For each reference RST file in `virdant/docs/source/reference/`:

Determine what the doc is about (types, modules, expressions, patterns, packages, sockets, items, platforms, etc.) and cross-check against the appropriate source:

- **Type docs** → compare against `virdant/virdant/src/types/typ.rs` (the `Type` enum)
- **Module/component docs** → compare against GRAMMAR.txt and `virdant/virdant/src/syntax/`
- **Expression docs** → verify operators, precedence, syntax against GRAMMAR.txt
- **Pattern docs** → verify pattern syntax against GRAMMAR.txt
- **Package docs** → verify import syntax, package structure
- **Socket docs** → verify socket definition and usage syntax
- **Item docs** → verify struct, union, enum, builtin, platform definitions
- **Platform docs** → verify platform definition syntax

For each reference doc:
1. **Syntax accuracy** — does documented syntax match GRAMMAR.txt and source?
2. **Examples compile** — do all code examples actually work?
3. **Completeness** — are there features in the source/grammar not covered?

**Report:**
- **BUG** — docs describe syntax/behavior that doesn't match source/grammar
- **BUG** — examples don't compile
- **STALE** — docs describe removed features
- **GAP** — documented description incomplete or misleading

### Step 4: Example file validation

For every `.vir` file in `virdant/docs/source/examples/`:

1. **Compile check** — run `virdant/target/release/vir check <file>`
2. **Referenced in docs** — search all .rst files to see if it's used anywhere
3. **Syntax validity** — does it use current, valid Virdant syntax?

**Report:**
- **BUG** — example file doesn't compile
- **ORPHAN** — example file exists but isn't used in any doc
- **STALE** — example uses old syntax

### Step 5: Cross-reference the standard library

Check `virdant/lib/` (the standard library) against tutorial and reference docs:

1. **Documented builtins** — are stdlib items mentioned in docs accurate?
2. **Import examples** — do docs correctly show how to import stdlib?
3. **Missing stdlib docs** — are there stdlib modules/items that should be documented?

**Report:**
- **BUG** — docs incorrectly describe stdlib behavior
- **GAP** — important stdlib features not documented

### Step 6: Getting started and principles

Check the top-level RST files in `virdant/docs/source/`:

1. **Installation instructions** — do they match current build/install process?
2. **Example commands** — do they work with current compiler?
3. **Philosophy/principle claims** — are they still accurate?
4. **Coding conventions** — are they consistent with compiler behavior?

**Report:**
- **BUG** — instructions don't work or contradict compiler behavior
- **STALE** — outdated information

### Step 7: Undocumented features

Scan the source for features that should be in the website docs but aren't:

1. **GRAMMAR.txt** — are there productions not covered in reference docs?
2. **Language features** — check `virdant/virdant/src/syntax/` for AST nodes/features not documented
3. **Builtins** — check `virdant/lib/` for stdlib items not mentioned
4. **Compiler features** — check `virdant/virdant/src/bin/vir.rs` for CLI flags/modes not documented

Compile a **plain list** of undocumented features. Do NOT embed these in the earlier "bug" sections. Present them as a simple bullet list under the heading "## Undocumented Features".

## Conventions

- Mark each finding as one of:
  - **BUG** — the doc says something that contradicts the source/grammar/compiler (wrong syntax, wrong behavior, wrong example)
  - **GAP** — the source has something that the doc omits (missing explanation, incomplete coverage)
  - **ORPHAN** — a doc/example file that isn't used or referenced anywhere
  - **STALE** — a doc that describes old behavior that no longer works (removed syntax, changed semantics)
- After all bugs, add a `---` separator, then the undocumented features list.

## Example output format

```
### tutorial/buffer.rst

BUG — Tutorial shows `reg x : Bit on clk` but correct syntax is `reg x : Bit on clk {`.

BUG — Example references `examples/buffer_old.vir` which doesn't exist.

---

## reference/grammar.rst

BUG — Docs claim `Word[N]` accepts any N but compiler requires N >= 1.

GAP — `union type` syntax exists in GRAMMAR.txt but isn't documented here.

---

## Undocumented Features

- `dependson` statement exists in GRAMMAR.txt but isn't in any reference doc
- `@attribute` annotations are in grammar but not documented
- `vir doc` command exists but isn't in getting-started.rst
```

## Source layout reference

Key locations for the audit:

| What | Path |
|------|------|
| Website docs (RST) | `virdant/docs/source/` |
| Tutorial docs | `virdant/docs/source/tutorial/` |
| Reference docs | `virdant/docs/source/reference/` |
| Example files | `virdant/docs/source/examples/` |
| Grammar reference | `virdant/GRAMMAR.txt` |
| Compiler source | `virdant/virdant/src/` |
| Standard library | `virdant/lib/` |
| CLI tool | `virdant/virdant/src/bin/vir.rs` |
| Type system | `virdant/virdant/src/types/` |
| Syntax/AST | `virdant/virdant/src/syntax/` |

## How to compile examples

Build the compiler first if needed:
```bash
cd virdant && cargo build --release --features "vir-bin"
```

Then check a Virdant file:
```bash
virdant/target/release/vir check <file>.vir
```

Or check inline code by saving to a temp file:
```bash
echo 'mod Top { incoming x : Bit }' > /tmp/test.vir
virdant/target/release/vir check /tmp/test.vir
```
