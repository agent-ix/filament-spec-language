---
id: SR-202611251158301
title: "code-review of PR 3915 (QSL-3307) exact policy-iteration threshold and always fallback"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3915 head 9ef3ee9a1540e203180e670bcef460a3d6687f00; fsl-analyze/src/exact_probabilistic/unbounded.rs, fsl-analyze/src/exact_probabilistic.rs, fsl-analyze/src/analyze/exact.rs, fsl-analyze/tests/it/cyclic_state_certificate.rs, fsl-analyze/tests/it/state_probabilistic_replay.rs"
review_set: subset
---

## Summary

Ticket: QSL-3307. Code review with the rust-review lane folded in, over `git diff origin/main...HEAD` at head 9ef3ee9a1540e203180e670bcef460a3d6687f00. No tests were run: MemAvailable was about 21 GB inside the cargo lock, under the 30 GB start rule, so the review is from source.

## Verdict

CONDITIONAL. The `always` fallback in analyze/exact.rs now complements every outcome correctly: Exact (all values, with +inf rejected as InvalidState), Bounds (pre-existing, ends swapped) and PrecisionBudget (ends swapped). Stopped and Err pass through to the existing limit and failure mapping. Reading the code, both new assertions in `an_always_claim_past_the_rational_budget_settles_on_its_own_value` fail without the fix. The uncomplemented Exact(1/25) does not hold against 0.95, so the ExactValue let-else panics. The uncomplemented PrecisionBudget interval sits near 1/25 and does not contain 24/25. The state_probabilistic_replay change is a correct test repair. Since #3708, FR-125 makes only initially held keys into instances, so the old "unrestricted" subject had one instance. The UnsupportedBinding assertion is unchanged. The analyzer's settlements are unchanged for until and reach. The exact.rs fallback runs only after the same scalar gate has rejected the same values, so `too_long` is always true there and the separates split is the same as on main. The `always` case changes only through the intended complement fix. No other consumer exists besides fair_interval.rs and the tests. The open finding is that the new rule for reporting by length contradicts itself.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | reachability_threshold's new "too long to report" rule contradicts itself: a too-long value that the rounded interval cannot separate is still returned Exact. Under the scalar measure it uses, one new test pins an ExactValue past the budget (24/25, 11 scalar bits, budget 8). Under the per-part measure that FR-1042 and the solver use, the other new test pins Bounds for a value that fits (1/2 at budget 3). | fsl-analyze/src/exact_probabilistic/unbounded.rs:613-665 |
| FND-002 | low | `too_long` checks every state's value and its complement for every caller (reach, until, fair, the public threshold API). A long non-initial value, or the long complement of a reach value, forces the rounded path. The analyzer is not affected. | fsl-analyze/src/exact_probabilistic/unbounded.rs:618-624 |
| FND-003 | low | The new unit test has no trace tag. The new integration test is tagged TC-3187/FR-201 (FR-201-AC-65 is about the cyclic stop) and FR-198 at FR level, but it exercises the acyclic threshold route. | fsl-analyze/src/exact_probabilistic/unbounded.rs:3486-3488; fsl-analyze/tests/it/cyclic_state_certificate.rs:649-651 |

## Coverage

- Examined: reachability_threshold (all return paths), exceeds_rational_budget, decide_reachability_threshold_with_blocked, fair_interval.rs consumer, analyze/exact.rs scalar gate and interval fallback (until, always, expected reward, reach arms), the interval_result mapping into Interval/ValueBounds, the new and changed tests, FR-198, FR-1042 Bases/Inputs, ADR-028 UR-4/LM-1/LM-2/XV-1, FR-125, FR-201-AC-65, FR-202-AC-20.
- Concurrent PR #3907 (head f168d5ef4c9442fcaca04fc0710e4a8946112785) touches build.rs, component_certificate.rs, exact_probabilistic.rs (tests) and prob_world.rs; at that head it does not touch analyze/exact.rs or cyclic.rs. Its change to prob_world::with_object_initials only adds support for `{"objects": {key: null}}` entries. That is compatible with this PR's use and applies the same FR-125 repair to a different test. No semantic conflict.
- Not run: any cargo test (memory gate refused).
