---
id: SR-2026111612172400000019
title: "Gap analysis — filament-spec-language QSL-2349 change slice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2349 change slice in fsl-package/src/emit.rs and fsl-command/tests/it/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-495", type: references }
---

## Summary

Reviewed the QSL-2349 emitter behavior and its integration tests against FR-495 and FR-062. The changed behaviors have test bindings and the focused tests pass; the repository-wide matrix contains substantial pre-existing untagged and ignored-test noise outside this change slice, so those baseline entries are recorded but not attributed to PR #2814.

## Verdict

**PASS** — within the QSL-2349 change slice, the checked equality identity behavior and guarded div/rem/mod cases are specified, tagged, and exercised; no reverse-spec gap or hollow test was found.

## Findings

| ID      | Severity | Summary                  | Refs |
| ------- | -------- | ------------------------ | ---- |
| FND-001 | low     | No findings (placeholder) | - |

## Coverage

- Reconciliation: `quoin matrix --repo /Users/peter/dev/qsl-2349-code --json` (quoin 0.29.4, quire 0.38.3); no run evidence read.
- Plan completion: not assessed
- Matrix totals across the repository: 5,586 criteria; tagged 3,946; untagged 1,151; tagged-by-ignored-test 373; method-without-symbol 103; retired 13. These are broad baseline results, not findings for this PR.
- Relevant criteria: FR-495-AC-3, FR-495-AC-17 and FR-495-AC-18 are tagged, including both added integration tests. No changed criterion is untagged or bound only by an ignored test.
- Changed behavior/code inventory: equality producer type identity in `checked_equality_term_data`; shop short-circuit behavior; guarded div/rem/mod under and/or/implies. Each has an owning FR-495 requirement. No changed source stub or test stub was found.
- Semantic review: ran for the changed requirement/test/code triples; both new tests reach `command::run` and assert exact completed Boolean outcomes for zero and nonzero divisors.
- Aggregate test gate: not run. Targeted integration test command passed 2 tests.
