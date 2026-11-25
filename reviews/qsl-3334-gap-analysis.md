---
id: SR-202611251425032
title: "gap-analysis of PR 3927 partial-product lower and red-main fixes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3927 head 345c3498d9c3a1a60bc9f6af7872ad4fb256f85e; criteria touched by the diff: FR-1042-AC-11, FR-1042-AC-18, FR-281-AC-4, FR-286-AC-2, FSpec FR-412-AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: references
---

## Summary

Ticket: QSL-3334 (with QSL-3331). This audit is planless and scoped to the criteria the diff touches. The matrix was computed with `quire matrix --scope . --format json` at the head. Plan completion: not assessed.

## Verdict

CONDITIONAL. FR-1042-AC-18 is now tagged by two tests, and both assert its clauses. FR-281-AC-4 and FR-286-AC-2 keep their binders. FSpec FR-412-AC-5 is asserted by the value_bounds and reward partial tests. FR-1042-AC-11 stays untagged: it was untagged before this PR, the PR description says it is not fully met, and it is deferred to QSL-3331's remaining work. The new code in partial.rs is owned by FR-1042 and FSpec FR-411 and contains no stubs. The semantic review step was skipped because no opt-in was given. Tests were not run, because memory was below the cargo admission floors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1042-AC-11 stays untagged. Partial enumeration covers the cyclic-chain `eventually` and the acyclic expected reward. AC-11's cyclic reward chain, its `shifted` measure base and its timed MeanTime case still stop at `max_states`. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md |

## Coverage

- FR-1042-AC-18: tagged by `a_partial_lower_over_the_cyclic_chain_proves_without_a_counterpart` and `a_partial_lower_checked_over_its_named_states_refuses_a_false_document` (correct).
- FSpec FR-412-AC-5: tagged by `a_partial_lower_checked_over_its_named_states_refuses_a_false_document` and `a_partial_reward_lower_checked_either_way_refuses_a_false_document` (correct).
- FR-281-AC-4: `a_rejected_certificate_is_inconclusive_and_an_accepted_one_certified` (correct).
- FR-286-AC-2: `check_refusal_document_carries_the_refusals_diagnostic` (correct).
- FR-1042-AC-11: untagged. The `FSpec-FR-1042-AC-11` tag at reward.rs:107 is dangling; code-review SR-202611251425031 FND-002 covers it.
- Plan completion: not assessed.
