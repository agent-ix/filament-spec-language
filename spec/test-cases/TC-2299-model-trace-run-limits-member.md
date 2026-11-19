---
id: TC-2299
title: "A model-trace clause run writes the limits it ran under"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: verifies
---
# TC-2299: A model-trace clause run writes the limits it ran under

## Description

Verify that a clause run over a `model` trace writes its effective
`max_candidates` bound, default included, in the result's `limits` member,
and that a run over any other trace writes none. Scope: FR-641-AC-8.

## Test Procedure

1. Through `command::run`, run FR-641-AC-7's `model` trace with no `max_candidates`, and again with `max_candidates` 5.
2. Run a `finite` sample trace.

## Expected Results

- Step 1: `limits` is `{"expansion.max_candidates": "1000000"}`, then `{"expansion.max_candidates": "5"}`; the two documents are otherwise equal.
- Step 2: no `limits` member.

Test: `tests/it/record_sample_counterexample.rs`
`a_model_trace_run_writes_its_limits`.
