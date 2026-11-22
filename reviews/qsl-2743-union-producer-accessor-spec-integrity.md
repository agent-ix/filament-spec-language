---
id: SR-967175026302197768900474181941935084683
title: "QSL-2743 PR #3398 spec integrity: FR-531 AC-45..48"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@5f6b1a76e5358f75d38c5d401dd0479d2d82183f; PR #3398 spec delta; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md body and AC-45..AC-48"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-531", type: reviews }
---

## Summary

Ticket: QSL-2743. This integrity analysis covers completeness, consistency and atomicity of the four new FR-531 ACs against the FR-531 body and the code. Companion base review: SR-967175026302197768900474181941935084682.

## Verdict

**CONDITIONAL**: one medium and one low finding. I found no contradiction with the existing ACs. AC-30 still owns only the enum, source-enum and collection conversion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-45 to AC-48 have no owning normative statement in the FR-531 body. The body specifies the identities channels and the source-union rows, including a SHALL for acquiring `source_unions`. It says nothing about model-union rows, `Emission::selected_model_union_rows`, `SelectedMonitorClosureRows.model_unions`, the payload classification, or `VariantRecord.position`. The ACs therefore exceed the requirement statement they verify. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:370-420,1097-1100 |
| FND-002 | low | AC-47 does not state what happens to a case with no payload form. The code refuses lowering with the writer fault `KeyFault::UnsupportedUnionPayload` (invariant `union-payload-has-ir-form`). The AC says "it yields no payload form, so no `Clock` row is retained for it", which could also be read as silently omitting the case or the row. It repeats "has no model-union payload form" in the same sentence. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1099; fsl-semantics/src/check/lowering/model.rs:2969-2974 |

## Coverage

Examined:
- The FR-531 body sections on the identities file and on source-union acquisition (lines 370-420), as context.
- AC-45 to AC-48 for atomicity, consistency with AC-26/27/30 and truth against the code.

Results:
- AC-45 compound: recorded in SR-...682 FND-001.
- AC-46 is atomic: one join and one refusal control.
- AC-48 is atomic.

EARS is not applicable, because no requirement statement changed. Hidden-assumption probes for external tools, APIs or concurrency do not apply.

## Dispositions

Disposition round 2. Head 7cfd1558f632b5f26841cdcc218f21b07638e1be. Same reviewer, run 59b28e57-7cbb-423a-b6ed-b607ed3538ff.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new FR-531 body section "Selected model union producer" (`FR-531...md:399-427`) uses SHALL for each of these: retention on `SemanticGraph::selected_model_unions`, `VariantRecord.position` semantics, Clock/Data/None classification, the typed refusals `UnsupportedUnionPayload`/`MissingUnionVariant` with no row retained, `Emission::selected_model_union_rows` and its complete row contents, and no empty stand-in or caller-built row. It also states the monitor-plan rekey, the `SelectedMonitorClosureRows::model_unions` use, and that this producer adds no `identities.json` member and no `Value::Clock`. Each statement matches `model_union_payloads`, `union_payload_form`, `model_union_rows` and `carry_rows` at this head. AC-45 to AC-49 are now each derived from a body statement. |
| FND-002 | fixed | AC-47 (`FR-531...md:1129`) now states classification only and does not repeat itself. The fault outcome (typed refusal, no row retained, no silent omission) is stated in the body and in the atomic AC-49 (`FR-531...md:1131`, Inspection). `model.rs:2954-2974` raises it through `fault(...)`. |
