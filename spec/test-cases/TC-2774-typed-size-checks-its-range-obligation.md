---
id: TC-2774
title: "A typed size query is typed at its result domain and proves its range"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-981
    type: verifies
---
# TC-2774: A typed size query is typed at its result domain and proves its range

## Description

A typed size query is typed at its result domain and proves its range.

Scope: FR-981-AC-3, FR-981-AC-5.

## Test Procedure

Check the bodies of FR-981-AC-3 and FR-981-AC-5 under `CheckMode::Linked`. Test: `fsl-semantics/tests/it/typed_size.rs`.

## Expected Results

- `Int[0, 4]` checks; `Int[0, 3]` refuses `undefined_expression`/`unproved-range` unless guarded; `Boolean` and the integer operand refuse `ill_typed`/`type-mismatch`; `size<Count>(c)` checks.
