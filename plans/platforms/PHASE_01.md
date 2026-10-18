# Phase 1: `item_children` Helper

**Why:** the platform accessors (Phase 4) need to walk a platform
item's port/slot children without tripping over the `Annotations`
wrapper node that every annotatable item carries as `child(0)`, nor
the `Annotation` nodes inside it. Without this helper, that walk would
need to special-case and skip those nodes inline, which is easy to
get wrong and would be duplicated logic already present at other call
sites (e.g. `package.rs:202`).

Edit `virdant/src/syntax/ast.rs`.
The `Annotations` wrapper is always `child(0)` of an annotatable node,
and `children()` returns it like any other child.
Add a helper that filters it and any `Annotation` nodes out:

```rust
/// Returns the structural children of `node`, skipping the leading
/// `Annotations` wrapper node and any `Annotation` children.
pub fn item_children(node: &AstNode<'_>) -> Vec<AstNode<'_>> {
    node.children().into_iter()
        .filter(|c| !matches!(c.payload(),
            AstNodePayload::Annotations(_)
            | AstNodePayload::Annotation(_)
        ))
        .collect()
}
```

Do NOT churn existing call sites of `children()`; they already handle
the wrapper correctly (explicit `Annotations` match at
`package.rs:202`, `_ => continue` patterns elsewhere).
Phase 4 uses `item_children` in the new `platform.rs` only.