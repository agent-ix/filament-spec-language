---
id: SR-20261120095318
title: "gap analysis of PR3148 FR-124 AC-11/AC-12 and FR-931 AC-16/AC-17"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3148 6f50eedd8b263f0ddf73e0926beb74454356d4f6; FR-124-AC-11, FR-124-AC-12, FR-931-AC-16, FR-931-AC-17"
review_set: subset
---

## Summary

Ticket: QSL-2624. Manual check of the acceptance criteria against the tests (Quoin matrix not run). The tests tagged FR-124-AC-12 build the vault-only subject through ModelSystem::over_model. Production builds the deadlock item through deadlock_model_item, and check_model calls ModelSystem::new over every selection. With no universe for test/vault-other, that system fails check_model_bounds. The production route therefore cannot settle AC-12's refuted case, and the tests do not exercise it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-12 vault-only cases are asserted as a class via over_model, not through deadlock_model_item/check_model, which builds ModelSystem::new over all selections and refuses bounds when the other identity has no universe; production path defect is pre-existing and needs a ticket | fsl-analyze/tests/it/terminal_two_models.rs:890-930; fsl-analyze/src/model_check/check.rs:547-549; fsl-analyze/src/analyze/derived_item.rs:115 |
| FND-002 | low | AC-12 request-level cases on the vault unit (Halt any on V writes no item; Halt any on N still writes one; both identities unsupported) and the package route of the vault-only cases are untested here | fsl-analyze/tests/it/terminal_two_models.rs:910-950 |

## Verdict

The AC-11 tests cover the following on both routes: node-key ordinals, the alias label, Done placed on either alias, the Stop-true and Halt-any proved cases, and the refused/inconclusive case. The FR-931-AC-16/17 source-route unit tests cover count, sum, only, fold, reduce, the alias key and the folded where. The package half of AC-16 and the QSL-2627 wiring are out of scope by decision. FSpec FR-368-AC-22 tests in derived_request.rs:994 and :1066 cover the "reads several or none" case: settled unsupported with no request item, but on the refinement and rpc fixtures, not the vault unit. derived_request.rs:614 covers opting out with the subject's own terminal any.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | check_model now builds the deadlock system over the identities the subject reads, and vault-only refuted Deadlock is asserted on both routes through deadlock_model_item and PackageDeadlockSubject. check_reduced (check.rs:1220), certificate admit (fsl-replay certificate/check.rs:70) and counterexample replay (fsl-replay model_trace.rs:195, model_check/replay.rs:849) still use ModelSystem::new over all selections; ticket needed |
| FND-002 | fixed | the vault-only package route and the request-writer readings (own any opts out, other any does not, both identities SeveralSelections) are now tested |
| FND-001 | fixed | the vault-only deadlock item settles refuted (replayed) and proved (certified) on both routes through analyze_items; the general ModelSystem::new mechanism is code-review FND-010 |
