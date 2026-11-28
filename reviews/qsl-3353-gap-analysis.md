---
id: SR-202611251525112
title: "gap-analysis of PR 3936 state-graph certificate shape checks and no-initial-state order"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3936 head 8c4787bfd8f41e634840d81ac3f5473283bdaf1a; criteria touched by the diff: FR-169-AC-14, FR-169-AC-15, FR-125-AC-13"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-169
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-125
    type: references
---

## Summary

Ticket: QSL-3353. This audit is planless and scoped to the criteria that the diff touches. The matrix was computed at the head with `quire matrix --scope <worktree> --format json`. Plan completion: not assessed.

## Verdict

PASS. All three criteria are `tagged`:

- **FR-169-AC-14:** bound to `a_closure_entry_that_is_no_state_key_is_malformed` and the it/settle tests.
- **FR-169-AC-15:** bound to `certificates_reject_the_other_forms_list_and_accept_an_empty_one`, `a_certificate_with_the_other_forms_list_settles_inconclusive`, `wrong_form_list_variants_settle_rejected_and_empty_lists_settle_proved` and `the_other_forms_list_is_malformed_in_stored_order`.
- **FR-125-AC-13 (TC-520):** gains `the_checker_refuses_no_initial_state_before_no_instance` and `a_clause_over_no_initial_state_is_refused_before_no_instance`.

Each new test asserts the clause it is tagged with. Both order tests would fail on base, which returns `NoInstance` there, so they discriminate. The new production code (`is_model_state_key`, `Carried`, `no_initial_state`) is owned by FR-169 Check and FR-125 Instancing and has no stubs. The semantic review step was skipped because no opt-in was given. The tests were run at the head by name, and all passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- Criteria examined: FR-169-AC-14, FR-169-AC-15, FR-125-AC-13. All are tagged, none untagged.
- Reverse gap: none. Stubs: none.
- Semantic review: skipped.
- Plan completion: not assessed
