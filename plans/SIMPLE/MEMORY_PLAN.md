# MEMORY_PLAN.md — The `memory` construct

This document specifies the `memory` statement for Virdant: a module-local,
clocked, word-addressable memory (behavioral RAM/ROM) with dynamic indexing,
explicit write drivers, and file-based initialization.

Everything here was decided in design discussion. Nothing in this document is
left open.

---

## 1. Summary

A `memory` statement declares a **new component** in a module. The component:

- holds `depth` words of `Word[n]` (both must be literals; both must be powers of 2),
- is **duplex**: it is a *sink* (writable) and a *source* (readable),
- is written by **clocked** drivers (`<=`), committed on the clock edge,
- is read **combinationally** anywhere in the module by indexing it,
- lowers to behavioral Verilog (`reg` array + sequential always block),
  which synthesis tools infer as block RAM.

The introductory example, valid verbatim in a single-clock module:

```virdant
mod Ram {
    incoming clock : Clock

    incoming write_data : Word[32]
    incoming addr : Word[10]
    incoming write_enable : Bit
    outgoing read_data : Word[32]

    memory mem[1024] : Word[32] {
        if {
            when write_enable {
                it[dyn addr] <= write_data
            }
        }
        read_data := it[dyn addr]
    }
}
```

One new keyword is introduced: `memory`. It becomes reserved.

The depth is written in brackets **after the component name**, mirroring
how the memory is accessed (`mem[dyn addr]`), and the element type follows
the colon as for every other component. No separator keyword is needed; the
only new token is `memory`. (`x` was considered and rejected: it is a live
identifier in roughly a fifth of existing `.vir` files, the lexer has no
contextual keywords, and it would be the language's only single-letter
keyword.)

---

## 2. Syntax

### 2.1 Statement form

```
ModDefStmtMemory :=
    DocString? Annotations "memory" Ident "[" Expr "]" ":" Type OnClause? ItBlock?

OnRegion := "on" Expr ModDefStmtBlock     // inside a memory body only
```

- `Expr` in the brackets after the name is the **depth**; it must be a
  literal constant (see §4).
- `Type` must be `Word[n]` (checked; see §4).
- `OnClause?` is the reg-style clock clause (`memory mem[1024] : Word[32] on clock`).

Reading order: `memory mem[1024] : Word[32]` is "a memory `mem` with 1024
  slots, each a `Word[32]`". The brackets belong to the *component*, not to
  the type — exactly as `mem[dyn addr]` indexes the component to yield a
  `Word[32]`.
- `ItBlock?` is the memory body; it may be omitted (see §2.3).
- `Annotations` carries `@image`/`@image_hex` (see §6).

### 2.2 Memory body

The body is a driver block in which `it` refers to the memory component
(exactly like the body of a `reg`). It contains:

- **clocked write context**: either an `on <clock> { ... }` region, or — in a
  module with exactly one `Clock` — writes at the top level of the body,
  implicitly clocked by that clock (the same inference rule as `reg`).
- **combinational reads**: `:=` drivers and uses of `it[dyn idx]` /
  `it[c]` in expressions, at the top level of the body.

Inside an `on` region, **only writes to `it[...]` are allowed**. Reads and
drivers to other components belong at the top level of the body.

### 2.3 Blockless declaration

Like `reg`, a memory may be declared without a body:

```virdant
memory mem[1024] : Word[32]
```

The write site may then appear anywhere in the module (see §3.3).

---

## 3. Clocking and drivers

### 3.1 Clock resolution

The memory has **one clock slot**, resolved exactly like `reg`:

- If the module declares exactly one `Clock`-typed component, that clock is
  implicit; an explicit `on clock` clause is allowed but warns
  (`RedundantOnClause`).
- Otherwise the memory must declare `on <clock-expr>`; omitting it is a
  `MissingOnClause`-style error.

Writes to the memory — inside the body or elsewhere in the module — are
clocked by this clock.

### 3.2 Writes

A write driver has an **indexed left-hand side** — the first indexed LHS in
the language:

```virdant
it[dyn addr] <= write_data     // inside the memory body
mem[dyn addr] <= write_data    // anywhere in the module
```

- `<=` is required. `:=` also parses but warns (`MemoryAssignInsteadOfLassign`)
  and otherwise behaves as a write site, so fixing the warning never changes
  whether the program is legal.
- A write driver's RHS is checked as `Word[n]` (the element type).
- A write driver is a `Latched` driver: it commits at the clock edge.
- The write site may be guarded by `if`/`when`/`match` structure like any
  driver block.

### 3.3 Driver-site rule (inherited from regs)

Memories inherit the existing driver rules unchanged:

- The memory has **exactly one write site** in the module. A second write
  site (another top-level statement, another it-block) is the existing
  `MultipleDrivers` error. A body block containing `if`/`match` structure is
  one site, as for regs.
- A write site may appear inside the memory body or anywhere else in the
  module (as with `reg r : Word[8] on clock` + `r <= ...` at top level).

### 3.4 Conflicting writes: compile error

**Overlapping-guard writes are a compile error, decided syntactically.**
The compiler does *not* attempt to prove that two write arms cannot collide.

Two write clauses *conflict* when:

- they are siblings in the same driver site, and
- their guards are not syntactically exclusive (i.e. not related by
  `when`/`else` or disjoint `match` arms), and
- both are writes to the memory.

Exclusive arms are fine — only one arm runs per cycle, so exactly one write
happens:

```virdant
// LEGAL: mutually exclusive arms
when sel {
    it[dyn addr_a] <= data_a
}
else {
    it[dyn addr_b] <= data_b
}
```

Overlapping guards are an error even if the addresses would never coincide at
runtime:

```virdant
// ERROR: guards overlap; both may fire in the same cycle
when write_enable {
    it[dyn addr] <= write_data
}
when flush {
    it[dyn flush_addr] <= 0w32
}
```

Rationale: with dynamic addresses, collision cannot be proven or refuted in
general; per-address priority resolution would be a subtle, hard-to-eyeball
semantic; and a collision in a single-port memory is almost always a bug.
A user who wants intentional undefined behavior has `dontcare` (existing).

Future note: when multi-clock memories with several `on` regions (ports)
arrive, this rule applies **per region**. Collisions *between* ports are a
hardware property, not a coding mistake; their semantics are deferred (§10).

---

## 4. Types and addressing

### 4.1 Memories introduce no new type

The `memory` statement declares a component; its type is plainly `Word[32]`,
and the depth `[1024]` is a property of the component, declared only in that
statement. There are no memory-typed ports, wires,
regs, socket members, or `let` bindings.

- Reading: `mem[dyn addr]` is an expression of type `Word[32]`, usable
  anywhere an expression is (driver RHS, guards, submodule connections...).
- Writing: `mem[dyn addr] <= e` checks `e` as `Word[32]`.

### 4.2 Element type and depth constraints

- The element type must be `Word[n]` with **n a power of 2**.
- The depth must be a **power of 2**.
- Both must be **literals**. No generics in v1 (no `Nat` parameters in
  either position). Generalizing later is additive: it only widens what is
  accepted.

### 4.3 Index width

The index of a dynamic access must be **exactly** `Word[k]` where
`depth == 2^k`:

```virdant
memory mem[1024] : Word[32]       // k = 10
it[dyn addr]                      // addr : Word[10]  — required
it[dyn addr[9..0]]                // addr : Word[12]  — must be narrowed explicitly
```

This is the same rule dynamic indexing already enforces on `Word[n]`
(subject width == 2^index width). Rationale: extra address bits never cost
storage on FPGA or ASIC (the array has a fixed address port of log2(depth)
bits), so the only real effect of accepting wider indices is *silent
truncation*, which hides bugs from both simulation and synthesis. Loosening
this rule later is a non-breaking change; tightening it later is not.

### 4.4 Index forms

- **Dynamic**: `it[dyn addr]` / `mem[dyn addr]` — the `dyn` keyword is
  required, as for `Word[n]`.
- **Static**: `it[3]` / `mem[7]` — no `dyn`, a constant index. Legal for both
  reads and writes; lowers to a plain select / unconditional write.

### 4.5 Reads anywhere, by name

The memory is readable anywhere in the module via its component name, exactly
as a `reg` is readable outside its own block. Inside the body, `it` is an
alias for the component name. Multiple reads at different addresses in one
cycle are legal (multiple async read ports); synthesis may fall back from
BRAM to LUTRAM/registers — a quality-of-synthesis note, not a semantic issue.

### 4.6 `dependson` is unchanged

Reading memory state is like reading a `reg`: the only *combinational*
dependency a read introduces is on the index expression, so an `outgoing`
port combinationally reading a memory needs `dependson` naming the index's
incoming ports:

```virdant
outgoing read_data : Word[32] dependson addr {
    read_data := mem[dyn addr]
}
```

Entries are component paths; no new `dependson` syntax.

---

## 5. Simulation semantics

The memory is a per-word state array in the simulator, participating in the
existing clock-domain machinery (`clock_sensitivities`, alias closure):

- **Writes commit at the clock edge** of the memory's clock (Latched, like
  regs: pending update discipline, pre-edge sampling).
- **Reads sample current state** (combinational).
- **Read-during-write**: a read of an address written in the same cycle sees
  the **old** value (read-before-write) — exactly what nonblocking Verilog
  semantics produce.
- **Never-written words** read as unspecified (pseudorandom, like regs
  without `initial`), unless `@image` initialized them (§6).
- Reads of any index are always in range (depth is exactly 2^k and the index
  is exactly k bits), so there is no out-of-range case.

---

## 6. Initialization: `@image` / `@image_hex`

Memory contents are initialized by an **annotation** on the memory statement
(not an `initial` statement — `initial` on memories is not supported in v1):

```virdant
@image_hex("rom_image.hex")
memory rom[256] : Word[32] {
    it[0] <= 0    // illegal in v1; contents come from the image
}
```

- `@image("path")` — raw binary file; the file's bytes are the memory image,
  word-packed in declaration order.
- `@image_hex("path")` — text hex file, one word per line (à la `$readmemh`).
- Paths resolve **relative to the project root** — the directory where
  `virdant.toml` is found.
- A file **shorter** than the depth leaves the remaining words unspecified
  (sim: pseudorandom, like un-`initial`ed regs). A file **longer** than the
  depth is an **error**.
- On platforms annotated `@noinitial`, memory images are **disallowed** —
  same gating as reg `initial`.

---

## 7. Verilog lowering

Plain behavioral style; synthesis tools infer block RAM from it. No vendor
attributes in v1.

```virdant
// Virdant
memory mem[1024] : Word[32] {
    if {
        when write_enable {
            it[dyn addr] <= write_data
        }
    }
    read_data := it[dyn addr]
}
```

```verilog
// Generated Verilog
reg [31:0] mem [0:1023];

always @(posedge clock) begin
    if (write_enable)
        mem[addr] <= write_data;
end

assign read_data = mem[addr];
```

- Static-index writes lower to unconditional assignments
  (`mem[3] <= x;` inside the always block).
- Static-index reads lower to a plain select (`mem[3]`).
- `@image_hex` images emit a `$readmemh`-style initial block (or the
  platform-appropriate equivalent); `@image` likewise.
- The read side is an ordinary continuous assign of an indexed state signal.

---

## 8. Diagnostics

New diagnostics (names indicative; final spelling at implementation):

| Diagnostic | Kind | Cause |
|---|---|---|
| `MemoryElementNotPowerOf2Word` | error | element type is not `Word[n]`, or n is not a power of 2 |
| `MemoryDepthNotPowerOf2` | error | depth is not a power of 2 |
| `MemoryDepthNotLiteral` | error | depth is not a literal (generics are not supported) |
| `MemoryIndexWidth` | error | index is not exactly `Word[k]` for `depth == 2^k` |
| `MemoryWriteOutsideOnRegion` | error | write outside a clocked context when the module has zero or ≥2 Clocks |
| `OverlappingMemoryWrites` | error | two sibling write clauses with non-exclusive guards in one site |
| `MemoryAssignInsteadOfLassign` | warning | `:=` used to drive a memory word; must be `<=` |
| `RedundantOnClause` / `MissingOnClause` | warning / error | existing; apply to the memory's clock clause |
| `MultipleDrivers`, `NoRegDrivers`-analog | error | existing machinery, unchanged (one write site per memory) |

---

## 9. Implementation plan

1. **Grammar** (`virdant/src/syntax/virdant.lalrpop`): reserve `memory`;
   add `ModDefStmtMemory` (`"memory" Ident "[" Expr "]" ":" Type ...`) and
   `OnRegion`. Run `make grammar` to
   regenerate `GRAMMAR.txt`.
2. **Tokens/AST** (`token.rs`, `ast.rs`, `payload.rs`): new keyword tokens,
   `ModDefStmtMemory` payload, indexed driver LHS (`Path [ dyn Expr ]` /
   `Path [ Expr ]` as a driver target), `on` region payload.
3. **Components** (`common.rs`, `analysis/component.rs`): add
   `ComponentKind::Memory` with element type + depth; clock slot resolved by
   the existing implicit-clock logic; power-of-2 checks.
4. **Drivers** (`analysis/drivers.rs`, `queries/check_drivers.rs`): accept
   indexed LHS for `Memory` targets; site-count rule via existing
   `MultipleDrivers`; the syntactical overlapping-writes check (§3.4).
5. **Typing** (`types/typing/infer.rs`, `typing/check.rs`): `mem[dyn idx]` /
   `mem[c]` read expressions; indexed driver RHS checked as element type;
   index-width check (§4.3).
6. **Sim** (`sim/`): memory state array; per-clock pending-write commit;
   `@image`/`@image_hex` loading; `vir poke` visibility of memory words.
7. **Verilog** (`verilog/`): memory elements, sequential always block with
   indexed nonblocking writes, continuous read assigns, image init.
8. **Tools**: `vir format` rules for the new syntax; `vir doc`/`ls` surface
   memory components; `vir check` passes all of the above.
9. **Tests** (`tests/`): `check/`, `run/` (self-checking simulations with
   cycle limits), `errors/`, `warnings/` cases for every diagnostic above.
10. **Docs/skill**: update `.agents/skills/virdant/` (`modules.md`,
    `driver-blocks.md`, `idioms.md` reserved-keyword list, `diagnostics.md`),
    and the website docs. The reserved-keyword list gains only `memory`.

---

## 10. Non-goals and future work

Explicitly **not** in v1 (each is an additive future change):

- **Generics**: depth/width as `Nat` parameters.
- **Memory-typed anything**: ports, wires, regs, socket members, `let`s.
- **Multi-clock / multi-port memories**: multiple `on` regions as ports;
  cross-port collision semantics (expected direction: undefined but detected
  by the simulator — a hardware property, not a coding mistake).
- **Unclocked (combinational-write) memories**: spelling deliberately
  deferred (candidates: an `always { }` region or a `comb`/`@comb`
  modifier); top-level `<=` stays reserved for implicit-clock writes.
- **`initial` statements** on memories (per-word or bulk constant init),
  beyond `@image`/`@image_hex`.
- Vendor synthesis attributes for BRAM inference hints.

The v1 syntax is chosen so that every one of these can be added later
without breaking any program valid under this plan.

---

## 11. Grammar

The complete set of grammar additions and changes, in the notation of
`GRAMMAR.txt`. Everything not listed here is unchanged. For *why* the
grammar is shaped this way, see §12.

### 11.1 Tokens

One new keyword token: `"memory"`. (`dyn`, `on`, and `it` are already
reserved.)

### 11.2 Statements

```
ModDefStmtCore :=
      ModDefStmtComponent
    | ModDefStmtMemory              // new
    | ModDefStmtDriver
    | ModDefStmtIndexedDriver       // new
    | ModDefStmtInstance
    | ModDefStmtSocket
    | ModDefStmtUnused
    | ModDefStmtInitial

ModDefStmtMemory :=
      DocString? Annotations "memory" Ident "[" Expr "]" ":" Type OnClause? MemoryBlock?
```

- The bracketed `Expr` is the depth. The grammar accepts any expression;
  analysis requires a literal (§4.2).
- `OnClause` is the existing `"on" Expr` clock clause.
- `ModDefStmtMemory` produces a `Component` node with
  `kind = ComponentKind::Memory`. Its children are, in order: the
  `Annotations` wrapper, the depth `Expr`, the `Type`, then the optional
  `OnClause` and the optional body.

### 11.3 Indexed drivers

```
ModDefStmtIndexedDriver :=
      Path DriverIndexedIndex "<=" Expr
    | Path DriverIndexedIndex ":=" Expr

DriverIndexedIndex :=
      "[" "dyn" Expr "]"
    | "[" DriverIndexedIndexConst "]"

DriverIndexedIndexConst := "Nat"
```

- Produces an `IndexedDriver` node with three children: the LHS `Path`,
  the index, and the RHS `Expr`.
- For `[dyn e]` the index child is the `Expr`; for `[NAT]` it is an
  `IndexedDriverIndex { index }` node carrying the value in its payload.
- `:=` is accepted so that `MemoryAssignInsteadOfLassign` can be a warning
  rather than a syntax error.
- Legal wherever `ModDefStmtCore` is: module level, It-blocks, arm blocks,
  memory bodies and `on` regions. (Analysis restricts the target to a
  memory component.)

### 11.4 Memory body and `on` regions

```
MemoryBlock := "{" MemoryBlockStmt* "}"

MemoryBlockStmt :=
      ModDefStmtDriver
    | ModDefStmtIndexedDriver
    | ModDefStmtUnused
    | ModDefStmtInitial
    | If
    | Match
    | Let
    | ModDefStmtOnRegion

ModDefStmtOnRegion := "on" Expr ModDefStmtBlock
```

- `MemoryBlock` produces an ordinary `ModDefStmtBlock` node, so downstream
  code sees a memory body exactly as it sees an It-block.
- `MemoryBlockStmt` is `ModDefStmt` minus the declaration forms
  (`ModDefStmtComponent`, `ModDefStmtMemory`, `ModDefStmtInstance`,
  `ModDefStmtSocket`) plus `ModDefStmtOnRegion`. Like `ModDefStmtCore` it
  has an error-recovery alternative.
- `ModDefStmtOnRegion` produces an `OnRegion` node with two children: the
  clock `Expr` and the region's `ModDefStmtBlock`.
- An `on` region is reachable *only* through `MemoryBlock`. At module level,
  in an It-block, or in an arm block it is a syntax error. Inside a region,
  "writes to `it[...]` only" (§2.2) is enforced by analysis.
- `ItBlock`, `ModDefStmtBlock`, and `ModDefStmt` are unchanged.

### 11.5 Arm-block expression form

The single-expression arm body `{ <expr> }` is restricted in its leftmost
operand. This is the only change visible outside the memory feature.

```
ArmBlock :=
      "{" ModDefStmt* "}"
    | "{" ArmBlockExpr "}"

ArmBlockExpr :=
      ArmBlockExpr BinOpLogical ArmBlockCompare
    | ArmBlockCompare

ArmBlockCompare :=
      ArmBlockCompare BinOpCompareNoLe ExprBinOpAdditive
    | ArmBlockAdditive                                  // was: ExprBinOpAdditive

ArmBlockAdditive :=                                     // new
      ArmBlockAdditive BinOpAdditive ExprUnOp
    | ArmBlockUnOp

ArmBlockUnOp :=                                         // new
      ExprUnOpPrefix
    | ArmBlockAscription

ArmBlockAscription :=                                   // new
      ArmBlockPrimary ":" Type
    | ArmBlockPrimary

ArmBlockPrimary :=                                      // new
      ExprAtomPath
    | ExprPrimaryWith<ArmBlockAtom>

ArmBlockAtom :=                                         // new
      ExprAtomOther
    | ExprAtomPath "->" Ident
```

Only the leftmost operand of the arm expression descends through the
`ArmBlock*` chain; the right operand of every binary operator, the operand
of every prefix operator, and everything inside parentheses is an ordinary
`Expr`.

### 11.6 Expression refactors (no language change)

```
ExprUnOp :=
      ExprUnOpPrefix
    | ExprAscription

ExprUnOpPrefix :=
      UnOp ExprUnOp
    | "tap" Path

ExprPrimary := ExprPrimaryWith<ExprAtom>

ExprPrimaryWith<Atom> :=                               // LALRPOP macro
      Ofness "(" ArgList? ")"
    | ExprPrimaryWith<Atom> "->" Ident
    | ExprPrimaryWith<Atom> "[" "dyn" Expr "]"
    | ExprPrimaryWith<Atom> "[" Index "]"
    | ExprPrimaryWith<Atom> "[" Index ".." Index "]"
    | "@" Ident "(" ArgList? ")"
    | Atom

ExprAtom :=
      ExprAtomPath
    | ExprAtomOther

ExprAtomPath := Path

ExprAtomOther :=
      WordLit | BitLit | "Str" | "#" Ident | "@" Ident | "?" | "dontcare" | "(" Expr ")"
```

`ExprPrimaryWith<ExprAtom>` is exactly the old `ExprPrimary`; the macro
exists so `ArmBlockPrimary` can reuse the postfix chain with a different
root.

### 11.7 Reads

No change. `mem[dyn addr]` and `mem[3]` are already
`ExprPrimary "[" "dyn" Expr "]"` and `ExprPrimary "[" Index "]"` applied to
a `Path`; typing gives them the element type.

After editing `virdant.lalrpop`, run `make grammar` to regenerate
`GRAMMAR.txt`.

---

## 12. Why the grammar is amended this way

Two parts of §11 are not the obvious spelling. Both exist because the
obvious spelling does not build: LALRPOP rejects it as not LR(1) or as
ambiguous. This section explains each, the alternatives considered, and
what the chosen fix costs.

### 12.1 `Path [` after `{` in an arm

#### The problem

The natural indexed-driver rule is

```
ModDefStmtIndexedDriver := Path "[" "dyn" Expr "]" "<=" Expr
```

and on its own it is fine. It fails because of how arm bodies are parsed.
`if`/`match` are both statements and expressions, and their arms share one
rule:

```
ArmBlock :=
      "{" ModDefStmt* "}"      // statement arm
    | "{" ArmBlockExpr "}"     // single-expression arm
```

After `{`, the parser does not yet know whether a statement or an
expression follows. Both can begin with `Path`, and both can continue with
`[`:

```
{ mem[3] }              // expression: Path -> ExprAtom, then "[" Index "]"
{ mem[dyn i] <= v }     // statement:  Path "[" ... "]" "<=" Expr
```

With `Path` on the stack and `[` as the next token, an LR(1) parser must
choose between *reduce `Path -> ExprAtom`* (commit to "expression") and
*shift `[`* (keep `Path` as a driver LHS). What actually decides it is
further ahead: `dyn` vs `Nat` inside the brackets, and ultimately whether a
`<=`/`:=` follows the `]`. One token of lookahead cannot see that, so
LALRPOP reports a shift/reduce conflict.

This is the same family of problem that already exists for `<=`: `{ a <= b }`
could be a driver or a comparison. The grammar resolves that one with
`BinOpCompareNoLe`, which removes `<=` from the arm expression's top-level
comparison so that after `{ Path` a `<=` can only be a driver. Every other
driver operator (`:=`, `#=`, `#-`, `#=#`, `:=:`) never appears in an
expression, so no conflict arises for them. `[` is the first driver
continuation that is *also* a legitimate expression continuation.

#### Why the expression side gives way

The `"{" ArmBlockExpr "}"` form is not the real expression arm — that is
`=> Expr`. It exists only so that `when c { expr }` parses and can be
reported by `check.rs` as `ArmBlockNeedsArrow` instead of as a bare syntax
error. It is a diagnostic convenience, so narrowing it slightly is cheap,
whereas changing the driver syntax (`mem[dyn addr] <= x` is the language's
chosen spelling) or the real expression syntax is not.

#### The fix

The leftmost operand of an arm expression may be a bare `Path`, or a
postfix chain rooted at anything other than a bare `Path`; it may not be a
bare `Path` immediately followed by `[`. That is what the `ArmBlock*` chain
in §11.5 encodes. After `{ Path`, the possible next tokens are then:

| next token | expression reading | statement reading | outcome |
|---|---|---|---|
| `->` | field access | — | reduce |
| `:` | type ascription | — | reduce |
| operator, `}` | bare reference | — | reduce |
| `<=` | — (removed by `NoLe`) | latched driver | shift |
| `[` | — (removed by this fix) | indexed driver | shift |
| `:=` `#=` `#-` `#=#` `:=:` | — | driver | shift |

No token admits both readings, so the state is LR(1).

The restriction applies only to the leftmost operand. The right operand of
any binary operator, the operand of any prefix operator, and anything in
parentheses is reached after some other token has been shifted, so those
states do not contain the driver items and the ordinary `Expr` chain is
used there.

#### What it costs

In the diagnostic-only `{ expr }` arm form:

| arm body | before | after |
|---|---|---|
| `{ x }`, `{ x + 1 }`, `{ x->f }`, `{ x->f[3] }` | `ArmBlockNeedsArrow` | `ArmBlockNeedsArrow` |
| `{ -x[3] }`, `{ (x[3]) }`, `{ y + x[3] }` | `ArmBlockNeedsArrow` | `ArmBlockNeedsArrow` |
| `{ x[3] }`, `{ x[dyn i] }`, `{ x[7..0] }` | `ArmBlockNeedsArrow` | syntax error |

`=> x[3]` and every other real expression position are unaffected.

#### Alternative considered

Spell the LHS without the `Path` nonterminal (a private
`DriverIndexedLhsHead := Ident | "it" | DriverIndexedLhsHead "." Ident`),
so that the parser never has a `Path` on the stack with `[` coming next in
the shared state. This also builds and does not touch `ArmBlock`, at the
cost of a second, parallel spelling of paths in the grammar and an LHS node
assembled by hand in action code. It was rejected in favour of keeping
`Path` as the single spelling of paths and letting the diagnostic-only form
absorb the restriction.

### 12.2 The `on` region vs. the `on` clause

#### The problem

The first draft made `on <clock> { ... }` a statement in It-blocks
(`ModDefStmtBlock := "{" (ModDefStmt | ModDefStmtOnRegion)* "}"`). That is
not an LR(1) limitation but a genuine ambiguity, because declarations end
in `OnClause? ItBlock?`:

```
reg r : T
on clk { ... }
```

has two parses: a `reg` with no clause followed by an `on` region, or
`reg r : T on clk { ... }`. The same holds for `incoming`, `outgoing`,
`wire`, and `memory` itself.

#### The fix

Two constraints together remove every such adjacency:

1. `ModDefStmtOnRegion` is reachable only through `MemoryBlock`, the body
   of a `memory` statement. It is not a `ModDefStmt`, so it cannot appear
   at module level, in an It-block, or in an arm block.
2. `MemoryBlockStmt` omits the declaration forms. Nothing in a memory body
   ends in an optional `on` clause, so an `on` token there can only begin
   a region.

Constraint 2 loses nothing: components are collected from the module's
direct children only (`analysis/component.rs`), so a declaration inside any
body was never meaningful. `ItBlock` and `ModDefStmtBlock` stay exactly as
before; `MemoryBlock` produces the same `ModDefStmtBlock` node so
downstream code needs no new case.

#### Alternative considered

A distinct keyword for the region would avoid the ambiguity without
restricting the body's statement set, but §2.2 fixes the spelling as `on`,
and the restriction has no practical cost.

### 12.3 Checking for conflicts

`cargo c -p virdant` runs LALRPOP. Note that it reports problems under three
different headings — `Conflict detected`, `Local ambiguity detected`, and
`Ambiguous grammar detected` — and that once any conflict exists, the
lookahead sets it prints are over-approximated (every token appears), so
some of the listed conflicts are artifacts. Fix the one with a concrete
example first and re-run.
