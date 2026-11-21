---
id: SR-2026112116405040524101
title: "Gap analysis (planless, targeted) — PR #3307 FR-390-AC-28 / FR-533 trace"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@e1900638963fc3fa3fe06b6968b3cd346400ed8c (base d25728d74d830ccde8e0aae5c14d0b315af0db1e); spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md, spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md, spec/test-cases/TC-1070, fsl-eval/tests/it/protocol_slots/acceptance.rs, fsl-package/src/reader_protocol_projection.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
---
# SR-2026112116405040524101: Gap analysis — PR #3307

## Summary

Ticket: QSL-2271 (PR filament-spec-language#3307). This is a planless gap analysis, targeted to the
criteria the PR's four new tests claim, plus the two FRs that own the changed behavior. It is not a
repository-wide matrix campaign. Evidence: one computed matrix,
`quire matrix --scope . --format tsv` (quire 0.52.0, engine 0.62.2), exit 0, filtered to FR-390 and
FR-533 rows. The headline: the PR's only trace tag, `FR-390-AC-28`, over-claims. Before the PR,
AC-28 had no binder at all. After it, the four new tests make AC-28 read `tagged`, but none of them
evaluates a slot that AC-28 lists. No acceptance criterion states the behavior the PR adds.

## Verdict

**CONDITIONAL**: one medium and one low finding. There are no stubs and no placeholder returns.
Every changed source path is owned at statement level by FR-390 ("the same `ProtocolSlotEvaluation`
and charge log as the checked package"). Semantic review was not separately opted into. The
intent-to-test reading below came out of the same pass.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | Coverage inflation: the four new tests carry `#[trace("TC-1070", "FR-390-AC-28")]`, making AC-28 `tagged` in the computed matrix (binders acceptance.rs:5320/5334/5348/5362, the only ones), yet each evaluates `Refunds.activation.guard` (TC-1070 row 1) on a rewritten source, a slot AC-28 does not list; AC-28's trigger guard, retry, recovery, effect/commit/finish, related, visible and case-guard rows over fixtures E and L remain unevidenced while reported tagged. | fsl-eval/tests/it/protocol_slots/acceptance.rs:5319 |
| FND-002 | low      | No acceptance criterion owns reader-route parity for bounded Rational literals or for the two operand shapes of `filament.op.rational.div`; FR-390 has per-type parity ACs (AC-18 Boolean/Integer/record, AC-24 native, AC-25 quantity) but none for Rational, so the new tests have no criterion they can bind to honestly. | fsl-package/src/reader_protocol_projection.rs:2541 |

## Coverage

- Criteria examined: FR-390-AC-28. On the base it had no binder, since no `FR-390-AC-28` tag exists
  outside this diff. At head it is `tagged`, by the four new tests only. Also examined: FR-390-AC-1,
  which lists the activation guard but expects TC-1070's own values over the unmodified protocol, so
  it is not a fit either. FR-390-AC-18, AC-24 and AC-43 were read as per-type precedents. AC-43 is
  untagged, but that is the pre-existing baseline and outside this diff. FR-533-AC-1 covers the bare
  route, not the `AdmittedPackageRead` reader view these tests use.
- Reverse gap: the changed code has an owning FR (FR-390) but no owning AC (FND-002).
- Stubs and hollow evidence: none. Every test asserts source `true` first, then parity of outcome,
  losses and charges.
- Repository-wide untagged criteria: not assessed, as a scope decision in the brief.
- Remediation options, for the owning lanes: drop `FR-390-AC-28` from the four tags and keep
  `TC-1070`; or have the spec lane add a Rational reader-parity AC to FR-390 and retag to it.
- Plan completion: not assessed

## Dispositions

Round 1, at head `d4f92088ca14fd09ff626c7b8a761b62dd01ae95`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | All four tests now carry `#[trace("TC-1070", "FR-390-AC-75")]`, and `FR-390-AC-28` is gone from them. `quire trace --id FR-390-AC-28` now gives `resolved: false` with 0 claims, which is the honest baseline. `quire trace --id FR-390-AC-75` gives exactly the 4 claims, at acceptance.rs:5320, 5334, 5348 and 5362. The test bodies are unchanged since `e1900638`. |
| FND-002 | fixed | FR-390-AC-75 is added. It is a narrow verification criterion for FR-390's existing parity statement: R1 to R4 on the `Refunds.activation.guard` slot of fixture E only, with no extension of AC-28. TC-1070 gains the section "Rational literal and division reader vectors" (rows R1 to R4, which match the four test sources, source first), and its scope line now includes AC-75. FR-390 owns reader projection and parity, while FR-533 is the bare route, so FR-390 is the right owner. The spec text is reviewed in SR-2026112116405040524102. |
