---
id: SR-967175026302197768900474181941935084868
title: "Gap analysis — QSL-3269 inline none history initial (PR #3962, diff-scoped)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3962 head 7d880035e7f090aaaec3575e5058e7e34b3ccb69, diff only; FR-901-AC-1, FR-138-AC-9, FR-630-AC-3"
review_set: subset
---

## Summary

Ticket: QSL-3269. A planless gap analysis, limited to the PR diff on the caller's
instruction. Trace tags were checked with `quire trace --id`. A full `quoin matrix` was not
run because the scope is the diff only.

- FR-901-AC-1 has two verifies tests,
  `history_initials_of_each_literal_form_are_written` (changed) and
  `history_population_and_seq_rows_emit`, plus one `implements` on `Lowering::history_initial`
  (new, structural).
- FR-138-AC-9 and FR-630-AC-3 are carried by `option_history_initials_prove_or_refuse`
  (TC-1590). Its tags are unchanged, and its cases now cover present initials as well as
  absent ones.
- The removed test was tagged FR-901-AC-1, which keeps its other verifying tests, so no
  criterion lost its only test.
- Reverse gap: the inline `none` form is owned by FSpec FR-375 `HistoryInitial` / AC-36
  (external, pinned 691ac5a), and by FR-901-AC-1 through admission.
- Stubs: none found. Both changed tests assert concrete wire fields (term kind, value_kind,
  type, referenced semantic_form/semantic_type, payload value) and reader admission.

## Verdict

**PASS** — repository claim only, limited to the diff. The coverage gap on the nested branch
is recorded as FND-002 in the code-review SR, not here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder) | - |

## Coverage

- Criteria examined: FR-901-AC-1, FR-138-AC-9, FR-630-AC-3. All tagged and not untagged.
- Semantic review: skipped. The caller did not ask for it, and the code review covered
  intent against the pinned reader.
- Plan completion: not assessed

## Disposition review round 1

Re-checked at head 79bba218c3aa1129ba8f3d07d138aced84c00abd.

- FR-901-AC-1 still has its verifying tests and its `implements` tag.
- New `implements` tags FR-093-AC-33 and FR-093-AC-50 on `Lowering::literal_at` and
  `Lowering::literal_value` are confirmed and listed by `quire trace`.
- The new nested-option unit test is untagged. That is recorded as FND-003 in the code-review
  SR (SR-967175026302197768900474181941935084867) and is not repeated here.
- No new gap findings. The placeholder row stands.
