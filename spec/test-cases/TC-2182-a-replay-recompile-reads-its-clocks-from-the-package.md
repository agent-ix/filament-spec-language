---
id: TC-2182
title: "A replay recompile reads its clocks from the package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: verifies
---
# TC-2182: A replay recompile reads its clocks from the package

## Description

A replay recompile reads its clocks from the package.

Scope: FR-640-AC-8.

## Test Procedure

Inspect the replay request reader and the replay recompile (`fsl-replay/src/execute.rs`): the reader admits no `clocks` member, and the recompile builds its `TemporalClockSelection`s from the proving package's `temporal_bindings` rows.

## Expected Results

- A replay request with `clocks` is refused `invalid-request` naming `clocks`.
- The recompile's selections equal the rows' `clock_name` and `clock`, so the recompiled package identity equals the original's.
