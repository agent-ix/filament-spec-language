---
id: TC-2663
title: "A presence quantifier is emitted over its collection's slot domain"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-953
    type: verifies
---
# TC-2663: A presence quantifier is emitted over its collection's slot domain

## Description

Verify that S4 writes a presence-only quantifier's first operand as the
slot-domain node, with the derived path naming the clock place by operand
segment. Scope: FR-953-AC-1, FR-953-AC-2.

## Test Procedure

1. Emit FR-930-AC-1's and AC-2's clauses and read the quantifier's operand 0, its node id and the binder type.
2. Evaluate the `forall` and `exists` forms over an empty `q` and over a `q` of size 2 with slot 1 absent; emit two quantifiers over `q` and one over `r`.

Test: `fsl-package/src/emit/tests/timed_clauses.rs`
`a_presence_quantifier_is_emitted_over_its_slot_domain`.

## Expected Results

- Step 1: one `filament.op.option.slots` node shared by every clause, with FR-953-AC-1's path, a `Set<Integer>` result bounded `[0, 2]`, and an `Integer` binder; I2 admits it.
- Step 2: `true`/`false` when empty, `false`/`true` with slot 1 absent; two slot-domain nodes.
