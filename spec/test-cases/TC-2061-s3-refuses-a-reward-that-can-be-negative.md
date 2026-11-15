---
id: TC-2061
title: "S3 refuses a reward that can be negative"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-185
    type: verifies
---
# TC-2061: S3 refuses a reward that can be negative

## Description

Verify that S3, not model admission, refuses a reward that is negative for some argument vector, within the S3 work budget.

Scope: FR-185-AC-11.

## Test Procedure

1. Check `Service` with `reward refund on Server::request = n - 3` on an operation with parameter `n: Int[0, 5]`, and confirm S4 is not reached.
2. Check the same with `reward refund on Server::request = n`.
3. Check a reward over a random parameter whose support holds only non-negative results while the parameter's declared type would admit negative ones.
4. Check step 1 with `s3.work_units` below the number of argument vectors.

Tag the tests `#[trace("TC-2061", "FR-185-AC-11")]`.

## Expected Results

- Step 1: `invalid_probabilistic_model`/`negative-reward` naming `Server::request`, `refund` and `n = 0`; no package.
- Step 2: checks.
- Step 3: checks, the reward being evaluated over the support only.
- Step 4: `stage_limit_exceeded`/`work-budget-exceeded` naming `s3.work_units` at the reward; no `negative-reward`.
