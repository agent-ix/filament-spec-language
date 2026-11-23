---
id: TC-3163
title: "Cyclic always refutation boundaries"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-202"
    type: verifies
---
# TC-3163: Cyclic always refutation boundaries

## Description

Verify FR-202-AC-37: both bound sides of a cyclic `always` claim, and the
strict comparison at equality.

## Test Procedure

Use a workload chain: `s` satisfies P and moves to `t` and to `u` with
probability `1/2` each; `t` does not satisfy P and returns to `s`; `u`
satisfies P and loops at `u`. Analyze `always holds(P)` with no window at
`>= 1/2`, `<= 1/2`, `>= 3/5` and `<= 2/5`. Offer the `>= 3/5` certificate
for `>= 1/2` and the `<= 2/5` certificate for `<= 1/2`. Offer a
rejecting-union certificate that gives `u` value 1.

## Expected Results

The value is `1/2`. `>= 1/2` and `<= 1/2` settle `proved` as before, with no
counterexample. `>= 3/5` settles `refuted` with the rejecting-union
certificate `x(s) = 1/2`, `x(t) = 1`, ranks 1 and 0. `<= 2/5` settles
`refuted` with the accepting-union certificate `x(s) = 1/2`, `x(u) = 1`,
ranks 1 and 0. At `>= 1/2` and `<= 1/2` the same certificates sit at
equality, reject `ThresholdMissed` at the initial key and yield no token.
Giving `u` value 1 in the rejecting-union certificate rejects `RankBroken`
at `u`.
