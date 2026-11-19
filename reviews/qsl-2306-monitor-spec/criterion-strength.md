---
id: SR-2026111611450303
title: "Criterion strength review of the QSL-2306 monitor stepping rulings"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; PR #2750 branch qsl-2306-monitor-spec at 0e9321f13bc364f95141619f57fdae15d9866974: FR-283-AC-17, FR-643-AC-5 to AC-9, FR-644-AC-5, FR-820-AC-1 to AC-3, TC-2320..TC-2323"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: reviews
---

## Summary

Ticket: QSL-2306. Jev (typesafe.ai System One) is not installed on this host (`jev not found`), so the Jev-calibrated judgement was skipped and each criterion was judged by hand over the spec text.

Several criteria are strong and name exact fixtures and values:

- FR-643-AC-6 gives B(3) against B(4) counterexample lengths.
- FR-643-AC-7 gives both decisive bases and the `Pending`-while-closed-`violation` control.
- FR-643-AC-8 gives the `Progressed` open-scope pair.
- FR-644-AC-5 separates `category` from `disposition`.
- FR-820-AC-1 and AC-3 compare member for member against the batch report and the uninterrupted state.

The weak ones rest on a tautological fixture or name no expected value.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Recent` (`count where holds(r.level = 2) over past[0, 3] min_count 0 >= 0`) is true for every trace, so FR-643-AC-9's "settles `success` at step 10" passes for an evaluator that ignores the window and returns `success`. The same holds for FR-820-AC-2's report-equality half over that clause. Fix: use a predicate that can fail, for example `<= 1` over levels that put two 2s inside one window, with the expected settle step and verdict. | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:213, :217; spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:104 |
| FND-002 | medium | FR-283-AC-17 names no expected values ("returns the closed false-extension record", "the finite-prefix record"). "No request member changes either reading" is an unbounded universal that no finite test can establish. Fix: state FR-330-AC-1's `violation` at position 0, exit 10, for `Bounded`, and `pending`, exit 0, for `Reaches`. List the members tried (for example `limits`, `temporal_selections`, a `Document` `closed`/`open` member) and the outcome of each. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:538 |
| FND-003 | low | FR-820-AC-2's "stays at most 4" and FR-643-AC-9's "holding at most four values" pass for a state that holds fewer values than the window needs, for example one that drops values early and so would compute wrong counts on a non-tautological predicate. Fix: require `retained()` to equal `min(k, 4)` at position `k`. | spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:104; spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:217 |
| FND-004 | low | FR-643-AC-5 opens with a `Timestamped` observation and then puts the event-position `Recent` and the infinite-trace `Reaches` and `Never` "in the same session". A `Timestamped` session refuses those profiles (`trace-kind-mismatch`), so the fixture's session kind is unstated. Fix: say the second half runs in a `Finite` session. | spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md:213 |

## Verdict

PASS WITH TWO MEDIUMS. The integrity review's FND-001 also affects FR-643-AC-6 and FR-644-AC-5. Those ACs are strong, but they assert a `Reaches` verdict that FR-328 forbids, so they cannot pass on a correct evaluator.

## Dispositions

Round 1, verified at fda2d6ef58dd0ac3c456fcdc3bdb2deb2d286c85.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `Recent` is now `... min_count 0 <= 1` in FR-643-AC-5, FR-643-AC-9 and FR-820-AC-2. AC-9 and FR-820-AC-2 add the failing case "over levels 2, 0, 2 `Recent` settles `violation` at step 3". |
| FND-002 | fixed | FR-283-AC-17 now states `violation` at position 0 with `truth` `false` for `Bounded` and `pending` with no `truth` for `Reaches`. It names the members tried (`limits.max_window_values` 1, empty `temporal_selections`) and the `"closed": true` and `"open": true` refusals. |
| FND-003 | fixed | FR-820-AC-2 and FR-643-AC-9 now require "exactly `min(k, 4)`" retained values. |
| FND-004 | fixed | FR-643-AC-5 now reads "In a separate `Finite` session with `retained_positions` 2". |

Round 1 verdict: APPROVE. The integrity review's FND-011 and FND-012 cover FR-643-AC-10's timed expectations.
