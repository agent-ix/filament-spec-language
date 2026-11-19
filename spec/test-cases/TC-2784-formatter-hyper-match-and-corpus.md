---
id: TC-2784
title: "The formatter keeps a hyper match block's braces together and formats the whole corpus"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-003
    type: verifies
---
# TC-2784: The formatter keeps a hyper match block's braces together and formats the whole corpus

## Description

The formatter keeps a hyper match block's braces together and formats the whole corpus.

Scope: FR-003-AC-13, FR-003-AC-14.

## Test Procedure

Format FR-003-AC-13's hyper clause, then every unit of the complete-grammar corpus. Test: `fsl-cst/tests/it/formatter_style.rs`.

## Expected Results

- `} {` on one line and `v.l @ a` spaced.
- Every corpus unit formats idempotently with an equal token sequence and production tree.
