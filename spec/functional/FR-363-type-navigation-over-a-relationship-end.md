---
id: FR-363
title: "Type navigation over a relationship end through the StateModel family"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-300
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-085
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-152
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
---
# FR-363: Type navigation over a relationship end through the StateModel family

## Description

When a model expression `r.name` reads a relationship end of the receiver's
static object type, the `StateModel` family's `check` hook SHALL type it as a
navigation whose result follows the destination end's typed multiplicity.

When the declared direction, the destination multiplicity or the name
resolution does not admit a navigation, the hook SHALL refuse it with a
`StateModel`-owned cause.

## Inputs

- A field-read form `r.name` where `r` has the static type `Reference<T>` and
  the source wrote no `deref(...)` (FR-300's `Form::Attribute` inputs).
- The unit's admitted model effective view and its `TypeEnvironment`,
  including each admitted relationship's declared direction and both ends'
  declared type, role and multiplicity
  ([FR-085](FR-085-resolve-relationship-end-references.md)).

## Outputs

A checked `Navigate` node holding the receiver, the relationship's
declaration key and the end's role, with the static type below; or a located
`StateModel` refusal.

## Behavior

### Resolution of `r.name`

The hook SHALL make a relationship end navigable from a receiver type by the
role of the end nearest the receiver, so that a relationship declared by its
source type is read under the source end's role:

- the relationship's `sourceEnd.role` names the forward end: receivers whose
  static type conforms to the source end's type navigate to the target end;
- the relationship's `targetEnd.role`, when the target end declares one, names
  the inverse end: receivers whose static type conforms to the target end's
  type navigate to the source end. A target end with no role has no inverse
  end, and no name is synthesised for it.

For example, `Order/lines` is declared with source end `Order`, role `lines`,
and target end `Line`, role `owner`. `self.lines` is navigable from an `Order`
and gives the `Line` end's multiplicity; `self.owner` is navigable from a
`Line` and gives the `Order` end's multiplicity. `self.lines` from a `Line`, or
`self.owner` from an `Order`, finds no candidate.

Conformance is FR-082's, so a receiver whose type is a subtype of the end's
type navigates. The destination end is the end the role does not name.

`r.name` has the candidates formed by the field `name` of `T`'s effective view
and by every relationship end navigable from `T` under that role. It resolves
to the one candidate; otherwise:

- no candidate: `ill_typed`/`type-mismatch` at the form, the refusal an
  undeclared record field and an undeclared object field both get (FR-300);
- two or more candidates (a field and an end, or two ends):
  `ambiguous_declaration`/`ambiguous-name` at the form, naming every
  candidate.

In a `Value` function body as in a state clause, a receiver that is not a
`Reference<T>` (an `Option`, a collection or a population) refuses
`ill_typed`/`type-mismatch` at the receiver operand, as it does for a field. `deref(r).name` over a relationship end refuses
`ill_typed`/`type-mismatch` at the form: a relationship end is not a member of
the dereferenced object, and only `r.name` navigates.

### Direction

Traversal along the forward end requires the relationship's direction to be
`source-to-target`, `bidirectional` or `undirected`. Traversal along the
inverse end requires `target-to-source`, `bidirectional` or `undirected`. Any
other traversal refuses `ill_typed`/`operator-ineligible` at the form
(FSpec FR-152 "Navigation").

### Destination multiplicity and result type

The result type follows the destination end's typed multiplicity `[l, u]`,
where `u` is a number or `unbounded`:

| Destination end | Static type of `r.name` |
| --- | --- |
| `[0,1]` | `Option<Reference<U>>` |
| `[1,1]` | `Reference<U>` |
| any other finite `[l,u]`, `ordered: false`, `unique: true` | `Set<Reference<U>>[l,u]` |
| any other finite `[l,u]`, `ordered: false`, `unique: false` | `Bag<Reference<U>>[l,u]` |
| any other finite `[l,u]`, `ordered: true`, `unique: true` | `OrderedSet<Reference<U>>[l,u]` |
| any other finite `[l,u]`, `ordered: true`, `unique: false` | `Sequence<Reference<U>>[l,u]` |
| `[l, unbounded]` | the collection of the row above for its `ordered` and `unique` flags, with no cardinality bound |

where `U` is the destination end's type. The `ordered` flag of a
single-valued end (`[0,1]` or `[1,1]`) changes nothing, because one target
has no order. An unbounded end gives an unbounded collection type, the same
kind of collection type FR-335 settles claims over; its lower bound `l` is
not part of the static type and is decided at evaluation, as FSpec FR-152
decides the bound of a finite end (`cardinality_out_of_bound`/`below-minimum`).
An ordered end yields its targets in the order the observation's instances
give them for that end (FR-364 "Positions of an ordered end"); an unordered
end yields them in canonical reference-key order.

A destination end that declares no multiplicity refuses
`ill_typed`/`operator-ineligible` at the form. FR-085-AC-2 retains an absent
multiplicity as absent and substitutes none, so the model gives the
navigation no result type; that is a property of the model, not a capability
this build lacks, so the refusal is a typed semantic refusal and not
`unsupported_construct`. Direction is decided before multiplicity.


### Lowering

A checked `Navigate` lowers by FR-093's `Navigate` row to one
`filament.op.model.navigate` application whose `relationship_end` member names
the relationship's model node and the end's role (FR-094), whose argument is
the receiver, and whose `result_type` is the type node of the static type
above. The lowered application satisfies FSpec FR-322-AC-57's checks in order
(receiver endpoint, direction, destination multiplicity, canonical result
type) by construction.

Evaluating a navigation node is outside this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-363-AC-1 | `r.name` over a relationship end types as `Reference<U>` for `[1,1]`, `Option<Reference<U>>` for `[0,1]`, `Set<Reference<U>>[l,u]` for another finite unique `[l,u]` and `Bag<Reference<U>>[l,u]` for another finite non-unique `[l,u]`; a subtype receiver, an inverse end under a `bidirectional` relationship and a role-bearing target end each navigate; `present(self.order)` over a `[0,1]` destination checks. | Test (TC-1031) |
| FR-363-AC-2 | A name matching a field and a relationship end, or two relationship ends, refuses `ambiguous_declaration`/`ambiguous-name`; a name matching no candidate refuses `ill_typed`/`type-mismatch`, except that outside the declaring refinement's rows a name that is a history field name of any refinement of the unit refuses `missing_declaration`/`missing-name` (FR-138). | Test (TC-1031) |
| FR-363-AC-3 | A forward traversal against `target-to-source`, and an inverse traversal against `source-to-target`, refuse `ill_typed`/`operator-ineligible`; `undirected` admits both. | Test (TC-1031) |
| FR-363-AC-4 | A destination end with no declared multiplicity refuses `ill_typed`/`operator-ineligible` at the form, never `unsupported_construct`; a direction refusal is reported ahead of it. | Test (TC-1031, TC-1901) |
| FR-363-AC-5 | A non-`Reference` receiver, and `deref(r).name` over a relationship end, refuse `ill_typed`/`type-mismatch`; every refusal's family is `StateModel` and its catalog code carries the `state-model` prefix. | Test (TC-1031) |
| FR-363-AC-6 | A checked `Navigate` lowers to one `filament.op.model.navigate` application with the member, argument and `result_type` stated above and no `filament.op.model.deref` node, and IR's v2 reader admits the emitted package. | Test (TC-1032) |
| FR-363-AC-7 | `r.name` over an unordered unique end `{0,unbounded}` types as `Set<Reference<U>>` with no cardinality bound and over an unordered non-unique one as `Bag<Reference<U>>` with no bound; over an ordered unique end `{0,3}` it types as `OrderedSet<Reference<U>>[0,3]`, over an ordered non-unique `{0,3}` as `Sequence<Reference<U>>[0,3]`, and over an ordered unique `{0,unbounded}` as `OrderedSet<Reference<U>>` with no bound; an ordered `{0,1}` end types as `Option<Reference<U>>`. None of these refuses, and no navigation refusal has code `unsupported_construct`. | Test (TC-1901) |

## Dependencies

- **Upstream:** [FR-300](FR-300-check-model-forms-through-the-state-model-family.md)
  gives the hook and the refusal attribution; [FR-085](FR-085-resolve-relationship-end-references.md)
  retains each end's role, multiplicity and the relationship's direction;
  [FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md) supplies
  conformance; [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  and [FR-094](FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  own the lowered node and the model node it names. FSpec FR-152
  "Navigation" owns the direction rule and the result types; FR-322-AC-56 and
  AC-57 alone own the role lookup and the lowered node's checks. FSpec TC-197
  Y07 to Y09 (and an FSpec model-definition sentence) name a traversal by the
  far, destination end's role; this requirement follows FR-322's near-end rule,
  so a port of Y07 or Y08 must use the near-end role.
- **Downstream:** none.
