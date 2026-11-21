---
id: SR-17953000000003
title: "QSL-2741 receipt spec-review-integrity review"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-2741; PR3344; FR-1046 effect-decision input binding"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1046
    type: reviews
---

## Summary

The bounded receipt binding is grounded in existing canonical contracts; its cross-item negative needs one premise to distinguish an absent match from independently equal item-local records.

Frozen head 97e8023162e29e3f1e5602956a37081860636f9b. Ticket QSL-2741 is In Progress. Only FR-1046 changes; no code, Cargo command, checkout, push, new worktree or runtime qualification was performed. Quoin 0.36.7; Quire 0.52.0 (engine 0.62.2); spec-artifacts-process@v0.29.2. Reviewer model id unavailable; run FA31F6CA-EED2-4006-BE4B-075BCFF7BEF2.

## Verdict

CONDITIONAL: clarify AC-4 before receipt implementation.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-4 requires refusal when a receipt matches item 0 and is selected only by item 1, without requiring that item 1 lacks an equal recommendation. Two identical analysis items may each emit the same canonical FR-822 object, which has no request-item index. If item 1 has one actual equal record, the approved full-equality rule requires acceptance, while this negative requires refusal based on an origin the receipt does not encode. Qualify the negative with no matching record in item 1 and keep item-local acceptance when its actual record is equal; do not introduce an opaque origin id. | spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md:230 | wrong-requirement |

## Coverage

Checked provision, canonical validation, actual typed matching, no-trace preservation, retry/contradiction handling, error precedence and existing budget/cancellation contracts for consistency. The behavior's candidate set is item-local, but AC-4 does not distinguish equal records emitted by two items. All other reviewed obligations have concrete inputs, outcomes and failure classes.

The architecture-approved request member is items[i].effect_decisions referencing unchanged FR-329 documents through existing wrappers. Formal request/result identity and output fields remain unchanged; no new catalog tag, opaque decision id, authorization operation or effect adapter is introduced. These are future obligations: no test binding or effect-receipt implementation is claimed.

## Assurance Context

No applicable type: AssuranceProfile document exists in the repository specification tree. Direct delegated review is applicable; no enforced workflow or human gate is inferred. Existing quoted historical profile mentions are not profiles.

## Deterministic evidence and selection

Scoped Quire validation exits 0: 1/1 document grammar-clean; 6/12 property-extractable. The computed Quire matrix mints all twelve criteria: eleven untagged Test obligations and one method-without-symbol Inspection obligation, expected before implementation. Quoin advise succeeds and reports one advisory mismatch at AC-12; its disposition is reviewer method judgement above, not an implementation claim. No global implementation gap audit was performed in this spec-only change. Spec-review lenses selected: base, integrity, ears-conformance, dependency, failure-domain and evidence. A separate object review is not selected because no new domain object/schema is defined; existing object anchoring was checked with dependency/integrity. Risk-complexity and criterion-strength/Jev are not selected; no availability or calibration claim is made.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-1046",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "When a `native-analyze/1` item selects effect-decision receipts, FSL SHALL\nattribute each selected receipt to one typed protocol recommendation that\nthe same item actually emits. This is an input binding to the existing\n[FSpec FR-329](ix://agent-ix/filament-specification/FR-329)\n`filament.effect-decision/v1` contract, not another receipt format."
    },
    {
      "id": "FR-1046-AC-1",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A real protocol analysis emits one typed recommendation before another role's model write. An FR-329 canonical document whose `decision` is that emitted object, supplied through `{file, identity}` and selected by `items[0].effect_decisions`, binds and returns the unchanged normal item: its native trace, replay, formal result and outcome member set equal the same run without the receipt."
    },
    {
      "id": "FR-1046-AC-2",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise."
    },
    {
      "id": "FR-1046-AC-3",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Changing only the emitted recommendation's type tag, node, role, instance, replicated actor, operation identity, typed receiver, one typed argument or argument order in `decision` refuses `invalid_runtime_input` / `invalid-value`, exit 20. The existing message begins `effect-decision wrong-decision` and names the selector, document and `/decision`, while `details` and `locus` are null; equal display names do not make distinct declaration identities match."
    },
    {
      "id": "FR-1046-AC-4",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A receipt whose decision matches a recommendation of item 0 but is selected only by item 1 refuses for item 1. A native trace containing two indistinguishable occurrences refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no decision id is generated."
    },
    {
      "id": "FR-1046-AC-5",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Selecting the same canonical receipt twice, including under two provision identities, returns the same normal outcome as one selection and performs no additional protocol step or external effect. Two different transport receipts with the same request key and canonical effect request validate against that same recommendation without an additional result entry or FSL-owned history."
    },
    {
      "id": "FR-1046-AC-6",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Two selected receipts with the same FR-329 request key and different canonical effect requests refuse `invalid_runtime_input` / `invalid-value` at the later document's `/effect_request`. The existing message begins `effect-decision request-key-contradiction` and names both provision identities; no successful receipt-binding output or replacement request is published."
    },
    {
      "id": "FR-1046-AC-7",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Invalid JSON, a missing required property, duplicate member, unknown outer member and mistyped property refuse `invalid_runtime_input` with `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member` and `wrong-value-kind`, respectively. A wrong contract version, out-of-domain value and non-canonical encoding use `invalid-value`. Each message begins `effect-decision malformed-receipt`; no data is repaired or converted into another canonical document version. The catalog selection comes from typed variants, not message parsing."
    },
    {
      "id": "FR-1046-AC-8",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A non-array item member, malformed or unresolved document selector, and a top-level `effect_decisions` member each refuse `invalid-request`, exit 20, at their request pointer. Absent and empty item selectors retain the old request identity and outcome. No selected receipt is forwarded into the admitted FR-331 request item, and all outcomes validate against the unchanged published schema."
    },
    {
      "id": "FR-1046-AC-9",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "With the request below its independent request-byte ceiling, the real selected program, model and canonical receipt files are admitted at their exact aggregate dependent-byte allowance. One byte less gives the existing `resource_exhausted` incomplete outcome and original dependent-byte ceiling before receipt consumption; the receipt bytes are neither omitted nor read through an uncharged path."
    },
    {
      "id": "FR-1046-AC-10",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A malformed selected receipt paired with a wrong decision fails canonical admission first. After canonical admission, a wrong decision on the first selector precedes a later key contradiction; a valid first selector followed by a conflicting request reports that later contradiction. Other request items keep their original settled records."
    },
    {
      "id": "FR-1046-AC-11",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "A bounded incomplete or unsupported protocol item with no binding-eligible trace retains its own cause and stage and reports no accepted receipts; it does not report a successful binding or a nonexistent-decision fact about an unexplored continuation."
    },
    {
      "id": "FR-1046-AC-12",
      "path": "spec/functional/FR-1046-bind-effect-decision-receipts-to-protocol-recommendations.md",
      "role": "examined",
      "excerpt": "Adding a transport receipt, grant, effect request, lineage or optional effect observation changes neither model fields nor compensation registrations, queues or formal truth. The binding path calls no adapter, starts no process and opens no network connection; receipt admission makes no claim that the supplied grant authorizes the action or the business effect occurred."
    },
    {
      "id": "FSpec-FR-329",
      "path": "filament-specification/spec/objects/interfaces/FR-329-effect-decision-receipt.md",
      "role": "context_only",
      "excerpt": "The request key is (`decision`, `authority_grant`, `idempotency_key`, canonical operation/arguments). Reuse with different inputs is a contradiction."
    },
    {
      "id": "FSpec-FR-822",
      "path": "filament-specification/spec/functional/protocol/FR-822-declare-observe-only-protocol-boundaries-and-typed-recommendations.md",
      "role": "context_only",
      "excerpt": "The **recommendation record** is the typed canonical object\n\n`{\"type\": \"protocol-recommendation\", \"node\": <the control's declaration path>,\n\"role\": <the role's qualified name>, \"actor\": <the acting instance, present\nfor a replicated role only>, \"instance\": <the protocol instance ordinal>,\n\"operation\": <the operation's qualified name>, \"receiver\": <the receiver\nreference>, \"arguments\": <the argument vector in parameter order>}`."
    },
    {
      "id": "FR-1032",
      "path": "spec/functional/FR-1032-bind-a-protocol-recording-to-the-analyze-request-by-document-identity.md",
      "role": "context_only",
      "excerpt": "A `documents` entry SHALL be `{file}` or `{file, identity}`, closed."
    },
    {
      "id": "FR-621",
      "path": "spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md",
      "role": "context_only",
      "excerpt": "An `analyze` document's `last_stage` is FR-286's greatest stage the request\nentered, so a reader sees how far the request got without reading its items."
    },
    {
      "id": "FSpec-FR-272",
      "path": "filament-specification/spec/objects/foundation/FR-272-typed-diagnostic-cause.md",
      "role": "context_only",
      "excerpt": "`invalid_runtime_input`"
    }
  ]
}
```
