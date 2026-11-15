---
id: FR-626
title: "Request deadlock-freedom, intended terminal states and time-lock freedom through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-240
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-368
    type: depends_on
---
# FR-626: Request deadlock-freedom, intended terminal states and time-lock freedom through analyze

## Description

A `native-analyze/1` item with `item_kind` `deadlock-freedom` or
`time-lock-freedom` SHALL request FR-368's derived item for one subject and
report its verdict in the FR-621 members. Intended terminal states are
declared in the unit (`terminal` clauses, FR-124); the request never lists
them, so the source stays the one statement of which stopping states are
intended.

## Inputs

### Request item

- `claim`: the declaration the derived item anchors at (FR-368 "Claim and
  anchor"): the model alias of the subject for a model subject.
- `item_kind`: `deadlock-freedom` or `time-lock-freedom`; `kind`
  `temporal-satisfaction`.
- `temporal.subject` and `temporal.limits` as FR-625; a time-lock item over a
  timed subject also takes `max_clocks` and runs on the zone engine (FR-632).

## Outputs

- `record`: `proved` when every reachable non-terminal state has an enabled
  step (deadlock freedom), or when time can always diverge (time-lock
  freedom, FR-240); `refuted` otherwise.
- `counterexample`: a finite prefix whose last state is a reachable state that
  no `terminal` clause declares intended and that enables no step, or for
  time-lock a symbolic run concretised with exact rational delays (FR-241)
  that reaches a state from which time cannot advance.

## Behavior

- **Derivation.** With no `items`, the derived items FR-368 gives for each
  subject are part of the default items (FR-620). FR-124's rule decides
  whether the subject's deadlock-freedom item exists.
- **Budgets and cancellation** are FR-625's.
- **Refusals.** `time-lock-freedom` over a subject with no clock settles
  `invalid-request` with `invalid_capability`/`inapplicable-kind`; a
  `claim` that names no subject anchor settles `invalid-request`
  `unknown-claim`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-626-AC-1 | Over the Counter fixture with no `terminal` clause, `{"claim": "M", "item_kind": "deadlock-freedom"}` settles `refuted` with a counterexample whose last state is `value = 3`. | Test (TC-1586) |
| FR-626-AC-2 | The same unit with `terminal Done using v on M when { self.value = 3 };` settles that item `proved`. | Test (TC-1586) |
| FR-626-AC-3 | `time-lock-freedom` over the untimed Counter subject settles `invalid-request` `inapplicable-kind`; over a timed subject whose invariant forbids time passing in a reachable location it settles `refuted` with a concretised counterexample. | Test (TC-1586) |

## Dependencies

- FR-124 owns intended terminal states; FR-232 and FR-240 own time-lock
  freedom; FR-368 owns the derived items.
