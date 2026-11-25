---
id: SR-967175026302197768900474181941935084839
title: "QSL-3177 spec-review base"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/filament-spec-language; spec/functional/FR-196-build-the-probabilistic-product.md; QSL-3177; PR #3842 at 7c6e028bc923f0a700bf323b46b8b928a0bd333d"
---

## Summary

Ticket: QSL-3177. FR-101 defines canonical initial-state order, and the new FR-196 rule preserves the eight-member encoding outside multi-root reset processes. The change conflicts with FSL FR-201's unchanged eight-member key rule and explicit multi-initial unsupported outcome, which leaves the newly required reset certificate path impossible to admit.

## Verdict

FAIL. Reconcile FR-201's key and admission requirements for the multi-root reset case before this change is accepted.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-196-AC-18 requires nine-member keys for multi-root reset products, while FR-201 still mandates eight-member keys and explicitly returns unsupported for multi-initial cases; the checker cannot satisfy both requirements. | FR-196-AC-18, FR-201 | wrong-requirement |

## Scope examined

- FR-196 canonical product-key statement and FR-196-AC-18: the new ninth component is conditional on a reset process with multiple canonical initial roots; root is the zero-based FR-101 index; single-root/no-reset retain exact old bytes.
- FR-101-AC-9: initial states are admitted in ascending state-key byte order, and equal keys coalesce.
- FR-201 key requirement: “Keys SHALL use FR-196's authoritative complete eight-member basis”; the same section says FSpec FR-412, FR-331 and the provider schema state those eight members.
- FR-201-AC-11: “Unbounded cyclic, timed, reward, fair, multi-initial and multi-binding cases ... return explicit unsupported.”
- Paired FSpec PR #837 FR-412-AC-35..41 and `ProbabilityProductKey` schema description, context only: they require root-indexed reset certificate rows and preserve eight-member bytes otherwise.

No applicable AssuranceProfile was found under `spec/assurance/`. The changed FR-196 document validates grammar-clean; no new IDs, links, hash/digest/pin records or schema object declarations were added.
