---
id: TC-3150
title: "Certify unbounded fair eventual verdicts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1041
    type: verifies
---
# TC-3150: Certify unbounded fair eventual verdicts

## Description

Verify that an unbounded eventual claim under a fairness set is proved with a
`cyclic-mdp-fair-eventual/v1` document that the checker accepts only after
recomputing the fair end components, and that the documents a `wait` loop
would otherwise admit are rejected.

Scope: FR-1041-AC-1 to FR-1041-AC-4, FR-1041-AC-6.

## Test Procedure

Fixture: `W` (`wait` loops at `s`; `go` reaches delivered `g` or the
undelivered terminal `f`, `1/2` each), and its variant with `drift` from `s`
to `u`, whose only action loops at `u`.

1. Settle `probability >= 0.5 [eventually holds(delivered)]` over `W` under
   `fair strong go`, under `fair weak go` and with no fairness set; run
   `fsl-probability-check` on each document.
2. Offer AC-1's document with `s` at 0 and at `1/4`, with `f` at `1/2`, with
   its fairness record removed, changed and duplicated.
3. Over the `drift` variant, settle `probability >= 0.25 […]`, then offer
   values `s` 0, `u` 0 (ranks 1, 0) and `u` at `1/2`.
4. Settle `probability <= 0.5 […]` and `probability <= 0.4 […]` over `W`
   under `fair strong go`.
5. Run AC-1 pre-cancelled and with each caller limit exhausted during
   refinement; add a constraint on an operation that is never enabled.

Tag the tests `#[trace("TC-3150", "FR-1041-AC-n")]`.

## Expected Results

- Step 1: `proved`, `ExactValue{1/2}`, values `1/2`, 1, 0, ranks 1, 0, 0,
  accepted; the same value with weak fairness; `refuted` without fairness.
- Step 2: `RankBroken` at `s` twice; `InequalityFails` at `f`; objective
  mismatch twice; `Malformed` at `objective`.
- Step 3: `refuted` with fair evidence; accepted; `InequalityFails` at `u`.
- Step 4: `proved`, `ExactValue{1/2}`; `refuted`.
- Step 5: `incomplete` with the actual limit and no truth value; the unused
  constraint is retained and changes no value.
