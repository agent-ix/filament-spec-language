---
id: TC-3186
title: "Prove cyclic always over a single-distribution every scheduler"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3186: Prove cyclic always over a single-distribution every scheduler

## Description

Verify FR-201-AC-64: the cyclic DTMC state-predicate profile uses optimization
`maximum` on both bound sides over every scheduler, and an eligible DTMC
producer keeps the DTMC profile.

## Test Procedure

Use FR-201-AC-58's chain over every scheduler, with one semantic enabled
distribution per live state. Check `always holds(P)` at `>= 1/2` with
optimization `maximum`, targets `{t}`, values `s = 1/2`, `t = 1`, `u = 0`
and ranks 1, 0, 0, and at `<= 1/2` with optimization `maximum`, targets
`{u}`, values `s = 1/2`, `t = 0`, `u = 1` and ranks 1, 0, 0. Repeat each
with optimization `minimum` and `workload`. Analyze both claims with EN-5.

## Expected Results

Both `maximum` documents are accepted. Each `minimum` or `workload` variant
refuses objective mismatch. EN-5 settles both claims `proved` and emits
`cyclic-dtmc-state-predicate/v1` documents, not
`cyclic-mdp-state-predicate/v1`.
