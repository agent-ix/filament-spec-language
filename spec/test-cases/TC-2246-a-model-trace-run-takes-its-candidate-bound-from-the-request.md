---
id: TC-2246
title: "A model trace run takes its candidate bound from the request"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: verifies
---
# TC-2246: A model trace run takes its candidate bound from the request

## Description

Verify the `model` trace's `max_candidates` member and the outcome when the bound is reached. Scope: FR-641-AC-7.

## Test Procedure

1. Through `command::run`, run FR-330-AC-16's `Model` trace as a `model` member with no `max_candidates`.
2. Run it with `max_candidates` 1 over a step whose reconstruction expands more than one candidate.
3. Run it with `max_candidates` 0, and with `max_candidates` on a `lasso` member.
4. Compare the result identities of two runs that differ only in `max_candidates`.

## Expected Results

- Step 1: FR-330-AC-16's outcome.
- Step 2: `incomplete`, exit 22, `LimitReached{limit: max_candidates, value: 1, setting: expansion.max_candidates}`, no truth and no counterexample.
- Step 3: `invalid-request` at stage `request`, exit 20, naming the member.
- Step 4: the identities differ.
