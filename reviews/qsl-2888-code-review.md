---
id: SR-20261123150413
title: "code review of PR3615 decimal and enum history rows (QSL-2888)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3615 6507d932eb1f7ff4f1a8ebb1b38e620b755a5754; Ticket QSL-2888; fsl-semantics check/refinement/history.rs, check/lowering/refinement_rows.rs, spec FR-135/FR-138/TC-543"
review_set: subset
---

## Summary

Ticket: QSL-2888. Decimal membership, enum member resolution, out-of-set refusal and the un-ignored analyze test are sound; tests re-run green on titan (29 refinement_history, decimal_and_enum_history_rows_check_and_prove). Three non-blocking findings: the Option-of-present history initial has no package-lowering path, a dead `subject` field remains, and decimal key scale behaviour is unasserted. Verdict: MERGE.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A present `Option<T>` history initial (Option<Phase> = Phase::OPEN, also Option<Int>, Option<Decimal>) reaches literal_at, which returns UnbuiltLiteral for a present option; FR-138-AC-9 claims option-of-enum rows check and compute but only replay keys are tested | fsl-semantics/src/check/lowering.rs:7340 |
| FND-002 | low | `RefinementCause::Unsupported.subject` is now always None and its doc still describes a per-type subject; dead leftover | fsl-semantics/src/check/refinement.rs:246 |
| FND-003 | low | Decimal history keys retain scale (FR-120-AC-16), so decimal(150,1) and decimal(15,0) key differently; neither FR-138 nor TC-543 states or asserts this | fsl-replay/tests/it/refinement_history.rs:790 |

## Scope and evidence

Examined: FR-138-AC-9, FR-630-AC-3, FR-135 unsupported-feature wording, TC-543 step 7. Option-some gap is pre-existing and shared with Option<Int[0,1]> (same literal_at path, not a regression); no ticket tracks it. Traces TC-543/FR-138-AC-9 and TC-1590/FR-630-AC-3 are real.
