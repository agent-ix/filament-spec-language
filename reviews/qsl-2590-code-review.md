---
id: SR-2026112023230400000000001
title: 'Code review of PR #3098: false-guard capture reports inactive (QSL-2590)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language; git diff origin/main...HEAD (PR #3098, branch qsl-2590-false-guard-activation, head 4257de2984): fsl-eval/src/temporal/{engine,mod,observed}.rs, fsl-replay/src/spine/clause/**, fsl-replay/src/spine/native_result/**, fsl-analyze/src/{monitor,protocol_analysis/recording}.rs, spec/functional/FR-267, FR-283, FR-621, tests/it/temporal_capture_run.rs; ticket QSL-2590'
review_set: subset
date: '2026-11-21'
---

## Summary

Ticket: QSL-2590. PR: filament-spec-language#3098. The review pass (round 0) was written by draco:QSL:arch at head 6efd9d82. This file carries its findings unchanged and adds the round-1 disposition pass at head 4257de2984, with the rust-review lane folded in. The disposition pass is a static review. The focused test log of the lead's run (fsl-eval temporal 25 passed, fsl-replay spine::clause 505 passed, it temporal_capture_run 11 passed, the canonical corpus test 1 passed, fsl-analyze protocol_recording 36 passed) was read as evidence.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FSpec FR-093-AC-5 | filament-specification spec/functional/temporal/FR-093 | examined |
| FSpec FR-093-AC-6 | filament-specification spec/functional/temporal/FR-093 | examined |
| FSpec FR-352-AC-25 | filament-specification spec/functional/foundation/FR-352 | context_only |
| FR-820 Behavior "Equality" | spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:121-125 | examined |
| FR-895 Requirement obligation row | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:76 | examined |
| FR-267 temporal table and AC-20, AC-28 | spec/functional/FR-267-write-run-results-as-native-run-result-2.md | examined |
| FR-283 Outputs table | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:250 | examined |
| FR-621 temporal kinds | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md:190-201 | examined |
| Evaluator inactive decision | fsl-eval/src/temporal/engine.rs:1166-1169 | examined |
| Timed observed run | fsl-eval/src/temporal/observed.rs:197,312-318 | examined |
| Trace-subject route | fsl-replay/src/spine/clause/trace_subject.rs:442-500 | examined |
| Carried clause state | fsl-replay/src/spine/clause/clause_state.rs:888,1036 | examined |
| Streaming evaluator | fsl-eval/src/temporal/stream.rs:440-445,756-832 | examined |
| Recording requirement obligation | fsl-analyze/src/protocol_analysis/recording.rs:2286-2337 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The inactive fact is derived only for `TemporalTrace::Lasso`. On a closed finite trace under a bounded profile (`Mode::Closed`), a guarded `on each` clause with every guard false still reports active, truth true. The trace-subject route never derives the fact. | fsl-eval/src/temporal/observed.rs:453-460 |
| FND-002 | medium | The timed observed run hard-codes `inactive: false`, so a timed lasso with every guard false still reports holds/active/true. | fsl-eval/src/temporal/observed.rs:312 |
| FND-003 | medium | The writer emits a `temporal` kind, `inactive`, that FSpec FR-352 does not register, and FR-267-AC-20 and FR-621 still say "sixteen". | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:312-317,651 |
| FND-004 | low | FR-283's Outputs table has no `Inactive` row for the new reachable `MonitorVerdict::Inactive`. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:243-256 |

## Verdict

Round 1: the engine now decides inactive at its root for `Mode::Closed` and `Mode::Lasso`, and both batch routes and the timed run carry it. FND-001 to FND-004 are fixed. The fix round leaves three new defects. The carried clause state that `native-monitor/1` uses still reports `holds` where the batch run now reports `inactive`, which breaks FR-820 Equality (FND-005, high). The recording `Inactive -> Pending` arm can run, through monitor `Samples` to the trace-subject route. It is untested, and on a closed recording it turns a `proved` result into `inconclusive` with no spec row behind it (FND-006, medium). FR-267 still describes the `inactive` row as lasso-only and says FSpec registration is outstanding, but FSpec FR-352 on origin/main already registers it (FND-007, low). Not mergeable until FND-005 and FND-006 are fixed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The carried clause state hard-codes `inactive: false`, and its streaming evaluator reads a bounded profile as `Reading::Closed`, so it returns `Holds` for a guarded `on each` clause whose every guard so far is false. The batch run over the same prefix now returns `Inactive`. FR-820 Equality requires the carried report to equal FR-330's batch report for every prefix, so `native-monitor/1` (monitor_session.rs:556, 687) reports `holds`, activation active, truth true over a closed, complete stream where `fsl run` reports inactive. That is FSpec FR-093-AC-5's defect again, on the monitor route. The FR-860-AC-5 equality tests compare only `category`, which is `success` for both, so they cannot catch it. Fix: have `ClauseStream` record whether any guard read `Some(true)` (stream.rs:444) and report inactive from `outcome` under `Reading::Closed`, use that at clause_state.rs:888 and 1036, and add a `monitored()` test over `GuardedClosed` with levels 0, 0, 0 that compares `temporal.kind` and `axes`. | fsl-replay/src/spine/clause/clause_state.rs:888,1036; fsl-eval/src/temporal/stream.rs:440-445,831 |
| FND-006 | medium | The coder's claim that the recording `Inactive -> Pending` arm cannot run is wrong. `assess_obligations` calls `monitor` with `MonitorTrace::Samples` (recording.rs:2291). `samples_run` (monitor.rs:809) sends that to `run_package_trace_subject`, then `run_subject_observing`, which evaluates `TemporalTrace::Finite` under the only profile it accepts, event-position false-extension (`Mode::Closed`). It now returns `Inactive` for a guarded requirement whose every guard is false. The arm has no test. It maps inactive to `pending` even when `closure.state` is `closed`. Before this PR that recording reported `satisfied`, and FR-990 settles a closed recording with a `pending` obligation as `inconclusive`, cause `open-history`, where it used to be `proved`. FR-895's Requirement row ("the clause holds over the recording read as its trace subject") does not say what an inactive requirement is. The spec needs a row: `satisfied` (FSpec FR-093-AC-6: false guards create no obligation) or `pending`. Then the code needs a protocol_recording test over both closure states. | fsl-analyze/src/protocol_analysis/recording.rs:2332; spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:76 |
| FND-007 | low | FR-267's `inactive` row and FR-267-AC-28 say "a closed, complete guarded lasso". The engine now decides inactive for a closed finite trace under a bounded profile too, and the new test `a_closed_finite_trace_with_every_guard_false_is_inactive` is tagged to AC-28. FR-267 also says registration in FSpec FR-352 "remains required", but FSpec FR-352 on origin/main (6ad873d5) already registers seventeen kinds with `inactive` (FR-352-AC-25: a closed finite trace and a lasso). Change the row and AC-28 to "a closed finite trace or a lasso", and say that FSpec FR-352 registers the same seventeen kinds. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:312-323,660 |

## Dispositions

Disposition pass 1 at head 4257de2984 (fix commits 99cff0523b, 6e354cfc27, 4257de2984).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | engine.rs:1166-1169 sets `inactive` for `Mode::Closed` and `Mode::Lasso` when no guard is true. `guarded` requires `Goal::Clause` (engine.rs:980), so a formula-at-position call never sets it. `run_seeking` returns the flag. `evaluate_observed_goal` (observed.rs:440-462) and the trace-subject route (`evaluate_reporting_inactive`, trace_subject.rs:442-472) carry it. The lasso-only check is gone. `a_closed_finite_trace_with_every_guard_false_is_inactive` covers the closed finite trace on the run route. The requested recording-arm test was not added. The arm can run, and it is recorded as FND-006. |
| FND-002 | fixed | observed.rs:197 and 312-318 set `inactive` for a timed lasso or a trace closed by `through` when the guarded run holds and no guard read true. `run_temporal` now uses the flag (temporal.rs:1896-1900). I checked the coder's claim that no timed end-to-end test can run: every timed run-wire test is `#[ignore]` on QSL-2254/QSL-2136 (tests/it/run_wire.rs:164, 198, 230, 717, 884, 1264, 1405), and `evaluate_observed_timed` has no other caller. The test is deferred to QSL-2254. |
| FND-003 | fixed | FR-267-AC-20 now says "outside the seventeen". FR-621 lists `inactive` as the seventeenth kind. FSpec FR-352 on origin/main 6ad873d5 already registers `inactive` (FR-352 line 270 and AC-25), so no companion PR is needed. The stale FR-267 prose that remains is FND-007. |
| FND-004 | fixed | FR-283 Outputs gains the row "`Inactive` / A closed, complete guarded scope whose every guard is false / none / success / 0". |
