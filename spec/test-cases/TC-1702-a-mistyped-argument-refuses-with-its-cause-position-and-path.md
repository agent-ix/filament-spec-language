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

Scope: FR-650-AC-3, FR-650-AC-24, FR-650-AC-25.

## Test Procedure

Run each defective argument FR-650-AC-3 lists over the identity fixture. Run
FR-650-AC-24's function with its record argument holding `3 ms`, then `4 ms`.
Run the same function through the command over the source and from the compiled
package, and composite arguments in canonical form (FR-650-AC-25).

Tag the tests `#[trace("TC-1702", "FR-650-AC-3")]`; they live in `tests/it/run_values.rs`.
The FR-650-AC-24 tests live in `fsl-replay/src/spine/call/tests.rs`; the FR-650-AC-25
tests live there and in `fsl-command/tests/it/package_run.rs`.

## Expected Results

Each refuses at stage `call`, exit 20, no result document, with the code, cause, `details.position` 0 and `details.path` FR-650-AC-3 gives for it.

FR-650-AC-24 and AC-25: `3 ms` completes; `4 ms` refuses with that code, cause and
`details`.
