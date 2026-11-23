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
