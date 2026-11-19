---
id: TC-2773
title: "A typed size query builds a Size form and malformed spellings refuse at S1"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-981
    type: verifies
---
# TC-2773: A typed size query builds a Size form and malformed spellings refuse at S1

## Description

A typed size query builds a Size form and malformed spellings refuse at S1.

Scope: FR-981-AC-1, FR-981-AC-2.

## Test Procedure

Build FR-981-AC-1's function and its untyped variant through S2, and parse `size<>(c)` and `size<Int[0, 4]>()`. Test: `fsl-forms/tests/it/typed_size.rs`.

## Expected Results

- One `Size` form with type argument `Int[0, 4]`, one with none, and no `UnrepresentedConstruct`.
- `invalid_syntax`/`unexpected-token` at `>` and at `)`.
