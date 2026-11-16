---
id: FR-943
title: "Check and plan a timed interval with a lower bound and no upper bound"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-233
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: specifies
  - target: ix://agent-ix/filament-specification/FR-766
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-367
    type: references
---
# FR-943: Check and plan a timed interval with a lower bound and no upper bound

## Description

Under `filament.temporal.timed/v1`, FSL's S3 checker SHALL admit a timed
interval `[q, *]` (or `(q, *)`) on every temporal operator, as FSpec FR-766
states, over a model subject bound `model-time` and over a trace subject whose
clock role takes a `timed` configuration (FR-642). The tick monitor plan
(FR-251) SHALL plan such an operator, and the step form `[a, *]` that the
infinite-trace profile admits, without refusal: a past operator by FR-251
"Unbounded past operators" with `T(n) = a`, and a future operator as an
unbounded future operator whose obligation a witness discharges only at a
distance of at least `a`.

### Decision: `[q, *]` is admitted under the timed profile

FR-251 "Unbounded past operators" plans `[a, *]` with `a > 0` and FR-251-AC-34
plans `once[5 ms, *]`, but no profile let a clause reach it: the timed checker
refused every open timed interval as a hybrid-claim form, and the bounded
profiles refuse `[a, *]` by design. The timed profile admits the unbounded
operator grammar, and `[q, *]` is that operator restricted to a least
distance, so admitting it adds no reading the profile lacks and makes
FR-251's planned form writable (FSpec FR-766). The bounded profiles are
unchanged.

## Inputs

A clause under the timed profile, or a clause under the infinite-trace
profile, with an operator interval whose upper end is `*`.

## Outputs

The checked `TimedInterval` with lower bound and openness and no upper bound
(or the checked step `TemporalInterval{lower: a, upper: Open}`), and a
`TimedMonitorPlan` for it.

## Behavior

- The timed checker SHALL convert the lower bound of `[q, *]` to the binding's
  unit as for a finite interval and record no upper bound; it SHALL NOT refuse
  `OpenInterval` under the timed profile. `[0 ms, *]` SHALL check to the same
  operator and key as the operator with no interval.
- A unitless lower bound under `model-time` SHALL refuse `ill_typed`/
  `type-mismatch` (FR-233 "Intervals").
- Under the timestamped-event finite-window and the other bounded profiles,
  `[q, *]` and `[a, *]` SHALL keep FR-326's refusals.
- The monitor plan SHALL plan a past operator with interval `[a, *]`, `a > 0`,
  in ticks of the clause's counter (time units converted to ticks for a timed
  clause, positions for a position-counted clause), with FR-251's retention
  `R(n)` and `T(n) = a + S(n)`, and SHALL NOT refuse it
  (`OpenIntervalLowerBound` is removed).
- The monitor plan SHALL plan a future operator with interval `[a, *]` as an
  unbounded future operator counted against the activation capacity, whose
  instance is discharged by a witness at a distance of at least `a` from its
  origin and never by one nearer.
- The monitor entries SHALL be generated for every unbounded past operator,
  `[0, *]` included, and for an unbounded operator over another unbounded
  operator, as FR-251 states; refusing them is not a specified outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-943-AC-1 | Under the timed profile over the ConfigVersion model timed in `ms` (FR-233-AC-5) with no `clock` member, `once[5 ms, *] holds(c.versionNumber = 2)` and `eventually[5 ms, *] holds(c.versionNumber = 2)` check with lower bound 5 ms and no upper bound; `once(5 ms, *)` checks with an open lower end; `once[5, *]` refuses `ill_typed`/`type-mismatch`; none refuses `OpenInterval`. | Test (TC-2623) |
| FR-943-AC-2 | FR-251-AC-34's `once[5 ms, *] holds(p)` under the timed profile is planned with retention 5001 ticks and capacity 12, and gives `indeterminate` at tick 4000 and `true` at tick 6000 with `p` true at tick 0. | Test (TC-2623) |
| FR-943-AC-3 | Under the infinite-trace profile, `once[3,*] holds(p)` is planned on the position counter with retention 4 and capacity 5, not refused `OpenIntervalLowerBound`; with `p` true at position 0 it is `indeterminate` at position 2 and `true` at position 3; `eventually[2,*] holds(p)` is planned as an unbounded future operator and, from an origin at position 0, is not discharged by `p` at position 1 and is discharged `true` by `p` at position 2. | Test (TC-2623) |
| FR-943-AC-4 | FR-251-AC-22's `once holds(p)` and `historically holds(p)` reach generated monitor entries; neither entry generation refuses `UnboundedPast` or `UnboundedOverUnbounded`. | Test (TC-2623) |

## Dependencies

- FSpec FR-766 owns the `[q, *]` interval under the timed profile; FSpec FR-367
  owns the step `[a, *]` under infinite-trace.
- FR-251 owns the monitor plan; FR-233 owns timed interval bounds.
