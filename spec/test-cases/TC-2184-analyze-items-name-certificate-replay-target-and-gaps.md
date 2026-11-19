---
id: TC-2184
title: "Analyze items name their certificate, replay target and aggregate gaps"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-2184: Analyze items name their certificate, replay target and aggregate gaps

## Description

Analyze items name their certificate, replay target and aggregate gaps.

Scope: FR-621-AC-8.

## Test Procedure

Analyze the Counter subject with required items `Bounded` and `BelowThree`. Test: `fsl-analyze/tests/it/analyze_request_wire.rs` `analyze_items_name_certificate_replay_target_and_gaps`.

Tag the tests `#[trace("TC-2184", "FR-621-AC-8")]`.

## Expected Results

- `Bounded`: `certificate.identity` has exactly `node_id`, `model`, `ordinal`.
- `BelowThree`: `replay.target` is `{"state_clause": Q}`, `Q` ending in `BelowThree`.
- `aggregate`: `incomplete`, one gap `{request_index, reason: not-proved, result: refuted}`.
