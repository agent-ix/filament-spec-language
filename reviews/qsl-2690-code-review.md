---
id: SR-2026111919340478491681421
title: "code-review of QSL-2690 exact replay at algebraic activation instants"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2690; PR3167 head 73f0a97484893149817c4d723f2a1d0b01740f5f; git diff origin/main...HEAD (base 29e24b46ca); fsl-replay/src/hybrid_replay.rs; fsl-replay/src/hybrid_replay/{exact.rs,exact/activation.rs,poly2.rs,section.rs,tests/activation_tests.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2690 (PARTIAL), PR filament-spec-language#3167, frozen head 73f0a97484893149817c4d723f2a1d0b01740f5f, scope `git diff origin/main...HEAD` over base 29e24b46ca. Code-review with the rust-review lane folded in. The change evaluates an obligation at an irrational activation instant `tau` through the section `t = tau` (new `Sample::of`, `Section::sign_of`, `poly2::eliminate`, `Atoms` evaluator, `formula_signal_over`) and narrows `ExactGap::ActivationInfimum` by resolving a moving no-value instant along `x = tau + c` (new `Decider::over`).

## Verdict

NOT MERGEABLE. FND-001 is a wrong answer with no gap.

What holds: `Sample::of` maps an `ExactReal::Root` (square-free, one root in the open interval, ends non-roots) onto `Root2` with a polynomial constant in `t`, which meets `Root2`'s contract. `sign_at_root` is a Sturm-Tarski query `TaQ(p, P; low, high)`, valid for any `P`, with every coefficient sign decided exactly at `u`. So signs at `(tau algebraic, x algebraic)` are exact. `eliminate` takes the formal Sylvester determinant, whose `x`-degree is at most `n * dx`, at `n * dx + 1` points. That is right, and a vanishing `t`-leading coefficient only adds roots. When `p(tau, .)` vanishes identically, its sign is `Equal` everywhere, so it needs no split. Multiple roots are roots of the resultant.

In a final open cell, `over` identifies the curve soundly. The projection keeps every coefficient, the discriminant PSCs and the pairwise PSCs of the read set, the lines (window offsets and segment ends) and the shifted copies, so at most one root curve passes through `(tau, time)`. A wrong branch can only be picked on a cell that is later split, and that result is discarded. The `x = tau + c` claim is right for either sign of `c`: the infimum over the open cell `(a, b)` is `a + c`, not attained, and `cell.0.shifted` handles an irrational `a`.

I recomputed by hand: Cube/CubeHeld (false only at `4^(1/3)`), Touch (`tau = 4^(1/3) - 1`), Meet/MeetHeld (`c + 1` inside `[c, c + 2]`, `c + 3` outside), Moves 0, MovesOne 1 and Fixed 2. All match.

I ran an independent probe of 22 new claims over Ramp. Hand expectations held for irrational guard ends with `<`/`<=`/`!=` at `(sqrt 5 - 1)/2`, a nested `eventually [0,1] always [0,1]` false exactly at the irrational instant, `x = tau - 1/2`, an open and a closed irrational cell start (`sqrt 2 + 1`), single-instant algebraic guards `q.v^3 = 4`, and honest `NotExact` for `x = 2 tau`, two curves and a crossing. The exceptions are FND-001, FND-002 and FND-003.

Mutation: each of the six new charge sites I deleted (`Atoms::at`, the `eliminate` closure, the `at_algebraic` capture, segment and projection charges, and `poly2::eliminate`'s per-point charge) fails `the_activation_scan_is_charged_to_the_meter`.

Gates: `-p fsl-replay --lib hybrid` 77 + probe pass; `hybrid_requests::` 11 pass, 12 ignored; `hybrid_provider::` 8; `-p fsl-hybrid-reference --lib` 3. Clippy (cap-lints warn) shows nothing in `hybrid_replay`, and rustfmt is clean. There are no floats, no `#[allow]` and no new `pub` items; the only public change is the `ActivationInfimum` doc.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `tau`'s polynomial is only square-free, not irreducible. When a read polynomial shares a factor in `t` with it, `eliminate` returns the zero polynomial, so the atom's cells are never split at the roots of `p(tau, .)` and one sample decides the whole dwell: a wrong answer with no gap. Reproduced: guard `q.v > 1 and q.v < 2`, `always [0, 5] holds((v0 - 7) * (v0^3 - 4) != 0 or (v0 - 7) * (h.v - 3) < 0)` over Ramp returns `NotReproduced`. The right answer is `Reproduced`: at `tau = 4^(1/3)`, `x = tau` gives `(tau - 7)(tau - 3) > 0`. The `tau` root carries `(t - 7)(t^3 - 4)` and the resultant with `(t - 7)(x - 3)` is identically 0. Rational cuts that reach `at_algebraic` as `Root`s of reducible polynomials (seen: `t^2 - 10t + 21` for 3) are exposed the same way | fsl-replay/src/hybrid_replay/exact/activation.rs:130-150, fsl-replay/src/hybrid_replay/exact/activation.rs:603, fsl-replay/src/hybrid_replay/poly2.rs:303-348 |
| FND-002 | medium | `over` runs on every freshly evaluated cell before the fixpoint has finished cutting, and its `Err(ActivationInfimum)` aborts the replay even when that cell is later split into cells that decide. Reproduced: `1 / ((v0 - 5)^2 + (h.v - 3)^2) != 0` (no-value point only at `tau = 5`, the first-pass sample of `(0, 10)`) returns `NotExact`, while the same shape at `tau = 4` returns `UndefinedEvaluation` at 3. This gap path is untracked and undocumented | fsl-replay/src/hybrid_replay/exact/activation.rs:845-880 |
| FND-003 | medium | The `ExactGap::ActivationInfimum` doc and the PR body's "Not done" list only "a curve other than `x = tau + c`". `over` also raises the gap whenever the no-value instant is irrational, including a stationary one: `1 / (h.v^2 - 2) != 0 and v0 > 0` returns `NotExact`, although its decisive instant `sqrt 2` is the same for every `tau` and attained. With FND-002 added, the public doc and the PR body understate what remains | fsl-replay/src/hybrid_replay.rs:389-396, fsl-replay/src/hybrid_replay/exact/activation.rs:793-795 |
| FND-004 | medium | The four new tests claim FR-450-AC-12, whose text covers only `Shuttle` cases (a)-(d). Irrational activation instants and moving no-value infima are FR-450 "Exact replay" statement text (:245-251, :286-294) with no acceptance criterion | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:361,383,402,423 |
| FND-005 | low | `over` evaluates every read polynomial at `(tau, time)` and composes `p(t, t + c)`, with `O(deg^2)` polynomial products per polynomial, with no charge. `eliminate` charges once per point for an `(m + n)`-square determinant | fsl-replay/src/hybrid_replay/exact/activation.rs:800-829, fsl-replay/src/hybrid_replay/poly2.rs:324-346 |

## New findings (disposition pass 1)

Head 8030b2498762d35b2eda41f72356bcbef4101a64 (main merged at 6a751ea206; fix commit 8030b24987).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The meter does not bound wall time once an obligation is evaluated at an algebraic activation instant, a path this PR makes reachable. `Lead3b` (guard `q.v > 1 and q.v < 2`, `always [0, 5] holds((v0^3 - 4) * h.v * h.v != 1)`) decides in 4,434 units but takes 75.7 s, about 17 ms per unit; `Lead3` (`(v0^3 - 4) * h.v != 1`) takes 122 ms for 2,700 units. `Lead3c` (`... * h.v * h.v * h.v != 1`) under a 5,000-unit meter neither finishes nor exhausts in 300 s. Each unit's cost grows without a bound: the work inside a charged step (pseudo-remainder coefficients over `Q[t]` not reduced modulo `tau`'s polynomial, uncharged `ExactReal` bisection and sign loops, Sylvester determinants) is not charged. The coder's degree-3 `Lead` test, which "did not finish", is this defect | fsl-replay/src/hybrid_replay/exact/activation.rs:118-180, fsl-replay/src/hybrid_replay/section.rs:155-200, fsl-replay/src/hybrid_replay/real.rs:100-120 |
| FND-007 | low | The `Lead` test does not exercise what its doc claims. `(tau^2 - 2) * x != 1` over `always [0, 5]` is false at many rational `tau` (at `tau = 3/2`, `x = 4` is in `[3/2, 13/2]`), so `Reproduced` does not depend on the reading at `tau = sqrt 2`, and the doc's "reproduced exactly there" is wrong. Behaviour there is correct: the reviewer's discriminating probes `(v0^2 - 2) x^2 - 1 < 0` and `(v0^2 - 2) x^2 + 1 < 0` at `q.v^2 = 2` give `NotReproduced` and `Reproduced` | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:500-516 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 8030b24987 divides out of `tau`'s polynomial the gcd of the decision's coefficients with it, or skips when `tau` is a root of that factor. Cofactor and content are coprime, so the resultant is nonzero unless `p(tau, .)` vanishes identically. Repro `Red`/`Shared` is now `Reproduced`. Adversarial probes all match hand derivation: a scaled factor (`2 v0 - 14`), a partially shared coefficient, two shared factors, a squared factor, mixed content and a rational cut as a reducible `Root` |
| FND-002 | fixed | `over` runs only on the pass with no new cuts. `Isolated` (pole at the first sample, `tau = 5`) now reports 3 |
| FND-003 | fixed | The stationary test now uses `poly(t, time) == 0` for every power of `t`, so it works for an irrational `time`: `1 / (h.v^2 - 2)` reports `sqrt 2`. The doc names the one remaining trigger and QSL-2706, which exists (Specced). The PR body is accurate |
| FND-004 | fixed | The new tests carry bare `#[trace("TC-1221")]` with a note that FR-450 "Exact replay" has no AC for this behaviour |
| FND-005 | fixed | A charge per polynomial in `over` and per decision elimination. The reviewer deleted each new charge (per-decision, per-polynomial in `over`, cofactor `eliminate` closure), and each deletion failed the metering test |
