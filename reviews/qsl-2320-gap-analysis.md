---
id: SR-2026111612172400000006
title: "Gap analysis of expected quantity threshold grammar"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs, fsl-cst/src/parser.rs, fsl-cst/tests/it/probabilistic.rs, fsl-forms/tests/it/probabilistic_forms.rs, fsl-semantics/tests/it/probabilistic_declarations.rs; FR-195-AC-2, FR-195-AC-7, FR-764-AC-5, FR-186-AC-11, FR-186-AC-12, FR-186-AC-14"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec/spec.md
    type: references
---

## Summary

The changed grammar order was checked against the current acceptance criteria, their tagged test evidence, parser behavior, and the 11 ticket cases. The scoped cases are covered and the PR changes no unrelated behavior. The installed `quoin` has no `gap-analysis` command, so this is a manual scoped criteria-to-test check, supplemented by the computed Test Matrix.

## Verdict

PASS for the scoped change. No acceptance-to-test or code-to-spec gap was found in the reviewed behavior. This is a PR-scoped result, not a repository-wide gap-audit claim.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No gaps found             | -    |

## Coverage

- `FR-195-AC-2`: expected accumulated-reward thresholds admit the supported exact and quantity forms and reject wrong dimensions/types. Test binding: `TC-630`, `fsl-semantics/tests/it/probabilistic_declarations.rs` (`an_expected_reward_claim_is_typed_by_its_rewards_unit`). Matrix status: tagged.
- `FR-195-AC-7`: expected elapsed accepts time quantities and rejects mismatches. Test binding: `TC-630`, `fsl-semantics/tests/it/probabilistic_declarations.rs`; test cases include elapsed quantity parsing and type checking. Matrix status: tagged.
- `FR-764-AC-5`: bare decimal expected-reward thresholds remain exact and equivalent to `decimal(5, 1)` and `rational(1, 2)`. Test bindings: `TC-2163`, `probabilistic_bare_decimal_threshold_parses` in `fsl-forms/tests/it/probabilistic_forms.rs`, and `bare_decimal_threshold_is_exact` / `decimal_forms_equivalent` in `fsl-semantics/tests/it/probabilistic_declarations.rs`. Matrix status: tagged.
- `FR-186-AC-11`, `FR-186-AC-12`, and `FR-186-AC-14`: elapsed threshold and quantity-expression parsing/refusal behavior is covered by `TC-2168`, `TC-2169`, and `TC-2171` cases in `fsl-semantics/tests/it/probabilistic_declarations.rs`. Matrix status: tagged.
- Ticket acceptance evidence: the PR reports all 11 cases pass after the fix: two CST cases, one forms case, six semantics cases, and two FSpec-vector cases. The test grouping also reports 8 CST probabilistic tests, one named complete-grammar test, 11 forms tests, and 72 semantics declaration tests passing (ignored counts are recorded in the code-review artifact).
- Matrix verification: `quire matrix --scope <repo> --format tsv` and `quoin matrix --repo <repo> --json` were available and completed; relevant criteria above are tagged. The repository-wide matrix includes unrelated legacy/unassigned rows and is not treated as this PR's scope.
- Reverse-gap inspection: the change only changes alternative order for an already specified expected threshold. Its behavior is owned by the criteria above; no new behavior without a requirement was introduced.
- Plan completion: not assessed.
