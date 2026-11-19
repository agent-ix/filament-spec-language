---
id: FR-944
title: "Map the evaluator's trace facts onto filament-observation's contexts, selections and views"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-861
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-264
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-966
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-001
    type: depends_on
  - target: ix://agent-ix/filament-observation/FR-005
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
---
# FR-944: Map the evaluator's trace facts onto filament-observation's contexts, selections and views

## Description

FR-861 makes `filament-observation` the one owner of the partial-value,
completeness, activation and repair rules that `fsl-eval` applies. The owner's
rules take owner values: a `Context` over an admitted `History`, a
module `Selection`, and the validated `View` of a derived document. This
requirement states how the evaluator builds each of them from the facts it
holds (a trace's positions, stamps and cover, a progress assertion, an
activation scope, a superseding request's prior results) and how it reads the
owner's answer back into the reports FR-501, FR-502, FR-524 and FR-526 state.
It also states the one rule the owner does not have yet, the completeness
revision rule FR-502 needs, as a requirement on the owner.

### Decision: one admitted history per request, built from the trace

Every owner module except the interval relation reads a `Context`, and a
`Context` needs a `History` of a `QualifiedObservation` that the owner's
`admit` produced. The evaluator therefore admits the request's trace through
`filament_observation::admit` once, before any owner rule runs, and every
owner call of that request shares that history. Building the history from the
trace the evaluator already admitted (FR-106) keeps one admission authority
for the trace's identities and stamps, so every position in an owner document
maps back to the identity FR-502's covered set records.

### Decision: every admission selection is derived, none is supplied

The owner's `AdmissionRequest` was designed for a caller that selects its
package, producer, binding, subject, membership, closure, sources and scope.
The evaluator has no such caller: a run request names a checked package, a
clause, a trace and progress, and nothing else. Every member of the admission
request is therefore derived, deterministically, from three things only: the
checked package (its identity, contract version and canonical bytes), the request's
trace with its clock profile and cover, and `fsl-eval`'s own crate-constant
producer bundle. No member is read from the run request as an owner selection
and no member has a default the caller could change. Two runs over equal
packages, traces and progress therefore build byte-equal admission requests,
and every owner-derived identity in them (record, membership, population) is
computed by the owner's own derivation functions, never spelled by the
evaluator.

The finite recorded trace is its own population. Its members are exactly the
distinct facts its positions state, its closure is that explicit enumeration, its scope is one
immutable snapshot of those members, and its one observation source is the
clock binding. FSL has no separate closure-definition artifact, no producer
relationship graph and no second source for a trace, so the evaluator does not
invent one.

### Decision: an owner refusal of the evaluator's request is an internal failure

Because every selection is derived from inputs FSL has already admitted, an
owner refusal or incomplete outcome of the admission request means the
evaluator built an inconsistent request. That is an evaluator defect, not a
fault in the caller's input, so it is reported as an internal failure (exit
`30`) and never as `invalid_runtime_input`. The two owner maxima a valid trace
can still exceed (the owner's population and string ceilings) are checked by
the evaluator before admission and reported as limits.

### Decision: an uncertain cover is a closed interval in quarter-ticks

The owner's `EventTimeInterval` is closed with integer endpoints, and its
`relation_to` answers which orders two intervals admit. FR-501's covers are
not all closed: the counter cover of a reading `k` is the half-open interval
`[k-u, k+1+u)` of the counter unit, and an interval stamp (FR-966) may be
open at either end (`earliest_open`, `latest_open`). Every cover is therefore
mapped, uniformly, to a closed integer interval in units of a quarter tick:
a closed endpoint `c` maps to `4c`, an open lower endpoint `a` to `4a+1` and
an open upper endpoint `b` to `4b-1`. The counter cover becomes
`[4(k-u), 4(k+1+u) - 1]`, an exact stamp `k` becomes `[4k, 4k]`, a bounded
cover `[t-u, t+u]` becomes `[4(t-u), 4(t+u)]`, and every bound `b` the
comparison reads is scaled to `4b`.

Quarter ticks keep every strict comparison strict. An open end moves its
endpoint inward by one quarter tick, less than the distance to the next whole
tick, so it never reaches a value a closed endpoint could hold. A difference
of two covers reads one endpoint of each, so a doubly-open difference moves
by at most two quarter ticks, less than the four between two whole-tick
thresholds: a difference whose true supremum is the integer `D` has a scaled
supremum in `[4D-2, 4D]`, and one whose true infimum is `D` a scaled infimum
in `[4D, 4D+2]`, so no scaled endpoint crosses an integer threshold `4b`
unless the true endpoint does. An open supremum `D = b` still lands at or
below `4b`, and an open supremum `D = b+1` lands at `4b+2`, above it. Half
ticks are not enough: an open lower end at `a` and an open upper end at `l`
give `2a+1` and `2l-1`, a difference of `2(l-a) - 2`, so a doubly-open
difference with supremum `l-a = B+1` against the bound `B` scales to exactly
`2B` and reads `True`, while the difference can lie anywhere in `(B, B+1)`
and the truth is `Indeterminate`. With the quarter-tick mapping the owner's
answers equal FR-501's on every reading: definitely-before of two covers, the
bound comparison `later - earlier <= b` as the order of `later` against
`earlier` shifted by `4b`, and the eligibility comparisons below. No owner
rule is added for it.

Scaling and shifting run in `i128` over the trace input's stamps and
uncertainties (`u64`) and a clause's bounds (64-bit integers). Every scaled
endpoint, scaled bound and shifted endpoint is then below `2^68` in
magnitude, far inside `i128`, so the mapping cannot overflow: every admitted
reading maps exactly, with no wrapping, saturation or refusal.

## Inputs

- The admitted trace of one request: each position's identity, stamp,
  interval stamp, order key, arrival watermark, admitted observation
  document (spine route) or FR-516 `EvidenceRef` (timed-call route), shared
  and instance-bound valuations and aggregate samples, the clock binding's name, unit, epoch and period, the profile
  identity and the cover (FR-501, FR-522, FR-966).
- The checked package's identity, contract version and canonical bytes, on the source
  route and on the bare-package route alike.
- The clause occurrence: its `temporal_clause` node, occurrence ordinal,
  activation and `over` parameter.
- The request's progress assertion and the binding's ledger entry (FR-502).
- The activation scope, deliveries and origins (FR-524).
- A superseding or invalidating request's prior results and their inputs
  (FR-526).

## Outputs

The owner documents and views below, and FR-501's, FR-502's, FR-524's and
FR-526's reports, read from them.

## Behavior

### History and context

- Before any owner call, the evaluator SHALL admit the request's trace with
  `filament_observation::admit` as one `AdmissionRequest` whose every member
  is derived as "The admission request" below states, and SHALL continue only
  on `AdmissionOutcome::Available`.
- The evaluator SHALL keep a side mapping from each record's owner-derived
  identity to its position's FR-106 identity, built at admission, and SHALL
  report every owner answer (FR-501, FR-502, FR-524, FR-526) under the FR-106
  identities it maps to. An owner record identity never stands in for an
  FR-106 identity in a report, and no owner document carries an FR-106
  identity: a record's value holds its document without the document's
  labels. The mapping is many-to-one where several positions state one fact
  (below). On the FR-516 route a position's FR-106 identity is its
  `EvidenceRef` identity (the observation identity FR-516 orders and reports
  by), and the record's value likewise omits the reference's labels, so
  everything this requirement says of FR-106 identities, the relabelling
  check included, applies to it unchanged.
- The tick-to-nanosecond conversion SHALL be exact: a stamp or `u` of `k`
  ticks of a timed unit is `k × s` nanoseconds, where `s` is the unit's
  declared rational factor to the second (FR-230's `time ... unit`) times
  10^9, so `ms` = `rational(1, 1000)` gives `s` = 1000000, with no
  rounding; a clock with no time unit (an event position or fixed sample
  count) takes `s` = 1. A product that is not an integer number of
  nanoseconds, or that exceeds the owner's nanosecond range, SHALL refuse the
  request at stage `admit`, `invalid_runtime_input`/`invalid-value`, naming
  the position, before any owner call.
- Before deriving any owner identity, the evaluator SHALL compare the trace
  with the two owner maxima a valid trace can exceed, read from
  `filament_observation::authority::Limits::owner_max()` and never copied: a
  trace with more positions than `max_population_entries` SHALL be refused at
  stage `admit`, `stage_limit_exceeded`/`node-count-exceeded`, `bound` that
  maximum, `actual` the position count, `setting`
  `filament_observation::authority::Limits.max_population_entries`; a
  position whose record value text (below) is longer in UTF-8 bytes than
  `max_string_bytes` SHALL be refused at stage `admit`,
  `stage_limit_exceeded`/`input-bytes-exceeded`, `bound` that maximum,
  `actual` the text's length, `setting`
  `filament_observation::authority::Limits.max_string_bytes`, naming the
  position. As for FR-330's reader limits, `setting` names the member of the
  limits value that bounds it, not an FR-461 dotted setting, because the
  owner fixes the maximum and a request cannot raise it.
- An `Err` from an owner derivation the evaluator calls to build the request
  (`record_identity`, `derive_membership`, `population_identity`), an
  `Incomplete` or `Refused` outcome of `admit`, and a refusal of the producer
  bundle's own admission SHALL each end the request as an internal failure:
  `runtime_invariant`/`established-invariant-broken`, invariant
  `observation-admission`, carrying the owner's error code, refusal cause or
  incomplete reasons unchanged, category internal failure, exit `30`, with no
  outcome reported for the clause. It is never ignored and never reported as
  `invalid_runtime_input`, because the evaluator, not the caller, selected
  every member the owner refused.
- Each owner call SHALL use `Context::new(History::batch(&qualified),
  &authority, &subject, revision, predecessor)` with `authority` the clause
  occurrence's key as definition identity, the package selection's `revision` as
  definition revision and the package digest as definition digest; `subject` the scope's
  snapshot identity (the membership rule identity, below) as scope identity,
  which is the scope identity the owner compares against the qualified scope,
  and the admitted population identity; `revision`
  1 plus the number of accepted requests already recorded under the binding;
  and `predecessor` the binding's last accepted owner document of the same
  module, or none.

### The package selection

- `package` SHALL be `PackageSelection { format:
  NATIVE_LINKED_PACKAGE_FORMAT, identity, revision, digest }` with `identity`
  the checked package's `name` (FSpec FR-322), `revision` the package's
  `contract_version` literal `filament.checked-package/v2`, and `digest` the
  SHA-256 of the package's canonical bytes.
- A checked package has no revision, identifier, hash, digest or seal (FSpec
  FR-322, FR-201): it is identified by its `name` and compared structurally.
  The owner requires a non-empty revision string, and the contract version is
  the one revision-like fact every package states, so it fills that member.
  It distinguishes no two packages of one contract and is never read back.
- The digest SHALL be an evaluator-local integrity value, held in memory
  only, as FSpec FR-322 permits a consumer: it is computed over the exact
  bytes the reader consumed, or, for a package held in memory from
  compilation, over the bytes of its canonical S4 emit (FR-264), and it is
  used only as the owner's required `digest` member and, through it, the
  owner `Context`'s definition digest. It is never written to any document
  or file FSL writes, never names, identifies or versions a package,
  and never decides whether two packages are the same.
- `fsl-package` SHALL give every checked package, on both routes, the
  accessor `canonical_bytes(&self) -> &[u8]`:
  - the bare-package reader (FR-531) SHALL retain the exact bytes it read.
    The reader admits only canonical bytes and refuses any other encoding
    `noncanonical_wire` (FSpec FR-322-AC-4), so these are the input bytes
    exactly;
  - a package held in memory from compilation SHALL obtain them by the
    canonical S4 emit (FR-264), once, and retain them.

  The same package therefore has the same bytes, and so the same digest, on
  the source route and the bare-package route, and every timed call, from
  either route, carries the digest into the admission request and the owner
  `Context`.

### The producer bundle

- `fsl-eval` SHALL own exactly one static producer bundle, a crate constant
  built and admitted through `StaticProducerBundle::admit` once per process
  and shared by every admission request:
  - `bundleIdentity` `fsl-eval`;
  - `bundleRevision` `Revision::producer(<fsl-eval crate version>)`, the
    owner's namespaced revision form;
  - `interfaceVersion` `1.2.0`, the owner's `PRODUCER_INTERFACE_VERSION`;
  - no `relationships`;
  - `configuration` `{configurationIdentity: "fsl-eval/default", digest}`,
    `digest` the owner's canonical producer-object digest of the one-member
    object `{"configurationIdentity": "fsl-eval/default"}`, computed by
    `StaticProducerBundle::canonical_digest_selection()` over a bundle that
    holds only that member;
  - `digest` the value `StaticProducerBundle::canonical_digest_selection()`
    returns for the bundle above.
- Nothing in a run request selects or changes the bundle. A new `fsl-eval`
  version is a new bundle revision, so its population and owner document
  identities differ from an earlier version's, as the owner intends.

### The admission request

Every member below is derived from the checked package, the clause
occurrence, the trace and its clock profile, and the bundle. `s` is the
tick-to-nanosecond factor above; `C` is the clock binding's name and `P` the
profile identity.

- `binding` SHALL be the `ObservationBinding`:
  - `identity`: the RFC 8785 text of `{"clause": <node_wire text of the
    temporal_clause node>, "clock": C, "occurrence": "<ordinal>",
    "package": <package identity>, "profile": P}`, the FR-502 ledger binding;
  - `source_identity`: `C`;
  - `schema_identity`: `fsl-eval/trace-position/1`, the form of a record
    value below;
  - `signal_identity`: the canonical `node_wire` text of the clause's `over`
    parameter node, or of the `temporal_clause` node when the clause has no
    `over` parameter;
  - `trigger_identity`: the canonical `node_wire` text of the
    `temporal_clause` node followed by `/activation`, naming the clause's
    activation (`on each` with its guard, or `on origin`). The per-instance
    trigger identities of FR-524 are not binding members: they select
    instances inside one binding, and every record of the binding shares one
    trigger identity;
  - `unit`: the owner-side unit label `<u>/4`, `<u>` the clock's declared
    time unit name, or `position` for an event position clock, which has
    none (`ms/4`, `position/4`). Every owner-side unit label is this
    quarter-tick label: the binding's `unit`, every record's `unit` and the
    `EventTimeInterval` unit below are one string, because the owner's
    `partial::derive` requires an interval's unit to equal its admitted
    record's unit. FSL reports name the FSL-side clock unit `<u>`;
  - `subject_kind`: `package`;
  - `required`: `true`.
- `expected_subject` SHALL be `QualifiedSubject::new(&bundle,
  SubjectKind("package"), SubjectIdentity(<package identity bytes>))`: the
  observed system is the system the checked package describes, and the trace
  has no other subject.
- `relationships` and `required_relationships` SHALL be empty, and every
  record's `causal_relationship_identity` SHALL be absent. A trace carries no
  producer relationship: an order key is a sequence order of one order
  authority (FR-501), not a typed relationship, and the bundle declares none.
- `records` SHALL hold one `AdmittedRecord` per distinct position fact, in
  canonical order (FR-501) of the first position stating it, with:
  - `binding_identity`, `source_identity`, `schema_identity`,
    `signal_identity`, `trigger_identity` and `unit` the binding's;
  - `subject` the expected subject;
  - `value`: `ValueState::Present { value_type: "fsl-eval/trace-position/1",
    canonical_value }`, `canonical_value` the RFC 8785 text of
    `{"aggregate_samples": G, "instance_aggregate_samples": IG,
    "instance_valuations": IV, "interval": I, "observation": D,
    "order_key": K, "valuations": V}`, integers in FR-038's decimal
    spelling, with:
    - `D` the observation's content without its labels, by how the position
      was admitted:
      - a position admitted from an FR-106 document (the spine trace route,
        FR-501): that document's canonical JSON value (the semantic-value
        canonical JSON FR-106's reader admits) without its `identity`
        member. The evaluator SHALL retain that document on the position
        (`EvidencePosition`) at admission, so the value is built from the
        admitted document and never re-read or reconstructed;
      - a position carried as an FR-516 `EvidenceRef` (the timed-call and
        checked-package route, which carries no FR-106 document): the
        object `{"authority_digest": h, "contract": c, "digest": h,
        "schema_digest": h}`, `c` the `contract` text and each `h` the
        member's bytes as lowercase hexadecimal (the empty string for empty
        bytes). The reference's labels (`identity`, `authority_identity`,
        `authority_revision`, `scope`, `population`) are omitted, as the
        FR-106 labels are; the content members stay opaque, FSL hexes their
        bytes and interprets nothing;
    - `V` the position's shared `valuations`: an object keyed by the atom
      index in decimal, each `"true"`, `"false"`, `"missing"` or
      `{"undefined": U}`; `{}` when there are none. On the spine route these
      are the valuations the evaluator reads from the admitted document;
    - `G` the position's shared `aggregate_samples`, keyed the same way,
      each `"excluded"`, `"counted"`, `{"holds": true}` or
      `{"holds": false}`, `{"value": Q}` or `{"undefined": U}`; `{}` when
      there are none;
    - `IV` and `IG` the instance-bound `instance_valuations` and
      `instance_aggregate_samples` (FR-524): objects keyed by the lowercase
      hexadecimal of the trigger identity's bytes (an opaque byte string,
      FR-516, FR-053, which FSL never decodes as text), each value a `V` or
      `G` object; `{}` when there are none;
    - `Q` an exact rational in FR-038's object shape and normalization,
      `{"kind": "rational", "numerator": "<n>", "denominator": "<n>"}` in
      lowest terms with a positive denominator, whose components are
      unbounded: FR-038's signed 64-bit domain does not apply to this value;
    - `U` the undefined cause, in this requirement's own spelling, one of:
      - `{"cause": "kernel", "reason": R}`, a kernel `Undefined`, `R` its
        FR-100 kebab-case kernel reason (the kernel cause has no other
        member);
      - `{"cause": "family", "fields": F, "reason": R}`, a family
        `UndefinedRecord`, `R` the catalog's `UndefinedReason` spelling
        (FR-100) and `F` the record's `fields` as an object of strings keyed
        by the catalog's field names (`{}` when it has none);
      - `{"cause": "insufficient-data", "count": "<n>", "min_count": "<n>"}`,
        a window aggregate's `InsufficientData`, with the window's value
        count and the term's `min_count`;
    - `K` `{"authority": A, "key": k}` or `null`;
    - `I` the interval stamp `{"earliest": "<n>", "earliest_open": B,
      "latest": "<n>", "latest_open": B}` or `null`.

    The value holds every fact of the position that the record's other
    fields do not, except its labels (the FR-106 labels, or an
    `EvidenceRef`'s labels), so two positions share a record identity
    exactly when they state one fact, on either route, and a superseding
    request that changes a valuation, an aggregate sample, an evidence
    content member, an order key or an interval changes that record's
    identity;
  - `visibility`: `Internal`, because a position is a snapshot of the
    assessed system's own state at its own anchor (FR-106);
  - `anchor`: `EventPosition(t)` for an event position clock,
    `FixedSample { index: t, epoch_nanos: epoch × s, period_nanos: period × s }`
    for a fixed-sample clock (FR-522's epoch and period), and
    `TimestampNanos(t × s)` for a timestamped clock, `t` the nominal stamp;
  - `event_time_nanos`: `t × s`;
  - `ingestion_time_nanos`: the arrival watermark `× s` when the position
    carries one (FR-966); for a position without one, the interim rule of
    "Owner requirement: an unrecorded ingestion instant" below;
  - `clock_identity` `C`, `clock_revision` `P`;
  - `clock_uncertainty_nanos`: the cover's `u × s` (`0` under `Exact`); an
    interval stamp is carried in the value and compared through the
    quarter-tick intervals below, never through this field;
  - `identity`: `authority::observation::record_identity(&record,
    Limits::owner_max())` over the record above.
- Two or more positions that state one fact (equal observation content,
  valuations and aggregate samples at one stamp with equal order keys,
  interval stamps and arrival watermarks, which FR-501 and FR-516 admit and
  order by observation identity) SHALL map to one owner record and one
  member. The side mapping is then many-to-one: that record maps to every
  FR-106 identity of those positions, and every owner answer about the record
  is reported under each of them. No such trace is refused.
- `scope` SHALL be the `ScopeSelection`:
  - `required_member_identities`: the record identities, in ascending byte
    order. A member's object identity is its record's identity, an opaque
    content key the owner itself derived, so it is stable across requests
    for one fact and carries no FR-106 identity;
  - `members`: one `Member { object_identity: id, record_identity: id,
    anchor }` per record, in the same order;
  - `membership_rule_identity`, `membership_digest` and
    `membership_document`: the identity, digest and bytes
    `authority::population::derive_membership(&required_member_identities,
    Limits::owner_max())` returns;
  - `closure_identity` and `closure_digest`: that membership rule identity
    and membership digest. The finite trace is closed by enumeration; the
    explicit-members document is its closure definition. Whether a decision
    scope is closed or open is a separate assertion (FR-501's scope closure,
    the activation `OpenClosed` authority below), not this definition;
  - `kind`: `ScopeKind::Snapshot { snapshot_identity }` with the membership
    rule identity, one immutable snapshot of exactly those members. A
    `Window` is not used: the owner admits a window only over a timestamped
    range, and the trace is the same immutable snapshot under every clock
    family;
  - `range`: the half-open range over the anchors, `EventPosition { start:
    min t, end_exclusive: max t + 1 }`, `FixedSample { start: min t,
    end_exclusive: max t + 1, epoch_nanos, period_nanos }` or `Timestamp {
    start_nanos: min t × s, end_nanos: max t × s + 1 }`; a trace with no
    position is refused before admission (FR-501, FR-966), so the range is
    never empty;
  - `observation_sources`, `completeness_dependencies` and
    `progress_dependencies`: each `[C]`. The trace's one source carries its
    progress watermark and its completeness assertion (FR-501), and the owner
    requires exactly the binding's source;
  - `clock_identity` `C`, `clock_revision` `P`;
  - `membership_complete`: `true`, because the request carries every on-time
    position of the trace (a late observation takes no position, FR-966);
  - `population_identity`: `authority::population::population_identity(&request,
    Limits::owner_max())` over the request with every other member set.
- `limits` SHALL be `ResourceLimits { max_records: n, max_members: n,
  max_relationships: 0, max_required_relationships: 0 }`, `n` the number of
  distinct records: the evaluator's own ceilings already bound the trace, and these
  exact counts never refuse a request built as above.

### Partial values and interval order

- Every cover SHALL map to a quarter-tick `EventTimeInterval::new(clock,
  profile, unit, earliest, latest)`, the unit the owner-side label `<u>/4`
  of the binding and its records ("The admission request"), each closed endpoint `c` as `4c`, an open lower endpoint
  `a` (FR-966 `earliest_open`) as `4a+1` and an open upper endpoint `b`
  (FR-966 `latest_open`) as `4b-1`: a reading `k` under
  `ClockCover::Counter { uncertainty: u }` maps to `[4(k-u), 4(k+1+u) - 1]`,
  under `ClockCover::Exact` to `[4k, 4k]`, a point stamp `t` under
  `ClockCover::Bounded { uncertainty: u }` to `[4(t-u), 4(t+u)]`, and an
  observation's interval stamp, which overrides the cover (FR-501, FR-966),
  to its own endpoints mapped by their flags. These intervals are compared
  with each other and never written into an owner document.
- Every bound `b` SHALL scale to `4b`. Every scaled endpoint, scaled bound and
  shifted endpoint SHALL be computed exactly in `i128` from the `u64` stamps
  and uncertainties and the 64-bit bounds; bounded below `2^68` in
  magnitude, no such value overflows.
- `definitely_before(a, b)` SHALL be `relation_to(a, b).definitely_before()`.
- `within_bound(b, request, ack)` SHALL be read from `relation_to(ack,
  request shifted by 4b)`: `True` when the relation admits no `after`, `False`
  when it admits neither `before` nor `equal`, and `Indeterminate` otherwise.
- Eligibility (FR-501 "Positions, anchor and eligible instants") SHALL read
  the scaled difference of the candidate and the anchor, `[lo, hi]` with
  `lo` the candidate's scaled earliest minus the anchor's scaled latest and
  `hi` the candidate's scaled latest minus the anchor's scaled earliest
  (anchor and candidate swapped for a backward bound, and `[0, 0]` for a
  position's distance to itself), and SHALL decide each one-sided bound by
  its endpoints alone:
  - `at_least b`: eligible when `lo >= 4b`, ineligible when `hi < 4b`, and
    otherwise `Indeterminate` with reason `StraddlesBound`;
  - `at_most b`: eligible when `hi <= 4b`, ineligible when `lo > 4b`, and
    otherwise `Indeterminate` with reason `StraddlesBound`;
  - a window `[a, b]` is `at_least a` and `at_most b`, eligible when both
    are, ineligible when either is, and otherwise `Indeterminate`.

  No unbounded owner interval is formed for a one-sided bound: the
  comparison reads the two endpoints against `4b` and nothing else.
- A position's Kleene value SHALL map to the owner's `PossibilitySet`: `true`
  to `{can_be_true}` and `false` to `{can_be_false}`, reason `Known`;
  `indeterminate` for missing history (FR-501 `MissingHistory`) to both,
  reason `Missing`, and for a contradicted input to both, reason
  `Conflicting`. A partial fact SHALL be derived with `partial::derive` over
  the position's record and read back with `partial::read`.
- A position whose atom value is undefined (an FR-327 cause: a kernel or
  family `Undefined`, or a window aggregate's `InsufficientData`) SHALL get
  no partial fact for that atom. The owner's `ValueReason` is closed
  (`Known`, `Missing`, `Conflicting`) and a `PossibilitySet` answers which
  Booleans remain possible; an undefined value is neither, so mapping it to
  `Missing` or `Conflicting` would claim absent or conflicting evidence the
  trace does not hold. The undefined fact reaches the owner only inside the
  record's value (`{"undefined": U}`, "The admission request"), and the
  evaluator SHALL report the undefined outcome from its own evaluation
  (FR-327, FR-501), unchanged. No owner read SHALL consume that atom's
  `PossibilitySet`: the clause outcome is the evaluator's FR-327
  `Undefined` or `InsufficientData`, and FR-944's eligibility stays the
  interval comparison above, unchanged by the undefined value.

### Completeness

- A retained `Complete { through: T }` SHALL map to a `completeness::Selection`
  whose required members are the positions stamped at or before `T`, each
  `Available` with its record, at the boundary `T` in the clock family's
  `TemporalBoundary` (watermark `T`); `Incomplete` maps to the same selection
  with the members not yet supplied `Incomplete`.
- Accepting, revising or refusing an assertion against a retained one SHALL be
  the owner's completeness revision rule below, preceded by the relabelling
  check that follows, and FR-502's `Completeness` contradiction SHALL be
  reported exactly when that check or that rule refuses, and on no other
  path.
- Relabelling check. A record's value omits its FR-106 labels ("The
  admission request"), so a position whose content, stamp, order key,
  interval and arrival equal a covered position's but whose FR-106 identity
  is new has the covered position's owner record identity, and the owner
  rule alone would hold it covered. FR-502 decides coverage by FR-106
  identity. Before calling `revise`, the evaluator SHALL therefore read each
  position stamped at or before the retained `through` through the side
  mapping and SHALL refuse, with FR-502's refusal, any whose FR-106
  identity the binding's covered set does not hold: a contradiction on
  `Completeness`, `invalid_runtime_input`/`invalid-value`, naming the
  binding's clock name, the retained assertion and the position, the ledger
  unchanged. `revise` is not called for a request this check refuses, and a
  position this check passes reaches `revise` exactly as before, so the owner
  rule and FR-502 agree on every request that reaches it.

### Activation

- An activation scope SHALL map to an `activation::Selection`: the trigger
  delivery or origin as `ActivationSelection::new` (or `without_trigger` for a
  scope with none), the scope's closure as the closure authority
  (`OpenClosed`), the trigger record's completeness as the evidence authority,
  and the FR-524 captures as `Capture`s with their source observation
  identities.
- FR-524's disposition SHALL be read from the view: `Active`, `Inactive` and
  `Unknown` from `activation()`. The `Unknown` kind SHALL be read from the
  view's axes and the evaluator's own admission outcome: `Missing` when the
  trigger evidence is absent from the admitted history, `Refused` when the
  trigger record was refused at the evaluator's admission (FR-106), and `Open`
  when `closure()` is `Open` or `evidence()` is `Incomplete`; its completeness
  is `evidence()` and its execution the request's execution disposition.

### Repair

- A superseding or invalidating request (FR-526) SHALL map to
  `repair::plan(prior, successor, selection, limits)`: `prior` and `successor`
  the bundle views of the two requests' owner documents, the selection's
  authority revision the prior bundle's identity, one `FactRegion` per owner
  fact with the region of its positions, one `PriorResult` per retained result
  with its region and its canonical bytes, and one `DependencyEdge` from each
  fact to each result whose evaluation read it.
- The results FR-526 reports as superseded SHALL be exactly those the plan
  invalidates, and every retained result SHALL keep its bytes.

### Owner requirement: completeness revision

The owner's `completeness` module derives one assessment from per-member
facts and has no rule that compares a new assertion with a retained one.
FR-502 needs it, so `filament-observation` SHALL provide it, as its own
requirement (filament-observation FR-013, "Revise a completeness assertion
monotonically across revisions", the normative source of this rule; tracked as
QSL-2327). Where the summary below and FR-013 differ, FR-013 governs:

- `completeness::Selection` SHALL carry a `TemporalBoundary`, and
  `completeness::revise(prior: &View, context, selection, limits)` SHALL derive
  a successor document whose context names `prior`'s document as predecessor.
- `revise` SHALL waive the owner's same-subject predecessor check: each
  request carries its own admitted history, so the successor's scope and
  population identity are its own (filament-observation FR-013).
- `revise` SHALL refuse with a typed error naming the contradicted member or
  boundary when the successor's boundary is below the prior's, when a prior
  `Complete` assessment is succeeded by an `Incomplete` one, or when the
  successor holds a member at or before the prior boundary that the prior did
  not hold, or when a member both hold `Available` carries a different
  observation identity in the successor (`MemberObservationChanged`, which
  backs FR-502's changed-fingerprint refusal); it SHALL accept a boundary at or above the prior's and a member
  absent from the successor that the prior held, which stays held.
- `revise` SHALL leave `prior` unchanged on refusal.

Until QSL-2327 lands, `fsl-eval` keeps only the `Complete { through }`
revision rule, in the one function FR-861's tracked exception names, pinned
to the rule text above by FR-861-AC-4's agreement test; everything else uses
the owner.

### Owner requirement: an unrecorded ingestion instant

The owner's `AdmittedRecord` requires an `ingestion_time_nanos` and keeps it
independent of the event time (filament-observation FR-001). An FSL position
has an ingestion instant only when it carries an arrival watermark (FR-966);
a position without one was never stamped on arrival, so no instant the
evaluator can write there (its event time, the request's watermark) is an
ingestion instant the trace holds. `filament-observation` SHALL therefore provide,
as its own requirement amending its FR-001, FR-004 and FR-005 (filed against
the owner as a requirement, as FR-013 was):

- `AdmittedRecord.ingestion_time_nanos` SHALL be `Option<i128>`, `None`
  meaning the ingestion instant was not recorded, never an instant;
- the owner SHALL edit in place, and spell an absent ingestion instant as
  the member present with JSON value `null` in, each of these artifacts:
  - `filament.observation.record/v2` (`schemas/observation-record-v2.schema.json`
    and the record document), member `ingestion_time_nanos`;
  - `filament.observation.record-candidate/v1`, the FR-014 event-input record
    line, member `ingestion_time_nanos`;
  - `filament.observation.partial-fact/v1`, whose only ingestion member is
    `ingestion_position`;
  - the `filament.observation.identity/v2` record-identity preimage
    (`authority::observation::record_identity`), member
    `ingestion_time_nanos`;
- the contract and preimage labels SHALL stay as they are, with no v2 or v3
  contract, no new identity version and no compatibility layer: the owner is
  pre-release, and this amends filament-observation FR-005 "Owner artifact
  versioning", whose rule of a new version for a changed member set does not
  apply to this change;
- `partial::Selection`'s ingestion position SHALL take the same `Option`, and
  no owner rule SHALL order, compare or classify by an absent ingestion
  instant.

Interim rule, in force only until that owner requirement lands (tracked as
QSL-2524): a record without an arrival watermark SHALL carry
`ingestion_time_nanos` equal to its `event_time_nanos`, and its
`partial::Selection` ingestion position the same value. This is a
placeholder that fills a member the owner does not yet let the evaluator
leave empty; the evaluator does not assert it as the time the observation
arrived, SHALL NOT report it, and SHALL read no owner answer that depends on
a record's ingestion instant. When the owner requirement lands, the record
SHALL carry `None` and this interim rule is removed.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-944-AC-1 | For readings 0 to 6 under `Counter { uncertainty: 1 }` and under `Exact`, with bounds 0 to 6, `definitely_before` and `within_bound` read through the owner's `relation_to` on the quarter-tick intervals (reading 3 maps to `[8, 19]` under `Counter { uncertainty: 1 }` and to `[12, 12]` under `Exact`, unit `ms/4`, and bound `b` shifts the request by `4b`) equal `ClockCover`'s answers on every pair. | Test (TC-2624) |
| FR-944-AC-2 | FR-501-AC-13's counter-cover trace admits through `filament_observation::admit` with one record per position whose identity is the owner's `record_identity`, and every report names the position's FR-106 identity through the side mapping; a `ms` stamp 3 admits as `event_time_nanos` 3000000, and stamp 3 under a declared unit of factor `rational(1, 2000000000)` (1/2 ns) refuses at stage `admit` naming its position; and the partial facts of its atoms read back through `partial::read` with the possibility sets and reasons of their Kleene values. | Test (TC-2624) |
| FR-944-AC-3 | FR-524-AC-27 to AC-29's scopes give `Active`, `Inactive` and the three `Unknown` kinds through `activation::derive` and the mapping above, equal to the dispositions the evaluator reports; FR-526-AC-2's superseding request gives a repair plan that invalidates exactly the superseded result and retains the other's bytes. | Test (TC-2624) |
| FR-944-AC-4 | Once filament-observation FR-013 lands, FR-502-AC-2's cases through `completeness::revise` refuse `Complete { through: 60 }`, `Incomplete` and the uncovered position stamped 80, refuse a covered position re-delivered under a different observation identity (`MemberObservationChanged`, FR-502's changed-fingerprint case), accept a successor over its own admitted history with another subject, and accept `Complete { through: 100 }` and `{ through: 140 }`, exactly as FR-502's ledger does. A position whose content, stamp, order key, interval and arrival equal a covered position's at or before the retained `through` but whose FR-106 identity is new is refused by the relabelling check before `revise` is called, with FR-502's `Completeness` contradiction (`invalid_runtime_input`/`invalid-value`, the clock name, the retained assertion and the position) and the ledger unchanged, although its owner record identity is the covered one; the same position under its covered FR-106 identity passes the check and `revise` accepts it. | Test (TC-2624, TC-3036) |
| FR-944-AC-5 | Under `Bounded { uncertainty: 0 }` and clock unit `ms` (owner unit `ms/4`), a request `holds(req)` stamped 1 with interval `(0, 1]` maps to `[1, 4]` and an `ack` stamped 3 with interval `[3, 4)` maps to `[12, 15]`; `within_bound(3, req, ack)` read through `relation_to(ack, req shifted by 12)` is `Indeterminate`, and `holds(req) implies eventually[0,3] holds(ack)` with `W` and `Complete { through: 10 }` gives `Indeterminate { reason: StraddlesBound }`, equal to `ClockCover`'s answer, where the half-tick mapping (`[1, 2]`, `[6, 7]`, shift 6) would read `True`. The same request with the `ack` stamped 2 with interval `[2, 3)`, mapped to `[8, 11]`, gives `Completed(true)`. | Test (TC-2624) |
| FR-944-AC-6 | With an anchor of interval `[0, 0]` (`[0, 0]` in quarter ticks) under `Counter { uncertainty: 0 }`: for `at_least 2`, reading 2 (`[8, 11]`) is eligible, reading 1 (`[4, 7]`) ineligible, and reading 2 under `Counter { uncertainty: 1 }` (`[4, 15]`) `Indeterminate` with reason `StraddlesBound`; for `at_most 2`, reading 1 is eligible, reading 3 (`[12, 15]`) ineligible and reading 2 `Indeterminate` with reason `StraddlesBound`; a candidate of interval `(1, 2]` (`[5, 8]`) is eligible for `at_least 1`, one of interval `[0, 1)` (`[0, 3]`) is ineligible for `at_least 1` and `Indeterminate` for `at_most 0`; and against an anchor of interval `(0, 1]` (`[1, 4]`) a candidate of interval `[3, 4)` (`[12, 15]`) is `Indeterminate` for `at_most 3` and eligible for `at_least 2`. Each decision equals `ClockCover`'s eligibility, and no owner interval with an unbounded end is formed. | Test (TC-2624) |
| FR-944-AC-8 | FR-501-AC-13's counter-cover request under `ms` with a position carrying arrival 2, one carrying an order key and one an interval stamp builds an admission request that `admit` returns `Available` for, in which: `package` is `native-linked-package/1`, the package `name`, revision `filament.checked-package/v2` and the SHA-256 of `canonical_bytes()`; `producer` is the `fsl-eval` bundle; `binding` has the stated identity text, source `C`, schema `fsl-eval/trace-position/1`, the `over` node's `node_wire` text as signal, the clause node's text plus `/activation` as trigger, unit `ms/4` (also every record's unit and every interval's unit), subject kind `package` and `required` true; `expected_subject` is kind `package` with the package identity; both relationship lists are empty; each record's identity equals `record_identity` of the record, its value text is the stated seven-member `{"aggregate_samples", "instance_aggregate_samples", "instance_valuations", "interval", "observation", "order_key", "valuations"}` object whose `observation` is the admitted document without its `identity` member and whose `valuations` are the valuations read from it, so no owner document carries an FR-106 identity, visibility `Internal`, anchor `TimestampNanos(t × 1000000)`, ingestion `2000000` for the arrival-2 position, uncertainty `1000000`; `required_member_identities` is the sorted record identities and equals the members' object and record identities; membership, closure and snapshot identity are `derive_membership`'s identity and digest; the range is `[min, max + 1)` in nanoseconds; sources and both dependency lists are `[C]`; `membership_complete` is true; `population_identity` equals `population_identity` of the request; limits are the record count and 0. Building it twice gives equal requests; changing one valuation, one order key or one interval changes exactly that record's identity. Adding a second observation equal to the first in content, stamp, order key, interval and arrival but with another FR-106 identity admits: the request has one record and one member for both, and every report about that record names both FR-106 identities through the many-to-one side mapping. A position without an arrival carries `ingestion_time_nanos` equal to its event time under the interim rule, and no report carries that value. An event-position clause's request uses `EventPosition` anchors, owner unit `position/4` and an `EventPosition` range and is also `Available`. | Test (TC-3036) |
| FR-944-AC-9 | The `fsl-eval` bundle has identity `fsl-eval`, revision `Revision::producer` of the crate version, interface `1.2.0`, no relationships, configuration `fsl-eval/default` whose digest equals `canonical_digest_selection()` of a bundle holding only `{"configurationIdentity": "fsl-eval/default"}`, and digest equal to its own `canonical_digest_selection()`; it is admitted once per process. An admission request mutated after derivation (the binding's source changed, the population identity changed, one record identity changed), each alone, ends the run `runtime_invariant`/`established-invariant-broken`, invariant `observation-admission`, carrying the owner's refusal cause, exit `30`, and no clause outcome; none is `invalid_runtime_input`. A trace of `max_population_entries + 1` positions refuses at stage `admit`, `stage_limit_exceeded`/`node-count-exceeded`, `bound` the owner maximum, `actual` the count, `setting` `filament_observation::authority::Limits.max_population_entries`, and a position whose value text is one byte over `max_string_bytes` refuses `stage_limit_exceeded`/`input-bytes-exceeded` with `setting` `filament_observation::authority::Limits.max_string_bytes`, both before any owner derivation. | Test (TC-3036) |
| FR-944-AC-10 | One compiled package evaluated on the source route and, after a canonical emit, read and evaluated on the bare-package route gives `canonical_bytes()` equal to the emitted bytes on both routes and to the bytes the reader consumed, exactly, and so equal admission `package` selections (revision `filament.checked-package/v2`, equal digest) and equal owner `Context` authorities on both routes; the same package as non-canonical JSON (reordered members, extra whitespace) is refused by the reader `noncanonical_wire` (FSpec FR-322-AC-4) and admits nothing; no document or file FSL writes carries the digest. | Test (TC-3036) |
| FR-944-AC-11 | Over FR-516-AC-9's request through `evaluate_temporal_evidence` (positions carried as `EvidenceRef`s with arbitrary, including empty, `contract`, `schema_digest`, `digest` and `authority_digest`), with one position carrying a `Value` aggregate sample of 1/3, one an `Undefined` valuation of kernel reason division by zero, one an `Undefined` aggregate sample of `InsufficientData { count: 1, min_count: 2 }`, and one an instance-bound valuation under a trigger whose identity bytes are `0x00 0xff`: each record's value is the RFC 8785 text of `{"aggregate_samples", "instance_aggregate_samples", "instance_valuations", "interval", "observation", "order_key", "valuations"}` whose `observation` is `{"authority_digest", "contract", "digest", "schema_digest"}` with the bytes in lowercase hexadecimal (`""` for empty) and no `identity`, `authority_identity`, `authority_revision`, `scope` or `population`; the sample is `{"value": {"kind": "rational", "numerator": "1", "denominator": "3"}}`, the valuation `{"undefined": {"cause": "kernel", "reason": "division-by-zero"}}`, the sample `{"undefined": {"cause": "insufficient-data", "count": "1", "min_count": "2"}}`, and the instance valuation keyed by `"00ff"`; a sample of 18446744073709551616/3 writes those unbounded components; `admit` returns `Available`. Changing one content member, one valuation, one aggregate sample or one instance-bound valuation, each alone, changes exactly that record's identity, and changing only an `EvidenceRef` label does not; two positions equal in all but their `EvidenceRef` identity map to one record whose reports name both identities, and the relabelling check refuses such a position at or before a retained `through` whose identity the covered set does not hold. Over FR-501-AC-13's spine request the position keeps its admitted FR-106 document and `observation` is that document's canonical JSON without `identity`, with `valuations` the valuations read from it. | Test (TC-3036) |
| FR-944-AC-12 | Over FR-327-AC-5's closed `Counter` trace (`c.value` 0, 1, 2) and `always[0,2] holds(3 mod (2 - c.value) >= 0)` `on origin`, the atom `p` is undefined at position 2: that position's record value carries `p`'s valuation `{"undefined": {"cause": "kernel", "reason": "division-by-zero"}}`, no partial fact is derived for `p` at that position, the evaluator reports its own undefined outcome (FR-327) at that position unchanged, and no owner answer for it carries reason `Missing` or `Conflicting` or is `Indeterminate`, and no owner read consumes `p`'s possibility set there: the clause outcome is FR-327-AC-5's `Undefined{where: 2, cause: division-by-zero}`. Positions 0 and 1 have their partial facts derived and read back as AC-2 states. | Test (TC-3036) |
| FR-944-AC-13 | FR-501-AC-13's request under clock unit `ms` admits with binding `unit` `ms/4`, every record `unit` `ms/4` and every quarter-tick `EventTimeInterval` unit `ms/4`, and `partial::derive` succeeds for every atom of every position; every FSL report names the clock unit `ms`. An event-position clause's request uses `position/4` for all three owner-side labels and derives every partial fact. | Test (TC-3036) |
| FR-944-AC-7 | Scaling is exact at the domain's edge: a reading `18446744073709551615` under `Counter { uncertainty: 18446744073709551615 }` maps to `[0, 147573952589676412923]`, reading 0 under the same cover to `[-73786976294838206460, 73786976294838206463]`, and the bounds `9223372036854775807` and `-9223372036854775808` scale to `36893488147419103228` and `-36893488147419103232`; `within_bound` and `definitely_before` over these readings and bounds equal `ClockCover`'s answers. | Test (TC-2624) |

## Dependencies

- FR-861: one owner for the four rule sets.
- FR-501, FR-502, FR-524, FR-526: the reports read from the owner's answers.
- FR-106, FR-522, FR-966: the trace facts each record carries.
- FR-264, FR-531: the canonical package bytes and revision on both routes.
- filament-observation FR-001 (admission), FR-004 (owner derivations and
  maxima), FR-005 (producer bundle), FR-007, FR-008, FR-010, and the owner
  requirements above (filament-observation FR-013, and the unrecorded
  ingestion instant).
