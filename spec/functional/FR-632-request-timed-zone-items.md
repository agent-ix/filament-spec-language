---
id: FR-632
title: "Request timed zone-search items through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-240
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-241
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-242
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-243
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-244
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-237
    type: depends_on
---
# FR-632: Request timed zone-search items through analyze

## Description

A `native-analyze/1` item over a timed subject SHALL be decided by the zone
engine: symbolic zone search over difference-bound matrices (FR-238,
FR-239), MITL claims through their claim automata (FR-242), timed liveness
and time-lock freedom on the symbolic graph (FR-240), with a zone
certificate for every proof (FR-244) and counterexamples concretised with
exact rational delays (FR-241). A closed timed subject may instead be
digitised and checked explicitly (FR-243).

## Inputs

### Request item

- `claim`: a timed temporal clause's name, or the subject anchor with
  `temporal.item_kind` `time-lock-freedom` (FR-626); `kind` `temporal-satisfaction`.
- `temporal.limits`: FR-625's limits plus `max_clocks`, the most live clocks
  a state may hold (FSpec FR-415 "Cardinality bound"). Like every FR-625
  limit it is a positive count; absent, it is the published default 1024.
- `options.method`: `zones` (the default) or `digital-clocks`; the second is
  admitted only for a closed subject and a claim whose every interval is
  closed (FR-243).

### Decision: `digital-clocks` is a caller-selected route, not an EN-6 mode

EN-6 decides every timed item it receives by zone search and never
digitises (FR-239). An item that names `options.method` `digital-clocks`
therefore does not reach EN-6: FR-243 routes it to the digital-clock route,
which reads the closed subject through integer-valued clocks (ADR-028 TA-1
to TA-3) and decides the claim by EN-1's explicit-state search (FR-126) over
that reading. The route stays a function of the item alone (its kind, its
evidence kind and its `options.method`), so one item always reaches one
engine. Digitisation decides the same verdict as dense time exactly when the
subject and the claim are both closed (Henzinger, Manna and Pnueli), so the
route is a cross-check of the zone verdict, with its own basis
`digitized-explicit-state` (FR-235), and is refused where it could disagree.

## Outputs

- `record`: `proved` with basis `zone-certified` and certification
  `certified` once FR-245's core checker accepts the zone certificate;
  `refuted` with the timed counterexample. A behaviour that lets time
  converge (a Zeno run) never witnesses a liveness violation: liveness is
  decided over non-Zeno runs, and a run whose only violating cycles are Zeno
  settles `proved` with the non-Zeno premise in its record.
- `counterexample`: the symbolic run concretised with exact rational delays,
  replayed by FR-237.

## Behavior

- **Budgets and cancellation** are FR-625's; a subject with more clocks than
  `max_clocks` settles `incomplete` `LimitReached{max_clocks}`, naming the
  limit, its value and its setting `model_check.max_clocks` (FSpec FR-415).
  A `max_clocks` of 0 is not a count a run can be stopped by: it refuses the
  whole request at the request stage with `invalid-request`, exit 20, naming
  `temporal.limits.max_clocks`, as every FR-625 limit of 0 does.
- **Digital-clock records.** An item decided on the digital-clock route
  settles `proved` with basis `digitized-explicit-state` and the
  certification FR-625 gives an EN-1 proof, or `refuted` with a
  counterexample whose delays are integers written as exact rationals and
  replayed by FR-237. Its record names backend `en-1`.
- **Refusals.** `options.method` `digital-clocks` on a subject with a strict
  clock constraint, or over a claim with a strict (open or half-open)
  interval, settles `declined` `invalid-input`, naming the constraint or the
  interval; an untimed subject routes to EN-1 as FR-625 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-632-AC-1 | A timer subject whose operation fires within 2 time units settles the MITL claim `eventually[0,2]` `proved` with basis `zone-certified`, certification `certified`, and `eventually[0,1]` `refuted` with a counterexample whose delays are exact rationals and replay. | Test (TC-1592) |
| FR-632-AC-2 | The same claims with `options.method` `digital-clocks` on the closed subject settle the same verdicts on backend `en-1`, the proof with basis `digitized-explicit-state` and the refutation with integer delays that replay. | Test (TC-1592) |
| FR-632-AC-3 | Over the `Rpc` subject provisioned with two calls (two live clocks), `max_clocks` 1 settles `incomplete` `LimitReached{max_clocks}` with value 1; `max_clocks` 0 refuses the whole request at the request stage, `invalid-request`, exit 20, naming `temporal.limits.max_clocks`, and runs no item. | Test (TC-1592) |
| FR-632-AC-4 | `options.method` `digital-clocks` over the strict-guard retry subject, and over the closed subject with the half-open claim `eventually[0 ms, 2 ms) holds(not c.late)`, each settle `declined` `invalid-input` naming the strict constraint or interval, and no engine runs. | Test (TC-2290) |

## Dependencies

- FR-237 to FR-245 own zone search, MITL, concretisation, digitisation and
  certificates.
