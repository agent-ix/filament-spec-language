---
id: FR-629
title: "Request x86-TSO, release/acquire and race-freedom items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-223
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-224
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-225
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-226
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-227
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-228
    type: depends_on
---
# FR-629: Request x86-TSO, release/acquire and race-freedom items through analyze

## Description

A `native-analyze/1` request whose subject has a weak `parallel` SHALL
select that parallel's memory model in the request-level `memory` member,
and its items SHALL be decided under that model: sequential consistency,
x86-TSO (FR-221) or release/acquire with the RC11 partial SC order (FR-222,
FR-223). The derived race-freedom item reports data races on non-atomic
locations (FR-224).

## Inputs

- `memory`: FSpec FR-331's `request.memory`, one selection per weak
  `parallel`, naming it and its model `sc`, `tso` or `ra`. A weak `parallel`
  with no selection is `sc`.
- Items: formula items over the subject (FR-625, FR-627) and
  `{"claim": "<parallel name>", "item_kind": "race-freedom"}`.
- `temporal.limits.max_store_buffer` bounds each store buffer (FR-225),
  beside FR-625's limits.

## Outputs

- `record` and `counterexample` as FR-625, the counterexample carrying the
  memory component: store-buffer contents, the modification order and the
  reads-from choices of each step (FR-227), and `replay` replaying it under
  the same model.
- A race-freedom refutation names the two racing accesses and their
  locations.
- Every weak-memory verdict carries FR-226's code-link preconditions in its
  record.

## Behavior

- **Routing.** Negotiation routes an item only to a backend whose
  `memory_models` advertises the selected model (FSpec FR-439); otherwise
  the item settles `unsupported` with `unsupported-requested-capability`.
- **Liveness over `ra`.** A liveness item over a `parallel` selected `ra`
  settles as the protocol-rule requirement for release/acquire liveness
  decides, with memory fairness derived by FR-228; that decision belongs to
  the release/acquire liveness rule, not to this request item.
- **Budgets and cancellation** are FR-625's; a store buffer that would exceed
  `max_store_buffer` settles `incomplete` `LimitReached{max_store_buffer}`.
- **Refusals.** A `memory` selection naming no weak `parallel`, or naming one
  twice, refuses the request `invalid-request` naming it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-629-AC-1 | The store-buffering litmus subject with the claim that both reads never return 0 settles `proved` under `sc` and `refuted` under `tso`, the TSO counterexample carrying both pending stores in the store buffers, with `replay.reproduced` `true`. | Test (TC-1589) |
| FR-629-AC-2 | The message-passing litmus subject under `ra` with release/acquire accesses settles `proved`, and with relaxed accesses settles `refuted`, agreeing with the RC11 oracle. | Test (TC-1589) |
| FR-629-AC-3 | A race-freedom item over a parallel with two unsynchronised non-atomic writes settles `refuted` naming both accesses. | Test (TC-1589) |
| FR-629-AC-4 | A liveness item over a parallel selected `ra` has a terminal record and, when it is not `proved` or `refuted`, a diagnostic with a catalog code: it never ends without a record. | Test (TC-1589) |
| FR-629-AC-5 | A `memory` selection naming no weak parallel refuses the request `invalid-request`, exit 20. | Test (TC-1589) |

## Dependencies

- FR-221 to FR-228 own the memory models, race freedom, budgets, verdict
  preconditions and replay.
