---
id: SR-967175026302197768900474181935084769
title: "Integrity review — QSL-2909 FSL result diagnostics"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; FR-281, FR-286, FR-290, FR-621 and TC-774"
review_set: subset
---

## Summary

Reviewed the cross-artifact result shape, diagnostic ownership and test boundaries. The typed reason is item-scoped and does not alter the existing root diagnostic or terminal-record shape.

## Verdict

**PASS** — the related requirements consistently preserve record replay-parity while adding one item-level diagnostic.

## Assurance Context

No applicable `AssuranceProfile` exists in this repository's specification scope.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-290 | examined | Item diagnostic is `{code, cause, reason}` and is distinct from record and document diagnostics. |
| FR-281-AC-24 | examined | Inconclusive/replay-parity result has neither counterexample nor replay. |
| FR-621 item `diagnostics` row | examined | Optional non-empty item-level array; absent without this diagnostic. |
| FR-621-AC-18 | examined | Item diagnostic count and unchanged document-level diagnostics are explicit. |
| FR-286 Analyze item members | examined | Optional item-level diagnostics are listed separately from root diagnostics and FSpec terminal-record diagnostics. |
| FR-286-AC-31 | examined | Schema completeness fixture includes an item-level diagnostic. |
| TC-774 | examined | The test covers all five individual reason cases and the mapped assignment replay case. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
