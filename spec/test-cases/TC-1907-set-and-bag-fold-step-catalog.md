---
id: TC-1907
title: "Set and bag fold step catalog"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-701
    type: verifies
---
# TC-1907: Set and bag fold step catalog

## Description

Verify FR-701-AC-1 to AC-3: the Set/Bag step catalog admits commutative operations, including min/max over bounded ordered accumulator types, and refuses invalid order, type and element cases.

Scope includes FR-701-AC-3.

## Test Procedure

Check each fold of FR-701-AC-1 to AC-3 as a function body. Include bounded `Int[0, 9]` and `Rational[0, 4; 1, 2]` min/max folds, empty `fold` and `reduce`, every permutation of fixed nonempty collections, an unordered Boolean or record accumulator, an identity type mismatch and an element/accumulator type mismatch. Confirm AC-2's bounded Rational addition still refuses. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

Tag the added cases with `#[trace("TC-1907", "FR-701-AC-3")]`.

## Expected Results

The AC-1 catalog steps check; AC-2's bounded Rational addition and other ineligible set steps refuse; the sequence subtraction checks. AC-3's bounded ordered min/max folds and nonempty reduces check and are permutation invariant, empty fold returns identity and empty reduce has no result; mismatched and unordered cases refuse with the typed causes stated by AC-3.
