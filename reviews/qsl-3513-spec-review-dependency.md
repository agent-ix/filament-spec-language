---
id: SR-967189
title: "QSL-3513 dependency analysis of the cross-binding order, conformance objects and policy instances (filament-spec-language PR 4040)"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language@adf503520a905d72041960c5832c201e87296279; spec/decisions/ADR-018, spec/functional/FR-126, FR-621, FR-896, FR-990, FR-1012, spec/test-cases/TC-1085, TC-2532, TC-2800"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3513. PR filament-spec-language#4040, head adf503520a905d72041960c5832c201e87296279.

New `depends_on` edges: FR-126 to FR-621 and FSpec FR-361; FR-896 to FR-125,
FR-126, FR-621 and FR-1033; FR-990 and FR-1012 to FR-125. Each is a requirement
frontmatter relationship that the body relies on. The FR-896, FR-990 and FR-1012
edges close no cycle. FR-126 and FR-621 were already mutually reachable on
`depends_on` before this PR (one large existing strongly connected set), so the
FR-126 to FR-621 edge introduces no new mutual dependency.

Not run: gap-analysis (the diff has no production code).

## Verdict

No defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
