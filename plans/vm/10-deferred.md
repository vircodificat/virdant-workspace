# Deferred, Open, and Out of Scope

Every item explicitly deferred during the design interview, so nothing gets
silently lost.

## Deferred decisions (to be made later, by design)

- **Ext mods in VM v1 → deferred to VM v2.**
  The `ExtMod` trait, `exteval`, the `.ext` assembly section, and the
  binding table are all v2 work.
  v1 (hand-written assembly + first suite bring-up) runs pure logic.
  Consequences carried: the one `tests/run` test needing an ext mod
  (`extmod-into-virdant-child`) is out of gate scope, and the virt board
  rewrite (UART, SPI flash, exit port) waits for v2 — `vir run` cannot
  serve the `virt::Virt` platform until then.
  Open when picked back up: flat vs. scoped port syntax in `.ext`.

- **Anomaly taxonomy and check placement.**
  What exactly counts as an anomaly (width violations, enum-tag conformance,
  uninitialized reads), whether checks are per-write / boundary-only /
  debug-build, and whether severity is configurable.
  Default until then: **terminate whenever an anomaly is detected**, but v1
  ships without mandatory checks.
- **`dontcare` fill conformance caveat.**
  v1 resolves `dontcare` eagerly with 0-fill, which need not conform to the
  target type; documented caveat, revisit if it matters.
- **Tri-state modeling** (`net` / `bus`).
  Deferred entirely; v1 does not model them.
- **Multi-clock scheduling.**
  Reserved for: reg→clock tagging, per-clock NEXT sub-sections, `tick(clk)`
  keyed off signal identity.
  A first-class domain construct has a language-side sketch and stays open.
- **Probe-ability restriction.**
  Today everything is probeable, so the compiler may not dead-code-eliminate
  any named net.
  The future change — a keep-list or liveness annotation — unlocks aggressive
  optimization.
  The side table and ID space are designed so this is a compiler policy
  change, not an ABI change.
- **Watchpoints, mid-run pokes, partial re-evaluation.**
  Constraint carried forward: the step function stays structured
  (settle → commit, clean entry points), never fused into one opaque blob.
- **Save/restore of a running session.**
  Eventual, not v1.
  Defined now so the ABI never breaks: the ext mod trait carries
  `init`/`save`/`load` from day one; slot-file snapshot is free (it is
  bytes).
- **Per-instance ext mod implementation overrides.**
  Binding is by ext mod name globally; per-instance override would touch only
  the binding table.
- **Dynamic-index writes.**
  `foo[dyn k]` is read-only today.
  If writes ever enter the language, they need a register-file-style write op
  and commit-model interaction — a future *language* decision.
- **The program artifact format.**
  In-memory only for v1, under the serialization-shaped constraint
  (flat data, indices, no closures, no cycles, version field).
- **Commit upgrade to minimal-snapshot.**
  The bulk-copy commit is v1; the compiler-analyzed snapshot of
  read-pre-edge registers is the performance-optimal endpoint and drops in
  locally.
- **Per-input dirty bits / incremental settle.**
  Global dirty flag in v1.
- **Fusion threshold tuning.**
  Deliberately dumb initial heuristic; tuned by measurement when
  instrumentation or workloads warrant.
- **Branchiness of target designs.**
  Unknown; to be measured. Affects how much fusion policy matters.
- **`branch!` mnemonic.**
  Semantics fixed; spelling negotiable.
- **`asclock` and clock-as-driven-signal edge cases.**
  `tick` keys off signal identity; any corner semantics of promoting
  arbitrary driven bits to clocks are a language-side question.
- **Static reports / instrumentation variants.**
  Chosen: none for v1.
  Retrofit constraint: program structure survives to the artifact (fusion is
  a pass), and the encoding leaves opcode room.
- **Tracing record-all default.**
  Assumed (flagged as an assumption in interview): v1 traces all signals,
  filters later.

## Open mechanical layers (to decide at implementation start)

- Exact instruction encoding widths and field layout.
- `Word[65+]` limb representation — design intent recorded (one opcode
  set, load-time width-specialized handlers, `.wide` mnemonic variants
  rejected for debugging/tooling uniformity); the limb mechanics themselves
  are the open layer.
- Slot assignment and locality heuristics.
- Shift-amount semantics for out-of-range shifts (`shl`/`shr`/`sra`).
  **Open at implementation time — panic (`todo!`) if hit:** the
  interpreter should reach a `todo!` when it encounters an out-of-range
  shift until the semantics are chosen (modulo width, clamp, or
  compile-time masked shift amount).

## Explicitly out of scope — do not design for these

- **Hot-ext-mod / netlist performance.**
  A netlist of thousands of trivial ext mods (`SB_LUT4`) would be a callback
  catastrophe; native implementations for standard ext mod families are a
  *future, optional* compiler-side inlining decision.
  The trait mechanism is the only ext mod path for now.
- **Escape hatches for dynamic instantiation, runtime-indexed module
  instances, or intra-cycle interleaving.**
  None of these constructs exist in Virdant and none will be accommodated.
  The static-schedule assumption is load-bearing with no reserved escape
  routes.
- **X/Z propagation, delta cycles, blocking/nonblocking semantics.**
  Two-state, atomic-commit, synchronous-only — by design, permanently.
- **Event-driven evaluation as the backbone.**
  Flat evaluation of the whole schedule is the backbone; activity-based
  cleverness may layer on later as optimization only.
- **Loops or recursion in the VM program.**
  Forward tail-calls only; hardware has no loops, so the machine never needs
  them.
  Any future language feature wanting iteration must unroll at compile time
  or live inside an ext mod.
