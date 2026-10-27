# Implementation Notes (PLANv3)

Date: 2026-10-26

## Implemented and verified

- **Grammar/AST**: `bus`/`net` component declarations (no `on`/`default`
  clauses — rejected at parse), `#=` shared-driver token and statement
  (`payload::Driver.shared`), socketdef `bus` slots (`ChannelDir::Bus`),
  `KwBus`/`KwNet` keywords. `GRAMMAR.txt` regenerated via `make grammar`.
- **Common enums**: `ComponentKind::{Bus, Net}`, `ChannelDir::Bus`,
  `PortDir::Bidir`; `DriverType` unchanged (sharedness carried by the
  driver payload).
- **Analysis**: flow mapping (`Bus`/`Net` → `Flow::Duplex`, child bus
  ports do not flip), submodule bus-port registration, socket bus slots
  registered `Duplex`+`Bus` on both roles, ports list (`Bus` →
  `PortDir::Bidir`; `Net` not a port).
- **Driver rules** (`check_drivers.rs`): `SharedDriverOnOwned` /
  `OwnedDriverOnShared` operator-vs-kind checks; `NoDrivers`,
  `NoRegDrivers`, `MultipleDrivers` skipped for shared targets;
  shared targets expect `DriverType::Continuous`; it-block targets
  resolved with the it-context so per-driver flags work.
- **Typing/fencing** (`types/typing`): `ExprCtx` {Owned,
  SharedDriverRhs, PullArg} threaded from the exprroot's parent driver;
  shared references illegal outside `#=` RHS and pull arguments
  (`UnsafeSharedRead`); `Primitive::{Pullup, Pulldown}` resolved by
  name+arity (`PullNotOnShared`, `PullDriveOutsideSharedDriver`);
  join type equality via the ordinary `WrongType` path.
- **Match coverage**: statement-level matches whose arms contain only
  `#=` drivers are exempt from exhaustiveness (absence-as-Z); the
  latched (`<=`) exemption is unchanged.
- **Diagnostics**: all five new payloads implemented with final
  wordings; error fixtures updated to match.
- **Verilog backend**: `inout` ports, `net` → wire declaration, ZLit
  (absence-as-Z) defaults for shared targets (XLit kept for owned),
  unguarded weak drives → `pullup`/`pulldown` primitives, guarded weak
  drives → documented strong conditional approximation, joins →
  per-bit scalar `tran` (iverilog rejects vector tran arrays and
  unnamed instance arrays), socket bus bulk-connect → `tran`, pull
  reads lowered to the net reference plus a pull primitive in the
  reading module.
- **Ancillary**: formatter (`#=` driver alignment, shared-class type
  alignment), virdoc (bus port docstrings in ports table, wire/reg/net
  internals section, `bus` direction badge), LSP/manifest/vir.rs
  `Bidir` arms, platform check rejects `bus` ports via the existing
  extra-port error.

## Test status

`make test` green end to end: `cargo test`, tests/errors (+ the two
project-style error dirs), tests/format, tests/pass, tests/virdoc,
tests/warnings, docs tests, and all seven project-style simulator
directories (`bus_tristate_sim`, `bus_floating_sim`, `bus_join_sim`,
`bus_socket_sim`, `bus_i2c_style_sim`, `bus_pull_guards_sim`,
`bus_word_sim`) pass under iverilog (run directly; they are not in
`tests/pass/Makefile`'s `VIR_PROJS` per the plan).

Fixture renames forced by the new keywords (old identifiers collided):
`bus` socket instances → `busif` (generics_instances.vir,
generics_mod/src/top.vir, submod_socket/src/top.vir), and the socket
slot named `wire` in bus_socket_slot.vir → `line`. bus_join_sim's
source gained `outgoing cseen` on Top so the testbench's expected port
exists.

## Deferred: §8 Rust simulator rework

The Rust `Sim` module (`virdant/src/sim/*` + elaboration net classes)
was NOT reworked. Rationale:

- `Sim` has no users: no CLI command and no test exercises it — the
  plan's own test section routes all simulator-semantics coverage
  through iverilog on the generated Verilog, which is complete and
  green.
- A faithful §8 (union-find net classes in elaboration, per-net
  multi-driver circuit with strengths, per-bit `NetState` resolution
  over arbitrary types, contention abort, `InitPolicy`, VCD net
  dumping) is a several-hundred-line rework of the resolution core
  whose correctness the suite cannot observe. A partial rework risks
  silently-wrong shared-net behavior in dead code.

The placeholder arms left behind (`Primitive::Pullup/Pulldown`
unreachable in `sim/expr.rs`) mark the seam; §8 should be implemented
as its own pass, ideally after a small `Sim`-driven test harness
exists so it can be verified.
