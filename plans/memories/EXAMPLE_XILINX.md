# Examples: every Xilinx 7-series memory primitive configuration, in style #1

Companion to `EXAMPLE_ICE40.md` and target shapes for PLAN.md Phase 3.
`lib/xilinx.vir` deliberately does *not* declare the 7-series RAM
primitives (yosys infers them from ordinary memories), so this file
enumerates the configurations by hand. Everything is **style #1**.

The 7-series memory primitives:

| Primitive   | Capacity | Shapes                | Ports                                    |
|-------------|----------|-----------------------|------------------------------------------|
| `RAMB36E1`  | 36 Kbit  | 36×1024 … 1×32768     | 2 R/W ports, byte enables, out-regs      |
| `RAMB18E1`  | 18 Kbit  | 18×1024 … 1×16384     | 2 R/W ports, byte enables, out-regs      |
| LUT RAM     | ≤ 64 bit/word, ≤ 256 deep | 1×256 … 32×8 | async (comb) reads, cheap ports  |
| `SRL16E`/`SRLC32E` | shift RAM | —                  | sequential access (adjacent, see §12)    |
| `FIFO36E1`/`FIFO18E1` | 36/18 Kbit | fixed 4/9-bit | built-in counters (adjacent, see §13)    |

Key structural differences from iCE40: **both BRAM ports are
read/write** (true dual port), **widths include the parity bits** (36 =
32 data + 4 parity; 9 = 8 + 1), **byte-write enables are per 9-bit
lane**, the **output register is optional** (`DOA_REG`), and there **is
distributed RAM**, so `comb` reads have a real primitive to map to.

---

## 1. Single-port, registered read: 1024×36

`RAMB36E1`, port A used as R/W, `DOA_REG=0` — one cycle from address
sample to `DO` valid.

```virdant
memory Bram36 {
    incoming clk   : Clock
    incoming we    : Bit
    incoming addr  : Word[10]
    incoming wdata : Word[36]

    outgoing rdata : Word[36]

    storage data[1024] : Word[36]

    on clk {
        data[addr] <= wdata when we
    }
    on clk {
        rdata <= data[addr]
    }
}
```

Notes: single port ⇒ write and read must share the address expression
(pairwise checker: trivially one address per cycle). `Word[36]` uses
the parity bits — widths 36/18/9 map onto the primitive's native
aspect ratios without waste.

## 2. Simple dual port, max width: 512×72

`RAMB36E1` ×2 composed for width (SDP shape: one write port, one read
port).

```virdant
memory Sdp72 {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[9]
    incoming wdata : Word[72]
    incoming raddr : Word[9]

    outgoing rdata : Word[72]

    storage data[512] : Word[72]

    on clk {
        data[waddr] <= wdata when we
    }
    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: 72 = 2 × 36 ⇒ two `RAMB36E1`s side by side, common addresses.
The write port and read port have *distinct* address expressions — the
SDP idiom. `Word[72]` is 8 native 9-bit lanes; all byte-write-enable
and parity plumbing is mapping detail, invisible to the user.

## 3. True dual port: two read/write ports

`RAMB36E1`, ports A and B both R/W.

```virdant
memory Tdp36 {
    incoming clk : Clock
    incoming we0 : Bit
    incoming addr0 : Word[10]
    incoming wdata0 : Word[36]
    incoming we1 : Bit
    incoming addr1 : Word[10]
    incoming wdata1 : Word[36]

    outgoing rdata0 : Word[36]
    outgoing rdata1 : Word[36]

    storage data[1024] : Word[36]

    on clk {
        data[addr0] <= wdata0 when we0
    }
    on clk {
        data[addr1] <= wdata1 when we1
    }
    on clk {
        rdata0 <= data[addr0]
    }
    on clk {
        rdata1 <= data[addr1]
    }
}
```

Notes: **this is where the conflict checker earns its keep.** Two
writes (`addr0`/`addr1` are unrelated inputs) cannot be proven
disjoint ⇒ rejected — a BRAM's two ports writing the same address in
one cycle has primitive-defined (not user-defined) behavior. The
declaration above is only acceptable if a later `when`-guard or the
escape hatch (PLAN.md decision 4) declares the collision outcome. The
read/write races (port 0 writes while port 1 reads) *are* mappable: the
primitive's `WRITE_FIRST`/`READ_FIRST` modes are the collision menu —
see §6.

## 4. Byte-write enables: per-9-bit-lane writes

`WEA[3:0]` — four lanes of 9 bits each.

```virdant
memory Bram36Lanes {
    incoming clk   : Clock
    incoming we    : Word[4]
    incoming addr  : Word[10]
    incoming wdata : Word[36]
    incoming raddr : Word[10]

    outgoing rdata : Word[36]

    storage data[1024] : Word[36]

    on clk {
        data[addr][35..27] <= wdata[35..27] when we[3]
        data[addr][26..18] <= wdata[26..18] when we[2]
        data[addr][17..9]  <= wdata[17..9]  when we[1]
        data[addr][8..0]   <= wdata[8..0]   when we[0]
    }
    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: disjoint-lane writes to one address (iCE40 §5's pattern) ⇒ the
lane guards are mutually exclusive by bit-slice disjointness ⇒ `WEA`
per lane. Reads are unaffected by `WEA` ⇒ a single unguarded read port.

## 5. `comb` reads: distributed RAM

LUT RAM is the *only* primitive with asynchronous reads — comb read
processes map here, and their smallness is enforced by capacity
(≤ 64-bit words, ≤ 256 deep per LUT-RAM cascade).

```virdant
memory Regfile {
    incoming we    : Bit
    incoming waddr : Word[5]
    incoming wdata : Word[32]
    incoming raddr : Word[5]

    outgoing rdata : Word[32]

    storage regs[32] : Word[32]

    comb {
        rdata := regs[raddr]
    }
    on clk {
        regs[waddr] <= wdata when we
    }
}
```

Notes: 32×32 = `RAM32M`-class LUT RAM (a write port, one async read
port). A 32-entry register file with comb read — the classic thing
LUT RAM exists for. Crucially this is **not** "a bunch of flops": the
mapping produces LUT-RAM primitives, honoring CONCEPT's never-flops
rule. A comb read *larger* than LUT-RAM capacity is rejected for the
platform (BRAM cannot serve it; flops are forbidden).

## 6. Read-during-write modes: `WRITE_FIRST` / `READ_FIRST` / `NO_CHANGE`

Per-port, same-clock collision outcomes. Style #1 expresses them
through the read site — statement order, guards, or (provisional)
qualifiers.

```virdant
memory TdpModes {
    incoming clk : Clock
    incoming we  : Bit
    incoming addr : Word[10]
    incoming wdata : Word[36]

    outgoing rdata_wf : Word[36]   // write-first: new data
    outgoing rdata_rf : Word[36]   // read-first:  old data
    outgoing rdata_nc : Word[36]   // no-change:   hold

    storage data[1024] : Word[36]

    on clk {
        data[addr] <= wdata when we
    }
    on clk {
        rdata_wf <= data.new[addr]   // provisional: post-write value
    }
    on clk {
        rdata_rf <= data.old[addr]   // provisional: pre-write value
    }
    on clk {
        when !we {
            rdata_nc <= data[addr]
        }
        // else hold: no assignment
    }
}
```

Notes: `data.new[a]`/`data.old[a]` are EXAMPLES.md sketch 11's
qualifiers — here they carry exactly the primitive's `WRITE_FIRST` /
`READ_FIRST` menu, and the checker ties each qualifier to what the
mapped primitive supports. The `NO_CHANGE` arm needs no new syntax at
all: "read only when not writing, else hold" *is* no-change.

## 7. Registered output register: `DOA_REG=1`

Two-cycle latency: address sampled, then re-sampled through the output
register. In style #1 that is simply "read, then register the read":

```virdant
memory Bram36Pipelined {
    incoming clk   : Clock
    incoming we    : Bit
    incoming addr  : Word[10]
    incoming wdata : Word[36]

    outgoing rdata : Word[36]

    storage data[1024] : Word[36]
    reg rdata_pipe : Word[36] on clk

    on clk {
        data[addr] <= wdata when we
    }
    on clk {
        rdata_pipe <= data[addr]
    }
    on clk {
        rdata <= rdata_pipe
    }
}
```

Notes: a `reg` chained after the storage read ⇒ the mapping sets
`DOA_REG=1` and folds the user's `reg` into the primitive's output
register (free pipeline stage). If the mapping proves the fold (the
`reg` is driven only by the storage read), the `reg` component
disappears from the netlist. Recognition failure ⇒ honest fallback:
BRAM with `DOA_REG=0` plus a fabric `reg` — still never flops for the
storage.

## 8. Mixed-width true dual port: A=36×1024, B=18×2048

Per-port aspect ratios are independent in TDP mode.

```virdant
memory TdpMixedWidth {
    incoming clk : Clock
    incoming we0 : Bit
    incoming addr0 : Word[10]
    incoming wdata0 : Word[36]
    incoming we1 : Bit
    incoming addr1 : Word[11]
    incoming wdata1 : Word[18]

    outgoing rdata0 : Word[36]
    outgoing rdata1 : Word[18]

    storage data[1024] : Word[36]

    on clk {
        data[addr0] <= wdata0 when we0
    }
    on clk {
        data[addr1] <= wdata1 when we1
    }
    on clk {
        rdata0 <= data[addr0]
    }
    on clk {
        rdata1 <= data[addr1][17..0]
    }
}
```

Notes: port B addresses twice the entries with half-width words —
`Word[11]` address, `Word[18]` data, reading `data[addr1]` as one
18-bit half of the 36-bit entry (the `[17..0]` slice pins which half).
The mapping folds "narrow port over a wide array" into the primitive's
per-port width. As with iCE40 §4, an idiom the recognizer cannot prove
falls out to rejection.

## 9. Narrow single-port: 16384×1 / 8192×2 (`RAMB18E1` class)

Tiny words, deep arrays.

```virdant
memory FlagRam<DEPTH: u32> {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[clog2(DEPTH)]
    incoming wdata : Bit
    incoming raddr : Word[clog2(DEPTH)]

    outgoing rdata : Bit

    storage data[DEPTH] : Bit

    on clk {
        data[waddr] <= wdata when we
    }
    on clk {
        rdata <= data[raddr]
    }
}

// mod flags of FlagRam[8192]  ⇒ RAMB18E1, 1×16384-class aspect ratio
// mod pairs of FlagRam[2048]  ⇒ RAMB36E1, 2×32768-class
```

Notes: `Bit` storage ⇒ width-1 aspect ratios (1×16384 / 1×32768).
Same generic shape as iCE40 §3 — the (depth, width) table is what the
mapping matches against each platform's aspect-ratio list.

## 10. Dual-clock BRAM

Two clocks, one per port — same shape as iCE40 §2.

```virdant
memory AsyncTdp {
    incoming clk_w : Clock
    incoming clk_r : Clock
    incoming we    : Bit
    incoming waddr : Word[10]
    incoming wdata : Word[36]
    incoming raddr : Word[10]

    outgoing rdata : Word[36]

    storage data[1024] : Word[36]

    on clk_w {
        data[waddr] <= wdata when we
    }
    on clk_r {
        rdata <= data[raddr]
    }
}
```

Notes: cross-clock collision outcome on 7-series BRAM is
"undefined/old-data depending on clocking" — the declared menu here
must be the conservative one, and the checker rejects declarations
claiming write-first behavior across clock domains.

## 11. Distributed RAM with more read ports (composition note)

LUT RAM is cheap to compose: a second comb read port is another LUT
cascade, not port replication of a scarce primitive.

```virdant
memory DualReadRegfile {
    incoming we     : Bit
    incoming waddr  : Word[5]
    incoming wdata  : Word[16]
    incoming raddr0 : Word[5]
    incoming raddr1 : Word[5]

    outgoing rdata0 : Word[16]
    outgoing rdata1 : Word[16]

    storage regs[32] : Word[16]

    comb {
        rdata0 := regs[raddr0]
        rdata1 := regs[raddr1]
    }
    on clk {
        regs[waddr] <= wdata when we
    }
}
```

Notes: EXAMPLES.md sketch 13 (the 4+-read-port case) said wider port
counts are *rejected* — that rule was about BRAM replication. LUT RAM
composition is the sanctioned exception candidates may argue for
later: two async read ports = two LUT-RAM read cascades. Left as a
noted extension; V1 mapping can reject it and stay consistent with
"no replication inference".

## 12. Adjacent primitive: shift RAM (`SRL16E`/`SRLC32E`)

Not a `memory` mapping target in V1 — a memory whose accesses are
*sequential* (one rolling address) matches the SRL shape.

```virdant
memory DelayLine {
    incoming clk  : Clock
    incoming din  : Bit

    outgoing dout : Bit

    storage line[32] : Bit
    reg head : Word[5] on clk

    on clk {
        line[head] <= din
        head <= head + 1
    }
    comb {
        dout := line[head - 1]
    }
}
```

Notes: every access is `head`-derived and `head` counts through the
whole space ⇒ the SRL idiom. Recognizing this needs an analysis the
plan does not fund in V1; shown to bound future scope. Until then this
maps to LUT RAM (legal: comb read, 32×1) — a fine fallback.

## 13. Adjacent primitive: built-in FIFOs (`FIFO36E1`/`FIFO18E1`)

A dual-clock FIFO is two pointers over a memory — the *counters and
status flags* are what the primitive hardens, not the array. Virdant's
`memory` covers the array; a `fifo`-shaped socket/component on top
(pointer regs, gray-coded sync, flags) is a library concern, not a
mapping target. Sketched for completeness:

```virdant
memory FifoCore {
    incoming clk_w : Clock
    incoming clk_r : Clock
    incoming we    : Bit
    incoming re    : Bit
    incoming wdata : Word[4]

    outgoing rdata : Word[4]

    storage ring[512] : Word[4]
    reg wptr : Word[9] on clk_w
    reg rptr : Word[9] on clk_r

    on clk_w {
        ring[wptr] <= wdata when we
        wptr <= wptr + 1 when we
    }
    on clk_r {
        rdata <= ring[rptr]
        rptr <= rptr + 1 when re
    }
}
```

Notes: maps to a dual-clock BRAM (§10) — not to `FIFO36E1`, whose
counters Virdant would have to pattern-match pointer arithmetic to
claim. Fine: the array is the memory; the FIFO is the module around
it.

## 14. Out of family: UltraScale+ `URAM288` (note)

`URAM288` is 288 Kbit, 72-bit true dual port, independent port clocks,
built-in cascade. It takes *no new declaration shapes* — §2 and §3 at
a larger size class, on a platform whose mapping table includes it:

```virdant
// mod bigbuf of Sdp72[16384]  ⇒ URAM288-class on UltraScale+ platforms
//                              ⇒ rejected (capacity) on 7-series
```

Notes: the same declaration is valid on both platforms; only the
mapping table differs. This is exactly the "rejected for that
platform" story of PLAN.md decision 5.

---

## Mapping summary

| Declaration (style #1)                | Primitive + config                            |
|---------------------------------------|-----------------------------------------------|
| 1 — 1024×36 SP, registered read       | `RAMB36E1`, port A R/W, `DOA_REG=0`           |
| 2 — 512×72 SDP                        | 2 × `RAMB36E1` composed, SDP                  |
| 3 — TDP, two R/W ports                | `RAMB36E1` TDP (writes need disjointness)     |
| 4 — 9-bit-lane slice writes           | `RAMB36E1`, `WEA[3:0]`                        |
| 5 — `comb` read, ≤ 32×32              | `RAM32M`-class distributed RAM                |
| 6 — `data.new` / `data.old` / hold    | `WRITE_FIRST` / `READ_FIRST` / `NO_CHANGE`    |
| 7 — `reg` chained on storage read     | `DOA_REG=1`, user `reg` folded                |
| 8 — mixed widths per port             | `RAMB36E1` TDP, 36×1024 + 18×2048             |
| 9 — `Bit` storage, deep               | `RAMB18E1`/`RAMB36E1` width-1 aspect ratios   |
| 10 — dual-clock                       | `RAMB36E1`, independent `CLKA`/`CLKB`         |
| 11 — 2 comb read ports                | LUT-RAM composition (noted; V1 may reject)    |
| 12 — sequential-address memory        | `SRLC32E`-class (deferred; LUT-RAM fallback)  |
| 13 — pointer ring over storage        | dual-clock BRAM; `FIFO36E1` is library-level  |
| 14 — 72-bit, 16 K-deep                | `URAM288`-class (UltraScale+ only)            |
| comb read too big for LUT RAM         | **rejected** (BRAM has no async read; flops forbidden) |
