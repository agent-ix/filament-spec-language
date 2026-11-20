---
id: SR-1518
title: "Scoped gap review of population admission budget cases"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2097; fsl-semantics/tests/it/model_population.rs; NFR-012-AC-8; NFR-012-AC-9; FR-084-AC-4 equal-content branch"
review_set: subset
---

## Summary

Reviewed requirement-to-test bindings and behavioral assertions for the two added population admission tests at 8d6128e42ca9729f796a365b26be3f85a170daa7. No confined coverage or reverse-gap findings.

## Verdict

PASS for the supplied diff and bounded acceptance scope. This is a manually scoped gap assessment, not an engine-backed matrix PASS. The dispatch excludes matrix, whole-spec and historical inventory work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Examined NFR-012-AC-8, spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:92: “At most the selected ceiling (default 100000) per admission”. Bound 3 rejects raw record 4, including a duplicate, before exposing a malformed foreign tail. Bound 4 admits the valid duplicate into exactly the three expected identities, or exposes the foreign member's typed refusal.
- Examined NFR-012-AC-9, spec/non-functional/NFR-012-bound-model-normalization-and-population-admission.md:93: “At most the selected ceiling (default 16777216 work units) per admission”. Three member admissions plus three width-three scans establish exact total 12. Bound 11 denies charge 3 after consumed 9 for valid and invalid final values; bound 12 admits or returns the typed subsetting refusal with object and field identities.
- Examined only the duplicate-collapse branch of FR-084-AC-4, spec/functional/FR-084-admit-closed-populations-and-resolve-lookup.md:200: “Given two runtime member records sharing universe and object identity but differing most-specific type, admission refuses conflicting-identity and admits no binding; given two records with equal key and equal content, admission collapses them into one member.” No claim for the conflicting-identity branch is added.
- Bindings: duplicate_records_use_member_capacity_before_a_foreign_tail_is_inspected → NFR-012-AC-8 and FR-084-AC-4 (duplicate branch), correct; subset_scans_charge_the_candidate_width_before_a_semantic_verdict → NFR-012-AC-9, correct.
- No production behavior is added without an owning requirement. Tests contain real failing assertions, exhaustive expected-outcome guards, real admission calls and independently derived budget boundaries. No stubs or coverage inflation in the added code.
- Matrix computation and whole-repository reverse-gap inventory were excluded by the dispatch. No optional repository-wide semantic analysis was run. Only the changed tests and their requirement intent were examined.
- Author reports two separate qualified exact tests each 1 passed, 0 failed, 1711 filtered, exit 0 and formatter exit 0; accepted without rerun. Initial zero-test invocation is invalid evidence. No fresh runtime, lint or gate claims.
- Plan completion: not assessed. Broader historical QSL-2097 work and known owners remain unchanged. Final affected component gate remains with coder/lead.
