---
id: FR-940
title: "Analyze a checked package through one sealed read view over source-built and byte-read packages"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-397
    type: references
---
# FR-940: Analyze a checked package through one sealed read view over source-built and byte-read packages

## Description

When an analyze request selects a `package` (FR-850), the analysis engines
(FR-281) SHALL read the package they analyze only
through `fsl_package::AnalysisView`, a sealed read trait implemented for
exactly two types: `AdmittedPackageRead`, the strict reader's admitted
read-back of an in-process `CheckedPackage`'s own S4 emission, and the
`BarePackage` (FR-531) over a `CheckedPackageV2` the strict reader admitted
from bytes. Both hold an admitted `CheckedPackageV2`; a `CheckedPackage`
holds none (its emission is an optional `OnceLock<Emission>` of bytes), so it
reaches analysis only through its admitted read-back. An analyze request that selects a `package` (FR-850)
analyzes its `BarePackage` through this view. A request that selects a
`program` analyzes the compiled `CheckedPackage`: each fact of FR-531's table
that an engine reads is read from the `AdmittedPackageRead` of its own S4
emission (`read_evaluation_package`) through the same view, and each
evaluator that FR-531's capability inventory lists as taking a
source-checked clause, node or state clause reads that value from the
compiled package's `CheckedGraph`. No value read from bytes becomes a
`CheckedPackage`.

### Decision: the selected package governs the package route, source analysis keeps its checked graph

FR-531 states that the model-check evaluators (`evaluate_holds`,
`evaluate_clause`, `evaluate_value_invariants`, `check_separation`,
`evaluate_frame` and the others of its inventory's source-checked row) take
a source-checked value that the bytes do not carry. Requiring every engine
to read only through the view on both routes would leave the source route
with no way to call them, so an explicit-state, invariant, weak-memory or
frame item over a program would settle `unsupported` for a fact the bytes
lack while its source is in hand. Analysis from source is the primary use of
`analyze` and is never narrowed. The view therefore governs the route that
has only bytes: a `package` selection, where the bytes are the whole input.
The source route keeps the compiled `CheckedGraph` for the evaluators that
need it and reads every byte-table fact through the view, so the facts both
routes share still have one value. The package route reaches the same
evaluators through their package forms ("Package forms of the source-checked
evaluators" below), so FR-850's equality with the source route holds for
every item family. An evaluator reads package bytes only when a package is
selected and the strict reader admitted it; a selection the reader refuses is
refused by FR-850 "Admission" before any engine runs, and never falls back to
source.

### Decision: a typed read view, not an exception to R-10

FR-087 R-10 says checked typestate is built only by S3 and S4, so that every
`CheckedPackage` is a package that passed the checker. Two ways reconcile it
with analysis from package bytes: a narrow named constructor that turns an
admitted v2 package into a `CheckedPackage`, or a read view that the engines
take instead of `CheckedPackage`. The view is chosen.

A `CheckedPackage` holds its S3 `CheckedGraph`: the source arena, the
checked bodies and the source-side names and spans. The v2 bytes do not carry
these (FR-531 "What the bytes do not carry"), so a `CheckedPackage` built from
bytes would have to invent them or leave them empty. Either way it would be a
`CheckedPackage` that never passed S3, which is what R-10 exists to rule out,
and every later reader of a `CheckedPackage` would have to ask which kind it
holds. The view needs no such value. FR-531 already lists every fact
evaluation reads from a package's bytes and requires the in-process route to
read the same bytes column, so one value exists per fact. The view is that
table as a trait. The engines read only those facts, so analysis gives the same
result on both routes because both routes read the same bytes.

## Inputs

- On the source route: the `AdmittedPackageRead` that
  `read_evaluation_package` returns for the `CheckedPackage` S4 built for the
  request's program: its emission's bytes read back by the strict reader.
- On the package route: the `BarePackage` over the `CheckedPackageV2` that the
  strict checked-package reader admitted from the `checked-package.json` of
  the FR-517 handoff directory the request's `package` selection names
  (FR-850 "Admission"), with the request's domain packages bound to its model
  selections and, through `BarePackage::with_supply`, the identity rows
  `ReadIdentityRows::read` admitted from that directory's `identities.json`
  and the equality rows `ReadEqualityRows::read` admitted from its
  `equality.json` when present. These rows are the identity witness the
  reader projection (`BarePackage::world()`, the bare walker and observation
  admission) reads; they come from the handoff files only, never from the
  package bytes (FR-531).

## Outputs

```rust
// fsl_package, behind `std`
pub trait AnalysisView: sealed::Sealed {
    fn package(&self) -> &CheckedPackageV2;
    fn identity(&self) -> &LibraryName;
    fn model_selections(&self) -> Vec<DomainPackageRef>;
    fn dependencies(&self) -> DependencyClosure<'_>;
    fn claim_name(&self, name: &str) -> Option<ClaimOccurrence>;
    fn claim_names(&self) -> ClaimNameRows<'_>;
    fn requirement_items(&self) -> &[RequirementItemRow];
    fn value_validity_records_of(&self, owner: &CheckedNodeId) -> &[usize];
    // ... one accessor per fact row of FR-531 "What the entry derives from
    // the bytes", each reading `package()`.
}
impl AnalysisView for AdmittedPackageRead<'_> { /* reads `admitted()` */ }
impl AnalysisView for BarePackage<'_> { /* reads the admitted package */ }
```

`sealed::Sealed` is private to `fsl_package`, so no other type implements the
view. The engines take `&dyn AnalysisView` or a type parameter bounded by it,
and `FR-621`'s outcome document is unchanged.

## Behavior

- While a `package` is selected, every analysis engine, the default-item
  rule, the claim-name lookup, the subject binding and counterexample replay SHALL read
  the package only through `AnalysisView`, and no engine SHALL take a
  `CheckedPackage` or a `CheckedGraph`.
- While a `program` is selected, every engine SHALL read each fact of
  FR-531's table through the `AdmittedPackageRead`'s view.
- While a `program` is selected, an evaluator FR-531's capability inventory
  lists as taking a source-checked value SHALL take it from the compiled
  `CheckedPackage`'s `CheckedGraph`.
- While a `program` is selected, FSL SHALL NOT settle an item `unsupported`
  because a fact is absent from FR-531's table or from the package bytes.

### Package forms of the source-checked evaluators

- Each evaluator of FR-531's source-checked inventory row that an analysis
  engine calls SHALL have a package form that reads the clause, node or state
  clause it evaluates from the view: the formula operations, operand edges
  and `holds` atoms from `formula_graph()`, the guard, frame and invariant
  bodies from the admitted checked nodes, and the binders from the checked
  predicate view (FR-515). The package form SHALL return the value the source
  form returns for the package and the source it was compiled from.
- `AnalysisView` SHALL be implemented for `AdmittedPackageRead<'_>` and for
  `BarePackage<'_>` and for no other type; it SHALL NOT be implemented for
  `CheckedPackage`.
- The `AdmittedPackageRead` implementation SHALL read each fact from its
  `admitted()` package, never from its `source()` package's `CheckedGraph`,
  so the two implementations return equal values for a package and the bytes
  it was written to.
- No function SHALL convert a `BarePackage`, a `CheckedPackageV2` or any value
  read from bytes into a `CheckedPackage` or a `CheckedGraph` (FR-087 R-10).
- A fact an engine needs that FR-531's table does not list SHALL be added to
  that table, with the row of the v2 package that carries it, before an engine
  reads it on the package route, so the package route settles every item
  with the verdict, record, counterexample and replay the source route gives
  (FR-850 "Equality with the source route"). No package-route item SHALL
  settle `unsupported` for a fact or an evaluator the source route reads.
- The package form of an expression evaluator SHALL charge the evaluation
  meter exactly what the source form charges, entry for entry in the charge
  log, for the package and the source it was compiled from. A charge that the
  source form decides from a fact of the checked graph SHALL be decided on
  both routes from a fact the package records; where the checked graph
  holds a fact the package does not, the charge is derived from what both
  hold, as FR-397 "Ordering parts" derives the charge of an IEEE ordering
  composition's `numeric_equal` parts from the expression's shape, and is
  never read from a fact only the source route has.
- On the package route, the subject builder SHALL build an item's protocol
  subject (FR-205) from FR-531's protocol-subject row: the protocol clause
  it builds from the admitted `protocol` nodes and `protocol_locations` rows
  SHALL equal S3's FR-218 checked clause for the package and the source it
  was compiled from in every member, as FR-850-AC-8 requires of a temporal
  clause. Each static role takes its declared model or model type from its
  `role` node and is bound by resolving the request's `roles` selection, or
  the binding by construction of a role on a whole model, against the
  package's `model_root` selections, with FR-205's `wrong-role-mapping`
  refusals; the `over` universe comes from the selected-model population
  catalog row of its population, the capacity names from the clause's
  `symbolic` queue capacities, and each outermost `parallel` is resolved by FR-219 from the
  request's `memory` member and the `parallel` node's source default. An
  item over a protocol subject, and a request carrying `memory`, therefore
  settle on the package route with the source route's record, and never
  settle `unsupported` or refuse because a package is selected.

### Requirement records

- `AnalysisView` SHALL expose the package's requirement records as Contract
  IR's typed read of FSpec FR-825's `requirement_items` rows and
  `claim`/`value_validity` nodes: `requirement_items()` in package order, a
  row's index being its record's request index, and
  `value_validity_records_of(owner)`, the indices of the rows a function
  node owns. Both implementations read their admitted package, so the source
  route's `AdmittedPackageRead` and the package route's `BarePackage` return
  equal rows for a package and the bytes it was written to.
- On the package route, the settlement of a run call's items (FR-961 steps 4
  and 5) SHALL take each item from these rows: its claim, request index,
  kind and extent from the row, and its result bound, conversion context,
  path condition, extent roots and referenced object types from the row's
  record node. The FR-331 request item it writes, the candidates FR-075
  computes from that kind and extent, and the record the linked engine or
  provider settles SHALL equal the source route's for the same item. The
  linked engine evaluates the application, its guards and its bound from
  the package's nodes through the package call path (FR-945), never from a
  `CheckedGraph`, and counterexample replay of a `refuted` item calls the
  owner function through the same path, binding only the parameters the
  record's `extent_roots` name (FR-098). No item is rebuilt from the graph,
  none is dropped, and none settles `unsupported` for a fact these rows
  carry.

### Claim names

- The package's claim-name source SHALL be its name tables: FSpec FR-322's
  `temporal_clause_names` for temporal clauses and `claim_names` for state
  clauses, refinements and every non-temporal claim. `AnalysisView` SHALL
  expose `claim_name(name) -> Option<ClaimOccurrence>`, which resolves a bare
  or qualified claim name to the (`declaration`, `occurrence`) of the claim
  row of exactly one declaration in those tables (FSpec FR-322 "Rows of one
  declaration": the declaration's row on its own node, its `claim`
  occurrence row first), and `claim_names()`, the admitted rows in package
  order; the claim-name lookup, the default-item rule and counterexample
  replay of both routes read claim names through these accessors only.
- The two tables SHALL be one claim namespace (FSpec FR-322 "One claim
  namespace"): a request's `{"claim": name}` names no table, and a temporal
  clause row carries only its bare name, so a name in both tables could not
  resolve to exactly one row. Every claim-bearing declaration of a unit (a
  temporal clause, a hyper clause, a state clause, a refinement, a step
  relation, and a state-graph, probabilistic, schedulability or other
  non-temporal claim) is one name in that namespace. When a unit declares two
  such declarations with one declared name, of any kinds (two probabilistic
  claims included), S3 SHALL refuse the later
  `ambiguous_declaration`/`ambiguous-name` at its name, naming both. The
  exception is two state clauses, or a state clause and a function, which
  FR-104 refuses at every declaration of the name. A request names one
  declaration, and two declarations of one name would leave the package with
  two rows of one `qualified_name`, which the strict reader refuses (FSpec
  FR-322 "Claim declaration names"), so `claim_name` never sees one.
- The writer SHALL give every occurrence a declaration introduces the
  declaration's name, `qualified_name` and whole-declaration
  `declaration_region`. A step relation has a row for its `declaration` and
  for its `claim` occurrence. A derived conjunction node has a row for each of
  its `claim` occurrences, carrying the contributing clause's name, path and
  region (FR-105). A hyper clause has rows on its `hyperproperty` node, and
  its body wrapper has none (FR-488). Rows of one declaration are one name,
  never a duplicate.
- On the source route the `AdmittedPackageRead` SHALL read the same tables
  from its own S4 emission, so both routes resolve a name to the same
  occurrence. A claim that the source declares and the tables do not name is
  a package-writer defect the strict reader refuses (FSpec FR-322-AC-458),
  never a claim the package route silently lacks.
- No implementation SHALL derive a claim name from a node body, a source
  region, a `NodeRef` path or source text.
- A model is named on the package route by the package identity of its
  model selection, read from `model_selections()`, never by a source alias:
  the package records model selections by identity only, so untimed units
  importing one domain package under different aliases write equal bytes; a
  timed model's qualified name carries its alias (`[alias, "Timed"]`), which
  is never read as the model's name. A derived
  item over a model subject that names its model by an alias settles
  `invalid-request` `invalid_capability`/`model-alias`, naming the identity
  to use, as FR-850 "Building the analysis input" states.

### Claim occurrence ordinals

Structurally equal declarations share one node, and each is its own `claim`
occurrence of that node with its own ordinal (FSpec FR-322 "Temporal clause
names"). Neither the node nor the name alone identifies a declaration: the
node is shared, and the name is not a node-identity input. Only the whole
occurrence key (node, `claim` role, ordinal) does. The two routes therefore
agree on an item only if they number the occurrences of a shared node the
same way and key every later step on the whole key.

- The `claim` occurrences of one node SHALL be numbered per node and role,
  from 0, in the source order of the declarations that introduce them. The
  first declaration of the node in source order is its `claim` occurrence 0,
  the next is 1, and so on. A declaration's ordinal SHALL NOT depend on the
  declarations of any other node. A unit that declares `BelowThree` and then
  `AtMostTwo` with one formula gives `BelowThree` occurrence 0 and
  `AtMostTwo` occurrence 1 of their shared node.
- The source route's checked declaration (a temporal clause's
  `TemporalClauseNode::claim`, and the checked occurrence of every other
  claim-bearing declaration) SHALL carry the ordinal its emitted
  `temporal_clause_names` or `claim_names` row carries. Giving a later
  declaration of a shared node ordinal 0 would make it the first
  declaration's occurrence: two declarations would have one key, and that
  declaration's record and replay would be reported at another
  declaration's occurrence.
- Negotiation, the item's claim-family and authored-occurrence checks, the
  engines, the item record and counterexample replay SHALL key on the
  whole occurrence key that `claim_name` returns, on both routes. They
  SHALL NOT select a declaration by its node alone, for example by taking
  the first checked clause whose node matches and comparing its occurrence.
  A node that structurally equal declarations share holds one declaration
  per `claim` occurrence, so the first declaration of the node is not
  necessarily the declaration a request names.

### Package form of a state-graph claim

A state-graph claim (`possible`, `always possible`, `unique path`, FR-165)
is analyzed on the package route from the `claim`/`state_graph` node FR-410
writes, and from nothing else. That node and the rows beside it already hold
every fact the source route's `CheckedStateGraphClaim` holds, so the package
route reads them and settles the item with the source route's record.

- FSL SHALL read a package state-graph claim from these package facts:

  | Fact of the checked claim | Package source |
  | --- | --- |
  | the claim's occurrence key | its `claim_names` row's (`declaration`, `occurrence`), resolved by `claim_name` ("Claim names", "Claim occurrence ordinals") |
  | name and span | the row's `name` and `declaration_region` (FR-850 "Equality with the source route") |
  | form | the node body's `kind` binding: `possible` is `Possible`, `always-possible` is `AlwaysPossible`, `unique-path` is `UniquePath` (FR-410) |
  | `over` | the body's `over` binding: a reference to the `value`/`parameter` node, whose `Reference<T>` type projection gives the object type `T`, or the empty aggregate for no `over` |
  | each predicate | the node the body's `from` or `target` binding references: an `expression` node is a written predicate, whose term is read through the same reader projection FR-940 "Package forms of the source-checked evaluators" uses for a state clause body, with the `over` parameter at slot 0; a Boolean `value`/`literal` node `true` is the constant predicate; `from` the empty aggregate is no `from` (`Possible`) |
  | the claim's capability and requirements | the form and the `over` type, classified by FR-165's rules over the package's type environment within S3's position limit, as the source route classifies them |

  The `from` of an `always possible` claim written without `from` is the
  `true` literal node (FR-410), so it reads back as the constant predicate,
  as the source route's claim holds it.
- The package route SHALL rebuild the claim as S3 builds it, through
  `CheckedStateGraphClaim::of_package_form`, from the record
  `BarePackage::state_graph_claim_form(claim)` returns for the claim's
  occurrence key. The rebuilt claim's name, span, `over`, form, capability,
  requirements and occurrence SHALL equal the source route's checked claim
  for the package and the source it was compiled from. A written predicate
  of the rebuilt claim holds its package node key and slot count, and no
  source-checked body (FR-087 R-10).
- A state-graph predicate SHALL be evaluated through the model-check
  evaluator port keyed by the claim's occurrence key and the predicate's
  role, the FR-475 body slot that references it (`from` or `target`; the
  end predicate of a `unique path` claim is its `target`), on both routes,
  as a temporal clause's
  `holds` atom is keyed by its clause and formula position. The source route
  evaluates the checked predicate body of its `CheckedGraph`; the package
  route evaluates the predicate node's reader projection. Both run the one
  `fsl-eval` expression evaluator over its two node sources, under the same
  meter and charges, so a predicate gives the same value, `Undefined` or
  refusal at every state on both routes. Every consumer of a predicate, the
  state labels of the exploration, the partial-order reduction's atom reads,
  counterexample and witness replay (FR-170) and the state-graph
  certificate's recomputation (FR-169, and over a timed subject the zone
  certificate of FR-852 and FR-244), SHALL read the predicate through
  that port or through the predicate's reads as the port reports them, never
  through a `CheckedNode` on the package route.
- So a package state-graph item SHALL settle with the record, verdict,
  witness or counterexample, replay and certificate the source route gives
  for the same item (FR-850 "Equality with the source route"). No package
  state-graph item settles `unsupported` for a fact this section lists.
- A `claim`/`state_graph` node that FR-475's reader admitted but whose
  `over` reference names no `value`/`parameter` node, or whose predicate
  node is neither an `expression` node nor a Boolean literal, is a reader
  defect, not an analysis outcome; the package form refuses it as the
  malformed node it is (`MalformedNode` with the node key), and the item
  settles `failed` with that internal fault, never a verdict.
- A name in both name tables is refused before any state-graph package form
  is built. On the source route, S3 refuses the later of two claim-bearing
  declarations that share a name `ambiguous_declaration`/`ambiguous-name` at
  its name, naming both ("Claim names"); the whole request refuses at S3,
  exit `20`, with no item settled. On the package route, the strict reader
  refuses the first `claim_names` row whose `name` equals a
  `temporal_clause_names` row's `name`, `ambiguous_declaration`/
  `ambiguous-name` at `/claim_names/{i}/name`, naming both rows (FSpec
  FR-322 "One claim namespace"). FR-850 "Admission" turns that reader
  refusal into a whole-request refusal at the request stage: category
  refusal, the reader's code and cause, `details.path`
  `/claim_names/{i}/name`, `items` `[]`, `last_stage` `null`, exit `20`,
  before any item is resolved and before any engine runs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-940-AC-1 | For FR-850-AC-1's Counter package, each `AnalysisView` accessor returns equal values over the `AdmittedPackageRead` of the compiled `CheckedPackage` and over the `BarePackage` of the `checked-package.json` of its handoff directory, written by a `native-compile/1` request carrying the `handoff` member (FR-665), with that directory's identity and equality rows attached; an analysis entry called with either value gives the same item records for `Bounded`, `BelowThree` and `AtMostTwo`. | Test (TC-2620) |
| FR-940-AC-2 | `AnalysisView` has no implementation outside `fsl_package` (a `compile_fail` doctest implementing it for a local type fails with `E0277`, and one passing a `CheckedPackage` where `&dyn AnalysisView` is expected fails the same way), and no public function of any FSL crate takes a `BarePackage` or a `CheckedPackageV2` and returns a `CheckedPackage` or a `CheckedGraph` (source scan of every function returning either type). | Inspection (TC-2620) |
| FR-940-AC-3 | `fsl analyze` with a `package` selection no longer refuses `package` as not implemented: FR-850-AC-1's request completes with FR-850's outcome, exit 0. | Test (TC-2620) |
| FR-940-AC-4 | Over source, the Counter request with the items `Bounded`, `AtMostTwo` and the `invariant` state clause `NotAboveThree` (`self.value <= 3`, FR-625-AC-4) settles `proved`, `refuted` with `replay.reproduced` `true`, and `proved` basis `exhaustive` respectively, and the store-buffering litmus claim over `SB` with `memory` `tso` settles `refuted` (FR-629-AC-1); no record of either request is `unsupported` and no diagnostic names a package fact. | Test (TC-2958) |
| FR-940-AC-5 | The `package` request over the handoff directory of AC-4's Counter unit, written by its compile request's `handoff` member, gives, for `NotAboveThree` and `AtMostTwo`, item records equal to the source route's, `AtMostTwo` with the same counterexample, replay target and `reproduced` `true`. | Test (TC-2958) |
| FR-940-AC-6 | For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-3, FR-850-AC-6). | Test (TC-2958) |
| FR-940-AC-7 | For AC-4's Counter unit, `claim_name("NotAboveThree")`, `claim_name("Bounded")` and `claim_name("AtMostTwo")` return equal occurrences over the `AdmittedPackageRead` and over the `BarePackage` of its handoff directory, the first from a `claim_names` row and the others from `temporal_clause_names` rows; `claim_name("Missing")` returns `None` on both. | Test (TC-2967) |
| FR-940-AC-8 | AC-4's Counter unit declaring the temporal clause `Bounded` and the state-graph claim `possible Bounded` is refused `ambiguous_declaration`/`ambiguous-name` before any item is analyzed; with the claim named `ReachThree` both items settle with equal records over source and over the package, and that package with its `ReachThree` `claim_names` row renamed `Bounded` is refused `ambiguous_declaration`/`ambiguous-name` with no item settled (FSpec FR-322-AC-459). | Test (TC-2970) |
| FR-940-AC-9 | A unit declaring `probabilistic Stable ... { probability >= 0.5 [ holds(true) ] }` and `probabilistic Stable ... { probability >= 0.25 [ holds(true) ] }` is refused `ambiguous_declaration`/`ambiguous-name` at the second `Stable`, naming both; with the second named `Other` it checks, and its package carries one `claim_names` row per claim. The package of a unit whose operation has two postconditions `Q1` and `Q2` constraining one field (a derived conjunction, FR-105-AC-14) is admitted by the strict reader, and `claim_name("Q1")` returns `Q1`'s own node, never the derived node, over the source route and over the package. A step relation's package carries rows for its `declaration` and `claim` occurrences under one name and region, is admitted, and `claim_name` returns the `claim` occurrence. A hyper clause's package is admitted with its name on its `hyperproperty` node's `claim_names` row, and `claim_name` returns that node. | Test (TC-3015) |
| FR-940-AC-10 | AC-4's Counter unit declares `BelowThree` and then `AtMostTwo`, both `always holds(c.value <= 2)`, which share one `temporal_clause` node. `claim_name("BelowThree")` returns that node's `claim` occurrence 0 and `claim_name("AtMostTwo")` returns occurrence 1, over the `AdmittedPackageRead` and over the `BarePackage` of its handoff directory. Each equals the source checked clause's `claim` origin and its `temporal_clause_names` row. With the two declarations swapped in source, the ordinals swap. A `program` request and a `package` request with the items `[{"claim": "BelowThree"}, {"claim": "AtMostTwo"}]`, and with `[{"claim": "AtMostTwo"}]` alone, each negotiate every item and settle it `refuted`, with `replay.reproduced` `true`, at its own declaration's occurrence key. The item records, counterexamples and replays are equal across the two routes. No item settles `unsupported` or `invalid-request`. | Test (TC-3026) |
| FR-940-AC-11 | Over AC-4's Counter unit with the state-graph claims `possible ReachThree using v over (c: M::Counter) { c.value = 3 }`, `always possible Back ... { c.value = 0 }` written without `from`, `always possible FromOne ... from (c.value = 1) { c.value = 0 }` and `unique path Straight ... from (c.value = 0) to (c.value = 1)`, `BarePackage::state_graph_claim_form` of each claim's `claim_name` occurrence, rebuilt by `CheckedStateGraphClaim::of_package_form`, equals the source route's checked claim in name, span, `over`, form, capability, requirements and occurrence, `Back`'s `from` being the constant predicate; a `program` request and a `package` request over its handoff directory with those four items give equal item records, witnesses or counterexamples, replays and certificates, and no package-route record is `unsupported`; each predicate evaluated through the evaluator port under its FR-475 role, `from` or `target` (`Straight`'s end predicate under `target`), at each explored state gives the same value on both routes. | Test (TC-3034) |
| FR-940-AC-12 | AC-8's colliding source unit refuses at S3 with `ambiguous_declaration`/`ambiguous-name` at the second `Bounded`, naming both declarations, exit `20`, no item settled; AC-8's package with the `ReachThree` row renamed `Bounded` refuses the whole `package` request at the request stage, category refusal, `ambiguous_declaration`/`ambiguous-name` with `details.path` `/claim_names/{i}/name` of that row, `items` `[]`, `last_stage` `null`, exit `20`, and runs no engine. | Test (TC-3034) |
| FR-940-AC-13 | Over FR-531-AC-35's unit, the `AdmittedPackageRead` of its emission and the `BarePackage` of its written bytes return equal `requirement_items()` and equal `value_validity_records_of` for `f`, `f2`, `sq` and `g`, and their rows equal `CheckedGraph::requirements()` position for position. For FR-961-AC-1's `Even` unit, the FR-331 request items written on the package route from these rows (claim, request index, kind, extent, result bound, path condition, candidates) equal the source route's for the same registry, and the linked engine settles each item with the source route's record; for RR-12's `f` with `n` an `Integer`, each item is `unbounded` with one `integer` domain at `n`'s parameter node and `finite_bound_available` true on both routes. A test-only view whose record of `f`'s `+` row lacks its one guard makes the package-route linked engine's record differ from the source route's (the guard `n >= 0 and n < 10` is no longer assumed), so the comparison can fail. | Test (TC-2620) |
| FR-940-AC-14 | For FR-1033-AC-1's `SB` unit and its handoff directory, the protocol clause the package route builds for `SB` from the admitted `protocol` nodes and `protocol_locations` rows equals S3's FR-218 checked clause in every member; the subject built on each route for `subject.protocol` with `roles` `w` and `c` bound to `test/litmus` binds `w` to `M::Cell` and `c` to `M::Core` with the same selection, the same `over` universe and the same resolved model of `SB::Both` (`Sc` with no `memory`, `Tso` with the `tso` entry); a `roles` entry for `w` naming a model the package does not select settles `declined` with `invalid_runtime_input`/`wrong-role-mapping` naming `w` on both routes. A test-only package whose `SB::Both` `parallel` node carries `memory` `tso` makes the package-built clause differ from the source clause, so the comparison can fail. | Test (TC-3050) |
| FR-940-AC-15 | For FR-397-AC-30's and FR-397-AC-31's bodies, the source form and the package form of the expression evaluator, given the same inputs and remaining budget, write equal outcomes and equal charge logs; a package form that charges every `numeric_equal` node one `compare_ieee` writes a log for `p < q` with entries the source form lacks, so the comparison can fail. | Test (TC-3077) |

## Dependencies

- FR-087 R-10: checked typestate is built only by S3 and S4.
- FR-531: the `BarePackage` entry and the table of facts read from bytes.
- FR-850: the analyze request's `package` selection and its admission.
- FR-281: the analysis engines.
