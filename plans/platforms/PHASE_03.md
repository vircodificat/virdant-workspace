# Phase 3: Symbol Table Support

> **Unchanged from v2** in substance. Under v3 this phase matters even
> more: `SymbolKind::Platform` items are what `for` clauses resolve to
> (Phase 4), and platform ports must be `SymbolKind::Component` slots
> so `get_ports_of` can compare them against a module's ports.

Edit `virdant/src/analysis/symbols.rs`.

### 3.1 `SymbolKind::Platform`

**Why:** `platform` items need their own symbol kind so they're recognized as
a distinct kind of item (for lookup, import resolution, etc.) rather than
being confused with module defs or typedefs.

- Add the variant to `SymbolKind` (`:44-58`).
- In `is_item()` (`:198`), return true for `Platform`.
- `is_typedef()` (`:211`) stays false.

### 3.2 `node_to_symbol_kind` arm

**Why:** This match is exhaustive over AST payload kinds, so a `platform`
item has no way to become a symbol until it's handled here.

Add to the match (`:487-498`), or the `unreachable!()` at `:496`
fires:

```rust
AstNodePayload::Platform(_) => SymbolKind::Platform,
```

### 3.3 `build_symboltable_item` arm

**Why:** This is the dispatch point that builds child symbols for each item
kind; without a `Platform` arm here, a platform's ports would never be
walked and registered in the symbol table.

Add to the dispatch match (`:319-350`):

```rust
AstNodePayload::Platform(_) => {
    build_symboltable_platform_slot(
        symbols, diagnostics, packages, package, parsing,
        item_name, &node, id,
    );
}
```

### 3.4 `build_symboltable_platform_slot`

**Why:** Platform ports need their own builder because they only accept a
restricted subset of component kinds (`Incoming`/`Outgoing`, no
`Reg`/`Wire`/etc.), while still needing the same duplicate-name checking
that module slots get.

Mirror `build_symboltable_moddef_slot` (`:353-418`) but accept only
`Incoming`/`Outgoing` components.  Keep the real signature
(`packages: &PackageTable, package: PackageId`) and build fqns via
`packages.name(package)`:

```rust
fn build_symboltable_platform_slot(
    symbols: &mut Vec<(BString, Symbol)>,
    diagnostics: &mut Vec<Diagnostic>,
    packages: &PackageTable,
    package: PackageId,
    parsing: &Parsing,
    item_name: &BString,
    node: &AstNode<'_>,
    parent_id: SymbolId,
) {
    use crate::syntax::ast::item_children;
    let mut seen: IndexMap<BString, Region> = IndexMap::new();

    for child in item_children(node) {
        let AstNodePayload::Component(component) = child.payload()
            else { continue };
        if !matches!(component.kind,
            crate::common::ComponentKind::Incoming
            | crate::common::ComponentKind::Outgoing)
        {
            continue;
        }

        let port_name: BString = parsing.string(component.name).to_owned();
        let port_region = Region::new(package, child.span());

        if seen.contains_key(&port_name) {
            diagnostics.push(Diagnostic::new(
                port_region,
                diagnostics::DuplicateSlot {
                    item: item_name.to_owned().into(),
                    slot: port_name,
                },
            ));
            continue;
        }
        seen.insert(port_name.clone(), port_region);

        let port_fqn: BString = format!(
            "{}::{}::{}",
            packages.name(package).to_str_lossy(),
            item_name,
            port_name,
        ).into();
        let location = Location::new(package, child.id());
        let port_id = SymbolId(symbols.len().try_into().unwrap());
        symbols.push((
            port_fqn.clone(),
            Symbol {
                id: port_id,
                fqn: port_fqn,
                name: port_name,
                location,
                kind: SymbolKind::Component,
                parent_id: Some(parent_id),
            },
        ));
    }
}
```

**Why:** Platform ports behave like any other port once they're in the
symbol table, so giving them a separate symbol kind would just duplicate
logic; reusing `SymbolKind::Component` lets the existing port machinery
handle them for free.

Reusing `SymbolKind::Component` for platform ports means
`get_ports_of`, docs, and Verilog conversion need no new port kind.