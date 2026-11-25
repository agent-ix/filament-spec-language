---
id: FR-135
title: "Check a refinement declaration's subjects and state mapping at S3"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-018
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-110
    type: depends_on
---
# FR-135: Check a refinement declaration's subjects and state mapping at S3

## Description

FSL's layer-3 `check` module SHALL check a refinement declaration as a
`Relation` family declaration (ADR-012 §3, ADR-017 AR-1) and produce a
`CheckedRefinement` (ADR-020 RM-1 to RM-4). This requirement covers the
header, the population map and the object map. Step rows are FR-136,
fairness rows FR-137, history rows FR-138 and population-valued expressions
FR-139. The declaration's surface spelling is the shared grammar's
(ADR-020 QS-1); this requirement checks the parsed S2 form.

## Use case

A specification author writes a refinement between an abstract model and a
concrete model. FSL tells them, at compile time and at the source span,
which abstract population or field has no mapping, which has two, and
which mapping expression has the wrong type, before any model check runs.

## Inputs

- The parsed S2 refinement form: name, profile selection, abstract and
  concrete model aliases, population rows, object rows with field rows, step
  rows, history rows and fairness rows, each with its span.
- The unit's admitted domain packages and model declarations (FR-103), the
  resolved profile selection (FR-110), and, when the abstract alias names a
  dependency's model, that dependency's checked package from the S4
  closure (ADR-015). For both aliases, the checker also retains the selected
  model package's canonical identity and version from the existing model
  selection; aliases are source-level names and are not emitted identities.

## Outputs

```rust
pub struct CheckedRefinement {
    pub name: DeclarationKey,
    pub profile: ProfileSelection,              // infinite-trace
    pub abstract_side: AbstractSide,
    pub concrete_model: ModelAlias,             // the declaring package
    pub concrete_protocol: Option<NodeKey>,     // the clause the step rows select (FR-136); None for a model subject
    pub populations: Vec<PopulationRow>,        // abstract pop <- concrete pop
    pub objects: Vec<ObjectRow>,
    pub steps: Vec<StepRow>,                    // FR-136
    pub history: Vec<HistoryRow>,               // FR-138
    pub concrete_fairness: Vec<FairnessConstraint>, // F_C, FR-137
    pub abstract_fairness: Vec<FairnessConstraint>, // F_A, FR-137
}

pub enum AbstractSide {
    Local(ModelAlias),
    Dependency { alias: ModelAlias, package_id: PackageId },
    Protocol { alias: ModelAlias, clause: NodeKey, dependency: Option<PackageId> }, // FR-147
}

pub struct ObjectRow {
    pub abstract_type: DeclarationKey,
    pub concrete_type: DeclarationKey,
    pub fields: Vec<FieldRow>,                  // visible fields
    pub hidden: Vec<DeclarationKey>,            // ADR-020 AX-2
}

pub struct FieldRow { pub field: DeclarationKey, pub expr: CheckedExpr }
```

A typed `CheckRefusal` with a span, and no `CheckedRefinement`, on refusal.

## Behavior

### Header and subjects

- The checker SHALL resolve the concrete alias to a `model` declaration of
  the declaring unit, and the abstract alias to a `model` declaration of the
  declaring unit or of a dependency package compiled against supplied
  libraries. An alias that resolves to neither SHALL refuse
  `missing_declaration`/`missing-name` at the alias's span.
- The checker SHALL refuse a refinement whose unit does not select the
  infinite-trace profile with `unknown_profile`/`wrong-selection-role` at
  the alias, as FSpec FR-375 states.
- The abstract subject's contract clauses SHALL be read from the declaring
  package when the abstract alias is local, and from the dependency's
  checked package otherwise.
- Each selected model package SHALL have one whole-model root identity,
  formed from that existing selection's canonical identity and version. This
  root names the model as a whole regardless of its number of object types or
  `state_machine` declarations. S3 SHALL retain the selected identity and
  version for each side in the checked refinement binding. It SHALL NOT choose
  an object type or state machine as a representative and SHALL NOT require a
  new FCD declaration or model-selection field.
- The header's abstract side is FSpec FR-375's `abstract-subject` production,
  `ident [ '::' member-name ]`: an alias alone selects a model subject and
  `abstract <A>::<protocol>` a protocol subject, which FR-147 resolves. The
  header's concrete side is one model alias and never names a protocol: the
  concrete protocol, when there is one, is selected by the step rows, one
  protocol per refinement (FR-136 "Selecting the concrete protocol").
- A concrete protocol's step-row coverage SHALL be checked after the
  protocol clause is checked, not with the rest of the declaration (FR-136
  "Order of the checks").

### Population map

- For each population of the abstract model, the checker SHALL require
  exactly one `population` row naming it on the left and one population of
  the concrete model on the right. An abstract population with no row SHALL
  refuse `missing_declaration`/`missing-name`, naming the population; one
  with two rows SHALL refuse `invalid_model_binding`/`conflicting-binding`,
  naming both rows.
- A row whose left side is not an abstract population, or whose right side
  is not a concrete population, SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at the row's span.
- A concrete population that no row names SHALL be concrete-only state and
  SHALL need no row.

### Object map

- For each object type that an abstract population admits, the checker
  SHALL require exactly one `object` row whose `from` names an object type
  of the row's source population. A missing row SHALL refuse
  `missing_declaration`/`missing-name`, naming the abstract type; two rows
  SHALL refuse `invalid_model_binding`/`conflicting-binding`; a `from` type
  outside the source population SHALL refuse
  `invalid_model_binding`/`malformed-declaration`.
- A `state_machine` declaration's declared fields are a field set in the
  model state and SHALL be represented by one `object` row naming that
  `state_machine` declaration as the row's abstract type and a concrete
  `state_machine` declaration as its source. This row is not population-bound:
  it maps the machine's fields and creates neither an object nor a population.
  Each abstract `state_machine` declaration SHALL have exactly one such row;
  a missing row SHALL refuse `missing_declaration`/`missing-name`, duplicate
  rows SHALL refuse `invalid_model_binding`/`conflicting-binding`, and a row
  pairing a `state_machine` declaration with an object type SHALL refuse
  `invalid_model_binding`/`malformed-declaration`. Multiple state-machine
  records are selected by their rows and are not ambiguous.
- A model with no `state_machine` declaration is still a valid refinement
  subject. Its object-type state is represented by the population and object
  rows above; there is no requirement to synthesize or select a
  `state_machine` node.
- Each field row `f = e` SHALL name a declared field of the abstract type.
  A field named twice SHALL refuse
  `invalid_model_binding`/`conflicting-binding`, naming both rows. An
  undeclared field SHALL refuse `missing_declaration`/`missing-name`.
- For a state-machine row, `self` in each field expression SHALL be bound to
  the concrete machine field set; for an object-type row it SHALL be bound to
  the concrete object type. In both cases the checker SHALL read concrete
  fields, functions, history fields (FR-138) and population names and forms
  as FR-139 states, admitting the model reads a state-clause body admits
  (ADR-016 FE-4).
- The checker SHALL lift references: a field of declared type
  `Reference<T_A>` SHALL accept an expression of type `Reference<T_C>`
  exactly when an object row maps `T_A` from `T_C`.
- The checker SHALL lift a `Reference<T_C>` nested inside an option,
  collection or map type, at every depth, to the `Reference<T_A>` of the object
  row that maps `T_A` from `T_C`, element by element, and in a map the key as
  well as the value (FR-630, ADR-020 RM-3). It SHALL admit a whole-population
  reference row (`Set<Reference<T_C>>[0, n]`, FR-139) as FR-139 types it. A
  `Reference<T_C>` nested inside a record, tuple or union SHALL be lifted by
  the same rule. The concrete and abstract composites are paired componentwise:
  record fields by field name, tuple elements by position when the tuples have
  equal arity, and the active union values by variant tag. The paired component
  values recursively use this same lifting rule. A missing or extra record
  field, a tuple arity mismatch, or an active union tag absent from the paired
  union SHALL refuse `ill_typed`/`type-mismatch` at the first mismatched
  component path; the checker SHALL produce no `CheckedRefinement`.
- The checker SHALL refuse a refinement whose abstract model declares a
  relationship with `unsupported_construct`/`declaration-form` at the
  abstract alias of the declaration header, once per refinement, naming the
  form, the refinement and the exact selected profile identity, whatever the
  support set holds, and SHALL produce no `CheckedRefinement` (FSpec
  FR-375). No row maps a relationship (ADR-020 RM-2 to RM-4), so its links
  could never hold in a mapped state; this is a gap in the language, not a
  feature this build has left unbuilt.
- The checker SHALL admit each field row, step-row argument and history update
  under the layers the governing infinite-trace profile requires for a
  refinement's rows (`filament.value.complete/v1` and `filament.model.complete/v1`
  with their `requires` closure), whatever the unit's other declarations
  select, and under no other layer (FSpec FR-375 "Row layers"). A form outside
  those layers SHALL refuse `unsupported_construct`/`expression-form` at its
  first token, naming the layer that admits it.
- The checker SHALL type `Some(e)` as `Option<E>` for `e` of type `E`, and
  `none` by the abstract field's type or the other arm of an enclosing `if`;
  a field declared `?` takes a row of type `Option<T>` and a field declared
  `Option<T>` takes its option value unchanged (FSpec FR-375 "Option rows").
- The checker SHALL check `e`'s type against the abstract field's declared
  type, after reference lifting, with `TypeEnvironment::conforms` (ADR-016
  SC-2), and SHALL refuse `ill_typed`/`type-mismatch` at `e`'s span when it
  does not conform.
- An abstract field with no row SHALL be recorded in `hidden` (ADR-020
  AX-2).

### Refinement features the build does not check

FSpec FR-375 and the FR-136 to FR-149 family specify more refinement forms
than a build may yet check. FSpec FR-375 owns the closed set of feature names
and the not-yet-built refusal; this section states how S3 applies them. The
build holds a closed **support set** of refinement feature names,
a build capability constant and not a request or test parameter. FSpec
FR-375's closed set holds four feature names, which it names: `history-row` (FR-138), spelled by a `history` row; `protocol-abstract-side` (FR-147), spelled by `abstract <A>::<protocol>`;
`refinement-internal-list` (FR-147), spelled by a non-empty resolved internal
list for the refinement, which is located at the first token of the name after
`abstract`, once per refinement; and `nested-reference-row`, spelled by a
field row whose type nests a reference (below), at the row's expression. The list is resolved first and only a
non-empty resolved list reaches this refusal (FR-147).

The internal list is the compilation's optional `refinement_internals`
input, which FSpec FR-375 names and FSL's compile input carries (FR-099): one
JSON value for the unit and one for each supplied library, each naming only
refinements declared in its own source. Its feature is
`refinement-internal-list`. The input is handed to S3 as supplied, and S3
resolves it per refinement (FR-147): shape, spelling, resolution and
repetition defects are located by the pair (input, RFC 6901 JSON Pointer)
and refuse the compilation whatever the support set holds; a non-empty
resolved list then refuses for the feature when the support set omits it,
and refuses `invalid_model_binding`/`malformed-declaration` for a model
subject once the feature is built.

- A declaration that uses a feature outside the support set SHALL refuse
  `unsupported_construct`/`not-yet-implemented`, once for each row or refinement that uses
  the feature, and SHALL produce no `CheckedRefinement`. This is the refusal
  FR-341-AC-5 classifies `unsupported` (an FSL implementation gap).
- The refusal's span is the span of the `<protocol>` name for a protocol
  abstract side. It names the feature, the refinement's name and the exact
  selected profile identity (FSpec FR-272's payload for the code). The payload has no
  `subject` member.
- The refusal is decided before the feature's own checks, so no other
  diagnostic about that row is reported; the rest of the declaration is
  checked as usual and its own refusals are reported beside it.
- This build's support set SHALL hold `protocol-abstract-side`
  and `refinement-internal-list`, which FR-147 checks, and `nested-reference-row`,
  whose rows FR-630 lifts element by element: a `Reference<T_C>` nested inside an
  option, collection, map value, record field, tuple element or union payload
  maps to the `Reference<T_A>` of the object row that maps `T_A` from `T_C`, at
  every nesting depth. Composite components are paired by record field name,
  tuple position with equal arity, or active union variant tag; each paired
  value is lifted recursively. A missing or extra field, unequal tuple arity,
  or active tag without a paired case refuses `ill_typed`/`type-mismatch` at
  the first mismatched component path.
- S3 SHALL check a `history` row whose declared type is an enum type or a
  decimal type, or an option of one, as it checks an integer row: the history key
  encodes each such value by its canonical literal (FR-630). FR-138-AC-9 states
  the case.
- A feature in the support set is checked by its own requirement.

`unknown_required_feature` is not used: a source refusal, which FR-341
classifies `refused`, would tell the author the unit is wrong when the build
is incomplete. `unimplemented_capability` is not used: FR-271 defines it as
valid complete-V1 meaning the selected producer does not implement, which
fits, but FR-272 lists no cause for it. `not-yet-implemented` under
`unsupported_construct` is the pair FSpec FR-375 and FR-272 name for this
standing.

### Package and identity

- The `CheckedRefinement` SHALL enter the checked package as a node, and its
  node key enters the obligation identity of every item that cites it
  (ADR-013 O-09). Its v2 wire node, node id included, is FSpec's (FSpec
  FR-375; ADR-020 QS-4). Remaining work: QSL-3277 (the `path` FSL writes in
  the node id).
- A row SHALL be part of the node's body and not of its node key. The key is
  the declaring package, a `path` the refinement's authored name determines
  and ordinal 0 (FSpec FR-375, FR-322), so two checks of one package whose
  refinement of one name differs in a row SHALL give that refinement equal
  node keys and unequal node bodies. Renaming the refinement SHALL change the
  key's `path`.
- The package identity (identity and version, ADR-013 O-02) SHALL NOT depend
  on a row. Two checked packages that differ in a row are told apart by
  comparing their values (FSpec FR-322).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-135-AC-1 | ADR-020 §8's `CasRefinesCounter`, with `Spec` and `Impl` both local, checks to a `CheckedRefinement` with one population row (`counters` from `counters`), one object row with the field row `value = self.value` and no hidden field. The same declaration with `Spec` supplied as a dependency package checks with `AbstractSide::Dependency` carrying that package's `package_id`. | Test (TC-540) |
| FR-135-AC-2 | Refusals, each at its span with no `CheckedRefinement`: the abstract alias `Nope`, resolving to no model (`missing_declaration`/`missing-name`); the `population` row removed (`missing_declaration`/`missing-name` naming `Spec::counters`); the row written twice (`invalid_model_binding`/`conflicting-binding`); the row reversed as `population Impl::counters from Spec::counters` (`invalid_model_binding`/`malformed-declaration`); the `object` row removed (`missing_declaration`/`missing-name` naming `Spec::Counter`); `value` mapped twice (`invalid_model_binding`/`conflicting-binding` naming both rows); the declaration in a unit that selects a bounded profile (`unknown_profile`/`wrong-selection-role` at the alias). | Test (TC-540) |
| FR-135-AC-3 | `value = self.busyA` refuses `ill_typed`/`type-mismatch` at the expression's span. In ADR-020 §8's `RingIsQueue`, a field of type `Reference<Q::Queue>` mapped by `self.ring` (of type `Reference<R::Ring>`) checks, and the same field mapped by an expression of type `Reference<R::Slot>` refuses `ill_typed`/`type-mismatch`. | Test (TC-540) |
| FR-135-AC-4 | With the `value` row removed, `CasRefinesCounter` checks with `value` in `hidden`. Checked with the row `value = self.value` and, in a package of the same identity and version, with `value = self.tmpA`, the two refinements have equal node keys (equal `package` and `path`, ordinal 0 in both) and unequal node bodies: the one object row's field row `value` reads `self.value` in the first and `self.tmpA` in the second, and neither has a hidden field. The two checked packages have the same package identity (ADR-013 O-02). | Test (TC-540) |
| FR-135-AC-5 | A declaration whose abstract side is a protocol (`CasTwice`, FR-147), in a build whose support set omits `protocol-abstract-side`, refuses `unsupported_construct`/`not-yet-implemented` at the span of `Twice`, naming `protocol-abstract-side`, `CasTwice` and the profile identity, with a payload that has no `subject` member, no other diagnostic about the abstract side and no `CheckedRefinement`. It classifies `unsupported` (FR-341-AC-5). `CasRefinesCounter`, which uses none of the features, checks as FR-135-AC-1 states. The vector refuses a row that uses a feature the build has not built; it does not select a support set. | Test (TC-540) |
| FR-135-AC-6 | `RingIsQueue` with a `Sequence<Reference<Queue>>[0, 2]` field mapped by a row of type `Sequence<Reference<Ring>>[0, 2]` checks, each nested reference lifted element by element through the object row that maps `Queue` from `Ring` (FR-630), and so do an `Option<Reference<Ring>>` row for an `Option<Reference<Queue>>` field and a `Sequence<Option<Reference<Ring>>>[0, 2]` row for a `Sequence<Option<Reference<Queue>>>[0, 2]` field, an option, collection or map at any depth; map-key and map-value lifting at any depth is also asserted by TC-1086/FR-140-AC-16's `lift_nested` unit cases, since no validated package declares a map type. A population-reference row `cells = R::slots` checks, as does the same field as a plain `Reference<Queue>` mapped by `self`. A nested reference whose concrete type no object row maps (a `Sequence<Reference<Slot>>[0, 2]` row for the `Sequence<Reference<Queue>>[0, 2]` field, `Slot` mapping to `Cell`) refuses `ill_typed`/`type-mismatch` at the row's expression, as a top-level one does (AC-3), and no `CheckedRefinement`. | Test (TC-540) |
| FR-135-AC-7 | `RingIsQueue` with the abstract `Queue` declaring a relationship refuses `unsupported_construct`/`declaration-form` at the abstract alias `Q` of the declaration header, once, whatever the support set holds, and no `CheckedRefinement`. | Test (TC-540) |
| FR-135-AC-8 | In a unit whose own declarations select no layer, `RingIsQueue`'s rows `s.ring = self` and `(self.head + i) mod 2` check, because they are admitted under the layers the governing infinite-trace profile requires; a Boolean field row naming a protocol declared in the unit refuses `unsupported_construct`/`expression-form` at the `protocol` token, naming the layer that admits it (`filament.protocol.complete/v1`). | Test (TC-540) |
| FR-135-AC-9 | A refinement given a non-empty resolved internal list refuses `unsupported_construct`/`not-yet-implemented` naming `refinement-internal-list` at the first token of the name after `abstract`, once for the refinement, with no `subject` member, no `CheckedRefinement` and the classification `unsupported` (FR-341-AC-5); a list with a resolution defect reports the defect and not this pair (FR-147-AC-7, AC-11). | Test (TC-540) |
| FR-135-AC-10 | The header `abstract Spec::Twice concrete Impl` is read as FSpec FR-375's `abstract-subject` (model alias `Spec`, protocol `Twice`), and `abstract Spec concrete Impl` as a model subject; the concrete side never names a protocol, and a refinement whose step rows name one protocol clause of `Impl` records it as `concrete_protocol`, one whose rows name none records `None` (FR-136-AC-6). | Test (TC-540) |
| FR-135-AC-11 | A mapped `state_machine` declaration has one `object` row whose target and source are both state-machine declarations; its declared fields check through that row without a population row. Omitting the row refuses `missing_declaration`/`missing-name`; duplicating it refuses `invalid_model_binding`/`conflicting-binding`; pairing it with an object type refuses `invalid_model_binding`/`malformed-declaration`. `CasRefinesCounter` over the object-type-only `Spec` and `Impl` models checks with its existing population and object rows, with no `state_machine` declaration or synthetic subject. | Test (TC-540) |
| FR-135-AC-12 | For each selected abstract and concrete model package, the checked refinement binding retains that selection's canonical identity and version; it does not use the source alias or a declaration as a representative. A model-only header binds to the whole-model identity; protocol refinement retains the same abstract model identity plus its exact protocol-clause key. Zero, one and multiple object types or state machines are all valid cardinalities for the same binding rule. The binding is derived from existing model selection state and adds no FCD field. | Test (TC-540) |
| FR-135-AC-13 | With `record Box { target: Reference<Queue>; }` on the abstract `Queue` and `record Box { target: Reference<Ring>; }` on concrete `Ring`, and `object Q::Queue from R::Ring { box = self.box; }`, S3 accepts the row and records its expression as `Box` with `target` paired by name; the nested reference type is lifted through the object row `Queue from Ring`. | Test (TC-540) |
| FR-135-AC-14 | With `tuple Pair(Reference<Queue>, Boolean);` on the abstract `Queue`, the same-arity `tuple Pair(Reference<Ring>, Boolean);` on concrete `Ring`, and the row `pair = self.pair` where `pair` has type `Pair` on each side, S3 accepts the row and pairs element 0 by position, lifting `Reference<Ring>` to `Reference<Queue>` while retaining element 1. | Test (TC-540) |
| FR-135-AC-15 | With `union Holder { Empty, Has(Reference<Queue>) }` on the abstract `Queue` and `union Holder { Empty, Has(Reference<Ring>) }` on concrete `Ring`, and `holder = self.holder`, S3 accepts the row; a `Holder::Has(r)` value pairs the active `Has` case by tag and lifts its payload reference through the `Queue from Ring` object row, while `Holder::Empty` retains that tag. | Test (TC-540) |
| FR-135-AC-16 | With `record Parcel { owners: Sequence<Option<Reference<Queue>>>[0, 2]; }` on the abstract `Queue` and the corresponding `Parcel` whose `owners` type is `Sequence<Option<Reference<Ring>>>[0, 2]` on concrete `Ring`, `parcel = self.parcel` is accepted; record field `owners`, sequence elements and `Some` payloads are paired recursively, `None` is retained, and every nested reference is lifted through `Queue from Ring`. | Test (TC-540) |
| FR-135-AC-17 | When the abstract record field is `Box.target: Reference<Queue>` and the concrete paired record has `link: Reference<Ring>` instead of `target`, a row `box = self.box` refuses `ill_typed`/`type-mismatch` at the mismatched record-field path `box.target`, and produces no `CheckedRefinement`. | Test (TC-540) |
| FR-135-AC-18 | When the abstract field type is `Pair(Reference<Queue>, Boolean)` and the concrete row value has type `Triple(Reference<Ring>, Boolean, Boolean)`, a row `pair = self.triplet` refuses `ill_typed`/`type-mismatch` at the first unmatched tuple position `[2]`, and produces no `CheckedRefinement`. | Test (TC-540) |
| FR-135-AC-19 | When the abstract field has type `Holder` with case `Has(Reference<Queue>)` and the concrete field has type `State` with case `Points(Reference<Ring>)`, row `holder = self.state` refuses `ill_typed`/`type-mismatch` at the active union-tag path `holder.variant`, and produces no `CheckedRefinement`. With a concrete `State` whose active tag is `Has` and whose payload is `Reference<Ring>`, the same row is accepted and its payload is lifted. | Test (TC-540) |

## Dependencies

- [FR-099](FR-099-compile-against-supplied-libraries.md) (the compile input
  that carries `refinement_internals`).
- ADR-020 §1 RM-1 to RM-4 and §4 AX-2; ADR-012 §3; ADR-017 AR-1; ADR-016
  SC-2 and FE-4; ADR-013 O-02 and O-09; ADR-015.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md) (admitted models),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (profile
  selection).
- FR-136 to FR-139 check the remaining rows into the same
  `CheckedRefinement`.

## References

- The FSpec half (grammar, semantics, wire node): FSpec FR-375 (Linear STD-133).
  This build checks every `history` row, so it holds no `history-row` refusal
  (FR-138-AC-9).
