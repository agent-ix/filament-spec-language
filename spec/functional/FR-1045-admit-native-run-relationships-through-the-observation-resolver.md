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
  - target: ix://agent-ix/filament-specification/FR-181
    type: references
  - target: ix://agent-ix/filament-specification/FR-480
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-038
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-944
    type: references
  - target: ix://agent-ix/filament-observation/FR-001
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-014
    type: references
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
  bytes (FR-106-AC-74, FSpec FR-181 and FR-480). The resolver receives the
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
- `binding`, `expected_subject`, `required_relationships`, `scope`, `records`
  and `limits` use the closed forms of "Member forms" below. Each form
  decodes into exactly one field of the resolver's typed `AdmissionRequest`;
  FSL SHALL NOT invent defaults, derive values or add members the resolver
  cannot take.

The `producer` member is the complete existing Producer interface 1.2 static-
bundle JSON document, embedded with its own camelCase member names and its
digest checked by the Producer interface reader before it is admitted as an
`AdmittedStaticBundle`. FSL SHALL NOT reduce, rename or reinterpret it. Its
encoded bytes count against the native-run request's input bounds. A malformed
or invalid bundle retains the Producer reader's exact refusal cause at
`/call/relationships/producer` and does not call the observation resolver.

### Member forms

The resolver's admission request is a typed Rust value, `AdmissionRequest`,
with the fields `package`, `producer`, `binding`, `expected_subject`,
`relationships`, `relationship_extents`, `required_relationships`, `scope`,
`records` and `limits` (filament-observation FR-001). The library's own
request reader (filament-observation FR-014) is private to its progress
admission entry, so FSL reads the members below itself and builds the typed
value. Member names, tag members and closed value sets follow FR-014's request
and record forms, so a record or scope written for the library reads the same
way here. FSL keeps two encodings of its own native-run request: opaque bytes
are lowercase hexadecimal (FSpec FR-181 and FR-480), not JSON byte arrays, and
the four limits are JSON integers, as FR-100's `accounting` counters are.

Value notation used by the tables:

- *text*: a JSON string, passed to `Identity::new` exactly as written. FSL
  does not trim, normalize or reject an empty text value; the resolver's
  typed `InvalidSelection` check decides emptiness.
- *hex*: a non-empty JSON string of lowercase hexadecimal digit pairs (FSpec
  FR-480); the resolver receives the exact decoded bytes.
- *digest*: a JSON string of exactly 64 lowercase hexadecimal digits, decoded
  to the 32 bytes of `Digest::new` (FR-014 "Digests are 64 lowercase
  hexadecimal digits").
- *u64* and *i128*: a JSON string in FR-038's integer spelling (optional
  leading `-` only for *i128*, no `+`, no leading zero, never `-0`), whose
  value lies in the Rust type's range. FR-106 snapshots and FR-014 use the same
  decimal strings because a JSON number above 2^53 does not survive RFC 8785
  readers.
- *null-or-X*: the member is present with JSON `null` or an X value; `null`
  is `None`. The member itself is never omitted.

The authority form `A` is a closed object:

| Member | JSON form | Library field |
|---|---|---|
| `bundle_identity` | non-empty JSON string | `AdmittedBundleKey.bundle_identity` |
| `bundle_revision` | closed `{"namespace": S, "value": S}`, both non-empty JSON strings | `AdmittedBundleKey.bundle_revision: Revision { namespace, value }` |
| `digest` | closed `{"label": S, "value": D}`, `S` a non-empty JSON string and `D` `sha256:` followed by 64 lowercase hexadecimal digits | `AdmittedBundleKey.digest: ProducerDigest { label, value }` |

These are the same members and order that FR-100's canonical slot JSON writes
for a refused or missing slot, so a slot read from a result can be resubmitted
unchanged. An authority that differs from the selected Producer bundle's key
is well-formed. The resolver refuses it as
`InvalidReference(ForeignAuthority)`.

The subject form `S` is a closed object, the FSpec FR-287 subject key:

| Member | JSON form | Library field |
|---|---|---|
| `authority` | `A` | the authority argument of `QualifiedSubject::from_authority` |
| `subject_kind` | non-empty JSON string, FR-287's String kind | `SubjectKind::new` over its exact UTF-8 bytes |
| `authority_local_object_identity` | *hex* | `SubjectIdentity::new` over the decoded bytes |

FSL builds every subject with `QualifiedSubject::from_authority`, which keeps
the written authority verbatim, so the resolver and not FSL decides whether
it is the selected authority.

The anchor form `N` is a closed object tagged by `kind`:

| `kind` | Other members | Library value |
|---|---|---|
| `"event-position"` | `position`: *u64* | `Anchor::EventPosition(position)` |
| `"fixed-sample"` | `index`: *u64*, `epoch_nanos`: *i128*, `period_nanos`: *u64* | `Anchor::FixedSample { index, epoch_nanos, period_nanos }` |
| `"timestamp"` | `instant_nanos`: *i128* | `Anchor::TimestampNanos(instant_nanos)` |

The range form `R` is a closed object tagged by `clock_family`:

| `clock_family` | Other members | Library value |
|---|---|---|
| `"event-position"` | `start`: *u64*, `end_exclusive`: *u64* | `ClockRange::EventPosition { start, end_exclusive }` |
| `"fixed-sample"` | `start`: *u64*, `end_exclusive`: *u64*, `epoch_nanos`: *i128*, `period_nanos`: *u64* | `ClockRange::FixedSample { start, end_exclusive, epoch_nanos, period_nanos }` |
| `"timestamped-event"` | `start_nanos`: *i128*, `end_nanos`: *i128* | `ClockRange::Timestamp { start_nanos, end_nanos }` |

An empty or reversed range, a non-positive period and an anchor of another
clock family are well-formed. The resolver decides them as `InvalidWindow`,
`ClockMismatch` or `MemberRecordMismatch`.

The members of `call.relationships` map onto `AdmissionRequest` as follows.
Pointers are relative to `/call/relationships`.

| Wire member | JSON form | Library type and field | Malformed |
|---|---|---|---|
| (none) | the selected checked package | `package: PackageSelection`, built as FR-944 builds it: format `native-linked-package/1`, the package name, revision `filament.checked-package/v2` and the SHA-256 of its canonical bytes | not a wire member |
| `/producer` | the Producer interface 1.2 static-bundle object | `producer: AdmittedStaticBundle`, from `StaticProducerBundle::admit_json` over the member's exact JSON value bytes | Producer reader cause at `/producer` |
| `/binding/identity`, `/source_identity`, `/schema_identity`, `/signal_identity`, `/trigger_identity`, `/unit` | *text* | `binding: ObservationBinding`, the field of the same name | `invalid-request` at the member |
| `/binding/subject_kind` | non-empty JSON string | `ObservationBinding.subject_kind: SubjectKind` over its UTF-8 bytes | `invalid-request` at the member |
| `/binding/required` | JSON Boolean | `ObservationBinding.required` | `invalid-request` at the member |
| `/expected_subject` | `S` | `expected_subject: QualifiedSubject` | `invalid-request` at the failing nested member |
| `/required_relationships/<i>` | closed `{"declaration_identity": text, "source": S, "target": S}` | one `required_relationships: Vec<RequiredRelationship>` entry, in array order, from `RequiredRelationship::new(producer, declaration_identity, source, target)` | `invalid-request` at the failing nested member |
| `/scope/population_identity`, `/membership_rule_identity`, `/clock_identity`, `/clock_revision` | *text* | `scope: ScopeSelection`, the field of the same name | `invalid-request` at the member |
| `/scope/membership_digest` | *digest* | `ScopeSelection.membership_digest: Digest` | `invalid-request` at the member |
| `/scope/membership_document` | JSON string | `ScopeSelection.membership_document: Vec<u8>`, the string's exact UTF-8 bytes, which hold one `filament.observation.explicit-members/v1` document | `invalid-request` at the member |
| `/scope/required_member_identities`, `/observation_sources`, `/completeness_dependencies`, `/progress_dependencies` | JSON array of *text* | the `Vec<Identity>` field of the same name, in array order | `invalid-request` at the array or element |
| `/scope/membership_complete` | JSON Boolean | `ScopeSelection.membership_complete` | `invalid-request` at the member |
| `/scope/closure_identity` | *null-or-text* | `ScopeSelection.closure_identity: Option<Identity>` | `invalid-request` at the member |
| `/scope/closure_digest` | *null-or-digest* | `ScopeSelection.closure_digest: Option<Digest>` | `invalid-request` at the member |
| `/scope/kind` | closed `{"kind": "snapshot", "snapshot_identity": text}` or `{"kind": "window", "window_identity": text}` | `ScopeSelection.kind: ScopeKind::Snapshot { snapshot_identity }` or `ScopeKind::Window { window_identity }` | `invalid-request` at the member |
| `/scope/range` | `R` | `ScopeSelection.range: ClockRange` | `invalid-request` at the failing nested member |
| `/scope/members/<i>` | closed `{"object_identity": text, "record_identity": text, "anchor": N}` | one `ScopeSelection.members: Vec<Member>` entry, in array order | `invalid-request` at the failing nested member |
| `/records/<i>` | closed record object below | one `records: Vec<AdmittedRecord>` entry, in array order | `invalid-request` at the failing nested member |
| `/limits/max_records`, `/max_members`, `/max_relationships`, `/max_required_relationships` | JSON integer in `0..=u64::MAX` | `limits: ResourceLimits`, the `usize` field of the same name | `invalid-request` at the member |
| (none) | each instance of the snapshot's FR-106 `relationships` extents | `relationships: Vec<Relationship>`, from `Relationship::new(producer, declaration identity, RelationshipIdentity::new(decoded identity), source, target)` | `invalid-request` at the snapshot path, after read |
| (none) | each FR-106 extent of the snapshot | `relationship_extents: Vec<RelationshipExtent>`, from `RelationshipExtent::new(producer, declaration identity, state)`, `complete: true` as `Complete` and `complete: false` as `Incomplete`; an unlisted declaration has no entry | not a wire member |

Each `records` entry is a closed object with exactly the members of the FR-014
record form, which are the fields of `AdmittedRecord`:

| Member | JSON form | `AdmittedRecord` field |
|---|---|---|
| `identity`, `binding_identity`, `source_identity`, `schema_identity`, `signal_identity`, `trigger_identity`, `unit`, `clock_identity`, `clock_revision` | *text* | the `Identity` field of the same name |
| `subject` | `S` | `subject: QualifiedSubject` |
| `value` | closed `{"state": "present", "value_type": text, "canonical_value": JSON string}` or `{"state": "missing"}` | `value: ValueState::Present { value_type, canonical_value }` or `ValueState::Missing` |
| `visibility` | `"internal"` or `"external"` | `visibility: Visibility::Internal` or `Visibility::External` |
| `anchor` | `N` | `anchor: Anchor` |
| `event_time_nanos` | *i128* | `event_time_nanos` |
| `ingestion_time_nanos` | *null-or-i128* | `ingestion_time_nanos: Option<i128>` |
| `causal_relationship_identity` | *null-or-hex* | `causal_relationship_identity: Option<RelationshipIdentity>` |
| `clock_uncertainty_nanos` | *u64* | `clock_uncertainty_nanos` |

A record is a candidate input, not an owner artifact. The resolver verifies its
`identity` against the owner's record-identity derivation. A mismatch returns
`DerivedIdentityMismatch { artifact: Observation }`, and FSL computes no record
identity of its own. In the same way, the resolver's owner derivations decide
the order and distinctness of the identity arrays, the agreement of
`membership_document` with `membership_rule_identity`, `membership_digest` and
`required_member_identities`, and the canonical bytes of that document. Those
defects return `DerivedIdentityMismatch` with `artifact` `membership`,
`membership-document`, `population`, `membership-population` or
`observation-sources`, and are not request-shape refusals. A closure pair with
exactly one `null` returns `InvalidSelection(Closure)`. Both `null` returns
the `MissingClosure` incomplete reason.

Every object in these forms is closed. An unknown member, a member name that
appears twice in one object, a missing member, a wrong JSON type, an unknown
tag or enum value, malformed *hex*, *digest* or decimal text, and a decimal
outside its type's range each refuse `invalid-request` at stage `request`,
exit 20, before any selected file is opened. The command-error details are
`{"path":P}`, where `P` is the full JSON Pointer of the offending member from
the request root (for example `/call/relationships/records/0/anchor/kind`).
No member takes a default.

A `ReferenceRefusal` returned by `RequiredRelationship::new`,
`Relationship::new` or `RelationshipExtent::new` while FSL builds the typed
request is the resolver's own reference validation. FSL reports it as
`AdmissionOutcome::Refused { cause: InvalidReference(refusal) }` under the
refusal table below, after snapshot read. It is not a request-shape
refusal.

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

- `call.relationships` without `call.snapshot`, or any member at any depth
  that is missing, unknown, duplicated or outside its form in "Member forms",
  SHALL refuse `invalid-request` at stage `request`, exit 20, before
  selected-file I/O. The command-error details are `{"path":P}`, `P` the exact
  JSON Pointer of the offending member.
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
| FR-1045-AC-8 | A `records` entry with every record member in its form reaches the resolver as an `AdmittedRecord` whose fields equal the decoded values. The entry covers a `present` and a `missing` value, an `event-position`, a `fixed-sample` and a `timestamp` anchor, `ingestion_time_nanos` `null` and `"-5"`, `causal_relationship_identity` `null` and `"00ff"`, and `clock_uncertainty_nanos` `"18446744073709551615"`. Hex decodes to the exact bytes and decimals to the exact `i128`/`u64`. A well-formed record whose `identity` is not the owner derivation returns refused `derived-identity-mismatch` with `artifact` `observation`, exit 20. Each of these refuses `invalid-request` at stage `request`, exit 20, before any selected file is opened, with details `{"path":P}` naming its exact pointer: an unknown record member, a repeated member name, an omitted `ingestion_time_nanos`, `value` `{"state":"absent"}`, `visibility` `"Internal"`, `event_time_nanos` as a JSON number or as `"+1"` or `"01"`, `clock_uncertainty_nanos` `"18446744073709551616"`, and `causal_relationship_identity` `"0A"`, `"0"` or `""`. | Test (`tests/it/native_run_relationships.rs::record_members_decode_into_admitted_records`, `tests/it/native_run_relationships.rs::malformed_record_members_refuse_at_their_pointer_before_io`) |
| FR-1045-AC-9 | A `scope` reaches the resolver as a `ScopeSelection` with every field decoded. Each of the three `range` forms with members anchored in the same family gives the matching `ClockRange` and `Anchor` variants. `kind` snapshot and window give the matching `ScopeKind`. The `membership_document` string's UTF-8 bytes pass unchanged, and `membership_digest` decodes to its 32 bytes. Both closure members `null` returns incomplete with reason `missing-closure`, exit 22. Exactly one `null` returns refused `invalid-selection` with `selection` `closure`. An event-position range with `start` equal to `end_exclusive` returns refused `invalid-window`. The owner's membership document with its four JSON members written in another order returns refused `derived-identity-mismatch` with `artifact` `membership-document`. Each of these refuses `invalid-request` at its exact pointer, stage `request`, exit 20, before any file is opened: `range.clock_family` `"timestamp"`, anchor `kind` `"timestamped-event"`, `kind` written as `{"snapshot":{"snapshot_identity":"s"}}`, `membership_digest` with an uppercase digit or 63 digits, `membership_document` as a JSON object, an omitted `closure_identity`, and a member with an extra `position` member. | Test (`tests/it/native_run_relationships.rs::scope_members_decode_into_scope_selection`, `tests/it/native_run_relationships.rs::malformed_scope_members_refuse_at_their_pointer_before_io`) |
| FR-1045-AC-10 | `expected_subject`, each required-slot endpoint and each record `subject` reach the resolver as `QualifiedSubject::from_authority` values with the written authority, the UTF-8 bytes of `subject_kind` and the decoded identity bytes. `binding` maps field-for-field onto `ObservationBinding`. The limits become `ResourceLimits`, and the package selection equals FR-944's. An authority whose `digest.label` differs from the selected bundle's returns refused `invalid-reference-foreign-authority`. A slot naming no Producer declaration returns refused `invalid-reference-unknown-relationship-declaration` with that `identity`. A binding `identity` of `""` returns refused `invalid-selection` with `selection` `binding-identity`, not a request refusal. Each of these refuses `invalid-request` at its exact pointer, stage `request`, exit 20, before any file is opened: a `digest.value` without the `sha256:` prefix, `bundle_revision` as a JSON string, `authority_local_object_identity` `""` or `"AB"`, a non-string `subject_kind`, binding `required` `"true"`, and a limit of `-1`, `"5"` or 18446744073709551616. | Test (`tests/it/native_run_relationships.rs::subject_binding_and_limit_members_decode_exactly`, `tests/it/native_run_relationships.rs::malformed_subject_binding_and_limit_members_refuse_before_io`) |

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
  owns authority-qualified runtime subjects and their String subject kind.
  FSpec [FR-181](ix://agent-ix/filament-specification/FR-181) and
  [FR-480](ix://agent-ix/filament-specification/FR-480) own lowercase
  hexadecimal transport for opaque identity bytes.
- filament-observation
  [FR-001](ix://agent-ix/filament-observation/FR-001) owns the typed
  `AdmissionRequest`, `admit` and its outcomes, and
  [FR-014](ix://agent-ix/filament-observation/FR-014) owns the request and
  record member names, tags and digest spelling that "Member forms" follows.
- [FR-038](FR-038-encode-exact-protocol-numbers.md) owns the decimal integer
  spelling of the *u64* and *i128* members.
- [FR-944](FR-944-map-evaluator-trace-facts-onto-filament-observation-contexts-and-views.md)
  owns the `PackageSelection` FSL builds from a checked package.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md) owns the
  outcome-category exit mapping.
