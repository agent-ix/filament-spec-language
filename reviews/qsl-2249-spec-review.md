---
id: SR-2026111518064800000005
title: "spec-review review of PR2726 FR-093 FR-104 FR-360 TC-1009 TC-459 TC-1422 TC-1423"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-104-check-state-clauses.md, spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md, spec/test-cases/TC-1009-variant-typed-operation-signature.md, spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md, spec/test-cases/TC-1422-an-if-is-an-operand-of-the-collection-forms.md, spec/test-cases/TC-1423-a-let-binds-an-option-value-under-its-presence-guard.md; PR2726 b3a8fd5effb5d01a1082c0f86a1a069424d87876"
review_set: subset
---

## Summary

Ticket: QSL-2249. Spec-review of the new ACs FR-360-AC-12, FR-093-AC-64, FR-093-AC-65, FR-104-AC-17, the added FR-104 and FR-360 statement sentences, TC-1009 step 5, TC-459 step 8 and the new TC-1422 and TC-1423, checked against FR-056 (variant types, field rule), FR-103 (parameter typing), FR-360 description and items 3, 5, 6 and the Semantic IR schema (filament-core-data schema/semantic/v1/semantic-ir.schema.json, `$defs.operation.params` items are `$defs.field`, which requires `presence` and `nullable`). Sub-analyses: none beyond the base review; the change adds AC rows and two statement sentences, with no structural, relationship or domain-object edit. `quire validate` on all seven changed spec files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | New FR-104 sentence "a parameter is typed by its type and multiplicity alone (FR-103): its nullable and presence members add no Option" contradicts FR-103, which types parameters by FR-056's field rule "with presence and multiplicity applied the same way" (optional presence wraps in Option). It codifies a code gap (OperationParameterRecord drops presence; assemble.rs types params by multiplicity only) as a requirement, and FR-104-AC-17 pins the gap (`present(p)` refuses, `exists(x in p ...)` checks, both the opposite of FR-103 typing). Disposition 3 "BY DESIGN" is not supported by the spec. | spec/functional/FR-104-check-state-clauses.md:199-204 |
| FND-002 | high | New FR-360 item 8 sentence says a payload-bearing variant type is a union whose parameter is a valid `case` scrutinee; FR-360's own Description ("the assembler SHALL type it by the variant type's enum type"; FR-208 variant types have "no payload") and item 5 ("a `case` expression whose scrutinee has a variant type" SHALL refuse type-mismatch at the scrutinee) still say the opposite with no payload carve-out. It also cites "item 6" for the case refusal, which is item 5. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:170-174 |
| FND-003 | low | FR-093-AC-64's closing sentence "No restriction on the position of an `if` is placed by the collection forms" is a universal claim the four listed cases cannot verify; it should be dropped or bounded to the listed positions. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1102 |

## Scope and evidence

- Examined clean: FR-360-AC-12 statement (matches FR-056 "a resolving payloadType makes the variant type a union composite" and FR-318 exhaustiveness); TC-1009 step 5 and its expected-result bullet; TC-459 step 8 text (faithful to AC-17 as written, so it inherits FND-001); TC-1422 (faithful to AC-64); TC-1423 (faithful to AC-65 except the unguarded-let clause, see SR-2026111518064800000004 FND-001).
- Disposition claim (b1), all-arity-0 variant types are enums so `case` over them refuses: correct against FR-056 line 532 and FR-360 item 5 (not item 6). "A domain package therefore declares no union whose members all have arity 0" holds for the variant-type meaning, the only model union source.
- Disposition claim (b2), domain-package parameters never Option-wrapped (FR-103): incorrect as a spec reading. FR-103 line 97 says presence is applied as for fields; FR-056 line 726-729 says an `optional` presence wraps the multiplicity's value type in `Option`; the Semantic IR schema makes `presence` and `nullable` required on every param. The code does drop presence (fsl-semantics/src/model/domain_package.rs:541-548 has no presence member; fsl-semantics/src/check/assemble.rs ~2880-2907 calls model_value_type with value_type and multiplicity only), and the doc at fsl-semantics/src/model/operation.rs:18-22 justifies it with "carry no independent presence flag", which the schema contradicts. So AC-166's failure is an FSL code-vs-FR-103 gap, not CG fixture error, unless FR-103 is deliberately amended.
- Disposition claim (b3), model-union S4 emission fails UnlocatedOccurrence/Declaration: consistent with the code path (open_composite in fsl-semantics/src/check/lowering.rs:3099-3165 routes only model records, tuples and events to model-owned nodes; a model payload variant falls to the source composite path and close_composite records a Declaration occurrence at type_declaration(name)). Not reproduced by the reviewer (a Kani job held the machine). Not papered over: AC-12 and TC-1009 step 5 claim check only, no emission; follow-up QSL-2261 is filed as a sub-issue.
- FR-104 sentence vs neighbours: contradicts FR-103 and FR-360 item 3 (FND-001). FR-360 sentence vs neighbours: FND-002.

## Verdict

Not mergeable as is. FND-001 needs either an FSL fix applying presence to parameters per FR-103 (which likely lands AC-166 in FSL) or an explicit amendment of FR-103/FR-360 item 3 with the design decision recorded, and AC-17 rewritten to match. FND-002 is a spec-text fix (narrow FR-360 Description and item 5 to payload-free variant types, fix the item reference). FND-003 is a nit.
