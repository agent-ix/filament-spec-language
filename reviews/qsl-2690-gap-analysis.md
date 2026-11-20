---
id: SR-2026111919340478491681422
title: "gap-analysis of QSL-2690 exact replay at algebraic activation instants"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2690; PR3167 head 73f0a97484893149817c4d723f2a1d0b01740f5f; FR-450 :204-258, :280-296, FR-450-AC-12; TC-1221; fsl-replay/src/hybrid_replay/**"
review_set: subset
---

## Summary

Ticket: QSL-2690 (PARTIAL), PR filament-spec-language#3167. This is a diff-scoped check that each acceptance criterion has a test and each requirement has code, for FR-450 "Exact replay": the decisive instant as the infimum, exact algebraic decision over `(tau, t)`, and `NotExact` only for non-point, non-polynomial or recursive traces. Plan completion: not assessed. Quoin 0.29.1 was present, but `quire trace --id FR-450-AC-12` did not finish within 120 s, so the AC-to-test mapping was checked by hand.

## Verdict

PARTIAL, tracked. FR-450 :280-282 still holds the temporary `ActivationInfimum` deviation, which is honest, settles `NotExact`, and is tracked by QSL-2690 staying open. No separate follow-up ticket exists; QSL-2690 is the tracker. Its description is stale and does not list the remaining cases: a curved moving instant, an irrational no-value instant (code-review FND-003), and the premature abort (code-review FND-002). The new tests' expectations are hand-derived and correct, and I recomputed five of them. The degenerate elimination paths have no test, and one of them is wrong (code-review FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the degenerate elimination paths at an algebraic instant: a `tau` polynomial with a factor shared by a read polynomial (resultant identically zero), a read polynomial vanishing identically at `tau`, or a rational cut reaching `at_algebraic` as a `Root`. The first is wrong (code-review FND-001) | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:355-439 |
| FND-002 | low | The moving-infimum test pins only `c = 0` and `c = 1` over a rational cell start. No test covers a negative `c` or an irrational cell start, where `cell.0.shifted` acts on a `Root` and gives `sqrt 2 + 1`; the reviewer's probes of both pass | fsl-replay/src/hybrid_replay/tests/activation_tests.rs:415-439 |
