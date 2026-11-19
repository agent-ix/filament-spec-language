---
id: TC-2295
title: "A record-sample counterexample names its positions by index"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: verifies
---
# TC-2295: A record-sample counterexample names its positions by index

## Description

Verify the counterexample step form of a record-sample trace: one
`{"sample": I}` step per position, `I` its index in the request trace.
Scope: FR-330-AC-18.

## Test Procedure

1. Through `command::run`, run `Never` (`always holds(r.level < 2)`, infinite-trace) over a `lasso` with prefix samples `level` 0, 0 and loop sample `level` 2.
2. Run `E` (event-position) over a `finite` sample trace that violates it.

## Expected Results

- Step 1: `violated`; `prefix` `[{"sample": "0"}, {"sample": "1"}]`, `loop` `[{"sample": "2"}]`; no step carries `document` or a sample value.
- Step 2: `violated` with `sample` steps of the same form.

Test: `tests/it/record_sample_counterexample.rs`
`a_record_sample_counterexample_names_positions_by_index`.
