---
id: SR-2175004
title: "criterion-strength review of ValueTypeRef::Resolved for source object members (filament-spec-language PR 2755)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/filament-spec-language; spec/functional/FR-088-clause-name-and-type-identity.md; spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md; spec/test-cases/TC-260-value-type-ref-is-exactly-five-member-union.md; spec/test-cases/TC-1940-source-object-member-types-check-as-record-fields.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-260
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-1940
    type: references
---

# Criterion-strength review of ValueTypeRef::Resolved (PR 2755)

## Summary

Ticket: QSL-2175. This pass asked of each changed criterion, FR-088-AC-8 and FR-569-AC-15
(with TC-260 and TC-1940) at head 6411877ebca3c0d1e007e19f410e7a274b668acf, whether a wrong implementation could still pass it.

This was a manual reviewer judgment. The quoin skill's Jev client is not installed (`jev` is
not on PATH, and the skill states it is blocked on that client), so no calibrated
`weakness_kind` or `adverse_case_coverage` scores were produced.

FR-088-AC-8 can fail. A sixth variant, a construction outside the assembler, a serializer arm or
`Debug` text each fail one of TC-260's steps. FR-569-AC-15's representation checks
(`Resolved` of the decimal, multiplicity `0..2` with the `Set` element, `Package`, `Clock`)
and its integer variance pair (one accepted, one refused) can each fail too.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-569-AC-15 checks a nested `Reference` only in the accepting direction: narrowing `Option<Reference<M::Node>>` to `Option<Reference<M::Leaf>>` checks. An implementation that treats any pair of Reference-bearing `Resolved` types as conforming, or that skips the nested check, still passes. The AC also has no cross-variant case and no case for a non-integer or collection pair. Fix: add (a) widening that result from `Option<Reference<M::Leaf>>` to `Option<Reference<M::Node>>` refuses `ill_typed`/`variance-result`; (b) redefining a result `Reference<Order>` as `Option<Reference<Order>>` refuses `ill_typed`/`variance-result`; (c) one non-integer pair, for example a `Decimal` parameter redefined with different bounds, with whichever outcome FND-001 of SR-2175001 settles. Mirror each in TC-1940. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:598; spec/test-cases/TC-1940-source-object-member-types-check-as-record-fields.md:32-36 | correct-requirement-no-evidence |
| FND-002 | low | "No member holds a temporary type" (FR-569-AC-15, TC-1940 expected results) rests on an undefined term. A placeholder `Integer` left on an `Integer` member cannot be told apart from the real type, so for such a member the clause cannot fail. The concrete per-member checks already catch a placeholder on `f` and `s`. Fix: replace the clause with an observable one ("every member's `value_type` equals the checked type of the same form as a record field"), or drop it. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:598 | wrong-requirement |

## Verdict

REQUEST CHANGES on FND-001. Without its adverse cases, the conformance half of AC-15 cannot
distinguish a correct nested-reference check from no check at all.

## Dispositions

Review round 2, at head d42af98e46641e08f433f8b31fb81e32bc496f9e (fix commit d42af98e4).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-569-AC-15 and TC-1940 add three adverse cases. Widening `Option<Reference<M::Leaf>>` to `Option<Reference<M::Node>>` refuses `variance-result`. `Reference<Order>` redefined as `Option<Reference<Order>>` refuses `variance-result`. A `Decimal[0, 100; 2, 2; exact]` parameter redefined as `Decimal[0, 200; 2, 2; exact]` refuses `variance-parameter`, which matches the closed list's equal-only rule for decimals and the model route. Each case can fail. |
| FND-002 | fixed | FR-569-AC-15 and TC-1940 replace the "temporary type" clause with "Every member's `value_type` equals the checked type of the same form as a record field", which is observable. |
