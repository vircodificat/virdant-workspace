# Examples: memories syntax explorations

A dozen-or-so sketches of what the `memory` sublanguage could look like.
These are **explorations, not proposals** — the goal is to survey the
design space and see which aesthetic fits Virdant best. Per PLAN.md, the
body is a set of **concurrent processes** (some clocked, some comb),
with per-port clocks, implicit registering, compile-time conflict
detection, and a declared collision menu.

All examples think of the storage as a plain **array** — `data[addr]` —
because that is the mental model the sublanguage should keep.

---

## 1. Baseline: Virdant components + array indexing

The closest to the CONCEPT sketch. Ports are components like in a
module; the body is a list of process blocks; reads are plain indexing.

```virdant
memory Sram<DEPTH: u32, DATA: type> {
    incoming clk    : Clock
    incoming we     : Bit
    incoming waddr  : Word[clog2(DEPTH)]
    incoming wdata  : DATA
    incoming raddr  : Word[clog2(DEPTH)]

    outgoing rdata  : DATA

    storage data[DEPTH] : DATA

    on clk {
        if {
            when we {
                data[waddr] <= wdata
            }
        }
    }

    comb {
        rdata := data[raddr]
    }
}
```

Notes: `<=` on a storage slot reads as "write on clock edge", identical
to a `reg` driver. `comb` is a clockless process. Registering of
`rdata` is implicit — read `rdata` inside `on clk` instead and it
becomes a registered output. Very Virdant, but `storage` reads/writes
are a new statement kind for the checker.

## 2. Verilog-flavored: always-style processes

Ports in a Verilog-like list; processes are named blocks.

```virdant
memory Sram<DEPTH: u32, DATA: type> (
    input  clk   : Clock,
    input  we    : Bit,
    input  waddr : Word[clog2(DEPTH)],
    input  wdata : DATA,
    input  raddr : Word[clog2(DEPTH)],
    output rdata : DATA,
) {
    storage data[DEPTH] : DATA

    always @(posedge clk) {
        if we:
            data[waddr] <= wdata
    }

    always {
        rdata = data[raddr]
    }
}
```

Notes: reads as C/Python for anyone; but `always @(posedge clk)`,
`input`/`output` and `=` vs `<=` drag in Verilog dialect without
Virdant's checker-friendly structure.

## 3. C-flavored: imperative functions

Each port is a function; the memory body reads like a C file.

```virdant
memory Sram<DEPTH: u32, DATA: type> {
    incoming clk  : Clock
    storage  data[DEPTH] : DATA

    writeport wp on clk(we : Bit, addr : Word[clog2(DEPTH)], wdata : DATA) {
        if we:
            data[addr] = wdata
    }

    readport rp comb(addr : Word[clog2(DEPTH)]) -> DATA {
        return data[addr]
    }
}
```

Notes: ports are first-class *things* with their own clock — exactly
the per-port clock model. `return` maps a comb read. Reads of storage
never appear on an LHS, so the checker sees "one address expression per
port" directly. Less Virdant in punctuation (`->`, parens), more C in
spirit.

## 4. Python-flavored: indented methods

```text
memory Sram<DEPTH: u32, DATA: type>:
    in clk : Clock
    storage data[DEPTH] : DATA

    def write(clk, we, addr, wdata):
        if we:
            data[addr] = wdata

    def read(addr) -> DATA:
        return data[addr]
```

Notes: maximally readable to the C/Python crowd; indented blocks break
Virdant's brace aesthetic and would need a separate parser mode.
Kept here as a contrast point.

## 5. Rust-flavored: storage as an object

Reads/writes are method calls on the storage — read-over-write variants
become method names.

```virdant
memory Sram<DEPTH: u32, DATA: type> {
    incoming clk : Clock
    storage data[DEPTH] : DATA

    process write on clk {
        data.write(we, waddr, wdata)   // single write port
    }

    process read comb {
        rdata := data.read(raddr)      // comb read
    }

    process read_sync on clk {
        rddata := data.read_registered(raddr)  // sync read
    }
}
```

Notes: `data.write(...)`, `data.read(...)`, `data.read_registered(...)`
— the collision menu hides in method names. Pleasant, but hides the
one-address-expression-per-port fact the checker needs, and `we`
inside the call is odd.

## 6. Port-as-object: read-over-write as method choice

Storage stays an array; *ports* are objects with behavioral methods.
Read-over-write behavior is expressed implicitly through which method
the code calls — the CONCEPT goal.

```virdant
memory DpRam<DEPTH: u32, DATA: type> {
    incoming clk : Clock
    storage data[DEPTH] : DATA

    port w0 : Write on clk {
        if we {
            data[waddr] <= wdata
        }
    }

    port r0 : Read comb {
        rdata := data.read_old(raddr)     // sees pre-write data on collision
    }

    port r1 : Read on clk {
        rdata1 := data.read_now(raddr)    // registered; sees post-write data
    }
}
```

Notes: `read_old` / `read_now` are the declared collision semantics —
the checker cross-references them against the mapped primitive's menu.
Explicit, but two spellings for "read an array element" may be too
clever.

## 7. Declarative port table + tiny body

Ports declare clock and collision behavior as properties; the body only
has to do the writes.

```virdant
memory DpRam<DEPTH: u32, DATA: type> {
    incoming clk  : Clock
    storage  data[DEPTH] : DATA

    port wp : write (we, waddr, wdata)   on clk
    port rp : read  (raddr) -> rdata     comb, old-data
    port rp2: read  (raddr2) -> rdata2   on clk, new-data, registered

    body {
        when we {
            data[waddr] <= wdata
        }
    }
}
```

Notes: the checker's favorite — the collision menu is declared
alongside the port, so proving "every conflict's outcome is declared"
is a table lookup. But ports as one-liners separate the port's *data
flow* from its behavior, which is what CONCEPT wants *implicit*.

## 8. Driver-statement form: arrays as driver targets

Storage slots are driven exactly like `reg`s, with Virdant's own driver
operators, including inside `when` blocks (the coding-conventions
"push in drivers" style).

```virdant
memory Sram<DEPTH: u32, DATA: type> {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[clog2(DEPTH)]
    incoming wdata : DATA
    incoming raddr : Word[clog2(DEPTH)]

    outgoing rdata : DATA

    storage data[DEPTH] : DATA

    on clk {
        data[waddr] <= wdata when we
    }

    comb {
        rdata := data[raddr]
    }
}
```

Notes: `data[waddr] <= wdata when we` is the whole write port —
suffix-`when` as a guard (not currently in the grammar, but very
Virdant in feel). The write is a *driver* into an array element; the
checker already has driver analysis to lean on.

## 9. Two-section body: section order is the behavior

One clocked section and one comb section; *ordering* of sections and
statements within them expresses read-over-write implicitly.

```virdant
memory Sram<DEPTH: u32, DATA: type> {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[clog2(DEPTH)]
    incoming wdata : DATA
    incoming raddr : Word[clog2(DEPTH)]

    outgoing rdata : DATA

    storage data[DEPTH] : DATA

    on clk {
        rdata <= data[raddr]      // read first: old data on collision
        data[waddr] <= wdata when we
    }
}
```

Notes: the sequential-program model from the interview (rejected as the
*only* model), but as a *style* within one process it still works and
is the most honest expression of "read-before-write". Sections may not
be separable though: `rdata <=` and `data[..] <=` interleave fine.

## 10. Transaction/block form: every port is a block

Each port is declared as a block with a header naming clock, direction
and collision behavior; the body inside is the port's behavior.

```virdant
memory DpRam<DEPTH: u32, DATA: type> {
    storage data[DEPTH] : DATA

    port wp on clk, write, old-data-ok {
        incoming we    : Bit
        incoming waddr : Word[clog2(DEPTH)]
        incoming wdata : DATA

        when we {
            data[waddr] <= wdata
        }
    }

    port rp comb, read, old-data {
        incoming raddr : Word[clog2(DEPTH)]
        outgoing rdata : DATA

        rdata := data[raddr]
    }
}
```

Notes: the most self-contained — each port is a mini-module with its
own ports, clock and declared behavior. instantiation wiring gets
verbose (`it.we`, `it.rdata` at every use site), but everything about a
port lives in one place.

## 11. Qualifier form: `old`/`new` on the read

One read syntax; read-during-write behavior is a qualifier on the
indexing expression.

```virdant
memory DpRam<DEPTH: u32, DATA: type> {
    incoming clk : Clock
    storage data[DEPTH] : DATA

    on clk {
        if we {
            data[waddr] <= wdata
        }
    }

    comb {
        rdata := data.old[raddr]    // read-first semantics on collision
    }

    on clk {
        rdata2 <= data.new[raddr]   // registered, write-first
    }
}
```

Notes: `data.old[a]` / `data.new[a]` keep one indexing syntax and make
the collision behavior visible *at the read site* — arguably the most
direct encoding of "implicitly through the behavior of the code". New
operator surface, though; and `new` only means something when a write
races it, so the checker must tie the qualifier to the mapped menu.

## 12. ROM / initialization

`storage` takes an initializer; comb read-only port. Initialization is
platform-checked (per PLAN.md decision 7); sim seeds unlisted entries
from `InitStrategy`.

```virdant
memory LookupRom<DEPTH: u32, DATA: type> {
    storage lut[DEPTH] : DATA = {
        0 => 0x11,
        1 => 0x22,
        2 => 0x44,
        else => 0x00,
    }

    readport r comb(addr : Word[clog2(DEPTH)]) -> DATA {
        return lut[addr]
    }
}
```

Notes: the `else` arm mirrors Virdant's `match` arms; a list form
(`= [0x11, 0x22, ...]`) is the Python-like alternative. On platforms
without bitstream memory init, this memory is rejected with a
"platform has no memory initialization" diagnostic.

## 13. Match-guarded region writes: what the checker accepts

Two write ports whose addresses are disjoint *by construction* — a
`match` splitting the address space. The light prover must accept this
(the interview's guard-mutual-exclusion case).

```virdant
memory RegionRam<DEPTH: u32, DATA: type> {
    incoming clk  : Clock
    incoming sel  : Bit
    incoming a0   : Word[clog2(DEPTH) - 1]
    incoming a1   : Word[clog2(DEPTH) - 1]
    storage  data[DEPTH] : DATA

    port w0 on clk { data[word(0, a0)] <= wdata0 }
    port w1 on clk { data[word(1, a1)] <= wdata1 }
}
```

Notes: both writes index the same storage, but the top bit is a
constant per port — pairwise address reasoning (constant folding, here)
proves disjointness, and the mapping proceeds. If the user instead
wrote `data[waddr0] <= ...` and `data[waddr1] <= ...` with two
unrelated inputs, that is rejected: cannot prove disjointness, and a
BRAM does not support two independent write ports anyway.

---

## Observations

- **Virdant-est**: 1, 8, 9 — components, `<=`/`:=` drivers, `when`
  guards, driver blocks. The body reuses the existing statement
  grammar wholesale, which the PLAN lists as a strong preference.
- **Most explicit about the collision menu**: 6, 7, 11. These make
  read-over-write behavior *visible* (method names, port properties,
  qualifiers) — which the compile-time conflict checker needs anyway.
  The implicit-sequence version (9) expresses it via statement order
  and needs the checker to infer the menu.
- **Ports-as-things (3, 6, 10)** fit the per-port-clock model best:
  each port carries its clock, and "one address expression per port"
  falls out of the syntax, which is exactly what the pairwise conflict
  prover consumes.
- **Reuse pressure**: examples 1/8/9 need almost no new expression or
  statement forms (only `storage`, `memory`, process blocks, and array
  LHS); examples 5/6/11 add method-call or qualifier semantics; 3/10
  add port-headers. The plan's Phase 0 cost grows in that same order.
