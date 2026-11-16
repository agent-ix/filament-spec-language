---
id: SR-2026111612172400000008
title: "Gap analysis — QSL-2154 decimal expression support"
type: SpecReview
analysis: gap-analysis
scope: "filament-spec-language; FR-764-AC-1 through FR-764-AC-7 and corrected QSL-2154 acceptance comment 91a7f4ce-27bf-46e3-b43f-8ae7c1b041c4; parser/form/semantic implementations and tests"
review_set: subset
---

## Summary

Quoin's `gap-analysis` command is unavailable in the installed Quoin 0.35.0 (`quoin gap-analysis --help` exits 2 with `command gap-analysis not found`). I manually compared FR-764-AC-1 through AC-7 with the changed source and tagged tests. Parser admission, exponent refusal, exact coefficient/scale values, contextual type refusals, exact-threshold preservation, and the threshold scale limit have direct coverage; three AC assertions were incomplete in the initial review.

## Verdict

**CONDITIONAL** — AC-2's S2 expression-span assertion remains missing; AC-3 and AC-7 semantic-equivalence assertions are now covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-2 test does not assert the decimal node span | fsl-forms/tests/it/value_forms.rs:1795-1804 |
| FND-002 | medium | AC-3 test does not compare successful checked results with the constructor spelling | fsl-semantics/tests/it/probabilistic_declarations.rs:3447-3453 |
| FND-003 | medium | AC-7 test does not compare successful checked results with the constructor spelling | fsl-semantics/tests/it/probabilistic_declarations.rs:3811-3824 |

## Dispositions

Rechecked at PR #2788 head `c9dd3bd403f6eb6b09aeff8189417ae4c450ac62`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The revised test asserts that the CST `Primary` span equals the literal, then asserts the S2 payload separately. It never inspects the S2 `ExpressionSpans`, so the literal-span assertion can still pass if the form builder assigns a different expression span. |
| FND-002 | fixed | The updated test compares `semantic_nodes` for checked bodies spelled `0.9` and `decimal(9, 1)`, asserting equality of the complete checked semantic graphs. |
| FND-003 | fixed | The updated test compares complete checked semantic graphs for bare and constructor spellings in both parameter and record-field contexts. |

### FND-002 after excerpt

```text
let bare = checked_body("0.9");
let constructor = checked_body("decimal(9, 1)");
assert_eq!(semantic_nodes(&bare), semantic_nodes(&constructor));
```

### FND-003 after excerpt

```text
assert_eq!(semantic_nodes(&bare_parameter), semantic_nodes(&constructor_parameter));
assert_eq!(semantic_nodes(&bare_field), semantic_nodes(&constructor_field));
```

Round 2, rechecked at PR #2788 head `b02b7bc56fc1474867e482351d31ed1483829fa6`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The S2 forms test now checks the root of the body span tree directly against the literal span, in addition to checking the CST primary and DecimalValue payload. |

### FND-001 after excerpt

```text
let spans = function.spans().expect("the form carries expression spans");
assert_eq!(
    spans.body.at(&[]),
    Some(literal_span),
    "the S2 expression span equals the `{source}` literal span"
);
```
