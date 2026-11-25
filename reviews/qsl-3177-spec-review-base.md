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

## Disposition pass 1 scope

Reviewed fix head `0c75b949cce41befab16250628d7053cfdadd65d` against FND-001. The updated FR-201-AC-11 limits its multi-initial unsupported result to the `reachability/v1` profile and explicitly exempts conditional optimum certificates whose reset process has multiple roots. FR-201 now states its eight-member basis applies to its no-reset reachability profile, while FR-196, ADR-028, FR-1041, FR-1043 and TC-3060 consistently limit exact eight-member bytes to no-reset/single-root cases. The paired FSpec PR #837 still defines root-specific reset rows and matching legacy bytes. All six changed spec documents validate grammar-clean; `quoin advise --mismatch-only` reported zero candidates. No new defect was found in the fix diff.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-201-AC-11 now scopes multi-initial refusal to reachability/v1 and explicitly admits conditional optimum certificates over multi-root reset processes; FR-201's eight-member key statement is explicitly limited to its no-reset profile. |

After excerpt from related FR-201-AC-11 (FR-196-AC-18 remains unchanged):

> This profile-specific rejection does not apply to a `conditional` `optimum` certificate: when its reset process has multiple canonical initial roots, FR-196 keys retain the root index and the checker validates each root's rows as required by FSpec FR-412-AC-35..41.
