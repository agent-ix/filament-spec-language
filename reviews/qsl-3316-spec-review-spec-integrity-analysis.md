---
id: SR-5590834
title: "Spec review — QSL-3316 FR-621, FR-282-AC-11, TC-3154 edits"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md, spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md, spec/test-cases/TC-3154-analyze-results-write-document-held-node-ids.md; PR #3926 at cb815d2f5e0040d8f3be068fc4cb0b2fcc27e6f4; against FSpec 691ac5a6"
review_set: subset
---

## Summary

Ticket: QSL-3316. Checked the edits against pinned FSpec 691ac5a6. backend-provider-v1 `CertificateIdentity.node_id` is `CertificateNodeId` (schema.json:1382-1384), and FR-412 says identity `node_id` "is certificate identity JSON, not a checked-package-v2 wire NodeId". FR-621's new "Node ids in an item" exception, the `certificate` row wording, AC-16 and AC-17 follow that. FR-282-AC-11's owner-model refusal matches the checker change. TC-3154 lists the new tests.

## Verdict

**CONDITIONAL.** The edits are consistent with FSpec and with each other. The new last clause of FR-621-AC-16 is under-scoped.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-621-AC-16 ends "a certificate whose identity nests past 128 JSON levels is read as written". It names no reader (`HeldCertificate::read`, or the checker accepting it) and no certificate kind. As implemented, only the held reader and resolver handle depth past 128. A value-bounds `lower`/`upper` document with such an identity is refused `Malformed` in `value_bounds::unwrap` (`serde_json::from_slice`). The only test is a unit test of `HeldCertificate::read`/`resolve`. Two readers could judge the clause met or unmet. | spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md (FR-621-AC-16); fsl-replay/src/probability_certificate/document/value_bounds.rs:164-165 |

## Scope

- FR-621 `certificate` row (examined): the identity `node_id` is now the structural id, consistent with FSpec FR-412.
- FR-621 "Node ids in an item" (examined): the exception for certificate identities, and the refusal of an identity holding an index segment.
- FR-621-AC-16, FR-621-AC-17 (examined).
- FR-282-AC-11 (examined): refuses a non-owning `model` even when the request names the same model.
- TC-3154 procedure and expected results (examined).
- FSpec FR-412 identity row and backend-provider-v1 schema `CertificateNodeId`/`CertificateIdentity` (context only).
