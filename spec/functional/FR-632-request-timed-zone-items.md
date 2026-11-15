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
- `temporal.limits`: FR-625's limits plus `max_clocks`.
- `options.method`: `zones` (the default) or `digital-clocks`; the second is
  admitted only for a closed subject (FR-243).

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
  `max_clocks` settles `incomplete` `LimitReached{max_clocks}`.
- **Refusals.** `options.method` `digital-clocks` on a subject with a strict
  clock constraint settles `declined` `invalid-input`; an untimed subject
  routes to EN-1 as FR-625 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-632-AC-1 | A timer subject whose operation fires within 2 time units settles the MITL claim `eventually[0,2]` `proved` with basis `zone-certified`, certification `certified`, and `eventually[0,1]` `refuted` with a counterexample whose delays are exact rationals and replay. | Test (TC-1592) |
| FR-632-AC-2 | The same claims with `options.method` `digital-clocks` on the closed subject settle the same verdicts. | Test (TC-1592) |
| FR-632-AC-3 | `max_clocks` 0 settles `incomplete` `LimitReached{max_clocks}`. | Test (TC-1592) |

## Dependencies

- FR-237 to FR-245 own zone search, MITL, concretisation, digitisation and
  certificates.
