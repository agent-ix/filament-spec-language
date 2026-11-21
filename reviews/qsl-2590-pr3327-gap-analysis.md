---
id: SR-2026112115372600000000002
title: 'Gap analysis of PR #3327: finite-window inactive and activation-unknown (QSL-2590)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/filament-spec-language; git diff origin/main...HEAD (PR #3327, head 214cdc7acb): FR-044-AC-5, FR-044-AC-7, FR-267-AC-28, FR-820 Equality, FR-895-AC-8 as implemented on the finite-window route; ticket QSL-2590'
review_set: subset
date: '2026-11-21'
relationships:
- target: ix://agent-ix/filament-spec-language/FR-044
  type: reviews
- target: ix://agent-ix/filament-spec-language/FR-820
  type: reviews
---

## Summary

Ticket: QSL-2590. PR: filament-spec-language#3327 at head 214cdc7acb. This is a planless, manual check of acceptance criteria against tests, scoped to the diff. `quoin matrix` was not run because the brief asked for no builds or tool runs on the worktree beyond reading. Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-044-AC-5 | spec/functional/FR-044-activate-temporal-obligations.md:154 | examined |
| FR-044-AC-7 | spec/functional/FR-044-activate-temporal-obligations.md:156 | examined |
| FR-267-AC-28 | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:657 | examined |
| FR-820 Equality | spec/functional/FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md:121-125 | examined |
| FR-895-AC-8 | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:150 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test drives the new no-activation branch through the carried `WindowState` (FR-820), which the observed/monitor session uses (observed.rs:697). Every new test uses the batch `run_window`. The equality FR-820 requires is unchecked on the one path where `base`, frozen positions and silent samples differ (code-review FND-001). Add a test in fsl-eval/tests/it/finite_window.rs. Over a guarded clause with every guard false, advance a `WindowState` across several steps so positions freeze, with silent samples, closed and complete. Assert that each step's verdict, basis and premises equal `evaluate_finite_window` over the same prefix. | fsl-eval/src/temporal/finite_window/carried.rs:199-271 |
| FND-002 | medium | The FR-044-AC-5 open-scope test does not check the `unknown` activation or the `unsettled` settlement the AC and FR-267's `pending` row require. It accepts `truth` `None` or `Pending` and asserts no axis (code-review FND-002, FND-003). | fsl-replay/src/spine/clause/tests/temporal.rs:4114-4124 |

## Verdict

Partial. FR-044-AC-7 and FR-267-AC-28 are covered on the batch finite-window route by `a_finite_window_scope_with_every_guard_false_matches_the_batch_inactive_result`. It checks verdict, category, truth, activation axis and the `closed-scope` basis against the batch run, with a true-guard control. FR-895-AC-8 is covered, with the global label, basis, completeness and cause now asserted for both closures. The open-scope half of FR-044-AC-5 is under-asserted (FND-002). FR-820 equality is untested for the new branch (FND-001).
