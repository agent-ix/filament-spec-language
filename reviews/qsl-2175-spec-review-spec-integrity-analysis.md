---
id: SR-2175002
title: "integrity review of ValueTypeRef::Resolved for source object members (filament-spec-language PR 2755)"
type: SpecReview
analysis: integrity
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

# Integrity review of ValueTypeRef::Resolved (PR 2755)

## Summary

Ticket: QSL-2175. This pass checked consistency, completeness and atomicity of the diff at head
6411877ebca3c0d1e007e19f410e7a274b668acf, against the rest of FR-569 and FR-088, TC-260, TC-1940 and ADR-013 O-14. The ids are
well formed and `FR-569-AC-15` is new and contiguous. TC-260's rename is not referenced by its
old file name anywhere in `spec/`. TC-1940's scope line now covers FR-569-AC-15, and FR-569
gains a `depends_on` edge to FR-088. `quire validate` exits 0.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-569 contradicts itself on a field's multiplicity. The unchanged opening paragraph of "The source model" says the assembler builds "one `FieldMemberRecord` per field (exactly-one multiplicity, ...)". The new paragraph says the outermost collection of a field type is the record's `Multiplicity`, so `s: Sequence<Set<Integer>[0, 2]>[0, 2]` has multiplicity `0..2` (FR-569-AC-15). Fix: replace "exactly-one multiplicity" with "the multiplicity its type gives (below)", and add to the new paragraph that a type whose outermost constructor is not a collection (`Option<..>` included) has exactly-one multiplicity. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-fsl-source.md:207-208, :223-226 | wrong-requirement |
| FND-002 | low | The FR-088 rationale says source members' types are source declarations and anonymous types "which have no domain package node". But FR-569 "Member types" also admits model-owned value, record, tuple and variant types (`M::Total`), which do have a domain package node, and the new FR-569 rule carries them as `Resolved` too. Fix: name model-owned value types in the sentence and say they are carried as `Resolved` as well, so every non-reference, non-clock member has one representation. | spec/functional/FR-088-clause-name-and-type-identity.md:236-242 | wrong-requirement |
| FND-003 | low | TC-260 step 5 requires every construction of `ValueTypeRef::Resolved` to lie in the source-object assembler, yet the same step builds `Resolved(Decimal[0, 100; 2, 2; exact])` for its `Display` check. FR-088 (":246-247 Only the assembler's source model SHALL build it") and FR-088-AC-8 make the same claim. Fix: scope the claim to non-test code ("no production code outside the source-object assembler constructs it"). | spec/test-cases/TC-260-value-type-ref-is-exactly-five-member-union.md:45-50; spec/functional/FR-088-clause-name-and-type-identity.md:246-247, :276 | wrong-requirement |
| FND-004 | low | ADR-013 O-14 still states that model field types are the four-member `ValueTypeRef{Clock, Native, Package, Unit}`, which the amended FR-088 now contradicts. Fix: add an amendment note to the O-14 row naming FR-088 and QSL-2175, or one line in FR-088 saying that it supersedes O-14's list. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:435 | wrong-requirement |

## Verdict

REQUEST CHANGES on FND-001. The others are wording or bookkeeping. Every pairing that FR-088-AC-8
and TC-260 cover is consistent with FR-088-AC-7 (the checked node id stays the only package type
identity) and with FR-092-AC-21 (the record field's `ValueType` and binding).

## Dispositions

Review round 2, at head d42af98e46641e08f433f8b31fb81e32bc496f9e (fix commit d42af98e4).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-569:207-208 now reads "the multiplicity its type gives, as below". FR-569:223-231 adds that a non-collection outermost constructor, `Option` included, gives exactly-one multiplicity with the whole type as `value_type`, and AC-15 checks `o: Option<Sequence<Integer>[0, 3]>`. |
| FND-002 | fixed | FR-088:236-244 names the model-owned value, record, tuple and variant types (`M::Total`) and says they are carried as `Resolved` too. |
| FND-003 | fixed | FR-088:249-251, FR-088-AC-8 and TC-260 steps and expected results now say "no production code outside the source-object assembler", and FR-088 adds "tests may". |
| FND-004 | fixed | ADR-013:435 (O-14) carries an amendment note naming FR-088, QSL-2175 and the fifth variant. |
