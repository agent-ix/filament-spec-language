---
id: TC-2965
title: "A finite-replay item decides required temporal clauses and refutes at an undefined step"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-941
    type: verifies
---
# TC-2965: A finite-replay item decides required temporal clauses and refutes at an undefined step

## Description

Verify that a `finite-replay` item over a protocol clause that requires
temporal clauses evaluates each one over the replayed behaviour and settles
by FR-941's table, and that a protocol expression undefined at a replayed
state refutes the item at that state, before a later stale step. Scope:
FR-941-AC-4 and FR-941-AC-5.

## Test Procedure

1. Over the order fixture with `requires temporal R;`, replay the trace
   `Placed`, `Got`, `Ship` with `R` each of `eventually holds(k.r1 = k.r1)`,
   `always holds(k.r1 != k.r1)`, `always holds(k.r1 = k.r1)` and
   `always holds(1 mod (k.r1 - k.r1) = 0)`; then with `P` pending and `R`
   violated, required in that order.
2. Record the trace from the fixture, then replay it against the fixture
   whose `Got` constraint is `{ 1 mod (k.r1 - k.r1) = 0 }`; again with step
   2's post-state key changed; again with the trace beginning with `Got`.

Test: `fsl-analyze/tests/it/protocol_composition.rs`.

## Expected Results

- Step 1: `proved` (`holds`, 3 steps); `refuted` (`violated`, position 0,
  reproduced); `inconclusive` (`pending`, exit 0); `refuted`
  (`undefined-evaluation`, `division-by-zero`, position 0); `R`'s `violated`.
- Step 2: `refuted`, `undefined-evaluation` at position 1 with locus
  `protocol` and a one-step counterexample that replays; still `refuted` at
  position 1; `declined` `invalid-value` at position 0.
- Assertions compare the outcome document's typed members, never message text.
