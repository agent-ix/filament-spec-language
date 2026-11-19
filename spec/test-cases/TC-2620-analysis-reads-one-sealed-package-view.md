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
typestate exists. Scope: FR-940-AC-1, FR-940-AC-2, FR-940-AC-3, FR-940-AC-13.

## Test Procedure

1. Compile FR-850-AC-1's Counter unit; take the `AdmittedPackageRead` of its emission and the `BarePackage` of its written bytes; compare every `AnalysisView` accessor and the item records for `Bounded`, `BelowThree` and `AtMostTwo` over both.
2. Build the `compile_fail` doctest implementing `AnalysisView` for a local type and the one passing a `CheckedPackage` as `&dyn AnalysisView`; scan every function returning `CheckedPackage` or `CheckedGraph`.
3. Run FR-850-AC-1's `package` request through `fsl analyze`.
4. Compile FR-531-AC-35's unit; compare `requirement_items()` and `value_validity_records_of` over the `AdmittedPackageRead` and the `BarePackage` and with `CheckedGraph::requirements()`; write `Even`'s and RR-12 `f`'s request items and linked-engine records on both routes and compare; repeat with the test-only view that drops the `+` record's guard (FR-940-AC-13).

Test: `fsl-analyze/tests/it/analysis_view.rs`.

## Expected Results

- Step 1: equal accessor values and equal item records.
- Step 2: the doctest fails with `E0277`; no such function takes a byte-read value.
- Step 3: FR-850's outcome, exit 0, no not-implemented refusal.
- Step 4: equal rows, request items and records on both routes, `f`'s items `unbounded` with one `integer` domain at `n` and `finite_bound_available` true; with the guard dropped, the `+` record differs.
