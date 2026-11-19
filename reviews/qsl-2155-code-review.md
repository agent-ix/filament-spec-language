---
id: SR-2026111612172400000046
title: "Code review — filament-spec-language QSL-2155 Clopper–Pearson"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/statistical.rs, fsl-analyze/src/statistical/method.rs, fsl-analyze/src/statistical/tests.rs, fsl-analyze/tests/it/main.rs, fsl-analyze/tests/it/statistical_coverage.rs, fsl-replay/src/statistical.rs; QSL-2155 acceptance criteria; FR-189; ADR-024 ST-9 and ST-11"
review_set: subset
---

## Summary

Reviewed the frozen PR #2966 diff, including its Rust implementation, tests, affected caller paths, and the QSL-2155 acceptance criteria. The new method has a valid-input decision edge that returns the wrong result, omits required clause-meter charging, and its claimed coverage test measures a deterministic event rather than an interior probability.

## Verdict

**CONDITIONAL** — three medium findings remain; no aggregate gates were run. The coder-reported focused tests passed: fsl-replay method admission (1), fsl-analyze Clopper–Pearson unit tests (3), and the statistical coverage integration filter (2). No whole `it` target was run.

## Scope examined

| Unit | Role | Excerpt |
| --- | --- | --- |
| QSL-2155 AC-1 | examined | `StatisticalMethod` has a ClopperPearson member selectable from source: `fsl-analyze/src/statistical/tests.rs`, `clopper_pearson_selectable`. |
| QSL-2155 AC-2 | examined | On a model with known exact p the interval contains p at rate at least 1-alpha over many seeds: `fsl-analyze/tests/it/statistical_coverage.rs`, `clopper_pearson_coverage` (shares the suite of T-082, owned by QSL-2158). |
| QSL-2155 AC-3 | examined | The same seed gives the same verdict and sample count: `clopper_pearson_reproducible`. |
| QSL-2155 AC-4 | examined | Interval endpoints match the published values for a fixed (successes, trials) table: `clopper_pearson_known_intervals`. |
| QSL-2155 AC-5 | examined | Witness replay accepts a Clopper-Pearson verdict: `fsl-replay` statistical witness tests, `clopper_pearson_witness_replays`. |
| FR-189 | examined | Each tail evaluation SHALL charge its `N + 1` terms to the clause meter. |
| ADR-024 ST-9 | examined | ST-11's binomial tails are pairs of binary floating bounds of `96 + ⌈log2(N + 1)⌉` bits, rounded outward at every operation, so each lies within a relative `2^-90` of the true tail; a test decides only on a tail's upper bound, and its interval ends are the outward bisection bracket ends. |
| ADR-024 ST-11 | examined | A `>= θ` test accepts when `G(k, θ) <= β'`, rejects when `L(k, θ) <= α'`, and is undecided (`IndifferenceRegion`) when neither holds or both do (both only when `α' + β' > 1`); `<= θ` swaps the tails, and the strict reduced bound `< q` of a quantile `>= c` claim accepts only when `L(k, q) < β'`. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Clopper–Pearson returns Accepted when both tail conditions hold, contrary to ST-11's required Undecided result. | fsl-analyze/src/statistical/method.rs:267-278 |
| FND-002 | medium | Binomial-tail recurrence terms are not charged to the clause meter, despite FR-189 requiring N + 1 charges for every evaluation. | fsl-analyze/src/statistical/method.rs:144-225 |
| FND-003 | medium | The coverage test uses `always[0,0]` on a coin whose value starts at zero, then checks coverage for p=1, so it does not exercise the claimed p=1/2 coverage. | fsl-analyze/tests/it/statistical_coverage.rs:58-78 |

## Finding details

### FND-001 — both tails true

With a valid request using α=β=0.9, θ=1/2, iota=1/10, N=6 and k=3, both inclusive binomial tails equal 42/64 and are at most their thresholds. The `if accepted` branch wins and emits `Accepted`; ADR-024 ST-11 explicitly requires `Undecided(IndifferenceRegion)` when both hold. The request parser allows both confidence values in (0,1), so this is reachable.

### FND-002 — unmetered tail work

`binomial_tail` has no meter input, and both recurrence loops process the distribution terms without charging. Decision evaluation invokes two tails; interval evaluation invokes up to 128 more tail evaluations through bisection. This leaves the work required by FR-189 outside its clause-meter budget.

### FND-003 — degenerate coverage oracle

`prob_world::coin` initializes `c.v` to 0. The test's `always[0,0] holds(c.v = 0)` checks only the initial state, before the randomized deletion step, making the event probability 1. It then compares each interval with `ratio(1, 1)`, while the test comment and ticket AC describe known p=1/2. The interval coverage assertion therefore does not validate coverage at the intended interior probability.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The decision now checks `accepted && rejected` first and returns `Undecided(IndifferenceRegion)`. |
| FND-002 | fixed | `binomial_tail` now receives the request meter and charges `n + 1` work units once per evaluation; decision and interval calls pass the same meter through. |
| FND-003 | fixed | The fixture now flips `v` to an equally weighted random bit and checks `eventually[1,1] holds(c.v = 1)` against the known p=1/2. |

### Disposition evidence

FND-001 after_excerpt:

```rust
Ok(if accepted && rejected {
    Decision::Undecided(UndecidedCause::IndifferenceRegion)
} else if accepted {
    Decision::Accepted
} else if rejected {
    Decision::Rejected
} else {
    Decision::Undecided(UndecidedCause::IndifferenceRegion)
})
```

FND-002 after_excerpt:

```rust
meter
    .charge(Charge::new(ChargePoint::CollectionVisit).work(Integer::from(n.saturating_add(1))))
    .map_err(|_| BINOMIAL_METER)?;
```

FND-003 after_excerpt:

```rust
post Flipped using v on M::Coin::flip { self.v = b }
random Coin::flip(b) ~ { 0: 1, 1: 1 };
```

The test sets `true_probability = ratio(1, 2)` and uses `eventually[1,1] holds(c.v = 1)`.
