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

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-194 now defines the `missing`-entry wire form per atom; AC-17 names reason position 18; test asserts position 18 for 0.95 and 0.9 |
| FND-002 | fixed | comment reads "stamps 3, 4 and 5" |
| FND-003 | accepted-no-change | min_count arm now covered (TwosMin, run test); the undefined arm is unreachable from source on this route (Divide/mod refused outside value.complete, check.rs:4542-4548) and ClauseFacts::of_checked refuses aggregate facts on the plain finite-window route, so no cheap test exists |
| FND-004 | fixed | aggregate_lag falls back to Some(0) |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | AC-16's through-9 insufficient-data reading can still become violated after a later assertion; neither AC-16 nor FR-327 says this result is not settled | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:291 |
| FND-006 | low | TwosMin doc says "the result is pending", but W 5 >= t0 + Hf 5 makes it indeterminate (IncompleteInput) | fsl-analyze/tests/it/monitor_incremental.rs:3813-3818 |
| FND-007 | low | The clock_reach comment says latency 100 "would decide the quantile if read"; 100 gives the same atom values as the original 6, and the 0.9 case is decided by the atom-level absence at 18 alone | tests/it/clock_reach.rs:398-402 |
