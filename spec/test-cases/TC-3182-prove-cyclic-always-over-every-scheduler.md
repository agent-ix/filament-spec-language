---
id: TC-3182
title: "Prove cyclic always over every scheduler"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3182: Prove cyclic always over every scheduler

## Description

Verify FR-201-AC-60: the cyclic MDP state-predicate profile certifies the
maximum violated quantity on both bound sides over every reconstructed
action.

## Test Procedure

`s` satisfies P with actions `a`, to terminal P state `g` with `3/4` and to
`f` with `1/4`, and `b`, to `s` with `1/2`, `f` with `1/4` and `g` with
`1/4`; `f` does not satisfy P and returns to `s`. Over every scheduler check
`always holds(P)` at `>= 1/2` with optimization `maximum`, targets `{f}`,
values `s = 1/2`, `f = 1`, `g = 0`, ranks 1, 0, 0, and at `<= 3/4` with
targets `{g}`, values `s = 3/4`, `g = 1`, `f = 0`, ranks 1, 0, 0. Offer them
at `>= 3/5` and `<= 7/10`; offer `s = 1/4` under targets `{f}` for `>= 3/4`;
offer rank 0 at `s` in the second; offer optimization `workload`.

## Expected Results

Both are accepted at equality. `>= 3/5` and `<= 7/10` reject
`ThresholdMissed`. `s = 1/4` rejects `InequalityFails` at `s` because
`Q_b = 3/8`. Rank 0 at `s` rejects `RankBroken` at `s`. `workload` refuses
objective mismatch.
