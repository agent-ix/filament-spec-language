---
id: SR-967179
title: PR 3897 red-main probabilistic settlements gap-analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-3293; fsl-analyze/src/analyze/exact.rs; fsl-replay/src/probability_certificate/document/timed.rs; cited criteria FR-921-AC-2, FR-921-AC-3, FR-203, FR-201-AC-30, FR-201-AC-62, FR-201-AC-65, FR-201-AC-74, FR-200"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-200
  type: references
- target: ix://agent-ix/filament-spec-language/FR-201
  type: references
- target: ix://agent-ix/filament-spec-language/FR-921
  type: references
---

## Summary

Ticket: QSL-3293. PR: filament-spec-language#3897, head 8b24beca8885dfc2354fe0041ef3efe2539eb0f6. Planless and scoped to the diff. The matrix comes from `quire matrix --scope . --format json` at head. It emitted symbol-id-collision diagnostics elsewhere in the repository and exited 0.

## Verdict

CONDITIONAL (low only). Every criterion the diff cites is tagged. The new production code carries its spec citations (FR-203, FR-201-AC-65, FR-201-AC-62). No stubs or coverage inflation were found in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two fair long-run tests now assert FR-200's LongRunFraction refutation rule (a 2^-k randomized witness with k raised to 2), which no other test exercises past k=1 (FR-200-AC-3). They carry only FSpec-FR-412-AC-27 and FSpec-FR-411-AC-18 bindings and no FR-200 binding. | fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:1587; fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs:1667 |

## Coverage

- FR-921-AC-2: tagged (a_wrong_clock_cap_is_malformed and others). FR-921-AC-3: tagged (timed_rows_cover_exactly_the_rebuilt_product).
- FR-201-AC-30 and FR-201-AC-74: tagged (finite_profile_claims_issue_no_long_run_document). FR-201-AC-62: tagged (bounded_nested_multiple_and_unresolved_forms_issue_no_proof_in_these_profiles and others). FR-201-AC-65: tagged (a_violated_quantity_past_the_rational_budget_settles_incomplete).
- FR-200-AC-3: tagged (fair_availability_uses_long_run_limit_and_randomized_refutation).
- FR-203: the max_states settlement is bound by a_state_budget_of_one_settles_incomplete (tagged FR-281-AC-6). FR-203-AC-4 and AC-16 to AC-27 are untagged on main. They predate this PR and lie outside its diff, so they are not findings here.
- Reverse gap: none. Every new branch cites its owning requirement.
- Semantic review: done manually for the tests named stale (see SR-967178). The full opt-in pass was skipped.
- Tests: not run (MemAvailable 28.9 GB, below the 30 GB gate).
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Both fair long-run tests now carry an FR-200 binding. |
