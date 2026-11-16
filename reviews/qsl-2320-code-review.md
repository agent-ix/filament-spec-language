---
id: SR-2026111612172400000005
title: "Code review of expected quantity threshold grammar"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-cst/src/grammar.rs, fsl-cst/src/parser.rs, fsl-cst/tests/it/probabilistic.rs, fsl-forms/tests/it/probabilistic_forms.rs, fsl-semantics/tests/it/probabilistic_declarations.rs; FR-195-AC-2, FR-195-AC-7, FR-764-AC-5, FR-186-AC-11, FR-186-AC-12, FR-186-AC-14"
review_set: subset
---

## Summary

Reviewed PR #2780 at frozen head `330aa3712dddc3b37e7a4495b659d50c289b6a0d`, including the grammar change, parser choice semantics, repository Rust conventions, relevant specifications, tests, and coder-provided focused test evidence. The expression-first choice consumes quantity expressions while backtracking permits an exact-value fallback, so the reviewed behavior preserves bare decimal thresholds. No defect was found.

## Verdict

PASS. The one-line ordering change addresses the numeric-prefix parse failure and preserves the exact-value fallback for bare decimal thresholds. Focused evidence reported for the PR covers the 11 ticket cases, and the existing trace-tagged tests cover the relevant requirements. No aggregate gate was run, per review scope.

## Findings

| ID      | Severity | Summary                   | Refs |
| ------- | -------- | ------------------------- | ---- |
| FND-001 | low      | No findings (placeholder) | -    |

## Coverage

- Changed production code: `fsl-cst/src/grammar.rs:2980-2987`.
- Parser behavior: `fsl-cst/src/parser.rs:1695-1705,1800-1825`; failed choice alternatives truncate pending nodes and retry at the original position.
- Existing relevant tests: `fsl-cst/tests/it/probabilistic.rs`, `fsl-forms/tests/it/probabilistic_forms.rs`, and `fsl-semantics/tests/it/probabilistic_declarations.rs`.
- Rust repository conventions and applicable implementation, testing, safety, API, boundary, and Tokio guidance were reviewed. No public API, safety, persistence, or concurrency surface changes.
- Coder-provided focused evidence: 8 `fsl-cst` probabilistic tests passed (1 ignored); the named complete-grammar test passed; 11 `fsl-forms` probabilistic tests passed; 72 `fsl-semantics` probabilistic declaration tests passed (4 ignored); both FSpec vector tests passed; formatting and diff checks passed. No aggregate gate run.
- AssuranceProfile: none applicable to the changed grammar behavior.
