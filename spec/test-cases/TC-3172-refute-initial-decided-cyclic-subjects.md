---
id: TC-3172
title: "Refute initial-decided cyclic subjects"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3172: Refute initial-decided cyclic subjects

## Description

Verify FR-202-AC-42 and the EN-5 clause of FR-202-AC-21: a cyclic subject
whose initial state is already decided is refuted through the subsystem
entry, not settled `unsupported`.

## Test Procedure

Under a workload, use two chains. Until: initial `b` satisfies B and not A
and moves to `s`, which satisfies A and not B and returns to `b`; analyze
`probability <= 1/2 [holds(A) until holds(B)]`. Always: initial `f` does
not satisfy P and moves to `p`, which satisfies P and returns to `f`;
analyze `probability >= 1/2 [always holds(P)]`. Call the path-set entry
directly on each, then run EN-5 and replay its counterexample.

## Expected Results

The until value is 1 and the always value is 0. The path-set entry reports
each cycle without a token. EN-5 starts no path-set search and emits a
`lower` certificate: accepting union `{b}` with `x(b) = 1`, rank 0, and
rejecting union `{f}` with `x(f) = 1`, rank 0. Replay checks `1 > 1/2` and
both items settle `refuted`, not `unsupported`.
