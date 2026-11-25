---
id: FR-531
title: "Evaluate from a checked package read from its bytes alone"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-517
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-520
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-532
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-251
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-397
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-contract-ir/FR-357
    type: references
  - target: ix://agent-ix/filament-contract-ir/FR-358
    type: depends_on
  - target: ix://agent-ix/filament-protocol/FR-045
    type: references
  - target: ix://agent-ix/filament-protocol/FR-046
    type: references
---
# FR-531: Evaluate from a checked package read from its bytes alone

## Description

A consumer that reads a handoff directory (FR-517) with Contract IR's
`read_checked_package` holds a `filament_contract_model::CheckedPackageV2` and holds
no FSL source `CheckedPackage`, no `Emission` and no `AdmittedPackageRead`. Until
this requirement the FSL evaluation entries needed one of those: `EvidencePackage`
is built by `from_emission` or `from_read`, both over an `AdmittedPackageRead`,
which exists only for a package FSL itself emitted from source and which retains
that source (`AdmittedPackageRead::source`); and `CheckedPackageEvaluation` is
sealed and implemented for the in-process `CheckedPackage`, its reader-backed
evaluation view still taking the slot selection, the binder names and the source
`CheckedBody` from that retained source. FR-515 recorded an evaluator over
`formula_graph()` with no source graph as out of scope. It is in scope: the
consumer of the handoff, filament-protocol, evaluates compensation slots and temporal
evidence from the package alone, and has nothing else to pass.

This requirement states the one entry that takes a `CheckedPackageV2` and nothing
from the source side, what it admits, what it derives from the package's bytes,
how it refuses, how its outputs spell identity where the bytes carry no source
names or spans, and that it gives the same results and the same charges as the
routes that start from source. FR-532 states the temporal evidence call from this
entry; FR-533 states the protocol-slot call from it.

## Use case

filament-protocol reads `checked-package.json` of a handoff directory with
`read_checked_package`, holds the admitted package, and calls FSL to evaluate a
compensation's retry relation and to assess a temporal obligation. It builds the
evaluation value from the package it holds, plus the files beside it (the domain
package documents and the identity rows, below), and never compiles source.

## Semantic authority and boundary

FSL owns evaluation (FR-107, FR-390, FR-516); Contract IR owns the reader and the
package's admission. This requirement adds no checked-package member and no Contract
IR code. It does not re-verify what the reader admitted: a package the reader
admitted is the package, and a difference between it and some source is the
caller's concern, not a refusal here (FR-515-AC-24 states the refusals of the two
routes that do hold a source).

## Inputs

In `fsl_package` (behind `std`, no new dependency):

```rust
pub struct BarePackage<'p> { /* private; neither Copy nor Clone */ }

impl<'p> BarePackage<'p> {
    pub fn new(package: &'p CheckedPackageV2) -> Self;
    pub fn with_supply(self, supply: BareSupply<'p>, budget: &mut NativeJoinBudget)
        -> Result<Self, ExactTypeProjectionError>;
    pub fn package(&self) -> &'p CheckedPackageV2;
}

pub struct BareSupply<'p> {
    pub identities: &'p ReadIdentityRows,
    pub equality: Option<&'p ReadEqualityRows>,
}

impl ReadIdentityRows {
    pub fn read(identities_json: &[u8], package: &CheckedPackageV2)
        -> Result<Self, IdentityFileRefusal>;
}

impl ReadEqualityRows {
    pub fn read(equality_json: &[u8], package: &CheckedPackageV2)
        -> Result<Self, EqualityFileRefusal>;
}
```

- `package` is the `CheckedPackageV2` that `read_checked_package` returned. A
  caller cannot build a `BarePackage` from bytes, from a decoded graph or from a
  term; `BarePackage` has no other constructor.
- `ReadIdentityRows` is the FSL-owned read-evidence wrapper of FR-390: the selected
  declaration/field identity rows and final source-record rows paired with the
  package they were produced for. A
  consumer obtains it with `ReadIdentityRows::read(bytes, package)` from the
  `identities.json` of the handoff directory and the package (below), and by no
  other route. Only the protocol-slot call reads it (FR-533); the temporal
  evidence call reads nothing but the package (FR-532).
- `ReadEqualityRows` is the equality counterpart: the producer's equality application
  rows, equality fact rows and selected source-unit rows (IR's
  `SelectedEqualityApplicationRow`, `SelectedEqualityFactRow` and
  `SelectedSourceUnitRow`) paired with the package they were produced for. A consumer
  obtains it with `ReadEqualityRows::read(bytes, package)` from the `equality.json` of
  the handoff directory and by no other route. Only a protocol-slot call whose walk
  holds an `==` or `!=` application reads it (FR-533 "Equality slots"); `equality` is
  `None` for a supply without it.
- The dependency closure is not a supply. `BarePackage` derives the admitted
  closure from the direct dependencies the reader retained in the package, which
  the consumer passed to `read_checked_package` as read evidence; a consumer
  builds no `AdmittedPackages`, which has no public constructor. FSL reads the
  retained dependencies through the read access Contract IR gives to the packages
  a `CheckedPackageV2` holds. That access is `CheckedPackageV2::direct_dependencies()`,
  a public read-only iterator over the retained direct dependencies in
  lock-selection order; `BarePackage::dependencies()` reads the closure from it,
  each dependency once, nearest first.
- The domain package documents (FR-056's `PackageInput`) stay a caller input of
  observation admission, as for the in-process route; they are bytes the caller
  holds, not source.

`NativeJoinBudget` and `ExactTypeProjectionError` are Contract IR's (FR-358).
`fsl_eval::bare` re-exports `BarePackage`, implements `CheckedPackageEvaluation`
for it and gives `EvidencePackage` the constructor
`EvidencePackage::from_package(&'p BarePackage<'p>) -> Result<Self,
PackageMismatch>`.

## Outputs

The values of the existing calls, unchanged: `Result<ProtocolSlotEvaluation,
CallFailure>` (FR-390) and `Result<EvidenceResult, CallFailure>` (FR-516). The
entry defines no result type of its own.

## Behavior

### Requirements

- The entry SHALL take a `CheckedPackageV2` and no value of the source side.
- The entry SHALL derive every fact of the table below from the package's bytes.
- The bare route SHALL give the same result, the same refusal and the same charge
  log as the in-process route for one package and one request.
- The entry SHALL refuse a call only with a cause, dimension or reason named here
  or in FR-390, FR-515, FR-516 and FR-532.
- Every capability of the in-process route SHALL be available from the bare
  package or be listed in the inventory as unmet parity with its reason; none
  is dropped.

### What the entry derives from the bytes

| Fact | Where it is read | In-process route reads it from |
| --- | --- | --- |
| clause identity, `over`, activation, captures, requirement entries | `temporal_subject` (FR-515) over the package | the same view |
| formula operations, intervals, operand edges and `holds` atoms | `formula_graph()` | the checked formula, cross-checked to the view |
| instance-bound atoms | the checked predicate view's `binders()` (FR-515) | the same view |
| profile, and the evaluator profile it names | the lock's `profile_selections` row of the clause's `temporal_profile` law | the same row, and the source clause's profile |
| clock configuration and clock name | the `temporal_bindings` row keyed `{ declaration, role, ordinal }` | the same row, and the source clause's clock name |
| activation map | `activation_relation(package)` (FR-449 rows) | the same |
| package identity of a progress binding and of a result identity | the package's `name()` | the source identity labels |
| protocol slot, binders, binder types and levels, locations, callees | the admitted protocol nodes and `protocol_locations` rows (FR-390, FR-464) | the source `CheckedBody` and checked package |
| location of each term of a function body, for its value losses and stops | the `body_locations` rows of the function ("Function body locations in the bytes") | the checked body's node locations, which equal those rows |
| protocol subject (FR-205): the protocol clause, the binding of each static role, the `over` universe, the capacity names, and the memory model of each outermost `parallel` | the protocol clause from the admitted `protocol` nodes and `protocol_locations` rows (FR-390, FR-464); each static role's declared model or model type from its `role` node, and its binding by resolving the request's `roles` selection (FR-1033), or the binding by construction of a role on a whole model, against the package's `model_root` selections (FSpec FR-425); the `over` parameter's population universe from the package's selected-model population catalog row of that population (FR-533 "Observation admission"); the capacity names from the clause's `symbolic(<name>)` queue capacities in its protocol nodes (FSpec FR-785); each outermost `parallel` the request's `memory` member targets, and its source default, from the `parallel` control nodes of that protocol node (FR-383: the memory model when written, `Sc` when absent, FR-219) | S3's checked protocol clause (FR-218) and the source graph's model selections |
| model selections, population types and type environment of an observation | the package's `model_root` selections, its selected-model population catalog and its selected object declarations, with the caller's domain package documents (FR-533 "Observation admission") | the source graph's model selections, protocol population types and scope |
| dependency closure | the direct dependencies the package retains from its read evidence | the source package's closure |
| HP-1 model relation occurrence, body, subjects, execution bindings, operation anchors, parameter slots and types, effective pre/postconditions, subject invariants and body model-read ownership (FR-940 "Package form of an HP-1 model relation") | the claim-name occurrence and its admitted `claim`/`step_relation` node's `step_relation_body()`, the referenced model-root selections, operation anchors, parameter nodes and Boolean expression nodes; slots retain their declared levels, and model reads retain their execution and pre/post observation | the same admitted facts of its own S4 emission, with source body evaluation through its checked relation |
| HP-1 distinct subject aliases in source model declaration order | the admitted `step_relation` body's optional `subject_declaration_order` Group, joined exactly to `subjects` by alias and model-root identity (FR-940 "Declaration-order member contract"); absence refuses only selection of an HP-1 package form | the same witness of its own S4 emission, emitted from `CheckedModelRelation::aliases()` |
| clause name, capture names and capture value types | the occurrence's `temporal_clause_names` row, the captures argument's binding names and the typed result types of the capture initializer terms | the source clause's `name()` and `captures()` |
| arithmetic exactness and bounds | the lock and the nodes' types | the same, through the source |
| the package's requirement records: each record's occurrence key, capability kind and extent, its request index (its position in package order) and, for a `value-validity` record, its owner function, application occurrence, result bound, direct conversion, path condition, extent roots and object types | the `requirement_items` rows and their `claim`/`value_validity` nodes (FSpec FR-825), through Contract IR's `requirement_items()` and `value_validity_records_of()` | the same rows of its own S4 emission, through `AdmittedPackageRead` |

**Requirement records in the bytes.** S4 SHALL write every record of
`CheckedGraph::requirements()` (FR-062 "Records") as one FSpec FR-825
`requirement_items` row, in the order `RequirementMap` iterates, which is
FR-825's package order (structural node key, occurrence role, occurrence
ordinal, then capability spelling), so a row's position is the record's
FR-395 request index. Each row's `claim` is the record's occurrence key,
`kind` is `Capability::to_wire()` and `extent` is the record's `ClaimExtent`
(no domain for `Bounded`; each `DomainKey` and `DomainKind` of `Unbounded`, in
domain-key order). Each `value-validity` record SHALL also be written as a
`claim`/`value_validity` node whose `owner` is the function node of the
record's owner, `application` and `occurrence` its key, `result_bound` the
type node of its `ResultBound`, `conversion` its conversion context,
`path_condition` its `PathGuard`s in order, `extent_roots` its
`extent_roots()` and `objects` the declaration nodes of its references'
object types; its `generated` ordinal is its position among its owner's
records. S4 writes no record by any other route and omits none, because the
package order counts every record of every kind. A record S4 cannot write
(an owner with no function node, a key with no `source_map` occurrence, an
object type with no declaration node) is `EmitRefusal::UnlocatedOccurrence`
with no bytes, as FR-062-AC-9 refuses an unlocatable occurrence; it is never
dropped. The package route reads the records from these rows and nodes, and
the source route reads the same rows of its own emission, so both routes
hold one value per record.

**Function body locations in the bytes.** S6a records every value loss, and
locates every stop, raised at a term of a function body at the position where
the operation ran, as the `ExpressionLocation` `{ origin: Body { function,
index }, path }` the checker assigned that position (FR-090). The rule is one
rule for every term of every body and names no operation, so every loss
record the evaluator makes there is covered, among them the IEEE flags
(inexact, divide-by-zero, overflow, underflow, invalid) that FR-397's
arithmetic, `sqrt` and conversions into a float raise over `Float32` and
`Float64` (FR-531-AC-37), and every stop, such as a denied meter charge
(FR-531-AC-38). A function body here is the body of every function the
checker locates under a `Body` owner: a declared pure function or predicate,
and a clause function (FR-094: a model operation's `body` or `precondition`,
an invariant, a reward) whose package node carries a `body` binding, because
the checker locates a clause function's expression under `Body` exactly as a
declared function's (FR-531-AC-39). A function's `decreases` measure is not a
body. This holds for a loss in a function the caller called directly and for
one in a function reached through a nested call: the loss is never moved to
a call site, because the source route records it inside the callee's body
and a call made from the request has no call site in any body. The bare route
cannot recompute these locations from the admitted body: the checker locates
a node that lowering introduces (an IEEE ordering's `#ieee_operand` `let` and
its composition parts under FR-397, an implicit coercion) at the source
position it stands for, so the child-index path of a position in the lowered
body differs from the checker's location wherever lowering reshaped the tree.

S4 SHALL therefore write, for every term of every such body it emits, one
FSpec FR-322 `checked_term_data` `body_locations` row carrying the checker's
location of that term, its origin `{kind: "function_body", function, name,
index}` holding the function node and the `Body` owner's `function` and
`index`, keyed as FR-383 "Protocol term locations" keys a protocol slot's
rows, over the walk FSpec FR-322 defines on the wire (FR-383's slot walk,
with the body's root in place of the slot's root, is the same walk over the
checked tree). A term S4 cannot key to a `source_map` occurrence is
`EmitRefusal::UnlocatedOccurrence` with no bytes, never an omitted row. Each row's
location SHALL be FSpec FR-322's relative form `{origin, up, down}`: the
checker's path of the term written against the path of the term the walk
visits just before it, with the least `up`. The rows of a body therefore
hold a number of entries linear in the body, and S4 SHALL charge
`emit.output_bytes` for the canonical bytes it writes. For AC-41's two
deep-body families, the complete canonical artifact size and the charged
native strict-read and location-reconstruction work SHALL have linear upper
bounds derived from the checked input's node and edge counts, source and
identifier byte lengths, and the fixed schema and body-walk structure. The
relative entry count is one `up` entry per row plus the number of `down`
entries; it SHALL not exceed the BL-D1 and BL-D2 bounds of FSpec
FR-322-AC-510. The caller SHALL
state finite byte, work and read-storage ceilings from those bounds before
emission. The bound coefficients SHALL be independent of body depth and of
the term or binding count; neither the ceilings nor their coefficients are
derived from the completed artifact or its measured consumption. Qualification SHALL record
each artifact's actual canonical byte length, relative entry count and
charged native work against its declared bounds. Success at one size alone
is not evidence of a linear bound.

Those explicit ceilings do not change the published default. With the same
checked input and the other stage ceilings unchanged, an artifact that
exceeds the default `emit.output_bytes` SHALL refuse `OutputLimit`, naming
that setting, its configured bound and the first unpaid byte, with no
partial package. The source occurrence attached to that refusal does not
identify the wire member or source offset of the denied byte. Default-limit
refusal alone does not qualify AC-41's explicit-limit positive case. Every
S4 path that writes a package writes the carrier: `emit_checked` writes the
rows from the checked graph, and `emit_timed_models`, which re-encodes a base
package with typed timed models, SHALL carry the base package's
`checked_term_data` with every `NodeRef` it holds rewritten by the same
identity map the re-encoding applies to graph nodes, never `None`. A package
from which a producer, test or tool drops `checked_term_data` while any of its
function bodies has a term is no longer admitted: the strict reader refuses
it at `/checked_term_data/body_locations` (FSpec FR-322-AC-490).

The bare route SHALL locate each term of a function body it evaluates from
its row, and the in-process route, which reads its own emission's rows for
every other fact of this table, gives the same locations from the checked
body, so the two routes return equal `Evaluation.location` and equal
`Evaluation.losses`, record for record.

The `function` of a `Body` origin is the same text on both routes. The
checker may name a function under a check-time package it assigns to the
unit's own source model: a clause function of an operation of a source object
type is named `ix://P/<path>`, `P` the private package the checker gives the
source model (`source/<unit>`). The bytes never carry `P`: S4 writes the
row's `name` as `ix://I/<path>`, `I` the `SourceOwner` identity of the source
declarations `P` holds (FSpec FR-322 `body_locations`, FR-610 "Declaration
keys"), and keeps every other name unchanged. The bare route reports a row's
`name` as written. The source route SHALL report the same spelling: a `Body`
origin it reports in an `Evaluation.location` or a loss record names a
function under `P` as `ix://I/<path>`, by the rewrite S4 applies, and every
other function by the checker's name. The source route takes the bytes' spelling because the reverse map cannot be made from the bytes: `P` is a check-time
name that no package holds, while `I` is the identity every emitted node of
those declarations carries. The rows add no graph node and change
no node id, so the package identity `name` and every structural `NodeRef` are
unchanged by them; the package bytes do change, and a package written
without them is refused by the strict reader, never evaluated with locations
derived from the lowered tree or read from source.

Every row of the in-process column that names the source is replaced on the
bare route by the bytes column, and FR-532 and FR-533 state that the in-process
routes then read the bytes column too, so that one value exists per fact.

### What the bytes do not carry

The package holds no source text and no source spans beyond its source map (FR-528
integrity claims and `CheckedSourceMapEntry` regions), no source arena order of a
checked formula, and no clause name on the clause node itself (FR-515 reads `name()` from the
`temporal_clause_names` row). The declared name of each clause occurrence is carried by its
`temporal_clause_names` row, and the bare route SHALL use that row wherever the
evaluator reads a clause name. The outputs of the bare route therefore spell positions and keys by what
the bytes carry, and the in-process routes spell them the same way:

- a clause is its `CheckedNodeId` plus an occurrence `{ role, ordinal }`;
- an atom is its index among the distinct atom nodes, numbered by first use in
  `formula_graph()` pre-order (FR-532 "Atom
  index");
- a position and an instance are the request's own values;
- a binder is its `NodeKey`, and its declared name only where the admitted binder
  node carries one;
- a location is the one the package's own location rows give (`protocol_locations`
  for a protocol slot, `body_locations` for a function body), and none where the
  wire carries none (FR-390: FSL never invents one);
- a progress binding and a result identity use the package `name()` and the clause
  occurrence, and carry no source authority or source identity label.

### Capability inventory

Every capability of `EvidencePackage` and of `evaluate_protocol_slot` is available
from the bare package, or is listed here with the reason it needs source:

| Capability | From the bare package |
| --- | --- |
| `EvidencePackage::from_emission`, `from_read` | unchanged; they hold a source and keep the cross-check of FR-515 |
| `EvidencePackage::from_package` | new; FR-532 |
| `with_progress_capacity`, `with_max_window_values`, `rows`, `temporal_subject`, `profile`, `clock`, `clock_of`, `subject`, `evaluate_temporal_evidence` | available; the rows are the package's own |
| the construction refusals of `PackageMismatch` | `AmbiguousName` applies; `ClauseRow`, `ProfileSelections` and `Bindings` need an `Emission` and have no bare route (FR-532) |
| `evaluate_protocol_slot` and `evaluate_selected_protocol_slot` | available; FR-533 |
| `AdmittedEvaluationView::admit_protocol_observation` | available as `BarePackage::admit_protocol_observation`; FR-533 "Observation admission" |
| `call`, `call_supplied`, `call_observing_applications` (call by `QualifiedName`) | available; FR-945: the one-segment name resolves by the `declaration.qualified_name` of the package's function declarations (FSpec FR-322), and parameter and result types are read from its checked type nodes |
| `call_requirements` (the items of a called function, FR-395) | available as `BarePackage::call_requirements(function)`: the `value-validity` rows whose record's `owner` is the function node `declaration.qualified_name` resolves (FR-945), each with its row index as request index, in package order; the empty list for a function that owns none; never rebuilt from the graph |
| `evaluate` (a `CheckedExpression`) | unmet parity: its argument is a source-checked value a bytes-only consumer cannot hold; the slot call is its package form |
| `evaluate_holds`, `evaluate_model_holds`, `evaluate_holds_traced`, `evaluate_guard`, `evaluate_model_guard`, `evaluate_hyper_holds`, `evaluate_step_predicate`, `evaluate_model_relation_transitions`, `evaluate_clause`, `evaluate_clause_set`, `evaluate_value_invariants`, `check_separation`, `evaluate_frame`, `evaluate_collection_write`, `evaluate_state_predicate`, `evaluate_state_predicate_at_absent_key` | required in a package form for analysis of a selected package (FR-940 "Package forms of the source-checked evaluators"): each source form takes a source-checked clause, node or state clause, which analysis from source keeps reading from its `CheckedGraph`; the two model variants additionally take the reference bindings for FR-107 model-read accounting; the evidence route takes the `holds` valuations as request facts (FR-516) |
| `evaluate_current_value_invariants`, `evaluate_value_invariant_selection` (FR-652: the type invariants over one admitted current snapshot, or over a no-clause request's snapshot and optional invocation values) | unmet parity: the type invariants are read from the source declarations, which the bytes do not carry; the bare package refuses at the `Package` locus with `unknown_required_feature`/`unsupported-feature`, reason `needs-source-declaration`, and charges nothing; their package form is not part of this inventory |

### The identity file

The directory FR-517 writes carries `identities.json`, written once beside
`checked-package.json`. Its encoding follows `selection.json`: one JSON object,
pretty-printed with two-space indent, members in the order below, closed (an
unknown member is a refusal), UTF-8, ending in one newline, byte-identical for
equal inputs.

| Member | Content |
| --- | --- |
| `format` | the string `filament.checked-handoff.identities/7` |
| `declarations` | the unit's `SelectedExactDeclarationRow` values in the order the emitter holds them; each is an object `{ "structural_node", "selected_document", "form", "original_identity", "exact_identity" }` with `structural_node` the wire node id string, `form` one of `record_value_type`, `object_type`, `union`, `declared_unit`, and `exact_identity` a tagged object `{ "kind": "record" \| "object" \| "union" \| "declared_unit", "value": <value> }` whose `value` is a node reference for `record` and `union`, an `EffectiveId` for `object` and a `UnitId` for `declared_unit`, each as the value encodings below state |
| `fields` | the unit's `SelectedExactFieldRow` values in emitter order; each is `{ "record_node", "selected_document", "record_original_identity", "field_original_identity", "member_id" }` |
| `source_records` | every final emission's source-owned record row, in final row order: `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "record_key", "members" }`; each ordered member is a **member entry** `{ "original_member_identity", "admitted_name", "presence", "type_node", "value_type", "member_type", "collections" }`: a data entry (a member whose type has a node) has `type_node` and `value_type`, `member_type` `null` and `collections` `[]`; a clock entry (a `Clock`, an `Option<Clock>` or a clock collection whose element holds no data, which has no node, FSpec FR-322 "Member type") has `type_node` and `value_type` `null`, `member_type` its FSpec FR-789 rendering with presence, and `collections` one `{ "kind", "minimum", "maximum" }` per collection layer of its type, outermost first, with `kind` one of `sequence`, `set`, `bag`, `ordered_set`, `map`, `minimum` a decimal string and `maximum` a decimal string or `unbounded`; members are every field of the record in declaration order, data and clock |
| `source_tuples` | every final emission's `SelectedSourceTupleRow`, in producer order: `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "type_handle", "positions" }`; each position, in declaration order, is a member entry `{ "ordinal", "type_node", "value_type", "member_type", "collections" }`, with `ordinal` a JSON integer and the other members as for `source_records` (a clock position is a clock entry) |
| `source_unions` | every final emission's `SelectedSourceUnionRow`, in producer order: `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "type_handle", "settled_key", "members" }`; each ordered member is `{ "declaration", "identifier", "payload_nodes", "payload", "clock_payload" }`, with `payload_nodes` and `payload` its data payload positions and `clock_payload` its clock payload positions, each a member entry `{ "ordinal", "presence", "member_type", "collections" }` with `ordinal` the position in the declared payload; a case whose payload is `Clock` has empty `payload_nodes` and `payload` and one `clock_payload` entry |
| `source_enums` | every final emission's `SelectedSourceEnumRow`, in the emitter's source enum declaration row order: `{ "declaration", "members" }`. The declaration is `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "node_key", "shape" }`; `shape` uses the original Exact value-type encoding below with `kind: "enum"`. Each member is `{ "structural_node", "source_authority", "source_identity", "declaration_node", "declaration_qualified_name", "case", "member_key", "variant" }`, with `variant` `{ "owner", "case" }`. Members follow the shape's canonical variant order. Opaque keys, including the variant owner, use the original structural-key encoding below. |
| `source_operations` | every final emission's source-operation identity row (FR-390 "Source-declared operation calls from the reader"), in producer order: `{ "structural_node", "source_authority", "source_identity", "anchor_id" }`, with `structural_node` the wire node id string of the source-declared `state`/`operation_anchor` node, `source_authority` the source authority of the declaring source unit, `source_identity` the original source operation identity (the operation name as the source declares it, qualified by its declaring source type) and `anchor_id` the full anchor id of that node |
| `source_objects` | every final emission's source-object identity row (FR-569 "Source-object identity rows"), in ascending order of their `structural_node`s: `{ "structural_node", "source_authority", "source_identity", "abstract", "supertypes", "identity_fields", "members", "populations", "operations" }`, with `structural_node` the wire node id string of the source-declared `model`/`object_type` node, `source_identity` the type's qualified source name, `abstract` a boolean, `supertypes` the wire node id strings of its direct supertype nodes in source order, `identity_fields` its identity field names in declared order, `members` one member entry per field of its effective view (inherited fields first in `extends` order, then its own in source order; its data entries are exactly its node body's bindings in order, and each clock field is a clock entry), each in the `source_records` member-entry encoding above with `original_member_identity` and `admitted_name` the field name, and `populations` its own populations in source order, each `{ "name", "symmetric" }`, and `operations` its own operations in source order, each `{ "identity", "query", "clock_parameters" }` with `identity` the string `<type>::<operation>`, `query` a boolean and `clock_parameters` the names of its parameters whose declared type is `Clock` or an alias of it, in declaration order (FR-569 "Source-object identity rows") |

| `model_enum_declarations` | every final emission's `SelectedModelEnumDeclarationRow`: `{ "structural_node", "selected_document", "original_identity", "node_key" }` |
| `model_enum_members` | every final emission's `SelectedModelEnumMemberRow`: `{ "structural_node", "declaration_node", "selected_document", "enum_original_identity", "member_original_identity", "case", "member_key" }` |
| `model_enum_shapes` | every final emission's `SelectedModelEnumShapeRow`: `{ "structural_node", "node_key", "selected_document", "original_identity", "shape", "cases" }`; `shape` uses the original Exact value-type encoding below with `kind: "enum"`; each case is `{ "case", "structural_node", "member_key", "member_original_identity", "variant" }`, with `variant` `{ "owner", "case" }`. All opaque keys, including a variant's owner, use the original structural-key encoding below. |
| `model_tuples` | every final emission's selected model tuple declaration, in producer order: `{ "structural_node", "selected_document", "original_identity", "type_handle", "settled_key", "positions" }`; each position, in declaration order, is `{ "ordinal", "member", "member_type" }`, where `member` is the sealed original member identity and `member_type` is the complete original Exact type encoding. Structural and declaration keys use the original opaque-key encoding below. |
| `model_unions` | every final emission's `SelectedModelUnionRow`: `{ "structural_node", "selected_document", "type_handle", "settled_key", "cases" }`; each case is `{ "rank", "position", "name", "original_case", "member", "payload" }`, with `payload` `{ "kind": "none" }`, `{ "kind": "clock", "payload_type" }` or `{ "kind": "data", "node", "original_type" }`. Cases are in ascending case-name byte order and `position` is the authored index. Keys use the original structural-key encoding below. |

The writer SHALL carry every emitted source enum declaration and every real
case node in `source_enums`, including cases no expression uses. The reader
SHALL require this channel, including an empty array when no source enum was
emitted, and SHALL refuse `identities/6` and older formats without a compatibility
path or source fallback. Objects in the channel SHALL be closed; malformed
opaque keys or a `shape` whose decoded kind is not `enum` SHALL return
`Malformed`. The writer SHALL preserve the checked shape, its ordered flag,
original declaration and member keys, and variant identities without deriving
them from display names, structural node ids or case keys.

The writer SHALL carry every selected model tuple declaration in
`model_tuples`, including declarations that are not otherwise read by the
request, and SHALL write an empty array when the admitted selected documents
declare none. The reader SHALL join each row against the admitted package's
selected documents, preserve the ordered positions and complete member types,
and reject a missing, duplicate, foreign, conflicting or noncanonical row
before exposing a partial tuple view. A Clock position remains a clock place;
the row does not construct a `Value::Clock`.

`ReadIdentityRows::selected` SHALL join these complete rows with Contract IR's
existing `SelectedExactIdentities::with_source_enum_rows`, using the same
cumulative native budget as materialization and the other identity joins.
The join SHALL check the selected source, declaration owner and qualified name,
enum form, ordered flag, complete canonical case population, member node and
variant uniqueness, member type and literal, and admitted occurrence source
regions. Missing, foreign, conflicting or noncanonical claims SHALL preserve
the existing typed native join refusal without exposing a partial result.
An absent row required by a later enum projection SHALL refuse
`UnavailableIdentityWitness`; joining an empty channel does not establish
coverage of every enum in the package. Original opaque identities remain
producer claims: structural joining does not authenticate their FSL-stage
issuance. This carrier does not change model-check input form support.

These three model-enum channels carry only payload-free selected model variant
types, including unused declarations and every real case node. A case with a
Clock payload is not payload-free. The writer retains the checked shape and
variant identities, never derives them from case keys, and orders cases by the
shape's canonical rank, independently of authored source positions or model
aliases. Contract IR's `SelectedExactIdentities::with_admitted_model_enum_rows`
joins all three channels against this package's selected model documents under
the invocation's cumulative native work and storage budget. Missing, foreign,
conflicting or noncanonical witnesses refuse before a partial enum view is
exposed. Full runtime projection and bare evaluation consume those same joined
shapes; an enum is never represented as a record or a union. Every producer-owned
copy and runtime enum-member materialization is charged before copying.

The `model_tuples` channel carries the selected model tuple declarations and all ordered member types, including Clock positions. Contract IR joins it against the package's selected model declarations; a package selecting no model tuple carries an empty channel and skips the join. The `model_unions` channel carries the selected payload-bearing variant types, including Clock payloads. Contract IR's `with_model_union_rows` joins it against the package; a package selecting no union carries an empty channel and skips the join. The reader and bare evaluation read tuple projections, union values and `case` arms through the joined rows, preserving each declaration's member order and each union member's canonical rank. A package that selects both model tuples and payload unions is admitted only when both channels join completely; omission or a foreign, conflicting or noncanonical row refuses before a partial package view is exposed.

Enum return values and `=` and `!=` evaluation use these joined shapes and native
equality decisions. Held structural-node membership establishes fact locality;
the original selected-model declaration identity remains unchanged.

### Selected model union producer

A selected model's payload-bearing variant type is a model-owned union. When
the checked package reaches one, the checking stage SHALL retain, on its
`SemanticGraph` (`selected_model_unions`), the selected document's identity and,
for each case, its name, original case identity, authored position and payload.
`VariantRecord.position` is the case's index in the selected document's authored
`variants`; reordering the authored cases changes positions and no key, export
or canonical order. A case's payload SHALL be classified `Clock` only when its
alias-resolved payload type is `Clock`, `Data` when it has one checked data
position, and `None` when it has no payload. Lowering SHALL refuse a case with
no payload form with the typed `UnsupportedUnionPayload` fault, and a case with
no authored variant record with `MissingUnionVariant`, and retain no row for it.

The final `Emission` SHALL expose these facts through
`Emission::selected_model_union_rows`, a borrowed slice of Contract IR's
`SelectedModelUnionRow`, one per emitted union. Each row carries the emitted
structural node, the selected document's identity, the original composite type
handle, the settled declaration key and the cases in ascending UTF-8 byte order
of case name, each with its canonical rank, authored position, original case
identity, sealed member and `Data`, `Clock` or `None` payload. The producer
SHALL NOT supply an empty stand-in or a caller-built row. A monitor plan
emitted over the emission keeps the rows, with node ids re-keyed to the plan's
nodes, and a caller passes them as `SelectedMonitorClosureRows::model_unions`
to Contract IR's native closure, which joins them against the package's selected
model. This producer adds no `model_unions` member to `identities.json`; the
identities format, the bare reader and the evaluation of union values are not
changed by it. A `Clock` payload is a clock place, not a data position, and
no `Value::Clock` is introduced.

A document whose `format` is `filament.checked-handoff.identities/6` or older is refused as the old format, with no read of its rows and no fallback to an empty `source_tuples`, `source_enums`, `source_operations` or `source_objects`, or to members without clock entries. Every member listed in "The identity file" is required, including empty arrays for empty producer channels. The reader SHALL reject `/1`, `/2`, `/3`, `/4`, `/5`, `/6`, an absent member, a member entry with both or neither of `type_node` and `member_type`, a `value_type` present without `type_node` or absent with it, a clock entry whose `member_type` is no clock rendering or that names a binding of its node,
an unknown or repeated member and a malformed row; it SHALL NOT supply an empty
default, read a legacy variant or silently omit a row that cannot be represented.
The writer SHALL emit an empty `source_records` array only when the admitted
producer set is actually empty. The format reader
does not certify whole-witness completeness from an empty array: a request for a
record whose row was omitted reaches native Exact projection and returns its
typed `UnavailableIdentityWitness`. The public bare view maps that native failure
to its existing `MissingTypeRepresentation` at the requested admitted node,
without a structural-key success fallback; this carrier adds no public native
diagnostic-payload variant.

The writer SHALL acquire `source_unions` from the same final `Emission`, retaining
the original `type_handle`, `settled_key`, each sealed member's `declaration` and
`identifier`, and the ordered final `payload_nodes` and original Exact `payload`
types. The three key fields use the original opaque key encoding below; payload
types use its original-value grammar and depth limits. The reader SHALL join
these rows through native `with_source_union_rows`, rejecting foreign, stale or
conflicting rows. `source_unions` is required even when empty; an empty list
does not witness an authored union. No display name or graph-derived key
substitutes for an absent producer row.
Selected model population catalogs are read from the retained native
`CheckedPackageV2` using the Contract IR capability delivered by IR-1358; the
identity carrier does not repeat that catalog. QSL-1762's selected object
closure uses the same declaration/field and source-record families and does
not add a population catalog to this carrier.

The writer SHALL acquire source-record rows from the same final `Emission` as
`checked-package.json`, retaining its final structural and member type IDs,
source owner, original declaration segments, original record key, original member
identities, final admitted names, presence and exact value types in order. A
rewritten monitor-root row therefore keeps the producer's original record key
while carrying the final renamed bindings. The writer SHALL NOT substitute an
emitted structural node key for an original opaque record key.

The new rows distinguish **graph references** (`structural_node`, `type_node`) from **original opaque values**. `record_key` is the shared
`fsl_foundation::node_wire` shape `{ "package", "path", "ordinal" }`, whose path
contains text or nested node-key objects. It preserves the producer key as a
value, never as a graph index or a key inferred from a name. A source member's
`value_type` uses the equality file's complete value-type grammar, except every
NodeKey inside it (composite, enum variant owner, declared or compound unit) is
this original-value shape, rather than a graph reference. Integers and rational
components remain canonical decimal text; presence is `required` or `optional`.

Each original key SHALL contain at most 32 nested node-key objects, counting the
outer key. The writer SHALL check this limit iteratively before recursive JSON
work and the reader SHALL refuse a key above it as `Malformed` at
`source_records`. The bridges use the shared iterative node text codec and SHALL
NOT construct or recursively discard an unbounded intermediate JSON value.

The bytes reader SHALL retain these original producer values and join each source
record through Contract IR's `with_source_record_rows` before exposing its Exact
record type. A healthy carried row SHALL use `source_record_view`; the legacy
`UnavailableIdentityWitness` structural-key substitution is not success for a
carried source record. Stale, foreign or conflicting supplied record rows return
a typed native join refusal with no partial file read result. A missing row is
refused by native projection when that record is requested. The carrier SHALL
NOT introduce an FSL mirror validator or claim that record-row joining qualifies
whole-witness validate/rebind owned by IR-1360.

Value encodings. A **node reference** (a `NodeKey`) is the wire node id string the
package spells that node with (`CheckedNodeId`, as `structural_node` does); the
`NodeKey` of the pinned kernel is a structural value, not a digest, so no digest
spelling applies to it, and `read` recovers the key from the package's wire graph.
An **`EffectiveId`** is `{ "chain": [ { "package": <text>, "node": <text> }, ... ] }`,
the root effective type first and then each member in owner order. A **`MemberId`**
(`member_id`) is `{ "owner": { "kind": "node", "node": <node reference> } | { "kind":
"effective", "effective": <EffectiveId> }, "member": <nonempty text> }`. A **`UnitId`**
of a `declared_unit` row is `{ "kind": "declared", "node": <node reference> }`, the
declared unit's node. The pinned `UnitId` is `Declared(NodeKey)` or
`Compound(list of (NodeKey, Integer))`, with no digest, so no digest spelling exists.
A compound unit is the evaluator's own product over declared root units and is no
selected declaration: the `declared_unit` form takes only a declared unit, `read`
refuses any other `kind` as `Malformed`, and the file carries no compound unit. A
slot over a quantity of a compound unit needs the reader projection of compound
`UnitId` (QSL-1626), which is the existing unmet-parity dependency of FR-533, not a
gap of the file. Text is UTF-8 as written, and a number is never used for an
identity.

Trust boundary. Rows read from a consumer-parsed file grant no authority: IR
states that a row constructed by a caller does not make its Exact identity
authoritative, and the file is trusted as `checked-package.json` is, as an artifact
of the producer the consumer chose to read. `read` verifies only what the package
can answer: that each `structural_node`, `record_node` and node reference is a node
of the package, and that each declaration's `form` matches its identity (`Unjoined`
otherwise), and IR's `SelectedExactIdentities` projection validates the structural
join. It does not verify that FSL's model stage issued an identity; a row that
passes the joins but is forged can change which exact identity a node is read as,
which the evaluator then checks against the admitted nodes and types of the slot.

`ReadIdentityRows::read(bytes, package)` parses the file, joins the rows to the
package by IR's `SelectedExactIdentities` projection, and never derives a row from
the package bytes, a display name or source text. `IdentityFileRefusal` is closed:
`Malformed` (not JSON or not this shape, an unknown or repeated member),
`Format` (another `format` string, naming it), `Unjoined` (a row whose
`structural_node` the package does not hold, or whose form does not match its
identity), naming the first row, and `Native` (a typed IR row-join refusal,
retaining its code and native structural locus). Thus tagging a record node as a
declared unit is a native mismatched-witness refusal even when the tag and key
encoding are individually well-shaped. A refusal returns no rows and charges no
scalar evaluation meter.
filament-protocol's admission input list (its FR-043) names this file beside
`checked-package.json`.

### The equality file

The directory FR-517 writes carries `equality.json`, written once beside
`identities.json`, with its encoding rules: one JSON object, pretty-printed with
two-space indent, members in the order below, closed (an unknown or repeated member
is a refusal), UTF-8, ending in one newline, byte-identical for equal inputs. The
rows are the producer's witness, issued at checking time (FR-533 "Equality slots");
the writer builds each row from the in-process unit's rows, and joins every node and
key member to the wire node id the emitted package spells that node with
(`WireGraph::of_package_bytes` over the emitted bytes, as the inventory file does for
its `node` member), because an in-process `CheckedNodeId` or `NodeKey` carries
structural path segments and the wire form carries operand indices. A node the
package does not name is the writer's `Error::Unnameable`.

The file's value-type vocabulary covers every `filament_exact::ValueType` variant (the
tags listed under "Value encodings" below, `carried_quantity`, `map` and
`bounded_quantity` included), so the writer names the value type of any equality
fact. FR-517-AC-14's `Full` recover compares a receipt sum with a captured target
that FR-517-AC-13 types as a carried quantity; whether the checker records that
operand type as `Quantity` or as `CarriedQuantity` is not established by FR-505 or
FR-517, and the vocabulary admits both.

| Member | Content |
| --- | --- |
| `format` | the string `filament.checked-handoff.equality/1` |
| `applications` | the unit's equality application rows in emitter order; each is `{ "application", "occurrence", "source_map_entry", "node_key", "location" }` in that member order |
| `facts` | the unit's equality fact rows in emitter order; each is `{ "application", "checked_application", "occurrence", "source_map_entry", "structural_key", "structural_node", "owner", "value_type", "fact" }` in that member order |
| `source_units` | the unit's selected source-unit rows in emitter order; each is `{ "structural_node", "source_authority", "source_identity", "qualified_declaration", "unit_id" }` in that member order |

Leaf encodings. Each is a JSON value whose members appear in the order given, with
no other member; a reader refuses an unknown or repeated member as `Malformed`.
Integers are JSON integers (`u64` or `usize` values, never a float or a string);
text is UTF-8 as written.

- A **node id** (the package's `CheckedNodeId`, used for `application`,
  `checked_application`, `structural_node`, and every `*_node` member) is the JSON
  value `serde` gives that type, the form `checked-package.json` spells a graph
  node's `node_id` with: `{ "package": <text>, "path": [ <segment>, ... ],
  "ordinal": <integer> }`, a segment being text (a name) or `{ "node": <integer> }`
  (a graph-local operand index). This is what "The identity file" above calls the
  wire node id; the identity file uses the same value.
- A **node reference** (a `NodeKey`: `node_key`, `structural_key`, `base`, `target`,
  `source_declaration`, `declaration`, `member`, and a `node` member) is the node id
  of the package node that key names, as "The identity file" states for a node
  reference; `read` recovers the key from the package's wire graph.
- An **occurrence** (`CheckedOccurrence`) is `{ "role": <role>, "ordinal":
  <integer> }`, `<role>` one of `declaration`, `type`, `expression`, `anchor`,
  `claim`, `generated`.
- A **source map entry** (`CheckedSourceMapEntry`) is `{ "node_id": <node id>,
  "role": <role>, "ordinal": <integer>, "regions": [ { "source": { "authority":
  <text>, "identity": <text> }, "start": <integer>, "end": <integer> }, ... ] }`,
  `regions` in the order the entry holds them and never empty.
- An **owner** (`CheckedNodeOwner`) is `null` or a tagged object: `{ "kind":
  "model_root", "identity": <text> }`, `{ "kind": "source", "authority": <text>,
  "identity": <text> }` or `{ "kind": "model", "identity": <text>, "node": <text> }`.
- A **location** (`filament_exact::Location`) is `{ "node": <node reference>,
  "occurrence": { "role": <text>, "ordinal": <integer> } }`, `role` the text of the
  location's origin role.
- A **checked rational** (`CheckedRational`) is `{ "numerator": <text>,
  "denominator": <text> }`, both canonical decimal integer strings: an optional `-`
  for a negative numerator, no `+`, no leading zero, `0` for zero, and a positive
  denominator.
- A **unit id** is `{ "kind": "declared", "node": <node reference> }`, or in a
  fact row `{ "kind": "compound", "terms": [ { "node": <node reference>,
  "exponent": <canonical decimal integer string> }, ... ] }` with the terms in the
  compound's own order. A `source_units` row takes the declared form only, as
  "The identity file" does for `declared_unit`.
- A **value type** (`filament_exact::ValueType`) is `null` or a tagged object `{
  "kind": <tag> [, members] }` with the tag and members: `boolean`, `integer`,
  `string`, `uuid`, `timestamp` (no member); `int` `{ "lower", "upper" }`;
  `rational` `{ "numerator": <interval>, "denominator": <interval> }`; `decimal`
  `{ "lower", "upper", "min_scale", "max_scale", "rounding" }`; `float` `{ "width",
  "rounding" }`; `quantity` `{ "unit": <unit id> }`; `text` `{ "min", "max",
  "profile" }`; `enum` `{ "ordered": <boolean>, "variants": [ { "owner": <node
  reference>, "case": <text> }, ... ] }`; `option` `{ "element": <value type> }`;
  `composite` `{ "node": <node reference> }`; `collection` `{ "collection": <kind>,
  "element": <value type>, "bound": null | { "minimum", "maximum" } }`; `reference`
  `{ "effective": <effective id> }`; `population` `{ "maximum": null | <integer> }`; `carried_quantity` `{ "unit":
  <unit id>, "carrier": <carrier> }`; `bounded_quantity` `{ "unit": <unit id>, "min":
  null | <quantity>, "max": null | <quantity> }`; `map` `{ "key": <value type>,
  "values": <value type>, "bound": null | { "minimum", "maximum" } }` (`key` is the
  map's declared key type, a type that nests no other type: filament-exact FR-386, so
  two maps differing only by key never encode alike, and a reader refuses a `key`
  that is an `option`, `collection` or `map`). A *quantity* is `{ "magnitude": <checked rational>, "unit":
  <unit id> }`. A *carrier* is a tagged object `{ "kind": "decimal", "lower", "upper",
  "min_scale", "max_scale", "rounding" }`, `{ "kind": "integer", "lower", "upper",
  "rounding" }` or `{ "kind": "rational", "numerator": <interval>, "denominator":
  <interval> }`, with the members and spellings of `decimal`, `int` and `rational`
  above. For example a `map` of `bounded_quantity` values over the unit node `["U"]` of package
  `p`, with `min` null, is, as a JSON value with members in the order shown (the
  bound members are JSON integers, the magnitude components canonical decimal
  strings): `{ "kind": "map", "key": { "kind": "string" }, "values": { "kind": "bounded_quantity", "unit": {
  "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal": 0 } },
  "min": null, "max": { "magnitude": { "numerator": "5", "denominator": "1" },
  "unit": { "kind": "declared", "node": { "package": "p", "path": ["U"], "ordinal":
  0 } } } }, "bound": { "minimum": 0, "maximum": 4 } }`.
  An *interval* is `{ "lower": <text>, "upper": <text> }` and `lower`, `upper` of
  `int` and `decimal` are canonical decimal integer strings; `rounding` is one of
  `exact`, `toward_zero`, `toward_positive`, `toward_negative`, `nearest_even`,
  `nearest_away`; `width` one of `binary32`, `binary64`; `profile` one of
  `unicode_scalars`, `nfc`, `nfd`, `nfkc`, `nfkd`, `binary_utf8`; `<kind>` one of
  `sequence`, `set`, `bag`, `ordered_set`; an *effective id* is as "The identity
  file" encodes it.
- A **fact** is `{ "kind": <tag>, ... }` with the members of the same-named IR fact
  in the order IR declares them: `operand_type` `{ "position", "role" }`; `unit` `{
  "unit_id", "quantity_type": <node id>, "operand_position", "role" }`;
  `dimension_declaration` `{ "unit_id", "operand_position", "role", "ordinal" }`;
  `dimension_term` `{ "unit_id", "operand_position", "ordinal", "base": <node
  reference>, "base_node": <node id>, "exponent": <text, as the row holds it> }`;
  `conversion_edge` `{ "unit_id", "operand_position", "ordinal", "target": <node
  reference>, "target_node": <node id>, "scale": <checked rational>, "offset":
  <checked rational> }`; `text_plan_type` `{ "position", "role" }`; `enum_member` `{
  "source_declaration": <node reference>, "source_declaration_node": <node id>,
  "declaration": <node reference>, "declaration_node": <node id>, "member": <node
  reference>, "member_node": <node id>, "case": <text>, "ordinal", "ordered":
  <boolean> }`. A `role` of `operand_type`, `unit` and `text_plan_type` is `source`
  or `conversion_target`; that of `dimension_declaration` is `unit_dimension`,
  `conversion_dimension`, `compound_base` or `base_dimension`.

Vector. A unit with one equality application `E` (package `p`, node path `["E"]`)
and one source-unit row for the declared unit `U` writes this file, the rows built
from `E` at occurrence `expression` 0 with one source region of source
`{ "authority": "a", "identity": "s" }` bytes 4 to 9, the operand-type fact of
position 0 role `source` with value type `string`, and `U` owned by source `a`/`s`
at declaration `["U"]`. The bytes are exactly these lines, each ending in a line
feed, written as `serde_json`'s pretty printer writes it (two spaces per level, every
object and array member on its own line, an empty array as `[]`), with no trailing
space. The source unit `U` of the vector is owned by source `a`/`s` at declaration
`["U"]`:

```
{
  "format": "filament.checked-handoff.equality/1",
  "applications": [
    {
      "application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "occurrence": {
        "role": "expression",
        "ordinal": 0
      },
      "source_map_entry": {
        "node_id": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "role": "expression",
        "ordinal": 0,
        "regions": [
          {
            "source": {
              "authority": "a",
              "identity": "s"
            },
            "start": 4,
            "end": 9
          }
        ]
      },
      "node_key": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "location": {
        "node": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "occurrence": {
          "role": "expression",
          "ordinal": 0
        }
      }
    }
  ],
  "facts": [
    {
      "application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "checked_application": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "occurrence": {
        "role": "expression",
        "ordinal": 0
      },
      "source_map_entry": {
        "node_id": {
          "package": "p",
          "path": [
            "E"
          ],
          "ordinal": 0
        },
        "role": "expression",
        "ordinal": 0,
        "regions": [
          {
            "source": {
              "authority": "a",
              "identity": "s"
            },
            "start": 4,
            "end": 9
          }
        ]
      },
      "structural_key": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "structural_node": {
        "package": "p",
        "path": [
          "E"
        ],
        "ordinal": 0
      },
      "owner": {
        "kind": "source",
        "authority": "a",
        "identity": "s"
      },
      "value_type": {
        "kind": "string"
      },
      "fact": {
        "kind": "operand_type",
        "position": 0,
        "role": "source"
      }
    }
  ],
  "source_units": [
    {
      "structural_node": {
        "package": "p",
        "path": [
          "U"
        ],
        "ordinal": 0
      },
      "source_authority": "a",
      "source_identity": "s",
      "qualified_declaration": [
        "U"
      ],
      "unit_id": {
        "kind": "declared",
        "node": {
          "package": "p",
          "path": [
            "U"
          ],
          "ordinal": 0
        }
      }
    }
  ]
}
```

`ReadEqualityRows::read(bytes, package)` parses the file and joins every wire node id
string to a node of the package; it derives no row from the package bytes, a display
name or source text, and grants no authority beyond what IR's `admitted_equality`
validates when a decision is read. `EqualityFileRefusal` is closed: `Malformed` (not
JSON or not this shape, an unknown or repeated member or row member, an unknown fact
tag, a non-integer number), `Format` (another `format` string, naming it) and
`Unjoined` (a row whose node id string the package does not hold, or whose
application is not an equality application of the package), naming the first row. A
refusal returns no rows and charges nothing. filament-protocol's admission input list
names this file beside `identities.json`.

### Refusals

- A call SHALL refuse with FR-516's cause table (temporal) or FR-390's catalog
  refusals (protocol), unchanged. This requirement adds no call cause of its own; the cause
  `aggregate-sample-kind` is FR-516's (window aggregate atoms), and it adds the
  `clause-unreadable` reasons `profile-unknown`, `clause-name` and `capture-type`
  and the supply refusals below, which FR-516's cause table lists.
- A slot call on a `BarePackage` that has no supply SHALL refuse
  `ViewFailure::Unsupported` with `MissingIdentityWitness` at the package locus
  (FR-390). A supply whose rows name no admitted identity or another package
  refuses `UnknownIdentityWitness` or `ForeignIdentityWitness`. There is no
  dependency refusal: `read_checked_package` refuses `MissingImport` for a lock
  that selects a dependency the read evidence does not supply, so a package it
  admitted retains every dependency its lock selects, and the type system leaves
  no package in the state a `DependencyUnheld` refusal would name. Each refusal
  above is evaluation-input admission before S6a, category
  unsupported, exit 21, `CallFailure::Admission` carrying the catalog pair
  `unknown_required_feature`/`unsupported-feature`, with nothing charged.
- A slot call whose walk reaches an `==` or `!=` application (FR-533 "Equality
  slots"), over a supply with no
  equality rows, refuses `ViewFailure::Unsupported` with `MissingEqualityWitness`;
  with rows of another package, `ForeignEqualityWitness` (decided before any row is
  joined, by comparing the rows' package name with the package's name; rows of a
  different package that has the same name are not detected); with rows of the package that hold no row for an equality application the
  walk reaches, or that Contract IR does not admit, `UnknownEqualityWitness`. Each is
  evaluation-input admission like the identity-witness refusals above, at the
  `Package` locus, which the bytes route reports as `CallAdmissionLocus::Package`
  (FR-533 "Refusals and loci of the bare slot call"). A slot call whose walk holds no
  equality application does not read the equality rows and is not refused for their
  absence.
- When an admitted package carries a `temporal_bindings` channel but has no
  row for a clause occurrence's clock, the clause's clock row is absent and
  the call returns the result FR-516 gives a withdrawn clock row
  (`activation = Unavailable`). Contract IR admission refuses a package
  whose whole channel is absent when a temporal declaration requires a
  binding (FSpec FR-322, `temporal_bindings` admission rule); absence is
  permitted only where that rule allows it.
  A lock with no `profile_selections` row for a temporal clause is malformed
  checked-package content: strict package admission refuses it before an
  evaluation call as `invalid_package`/`operation-law-unselected` at the
  clause application's law. It does not produce a call-time
  `clause-unreadable/profile-selection` result. Once strict admission has
  accepted a row, a call whose selected definition is not one of the
  evaluator's profiles refuses at call time as
  `clause-unreadable/profile-unknown`; a call whose selected definition is an
  infinite-trace profile returns the `not-finite-window-profile` temporal
  refusal dimension at call time, as on the in-process route.
- The 18 temporal refusal dimensions of FR-516, the five `PackageMismatch`
  variants and FR-390's refusals are the whole vocabulary of the bare route.

### The native join budget

Joining the supply's identity rows to the package is native work and native
logical storage, which Contract IR accounts for in one caller-owned
`NativeJoinBudget` (Contract IR FR-358). Its work and storage are not the
`Meter`'s counters and are not an input-byte count. FSL states the budget as
follows.

- **One finite budget per pass, built by the pass entry.** The entry of a pass
  (the read's evaluation view, `BarePackage::with_supply`, the temporal
  evidence constructors that hold a source, the handoff writer) builds or is
  given one `NativeJoinBudget` of finite work and storage and passes that same
  value to every join of the pass. No helper below an entry builds, defaults,
  resets or enlarges a budget, and no entry has an unbounded form. The helpers
  `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`,
  `CheckedGraph::selected_exact_identities` and `ReadIdentityRows::selected`
  take `&mut NativeJoinBudget` directly, because `fsl-semantics` is below
  `fsl-package` and cannot name the package limits.
- **Where the ceilings come from.** `V2ReadLimits` (FR-255's `i2.*` row owner)
  gains the required member `storage`, the logical-storage ceiling, and has no
  `Default`; `V2ReadLimits::bounded(storage)` states it and takes the other
  members from Contract IR's bounded reader limits. A read's view builds its
  budget once from the read's `work` and `storage`. The storage ceiling is a
  count of logical ownership units, never derived from `artifact_bytes`, and
  is the `i2.native_join_storage` setting (FR-255); `V2ReadLimits.work` is
  `i2.work_units`. The table default 16777216 applies only where the outer
  configuration reads the `i2.*` settings table: the compile entry's
  `SpineLimits` supplies it, and a request's `native_join` limits and a test
  fixture state their finite values explicitly. Nothing below an entry
  supplies a budget.
- **One join, then borrowing.** `with_supply` joins the supply once, moves the
  joined identities into the `BarePackage`, and every later selection, world,
  projection and temporal call borrows them. A `BarePackage` is therefore
  neither `Copy` nor `Clone`, and no call re-joins or copies the identities.
- **Exact projection shares the admission budget.** The source evaluation view
  and supplied bare package retain the owner `ExactTypeProjection` answers
  needed by their protocol world and slots under that same native budget.
  Unsupported owner answers remain available to the reading stage; a denied
  native charge refuses admission. Subsequent world, slot and capture reads
  borrow these answers and never create a replacement allowance. A bytes-only
  temporal evidence entry without a supply takes the caller's native budget
  explicitly for its capture projections, as do native report admission and
  evaluation entries. These native charges remain separate from the scalar
  evaluator's `Meter`.
- **Witness defects keep their stage.** A missing, foreign or structurally
  unknown identity witness is not refused by `with_supply`; it is held and
  refused, with the order FR-390 states, by the call that reads the identities
  (FR-531-AC-5). `with_supply` returns an error only for a denied native charge.
- **Denial is its own typed refusal.** A denied work or storage charge returns
  `NativeJoinFailure` retaining Contract IR's original typed cause: an
  `ExactTypeProjectionError` with code `ExhaustedWork` or `ExhaustedStorage`
  for identity/type projection, or an `AdmittedEqualityReadError` with code
  `WorkLimit` or `StorageLimit` for equality admission. The shared budget
  retains the first unpaid charge and cumulative admitted counters. No
  `BarePackage` is returned. It is not an `UnknownIdentityWitness`, not an
  `InternalFault` and not an empty set of identities. A denial surfaced through
  the stage-failure or diagnostic path is a `stage_limit_exceeded`-class refusal
  naming `i2.work_units` or `i2.native_join_storage`; the original owner error
  and its code remain available in the library form. The same denial from a
  view's join is `ViewFailure::Exhausted`, from an evidence constructor
  `PackageMismatch::Exhausted`, and from the handoff writer
  `Error::NativeJoinExhausted`. `fsl-eval`'s `CallFailure` has no arm for it:
  the denial happens at the pass entry, before any call that returns one.
  Every entry, including `with_supply`, `from_read` and `from_emission`
  with an opaque caller budget, retains the native error together with a
  `NativeJoinLimitError`: its `limit` is the diagnostic
  `LimitExceeded` with the original configured bound and the cumulative
  attempted count (consumed plus the first unpaid charge), and its catalog
  fields include the setting. Original ceilings come from IR's owner budget
  getters and configured-bound refusal field, including after prior successful
  joins; remaining allowance is never substituted for the original bound.
  The native error retains its structural locus.

### Selected-monitor row materialization

When materializing selected-monitor rows from an `Emission`,
`identity_rows`, `equality_witness` and `selected_source_enum_rows` SHALL take the pass's
caller-owned `NativeJoinBudget` by mutable reference. Their shared row-building
helpers SHALL use that same allowance on source-view admission and handoff
writing paths.

Each adapter SHALL charge its inspections and every owned allocation through
Contract IR's public accounted-charge API before the corresponding inspection
or allocation. The charged material includes vector and temporary-container
storage, copied node and occurrence identities, paths, source-map entries,
owner/type/unit payloads, enum declarations, canonical case rows and source-unit
rows. A borrowed row accessor incurs no owned-storage charge.

The `identity_rows` adapter SHALL charge its FSL-owned materialization in
addition to the native identity join. It SHALL charge owned producer input
rows before presenting them to the native join, including copies an input
iterator would otherwise create while the join first peeks at a row. The
producer selection SHALL retain final-emission membership after carrying or
rekeying rows; checking that membership SHALL NOT allocate uncharged scratch. This material includes converted source
record and member rows, copies of joined declarations and fields, and copied
source union rows. A successful native join SHALL NOT count as payment for
those later FSL allocations. A denied materialization SHALL return no partial
`ReadIdentityRows` and SHALL retain the native charges already admitted.

Footprint inspection SHALL use borrowed owner data without allocating. When
inspection itself requires an allocation, that allocation SHALL be charged
before inspection proceeds. A fresh or lazily represented numeric payload
SHALL NOT be materialized merely to calculate an uncharged footprint. Charges
SHALL cover every owned numeric representation that is actually copied,
including heap-backed values at the signed 128-bit boundary.

These FSL materialization charges count one logical container and one unit per
initialized entry, plus each copied text or numeric payload byte and its owned
buffer. A shared Exact handle contributes its containing entry rather than a
deep copy of the shared payload. The charge does not depend on host pointer
width, allocator capacity or artifact JSON byte length.

FSL's conversion of its model-enum shapes and collection rows to Contract IR's
selected-monitor row types SHALL use the same budget for all copied containers
and payloads. An admitted conversion SHALL preserve complete original node,
declaration and member identities, canonical variant order and Exact value
types. The native collection row SHALL retain the original optional
`key_node`: `Some` of the emitted map key type node for a map, and `None` for
other collections. A map conversion SHALL charge that node's owned copy before
copying it and SHALL preserve the complete original Exact `ValueType::Map`,
including its value type and bound. It SHALL NOT drop the key, substitute the
value node for it, change the collection kind or replace a representable map
with an unsupported result.

Collection conversion SHALL inspect only the retained row the caller selected.
Converting and natively joining or projecting a selected sequence SHALL NOT
inspect or refuse another retained map. A selected map SHALL pass its original
key and value type nodes to native joining and projection in the admitted
key-then-value order. Missing, wrong or misplaced key claims SHALL reach the
owner's typed collection refusal; FSL SHALL NOT reinterpret that refusal or
return a partial native result.

A denied adapter charge SHALL return a fixed-size typed refusal without
allocating the denied material or an owned diagnostic. The refusal SHALL retain
the owner's first unpaid resource, amount, original configured bound and locus,
together with the admitted budget prefix. Each external input-row conversion
SHALL identify its actual retained input position with
`NativeJoinLocusSource::AdapterRow(i)` and name its row channel in the locus
`form`. These FSL adapter charges SHALL NOT use the native read's `ReachedRow`
namespace. A standalone collection conversion SHALL take the caller's
non-owning `AdapterRow(i)` locus and preserve that input position and form;
no graph node or row index is invented. A package-wide container allocation
MAY use the package locus. No partial witness or converted row population is
authoritative after a refusal.

Identity-input declaration and field copies SHALL use the original retained
index before selection filtering. Joined declaration and field output copies
SHALL use their joined channel's unchanged iterator position. Source record
and union output copies SHALL use their original retained slice position;
nested member or payload charges SHALL retain that outer row index and name
their own copy phase in `form`.

The adapters SHALL continue the pass through the owner's accounted-charge and
resume surface without reconstructing, resetting or enlarging the allowance.
Materialization charges are separate from the subsequent native validation and
join charges: both operations spend the same budget once for the work and owned
storage each actually performs.

An explicit caller resume MAY start a new phase without restoring spent
allowance. An adapter SHALL NOT implicitly resume over an existing first
denial. After a denied charge, another attempted charge or adapter call in that
phase SHALL retain that first resource, amount and locus and SHALL perform no
further inspection or allocation.

### Same-emission tick-monitor context

When preparing a selected tick-monitor plan, FSL SHALL keep the [FR-251](./FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md) plan,
obligations, admitted order and progress association bound to the same final
emission and its original selected package/model identities. The order and
progress authority remain FR-251's admitted values; selected-row adapters
SHALL NOT recreate them from names, source spans, bytes, guessed node indices
or tracking values.

The public selected-monitor preparation entry SHALL return a typed refusal
when required order or progress authority is absent or belongs to another
emission. It SHALL NOT produce an executable context with an empty or invented
authority value. Retained context accessors MAY borrow the already admitted
values without owned-storage charges; any owned copy SHALL use the same
caller budget as row materialization and native closure validation.

FSL SHALL pass the complete converted rows to the actual Contract IR native
join and selected type-closure projection before treating them as admitted
monitor inputs. Materialization alone SHALL NOT confer native identity or
execution authority. Successful composition SHALL retain the original plan,
obligation, node, declaration and member identities and the native projected
Exact types. A typed unsupported row SHALL remain unsupported at its requested
locus rather than becoming a partial or differently typed witness.

### Charges

The bare route charges exactly the counters, at exactly the charge points, that
the in-process route charges for the same package and the same request (FR-523's
thirteen temporal counters; FR-255's ten value counters, attributed per binder as
FR-520-AC-11 states). Construction of a `BarePackage`,
supply admission and view creation charge nothing to the `Meter` and accept
none; supply admission and view creation charge only the caller's
`NativeJoinBudget`.

### Relationship to the consumers

filament-protocol FR-045 evaluates compensation slots, and FR-046 assesses temporal
evidence, through this entry and pass it no source package and no emission.
FR-515 and FR-516 state the evidence package and the call; FR-520 states the
outcome map. Each carries a "Consumer entry" section that points here.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-531-AC-1 | In a test crate whose dependencies are `filament-contract-model`, `fsl-package` and `fsl-eval` and which names no `fsl_semantics::CheckedPackage`, no `Emission` and no `AdmittedPackageRead`, the package of **R**'s handoff directory (FR-517) read with `read_checked_package` gives a `BarePackage` by `BarePackage::new`, and from it a temporal evidence call (FR-532) and a protocol-slot call (FR-533) each return a result. | Test (TC-1331) |
| FR-531-AC-2 | `BarePackage` has no constructor from bytes, from a decoded graph or from a term, and no public signature of `BarePackage`, `BareSupply`, `EvidencePackage::from_package` or the `BarePackage` evaluation names `CheckedPackage`, `Emission` or `AdmittedPackageRead`; compile-fail probes of each construction fail, and an API-surface inspection lists the signatures. | Test (TC-1331) |
| FR-531-AC-3 | The evaluation code reached from `BarePackage` does not read a source graph: the same calls over a `BarePackage` whose package was read in a run where the source unit was never compiled give the results the calls give where it was (TC-1331 row 1 runs the calls in a child run that is given the handoff directory only). | Test (TC-1331) |
| FR-531-AC-4 | Over **R**'s three packages the bare route evaluates every temporal clause form (event position, fixed sample, timestamped event) and every protocol slot kind of FR-390's slot table that **R** holds; a node that is not a temporal clause refuses `clause-root-unresolved` and a slot node that is not a protocol node refuses as FR-390-AC-4 states, each with an empty charge log. | Test (TC-1331) |
| FR-531-AC-5 | Over a `BarePackage` with no supply, a protocol-slot call refuses `MissingIdentityWitness`; with rows of another package, `ForeignIdentityWitness`; with rows of the package naming no admitted identity, `UnknownIdentityWitness`; rows of another package are `ForeignIdentityWitness` also when they name no admitted identity. Each is `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature`, category unsupported, exit 21, and the meter untouched. A temporal evidence call over the same `BarePackage` with no supply returns its result; and `read_checked_package` over bytes whose lock selects a dependency the evidence does not supply refuses `MissingImport`, so no `BarePackage` exists for it and no dependency refusal is defined. | Test (TC-1331) |
| FR-531-AC-6 | For an admitted temporal clause whose clock requires a `temporal_bindings` row but whose admitted channel has no row for that clause, the call returns `activation = Unavailable` with the `Missing` non-value for that clock, and the meter's charge log remains empty. | Test (TC-1331) |
| FR-531-AC-7 | No result of the bare route holds, in its canonical bytes or its fields, a clause name, a source label or a source span: over **R**, the encoded `EvidenceResult` and `ProtocolSlotEvaluation` values do not contain the unit's `source:` labels or a byte region of the source map, and a clause name appears only as the `temporal_clause_names` row of its occurrence gives it, identically on every route. Identity is spelled as the "What the bytes do not carry" list states, and the progress binding of a result holds `package.name()`. | Test (TC-1331) |
| FR-531-AC-8 | The handoff directory of **R** holds required `identities.json` with `format` `filament.checked-handoff.identities/7`; `ReadIdentityRows::read` over it and the package gives a slot call that evaluates; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files; omitting the file from a `BareSupply` makes the slot call refuse `MissingIdentityWitness` while the temporal call is unaffected. | Test |
| FR-531-AC-12 | `ReadIdentityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with another `format`, one with a row whose `structural_node` the package does not hold and one whose `form` does not match its identity returns `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the format string or the first row, and returns no rows. | Test (TC-1331) |
| FR-531-AC-13 | The identity file of **R** encodes every identity as the "Value encodings" state: a node reference is the wire node id string of the package, an `EffectiveId` has the `chain` list root first, a `MemberId` has `owner` and `member`, and a declared `UnitId` has `kind` `declared` and a `node` reference; `read` returns rows equal to the writer's, a unit with `kind` other than `declared` returns `Malformed`, a declared unit whose node reference the package does not hold returns `Unjoined`, and a `declared_unit` row whose `exact_identity` has another `kind` returns `Unjoined`. A forged row that passes the joins is accepted by `read` and the slot call over it evaluates against the admitted nodes, so the test records that `read` makes no claim of authority. | Test (TC-1331) |
| FR-531-AC-9 | For each public method of `EvidencePackage` and each method of the `CheckedPackageEvaluation` trait, enumerated by a test that reads those two items' declarations in the crate source with `syn` and compares the names to the first column of the inventory table, the table has exactly one row: available from the bare package, or listed as not carried with a reason; a method added to either type without a row fails the test. | Test (TC-1331) |
| FR-531-AC-10 | The charge log of each call over the bare package equals the charge log of the same call over the in-process route (FR-532-AC-1, FR-533-AC-1); `BarePackage::new`, `with_supply` and view creation take no `Meter` and a call that refuses at supply admission leaves a caller's meter byte-identical. | Test (TC-1331) |
| FR-531-AC-11 | A search of FR-515 for the phrases `bytes-only` and `separate future capability` and of FR-515, FR-516 and FR-520 for the heading `Consumer entry` finds that FR-515 no longer states that an evaluator over `formula_graph()` with no source graph is out of scope, and FR-515, FR-516 and FR-520 each carry a "Consumer entry" section that names this requirement. | Inspection (TC-1331) |
| FR-531-AC-14 | The handoff directory written for TC-1070 fixture E holds `equality.json` with `format` `filament.checked-handoff.equality/1`; `ReadEqualityRows::read` over it and the package gives a slot call on `Refund`'s retry relation (an integer `=` application) that evaluates to the in-process route's value and charge log; the file equals, byte for byte, the encoding above of the in-process unit's rows, and two writes of equal requests give equal files. | Test (TC-1331) |
| FR-531-AC-15 | `ReadEqualityRows::read` over a file that is not JSON, one with an unknown or repeated member, one with an unknown fact tag, one with another `format`, one with a row whose node id string the package does not hold and one whose application is not an equality application returns `Malformed`, `Malformed`, `Malformed`, `Format`, `Unjoined` and `Unjoined` respectively, naming the member, the tag, the format string or the first row, and returns no rows. | Test (TC-1331) |
| FR-531-AC-16 | A slot call on TC-1070 fixture E's `Refund` retry relation refuses `MissingEqualityWitness` over a supply with `equality: None`, `ForeignEqualityWitness` with rows of another package and `UnknownEqualityWitness` with rows of the package from which the row of the retry relation's application is removed, each as `CallFailure::Admission` with `unknown_required_feature`/`unsupported-feature`, category unsupported, exit 21, `CallAdmissionLocus::Package` and the meter untouched; `Refund`'s `trigger.guard`, whose walk holds no equality application, over the same supply evaluates. | Test (TC-1331) |
| FR-531-AC-17 | The encoding of the rows of "The equality file" vector equals the vector's bytes exactly; `ReadEqualityRows::read` over those bytes and a package that holds the node `["E"]` and `["U"]` returns rows equal to the input rows; for a row of every fact tag, every value-type tag, every owner kind and a compound unit id in a fact row, writing then reading returns equal rows and a second write is byte-identical; a checked rational whose numerator is spelled `+1`, `01` or `-0`, or whose denominator is `0` or `-2`, returns `Malformed`; a compound unit id in a `source_units` row returns `Malformed`. | Test (TC-1331) |
| FR-531-AC-18 | For a row whose fact value type is each of a `carried_quantity` over a decimal carrier, over an integer carrier and over a rational carrier, a `bounded_quantity` with both bounds and with `min` and `max` null, and a `map` with a bound and without one (including the example of "Value encodings"), `ReadEqualityRows::to_bytes` followed by `ReadEqualityRows::read` returns equal rows, a second write is byte-identical, and the value-type subtree the writer emits for the example, parsed as JSON, equals the example's JSON value with its members in the stated order (members compared in order, not as a set); a `bounded_quantity` whose bound is in another unit than `unit`, and a `carried_quantity` carrier tagged other than `decimal`, `integer` or `rational`, return `Malformed`. Two `map` types that differ only by `key` (a `string` key against an `integer` key) write different value-type subtrees, and a `map` whose `key` is an `option`, `collection` or `map` returns `Malformed`. | Test (TC-1331) |
| FR-531-AC-19 | An authentic final emission with a source record round-trips its original opaque record key, source owner, final structural/type IDs, ordered original member identities, final admitted names, presence and exact value types, including nested opaque identities. The bytes-only read joins it through native IR and `source_record_view` equals the original row; no healthy source record succeeds by structural-key substitution. | Test (TC-1331) |
| FR-531-AC-20 | A file with `/1`, absent `source_records`, an unknown/repeated member, malformed or over-depth opaque key/type, foreign structural/type node, stale admitted name or conflicting record row refuses with the named typed format/shape/native-join condition and no partial identity read. With a supplied empty record-row list, an actual request for that record preserves native `UnavailableIdentityWitness`, never succeeding by structural-key substitution. Whole-witness completeness is not asserted by this carrier read. | Test (TC-1331) |
| FR-531-AC-21 | A `BarePackage::with_supply` over a fixture whose identity join is measured on a fresh budget at work **W** and storage **S** returns the package on `NativeJoinBudget::new(W, S)`, and returns `ExhaustedWork` on `new(W - 1, S)` and `ExhaustedStorage` on `new(W, S - 1)`, each with IR's locus and attempted charge, no package, the work denial naming `i2.work_units` and the storage denial naming `i2.native_join_storage`, and the budget's consumed counters equal to those IR reports. The three results differ in kind from `UnknownIdentityWitness`, `InternalFault` and an empty identity set. | Test (TC-1331) |
| FR-531-AC-22 | Over a supply with rows of another package, with rows IR does not join, and with no supply, `with_supply` over a budget that admits the join returns a package whose slot call refuses `ForeignIdentityWitness`, `UnknownIdentityWitness` and `MissingIdentityWitness`, as FR-531-AC-5 states, and the same rows over a budget one unit short of the unknown rows' join return the budget denial and not `UnknownIdentityWitness`. | Test (TC-1331) |
| FR-531-AC-23 | Two joins on one budget are charged cumulatively: after a successful `with_supply` the budget's consumed counters equal the join's measured work and storage, a second `with_supply` on the same budget is refused at the first charge that crosses the remaining allowance, and a selection, a world and a projection call made on the first package consume no further native work or storage. | Test (TC-1331) |
| FR-531-AC-24 | `V2ReadLimits` has a required `storage` member and no `Default`; a read's evaluation view over limits whose `storage` is exactly the measured join storage succeeds and over one unit less returns `ViewFailure::Exhausted` with `ExhaustedStorage`, whose refusal names `i2.native_join_storage`; `EvidencePackage::from_read` and `from_emission` over a budget one unit short of the source join return `PackageMismatch::Exhausted`; the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted` and writes no directory. | Test (TC-1331) |
| FR-531-AC-25 | An inspection of the public signatures of `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`, `CheckedGraph::selected_exact_identities`, `ReadIdentityRows::selected`, `BarePackage::with_supply` and the evidence constructors finds each takes a `NativeJoinBudget` by mutable reference, finds no `NativeJoinBudget::new` and no `u64::MAX` allowance in a library path below an entry, and finds that `BarePackage` implements neither `Copy` nor `Clone`. | Inspection (TC-1331) |
| FR-531-AC-26 | An authentic final emission's source-union rows round-trip original type handles, settled keys, sealed member identities, final structural and payload node IDs and ordered Exact payload types; the read rows equal the emitter rows, native `source_union_rows` retains them and rewriting is byte-identical. | Test (TC-1331) |
| FR-531-AC-27 | `/2`, absent `source_unions`, malformed member identity, stale member name, foreign source owner and conflicting union rows return typed format, malformed or native-join refusals without a partial read. An empty union list never fabricates a producer witness from the graph. | Test (TC-1331) |
| FR-531-AC-28 | Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget` through IR's admitted equality reader after identity/type projection. Decisions and non-resource witness refusals are retained for later unmetered selection and projection. Successful reads consume native work and storage cumulatively; exact measured bounds admit, and one unit less in either resource refuses with the original typed IR equality denial and recorded first unpaid charge, never an unknown witness or internal fault. Reusing an admitted slot consumes no additional native allowance. | Test (TC-1331) |
| FR-531-AC-29 | Materializing an authentic emission's equality witness with a shared native budget preserves every application, fact and source-unit row, including original identities and order. The source admission and handoff paths use the same budgeted row builder. Exact measured work and storage allowances return the complete witness; one unit less in either resource refuses without allocating the denied material or a diagnostic, retains the first unpaid resource, amount and original locus, and returns no partial witness. Two individually affordable materializations that exceed one shared allowance refuse at the second call's first unpaid step without resetting prior consumption. | Test |
| FR-531-AC-30 | Source-enum materialization and FSL-to-IR model-enum-shape and selected-collection conversion preserve original structural/declaration/member identities, canonical case order and complete Exact types on one shared native budget. Exact measured bounds admit; work or storage one short and zero storage return allocation-free compact denials at the first unpaid locus with prior consumption intact. A later conversion cannot reset a preceding adapter's charges or replace its first denial. A selected source map preserves its original optional key node and complete Exact map value type through actual native joining and projection; absent or wrong map keys and a map key misplaced on a non-map row return the native typed collection refusal. An unrelated retained map leaves conversion and native projection of a selected sequence unchanged. External row denials name AdapterRow(i) at the actual retained input index and keep that locus across a later attempted charge; native read loci remain in their own namespace. Borrowing already retained rows incurs no owned-storage charge. | Test |
| FR-531-AC-31 | An authentic emission with source records, record members and unions produces complete identity rows equal to the original producer claims after native joining and FSL materialization. The measured work and storage include the FSL producer input copies, the single native join and each later FSL declaration, field, record/member and union copy. Exact allowances admit; one unit below either measured allowance refuses at its first unpaid step with no partial identity rows, the original configured bound and the admitted prefix intact. Denied external row copies retain AdapterRow(i) at their original per-channel retained input or joined-output position; sparse selection does not renumber inputs, and nested members retain the outer record or union index. Two materializations on one budget compose without reconstructing or resetting it. | Test |
| FR-531-AC-32 | A real tick-monitor plan's retained context supplies the same emission's plan, obligations, admitted order and progress association to the actual native row join and selected type-closure projection. The resulting native nodes, original declaration/member identities and Exact types equal those of that emission. Substituting another genuine emission's context, even with identical authored names, or omitting required order/progress authority returns a typed refusal and no executable context. Borrowing retained context leaves native storage consumption unchanged. | Test |
| FR-531-AC-33 | Allocation observation encloses only the adapter/conversion call, with fixture preparation and assertions outside it. A successful owned materialization performs observed allocations; a denial before its first allocation performs none, and a later denial allocates neither the denied material nor an owned diagnostic. The controls include fresh owner-produced lazy numeric payloads, values `2^127` and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting. These values remain exact on successful copies; insufficient work/storage retains the first real row locus and original bound. | Test |
| FR-531-AC-34 | Over the unit `record W { c: Clock; n: Integer; }`, `tuple P (Clock, Integer);`, `union U { armed(Clock), idle }`, `object O { w: W; d: Clock; q: Sequence<Clock>[0, 3]; p: P; u: U; operation arm(x: Clock, k: Integer) modifies self.d; }` and `time O dense unit ms;`, `identities.json` has `format` `/7`; `O`'s `source_objects` row has the data entries `w`, `p`, `u` and the clock entries `d` (`member_type` `ix://filament/native/Clock`, `collections` `[]`) and `q` (`Sequence<ix://filament/native/Clock>`, `[{"kind": "sequence", "minimum": "0", "maximum": "3"}]`) in field order, and the operation `{"identity": "O::arm", "query": false, "clock_parameters": ["x"]}`; `W`'s `source_records` row has the clock entry `c` and the data entry `n`; `P`'s `source_tuples` row has the clock position `0` and the data position `1`; `U`'s `armed` member has empty `payload_nodes` and one `clock_payload` entry; the file reads back equal. A `/6` document, an absent `source_tuples`, an entry with both `type_node` and `member_type`, one with neither, a clock entry named `w`, and an operation entry without `clock_parameters` each refuse `Malformed` with no partial read. | Test (TC-1331) |
| FR-531-AC-35 | The unit of FR-062's RR-12 (`g` and `f`) and RR-16 (`g`, `h` and `f`, renamed `g2`, `h2` and `f2`), with RR-5's `sq` added, is emitted with one `requirement_items` row per record of `CheckedGraph::requirements()` in its iteration order (eight rows: three for `f`, two for `f2`, three for `sq`), each row's `claim`, `kind` and `extent` equal to its record's key, capability and extent and each `value-validity` row's record node equal to its record's owner, bound, conversion, guards, roots and objects; the package read with `read_checked_package` gives a `BarePackage` whose `call_requirements("f")`, `("f2")` and `("sq")` equal, member for member and request index for request index, FR-395's `call_requirements_for` over the compiled unit, and `call_requirements("g")` is empty. The same package with `f2`'s second row removed is refused by the strict reader (FSpec FR-825 rule 14), so no `BarePackage` gives a shorter member list. | Test (TC-1331) |
| FR-531-AC-36 | Over `function f using v(p: Float64[nearest-even], q: Float64[nearest-even], r: Float64[nearest-even]): Boolean pure { sqrt(p) <= sqrt(q) }` and `function g using v(p: Float64[nearest-even], q: Float64[nearest-even], r: Float64[nearest-even]): Boolean pure { f(p, q, r) }`, each called with `p`, `q` and `r` at `2.0` on the source route and on the `BarePackage` of the same unit's written bytes with the same remaining budget: for each of `f` and `g` both routes return `Completed(true)` and equal charge logs, and both return exactly two losses, each `IeeeFlags` with the inexact flag, located at `Body { function: "f", index: 0 }` paths `[0]` and `[1]` in that order (for `g` too, since the loss is in `f`'s body, never at `g`'s call site); the package's `body_locations` rows of `f` give those two paths for the two `sqrt` terms. A bare route that locates `f`'s body from the child-index paths of the lowered body places the second loss at `[1, 0]` and fails the equality; one that has no location faults `value-loss-site-is-located` and fails; the same bytes with the second `sqrt`'s row removed are refused by the strict reader (FSpec FR-322-AC-490) and evaluate nothing. | Test (TC-3133) |
| FR-531-AC-37 | For `T` each of `Float32[nearest-even]` and `Float64[nearest-even]`, each row below is a unit holding `function f using v(...): Boolean pure { r <= e }` and `function g using v(...): T pure { e }` with the same parameters (`r` absent from `g`), called with `r` `0.0` on the source route and on the `BarePackage` of the unit's written bytes with the same remaining budget: `e` is `p + q` and `p - q` over `p: T, q: T` at `1.0` and `2^-60` (inexact); `p * q` at both the largest finite value of `T` (overflow, inexact); `p * q` at the smallest positive subnormal of `T` and `0.5` (underflow, inexact); `p / q` at `1.0` and `3.0` (inexact) and at `1.0` and `0.0` (divide-by-zero); `filament::value::ieee::sqrt(p)` over `p: T` at `2.0` (inexact) and at `-1.0` (invalid); `convert<T>(a)` over `a: Rational[-10, 10; 1, 10]` at `1/10` (inexact); and, for `T` `Float32[nearest-even]` only, `convert<Float32[nearest-even]>(b)` over `b: Float64[nearest-even]` at `0.1` (inexact) and at the largest finite `Float64` (overflow, inexact). Each call on each route records exactly one `IeeeFlags` loss holding exactly the flags named, located for `f` at `Body { function: "f", index: 0 }` path `[1]` (the right operand of the ordering, which lowering binds by an `#ieee_operand` `let` at path `[0]` of the lowered body) and for `g` at `Body { function: "g", index: 1 }` path `[]`, and the two routes return equal outcomes, equal losses and equal charge logs; no call faults `value-loss-site-is-located`. A bare route that derives `f`'s locations from the lowered body records `[0]` and fails every `f` case; one without the rows faults. | Test (TC-3133) |
| FR-531-AC-38 | For FR-531-AC-36's `f` at `2.0`, `2.0` and `2.0`, with `U` the `work_units` the source route's completed call spends, each call with a `work_units` limit from `0` to `U - 1` returns `Incomplete` naming `work_units` on both routes, with equal `Evaluation.location` and equal charge logs on the two routes at every limit; the locations over the sweep include `Body { function: "f", index: 0 }` path `[1]`, the denied charge of the second `sqrt`, which a bare route that derives locations from the lowered body reports as `[1, 0]`. | Test (TC-3133) |
| FR-531-AC-39 | Over the `Config` domain fixture of `tests/it/dispatch_emission.rs`, the unit holding its three `size` bodies, `pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }` (`rootCall`'s call of `size` is what makes `sqrtPre` a clause function: the checker builds a clause function only for an operation a dispatch candidate reaches, and no candidate reaches `probe`) and `pre sqrtPre using v on Config::ConfigVersion::size { filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) <= filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) }` emits, for the precondition's clause function (a `function` node with a `clause` binding `precondition` and a `body` binding), one `body_locations` row per term of its body walk, every row's origin carrying the checker's `Body` owner `function` and `index` of that clause function, and the two `sqrt` rows the paths `[0]` and `[1]` that the checked graph's locations of those two nodes hold (the lowered body holds them at `[0]` and `[1, 0]`); each `size` body, a literal with no term, has no row. The strict reader admits the package, and refuses it at `/checked_term_data/body_locations` once the second `sqrt`'s row is removed. | Test (TC-3133) |
| FR-531-AC-40 | FR-531-AC-39's unit with `ConfigVersion` declared in the unit's own source as `object ConfigVersion { versionNumber: Integer; operation probe(): Integer; operation size(): Integer; }` with its three `size` bodies (`SourceOwner` identity `I`), in place of the selected `Config` model, and with `rootCall` and `sqrtPre` written `on ConfigVersion::probe` and `on ConfigVersion::size`, whose body is `filament::value::ieee::sqrt(convert<Float64[nearest-even]>(float64(bits: 0x4000000000000000))) <= filament::value::ieee::sqrt(convert<Float64[nearest-even]>(float64(bits: 0x4000000000000000)))`. The `body_locations` rows of the precondition's clause function carry a `name` `F` beginning `ix://I/`, and no row and no other byte of the package holds `ix://source/`. Evaluating that clause function at `size`'s pre-state on the source route and on the `BarePackage` of the unit's written bytes with the same remaining budget returns two `IeeeFlags` losses with the inexact flag on each route, located at `Body { function: F, index: k }` paths `[0]` and `[1]`, with the same `F` and `k` on both routes, and equal `Evaluation.location` under every `work_units` limit below the completed call's spend (FR-531-AC-38's sweep). A source route that reports the checker's `ix://source/<unit>/...` name fails the equality. A declared function `f` keeps the location `Body { function: "f", index: 0 }` on both routes (FR-531-AC-36). | Test (TC-3133) |
| FR-531-AC-41 | For two separate units, one holding `function d(x: Integer): Integer pure { x + x + ... + x }` with 100,000 `+` applications (100,001 terms), the other holding 100,000 nested `let`s whose first binding is `x + 1`, each subsequent binding is the previous binder plus 1 and the result is the last binder, S4 writes complete packages and their `body_locations` on a 512 KiB stack under finite explicit caller ceilings chosen before emission from the checked input and the declared linear bounds above. The actual complete canonical byte lengths, relative entry counts and charged native strict-read and lending-cursor work are recorded separately for the two units and do not exceed those bounds. The rows use FSpec FR-322's relative form (FSpec FR-322-AC-510's BL-D1 and BL-D2 rows exactly); the strict reader admits each package, and each typed path lent in body-walk order equals the checker's path without retaining all expanded paths. With the same checked units and other stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16,777,216 instead refuses `OutputLimit` with setting `emit.output_bytes`, configured bound 16,777,216 and first unpaid byte 16,777,217, and no partial package. A row written with a whole `path` is refused `malformed_wire` at its `location`. | Test |
| FR-531-AC-42 | A selected model with `Tier` cases `Gold`, `Standard` and an unused payload-free variant with non-identifier cases in noncanonical authored order writes all three required model-enum channels in identities/7. Reading the file preserves every original declaration, member and shape; full runtime projection retains both enums, and the bare world resolves every case to its authenticated canonical rank and member identity. Calling enum-valued functions, including a nested typed parameter round trip, and equality and inequality callers from source and the bytes gives equal values, variants, ranks and semantic charge logs; renaming the model alias changes no enum value identity. | Test |
| FR-531-AC-43 | Omitting or repeating any required enum channel, including `source_enums`, an unknown channel, or identities/6 or older refuses without a partial read. Foreign selected-document claims, missing or repeated members, reversed shape variants, and reordered canonical case rows each fail the native witness join. For source enum rows, an unknown or repeated nested field, malformed original key or non-enum shape returns `Malformed`; changing the source owner, qualified declaration, declaration node, ordered flag, member literal, variant owner or case likewise fails the native witness join. A present empty `source_enums` array supplies no row for a demanded enum: projection refuses `UnavailableIdentityWitness`, without a source fallback. | Test |
| FR-531-AC-44 | For original model-enum rows and, separately, original source-enum rows, one cumulative native budget covers producer materialization, native identity join and full runtime projection. The exact measured work and storage ceilings succeed with the same consumed totals; zero and one-less work or storage ceilings refuse with the original first-unpaid resource, attempted amount, configured bound, locus and admitted prefix, and no partial enum view. | Test |
| FR-531-AC-45 | For an admitted selected model that declares a payload-bearing variant type reached by the checked package, `Emission::selected_model_union_rows` returns one `SelectedModelUnionRow` per emitted union. Each row carries the emitted structural node, the selected document's identity, the original composite type handle, the settled declaration key, and its cases in ascending UTF-8 byte order of case name. Each case carries its canonical rank, authored position in the selected document, original case identity, sealed member and exactly one payload: `Data` with the lowered payload node and original value type, `Clock`, or `None`. | Test (TC-1331) |
| FR-531-AC-46 | When the rows of `Emission::selected_model_union_rows` are passed as `model_unions` of `SelectedMonitorClosureRows` for a monitor whose typed input reaches a union with `Idle`, `Circle(Integer)` and `Armed(Clock)` cases authored out of canonical order, the native closure admits and projects the union node, and the rows retain each case's rank, authored position and payload kind. The same closure given no union rows refuses with `Exact(UnavailableIdentityWitness)`. | Test (TC-1331) |
| FR-531-AC-47 | A union case's payload is classified `Clock` when its alias-resolved payload type is `Clock`, including through an alias. A payload that resolves to a sequence or map over `Clock`, a payload with a data position but no authored payload type, and a payload with more than one checked data position are each not classified `Clock` and have no payload form. | Test (TC-1331) |
| FR-531-AC-48 | `VariantRecord.position` is the case's index in the selected document's authored `variants`. Reordering the authored cases changes each case's position and no key, export or canonical case order. | Test (TC-1331) |
| FR-531-AC-49 | Lowering a union case that has no payload form refuses with the typed `UnsupportedUnionPayload` fault (invariant `union-payload-has-ir-form`), and lowering a case with no authored variant record refuses with `MissingUnionVariant` (invariant `union-case-variant-retained`). Each refusal retains no row for the union and omits no case silently. | Inspection |
| FR-531-AC-50 | A checked source declaring unordered `Status { READY, DONE }` with only `READY` referenced, and ordered `Level { LOW, MID, HIGH }`, writes `source_enums` in identities/7. Strict package admission and identity-file reading supplied only the emitted artifacts preserve the checked declaration shapes, ordered flags, declaration and member keys, variant identities and canonical ranks: `DONE, READY` for Status and `LOW, MID, HIGH` for Level. The unused `DONE` case is retained; source rows do not appear in the model-enum channels. Native projection returns each original `ValueType::Enum` and its complete source enum row. Two writes of equal inputs are byte-identical. | Test |
| FR-531-AC-51 | A selected model declares `tuple Slot(Clock, Integer)` and `union Phase { Armed(Clock), Idle }`, and a checked package reaches both declarations. `identities.json` uses format `/7` and carries one `model_tuples` row for `Slot` and one `model_unions` row for `Phase`; the tuple row preserves the selected document, original declaration identity, type handle, settled key, both positions in declaration order and each complete member type, including `Clock` at position zero. `ReadIdentityRows::read` joins both rows against the admitted package and the bare type projection preserves the tuple position and union case identities. Omitting `model_tuples`, omitting `model_unions`, or changing either row's selected document, declaration identity, key, position order or member type refuses before any partial projection; an empty channel is accepted only when the admitted selected documents contain no declaration of that kind. Two writes of equal input are byte-identical. | Test (TC-1331) |
| FR-531-AC-52 | A checked package with a clock-configured temporal declaration that requires a `temporal_bindings` row, but whose whole channel is absent, is refused by Contract IR admission as `invalid_package`/`timed-node-reference` at the clause body before any evaluation call. It is never accepted as an empty channel. This is distinct from an admitted channel missing only that clause's row, which follows AC-6 and returns `activation = Unavailable`. | Test (TC-1331) |
| FR-531-AC-53 | A checked package containing a temporal clause whose `temporal_profile` law has no matching `profile_selections` row is refused by strict package admission as `invalid_package`/`operation-law-unselected`, with typed cause `OperationLawUnselected` at that clause node's `/laws/0/definition` locus; no package is returned and no evaluation call occurs. It does not produce a call-time `clause-unreadable/profile-selection` result. | Test (TC-1331) |
| FR-531-AC-54 | After strict admission of a package with a `profile_selections` row, when a call selects a definition identity that is not one of the evaluator's supported temporal profiles, the call refuses as `clause-unreadable/profile-unknown`. | Test (TC-1331) |
| FR-531-AC-55 | After strict admission of a package with a `profile_selections` row, when a call selects an infinite-trace profile, the call returns the `not-finite-window-profile` temporal refusal. | Test (TC-1331) |
| FR-531-AC-56 | For the profile-unknown refusal of AC-54, the call's charges equal the FR-516 source-route counterpart. | Test (TC-1331) |
| FR-531-AC-57 | For the infinite-profile refusal of AC-55, the call's charges equal the FR-516 source-route counterpart. | Test (TC-1331) |
| FR-531-AC-58 | The strict-read refusal of AC-53 occurs before any evaluation call and incurs no evaluation work. | Test (TC-1331) |

## Dependencies

- **Upstream:** FR-337, FR-383 (the nodes and rows), FR-390 (the reader view and
  its identity rows; the reader-view criteria it states are the ones the bare slot
  call rests on), FR-515, FR-516, FR-517, FR-520; Contract IR `read_checked_package`
  and `SelectedExactIdentities`; QSL-1377 (selected-model identity-row production).
- **Selected monitor composition:** [FR-251](./FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md) owns the admitted tick-monitor plan,
  order and progress authority. Contract IR FR-357 and FR-358 own native closure
  validation and the public accounted-charge/resume budget contract.
  Contract IR FR-357-AC-16 owns optional map-key row representation and native
  map validation; FR-358-AC-10 owns the external `AdapterRow` locus. This
  requirement owns FSL's intervening row materialization and conversion; it does
  not replace either owner's admission or claim monitor/runtime qualification.
- **Downstream:** filament-protocol FR-045 and FR-046; FR-532 and FR-533. filament-protocol
  FR-043's admission input list follows the two files.
- **Writer and reader encoders for the three added tags:** the code ticket for the
  `carried_quantity`, `bounded_quantity` and `map` encoders follows in the B2a work;
  until it lands FR-531-AC-18 is unmet and the writer refuses the three types
  `Error::Unnameable`.
- **Wire forms the writer adds:** the equality file's leaf encodings are pinned in
  "The equality file". `CheckedNodeId`, `CheckedOccurrence`, `CheckedSourceMapEntry`,
  `CheckedNodeOwner` and `CheckedRational` have `serde` forms in Contract IR today and
  the file uses them. `filament_exact::Location`, `ValueType` (with its interval,
  decimal, float, text, enum and collection parts), `UnitId`, `NodeKey` and
  `EffectiveId` have none; the FSL writer and `ReadEqualityRows::read` encode and
  decode them by hand to the forms above, from those types' public accessors and
  constructors, and no change to Contract IR or filament-exact is required for the file.
