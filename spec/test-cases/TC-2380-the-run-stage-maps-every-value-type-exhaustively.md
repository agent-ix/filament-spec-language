---
id: TC-2380
title: "The run stage maps every value type exhaustively"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-650
    type: verifies
---
# TC-2380: The run stage maps every value type exhaustively

## Description

The run stage maps every value type exhaustively.

Scope: FR-650-AC-16.

## Test Procedure

Inspect the run stage's mapping from `ValueType` to the run value form in `fsl-replay/src/spine/run_value.rs`.

## Expected Results

- The mapping is one match over `ValueType` with no wildcard arm.
- Only a population held below the top level yields the exit-21 refusal, with `details` `{"position": P}` or `{"function": F}`.
