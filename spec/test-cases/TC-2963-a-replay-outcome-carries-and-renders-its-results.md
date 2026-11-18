---
id: TC-2963
title: "A replay outcome carries and renders its results"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-865
    type: verifies
---
# TC-2963: A replay outcome carries and renders its results

## Description

Verify that a replay's outcome document carries the `replayed` result with
each `results` row's arm and settlement, and that its text form renders one
`result` line per row. Scope: FR-286-AC-21, FR-865-AC-6.

## Test Procedure

1. Replay `entry` through `command::run`; compare the outcome document's
   `result` with the `results` document's rows.
2. Render the outcome as text.
3. Replay FR-862-AC-1's refused request and render it.

Test: `tests/it/replay_request_file.rs`.

## Expected Results

- Step 1: `{"kind": "replayed", "results": [R]}`, `R` equal to the `results` row
  (`input`, `reproduced-without-witness`).
- Step 2: the header `replay: success (exit 0) at S8`, then
  `result 0: input reproduced-without-witness`.
- Step 3: `result` `null`; refusal header and diagnostic, no `result` line.
