---
id: SR-202611210022442313144108509
title: "QSL-2734 PR3286 EARS analysis"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR3286 1d666e7daa977b75f02e1bdefbd89b361d0449fd; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
---

## Summary

Ticket: QSL-2734. EARS review of the new and edited requirement statements and the new criterion. Criterion-strength analysis was not run: it needs a Jev client, which is not available in this session.

## Verdict

CONDITIONAL. The new statements are well-formed state-driven ("While the package reader cannot reconstruct ... SHALL refuse") and ubiquitous SHALL clauses, and each names a single responsible subject. One criterion clause is ambiguous.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-533-AC-23's quotient clause ("whose checked result combines distinct nominal class roots") names no fixture and states a different condition from the prose (nominal compound unit differs from the checked class-root result), so two implementers could pick different trigger sets. | spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md:498 |

## Scope

- FR-533-AC-23 (examined).
- FR-533 boundary statements (examined): state-driven refusal, location and no-value clauses, nominal/affine preservation.
- FR-093 runtime-graph paragraph (examined): source checker/evaluator SHALL use FSV FR-134 graph; bytes-only route SHALL refuse; SHALL return no value.

## Dispositions

Round 1, fix head 4219c63abebbce1dd776d930e126e778c4bd5d96 (original head 1d666e7daa977b75f02e1bdefbd89b361d0449fd). The reason column records the evidence; after-excerpts are in the round-1 dispositions comment.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-23 now names fixture Q and the first concrete product/quotient spans. It states the prose's trigger (nominal result unit differs from the checked class-root result unit) and selects the noncanonical member by its projected canonical root. |

Round 2, final head 7eb36c927d92a4b7637c05c608d09db5e819f2b8 (delta from 4219c63abebbce1dd776d930e126e778c4bd5d96). Each round-1 fix was re-verified at this head; none of the fixed files or spec rows changed in the delta. 

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. AC-23 now names fixture Q and the first concrete product/quotient spans. It states the prose's trigger (nominal result unit differs from the checked class-root result unit) and selects the noncanonical member by its projected canonical root. |
