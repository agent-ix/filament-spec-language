---
id: SR-2026111610291803
title: "Criterion strength review of the FSL wave 6 spec readiness batch, language side"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; PR #2747 branch fsl-spec-w6 at 0ed71f1385f075422c2d149b4ba0f2fd4e3dc188: FR-243-AC-4, FR-330-AC-18, FR-489-AC-1, FR-489-AC-5, FR-495-AC-23, FR-504-AC-1, FR-621-AC-9, FR-621-AC-10, FR-625-AC-4, FR-628-AC-1, FR-628-AC-3 to AC-5, FR-632-AC-2 to AC-4, FR-634-AC-1, FR-634-AC-4, FR-634-AC-5, FR-635-AC-1, FR-635-AC-4, FR-641-AC-8, FR-810-AC-1 to AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-634
    type: reviews
---

## Summary

Ticket: QSL-2296 (wave 6 batch). Jev (typesafe.ai System One) is not installed on this host (`jev not found`), so the Jev-calibrated judgement was skipped and each criterion was judged by hand over the spec text.

Each examined AC names concrete fixtures and exact values: counts, basis, backend, cause, exit code and member names. Almost all of them have a negative or control half that fails on the wrong behaviour:

- FR-621-AC-10 has a disposition item with no `backend`.
- FR-641-AC-8 has a `finite` run with no `limits`.
- FR-495-AC-23 has the `Int[0, 5]` refusal.
- FR-810-AC-2 has the second run, the non-checking program and the malformed members.
- FR-628-AC-5 has the `always possible` stop.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-634-AC-5's coverage check, "`coverage.states` no more than 4", holds for any run over the four-state staying Counter, so it cannot fail and does not test what sample-mode coverage means. Fix: state the exact coverage for seed 5, or require `coverage.states` to equal the number of distinct state keys on the three reported traces. | spec/functional/FR-634-request-a-simulation-item.md:99 |

## Verdict

PASS WITH ONE LOW.

## Dispositions

Round 1, verified at b3ee227de56d7909ddfd92346cf289c31aa1aac4.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-634-AC-5 now requires `coverage.states` to equal the distinct states on the three reported traces, and `coverage.transitions` the distinct steps on them. TC-2293 and the stub assert both, so the criterion can fail. |

Round 1 verdict: APPROVE.
