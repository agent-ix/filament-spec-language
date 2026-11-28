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
  - target: ix://agent-ix/filament-spec-language/ADR-038
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-421
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

When FR-204 Admission "Route" does not give a claim the digital MDP, EN-5
SHALL decide it over the region MDP in the same way, with a `route: "region"`
document, even when every constraint is closed (FR-204-AC-26, ADR-038).

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
  after 1 ms") SHALL be certified as an unattained bound: the certificate
  states the value and `attained: false`. FR-203's comparison of a value with a
  threshold applies, with the strictness decided by attainment: an attained
  value `v` meets `<= c`, `< c`, `>= c` and `> c` as FR-203 compares `v` with
  `c`; an unattained supremum `S` meets "at most S" and "less than S" and fails
  "at least S" and "greater than S"; an unattained infimum `I` meets "at least
  I" and "greater than I" and fails "at most I" and "less than I". A
  refutation at or beyond the bound carries any concrete run; a refutation
  strictly inside `(I, S)` carries a concrete run whose recomputed value
  crosses the threshold.
- A refutation of an expected-elapsed claim SHALL carry a finite induced
  Markov chain: states with exact rational clock valuations, for each state
  its chosen exact rational delay and its branches (target state, exact
  rational probability), and the claimed value of the initial state. The
  checker SHALL rebuild the chain through the timed system's own delay, guard
  and reset semantics, require each stated target to equal the recomputed one,
  solve the chain's expected elapsed time itself in exact rationals, and
  compare the solved value with the threshold under the claim's strictness. A
  chain with one branch of probability 1 per state and no cycle is the path
  case. A chain beyond a limit settles `incomplete` through `LimitReached`
  naming the setting.
- Expected-elapsed forms SHALL use boundary-region keys: the region MDP's
  states paired with corners, each delay edge carrying the exact elapsed time
  between corners, whose minimum and maximum are the dense infimum and
  supremum.
- Interim, until the symbolic region MDP of ADR-038 RL-3 replaces it
  (QSL-3495): a claim whose only reason for the region route is a strict
  constraint (`Region(Strict)` in the route admission table) SHALL be decided
  by the existing grid walk, and its verdict is only as sound as the dense
  oracle rows that pin it. Those rows are the strict `two_clocks` rows
  `row_3444_two_clock_regions_minimum_routes`,
  `row_3444_two_clock_regions_maximum_routes`,
  `row_3444_two_clock_regions_bounded_minimum_routes` and
  `row_3444_two_clock_regions_bounded_maximum_routes` (FR-922-AC-4 subject A)
  and the strict time-lock row `row_3468_time_lock_strict_routes` with its
  closed twin `row_3468_time_lock_closed_early_routes` (a locked run is read
  as FSpec FR-416 "How claims read a time-lock" states, ADR-038 RL-7). The
  closed twin `row_3468_time_lock_closed_later_routes` waits on the region
  MDP (QSL-3495) and is ignored until then; its live companion
  `row_3468_time_lock_closed_later_settles_incomplete` pins the interim
  outcome. The interim's stated limit: the grid walk is not checked against
  the dense oracle for a claim outside these rows, and a wrong verdict there
  is a defect of the interim, closed by the region MDP of QSL-3495 (After
  alpha).
- EN-5 SHALL build a region product only within three limits, each a setting
  of FR-255: `exact_prob.max_clocks` (the model clocks plus the claim clock,
  default 10 clocks), `exact_prob.max_clock_scale` (the plain LCD scale of the
  constraint constants, default 1048576 ticks per base unit) and
  `exact_prob.max_clock_cap` (the largest constraint constant, default 1048576
  ticks). A subject over one of them SHALL settle `incomplete` through
  `LimitReached`, naming that setting and its bound, before any state is built
  and never as a verdict.
- The checker SHALL refuse `ObjectiveMismatch` for a digital document over a
  claim the region route decides (FR-204 Admission "Route"), and for a region
  document over a claim the digital route decides.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-922-AC-1 | `Retx` with the guard `self.x > 1 ms` over every scheduler: `Deadline` at 0.99 settles `proved`, `ExactValue{99/100}`, with a `region` document the checker accepts; `Max` (`probability <= 0.999`) settles `proved`, `ExactValue{999/1000}`, with a `region` document the checker accepts. | Test (TC-2491) |
| FR-922-AC-2 | AC-1's `Deadline` document with `route` set to `"digital"` is refused `ObjectiveMismatch`; the `Max` `exact` certificate with every value replaced by the corresponding value of the `>= 1 ms` guard's digital product, initial value `9999/10000`, is rejected `InequalityFails`, and the tampered item settles `inconclusive`, `CertificateRejected`. | Test (TC-2491) |
| FR-922-AC-3 | `MeanTime` with the guard `self.x > 1 ms` settles its maximum `20/9 ms` with a `boundary-region` document the checker accepts; the minimum is the infimum `10/9 ms`. | Test (TC-2491) |
| FR-922-AC-4 | Interim, until the symbolic region MDP of ADR-038 RL-3 replaces the grid walk (QSL-3495): RL-3 gives a grid walk no exactness, and this criterion claims none; it pins that the walk does not step over a region on the subjects below, whose stated values are derived by hand for dense time. Subject A: clocks `x` and `y`; `first` is guarded `self.phase = 0 and self.x > 0 ms and self.x < 1 ms`, sets `phase` 1 and resets `x`; `third` is guarded `self.phase = 1 and self.x < 1 ms and self.y > 1 ms` and sets `late`; `y <= 3 ms` while `phase < 2`. Over every scheduler `eventually[0 ms, 4 ms] holds(late)` has maximum 1 (`first` at 0.5 ms, `third` at 1.2 ms) and minimum 0 (`third` never taken): `probability <= 0.999`, `probability <= 0.5` and `probability >= 0.001` each settle `refuted`. Subject B: `first` is guarded `self.x > 0 ms`, `third` as before, with `y < 1 ms` while `phase = 0` and `x < 1 ms` while `phase = 1`, so time diverges only through `third`: `probability >= 0.5` settles `proved`, `ExactValue{1}`. Subject C is B with `x <= 1 ms` in the guard of `third` and in the invariant, with the same result. | Test (TC-2491) |
| FR-922-AC-5 | With subject B's expected elapsed time (infimum 1 ms, supremum 2 ms, neither attained): "at most 2 ms", "less than 2 ms", "at least 1 ms" and "greater than 1 ms" settle `proved`; "at least 2 ms", "greater than 2 ms", "at most 1 ms" and "less than 1 ms" settle `refuted`. | Test (TC-2491) |
| FR-922-AC-6 | A refutation of an expected-elapsed claim carries a finite chain with exact rational delays and probabilities and its value; the checker accepts it only if its own recomputation of that value violates the claim. | Test (TC-2491) |
| FR-922-AC-7 | The checker recomputes the value of a refuting chain from the replayed chain alone; a chain whose stated value differs from the recomputed one, whose branch probabilities do not sum to 1, or whose stated state differs from the replayed state is refused. | Test (TC-2491) |
| FR-922-AC-8 | `Retx` with the guard `self.x > 1 ms` and `exact_prob.max_clocks` below its clock count settles `incomplete`, `LimitReached` naming `exact_prob.max_clocks` and its bound; with the bound equal to its clock count the claim builds and settles as in AC-1. The same holds for `exact_prob.max_clock_scale` against the plain LCD scale and `exact_prob.max_clock_cap` against the largest constraint constant. | Test (TC-2491) |

## Dependencies

- [FR-204](FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)
  Admission (strict constraints); FSpec FR-752.
