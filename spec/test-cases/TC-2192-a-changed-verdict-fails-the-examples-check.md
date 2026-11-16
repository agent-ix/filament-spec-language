---
id: TC-2192
title: "A changed verdict fails the examples check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2192: A changed verdict fails the examples check

## Description

A recorded verdict this checkout's `fsl` does not give fails
`make examples-check`.

Scope: FR-765-AC-3.

## Test Procedure

From `make examples-check`, copy the examples checkout without `.git` and
`target` under `CARGO_TARGET_TMPDIR`, append `-edited` to the recorded
`category` of the first running case of the first system that has one, and
run `make examples-check FSL_EXAMPLES_DIR=<copy> EXAMPLES_CHECK_TESTS=`.

Tag the test `#[trace("FR-765-AC-3", "TC-2192")]`.

## Expected Results

- A nonzero exit.
- The output contains `<system>/<case>: outcome differs from
  expected/<case>.json in category` and `examples-check: the driver examples
  gate failed`.
