---
id: SR-1535
title: "Bounded gap analysis of source catalog producers"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2051; Cargo.lock; fsl-conformance/Cargo.toml; fsl-conformance/cases/catalog-source-structural.native; fsl-conformance/tests/it/catalog_coverage.rs; fsl-conformance/tests/it/main.rs"
review_set: subset
reviewed_commit: d71733b414ac6caf0cac66a7b84c4b25ba0cbc6d
pr: 3198
pr_url: https://github.com/agent-ix/filament-spec-language/pull/3198
module: spec-artifacts-process@0.2.0
reviewer_model: unreported
---

## Summary

Ticket: QSL-2051. This is the requested bounded, diff-only acceptance-to-test and evidence-strength assessment, not a repository-wide matrix audit. The delivered source-producer area has real public-path assertions and honestly retains the remaining qualification gaps.

Plan completion: not assessed

## Examined Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| source operation producers | examined | Cases name real source fixtures and demand operation identities after strict admission |
| structural producers | examined | Node checks distinguish `composite_type` and `expression` tags and their declared semantic forms |
| authoritative catalog consumption | examined | `serde_json::from_str(CHECKED_OPERATION_CATALOG_V1)` |
| independent mismatch control | examined | Altered expectation demands an absent Integer operation from actual compiled Boolean source |
| closed catalog dispatch control | examined | A newly inserted identity is reported in `unrepresented_operations` |
| full qualification gap | examined | The strict full-catalog test remains ignored with QSL-2051 until additional source adapters are authored |
| strict-reader gaps | examined | Index and composite IEEE admission remain desired successes under IR-1491 and IR-1320 / STD-654 |
| test registration | examined | `mod catalog_coverage;` |
| requirement-level references | context_only | `#[trace("FR-350", "FSpec-FR-322")]` |
| full QSL-2051 acceptance | context_only | Every catalog operation and every FSpec FR-322 node form requires source production and strict admission |

## Verdict

**PASS** for integrity of the delivered subset. Public source parsing/checking, emission, no-omission checking and strict admission precede operation and node-form assertions. The expected identities describe producer behavior; their independent authoritative membership is checked against the published catalog. The test-control cases can fail and are described as controls rather than language coverage.

Complete QSL-2051 acceptance remains **unmet**. The current table does not cover all catalog operations or all FSpec node forms. The ignored aggregate remains a strict catalog assertion, not a successful claim that all node forms are covered. Reader-gated cases preserve strict desired admission, with six pending regressions excluded from passing coverage. Existing integer composite-equality cases remain active. No passing test asserts that a capability is missing.

The changed helpers are confined to this integration module and owned by the source-conformance task; no new production behavior lacks a requirement, no external catalog/schema/expected package is copied, and no stub or guard suppresses the delivered positive assertions. A missing operation, changed form or strict-reader failure makes the corresponding source case fail.

No computed matrix was rerun, no whole-repository reverse-gap/stub inventory was performed, and no individual criterion closure is claimed from the whole-requirement trace references. Those are explicit scope limits imposed by the user and lead, not a claim of complete repository assurance. The peer's reported focused executions were accepted without remeasurement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and Remaining Work

Author receipts cover 41 source cases and two controls with focused passing evidence, scoped Clippy and owned-file formatting. The failed adapter assertions were repaired without repeating unchanged passing cases. The sequence-index and Float-leaf strict-reader failures remain recorded under their existing owners; they were not re-probed here.

The code lead reports the final affected-component gate passed 43 tests with zero failures and seven explicitly ignored cases. The coder and code lead retain completion state, remaining adapters and eventual full qualification. Public PR #3198 is recorded here; this posting-only update does not repeat source review or runtime checks and does not claim full acceptance.
