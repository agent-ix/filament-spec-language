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

- **Supplied traces.** An analyze item decides a temporal clause over every
  behaviour of a model subject; it does not evaluate one over a supplied
  trace. `finite-replay` replays a recorded execution against protocol
  semantics and applies to the `protocol` family only (FR-507, FSpec FR-290),
  so a `finite-replay` item naming a temporal clause settles `invalid-request`
  with `invalid_capability`/`inapplicable-kind`, `family` `temporal-trace`,
  exit 20, and no engine runs. A temporal clause is evaluated over a supplied
  trace by the clause-run request (FR-641) and by the monitor (FR-251); a
  clause with any fairness constraint evaluated there settles `unsupported`
  with `MissingFairnessPremise{constraint}` naming the first constraint, exit
  21 (FR-132). A supplied trace has states and steps and no enabledness, so
  no fairness constraint can be checked on it; this is the stated limit of
  fairness, and the same clause is decided over a model subject here. A
  `finite-replay` item over a protocol clause with a fairness constraint
  settles by FR-132 too (FR-636).

### Decision: the supplied-trace path is the clause run's, not analyze's

FR-507 reads the family from the claim and admits `finite-replay` on
`protocol` claims only, and FSpec FR-520 defines an analyze item's `trace`
member as a protocol trace for a `finite-replay` item. Admitting
`finite-replay` on `temporal-trace` claims would add a second family to that
kind in the standard's closed table and a second trace spelling to the
analyze wire, while the clause-run request already carries every temporal
trace kind (FR-641). The capability, evaluating a fair clause over a recorded
trace and naming the premise that cannot be checked there, stays whole on the
clause-run and monitor paths.
- **Budgets and cancellation** are FR-625's; `max_automaton_states` bounds the
  claim automaton.
- **Refusals.** A lasso whose replayed cycle is not fair settles
  `inconclusive` `replay-parity`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-627-AC-1 | Over the staying Counter (the Counter fixture whose domain adds `setTo(n: Int[0, 3])` with `pre Stay { self.value = 0 and n = 0 }` and `post Keeps { self.value = n }`, a self-loop at `value = 0`), a clause `fair weak M::Counter::inc; eventually holds(c.value = 3)` settles `proved` with certification `certified`; without the fairness line it settles `refuted` with a lasso whose `loop` repeats the `setTo(0)` step at `value = 0` and never reaches 3. Over the inc-only Counter the clause without fairness settles `proved`: its one behaviour reaches 3 and stutters only at its terminal state (FR-125-AC-2), so fairness is what separates the two items only where the subject can stay away from 3. | Test (TC-1587) |
| FR-627-AC-2 | FR-129's mutex fixture gives `refuted` under `fair weak each M::Counter::setTo` and `proved` under `fair strong each M::Counter::setTo` in one request with two items. | Test (TC-1587) |
| FR-627-AC-3 | The refuted lasso of AC-1 carries `replay.reproduced` `true` and its cycle under the FSpec FR-364 member `loop`, and the replay records that the cycle is fair for each constraint of the clause. | Test (TC-1587) |
| FR-627-AC-4 | A `finite-replay` item naming a temporal clause carrying `fair strong M::Counter::inc`, with a supplied trace, settles `invalid-request` with `invalid_capability`/`inapplicable-kind`, `detail.family` `temporal-trace`, and no engine runs; the document exits 20. The same clause evaluated over a supplied trace settles `unsupported` `missing-fairness-premise` naming that constraint, exit 21, as FR-132-AC-1 states. | Test (TC-1587) |

## Dependencies

- FR-123, FR-129, FR-130 and FR-131 own fairness checking and replay;
  FR-132 owns the supplied-trace settlement.
