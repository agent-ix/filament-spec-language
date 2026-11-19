---
id: TC-2190
title: "make ci runs the examples check"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-765
    type: verifies
---
# TC-2190: make ci runs the examples check

## Description

`make ci` reaches the examples check through its aggregate graph.

Scope: FR-765-AC-1.

## Test Procedure

Read the Makefile's `CI_CHECKS` line, then run `make -n ci-internal` in the
workspace.

Tag the test `#[trace("FR-765-AC-1", "TC-2190")]`.

## Expected Results

- `CI_CHECKS` names `examples-check`.
- The dry run prints a line ending in ` examples-check` and a line ending in
  `--package xtask --test examples_check`.
