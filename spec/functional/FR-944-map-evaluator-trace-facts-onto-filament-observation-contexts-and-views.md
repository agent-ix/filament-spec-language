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
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-993
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
  - target: ix://agent-ix/filament-observation/FR-008
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: references
---
# FR-944: Map the evaluator's trace facts onto filament-observation's contexts, selections and views

## Description

FR-861 makes `filament-observation` the one owner of the partial-value,
completeness, activation and repair rules, and `fsl-eval` applies the first
three. The owner's rules take owner values: a `Context` over an admitted
`History`, a module `Selection`, and the validated `View` of a derived
document; the completeness revision check takes instead the plain values a
progress ledger entry retains. This requirement states how the evaluator
builds each of them from the facts it holds (a trace's positions, stamps
and cover, a progress assertion and the ledger entry it revises, an
activation scope) and how it reads the owner's answer back into the reports
FR-501, FR-502 and FR-524 state. It
also states, as requirements on the owner, the completeness revision rule
FR-502 needs and an unrecorded ingestion instant.

### Decision: one admitted trace history per request, built from the trace

Every owner module except the interval relation reads a `Context`, and a
`Context` needs a `History` of a `QualifiedObservation` that the owner's
`admit` produced. The evaluator therefore admits the request's trace through
`filament_observation::admit` once, before any owner rule runs, and every
owner call of that request except activation (next decision) shares that history. Building the history from the
trace the evaluator already admitted (FR-106) keeps one admission authority
for the trace's identities and stamps, so every position in an owner document
maps back to the identity FR-502's covered set records.

### Decision: a clause's activation is its own optional history

The owner classifies activation over a history whose records are the
trigger's own observations (filament-observation FR-008): a scope is
`Inactive` only when its binding is optional and its history holds no
observation of the trigger, and it refuses a required binding with no trigger
and a claimed absence that the history contradicts. The trace positions are
not activation observations; they are the states the clause is evaluated
over, and every one of them is required. The evaluator therefore admits activation
histories beside the trace, under an optional binding: one per instance,
holding exactly its instance-creating delivery or origin (FR-524) and its
captures, and one empty history for a trigger scope with no instance. A scope with no
instance then has an empty activation history, and the owner's own rule
gives `Inactive` or `Unknown` for it unchanged; the trace request keeps its
required binding. Every activation and capture
judgement stays with the owner: the evaluator selects which facts are
activation facts, as FR-524 defines them, and interprets none of the owner's
rules.

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

## Outputs

The owner documents and views below, and FR-501's, FR-502's and FR-524's
reports, read from them.

## Behavior

### History and context

- Before any owner call, the evaluator SHALL admit the request's trace with
  `filament_observation::admit` as one `AdmissionRequest` whose every member
  is derived as "The admission request" below states, and SHALL continue only
  on `AdmissionOutcome::Available`.
- The evaluator SHALL keep a side mapping from each record's owner-derived
  identity to its position's FR-106 identity, built at admission, and SHALL
  report every owner answer (FR-501, FR-502, FR-524) under the FR-106
  identities it maps to. An owner record identity never stands in for an
  FR-106 identity in a report, and no owner document carries an FR-106
  identity: a record's value holds its document without the document's
  labels. The mapping is many-to-one where several positions state one fact
  (below). On the FR-516 route a position's FR-106 identity is its
  `EvidenceRef` identity (the observation identity FR-516 orders and reports
  by), and the record's value likewise omits the reference's labels, so
  everything this requirement says of FR-106 identities, the completeness
  holding's key included, applies to it unchanged.
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
  and `predecessor` none: the ledger retains no owner document, and each
  request derives over its own admitted history.

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

- The producer bundle SHALL be caller-owned: the caller of `fsl-eval` builds
  it with `fsl-eval`'s bundle constructor and admits it once through `StaticProducerBundle::admit` (the owner's
  validated value type), and passes the admitted `AdmittedStaticBundle` by
  reference to each run. The owner's `AdmissionRequest.producer` is an owned
  `AdmittedStaticBundle`, so `fsl-eval` clones the passed bundle into each
  admission request it builds. `fsl-eval` SHALL hold no
  static cell, lazy global, lock or cache of `StaticProducerBundle` or
  `AdmittedStaticBundle`, with no exception,
  so the qualified core keeps FR-284's rule against global state; a host
  MAY keep its admitted bundle across runs. The constructor fixes the
  bundle's content from the crate version:
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
    trigger identity. The trace binding's records are trace positions, never
    activation observations, and no activation derivation reads the trace
    request's history; activation is admitted through the activation
    request below;
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

The progress ledger (FR-502) decides every completeness revision with the
owner's history-free revision check, `completeness::check_revision`
(filament-observation FR-013, "Owner requirement: completeness revision"
below). The check reads only values a ledger entry already retains, its
completeness assertion and its covered set with fingerprints (FR-502,
FR-993), so the ledger keeps no owner `View`, `Document` or history, its
carried-state bytes and `filament.progress-ledger/1` file form hold only
those values, and `evaluate_temporal_evidence` (FR-516), the spine's batch
run (FR-330) and carried evaluation (FR-820) give one answer from one
function over one data source. The cost of a request stays one covered-set
lookup per position it lists.

- A ledger request SHALL map onto `check_revision`'s arguments as follows,
  where the retained assertion is the binding's entry in the request's input
  ledger and the asserted one is the request's:
  - **Prior upper bound.** A retained `Complete { through: T }` maps to
    `Some(T)`; a retained `Incomplete` maps to `None`, which states no
    boundary, so the check places no listed position at or before it. A
    binding with no entry has no prior and the check is not called.
  - **Prior state.** `Complete` or `Incomplete`, as retained.
  - **Successor upper bound.** An asserted `Complete { through: T' }` maps
    to `Some(T')`; an asserted `Incomplete` maps to the prior upper bound, so
    `Incomplete` after `Complete { through: T }` is a successor of
    `Incomplete` state at an equal boundary.
  - **Successor state.** `Complete` or `Incomplete`, as asserted.
  - **Retained holding.** The check looks a listed position up in the
    entry's retained covered set by the position's identity alone: its
    FR-106 identity, or a record sample's index (FR-330). The stamp says
    only where the position is placed against the boundary, as FR-502 and
    FR-993 decide coverage by identity. A held position is `Available`, and
    its observation is its retained fingerprint when it has one and absent
    when it has none. In carried evaluation the holding is the covered set
    of the binding's entry in the ledger the call starts from
    (`ClauseOpen.ledger`, or the ledger a previous call of the state
    returned), never that set extended with the positions the call itself
    admits, exactly as batch evaluation reads the request's input ledger.
  - **Listed facts.** One fact per position of the request's trace, in
    trace order, each with status `Available`, placed at the position's
    stamp, and with its content fingerprint (FR-993) as its observation
    when it has one and no observation when it has none, so a fingerprinted
    position and one without a fingerprint under the same identity differ.
- The ledger SHALL report FR-502's contradiction on `Completeness` exactly
  when `check_revision` refuses, and on no other path, with the binding's
  clock name and the retained assertion, the ledger unchanged, reading the
  typed refusal as follows:
  - `BoundaryRegressed` (an asserted `through` below the retained one) and
    `CompleteRegressed` (`Incomplete` after `Complete`): the asserted value
    is the asserted completeness;
  - `UncoveredMember` (a listed position placed at or before the retained
    `through` whose identity the covered set does not hold, FR-502's
    uncovered position): the asserted value is that position;
  - `MemberObservationChanged` (a held position listed with a different
    fingerprint, FR-993's changed sample): the asserted value is that
    position without its fingerprint, since no report carries one.
- Because the holding is keyed by FR-106 identity, a position whose content
  equals a covered position's but whose FR-106 identity is new is an
  uncovered member, although the two share one owner record identity in the
  admitted trace history: coverage is decided by the identity FR-502
  records, and the owner record identity, which omits the FR-106 labels
  ("The admission request"), takes no part in it. A covered record-sample
  index re-delivered at another stamp is the same member: with the same
  content it is accepted, and the ledger keeps the one covered entry it
  holds for that identity, adding none; with changed content it is refused
  `MemberObservationChanged` (FR-993).
- A covered position that a later request omits stays held, so the same
  position re-supplied by a still later request at or before the retained
  `through` is accepted (FR-502). After a retained `Incomplete` no position
  is placed at or before a boundary, so no listed position is uncovered,
  whatever its stamp; a held record sample listed with a changed
  fingerprint is still refused, because the fingerprint comparison does not
  depend on the retained state (FR-993).
- The ledger SHALL run the watermark check first, then `check_revision`,
  then the two closure checks (a retained `Closed` decision scope asserted
  `Open`, and a retained `Closed` surrounding execution asserted `Open`).
  The watermark check and the closure checks stay the ledger's own and are
  outside FR-861's four rule sets: filament-observation FR-013 names
  watermark monotonicity the progress owner's rule, which FSL's watermark
  is, and the closures are FR-502's rules over FSL's own decision scope and
  surrounding execution, which the owner's completeness rules do not read.

### Activation

- Each clause occurrence's activation SHALL be admitted through
  `filament_observation::admit` as activation requests separate from the
  trace request: one per instance of the occurrence, and exactly one empty
  request for a trigger scope that creates no instance. One request per
  instance is required because the owner's capture environment covers the
  complete record set of its history at one anchor, and an instance's
  captures are taken once at its own delivery's anchor (FR-524). Each
  activation request is built as the trace request is, with these
  differences:
  - binding `identity`: the RFC 8785 text of the trace binding's identity
    object with the added members `"role": "activation"` and, for an
    instance's request, `"instance": X`, `X` the lowercase hexadecimal of
    the FR-524 per-instance trigger identity's bytes; the empty request has
    no `instance` member;
  - `schema_identity`: `fsl-eval/activation/1`;
  - `signal_identity` and `trigger_identity`: the trace binding's
    `trigger_identity` (the clause node's text followed by `/activation`);
  - `required`: `false`. A clause's activation may correctly never fire, and
    the owner classifies a scope as `Inactive` only over an optional
    binding;
  - `records`: for an instance's request, the instance's trigger record and
    one capture record per declared capture; the empty request has none.
    The instance-creating deliveries and the origin of an `on origin`
    clause (FR-524) are the instances; a delivery whose guard is
    `Some(false)`, and a delivery refused at admission, makes no request
    and no record. Every record carries the binding members above, the
    expected subject, visibility `Internal`, and the anchor, event time,
    ingestion time, clock and uncertainty of the position that delivered
    the trigger, built as for the trace request, so every record of one
    request shares one anchor. A trigger record's value is
    `ValueState::Present { value_type: "fsl-eval/activation-trigger/1",
    canonical_value }`, `canonical_value` the RFC 8785 text of
    `{"instance": X}`. A capture record's value is `ValueState::Present {
    value_type: "fsl-eval/capture/1", canonical_value }`, `canonical_value`
    the RFC 8785 text of `{"capture": N, "instance": X, "type": T,
    "value": W}`, `N` the capture's declared name, `T` its declared type's
    canonical text and `W` the retained capture fact (FR-524): `{"value":
    J}` for `CaptureFact::Value`, `J` the value as the semantic-value
    canonical JSON FR-106's reader admits, or `{"fault": F}` for a faulted
    capture, `F` one of `"missing"`, `"null"`, `"wrong-type"` and
    `"stale"`, so a faulted instance's captures are recorded as faithfully
    as a sound one's. Naming the instance in every value keeps two
    instances' equal captured values distinct records, as the owner
    requires of equal captures under distinct triggers;
  - `scope`: derived over these records exactly as the trace request's
    scope, with `membership_complete` `true` (the request carries every
    record the evaluator admitted for it), except that `range` is the trace
    request's range, so the empty request still has the clause's range;
  - `limits`: the trace request's rule over this request's record count.
- Every activation selection SHALL take as `obligation_identity` the trace
  binding's identity text: the obligation is the clause occurrence, and its
  instances are told apart by their trigger observations and bindings.
- Over each activation request's `Context` the evaluator SHALL derive these
  owner views and pass them to `AuthorityProofs`:
  - closure: `closure::derive` with `closure::Selection::new(C, P, [C],
    boundary, state)`, `boundary` the trace request's range as the clock
    family's `TemporalBoundary`: `lower` the range's start, `upper_inclusive`
    its last stamp (`max t`, or `max t × s` nanoseconds for a timestamped
    clock), and `carrier_end_exclusive` and `watermark` both the range's
    exclusive end (`max t + 1`, or the range's `end_nanos`), so the
    watermark lies beyond the upper bound as the owner requires of a
    `Closed` boundary and the boundary matches the admitted range; `state` the scope's FR-524 closure
    (`OpenClosed::Open` or `OpenClosed::Closed`);
  - completeness: when FR-524's trigger evidence is `Admitted` and its
    trigger record is complete, `completeness::derive` with
    `completeness::Selection::new` over the same boundary (its identity the
    boundary's RFC 8785 text) and one `Available` fact, naming its record,
    per member of the activation request's admitted population, giving `Complete` (and `Complete` over no fact for
    the empty request). When the trigger evidence is `Missing` or `Refused`,
    or the trigger record is incomplete, no completeness view is passed:
    the owner then holds the evidence `Incomplete`, which an empty history
    cannot otherwise state, since a completeness selection covers exactly
    the required members and the empty request has none;
  - capture, for an instance's request only: `capture::derive` with
    `capture::Selection::new(trigger identity, the delivery's anchor,
    bindings)`, one `capture::Binding` per record of the request (identity
    `fsl-eval/capture/<N>` for a capture record, `fsl-eval/instance` for the
    trigger record, the request's binding identity, the record's identity),
    sorted by capture identity, so the environment covers the complete
    record set;
  - no progress view: the evaluator reads no progress or silence axis from
    an activation view.
  An instance's proofs are `AuthorityProofs::new(capture, None, closure,
  completeness)`; the empty request's are
  `AuthorityProofs::without_capture(binding identity, None, closure,
  completeness)`.
- An instance SHALL map to `ActivationSelection::new(obligation, its
  request's binding identity, trigger identity, its trigger record
  identity, activation interval, captures)` over its request's `Context`,
  the captures sorted by identity and equal to the capture view's bindings:
  one `Capture` `fsl-eval/capture/<N>` per declared capture, sourced from
  its capture record, of value type `fsl-eval/capture/1` and that record's
  canonical value, and the instance capture `fsl-eval/instance`, sourced
  from the trigger record, of value type `fsl-eval/activation-trigger/1`
  and value `{"instance": X}`. Each capture's provenance is the binding's
  source `C`. The instance capture is a real capture: an instance's identity
  is fixed at activation like any captured value, and it gives an instance
  that declares no capture the nonempty capture set the owner requires of an
  admitted trigger. The owner gives `Active`.
- A trigger scope with no instance SHALL map to
  `ActivationSelection::without_trigger(obligation, the empty request's
  binding identity, trigger identity, activation interval)` over the empty
  request's `Context`. The owner's rule gives `Inactive` for a closed scope
  with a `Complete` completeness view and `Unknown` otherwise, and refuses
  none of these scopes. The evaluator SHALL NOT derive activation over the
  trace request's `Context`.
- FR-524's disposition SHALL be read from the view: `Active`, `Inactive` and
  `Unknown` from `activation()`. The `Unknown` kind SHALL be read from the
  view's axes and the evaluator's own admission outcome: `Missing` when the
  trigger evidence is absent from the admitted history, `Refused` when the
  trigger record was refused at the evaluator's admission (FR-106), and `Open`
  when `closure()` is `Open` or `evidence()` is `Incomplete`; its completeness
  is `evidence()` and its execution the request's execution disposition.

### Owner requirement: completeness revision

The owner's `completeness` module derives one assessment from per-member
facts. FR-502 needs a rule that compares a new assertion with a retained one,
and the ledger holds no owner history to compare against, so
`filament-observation` SHALL provide that rule in two forms, as its own
requirement (filament-observation FR-013, "Revise a completeness assertion
monotonically across revisions", the normative source of this rule). Where
the summary below and FR-013 differ, FR-013 governs:

- `completeness::check_revision(prior_upper: Option<i128>, prior_state:
  State, prior: &H, successor_upper: Option<i128>, successor_state: State,
  listed: &[ListedFact<'_, H::Key>]) -> Result<(), ListedContradiction>`,
  with `H: RetainedHolding`, SHALL be a pure, history-free function. The
  states are read as the caller states them. `RetainedHolding` answers, for
  a caller-chosen `Key`, the held fact as a `HeldFact` (its `FactStatus`
  and optional observation identity), whatever its status, or none. Each
  `ListedFact` carries its key, `FactStatus`, optional observation identity
  and `placed` coordinate. It derives no document and has no authority
  failure.
- It SHALL run FR-013's checks in order and return the first refusal as a
  `ListedContradiction`: the successor's upper bound below the prior's
  (`BoundaryRegressed { prior_upper, successor_upper }`); a prior `Complete`
  state succeeded by an `Incomplete` one (`CompleteRegressed { prior_upper
  }`, the prior upper bound optional); a listed `Available` or
  `Contradicted` fact placed at or before the prior upper bound whose key
  the prior does not hold (`UncoveredMember { listed }`); a listed
  `Available` fact whose key the prior holds `Available` under a different
  observation identity, a present and an absent identity differing
  (`MemberObservationChanged { listed, prior_observation }`, carrying the
  held observation identity). `listed` is the index of the listed fact. A
  `None` upper bound takes part in no boundary comparison, so no fact is
  placed at or before it. Per-fact status governs only the last two checks.
  It SHALL accept a boundary at or above the prior's, and a held key the
  successor does not list stays held.
- `completeness::Selection` SHALL carry a `TemporalBoundary`, and
  `completeness::revise(prior: &View, context, selection, limits)` SHALL
  derive a successor document whose context names `prior`'s document as
  predecessor, deciding its refusals through `check_revision` with the
  prior view as the holding, keyed by member identity, and keeping its own
  population check.
- `revise` SHALL waive the owner's same-subject predecessor check: each
  request carries its own admitted history, so the successor's scope and
  population identity are its own (filament-observation FR-013).
- `revise` SHALL carry each fact `prior` holds for a member outside the
  successor's required population into the successor with origin
  `Carried`, and SHALL leave `prior` unchanged on refusal.

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
| FR-944-AC-3 | FR-524-AC-27 to AC-29's scopes give `Active`, `Inactive` and the three `Unknown` kinds through `activation::derive` and the mapping above, equal to the dispositions the evaluator reports. | Test (TC-2624) |
| FR-944-AC-4 | Under `ms`, after `Complete { through: 100 }` is retained under binding B over positions including `p1` stamped 80, the ledger's mapping gives `check_revision` prior upper bound `Some(100)` and state `Complete`, and: `Complete { through: 60 }` is refused `BoundaryRegressed`; `Incomplete` is refused `CompleteRegressed` at successor upper bound 100; a position of a new FR-106 identity stamped 80 is refused `UncoveredMember`, and so is one whose content, stamp, order key, interval and arrival equal `p1`'s under a new FR-106 identity, although it shares `p1`'s owner record identity; `p1` stamped 80, `Complete { through: 100 }` and `Complete { through: 140 }` are accepted. Each refusal is FR-502's contradiction on `Completeness` (`invalid_runtime_input`/`invalid-value`, B's clock name, the retained assertion, and the asserted completeness or the listed position), with the ledger unchanged. A position covered by request 1, omitted by request 2 and re-supplied by request 3 at a stamp at or before the retained `through` is accepted by both later requests, and the ledger lists it as covered throughout. Each answer equals FR-502's. | Test (TC-2624, TC-3036) |
| FR-944-AC-5 | Under `Bounded { uncertainty: 0 }` and clock unit `ms` (owner unit `ms/4`), a request `holds(req)` stamped 1 with interval `(0, 1]` maps to `[1, 4]` and an `ack` stamped 3 with interval `[3, 4)` maps to `[12, 15]`; `within_bound(3, req, ack)` read through `relation_to(ack, req shifted by 12)` is `Indeterminate`, and `holds(req) implies eventually[0,3] holds(ack)` with `W` and `Complete { through: 10 }` gives `Indeterminate { reason: StraddlesBound }`, equal to `ClockCover`'s answer, where the half-tick mapping (`[1, 2]`, `[6, 7]`, shift 6) would read `True`. The same request with the `ack` stamped 2 with interval `[2, 3)`, mapped to `[8, 11]`, gives `Completed(true)`. | Test (TC-2624) |
| FR-944-AC-6 | With an anchor of interval `[0, 0]` (`[0, 0]` in quarter ticks) under `Counter { uncertainty: 0 }`: for `at_least 2`, reading 2 (`[8, 11]`) is eligible, reading 1 (`[4, 7]`) ineligible, and reading 2 under `Counter { uncertainty: 1 }` (`[4, 15]`) `Indeterminate` with reason `StraddlesBound`; for `at_most 2`, reading 1 is eligible, reading 3 (`[12, 15]`) ineligible and reading 2 `Indeterminate` with reason `StraddlesBound`; a candidate of interval `(1, 2]` (`[5, 8]`) is eligible for `at_least 1`, one of interval `[0, 1)` (`[0, 3]`) is ineligible for `at_least 1` and `Indeterminate` for `at_most 0`; and against an anchor of interval `(0, 1]` (`[1, 4]`) a candidate of interval `[3, 4)` (`[12, 15]`) is `Indeterminate` for `at_most 3` and eligible for `at_least 2`. Each decision equals `ClockCover`'s eligibility, and no owner interval with an unbounded end is formed. | Test (TC-2624) |
| FR-944-AC-8 | FR-501-AC-13's counter-cover request under `ms` with a position carrying arrival 2, one carrying an order key and one an interval stamp builds an admission request that `admit` returns `Available` for, in which: `package` is `native-linked-package/1`, the package `name`, revision `filament.checked-package/v2` and the SHA-256 of `canonical_bytes()`; `producer` is the `fsl-eval` bundle; `binding` has the stated identity text, source `C`, schema `fsl-eval/trace-position/1`, the `over` node's `node_wire` text as signal, the clause node's text plus `/activation` as trigger, unit `ms/4` (also every record's unit and every interval's unit), subject kind `package` and `required` true; `expected_subject` is kind `package` with the package identity; both relationship lists are empty; each record's identity equals `record_identity` of the record, its value text is the stated seven-member `{"aggregate_samples", "instance_aggregate_samples", "instance_valuations", "interval", "observation", "order_key", "valuations"}` object whose `observation` is the admitted document without its `identity` member and whose `valuations` are the valuations read from it, so no owner document carries an FR-106 identity, visibility `Internal`, anchor `TimestampNanos(t × 1000000)`, ingestion `2000000` for the arrival-2 position, uncertainty `1000000`; `required_member_identities` is the sorted record identities and equals the members' object and record identities; membership, closure and snapshot identity are `derive_membership`'s identity and digest; the range is `[min, max + 1)` in nanoseconds; sources and both dependency lists are `[C]`; `membership_complete` is true; `population_identity` equals `population_identity` of the request; limits are the record count and 0. Building it twice gives equal requests; changing one valuation, one order key or one interval changes exactly that record's identity. Adding a second observation equal to the first in content, stamp, order key, interval and arrival but with another FR-106 identity admits: the request has one record and one member for both, and every report about that record names both FR-106 identities through the many-to-one side mapping. A position without an arrival carries `ingestion_time_nanos` equal to its event time under the interim rule, and no report carries that value. An event-position clause's request uses `EventPosition` anchors, owner unit `position/4` and an `EventPosition` range and is also `Available`. | Test (TC-3036) |
| FR-944-AC-9 | The `fsl-eval` bundle has identity `fsl-eval`, revision `Revision::producer` of the crate version, interface `1.2.0`, no relationships, configuration `fsl-eval/default` whose digest equals `canonical_digest_selection()` of a bundle holding only `{"configurationIdentity": "fsl-eval/default"}`, and digest equal to its own `canonical_digest_selection()`; the caller admits it once and passes it by reference to every run and admission request, and every request of two runs given one admitted bundle names equal producer selections. An admission request mutated after derivation (the binding's source changed, the population identity changed, one record identity changed), each alone, ends the run `runtime_invariant`/`established-invariant-broken`, invariant `observation-admission`, carrying the owner's refusal cause, exit `30`, and no clause outcome; none is `invalid_runtime_input`. A trace of `max_population_entries + 1` positions refuses at stage `admit`, `stage_limit_exceeded`/`node-count-exceeded`, `bound` the owner maximum, `actual` the count, `setting` `filament_observation::authority::Limits.max_population_entries`, and a position whose value text is one byte over `max_string_bytes` refuses `stage_limit_exceeded`/`input-bytes-exceeded` with `setting` `filament_observation::authority::Limits.max_string_bytes`, both before any owner derivation. | Test (TC-3036) |
| FR-944-AC-10 | One compiled package evaluated on the source route and, after a canonical emit, read and evaluated on the bare-package route gives `canonical_bytes()` equal to the emitted bytes on both routes and to the bytes the reader consumed, exactly, and so equal admission `package` selections (revision `filament.checked-package/v2`, equal digest) and equal owner `Context` authorities on both routes; the same package as non-canonical JSON (reordered members, extra whitespace) is refused by the reader `noncanonical_wire` (FSpec FR-322-AC-4) and admits nothing; no document or file FSL writes carries the digest. | Test (TC-3036) |
| FR-944-AC-11 | Over FR-516-AC-9's request through `evaluate_temporal_evidence` (positions carried as `EvidenceRef`s with arbitrary, including empty, `contract`, `schema_digest`, `digest` and `authority_digest`), with one position carrying a `Value` aggregate sample of 1/3, one an `Undefined` valuation of kernel reason division by zero, one an `Undefined` aggregate sample of `InsufficientData { count: 1, min_count: 2 }`, and one an instance-bound valuation under a trigger whose identity bytes are `0x00 0xff`: each record's value is the RFC 8785 text of `{"aggregate_samples", "instance_aggregate_samples", "instance_valuations", "interval", "observation", "order_key", "valuations"}` whose `observation` is `{"authority_digest", "contract", "digest", "schema_digest"}` with the bytes in lowercase hexadecimal (`""` for empty) and no `identity`, `authority_identity`, `authority_revision`, `scope` or `population`; the sample is `{"value": {"kind": "rational", "numerator": "1", "denominator": "3"}}`, the valuation `{"undefined": {"cause": "kernel", "reason": "division-by-zero"}}`, the sample `{"undefined": {"cause": "insufficient-data", "count": "1", "min_count": "2"}}`, and the instance valuation keyed by `"00ff"`; a sample of 18446744073709551616/3 writes those unbounded components; `admit` returns `Available`. Changing one content member, one valuation, one aggregate sample or one instance-bound valuation, each alone, changes exactly that record's identity, and changing only an `EvidenceRef` label does not; two positions equal in all but their `EvidenceRef` identity map to one record whose reports name both identities, and such a position at or before a retained `through` whose identity the covered set does not hold is refused `UncoveredMember` (AC-4). Over FR-501-AC-13's spine request the position keeps its admitted FR-106 document and `observation` is that document's canonical JSON without `identity`, with `valuations` the valuations read from it. | Test (TC-3036) |
| FR-944-AC-12 | Over FR-327-AC-5's closed `Counter` trace (`c.value` 0, 1, 2) and `always[0,2] holds(3 mod (2 - c.value) >= 0)` `on origin`, the atom `p` is undefined at position 2: that position's record value carries `p`'s valuation `{"undefined": {"cause": "kernel", "reason": "division-by-zero"}}`, no partial fact is derived for `p` at that position, the evaluator reports its own undefined outcome (FR-327) at that position unchanged, and no owner answer for it carries reason `Missing` or `Conflicting` or is `Indeterminate`, and no owner read consumes `p`'s possibility set there: the clause outcome is FR-327-AC-5's `Undefined{where: 2, cause: division-by-zero}`. Positions 0 and 1 have their partial facts derived and read back as AC-2 states. | Test (TC-3036) |
| FR-944-AC-13 | FR-501-AC-13's request under clock unit `ms` admits with binding `unit` `ms/4`, every record `unit` `ms/4` and every quarter-tick `EventTimeInterval` unit `ms/4`, and `partial::derive` succeeds for every atom of every position; every FSL report names the clock unit `ms`. An event-position clause's request uses `position/4` for all three owner-side labels and derives every partial fact. | Test (TC-3036) |
| FR-944-AC-14 | For FR-524-AC-10's closed scope whose every delivery has a false guard, with `Admitted` evidence and a complete trigger record: exactly one activation request is built, with binding `required` false, schema `fsl-eval/activation/1`, the trace binding's trigger identity as signal and trigger, the trace identity text with `"role": "activation"` and no `instance` member, the trace request's range, `membership_complete` true and no record; `admit` returns `Available`; `closure::derive` gives a `Closed` view without refusal over the boundary whose `upper_inclusive` is the last stamp and whose `carrier_end_exclusive` and `watermark` are the range's exclusive end, a boundary with `watermark` equal to `upper_inclusive` being the one the owner refuses for `Closed`; the completeness view `Complete` over no fact, and `activation::derive` with `without_trigger`, obligation the trace binding's identity text and `AuthorityProofs::without_capture` gives `Inactive` and no refusal, while the trace request for the same clause keeps `required` true and one record per position. The same scope open gives `Unknown(open)`; with an incomplete trigger record, and with `Missing` and `Refused` trigger evidence, the request still admits `Available` with `membership_complete` true, no completeness view is passed, and `derive` gives `Unknown` with evidence `Incomplete`, reported as `Unknown(open)` with incomplete completeness, `Unknown(missing)` and `Unknown(refused)`, each without a refusal. A scope with one instance-creating delivery and no declared capture gives one request with one trigger record whose value is `{"instance": X}`, a capture view binding exactly that record as `fsl-eval/instance`, and an `Active` view whose captures are exactly `fsl-eval/instance` with value `{"instance": X}`; one declaring captures `a` and `b` gives one request of three records at one anchor and captures `fsl-eval/capture/a`, `fsl-eval/capture/b`, `fsl-eval/instance` in that order, the two declared captures with value text `{"capture", "instance", "type", "value"}` of the instance's FR-524 capture, and a capture faulted `Missing` gives `{"fault": "missing"}`; a delivery with guard `Some(false)` beside it makes no request. Two instances of one clause occurrence capturing equal values give two requests with distinct binding identities, each `capture::derive` and `activation::derive` succeeds and gives `Active`, and the capture record identities and the activation identities differ. No `activation::derive` call of a run is made over the trace request's `Context`. | Test (TC-2624) |
| FR-944-AC-15 | Two runs given one caller-admitted `AdmittedStaticBundle` by reference build admission requests whose `producer` members are clones equal to that bundle, one clone per request; the bundle is built by `fsl-eval`'s constructor and admitted by the caller through `StaticProducerBundle::admit`. A source scan of `fsl-eval`'s shipped source, `#[cfg(test)]` code excluded, finds no `static`, `OnceLock`, `LazyLock`, `lazy_static!` or `thread_local!` item, and no `Mutex` or `RwLock` field, whose type names `StaticProducerBundle` or `AdmittedStaticBundle`, and fails naming the file and line of any it finds (a fixture source holding `static B: OnceLock<AdmittedStaticBundle>` is reported); a run whose caller passes no bundle cannot be built (the bundle is a required parameter). | Test (TC-3036) |
| FR-944-AC-16 | For an event-position clause with a retained `Incomplete` whose covered set holds positions stamped 1 to 3, the mapping gives `check_revision` prior upper bound `None`, and a request listing a new position stamped 0, with a watermark not below the retained one, is accepted with `Incomplete` (state `Incomplete`) and with `Complete { through: 5 }`. For a fixed-sample clause over record samples with a retained `Incomplete` covering index 1 with fingerprint F, index 1 listed with fingerprint F is accepted and with another fingerprint G is refused `MemberObservationChanged`, reported as FR-502's contradiction on `Completeness` naming the position without a fingerprint (FR-993), the ledger unchanged; the same two answers hold after a retained `Complete { through: 5 }`. For a timestamped clause over record samples (a `Timestamped` record-sample trace, FR-993 Inputs, under `ms`), after a retained `Complete { through: 5 }` covering index 1 at stamp 1, index 1 re-delivered at stamp 2 with changed content is refused `MemberObservationChanged` (FR-993), and with the same content is accepted as the same member, and the returned ledger lists index 1 once, at stamp 1 with fingerprint F. A held position with fingerprint F listed with no fingerprint, and a held position with no fingerprint listed with F, are each refused `MemberObservationChanged`; a held position with no fingerprint listed with none is accepted. Batch evaluation and carried evaluation (FR-820), the carried state passed through `to_bytes` and `from_bytes` between requests, give equal answers and equal ledgers on every case of this criterion that a trace can express, and on every case of AC-4; the fingerprint present-versus-absent cases are ledger tests. | Test (TC-2624) |
| FR-944-AC-7 | Scaling is exact at the domain's edge: a reading `18446744073709551615` under `Counter { uncertainty: 18446744073709551615 }` maps to `[0, 147573952589676412923]`, reading 0 under the same cover to `[-73786976294838206460, 73786976294838206463]`, and the bounds `9223372036854775807` and `-9223372036854775808` scale to `36893488147419103228` and `-36893488147419103232`; `within_bound` and `definitely_before` over these readings and bounds equal `ClockCover`'s answers. | Test (TC-2624) |

## Dependencies

- FR-861: one owner for the four rule sets.
- FR-501, FR-502, FR-524: the reports read from the owner's answers.
- FR-820, FR-993: carried evaluation, which shares the ledger, and the
  fingerprints the completeness check compares.
- FR-106, FR-522, FR-966: the trace facts each record carries.
- FR-264, FR-531: the canonical package bytes and revision on both routes.
- filament-observation FR-001 (admission), FR-004 (owner derivations and
  maxima), FR-005 (producer bundle), FR-007, FR-008, FR-010, and the owner
  requirements above (filament-observation FR-013, and the unrecorded
  ingestion instant).
