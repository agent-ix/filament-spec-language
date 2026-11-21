---
id: SR-2026112115372600000000001
title: 'Code review of PR #3327: finite-window inactive and activation-unknown (QSL-2590)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language; git diff origin/main...HEAD (PR #3327, head 214cdc7acb): fsl-eval/src/temporal/finite_window/{run,outcome}.rs, fsl-eval/src/temporal/evidence/call.rs, fsl-replay/src/spine/clause/temporal.rs, fsl-replay/src/spine/clause/tests/temporal.rs, fsl-analyze/tests/it/protocol_recording.rs; ticket QSL-2590 (FND-009, FND-010)'
review_set: subset
date: '2026-11-21'
relationships:
- target: ix://agent-ix/filament-spec-language/FR-044
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-820
  type: reviews
---

## Summary

Ticket: QSL-2590. PR: filament-spec-language#3327 at head 214cdc7acbb57fa22be31ec03601e2c2a0fa9ccb. This is a static, diff-only review with the rust-review lane folded in. Nothing was built or run. It covers FND-009 (the finite-window evaluator now reports `Inactive` or `ActivationUnknown` for a guarded `on each` clause with no activation) and FND-010 (the FR-895-AC-8 test asserts the global labels).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-044 Outputs, Dispositions, AC-5, AC-7 | spec/functional/FR-044-activate-temporal-obligations.md | examined |
| FR-524 Dispositions of a scope with no instance | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:267-283 | examined |
| FR-267 temporal table, AC-14, AC-18, AC-28 | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:318-336 | examined |
| FR-283 Outputs `Inactive` row | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:250 | context_only |
| FR-622 axes | spec/functional/FR-622-report-activation-execution-truth-and-settlement-as-four-axes.md | context_only |
| FR-820 Equality | spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:121-125 | examined |
| FR-895-AC-8 | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:150 | examined |
| FR-331-AC-9 | spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md:141 | context_only |
| settle no-activation branch | fsl-eval/src/temporal/finite_window/run.rs:1386-1411 | examined |
| Prior.activated | fsl-eval/src/temporal/finite_window/run.rs:161-172 | examined |
| carried state | fsl-eval/src/temporal/finite_window/carried.rs | examined |
| window_verdict mapping | fsl-replay/src/spine/clause/temporal.rs:3680-3683 | examined |
| ClauseRunReport::axes | fsl-replay/src/spine/clause.rs:1515-1587 | context_only |
| anchored evidence call | fsl-eval/src/temporal/evidence/call.rs:1937-1940 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `last` is computed as `self.stamps.iter().take(self.trace.observed)`. `stamps` is indexed from `base`, but `observed` is absolute (`base + held`). For a carried `WindowState` with `base > 0` and silent samples, the take reaches up to `base` silent-sample stamps, so `last` exceeds the batch value and a closed-complete scope reports `ActivationUnknown`/`Pending` where the batch reports `Inactive` (FR-820 Equality). When every observation has been frozen (`positions` empty), `unwrap_or(0)` makes `last` 0, so completeness is checked against tick `margin` rather than the last observation. That is only safe while completeness never retreats, which `WindowState` does not enforce. Fix: take `self.trace.observed.saturating_sub(self.base)` stamps, and when no observation is held use the last observation's stamp, carried on `WindowState` and `TimestampedTrace` next to `first` (for example `last_observed: Option<u64>`). Add a carried-state test: guard false everywhere, positions frozen, silent samples, closed, complete. Compare the report with the batch report. | fsl-eval/src/temporal/finite_window/run.rs:1391-1401 |
| FND-002 | medium | `ActivationUnknown` maps to `TemporalVerdict::Pending`, and `ClauseRunReport::axes` reads a `Temporal(Pending)` as `ActivationAxis::Active` (clause.rs:1542). An open, guard-false scope therefore reports activation `active` with no instance. FR-044 Outputs says "an open scope in the same shape is `unknown` with open completeness", and FR-524 says `Unknown(open)`. The new open-scope test checks only `truth` (`None` or `Pending`), so it cannot catch this. Fix: keep the wire kind `pending`. In `axes()`, add `ClauseDisposition::Temporal(TemporalVerdict::Pending) if self.finite_window.is_some() => ActivationAxis::Unknown`. On the window path a bare `Pending` comes only from `ActivationUnknown`, because an activation's pending is `PendingWindow` or `PendingNextEvent`. In `a_finite_window_open_scope_with_every_guard_false_is_not_decided`, assert `open.axes().activation == ActivationAxis::Unknown` and `open.axes().truth == Some(TruthAxis::Pending)` exactly. | fsl-replay/src/spine/clause/temporal.rs:3681-3683; fsl-replay/src/spine/clause.rs:1536-1542; fsl-replay/src/spine/clause/tests/temporal.rs:4114-4124 |
| FND-003 | medium | `ActivationUnknown` is settled with `Basis::Unavailable`. The written document's top-level `basis` is `TemporalVerdict::Pending.basis()`, which is `unsettled`, as FR-267's `pending` row requires. `axes.settlement` copies the window report's `unavailable` (clause.rs:1582-1585), so one document carries two different settlements. FR-895-AC-8's open item is `unsettled` too. Fix: settle `ActivationUnknown` with `Basis::Unsettled` (run.rs:1406). Assert `window_report(&open).basis == SettlementBasis::Unsettled` in the open-scope test. | fsl-eval/src/temporal/finite_window/run.rs:1404-1409 |
| FND-004 | low | `a_finite_window_unknown_activation_is_neither_inactive_nor_decided` tags FR-044-AC-7 and pins `ActivationIncomplete` for a kernel-undefined guard (`3 mod (2 - q.value)` at value 2). FR-267-AC-14 and FR-331-AC-9 require that guard to be a typed `undefined` (division-by-zero, locus guard `on-each`), which the batch run reports. The window/batch divergence predates this PR. This test records the divergent side as the AC-7 oracle. Fix: drop this test from the PR, or keep it untagged with a comment naming a follow-up ticket that makes the window report `Undefined` for a kernel-undefined guard. | fsl-replay/src/spine/clause/tests/temporal.rs:4126-4146 |

## Verdict

Fix round needed (three medium findings, one low). The core change is sound. A clause that activated cannot be reported `Inactive`: every activation the run drove is in `done`, and every activation before the carried `cursor` was either summarized (`Prior::summarize` sets `activated` first) or skipped for a false guard. `GuardUnresolved` and `Undefined` tails are matched before `Tail::Done`, and a `stopped` carried state never has `Tail::Done`, so an undefined guard anywhere is never `Inactive`. `Prior` and `WindowState` are in-memory only (no serde), so there is no persisted old shape to refuse. The anchored evidence call sets `guarded: false` and cannot reach the new verdicts, so the fault arm is unreachable and acceptable. The changed `timestamped_guarded_on_each_activations_are_evaluated` assertion is a correction, not a weakening. Its trace is closed, complete through 12 and stamped 0 and 10, with every guard false, and FR-044-AC-7 says "a closed-complete scope whose every admitted trigger is guard-false reports `inactive`". The old `Holds` result was vacuous truth. The `Inactive` and `ClosedScope` result matches FR-267's `inactive` row. The FR-895-AC-8 assertions (FND-010) match the AC: closed gives `satisfied` and `proved`, open gives `pending`, `inconclusive` and `open-history`. Tags are TC-1278 and FR-044-AC-5/AC-7, which FR-044 lists. There is no Python and no logic-bearing shell.
