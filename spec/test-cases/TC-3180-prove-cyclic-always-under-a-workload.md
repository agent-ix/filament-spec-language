---
id: TC-3180
title: "Prove cyclic always under a workload"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3180: Prove cyclic always under a workload

## Description

Verify FR-201-AC-58: the cyclic DTMC state-predicate profile certifies both
bound sides of an unwindowed `always` claim through the violated quantity,
with the checker's own component classification.

## Test Procedure

Use a workload chain: `s` satisfies P and moves to `t` and to `u` with
probability `1/2` each; `t` does not satisfy P and returns to `s`; `u`
satisfies P and loops at `u`. Check `always holds(P)` with no window at
`>= 1/2` with targets `{t}`, values `s = 1/2`, `t = 1`, `u = 0`, ranks 1, 0,
0, and at `<= 1/2` with targets `{u}`, values `s = 1/2`, `t = 0`, `u = 1`,
ranks 1, 0, 0. Offer the same documents at `>= 3/5` and `<= 2/5`. Offer
values `s = 1`, `t = 1`, `u = 1` under targets `{t}`. Offer, for `<= 2/5`,
a document with empty targets and every value 0.

## Expected Results

Both documents are accepted at equality (`1/2 <= 1 − 1/2`, `1/2 <= 1/2`).
At `>= 3/5` and `<= 2/5` they reject `ThresholdMissed` at `s`. The all-one
values reject `InequalityFails` at `u`. The empty-target document refuses
objective mismatch before any row check.
