---
id: FR-1020
title: "Transfer previous and interval clauses through a step-for-step model refinement"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-380 filament-specification/FR-375 filament-spec-language/FR-144
    type: depends_on
---
# FR-1020: Transfer previous and interval clauses through a step-for-step model refinement

## Description

FSL SHALL decide FSpec FR-380's step-for-step condition from a model refinement's step map alone, and SHALL transfer a proved abstract clause that uses previous (`once[1,1]`) or interval operators exactly when the refinement is step-for-step, settling it `inconclusive`, `transfer-not-preserving` naming the first such operator otherwise (FSpec FR-380-AC-9).

## Behavior

- A refinement SHALL be step-for-step when its step map sends every concrete operation to an abstract operation step, none to `stutter` or `any`, and its concrete subject is not `time dense`.
- The decision SHALL read only the refinement node's step rows (FR-136); it SHALL NOT explore the concrete subject.
- This rule governs model-to-model refinement. An implementation guarantee (FR-529) is never step-for-step, because FSpec FR-488's transition simulation always admits allowed stutters, so FR-529-AC-11 is unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1020-AC-1 | Over a step-for-step refinement, proved abstract `always (holds(c.value = 2) implies once[1,1] holds(c.value = 1))` and `always (holds(c.value = 1) implies eventually[0,2] holds(c.value = 2))` transfer `proved`, `decisive-witness`, with no concrete exploration. | Test (TC-2870) |
| FR-1020-AC-2 | Over a refinement whose step map sends a concrete operation to `stutter`, the same clauses settle `inconclusive`, `transfer-not-preserving`, naming `once[1,1]` and `eventually[0,2]`; one with an `any` arm or a `time dense` concrete subject is not step-for-step. | Test (TC-2870) |

## Dependencies

- FR-380 filament-specification/FR-375 filament-spec-language/FR-144 (filament-specification).
