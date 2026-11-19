---
id: SR-2026111911313600000006
title: "QSL-957 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-replay/src/certificate/closed_form/task_set.rs; fsl-analyze/tests/it/schedulability.rs; QSL-957"
review_set: subset
date: "2026-11-19"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: references
---

## Summary

Ticket: QSL-957. Frozen PR #3032 head b2a92d2773bd2b2a408a63791493e5913e77d010. Manual changed-scope AC/code/test gap analysis, explicitly narrowed by Peter's instruction; whole-repository matrix and aggregate builds are out of scope. No applicable AssuranceProfile found.

The named empty refusal carries the original QualifiedName, and exhaustive code/cause matches preserve duplicate/unknown exceptions. Retained vectors exercise non-positive periods, sporadic intervals, deadlines and both HI budgets, negative jitter/blocking, budget order and LO dual budgets; priority absence/clash under fixed priority, AMC and response; EDF admission despite both priority faults; and AMC D > T. Existing S3 task_sets::each_defect_refuses_with_its_code_naming_its_task covers BCET, dimensions, names and spans. AC-2 uses fixed-priority iterates [6,10,13] and EDF utilization 13/12, with settlement checked by its companion. AC-7 uses b/job 0/[5,7]; AC-8 runs b/job 0/[2], rejects forged job 2/[4,6,7,8] at closure job 1 and settles replay-parity; AC-9 proves bound 12 and refutes bound 11 with [5,9,12]. Real run/check/settle paths are used; no mocks or gate changes.

## Scope

- FR-246-AC-2; spec/functional/FR-246-check-task-sets-and-schedulability-claims.md; role: examined; excerpt: "Each of these refuses `invalid_timed_model`/`invalid-task-set` naming the task (the task set for the empty one), and both tasks for a clash: a task set with no task; a period of 0 ms; a deadline of 0 ms; a WCET of 0 ms; a BCET of -1 ms; a `LO` task whose BCET exceeds its WCET and a `HI` task whose BCET exceeds its `C(HI)`; a jitter of -1 ms; a blocking term of -1 ms; a sporadic task with minimum inter-arrival time 0 ms; a `HI` task with `C(LO)` 3 and `C(HI)` 2; a `LO` task with dual budgets; a task with no priority under `fixed_priority`; two tasks with priority 1 under `fixed_priority`; a tas"
- FSpec-FR-419-AC-2; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "With `C = 6` for the third task, fixed priority settles `refuted` with the third task, job 0 and iterates 6, 10, 13 (response time 13 above 12); EDF settles `refuted` with utilization `13/12`."
- FSpec-FR-419-AC-5; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "Changing one stated premise changes the obligation identity; an invalid task set refuses `invalid_timed_model`/`invalid-task-set` naming the task; a `fixed_priority`, `amc` or `response` claim over a task with no `priority`, and a `fixed_priority`, `amc` or `response` claim over two tasks of equal priority, refuse `invalid_timed_model`/`invalid-task-set` naming that task (both tasks for the clash), and an `edf` claim over the same sets is admitted; a non-positive `D` and a `LO` task with dual budgets refuse the same way."
- FSpec-FR-419-AC-7; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "Tasks `a` (`C = 1`, `T = D = 4`, priority 1) and `b` (`C = 2`, `T = D = 6`, jitter 1, blocking 3, priority 2) under fixed priority settle `refuted` with task `b`, job 0 and iterates 5, 7 (response time `7 + 1 = 8` above 6)."
- FSpec-FR-419-AC-8; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "Tasks `a` (`C = 1`, `T = D = 2`, priority 1) and `b` (`C = 1`, `T = 3`, `D = 1`, blocking 1, priority 2) under fixed priority settle `refuted` with task `b`, job 0 and iterate 2. Evidence naming `b`'s job 2 with iterates 4, 6, 7, 8 settles `inconclusive`, `replay-parity`, because job 1's completion 6 does not exceed `2 · 3`."
- FSpec-FR-419-AC-9; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "`Ctl`'s `response T3 on Ctl.t3 <= 11 ms under fixed_priority` (`t3` the third task) settles `refuted` with the third task, job 0 and iterates 5, 9, 12 (the bound is the claim's `d`, 11); `<= 12 ms` settles `proved`."
- FSpec-FR-419-AC-10; spec/functional/analysis/FR-419-analyze-task-set-schedulability.md; role: examined; excerpt: "Each of these parses and refuses at check time with `invalid_timed_model`/`invalid-task-set`, never `invalid_syntax`: `taskset Empty { }` (naming `Empty`); a task with `bcet -1 ms`; a task with `bcet 2 ms` and `wcet 1 ms`; a `hi` task with `bcet 4 ms`, `wcet 2 ms`, `wcet_hi 3 ms`; a task with `jitter -1 ms`; a task with `blocking -1 ms`; a `lo` task with `wcet_hi 2 ms`; a task with `blocking 1 ms` under `edf`; a task with `jitter 1 ms` under `edf`; a task with `deadline 5 ms` and `periodic 4 ms` under `amc` (each naming the task). A task set with `t1`'s `jitter -1 ms` and `t2`'s `blocking -1 m"
- TC-701; spec/test-cases/TC-701-task-sets-and-schedulability-claims-check.md; role: context_only; excerpt: "Task sets and schedulability claims check"
- FSpec-TC-372; spec/test-cases/TC-372-schedulability.md; role: context_only; excerpt: "Schedulability"

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for the reviewed source and test scope. No source-level merge blocker found. Merge readiness remains conditional on the existing targeted test run and the lead's single gate; no runtime pass is claimed.

## Coverage

Plan completion: not assessed

Both changed files and the above criteria were examined; no new unspecified behavior, duplicate helper, stub, assertion bypass or weakened gate was found. The newly bound tests cover clauses jointly with existing companion and S3 tests; they do not individually claim every clause of a multi-clause criterion. Specification authoring review does not apply because no spec/plan files changed. Tokio, new wire codecs and unsafe changes do not apply. Whole-repository gap analysis and computed matrix were intentionally skipped under Peter's direct scope restriction.

Touched rustfmt and diff checks passed as supplied by the lead. Scoped Clippy failed in 567 unrelated upstream fsl-semantics lints, retained at /private/tmp/qsl-957-clippy.log; this is not a clean Clippy claim. The existing name-filtered schedulability test run (session 35867, /private/tmp/qsl-957-schedulability.log) remains queued behind the six build slots at review time. No duplicate build or full suite was launched.

### Examined bindings

- a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code -> FR-246-AC-2: correct
- a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code -> FSpec-FR-419-AC-5: correct
- a_third_wcet_of_six_misses_under_fixed_priority_and_edf -> FSpec-FR-419-AC-2: correct
- ctl_settles_proved_with_its_fixpoints_and_a_third_wcet_of_six_settles_refuted -> FSpec-FR-419-AC-2: correct
- jitter_and_blocking_move_the_miss_to_b_and_its_evidence_rechecks -> FSpec-FR-419-AC-7: correct
- a_miss_for_a_job_past_the_busy_period_names_the_job_that_closes_it -> FSpec-FR-419-AC-8: correct
- a_response_claim_holds_at_twelve_and_misses_at_eleven -> FSpec-FR-419-AC-9: correct
- an_edf_claim_refuses_jitter_and_blocking -> FR-246-AC-2: correct
- an_edf_claim_refuses_jitter_and_blocking -> FSpec-FR-419-AC-10: correct
- each_defect_refuses_with_its_code_naming_its_task -> FR-246-AC-2: correct

