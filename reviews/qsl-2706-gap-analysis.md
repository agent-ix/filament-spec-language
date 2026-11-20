---
id: SR-2026112013172122111220104
title: "gap-analysis of QSL-2706 curved moving-instant infimum and QSL-2724 meter calibration"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2706, QSL-2724; PR3190 head 1f7a5af868ca68d337709bcc14c531c872ef2274; FR-450 :204-258, :272-296; TC-1221; fsl-replay/src/hybrid_replay/**"
review_set: subset
---

## Summary

Tickets: QSL-2706 and QSL-2724, PR filament-spec-language#3190. This is a diff-scoped check that FR-450 "Exact replay" has the code and tests it needs for three statements: the decisive instant is the infimum (:245-258), the time is reported as a point interval when rational and as an isolating interval otherwise (:252-256), and `NotExact` is returned only for a non-point trace, a non-polynomial flow or a recursive call (:272-275). Plan completion: not assessed. Quoin 0.29.1; checked by hand.

## Verdict

PARTIAL. `ExactGap::ActivationInfimum` is gone, so the FR-450 :272-275 deviation for activation infima is closed; only `CoupledReset` (QSL-2639) remains, and it is tracked. Two gaps remain:

- The statement that a rational decisive instant is reported as a point interval is not met on the new curve path (code-review FND-002).
- No test pins the soundness of the stationary cut (code-review FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test pins FR-450 :252-256 for a decisive instant that is the limit of a curve. The new tests assert the value through `sign_of`, `> 0` or `Rational(q(n, 1))` only where the limit happens to come out as `Rational`. None asserts that a rational limit is reported as a point, which fails for CrossPair, TwoBranch, TwoBranchQ and DipC (code-review FND-002) | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:776-825 |
