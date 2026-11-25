---
id: SR-967175026302197768900474181941935084841
title: "QSL-3177 spec-review integrity"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/filament-spec-language; spec/functional/FR-196-build-the-probabilistic-product.md; QSL-3177; PR #3842 at 7c6e028bc923f0a700bf323b46b8b928a0bd333d"
---

## Summary

The new AC is observable and the root index has a defined ordering source. Integrity review found a contradiction with the checker contract: the new key cardinality and newly required admission case do not reconcile with FR-201's eight-member basis and multi-initial refusal.

## Verdict

FAIL. FR-201 must state how the checker admits the new multi-root reset keys and how the feature fits its multi-initial refusal boundary.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The nine-member key required by FR-196-AC-18 contradicts FR-201's eight-member key basis and FR-201-AC-11's unsupported result for multi-initial cases, so producer and checker requirements cannot both be met for this case. | FR-196-AC-18, FR-201 | wrong-requirement |

## Scope examined

- FR-196 canonical product-key statement and FR-196-AC-18.
- FR-101-AC-9 initial-root ordering.
- FR-201 complete-key basis, its checked-subject key admission rule and FR-201-AC-11 multi-initial outcome.
- Paired FSpec PR #837 FR-412-AC-35..41 and schema description, context only.

No applicable AssuranceProfile was found under `spec/assurance/`. The change is atomic at the representation-rule level: the ninth member is present precisely for multi-root reset processes and absent otherwise.
