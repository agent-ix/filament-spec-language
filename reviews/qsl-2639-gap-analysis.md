---
id: SR-202611210524091
title: "gap-analysis of PR3262 coupled free post-jump variables against FSL FR-450 Exact replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-450 Exact replay (fairness enabledness over free post-jump variables; NotExact conditions), FR-750-AC-12, TC-3037 against fsl-replay/src/hybrid_replay/exact.rs, fsl-replay/src/hybrid_replay/exact/joint.rs, fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs; PR3262 head 93e630d56f2d27995545728b7b87a942e8acf8d5"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-750
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-3037
    type: references
---

## Summary

Ticket: QSL-2639. Manual criterion-to-test check of the criteria this PR touches (the computed `quoin matrix` was not used for the verdict). Plan completion: not assessed. Examined: FR-450 "Exact replay" enabledness bullet (spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:260-269), FR-450 NotExact bullet (:270-273), FR-750-AC-12 clause (b), TC-3037 step 2. The implementation meets the enabledness bullet for linearly coupled free variables (finite union of intervals with algebraic ends via exact cell splitting).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No acceptance criterion states the coupled-reset behaviour: FR-750-AC-12 (b), which the two new tests are tagged with, is the single-free-variable `Shuttle` reset `v >= 0` / `v <= 1/2`, and FR-450 has no AC for its enabledness bullet's several-free-variable case, so the tests verify a SHALL statement no criterion owns (spec gap; an FR-450 AC, or an FR-750-AC-12 clause, for a reset leaving `v`, `w` free under `v + w <= 1` is needed) | spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:260-269, fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:339, fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs:389 |
| FND-002 | low | FR-450 says no reset other than the listed cases gives `NotExact`, but a non-linearly coupled reset still settles `NotExact` through `ExactGap::CoupledReset`; the PR declares this as remaining QSL-2639 work, so it is a known open gap, not a regression | spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md:270-273, fsl-replay/src/hybrid_replay.rs:529-535 |

## Verdict

CONDITIONAL — the code meets FR-450's enabledness bullet for linear coupling; FND-001 is a spec gap that should be filed or closed with an AC (the PR body already notes it); FND-002 stays tracked by the open QSL-2639.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The wrong trace is removed in fe4ba64cbe (the three coupled replay tests no longer claim FR-750-AC-12/TC-3037; the joint.rs unit tests are untagged); the missing FR-450 AC for coupled resets is a spec gap stated in the PR body and left to QSL-2639, which stays open |
| FND-002 | deferred | Non-linear coupling still settles NotExact via ExactGap::CoupledReset; remaining work on QSL-2639, which stays open after this partial slice |
