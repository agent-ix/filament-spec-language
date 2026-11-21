---
id: TC-3152
title: "Certify finite-horizon measure verdicts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1043
    type: verifies
---
# TC-3152: Certify finite-horizon measure verdicts

## Description

Verify that comparison events, quantiles and means over a bounded measure are
certified in `finite-horizon-measure/v1`, the checker computing every decided
state's payoff from its key.

Scope: FR-1043-AC-1 to FR-1043-AC-3, FR-1043-AC-5, FR-1043-AC-7.

## Test Procedure

Fixture: `Retry` (`try` delivers with `1/2`, otherwise stays; delivered is
terminal) and its variant with `fast` (`1/2`) and `slow` (`1/4`); the measure
`M = steps from holds(true) until holds(delivered) within 4`.

1. Under the workload settle `quantile 0.5 of (M) <= 2` and `quantile 0.8
   of (M) <= 2`; check the `ratio` certificate and the same with a rank
   changed.
2. Under every scheduler over the variant settle `quantile 0.4 of (M) <= 2`
   and `quantile 0.5 of (M) <= 2`; offer the level-0.4 document with its
   initial value replaced by `9/40`.
3. Settle `quantile 0.6 of (M) >= 2` and `quantile 0.5 of (M) >= 2` over the
   variant under every scheduler.
4. Under the workload settle `mean of fraction holds(delivered) over [0, 2]`
   at `rational(2,5)` and `rational(1,2)`, and `probability >= 0.7 [M <= 2]`.
5. Change each `measure` member and `payoff` of step 1's numerator; alter one
   key's accumulator; offer a `shifted` value below `−level`; run step 1
   pre-cancelled and with each caller limit exhausted.

Tag the tests `#[trace("TC-3152", "FR-1043-AC-n")]`.

## Expected Results

- Step 1: `proved`, `ExactValue{3/4}`, numerator `3/4`, `1/2`, 0, 0, ranks
  4, 3, 2, 1, denominator 1, accepted; `refuted`; `RankBroken`.
- Step 2: `proved`, values `3/80`, `−3/20`, `−2/5`, `−2/5`; `refuted`;
  `InequalityFails` at the initial key.
- Step 3: `proved` with initial value `−1/10`; `refuted`.
- Step 4: `proved`, `ExactValue{5/12}`; `refuted`; `proved`,
  `ExactValue{3/4}`.
- Step 5: objective mismatch for each; coverage failure; `Malformed` at
  `values`; `incomplete` with the actual limit.
