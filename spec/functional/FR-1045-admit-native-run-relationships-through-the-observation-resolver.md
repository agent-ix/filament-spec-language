---
id: FR-1045
title: "Admit native-run relationship selections through the observation resolver"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1039
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-111
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-262
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-287
    type: depends_on
---
# FR-1045: Admit native-run relationship selections through the observation resolver

## Description

When a `native-run/1` call selects relationship correlation, FSL SHALL carry
the explicit selections through `filament-observation::admit` before evaluating
the call. FSL SHALL add no correlation semantics of its own: the observation
resolver decides relationship identity, extent completeness, slot membership,
and the outcome. FSL SHALL preserve the resolver's typed outcome in the
native-run result and exit category.

## Inputs

- The existing native-run call selection and its selected checked package or
  source-compiled domain package (FR-100, FR-1039).
- The selected current snapshot through `call.snapshot` (FR-106). Its existing
  `relationships` extents supply the candidate relationships.
- The optional closed `call.relationships` object. It contains exactly
  `producer`, `binding`, `expected_subject`, `required_relationships`,
  `scope`, `records` and `limits`. The `limits` object contains exactly the
  resolver's names `max_records`, `max_members`, `max_relationships` and
  `max_required_relationships`. Their values use the resolver's corresponding
  selected types, with `expected_subject` in FR-287's authority-qualified
  form. The selected checked package supplies package context; this object
  adds no parallel package selector.
- `required_relationships` is an array of typed FR-262 slots on the wire and a
  set in meaning. The resolver applies `max_required_relationships` to the raw
  array count before collapsing equal slots. Equal slots identify one slot;
  the resolver evaluates distinct slots once in canonical
  `RequiredRelationship` order.
- Every instance in each supplied relationship extent supplies a non-empty `identity` in the
  lowercase-hex encoding of its producer-supplied `RelationshipIdentity`
  bytes (FR-106-AC-72, FSpec FR-181 and FR-480). The resolver receives the
  exact decoded bytes; FSL SHALL NOT derive this identity. These instances are
  the complete candidate set passed to the resolver; FSL SHALL NOT filter
  them by required declaration before identity validation.
- The selected Producer declaration maps each FR-106 source and target object
  reference to the complete FR-287 subject and the FR-262 relationship type
  and declaration identity. A reference whose type has no declaration in the
  selected package refuses `invalid-request` at that reference path after
  snapshot read and before resolver evaluation.
- The resolver receives each selected FR-106 relationship extent state
  unchanged: unlisted means not supplied, `complete: false` is incomplete, and
  `complete: true` with no instances is known empty. FSL SHALL NOT collapse or
  reinterpret these states.
- `scope`, `records` and `limits` use the observation resolver's own types and
  exact resource-limit names; FSL SHALL NOT invent defaults.

The `producer` member is the complete existing Producer interface 1.2 static-
bundle JSON document, embedded with its own camelCase member names and its
digest checked by the Producer interface reader before it is admitted as an
`AdmittedStaticBundle`. FSL SHALL NOT reduce, rename or reinterpret it. Its
encoded bytes count against the native-run request's input bounds. A malformed
or invalid bundle retains the Producer reader's exact refusal cause at
`/call/relationships/producer` and does not call the observation resolver.

The remaining member values use these FSL-owned forms. `binding` has exactly
`identity`, `source_identity`, `schema_identity`,
`signal_identity`, `trigger_identity`, `unit`, `subject_kind` and `required`;
the first six are non-empty strings, `subject_kind` is the FR-181 subject-kind
string, and `required` is Boolean. `expected_subject` and each required-slot
endpoint have exactly `authority`, `subject_kind` and
`authority_local_object_identity`; `authority` has exactly `bundle_identity`,
`bundle_revision` and `digest`, using the full authority values of the
selected Producer interface bundle. `authority_local_object_identity` is
non-empty lowercase-hex bytes. Each required slot has exactly
`declaration_identity`, `source` and `target`, and the declaration identity
names the selected package's exact relationship declaration. The slot's
`authority` is the selected Producer bundle key and `interface_version` is its
version; neither is duplicated in the wire entry. `scope` is a closed object
with exactly `population_identity`, `membership_rule_identity`,
`membership_digest`, `membership_document`, `required_member_identities`,
`observation_sources`, `completeness_dependencies`, `progress_dependencies`,
`clock_identity`, `clock_revision`, `membership_complete`,
`closure_identity`, `closure_digest`, `kind`, `range` and `members`. Its
`membership_document` is the closed `quire.observation.explicit-members/v1`
object `{contract, identity, digest, members}`; `identity` and `digest` equal
`membership_rule_identity` and `membership_digest`, and `members` equals the
sorted distinct `required_member_identities`. The source and dependency arrays
are sorted distinct non-empty identity strings. `kind` is exactly
`{"snapshot":{"snapshot_identity":S}}` or
`{"window":{"window_identity":S}}`; `range` is one of FR-294's closed
`event_position`, `fixed_sample` or `timestamp` forms; each member has exactly
`object_identity`, `record_identity` and `anchor`, with `anchor` one of FR-294's
clock-position forms. Identity bytes use FR-181 lowercase hexadecimal and
digests use FR-295's lowercase-hex form. `records` is an array of complete
`quire.observation.record/v2` documents under FR-288. `limits` has exactly the
four named members; each value is an unsigned JSON integer in `0..=u64::MAX`.
No member accepts an omitted default or an additional field.

## Outputs

- The ordinary native-run call result after the resolver returns
  `AdmissionOutcome::Available`.
- A native-run incomplete result with no call value when the resolver returns
  `AdmissionOutcome::Incomplete`, preserving every reason and payload.
- A native-run refusal with no call value when the resolver returns
  `AdmissionOutcome::Refused`, except `ResourceLimit`, which maps to
  incomplete under FR-285.
- An `invalid-request` refusal before selected-file I/O for request-shape
  defects, including `call.relationships` selected without `call.snapshot`.

## Behavior

### Explicit selection and relationship identity

- `call.relationships` is nested in the existing `call` object and adds no
  top-level native-run request member. It is a closed object with the exact
  members listed in Inputs. Candidate extents come only from the selected
  `call.snapshot`'s FR-106 `relationships` member.
- FSL SHALL pass the selected Producer bundle, binding, expected subject,
  required slots, scope, records, limits and extent states to the resolver. It
  SHALL NOT substitute defaults or derive a selection from a provider name,
  trace identifier, foreign key, display label or ambient configuration.
- The package's selected declaration supplies relationship type and exact
  declaration identity. Each subject retains FR-287's complete authority,
  kind and opaque identity. A relationship retains FR-262's relationship type,
  package, declaration identity and ordered source/target endpoints. FSL SHALL
  NOT sort, normalize, infer or replace endpoint identities.
- Every instance in each supplied extent is a candidate and SHALL carry its
  producer-supplied relationship identity. An absent, empty, non-string,
  uppercase, odd-length or otherwise malformed identity is `invalid-request`
  at that instance's identity path after the snapshot is read and before
  resolver evaluation. It is not reconstructed from endpoints, array position,
  foreign key, trace ID, provider name or ambient configuration.
- Required slots SHALL be caller-selected; FSL SHALL NOT infer them from the
  candidates present. Equal slots are one set member. The resolver applies the
  raw required-slot array count to `max_required_relationships` before
  deduplication, then reports each missing or incomplete-extent slot once in
  canonical order if slot evaluation completes without refusal.
- Exact replay of one producer identity and complete relationship value is one
  candidate. Equal relationship facts with different producer identities are
  distinct candidates. Reusing one producer identity with different facts
  refuses `RelationshipIdentityContradiction` before slot evaluation. Two or more
  distinct matching candidates with no incompatible candidate are ambiguous.
  Any incompatible candidate with the required declaration, producer authority
  and interface version, and the required source makes the slot conflicting,
  even when another candidate matches. These are the
  resolver's FR-111/FR-262 semantics; FSL SHALL implement none of them locally.

### Resolver outcomes

FSL SHALL call `filament-observation::admit` with the checked package, Producer,
binding, expected subject, candidate relationship extents, required slots,
scope, records and limits. It SHALL map the returned `AdmissionOutcome` as
follows:

| Resolver outcome | Native-run result | Exit category | Result value |
| --- | --- | --- | --- |
| `AdmissionOutcome::Available` | Continue the selected call and return its ordinary result. | The call's ordinary result category and FR-285 exit. | The call's value, if it completes. |
| `AdmissionOutcome::Incomplete { reasons }` | Return `spine-run-result/1` with `incomplete`; do not evaluate the call. | `incomplete` (exit 22 under FR-285). | No call value; preserve every typed incomplete reason and payload in `reasons`. Missing relationship and incomplete-extent slots have one reason each in canonical order. |
| `AdmissionOutcome::Refused { cause: ResourceLimit }` | Return `spine-run-result/1` with `incomplete`; do not evaluate the call. | `incomplete` (exit 22 under FR-285). | No call value; preserve the exhausted bound through the existing `limit` member. |
| `AdmissionOutcome::Refused { cause }` for every other cause | Return `spine-run-result/1` with `refused`; do not evaluate the call. | `refusal` (exit 20 under FR-285). | No call value; preserve the exact typed cause. Conflict and ambiguity retain the exact required slot in refusal fields. No stage member is emitted. |

For every non-resource-limit refusal, `code` is `invalid_runtime_input` and
`cause` and `fields` are:

| Typed cause | `cause` | `fields` |
|---|---|---|
| `InvalidSelection(field)` | `invalid-selection` | `{"selection": field}` |
| `ProducerVersion` | `producer-version` | no fields |
| `BindingMismatch { record, field }` | `binding-mismatch` | `{"record": record, "field": field}` |
| `SubjectMismatch { record }` | `subject-mismatch` | `{"record": record}` |
| `InvalidReference(Empty(component))` | `invalid-reference-empty` | `{"component": component}` |
| `InvalidReference(ForeignAuthority)` | `invalid-reference-foreign-authority` | no fields |
| `InvalidReference(WrongSubjectKind)` | `invalid-reference-wrong-subject-kind` | no fields |
| `InvalidReference(UnknownRelationshipDeclaration { identity })` | `invalid-reference-unknown-relationship-declaration` | `{"identity": identity}` |
| `InvalidReference(ReversedEndpoints)` | `invalid-reference-reversed-endpoints` | no fields |
| `InvalidReference(WrongEndpointKind { endpoint })` | `invalid-reference-wrong-endpoint-kind` | `{"endpoint": endpoint}` |
| `RelationshipIdentityContradiction { identity }` | `relationship-identity-contradiction` | `{"identity": lowercase-hex(identity bytes)}` |
| `RelationshipConflict { required }` | `relationship-conflict` | `{"required": canonical-slot-JSON}` |
| `AmbiguousRelationship { required }` | `ambiguous-relationship` | `{"required": canonical-slot-JSON}` |
| `ClockMismatch { record }` | `clock-mismatch` | `{"record": record}` |
| `InvalidWindow` | `invalid-window` | no fields |
| `DuplicateRecord { record }` | `duplicate-record` | `{"record": record}` |
| `DuplicateMember { record }` | `duplicate-member` | `{"record": record}` |
| `MemberRecordMismatch { member, record }` | `member-record-mismatch` | `{"member": member, "record": record}` |
| `DerivedIdentityMismatch { artifact }` | `derived-identity-mismatch` | `{"artifact": artifact}` |

Each field value is a JSON string. `field`, `component`, `endpoint` and
`artifact` use the exact kebab-case spelling of their typed enum member;
record and member identities retain their exact text. Opaque relationship
identity bytes use lowercase hexadecimal without a prefix. `canonical-slot-JSON`
is the same byte-for-byte canonical JSON text defined for FR-100's incomplete
reason payload.

The native-run result carries the existing `spine-run-result/1` outcome
member. Resource exhaustion uses the existing
`{"kind":"incomplete","limit":S}` form, where `S` names the exhausted
`max_records`, `max_members`, `max_relationships` or
`max_required_relationships` limit. Typed correlation incompleteness uses
`{"kind":"incomplete","reasons":[R,...]}` as added by FR-100-AC-24.
Refusal uses `{"kind":"refused","code":S,"cause":S,"fields":{S:S}}`
when the refusal record has fields; every refusal field value is text. The
`required` field for relationship conflict or ambiguity is the UTF-8 canonical
JSON text of the complete slot specified by FR-100-AC-24. The command exit code
is the FR-285 code for the mapped category.

The resolver's `IncompleteReason` set is `MissingValuation`,
`MissingMembership`, `MissingRelationship { required }`,
`IncompleteRelationshipExtent { required }` and `MissingClosure`.
Its `RefusalCause` set is `InvalidSelection`, `ProducerVersion`,
`BindingMismatch`, `SubjectMismatch`, `InvalidReference`,
`RelationshipIdentityContradiction`, `RelationshipConflict`,
`AmbiguousRelationship`, `ClockMismatch`, `InvalidWindow`, `ResourceLimit`,
`DuplicateRecord`, `DuplicateMember`, `MemberRecordMismatch` and
`DerivedIdentityMismatch`. `ResourceLimit` is the sole `Refused` cause that
maps to native-run incomplete (exit 22), as FR-285 requires for a resource
limit; every other `Refused` cause maps to refusal (exit 20). FSL SHALL
preserve the specific variant and payload for every returned reason or cause.

FR-106 snapshot read/admission failures precede correlation and retain their
existing result, except that when correlation is selected, an unknown
relationship type in a snapshot reference is `invalid-request` at that
reference path after read and before resolver evaluation. Within correlation,
required slots are evaluated in canonical resolver order; the first refusal
stops admission and returns alone, discarding incomplete reasons accumulated
during this admission. If an incomplete outcome is reached before slot
evaluation, that outcome returns unchanged and slot evaluation does not occur.
When slot evaluation is reached and no refusal occurs, it returns every
`MissingRelationship` or `IncompleteRelationshipExtent` reason in canonical
slot order.

### Request refusals

- `call.relationships` without `call.snapshot`, a missing or malformed member,
  an unknown member, or a malformed required-slot list SHALL refuse
  `invalid-request` at stage `request`, exit 20, before selected-file I/O; the
  command-error envelope details name the exact request member path.
- A malformed or invalid Producer interface 1.2 static bundle SHALL refuse
  with the Producer reader's exact cause at `/call/relationships/producer`,
  stage `request`, exit 20, before resolver evaluation, with command-error
  details `{"path":"/call/relationships/producer","cause":S}`, `S` the
  exact Producer reader cause text.
- An absent, empty, non-string, uppercase, odd-length or otherwise malformed
  relationship `identity` on any instance in a supplied extent SHALL refuse
  `invalid-request` at that instance's `identity` member path, stage `request`,
  exit 20, after snapshot read and before resolver evaluation; command-error
  details are `{"path":P}` for that exact JSON Pointer `P`. Identity is the producer's exact value,
  not a value derived by FSL.
- When correlation is selected, a snapshot relationship type that has no
  declaration in the selected package SHALL refuse `invalid-request` at the
  exact relationship reference path, stage `request`, exit 20, after snapshot
  read and before resolver evaluation; command-error details are
  `{"path":P}` for that exact JSON Pointer `P`.
  Without correlation selection, FR-106's
  existing `wrong-role-mapping` behavior remains. A foreign authority,
  reversed direction or otherwise invalid FR-287 endpoint retains the
  resolver's exact typed refusal after file admission; it is not missing or
  ambiguous.
- FR-106 read failures retain their existing result and precede resolver
  admission. Request-member shape errors remain FSL-owned and are detected
  before any selected file is opened; content-validation errors retain the
  resolver's typed cause after the file is read.
- A request that does not select relationship correlation SHALL keep the
  existing native-run behavior unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1045-AC-1 | `call.relationships` is a closed nested object with exactly `producer`, `binding`, `expected_subject`, `required_relationships`, `scope`, `records` and `limits`, whose four limit members are the resolver names; a missing snapshot or malformed/unknown request member refuses `invalid-request` at stage `request`, exit 20, before selected-file I/O. | Test (`tests/it/native_run_relationships.rs::invalid_relationship_selection_fails_before_snapshot_io`) |
| FR-1045-AC-2 | Every instance in each supplied extent maps to the selected package's exact relationship type/declaration and complete ordered FR-287 endpoints; reversed direction refuses `InvalidReference`. Each candidate requires non-empty lowercase-hex identity bytes passed unchanged to the resolver; replay with equal identity and facts deduplicates, identity rebind refuses `RelationshipIdentityContradiction`, and equal facts with distinct identities are distinct candidates. An absent/empty/non-string/uppercase/odd-length identity or undeclared relationship type refuses `invalid-request` at its exact path after read and before resolver evaluation; without correlation, FR-106 keeps `wrong-role-mapping`. | Test (`tests/it/native_run_relationships.rs::snapshot_relationship_identity_round_trips_exact_bytes`, `tests/it/native_run_relationships.rs::invalid_relationship_identity_refuses_after_snapshot_read`, `tests/it/native_run_relationships.rs::relationship_identity_defines_exact_replay`, `tests/it/native_run_relationships.rs::unknown_relationship_type_in_correlation_is_invalid_request_after_snapshot_read`, `tests/it/native_run_relationships.rs::invalid_snapshot_relationship_references_keep_exact_refusals`) |
| FR-1045-AC-3 | For each required slot, an unlisted or complete-empty extent with no candidate yields `MissingRelationship`; a `complete: false` extent yields `IncompleteRelationshipExtent` even with a matching candidate, unless a known incompatible candidate establishes `RelationshipConflict`. Any incompatible candidate conflicts even with a match; two distinct matches with no incompatible candidate yield `AmbiguousRelationship`. | Test (`tests/it/native_run_relationships.rs::relationship_extent_states_and_conflict_precedence_reach_resolver`) |
| FR-1045-AC-4 | Equal required slots are one set member. The resolver applies `max_required_relationships` to the raw list before deduplication; afterward, distinct slots are evaluated in canonical `RequiredRelationship` order. Every missing or incomplete-extent slot is returned once in that order only if slot evaluation completes without refusal; a slot refusal returns alone and discards collected incomplete reasons. An incomplete outcome before slot evaluation is returned unchanged. Candidate relationships do not create required slots that the caller omitted. | Test (`tests/it/native_run_relationships.rs::required_slots_are_a_set_and_missing_reasons_use_canonical_order`, `tests/it/native_run_relationships.rs::required_relationship_limit_counts_raw_entries`, `tests/it/native_run_relationships.rs::candidate_relationships_do_not_infer_required_slots`, `tests/it/native_run_relationships.rs::slot_refusal_discards_collected_incomplete_reasons`) |
| FR-1045-AC-5 | `Available` proceeds to the ordinary call; typed incompletes return a `spine-run-result/1` incomplete outcome with reasons and payloads, exit 22; `ResourceLimit` returns incomplete exit 22 with the corresponding limit; all other refusals return a `spine-run-result/1` refused outcome with the exact code, cause and text fields, exit 20. Resolver outcomes have no stage member. | Test (`tests/it/native_run_relationships.rs::resolver_outcomes_keep_native_run_stage_and_category`, `tests/it/native_run_relationships.rs::resolver_incomplete_reasons_preserve_typed_payloads`, `tests/it/native_run_relationships.rs::resolver_refusals_keep_cause_codes_and_fields`) |
| FR-1045-AC-6 | FR-100/FR-106 snapshot read failures occur before correlation and keep their existing outcome. A snapshot relationship type lacking a package declaration is `invalid-request` at the relationship reference path, after read and before resolver evaluation, only when correlation is selected; without correlation, FR-106's existing `wrong-role-mapping` remains. Correlation selected without `call.snapshot` refuses `invalid-request` at `call.relationships`, stage `request`, exit 20, before file I/O. | Test (`tests/it/native_run_relationships.rs::snapshot_read_failure_precedes_relationship_admission`, `tests/it/native_run_relationships.rs::unknown_relationship_type_in_correlation_is_invalid_request_after_snapshot_read`, `tests/it/native_run_relationships.rs::unknown_relationship_type_without_correlation_keeps_wrong_role_mapping`) |
| FR-1045-AC-7 | When `call.relationships` is absent, the native-run result and exit category equal existing call behavior, and snapshot relationship identity remains optional for ordinary admission. | Test (`tests/it/native_run_relationships.rs::relationship_identity_is_optional_without_correlation`) |

## Dependencies

- [FR-100](FR-100-run-a-named-function-through-the-spine.md) owns native-run
  call selection and result production.
- [FR-106](FR-106-admit-snapshots-and-invocations.md) owns snapshot admission,
  relationship extents, and producer-supplied relationship identity.
- [FR-1039](FR-1039-run-every-native-run-member-over-a-selected-checked-package.md)
  owns the checked-package call route.
- FSpec [FR-111](ix://agent-ix/filament-specification/FR-111) and
  [FR-262](ix://agent-ix/filament-specification/FR-262) own explicit typed
  relationship correlation, set/canonical-slot semantics, and conflict
  precedence. FSpec [FR-287](ix://agent-ix/filament-specification/FR-287)
  owns authority-qualified runtime subjects. FR-181 and FR-480 own lowercase
  hexadecimal transport for opaque identity bytes.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md) owns the
  outcome-category exit mapping.
