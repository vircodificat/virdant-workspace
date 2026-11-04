# Plan: Component Rename for the Virdant LSP

Date:   2026-10-24
Status: In Planning

## Goal

Support LSP rename for **components** (signals, ports, registers, sockets,
etc. declared inside a moddef) in `virlsp`. This is Phase 1; renaming
**items** (moddefs, structs, enums, sockets, fn defs) across packages is a
follow-up (Phase 2).

## Current State

The LSP backend is `virdant/src/bin/virlsp.rs` (tower-lsp). It implements
hover, goto-definition, completion, code actions, and diagnostics. There is
**no** `rename` or `prepare_rename` handler, and `ServerCapabilities` does not
advertise rename support.

All the hard analysis needed already exists:

- `virdant/src/analysis/component.rs` defines `ComponentAnalysis`, which
  collects, per moddef:
  - every component with its `Location` (declaration),
  - `references: IndexMap<Location, (ComponentId, ReferenceKind)>`, built by
    `collect_references_in_expr` / `collect_references`, which already
    handles driver targets, `it.` rewriting, and match-pattern shadowing.
- `goto_definition` in `virlsp.rs` already contains the position → component
  resolution logic: find node at `LineCol`, handle `Path` /
  `ExprReference` payloads, walk up to the enclosing item, resolve the path
  string via `db.get_component_analysis(item_symbol.id).resolve(path_str)`.

So component rename is mostly plumbing, not new analysis.

## Phase 1: Component Rename

### 1. Expose references from the library (small)

`references` is private with no accessor. Add to `ComponentAnalysis`:

```rust
pub fn references_of(&self, id: ComponentId) -> Vec<Location>
pub fn references(&self) -> Vec(Location, ComponentId, ReferenceKind)>
```

### 2. Shared position → component resolution (moderate, mostly refactoring)

Factor the logic out of `goto_definition` into a shared helper, e.g.:

```rust
fn resolve_component_at(
    db: &Db,
    parsing: &Parsing,
    package: PackageId,
    linecol: LineCol,
) -> Option<Component>;
```

Reuse it in `goto_definition`, and in `document_highlight` (which currently
just highlights the node under the cursor).

### 3. LSP handlers (small)

- Advertise capability in `initialize`:

  ```rust
  rename_provider: Some(OneOf::Right(RenameOptions {
      prepare_provider: Some(true),
      ..Default::default()
  })),
  ```

- `prepare_rename`: resolve the component at the position; return the name
  token's `span_to_range(...)` plus the name as placeholder.
  (Verify whether component `location()` points at the name token or the
  whole declaration; if the latter, use the parsing node's name span.)

- `rename`:
  1. Validate the new name (identifier rules; check
     `component_analysis.resolve(new_name)` for collisions → return a
     `jsonrpc::Error` if invalid).
  2. Collect the declaration `Location` + all `references_of(id)` locations.
  3. Group edits by package; convert each `Location` to a URI via the
     existing `package_id_to_uri`, and to a
     `TextEdit { range: span_to_range(node.span()), new_text }` via
     `db.get_parsing(loc.package()).ast_node(loc.ast_node_id())`.
  4. Build a `WorkspaceEdit` — the two code-action functions
     (`missing_match_cases_action`, `missing_platform_ports_action`) already
     show the exact pattern (`changes: HashMap<Url, Vec<TextEdit>>`).

Server-side edits round-trip through `did_change` (full-text sync,
re-analyzes the whole db), so no extra invalidation work is needed.

### 4. Decisions / gotchas

- **Synthetic components** (platform `for` clause, `synthetic: true`): their
  declaration lives in the platform library (`lib/*.vir`, a different
  package). Decide: refuse renaming those (recommended), or edit the library
  file too.
  DECISION: Refuse to rename
- **Edit locality**: component paths are module-local, so all edits land in
  the moddef's own package — simple. This is why components are the right
  first rename target.

### Estimated effort

~150–250 lines total: one accessor, one shared resolve helper, two handlers,
capability registration, plus a few tests.

## Phase 2 (follow-up): Item Rename

Renaming **items** (moddefs, structs, enums, sockets, fn defs) is a bigger
job: the `SymbolTable` gives the declaration `Location`, but there is no
cross-package "find all references to this item" query.

Needed: a query that walks every package's `Parsing` looking for `Ofness` /
`Type` / qualified `Path` payloads whose package+name resolve to the target
symbol (mirroring the `resolve_item(item_name, pkg)` logic in
`goto_definition`).

Phase 1 gets the LSP plumbing (`prepare_rename`, `rename`, cross-file
`WorkspaceEdit`) proven end-to-end first.

## Open Questions

1. Synthetic components: refuse rename, or edit `lib/*.vir`?
ANSWER: Refusal for renaming synthetic components.
2. Name validation: how strict? (Reuse whatever identifier rules the parser
   enforces; reject shadowing an existing component in the same moddef.)
ANSWER: Be strict to begin with.
3. Should `document_highlight` be upgraded to highlight *all* references of
   the component under the cursor as part of this work (it is nearly free
   once step 2 is done), or kept minimal?
ANSWER: Yes.
