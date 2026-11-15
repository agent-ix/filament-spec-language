---
id: TC-1900
title: "Two recursion groups with equal bodies key apart"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-092
    type: verifies
---
# TC-1900: Two recursion groups with equal bodies key apart

## Description

Verify FR-092-AC-31: `check` keys every recursion group and never refuses one because its nodes cannot be told apart from another group's.

## Test Procedure

Check the unit of FR-092-AC-31 (pairs `a`/`b` and `c`/`d`) under `CheckMode::Linked` and read the semantic graph's nodes that carry a recursion group. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

The unit checks with no refusal; there are two recursion groups; the four function ids and every cross-group pair of derived node ids are distinct; no refusal has code `unknown_required_feature`.
