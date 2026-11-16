---
id: FR-922
title: "Certify strict-clock verdicts over clock regions"
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
  - target: ix://agent-ix/filament-specification/FR-752
    type: depends_on
---
# FR-922: Certify strict-clock verdicts over clock regions

## Description

When a guard, time invariant, timed interval or elapsed comparison of a timed
claim is strict, EN-5 SHALL decide the claim over the region MDP (FR-204
Admission), certify it with a `route: "region"` document (FR-920), and the
checker SHALL rebuild the region MDP itself (FR-921). The digital MDP gives a
wrong value once a constraint is strict: a guard `x > 1` excludes the delay 1
that digital clocks would take, so the strict case has its own route and its
own certificate.

## Use case

With `send` guarded `self.x > 1 ms`, `Retx`'s best scheduler sends just after
each 1 ms, fitting three attempts, not four, in 4 ms. The maximum is
`999/1000`, which the region MDP attains; the digital value `9999/10000` is
unsound, and the checker rejects a certificate that states it.

## Inputs

- A timed claim with a strict constraint and EN-5's region result.

## Outputs

- A `route: "region"` document, or for an expected-elapsed form a
  boundary-region one; FR-921's check result.

## Behavior

- The region MDP's states SHALL be FR-920's region keys. A delay move SHALL
  go from a region to its time successor and be admissible when the time
  invariants hold on the successor and the state is not urgent; a discrete
  move SHALL apply its guard to the region, which decides each
  diagonal-free constraint exactly, and a reset SHALL set the clock's integer
  part to the scaled reset value and its fraction to `"0"`.
- An infimum or supremum that no dense scheduler attains (a delay "just
  after 1 ms") SHALL be stated as the value the region MDP attains; the
  verdict compares that value with the threshold as FR-203 does.
- Expected-elapsed forms SHALL use boundary-region keys: the region MDP's
  states paired with corners, each delay edge carrying the exact elapsed time
  between corners, whose minimum and maximum are the dense infimum and
  supremum.
- The checker SHALL refuse `ObjectiveMismatch` for a digital document over a
  claim with a strict constraint, and for a region document over a claim with
  none.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-922-AC-1 | `Retx` with the guard `self.x > 1 ms` over every scheduler: `Deadline` at 0.99 settles `proved`, `ExactValue{99/100}`, with a `region` document the checker accepts; `Max` (`probability <= 0.999`) settles `proved`, `ExactValue{999/1000}`, with a `region` document the checker accepts. | Test (TC-2491) |
| FR-922-AC-2 | AC-1's `Deadline` document with `route` set to `"digital"` is refused `ObjectiveMismatch`; the `Max` document with every value replaced by the corresponding value of the `>= 1 ms` guard's digital product, initial value `9999/10000`, is rejected `InequalityFails`, and the tampered item settles `inconclusive`, `CertificateRejected`. | Test (TC-2491) |
| FR-922-AC-3 | `MeanTime` with the guard `self.x > 1 ms` settles its maximum `20/9 ms` with a `boundary-region` document the checker accepts; the minimum is the infimum `10/9 ms`. | Test (TC-2491) |

## Dependencies

- [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  Admission (strict constraints); FSpec FR-752.
