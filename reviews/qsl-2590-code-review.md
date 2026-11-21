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

Disposition pass 2 at head 586c552b0e (fix commits 59a2658636, 362fdab606, 00e6c51bed; review copies in 586c552b0e), rebased on main d84ed5685b. This is a static review plus the lead's log /tmp/qsl2590-focused-4.log.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | still-open | The code change is in place: `ClauseStream::admitted`/`inactive()` (stream.rs:204,414,956-963), clause_state.rs:888-891 and 1038-1043, snapshot.rs:439-442, and the override in `streamed_sample_report` (temporal.rs:1349-1356). But the PR's own new test fails at this head. In /tmp/qsl2590-focused-4.log, `the_monitor_reports_a_closed_false_guard_scope_inactive_like_the_batch_run` FAILED at tests/it/temporal_capture_run.rs:562 and the `--test it temporal_capture_run` command exited 101 (`exit-3=101`), not 0. The failing assertion is the active control: the monitor run over levels 0, 1, 0 ends with disposition `{"category":"success","stage":"evaluate","temporal":{"kind":"inactive"}}`, while the batch run over the same records reports `holds`. So the monitor route now reports inactive, with no truth, for a stream whose guard was true. That is a wrong result in the other direction. The worktree holds uncommitted edits to this test (strict comparison for the active case, plus event dumps), so the coder is still debugging it. I have not found the root cause by reading the code: `admitted` is set from the same `letter.when` that opens an activation (stream.rs:414 and 447), and it is carried through `to_bytes`/`from_bytes`. |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | `an_inactive_requirement_is_satisfied_when_closed_and_pending_when_open` is tagged `FR-895-AC-1`. That AC is the all-satisfied order fixture that settles `proved`, and it does not state the new inactive-requirement rule. The new FR-895 Requirement row sentence ("satisfied when the recording is closed and pending when it is open") has no acceptance criterion of its own. Add one, for example FR-895-AC-N, and tag the test to it. | fsl-analyze/tests/it/protocol_recording.rs:1593; spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:76 |

Disposition pass 3 at head 883c0b4a7c (fix commits 74b61f03e6 and 883c0b4a7c), rebased on main c9e9a4d5c8. This is a static review plus /tmp/qsl2590-focused-6.log (the monitor test, 1 passed, exit 0). The draco run of the four focused commands was still in flight.

Soundness of the `decided` change (clause_state.rs:1607-1617). It returns `None` only for the two bounded false-extension profiles, and only when the streamed report is `Inactive`. That report means no guard over the positions seen has read true (stream.rs `inactive()`: closed reading, guarded `on each`, `admitted` false). With no activation there is no obligation, so the open-scope reading has nothing it could legitimately decide. The only decisive answer it could give is the vacuous `Holds` of an empty activation set, described in FND-009. No genuinely decided clause can be `Inactive`. A guard error leaves the report undefined or refused, not inactive. A true guard sets `admitted`. At stream end, the session reports the latest record whether or not it settled, so the inactive result is still reported, as the 0, 0, 0 half of the test shows. The `InfiniteTrace` arm still lists `Inactive` as decided. On a streamed reading it cannot be reached, because `inactive()` requires the closed reading, and a lasso batch is complete, so settling there is correct. The change is sound.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | - |
| FND-008 | fixed | - |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | The finite-window evaluator, which runs every `Progressed` and `Timestamped` trace (`run_window`, temporal.rs:2553), has no inactive result. When a guarded `on each` run has no activation, `Tail::Done` with `done` empty reports `WindowVerdict::Holds`. Its basis is `ClosedScope` under a closed scope and `DecisiveWitness` under an open one, and the truth is true either way. So a progressed trace with a closed, complete scope and every guard false still reports active, truth true (FSpec FR-093-AC-5, closed half). An open scope reports a decisive witness and truth, where AC-5 requires activation-unknown and no truth (open half). This vacuous decisive `Holds` is also the real source of the monitor freeze, which the `decided` guard now catches at the caller. It is pre-existing and not introduced by this PR. Fix it in the evaluator (no activation and a closed scope gives inactive, an open scope gives activation unknown), here or in a linked follow-up ticket. | fsl-eval/src/temporal/finite_window/run.rs:1383-1395 |
| FND-010 | low | FR-895-AC-8 (from main, spec batch 4) also states the item labels: `proved` when the recording is closed, and `inconclusive`/`open-history` when it is open. `an_inactive_requirement_is_satisfied_when_closed_and_pending_when_open` asserts only the requirement obligation's verdict. Add `assessed.global` label assertions to the same test. | fsl-analyze/tests/it/protocol_recording.rs:1593-1598 |
