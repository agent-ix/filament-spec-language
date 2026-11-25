---
id: SR-669546960
title: "gap-analysis of PR 3911 (QSL-3302) hyper red-test fixes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3911 head 2f5031cd8fdf6c96fd6d345edbd43f1a6946e2be; criteria FR-124, FR-158, FR-169, FR-172, FR-173, FR-176, FR-181, FR-183, FR-1000 touched by the diff"
review_set: subset
---

## Summary

Ticket: QSL-3302. Planless gap analysis scoped to the criteria the diff
touches, from `quire matrix --scope . --format json` on the PR head. Semantic
intent checks for the touched criteria were done in the code-review lane
(SR-919264714).

## Verdict

CONDITIONAL. Every criterion whose test the PR edits is tagged: FR-1000-AC-8,
FR-124-AC-11, FR-124-AC-12, FR-158-AC-1, FR-169-AC-13, FR-173-AC-2 to AC-5,
FR-176-AC-5. The PR adds and removes no trace tag. No stub or coverage
inflation in the diff; every new production function names its owning
requirement (FR-172, FR-173, FR-124). Two criteria that govern change D are
untagged on the head (pre-existing, not removed by this PR).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-172-AC-1 and FR-183-AC-8 are untagged, so the hyper instance set that change D redefines (every universe key; a key outside the universe refused at replay) has no test tagged to its owning criteria; it is exercised only through FR-181-AC-7's `no_instance_and_no_initial_state_decide_nothing` | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:222, spec/functional/FR-183-replay-a-hyper-counterexample.md:282 |

## Coverage

- Matrix: computed by quire matrix on the head; repo-wide untagged count 1053 (not assessed beyond the touched criteria).
- Reverse gap: `ModelSystem::hyper_instances` (FR-172), `ClaimReads::hyper_subject_deadlock`/`opts_out`/`deadlock_item` (FR-173, FR-124), source-route HP-4 candidate override (FR-173, FR-182 V-8) all owned.
- Stubs: none in the diff.
- Semantic review: done in the code-review lane for the touched criteria only.
- Plan completion: not assessed
