---
id: FR-953
title: "Emit a presence quantifier over the slot domain of its clock collection"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-015
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-930
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-772
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-760
    type: depends_on
---
# FR-953: Emit a presence quantifier over the slot domain of its clock collection

## Description

For a quantifier that FR-930 writes in a timed clause predicate, such as
`forall(e in self.q: present(e))` or `exists`, the emitter SHALL write its
first operand as the slot domain of the quantified clock collection: the
FSpec FR-772 `filament.op.option.slots` application. The binder ranges over
the collection's live slots by their selectors, so `e` is a position of a
`Sequence` or a key of a `Map`, and `present(e)` reads whether that slot
holds a clock. Writing the collection's values as the operand would need a
value form for clocks, which the package does not have. Writing the
presences as the operand would leave no selector for the binder. The slot
domain is what the source binder ranges over.

The application's arguments are the `clock_place` template node of the
element place that `present(e)` reads, the owner term, and the outer slot
terms of a nested collection. Its `result_type` is `Set<D>`, bounded by the
collection's declared maximum. The binder parameter is typed `D`, the last
slot step's selector domain.

When the source quantifies over the place directly (`e in self.q`), the
emitter SHALL give the slot-domain node FSpec FR-772's derived path, with
the `clock_place` node as operand segment 0 and never as an embedded
`node_id` object. Every quantifier over one place at one owner term SHALL
share that one node.

## Inputs

A checked timed clause predicate holding a presence-only quantifier over a
clock collection place (FR-930).

## Outputs

The quantifier application whose operand 0 is the slot-domain node.

## Behavior

- S4 SHALL write the slot-domain node and the quantifier as above, and
  SHALL NOT write a collection value, a presence collection or an
  integer-range node as the quantifier's operand.
- FSL's I2 read of the package SHALL admit the node by FSpec FR-772's reader
  rule. Until the IR reader admits that rule, the IR's I2 refuses the
  operand `ill_typed`/`operator-ineligible`, and the item stays refused
  rather than being written in another form.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-953-AC-1 | FR-930-AC-1's guard `self.x >= 1 ms and forall(e in self.q: present(e))` emits a `forall` whose operand 0 is a `filament.op.option.slots` node with arguments `q`'s element `clock_place` node and `self` and no slot term, `result_type` the `collection_bounds` `[0, 2]` node over `Set<Integer>`, and `node_id.path` `["expression","presence_read","filament.op.option.slots",{"operand":0},{"operand":1}]` holding no `node_id` object; its binder parameter is typed `Integer`; FR-930-AC-2's time invariant and urgency over `self.q` reuse the same slot-domain node; and the package passes the I2 read of FSpec FR-772-AC-1. | Test (TC-2663) |
| FR-953-AC-2 | `forall(e in self.q: present(e))` evaluated at a snapshot where `q` has size 0 reads `true` and its `exists` form reads `false` (FSpec FR-415-AC-20); at size 2 with slot 1 absent, `forall` reads `false` and `exists` reads `true`; and the emitted guard of a package with two quantifiers over `self.q` and one over another collection `r` holds exactly two slot-domain nodes. | Test (TC-2663) |

## Dependencies

- FR-930: the quantifier's place in the predicate.
- FSpec FR-772, FR-760, FR-322 "Presence": the node, its identity and its
  reader rule.
