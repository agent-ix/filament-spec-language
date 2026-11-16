---
id: TC-2620
title: "Analysis reads one sealed view over a compiled and a byte-read package"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2620: Analysis reads one sealed view over a compiled and a byte-read package

## Description

Verify that the analysis engines read a package only through `AnalysisView`,
that the two implementations agree, and that no conversion into checked
typestate exists. Scope: FR-940-AC-1, FR-940-AC-2, FR-940-AC-3.

## Test Procedure

1. Compile FR-850-AC-1's Counter unit; take the `AdmittedPackageRead` of its emission and the `BarePackage` of its written bytes; compare every `AnalysisView` accessor and the item records for `Bounded`, `BelowThree` and `AtMostTwo` over both.
2. Build the `compile_fail` doctest implementing `AnalysisView` for a local type and the one passing a `CheckedPackage` as `&dyn AnalysisView`; scan every function returning `CheckedPackage` or `CheckedGraph`.
3. Run FR-850-AC-1's `package` request through `fsl analyze`.

Test: `fsl-analyze/tests/it/analysis_view.rs`.

## Expected Results

- Step 1: equal accessor values and equal item records.
- Step 2: the doctest fails with `E0277`; no such function takes a byte-read value.
- Step 3: FR-850's outcome, exit 0, no not-implemented refusal.
