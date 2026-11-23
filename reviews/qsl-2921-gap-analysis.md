---
id: SR-967175026302197768900474181941935084786
title: "Gap analysis of QSL-2921 subsystem refutation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3552 head eaccebc42040d1a21b51e308ac03ef17bf8120dc; FR-202-AC-21, FR-202-AC-31..39, FR-202-AC-41, FR-202-AC-42, FR-203-AC-15, FSpec FR-412-AC-24, FSpec FR-413-AC-16 against the PR's tests and code"
review_set: subset
---

## Summary

Ticket: QSL-2921. A planless gap analysis scoped to the criteria this PR implements. The full `quire matrix` was not run, because it took 15 minutes in another review of this repo. A grep index of `#[trace(...)]` tags over the PR head stands in for the matrix and is declared as such. Every in-scope criterion has at least one tagged test that exercises it: AC-31 to AC-35 in subsystem_refutation.rs, AC-36 to AC-39, AC-41 and AC-42 in cyclic_state_refutation.rs, AC-35 also in cyclic_reward_certificate.rs, and FSpec FR-412-AC-24 and FR-413-AC-16 in TC-432. Reverse gap: every new code unit has an owning requirement (the test-only `path_searches` counter serves AC-36 and AC-41 instrumentation; `avoiding_policy` serves AC-34 and AC-38 witness schedulers). No stubs or coverage inflation were found. The caller requested intent review, so it ran inline.

## Verdict

**CONDITIONAL**: no untagged criterion. One spec-side inconsistency (AC-33) and minor vocabulary and coverage findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-202-AC-33 is internally inconsistent: an every-scheduler `expected total >= 1` with induced total 0 needs an initial target state, so no choice is made and the counterexample cannot "include the optimizer's scheduler" or fail when it is omitted. TC-3158 handles this by splitting into a degenerate θ=1 case with an empty scheduler and a θ=2 variant the AC does not name; the spec needs correcting | spec/functional/FR-202-replay-a-probabilistic-witness.md:667; fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:428-542 |
| FND-002 | low | Refutation documents for cyclic always/until wrap the acyclic `state-predicate/v1` base, while eventually and reward wrap cyclic profiles, and #3551 (merged on main after this PR's base) defines `cyclic-dtmc-state-predicate/v1` and `cyclic-mdp-state-predicate/v1`. Consider sharing that vocabulary in a follow-up (flag, not a merge demand) | fsl-replay/src/probability_certificate/document/refutation.rs:265-330; fsl-analyze/tests/it/exact_probabilistic_item/cyclic_state_refutation.rs:321-325 |
| FND-003 | low | No test drives the new checked-subject entry through limit exhaustion or cancellation, although FR-202 requires exhaustion and cancellation to keep the typed `incomplete` cause | fsl-replay/src/probability_certificate/document/refutation.rs; fsl-replay/src/probability_certificate/witness/subsystem.rs |

## Coverage

- Matrix source: grep index of trace tags (declared fallback; `quire matrix` not run).
- In-scope criteria tagged: 14 of 14.
- Semantic review: run inline at the caller's request. Test-intent findings are recorded in the code-review SR (FND-005 to FND-008).
- Reverse gaps: none. Stubs: none.
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Spec correction of FR-202-AC-33's degenerate example is tracked as QSL-2941. |
| FND-002 | deferred | Flagged as optional vocabulary alignment, not a merge demand; this PR's original base predates #3551, so alignment with cyclic-*-state-predicate/v1 is a follow-up decision for the code lead. |
| FND-003 | fixed | `the_subsystem_entry_keeps_typed_limit_and_cancellation_causes` drives the entry to `Limit{ExactProbMaxStates, 1}`. The cancellation half is weaker than it looks; see code-review FND-009. |
