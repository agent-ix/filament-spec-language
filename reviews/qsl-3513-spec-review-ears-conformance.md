---
id: SR-967188
title: "QSL-3513 EARS conformance analysis of the cross-binding order, conformance objects and policy instances (filament-spec-language PR 4040)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language@adf503520a905d72041960c5832c201e87296279; spec/decisions/ADR-018, spec/functional/FR-126, FR-621, FR-896, FR-990, FR-1012, spec/test-cases/TC-1085, TC-2532, TC-2800"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3513. PR filament-spec-language#4040, head adf503520a905d72041960c5832c201e87296279.

New statements in FR-126, FR-621, FR-896, FR-990 and FR-1012 were checked. The
engine reports no EARS warning on a new statement. One statement has the wrong
actor.

Not run: gap-analysis (the diff has no production code).

## Verdict

One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The subject of `SHALL monitor` is a clause's `over` parameter, which performs nothing. Failure: a reader cannot tell whether the obligation is on the plan search (one monitor per admitted key) or on how a request declares the clause. Name the plan search as the actor and say it keeps one monitor per admitted key. | spec/functional/FR-1012-search-plans-with-policies-costs-and-canonical-order.md:37 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at head 8bc5cfda3663de78b37a32d5be17391ce9345c54. |
