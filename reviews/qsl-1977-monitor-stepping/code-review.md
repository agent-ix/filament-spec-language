---
id: SR-2026111612480001
title: "Code review (rust-review lane) of QSL-1977 monitor stepping, PR 2748"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2748 head bd5dcfbf34db10a922dcb2a57560963d297c12c8, git diff d71c0a8df...HEAD: fsl-analyze/src/monitor.rs, fsl-analyze/src/monitor_session.rs, fsl-command/src/command.rs, fsl-command/src/command/monitor.rs, fsl-command/src/command/wire.rs, fsl-command/src/command/output.rs, fsl-replay/src/spine.rs, fsl-replay/src/spine/clause.rs, fsl-replay/src/spine/clause/temporal.rs, fsl-replay/src/spine/native_result/temporal.rs, fsl-replay/src/spine/request.rs, fsl-foundation/src/wire_format.rs, fsl-conformance/src/run.rs, fsl-analyze/tests/it/monitor_incremental.rs, tests/it/run_wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: reviews
---

## Summary

Ticket: QSL-1977. PR #2748 head bd5dcfbf3, merge base d71c0a8df (QSL-2306, #2750). The diff was checked against FR-643, FR-644, FR-283 as amended by QSL-2306, FR-820, FR-502, FR-504 and FR-641. The rust-review lane is folded into this file.

Accepted and out of scope, so not findings: retention bounds approximated by rule pending QSL-2307 (the ignored `Recent` test); `Timed` and `assert_through` refusing until QSL-2254; the driver `fsl monitor` verb; `integer_division` on `native-monitor/1` (ignored under QSL-1989).

Four probes were run as temporary tests appended to `fsl-analyze/tests/it/monitor_incremental.rs`. The file was reverted afterwards and the worktree is clean.
- P1: a `Finite` session over `E` with empty `temporal_selections` refuses at open with `missing_declaration`/`missing-selection`. The batch run over the same trace gives `Temporal(Holds)`.
- P2: `E`'s settled report over levels 0, 1, 2 has the same disposition and top-level basis as `B(3)`. It also carries `ledger` and `finite_window`, which `B(3)` does not, so `Debug` equality fails.
- P3: in a `Timestamped` session (step at stamp 0, progress W 2, step at stamp 3, checkpoint), the original then reports `PendingWindow { at: 0, deadline: 5 }` and the resumed session reports `Pending`.
- P4: with an opening ledger that holds W 5, an observation stamped 3 is accepted. With the newest stamp at 10, an assertion of W 2 is also accepted.

Focused cargo: `cargo test -p fsl-analyze --test it monitor`, `cargo test -p fsl-replay --lib` and `cargo test -p fsl-command`, all through qsl-slot: 138 passed, 2 ignored (fsl-analyze monitor); 1257 passed, 17 ignored (fsl-replay lib); 14 + 15 passed, 8 ignored (fsl-command); tests/it monitor_* and run_wire monitor tests all passed, one ignored (QSL-1989). No Python and no logic-bearing shell in the diff.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | Resume of a progress-carrying session loses a pending clause's report when the last call was not evaluable (newest stamp above the watermark): resume rebuilds it as a synthesized `Pending`, so later steps differ from the original (P3), breaking FR-643 Checkpoint / AC-4 | fsl-analyze/src/monitor_session.rs:770-779, 1229-1236 |
| FND-002 | high | FR-643 Order checks stamps, closure and regression only against the session's own latest assertion, never the opening ledger: an observation at or below the ledger's watermark is admitted, and a regressing assertion is retained unchecked whenever the run is not evaluable (P4) | fsl-analyze/src/monitor_session.rs:535-569, 670-723 |
| FND-003 | medium | A `Finite` session with an event-position or fixed-sample clause runs `authenticate_emission` and refuses empty `temporal_selections`, which the batch `Finite` run accepts (FR-283-AC-17; FR-643 says these members are FR-283's) (P1) | fsl-analyze/src/monitor_session.rs:378-400, 903-911 |
| FND-004 | medium | A closed-trace clause settles with the synthesized `Progressed` open-scope report rather than `B(s)`. It carries a `ledger` and a `finite_window` that `B(s)` lacks (P2), so a `Finite` stream's summary shows a ledger. Batch equality requires exactly `B(s)` while FR-643-AC-7 requires basis `decisive-witness`, which only the open-scope report has. Needs a spec ruling | fsl-analyze/src/monitor_session.rs:847-861, 916-952; fsl-command/src/command/monitor.rs:576-599 |
| FND-005 | medium | `finish` drops the session ledger and `MonitorOutcome` has no final ledger (FR-643 Outputs). The FR-644 summary takes the last clause's report ledger instead, which is stale for a clause settled before later assertions | fsl-analyze/src/monitor_session.rs:1000-1007; fsl-command/src/command/monitor.rs:567-599 |
| FND-006 | medium | Session refusals reach events and the command-error envelope only as `invalid-request` with a `Debug` string. FR-502's contradiction record (`contradicted_dimension`), FR-645 `trace-kind-mismatch` and `missing-selection` codes are lost (FR-644 "reported the same way with FR-643's record") | fsl-command/src/command/monitor.rs:80-94, 553-560 |
| FND-007 | medium | Pasted code corrupted the SPDX header on line 1 and the `Admission::failure` doc comment | fsl-analyze/src/monitor.rs:1, 235 |
| FND-008 | medium | `command::monitor` lacks the `providers` parameter that FR-644 In process requires by FR-635's rule, and the deviation is not recorded | fsl-command/src/command/monitor.rs:640-646 |
| FND-009 | low | Checkpoint identity members are `Debug` strings (`binding`, `lower_boundary`, node identities in `package`), not a closed JSON form. `write_ledger` and `to_vec` errors are swallowed (`.ok()`, `unwrap_or_default`), so a state can silently lose its ledger | fsl-analyze/src/monitor_session.rs:1009-1019, 1051-1054, 1066-1079 |
| FND-010 | low | Resume replays a settled clause over its deciding prefix but never checks that the replay settles. A replay that does not settle silently becomes a pending clause rather than refusing `monitor-state-mismatch` | fsl-analyze/src/monitor_session.rs:1211-1227 |
| FND-011 | low | Change detection compares `format!("{:?}")` of dispositions, and `assert_through` refuses `invalid_runtime_input`/`invalid-value` where FR-643 Through says `invalid-request` naming `through` | fsl-analyze/src/monitor_session.rs:729-735, 635-642 |

## Verdict

REQUEST CHANGES. The settling rules match QSL-2306 ruling 3:
- infinite-trace settles on `Violated`/`Undefined` only, and `Reaches` never settles;
- closed-trace clauses settle on a decisive FR-504 open-scope reading with S = n - 1, which is faithful to FR-504;
- finite-window settles on a decisive basis or under a `Closed` scope.

The settled flag, the summary flag, the event rule, the `end`/`through` record refusals and the closed-scope refusal are correct. The fsl-replay changes keep existing callers' behaviour: `monitor.rs` passes `Some(authentication)`, `ledger: None` and `Some(over)`. No internal fault was found reachable from a request.

FND-001 and FND-002 break checkpoint exactness and the Order rule for progress kinds. FND-004 needs a spec ruling on what a settled closed-trace clause reports.

Fixes:
- FND-001: checkpoint each unsettled clause's report, or the call it was last evaluated at, and restore it. Add a `Timestamped` resume test after an unasserted step.
- FND-002: check observations and assertions against the opening ledger's entry for the binding as well as the session's own assertion.
- FND-003: authenticate only when the kind carries progress. Derive the decided-reading binding without an emission (FR-820: "a Finite trace needs no binding").
- FND-004: report `B(s)` and carry the decided basis separately, or amend FR-643. Either way, a `Finite` report carries no ledger.
- FND-005: add `ledger` to `MonitorOutcome` from the session and use it in the summary only for progress kinds.
- FND-006: render `MonitorRefusal::document()` into the event disposition and the envelope.
- FND-007: restore both lines.
- FND-008: add `providers` or record the deviation in FR-644.
- FND-009: write structured JSON and refuse on a write error.
- FND-010: refuse when a replay does not settle.
- FND-011: compare by value, and use `invalid-request`.

## New findings (disposition pass 1)

Disposition pass 1 reviewed head dbda8a9a9, which is rebased on QSL-2311 (#2752). Rulings from the coordinator: FND-004 is resolved by QSL-2311, and FND-008 waits on QSL-2132.

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-012 | low | A `Finite` session skips `authenticate_emission` only when `temporal_selections` is empty. A non-empty list that is incomplete or mismatched still refuses at open, although the batch `Finite` run reads no selection (FR-283-AC-17's intent). Gate the emission on the kind alone | fsl-analyze/src/monitor_session.rs:422-445 |
| FND-013 | low | `checkpoint()` now returns `Result<MonitorState, Failure>`, against FR-643's `-> MonitorState`. This is acceptable, because the error path is a fault-only serialization path. Record it in FR-643 Inputs together with the other signature deviations (`StageFailure`, `MonitorOpen` by value, `Sample(&str)`, the session lifetime) | fsl-analyze/src/monitor_session.rs:1250 |
| FND-014 | low | `authenticate_trace` returns early whenever `authentication` is `None`, so the `None` branch of `subject_of` (`clock-bearing-trace-has-authentication-rows`) is now dead. The comment says "asserts none", but the trace's binding is skipped even when one is asserted. Existing callers are unaffected: the spine (`clause.rs:1367`, `:1396`) and the batch `monitor` (`Runs::authentication`) always pass `Some` for `Progressed` and `Timestamped` traces | fsl-replay/src/spine/clause/temporal.rs:563-573, 600-606 |

## Dispositions

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | Each unsettled clause checkpoints `evaluated_at` (positions seen, assertion), and resume replays that call (monitor_session.rs:1281-1284, 1481-1496). `a_resumed_timestamped_session_keeps_the_pending_window_report` covers it. |
| FND-002 | fixed | `retained()` seeds Order and regression from the ledger's entry for each clause binding, opening ledger included (monitor_session.rs:583-617, 718-759). `the_opening_ledger_orders_stamps_and_assertions` refuses stamp 3 and W 2 under a ledger at W 5. |
| FND-003 | fixed | A `Finite` session with no selections opens and reads the decided reading unauthenticated (monitor_session.rs:422-431, 1045-1058; `a_finite_session_needs_no_selection_for_a_closed_trace_clause`). A narrower residue remains as FND-012. |
| FND-004 | fixed | Per QSL-2311 (#2752), the settled report is exactly `B(s)` and the deciding reading is in `Settlement`. Tests assert no `finite_window` and no ledger on a `Finite` report, and the settlement members (FR-643-AC-7, FR-644-AC-7). |
| FND-005 | fixed | `MonitorOutcome` gains `ledger` (progress kinds only) and `settlements`, and the summary writes `outcome.ledger` (monitor_session.rs:1157-1182; command/monitor.rs summary). |
| FND-006 | fixed | `RunCause::MonitorRefused` carries `MonitorRefusal::document()` into the envelope, and `session_refusal_event` puts FR-643's record in the event disposition (command/monitor.rs:81-91, session_refusal_event). |
| FND-007 | fixed | The SPDX header and the `Admission::failure` doc comment are restored (monitor.rs:1, 283). |
| FND-008 | deferred | QSL-2132: the provider type FR-635 names does not exist yet (coordinator ruling). |
| FND-009 | fixed | The checkpoint identity is typed JSON (sources, `node_key_to_text`, serde clock parameters, boundary label), and ledger and serialization errors are faults (monitor_session.rs:1186-1305). The signature change is FND-013. |
| FND-010 | fixed | Resume refuses `monitor-state-mismatch` when a settled clause's replay does not settle, and when a pending clause's replay does (monitor_session.rs:1473-1475, 1492-1494). |
| FND-011 | fixed | Status changes use `same_disposition` (temporal verdicts by value), and `assert_through` refuses `invalid-request` naming `through` (monitor_session.rs:689, 1522-1528). |
