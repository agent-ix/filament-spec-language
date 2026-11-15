---
id: TC-1665
title: "Timed trace refusals"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: verifies
---
# TC-1665: Timed trace refusals

## Description

Verify FR-642-AC-5: a timed clause over untimed snapshots, decreasing stamps and a non-positive loop period refuse at stage `admit` with FR-645's fields.

## Test Procedure

Run a timed clause over a model subject over `finite` snapshots; run `Ack` over stamps 0, 5, 3; run a timed lasso with `period` 0.

## Expected Results

Each refuses stage `admit`, `invalid_runtime_input`/`invalid-value`, with `reason` `trace-kind-mismatch` (`expected` `["timed"]`, `supplied` `finite`), `timed-stamps-decrease` (position 2) and `timed-period-too-short`.
