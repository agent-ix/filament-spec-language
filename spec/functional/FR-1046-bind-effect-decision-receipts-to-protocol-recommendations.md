---
id: FR-1046
title: "Bind effect-decision receipts to emitted protocol recommendations"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1032
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-624
    type: references
  - target: ix://agent-ix/filament-specification/FR-329
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-822
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-182
    type: references
  - target: ix://agent-ix/filament-specification/interface_011
    type: references
---
# FR-1046: Bind effect-decision receipts to emitted protocol recommendations

## Description

When a `native-analyze/1` item selects effect-decision receipts, FSL SHALL
attribute each selected receipt to one typed protocol recommendation that
the same item actually emits. This is an input binding to the existing
[FSpec FR-329](ix://agent-ix/filament-specification/FR-329)
`filament.effect-decision/v1` contract, not another receipt format.

FSL SHALL perform no adapter authorization, dispatch, transport, external
effect or model-state update as a consequence of this binding. A successful
binding establishes which emitted recommendation the supplied document
names; it establishes neither authority nor delivery nor business-effect
truth.

## Inputs

- The existing source or checked package analysis request of
  [FR-620](FR-620-read-a-native-analyze-1-request-and-analyze-its-items.md).
- The optional item member `items[i].effect_decisions`, an array of the
  existing closed document selectors `{"authority": text, "identity": text}`.
  Each selector names a document supplied through `documents[j]`.
- Each receipt file, unchanged, supplied through the existing
  `documents[j]` entry `{file, identity: {authority, identity}}` of
  [FR-1032](FR-1032-bind-a-protocol-recording-to-the-analyze-request-by-document-identity.md).
  Its provision identity is request metadata, not an added member of the
  canonical receipt document.

The receipt is exactly the FR-329 document: `contract_version`
`filament.effect-decision/v1`, required `decision`, `authority_grant`,
`idempotency_key`, `effect_request`, `transport_receipt` and `lineage`, and
optional `effect_observation`, with FR-329's stated JSON types. This binding
adds no property or nested schema to those objects. In particular, it
introduces no opaque decision id and does not reinterpret a temporal
delivery receipt as an effect-decision receipt.

## Outputs

An item whose selected receipts bind SHALL emit its unchanged normal
outcome. This binding adds no output member, canonical wrapper or result
format: the item's formal result, certificate, trace, replay result and
model states remain unchanged. Receipt selectors SHALL NOT be forwarded as
new properties of FSpec FR-331's admitted request item or its formal
decision identity.

FSL SHALL validate the current selected receipt bytes on each invocation,
including when the unchanged formal analysis result is reused. A previous
successful validation is not authority to consume different receipts.
Absent and empty item selectors perform no binding validation and retain
the existing request identity and outcome.

## Behavior

### Provision and canonical admission

FSL SHALL read selected receipt files through the existing bounded document
intake, with the same `{authority, identity}` provision rules as FR-1032.
An unresolved selector, a malformed selector or a non-array
`effect_decisions` member SHALL refuse the request as `invalid-request`,
exit 20, naming its request pointer. Adding `effect_decisions` as a new
top-level request member SHALL remain an unknown-member refusal.

FSL SHALL admit a selected receipt only if its bytes satisfy FR-329's
version, required properties, JSON types and canonical-encoding rules.
FSL SHALL use the existing RFC 8785 canonical JSON contract of
[FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md),
without repairing or re-encoding a non-canonical input into acceptance.
Wrong versions, missing required properties, repeated JSON keys,
out-of-domain values and non-canonical bytes SHALL refuse before receipt
consumption with code `invalid_runtime_input` and the existing typed cause
from the table below. The binding's stable message reason is
`malformed-receipt`; it is not a new catalog cause.

FSL SHALL charge every provisioned receipt file's actual bytes to the same
aggregate dependent-file ceiling as other selected files. Receipt selector
bytes belong to the request-byte ceiling. A reached ceiling SHALL keep
[FR-624](FR-624-name-every-analyze-refusal-and-every-unrouted-verb.md)'s
`resource_exhausted` incomplete outcome, exit 22, naming the original
ceiling; the byte after an exact allowance is the first unpaid byte.
Canonical admission SHALL NOT provide an uncharged alternate file read.
FSL SHALL keep the receipt bytes out of the subject's observation, source
and proof-premise inputs; provisioning the file does not make it one of
those document kinds.

### Typed recommendation matching

FSL SHALL match `decision` to the complete canonical recommendation object
that FSpec FR-822 gives an actual native protocol step: `type`, `node`,
`role`, `instance`, optional `actor`, `operation`, `receiver` and
`arguments`. The operation's existing canonical declaration text and all
typed receiver and argument values SHALL retain their emitted identities
and parameter order. Matching SHALL NOT use a display name, a subset of
these fields, a newly generated decision id or a caller's assertion that
an object was emitted.

The matching candidates SHALL be the typed recommendation steps actually
retained in this item's native counterexample or witness trace, before
serialization. FSL SHALL NOT scan untrusted outcome JSON for lookalike
objects or match another item's record. One matching occurrence is
required. No match SHALL produce binding reason `wrong-decision`; several
matching occurrences SHALL produce binding reason `ambiguous-decision`,
rather than selecting one by discovery order. This includes
indistinguishable recommendations reached again through a loop.

An existing refusal, unsupported result, cancellation, incomplete result or
unsettled result that yields no binding-eligible native trace SHALL retain
its own outcome and consume no receipt. FSL SHALL NOT turn such a result
into either a successful binding or a claim that an unexplored
recommendation does not exist.

### Retries and contradictions

FSL SHALL use FR-329's request key: the canonical `decision`, canonical
`authority_grant`, `idempotency_key` and the matched recommendation's
canonical operation and arguments. The grant, effect request, transport
receipt, lineage and optional effect observation remain FR-329's objects;
this binding supplies no nested field spellings or authorization algorithm.

Repeating the same canonical receipt, including selecting it through
different provision identities, SHALL reconcile with its first occurrence.
Distinct canonical transport receipts or lineage for the same key and
byte-identical canonical `effect_request` SHALL be checked against that
same matched recommendation in selector order. Neither form of retry
creates another action, effect or result entry. The caller's supplied
receipt history stays with those unchanged documents; FSL creates no
durable history or new history output.

If receipts reuse that request key with different canonical
`effect_request` content, FSL SHALL produce binding reason
`request-key-contradiction` at the later selector. It SHALL retain the first
and conflicting provision identities in the diagnostic message. FSL SHALL
NOT merge the conflicting requests,
replace the first receipt silently or invent lineage to explain the
contradiction.

The optional `effect_observation` SHALL NOT enter the model or count as
business-effect evidence through this binding. Domain evidence continues
to require the existing observation-authority intake of
[FSpec FR-182](ix://agent-ix/filament-specification/FR-182).

### Refusal precedence and location

The existing envelope, selected-file and selected-program or Package
admission order SHALL remain unchanged. Once receipt bytes are acquired,
canonical admission precedes recommendation matching, which precedes
request-key reconciliation. After canonical admission, items and their
selectors are considered in request order; within one selector, a matching
refusal precedes its key reconciliation.

FSL SHALL select every refusal from a closed typed binding variant at the
failing operation, before formatting. Its code is
`invalid_runtime_input`; its cause is one of the existing closed variants
of [FSpec FR-272](ix://agent-ix/filament-specification/FR-272), as follows.
The binding reason below is additional message context, not a catalog
cause or a newly serialized field.

| Condition | Existing catalog cause | Binding reason | Pointer inside the selected receipt |
| --- | --- | --- | --- |
| Invalid JSON | `malformed-json` | `malformed-receipt` | the empty string, naming the whole document |
| A required receipt property is missing | `missing-member` | `malformed-receipt` | the absent property's pointer |
| A JSON member is repeated | `duplicate-member` | `malformed-receipt` | the repeated member's pointer |
| An outer member is outside the FR-329 property list | `unknown-member` | `malformed-receipt` | the unknown member's pointer |
| A property has the wrong JSON kind | `wrong-value-kind` | `malformed-receipt` | that property's pointer |
| The version, value domain or canonical encoding is invalid | `invalid-value` | `malformed-receipt` | the value's pointer, or the empty string for whole-document encoding |
| No emitted recommendation matches | `invalid-value` | `wrong-decision` | `/decision` |
| Several emitted occurrences match | `invalid-value` | `ambiguous-decision` | `/decision` |
| A request key is reused with different canonical effect-request content | `invalid-value` | `request-key-contradiction` | `/effect_request` |

The unknown-member row concerns the outer receipt only. FR-329's nested
JSON objects gain no member closure from this binding; `decision` is
matched as a complete recommendation object by the rule above.

These are refusal-category outcomes, exit 20. A malformed receipt refuses
the request before receipt consumption; a matching or reconciliation error
refuses its item as an invalid-input record in the existing outcome. Its
existing diagnostic message SHALL begin with `effect-decision` followed
by the binding reason above, and SHALL name the item selector pointer,
selected document's authority and identity and the receipt pointer above.
A contradiction's message also names the first document using its key.
These values remain typed document references and pointers in the binding
variant; the formatter reads them and does not select the code, cause or
reason by parsing prose. The diagnostic SHALL keep `details` and source
`locus` null: neither a new detail vocabulary nor a program source location
is introduced.

A receipt-refused item SHALL publish no certificate or counterexample.
Other items remain independently settled.
The outcome SHALL retain the greatest stage actually entered under
[FR-621](FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md);
this binding introduces no stage name. Canonical document refusal is a
request refusal before an item consumes it; an existing engine refusal is
not replaced by a later receipt diagnostic.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1046-AC-1 | A real protocol analysis emits one typed recommendation before another role's model write. An FR-329 canonical document whose `decision` is that emitted object, supplied through `{file, identity}` and selected by `items[0].effect_decisions`, binds and returns the unchanged normal item: its native trace, replay, formal result and outcome member set equal the same run without the receipt. | Test |
| FR-1046-AC-2 | Source and checked package intake for the same admitted protocol and canonical receipt perform equal recommendation validation and return their normal outcomes without a new output member. The receipt file has no added `identity`, `format` or FSL envelope member; neither its provision identity nor its bytes become an observation or proof premise. | Test |
| FR-1046-AC-3 | Changing only the emitted recommendation's type tag, node, role, instance, replicated actor, operation identity, typed receiver, one typed argument or argument order in `decision` refuses `invalid_runtime_input` / `invalid-value`, exit 20. The existing message begins `effect-decision wrong-decision` and names the selector, document and `/decision`, while `details` and `locus` are null; equal display names do not make distinct declaration identities match. | Test |
| FR-1046-AC-4 | A receipt whose decision matches a recommendation of item 0 but is selected only by item 1 refuses for item 1. A native trace containing two indistinguishable occurrences refuses `invalid_runtime_input` / `invalid-value` with message prefix `effect-decision ambiguous-decision`; no occurrence is chosen by discovery order and no decision id is generated. | Test |
| FR-1046-AC-5 | Selecting the same canonical receipt twice, including under two provision identities, returns the same normal outcome as one selection and performs no additional protocol step or external effect. Two different transport receipts with the same request key and canonical effect request validate against that same recommendation without an additional result entry or FSL-owned history. | Test |
| FR-1046-AC-6 | Two selected receipts with the same FR-329 request key and different canonical effect requests refuse `invalid_runtime_input` / `invalid-value` at the later document's `/effect_request`. The existing message begins `effect-decision request-key-contradiction` and names both provision identities; no successful receipt-binding output or replacement request is published. | Test |
| FR-1046-AC-7 | Invalid JSON, a missing required property, duplicate member, unknown outer member and mistyped property refuse `invalid_runtime_input` with `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member` and `wrong-value-kind`, respectively. A wrong contract version, out-of-domain value and non-canonical encoding use `invalid-value`. Each message begins `effect-decision malformed-receipt`; no data is repaired or converted into another canonical document version. The catalog selection comes from typed variants, not message parsing. | Test |
| FR-1046-AC-8 | A non-array item member, malformed or unresolved document selector, and a top-level `effect_decisions` member each refuse `invalid-request`, exit 20, at their request pointer. Absent and empty item selectors retain the old request identity and outcome. No selected receipt is forwarded into the admitted FR-331 request item, and all outcomes validate against the unchanged published schema. | Test |
| FR-1046-AC-9 | With the request below its independent request-byte ceiling, the real selected program, model and canonical receipt files are admitted at their exact aggregate dependent-byte allowance. One byte less gives the existing `resource_exhausted` incomplete outcome and original dependent-byte ceiling before receipt consumption; the receipt bytes are neither omitted nor read through an uncharged path. | Test |
| FR-1046-AC-10 | A malformed selected receipt paired with a wrong decision fails canonical admission first. After canonical admission, a wrong decision on the first selector precedes a later key contradiction; a valid first selector followed by a conflicting request reports that later contradiction. Other request items keep their original settled records. | Test |
| FR-1046-AC-11 | A bounded incomplete or unsupported protocol item with no binding-eligible trace retains its own cause and stage and reports no accepted receipts; it does not report a successful binding or a nonexistent-decision fact about an unexplored continuation. | Test |
| FR-1046-AC-12 | Adding a transport receipt, grant, effect request, lineage or optional effect observation changes neither model fields nor compensation registrations, queues or formal truth. The binding path calls no adapter, starts no process and opens no network connection; receipt admission makes no claim that the supplied grant authorizes the action or the business effect occurred. | Inspection |

## Dependencies

- FSpec FR-329 and interface_011 retain the canonical document and external
  authorization/reconciliation boundary unchanged.
- FSpec FR-822 owns the typed recommendation emitted by a real protocol
  step; FSpec FR-182 keeps all effects in the authorized external adapter.
- FSpec FR-272 owns the closed catalog causes; this binding adds no cause
  tag to that vocabulary.
- FR-620, FR-1032, FR-621, FR-624 and FR-286 own the reused provision,
  request, result, refusal and canonical JSON contracts.
