---
id: TC-2381
title: "A settled analyze document names the greatest stage its items entered"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-2381: A settled analyze document names the greatest stage its items entered

## Description

A settled analyze document names the greatest stage its items entered.

Scope: FR-621-AC-11.

## Test Procedure

Analyze the Counter subject with a proved item, a proved and a replayed refuted item, an item stopped at `max_states` 2, and an unknown-claim item alone. Test: `fsl-analyze/tests/it/analyze_last_stage.rs` `settled_documents_name_their_greatest_stage`.

Tag the tests `#[trace("TC-2381", "FR-621-AC-11")]`.

## Expected Results

- `last_stage` is `S6c`, `S8`, `S6c` and `S4` respectively.
