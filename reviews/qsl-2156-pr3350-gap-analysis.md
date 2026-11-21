---
id: SR-2026112117314700013350
title: "QSL-2156 PR 3350 gap analysis"
type: SpecReview
date: 2026-11-21
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/timed_product_certificate.rs, fsl-analyze/src/exact_probabilistic/finite_horizon.rs, fsl-replay/src/probability_certificate/timed_axes.rs"
---
# SR-2026112117314700013350: QSL-2156 PR 3350 gap analysis

## Summary

Ticket: QSL-2156. PR 3350 at 860ae812a837726fc50e10f41fb5c73a677d82a5. Manual acceptance-criteria-to-tests check over the diff (quoin matrix not run: one-pass diff review, no builds). Plan completion: not assessed. Criteria examined: FR-924-AC-3, FR-924-AC-5, FR-924-AC-6.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | reset_loop_zeno_values_are_rejected (FR-924-AC-6) is a Deadline claim, so the loop is collapsed on the Deadline axis that the checker already applied before this PR (old timed_key_progresses compared the deadline whole part); no integration test makes the new Clock(i) axes decisive, in the producer or the checker | fsl-analyze/tests/it/timed_product_certificate.rs:780-808, fsl-replay/src/probability_certificate/timed_axes.rs:102-112 |

## Verdict

The three un-ignored tests assert what their criteria state, with correct tags: delay_cycles_are_not_collapsed (TC-2493, FR-924-AC-3: proved 10/9, accepted, initial value 1 rejected InequalityFails), time_locked_state_is_certified_terminal (TC-2493, FR-924-AC-5: under One, proved 0, accepted, value 1 rejected InequalityFails), reset_loop_zeno_values_are_rejected (TC-2493, FR-924-AC-6: proved 99/100, region route, accepted, interior values 0 rejected InequalityFails). The remaining gap is that the Clock-axis code path is pinned only by the two timed_axes unit tests.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | reset_loop_is_nonprogressive_without_a_deadline_clock (AC-6 two-clock model, expected elapsed, no deadline entry) reaches the collapse only through the Closed / Alur-Dill path. 10/9 checked by hand: each send attempt needs 1 ms and succeeds with 9/10, E = 1 ms / (9/10) = 10/9 ms; `again` only resets y and cannot shorten it. |
