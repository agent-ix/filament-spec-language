---
id: SR-2026111612172400000007
title: "Code review — QSL-2154 decimal expression support"
type: SpecReview
analysis: code-review
scope: "filament-spec-language; fsl-cst/src/grammar.rs, fsl-cst/src/parser.rs, fsl-cst/tests/it/expression_syntax.rs, fsl-forms/src/value.rs, fsl-forms/tests/it/value_forms.rs, fsl-semantics/tests/it/probabilistic_declarations.rs; FR-764"
review_set: subset
---

## Summary

Reviewed PR #2788 at `8eb4f2d5085a2e59aa50735512c5a43335b9b1e4`, including its parser, form mapping, and semantic tests against FR-764 and the corrected QSL-2154 acceptance comment. Decimal coefficient and scale are preserved, but S2 drops the required `decimal-literal` spelling kind when it builds the expression form.

## Verdict

**CONDITIONAL** — one medium contract mismatch remains: an expression form cannot distinguish a bare decimal literal from a `decimal(c, s)` constructor as FR-764-AC-2 requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | S2 loses the required decimal-literal spelling kind | fsl-forms/src/value.rs:1865-1874; fsl-forms/src/syntax.rs:608-613 |
