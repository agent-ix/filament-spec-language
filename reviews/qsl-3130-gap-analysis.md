---
id: SR-967175026302197768900474181941935084834
title: "QSL-3130 gap analysis"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-3130; FR-204-AC-21; changed analyzer/replay monitor and tests"
---

## Summary

Repository matrix evidence was obtained with the static Quire fallback after Quoin refused the full request. FR-204-AC-21 has tagged tests. Reverse-gap and stub review was limited to the changed monitor/key/test surface, so this is a scoped audit and does not claim a repository-wide reverse-gap pass.

## Verdict

**CONDITIONAL.** The owning criterion is tagged, but full repository reverse-gap inventory was not completed and resource-limit enforcement needs correction.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Replay compares `storage_bytes` only after constructing the complete JSON key and serialized byte vector; analyzer similarly builds the certificate key before the byte comparison. The declared bound therefore does not cap transient key construction. Use incremental capped construction and test the first over-budget cell/byte. | fsl-replay/src/probability_certificate/document/formula_monitor.rs:205; fsl-analyze/src/exact_probabilistic/digital.rs:1351 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` refused with `CORE_REFUSED: request exceeds the accepted size`; fallback `quire matrix --scope . --format json` succeeded and produced repository-wide static coverage data, with repository-wide symbol-collision and trace-marker warnings.
- FR-204-AC-21: tagged by the two changed tests `timed_bounded_operators_replay_with_literal_values_and_occurrences` and `timed_non_punctual_nested_always_keeps_each_operator_window`; also tagged by existing `fsl-eval::tests::shared_nested_deadline_uses_both_windows_and_latches_acceptance`.
- Plan completion: not assessed
- Criteria for owning AC: tagged 1 · untagged 0 · tagged-by-ignored-test 0 · method-without-symbol 0
- Evidence: no run evidence read
- Reverse-gap inventory: changed production surface only; no untraced behavior identified there. Whole-repository reverse-gap inventory remains incomplete.
- Stub scan: changed production monitor/key files contain no TODO, `todo!`, or `unimplemented!`; one panic is in an unrelated test helper in `key.rs`. Changed AC tests exercise literal values and replay behavior.
- Semantic review: skipped (not requested).
