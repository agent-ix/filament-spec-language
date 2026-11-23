---
id: TC-3181
title: "Prove cyclic until under a workload"
type: TC
relationships:
  - target: "ix://agent-ix/filament-spec-language/FR-201"
    type: verifies
---
# TC-3181: Prove cyclic until under a workload

## Description

Verify FR-201-AC-59: a bottom component with no decided state is rejecting
for `until`, the certificate graph stops at the violating union, and a
producer-supplied union is not evidence.

## Test Procedure

Workload chain: `s` satisfies A and not B and moves to `b` with `1/4`, to
`n` with `1/4` and to `c` with `1/2`; `b` satisfies B and not A, `n`
satisfies neither, and each returns to `s`; `c` and `d` satisfy A and not B
and alternate deterministically. Check `holds(A) until holds(B)` at
`>= 1/4` with targets `{c, n}`, values `s = 3/4`, `b = 0`, `c = 1`,
`n = 1`, ranks 1, 0, 0, 0, and at `<= 1/4` with targets `{b}`, values
`s = 1/4`, `b = 1`, `n = c = d = 0`, rank 1 at `s` and 0 elsewhere. Offer
the valid documents at `>= 3/10` and `<= 1/5`. In a separate check, offer the
`>= 1/4` document with rank 2 at `s` at `>= 1/4`. Offer for `>= 1/2` a
document with targets `{n}` and values `s = 1/4`, `n = 1`, `b = c = d = 0`.

## Expected Results

Both documents are accepted; the first has no row for `d`. The valid
documents at `>= 3/10` and `<= 1/5` reject `ThresholdMissed` at `s`. The
rank-2 document at `>= 1/4` rejects `RankBroken` at `s`. The `{n}` document refuses objective mismatch because the checker
derives `{c, n}`, although its initial value would pass the threshold.
