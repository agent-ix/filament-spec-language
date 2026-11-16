---
id: FR-088
title: "S-3b: frame and clause identity, clause kind, qualified names, and type descriptors"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-005
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: traces_to
---
# FR-088: S-3b: frame and clause identity, clause kind, qualified names, and type descriptors

## Description

ADR-013 §7 slice **S-3** has no owning FR. QSL-158's comments
split S-3 into S-3a (FR-087: T-1, T-3, O-15) and **S-3b** (this
requirement): O-08 (frame identity), O-09's clause half (obligation
identity is CG's own conformance work, out of this requirement's scope),
O-10 (clause kind), O-11 (qualified names), O-14 (type descriptors) and
C-26 (checked type node → kernel `ValueType`). S-3b does not gate ADR-011 M-4
(#242), so it can overlap that work; it depends on the S-3a
types this requirement's cited types are checked-graph members of
(`CheckedGraph`, FR-087), but does not depend on `ImportView` or
`library`.

This requirement's gate is ADR-013 §7's "S-2, QC-10" for S-3 as a whole
(clear: S-2 landed as PR #260; QC-10 landed under STD-2, Done —
see FR-087's Description for the verification detail) plus S-3a's
`CheckedGraph` type (FR-087), since every object this requirement owns is a
member of that checked graph.

Three of this requirement's five owned items carry an ADR-013 instruction to
turn prose into tests, not to restate the prose as the acceptance criterion:

- O-15 (FR-087's, cited here only because the forbidden-construction
  evidence rule also governs this requirement's own types): "`compile_fail`
  tests for every forbidden construction."
- O-10: "Wire-string totality tests in both directions; mutation tests on
  each mapping."
- C-26: "test per type-node form, including a sum."

Acceptance Criteria FR-088-AC-4 (O-10) and FR-088-AC-9/AC-10 (C-26) enumerate
the concrete forms these three instructions cover, rather than repeating the
ADR's summary sentence as if it were itself testable.

## Inputs

- ADR-013 O-08 (frame identity), O-09 (clause half only; the obligation half
  is CG conformance work, ADR-013 §7 lists no ticket for it), O-10 (clause
  kind), O-11 (qualified names), O-14 (type descriptors), C-26 (checked type
  node → kernel `ValueType`), O-04 (checked node identity, pre-existing:
  S-1/S-2), O-06 (member identity, pre-existing: S-2), O-07 (source
  occurrence identity, ADR-013 §3 O-07: occurrence key disambiguates two
  structurally identical clauses; the full occurrence-key-keyed source map
  is S-4's, this requirement uses only the key shape).
- ADR-013 §6 lane-private table: `syntax::ClauseKind` (native-v1, canonical
  owner O-10); native-v1's use of `ir::SymbolName` (canonical owner O-11);
  `checking::types::NativeType` and native-v1's use of `ir::ValueType`
  (canonical owner O-14).
- ADR-013 §8 QC-15 (the kernel `Value`/`ValueType` component types
  this requirement's sum shape depends on, already added to the kernel row
  by S-1/S-2: `VariantId`, an opaque node-key newtype with one public
  constructor from a node key, no dependency on `check` or `model`).
- ADR-013 O-14, T-6, C-30, QC-22 and the OQ-B, OQ-D and OQ-F rulings: an
  enum variant's `VariantId` is its FSpec FR-141 enum member node key, an
  enum value carries its `VariantId` and its rank in the FR-141 canonical
  member list, and the rank is its FSpec FR-144 canonical key; a quantity
  type's `UnitId` is a two-arm record: a declared unit's FSpec FR-142 node key, or a compound unit's ordered (unit node key, exponent) factors.
- The layer-3 `check` core module ADR-011 §6.1 places the clause-kind enum
  in (`CheckContext`, family checker trait, shared checked types,
  `FamilyOutcome`, `FamilyResult`, `EvalOutcome`).

## Outputs

- One closed checked clause-kind enum, defined once in the layer-3 `check`
  core (O-10). `syntax::ClauseKind` is untouched and gains no new variant
  (it stays lane-private).
- Frame identity (O-08): the checked node id of the `state` node with
  `semantic_form: "frame"`, plus the resolution of its FR-340
  `modifies`/`creates`/`deletes` node-id sets to their `DeclarationKey`s
  through the model correspondence (O-04). This requirement implements the
  identity and the resolution step only; FR-340's frame semantics
  themselves are #210's. Amended by FR-105: a `modifies` entry
  naming a field is the pair (declaring `object_type` node, field name),
  not a node of its own (ADR-013 O-06, FR-094); it resolves by the object
  type node's `DeclarationKey` and the member name.
- Clause identity (O-09, clause half): the checked node id of the `claim`,
  `temporal`, `protocol` or `state`/`state_clause` node (the last added by
  FR-105), with the O-07 occurrence key available to
  disambiguate two structurally identical clauses at different source
  occurrences. This requirement does not build `KaniObligationIdentity` or
  any CG-side type.
- Qualified names (O-11): the `QualifiedName` type (a non-empty sequence of
  identifiers), and the checker's own name → node id resolution function.
  No name → identity lookup function is exposed for use after the check
  stage (R-06); the one documented exception (the `replay` facade's E9
  lookup of the executor's `QualifiedName`, OQ-5) is S-3a/`replay`'s own
  scope, not built here, though this requirement's `QualifiedName` type is
  what that lookup resolves against.
- Type descriptors (O-14): package types as checked graph nodes
  (`scalar_type`, `composite_type`, `bounded_domain`), identified by node
  id; `ValueTypeRef{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String), Resolved(Box<ValueType>)}`
  for model field types, `Resolved` only for a source object member (FR-569); the sum-type checked node form, whose variants are
  declared members (O-06) and whose kernel counterpart is the QC-15 sum
  shape (opaque `VariantId`s, no `NodeKey`).
- C-26: the total conversion function, checked type node → kernel
  `ValueType`, owned by the FSL checker, covering every checked type-node
  form including the sum form.
- The source spelling registry (`SourceSpellings`) on a source model's
  `AdmittedModel`, the context-aware formatter `fsl_spelling`, and
  `ValueTypeRef`'s `Display` as a non-user-facing identity form.

## Behavior

### One closed clause-kind enum; each downstream layer maps to it, never from it

FSL SHALL define exactly one clause-kind enum in the layer-3 `check` core.
Every conversion out of it (FSL → v2 wire strings; v2 → IR `ClauseKind`; IR
→ RT observation kind; IR → CG obligation kind) is each layer's own owned
mapping (ADR-013 O-10), not built by this requirement, but this
requirement's own FSL → v2 direction SHALL be total with no `_` arm: every
variant of the checked clause-kind enum maps to exactly one v2 `node_tag`/
`semantic_form` pair or clause operation identity, and no v2 string this
enum's wire spelling admits maps to zero or more than one variant.

Amended by FR-105. The enum gains `Invariant`, `Constraint` (FR-158),
`Precondition` and `Postcondition`. All four spell the pair (`state`,
`state_clause`) and the clause operation `filament.op.state.clause` (STD-111);
their application's kind member tells them apart, so for these four the
injective spelling is the
triple (pair, operation identity, kind member), and a decoder reads the kind
member after the operation identity. `StateTransition` spells
(`state`, `transition`), its FSpec form as STD-111 item 5 defines it (a
`filament.op.state.transition` body root), not (`state`, `frame`). The TC-250
fixed table (`fsl-semantics/src/check/identity.rs:619-631`), which asserts
(`state`, `frame`) today, is updated with this change. The pair
(`state`, `frame`) decodes to no clause kind: a frame node's body is FSpec
FR-340's frame term, which holds no clause application, so a frame is never
read as a clause. Frame identity (O-08) is unchanged by this: a frame is
identified by its own `state`/`frame` node id, whatever operation anchor
references it.

### Two identities resolve through the model correspondence, never by search

Frame identity's `modifies`/`creates`/`deletes` sets (O-08) and any other
lookup from a checked node id to its `DeclarationKey` in this requirement's
scope SHALL resolve only through the model correspondence: the S3 checker
records it on `CheckedGraph`, its own stage output (FR-087 T-1), and the S4
link step carries it, unchanged, into the `CheckedPackage` it builds from
that `CheckedGraph` — so a layer-4 or layer-5 consumer reads the
correspondence from the `CheckedPackage` it holds (ADR-013 O-04's own text:
"Consumers read the correspondence from the `CheckedPackage`"), without
`check` itself ever naming `CheckedPackage` (FR-087-AC-9). No consumer of
this requirement's types SHALL search a collection whose order no
declaration defines to find a matching node (R-05).

### Clause identity is the node id; the occurrence key, not the node id alone, disambiguates

FSL SHALL identify a clause by the checked node id of its `claim`,
`temporal` or `protocol` node. For two occurrences of a structurally
identical clause (same node id, since node ids are content-addressed,
ADR-013 O-04), FSL SHALL distinguish them only by their O-07 occurrence key
(node id, role, ordinal), and SHALL NOT distinguish them by display text,
source order, or collection iteration order (R-05). This requirement
exposes the occurrence key as an input to the identity, not as a field
CG's `KaniObligationIdentity` computes independently a second time.

### Names resolve to node ids only inside the checker; no later lookup exists

FSL SHALL resolve a `QualifiedName` to a node id only inside the check
stage. This requirement SHALL NOT expose a name-resolution function whose
signature would let a post-check module (a stage after S3, a backend
repository, or a CLI command) look up a node by name (R-06). FSL SHALL
treat a `QualifiedName` only as a declared component of an identity
preimage (used, for example, in `DeclarationKey`'s or the checked node's own
preimage where a domain package's declared name participates), and SHALL
NOT treat it as an identity in its own right.

### Package types are checked nodes; the kernel shape is a separate, evaluation-only representation

FSL SHALL identify a package type (`scalar_type`, `composite_type`,
`bounded_domain`) by its checked node id (record, tuple and union identity
follow FR-143-AC-6). The kernel `ValueType` (already extended with the sum
shape by S-1/S-2's QC-15 work) is the type's evaluation-time shape, produced
from the checked node by C-26; it is not itself identity-bearing the way
the checked node id is (ADR-013 O-14: "Equality | normalized (node id) ...
Semantic (structural) for kernel `ValueType` during evaluation").

### C-26 is total, and a sum keeps its node id and its variant identities

FSL's checker SHALL convert every checked type-node form to the kernel
`ValueType` with no `_` arm: a source checked type node with no matching
arm is a compile error, not a runtime refusal. For a sum-type checked node,
the conversion SHALL preserve both the node's own id (unchanged, since the
node id is never re-minted by a conversion) and each variant's `VariantId`
(computed from the declaring sum and its member, QC-15); the resulting
kernel sum shape SHALL carry no `NodeKey`, and a sum value produced from it
SHALL carry its `VariantId`, never a variant index in its place (ADR-013
O-14 "Sum types").

### An enum value carries its FR-141 member key and its canonical rank

For an enum, each variant's `VariantId` SHALL be the enum member node key
specified by FSpec FR-141 (ADR-013 O-14, C-30). The kernel enum shape SHALL
carry the canonical member list in FSpec FR-144 order, and an enum value
SHALL carry its `VariantId` and its rank, the variant's index in that list.
Admission SHALL refuse a value whose rank disagrees with its `VariantId`'s
index in the shape. Identity and equality of an enum value SHALL use its
`VariantId` only, and its canonical key SHALL be its rank. C-30 SHALL admit
only an enum member node key as a `VariantId`.

### A quantity type carries a two-arm UnitId

For a quantity type, C-26 SHALL produce a `UnitId` that is the declared
unit's FSpec FR-142 node key under the `filament.checked-semantic-node/v1`
label, and `semantic_value` SHALL produce the ordered (unit node key, exponent) factor record
for a compound unit (ADR-013 T-6, OQ-B). A
declared unit and a compound unit SHALL never be equal. C-30 SHALL admit
only a unit node key as a declared-arm `UnitId`.

### A model field type names a package node, except a source object member's

A model field type SHALL be a `ValueTypeRef`, the union of five variants:

- `Clock`, a place in a timed model (FR-230), with no fields;
- `Native(NativeValueType)`, a native value type;
- `Package(DeclarationKey)`, a type or object type node of a domain package,
  or a source object type (FR-569);
- `Unit(String)`, a unit name the assembler resolves (FR-056-AC-42), not a
  type name;
- `Resolved(Box<ValueType>)`, the checked value type of a field, operation
  parameter or operation result of a source-declared object type.

A domain package declares every type its members name, so its members use
the first four variants only. A source object's members are typed by the
unit's own source declarations (enums, records, tuples, unions, aliases), by
anonymous types such as `Decimal[0, 100; 2, 2; exact]`, `Float64` or
`Quantity<m>`, which have no domain package node, and by model-owned value,
record, tuple and variant types such as `M::Total`. Every such member's type,
model-owned ones included, is the `ValueType` the same type form has as a
source record field (FR-569 "Member types", FR-092-AC-21), carried as
`Resolved`, so every member that is neither a clock nor an object reference
has one representation. Copying that type
into domain package records would give one type two identities and would
need record kinds FSpec FR-154 does not define, so the checked type itself is
carried instead.

No serializer or wire writer SHALL encode `Resolved`. No production code
outside the assembler's source model (FR-569 "The source model") SHALL
construct it; tests may. Domain package intake SHALL never produce it, and no domain package, snapshot, `identities.json` or
package document SHALL carry it. User-facing text SHALL print it through the
source spelling formatter, never through `Display` or `Debug` (next section).
In the FR-569
target emission, the binding of a `Resolved` member SHALL name the type node
that a record field of that `ValueType` binds; the reference itself is never
serialized.

### A `Resolved` type is spelled through the source spelling registry

A kernel `ValueType` does not hold the names a source author wrote:
`Composite` holds an opaque `NodeKey`, `Enum` holds opaque `VariantId`s,
`Reference` holds an `EffectiveId`, `Quantity` holds a `UnitId`, and an alias
is gone once its target is checked. The normalized records of the source model
hold none of these names either. Two parts therefore print a `Resolved` type:
a registry that keeps the names, and a formatter that reads it.

**The registry.** The assembler SHALL build one `SourceSpellings` registry for
a source model (FR-569 "The source model"), at the same time it fills the
members' types. The source model's `AdmittedModel` SHALL keep the registry
beside the normalized records, and every model built by this assembler SHALL
carry one. A model admitted from a domain package carries an empty registry,
because it holds no `Resolved` type. The registry SHALL hold:

- `NodeKey` → FSL name, for every record, tuple and union declaration that a
  `Composite` in the view names. A declaration of the unit is spelled by its
  declared name (`Point`), and a model-owned declaration by its selection
  alias and name (`M::Total`);
- enum declaration `NodeKey` → FSL name, for every enum whose `VariantId`s an
  `Enum` in the view holds. A variant is spelled by that name and the
  `VariantId`'s case;
- `EffectiveId` → FSL name, for every object type a `Reference` in the view
  names (`Order`, `M::Node`);
- declared-unit `NodeKey` → FSL unit name, for every unit a `UnitId` in the
  view names (`m`, `Config::cm`);
- member slot → alias tree, for every `Resolved` member type whose source
  spelling names an alias at any depth. A member slot is a field's
  declaration key, an operation's declaration key and parameter position, or
  an operation's declaration key and its result. The alias tree has the shape
  of the member's `ValueType`, and each node holds the alias name written at
  that position, or none. When an alias names the outermost collection of a
  member, the collection becomes the member's multiplicity (FR-569), so the
  alias is not part of the `value_type` and no tree node holds it.

The registry is not identity-bearing. It does not take part in any key,
equality, hash, conformance decision or serialized record, and two models
whose spellings differ check, admit and evaluate identically.

**The formatter.** `fsl_spelling(value_type, spellings, slot)` SHALL return a
`ValueTypeRef`'s FSL spelling as a pure function of its three inputs; `slot`
is optional. For `Clock`, `Native`, `Package` and `Unit` it SHALL return
exactly the text `Display` returns. For `Resolved` it SHALL print each node of
the `ValueType`. A node whose alias tree entry holds an alias prints the alias
name and nothing below it. Every other node prints by its form:

| `ValueType` form | Spelling |
| --- | --- |
| `Boolean`, `Integer`, `String`, `Uuid`, `Timestamp` | the type's name |
| `Int` | `Int[lo, hi]` |
| `Rational` | `Rational[n1, n2; d1, d2]` |
| `Decimal` | `Decimal[lo, hi; smin, smax; mode]` |
| `Float` | `Float32[mode]` or `Float64[mode]` |
| `Text` | `Text[min, max; profile]` |
| `Quantity`, declared unit | `Quantity<u>`, `u` the unit's registry name |
| `Quantity`, compound unit | `Quantity<u1^e1*u2^e2>` in the `UnitId`'s factor order, with each unit's registry name and every exponent, `^1` included, so it never equals a declared unit's spelling |
| `CarriedQuantity` | `Quantity<u, C>`, `C` the carrier's spelling |
| `BoundedQuantity` | `Quantity<u>[min, max]`, the bounds as quantity literals |
| `Enum` | the enum's registry name |
| `Option` | `Option<T>` |
| `Composite` | the declaration's registry name |
| `Collection` | `Sequence<T>`, `Set<T>`, `Bag<T>` or `OrderedSet<T>`, followed by `[min, max]` when the type is bounded |
| `Map` | `Map<K, V>`, followed by `[min, max]` when the type is bounded |
| `Reference` | `Reference<T>`, `T` the object type's registry name |
| `Population` | the identity form below, `resolved:Population[N]`, or `resolved:Population` with no maximum (a fallback, not a spelling) |

Each nested type is spelled by the same rule, so
`Option<Sequence<Money>[0, 3]>` keeps the alias `Money`. With no slot, or a
slot that has no alias tree, no node prints an alias. If the registry has no
entry for a key the type holds, the formatter SHALL print that subterm in the
identity form below; assembly is required to leave no such key (FR-088-AC-13).

A `Population` node prints its identity form for the same reason. The
kernel `ValueType::Population` keeps only its maximum `N`, not its member
type `T`, and the registry has no population entry, so `T` cannot be named.
The formatter SHALL NOT reconstruct `T` from any other input. The fallback is
never user-facing for an assembled member, because the assembler refuses a
source object member typed `Population<..>` (FR-569-AC-13), so no `Resolved`
member type of a source model holds a `Population` node; the formatter prints
it only for a type it is handed directly.

**`Display` is an identity form.** `ValueTypeRef`'s `Display` takes no
registry, so it cannot print FSL names. For `Resolved` it SHALL print a
deterministic identity form: `resolved:` followed by the table's spelling,
where each name the registry would supply is replaced by the key's canonical
text (a `NodeKey` by its `Display`, a variant by its owner `NodeKey` and case,
an `EffectiveId` by its declaration keys in chain order, a compound unit by
its (unit node key, exponent) factors), and a `Population` node is
`Population[N]`, or `Population` with no maximum. It SHALL never print a Rust `Debug`
rendering. The form is for logs, test failure text and internal keys only. It
is not user-facing and is not a stable interface, and no user-facing output
SHALL contain it.

**Outputs that SHALL use the formatter.** Every user-facing text that names a
`ValueTypeRef` held by a source model SHALL get the text from `fsl_spelling`,
passing the member slot when the text is about a member's declared type. This
covers diagnostic and refusal messages from S3 to S6 (for example the
operation member type message of model normalization) and CLI and inspect
output.

**Emitted rows do not use the formatter.** A FSpec FR-322 `resolved_path`
row is checked package data: the reader joins it to its path step and the
selected declaration without reopening source, and the row's `member_type`
is the next row's `declaring_type`. Its text is therefore a function of the
path and the selected declaration, and it SHALL take no input from the
registry. The `member_type` of a row whose member is `Resolved` SHALL be the
FR-322 identity rendering of the type node that the member's emitted binding
names (the section above), which is the same text a record field of that
`ValueType` gets in its row. It SHALL never come from `fsl_spelling`,
`Display` or `Debug`, so it holds no `resolved:` text and no `Debug` text.
Replacing a registry entry changes no byte of the emitted package.
Code that reads a `resolved_path` row's collection element SHALL take it from
the `ValueType`, not by parsing the rendered text.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-088-CON-1 | This requirement builds the clause identity (the checked node id and its occurrence-key disambiguation) only. It does not build `KaniObligationIdentity`, the CG obligation-kind mapping, or any CG-owned type; those are CG conformance work with no #213 ticket (ADR-013 §7). | Design | Inspection |
| FR-088-CON-2 | This requirement does not implement FR-340's frame semantics (the meaning of `modifies`/`creates`/`deletes`); it implements only the identity and the resolution of those sets' node ids to `DeclarationKey`s. Frame semantics are #210's (ADR-012 §12.2), specified by FR-114 to FR-116. | Design | Inspection |
| FR-088-CON-3 | This requirement does not build the occurrence-key-keyed source map itself (that is S-4, ADR-013 O-07/O-12); it uses only the occurrence-key shape (node id, role, ordinal) as an input to clause identity. | Design | Inspection |
| FR-088-CON-4 | This requirement's `QualifiedName` type and the checker's name-resolution function do not implement the `replay` facade's E9 lookup (OQ-5); that lookup is `replay`'s own module (layer 6), built separately, though it resolves against this requirement's `QualifiedName` type. | Design | Inspection |
| FR-088-CON-5 | This requirement does not modify `syntax::ClauseKind`, `checking::types::NativeType`, native-v1's use of `ir::ValueType`, or native-v1's use of `ir::SymbolName`: all four are lane-private (ADR-013 §6) and gain no new variant, field or consumer under R-09. | Design | Test (TC-257) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-088-AC-1 | Exactly one checked clause-kind enum is defined in the layer-3 `check` core. A whole-crate definition scan confirms one canonical enum (the lane-private `syntax::ClauseKind` was deleted with the `0-draft` front end in QSL-1810). | Test (TC-257) |
| FR-088-AC-2 | Frame identity resolves each `modifies`/`creates`/`deletes` `NodeKey` to its `DeclarationKey` only by reading the model correspondence the S3 checker recorded on `CheckedGraph` and the S4 link step carried, unchanged, into the `CheckedPackage` a layer-4/layer-5 consumer holds (ADR-013 O-04: "Consumers read the correspondence from the `CheckedPackage`"); an adverse test that removes an entry from the correspondence and re-derives the same frame from source (rather than reusing the correspondence) demonstrates the resolution is not re-derived by search. | Test (TC-248) |
| FR-088-AC-3 | Given two occurrences of a structurally identical clause (same checked node id) at two distinct source positions, the two clauses' occurrence keys (node id, role, ordinal) differ, and a consumer keying on (node id, occurrence key) together, not on node id alone, distinguishes them; an adverse test that changes display text, diagnostic text, or the collection order the clauses are iterated in leaves both clauses' identities (node id, occurrence key) unchanged (R-05). | Test (TC-249) |
| FR-088-AC-4 | Every checked clause-kind variant maps to exactly one v2 `node_tag`/`semantic_form`/clause-operation-identity spelling (for `Invariant`, `Precondition` and `Postcondition`, that spelling plus the `filament.op.state.clause` kind member, FR-105), every v2 spelling this enum's wire vocabulary defines maps back to exactly one variant, and (`state`, `frame`) maps back to none: a wire-string totality test iterates every variant forward and every wire string backward and finds no gap and no ambiguity. A mutation test on the forward mapping function (each mutant flips one variant's target wire string, or deletes one match arm) fails the totality test for every mutant, with no mutant allow-listed (ADR-013 §4 preamble; ADR-012 §5.3 evidence convention). | Test (TC-250) |
| FR-088-AC-5 | No function exists whose signature accepts a `QualifiedName` or a bare string and returns a node id or a declaration, callable from outside the check stage; a source and call-graph scan over every crate module confirms every call site of the checker's name-resolution function lies inside the check stage, with the sole documented exception being the `replay` facade's own E9 lookup (a separate module, not built by this requirement) never called from `check` itself. | Test (TC-251) |
| FR-088-AC-6 | `QualifiedName` is used only as a declared component of an identity preimage (for example inside `DeclarationKey`'s or a checked node's own preimage where a name participates); no equality or hashing implementation on any identity type treats a `QualifiedName` as the sole identity-bearing field where a node id is available instead. | Inspection (TC-258) |
| FR-088-AC-7 | A package type's identity is its checked node id: node keys are content keys scoped only by owner (ADR-013 O-04, OQ-G). Two type declarations with the same structure under the same qualified name in two packages of the same owner share one node id. Two with the same structure and qualified name in packages with different source owners get distinct node ids, and two with different qualified names get distinct node ids. A builtin or anonymous type node, such as `Int[0, 9]`, has no owner and shares one id across packages and owners. A reference into another package is distinguished by the `package` component of its structural `NodeKey` (ADR-013 T-3). Within one package, two source *occurrences* of a reference to the same declared type (ADR-013 O-07's own term: "each source occurrence of a node is keyed by (node id, role, ordinal)" — for example the same type named as the field type of two different fields) resolve to the one node id the single declaration was minted with; this is not a claim about declaring a type twice; declaring a structurally identical type a second time under a colliding name is a name-binding refusal (a duplicate declaration), a different criterion, not a second occurrence of the first node id, and is out of this criterion's scope. This repository's own precedent for a duplicate name/binding — `checking::composed::proofs::CorrespondenceError::DuplicateDeclaration`, `src/checking/composed/proofs/correspondence.rs:96` — refuses rather than admits with a shared id. | Test (TC-259) |
| FR-088-AC-8 | `ValueTypeRef` is exactly the five-member union `{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String), Resolved(Box<ValueType>)}`; no sixth variant and no second definition exists, and no model field type is represented by a bare `NodeKey` or a raw string type name. `Clock` carries no fields (a place in a timed model, FR-230), and the `String` of `Unit` is a unit name the assembler resolves (FR-056-AC-42), not a type name. No production code outside the source-object assembler (FR-569 "The source model") constructs `Resolved`, domain package intake never does, and no serializer or wire writer encodes it; and `ValueTypeRef`'s `Display` renders it in the `resolved:` identity form, never by `Debug`. | Inspection (TC-260) |
| FR-088-AC-9 | C-26's conversion function is total: for every checked type-node form (`scalar_type`, `composite_type`, `bounded_domain`, and the sum form), a test constructs a checked node of that form and confirms the function returns a kernel `ValueType` with no panic, no `_`-arm fallback, and no lossy substitution; a checked type-node form added without a corresponding match arm fails to compile (`clippy::wildcard_enum_match_arm`, following ADR-011 §5's convention for other exhaustive maps). | Test (TC-252) |
| FR-088-AC-10 | For the sum form specifically, the source checked node's own id is not re-minted by C-26's conversion (it is the same content-addressed id the node carried before conversion, read from `CheckedGraph`/`CheckedPackage`, not a value the kernel shape carries); C-26's output kernel `ValueType` carries, per variant, a `VariantId` computed from the declaring sum and that member, and the kernel shape carries no `NodeKey` anywhere in its variant representation. A sum value built from this conversion carries its `VariantId`, and an adverse test confirms no code path reads or stores a bare variant index in place of the `VariantId`. | Test (TC-252) |
| FR-088-AC-11 | For an enum `E` declared with cases `b`, `a`, `c` in that order, each variant's `VariantId` equals the node key over `{version: filament.enum-member-node/v1, declaration_node_id: E's node key, case}`. When `E` is ordered, the variants' ranks are `b` 0, `a` 1, `c` 2, and a set of all three values visits `b`, `a`, `c`. When `E` is unordered, its canonical member list is `a`, `b`, `c`, the ranks are `a` 0, `b` 1, `c` 2, and the set visits `a`, `b`, `c`. A value that pairs `E::a`'s `VariantId` with rank 2 is refused at admission. Renaming the declaration `E` to `F` changes every `VariantId` and changes no rank and no visiting order. C-30 refuses `E`'s own declaration node key as a `VariantId`. | Test (TC-409) |
| FR-088-AC-12 | For declared units `m` and `km` (a scaled unit of `m`) and the compound unit `m^1`, the three `UnitId`s are pairwise unequal. `m`'s `UnitId` carries `m`'s node key under the `filament.checked-semantic-node/v1` label. For every compound unit, the compound `UnitId` equals its ordered (unit node key, exponent) factor record by structural equality. C-30 refuses a dimension node key as a declared-arm `UnitId`. | Test (TC-411) |
| FR-088-AC-13 | Over a source unit that selects a model as `M` (with record type `Total`), declares `type Money = Decimal[0, 100; 2, 2; exact];`, `record Point { x: Integer; }`, `enum Color { Red, Green }`, `dimension Length;`, `unit m : Length = rational(1, 1);` and `object Order { a: Money; b: Point; c: Color; d: Option<Sequence<Money>[0, 3]>; e: M::Total; g: Quantity<m>; h: Decimal[0, 100; 2, 2; exact]; operation f(p: Money): Option<Reference<Order>>; }`, `fsl_spelling` with each member's slot returns `Money`, `Point`, `Color`, `Option<Sequence<Money>[0, 3]>`, `M::Total`, `Quantity<m>`, `Decimal[0, 100; 2, 2; exact]`, `Money` for `p` and `Option<Reference<Order>>` for the result. Without a slot, `a` returns `Decimal[0, 100; 2, 2; exact]`. Every `NodeKey`, enum owner key, `EffectiveId` and declared unit that a `Resolved` type in the view holds has a registry entry. `Display` of each of these types starts with `resolved:`, holds no `Debug` text, and is the same in two runs. Replacing the registry entry of `Point`'s key with `Spot` changes the spelling of `b` to `Spot` and changes no record, `ValueType`, equality or conformance verdict. In a subtype `object Rush extends Order`, an override `f(p: Decimal[0, 200; 2, 2; exact]): Option<Reference<Order>>` is refused `ill_typed`/`variance-parameter`, and the refusal message names the base parameter type `Money` and the override's `Decimal[0, 200; 2, 2; exact]`. Neither the formatter output nor the refusal message contains `resolved:` text. For a member `w: W`, where `record W { c: Clock; }`, the `resolved_path` row of the step `field(w)` has as `member_type` the FR-322 identity rendering of the type node `W`'s emitted binding names, equal to the `member_type` of a record field typed `W`; it holds no `resolved:` and no `Debug` text, it is the next row's `declaring_type`, and the package reader joins it at intake. Replacing the registry entry of `Point`'s key with `Spot` leaves the emitted checked package bytes unchanged. | Test (TC-2360) |
| FR-088-AC-14 | `fsl_spelling` handed `Resolved` of `ValueType::Population(Some(3))`, with any registry and no slot, returns `resolved:Population[3]`, and handed `Resolved` of `ValueType::Population(None)` returns `resolved:Population`; each equals the type's `Display`, names no member type and holds no `Debug` text. `Option` of `Population(Some(3))` returns `Option<resolved:Population[3]>`. The FR-088-AC-13 unit's source model holds no `Population` node in any member type, and a source object member typed `Population<Order>[3]` is refused (FR-569-AC-13). | Test (TC-2360) |

For AC-6, a hand-written `PartialEq` or `Hash`, a name beside a filler field,
a local map, and the root crate's lane-private `QualifiedName` types are
not covered.

For AC-8, the scan covers the struct fields and enum variant fields under
`model` whose name contains `type`; a field type under another name is not
covered, and TC-260 step 4 is not backed. Three raw wire views are allowed a
bare string `type_identity` and are named in the scan: `RawRecord`, `RawNode`
and the reader's `Open`, all of which hold a snapshot document's `record`
`type` string until a checked declaration resolves it by lookup. They are wire
shapes, not model field types.

AC-5 is enforced for its `QualifiedName` half only (TC-251); the
"or a bare string" half is investigated and documented as a gap, not
enforced (see `tests/it/name_resolution_confinement.rs`'s own module doc).

## Dependencies

- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §1 R-05, R-06, §3 O-04, O-06, O-07, O-08, O-09, O-10, O-11, O-14, §4 C-26,
  §6 (lane-private table), §7 (the S-3 slice row).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §2.1 (admitted types), §6.1 (layer-3 `check` core position).
- [FR-087](FR-087-typestate-and-cross-package-node-key.md): S-3a's
  `CheckedGraph` is the type this requirement's checked nodes are members
  of, and is where the S3 checker records the model correspondence this
  requirement's frame-identity resolution (FR-088-AC-2) reads, through the
  `CheckedPackage` the S4 link step carries it into (ADR-013 O-04); this
  requirement does not depend on FR-087's `ImportView` or `library`
  module.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- ADR-013 §8 OQ-B, OQ-D and OQ-F; FSpec FR-141, FR-142 and FR-144
  (`ix://agent-ix/filament-specification/FR-141`, `FR-142`, `FR-144`).
