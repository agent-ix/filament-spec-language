---
id: SR-2026112017124277723988802
title: "gap-analysis of PR 3209 analyze EN-1/EN-6 verdict defects (QSL-2714, QSL-2719, QSL-2689)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2714, QSL-2719, QSL-2689; PR3209 head 293e4ec164febfa6b1d8fe97a164658f458919f1; FR-125-AC-2, FR-126-AC-2, FR-124-AC-12, FR-368-AC-22, FR-242, FR-234, FR-1034, FR-240, FR-244; TC-519, TC-431"
review_set: subset
---

## Summary

Tickets: QSL-2714, QSL-2719, QSL-2689. PR filament-spec-language#3209. This is a diff-scoped check that each ticket's acceptance has code and passing tests behind it. Plan completion: not assessed. Quoin 0.29.1; checked by hand against the diff, the tickets' acceptance text and the test results.

## Verdict

COMPLETE for QSL-2719 and QSL-2689. PARTIAL for QSL-2714, which the PR states as remaining work and only `Refs`, so it closes nothing.

**QSL-2689 (FR-242, FR-234, FR-1034).** `nested_until_with_inner_and_outer_bounds` and `unbounded_upper_bound_matches_the_actual_untimed_checker` are un-ignored and pass. Each `Holds` certificate is core-accepted, and each `Violated` replays. All 28 `zone_mitl` tests pass.

**QSL-2719 (FR-124-AC-12, FR-368-AC-22).** Three new document-entry tests pass:

- a vault-only subject gets `refuted` with a one-step replayed counterexample, for both the named item and the default items;
- another identity's `terminal any` keeps the item;
- a subject that reads both identities is `unsupported`, and the default items write none.

**QSL-2714 (FR-125-AC-2, FR-126-AC-2).** `a_bounded_state_predicate_is_false_past_the_terminal_closure` is un-ignored and passes. Two tests stay ignored with stated reasons:

- `source_atoms_distinguish_sufficient_and_short_bounded_each_horizons` (`on each` bounded);
- `a_bounded_operator_reads_true_over_the_infinite_terminal_stutter` (infinite-trace `always[0,5]`).

Those two normative cases still settle as failed on this head, so QSL-2714 stays open. That is a fact for arch, not a defect of this PR.

**Test results at head.**

- fsl-analyze `it` filtered `terminal_two_models|source_model_boundaries|zone_`: 160 passed, 4 failed, 3 ignored. The run includes 9 local review-only adversarial tests.
- The 4 failures are identical on origin/main 438a9f3b79 (145 passed, 4 failed, 6 ignored).
- fsl-replay `zone`: 10 passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
