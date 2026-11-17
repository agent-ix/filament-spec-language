---
id: TC-2835
title: "A reference literal resolves fixed or created-object at subject binding"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1000
    type: verifies
---
# TC-2835: A reference literal resolves fixed or created-object at subject binding

## Description

Verify the binding-time resolution table, its two refusals, the resolved
footprint and evaluation. Scope: FR-1000-AC-4 and FR-1000-AC-5.

Catches a resolver that admits a created-object literal in a barred slot, an
unknown key that reaches exploration, and a created-object read whose
footprint omits population membership.

## Test Procedure

1. Analyze over the Counter subject with initial state `{a}` and universe
   `{a, b}` a clause reading `M::Counter#"a"`, then one reading
   `M::Counter#"zz"`.
2. Analyze a protocol whose `choice` guard compares the received `got.target` to `M::Counter#"b"`,
   then to `M::Counter#"a"`.
3. Analyze FR-1000-AC-5's two clauses.

## Expected Results

- Step 1: decided; then `missing_declaration`/`missing-name` at the literal
  naming `Counter` and `7a7a`, with no state explored.
- Step 2: `ill_typed`/`operator-ineligible` at the literal; then admitted.
- Step 3: the `b` clause is undefined, reason `absent-key`, with
  `counters` membership in its footprint; the `a` clause is decided with no
  membership location for the literal.
