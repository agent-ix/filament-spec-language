---
id: SR-967175026302197768900474181941935084842
title: "QSL-3177 spec-review failure-domain"
type: SpecReview
analysis: failure-domain
review_set: subset
scope: "agent-ix/filament-spec-language; spec/functional/FR-196-build-the-probabilistic-product.md; QSL-3177; PR #3842 at 7c6e028bc923f0a700bf323b46b8b928a0bd333d"
---

## Summary

The root is an explicit identity component, remains attached through convergence and reset edges, and has a canonical source order. Invalid or missing key members are handled by the existing certificate-key admission contract; no separate failure-domain gap was found in the changed FR-196 text. The downstream FR-201 contradiction is recorded in the base review.

## Verdict

PASS for the scoped failure-domain method, subject to the open FR-201 consistency finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope examined

- FR-196 canonical product-key statement and FR-196-AC-18: root identity, canonical index, convergence and reset retention, and legacy form boundary.
- FR-101-AC-9: canonical initial-state ordering and equal-key coalescing.
- FR-201 key admission and FR-201-AC-11, context for downstream refusal behavior.
- Paired FSpec PR #837 FR-412-AC-35..41, context only for per-root reset and certificate rows.

No applicable AssuranceProfile was found under `spec/assurance/`.
