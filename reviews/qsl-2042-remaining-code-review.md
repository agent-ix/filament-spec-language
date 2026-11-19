---
id: SR-2026111911313600000015
title: "code-review of QSL-2042 remaining temporal source routes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3071; tests/it/temporal_package_route.rs; tests/it/temporal_package_route/remaining_routes.rs; tests/corpus/temporal/remaining/**"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-194
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: references
---

## Summary

Ticket QSL-2042, PR 3071, frozen head 56f74d6927f8ccb9aef09ee2adb8ffa13855f8da. Diff-scoped code-review found one narrow record-window oracle gap. The same fix is recorded by the companion method; one additional trace suffices, not a broader test-suite expansion.

## Verdict

**CONDITIONAL for the reviewed diff.** Fix FND-001 before claiming the remaining record-window behavior fully guarded. The other seven added tests have meaningful controls. This is not a repository-wide assurance or plan-completion verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The record-count vectors never require a historical value to change the verdict: all-true is tested, false-first is refuted at origin, and insufficient-history is also checked only at origin. A regression that counts only the current position passes these cases. Add a history-dependent trace such as true,false,true, expected tested for Count's past[0,2] >= 1, which a current-only implementation refutes at position 1. | tests/it/temporal_package_route/remaining_routes.rs:130 |

## Coverage

Code review includes the Rust review/style lane. Examined the complete ten-file diff: eight original native fixtures, the parent module registration and helper visibility changes, and all eight new test functions. Public compile/run/monitor/analyze commands are exercised directly. Shared authoritative rover/lock inputs are read through the explicitly authorized FSL_EXAMPLES_DIR root; no source/model/request copy, compatibility layer, ignored test, unsafe code, new public production API or gate weakening is introduced. Test-only tempfiles, unwraps and deterministic source-derived loci follow existing harness idioms. Model aggregate refusals have an accepting no-aggregate control and stable code/cause/location assertions. Trace-kind refusals name supplied/expected profile variants; sampled and timestamped cases distinguish completion, silent positions, inclusive deadlines and open windows. Monitor tests cover successful/violating streams plus missing-clause refusal before input reads. Analyze tests compare two counterexample loops with a proved certificate, unknown-claim disposition and bounded incomplete result. The count table is the single concrete oracle weakness: its false-first vector never needs the past window to preserve a true verdict.

Reported evidence accepted: all eight remaining_routes tests passed across focused runs (six initially, then the changed aggregate clock and analyze-wire oracles). Prior merged-slice 13-program/five-journey/22-rover evidence is accepted unchanged. No test, build, computed matrix, full gate or peer measurement was repeated. The user limits this review to the PR diff; the saved matrix is historical context, not a whole-repository coverage verdict. The lead owns the final gate.

No applicable AssuranceProfile was selected in the lead's established context; no new profile inventory was performed. Spec review is inapplicable because no spec artifact changed. Plan completion: not assessed.

## Examined Scope

```json
{
  "scope": [
    {
      "id": "model_subject_window_aggregates_refuse_at_the_aggregate",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn model_subject_window_aggregates_refuse_at_the_aggregate() {"
    },
    {
      "id": "record_window_count_has_true_false_and_insufficient_history_cases",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn record_window_count_has_true_false_and_insufficient_history_cases() {"
    },
    {
      "id": "timed_model_declines_untimed_snapshots_with_trace_kind_evidence",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn timed_model_declines_untimed_snapshots_with_trace_kind_evidence() {"
    },
    {
      "id": "sampled_replay_settles_silent_positions_and_reports_incomplete_input",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn sampled_replay_settles_silent_positions_and_reports_incomplete_input() {"
    },
    {
      "id": "timestamped_replay_distinguishes_inclusive_deadlines_and_open_windows",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn timestamped_replay_distinguishes_inclusive_deadlines_and_open_windows() {"
    },
    {
      "id": "public_monitor_streams_settle_the_three_clock_families",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn public_monitor_streams_settle_the_three_clock_families() {"
    },
    {
      "id": "public_monitor_refuses_a_missing_clause_before_consuming_the_stream",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn public_monitor_refuses_a_missing_clause_before_consuming_the_stream() {"
    },
    {
      "id": "public_analyze_proves_fair_each_service_and_refutes_unfair_service",
      "path": "tests/it/temporal_package_route/remaining_routes.rs",
      "role": "examined",
      "excerpt": "fn public_analyze_proves_fair_each_service_and_refutes_unfair_service() {"
    },
    {
      "id": "remaining_routes",
      "path": "tests/it/temporal_package_route.rs",
      "role": "examined",
      "excerpt": "mod remaining_routes;"
    },
    {
      "id": "model-control",
      "path": "tests/corpus/temporal/remaining/model-control.native",
      "role": "examined",
      "excerpt": "temporal HasStopped using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-count",
      "path": "tests/corpus/temporal/remaining/model-window-count.native",
      "role": "examined",
      "excerpt": "temporal Count using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-sum",
      "path": "tests/corpus/temporal/remaining/model-window-sum.native",
      "role": "examined",
      "excerpt": "temporal Sum using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-min",
      "path": "tests/corpus/temporal/remaining/model-window-min.native",
      "role": "examined",
      "excerpt": "temporal Minimum using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-max",
      "path": "tests/corpus/temporal/remaining/model-window-max.native",
      "role": "examined",
      "excerpt": "temporal Maximum using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-fraction",
      "path": "tests/corpus/temporal/remaining/model-window-fraction.native",
      "role": "examined",
      "excerpt": "temporal Fraction using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "model-window-quantile",
      "path": "tests/corpus/temporal/remaining/model-window-quantile.native",
      "role": "examined",
      "excerpt": "temporal Quantile using e over (r: Fleet::Rover) clock \"model-steps\" on origin {"
    },
    {
      "id": "record-window-count",
      "path": "tests/corpus/temporal/remaining/record-window-count.native",
      "role": "examined",
      "excerpt": "temporal Count using e over (p: Pulse) clock \"events\" on origin {"
    },
    {
      "id": "FR-194-AC-2",
      "path": "spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md",
      "role": "examined",
      "excerpt": "Under event-position, `count where holds(e is Error) over past[0, 9] <= 2` at position 4 reads positions 0 to 4 only; at position 15 it reads positions 6 to 15. `fraction holds(e.ok) over past[0, 3]` over `ok` values `true, false, true, true` is `3/4` at position 3. `sum`, `min` and `max` of `1, 4, 2` are 7, 1 and 4."
    },
    {
      "id": "FR-194-AC-4",
      "path": "spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md",
      "role": "context_only",
      "excerpt": "S3 refuses an aggregate term in a claim over a model subject"
    },
    {
      "id": "FR-641",
      "path": "spec/functional/FR-641-carry-every-temporal-trace-kind-and-the-progress-ledger-on-the-run-request.md",
      "role": "context_only",
      "excerpt": "The `temporal` selection of FR-312's clause-run request SHALL carry every trace"
    },
    {
      "id": "FR-644",
      "path": "spec/functional/FR-644-stream-observations-through-the-monitor-command.md",
      "role": "context_only",
      "excerpt": "Stream observations through the monitor command"
    },
    {
      "id": "FR-621",
      "path": "spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md",
      "role": "context_only",
      "excerpt": "There is no separate analyze result format."
    },
    {
      "id": "authoritative-fairness-program",
      "path": "filament-examples:examples/lock/rules/fairness/one-served.fsl",
      "role": "context_only",
      "excerpt": "fair strong each Mutex::Lock::acquire;"
    }
  ]
}
```

