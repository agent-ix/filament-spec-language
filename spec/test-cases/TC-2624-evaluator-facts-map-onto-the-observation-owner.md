---
id: TC-2624
title: "Evaluator trace facts map onto the observation owner's contexts and views"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-944
    type: verifies
---
# TC-2624: Evaluator trace facts map onto the observation owner's contexts and views

## Description

Verify the half-tick interval mapping, trace admission, partial facts,
activation and repair mapping, and the completeness revision rule. Scope:
FR-944-AC-1 to FR-944-AC-4.

## Test Procedure

1. Compare `ClockCover`'s `definitely_before` and `within_bound` with the owner's `relation_to` on half-tick intervals for readings 0 to 6 and bounds 0 to 6, under `Counter { uncertainty: 1 }` and `Exact`.
2. Admit FR-501-AC-13's trace through `filament_observation::admit` and read the atoms' partial facts.
3. Derive FR-524-AC-27 to AC-29's activation views and FR-526-AC-2's repair plan.
4. Apply FR-502-AC-2's cases through `completeness::revise`.

Test: `fsl-eval/tests/it/observation_owner.rs`.

## Expected Results

- Step 1: equal answers on every pair.
- Step 2: one record per position with its FR-106 identity; possibility sets and reasons of the Kleene values.
- Step 3: the evaluator's dispositions; exactly the superseded result invalidated.
- Step 4: the refusals and acceptances FR-502's ledger gives.
