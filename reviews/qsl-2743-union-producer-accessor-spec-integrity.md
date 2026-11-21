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
