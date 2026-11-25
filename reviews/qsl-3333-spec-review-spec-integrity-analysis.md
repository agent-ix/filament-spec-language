---
id: SR-967175026302197768900474181941935084866
title: "Spec review (integrity) — filament-spec-language QSL-3333 FR-169 duplicate line and FR-163 edge spelling"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md, spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md (context)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-169", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-163", type: references }
---

## Summary

Ticket: QSL-3333. PR #3949 (head `9edac2a8a33f638a059f3a5a0aaa30a898557726`) changes one spec line. It deletes the duplicated "`Entry { index }` when the index names none;" from FR-169's Malformed paragraph, which leaves the bullet grammatical and keeps its meaning. FR-163 was read as context for the certificate-edge/rejection-tuple split that the code now relies on.

## Verdict

**CONDITIONAL.** The FR-169 edit is correct. FR-163 should say explicitly which of its two edge spellings a reader uses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-163's HP-1 closure paragraph says each certificate edge "SHALL carry the existing `pre_state`, `transition`, `post_state` and typed `result` of FSpec FR-331's `HyperTransitionTuple`" and then that "Each key SHALL serialize in the existing unpadded base64url StateKey spelling". FR-331's tuple now carries `pre_state`/`post_state` as State objects `{key}`, so two readers could spell certificate edges differently. Code and test now pin bare strings for certificate edges and State objects for the rejection tuple. FR-163 should state that a certificate edge's `pre_state`/`post_state` are bare StateKey strings, unlike the rejection locus. | spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md:94-101,224-227 |

## Dispositions

Round 1, head `c2b7d10ab8bb90231e42f578a2fcda7c62648849`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-163 now states that a certificate edge's `pre_state`/`post_state` are base64url StateKey strings and that a rejection tuple writes State objects. |
