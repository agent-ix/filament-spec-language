---
id: TC-2958
title: "Source analysis keeps its checked graph and the package route reaches every evaluator"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-2958: Source analysis keeps its checked graph and the package route reaches every evaluator

## Description

Verify that analysis from source decides every item family over its compiled
package, that a selected package reaches the source-checked evaluators through
their package forms with equal records, and that a missing package form
settles only the package-route item. Scope: FR-940-AC-4 to FR-940-AC-6.

## Test Procedure

1. Analyze the Counter program over source with `Bounded`, `AtMostTwo` and the
   invariant `NotAboveThree`, and the `SB` litmus claim under `memory` `tso`.
2. Compile the Counter unit, select its bytes as `package`, and analyze
   `NotAboveThree` and `AtMostTwo`; compare with step 1.
3. Analyze one claim whose evaluator has no package form over both routes, and
   a `package` file the strict reader refuses.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `proved`, `refuted` (reproduced), `proved` (`exhaustive`); `SB`
  `refuted`; no `unsupported` record and no package-fact diagnostic.
- Step 2: item records equal to step 1's.
- Step 3: package item `unsupported` naming the evaluator, source item its
  verdict; the refused package settles no item and compiles no source.
