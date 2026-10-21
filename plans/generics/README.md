# Generics

Date: 2026-10-20
Status: Completed

Full generics for Virdant: parameterized socket defs, type defs
(structs/enums/unions), and mod defs, with a single substitution core
rather than per-item special cases. The motivating example is a
parameterized handshake socket (`Pipe[T]`, see `plans/pipe_socket/CONCEPT.md`),
but the goal is general machinery — `Valid[T]` becomes a spellable
generic enum instead of a hardcoded builtin, and modules can be
parameterized the same way types are.

Today, "generics" in Virdant means hardcoded builtin types: `Word[n]`
and `Valid[T]` are name-matched special cases inside `Type` and the
type-resolver. This plan replaces that with real type parameters and a
monomorphizing (substitute-early) implementation strategy chosen to fit
the existing hand-rolled query cache and per-instance elaboration model.

See also:

* `plans/pipe_socket/CONCEPT.md` — the concrete proposal that motivates generic sockets
* `plans/PARAMETERS.md` — Verilog module parameters via annotations (a separate,
  non-type mechanism for `ext mod` cells; orthogonal to this plan)
* `GRAMMAR.txt` — existing `Generics := "[" Ident ":" Kind "]"` production
