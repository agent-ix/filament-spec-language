---
id: FR-625
title: "Request and report an explicit-state model-check item through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
---
# FR-625: Request and report an explicit-state model-check item through analyze

## Description

A `native-analyze/1` item that names a temporal clause or a checked
`invariant` state clause over a model subject SHALL run on EN-1's
explicit-state search (FR-126) and report its exploration, its verdict, its
counterexample and replay, its certificate and its budget use in the FR-621
item members. The library request types are not the public contract; this
requirement is.

## Inputs

### Request item

- `claim`: the declared name of a temporal clause, or of an `invariant`
  state clause, whose subject is a model. An invariant is decided as the
  safety claim that it holds at every reachable state.
- `kind`: `temporal-satisfaction` (the default for these families);
  `temporal.item_kind` `formula`.
- `temporal.subject`: optional; the request's default subject otherwise
  (FR-620).
- `temporal.limits`: `max_states`, `max_transitions`, `max_automaton_states`
  and `max_candidates`, each a positive count, each absent one EN-1's
  published default; `temporal.max_depth`: optional search horizon, none
  when absent; `temporal.stutter_closure` and `temporal.smt_encoding` as
  FSpec FR-331 states.
- `reductions`: FR-623.

## Outputs

### Result members

- `record`: `proved` with proof basis `exhaustive` (or `reduced`, FR-623),
  certification `certified` once FR-282's closure checker accepts the
  certificate and `uncertified` otherwise; `refuted` with the counterexample;
  or the incomplete, inconclusive and refusal records below.
- `counterexample`: FR-125's counterexample contract, a finite prefix for a
  safety violation and a lasso for a liveness violation, in FSpec FR-331's
  `TemporalCounterexample` spelling; `replay` names FR-128's replay target and
  is present only after S6a replay reproduced it.
- `certificate`: the closure certificate's identity, the core checker and its
  acceptance.
- `exploration`: the states stored, the transitions expanded and the
  automaton states of the run; `accounting.reached` names the limits the run
  reached.

## Behavior

- **Budgets.** A run that reaches a limit stops and settles `incomplete`
  with `LimitReached{limit}` naming it; `exploration` reports what was
  explored. A search horizon that cuts exploration settles `incomplete` with
  `LimitReached{max_depth}`, never `proved`.
- **Cancellation.** A cancelled or expired run settles `incomplete` with the
  cancellation or deadline cause, reporting what was explored.
- **Refusals.** A root domain no proof bound covers settles `unsupported`
  with `unsupported_projection`/`unbounded-extent` (FR-281 "Engine
  refusals"); a refused subject admission settles `declined` `invalid-input`;
  a counterexample replay that does not reproduce settles `inconclusive`
  `replay-parity`.

## Acceptance Criteria

The Counter fixture is FR-620's.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-625-AC-1 | `{"claim": "Bounded"}` settles `proved`, basis `exhaustive`, certification `certified`, with `exploration.states` 4 and `exploration.transitions` 3, equal to an independent breadth-first count of the Counter subject's reachable states and steps. | Test (TC-1585) |
| FR-625-AC-2 | A clause `always holds(c.value <= 2)` settles `refuted` with a finite-prefix counterexample of four states ending at `value = 3`, and `replay.reproduced` `true`. | Test (TC-1585) |
| FR-625-AC-3 | `temporal.limits.max_states` 2 settles `incomplete` `LimitReached{max_states}` with `exploration.states` 2; `temporal.max_depth` 1 settles `incomplete` `LimitReached{max_depth}`. | Test (TC-1585) |
| FR-625-AC-4 | An `invariant` state clause `self.value <= 3` named as `claim` settles `proved`; `self.value <= 2` settles `refuted` with a counterexample ending at `value = 3`. | Test (TC-1585) |
| FR-625-AC-5 | A Counter domain whose `setTo(n: Integer)` parameter is unbounded and no proof bound settles the item `unsupported` with `unbounded-extent`. | Test (TC-1585) |

## Dependencies

- FR-126 owns EN-1's search; FR-125 the counterexample contract; FR-128 the
  replay; FR-282 the certificate checker.
