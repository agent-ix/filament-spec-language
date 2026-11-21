---
id: SR-60055
title: "gap-analysis of filament-spec-language PR 3267, FR-710 contextual model-unit words"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3267 head b51b397210; FR-710 against quire matrix and fsl-cst/tests/it/model_unit.rs"
review_set: subset
---

## Summary

Ticket: QSL-2600. A scoped audit of FR-710 against the computed matrix (`quire matrix --scope . --format json`) and the PR's one new test. Plan completion: not assessed. The full-repository reverse-gap and stub sweep was not run, because the scope is the PR diff. The semantic review was not separately opted into, and the test oracle was read directly instead.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Reverse gap: the behaviour this PR adds (23 model-unit words are identifiers outside their declaration) has no owning requirement text or AC. FR-710's Grammar paragraph states the opposite. See SR-60054 FND-001. | spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md:71 |
| FND-002 | low | Trace: `model_unit_words_parse_as_identifiers_outside_their_declarations` binds FR-710-AC-3 in the matrix, but it does not test AC-3's compile-every-form claim. See SR-60054 FND-003. | fsl-cst/tests/it/model_unit.rs:133 |

## Scope and evidence

The matrix shows FR-710-AC-1 to AC-9 as all `tagged`. AC-3's binders now include the new test at fsl-cst/tests/it/model_unit.rs:135.

- FR-710-AC-3: examined. The existing `a_model_unit_with_every_declaration_form_parses` still covers the `occurs` and `states` terminators that `backtracking_star` needs. Clean, apart from the mis-trace in FND-002.
- FR-710 Grammar paragraph: examined. It contradicts the code (FND-001).
- New test oracle: examined. It asserts admissibility and a byte-identical render for all 23 words as record fields, function parameters and operands of a sum. It would fail before this PR. It does not cover model-unit-internal positions (SR-60054 FND-002).

## Verdict

CONDITIONAL. The tests that exist are real, but the behaviour has no owning AC and the new test is traced to the wrong AC.
