---
id: FR-944
title: "Map the evaluator's trace facts onto filament-observation's contexts, selections and views"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-861
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-013
    type: depends_on
---
# FR-944: Map the evaluator's trace facts onto filament-observation's contexts, selections and views

## Description

FR-861 makes `filament-observation` the one owner of the partial-value,
completeness, activation and repair rules that `fsl-eval` applies. The owner's
rules take owner values: a `Context` over an admitted `History`, a
module `Selection`, and the validated `View` of a derived document. This
requirement states how the evaluator builds each of them from the facts it
holds (a trace's positions, stamps and cover, a progress assertion, an
activation scope, a superseding request's prior results) and how it reads the
owner's answer back into the reports FR-501, FR-502, FR-524 and FR-526 state.
It also states the one rule the owner does not have yet, the completeness
revision rule FR-502 needs, as a requirement on the owner.

### Decision: one admitted history per request, built from the trace

Every owner module except the interval relation reads a `Context`, and a
`Context` needs a `History` of a `QualifiedObservation` that the owner's
`admit` produced. The evaluator therefore admits the request's trace through
`filament_observation::admit` once, before any owner rule runs, and every
owner call of that request shares that history. Building the history from the
trace the evaluator already admitted (FR-106) keeps one admission authority
for the trace's identities and stamps, so the identity of a position in an
owner document is the identity FR-502's covered set records.

### Decision: an uncertain cover is a closed interval in quarter-ticks

The owner's `EventTimeInterval` is closed with integer endpoints, and its
`relation_to` answers which orders two intervals admit. FR-501's covers are
not all closed: the counter cover of a reading `k` is the half-open interval
`[k-u, k+1+u)` of the counter unit, and an interval stamp (FR-966) may be
open at either end (`earliest_open`, `latest_open`). Every cover is therefore
mapped, uniformly, to a closed integer interval in units of a quarter tick:
a closed endpoint `c` maps to `4c`, an open lower endpoint `a` to `4a+1` and
an open upper endpoint `b` to `4b-1`. The counter cover becomes
`[4(k-u), 4(k+1+u) - 1]`, an exact stamp `k` becomes `[4k, 4k]`, a bounded
cover `[t-u, t+u]` becomes `[4(t-u), 4(t+u)]`, and every bound `b` the
comparison reads is scaled to `4b`.

Quarter ticks keep every strict comparison strict. An open end moves its
endpoint inward by one quarter tick, less than the distance to the next whole
tick, so it never reaches a value a closed endpoint could hold. A difference
of two covers reads one endpoint of each, so a doubly-open difference moves
by at most two quarter ticks, less than the four between two whole-tick
thresholds: a difference whose true supremum is the integer `D` has a scaled
supremum in `[4D-2, 4D]`, and one whose true infimum is `D` a scaled infimum
in `[4D, 4D+2]`, so no scaled endpoint crosses an integer threshold `4b`
unless the true endpoint does. An open supremum `D = b` still lands at or
below `4b`, and an open supremum `D = b+1` lands at `4b+2`, above it. Half
ticks are not enough: an open lower end at `a` and an open upper end at `l`
give `2a+1` and `2l-1`, a difference of `2(l-a) - 2`, so a doubly-open
difference with supremum `l-a = B+1` against the bound `B` scales to exactly
`2B` and reads `True`, while the difference can lie anywhere in `(B, B+1)`
and the truth is `Indeterminate`. With the quarter-tick mapping the owner's
answers equal FR-501's on every reading: definitely-before of two covers, the
bound comparison `later - earlier <= b` as the order of `later` against
`earlier` shifted by `4b`, and the eligibility comparisons below. No owner
rule is added for it.

Scaling and shifting run in `i128` over the trace input's stamps and
uncertainties (`u64`) and a clause's bounds (64-bit integers). Every scaled
endpoint, scaled bound and shifted endpoint is then below `2^67` in
magnitude, far inside `i128`, so the mapping cannot overflow: every admitted
reading maps exactly, with no wrapping, saturation or refusal.

## Inputs

- The admitted trace of one request: each position's identity, stamp and
  value, the clock binding's name, unit and cover (FR-501), and the checked
  package's identity and digest.
- The request's progress assertion and the binding's ledger entry (FR-502).
- The activation scope, deliveries and origins (FR-524).
- A superseding or invalidating request's prior results and their inputs
  (FR-526).

## Outputs

The owner documents and views below, and FR-501's, FR-502's, FR-524's and
FR-526's reports, read from them.

## Behavior

### History and context

- Before any owner call, the evaluator SHALL admit the request's trace with
  `filament_observation::admit` as one `AdmissionRequest`:
  - `package`: the checked package's FR-322 identity, its revision, and the
    SHA-256 digest of its canonical bytes, format `NATIVE_LINKED_PACKAGE_FORMAT`;
  - `producer`: one static FSL producer bundle, identity `fsl-eval`, revision
    the `fsl-eval` crate version;
  - `binding`: the FR-502 ledger binding (package, clause node, occurrence,
    clock name, profile identity) spelled as the binding identity, the clock
    name as `source_identity`, the clause's trigger as `trigger_identity`, the
    clock unit as `unit`, and `required` true;
  - `records`: one `AdmittedRecord` per position, in trace order, its identity
    the position's FR-106 identity, its anchor the position's anchor in the
    clock family (`EventPosition`, `FixedSample` or `TimestampNanos`), its
    event time the stamp, its value the position's admitted value, its clock
    identity the clock name, its clock revision the profile identity and its
    uncertainty the cover's `u`;
  - `scope`: the window `[first stamp, last stamp]` of the trace, its required
    members the trace's positions.
- The evaluator SHALL keep a side mapping from each record's owner-derived
  identity to its position's FR-106 identity, built at admission, and SHALL
  report every owner answer (FR-501, FR-502, FR-524, FR-526) under the FR-106
  identities it maps to; no owner document carries an FR-106 identity.
- The tick-to-nanosecond conversion SHALL be exact: a stamp or `u` of `k`
  ticks of a timed unit is `k × s` nanoseconds, where `s` is the unit's
  declared rational factor to the second (FR-230's `time ... unit`) times
  10^9, so `ms` = `rational(1, 1000)` gives `s` = 1000000, with no
  rounding; a clock with no time unit (an event position or fixed sample
  count) takes `s` = 1. A product that is not an integer number of
  nanoseconds, or that exceeds the owner's nanosecond range, SHALL refuse the
  request at stage `admit`, `invalid_runtime_input`/`invalid-value`, naming
  the position, before any owner call.
- A refusal of `admit` SHALL refuse the request at stage `admit`,
  `invalid_runtime_input`/`invalid-value`, carrying the owner's error code;
  it is never ignored.
- Each owner call SHALL use `Context::new(History::batch(&qualified),
  &authority, &subject, revision, predecessor)` with `authority` the clause
  occurrence's key as definition identity, the package identity as definition
  revision and the package digest as definition digest; `subject` the trace
  window as scope identity and the admitted population identity; `revision`
  1 plus the number of accepted requests already recorded under the binding;
  and `predecessor` the binding's last accepted owner document of the same
  module, or none.

### Partial values and interval order

- Every cover SHALL map to a quarter-tick `EventTimeInterval::new(clock,
  profile, unit, earliest, latest)`, the unit identity the clock unit with
  the suffix `/4`, each closed endpoint `c` as `4c`, an open lower endpoint
  `a` (FR-966 `earliest_open`) as `4a+1` and an open upper endpoint `b`
  (FR-966 `latest_open`) as `4b-1`: a reading `k` under
  `ClockCover::Counter { uncertainty: u }` maps to `[4(k-u), 4(k+1+u) - 1]`,
  under `ClockCover::Exact` to `[4k, 4k]`, a point stamp `t` under
  `ClockCover::Bounded { uncertainty: u }` to `[4(t-u), 4(t+u)]`, and an
  observation's interval stamp, which overrides the cover (FR-501, FR-966),
  to its own endpoints mapped by their flags. These intervals are compared
  with each other and never written into an owner document.
- Every bound `b` SHALL scale to `4b`. Every scaled endpoint, scaled bound and
  shifted endpoint SHALL be computed exactly in `i128` from the `u64` stamps
  and uncertainties and the 64-bit bounds; bounded below `2^67` in
  magnitude, no such value overflows.
- `definitely_before(a, b)` SHALL be `relation_to(a, b).definitely_before()`.
- `within_bound(b, request, ack)` SHALL be read from `relation_to(ack,
  request shifted by 4b)`: `True` when the relation admits no `after`, `False`
  when it admits neither `before` nor `equal`, and `Indeterminate` otherwise.
- Eligibility (FR-501 "Positions, anchor and eligible instants") SHALL read
  the scaled difference of the candidate and the anchor, `[lo, hi]` with
  `lo` the candidate's scaled earliest minus the anchor's scaled latest and
  `hi` the candidate's scaled latest minus the anchor's scaled earliest
  (anchor and candidate swapped for a backward bound, and `[0, 0]` for a
  position's distance to itself), and SHALL decide each one-sided bound by
  its endpoints alone:
  - `at_least b`: eligible when `lo >= 4b`, ineligible when `hi < 4b`, and
    otherwise `Indeterminate` with reason `StraddlesBound`;
  - `at_most b`: eligible when `hi <= 4b`, ineligible when `lo > 4b`, and
    otherwise `Indeterminate` with reason `StraddlesBound`;
  - a window `[a, b]` is `at_least a` and `at_most b`, eligible when both
    are, ineligible when either is, and otherwise `Indeterminate`.

  No unbounded owner interval is formed for a one-sided bound: the
  comparison reads the two endpoints against `4b` and nothing else.
- A position's Kleene value SHALL map to the owner's `PossibilitySet`: `true`
  to `{can_be_true}` and `false` to `{can_be_false}`, reason `Known`;
  `indeterminate` for missing history (FR-501 `MissingHistory`) to both,
  reason `Missing`, and for a contradicted input to both, reason
  `Conflicting`. A partial fact SHALL be derived with `partial::derive` over
  the position's record and read back with `partial::read`.

### Completeness

- A retained `Complete { through: T }` SHALL map to a `completeness::Selection`
  whose required members are the positions stamped at or before `T`, each
  `Available` with its record, at the boundary `T` in the clock family's
  `TemporalBoundary` (watermark `T`); `Incomplete` maps to the same selection
  with the members not yet supplied `Incomplete`.
- Accepting, revising or refusing an assertion against a retained one SHALL be
  the owner's completeness revision rule below, and FR-502's `Completeness`
  contradiction SHALL be reported exactly when that rule refuses.

### Activation

- An activation scope SHALL map to an `activation::Selection`: the trigger
  delivery or origin as `ActivationSelection::new` (or `without_trigger` for a
  scope with none), the scope's closure as the closure authority
  (`OpenClosed`), the trigger record's completeness as the evidence authority,
  and the FR-524 captures as `Capture`s with their source observation
  identities.
- FR-524's disposition SHALL be read from the view: `Active`, `Inactive` and
  `Unknown` from `activation()`. The `Unknown` kind SHALL be read from the
  view's axes and the evaluator's own admission outcome: `Missing` when the
  trigger evidence is absent from the admitted history, `Refused` when the
  trigger record was refused at the evaluator's admission (FR-106), and `Open`
  when `closure()` is `Open` or `evidence()` is `Incomplete`; its completeness
  is `evidence()` and its execution the request's execution disposition.

### Repair

- A superseding or invalidating request (FR-526) SHALL map to
  `repair::plan(prior, successor, selection, limits)`: `prior` and `successor`
  the bundle views of the two requests' owner documents, the selection's
  authority revision the prior bundle's identity, one `FactRegion` per owner
  fact with the region of its positions, one `PriorResult` per retained result
  with its region and its canonical bytes, and one `DependencyEdge` from each
  fact to each result whose evaluation read it.
- The results FR-526 reports as superseded SHALL be exactly those the plan
  invalidates, and every retained result SHALL keep its bytes.

### Owner requirement: completeness revision

The owner's `completeness` module derives one assessment from per-member
facts and has no rule that compares a new assertion with a retained one.
FR-502 needs it, so `filament-observation` SHALL provide it, as its own
requirement (filament-observation FR-013, "Revise a completeness assertion
monotonically across revisions", the normative source of this rule; tracked as
QSL-2327). Where the summary below and FR-013 differ, FR-013 governs:

- `completeness::Selection` SHALL carry a `TemporalBoundary`, and
  `completeness::revise(prior: &View, context, selection, limits)` SHALL derive
  a successor document whose context names `prior`'s document as predecessor.
- `revise` SHALL waive the owner's same-subject predecessor check: each
  request carries its own admitted history, so the successor's scope and
  population identity are its own (filament-observation FR-013).
- `revise` SHALL refuse with a typed error naming the contradicted member or
  boundary when the successor's boundary is below the prior's, when a prior
  `Complete` assessment is succeeded by an `Incomplete` one, or when the
  successor holds a member at or before the prior boundary that the prior did
  not hold, or when a member both hold `Available` carries a different
  observation identity in the successor (`MemberObservationChanged`, which
  backs FR-502's changed-fingerprint refusal); it SHALL accept a boundary at or above the prior's and a member
  absent from the successor that the prior held, which stays held.
- `revise` SHALL leave `prior` unchanged on refusal.

Until QSL-2327 lands, `fsl-eval` keeps only the `Complete { through }`
revision rule, in the one function FR-861's tracked exception names, pinned
to the rule text above by FR-861-AC-4's agreement test; everything else uses
the owner.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-944-AC-1 | For readings 0 to 6 under `Counter { uncertainty: 1 }` and under `Exact`, with bounds 0 to 6, `definitely_before` and `within_bound` read through the owner's `relation_to` on the quarter-tick intervals (reading 3 maps to `[8, 19]` under `Counter { uncertainty: 1 }` and to `[12, 12]` under `Exact`, unit `ms/4`, and bound `b` shifts the request by `4b`) equal `ClockCover`'s answers on every pair. | Test (TC-2624) |
| FR-944-AC-2 | FR-501-AC-13's counter-cover trace admits through `filament_observation::admit` with one record per position whose identity is the owner's `record_identity`, and every report names the position's FR-106 identity through the side mapping; a `ms` stamp 3 admits as `event_time_nanos` 3000000, and stamp 3 under a declared unit of factor `rational(1, 2000000000)` (1/2 ns) refuses at stage `admit` naming its position; and the partial facts of its atoms read back through `partial::read` with the possibility sets and reasons of their Kleene values. | Test (TC-2624) |
| FR-944-AC-3 | FR-524-AC-27 to AC-29's scopes give `Active`, `Inactive` and the three `Unknown` kinds through `activation::derive` and the mapping above, equal to the dispositions the evaluator reports; FR-526-AC-2's superseding request gives a repair plan that invalidates exactly the superseded result and retains the other's bytes. | Test (TC-2624) |
| FR-944-AC-4 | Once filament-observation FR-013 lands, FR-502-AC-2's cases through `completeness::revise` refuse `Complete { through: 60 }`, `Incomplete` and the uncovered position stamped 80, refuse a covered position re-delivered under a different observation identity (`MemberObservationChanged`, FR-502's changed-fingerprint case), accept a successor over its own admitted history with another subject, and accept `Complete { through: 100 }` and `{ through: 140 }`, exactly as FR-502's ledger does. | Test (TC-2624) |
| FR-944-AC-5 | Under `Bounded { uncertainty: 0 }` and unit `ms`, a request `holds(req)` stamped 1 with interval `(0, 1]` maps to `[1, 4]` and an `ack` stamped 3 with interval `[3, 4)` maps to `[12, 15]`; `within_bound(3, req, ack)` read through `relation_to(ack, req shifted by 12)` is `Indeterminate`, and `holds(req) implies eventually[0,3] holds(ack)` with `W` and `Complete { through: 10 }` gives `Indeterminate { reason: StraddlesBound }`, equal to `ClockCover`'s answer, where the half-tick mapping (`[1, 2]`, `[6, 7]`, shift 6) would read `True`. The same request with the `ack` stamped 2 with interval `[2, 3)`, mapped to `[8, 11]`, gives `Completed(true)`. | Test (TC-2624) |
| FR-944-AC-6 | With an anchor of interval `[0, 0]` (`[0, 0]` in quarter ticks) under `Counter { uncertainty: 0 }`: for `at_least 2`, reading 2 (`[8, 11]`) is eligible, reading 1 (`[4, 7]`) ineligible, and reading 2 under `Counter { uncertainty: 1 }` (`[4, 15]`) `Indeterminate` with reason `StraddlesBound`; for `at_most 2`, reading 1 is eligible, reading 3 (`[12, 15]`) ineligible and reading 2 `Indeterminate` with reason `StraddlesBound`; a candidate of interval `(1, 2]` (`[5, 8]`) is eligible for `at_least 1`, one of interval `[0, 1)` (`[0, 3]`) is ineligible for `at_least 1` and `Indeterminate` for `at_most 0`; and against an anchor of interval `(0, 1]` (`[1, 4]`) a candidate of interval `[3, 4)` (`[12, 15]`) is `Indeterminate` for `at_most 3` and eligible for `at_least 2`. Each decision equals `ClockCover`'s eligibility, and no owner interval with an unbounded end is formed. | Test (TC-2624) |
| FR-944-AC-7 | Scaling is exact at the domain's edge: a reading `18446744073709551615` under `Counter { uncertainty: 18446744073709551615 }` maps to `[0, 147573952589676412923]`, reading 0 under the same cover to `[-73786976294838206460, 73786976294838206463]`, and the bounds `9223372036854775807` and `-9223372036854775808` scale to `36893488147419103228` and `-36893488147419103232`; `within_bound` and `definitely_before` over these readings and bounds equal `ClockCover`'s answers. | Test (TC-2624) |

## Dependencies

- FR-861: one owner for the four rule sets.
- FR-501, FR-502, FR-524, FR-526: the reports read from the owner's answers.
- filament-observation FR-007, FR-008, FR-010 and the owner requirement
  above (filament-observation FR-013).
