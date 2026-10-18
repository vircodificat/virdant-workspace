# Clock Domain Analysis Algorithm

This document describes the three-pass constraint-based algorithm for inferring and checking clock domains in Virdant modules.

## Overview

Domain analysis is a **per-module** analysis that produces:
1. A domain assignment for all components in the module
2. A polymorphic `DomainSignature` for the module's external interface
3. Diagnostics for any domain violations

The algorithm has three passes:

```
Pass 1: Constraint Generation
    Walk AST → generate domain variables and constraints

Pass 2: Constraint Solving
    Union-find over equivalence classes → find satisfying assignment

Pass 3: Signature Extraction
    Build polymorphic signature for module interface
```

## Data Structures

### Constraint System Result

```python
@dataclass
class ConstraintSystem:
    variables: Set[Variable]
    fixed: List[Constraint]
    strict_upper_bounds: List[Constraint]
    upper_bounds: List[Constraint]
    equalities: List[Constraint]
```

### Solve Result

```python
@dataclass
class SolveResult:
    union_find: UnionFind
    errors: List[str]
```

### Domain Categories

```python
Domain = Async | Sync(ComponentId) | Clock | Reset | Constant

# Lattice partial order: d₁ ≤ d₂ means "d₁ is at most as specific as d₂"
#
#          Async (top)
#         /   |   \
#   Sync(C₁) Sync(C₂) ...
#         \   |   /
#       Constant (bottom)
#
# Clock and Reset are special (don't participate in lattice)
```

### Constraint Types

```python
Constraint =
    | Fixed(var, domain)              # dom(var) = domain
    | StrictUpperBound(var, domain)   # dom(var) < domain
    | UpperBound(var, domain)         # dom(var) ≤ domain
    | Equality(var1, var2)            # dom(var1) = dom(var2)
```

### Union-Find State

```python
@dataclass
class Class:
    parent: Variable
    rank: int
    domain: Option[Domain]

# Union-Find maintains equivalence classes
# Variables are ComponentId or ExprId
# Domain is None (unresolved) or a concrete Domain
```

---

## Pass 1: Constraint Generation

Walk the module's AST to generate domain variables and constraints.

```python
def generate_constraints(module: Module) -> ConstraintSystem:
    variables = set()
    fixed = []
    strict_upper_bounds = []
    upper_bounds = []
    equalities = []

    # Step 1: Create domain variables for all components
    for component in module.components:
        variables.add(component.id)

    # Step 2: Walk declarations and drivers, identify expressions
    expr_id = 0
    for decl in module.declarations:
        expr_id = walk_declaration(decl, &mut variables, &mut fixed, &mut strict_upper_bounds, expr_id)

    for driver in module.drivers:
        expr_id = walk_driver(driver, &mut variables, &mut equalities, &mut upper_bounds, expr_id)

    return ConstraintSystem(
        variables=variables,
        fixed=fixed,
        strict_upper_bounds=strict_upper_bounds,
        upper_bounds=upper_bounds,
        equalities=equalities
    )


def walk_declaration(
    decl: Declaration,
    variables: &mut Set[Variable],
    fixed: &mut List[Constraint], strict_upper_bounds: &mut List[Constraint],
    expr_id: int,
) -> int:
    """Process a component declaration."""
    component = decl.component

    # Clock-typed signals
    if decl.type == Clock:
        fixed.append(Fixed(component, Clock))
        strict_upper_bounds.append(StrictUpperBound(component, Async))

    # Reset-typed signals
    elif decl.type == Reset:
        fixed.append(Fixed(component, Reset))
        strict_upper_bounds.append(StrictUpperBound(component, Async))

    # Registers with `on clock` clause
    elif decl.kind == Register and decl.on_clause:
        clock_component = decl.on_clause
        fixed.append(Fixed(component, Sync(clock_component)))
        strict_upper_bounds.append(StrictUpperBound(component, Async))

        # Clock reference in `on` clause creates an expression
        expr = expr_id
        expr_id += 1
        variables.add(expr)
        equalities.append(Equality(expr, clock_component))

    # Explicitly async components
    elif decl.async_modifier:
        fixed.append(Fixed(component, Async))
        # No strict upper bound for async components!

    # Other components (incoming ports, wires, etc.)
    else:
        # Implicit: dom(component) < Async (async must be explicit)
        strict_upper_bounds.append(StrictUpperBound(component, Async))

    return expr_id


def walk_driver(
    driver: Driver,
    variables: &mut Set[Variable],
    equalities: &mut List[Constraint],
    upper_bounds: &mut List[Constraint],
    expr_id: int,
) -> int:
    """Process a driver (assignment)."""
    target = driver.lhs  # ComponentId
    expr_id, rhs_expr = walk_expression(driver.rhs, &mut variables, &mut equalities, &mut upper_bounds, expr_id)

    # Target driven by RHS expression
    equalities.append(Equality(target, rhs_expr))

    return expr_id


def walk_expression(
    expr: Expression,
    variables: &mut Set[Variable],
    equalities: &mut List[Constraint],
    upper_bounds: &mut List[Constraint],
    expr_id: int,
) -> (int, ExprId):
    """
    Walk an expression tree, generating expression nodes and constraints.
    Returns: (next_expr_id, expr_root_id)
    """

    # Component reference
    if expr.kind == Reference:
        expr_node = expr_id
        expr_id += 1
        variables.add(expr_node)

        # Expression tied to referenced component
        equalities.append(Equality(expr_node, expr.component))
        return (expr_id, expr_node)

    # Literal (free variable, no constraints)
    elif expr.kind == Literal:
        expr_node = expr_id
        expr_id += 1
        variables.add(expr_node)
        # No constraints generated for literals
        return (expr_id, expr_node)

    # Binary operator
    elif expr.kind == BinaryOp:
        # Recurse on operands
        expr_id, left_expr = walk_expression(expr.left, &mut variables,
                                             &mut equalities, &mut upper_bounds, expr_id)
        expr_id, right_expr = walk_expression(expr.right, &mut variables,
                                              &mut equalities, &mut upper_bounds, expr_id)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # Unified operator rule: operands can upcast to result
        upper_bounds.append(UpperBound(left_expr, result_expr))
        upper_bounds.append(UpperBound(right_expr, result_expr))

        return (expr_id, result_expr)

    # Match expression
    elif expr.kind == Match:
        expr_id, subject_expr = walk_expression(expr.subject, &mut variables,
                                                &mut equalities, &mut upper_bounds, expr_id)

        arm_exprs = []
        for arm in expr.arms:
            expr_id, arm_expr = walk_expression(arm.result, &mut variables,
                                               &mut equalities, &mut upper_bounds, expr_id)
            arm_exprs.append(arm_expr)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # Subject and all arms must equal result
        equalities.append(Equality(subject_expr, result_expr))
        for arm_expr in arm_exprs:
            equalities.append(Equality(arm_expr, result_expr))

        return (expr_id, result_expr)

    # When expression (similar to match)
    elif expr.kind == When:
        guard_exprs = []
        arm_exprs = []

        for case in expr.cases:
            expr_id, guard_expr = walk_expression(case.guard, &mut variables,
                                                 &mut equalities, &mut upper_bounds, expr_id)
            expr_id, arm_expr = walk_expression(case.result, &mut variables,
                                               &mut equalities, &mut upper_bounds, expr_id)
            guard_exprs.append(guard_expr)
            arm_exprs.append(arm_expr)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # All guards and all arms must equal result
        for guard_expr in guard_exprs:
            equalities.append(Equality(guard_expr, result_expr))
        for arm_expr in arm_exprs:
            equalities.append(Equality(arm_expr, result_expr))

        return (expr_id, result_expr)

    # Struct construction
    elif expr.kind == StructLiteral:
        field_exprs = []
        for field in expr.fields:
            expr_id, field_expr = walk_expression(field.value, &mut variables,
                                                 &mut equalities, &mut upper_bounds, expr_id)
            field_exprs.append(field_expr)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # All fields must equal result
        for field_expr in field_exprs:
            equalities.append(Equality(field_expr, result_expr))

        return (expr_id, result_expr)

    # Array indexing
    elif expr.kind == ArrayIndex:
        expr_id, array_expr = walk_expression(expr.array, &mut variables,
                                             &mut equalities, &mut upper_bounds, expr_id)
        expr_id, index_expr = walk_expression(expr.index, &mut variables,
                                             &mut equalities, &mut upper_bounds, expr_id)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # Result has same domain as array, index has same domain as array
        equalities.append(Equality(result_expr, array_expr))
        equalities.append(Equality(index_expr, array_expr))

        return (expr_id, result_expr)

    # Struct field access
    elif expr.kind == FieldAccess:
        expr_id, struct_expr = walk_expression(expr.struct, &mut variables,
                                              &mut equalities, &mut upper_bounds, expr_id)

        # Result expression
        result_expr = expr_id
        expr_id += 1
        variables.add(result_expr)

        # Result has same domain as struct
        equalities.append(Equality(result_expr, struct_expr))

        return (expr_id, result_expr)

    # ... other expression kinds
```

---

## Pass 2: Constraint Solving

Use union-find to solve the constraint system, finding a domain assignment that satisfies all constraints (or report errors).

```python
def solve_constraints(constraints: ConstraintSystem) -> SolveResult:
    uf = UnionFind()
    errors = []

    # Initialize: each variable in its own equivalence class
    for var in constraints.variables:
        uf.make_set(var)

    # Step 1: Process fixed constraints
    for Fixed(var, domain) in constraints.fixed:
        root = uf.find(var)
        if uf.classes[root].domain is None:
            uf.classes[root].domain = domain
        elif uf.classes[root].domain != domain:
            # Error: conflicting fixed constraints
            errors.append(f"Conflicting fixed: {var} has {uf.classes[root].domain} and {domain}")

    # Step 2: Process equality constraints (union-find merges)
    for Equality(var1, var2) in constraints.equalities:
        root1 = uf.find(var1)
        root2 = uf.find(var2)

        if root1 == root2:
            continue  # Already in same class

        # Merge classes
        domain1 = uf.classes[root1].domain
        domain2 = uf.classes[root2].domain

        # Both have domains: check compatibility
        if domain1 is not None and domain2 is not None:
            if not compatible(domain1, domain2):
                errors.append(f"Unification error: {domain1} vs {domain2}")
                continue
            # Compatible: use the more specific one
            merged_domain = unify(domain1, domain2)
        elif domain1 is not None:
            merged_domain = domain1
        elif domain2 is not None:
            merged_domain = domain2
        else:
            merged_domain = None

        # Perform union
        new_root = uf.union(root1, root2)
        uf.classes[new_root].domain = merged_domain

    # Step 3: For unresolved classes, use upper bound constraints
    for UpperBound(var, target_var) in constraints.upper_bounds:
        # var's domain must be ≤ target_var's domain
        # Collect all upper bound constraints on each variable
        pass

    # Compute least upper bound (LUB) for unresolved classes with upper bounds
    for var in constraints.variables:
        root = uf.find(var)
        if uf.classes[root].domain is None:
            # Collect upper bounds on this class
            bounds = [b.target for b in constraints.upper_bounds if uf.find(b.var) == root]

            if bounds:
                # Find domains of all bounds
                bound_domains = [uf.classes[uf.find(b)].domain for b in bounds]
                bound_domains = [d for d in bound_domains if d is not None]

                if bound_domains:
                    lub = least_upper_bound(bound_domains)
                    uf.classes[root].domain = lub

    # Step 4: Check strict upper bound constraints
    for StrictUpperBound(var, domain) in constraints.strict_upper_bounds:
        root = uf.find(var)
        assigned = uf.classes[root].domain

        if assigned is not None:
            if not strictly_less_than(assigned, domain):
                errors.append(f"Strict upper bound violation: {var} has {assigned}, required < {domain}")

    return SolveResult(union_find=uf, errors=errors)


# Lattice operations

def compatible(d1: Domain, d2: Domain) -> bool:
    """Check if two domains can be unified."""
    if d1 == d2:
        return True

    # Constant unifies with any Sync
    if d1 == Constant and isinstance(d2, Sync):
        return True
    if d2 == Constant and isinstance(d1, Sync):
        return True

    # Different Sync domains are incompatible
    if isinstance(d1, Sync) and isinstance(d2, Sync):
        return d1.clock == d2.clock

    # Async, Clock, Reset don't unify with anything except themselves
    return False


def unify(d1: Domain, d2: Domain) -> Domain:
    """Return the unified domain (assumes compatible)."""
    if d1 == d2:
        return d1

    # Constant weakens to Sync
    if d1 == Constant:
        return d2
    if d2 == Constant:
        return d1

    # Should not reach here if compatible() returned True
    raise Exception(f"Cannot unify {d1} and {d2}")


def least_upper_bound(domains: List[Domain]) -> Option[Domain]:
    """Find the least upper bound of a set of domains in the lattice."""
    if not domains:
        return None

    # Remove duplicates
    domains = list(set(domains))

    if len(domains) == 1:
        return domains[0]

    # If any is Constant, ignore it (bottom element)
    domains = [d for d in domains if d != Constant]

    if len(domains) == 0:
        return Constant
    if len(domains) == 1:
        return domains[0]

    # Check if all are the same Sync
    if all(isinstance(d, Sync) for d in domains):
        clocks = [d.clock for d in domains]
        if len(set(clocks)) == 1:
            return domains[0]

    # Different Sync domains → LUB is Async
    return Async


def strictly_less_than(d1: Domain, d2: Domain) -> bool:
    """Check if d1 < d2 in the lattice (strictly less specific)."""
    if d1 == d2:
        return False

    # Constant < everything
    if d1 == Constant:
        return d2 in [Async, Clock, Reset] or isinstance(d2, Sync)

    # Sync < Async
    if isinstance(d1, Sync) and d2 == Async:
        return True

    # Clock, Reset < nothing (except themselves, but that's handled above)
    return False


# Union-Find operations

class UnionFind:
    def __init__(self):
        self.classes = {}  # Variable → Class

    def make_set(self, var: Variable) -> None:
        self.classes[var] = Class(
            parent=var,
            rank=0,
            domain=None
        )

    def find(self, var: Variable) -> Variable:
        """Find with path compression."""
        if self.classes[var].parent != var:
            self.classes[var].parent = self.find(self.classes[var].parent)
        return self.classes[var].parent

    def union(self, var1: Variable, var2: Variable) -> Variable:
        """Union by rank. Returns the new root."""
        root1 = self.find(var1)
        root2 = self.find(var2)

        if root1 == root2:
            return root1

        # Union by rank
        if self.classes[root1].rank < self.classes[root2].rank:
            self.classes[root1].parent = root2
            return root2
        elif self.classes[root1].rank > self.classes[root2].rank:
            self.classes[root2].parent = root1
            return root1
        else:
            self.classes[root2].parent = root1
            self.classes[root1].rank += 1
            return root1
```

---

## Pass 3: Signature Extraction

Build a polymorphic `DomainSignature` for the module's external interface.

```python
def extract_signature(module: Module, uf: UnionFind) -> DomainSignature:
    """
    Returns: DomainSignature
    """
    domain_params = {}  # SymbolId → DomainVar
    port_domains = {}   # SymbolId → SignatureDomain

    domain_var_counter = 0
    clock_to_var = {}  # ComponentId → DomainVar

    # Step 1: Identify domain parameters from clock ports
    for port in module.ports:
        if port.type == Clock:
            var = DomainVar(domain_var_counter)
            domain_var_counter += 1
            domain_params[port.name] = var
            clock_to_var[port.id] = var

    # Step 2: Express each port's domain in terms of parameters
    for port in module.ports:
        root = uf.find(port.id)
        assigned_domain = uf.classes[root].domain

        if assigned_domain is None:
            # Unconstrained port → free domain variable
            port_domains[port.name] = SignatureDomain.Free()

        elif assigned_domain == Clock:
            port_domains[port.name] = SignatureDomain.Concrete(Clock)

        elif assigned_domain == Reset:
            port_domains[port.name] = SignatureDomain.Concrete(Reset)

        elif assigned_domain == Async:
            port_domains[port.name] = SignatureDomain.Concrete(Async)

        elif assigned_domain == Constant:
            port_domains[port.name] = SignatureDomain.Concrete(Constant)

        elif isinstance(assigned_domain, Sync):
            # Map to domain parameter
            clock_component = assigned_domain.clock
            if clock_component in clock_to_var:
                var = clock_to_var[clock_component]
                port_domains[port.name] = SignatureDomain.Var(var)
            else:
                # Clock is internal, not a port → concrete Sync
                port_domains[port.name] = SignatureDomain.Concrete(assigned_domain)

    return DomainSignature(domain_params, port_domains)


# Signature representation (for reference, not exact)
SignatureDomain = Concrete(Domain) | Var(DomainVar) | Free()
```

---

## Top-Level Entry Point

```python
def analyze_module_domains(
    module: Module,
    child_analyses: Dict[SymbolId, DomainAnalysis],
) -> DomainAnalysis:
    """
    Top-level domain analysis for a single module.

    Args:
        module: The module to analyze
        child_analyses: Domain analyses for all child modules this module instantiates

    Returns:
        DomainAnalysis for this module
    """

    # Special case: @unsafe_clocks modules
    if module.has_annotation("unsafe_clocks"):
        return unsafe_clocks_signature(module)

    # PASS 1: Constraint Generation
    print(f"Pass 1: Generating constraints for {module.name}")
    constraints = generate_constraints(module)
    print(f"  Generated {len(constraints.variables)} variables")
    print(f"  {len(constraints.fixed)} fixed constraints")
    print(f"  {len(constraints.strict_upper_bounds)} strict upper bounds")
    print(f"  {len(constraints.upper_bounds)} upper bounds")
    print(f"  {len(constraints.equalities)} equalities")

    # PASS 2: Constraint Solving
    print(f"Pass 2: Solving constraints for {module.name}")
    result = solve_constraints(constraints)

    if result.errors:
        print(f"  Found {len(result.errors)} domain errors")
        for error in result.errors:
            print(f"    - {error}")
    else:
        print(f"  All constraints satisfied")

    # PASS 3: Signature Extraction
    print(f"Pass 3: Extracting signature for {module.name}")
    signature = extract_signature(module, result.union_find)
    print(f"  Domain parameters: {len(signature.domain_params)}")
    print(f"  Port domains: {len(signature.port_domains)}")

    # Extract component domains
    component_domains = {}
    for component in module.components:
        root = result.union_find.find(component.id)
        component_domains[component.id] = result.union_find.classes[root].domain

    return DomainAnalysis(
        moddef=module.symbol_id,
        component_domains=component_domains,
        signature=signature,
        diagnostics=result.errors,
        unsafe_clocks=False
    )


def analyze_domain(module: Module) -> DomainAnalysis:
    """
    Main entry point for domain analysis.
    Returns: DomainAnalysis
    """

    # Special case: @unsafe_clocks modules
    if module.has_annotation("unsafe_clocks"):
        return unsafe_clocks_signature(module)

    # Simplified entry point - delegates to analyze_module_domains
    return analyze_module_domains(module, {})


def unsafe_clocks_signature(module: Module) -> DomainAnalysis:
    """
    For @unsafe_clocks modules, synthesize signature from port declarations
    without analyzing the body.
    """
    # Synthesize signature based on port types and async annotations
    # Body analysis is skipped
    # ... (implementation details)
    pass


def analyze_all_modules(modules: List[Module]) -> Dict[SymbolId, DomainAnalysis]:
    """
    Analyze domains for all modules in dependency order.

    This is the true top-level function that would be called during compilation.
    Modules must be provided in topological order (leaves first, roots last).

    Args:
        modules: All modules in the program, in dependency order

    Returns:
        Map from module SymbolId to its DomainAnalysis
    """
    analyses = {}

    for module in modules:
        print(f"\n{'='*60}")
        print(f"Analyzing module: {module.name}")
        print(f"{'='*60}")

        # Collect child analyses for instantiation checking
        child_analyses = {}
        for inst in module.instantiations:
            child_symbol = inst.child_module_symbol
            if child_symbol in analyses:
                child_analyses[child_symbol] = analyses[child_symbol]

        # Analyze this module
        analysis = analyze_module_domains(module, child_analyses)
        analyses[module.symbol_id] = analysis

        # Report
        if analysis.diagnostics:
            print(f"\n❌ Module {module.name} has {len(analysis.diagnostics)} domain error(s)")
        else:
            print(f"\n✓ Module {module.name} passed domain analysis")

    return analyses
```

---

## Module Instantiation

When a child module is instantiated in a parent, apply domain substitution using the child's `DomainSignature`.

```python
def check_instantiation(
    parent_module: Module,
    child_signature: DomainSignature,
   connections: Dict[str, ComponentId],
) -> List[str]:
    """
    Check domain compatibility at instantiation site.
    connections: Dict[child_port_name, parent_signal]
    """
    bindings = {}  # DomainVar → Domain
    errors = []

    # Step 1: Establish bindings from clock connections
    for child_port, parent_signal in connections.items():
        if child_port in child_signature.domain_params:
            # This is a clock port introducing a domain parameter
            var = child_signature.domain_params[child_port]
            parent_domain = get_domain(parent_signal)

            if var in bindings:
                # Check consistency
                if bindings[var] != parent_domain:
                    errors.append(f"Inconsistent binding for {var}")
            else:
                bindings[var] = parent_domain

    # Step 2: Check data port connections with substitution
    for child_port, parent_signal in connections.items():
        if child_port not in child_signature.domain_params:
            # Data port
            child_domain = child_signature.port_domains[child_port]

            # Substitute domain variables
            substituted = substitute(child_domain, bindings)

            # Check unification with parent signal's domain
            parent_domain = get_domain(parent_signal)

            if not compatible(substituted, parent_domain):
                errors.append(f"Port {child_port}: {substituted} vs {parent_domain}")

    return errors


def substitute(sig_domain: SignatureDomain, bindings: Dict[DomainVar, Domain]) -> Option[Domain]:
    """Apply domain variable substitution."""
    if isinstance(sig_domain, SignatureDomain.Concrete):
        return sig_domain.domain
    elif isinstance(sig_domain, SignatureDomain.Var):
        if sig_domain.var in bindings:
            return bindings[sig_domain.var]
        else:
            return None  # Unbound variable
    elif isinstance(sig_domain, SignatureDomain.Free):
        return None  # Free variable
```

---

## Notes

- **Expression domains are transient**: Computed during solving but only component domains are stored in the final `DomainAnalysis`.

- **Implicit strict upper bounds**: Every non-async component gets `dom(c) < Async`. This enforces "async must be explicit."

- **Upper bound constraints from operators**: The unified binary operator rule generates `dom(operand) ≤ dom(result)`, allowing upcasting to Async when the target is async.

- **Error detection**: Unification errors are caught during union-find merges. Strict upper bound violations are checked after solving.

- **Free variables**: Literals and unconstrained ports have no initial constraints. They adopt domains through equality constraints.

- **Cross-module**: Domain parameters are introduced by clock ports. At instantiation, clock connections bind variables, and data port domains are checked after substitution.
