---
id: TC-3027
title: "S3 evaluates a reward's sign through the reward evaluator port"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: verifies
---
# TC-3027: S3 evaluates a reward's sign through the reward evaluator port

## Description

Verify that S3 refuses a reward that can be negative by evaluating it
through the layer-3 `RewardEvaluator` port with `fsl-eval`'s S6a
implementation, and that the port's results decide the refusal. Scope:
FR-185-AC-13.

## Test Procedure

1. Check `Service` with `reward refund on Server::attempt = d - 3`, `d` over
   the support `{1, 3, 12}`, through the spine; repeat with `d - 1`.
2. Check the same unit at S3 with a recording `RewardEvaluator` that returns
   a non-negative value at the first vector and a negative value at the
   second; repeat with one that reports `Undefined` at the first vector.

## Expected Results

- Step 1: `invalid_probabilistic_model`/`negative-reward` at S3 naming
  `Server::attempt`, `refund` and `d = 1`; `d - 1` checks.
- Step 2: the evaluator is called at the first two vectors in canonical order
  and no further, and S3 refuses `negative-reward` at the second; with
  `Undefined`, S3 refuses with that record, not `negative-reward`, and
  returns no checked unit.
