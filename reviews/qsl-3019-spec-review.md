---
id: SR-967175026302197768900474181941935084852
title: "Spec review of FSL PR #3885: FR-136 and FR-147 compensation classes"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@7508fadd789835811f58886d1a769e587321c5a0; spec/functional/FR-136-check-a-refinement-s-step-rows.md, spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md; checked against FSpec origin/main 691ac5a674 FR-432, FR-430, FR-736 and FSL ADR-027 ST-16 to ST-18"
review_set: subset
---

## Summary

Ticket: QSL-3019. This is a base spec review (integrity and consistency) of the two edited FRs, checked against FSpec FR-432's row and spelling tables and FSL ADR-027.

- FR-136. The `StepClass` comment, the protocol-selection keyword list, the coverage rule and the ST range now include `cfail`, `ctimeout` and `cexpire`. The coverage rule says each template needs its own row for `cend`, `cfail`, `ctimeout` and `cexpire`, and that `cattempt` is decided by a template or operation row. This matches FSpec FR-432's deciding-row rule ("every other class is decided by its own row"). ADR-027 defines ST-16 to ST-18, so the range edit to ST-1 to ST-18 is sound.
- FR-147. The spelling table row, the `Rich` fixture class list, AC-8 and the ST range match FSpec FR-432's internal-list spelling table.

## Verdict

**CONDITIONAL.** The edits are consistent with FSpec and with the code. Two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-136 now requires each compensation template to have its own `cfail`, `ctimeout` and `cexpire` rows, but no FR-136 criterion states it. The only evidence, `each_compensation_outcome_class_needs_its_own_row`, is traced to FSpec FR-432-AC-25 alone, so the FSL matrix shows nothing for the new clause. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:223-225 |
| FND-002 | low | The PR records a contradiction in FSpec FR-432-AC-25. TC-380's `Pay` is `on origin`, yet the criterion requires a `step activate M::Pay` row, a class that exists only for `on each`, and its source lacks `population` and `object` rows. The PR says only that an FSpec correction "is to be filed", with no ticket id. Until the correction is filed, the AC-25 test's two-variant reading is untracked. | fsl-semantics/src/check/refinement/protocol/tests.rs:1195-1240 |

## Dispositions

Round 1 at `52ab15215f39a4552abc526dfb3444ce44d787b0`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 4dfd9a865c adds FR-136-AC-14 and TC-541 step 14, and tags `each_compensation_outcome_class_needs_its_own_row` with `TC-541`, `FR-136-AC-14`. The test asserts exactly what the AC states, and also the `cend` removal. |
| FND-002 | fixed | QSL-3287 (Backlog) tracks the FSpec FR-432-AC-25 correction, and the PR body names it. |
