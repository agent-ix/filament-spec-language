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
  - target: ix://agent-ix/filament-spec-language/FR-488
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-532
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-163
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-173
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-631
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

### Package form of a hyper clause

A hyper clause (FR-172) is analyzed from its `claim`/`hyperproperty` node,
the body wrapper that node's `body` binding references, and the nodes they
reference (FR-488), and from nothing else. The engines of FR-631 (EN-1's
product, copy-swap, witness-set, projected and single-existential checks,
settlement, replay and FR-163's product-closure checker) read the clause
through one record, the **hyper form**, which both routes read through this
view.

#### Decision: one hyper form on both routes, read from the bytes

The engines take a `CheckedHyperClause` and evaluate its atoms and `match`
conjuncts. S3's clause cannot serve the package route, and it also cannot
serve as the value the package route's rebuilt clause must equal member for
member. Three of its members are spelled by facts the bytes do not carry
(FR-531 "What the bytes do not carry"): its formula arena is numbered in
source arena order, its copy-swap normal form sorts operands by their
checked encoding (FR-174), and its atom conditions and conjuncts are
source-checked bodies. A package-route clause numbered and sorted from the
bytes would hand the engines different atom indices, a different body
automaton numbering and therefore different product-state keys, so the
certificate, counterexample letters and refusal positions of the two routes
would differ for one package.

The hyper form is therefore read from the package on both routes, as FR-531
already requires for every other fact the bytes carry: on the source route
from the `AdmittedPackageRead` of the unit's own S4 emission, on the package
route from the `BarePackage`. Both are the same bytes, so the engines read
one value per fact and FR-850's equality holds by construction. S3's
`CheckedHyperClause` keeps its role in checking and emission (FR-172 to
FR-174, FR-487, FR-488) and is never read by an analysis engine. The two
routes still differ in one place only, the body of an atom or conjunct: the
source route evaluates S3's checked body, the package route the node's
reader projection, through the one evaluator port, as for a state-graph
predicate.

The rebuilt clause is a `CheckedHyperClause` built by
`CheckedHyperClause::of_package_form`, as a state-graph claim and a temporal
clause are rebuilt. It holds package node keys and slot counts for its atom
and conjunct bodies and no source-checked body, and it is neither a
`CheckedPackage` nor a `CheckedGraph`, so FR-087 R-10 is not touched. Neither
route recompiles source to obtain it: on the package route there is no
source, and FSL never falls back to it.

#### What a hyper form receives

`AnalysisView::hyper_clause_form(claim: &OccurrenceKey) ->
Result<PackageHyperForm, BareClauseRefusal>` SHALL read the hyper form of
the clause whose `claim` occurrence `claim` is, as `claim_name` resolves it
("Claim names", "Claim occurrence ordinals"), from these package facts:

| Fact of the hyper form | Package source |
| --- | --- |
| occurrence key | the clause's `claim_names` row's (`declaration`, `occurrence`) on its `hyperproperty` node |
| name and span | that row's `name` and `declaration_region` (FR-850 "Equality with the source route") |
| profile | the `temporal_profile` law of the body wrapper's `filament.op.temporal.clause` application, read as the package's `profile_selections` row it equals |
| aliases | the `subjects` bindings, in order: each binding's name is the alias and its `model_root` reference the package identity it selects, read from `model_selections()` |
| object parameters | the `parameters` bindings in order: the name, and the object type `T` of the `model` node it references; its alias is the alias of the trace variables whose atom slots read it (S3 refuses one parameter read under two aliases); a parameter no atom slot reads takes the first `subjects` alias whose package declares `T`, and reads nothing in any engine |
| quantifier prefix | the `quantifiers` Tuple: per Group in order, `variable`, `quantifier` (`forall` or `exists`), `alias`, and the fairness set as the `temporal`/`fairness` nodes its references name, in order, each read as FR-337's fairness argument is read |
| atom slots | the `value`/`parameter` nodes named in the atom slot form `slot_s<esc(p)>_s<esc(t)>` that the wrapper's `over` and the atom terms reference (FR-488 "Atom slots as parameter nodes"), atom slot `k` being the one such node at level `k`, the wrapper's `over` reference being atom slot 0; the levels form one scope for the atom slots only, so each level from 0 to the highest names exactly one atom slot node; each slot's trace variable `t` and parameter `p` are decoded from its reserved name `slot_s<esc(p)>_s<esc(t)>` (FR-488 "Slot node names") |
| body | the wrapper's `formula` reference, read as `formula_graph()` in pre-order as FR-515 reads a temporal clause's formula, operators, intervals and operands as FR-337 writes them |
| atoms | each distinct `holds` operand node, numbered by first use in that pre-order (FR-532 "Atom index"), with its slot count; the component of each model read in it is the trace variable of the slots at the read's roots, which FR-488 "Clauses S4 does not emit" makes exist and agree for every read S4 writes, and an indexed reference literal takes the component of the read that encloses it |
| `match` conjuncts | the `match` aggregate's terms in package order; a conjunct belongs to `μ_E` when a slot node it reads names an existential variable of the prefix in its decoded reserved name (`op_s<t>`, `rcv_s<t>`, `arg_s<t>_s<T>_s<name>_s<p>`, FR-488 "Slot node names"), and to `μ_U` otherwise, a constant conjunct included; each conjunct has its own slot scope: its step-label slot nodes are numbered from level 0 in the order it reads them, independently of the atom slots and of every other conjunct, so a conjunct slot and an atom slot may both sit at level 0; its index within its part is its rank among the package-order conjuncts of that part, which is S3's index (FR-488 "Match conjuncts" writes them in source order) |
| `align skip` | the `align_skip` references in order, each `state`/`operation_anchor` node read as the operation key it anchors |
| form | FR-173's classification applied to the profile, prefix, fairness sets, `align skip` and body above |
| copy group | FR-174's copy-swap detection applied to the prefix, fairness sets, `μ_U` and body above, over the package normal form defined below ("The package normal form") |

Each row of this table is a fact of FR-531's table of facts read from the
bytes, so both implementations of the view return it ("Package forms of the
source-checked evaluators"). The record charges nothing. It reads no source text, no `source_map` region
other than the `declaration_region` of its row, no occurrence site label and
no `NodeRef` path.

#### The package normal form

S3 decides copy-swap (FR-174 "Copy-swap detection") by encoding the body,
each `holds` atom and the conjunction of `μ_U` in a normal form, once as
written and once under the swap `σ` of two universal variables, and
comparing the bytes. The source encoding names a model read by its operand
and trace (`c.value @ a`). The package term has no `@`: the read is the
atom slot node of (`p`, `t`). The term `σ(φ)` is not a term the package
holds, because renaming a trace changes which slot node a read names, so
the package normal form is computed as an encoding, never looked up as a
node key.

- The package normal form SHALL encode each term bottom-up from the
  package nodes it references:
  - An atom or conjunct slot node is encoded as its decoded pair (`p`,
    `t`), or its decoded step-label read (`op`, `rcv` or `arg` with its
    fields, and `t`). `σ` is applied to `t` before the encoding, and the
    slot's level is not part of the encoding. The level is a position in
    one term's slot list, and `σ` moves a read from one level to another.
  - Every other node is encoded by its node tag, its operation identity or
    literal value, and its operands' encodings in order. A local binder of an atom (a
    `value`/`parameter` node at a level after the atom slots, FR-488 "The
    body") is encoded by its level, which `σ` does not change.
  - The operands of `and` and `or` (expression and temporal) are
    flattened through nested `and` or `or` of the same operator. The
    operands of `and`, `or`, `=` and `!=` are then sorted by their
    encodings, after each operand is encoded under `σ` and before the parent
    is encoded.
  - The `match` conjunction is the `μ_U` conjuncts' encodings, sorted.
  - An **opaque** node is a node whose operand subtree reads a slot and
    whose form is in the opaque table below. Such a node makes the pair
    not swappable, because S3 does not normalize through the source form
    it lowers from. Every other node is encoded by its tag and normalized
    through as above. That includes the forms S3 also normalizes through:
    `if`, `call`, field select, index, `value`, `deref`, `ref`, `present`,
    `size`, `contains`, `drop first`, `remove one`, `pre`, negation,
    `not` and every binary operator.

    | S3 opaque source form (FR-174, `is_opaque`) | Package node it lowers to (FR-093) |
    | --- | --- |
    | `case` | `filament.op.control.case` |
    | `let` | `filament.op.control.let` |
    | record construction | the record `value` node S4 writes for the construction |
    | collection literal | the constructor application `filament.op.collection.<kind>` of its collection kind |
    | query (`map`, `filter`, `forall`, `exists`, `count`, `only`, `sum`) and the aggregate `count` and `sum` | `filament.op.collection.map`, `.filter`, `.forall`, `.exists`, `.count`, `.only`, `.sum.<carrier>` |
    | `flatten` | `filament.op.collection.flatten`, `filament.op.collection.flat_map` |
    | `accumulate` | `filament.op.collection.fold` |
    | `seq` | `filament.op.collection.map` over an `integer_range` bounded domain |
    | `lookup`, all instances | `filament.op.model.lookup` |
    | `dispatch` | `filament.op.model.dispatch_call` |
    | power | `filament.op.quantity.pow` |
    | `reaches` | `filament.op.model.reaches`, `filament.op.model.reaches_field` |
    | population, quantity literal, reference literal, protocol reference | leaf nodes with no operand (a population reference, a quantity `literal`, a reference literal), which never hold a slot read and so are never opaque |
    | `convert` | none: see the next item |

  - **Conversions are normalized through on both routes.** S3 treats an
    explicit `convert` as opaque, but an implicit conversion that S3 types
    into an equality operand (FR-397) is not an S3 expression node, so S3
    normalizes through it. Both lower to the same package conversion
    applications: `filament.op.numeric.convert`,
    `numeric.convert_rounding`, `numeric.narrow`, `quantity.convert`,
    `collection.convert`, the `ieee.to_*`, `ieee.from_exact` and
    `<family>.to_<kind>` text conversions. So the package cannot tell the
    two apart. A conversion is a total, deterministic function of its one
    operand, so exchanging two traces commutes with it, and normalizing
    through it is sound. The package normal form therefore encodes every
    conversion by its tag and normalizes through it. A clause whose only
    obstacle to a swap in S3 is an explicit `convert` over a slot read
    therefore has a larger copy group on the rebuilt clause than in S3's
    record. The engines read the rebuilt clause on both routes, so the two
    routes still agree.
  - S4 SHALL NOT lower a node that S3 normalizes through to an application
    of the opaque table, and SHALL NOT lower a source form of the opaque
    table to any other form, so the table decides opacity from the package
    alone.

- Two operands then have equal encodings exactly when the source route's
  operands have equal checked encodings, because both encodings are
  injective on structurally equal terms and each read of `x @ t` is a slot
  of (`x`, `t`). The swappable relation, the classes and each generator's
  atom exchange (equal encodings paired in atom-index order) are therefore
  S3's, with S3's atoms renumbered by FR-532's atom index. The one exception
  is the conversion rule above.
- Worked example. AC-16's `Equal` has atom slots `slot_sc_sa` (level 0)
  and `slot_sc_sb` (level 1) and one atom,
  `and(<=(value(slot(c, a)), 3), <=(value(slot(c, b)), 3))`. Under
  `σ = (a b)` the operands encode as `<=(value(slot(c, b)), 3)` and
  `<=(value(slot(c, a)), 3)`. Sorting restores the written order, so the body
  and atom encodings equal the plain ones. `a` and `b` form one class, with
  the generator `{first: 0, second: 1, atoms: [0]}`. For the body
  `always holds(c.value @ a <= 3)` alone, `σ` turns `slot(c, a)` into
  `slot(c, b)`, the encodings differ, and there is no class.

#### What a hyper form returns

- `CheckedHyperClause::of_package_form(&PackageHyperForm, types)` SHALL
  return the clause the engines read, with `occurrence()` the form's key,
  `condition(atom)` returning `None` for every atom (no source-checked body),
  and the atom node key and slot count of each atom and the node key of each
  conjunct in their place.
- On both routes the analysis entry SHALL resolve a hyper item by
  `claim_name`, read the form through the route's view and rebuild the
  clause, and SHALL NOT take the clause from `CheckedGraph::hyper_clauses()`.
- The rebuilt clause's name, span, profile, aliases, object parameters that
  an atom slot reads, quantifier prefix with fairness sets, atom slots,
  number of conjuncts in each part, `align skip`, form, copy-group classes
  and occurrence SHALL equal S3's checked clause for the package and the
  source it was compiled from. Its formula arena, atom indices and copy
  generators SHALL equal S3's after S3's atoms are renumbered by FR-532's
  atom index. The copy group is equal to S3's by "The package normal form", except for a clause that an explicit `convert` over a slot read alone keeps from a swap in S3.
- `CheckedHyperLetters::new`, `CheckedStepMatch::new` and
  `ProductClosureRequest` SHALL take the route's `ModelEvaluator` in place of
  a `CheckedPackage`, and SHALL read reference literals, component reads and
  atom slots from the rebuilt clause, never from a `CheckedGraph`.
- `ModelEvaluator::evaluate_hyper_holds` and `evaluate_step_predicate` SHALL
  be keyed by the clause's occurrence key and the atom index or the
  (`part`, conjunct index) pair. The package implementation evaluates the
  atom or conjunct node's reader projection, its slots bound to the slot
  nodes at their levels. The source implementation evaluates S3's checked
  body of the clause whose `assembled_occurrence()` is that key, at the atom
  whose condition S4 lowered to that atom node, never by arena position.
  Both run the one `fsl-eval` expression evaluator under the same meter and
  charges ("Package forms of the source-checked evaluators"), so an atom or
  conjunct gives the same value, `Undefined`, refusal or charge log on both
  routes.
- FR-163's checker SHALL rebuild the clause from the same view the item was
  analyzed through, recompute successors through that route's
  `ModelEvaluator`, and read nothing from the engine's search but the
  certificate. On the package route it reads no source.
- So a hyper item SHALL settle on the package route with the verdict,
  record, counterexample (every copy's trace, the exhausted witness set),
  copy-by-copy replay and certificate the source route gives for the same
  item (FR-850 "Equality with the source route"). The package route SHALL NOT
  settle a hyper item `unsupported`, or refuse it or its request
  `unsupported_construct`/`not-yet-implemented`, for any fact this section
  lists.

#### What changes on the source route

Reading the hyper form on the source route changes what that route's
hyper engines produce, and leaves S3 and S4 unchanged.

- Unchanged:
  - the emitted package bytes, every node key and the package identity;
  - every verdict and record;
  - every counterexample's traces;
  - FR-163's product-state counts and rule outcomes.
- Changed:
  - atom indices, which follow FR-532's atom index in place of S3's arena
    order;
  - the body automaton's state numbering, built from the package normal
    form;
  - the product-state keys that hold those automaton states;
  - the atom indices a refused or undefined letter names;
  - the bytes of a product-closure certificate.
- A fixture that pins any changed value is regenerated from the new form. A
  fixture that pins the old value is not kept beside it.

#### Refusals

- A `package` selection whose bytes the strict reader refuses, or whose
  directory holds no `identities.json`, refuses the whole request at the
  request stage (FR-850 "Admission"): category refusal, the reader's code,
  cause and pointer, `items` `[]`, `last_stage` `null`, exit `20`, and no
  engine runs and no source is compiled. This covers every `hyperproperty`
  node shape FSpec FR-402 "Reader order" refuses: for example, a
  `behaviours` node whose `body` binding references no
  `temporal`/`temporal_clause` node is refused `ill_typed`/
  `operator-ineligible` at that node, and a wrapper with a non-empty fairness
  argument `invalid_semantic_graph` at the argument. `hyper_clause_form`
  never sees such a node.
- An item whose occurrence `hyper_clause_form` finds no `claim`/
  `hyperproperty` node at, after `claim_name` returned it, is a view defect:
  the lookup returns `hyper-root-unresolved` at the triple (FR-488 "Outputs")
  and the item settles `failed` with that internal fault.
- The strict reader checks the `hyperproperty` aggregate's layout, its
  subjects and quantifier Groups, its body wrapper's kind and fairness, but
  not the slot nodes the wrapper and the `match` terms read. A
  `hyperproperty` node the reader admitted whose slot nodes do not read as
  FR-488 writes them is a writer defect, not an analysis outcome:
  `hyper_clause_form` refuses it `MalformedNode` with the node key of the
  `hyperproperty` node, and the item settles `failed` with that internal
  fault, never a verdict. The cases are:
  - an atom slot node (the wrapper's `over` or a free parameter of an atom
    term) whose `name` is not in the form `slot_s<esc(p)>_s<esc(t)>`, or
    decodes to a trace variable no quantifier Group declares or a parameter
    no `parameters` binding names;
  - two atom slot nodes at one level, or a level below the highest with no
    atom slot node;
  - a model read in an atom whose root slots name two trace variables;
  - a conjunct slot node whose `name` is in no step-label form of FR-488
    "Slot node names", or decodes to a trace variable no Group declares.
- A clause FR-173 places in HP-4 settles `unsupported` V-8
  `unsupported-requested-capability` at negotiation (FR-173) on both
  routes, because the classification reads the same form.

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
| FR-940-AC-16 | Over AC-4's Counter unit with the hyper clauses `Equal` (`forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b <= 3) }`, universal, copy-swap symmetric), `Partner` (forall-exists equality), `CanReachThree` (single existential) and a projection-aligned clause with an `align skip` list and a `match` block holding one universal and one existential conjunct, `hyper_clause_form` of each clause's `claim_name` occurrence returns equal records over the `AdmittedPackageRead` and over the `BarePackage` of its handoff directory, and the clause `of_package_form` rebuilds equals S3's checked clause in name, span, profile, aliases, read object parameters, quantifier prefix with fairness sets, atom slots, conjunct count per part, `align skip`, form, copy-group classes and occurrence, and in formula arena, atom indices and copy generators after S3's atoms are renumbered by FR-532's atom index; `condition` of every rebuilt atom is `None`. `Equal`'s rebuilt copy group is the class `[a, b]` with the generator `{first: 0, second: 1, atoms: [0]}` (FR-940 "The package normal form"), and that of a clause whose body is `always holds(c.value @ a <= 3)` alone is empty, on both routes. A test-only view that files the existential conjunct under `μ_U` makes the rebuilt clause differ from S3's, so the comparison can fail. | Test (TC-3126) |
| FR-940-AC-17 | A `program` request and a `package` request over the handoff directory of AC-16's unit, written by its compile request's `handoff` member, with the items `Equal`, `Partner`, `CanReachThree` and the projection-aligned clause, and the same items over the unit with `Equal`'s bound lowered to `c.value @ b <= 2` so it refutes, give equal item records, counterexamples (one trace per copy; for `Partner` the exhausted witness set), copy-by-copy replays with `reproduced` `true` and product-closure certificates accepted by FR-163's checker; `Equal` settles `proved` with certification `certified` on both routes; no record is `unsupported` and no diagnostic is `not-yet-implemented`. Each atom and conjunct evaluated through the `ModelEvaluator` at each explored joint state gives the same value and charge log on both routes. | Test (TC-3127) |
| FR-940-AC-18 | AC-17's `package` request run after the unit's source file is overwritten with bytes that are not a native unit and the handoff directory's `sources/` is deleted settles `Equal` `proved` with the record of AC-17's source request; FR-163's checker run on that route accepts the certificate with no source present. | Test (TC-2958) |
| FR-940-AC-19 | Test-only packages of AC-16's unit pass the strict reader, because it reads no slot node names or levels. Four variants change `Equal`: its slot node `slot_sc_sb` is renamed `slot_sc_sz` (`z` is declared by no Group); its slot node `slot_sc_sb` is written at level 0 in place of level 1, leaving two atom slot nodes at level 0 and none at level 1; the atom's `c.value` read is rewritten so its operand roots in both `slot_sc_sa` and `slot_sc_sb`; or a conjunct slot of the projection-aligned clause's `match` is renamed `opx_sa`. In each variant, `Equal` (or the projection-aligned clause in the last) settles `failed` with `MalformedNode` naming its `hyperproperty` node key and no verdict, while a sibling `Partner` item settles as in AC-17. The unmodified projection-aligned clause has one atom slot and one conjunct slot at level 0, and its item settles as in AC-17, so a level rule that does not separate atom slots from conjunct slots fails. A handoff directory whose package's `Equal` `body` binding references the atom's `expression` node, and one whose `quantifiers` Tuple is removed, each refuse the whole request at the request stage with the reader's code, cause and pointer (`ill_typed`/`operator-ineligible` at the `hyperproperty` node for the first), `items` `[]`, `last_stage` `null`, exit `20`, compiling no source. | Test (TC-3127) |
| FR-940-AC-20 | AC-4's Counter unit, extended with a record field `pos` and a sequence field `log` and with the function `f`, declares six two-trace universal clauses under the infinite-trace profile. The first three use only forms S3 normalizes through: `Called`, `always holds(f(c.value @ a) = f(c.value @ b))` (call); `Selected`, `always holds(c.pos.x @ a = c.pos.x @ b)` (field select); and `Indexed`, `always holds(c.log[0] @ a = c.log[0] @ b)` (index). Each has the copy-group class `[a, b]` in S3's record and in the clause rebuilt from the package. A `program` request and a `package` request with copy-swap `symmetry` settle each `proved` with basis `reduced`, certification `certified` and equal certificates. `LetBound`, `always holds((let v = c.value @ a in v <= 3) and (let w = c.value @ b in w <= 3))`, and `CaseBound`, the same body with each conjunct a `case` over a union field, have an empty copy group in S3's record and in the rebuilt clause, and settle `proved` with basis `exhaustive` on both routes. `Converted`, `always holds(convert(c.value @ a) = convert(c.value @ b))` with an explicit rational conversion, has an empty copy group in S3's record and the class `[a, b]` in the rebuilt clause, and settles `proved` with basis `reduced` and equal certificates on both routes. A test-only package normal form that treats `filament.op.function.call` as opaque gives `Called` an empty copy group, so the comparison can fail. | Test (TC-3126) |

## Dependencies

- FR-087 R-10: checked typestate is built only by S3 and S4.
- FR-531: the `BarePackage` entry and the table of facts read from bytes.
- FR-850: the analyze request's `package` selection and its admission.
- FR-281: the analysis engines.
