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
- `kind`: for a temporal clause, `temporal-satisfaction`, the one kind its
  `temporal-trace` family admits, with `temporal.item_kind` `formula`. For an
  `invariant` state clause, `operation-contract`, the one kind its
  `state-model` family admits (FR-507); an absent kind takes it, since the
  family admits no other.
- `temporal.subject`: optional; the request's default subject otherwise
  (FR-620).
- `temporal.limits`: `max_states`, `max_transitions`, `max_automaton_states`
  and `max_candidates`, each a positive count, each absent one EN-1's
  published default; `temporal.max_depth`: optional search horizon, none
  when absent; `temporal.stutter_closure` and `temporal.smt_encoding` as
  FSpec FR-331 states.
- `reductions`: FR-623.

### Decision: an invariant item keeps its family's kind

An invariant is a state-model claim, and FR-507 and FSpec FR-290 admit only
`operation-contract` on that family; the family is read from the claim, never
from the engine that decides it. The item therefore stays `operation-contract`.
An invariant over a model subject is a reachability check, so EN-1
advertises `operation-contract` for it, and negotiation routes it to EN-1 by
FR-489's tie-break: an `invariant` state clause in a request that names a
model subject goes to EN-1, and an operation's `pre` or `post` contract or a
frame obligation goes to the contract checker. EN-1 then decides it
exhaustively over the subject's reachable states, with the certificate and
replay below. Running it under
`temporal-satisfaction` would need a second kind on `state-model` in the
standard's closed table, and would give one claim two request identities
depending on which kind the caller wrote. A `temporal-satisfaction` item that
names an invariant settles `invalid-request` with `inapplicable-kind`,
`family` `state-model` (FR-507).

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
  explored.
- **Search horizon.** `temporal.max_depth` is a horizon the caller chose, not
  a resource limit. A run that explores every behaviour up to that depth,
  finds no counterexample and leaves states beyond it unexplored settles as
  FR-127's V-5 row states: `inconclusive` with `BoundReached{depth}`, written
  `bound-reached` with its `depth`, execution `completed` and truth
  `pending`, never `proved` and never `incomplete`. The search did all the
  work it was asked to do, so the record says the claim is unsettled at that
  depth rather than that a budget stopped it; FR-127 is the one settlement
  map, and this item reports its row. A counterexample found within the
  horizon still settles `refuted`, and a run that reaches a resource limit
  before completing the horizon settles `incomplete` `LimitReached{limit}`
  (V-7). The `exploration` member reports what was explored in both cases.
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
| FR-625-AC-3 | `temporal.limits.max_states` 2 settles `incomplete` `LimitReached{max_states}` with `exploration.states` 2; `temporal.max_depth` 1 on `Bounded` settles `inconclusive` with cause `bound-reached`, `depth` 1, never `incomplete` and never `proved`; the document of the two items exits 22. | Test (TC-1585) |
| FR-625-AC-4 | An `invariant` state clause `self.value <= 3` named as `claim` with no `kind` is read as kind `operation-contract`, family `state-model`, is routed to EN-1 by FR-489's tie-break (its record has `settled_by` `result` and names `backend` `en-1` as FR-621 "The record's `backend` member" states, never `contract-checker`) and settles `proved`, basis `exhaustive`; `self.value <= 2` settles `refuted` with a counterexample ending at `value = 3`. The same claim with `kind` `temporal-satisfaction` settles `invalid-request` with `inapplicable-kind`, `family` `state-model`, and no engine runs. | Test (TC-1585) |
| FR-625-AC-5 | A Counter domain whose `setTo(n: Integer)` parameter is unbounded and no proof bound settles the item `unsupported` with `unbounded-extent`. | Test (TC-1585) |

## Dependencies

- FR-126 owns EN-1's search; FR-125 the counterexample contract; FR-128 the
  replay; FR-282 the certificate checker.
