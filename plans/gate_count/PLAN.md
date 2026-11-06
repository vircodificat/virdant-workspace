# Plan: gate_count

Date:   2026-11-04
Status: In Planning

Static gate-count model for elaborated Virdant designs: an approximate,
technology-independent count of 1-bit gates and flip-flops per
construct, aggregated per component and per module. Spawned out of
`plans/gates_and_prop` (whose `CONCEPT.md` asked for "a simple
gate-count and propagation delay model"); sibling of `plans/prop_cost/`.

## Motivation

Educational: "how big is this design, really?" A student writes a 32-bit
ripple adder and can see it costs ~160 gates; a naive multiplier shows up
as dramatically more expensive than a shift. Like `prop_cost`, this is a
crude model — it counts *logical* gates, not LUTs or standard cells, and
makes no technology claims. It is a pedagogical magnifying glass, not a
synthesis estimate.

Independent of `prop_cost` and of the VCD transform
(`plans/gates_and_prop/PLANv2.md`): different table, different query, no
shared machinery beyond the elaboration. Can be implemented at any time.

## Model

`w` = operand width in bits, `n` = match arm count, `a` = dynamic-index
address width. All counts are 1-bit logic gates; flip-flops are counted
separately (a register is not "w gates" in any honest sense).

**Binary operators**

| Op | Hardware picture | Gates |
|---|---|---|
| `And`, `Or`, `Xor` | `w` parallel 1-bit gates | `w` |
| `LogicalAnd`, `LogicalOr`, `LogicalXor` | one gate on 1-bit operands | `1` |
| `Eq`, `Neq` | `w` XNORs + AND reduction tree | `2w − 1` |
| `Lt`, `Lte`, `Gt`, `Gte` | ripple comparator: `w` compare cells (XOR + 2 gating gates) + final combine | `3w + 1` |
| `Add`, `Sub` | `w` full adders (2 XOR + 2 AND + 1 OR each) | `5w` |
| `AddCarry`, `SubBorrow` | same; carry-out is the last FA's carry | `5w` |

**Unary operators**

| Op | Hardware picture | Gates |
|---|---|---|
| `Not` | one gate on a 1-bit operand | `1` |
| `Inv` | `w` inverters | `w` |
| `Neg` | `w` inverters into a ripple add-1 | `6w` |

**Other expression constructs**

| Construct | Hardware picture | Gates |
|---|---|---|
| `If` (`when`, `mux`) | per-bit 2:1 mux (2 AND + 1 OR) | `3w` |
| `Match` | tree of 2:1 muxes | `3w · ⌈log₂ n⌉` |
| `Any`, `All` | reduction tree | `w − 1` |
| `IndexDyn` | decoder (`2^a` AND gates) + mux tree | `2^a + 3w·a` |
| `Index`, `IndexRange`, `Word`, `Zext`, `Sext`, `Trunc`, `Cast`, `As`, `AsClock` | wiring | `0` |
| `Ctor`, `Struct`, `Enumerant` | aggregate wiring | `0` |
| `Reference`, `Paren`, literals, `Hole`, `Dontcare` | wiring / constants | `0` |
| `Method`, `Fn` | not evaluable today; composed when they are | inline |

**Registers**

| Construct | Count |
|---|---|
| register (any kind) | `w` **flip-flops** (reported in a separate `flops` bucket, not summed into gates) |

The numbers are deliberately round and derivable by hand — a student
should be able to check `adder` (four full adders = 20 gates) against
the report and see it match.

## Implementation

A cached `Db` query, structurally identical to `prop_cost` (same
driver→expression conversion, same memoized walk over the same DAG) with
a different per-node table and an accumulator instead of a max:

```rust
// src/analysis/gate_count.rs
pub struct GateCount {
    per_signal: Vec<(u64 /*gates*/, u64 /*flops*/)>,   // by SignalId::index()
}
impl GateCount {
    pub fn of_signal(&self, signal: SignalId) -> (u64, u64);
}

#[query]
pub fn get_gate_count(db: &Db, top: SymbolId) -> Arc<GateCount>;
```

* A signal's cost is the cost of its driver expression (registers
  contribute `w` flops, 0 gates; wiring costs nothing).
* Module/instance totals are derived on demand by summing component
  costs grouped by elaborated path prefix — no extra query state.

### CLI

A small report command over the query:

```
vir gates              # top module: total gates, total flops, per-module breakdown
vir gates --detail     # additionally the N most expensive individual signals
```

Output is a plain table (module path, gates, flops), sorted by gates
descending in `--detail`. Reuses the companion-binary mechanism.

## Tests

* Table unit tests: every row, including `w = 1` degenerate cases
  (`Eq` on 1 bit = 1; `Any` on 1 bit = 0).
* Whole-design tests: the docs' `adder` example totals exactly 20 gates,
  0 flops; the `buffer` example totals 0 gates, 32 flops.
* Aggregation test: a top module instantiating two `Adder4`s reports 40.

## Out of scope

* Technology mapping (LUTs, cells), synthesis realism of any kind.
* Delay modeling — see `plans/prop_cost/`.
* Power/area estimates.
