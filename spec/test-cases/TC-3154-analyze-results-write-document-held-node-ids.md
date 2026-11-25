---
id: TC-3154
title: "Analyze results write document-held node ids"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
---
# TC-3154: Analyze results write document-held node ids

## Description

Analyze results write document-held node ids.

Scope: FR-621-AC-16, FR-621-AC-17.

## Test Procedure

Analyze the probability fixtures `Eventually` and `TooLikely` with exact evidence and validate the written items against the pinned backend-provider schema. Test: `fsl-analyze/tests/it/probability_from_source.rs` `measured_and_proved_items_validate_against_the_backend_provider_schema` and `a_refuted_item_names_its_claim_in_the_document_held_form`. Test: `fsl-analyze/src/analyze/document/held_identity.rs` `a_certificate_identity_stays_structural`; `fsl-replay/src/probability_certificate/source.rs` `a_deeply_nested_certificate_identity_resolves` and `a_held_certificate_identity_is_refused`. Check the proved item's certificate in a fresh process. Test: `tools/fsl-probability-check/tests/it/roundtrip.rs` `certificate_checks_independently`. Analyze a composition with a component-guarantee premise and scan every item for a node id object whose path holds a node id. Test: `fsl-analyze/tests/it/analyze_composition_cli.rs` `a_shared_premise_is_written_once`. Settle a run whose package exceeds the emit limit. Test: `fsl-analyze/tests/it/clause_item.rs` `a_package_over_the_emit_limit_reports_the_typed_cause_not_a_nested_id`.

Tag the tests `#[trace("TC-3154", "FR-621-AC-16")]` and `#[trace("TC-3154", "FR-621-AC-17")]`.

## Expected Results

- The proved item's identity and document, and both claim loci, validate; the identity's `node_id` is the structural id of the claim node; the refuted `counterexample` holds no node id object in any node id's path; the test does not assert that the `counterexample` validates against `CounterexampleEntry`.
- The checker accepts the document as written and refuses one whose identity holds an index segment.
- A premise item names its dependency in `package` and holds no nested node id object outside a certificate identity; an own-package item has no `package` member.
- The over-limit run reports the typed `NotEmitted` graph cause or the strict reader refusal, never a result with an unheld node id.
