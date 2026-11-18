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
their package forms with equal records, and that the package route settles every item
family as the source route does. Scope: FR-940-AC-4 to FR-940-AC-6.

## Test Procedure

1. Analyze the Counter program over source with `Bounded`, `AtMostTwo` and the
   invariant `NotAboveThree`, and the `SB` litmus claim under `memory` `tso`.
2. Compile the Counter unit with the compile request's `handoff` member,
   select the handoff directory as `package` `{"directory": ...}`, and analyze
   `NotAboveThree` and `AtMostTwo`; compare with step 1.
3. Analyze `Bounded`, `AtMostTwo`, `NotAboveThree`, the deadlock-freedom item
   and the `SB` litmus claim under `tso` over both routes (the package route
   over the handoff directory), a handoff directory whose `checked-package.json`
   the strict reader refuses, and one with no `identities.json`.

Test: `fsl-analyze/tests/it/analyze_package_selection.rs`.

## Expected Results

- Step 1: `proved`, `refuted` (reproduced), `proved` (`exhaustive`); `SB`
  `refuted`; no `unsupported` record and no package-fact diagnostic.
- Step 2: item records equal to step 1's.
- Step 3: equal records, counterexamples and replays, none `unsupported`; the
  refused directories settle no item and compile no source.
