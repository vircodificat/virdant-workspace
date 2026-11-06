# Examples: every iCE40 memory primitive configuration, in style #1

Target shapes for PLAN.md Phase 3 (synthesis mapping). Every example is
**style #1** (Virdant-native: components, `<=`/`:=` drivers, `on clk`
and `comb` processes, `storage` arrays). Each section names the
primitive and configuration the declaration must infer to.

The iCE40 has exactly two memory primitives (see `lib/ice40.vir`):

| Primitive        | Capacity | Shape            | Ports                          |
|------------------|----------|------------------|--------------------------------|
| `SB_RAM40_4K`    | 4 Kbit   | 16×256 … 2×2048  | 1 write + 1 read, separate clocks |
| `SB_SPRAM256KA`  | 256 Kbit | 16×16384         | 1 combined R/W, shared clock   |

`SB_RAM40_4K` comes in 4 clock-polarity variants (`-NR`, `-NW`,
`-NRNW`). There is **no distributed (LUT) RAM** on iCE40.

A note up front: the 4K block RAM has **no combinational read** — its
read port is always clocked (`RCLK`/`RE`). Any `comb` read process in
this file would have to map to a mux network of LUTs (not flops, but
also not a memory primitive); the mapping may reject it or cap its
size. All examples below therefore use clocked reads.

---

## 1. The hello-world: 256×16 dual-port, shared clock

`SB_RAM40_4K`, `WRITE_MODE=0` (16×256), `READ_MODE=0` (16×256),
`RCLK = WCLK`.

```virdant
memory Sram {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[8]
    incoming wdata : Word[16]
    incoming raddr : Word[8]

    outgoing rdata : Word[16]

    storage data[256] : Word[16]

    on clk {
        data[waddr] <= wdata when we
    }

    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: the read process is a registered output (assigned inside `on
clk`) — this is exactly the BRAM's native timing: address sampled on
the edge, `RDATA` valid one cycle later. `RE` is inferred from "the
read always runs" (`RE=1`); if the user wants data held when idle, see
example 6.

## 2. True dual-clock: `RCLK ≠ WCLK`

Same declaration, two clock components — the per-port-clock model pays
off directly.

```virdant
memory AsyncSram {
    incoming clk_w : Clock
    incoming clk_r : Clock
    incoming we    : Bit
    incoming waddr : Word[8]
    incoming wdata : Word[16]
    incoming raddr : Word[8]

    outgoing rdata : Word[16]

    storage data[256] : Word[16]

    on clk_w {
        data[waddr] <= wdata when we
    }

    on clk_r {
        rdata <= data[raddr]
    }
}
```

Notes: cross-clock port pair ⇒ the checker treats the read/write race
as necessarily asynchronous; the 4K BRAM's dual-clock read-during-write
behavior is "undefined", so the declared collision menu here must be
"undefined-on-collision" — and the checker's pairwise analysis flags
any declaration that claims old-data or new-data across clocks.

## 3. Narrower words: 512×8, 1024×4, 2048×2

`WRITE_MODE=1/2/3`. The generic form once, then instantiations.

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
    on clk {
        rdata <= data[raddr]
    }
}

// elsewhere, in a mod:
//   mod bytes  of Sram[512,  Word[8]]
//   mod nibble of Sram[1024, Word[4]]
//   mod pair   of Sram[2048, Word[2]]
```

Notes: `Word[8]` ⇒ `WRITE_MODE=1` (8×512), `Word[4]` ⇒ `2`,
`Word[2]` ⇒ `3`. The mapping keys off (depth, width): 16×256 / 8×512 /
4×1024 / 2×2048 are exactly the 4K BRAM's four aspect ratios, and the
power-of-2 rule (CONCEPT) is what keeps this table closed.

## 4. Mixed write/read widths: write 8, read 16

`WRITE_MODE=1` (8×512) with `READ_MODE=0` (16×256) — the modes are
independent per primitive.

```virdant
memory MixedWidth {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[9]
    incoming wdata : Word[8]
    incoming raddr : Word[8]

    outgoing rdata : Word[16]

    storage data[512] : Word[8]

    on clk {
        data[waddr] <= wdata when we
    }
    on clk {
        rdata <= word(data[word(0, raddr)], data[word(1, raddr)])
    }
}
```

Notes: the read port reads two adjacent 8-bit entries per cycle and
concatenates — which *is* the hardware's `READ_MODE=0` view of the same
array. The mapping must recognize this concat-of-two-addresses idiom
(both reads share all address bits but the top one) and fold it into a
single `READ_MODE=0` port. If the idiom recognizer can't prove it,
this falls out to rejection — honest, if annoying.

## 5. Per-bit write masking: `MASK`

`SB_RAM40_4K` masks writes per bit. Slice-conditional writes express
it.

```virdant
memory MaskedSram {
    incoming clk    : Clock
    incoming we_hi  : Bit
    incoming we_lo  : Bit
    incoming waddr  : Word[8]
    incoming wdata  : Word[16]
    incoming raddr  : Word[8]

    outgoing rdata : Word[16]

    storage data[256] : Word[16]

    on clk {
        data[waddr][15..8] <= wdata[15..8] when we_hi
        data[waddr][7..0]  <= wdata[7..0]  when we_lo
    }
    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: two writes to the same address, disjoint bit lanes — the
conflict checker's "mutual exclusion of guards" case, extended to bit
lanes: disjoint slices of the same slot never conflict, so this maps
to `MASK[15:8]=~we_hi`, `MASK[7:0]=~we_lo`. (The primitive's MASK
convention: set bits are *masked out*.)

## 6. Clock enables and held reads: `RCLKE` / `RE`, `NO_CHANGE`-style

The read holds its last value when idle — this is the only way the 4K
BRAM's output behaves "comb-ish" between edges.

```virdant
memory HeldSram {
    incoming clk   : Clock
    incoming ce    : Bit
    incoming we    : Bit
    incoming waddr : Word[8]
    incoming wdata : Word[16]
    incoming raddr : Word[8]

    outgoing rdata : Word[16]

    storage data[256] : Word[16]

    on clk {
        when ce {
            data[waddr] <= wdata when we
        }
    }
    on clk {
        when ce && !we {
            rdata <= data[raddr]
        }
        // else: rdata holds (no assignment ⇒ no driver change)
    }
}
```

Notes: `ce` ⇒ `WCLKE`/`RCLKE`; the guarded read ⇒ `RE` low during
writes, i.e. the read port holds through a write (`RE=WE?` — here
`RE = ce && !we`). The "no assignment ⇒ hold" rule is Virdant's
existing register behavior; the checker sees two guarded accesses and
proves the guards don't fight.

## 7. Falling-edge clocks: `SB_RAM40_4KNR` / `-NW` / `-NRNW`

Same declarations as 1/2, with a port clocked on the falling edge.

```virdant
memory NegEdgeRead {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[8]
    incoming wdata : Word[16]
    incoming raddr : Word[8]

    outgoing rdata : Word[16]

    storage data[256] : Word[16]

    on clk {
        data[waddr] <= wdata when we
    }
    on negedge clk {
        rdata <= data[raddr]
    }
}
```

Notes: `on negedge clk` is **provisional syntax** (the `on` clause
takes an expression today; a polarity qualifier is new grammar). It
maps to `SB_RAM40_4KNR`; a falling-edge *write* port maps to `-NW`,
both to `-NRNW`. Alternative without new grammar: pass a second
`Clock` input that the user drives inverted — uglier, but zero grammar
cost. The mapping only needs "read clock polarity" as a knob either
way.

## 8. The SPRAM: 16384×16 single-port (`SB_SPRAM256KA`)

One combined read/write port, nibble-wide write enables, chipselect.

```virdant
memory Spram {
    incoming clk    : Clock
    incoming cs     : Bit
    incoming we     : Bit
    incoming we_nib : Word[4]
    incoming addr   : Word[14]
    incoming wdata  : Word[16]

    outgoing rdata : Word[16]

    storage data[16384] : Word[16]

    on clk {
        when cs {
            if {
                when we {
                    data[addr][15..12] <= wdata[15..12] when we_nib[3]
                    data[addr][11..8]  <= wdata[11..8]  when we_nib[2]
                    data[addr][7..4]   <= wdata[7..4]   when we_nib[1]
                    data[addr][3..0]   <= wdata[3..0]   when we_nib[0]
                }
            }
        }
    }
    on clk {
        when cs {
            rdata <= data[addr]
        }
    }
}
```

Notes: single-port ⇒ the write and read processes must share an
address expression (here, literally the same component) — the pairwise
checker proves one address per cycle trivially. The four nibble
writes are disjoint-lane (example 5's pattern) ⇒ `MASKWREN[3:0]`.
`STANDBY`/`SLEEP`/`POWEROFF` are tied low by the mapping (per the lib
comment) unless the user declares them. Depth 16384 exceeds the 4K
BRAM ⇒ mapping selects SPRAM by size class.

## 9. Initialization (`INIT_FILE` / initializer)

Both primitives support initialization (`@parameter("INIT_FILE", "")`
on the 4K RAM; SPRAM contents come up as part of configuration on
supported flows).

```virdant
memory RomTable {
    incoming clk  : Clock
    incoming addr : Word[8]

    outgoing rdata : Word[16]

    storage lut[256] : Word[16] = {
        0 => 0xbeef,
        1 => 0x0dd0,
        else => 0x0000,
    }

    on clk {
        rdata <= lut[addr]
    }
}
```

Notes: read-only use ⇒ `WE=0` tied. The initializer maps to
`INIT_FILE` (or Virdant-generated hex); per PLAN.md decision 7 this is
platform-checked and rejected where the flow can't honor it. The `{}`
arm-with-`else` shape mirrors `match`.

## 10. Wider than 16: width composition

32- and 64-bit words — the mapping composes multiple 4K BRAMs side by
side (each carries different bits; this is *composition*, not the
port-replication the plan rejects).

```virdant
memory WideSram {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[8]
    incoming wdata : Word[32]
    incoming raddr : Word[8]

    outgoing rdata : Word[32]

    storage data[256] : Word[32]

    on clk {
        data[waddr] <= wdata when we
    }
    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: `Word[32]` ⇒ two `SB_RAM40_4K`s (write `WRITE_MODE=0`, read
`READ_MODE=0`), one per 16-bit half, addresses wired in common. The
conflict story is unchanged (same address space, wider lanes). `Word[64]`
⇒ four. Beyond that, capacity per instance (256×64 = 16 Kbit) still
fits, so this composes arbitrarily in width.

## 11. Deeper than the primitive: address banking (advanced)

2048×8 from two 512×8 halves, selected by the top address bit.

```virdant
memory BankedSram {
    incoming clk   : Clock
    incoming we    : Bit
    incoming waddr : Word[11]
    incoming wdata : Word[8]
    incoming raddr : Word[11]

    outgoing rdata : Word[8]

    storage data[2048] : Word[8]

    on clk {
        if {
            when waddr[11..10] == 0b0 {
                data[waddr] <= wdata when we
            }
            else {
                data[waddr] <= wdata when we
            }
        }
    }
    on clk {
        rdata <= data[raddr]
    }
}
```

Notes: the two writes index the same storage under guards the checker
reads as *address ranges* (`waddr[11..10]` constant per arm ⇒ disjoint
halves) ⇒ one write port per physical BRAM, muxed. This is the
interview's guard-range-analysis case. **Deferred from V1** (PLAN.md):
the mapping here is a mux + two primitives, and the read side similarly
splits. Shown to pin down what the checker must accept, not to promise
early support.

---

## Mapping summary

| Declaration (style #1)            | Primitive + config                          |
|-----------------------------------|---------------------------------------------|
| 1 — 256×16, shared clock          | `SB_RAM40_4K`, WM=0 RM=0, RCLK=WCLK         |
| 2 — 256×16, two clocks            | `SB_RAM40_4K`, WM=0 RM=0, RCLK≠WCLK         |
| 3 — 512×8 / 1024×4 / 2048×2       | `SB_RAM40_4K`, WM=RM=1/2/3                  |
| 4 — write 8, read 16              | `SB_RAM40_4K`, WM=1 RM=0 (folded read)      |
| 5 — disjoint slice writes         | `SB_RAM40_4K`, MASK per-bit                 |
| 6 — guarded write/read, hold      | `SB_RAM40_4K`, WCLKE/RCLKE/RE               |
| 7 — `negedge` read/write          | `SB_RAM40_4KNR` / `-NW` / `-NRNW`           |
| 8 — 16384×16 single-port          | `SB_SPRAM256KA`                             |
| 9 — initializer, read-only        | `INIT_FILE`, WE tied 0                      |
| 10 — width ≥ 32                   | 2/4 × `SB_RAM40_4K` composed                |
| 11 — guard-split address ranges   | 2 × `SB_RAM40_4K` banked (deferred)         |
| `comb` read process               | **no primitive** — reject, or small LUT mux (not flops) |
