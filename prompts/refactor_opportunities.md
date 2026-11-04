# Refactoring Opportunities Prompt

Your job: identify the highest-value refactoring opportunities in this codebase,
judged solely by HUMAN comprehensibility — how quickly a person can read,
understand, and safely change the code. Ignore style/performance concerns
unless they also reduce reader friction.

Scan for:

- Functions exceeding ~40 lines, or with many decision points (if/else, match arms, loops)
- Nesting deeper than 3 levels (if/loop/try/match)
- Parameter lists longer than 4, especially positional booleans or bare primitives
- Long or compound boolean conditions not extracted into named variables/predicates
- Vague, abbreviated, single-letter, or behavior-misleading names
- The same concept named or styled differently across modules
- Magic numbers, strings, timeouts, or limits used inline without named constants
- Duplicated or near-duplicated logic, including copies with drift
- The same conceptual operation implemented differently in different places
- Mixed abstraction levels: high-level orchestration and low-level detail in one function
- Functions that do more than their name implies (hidden side effects, I/O, argument mutation)
- Hidden inputs/outputs: globals, singletons, ambient config, environment reads
- Temporal coupling: call-order requirements not enforced by the API
- Dead code: unused functions/variables/parameters, unreachable branches
- Comments that restate the code or are outdated; missing "why" comments on non-obvious logic
- God files/classes/modules: many responsibilities, high fan-in or fan-out
- Tracing one flow requires holding many items in mind or jumping across many files
- Long call chains or nested calls that must be read inside-out
- Double negatives or negated conditions where a positive form reads better
- Unstructured data shapes: bare tuples, dicts-as-records, `Any`, missing type annotations
- Call-site values whose meaning can't be read without opening the definition
- Return shapes that force reading the implementation (raw tuples, mixed types)
- Inconsistent handling of the same failure class
- Deep or wide inheritance hierarchies; non-obvious dynamic dispatch
- Logic fragmented across callbacks/lambdas/closures where a plain sequence would read linearly
- Comments that are out of date
- Comments that look like they were written by an LLM

Rank findings by (reading friction removed × how many people touch that code) ÷ cost and
risk of the refactor. Report only the top opportunities. Skip anything a linter or
auto-formatter already catches.

Use subagents to perform this task.
