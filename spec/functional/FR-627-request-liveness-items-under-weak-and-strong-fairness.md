---
id: FR-627
title: "Request liveness items under weak and strong fairness through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-129
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-130
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-131
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-132
    type: depends_on
---
# FR-627: Request liveness items under weak and strong fairness through analyze

## Description

When a `native-analyze/1` item names a temporal clause with a liveness
formula, FSL SHALL decide the clause over every behaviour of the subject that
is fair under the clause's own fairness constraints (weak or strong, each or
whole; FR-123, FR-129, FR-130). The FR-621 members of the item carry its
verdict, lasso counterexample and certificate.

## Inputs

### Request item

FR-625's item. The fairness set is the checked clause's own (`fair weak`,
`fair weak each`, `fair strong`, `fair strong each`); the document carries
no fairness (FR-620). Strong fairness decides on the explicit-state product
(FR-130).

## Outputs

- `record`: `proved` with the fair-cycle certificate checked in the core;
  `refuted` with a lasso counterexample whose cycle is fair under every
  constraint of the clause.
- `replay`: FR-131's replay, which re-checks each fairness constraint on the
  lasso's cycle before the counterexample counts.

## Behavior

- **Supplied traces.** A clause with any fairness constraint evaluated over
  a supplied trace, through a `finite-replay` item (FR-636) or a `monitor`
  item, settles `unsupported` with `MissingFairnessPremise{constraint}`
  naming the first constraint, exit 21 (FR-132). A supplied trace has states
  and steps and no enabledness, so no fairness constraint can be checked on
  it; this is the stated limit of fairness, and the same clause is decided
  over a model subject.
- **Budgets and cancellation** are FR-625's; `max_automaton_states` bounds the
  claim automaton.
- **Refusals.** A lasso whose replayed cycle is not fair settles
  `inconclusive` `replay-parity`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-627-AC-1 | Over the Counter fixture, a clause `fair weak M::Counter::inc; eventually holds(c.value = 3)` settles `proved` with certification `certified`; without the fairness line it settles `refuted` with a lasso that stutters at a value below 3. | Test (TC-1587) |
| FR-627-AC-2 | FR-129's mutex fixture gives `refuted` under `fair weak each M::Counter::setTo` and `proved` under `fair strong each M::Counter::setTo` in one request with two items. | Test (TC-1587) |
| FR-627-AC-3 | The refuted lasso of AC-1 carries `replay.reproduced` `true`, and the replay records that the cycle is fair for each constraint of the clause. | Test (TC-1587) |
| FR-627-AC-4 | A `finite-replay` item of a clause carrying `fair strong M::Counter::inc` over a supplied trace settles `unsupported` with `missing-fairness-premise` naming that constraint, exit 21. | Test (TC-1587) |

## Dependencies

- FR-123, FR-129, FR-130 and FR-131 own fairness checking and replay;
  FR-132 owns the supplied-trace settlement.
