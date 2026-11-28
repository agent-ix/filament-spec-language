---
id: SR-5590833
title: "Gap analysis — QSL-3316 keep certificate identity node ids structural"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-621-AC-16, FR-621-AC-17, FR-282-AC-11, TC-3154 and the production lines of PR #3926 at cb815d2f5e0040d8f3be068fc4cb0b2fcc27e6f4"
review_set: subset
---

## Summary

Ticket: QSL-3316. This is a planless gap analysis bounded by the diff. `quire matrix --scope . --format json` (quire 0.52.1, quoin 0.36.10) binds FR-621-AC-16 to `a_certificate_identity_stays_structural`, `measured_and_proved_items_validate_against_the_backend_provider_schema`, `a_refuted_item_names_its_claim_in_the_document_held_form`, `a_deeply_nested_certificate_identity_resolves` and `a_held_certificate_identity_is_refused`. It binds FR-282-AC-11 to the TC-764 roundtrip tests. Every changed production line has an owning requirement: FR-621 "Node ids in an item", FR-282-AC-11 and FSpec FR-412. No stub was introduced.

## Verdict

**CONDITIONAL.** There is one low trace gap. The FR-621-AC-16 clause on the checker refusing a held identity is asserted in a fresh process only by `certificate_checks_independently`, and that test does not carry the TC-3154 / FR-621-AC-16 tag that TC-3154 asks for.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-3154 names `tools/fsl-probability-check/tests/it/roundtrip.rs` `certificate_checks_independently` for FR-621-AC-16's "the independent EN-5 checker accepts ... and refuses it when `identity.node_id` is replaced by the held `request.claim.node`", and says to tag the tests `#[trace("TC-3154", "FR-621-AC-16")]`. The test carries only `#[trace("TC-764", "FR-282-AC-1", "FR-201-AC-11")]`, so the matrix does not bind the fresh-process half of AC-16 (the tag was already missing on main). | tools/fsl-probability-check/tests/it/roundtrip.rs:578; spec/test-cases/TC-3154-analyze-results-write-document-held-node-ids.md |

## Coverage

- FR-621-AC-16 (examined): the structural identity, the resolved claim equality, the held refusal (unit test and fresh process), and depth past 128 (unit test of `HeldCertificate` only).
- FR-621-AC-17 (examined): the existing composition tests are unchanged. `assert_no_nested_node_ids` now exempts certificate identities, as the edited AC states.
- FR-282-AC-11 (examined): the new owner refusal is exercised by the existing `foreign-model` case in `a_fresh_checker_refuses_altered_values_objectives_identities_ranks_and_provisions`, where request and certificate agree on a non-owning model.
- Reverse gap: `CheckedGraph::owning_model` is the former `fsl-analyze` private function, moved unchanged and owned by FR-621 / FSpec FR-412.
- `unmarked-trace-id` warnings for `#[trace]` in `src/` unit tests are systemic (8494 in the matrix) and not specific to this PR.
- Plan completion: not assessed
- Evidence: no tests run (cargo admission refused on memory; locks A and B held).
