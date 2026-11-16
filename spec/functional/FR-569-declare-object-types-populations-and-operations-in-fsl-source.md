---
id: FR-569
title: "Declare object types, populations and operations in FSL source"
type: FR
relationships:
  - target: ix://agent-ix/filament-specification/FR-610
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-710
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/US-001
    type: implements
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-094
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-104
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-105
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-107
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-383
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-531
    type: traces_to
  - target: ix://agent-ix/filament-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-342
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-465
    type: depends_on
---
# FR-569: Declare object types, populations and operations in FSL source

## Description

When a complete-V1 unit declares an object type in its own source (its
fields, its object identity, its supertypes, the populations of its objects
and its operations with their signatures and frames), S1, S2, the assembler,
S3 and S4 SHALL admit, check and emit it as below. A
source-declared object type SHALL check, lower and evaluate exactly as an
object type that a selected domain package declares (FR-056, FR-094), except
that it is owned by the unit's source rather than by a domain package. Its
operations take the unit's existing `pre`, `post` and `body` declarations
(FR-104, FSpec FR-151 `operation-body`), and its `state`/`operation_anchor`
nodes are the source-declared anchors that FR-390 "Source-declared operation
calls from the reader" evaluates.

Without this declaration a unit names object types and operations only
through a model alias (`M::Node`, `M::Node::step`) and needs a domain package
document for every model it reasons about. FSpec FR-610 defines the
declaration in the standard: its grammar (the shared grammar's `object-decl`
productions), its member-type rule, its identity and, with FSpec FR-322
"Source-declared object types", FR-464 and FR-465, its checked-package form.
That adopts ADR-013 QC-30 items (1) to (3); item (4) is the IR reader work.

Out of scope: a source object type declares no relationship, component,
endpoint, interface feature, state machine or process; a source field
declares no `subsets` or field redefinition; a source object type does not
specialize a model-owned type and a model-owned type does not specialize a
source-declared one. Each of those stays a domain package construct, which an
author declares in a model unit (FR-710) and selects with `model`.

## Inputs

- A complete-V1 source unit (FR-091's E1 and E2 input) and its selected
  domain packages, if any.
- The unit's source owner (authority, identity) and its checked package name.

## Outputs

- S2: one `DeclarationForm::Object(ObjectForm)` per object declaration,
  carrying its span and the spans of its parts.
- Assembly: the unit's source model, the object-type, field, operation and
  population records below.
- S4: the `model`/`object_type` nodes, the `state`/`operation_anchor` and
  `state`/`frame` nodes of its named operations, its clause functions, and one
  source-object identity row per emitted source-declared object type, its
  `operation_effects` rows and dispatch-table rows (the target emission); or,
  until ADR-013 QC-30 lands, the one typed `SourceObjectEmission` refusal and
  no package.
- Or a located refusal and no checked package.

## Behavior

### Surface syntax

S1 SHALL parse these productions as declarations of a complete-V1 unit,
written in the shared-grammar notation; `field` and `parameter` are the
existing productions:

```ebnf
object-decl       = [ 'abstract' ], 'object', ident,
                    [ 'extends', type-name, { ',', type-name } ],
                    [ 'identity', '(', ident, { ',', ident }, ')' ],
                    '{', { object-member }, '}' ;
object-member     = field | population-member | operation-member ;
population-member = [ 'symmetric' ], 'population', ident, ';' ;
operation-member  = 'operation', ident,
                    '(', [ parameter, { ',', parameter } ], ')',
                    [ ':', type-ref ],
                    [ 'redefines', ident, '::', member-name ],
                    { frame-clause }, ';' ;
frame-clause      = 'modifies', field-target, { ',', field-target }
                  | 'creates', ident, { ',', ident }
                  | 'deletes', delete-target, { ',', delete-target } ;
field-target      = 'self', '.', ident | ident, [ '.', ident ] ;
delete-target     = 'self' | ident ;

type-name         = model-name | ident ;
operation-name    = type-name, '::', member-name ;
population-name   = model-name | ident ;
```

Every word of these productions is a reserved spelling, as every grammar
word of a complete-V1 unit is (`base_reserved_spellings`): `object`,
`operation`, `population` and `abstract` stay contextual words, as they are
today, `identity` already is reserved, and S1 SHALL treat `extends`, `symmetric`, `redefines`, `modifies`, `creates` and
`deletes` as reserved and SHALL refuse each written as an identifier with the
`invalid_syntax`/`unexpected-token` diagnostic at the word. The last three
productions replace `TypeName = ModelName` and `OperationName = TypeName ::
member` and extend the population positions (`PopulationBinder`,
`PopulationBound`, `PopulationRow` and FR-391's population positions) to a
single identifier. The assembler SHALL resolve a one-segment type name,
operation type or population name to a source declaration and a two-segment
model name to a model declaration, as before. Everything that takes a
`TypeName`, `OperationName` or population name today takes the source form
with no other change: a state clause `on Node` or `on Node::step`, an
operation body `body b using P on Node::step(x) { ... }`, a protocol `over
(view: Node)`, `role R on Node` and `by R on Node::step`, a population bound
`bound b on nodes [3]`, a quantifier domain `x in nodes`, `Reference<Node>`,
`Population<Node>[3]`, `allInstances<Node>(nodes)` and
`lookup<Node>(nodes, r)`. The one exception is FR-218's role on a whole
model: in `role w on M`, a one-segment `M` that is a model alias of the unit
SHALL resolve to that alias's whole model, as FR-218 states, and never to a
source object type; no source object type can share the name of a model
alias (below), so the two never compete.

The parts of an object declaration mean:

- `abstract`: the type has no direct instances (FSpec FR-151, D05).
- `extends`: the type's direct supertypes, in source order.
- `identity`: the type's identity fields, own or inherited, in source order;
  without it the type declares none of its own and its identity fields are
  its supertypes' (the first supertype's that has any, in `extends` order).
  The assembler checks the list (the refusals below stand); emission's
  source-object identity rows read it from the source declaration once
  emission lands.
- a `field`: a field member; `?` makes its presence optional. Its type is the
  written type, a collection type for a many-valued field, exactly as a
  source record field.
- a `population-member`: a closed population whose member type is the
  declaring type; `symmetric` marks it symmetric (FSpec FR-150). A population
  is not inherited: an object of a subtype is a member of a population of its
  supertype, as an object of a domain subtype is.
- an `operation-member`: an operation with its parameters in order, its
  result (none when `:` is absent), the inherited operation it redefines and
  its frame. `modifies self.f` is a receiver-scoped field entry, `modifies
  f` an every-object entry of the declaring type's field `f` (own or
  inherited) and `modifies T.f` one of the field `f` of the source object
  type `T`, `creates T` and `deletes T` type-wide entries over a source
  object type, `deletes self` the receiver deletion and `deletes p` the
  deletion of the reference parameter `p` (FR-154). A frame names only the
  unit's source object types, their fields and the operation's own
  parameters; the grammar admits no model name in it. An operation with no
  frame clause writes nothing (a query).

S2 SHALL build one `ObjectForm` per `object-decl`, its members in source
order, without reordering, merging or deduplicating them; S3 and the
assembler check them. An `object-decl` that does not parse is the
`invalid_syntax` diagnostic at the offending token, as for any declaration.

### The source model

The assembler SHALL build, from the unit's object declarations, the same
normalized records that domain intake builds from an admitted domain package
(FR-056, `fsl-semantics/src/model/domain_package.rs`): one
`ObjectTypeRecord` per type (its `abstract_type`, its `supertypes`, no
`interface_features`), one `FieldMemberRecord` per field (the multiplicity its type gives, as
below, the declared presence, no subsets), one `OperationMemberRecord`
per operation (its parameters, its result, its `OperationEffect` from the
frame clauses, its `redefines`, its `requires` and `ensures` as below, and
`has_body` set from the unit's `body` declarations as it is for a model
operation) and one `PopulationRecord` per population (member type the
declaring type, extent `closed`, the `symmetric` marker). Their declaration
keys belong to the unit's source owner, never to a domain package, and
no ModelSelection, domain package bytes, FR-154 admission or `ModelOwner`
takes part. The source model is one closed declaration set: FR-081's
effective view, FR-103's operations and frames, FSpec FR-150 and FR-151
(conformance, redefinition, abstract types and closed dispatch), FR-104's
clause resolution and FR-106 and FR-107's admission and evaluation SHALL run
over it with the same code paths and the same rules as over a model's
effective view.

The assembler SHALL record a source member's type the way a model member's
is recorded. When the outermost constructor of a field, parameter or result
type is a collection, the record's `Multiplicity` SHALL be that collection's
kind and bounds, and the record's `value_type` SHALL be the element type left
after removing it. When the outermost constructor is not a collection
(`Option<..>` included), the multiplicity SHALL be exactly one and the
`value_type` SHALL be the whole type. A `Clock` element SHALL be
`ValueTypeRef::Clock`. A `Reference<T>` element over a source or model object
type SHALL be `ValueTypeRef::Package` naming `T`'s declaration key. Every
other element SHALL be `ValueTypeRef::Resolved`, holding the `ValueType` the
element has as a source record field (FR-088 "A model field type names a
package node, except a source object member's"). For example,
`Sequence<Set<Integer>[0, 2]>[0, 2]` is an ordered multiplicity `0..2` with
`Resolved` of `Set<Integer>[0, 2]`, and `Option<Sequence<Integer>[0, 3]>` is
multiplicity one with `Resolved` of the whole option. The assembler SHALL
fill every member's type once the unit's type namespace is assembled, and the
view that conformance, dispatch, frames, admission and evaluation read SHALL
hold no type from an earlier assembly step.

When it fills the member types, the assembler SHALL also build the source
model's `SourceSpellings` registry (FR-088 "A `Resolved` type is spelled
through the source spelling registry"). The registry keeps the FSL names that
the normalized records drop: alias names, record, tuple, union and enum names,
object type names and unit names, keyed by `NodeKey`, enum owner key,
`EffectiveId`, declared unit and member slot. The source model's
`AdmittedModel` SHALL hold the registry with its records, and every
user-facing text naming a source member's type SHALL be printed from it
through FR-088's formatter. The registry takes no part in identity,
conformance or emission.

The checker SHALL decide conformance of a `Resolved` type as follows. The list
applies to fields, operation parameters and operation results. For operation
parameters and results, whose FSpec FR-151 rows say only "conforms", it is
this requirement's own rule, and it agrees with the conformance the model
route applies to the same types. A `Resolved` type S conforms to a
`Resolved` type T exactly when one of these holds:

1. S and T are the same type after alias expansion.
2. S and T are both integer types and S's interval lies within T's, which
   includes `Int[..]` within `Integer` (FSpec FR-151, using FSpec FR-149's
   `Int` rows).
3. S and T are both `Option`, and S's payload conforms to T's.
4. S and T are collections of the same kind, S's element conforms to T's,
   and S's bounds lie within T's.
5. S is `Reference<A>` and T is `Reference<B>` (reached directly, as an
   `Option` payload or as a collection element), and A conforms to B by
   FSpec FR-151's object conformance, for source and model object types
   alike. Cases 3 and 4 reach a nested reference by recursion.

No other pair conforms. In particular, two `Decimal`, `Rational`, `Float`,
`Quantity`, `Text`, enum, record, tuple or union types conform only when they
are the same type. A tuple or union with `Reference` components likewise
conforms only when it is the same type; its components are not compared. A `Resolved` type does not conform to a type of any other
variant, and no type of another variant conforms to a `Resolved` type.

The checker SHALL record a clock place for a `Resolved` type exactly where
the record field's type has one, for example in a member of a source record
type that has a `Clock` field (FR-230 "Records and admitted aggregate
paths"). Those places SHALL be the ones the record field has.

The `pre` and `post` state clauses that name a source-declared operation are
that operation's contract clauses, its `requires` and `ensures` in source
order: FR-104 checks them as clauses, and FSpec FR-151 combines them by
construction (an effective precondition is the disjunction of the
redefining operation's own clauses with the inherited ones, own first, and an
effective postcondition their conjunction) and its dispatch reads them as a
domain operation's `pre` and `post` entries. A `body` declaration naming a
source-declared operation supplies its query body as for a model operation.

### Name resolution and collisions

- The unit's source namespace holds its records, tuples, unions, enums,
  aliases, functions, predicates, state clauses, object types and
  populations. A source object type or population whose name equals another
  declaration of that namespace, a model alias, a profile alias or an import
  alias of the unit SHALL be refused `ambiguous_declaration`/`ambiguous-name`
  at every declaration of that name, naming each. A function contributes its
  declaration span.
- A source object type and its populations are scoped to the unit that
  declares them: a clause, role or `over` of another unit of the package
  that names the type, or a clause that names the population, SHALL be
  refused `missing_declaration`/`missing-name` at the name.
- A one-segment type name SHALL resolve only to a source object type and a
  two-segment one only to a model type; neither falls back to the other.
  A source object type whose name equals a type of a selected domain package
  (`Node` beside `M::Node`) is a different type: `Node` always names the
  source type, `M::Node` always the model type, and no resolution, identity
  row, snapshot name or evaluation substitutes one for the other.
- A one-segment type name that names no source object type (and, in a role
  position, no model alias) SHALL be refused `missing_declaration`/
  `missing-name` at the name, also when it names a source record or another
  source declaration.
- `T::op` SHALL resolve `op` through `T`'s effective view as FR-104 resolves
  `M::T::op` (a more derived declaration hides an ancestor's; two
  incomparable declaring ancestors refuse `ambiguous_declaration`/
  `ambiguous-name`), and an operation `T` neither declares nor inherits SHALL
  be refused `missing_declaration`/`missing-name` at `op`.
- A one-segment name in a population position SHALL resolve to the source
  population of that name. A name that is both a source population and a
  binder or parameter in scope SHALL be refused `ambiguous_declaration`/
  `ambiguous-name` at the use, naming both.
- Within one object type, two members (fields, populations, operations) of
  one name SHALL be refused `ambiguous_declaration`/`ambiguous-name` at the
  second, and so SHALL a field whose name equals an inherited field. An
  operation whose name equals an inherited operation's and that carries no
  `redefines` SHALL be refused `ambiguous_declaration`/`ambiguous-name` at
  the operation.

### Declaration checks

The assembler and S3 SHALL refuse, each at the offending name:

- an `extends` entry that names a model type, or a source declaration that is
  not an object type: `invalid_model_binding`/`malformed-declaration`; one
  that names nothing: `missing_declaration`/`missing-name`; a supertype
  cycle: `invalid_package`/`definition-cycle`, naming the cycle's edges;
- an `identity` entry that names no field of the type's effective view:
  `missing_declaration`/`missing-name`;
- a `redefines T::op` whose `T` is not a proper supertype or whose `op` `T`
  does not expose: `missing_declaration`/`missing-name`; a redefinition that
  breaks an FSpec FR-151 conformance axis: that axis's refusal, as FR-151
  gives it for the same domain declaration (`ill_typed`/`variance-parameter`,
  `variance-result`, `type-mismatch` for arity, `multiplicity-narrowing` or
  `effect-escape`);
- a frame entry that names no field, source object type or parameter:
  `missing_declaration`/`missing-name`; a `deletes p` whose `p` is not a
  `Reference` parameter: `invalid_model_binding`/`malformed-declaration`; a
  `deletes` name that is both a parameter and a source object type:
  `ambiguous_declaration`/`ambiguous-name`;
- a field, parameter or result type that does not resolve: the type
  refusals FR-091 gives any declaration.

### Member types

The assembler and S3 SHALL admit, for a field, operation parameter and
operation result of a source object type, every type a source `record` field
admits, with the same checks and the same refusals (FSpec FR-610 "Member
types"):

- `Boolean`, `Integer`, `Int[lo, hi]`, `Rational[..]`, `Decimal[..]`,
  `Float32` and `Float64` (selecting the IEEE profile as a float-bearing
  record field does, FR-397), and `Text[..]`;
- `Quantity<u>` and `Quantity<u, C>` over a unit the unit declares or a
  selected domain package declares;
- `Clock`, under FR-230's rules for a clock field (a dense time source, resets
  and constraints), read as a clock of the declaring object type. The dense
  time source is the source model's own time member, written with a source
  object type's name, `time O dense unit ms;` (FR-230 "The source model's
  time member"); a selected model's time member never times a source
  model clock;
- `Option<T>`, `Reference<T>` over a source or model object type, and
  `Sequence`, `Set`, `Bag` and `OrderedSet` of any admitted element type, a
  nested collection and an `Option` of a collection included;
- a source `enum`, `record`, `tuple`, `union` or `type` alias, and a
  model-owned value, record, tuple or variant type (`M::Total`).

The checked member type is the type the same form has as a record field: a
source enum member reads as that enum, a record member as that record (FR-359
for a child record at an ancestor-typed slot), a union member as that union
(FR-316), and every value read through `self.f`, `deref(r).f`, frames,
snapshots (FR-106) and the evaluator is the value the record field would
hold. A many-valued field is its collection type, as for a record field. In
the target emission the member's binding on the `model`/`object_type` node
names the type node the record field's binding would name.

A type that a source record field refuses SHALL be refused for an object
member with the same code and cause at the type, and no other type is refused
for an object member: `Population<T>[N]` as a field type, and a one-segment
name that names a function, predicate, state clause or population, refuse
exactly as they do in a record field. Admitting the record field rule exactly
keeps one member-type rule in the language, so a type moved from a `record`
into an `object` keeps its meaning and its refusals.

### Lowering and identity

This section, "Source-object identity rows" and FR-383's `owner_model` state
the target emission: what S4 produces for a unit that declares a source object
type once ADR-013 QC-30 is adopted by FSpec and IR. Until then S4 refuses such
a unit as "Interim behavior" below states and writes none of it.

In the target emission, S4 SHALL emit, for every source-declared object type,
whether or not a checked node names it, one `model`/`object_type` node with:

- `node_id`: FSpec FR-322's structural id of a declared node, as FR-092
  keys every declared node: `package` the checked package's name, `path` the
  declaration path `[<type name>]`, ordinal by FR-092's producer rule;
- `declaration`: `{qualified_name: [<type name>]}`;
- `owner`: the unit's `SourceOwner` `{kind: "source", authority, identity}`;
- `semantic_type`: itself;
- body: an `aggregate` of one binding per field of the type's effective view,
  inherited fields first (in `extends` order, each supertype's fields in its
  own body order) and then its own fields in source order, each binding
  written exactly as FR-092 writes a declared record's field binding (the
  field name bound to a reference to its type node, an optional field with
  its `optional` group); a type with no field has `aggregate{[]}`;
- `dependencies`: the type nodes its bindings reference, ascending;
- one `declaration` occurrence at the object declaration's span.

The body names the fields so that a reader resolves a field member on the
node from the package alone, by the binding of the member's name (QC-30 item
(2a)).

The node id is a structural id. Changing a field, an operation or a
population leaves the type's node id unchanged; renaming the type changes
it. A model-owned `Node` and a source-declared `Node` have different ids and
different owners.

A field, operation or population member has no node of its own, exactly as
for a model-owned type (FR-094): an FR-322 member names it by the node of the
receiver's static source object type and its name, and a
`state`/`operation_anchor` names an operation by its `context`, the node of
the source object type that declares it, and its `operation` text (FR-105).
The anchor's frame node is built from the source `OperationEffect` as for a
model operation.

**Operation effects.** For every operation a source object type declares
(a query included, whose effect is four empty arrays), the target emission
SHALL write one FR-496 `operation_effects` row
`{document, declaring, operation, effect, provenance}`:

- `document`: a string, as FR-496 types it: the `identity` of the declaring
  node's `SourceOwner` (the unit's source identity), in place of a selected
  document identity. Its type is unchanged, so FR-496's sort key
  `(document, declaring, operation)` and its byte order are unchanged; a
  source row and a model row whose `document` strings are equal still have
  distinct `declaring` nodes, so the order stays total and no two rows share
  a key.
- `declaring`: the local `NodeRef` of the source-declared `model`/`object_type`
  node.
- `operation`: the operation name as the source declares it.
- `effect`: exactly as FR-496 states for a model operation (all four effect
  members, the same entry shapes, scopes and order).
- `provenance`: `origin` the new variant `source_declaration_operation_frame`
  (a model row keeps `model_document_operation_frame`), and `member` the
  operation's source identity `<type>::<operation>` (`Order::bump`), the
  `source_identity` of its source-operation row (FR-390).

An operation only inherited has no row under the subtype. A reader joins a
`source_declaration_operation_frame` row to its declaring node through that
node's `SourceOwner` (whose `identity` equals `document`) and its
source-object identity row, and checks `effect` against the source frame,
not through a selected model row (QC-30 items (2e) and (4)).

**Dispatch tables.** For every dispatched application whose receiver context
is a source-declared object type, the target emission SHALL write the FR-494
`checked_term_data.dispatch_tables` row exactly as FR-494 states, with the
receiver context the source-declared `model`/`object_type` node, each
candidate's `EffectiveId` the source declaration key the check selected it
by, its declaring type the candidate's source-declared node, and its
operation-body and effective-precondition identities the source clause
function nodes below. Candidate order and witnesses are FR-494's. A reader
joins the context and each candidate's declaring family through the nodes'
`SourceOwner` and source-object identity rows, not through a selected model
row (QC-30 items (2f) and (4)).

S4 SHALL build the clause functions of a source-declared operation exactly
when FR-094 builds them for a model operation, that is for a
dispatch-eligible query (a result and an empty effect), and of the same kinds
(authored precondition, effective precondition, body), with these
differences:

- `declaration`: a qualified name distinct for every clause function of the
  package, so that a reader's declaration-name rule (one holder per
  qualified name) admits sibling clause functions with no change:
  `[<type>, <operation>, "precondition", <label>]` for an authored
  precondition, `<label>` the label of the `pre` declaration it lowers;
  `[<type>, <operation>, "effective_precondition"]` for an effective
  precondition; and `[<type>, <operation>, "body"]` for a body. Every segment
  matches the identifier pattern `[A-Za-z_][A-Za-z0-9_]*` that a reader
  requires of a qualified-name segment: `<type>` and `<operation>` are source
  identifiers, `<label>` is the identifier the grammar requires of a `pre`
  label, and the three fixed segments are identifiers. `<type>` is
  the declaring type of the operation member whose clause it is, as FR-094
  chooses it (the authoring operation for an authored precondition, the
  candidate for an effective precondition and a body). Clause labels are
  unique in the unit's source namespace and an operation has at most one
  effective precondition and one body, so no two clause functions share a
  name, and none shares one with an object type (`[<type>]`);
- `owner`: the unit's `SourceOwner`;
- `node_id`: `package` the checked package's name, `path` the declaration
  path above, and ordinal `0`;
- one `declaration` occurrence: for an authored precondition, the span of the
  `pre` declaration it lowers; for a body, the span of the `body`
  declaration; for an effective precondition, the span of the redefining
  `operation` member;
- the same body as a model operation's clause function, with its trailing
  `clause` binding (FR-094).

No other node shares that path, since no member has a node. A source object
type declares no type-level invariant clause, so it has no invariant clause
function.

A source population that a checked node names (FR-391) is lowered as a
`relation`/`population` population reference node with `declaration`
`{qualified_name: [<population name>]}` and the unit's `SourceOwner`, keyed
as a declared node; a source population no checked node names has no node.
That shape is ADR-013 QC-30's proposal to FSpec FR-322 (item (2c)).

### Source-object identity rows

A bytes-only consumer has no domain package document for a source-declared
type, so in the target emission S4 SHALL produce, at the model-selection identity stage with the
source-record and source-operation rows (FR-390), one source-object identity
row for each emitted source-declared `model`/`object_type` node:
`structural_node` (the node), `source_authority`, `source_identity` (the
type's qualified source name), `abstract`, `supertypes` (the structural ids
of the direct supertype nodes in source order), `identity_fields` (the
type's effective identity field names: its own `identity` list, or the
inherited one when it declares none), `members` (one entry per field of the
effective view, in the node body's binding order, in the `source_records`
member encoding of FR-531) and `populations` (one `{name, symmetric}` entry
per population the type itself declares, in source order). The row's key is
(`source_authority`, `source_identity`). The rows are written to and read
from the `source_objects` member of `identities.json` (FR-531 "The identity
file", FR-517); none is derived from package bytes, decoded nodes, display
names or source text. The reader view and the bytes route SHALL use these
rows where they use a model-owned type's admitted document: for field types,
identity fields, supertypes, abstractness and populations.

View creation SHALL admit the source-object rows with FR-390's
source-operation rows, at the `Package` locus, each failure with no partial
view and no meter charge, and SHALL classify each supplied row by these
rules, in this order:

1. A row whose `source_authority` is the authority of no `SourceOwner` of the
   package is `ForeignIdentityWitness`, whatever its other fields hold.
2. Otherwise, a row whose `structural_node` is not a source-declared
   `model`/`object_type` node of the package, or whose `source_identity`
   disagrees with that node's `declaration`, is `UnknownIdentityWitness`.
3. After every supplied row is classified, an emitted source-declared object
   type that no admitted row names is `MissingIdentityWitness`.

A rejected row is never counted as naming its node, and the reported reason
is the first of FR-390-AC-67's reason order among those that arise. A
supplier that adds a foreign row beside the package's own complete row set
therefore gives `ForeignIdentityWitness` and no other reason.

### Observations and evaluation

FR-106 SHALL admit a snapshot (FR-106 "Document forms") that names a
source-declared population or type by its `source_identity`, the declared
name as the unit writes it (`"population": "nodes"`, `"type": "Node"`), in
the `population`, `type` and object-reference positions where it names a
model declaration by its `ix://` declaration identity. FR-106 SHALL resolve
a name with no `ix://` scheme only among the source declarations of the
checked package and an `ix://` name only among the model declarations. FR-106
SHALL require a snapshot's `model` member exactly when the checked package
selects a domain package, and SHALL refuse a `model` member in a snapshot of a
package that selects none `invalid_runtime_input`/`unknown-member` naming
`model`. An object's model component is the unit's source owner for a
source-declared type. A population name that names no population of the
package, or an object whose type is not a member type of its population, is
refused `invalid_runtime_input`/`wrong-role-mapping` (FR-106 check 6.1), and
an object whose type is abstract is refused `invalid_runtime_input`/
`abstract-instance`, as for a model type.

Every evaluation over a source-declared object type SHALL give the outcome
and the ordered charge log that the same evaluation gives over its model
twin, the domain package type with the same fields, identity, supertypes,
populations, operations, frames and clauses: invariants, constraints,
preconditions and postconditions over pre and post snapshots (FR-107),
`allInstances`, `lookup` and population domains (FR-084, FR-391), dispatched
query calls and operation bodies (FSpec FR-151), frame checks (FR-103,
FR-154), protocol slots (FR-390), and every temporal claim and offline monitor
(FR-283) whose `over` parameter or state reads the type. Only the node ids,
owners and identity rows differ.

### Interim behavior until FSpec and IR adopt QC-30

IR's strict reader assumes model ownership in many joins, so no part of the
target emission is claimed admissible before ADR-013 QC-30 lands. FSpec
adopts items (1) to (3) in FSpec FR-610, its shared grammar, FR-322
"Source-declared object types", FR-464 and FR-465. Until FSpec
adopts QC-30 items (1) to (3) and IR adopts every change of item (4) other
than the FR-352 witness kind:

- The source route (S1 parse, S2 forms, assembly, S3 check, and in-process
  evaluation of state clauses, protocol slots, dispatched calls, frames,
  temporal claims and monitors, none of which reads an emitted wire) SHALL
  support every construct of this requirement.
- S4 SHALL refuse to emit any unit that declares a source object type, with
  one typed refusal, `unsupported_construct`/`not-yet-implemented` naming the
  construct `SourceObjectEmission`, at the span of the unit's first `object`
  declaration in source order. It SHALL write no package, no
  `identities.json` and no handoff directory, and SHALL report no other
  refusal for the unit's source object types, operations, populations,
  protocols or rows.
- A unit that declares no source object type is emitted as before.

Once QC-30 lands, S4 emits the target emission above; a protocol that reaches
a source-declared operation anchor then gets the typed `MissingSelectedJoin`
view refusal on its reader and bytes routes until IR's FR-352 witness kind
lands, as FR-390 states.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-569-AC-1 | Passes now (source route). TC-1408 fixture S parses: one `ObjectForm` per object declaration, `Order` with fields `total: Int[0, 9]` (required) and `note: Text[0, 8; nfc]?` (optional), identity `(total)`, population `orders`, operations `size(): Integer`, `count(): Integer`, `scaled(n: Integer): Integer` and `bump(): Boolean modifies self.total`; `Sub extends Order` with `size(): Integer redefines Order::size` and `scaled(m: Integer): Integer redefines Order::scaled`; `abstract object Shape { population shapes; }` with one population and no field or operation; each part in source order with its span. `operation step()` with no `;`, `modifies self.` with no field name and `object Node extends {}` each give one `invalid_syntax` diagnostic at the offending token and no `ObjectForm`. A parameter named `extends`, and one named `modifies`, each give one `invalid_syntax`/`unexpected-token` diagnostic at the word. | Test (TC-1408) |
| FR-569-AC-2 | Passes now (source route). Over fixture S, `invariant I using P on Order at current { self.total >= 0 }`, `pre Q using P on Order::bump { self.total < 9 }`, `body B using P on Sub::size() { 7 }`, `forall(o in orders : deref(o).total >= 0)` and `Reference<Sub>` resolve to the source declarations; `on Order::reset` refuses `missing_declaration`/`missing-name` at `reset`; `on Receipt` naming a source record and `on Nothing` refuse `missing_declaration`/`missing-name` at the name. With a domain package that also declares `Order` selected as `M`, `on Order` checks against the source fields and `on M::Order` against the model's, and no name resolves to the other; in a protocol of that unit, `role w on M` checks as FR-218's role on the whole model `M`. | Test (TC-1408) |
| FR-569-AC-3 | Passes now (source route). Each refuses `ambiguous_declaration`/`ambiguous-name` at the stated spans and yields no checked package: a source `record Order` beside `object Order` (both declarations); `object M` in a unit with `model M` (both); population `orders` beside a function `orders` (both); a quantifier binder `orders` over `orders` (the use, naming both); two fields `total` in one type (the second); a field of `Sub` named `total` (the field); an operation `size` in `Sub` without `redefines` (the operation); `deletes Order` in an operation whose parameter is also named `Order` (the entry). | Test (TC-1408) |
| FR-569-AC-4 | Passes now (source route). Each refuses with the code and cause stated, at the offending name: `extends M::Order` and `extends Receipt` (a source record) `invalid_model_binding`/`malformed-declaration`; `extends Ghost` `missing_declaration`/`missing-name`; `A extends B` with `B extends A` `invalid_package`/`definition-cycle` naming both edges; `identity (ghost)`, `redefines Order::reset`, `modifies self.ghost` and `creates Ghost` `missing_declaration`/`missing-name`; `deletes n` with `n: Integer` `invalid_model_binding`/`malformed-declaration`; `Sub`'s `scaled` redeclared `scaled(m: Text[0, 8; nfc]): Integer redefines Order::scaled` `ill_typed`/`variance-parameter` on the member `Sub.scaled`, as FSpec FR-151 refuses it in D. | Test (TC-1408) |
| FR-569-AC-5 | Passes now (source route). Over fixture S and its model twin D (TC-1408), the checked effective views, the dispatch tables and the checked types agree member for member: `self.size()` in a state clause on `Sub` dispatches to `Sub.size` and on `Order` has candidates `Order.size` and `Sub.size`; `deref(s).total` over `s: Reference<Sub>` names `Sub`'s node and `total`; `allInstances<Order>(orders)` has type `Set<Reference<Order>>`, and `Set<Reference<Order>>[0, 3]` with `bound b on orders [3]`. `Q` is `bump`'s `requires` and `Qs` is `size`'s. In variant S-R, where the unit adds `pre Qs2 using P on Sub::size { self.total >= 1 }`, `Sub.size`'s effective precondition is `Qs2 or Qs`, own clause first, as in D-R; in variant S-R', where `Sub` adds `operation bump(): Boolean redefines Order::bump modifies self.total, self.note;`, both S-R' and D-R' refuse `ill_typed`/`effect-escape` once, on the member `Sub.bump` and the field `note`. | Test (TC-1408) |
| FR-569-AC-6 | Waits on QC-30 (target emission). S4 over fixture S emits `Order`, `Sub` and `Shape` as `model`/`object_type` nodes with `declaration` `["Order"]`, `["Sub"]` and `["Shape"]`, the unit's `SourceOwner`, self `semantic_type` and one `declaration` occurrence each; `Order`'s body binds `total` then `note` (with its `optional` group), `Sub`'s binds the inherited `total` and `note`, and `Shape`'s is `aggregate{[]}`; `Shape`, which nothing names, is emitted. No node is emitted for a field, operation or population member. `Q`'s anchor has `context` `Order`'s node, `operation` `"bump"` and a frame node whose `semantic_type` is `Order`'s node and whose `modifies` holds (`Order`'s node, `total`, `receiver`). Clause functions, built for the queries only: `Qs`'s authored precondition has `declaration` `["Order", "size", "precondition", "Qs"]`, `C`'s body `["Order", "size", "body"]` and `B`'s body `["Sub", "size", "body"]`, each with the `SourceOwner`, its declaration as id path, ordinal `0`, one `declaration` occurrence at the authoring declaration's span and its `clause` binding; `bump`, which is not a query, has none. In S-R, `Qs2`'s is `["Sub", "size", "precondition", "Qs2"]` and `Sub.size`'s effective precondition `["Sub", "size", "effective_precondition"]`, with its occurrence at `Sub`'s `size` member; no two clause functions share a qualified name, and every segment is an identifier. `operation_effects` has one row per own operation (four of `Order`, two of `Sub`), each with `document` the unit's source identity string, `declaring` the declaring node and `provenance` `{origin: "source_declaration_operation_frame", member: "<type>::<operation>"}`; `bump`'s (`member` `Order::bump`) holds the receiver `modifies` entry for `total`, the others four empty arrays, and the rows are in `(document, declaring, operation)` byte order. The `self.size()` application in `Sized` has a dispatch-table row whose context is `Order`'s node and whose candidates `Order.size` and `Sub.size` carry the body identities of `C` and `B` and the effective-precondition identity of `Qs`'s authored function (in S-R, `Sub.size`'s is its `effective_precondition` function). IR's strict reader admits the package | Test (TC-1408) |
| FR-569-AC-7 | Waits on QC-30 (target emission). `Order`'s node id is unchanged when a field is added to `Order`, an operation is renamed or the declaration is moved within the unit, and changes when `Order` is renamed; its body changes with the added field. D's `Order` node is `{<domain package>, [model, object_type, <IR node>], 0}` with a `ModelOwner` and S's is `{<checked package>, ["Order"], 0}` with the `SourceOwner`; the two differ | Test (TC-1408) |
| FR-569-AC-8 | Waits on QC-30 (target emission). The emission over S carries one source-object row per type: `Order` with `abstract` false, no supertypes, `identity_fields` `["total"]`, members `total` then `note` (optional) and populations `[{name: "orders", symmetric: false}]`; `Sub` with `abstract` false, `supertypes` `[Order's node]`, `identity_fields` `["total"]` (inherited), members `total` then `note` and populations `[]`; `Shape` with `abstract` true, no member, `identity_fields` `[]` and populations `[{name: "shapes", symmetric: false}]`. D's emission has none. The rows are written to `identities.json` `source_objects` and read back equal. Over S: a supplier with the `Order` row removed gives `MissingIdentityWitness`; one whose `Order` row has `source_identity` `Ghost`, or `structural_node` a frame node, gives `UnknownIdentityWitness` only; one that supplies S's complete rows plus the `Order` row of an emission of S under another source authority gives `ForeignIdentityWitness` only; each at the `Package` locus with no view and no charge. | Test (TC-1408) |
| FR-569-AC-9 | Passes now (source route). A snapshot of S naming population `orders` and objects of type `Order` and `Sub` admits. The same snapshot with an object of `orders` typed `ix://test/orders/Order` refuses `invalid_runtime_input`/`wrong-role-mapping`. An object of type `Shape` in population `shapes` refuses `invalid_runtime_input`/`abstract-instance`. S selects no domain package: a snapshot with no `model` member admits and one with a `model` member refuses `invalid_runtime_input`/`unknown-member` naming `model`. In variant S-M, S with `model M = "test/orders"` selecting D's package: a snapshot with `"population": "orders"`, `"type": "Order"` admits its objects as source `Order` objects, one with `"population": "ix://test/orders/orders"`, `"type": "ix://test/orders/Order"` admits them as model `M::Order` objects, `I` reads only the first, and one placing a `"type": "Order"` object in `ix://test/orders/orders` refuses `invalid_runtime_input`/`wrong-role-mapping`. | Test (TC-1408) |
| FR-569-AC-10 | Passes now (source route). For each TC-1408 vector V1 to V6 (`I`, `Q`, a `post` on `bump` over a pre and post snapshot, `forall(o in orders : ...)`, `self.size()` dispatched to `Sub.size`, and the frame check of an invocation of `bump` that writes `total` of a second object), S and D give equal outcomes and equal ordered charge logs on the source route. | Test (TC-1408) |
| FR-569-AC-11 | Passes now (interim). S4 over TC-1408 fixture S, over its variants S-R, S-P and S-M, and over TC-1407 fixture E-S refuses with exactly one refusal, `unsupported_construct`/`not-yet-implemented` naming `SourceObjectEmission` at the span of the unit's first `object` declaration (`object Order` in S and its variants, `object Node` in E-S), and writes no package, no `identities.json` and no handoff directory. The same units check, and the source route evaluates TC-1408 vectors V1 to V6 and TC-1407 vectors S1 to S3 over them. Fixture D and TC-1070's fixture E, which declare no source object type, emit as before | Test (TC-1408, TC-1407) |
| FR-569-AC-12 | Source route. For each member type, an `object` field of that type, and an operation parameter or result of that type, assembles and checks exactly when a source `record` field of that type does, and `self.f` in a state clause on the object has the type the record field has: a source `enum` (`Tier { Gold, Standard }`), a source `record` (`Point { x: Integer; }`), a `union` with a payload member, a `tuple`, a `type` alias, `Decimal[0, 100; 2, 2; exact]`, `Rational[0, 9; 1, 4]`, `Float64` under the IEEE profile, `Quantity<m>` over a declared `dimension L; unit m : L = 1;`, `Option<Sequence<Integer>[0, 3]>` and `Sequence<Set<Integer>[0, 2]>[0, 2]`; the enum and record cases also hold as an operation parameter and as an operation result. None is refused `unknown_required_feature`/`unsupported-feature`. | Test (TC-1940) |
| FR-569-AC-13 | Source route. A field typed `Population<Order>[3]`, and a field typed by the name of a source function, are each refused at the type with the code and cause a source record field of the same type gets, and the unit yields no checked package. | Test (TC-1940) |
| FR-569-AC-14 | Waits on QC-30 (target emission). Over fixture S, S4 emits a package that FSL's I2 reader admits, holding `Order` as a `model`/`object_type` node with `declaration` `["Order"]` and the unit's `SourceOwner`. With `Order`'s owner replaced by a `ModelOwner` naming fixture D's package and IR node of `Order`, the I2 reader refuses `malformed_wire` at the node's `owner`; with its `declaration` removed and its `SourceOwner` kept, it refuses `malformed_wire` at the node's `owner`. A member type of AC-12 emits a binding naming the type node a record field of that type is bound to. | Test (TC-1941) |
| FR-569-AC-15 | Source route. After a source object unit is assembled, its view holds every member type as below. `f: Decimal[0, 100; 2, 2; exact]` holds `Resolved` of that decimal type. `s: Sequence<Set<Integer>[0, 2]>[0, 2]` holds multiplicity `0..2` (ordered) and `Resolved` of `Set<Integer>[0, 2]`, not a sequence nested inside a sequence. `r: Reference<Order>` holds `Package` of `Order`'s key, and `c: Clock` holds `Clock`, and `o: Option<Sequence<Integer>[0, 3]>` holds multiplicity one and `Resolved` of the whole option. Every member's `value_type` equals the checked type of the same form as a record field. Over `object Base { operation f(p: Int[0, 9]): Integer; }`, an override `f(p: Integer): Int[0, 9]` checks and an override `f(p: Int[0, 3]): Integer` is refused `ill_typed`/`variance-parameter`. With `M` selecting a model whose `Leaf` extends `Node`, an override narrowing an operation result from `Option<Reference<M::Node>>` to `Option<Reference<M::Leaf>>` checks, and, over a base whose result is `Option<Reference<M::Leaf>>`, an override widening it to `Option<Reference<M::Node>>` is refused `ill_typed`/`variance-result`. An override redefining a result `Reference<Order>` as `Option<Reference<Order>>` is refused `ill_typed`/`variance-result`. An override redefining a parameter `Decimal[0, 100; 2, 2; exact]` as `Decimal[0, 200; 2, 2; exact]` is refused `ill_typed`/`variance-parameter`. A member `w: W` of `object O`, where `record W { c: Clock; }`, in a unit that also declares `dimension Time;`, `unit ms : Time = rational(1, 1);` and `time O dense unit ms;`, passes the whole S3 check, timed-model step included, and records the clock place `w.c` that a record field of type `W` records; a `Sequence<W>[0, 2]` member and a member typed by a record holding `w: W` record the places their record fields record. | Test (TC-1940) |


## Dependencies

- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md):
  S2 forms, assembly and the catalog codes used above.
- [FR-081](FR-081-preserve-model-correspondence-and-declaration-identity.md),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md),
  [FR-154](FR-154-scope-a-modifies-entry-to-the-receiver.md): the effective
  view, operations, frames and scoped entries the source model reuses.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) and
  [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md):
  declared-node identity, record field bindings, clause functions and the
  member naming the source type reuses.
- [FR-104](FR-104-check-state-clauses.md),
  [FR-105](FR-105-emit-state-nodes.md),
  [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-391](FR-391-name-a-population-in-a-state-clause.md): clause checking,
  emission, admission and evaluation over the source model.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) and
  [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md): roles,
  the `owner_model` of a protocol over a source-declared type and its interim
  refusal.
- [FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md),
  [FR-531](FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md),
  [FR-517](FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md):
  the source-operation rows and the identity file that carries the
  source-object rows.
- FSpec FR-150, FR-151 (effective view, conformance, redefinition and
  dispatch), FR-322 (node identity, owners, field members, population
  references), FR-342 (operation anchors), FR-464 and FR-465 (protocol owner
  and role model).
- FSpec FR-610: the standard's object declaration, member-type rule and
  identity.
- [FR-710](FR-710-author-a-domain-package-in-an-fsl-model-unit.md): the model
  unit that declares the domain-package constructs a source object type does
  not.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  QC-30: the FSpec changes and the IR changes the target emission needs.
  Until they land, S4 gives the single `SourceObjectEmission` refusal above;
  the source route is complete now.
