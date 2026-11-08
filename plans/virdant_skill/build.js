// Workflow: launch 9 children to write/update the virdant skill topic files.
// Parent (session) later consolidates BUGS and writes SKILL.md itself.

const REPO = "/home/vircodificat/projects/virdant-workspace/virdant";

const PREAMBLE = `You are writing/updating documentation files for the Virdant HDL skill at agents/skills/virdant/ inside the virdant repo (your cwd). The audience is CODING AGENTS: token-efficient, rule-first, copyable examples.

STYLE CONTRACT (mandatory for every file you write):
- Rules as bullets; prose only when a rule needs justification.
- Every example must compile against v0.4.0 idiom: implicit \`on\` when the module has exactly one Clock (a redundant \`on clock\` is now a WARNING); \`pub\` on cross-package items; \`//>\` docstrings; snake_case .vir filenames (hyphens in filenames are compile errors); 4-space indent; no tabs; no trailing spaces.
- Cross-reference sibling topic files by relative path (e.g. see ./let-bindings.md).
- Ground every claim in docs/source/reference/*.rst (under your cwd). Where docs and tests disagree, THE TESTS WIN — and report the disagreement under BUGS.
- Touch ONLY your assigned files. Do NOT write SKILL.md or MISTAKES.md.
- Read the existing file first if updating; preserve good content.
- End your output with a \`BUGS:\` section listing repo bugs/inconsistencies found (docs vs tests, broken links, wrong claims), one line each.`;

const tasks = [
  {
    key: "A-let-control-flow",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/let-bindings.md:
   - \`let\` in statement AND expression position.
   - Comma-separated binding lists (\`let lo = a, hi : Word[8] = b => hi + lo\`).
   - Ordered bindings (later bindings may reference earlier ones; later shadows earlier).
   - Bindings shadow components.
   - Interaction with match-pattern variables (innermost wins).
   - Untyped bindings infer from first use — widthless literal RHS must be anchored by a use site or \`CantInfer\`.
   - TERMINATOR STYLE (compiler-enforced): \`{\` performs (driver statements), \`=>\` produces (value); statement \`let\` must not use \`=>\`; expression \`let\` must not be a bare \`{...}\` block.
   - Chained forms \`let x = e if {...}\` / \`let x = e match {...}\` are legal in both positions, never flagged.
   - \`let\` as arm body (\`when cond let x = expr {...}\`).
   SOURCES: docs/source/reference/statements.rst (Let Bindings, Terminator Style), expressions.rst (Let Expressions), tests/run/let*, tests/check/let*, tests/sim/let*, tests/errors/style/lets, tests/errors/naming/undefined_let.

2. UPDATE agents/skills/virdant/control-flow.md:
   - Keep statement-vs-expression guidance + direct chaining.
   - Add the five arm forms (bare expr / block / nested if / nested match / let arm) for \`when\`, \`with\`, \`else\`.
   - Match GUARDS (\`with @Running(x, y) if {...}\`) as an explicit feature.
   - \`else\` placement (last, one only).
   - Exhaustiveness per scrutinee type (enum incl. payload enumerants via @Ctor; union variants; word literal widths, else \`else\` arm).
   - Undriven arm => registers hold, wires unassigned (tristate-only guards may float => cross-ref ./tristate.md).
   SOURCES: statements.rst, expressions.rst, patterns.rst, tests/errors/match/*, tests/errors/style/if_match_style.`,
  },
  {
    key: "B-generics-sockets",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/generics.md:
   - Generic params on ALL definition forms (struct/union/enum/builtin type, mod, socket), \`[name : kind]\`, kinds \`Type\` and \`Nat\` only.
   - Type arguments at use sites (\`Pair[Word[4], Bit]\`, \`mod r8 of Reg[Word[8]]\`, \`client socket bus of Bus[Word[8]]\`).
   - Nesting (\`Valid[Pair[Word[4], Bit]]\`).
   - Reference-site checks with ACTUAL error messages (wrong arity; kind mismatch both directions; unbound parameter; parameter takes no arguments).
   - Mono modules = (definition, type args), shared by identical instantiations, one Verilog module per mono module with hashed names.
   - RESTRICTIONS: ext mods cannot be generic (use @parameter => cross-ref ./modules.md); ext-mod instantiations take no type args; \`[prog] top\` must be a concrete instantiation.
   SOURCES: docs/source/reference/types.rst (Generics, Kind System), modules.rst (Generic Modules), sockets.rst, tests/check/generics_types, tests/check/generics_instances, tests/golden/generics, tests/errors/platforms/top_generics, tests/run/generic_*.

2. UPDATE agents/skills/virdant/sockets.md:
   - Keep cosi/soci, client/server, it-blocks, \`: = :\` connect operator, naming conventions.
   - Add generic sockets + type args.
   - Mono-socket matching for connect (same definition AND same args; composition through generic module instances).
   - Channel \`dependson\` (soci applies as server, cosi as client; inert in receiving role; exempt from unused warning => cross-ref ./dependson.md).
   - Keep \`unused\` on members/it-blocks.
   SOURCES: sockets.rst, modules.rst (dependson socket section), tests/check/dependson_sockets, tests/run/generic-socket-two-args, tests/run/generic-mod-of-generic-socket.`,
  },
  {
    key: "C-dependson-modules",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/dependson.md:
   - Declaration of POTENTIAL combinational dependency on \`outgoing\`/\`outgoing wire\` ports.
   - Part of the public interface contract; must list EVERY incoming port the port can combinationally depend on (direct, via internal signals, via submodule declarations).
   - Entries must resolve to the module's incoming ports; undeclared = error; declared-but-unused = warning (ext mod bodies exempt, opaque).
   - Placement: only outgoing/outgoing wire (outgoing reg / incoming / bus / wire / reg / net = errors, each diagnosed).
   - Socket channel obligations (=> cross-ref ./sockets.md).
   - Combinational cycle detection over the potentiality graph (driver edges + own dependson + submodules' declared dependson; \`<=\` never traversed; cycle reported where the local edge closes it; error even without an actual path today).
   - Emphasize: agents MUST write correct dependson lists or \`vir check\` fails.
   SOURCES: docs/source/reference/modules.rst (dependson List, Combinational Cycle Detection), tests/errors/cycles/*, tests/errors/dependson/*, tests/check/dependson_*, tests/check/depends_on, tests/warnings/dependson_unused.

2. UPDATE agents/skills/virdant/modules.md:
   - Keep instantiation, it-blocks, export, ext mod, import, \`::\`, incoming forwarding block.
   - Add \`pub\` visibility (error at reference site + warning at definition site naming the referencing package; pub surface must be transitively pub: port types, field types, variant payload types, socket channel types; pub only matters at package boundaries).
   - Import rules (file-level at top, package name = filename stem, no circular imports, bare-name fallback through imports).
   - \`ext mod\` + \`@parameter\` (declare \`@parameter("WIDTH", 8)\` on the ext mod; override per instance on the \`mod ... of\` statement; instance extras appended; rendering: numbers decimal, strings quoted, booleans 1'b1/1'b0).
   - Generic instances => cross-ref ./generics.md.
   - RESOLVE: is \`export\` still valid or replaced by \`pub\` + virdant.toml \`[prog] top\`? (tests use \`pub mod X for virt::Virt\` as tops — check modules.rst/packages.rst and report in BUGS if docs disagree).
   SOURCES: modules.rst, packages.rst, annotations.rst (Parameter Annotations), tests/check/pub_kinds, tests/warnings/unused_non_pub, tests/run/mod_*, tests/run/extmod_into_virdant_child, tests/run/multi-file-*.`,
  },
  {
    key: "D-tristate-types-patterns",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/tristate.md:
   - \`bus\` (bidirectional port, \`inout\` in Verilog) vs \`net\` (internal).
   - \`#=\` strong (undriven guard arm floats), \`#-\` weak (pull resistor; strong overrides weak; emits \`assign (weak1, weak0)\`), \`#=#\` attach submodule bus port (emits \`tran\`).
   - Reading requires \`tap\` (plain reference = \`NetReadWithoutTap\`; tap on non-net = \`TapNotOnNet\`).
   - No \`on\`/\`default\` clauses on bus/net.
   - Tristate-only if/match guards need not be exhaustive (float, not hold).
   - Resolution: strong > weak, none => floating.
   - I2C open-drain idiom from tests/sim/i2c_open_drain.
   SOURCES: docs/source/reference/modules.rst (Tristate Nets), statements.rst (Tristate Drivers), expressions.rst (tap), tests/sim/i2c_open_drain, tests/run/dontcare_*.

2. UPDATE agents/skills/virdant/types-and-patterns.md:
   - Keep struct/union/enum basics, \`\${}\` literals, \`->\`, \`@\` constructors, patterns, ascription.
   - ADD: ONE-HOT enum default (no values given => \`1 << i\`, width = number of enumerants; rationale: all-zeros never valid; single-bit tests).
   - All-or-nothing values rule; width from FIRST enumerant's literal (write \`1w4\` explicitly).
   - Duplicate values error across bases.
   - PAYLOAD ENUMERANTS (\`enum type Opt[T : Type] { None; Some(value : T) }\`; construct \`@Some(v)\`, \`#None\` or \`@None()\`; \`#Some\` on payload enumerant = error "Ctor Some requires arguments: use @Some(...)"; match \`with @Some(v)\`).
   - Valid[T] is a generic payload enum in the builtin package and is ONE-HOT => hardware width is width(T) + 2 (state this explicitly).
   - \`dyn\` indexing (\`arr[dyn idx]\`, Word[n] subject, Word[k] index, n == 2^k, result Bit).
   - \`_\` discards.
   - Bare Ident / dontcare patterns parse but the coverage checker doesn't handle them yet — do NOT recommend them.
   SOURCES: types.rst, patterns.rst, tests/check/enum_onehot, tests/check/index_dyn, tests/run/enum_*, tests/run/match_*.`,
  },
  {
    key: "E-platforms",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/platforms.md:
   - \`platform\` item = board (family/part annotations + pinned ports); platforms are DATA — vir check does not validate contents, errors surface in the build flow.
   - \`for\` clause (\`mod Top for ice40::IceSugar\`, requires \`import ice40\`).
   - Port matching enforced by vir check (exact names/directions/types).
   - Missing platform port = error UNLESS it has a \`default\` clause.
   - \`default <EXPR>\` on outgoing platform ports synthesizes an indistinguishable port (defaults ONLY legal in platform defs — \`DefaultOutsidePlatformPort\` on modules).
   - Omitted incoming ports synthesize as never-read, no unused warning.
   - \`@noinitial\` forbids \`initial\` in the top AND everything reachable (\`InitialNoinitialPlatform\`).
   - Annotations @fpga("..."), @part("..."), @pin(35)/@pin("A1"), @period_ns(83).
   - Shipped \`ice40\` package at lib/ice40.vir (iCESugar, iCEstick); lib/*.vir auto-loads.
   - FLOWS chain: \`vir build\` => \`synth\` => \`pnr\` => \`bitstream\` => \`upload\` (upload has multi-seed PnR retry); cross-ref ./tools.md.
   - Ext mods ignore port-match.
   SOURCES: docs/source/reference/platforms.rst, annotations.rst, lib/ice40.vir, tests/errors/platforms/*.`,
  },
  {
    key: "F-tools",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/tools.md: the \`vir -h\` public surface ONLY.
   - Groups exactly as the CLI shows: Basic: new (bundled template, git init with --no-git opt-out), check (primary verification loop: run after every edit; nonzero on error, warnings printed), run (simulator; -v/--verbose, --fps), poke (interactive peek-poke). Tools: doc, format, ls (language server; also serve mode), completion. Flows: build, synth, pnr, bitstream, upload (multi-seed retry). About/Learning: version, tutor (refuses inside an existing project).
   - Per subcommand: one-line purpose, typical invocation, agent-relevant notes.
   - Depth on check/run/poke: poke script mode per tests/poke/*/script.poke + run.poke (document ACTUAL command syntax: peek/poke/set/tick/advance etc.; \`advance\` steps time without a clock; recent change: lexer keywords usable as plain names in REPL).
   - Also: \`virdant.toml\` (lowercase manifest; \`[prog] top\`) and the \`vir new\` project layout.
   - Run \`vir -h\` and \`vir <sub> -h\` yourself (find the vir binary under the repo — likely target/ or via cargo; check AGENTS.md/Makefile for how to build it).
   SOURCES: \`vir -h\` output, docs/source "Tools" section, tests/poke/*, tests/cli/*.sh, tests/run/*/virdant.toml.`,
  },
  {
    key: "G-diagnostics",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/diagnostics.md: cheat sheet mapping diagnostics to cause/fix.
   - SECTION 1 Errors grouped like tests/errors/ (add_carry, cycles, dependson, drivers, match, naming, platforms, style, syntax): grep \`CHECK\` in tests/errors/**/src/*.vir for ACTUAL message text; per family: message fragment, cause, fix, BAD/GOOD pair where useful.
   - Must cover: combinational loops (incl. via when guards/match subjects, dependson cycles); dependson errors (bad entries, undeclared, wrong carrier, socket forms, statement form); drivers (drive sink, missing on clause, multiple drivers in when/match, redundant unused, unused_but_used, unused sink, initial errors); match (non-exhaustive enum/union/word/bit, overlap, pattern types, else placement); naming (bad filename — valid package identifiers, no hyphens; duplicate items; unresolved; undefined let); platforms (port mismatch, default outside platform, driving undeclared platform port, @noinitial, generic top); style (let terminator, if/match arm style); syntax (tabs, indentation not multiple of 4, docstring marker missing space, parser basics).
   - SECTION 2 Warnings from tests/warnings/: dependson_unused, empty_driver_block, no_reg_drivers, not_it / not_it_initial, platform_ports_no_unused, read_from_sink, redundant_driver, redundant_on_clause (omit \`on\` when exactly one Clock), trailing_spaces, unfilled_hole, unused_non_pub, unused_outgoing_wire, unused_source.
   SOURCES: tests/errors/**, tests/warnings/** (.vir files + CHECK: comments), reference rst for context.`,
  },
  {
    key: "H-idioms",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES:
1. NEW agents/skills/virdant/idioms.md: canonical patterns with compact complete examples adapted from the test corpus (prefer adapting a real test module over inventing syntax):
   (1) FSM: enum state + \`match state\` with self-reference (\`state <= match state { ... }\`), direct arm chaining + guards — tests/run/match-on-reg-selfref, tests/sim/statemachine.
   (2) handshake/Valid[T]-Pipe[T] producer-consumer with @Valid/@Invalid + ready/backpressure — tests/sim/pipe, tests/run/handshake-skid-buffer.
   (3) debouncer + edge detector (find the actual test under tests/run or tests/sim).
   (4) counters: modulo, gray, johnson — tests/run/counter*, gray-counter, johnson-counter.
   (5) shift/barrel-shifter via word() + slices incl. variable-shift match idiom — tests/run/barrel-shifter.
   (6) arithmetic: carry/borrow with \`+.\`/\`-.\` — tests/run/add_carry; comparator-three-way.
   (7) submodule fanout: incoming forwarding block + clock/reset wiring — tests/run/mod-fanout-forwarding, mod-constants-into-child.
   (8) NO self-checking-test-top pattern (out of scope; tools.md covers tools).
   THEN two tables:
   (A) Virdant vs Verilog: exclusive-top slicing (x[N-1:M] => x[N..M], width = hi - lo), no \`<<\`/\`>>\` (word() + slices), no always blocks (driver blocks), \`<=\` only on regs, one-hot enum default, dependson contracts, tristate via bus/net/#-operators, \`->\` projection.
   (B) reserved keywords from grammar.rst (builtin bus client cosi default dependson dontcare dyn else enum export ext false fn for if import incoming initial it let match mod net of on outgoing platform pub reg server soci socket submod struct tap true type union unused when width wire with) — naming a component \`width\` etc. is an error.
   SOURCES: tests/run/**, tests/sim/**, grammar.rst.`,
  },
  {
    key: "I-conventions-driver-literals",
    agent: "worker",
    cwd: REPO,
    task: PREAMBLE + `

ASSIGNED FILES (three updates):
1. agents/skills/virdant/coding-conventions.md:
   - Keep naming/ordering/brace/expression conventions.
   - Update: filenames MUST be valid package identifiers (snake_case, lowercase start, no hyphens — compile error now); tabs = errors; indentation must be multiples of 4 (error); trailing spaces warn; \`//>\` needs a space after the marker (error), \`//!\` at line 1; \`pub\` convention; <100 char lines; keep Valid[T]/Pipe[T] + Sync sections.

2. agents/skills/virdant/driver-blocks.md:
   - Keep block preference, push-in-drivers, culling \`else { it <= it }\`, direct chaining, \`it\` self-reference.
   - UPDATE: clock inference — omit \`on\` when the module has exactly one Clock (redundant_on_clause warning); rewrite ALL examples accordingly (multi-clock examples keep explicit \`on\`).
   - Keep initial section; add @noinitial cross-ref (./platforms.md) + pseudorandom sim init note.
   - Note empty_driver_block / no_reg_drivers warnings.
   - FIX stale \`module-system.md\` reference => \`./modules.md\`.

3. agents/skills/virdant/literals-and-builtins.md:
   - Keep literals/bases/underscores/word()/zext/sext/asclock/cast/dontcare/mux/shifts.
   - ADD: \`+.\`/\`-.\` (result Word[n+1], carry/borrow in top bit; contrast wrapping \`+\`/\`-\`); \`all(w)\` and \`trunc(w)\`; \`?\` holes (typecheck as anything, unfilled_hole warning — use when scaffolding); over-wide decimal literal behavior (check tests/run/literal-truncate-decimal and document what actually happens); \`tap\` one-line pointer => ./tristate.md; unary \`-\` (two's-complement negation) and \`~\` (bitwise NOT for Word).
   SOURCES: expressions.rst, grammar.rst, statements.rst, tests/run/add_carry, tests/run/literal_*, tests/warnings/unfilled_hole.`,
  },
];

const results = await runs.all(tasks);

return results.map((r, i) => ({
  key: tasks[i].key,
  ok: r.ok,
  runId: r.runId,
  outputTail: (r.output || "").slice(-3500),
}));
