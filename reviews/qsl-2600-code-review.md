---
id: SR-60052
title: "code-review (Rust lane) of filament-spec-language PR 3230, QSL-2600 model-unit compile path"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3230 head 089952824309ead35c2797dc83ae98c378ba66d9; fsl-semantics/src/model/intake/model_unit.rs, fsl-cst/src/grammar.rs, fsl-cst/tests/it/complete_grammar.rs, fsl-command/tests/it/model_unit_request.rs, fsl-semantics/tests/it/model_unit_input.rs"
review_set: subset
---

## Summary

Ticket: QSL-2600. Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD`. Compile-path changes were checked against FSpec FR-154, FR-208 and FR-322 (relationship ends), not against the semantic-IR schema's refusal text. A probe build of HEAD (scratch copy, since deleted) confirmed four behaviours: `note: Text;` and `note: Option<Text>;` are refused by S1 (`expected [`); `note: UUID;` compiles to `ix://example/p/UUID` and is refused `missing-name`; `relationship target: Node [0, 1];` emits `sourceEnd.role` and `targetEnd.role` both `target` and is admitted; and `inverse prev [0, 1]` keeps `prev` as the target role but leaves `sourceEnd.multiplicity` at `[1, 1]`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A relationship with no `inverse` gets a made-up `targetEnd.role` of `"target"`. FR-322 says no name is made up for a missing inverse, and FR-154 admits an absent `targetEnd.role`. The made-up name creates an inverse end the author never declared, and `relationship target: Node [0, 1];` gives both ends the role `target`, so `reaches(.., target)` refuses `ambiguous-name` (FR-322-AC-56 RE-ends-9). | fsl-semantics/src/model/intake/model_unit.rs:1084 |
| FND-002 | medium | The FR-710-AC-2 reaches test cannot detect direction. Its snapshot is symmetric (a->b, b->a), and its only query is reflexive, `reaches(self, self, next)`. A node lies on a cycle exactly when it lies on the reversed cycle, so swapped ends or a backward traversal still give true for a and false for c. It also never runs the AC's `reaches(a, b, next)` over two distinct nodes. | fsl-command/tests/it/model_unit_request.rs:585 |
| FND-003 | medium | `type_ref` maps only Boolean, Int, Text, Integer, Rational, Float32, Float64 and Timestamp to native types. FR-154 natives `String`, `UUID`, `Decimal`, `Bytes`, `Duration`, `JsonObject` and `Clock` become `ix://<pkg>/<Name>` and are refused `missing_declaration`/`missing-name`, against FR-710's statement that a model unit names its own declarations and the native types. The code was already there, but this PR edits this function. | fsl-semantics/src/model/intake/model_unit.rs:2507 |
| FND-004 | medium | The `inverse` multiplicity is parsed and thrown away, and `sourceEnd.multiplicity` is always `[1, 1]`. `relationship next: Node [0, 1] inverse prev [0, 1];` therefore claims every Node has exactly one `prev`, and a relationship with no inverse claims every target has exactly one source. FR-710-AC-2 requires the ends and their multiplicity. The code was already there, but this PR edits this block. | fsl-semantics/src/model/intake/model_unit.rs:1090 |
| FND-005 | low | `population`, `interface` and `part` are now contextual, so they are not reserved, while every other model-unit word (`event`, `machine`, `store`, `port`, `connection`, `allocate`, ...) stays reserved. FR-710 says every grammar word is a reserved spelling of a model unit. The change is justified (for example `part(c.value)` in fsl-analyze/tests/it/package_forms.rs:612), but FR-710 does not record the exception. | fsl-cst/src/grammar.rs:496 |
| FND-006 | low | No test asserts the relationship end roles that this PR changes: `sourceEnd.role` = `next`, and `targetEnd.role` absent without an inverse or `prev` with one. The AC-2 compile test checks only the identity and the target lower bound. | fsl-semantics/tests/it/model_unit_input.rs:296 |

## Scope and evidence

Fixture changes, checked one by one (no assertion was weakened, and every assertion line is unchanged):
- field `value` -> `amount` and parameter `record` -> `entry`: these are genuine defects. Both spellings are reserved FSL tokens (fsl-cst/src/token.rs:162 `Value`, :189 `Record`), and on main they parse-fail.
- event `timestamp: Integer` -> `Timestamp`: a genuine defect. FR-208 event-type/v1 requires an `occurrenceField` whose type resolves to native `Timestamp`, and FR-154 lists `Timestamp` as native.
- `note: Option<Text>` -> `Option<Integer>`: a genuine fixture defect, but for a different reason than the PR states. S1 refuses bare `Text` (`expected [`), so this is a grammar refusal, not a schema one. FR-154 intake would have admitted the typeRef. AC-3 requires only "option field".

Compile-path changes measured against FSpec:
- Omitting an empty identityFields, with object_type/process `optional`, is sound. The FCD schema refuses an empty array, FR-208 requires no object-type identity, and a root process without identity fields is still refused in normalize.rs:1809.
- Map constraints/extensions/unknownPolicy and parameter presence/nullable/defaultKind are TypeDefinition/parameter members that FR-154 states for every type and parameter. Sound.
- Mapping Timestamp to native is sound.
- Relationship roles: putting the forward name on `sourceEnd.role` matches FR-322 ("its sourceEnd.role (forward)"). The fallback `"target"` is FND-001.
- Trimming clause text is sound.
- Contextual keywords: no domain-decl or object-body alternative starts with a bare identifier where `population`/`interface`/`part` could be read as a name, so the declarations stay unambiguous. See FND-005 for the spec wording.

Rust lane: no panics on the changed paths (`?` on token lookups inside an `Option` reader), no unsafe code, no public API change. Gates: the coder's focused log (qsl-2600-focused-0899528243.log) is green. The reviewer ran one probe build only, with no clippy or workspace suite (per brief).

- FR-710-AC-2 (spec/functional/FR-710-author-a-domain-package-in-an-fsl-model-unit.md): examined, FND-001/002/004/006
- FR-710-AC-3: examined, fixtures verified genuine
- FR-710-AC-1, AC-4, AC-5: examined, no defect in the diff

## Verdict

The compile-path fixes are faithful to FR-154/FR-208, and the fixture edits are genuine defects, not weakened assertions. FND-001 is a regression introduced by this diff, and FND-002 leaves the role and direction fix without a sound oracle. Both need fixing in this PR. FND-003 and FND-004 were already in the code but sit in the edited functions, and FND-005 and FND-006 are low.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 48aee5b858: `target_role` is `None` without `inverse`, and `targetEnd.role` is set only when an inverse is written (model_unit.rs:1084-1110); asserted absent in model_unit_input.rs:325 |
| FND-002 | fixed | 48aee5b858: the snapshot is now a one-way chain a->b->c with ranks. Forward(a), Forward(b) hold; Forward(c), Backward(c), Backward(b) fail; swapped ends or a reversed traversal would make Forward(a) false (model_unit_request.rs:585-672) |
| FND-003 | fixed | 48aee5b858: type_ref maps UUID, String, Decimal, Bytes, Duration, JsonObject and Clock (model_unit.rs:2535-2541); a_model_unit_names_every_native_type admits all but Clock, which is mapped the same way and admitted as a field typeRef by FR-154 |
| FND-004 | fixed | 48aee5b858/ee47203e3b: the inverse multiplicity now goes on sourceEnd. Without an inverse, sourceEnd is {lower 0, no upper}, i.e. [0, *]. The schema's multiplicity has an optional `upper` (absent means unbounded), and FR-154/FR-322 put no constraint on an unnamed reverse end beyond its presence, so [0, *] claims nothing and is admissible. The previous [1, 1] was the wrong claim. Tests: model_unit_input.rs:326-327, 341-344 |
| FND-005 | fixed | 48aee5b858: FR-710 Grammar now lists population, interface and part as keywords only in their declaration position |
| FND-006 | fixed | 48aee5b858/ee47203e3b: asserts sourceEnd.role = next, targetEnd.role absent and source multiplicity in the no-inverse test, plus a_relationship_inverse_names_and_bounds_the_source_end for role prev and bounds [0, 2] |
