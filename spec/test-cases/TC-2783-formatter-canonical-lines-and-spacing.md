---
id: TC-2783
title: "The formatter prints the canonical line layout and spacing"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-003
    type: verifies
---
# TC-2783: The formatter prints the canonical line layout and spacing

## Description

The formatter prints the canonical line layout and spacing.

Scope: FR-003-AC-11, FR-003-AC-12.

## Test Procedure

Format the protocol and the spacing inputs of FR-003-AC-11 and FR-003-AC-12. Test: `fsl-cst/tests/it/formatter_style.rs`.

## Expected Results

- Each protocol statement after a `}` starts its own line, and the token sequence is unchanged.
- The canonical spellings of FR-003-AC-12, with whitespace-stripped bytes equal.
