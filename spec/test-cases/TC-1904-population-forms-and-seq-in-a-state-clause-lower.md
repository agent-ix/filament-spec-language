---
id: TC-1904
title: "Population forms and seq in a state clause check and lower"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-139
    type: verifies
---
# TC-1904: Population forms and seq in a state clause check and lower

## Description

Verify FR-391-AC-10 and FR-139-AC-2: a state clause body may use all six population forms, `seq` included, and each lowers to FSpec FR-322 terms.

## Test Procedure

Check and emit the four invariants of FR-391-AC-10 over the `Counter` unit. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

Each invariant checks and emits with no refusal; `seq` types `Sequence<Int[0, 2]>[0, 3]`; no refusal has code `unknown_required_feature` or `unsupported_construct`.
