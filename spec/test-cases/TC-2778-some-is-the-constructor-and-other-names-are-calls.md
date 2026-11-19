---
id: TC-2778
title: "Some is the option constructor everywhere and other call names stay ordinary calls"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-983
    type: verifies
---
# TC-2778: Some is the option constructor everywhere and other call names stay ordinary calls

## Description

Some is the option constructor everywhere and other call names stay ordinary calls.

Scope: FR-983-AC-1, FR-983-AC-2.

## Test Procedure

Check the units of FR-983-AC-1 and FR-983-AC-2. Test: `fsl-semantics/tests/it/some_constructor.rs`.

## Expected Results

- `Some` checks in a function body and a state clause.
- `some` and `Something` resolve to the declared functions; `Somex` refuses as an unresolved call.
