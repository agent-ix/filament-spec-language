---
id: SR-2026112103362300000000001
title: 'Gap analysis of PR #3098: false-guard capture reports inactive (QSL-2590)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/filament-spec-language; git diff origin/main...HEAD (PR #3098, head 4257de2984): FSpec FR-093-AC-5/6 as implemented, FR-267 (temporal table, AC-20, AC-28), FR-283 Outputs, FR-621 temporal kinds, FR-820 Equality, FR-895 Requirement row; ticket QSL-2590'
review_set: subset
date: '2026-11-21'
relationships:
- target: ix://agent-ix/filament-spec-language/FR-267
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-283
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-621
  type: reviews
---

## Summary

Ticket: QSL-2590. The audit is planless and scoped to the PR diff. It used `quoin matrix --json` (quoin 0.36.8) from the worktree root. The new behaviour is tested on the `fsl run` route, over a lasso and a closed finite trace, and on the package route over a lasso. The monitor carried-state route, the trace-subject batch route and the recording route have no test of the inactive result. On the carried-state route the behaviour is also wrong, as SR-2026112023230400000000001 FND-005 records.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-267-AC-28 | spec/functional/FR-267-write-run-results-as-native-run-result-2.md | examined |
| FR-267-AC-20 | spec/functional/FR-267-write-run-results-as-native-run-result-2.md | examined |
| FR-283 Outputs `Inactive` row | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:250 | examined |
| FR-621 temporal kinds | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:190-201 | examined |
| FR-820-AC-1 | spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:163 | examined |
| FR-895 Requirement row | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:76 | examined |
| FSpec FR-093-AC-5, AC-6 | filament-specification FR-093 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | No test covers the inactive result on the `native-monitor/1` carried-state route, and the code there is wrong: it reports `holds` where the batch run reports `inactive` (SR-2026112023230400000000001 FND-005). The FR-820-AC-1 and FR-860-AC-5 equality tests compare only `category`, which is `success` for both. | fsl-replay/src/spine/clause/clause_state.rs:888,1036; tests/it/temporal_capture_run.rs:336-366 |
| FND-002 | medium | The trace-subject batch route (`evaluate_reporting_inactive`, used by monitor `Samples` and by recording requirements) has no test that reaches inactive. The recording arm `Inactive -> Pending` has no test and no spec row in FR-895 (SR-2026112023230400000000001 FND-006). | fsl-replay/src/spine/clause/trace_subject.rs:442; fsl-analyze/src/protocol_analysis/recording.rs:2332 |
| FND-003 | low | The FR-283 Outputs `Inactive` row and FR-621's `inactive` kind have no acceptance criterion and no tagged test. The monitor's `MonitorVerdict::Inactive` mapping (monitor.rs:960) is reached by no test. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:250; fsl-analyze/src/monitor.rs:960 |
| FND-004 | low | The timed inactive branch (observed.rs:312-318, temporal.rs:1896-1900) has no test. Every timed run-wire test is ignored on QSL-2254/QSL-2136, so this is deferred to QSL-2254. | fsl-eval/src/temporal/observed.rs:312-318 |

## Verdict

FAIL: FND-001 is high. FR-267-AC-28 is tagged by `a_closed_false_guard_capture_scope_is_inactive_on_both_routes` and `a_closed_finite_trace_with_every_guard_false_is_inactive`, both of which ran and passed. FSpec FR-093-AC-6 is tagged by `false_guard_deliveries_create_no_capture_obligation_on_both_routes`, which has active controls. FR-267-AC-20 keeps `an_unregistered_temporal_kind_refuses`. The ignored corpus test `the_canonical_false_guard_capture_example_is_inactive` passed in the lead's run with FSL_EXAMPLES_DIR set. The production code has no stubs. The only reverse gap is the recording arm in FND-002.

## Coverage

- Criteria examined: FR-267-AC-20, FR-267-AC-28, FR-820-AC-1, FSpec FR-093-AC-5, FSpec FR-093-AC-6. All are tagged. Gaps are by route, not by untagged criterion.
- Semantic review: skipped as a separate pass. The intent check for the changed routes was done by hand inside the code review.
- Plan completion: not assessed

## Dispositions

Disposition pass 2 at head 586c552b0e, a static review plus /tmp/qsl2590-focused-4.log.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The monitor-route test `the_monitor_reports_a_closed_false_guard_scope_inactive_like_the_batch_run` now exists, and it compares `temporal` and `truth` for every prefix. It fails at this head: its active control (levels 0, 1, 0) ends `inactive` on the monitor, and the command exited 101 (see the code-review SR, FND-005). |
| FND-002 | fixed | - |
| FND-003 | still-open | No AC was added for FR-283's `Inactive` row or for FR-621's `inactive` kind. `MonitorVerdict::Inactive` is reached only by the failing monitor test. This closes with FND-001 if that test passes. A separate AC is optional. |
| FND-004 | deferred | QSL-2254: every timed run-wire test is still ignored, so the timed inactive branch cannot have an end-to-end test yet. |
