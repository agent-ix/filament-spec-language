---
id: TC-2383
title: "A monitor document names S6a, or S4 by its refusal"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
---
# TC-2383: A monitor document names S6a, or S4 by its refusal

## Description

A monitor document names S6a, or S4 by its refusal.

Scope: FR-286-AC-20.

## Test Procedure

Build the document of a settled `MonitorOutcome` and of the `UnknownClause` and `Observation` refusals. Test: `fsl-analyze/tests/it/analyze_last_stage.rs` `monitor_documents_name_their_stage`.

Tag the tests `#[trace("TC-2383", "FR-286-AC-20")]`.

## Expected Results

- `last_stage` is `S6a`, `S4` and `S6a` respectively.
