---
id: TC-2976
title: "Simulation reports invariant violations as findings and settles no claim"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-634
    type: verifies
---
# TC-2976: Simulation reports invariant violations as findings and settles no claim

## Description

Verify that a simulation item reports each invariant violation on its
reported traces as a finding naming the trace, the position, the invariant
and the object, without settling a claim. Scope: FR-634-AC-6, FR-634-AC-7,
FSpec FR-181-AC-33.

## Test Procedure

1. Enumerate the Counter subject with `AtMostTwo` (`self.value <= 2`).
2. Sample it with seed 5 and `samples` 2, twice.
3. Enumerate and sample the Counter subject with `AtMostThree` (`self.value <= 3`) and with no invariant.
4. Enumerate the `AtMostTwo` subject under `max_states` 2.

## Expected Results

- Step 1: `success`; one trace, `value` 0 to 3; one `invariant-violated` finding naming `AtMostTwo`, counter `a`, trace 0, position 3; no `record`, no `counterexample`.
- Step 2: `success`; one finding at trace 0 and one at trace 1, each at position 3; the two documents are equal.
- Step 3: `findings` `[]`; category, traces and coverage equal those of the subject with no invariant.
- Step 4: `incomplete` `LimitReached{max_states}`; `findings` `[]`.

Test: `fsl-analyze/tests/it/simulation_findings.rs`.
