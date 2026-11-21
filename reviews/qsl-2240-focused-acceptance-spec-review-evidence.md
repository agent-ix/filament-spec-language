---
id: SR-224009004
title: "QSL-2240 focused acceptance spec-review-evidence"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md, spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md, fsl-analyze/tests/it/task_automaton_lowering.rs, fsl-analyze/tests/it/schedulability.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: references
---

## Summary

Ticket: QSL-2240. Candidate: 12b2c554c2280b98c127e37e1e4b4797fbbbc361; base: 7aae4c894a5887d02d2dfc110da86fe5319cac7a. GitHub PR 3372 and local HEAD match; local status is clean, and exactly the three requested files changed. No applicable type: AssuranceProfile artifact was found. Review selection is the dispatch-authorized subset.

Evidence review inspected the six named focused tests, exact rational response-time oracles and recorded compile/runtime/check results. Original B: work3 fixpoints (1,3,10), t3 3,6,7,9,10,10 and EDF 53/60; work4 t3 miss 4,7,10,11 and EDF 59/60. Reduced: work3 (1,3,6), t3 3,6,6 and EDF 7/8; work4 t3 4,7,10 > 8 and EDF 1. The real API frontend tests distinguish policy/work cases, and retained EN-6 FP4 requires reproduced replay. These are discriminating assertions: incorrect engine label, lost replay, wrong response iterate, fixpoint, utilization or missed task/job fails. Ordinary defaults flow from request_over without temporal limits through the real command entry and Cancel::new(). CertificateCheckLimits defaults max_check_work to 100000000. The inherited advise/coverage metadata extraction hang was not repeated under the explicit owner direction. The deterministic advisor portion is incomplete, so this is a bounded evidence assessment by inspection, not catalog-backed full-method PASS.

## Verdict

CONDITIONAL — No defects found in the inspected acceptance scope; inherited extraction limits prevent a full-method assurance claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All examined and context-only units are recorded below; excerpts are verbatim portions of the named unit. The complete AC-8 and amended TC text were read.

```json
[
  {
    "id": "FR-249-AC-8",
    "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
    "role": "examined",
    "excerpt": "Task set `B` in `ms`, `(C, T = D)` of `t1 (1, 4)`, `t2 (2, 6)`, `t3 (3, 10)`, `bcet` = `wcet`, priorities 1 to 3, each task released by its own always-enabled operation. EN-7 returns `Schedulable` under `fixed_priority` with fixpoints `(1, 3, 10)`, `t3` iterating `3, 6, 7, 9, 10, 10`, and under `edf` with utilization `53/60`. With `t3`'s `bcet` = `wcet` = 4, EN-7 returns `Miss` for `t3`, job 0, iterates `4, 7, 10, 11` under `fixed_priority` and `Schedulable` (utilization `59/60`) under `edf`. Original `B` remains an EN-7 acceptance fixture. EN-6 retains its configured default limits; no comple"
  },
  {
    "id": "TC-3008-description",
    "path": "spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md",
    "role": "examined",
    "excerpt": "Verify that the lowered task-product subject carries the declared periods\nand deadlines, that EN-7 checks all four work/policy cases of the retimed\nthree-task set `BReduced`, that EN-6 agrees on its work-4 fixed-priority\nrefutation with reproduced replay, that an unschedulable set\nyields a replayed counterexample whose first deadline miss is exact, and\nthat a non-positive period is refused at check. Scope: FR-249-AC-7 to\nFR-249-AC-10.\n"
  },
  {
    "id": "TC-3008-step-2",
    "path": "spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md",
    "role": "examined",
    "excerpt": "2. Analyze `schedulable BFp on B under fixed_priority;` and\n   `schedulable BEdf on B under edf;` on EN-7; repeat for the work-4 variant.\n   Original `B` has no EN-6 baseline measurement in this test procedure.\n   Analyze the corresponding four `BReduced` work/policy cases on EN-7.\n   Analyze only the `BReduced` work-4 fixed-priority claim on EN-6 with\n   ordinary defaults, and read its reproduced counterexample replay."
  },
  {
    "id": "TC-3008-expected-step-2",
    "path": "spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md",
    "role": "examined",
    "excerpt": "- Step 2: EN-7 returns `Schedulable` with fixpoints `(1, 3, 10)` (`t3`\n  iterating `3, 6, 7, 9, 10, 10`) and, under EDF, utilization `53/60`;\n  original `B` remains an EN-7 acceptance fixture. EN-6 retains its configured\n  default limits; no completed EN-6 baseline or first exhausted bound is\n  claimed for it. For original work 4 EN-7 returns `Miss` for\n  `t3`, job 0, iterates `4, 7, 10, 11`, and EDF `Schedulable` at `59/60`.\n  On `BReduced`, work 3 has fixed-priority fixpoints `(1, 3, 6)`, `t3`\n  iterates `3, 6, 6`, and EDF utilization `7/8`; both EN-7 claims prove.\n  Work 4 has fixed-priorit"
  },
  {
    "id": "FR-249",
    "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
    "role": "context_only",
    "excerpt": "A timed model whose `release S.τ on M::o;` declarations bind source\ntransitions to tasks of a checked task set is a task automaton. S3 SHALL\ncompile it with the selected scheduler to an ordinary unit-rate timed product\nand EN-6 SHALL check `always holds(not deadline_missed)` on that product.\nThe product SHALL preserve the source state and exact job identities while\nrepresenting the scheduler state and task-derived timed data. Its task\nsemantics, job identity, demand, release and mode-change rules, bounded\nsubtraction contract, and certificate/replay carriers are owned by FSpec\nFR-419"
  },
  {
    "id": "task_automaton_lowering-AC8",
    "path": "fsl-analyze/tests/it/task_automaton_lowering.rs",
    "role": "examined",
    "excerpt": "fn task_set_b_reduced(third: u32) -> String {\n    task_set_b(third)\n        .replace(\"taskset B\", \"taskset BReduced\")\n        .replace(\"on B under\", \"on BReduced under\")\n        .replace(\"release B.\", \"release BReduced.\")\n        .replace(\"periodic 4 ms deadline 4 ms\", \"periodic 6 ms deadline 6 ms\")\n        .replace(\n            \"periodic 10 ms deadline 10 ms\",\n            \"periodic 8 ms deadline 8 ms\",\n        )\n}\n\nfn reduced_en7(third: u32, claim: &str) -> Value {\n    run(\n        &format!(\"b-reduced-{third}-{claim}-en7\"),\n        &[\"t1\", \"t2\", \"t3\"],\n        &task_set_b_reduced(th"
  },
  {
    "id": "schedulability-AC8",
    "path": "fsl-analyze/tests/it/schedulability.rs",
    "role": "examined",
    "excerpt": "fn b_response_time_results(\n    periods: [i64; 3],\n    third_fixpoint: i64,\n    three: &[i64],\n    four: &[i64],\n    loads: [Rational; 2],\n) {\n    let fixed = schedulable(b_set(3, periods), SchedulingPolicy::FixedPriority);\n    let outcome = run(&fixed);\n    let points = fixed_points(&outcome);\n    assert_eq!(fixpoints(points), vec![n(1), n(3), n(third_fixpoint)]);\n    assert_eq!(points[2].jobs, vec![JobIterates(iterates(three))]);\n    assert_eq!(\n        settled(&fixed, &outcome),\n        proved(ClosedFormAnalysis::FixedPriorityRta)\n    );\n    let missed = schedulable(b_set(4, perio"
  },
  {
    "id": "FR-249-AC-8-qualification",
    "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
    "role": "examined",
    "excerpt": "On `BReduced`, EN-6 under ordinary defaults refutes work-4 fixed priority, matches that EN-7 label, and carries reproduced counterexample replay. The other three EN-6 agreements are not asserted: the default checker work budget is 100000000 and there is no default wall-time deadline. No EN-6 verdict or typed limit stop is claimed for those three cases."
  },
  {
    "id": "TC-3008-retiming",
    "path": "spec/test-cases/TC-3008-a-task-automaton-carries-its-timing-and-agrees-with-response-time-analysis.md",
    "role": "examined",
    "excerpt": "This retiming minimizes the maximum period in the integer-ms domain with\nunchanged work and all four EN-7 policy/work outcomes preserved: if every period\nwere at most 7, work-4 EDF utilization would be at least `7/7`; equality\nrequires all three periods to be 7, where fixed priority also proves, losing\nthe required refutation. Maximum period 8 is therefore minimal. The\nhyperperiod is 24 ms versus the original 60 ms. This is retiming, not uniform\nor componentwise period downscaling; no smaller region quotient is assumed.\n\n1. Analyze `schedulable PairFp on Pair under fixed_priority;` unnamed and"
  }
]
```

Six trace bindings were manually checked as correct; no new criteria or coverage count is claimed. Source unchanged. Existing compilation succeeded and six exact named tests passed (0 failed, 0 ignored) in 0.74 seconds under an external hard 10 second watchdog, runtime exit 0. No new compilation, runtime, benchmark, old-head comparison or broad gate was run. Existing touched formatting and targeted spec validation each exit 0; independent diff check passes. Full matrix/coverage/advise extraction and full-repository gap assurance remain outside the accepted bounded review. Plan completion: not assessed.

## Tracker custody

No Linear query or write was performed. Marked comment prepared locally only; deduplication, posting, read-back and has-review lookup/addition remain deferred to the lead after the user hold. This artifact is canonical scratch output for verbatim lead custody; reviewer made no repository edits, pushes or merges.

