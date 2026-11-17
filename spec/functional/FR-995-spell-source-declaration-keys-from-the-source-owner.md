---
id: FR-995
title: "Spell source declaration keys from the source owner"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-494
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/filament-specification/FR-150
    type: implements
  - target: ix://agent-ix/filament-specification/FR-322
    type: implements
  - target: ix://agent-ix/filament-specification/FR-610
    type: implements
---
# FR-995: Spell source declaration keys from the source owner

## Description

When S4 emits a checked package for a unit that declares source object
types, every source declaration key it writes SHALL be FR-569's "Source
declaration keys" spelling: an FSpec FR-150 original declaration key
`{package, node}` with `package` the declaring unit's `SourceOwner`
`identity` and `node` `ix://{identity}/{type}` for a type, or
`ix://{identity}/{type}/{member}` for a member. Each FR-494 dispatch
candidate's `selection_key` SHALL be the FSpec FR-150 `EffectiveId` over that
key, `owner_effective_type` `null`, exactly as FSpec FR-322-AC-290 requires
of every candidate.

## Rationale

FSpec FR-322-AC-290 makes a candidate's selection key the producer's exact
FR-150 `EffectiveId`, never a name and never derived from a graph node.
FR-150 keys a declaration by its package identity and its IR node identity,
and a domain package spells the IR node identity of a type `T` as
`ix://{package}/{T}`. A source declaration has no domain package; the string
that owns it in the package is its `SourceOwner` `identity`, which already
stands in for the document in an `operation_effects` row (FSpec FR-322
"Source-declared object types" item 6). Using that string in the key's
`package` and the domain spelling in its `node` gives a key that is unique
per unit (two units of one package have two owners), that a reader recomputes
from the node's `SourceOwner` and qualified name alone, and that never equals
a domain package's key, whose `package` is a ModelSelection identity. A key
built from the checked package's name or the unit's position among the
package's units would carry information no reader has, so a reader could
only accept it on trust.

## Inputs

A checked graph whose source model (FR-569) declares object types, and the
unit's `SourceOwner`.

## Outputs

The emitted keys: dispatch `selection_key` values, `resolved_path` member
identities (FR-996) and any other member naming a source declaration.

## Behavior

- The `package` of a source declaration key SHALL equal the `identity` of
  the `SourceOwner` the declaring node carries.
- The `node` SHALL be `ix://{identity}/{type}` for an object type or native
  record `{type}` and `ix://{identity}/{type}/{member}` for a field or
  operation `{member}` it declares; `{type}` and `{member}` are the source
  names, and `{identity}` is the same string as `package`.
- An emitted dispatch candidate's `selection_key` SHALL be `{owner_effective_type:
  null, original: <the key of the candidate's receiver type>}`.
- No emitted key SHALL use another string in place of the `SourceOwner`
  identity; S4 rewrites any check-time key whose `package` differs before it
  writes the member.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-995-AC-1 | For FR-569's fixture S, the dispatch table of the `self.size()` call in `Sized` has two candidates, `Order` then `Sub`, whose `selection_key`s are `{owner_effective_type: null, original: {package: I, node: "ix://I/Order"}}` and the same with `ix://I/Sub`, where `I` is the `identity` of the `SourceOwner` that `Order`'s node carries and equals the `document` of `Order`'s `operation_effects` rows. | Test (TC-2816) |
| FR-995-AC-2 | No emitted member of fixture S holds the checked package's name as a source key's `package` or under `ix://`, and the same unit compiled as the second of two units of one package that each declare an object type gives each unit's keys its own `SourceOwner` identity, so the two `Order` keys differ. | Test (TC-2816) |
| FR-995-AC-3 | The FR-995-AC-1 package read back through FSL's strict I2 read admits the dispatch table; with a candidate's `selection_key.original.node` replaced by the bare `Order`, or its `package` replaced by another unit's `SourceOwner` identity, the read refuses `invalid_package`/`invalid-value` at that candidate's `selection_key` (FSpec FR-788-AC-2). | Test (TC-2817) |

## Dependencies

- [FR-569](FR-569-declare-object-types-populations-and-operations-in-fsl-source.md)
  "Source declaration keys" pins the spelling; FR-494 owns dispatch tables.
- FSpec FR-150 (the key), FR-322-AC-290 (the selection key) and FSpec FR-788
  (the reader's join of a source selection key).
