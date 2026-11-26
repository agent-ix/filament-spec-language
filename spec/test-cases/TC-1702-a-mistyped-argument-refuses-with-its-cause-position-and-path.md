---
id: TC-1702
title: "A mistyped argument refuses with its cause, position and path"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-1702: A mistyped argument refuses with its cause, position and path

## Description

A mistyped argument refuses with its cause, position and path.

Scope: FR-650-AC-3, FR-650-AC-24.

## Test Procedure

Run each defective argument FR-650-AC-3 lists over the identity fixture. Run
FR-650-AC-24's function with its record argument holding `3 ms`, then `4 ms`.

Tag the tests `#[trace("TC-1702", "FR-650-AC-3")]`; they live in `tests/it/run_values.rs`.
The FR-650-AC-24 test lives in `fsl-replay/src/spine/call/tests.rs`.

## Expected Results

Each refuses at stage `call`, exit 20, no result document, with the code, cause, `details.position` 0 and `details.path` FR-650-AC-3 gives for it.

FR-650-AC-24: `3 ms` completes; `4 ms` refuses with that code, cause and
`details`.
