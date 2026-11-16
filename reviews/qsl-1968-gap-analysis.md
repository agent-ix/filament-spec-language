---
id: SR-2026111612172400000004
title: "gap-analysis of QSL-1968 README profile correction"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-1968; README.md; examples/config-version/spine_unit.rs; docs/images/first-rule-*.svg; fsl-semantics/tests/it/; tests/it/doc_snippets.rs"
review_set: subset
---

## Summary

Reviewed QSL-1968's acceptance criteria against PR #2754, repository requirements, computed matrix, and existing test source. Both requested README-specific regressions are absent. Plan completion: not assessed.

## Verdict

**CONDITIONAL** — 2 medium finding(s) require disposition before QSL-1968 is complete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The requested positive README-rule regression is absent. Only README.md changes, no readme_first_rule_checks module exists, and the broader README snippet checks are ignored under QSL-2047. Add an active Rust check against the named model so the repaired example cannot silently regress. | fsl-semantics/tests/it/:missing requested module |
| FND-002 | medium | The requested negative regression is absent: no active test checks the same README rule under value.complete and asserts unsupported_construct/expression-form. Add that companion assertion to pin the reason the model profile is required. | fsl-semantics/tests/it/:missing requested module |

## Examined scope

```yaml
[
  {
    "id": "QSL-1968",
    "path": "https://linear.app/agent-ix/issue/QSL-1968/language-readme-first-rule-is-refused-valuecomplete-profile#comment-af2a079e",
    "role": "examined",
    "excerpt": "A Rust test in `fsl-semantics/tests/it/` named `readme_first_rule_checks.rs::the_readme_first_rule_compiles_against_its_model` extracts the README's first rule (a fixed copy of the text, no hash) and checks it with the model it names."
  },
  {
    "id": "QSL-1968",
    "path": "https://linear.app/agent-ix/issue/QSL-1968/language-readme-first-rule-is-refused-valuecomplete-profile#comment-af2a079e",
    "role": "examined",
    "excerpt": "`readme_first_rule_checks.rs::the_same_rule_under_value_complete_refuses_expression_form` (negative, pins the layer rule)."
  },
  {
    "id": "README.md",
    "path": "README.md:34",
    "role": "examined",
    "excerpt": "profile v = \"filament.model.complete/v1\";"
  },
  {
    "id": "QSL-1968",
    "path": "https://linear.app/agent-ix/issue/QSL-1968/language-readme-first-rule-is-refused-valuecomplete-profile#comment-af2a079e",
    "role": "examined",
    "excerpt": "The README's floating-point sentence is replaced by a statement of the float types and rounding modes supported."
  },
  {
    "id": "NFR-017-AC-5",
    "path": "tests/it/doc_snippets.rs:173",
    "role": "context_only",
    "excerpt": "#[ignore = \"QSL-2047: the README and docs fences do not declare their stages yet, and the check and compile stages have no runner\"]"
  }
]
```

## Coverage

Reconciliation: quoin matrix 0.29.4 with quire 0.38.3 (engine 0.52.1), saved in matrix.json; coverage.json retains the repository diagnostic output. Relevant context rows: FR-110-AC-8 and FR-397-AC-1 are tagged; NFR-017-AC-5 is tagged-by-ignored-test. All three report no run evidence. QSL-2047 already owns the broader ignored doc-snippet runner; no duplicate finding is filed for that pre-existing work. Repository-wide unrelated untracked tags/diagnostics are outside this PR review. The findings concern the two explicit QSL-1968 regressions, not a claim that the underlying layer implementation has no tests. Plan completion: not assessed. Optional semantic fan-out skipped. No tests or gate were run.

