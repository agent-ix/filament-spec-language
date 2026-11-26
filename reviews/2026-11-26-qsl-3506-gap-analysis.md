---
id: SR-5590836
title: "Gap analysis of QSL-3506 changes in PR 4037"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze, fsl-eval, fsl-replay (QSL-3506 changes of PR 4037)"
review_set: subset
---

## Summary

Ticket: QSL-3506. Planless audit limited to the criteria and tests the QSL-3506 changes touch. quire matrix shows each tagged criterion backed; three tags are wrong or missing.

## Verdict

**CONDITIONAL**: low trace findings only. compared_constraints, source_clock_maxima, Scale::of and run_timed are untagged, which the PR declares; source_clock_maxima's family rule has no FSL criterion. No stub or placeholder was added. Mutation check at head: with the family walk removed from fsl-replay timed::clock_constraints and fsl-eval compared_constraints, bounds_below_a_quantifier_family_constant_are_rejected and the three family_constants tests fail (4 of 4); reverted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | a_task_automaton_ceiling_reads_a_quantifier_family is tagged FR-249-AC-3 (task-product state and derived-step identity across preemption/resumption, and replay) but asserts only that a source clock's ceiling is 5. The matrix now counts a test that does not exercise the criterion. No FSL criterion for the ceiling was found; tag none and say so. | fsl-analyze/tests/it/family_constants.rs:138-140 |
| FND-002 | low | a_time_lock_past_a_quantifier_family_constant_reproduces carries only TC-692 and no criterion, against the repository rule that a new test names the criteria it asserts. It asserts the non-local time-lock replay of FR-237-AC-4. | fsl-analyze/tests/it/family_constants.rs:90-92 |
| FND-003 | low | computed_bounds is marked implements FR-245-AC-7, which states rejection rules and loci. The function produces no rule or locus; the Malformed-at-bounds rejection is raised by its caller. | fsl-replay/src/certificate/zone_certificate.rs:4510-4516 |

## Examined

- examined: FR-236-AC-3
- examined: FR-245-AC-2
- examined: FR-245-AC-7
- examined: FR-249-AC-3
- examined: FR-237-AC-4
- examined: FR-852-AC-2

## Coverage

Plan completion: not assessed. Semantic review skipped. Full-repository reverse-gap inventory not run; scope is the changed code. FSpec criteria are external to this repository's matrix.
