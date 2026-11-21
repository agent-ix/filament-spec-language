---
id: SR-61007
title: "gap analysis of PR #3256 (QSL-2725): refuse extracted bodies declaring more than one clause"
type: SpecReview
analysis: gap-analysis
date: 2026-11-21
scope: "agent-ix/filament-spec-language; PR #3256 git diff origin/main...HEAD (fsl-replay/src/spine/clause.rs, fsl-replay/src/spine/clause/tests.rs, fsl-command/tests/it/native_workflow_family_cases.rs, spec/functional/FR-031-run-extracted-native-source.md)"
review_set: subset
relationships:
- target: ix://agent-ix/filament-spec-language/FR-031
  type: references
---
# SR-61007: gap analysis of PR #3256 (QSL-2725)

## Summary

Ticket: QSL-2725. This is a planless gap analysis scoped to the PR diff and FR-031-AC-9. Plan completion: not assessed. The review pass ran at head dc74b683. This file was rebuilt from the review comment posted on Linear QSL-2725.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-031-AC-9 | spec/functional/FR-031-run-extracted-native-source.md | examined |
| FR-031-AC-5 | spec/functional/FR-031-run-extracted-native-source.md | context_only |
| FR-031-AC-10 | spec/functional/FR-031-run-extracted-native-source.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-9's syntax-error clause has no assertion on an extracted body; only clause 1 (two clauses) is AC-9-tagged. missing_declaration is asserted only by the TC-109-only test extracted_bodies_refuse_missing_models_and_absent_declarations. No test asserts the source refusal code for an unparsable extracted body, which is the path the new pre-compile parse could break; the AC-8 test checks only stage compile and a locus. | fsl-command/tests/it/native_workflow_family_cases.rs:195-228, fsl-command/tests/it/native_workflow_family_cases.rs:121-146, fsl-replay/src/spine/clause.rs:316-326 |

## Verdict

**Changes requested at dc74b683.** The two-clause case of FR-031-AC-9 is covered; the syntax-error case is not.

## Disposition pass 1 scope and evidence

Replacement disposition reviewer at head `e3c70d886fdb86704ef453329999e0cca248415d`, diff and git objects only, no cargo run. Two new tests now cover the syntax-error clause of FR-031-AC-9, both tagged FR-031-AC-9 and TC-109. `run_clause_refuses_an_extracted_body_with_a_syntax_error_at_the_source` (fsl-replay/src/spine/clause/tests.rs:3722) asserts `ClauseDisposition::Compile` with code `InvalidSyntax`, stage compile and no truth. `an_extracted_body_with_a_syntax_error_refuses_at_the_source` (fsl-command/tests/it/native_workflow_family_cases.rs:232) asserts exit 20, stage compile, refusal code `invalid_syntax` and null truth through the command. `invalid_syntax` is the source-phase refusal that FR-109's "compile refuses with that code" rule reports. The pre-compile parse the original finding pointed at is removed; the count now runs on the front end's own parse, after a parse failure has already returned. The previously ignored two-clause command test is un-ignored, tagged FR-031-AC-9/TC-109, and asserts `fields` {"actual": "2"}.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Library and command tests, tagged FR-031-AC-9, assert the invalid_syntax source refusal at compile for an unparsable extracted body. |

## Current verdict after disposition pass 1

**PASS.** FND-001 is fixed; no new gap findings.
