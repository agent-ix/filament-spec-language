---
id: FR-636
title: "Request protocol analysis items and replay supplied finite protocol traces through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-209
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-211
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-212
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-572
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-573
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-574
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-132
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-575
    type: depends_on
---
# FR-636: Request protocol analysis items and replay supplied finite protocol traces through analyze

## Description

A `native-analyze/1` item naming a protocol clause SHALL run on the protocol
analysis engine, deciding over the protocol system (FR-205): deadlock
freedom against declared terminal states (FR-211), liveness and bounded
lassos under scheduler fairness (FR-212), replicated roles and runtime role
instances (FR-209), identities, causal edges and delivery, per-instance
obligations, compensation and retry or commit identities, channel capacity
and overflow, and parallel joins. Local projection (FR-570) and
realizability (FR-571) are items of their own kinds. Results are keyed and
settled as FR-572 and FR-573 state and carry the separating witness record
on a decisive counterexample (FR-574).

### Decision: supplied finite protocol traces are accepted

A `finite-replay` item SHALL replay a finite protocol trace the caller
supplies, not only one this engine produced. Checking a recorded run of a
deployed system against its protocol is the use the feature exists for, and
the trace is replayed by the same re-execution that replays the engine's own
counterexamples (FR-217), so accepting it adds no second semantics. The
supplied trace uses the protocol counterexample's trace spelling (FSpec
FR-433), so one reader serves both. A recording of a deployed system that is
not in that spelling, or that holds steps the protocol does not enable, is
assessed in replay mode (FR-895), which reports each deviation as a violated
obligation rather than declining the input.

### Decision: the profile and the bounds are item members with no default

A `local-projection` or `realizability` item SHALL carry its profile and its
bounds as the members `temporal.profile`, `temporal.max_depth` and, for
`realizability`, `temporal.max_history`, beside each other in the item's
`temporal` group, where every other temporal item already carries its
horizon. They are members of the result's key (FR-572, ADR-036 PA-14), and a
key member has no default: two callers who wrote nothing would otherwise get
results keyed by a value neither chose, and a horizon picked by FSL would
make an `inconclusive` result read as the caller's own bound. FSL therefore
derives none of them from another bound or limit, and an item that needs one
and lacks it settles `invalid-request` naming the missing member rather than
running. A `local-projection` item has no `max_history` key member (FR-572),
so it carries no `temporal.max_history`; a member that bounds no work would
be accepted while changing nothing.

## Inputs

### Request items

- `claim`: the protocol clause's name. `kind`: `global-conformance` (FR-896:
  over every behaviour, or over a `recording` in replay mode), `local-projection`, `realizability`,
  `composition` or `monitorability`, or `finite-replay`; FR-620's
  `absent-kind` rule applies, since a protocol clause admits several kinds.
- `temporal.item_kind` `deadlock-freedom` with the protocol as `claim` requests the
  protocol deadlock item (FR-211).
- `assessed_root`: required for `local-projection` and `realizability`
  (FSpec FR-331); `options.role` names the role a `local-projection` item
  projects onto.
- `temporal.profile`: required for `local-projection` and `realizability`;
  a JSON string, one of FR-575's profile words `bounded` or `bounded-static`.
- `temporal.max_depth`: required for `local-projection` and `realizability`;
  the horizon in events along a path or play (FR-570, FR-571), a JSON
  integer from 1 to 2^53 - 1.
- `temporal.max_history`: required for `realizability` and refused on
  `local-projection`; the length of a role's local history (FR-571), a JSON
  integer from 1 to 2^53 - 1.
- `temporal.limits`: FR-625's limits plus `max_live_instances`,
  `max_messages` (FR-225: the live messages per RA location after garbage
  collection, and each channel's occupancy, counted per channel) and
  `max_walk_steps`.
- `trace`: for `finite-replay`, the `{authority, identity}` of a provisioned
  document holding the trace in FSpec FR-433's spelling.
- `recording`: for `global-conformance` in replay mode, the `{authority, identity}`
  of a provisioned FSpec FR-737 recording document (FR-895).
- `population`: optional, only beside `recording`: the activated role
  instances of each replicated role per protocol instance, which enumerate
  the fan-out members whose closure the recording must show (FR-990
  "Fan-out members over a recording").

## Outputs

- `record`: FR-573's labels; a protocol deadlock counterexample ends in a
  non-terminal protocol state with no enabled step; a liveness refutation is
  a bounded lasso fair for the scheduler fairness FR-212 derives.
- `counterexample`: FSpec FR-433's protocol counterexample with FR-574's
  separating witness record, replayed by FR-217.
- For `finite-replay`: `proved` when the trace replays step by step and the
  clause holds on it; `refuted` with the trace as counterexample when it
  replays and the clause fails on it.

## Behavior

- **Budgets.** A run that reaches a limit settles `incomplete` naming it
  (FR-572 keys the result).
- **Cancellation** is FR-625's.
- **Profile and bounds.** A `local-projection` or `realizability` item
  SHALL be analysed only when every member it requires above is present and
  in range. An absent `temporal.profile`, `temporal.max_depth` or (for
  `realizability`) `temporal.max_history` settles the item `invalid-request`
  with `invalid_runtime_input`/`missing-member` naming the member, as FR-572
  refuses a key with an absent member. A profile word other than `bounded`
  or `bounded-static` (FR-575), or a bound outside 1 to 2^53 - 1, settles the
  item `invalid-request` with `invalid_runtime_input`/`invalid-value` naming
  the member. A `local-projection` item carrying `temporal.max_history`
  settles `invalid-request` with `invalid_runtime_input`/`unknown-member`
  naming it. Each refusal runs no analysis for that item, the request's other
  items still run, and the request exits 20. A member of the wrong JSON type
  refuses the whole request as FR-620 states.
- **Supplied trace refusals.** A trace step that is stale or not enabled in
  the replayed protocol state settles `declined` `invalid-input`, diagnostic
  `invalid_runtime_input`/`invalid-value` naming the step's position; a
  clause with a fairness constraint settles `unsupported`
  `missing-fairness-premise` (FR-132, FR-627); a `trace` naming no
  provisioned document refuses the request `invalid-request` naming it.
- **Agreement.** A protocol deadlock found by `global-conformance` and the
  projection of the same protocol agree: a protocol whose every local
  projection is deadlock-free and realizable has no global deadlock.

## Acceptance Criteria

The order fixture: roles `Buyer` and `Seller`, channel `Order` from `Buyer`
to `Seller`, and a run `send Placed via Order`, `receive Got via Order of Placed`,
then a choice by `Seller` between `Ship` and `Cancel`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-636-AC-1 | The order fixture's protocol deadlock item settles `proved`; the fixture with the `receive` removed settles `refuted` with a counterexample ending in the sent, never-received message, carrying the separating witness record, and replays. | Test (TC-1596) |
| FR-636-AC-2 | `local-projection` onto `Seller` (`temporal.profile` `bounded-static`, `temporal.max_depth` 16) and `realizability` (the same with `temporal.max_history` 16) of the order fixture settle `proved`, and the deadlock verdict of AC-1 agrees with the projection's for both fixtures. | Test (TC-1596) |
| FR-636-AC-3 | A `finite-replay` item over a supplied trace of `Placed` then `Got` then `Ship` settles `proved`; a supplied trace beginning with `Got` settles `declined` `invalid-input` naming step 0. | Test (TC-1596) |
| FR-636-AC-4 | A protocol run with a parallel join whose branches each send on `Order` with capacity 1 settles the overflow as FR-572 keys it, and `max_messages` 1 settles `incomplete` naming it. | Test (TC-1596) |
| FR-636-AC-5 | Over the order fixture, a `realizability` item with `temporal.profile` `bounded-static`, `temporal.max_depth` 16 and `temporal.max_history` 16, and a `local-projection` item onto `Seller` with the same profile and `max_depth` and no `max_history`, each settle `proved`; the same items with profile `bounded` settle `proved`; the request exits 0. | Test (TC-2988) |
| FR-636-AC-6 | Over the order fixture, a `realizability` item lacking `temporal.max_history`, one lacking `temporal.profile`, one lacking `temporal.max_depth`, and a `local-projection` item lacking `temporal.profile` each settle `invalid-request` with `invalid_runtime_input`/`missing-member` naming that member, with no verdict and no derived default; a complete `realizability` item in the same request still settles `proved`, and the request exits 20. | Test (TC-2988) |
| FR-636-AC-7 | Over the order fixture, a `realizability` item with `temporal.profile` `static`, one with `temporal.max_history` 0 and one with `temporal.max_depth` 0 each settle `invalid-request` with `invalid_runtime_input`/`invalid-value` naming that member, and a `local-projection` item carrying `temporal.max_history` 16 settles `invalid-request` with `invalid_runtime_input`/`unknown-member` naming it; the request exits 20. | Test (TC-2988) |

## Dependencies

- FR-205 to FR-217 and FR-570 to FR-574 own the protocol system, its
  analysis, replay and result keys.
