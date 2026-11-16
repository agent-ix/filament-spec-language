---
id: TC-2627
title: "Handoff execution points are an object keyed by state-clause name"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: verifies
---
# TC-2627: Handoff execution points are an object keyed by state-clause name

## Description

Verify that the `handoff` request member `execution_points` is an object keyed by authored state-clause name, that absent means `{}`, that the sidecar records the object the request gave, and that an unknown clause name or an unknown point form refuses at stage `request`.

Scope: FR-665-AC-4.

## Test Procedure

1. Run `fsl_command::command::compile` over a handoff request for `tests/fixtures/parent.native` with `execution_points` `{"CanAttempt": {"kind": "pre", "operation": "attempt"}}`; read the written `selection.json`.
2. Run the same request with `execution_points` absent, into a new directory; read `selection.json`.
3. Run the request with `execution_points` `{"Missing": {"kind": "pre", "operation": "attempt"}}`, then with `{"CanAttempt": {"kind": "loop", "name": "x"}}`; record category, stage, pointer, exit code and whether the directory exists.

Tag the tests `#[trace("TC-2627", "FR-665-AC-4")]`.

## Expected Results

- Step 1: exit 0, and `selection.json`'s execution points are exactly `{"CanAttempt": {"kind": "pre", "operation": "attempt"}}`.
- Step 2: exit 0, and `selection.json`'s execution points are `{}`.
- Step 3: each refuses `invalid-request` at stage `request`, exit 20, at `/handoff/execution_points/Missing` and `/handoff/execution_points/CanAttempt`; neither writes a directory.
