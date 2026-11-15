---
id: TC-1670
title: "Empty trace refusals carry a reason, message and locus"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: verifies
---
# TC-1670: Empty trace refusals carry a reason, message and locus

## Description

Verify FR-645-AC-1 and AC-2: an empty lasso loop and an empty finite trace refuse with a reason, a request pointer, a message and the clause locus.

## Test Procedure

Through `command::run`, send FR-330-AC-3's `Reaches` over a lasso with an empty loop, then over a `finite` array of zero keys.

## Expected Results

Each reports stage `evaluate`, `refusal`, `invalid_runtime_input`/`invalid-value` with `reason` `empty-lasso-loop` or `empty-finite-trace`, the `request_pointer` of the member, a non-empty `message` and the locus of the clause declaration.
