---
id: SR-2026111612172400000013
title: "Code review — QSL-1989 checker APIs and native fences"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/execute.rs, fsl-replay/src/outcome.rs, fsl-replay/src/spine.rs, fsl-replay/src/spine/lifecycle.rs, fsl-replay/src/spine/lifecycle/tests.rs, fsl-replay/src/spine/native_fences.rs, fsl-replay/tests/it/integer_division_check.rs, fsl-replay/tests/it/main.rs, fsl-source/src/lib.rs; FR-767-AC-11, FR-662-AC-1/2"
review_set: subset
---

## Summary

Reviewed PR #2798 at `4c6e8a19aff44e9a965d6e24b3cdbc833e758d6b` against the scoped checker APIs, per-fence selection, outcome serialization, and source correspondence behavior. The selection is forwarded through the check path and body spans are mapped to the original source, but indented Markdown clause headings are misclassified as headless fences.

## Verdict

**CONDITIONAL** — one medium behavior defect in clause-heading recognition.

## Coverage

Scope examined: `spine::check_with_integer_division`, `check_units_with_integer_division`, multi-unit law application, ordinary `spine::check` without a selection, `check_native_fences` with and without a selection, FR-662 report serialization, source identity and original-region mapping, changed tests, and all changed Rust files. The author-reported focused module (5/5), fmt, and scoped Clippy results were read from the PR description; they were not rerun. No Cargo command was run. No CI workflow changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Indented ATX clause headings are treated as absent, so valid Markdown fences are refused as `missing-name` | fsl-source/src/lib.rs:452 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The parser now recognizes up to three leading ASCII spaces before an ATX heading, while leaving four-space and tab-indented lines unrecognized; the regression test checks the supported and headless cases. |
