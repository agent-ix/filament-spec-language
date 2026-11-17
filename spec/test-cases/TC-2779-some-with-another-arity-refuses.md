---
id: TC-2779
title: "Some with another argument count refuses with exit 20"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-983
    type: verifies
---
# TC-2779: Some with another argument count refuses with exit 20

## Description

Some with another argument count refuses with exit 20.

Scope: FR-983-AC-3.

## Test Procedure

Check `Some(b, b)` and `Some()` and read the `check` outcome document. Test: `fsl-semantics/tests/it/some_constructor.rs`.

## Expected Results

- `ill_typed`/`type-mismatch` at the call, category `refusal`, exit 20.
