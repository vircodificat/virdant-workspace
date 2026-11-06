# Plan: prop_cost

Date:   2026-11-04
Status: In Planning

Static per-signal propagation-cost model for elaborated Virdant designs:
a gate-delay table plus a longest-path ("pole") computation, exposed as a
`Db` query. Spawned out of `plans/gates_and_prop` — see `PLANv2.md` there
for the consumer. Source idea: `plans/gates_and_prop/CONCEPT.md`.
