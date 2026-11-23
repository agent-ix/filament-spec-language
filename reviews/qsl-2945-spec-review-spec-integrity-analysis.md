---
id: SR-20261123010003
title: "Spec integrity review of filament-spec-language PR #3575: nested reference lifting"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@af2554ebd81b6632cec71c046914f1ef2c2a102e; spec/functional/FR-135, FR-140, FR-630 (context), spec/test-cases/TC-540, TC-1086, TC-1590 (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-140
    type: reviews
---
# Spec integrity review of filament-spec-language PR #3575

## Summary

Ticket: QSL-2945. Under the arch ruling, composite nested references stay
specified as lifted in FR-135 and FR-630, and the spec must not contradict
itself about them. The rewrites of FR-135-AC-6, TC-540 step 6, FR-140-AC-16 and
TC-1086 step 10 agree with each other and with the code for sequences and
options. The defects are elsewhere: in FR-135 body text that this PR left in
place, and in how FR-140 now words the carriers.

Examined: FR-135 statement lines 171-180 and 245-254, FR-135-AC-6, FR-140 lines
130-138, FR-140-AC-16, FR-630 lines 56-61, FR-630-AC-4, TC-540 step 6, TC-1086
step 10, and TC-1590 step 4.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-135 line 174 still reads "The checker SHALL lift a reference only at the top level of a row's type". That contradicts the next sentence (lines 175-180, nested references lifted element by element), line 253, FR-135-AC-6 as rewritten here, and FR-630. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:174 |
| FND-002 | medium | The spec contradicts itself about composites. FR-140 (rewritten here) lifts references nested in an "option or collection" and no longer mentions record or union. FR-135 (lines 177, 253) and FR-630 (line 57) say option, record, union or collection rows are lifted. So FR-140 has no replay rule for a composite row that FR-135 says S3 admits, and nothing says what a composite row does today (it refuses type-mismatch). Under the ruling, FR-140 should state record and union lifting too, or all three should mark the composite carrier as pending, citing the follow-up ticket. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:135 |
| FND-003 | medium | No spec text names map as a carrier: FR-135 says option, record, union or collection, FR-140 says option or collection, and FR-135-AC-6 says option or collection. The code lifts map values at S3 (`lift`) and in replay (`lift_nested`), and the PR title claims map. Map keys of `Reference` type (a leaf, so an admitted key) are lifted by neither layer, and no spec text says whether they should be. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:135 |
| FND-004 | low | The new FR-140 sentence "A reference nested inside an option or collection value is lifted ..." has no SHALL, so it reads as description, not as a requirement, unlike the bullet it sits in. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:135 |

## Verdict

FR-140-AC-16 and TC-1086 step 10 are atomic and testable. FR-135-AC-6 and TC-540
step 6 match the test. FND-001 must be fixed before merge: the requirement
contradicts itself. FND-002 is the composite contradiction the arch ruling asks
to avoid.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | This PR now lifts references nested in step-row and node-row arguments (`lift_value`), but no requirement says so. FR-136 and FR-141 state lifting only for top-level receiver and argument conformance, and FR-140 speaks of field rows. The behaviour has no requirement and no AC. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:163 |
| FND-006 | low | FR-140-AC-16 says "Over RingIsQueue ... the same field typed ... Map<Reference<_>, Reference<_>>", and FR-135-AC-6 says "an option, collection or map at any depth". No validated package can declare a map type, so the map clause cannot be realized over that fixture; TC-1086 step 10 says it is checked by unit tests of `lift_nested`. The ACs should say the map clause is verified at that level. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:226 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Line 174 is replaced: "The checker SHALL lift a Reference<T_C> nested inside an option, collection or map type, at every depth ..." |
| FND-002 | fixed | FR-135 (two places), FR-140 and FR-630 now agree: option, collection and map are lifted; record, tuple and union are pending QSL-2959 and refuse ill_typed/type-mismatch until then |
| FND-003 | fixed | FR-135 and FR-140 name map and state that reference-typed keys are lifted and entries keep their order |
| FND-004 | fixed | FR-140 now reads "`map_state` SHALL lift a reference nested inside an option, collection or map value ..." |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The new FR-135 SHALL and FR-135-AC-13 ("Two object-type rows whose `from` names the same concrete object type SHALL refuse `invalid_model_binding`/`conflicting-binding`") contradict ADR-020 RM-2 and RM-3. Those rules let one concrete population be the source of several abstract populations and choose the most-specific abstract type per abstract population, and the existing clock-refinement fixture relies on it (`Q::Queue` and `Q::Cell` from `C::Call`). | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:141-148 |
| FND-008 | low | FR-141-AC-13 states the `enq` postcondition `self.chain = rs`, but the fixture uses `size(rs) = size(self.chain)` (ring_world.rs:327), and the `Variant::chain` doc comment repeats the stale text. The test's doc comment cites "TC-546 step 13"; the step is 8. The lifted `rs` itself is asserted exactly, so the AC is still verified, but its wording describes a fixture that does not exist. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:283 |

## Dispositions (round 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | FR-141 has a new SHALL bullet for lifting nested references in the receiver and arguments, plus FR-141-AC-13 and TC-546 step 8 |
| FND-006 | fixed | FR-135-AC-6 and FR-140-AC-16 now say the map clause is checked by unit tests of `lift()` / `lift_nested`, because no validated package declares a map type |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | FR-141 now states that the receiver lifts "from the first such row". That contradicts ADR-020 RM-5, under which the receiver is typed against the abstract operation's receiver (the type that declares the operation), and RM-3, which gives the abstract type per abstract population and not by source order. Making declaration order decide the receiver's abstract type is a rule RM-3 does not contain. The receiver's type is determined and should be stated as such. For node-row arguments the record field types are also known; "first row" there is a shortcut, not a rule from ADR-020. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:164-167 |
| FND-010 | low | FR-140's new fallback, "When the declared type names none of those rows' abstract types, it SHALL lift by the first of those rows in declaration order", picks an arbitrary abstract object when the declared type is a supertype of one candidate. RM-3 gives no such order rule. Either specify conformance-based choice, or refuse the ambiguous case at S3. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:145-150 |

## Dispositions (round 3)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-007 | fixed | The FR-135 one-row SHALL and FR-135-AC-13 are removed (TC-540 scope too); FR-140 now states RM-2 multi-row sourcing with declared-type choice |
| FND-008 | fixed | FR-141-AC-13 now names the `size(rs) = size(self.chain)` postcondition; the ring_world doc comment and TC step number are corrected |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | FR-140's "the most specific such (it conforms to every other conforming one)" choice between two candidate rows whose abstract types are related by subtyping is this PR's rule. ADR-020 RM-3's "most-specific type" is the object's type within one abstract population, not a choice between populations. The rule is reasonable, but it is new semantics, which ADR-020 should record (or S3 should refuse the case). | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:148-156 |

## Dispositions (round 4)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-009 | fixed | FR-141 now takes the receiver's type from the type that declares the abstract operation (RM-5), by FR-140's conformance rule; FR-141-AC-14 and TC-546 step 9 are added |
| FND-010 | fixed | FR-140 replaces the first-row fallback with conformance and most-specific choice, and `MappingFailure::Fault` when none or no most-specific row conforms, stating that S3 admits no such row |
