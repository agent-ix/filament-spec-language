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

**CONDITIONAL (disposition pass 1)** — original FND-001 and FND-002 are fixed, but new medium FND-003 remains open at 7069d2d730845a06fb0b0d76e2133570db17413d.

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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Both new tests incorrectly tag NFR-017-AC-5: neither scans all documentation fences, compares counts, nor enforces stage annotations. The first rule has no stage annotation and these tests accept it, while that criterion requires refusal. These tags change the computed matrix from tagged-by-ignored-test to tagged although the actual doc-snippet runner remains ignored under QSL-2047. Remove the NFR-017-AC-5 tags and retain the relevant FR-110/FR-361 bindings. | fsl-semantics/tests/it/readme_first_rule_checks.rs:89; fsl-semantics/tests/it/readme_first_rule_checks.rs:109; NFR-017-AC-5 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The active positive test extracts the first README fence, compares its fixed literal, checks it against the named model, and asserts all three clause names and kinds. |
| FND-002 | fixed | The active negative test changes only the profile and asserts a nonempty set containing only UnsupportedConstruct/expression-form refusals. |
| FND-003 | still-open | New in disposition pass 1; the two incorrect NFR-017-AC-5 tags need removal. |

## Disposition pass 1 evidence

The unchanged original artifacts were verified byte-for-byte in reviews/. The fresh computed matrix is round1/matrix.json: NFR-017-AC-5 now reports tagged, naming both new tests alongside the two ignored doc_snippets tests. No run evidence is bound. This is the precise coverage inflation introduced by the incorrect tags; the broader QSL-2047 runner remains outside this ticket's implementation scope. The actual new test behavior satisfies QSL-1968's requested positive and negative checks. No code/test/gate execution was performed during review. Plan completion: not assessed.

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "fn the_readme_first_rule_compiles_against_its_model() {\n    let graph = check(first_rule()).expect(\"the README rule checks against its model\");\n    let clauses: Vec<_> = graph\n        .state_clauses()\n        .iter()\n        .map(|clause| (clause.name(), clause.kind()))\n        .collect();\n    assert_eq!(\n        clauses,\n        [\n            (\"ParentOrder\", StateClauseKind::Invariant),\n            (\"NoCycle\", StateClauseKind::Invariant),\n            (\"VersionUnchanged\", StateClauseKind::Postcondition),\n        ]\n    );\n}"
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "fn the_same_rule_under_value_complete_refuses_expression_form() {\n    let source = first_rule().replace(\"filament.model.complete/v1\", \"filament.value.complete/v1\");\n    let refusals = check(&source).expect_err(\"the value layer refuses graph expressions\");\n    assert!(!refusals.is_empty(), \"a refusal has at least one cause\");\n    for refusal in refusals {\n        assert_eq!(\n            refusal,\n            (Code::UnsupportedConstruct, Some(\"expression-form\"))\n        );\n    }\n}"
    },
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "New in disposition pass 1; remove the two incorrect NFR-017-AC-5 tags."
    }
  ],
  "scope": [
    {
      "id": "NFR-017-AC-5",
      "path": "spec/non-functional/NFR-017-build-a-clean-clone-with-locked-dependencies-and-run-the-documented-examples.md:121",
      "role": "examined",
      "excerpt": "`doc_snippets` finds every FSL fence of `README.md` and `docs/**/*.md`, its count equal to a second line scan's count of opening fence lines whose info string starts with `ix:native` or `fsl`, and every fence with no `reject` or `pending` passes its declared stage; a fence with no `stage=` fails the test, naming its file and line."
    }
  ],
  "findings": [
    {
      "fnd": "FND-003",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "trace",
      "artifact_id": "NFR-017-AC-5",
      "related_ids": [
        "FR-110-AC-7",
        "FR-361-AC-13"
      ],
      "test_id": "readme_first_rule_checks::the_readme_first_rule_compiles_against_its_model",
      "claimed_ac": "NFR-017-AC-5",
      "correct_ac": "FR-361-AC-13",
      "path": "fsl-semantics/tests/it/readme_first_rule_checks.rs",
      "lines": "89,109",
      "excerpt": "`doc_snippets` finds every FSL fence of `README.md` and `docs/**/*.md`, its count equal to a second line scan's count of opening fence lines whose info string starts with `ix:native` or `fsl`, and every fence with no `reject` or `pending` passes its declared stage; a fence with no `stage=` fails the test, naming its file and line.",
      "finding": "Both new tests incorrectly tag NFR-017-AC-5: neither scans all documentation fences, compares counts, nor enforces stage annotations. The first rule has no stage annotation and these tests accept it, while that criterion requires refusal. These tags change the computed matrix from tagged-by-ignored-test to tagged although the actual doc-snippet runner remains ignored under QSL-2047. Remove the NFR-017-AC-5 tags and retain the relevant FR-110/FR-361 bindings."
    }
  ],
  "bindings": [
    {
      "test_id": "readme_first_rule_checks::the_readme_first_rule_compiles_against_its_model",
      "ac_id": "FR-110-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "readme_first_rule_checks::the_readme_first_rule_compiles_against_its_model",
      "ac_id": "FR-361-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "readme_first_rule_checks::the_same_rule_under_value_complete_refuses_expression_form",
      "ac_id": "FR-361-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "readme_first_rule_checks::the_readme_first_rule_compiles_against_its_model",
      "ac_id": "NFR-017-AC-5",
      "trace": "wrong"
    },
    {
      "test_id": "readme_first_rule_checks::the_same_rule_under_value_complete_refuses_expression_form",
      "ac_id": "NFR-017-AC-5",
      "trace": "wrong"
    }
  ]
}
```

