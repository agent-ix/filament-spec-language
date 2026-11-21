---
id: SR-20261121153137
title: "code review of PR 3325 FR-194-AC-16 incomplete windows and AC-17 missing window members"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3325 head fbe9090b11bf8c247f3e1cfbf77b25c3eae96bb4; fsl-eval/src/temporal/finite_window/run.rs, fsl-eval/src/temporal/letters.rs, fsl-eval/src/temporal/observed.rs, fsl-eval/src/temporal/observed_aggregate.rs, fsl-analyze/tests/it/monitor_incremental.rs, tests/it/clock_reach.rs"
review_set: subset
---

## Summary

Ticket: QSL-2138. The two-valued incomplete-window rule is sound. A covered window stays definite, so a refutation decided by covered data stays final. An uncovered window is never final on its own. The carried and batch routes run one rule through ObservedLetters. The un-ignored test's data change is correct against FR-194. The two failing monitor_incremental tests exercise no aggregate clause, so this diff does not change them. AC-17 is implemented on the batch route over a `missing` entry that names the aggregate atom. That is not the input AC-17 names, which is a response holding no `latency` valuation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-17 is implemented and tested as an absent aggregate-atom valuation at position 18, not a response holding no latency valuation; the member's sample still holds latency | fsl-eval/src/temporal/observed_aggregate.rs:158-160 |
| FND-002 | low | The un-ignored test's comment says the new levels refute only at stamps 3 and 4; they also refute at stamp 5 | fsl-analyze/tests/it/monitor_incremental.rs:3764-3767 |
| FND-003 | low | No test covers the min_count two-valued arm or the undefined-stays arm of the unfinished-window rule | fsl-eval/src/temporal/finite_window/run.rs:850-861 |
| FND-004 | low | aggregate_lag fails open: a lower bound that does not convert returns None, which reads the window as complete | fsl-eval/src/temporal/observed.rs:1182-1192 |

## Verdict

Fix round needed, small. FND-002 to FND-004 are in-PR edits. FND-001 needs a spec ruling, given in the ticket comment, then a test edit. Checked clean: exact ceiling lag, monotone `established`, covered atoms never two-valued, carried equals batch at each progress step, evidence route reads only established activations, failing tests not touched.

## Disposition (fix round 1)

- FND-001 fixed: spec wording and test follow FR-641 (a `missing` entry marks an admitted observation's valuation absent; reason position 18).
- FND-002 fixed.
- FND-003: min_count arm covered (`an_incomplete_window_short_of_min_count_is_two_valued`, `a_covered_window_short_of_min_count_is_undefined_through_run`). Undefined-stays arm: no source form under the timestamped-event profile produces an undefined read (Divide is profile-prohibited, check.rs:4542-4548, and `mod` is refused likewise as `ProfileProhibitedOperator(IntegerMod)`); no unit test exists for it at the finite-window run level.
- FND-004 fixed: `aggregate_lag` falls back to 0.
