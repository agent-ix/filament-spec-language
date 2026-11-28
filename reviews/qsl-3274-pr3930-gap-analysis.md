---
id: SR-202611251433002
title: "gap-analysis of PR 3930 no-run-member refusal ordering"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3930 head 1a2e899a9d5982f804bab095c670c1184153185a; criteria FR-1047-AC-10 to FR-1047-AC-16"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1047
    type: references
---

# Gap analysis of PR 3930 (QSL-3274 item 1)

## Summary

Ticket: QSL-3274 item 1. This audit is planless and scoped to FR-1047-AC-10 through AC-16, the criteria this diff implements. The matrix was computed with `quire matrix --scope <worktree> --format json` at the head. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-1047's table row "an extension entry FR-660 refuses from the request alone" can be read as covering FR-660's unknown-capability refusal, which needs no file to decide. FR-660 orders that check at the catalog link, after the source reads, and the code follows FR-660. The wording should say "before any dependent file is read" | spec/functional/FR-1047-name-every-request-defect-and-return-a-request-example.md:242 |

## Verdict

CONDITIONAL, on one low spec-wording finding that does not touch this PR's code. All seven criteria (FR-1047-AC-10 to AC-16) are tagged in the computed matrix, each by a test in `fsl-command/tests/it/request_diagnostics.rs` that asserts its clauses at the entries the criterion names. The new code is owned by FR-1047 and FR-100-AC-25 and contains no stubs. The `FR-1047-AC-10` binder on `request_shape::no_run_member` is a source doc mention, not test evidence; the test binders carry the coverage. The semantic review step was skipped because no opt-in was given. Tests were not run, because memory was below the cargo admission floors and the alternate locks were held.

## Coverage

- FR-1047-AC-10: `a_selected_source_with_no_run_member_names_the_group_before_reading`
- FR-1047-AC-11: `size_and_decode_refusals_precede_the_no_run_member_refusal`
- FR-1047-AC-12: `program_sources_refusal_precedes_the_no_run_member_refusal`
- FR-1047-AC-13: `package_member_refusals_precede_the_no_run_member_refusal`
- FR-1047-AC-14: `program_request_checks_precede_the_no_run_member_refusal`
- FR-1047-AC-15: `extraction_clause_count_precedes_the_no_run_member_refusal`
- FR-1047-AC-16: `later_checks_give_way_to_the_no_run_member_refusal`
- Plan completion: not assessed
