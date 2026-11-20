---
id: SR-2026112013172122111220103
title: "code-review of QSL-2706 curved moving-instant infimum and QSL-2724 meter calibration"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2706, QSL-2724; PR3190 head 1f7a5af868ca68d337709bcc14c531c872ef2274; git diff origin/main...HEAD (base 374fe2a8ce); fsl-replay/src/hybrid_replay.rs; fsl-replay/src/hybrid_replay/{exact/activation.rs,poly.rs,poly2.rs,real.rs,section.rs,tests/activation_tests.rs,tests/quantifier_tests.rs}"
review_set: subset
---

## Summary

Tickets: QSL-2706 and QSL-2724, PR filament-spec-language#3190, frozen head 1f7a5af868ca68d337709bcc14c531c872ef2274, scope `git diff origin/main...HEAD` over base 374fe2a8ce. Code-review with the rust-review lane folded in.

- **QSL-2706:** places a no-value instant that moves along a root curve as the curve's infimum over the open `tau` cell (`Decider::curve` and `limit`; `Poly2::primitive_t`, `without_vertical_factors` and `squarefree_x`; `poly2::stationary`; `Over::Cuts`; `Section::separators` and `real_root`). It removes `ExactGap::ActivationInfimum`.
- **QSL-2724:** recalibrates the meter: operand words squared plus a fixed call cost, 200 words to the unit.

## Verdict

NOT MERGEABLE. FND-001 (high): the meter still does not bound wall time on the new curve path.

What holds:

- **Algebra.**
  - The common factor in `t` (the `t`-content) has no roots on the curve.
  - Vertical factors are curves `x = c`, which the earlier stationary check already handles.
  - After primitive-part, vertical-factor and square-free reduction, `Res_x(g, g_t)` is identically zero only if `g` and `g_t` share a factor. That factor would divide its own `t`-derivative, so it would be either `t`-content or vertical, and both have been removed. Between the roots of `Res_x(g, g_t)`, every simple branch is monotone; vertical tangents are already discriminant cuts of the projection.
  - The rank of the branch is constant over the cell, because the projection of the read polynomial keeps the number and order of its distinct roots.
  - The branch stays inside the dwell, because the lines `x = s` at segment ends cut the cell where it would leave.
  - The branch does not cross a separator between the near sample and the cell end, so it lies in that separator gap and converges to the root of `g(e, .)` there.
  - `Res_t(minimal, g)` is not identically zero, because `g` is primitive in `t`.
  - The limit is not attained in the open cell, so `open = true` is right.
- **Hand-derived probes.** 16 of 16 values match: interior minimum at a rational (Dip2, DipA) and an irrational `tau` (DipIrr); cusp `x^2 = (tau - 1)^3`; two branches crossing inside the guard range; a vertical asymptote `x = 1 / (tau - 1)` (1 and 2); irrational cell ends (`2 sqrt 2`, 2, 3); a limit on the window line `x = tau`; closed and open starts; the lower of two merging branches (0 and 1/2); a hump; an inflection. The 30 probes from #3167 give the same results, except the earlier `NotExact` gaps (Double, TwoCurves, TwoCurvesB, Cross), which now decide 0, 1, 1 and 0, all hand-checked.
- **Mutation.** All 15 new or changed charge sites fail a test when deleted or minimised (13 deletions, 2 weight changes).
- **Re-pins.** They follow operand size: CurveBig with `10^300` costs 17,734 units against CurveTwice's 3,566.
- **Gates.** `-p fsl-replay --lib hybrid`, `hybrid_requests::`, `hybrid_provider::` and `-p fsl-hybrid-reference --lib` all pass. Clippy shows nothing in `hybrid_replay`, rustfmt is clean on changed files, and there are no floats and no `#[allow]`.

Measured unit cost outside the curve path: 0.6-28 us a unit, against the 1-9 us the PR body claims (TauDeg 28 us, BigCoef 17 us, Deg6Le 0.9 us, DipC 0.6 us).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | On the new curve-infimum path the meter does not bound wall time. `CurveHi5` (guard `q.v > 0 and q.v < 2`, `eventually [0, 1] holds(1 / (h.v^5 - 3 h.v^3 + v0^3 h.v - 4 v0) != 0)`) exhausts 20,000 units in 0.37 s, but under a 100,000- or 300,000-unit meter it neither finishes nor exhausts in 120 s. `CurveBig5` (`1 / (h.v^3 - 10^30/7 v0^2 h.v + v0 - 3) != 0`, same guard) takes 4.6 s for 20,000 units (229 us a unit) and 36.7 s for 93,000 (395 us a unit), against 0.6-28 us elsewhere. That is far beyond the PR's 50x bound, and a small meter does not exhaust promptly. The per-step work in `curve`/`limit` is charged once by `size^2` but not per step: the primitive and square-free reductions with their pseudo-remainder gcd in `x`, `real_roots`/`roots_in` isolation at the rank and near samples, `g.at_x(separator).roots_in` for every separator, and `between`. The PR's own test omits these shapes | fsl-replay/src/hybrid_replay/exact/activation.rs:940-1026, fsl-replay/src/hybrid_replay/poly2.rs:147-253 |
| FND-002 | medium | A rational decisive instant reached as a curve limit is reported as a `Root` with an isolating interval, not a point, which FR-450 :252-256 requires. `real_root` returns `roots_in` of the square-free resultant, which yields `Root` for every root of a degree-2-or-higher polynomial. Seen: CrossPair 1 written as `[0, 2]`, TwoBranch 0 as `[-2, 1/14]`, TwoBranchQ 1/2 as `[0, 1]`, DipC 3 as `[5/2, 25/8]`; `to_wire` writes `root.lower()`/`root.upper()` | fsl-replay/src/hybrid_replay/section.rs:489-511, fsl-replay/src/hybrid_replay.rs:196-202 |
| FND-003 | medium | The stationary cut (`Over::Cuts`) is needed for soundness, but no test can fail without it. CurveDip's interior minimum at `tau = 4/3` is also a cut for an unrelated reason, so with the cut disabled CurveDip still reports 1. Counter-example that is wrong without the cut: DipA (guard `q.v > 0 and q.v < 2`, `1 / (h.v - (v0 - 7/10)^2 - 13/10) != 0`) reports 13/10 with the cut and `(22 - sqrt 65)/10` (about 1.394) without it, a wrong decisive instant with no gap. A value test of this shape should be added | fsl-replay/src/hybrid_replay/exact/activation.rs:948-952, fsl-replay/src/hybrid_replay/tests/activation_tests.rs:776-825 |
| FND-004 | low | The PR body claims "a unit costs 1 to 9 us on every path". Measured: 0.6-28 us on paths the PR covers (TauDeg 28 us, BigCoef 17 us) and 229-395 us, or unbounded, on the curve path (FND-001). `a_unit_of_the_meter_costs_the_same_time_on_every_path` passes only for its chosen claims | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:700-745 |
