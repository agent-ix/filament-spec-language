---
id: TC-1905
title: "A Clock parameter of a function"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: verifies
---
# TC-1905: A Clock parameter of a function

## Description

Verify FR-230-AC-64: a `Clock`-typed function or predicate parameter is admitted under the temporal layer and gated without it.

## Test Procedure

Check the declarations of FR-230-AC-64 under `filament.temporal.complete/v1` and under `filament.value.complete/v1` alone. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

Under the temporal layer both check and nothing is refused `not-yet-implemented`; under the value layer alone each refuses `unsupported_construct`/`declaration-form` from the facet gate.
