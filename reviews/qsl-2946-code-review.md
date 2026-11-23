---
id: SR-9138
title: "Code review: QSL-2946 caller limits batch (PR 3597)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze, fsl-command, fsl-foundation, fsl-replay, fsl-semantics at 4c7671e65f; Ticket: QSL-2946"
review_set: subset
evaluated_revision: "4c7671e65f"
---

## Summary

Ticket: QSL-2946. The engine-bound plumbing (EngineBounds, ProbabilisticLimits::under, refuse_engine_conflict) is consistent across the source and package routes for the settings it threads. One doc comment misstates the invariant.

## Verdict

MERGE after the gap-analysis findings are resolved; this method has one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The ENGINE_SETTINGS doc says no accepted setting is ignored, but smt.max_solver_* and certificate_check.max_rank are listed and read by nothing | fsl-analyze/src/analyze.rs:680-686 |

## Coverage

Examined limits.rs exposure lists, refuse_engine_conflict call sites in analyze and monitor, EngineBounds::conflict, and the replay refusal lists. The run_text_witness failure is not caused by this diff (see ticket comment).
