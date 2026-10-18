# Queries
Vibeslop documentation for the queries found in the Virdant `Db` database.

The `Db` type is the Virdant Database,
an incremental query cache used to build up information about a design for checking and elaboration.

The Db struct in Virdant implements an incremental query database
that serves as the caching layer for the entire Virdant compiler pipeline.
At its core, it's a memoization system designed to avoid redundant computation
when source files change during development.
Instead of recompiling everything from scratch,
the database tracks dependencies between queries and
only recomputes what's actually affected by a change.

The architecture centers around two types of queries:
- inputs (source code and package metadata)
- and derived queries (everything else—parsing, type checking, symbol tables, elaboration, etc.)

When you modify a source file, only that file's Source input changes,
which increments a revision counter and invalidates the dirty-check cache.
The system then uses dependency tracking to determine which derived queries depend on the changed input,
directly or transitively, marking them as "dirty."
Clean queries return their cached results immediately,
while dirty queries trigger rebuilds that automatically track their own dependencies for future invalidation cycles.

The queries! macro provides the declarative interface—developers simply list query signatures
like `TypeCheck(symbol_id: SymbolId) -> Arc<Vec<Diagnostic>>`
and the macro machinery generates the corresponding enums, dispatch logic, and typed getter methods.

The `Builder` struct tracks dependencies as queries execute,
allowing the database to construct a precise dependency graph.
This design makes it straightforward to add new compiler phases:
just declare the query, implement its build function, and the incremental caching works automatically.
The result is a responsive compiler that scales well to large codebases by doing the minimum work necessary after each edit.

