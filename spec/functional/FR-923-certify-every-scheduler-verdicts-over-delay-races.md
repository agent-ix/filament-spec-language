---
id: FR-923
title: "Certify every-scheduler verdicts over delay races"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-023
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-204
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-920
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-752
    type: depends_on
---
# FR-923: Certify every-scheduler verdicts over delay races

## Description

Over every scheduler, a `discrete` delay SHALL apply as under a workload
(FR-204), the scheduler keeping every choice a workload's weights would make,
and EN-5 SHALL certify the result in `timed-product/v1` with
`optimization` `minimum` or `maximum`; the checker SHALL rebuild each race and
each scheduler choice itself.

## Use case

`Retx` declares `delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };` and
claims `Deadline` under every scheduler. `send` is the only identity, so the
scheduler has nothing left to choose: minimum and maximum are both
`159129/160000`, the workload `One`'s value. A certificate whose values read
`send`'s delay as the scheduler's free choice in `[1, 2]` ms (minimum
`99/100`) does not match the race the checker rebuilds and is rejected.

## Inputs

- A timed claim under every scheduler over a subject with `discrete` delays.

## Outputs

- A `timed-product/v1` document; FR-921's check result.

## Behavior

- At a digital or region state where an identity with a `discrete` delay
  races, every racing identity's delay SHALL be drawn from its distribution
  conditioned on its window, as ADR-026 SD-3 states, and the least delay `m`
  wins.
- An identity with no `delay` declaration SHALL have its delay chosen by the
  scheduler after the draw, as FR-196's `FreeDelays` rows give it.
- When two or more identities tie at `m`, the scheduler SHALL choose the
  winner after the draw. A `FreeDelays` row under every scheduler SHALL carry
  that choice as `tie`, the winning identity, present exactly when the row's
  race ties; a row without `tie` at a tie, or naming an identity not tied at
  `m`, refuses `invalid_runtime_input`/`invalid-value` at replay. Without a
  workload no weight orders a tie, and a choice among enabled identities is
  the scheduler's (FSpec FR-406).
- Each action SHALL be one function from drawn vectors to a row; its
  distribution is `Σ_g P(g)` times the row's race outcome, and FR-204's
  row-factored operator evaluates it. The checker SHALL rebuild the drawn
  vectors, their probabilities and every action from the checked
  `delay_distribution` nodes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-923-AC-1 | `Retx` (guard `self.x >= 1 ms`) with `delay M::Call::send ~ discrete { 1 ms: 1, 2 ms: 1 };`: `Deadline` over every scheduler settles `proved`, `ExactValue{159129/160000}`, which is both its minimum and its maximum, with a document the checker accepts; `probability >= 0.995` over the same event settles `refuted`, its minimum `159129/160000` below the threshold. | Test (TC-2492) |
| FR-923-AC-2 | AC-1's `Deadline` item, its certificate values replaced by those of the same model without the `delay` line (initial value `99/100`), settles `inconclusive`, `CertificateRejected` with rule `InequalityFails`. | Test (TC-2492) |
| FR-923-AC-3 | `Retx` (guard `self.x >= 1 ms`, reset) with `delay M::Call::send ~ discrete { 2 ms: 1 };` and an operation `tick` with no `delay` declaration, guarded `not self.late and not self.retried and self.x >= 2 ms` and setting `retried`, so that `tick` can fire only at `x = 2 ms`, tied with `send`: under every scheduler the maximum of `eventually[0 ms, 5 ms] holds(c.retried)` is `ExactValue{1}` (the scheduler gives `tick` the tie at 2 ms, its witness row naming `tie` `tick`), so `probability <= 0.6` settles `refuted`; under the workload `Both` weighting `send` and `tick` 1 the value is `21/40` (`1/2` at 2 ms, then `1/2 · 1/10 · 1/2` at 4 ms after a loss) and the same claim settles `proved`. | Test (TC-2492) |

## Dependencies

- [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  Admission (delay distributions over every scheduler);
  [FR-196](FR-196-build-the-probabilistic-product.md) `FreeDelays`; FSpec
  FR-752.
