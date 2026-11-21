---
id: SR-202611210022442313144108541
title: "QSL-2750 bounded implementation gap review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2750 PR3346; new FR643/644 AC13/14 and monitor implementation/tests"
review_set: subset
---

## Summary

No new missing criterion binding, unowned behavior, stub or hollow test was found in the frozen PR3346 implementation at 96033bff06dc494dfe4615ad85f4e1f71aa4f076. Plan completion: not assessed

## Verdict

CONDITIONAL — bounded new behavior has real static binders and source/test alignment; five focused runtime results remain pending. No whole-repository PASS is claimed, and the requested bounded review does not reclassify inherited audit debt as PR defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No new behavior gap found in the inspected scope. | - |

## Coverage

The installed static engine computes all four new criteria as tagged: FR-643-AC-13 has two binders (analyze prefix/checkpoint and command/batch), FR-643-AC-14 has one (analyze refusal/rollback), FR-644-AC-13 has one (command/batch), and FR-644-AC-14 has two (malformed recovery and non-timestamped empty rejection). They identify five distinct nonignored tests, with real production calls and observable negative assertions. None uses a mock of the evaluator, a test-only production branch, a source-text oracle or a skip.

Baseline matrix context only: 634 requirements, 6225 criteria, 4881 tagged, 1144 untagged, 81 tagged-by-ignored-test, 106 method-without-symbol, 13 retired. These inherited repository totals and diagnostic debt are not introduced PR findings. No qualified run evidence was read.

On terra, /home/peter/.npm-global/bin/quoin 0.36.8, `quoin matrix --repo . --json` in /home/peter/dev/worktrees/terra-codex2-qsl-2750 exited 1 with `Error: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`; expected a computed matrix, stdout empty. The documented fallback /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2), `quire matrix --scope . --format json`, exits 0. Its static matrix and extensive preexisting extraction diagnostics are retained in this scratch directory, and the four new binders were inspected directly in its JSON. Static tagging establishes neither execution nor assurance qualification.

## Reverse gaps and stubs

Two production files introduce four bounded behaviors: public missing evidence input, semantic admission and atomic handoff (FR643 AC13/14), strict stream decode and recovery/default/kind behavior (FR644 AC13/14). All are owned by the reviewed contract. Four new API/behavior obligations mapped, zero unowned in the delta; zero source stubs, zero test stubs. The five tests exercise genuine compiler/session/batch/command seams. Semantic alignment was included by owner request.

The public borrowed slice has a command consumer and a reviewed FR-643 contract. Admission checks kind, accepted observation index, checked usize conversion and actual Holds/Aggregate node membership for every selected clause before sample admission or session mutation. Only admitted atom IDs enter AdmittedPosition.absent; merged upstream c5be407d owns normalization, carried-window evaluation and strict checkpoint bytes. Existing transactional advancement/rollback is reused; refused entries do not advance observation or counted-call indices.

The command uses its existing quire-canonical dependency to detect duplicate raw names before serde_json::Value could discard them. The decoder refuses malformed list/entry shapes, absent/null/unknown/wrong-type/out-of-u64 members and non-timestamped presence (including empty); omission defaults to empty. It forwards the validated slice to the real session, and existing refusal/recovery processing continues with the next record. No new producer/consumer schema copy, external semantic implementation or aggregate-gate wiring was added.

The analyze parity test runs four combinations (absence/no absence and checkpoint/no checkpoint), comparing each step and each progress assertion with the real whole-trace batch report. Final absence must report MissingValuation at position18; the control must violate. Invalid indices, nonatom and out-of-range IDs and Finite/Progressed kind errors leave checkpoint bytes unchanged, followed by a good accepted observation. Timed sessions already refuse at open; no reachable Timed-step evidence is claimed. The command tests run a real native batch request, check indeterminate exit11 versus normal exit10, byte-equal omitted/empty streams, count all malformed refusal events and compare subsequent events/clauses/ledger with the clean stream.

## Validation and boundaries

No Cargo command ran in this reviewer lane. Parent reports focused check of both modified libs and their it test targets passed at 96033bff06dc494dfe4615ad85f4e1f71aa4f076. The two analyze and three command runtime tests are delegated to the parent/Titan lane and remain pending here. git diff --check passes. Broad gates and workspace runs were explicitly prohibited; no result is inferred for them. No CI, Cargo manifest, dependency or workflow diff narrows a gate.

Repository AGENTS/CLAUDE/CONTRIBUTING, workspace unsafe/documentation lints and Agent IX rust-style API, implementation, safety, wire and testing rules were applied. Existing source SPDX headers remain. No Tokio or unsafe path is introduced. No applicable AssuranceProfile exists under spec. The completed contract base/integrity reviews remain preserved; they were not restarted. This scoped review does not claim a whole-repository reverse-source audit or a whole-repository PASS.

## Examined scope

```yaml
clean: true
scope:
- id: FR-643-AC-13
  path: spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md
  role: examined
  excerpt: 'Feed FR-194-AC-17''s timestamped observations one at a time, marking the
    aggregate atom absent with `{position: 18, atom: A}` on observation 18, then assert
    its closed progress complete through 300000. Each pending report equals the corresponding
    batch prefix with the same `missing` entries; the final report is `indeterminate`
    with `MissingValuation { position: 18, atom: A }`, never `invalid-value` or a
    quantile over the other values. A checkpoint immediately after observation 18,
    resumed before observation 19 and the progress assertion, gives the same reports
    and ledger as the uninte'
- id: FR-643-AC-14
  path: spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md
  role: examined
  excerpt: At next observation index 1, a missing entry whose position is 0 or 2,
    or whose atom is not a `holds` or aggregate node admitted by the selected clauses'
    batch run, refuses `invalid_runtime_input`/`invalid-value` naming `missing`. A
    nonempty missing slice on a `Finite`, `Progressed` or `Timed` session refuses
    likewise. Each refusal leaves checkpoint bytes and observation/count-call indices
    unchanged; the corrected observation is accepted at index 1. An empty slice is
    accepted wherever the same observation without absences is accepted.
- id: FR-644-AC-13
  path: spec/functional/FR-644-stream-observations-through-the-monitor-command.md
  role: examined
  excerpt: 'Stream FR-643-AC-13''s timestamped observations with `missing: [{"position":
    18, "atom": A}]` on observation 18. Events and final summary match the session
    and batch reports, including `MissingValuation { position: 18, atom: A }` and
    exit 11. An omitted `missing` and `missing: []` give identical events and summary
    for the same timestamped stream.'
- id: FR-644-AC-14
  path: spec/functional/FR-644-stream-observations-through-the-monitor-command.md
  role: examined
  excerpt: 'Between good timestamped observations, supply records with `missing: null`,
    a non-array value, entries with absent, unknown, duplicate or null members, negative,
    fractional, string or overflowing-u64 `position` or `atom`, and a duplicate top-level
    `missing`. Each yields one stage-`request` `invalid-request` refusal event naming
    the member and changes no session state; subsequent good records give the events
    and final clause reports of the stream without the refused records. A `missing`
    member on a progress record, or on a non-timestamped observation even as `[]`,
    is likewise refused '
- id: session-missing
  path: fsl-analyze/src/monitor_session.rs
  role: examined
  excerpt: '    pub missing: &''a [TimestampedMissingValuation],'
- id: session-admission
  path: fsl-analyze/src/monitor_session.rs
  role: examined
  excerpt: '            if kind != MonitorTraceKind::Timestamped || missing.position
    != self.seen {'
- id: session-carried-handoff
  path: fsl-analyze/src/monitor_session.rs
  role: examined
  excerpt: '                .map(|entry| entry.atom)'
- id: stream-decoder
  path: fsl-command/src/command/monitor.rs
  role: examined
  excerpt: '    quire_canonical::read(line, bound).map_err(|error| {'
- id: stream-default-and-kind
  path: fsl-command/src/command/monitor.rs
  role: examined
  excerpt: '            None => Vec::new(),'
- id: stream-session-handoff
  path: fsl-command/src/command/monitor.rs
  role: examined
  excerpt: '                        missing: &item.missing,'
- id: qsl_2750_missing_window_matches_batch_and_checkpoint
  path: fsl-analyze/tests/it/monitor_incremental.rs
  role: examined
  excerpt: fn qsl_2750_missing_window_matches_batch_and_checkpoint() {
- id: qsl_2750_missing_marker_refusals_preserve_session_state
  path: fsl-analyze/tests/it/monitor_incremental.rs
  role: examined
  excerpt: fn qsl_2750_missing_marker_refusals_preserve_session_state() {
- id: qsl_2750_stream_missing_window_matches_the_real_batch_run
  path: fsl-command/tests/it/cli_contract/monitor.rs
  role: examined
  excerpt: fn qsl_2750_stream_missing_window_matches_the_real_batch_run() {
- id: qsl_2750_malformed_missing_entries_are_refused_atomically
  path: fsl-command/tests/it/cli_contract/monitor.rs
  role: examined
  excerpt: fn qsl_2750_malformed_missing_entries_are_refused_atomically() {
- id: qsl_2750_empty_missing_is_refused_on_other_trace_kinds
  path: fsl-command/tests/it/cli_contract/monitor.rs
  role: examined
  excerpt: fn qsl_2750_empty_missing_is_refused_on_other_trace_kinds() {
findings: []
```

## Runtime fix round 1

The source96033bff runtime receipt is4PASS/1FAIL, so the earlier conditional verdict did not establish acceptance. The failed analyze parity test caught an actual FR820 stale-report defect, recorded as FND-002 in the code review and statically dispositioned fixed at67dbd9c5e2414a006b7293c08542c1c4a760db98. The original batch oracle and all five selected tests are preserved. The same analyze test now also calls a real out-of-order2,1,3/checkpoint/absence regression; it checks saved mutable order, original values, null provisional ledger and canonical cursor, byte-equal resume, and final batch parity after real progress. No missing binding, test stub or further bounded gap was found in this fix. Static binding counts above remain applicable to the same test functions; no new repository matrix or broad audit was run.

CONDITIONAL remains: parent reports focused compilation PASS at67dbd9c5 (56s), but the same five runtime results are pending. This is not a whole-repository PASS.

## Focused round 2 evidence

The partial runtime receipt at67dbd9c5 is analyze1PASS/1FAIL: the no-absence control rejects the correct frozen deciding counterexample after settlement at step19. This oracle mismatch is recorded as code FND-003 and statically fixed in the test-only delta0629b603a6df55786546996c3c7fefded130cdcc. Pending reports still compare all members with current B(k); settled reports compare all twelve nonusage fields with independently captured B(s), while category/truth must match current B(k) on every call and finish. Independent resume/finish probes cover both pending and settled checkpoints. The original missing-evidence and ordering tests remain, and the same tagged test now invokes a no-marker high6/late-low1 regression requiring Pending/full batch parity until actual progress, with independently evaluated violation and success controls. No production or gate code changes and no test removal occurred. FND-002's real provisional-report fix remains untouched. Static scope CLEAR, no additional gap identified; runtime CONDITIONAL pending the same five rerun. No whole-repository PASS or new matrix run is claimed.

## Architecture-authorized fixture alignment

The parent reports the same five-test run at0629b603 as4PASS/1FAIL. The remaining analyze failure is uncoveredstep0: authenticated_temporal_selection is Some in the session and None in the batch fixture. Architecture authorized one confined fixture correction after the two focused rounds. This is separate from the real provisional-report bug (FND-002) and the settled-report oracle bug (FND-003); their original findings and dispositions remain preserved.

The exact delta0629b603..c744e7ab6c285faf9a3068dde5a72c469a62576f changes only monitor_incremental.rs. All eight QSL2750 batch comparisons now call a dedicated fixture helper. That helper invokes the same production authenticate_emission with the same checked package, world temporal selections, package input and SpineLimits::default as MonitorSession::open, and passes its PackageAuthentication::temporal to the real PackageTemporalRequest. Other evaluator/request inputs match the existing oracle: default normalization/observation limits, unlimited accounting, same source and trace, no supplied ledger. The fixture generates its own authentication through the real API; it does not copy the session's result or modify any expected report.

The full twelve-field comparison remains byte-for-byte unchanged, including authenticated_temporal_selection; none is omitted or normalized away. All pending B(k), frozen independent B(s), every-call current category/truth, checkpoint and late-input/order/missing-value assertions remain. No product code, gate or test selection changed. The independent narrow alignment disposition is CLEAR: the architecture ruling is fulfilled and the two compared contexts now match. Parent reports --locked focused compilation PASS in33.55s atc744e7ab. git diff --check passes. No Cargo, fresh full review or wider measurement ran in this reviewer lane.

Runtime status remains CONDITIONAL pending the same five focused tests atc744e7ab. Linear operations are deferred during the owner-imposed blackout through2026-11-21T19:37Z; this review made no Linear query, comment, state change or read-back.

## Final focused runtime verdict

PASS for the bounded QSL-2750 code and acceptance scope at c744e7ab6c285faf9a3068dde5a72c469a62576f. Parent-relayed Titan final result: c1 rc0 2 PASS / 0 FAIL / 0 IGNORE; c2 rc0 3 PASS / 0 FAIL / 0 IGNORE; all five selected tests pass. This clears the pending focused runtime condition in the earlier sections. The completed tests include the original missing-window parity, refusal recovery, strict stream reader, frozen deciding-report oracle, checkpoint and uncovered/late-input controls described above. Earlier runtime failures and their fix dispositions remain preserved.

No open finding remains in this bounded review. This is no whole-repository matrix, evidence qualification, or broad-gate PASS. The inherited quoin accepted-size refusal, static-only matrix limitations and baseline assurance debt above remain unchanged. No further review, Cargo command, archive or external receipt request ran here. Linear remains untouched during the owner blackout; its one consolidated evidence update remains deferred.
