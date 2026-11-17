---
id: SR-2026111612172400000014
title: "Gap analysis — QSL-1989 scoped checker criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-767-AC-11, TC-2197, FR-662-AC-1/2; fsl-replay/tests/it/integer_division_check.rs, fsl-replay/src/spine/native_fences.rs, fsl-source/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: references
---

## Summary

Scoped the requirement-to-test-to-code chain for FR-767-AC-11 / TC-2197 and the FR-662-AC-1/2 report and correspondence behavior. The caller law reaches the checking function, and tests cover ordinary no-selection refusals, multi-unit selection, per-fence outcomes, and body-to-document coordinates; the native extractor misses clause headings with valid Markdown indentation.

## Verdict

**CONDITIONAL** — the scoped behavior has a concrete Markdown heading edge-case gap.

## Coverage

- FR-767-AC-11 and TC-2197: examined against `check_operation_takes_the_supplied_law`, `multi_unit_checker_takes_one_caller_selection`, and `every_fence_takes_the_supplied_law` plus the corresponding check implementations.
- FR-662-AC-1/2: examined against `refused_fence_keeps_body_span_and_original_coordinates` and `duplicate_and_headless_fences_refuse_independently_and_cancel_returns_no_report`, report serialization, and the extractor.
- AC-7, AC-9, AC-10 and QSL-2279: explicitly outside this PR's scoped criteria, not assessed as gaps.
- Plan completion: not assessed.
- Cargo tests and computed repository-wide Test Matrix were not run; the review was restricted to the requested criteria and the worktree was frozen. The author-reported focused module result was read from the PR description.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Indented ATX clause headings are treated as absent, so valid Markdown fences are refused as `missing-name` | fsl-source/src/lib.rs:452 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The parser now recognizes up to three leading ASCII spaces before an ATX heading, while leaving four-space and tab-indented lines unrecognized; the regression test checks the supported and headless cases. |
