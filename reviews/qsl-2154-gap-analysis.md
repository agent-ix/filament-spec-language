---
id: SR-2026111612172400000008
title: "Gap analysis — QSL-2154 decimal expression support"
type: SpecReview
analysis: gap-analysis
scope: "filament-spec-language; FR-764-AC-1 through FR-764-AC-7 and corrected QSL-2154 acceptance comment 91a7f4ce-27bf-46e3-b43f-8ae7c1b041c4; parser/form/semantic implementations and tests"
review_set: subset
---

## Summary

Quoin's `gap-analysis` command is unavailable in the installed Quoin 0.35.0 (`quoin gap-analysis --help` exits 2 with `command gap-analysis not found`). I manually compared FR-764-AC-1 through AC-7 with the changed source and tagged tests. Parser admission, exponent refusal, exact coefficient/scale values, contextual type refusals, exact-threshold preservation, and the threshold scale limit have direct coverage; three AC assertions are incomplete as written below.

## Verdict

**CONDITIONAL** — three medium test-evidence gaps remain for explicit span and semantic-equivalence assertions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-2 test does not assert the decimal node span | fsl-forms/tests/it/value_forms.rs:1795-1804 |
| FND-002 | medium | AC-3 test does not compare successful checked results with the constructor spelling | fsl-semantics/tests/it/probabilistic_declarations.rs:3447-3453 |
| FND-003 | medium | AC-7 test does not compare successful checked results with the constructor spelling | fsl-semantics/tests/it/probabilistic_declarations.rs:3811-3824 |
