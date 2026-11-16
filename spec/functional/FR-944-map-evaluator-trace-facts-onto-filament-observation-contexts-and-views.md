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

### Decision: an uncertain cover is a closed interval in half-ticks

The owner's `EventTimeInterval` is closed with integer endpoints, and its
`relation_to` answers which orders two intervals admit. FR-501's counter cover
of a reading `k` is the half-open interval `[k-u, k+1+u)` of the counter unit.
Mapping it to the closed integer interval `[2(k-u), 2(k+1+u) - 1]` in units of
half a tick, and an exact stamp `k` to `[2k, 2k]`, makes the owner's answers
equal FR-501's on every reading: definitely-before of two covers, and the
bound comparison `later - earlier <= b` as the order of `later` against
`earlier` shifted by `2b`. No owner rule is added for it.

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

- A reading `k` under `ClockCover::Counter { uncertainty: u }` SHALL map to
  `EventTimeInterval::new(clock, profile, unit, 2(k-u), 2(k+1+u) - 1)`, and
  under `ClockCover::Exact` to `[2k, 2k]`, the unit identity the clock unit
  with the suffix `/2`. These intervals are compared with each other and never
  written into an owner document.
- `definitely_before(a, b)` SHALL be `relation_to(a, b).definitely_before()`.
- `within_bound(b, request, ack)` SHALL be read from `relation_to(ack,
  request shifted by 2b)`: `True` when the relation admits no `after`, `False`
  when it admits neither `before` nor `equal`, and `Indeterminate` otherwise.
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
monotonically across revisions", not yet filed upstream; tracked as
QSL-2327):

- `completeness::Selection` SHALL carry a `TemporalBoundary`, and
  `completeness::revise(prior: &View, context, selection, limits)` SHALL derive
  a successor document whose context names `prior`'s document as predecessor.
- `revise` SHALL refuse with a typed error naming the contradicted member or
  boundary when the successor's boundary is below the prior's, when a prior
  `Complete` assessment is succeeded by an `Incomplete` one, or when the
  successor holds a member at or before the prior boundary that the prior did
  not hold; it SHALL accept a boundary at or above the prior's and a member
  absent from the successor that the prior held, which stays held.
- `revise` SHALL leave `prior` unchanged on refusal.

Until QSL-2327 lands, `fsl-eval` keeps only the `Complete { through }`
revision rule, in the one function FR-861's tracked exception names, pinned
to the rule text above by FR-861-AC-4's agreement test; everything else uses
the owner.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-944-AC-1 | For readings 0 to 6 under `Counter { uncertainty: 1 }` and under `Exact`, with bounds 0 to 6, `definitely_before` and `within_bound` read through the owner's `relation_to` on the half-tick intervals equal `ClockCover`'s answers on every pair. | Test (TC-2624) |
| FR-944-AC-2 | FR-501-AC-13's counter-cover trace admits through `filament_observation::admit` with one record per position whose identity is the owner's `record_identity`, and every report names the position's FR-106 identity through the side mapping; a `ms` stamp 3 admits as `event_time_nanos` 3000000, and stamp 3 under a declared unit of factor `rational(1, 2000000000)` (1/2 ns) refuses at stage `admit` naming its position; and the partial facts of its atoms read back through `partial::read` with the possibility sets and reasons of their Kleene values. | Test (TC-2624) |
| FR-944-AC-3 | FR-524-AC-27 to AC-29's scopes give `Active`, `Inactive` and the three `Unknown` kinds through `activation::derive` and the mapping above, equal to the dispositions the evaluator reports; FR-526-AC-2's superseding request gives a repair plan that invalidates exactly the superseded result and retains the other's bytes. | Test (TC-2624) |
| FR-944-AC-4 | Once filament-observation FR-013 lands, FR-502-AC-2's cases through `completeness::revise` refuse `Complete { through: 60 }`, `Incomplete` and the uncovered position stamped 80, and accept `Complete { through: 100 }` and `{ through: 140 }`, exactly as FR-502's ledger does. | Test (TC-2624) |

## Dependencies

- FR-861: one owner for the four rule sets.
- FR-501, FR-502, FR-524, FR-526: the reports read from the owner's answers.
- filament-observation FR-007, FR-008, FR-010 and the owner requirement
  above (filament-observation FR-013).
