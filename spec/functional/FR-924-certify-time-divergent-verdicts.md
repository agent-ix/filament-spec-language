---
id: FR-924
title: "Certify verdicts over time-divergent behaviours"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-920
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-753
    type: depends_on
---
# FR-924: Certify verdicts over time-divergent behaviours

## Description

EN-5 SHALL decide a timed claim over the time-divergent behaviours only
(FR-204 Admission, zero-delay cycles and time-locks) and certify it in
`timed-product/v1`; the checker SHALL find the zero-time end components and
the time-locks itself and apply FR-201's conditions to the product in which
each such component is one state with only its exits.

## Use case

`Retx` gains an operation `toggle` that is always enabled, takes no time and
flips `retried`. A scheduler that toggles forever at time 0 never delivers,
so the plain minimum condition would accept the value 0. That scheduler is
Zeno and not a behaviour; over the time-divergent schedulers the minimum of
`Deadline` stays `99/100`. The checker finds the toggle pairs, requires one
value per pair equal to its best exit, and rejects the Zeno certificate.

## Inputs

- A timed claim and EN-5's result.

## Outputs

- A `timed-product/v1` document; FR-921's check result.

## Behavior

- A zero-time action SHALL be one whose every outcome takes no time: a
  discrete move, a `WorkloadResolve`, a race won at delay 0, or
  `IdleObserve`. The checker SHALL compute, by its own end-component
  decomposition, the maximal end components of the rebuilt product whose
  actions are all zero-time and whose states are undecided.
- All states of such a component SHALL carry one value. Its exits are the
  enabled actions at its states that are not actions of the component. For
  a minimum the value SHALL equal the least exit value, for a maximum the
  greatest, for an expected elapsed time the same with each exit's reward,
  and ranks SHALL be checked with the component as one state. Actions that
  stay in the component neither bound the value nor give progress.
- On the region route a positive-time action does not by itself make time
  diverge: a loop that resets one clock and delays it into a region below
  another clock's fraction returns to the same regions while the total delay
  converges. The checker SHALL apply the Alur–Dill progress condition: an end
  component is progressive when, for every clock entry of the key (the
  deadline and elapsed clocks included), an action of the component resets it,
  a delay of the component takes it across an integer boundary, or it is at
  its cap at a state of the component. Every maximal end component that is not
  progressive SHALL be checked as a zero-time component above, with one value
  and its exits; a progressive end component SHALL keep FR-201's plain
  conditions, because a scheduler may stay in it forever with time diverging.
  On the digital route every delay adds a whole unit, so every end component
  that holds a delay is progressive and only zero-time components collapse.
  The progress condition was chosen over a unit tick clock in the region key
  because it needs no extra key member and leaves digital keys unchanged.
- A component with no exit, and a state with no admissible delay and no
  enabled discrete move, SHALL be a time-lock: a terminal state, value 0 for
  an event not yet met and no further elapsed reward, a pending deadline
  rejected.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-924-AC-1 | `Retx` (guard `self.x >= 1 ms`, reset) with `toggle` (pre `true`, post `self.retried = not self.retried`): `Deadline` at 0.99 over every scheduler settles `proved`, `ExactValue{99/100}`, with a document the checker accepts in which the two states of each toggle pair carry one value; `Max` settles `ExactValue{9999/10000}`. | Test (TC-2493) |
| FR-924-AC-2 | AC-1's `Deadline` item, every certificate value strictly between 0 and 1 set to 0 (the toggle-forever value), settles `inconclusive`, `CertificateRejected`, rule `InequalityFails` at a state of a toggle component. | Test (TC-2493) |
| FR-924-AC-3 | `probabilistic MeanLow using inf on M over (c: M::Call) under every scheduler on origin { expected elapsed until holds(c.late) >= 1 ms }` over `Retx` settles `proved` on the minimum `10/9 ms`; the item whose `exact` certificate has the initial value set to `1 ms` (the retry cycle, which holds delays, read as taking no time) settles `inconclusive`, `CertificateRejected`, rule `InequalityFails`. | Test (TC-2493) |
| FR-924-AC-4 | The `Retx` variant whose `send` has no guard and no reset (FR-204-AC-16): `Deadline` settles `proved`, `ExactValue{1}`, with a document the checker accepts, no component made transient, because every action leaves its zero-time cycle with positive probability. | Test (TC-2493) |
| FR-924-AC-5 | `Retx` under `One` with `delay M::Call::send ~ discrete { 5 ms: 1 };` (FR-204-AC-17) is time-locked at its initial state: `UnderMax` (`probability <= 0.999`) settles `proved`, `ExactValue{0}`, with a document the checker accepts; the item with that state's value set to 1 settles `inconclusive`, `CertificateRejected`, rule `InequalityFails`. | Test (TC-2493) |
| FR-924-AC-6 | `Retx` (guard `self.x >= 1 ms`, reset) with a second clock `y` and an operation `again`, guarded `not self.late and self.y < 1 ms` and setting `self.y = 0 ms`, whose strict guard selects the region route: `Deadline` at 0.99 over every scheduler settles `proved`, `ExactValue{99/100}`, with an accepted `region` certificate. The loop "reset `y`, delay into `0 < frac(y) < frac(x)`, reset `y`" crosses no integer for `x` and is not progressive; the item whose `exact` certificate has every value strictly between 0 and 1 set to 0 settles `inconclusive`, `CertificateRejected`, rule `InequalityFails`. | Test (TC-2493) |

## Dependencies

- [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  Admission; ADR-026 TS-4, TS-5, TD-2; FSpec FR-753.
