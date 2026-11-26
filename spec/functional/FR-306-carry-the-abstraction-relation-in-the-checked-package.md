---
id: FR-306
title: "Carry the abstraction relation in the checked package"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-031
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-353
    type: depends_on
---
# FR-306: Carry the abstraction relation in the checked package

## Description

When the S4 emitter writes a checked package whose unit declares an
abstraction relation, the emitter SHALL write the `CheckedAbstractionRelation`
as one `filament.checked-package/v2` node, so the relation is part of the
checked package's value, and the checked package SHALL be the relation's only
authority.

## Inputs

- The `CheckedAbstractionRelation` of
  [FR-304](FR-304-check-an-authored-abstraction-relation.md) in the
  in-process `CheckedPackage`.

## Outputs

The v2 relation node, carrying every binding with its key and value, in the
emitted package.

## Behavior

### One authority

The relation SHALL be emitted as the v2 node whose tag, form and body FSpec
spells for the FSpec FR-353 relation (FSpec FR-451), carrying ADR-017 AR-2's
keys and values. Kani and Verus generation read the relation only from the v2
bytes (ADR-011 FB-05).

### Rebinding is a new package value

Changing a binding SHALL change the relation node's body, and so the checked
package's value (FSpec FR-451). The package identity (identity and version,
ADR-013 O-02) does not depend on a binding. A rebinding is a new relation
revision and SHALL NOT reinterpret a contract generated from the earlier
package (FSpec FR-353-AC-5). The two revisions are told apart by comparing
checked-package values: while the bindings target the same nodes, their
relation nodes have an equal `node_id` (FSpec FR-451 `node_id`), and the two
packages are unequal values, so they are not the same package (FSpec FR-322).

A binding made on a linked package (`CheckedPackage::bind_abstraction` or
`bind_abstraction_at`) derives again everything checking derives from the
relation: each guarantee's item refusal (FSpec FR-490-AC-6) and relation
node, and each step relation's claim node, implementation references,
occurrences and requirement records. A graph shared before the binding is
unchanged. A relation a derivation cannot write as a node refuses with the
node refusal as its cause, on a binding as on a check.

### The relation is a premise, not a claim

The relation SHALL record no requirement record (FSpec FR-290: it has no
capability kind; [FR-057](FR-057-admit-shared-capability-kinds.md)-AC-10).
Each claim that references bound elements keeps its own kind
(`operation-contract` or `value-validity`) and its extent, and `route`
routes it as ADR-012 §7.2 states. The relation is not an S6a input: a
counterexample from a claim that used it replays as clause expressions
through S6a (ADR-012 §8; ADR-017 AR-5).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-306-AC-1 | A unit with FR-304-AC-1's relation emits one relation node, and reading the emitted v2 bytes yields the same three keys and binding values that the in-process `CheckedAbstractionRelation` holds. | Test (TC-804) |
| FR-306-AC-2 | Recompiling the unit unchanged gives an equal relation node id and body. Changing one `ObjectBinding` field's `RustField` gives a relation node with an equal node id and an unequal body, whose `objects` entry for that type holds the new `RustField` in `fields` where the first holds the old one, and the changed package reads back. | Test (TC-804) |
| FR-306-AC-3 | The unit's requirement records and their `route` results are equal with and without its relation declaration. | Test (TC-805) |
| FR-306-AC-4 | A clause run of the unit's ConfigVersion clauses through the spine run entry gives the same disposition with and without its relation declaration. | Test (TC-805) |
| FR-306-AC-5 | After a successful binding on a linked package, including one that drops a frame or changes the nodes the bindings target, each guarantee's item refusal and relation node, and each step relation's claim node, implementation references and requirement records and claim occurrences, equal those of checking a unit that declares the bound relation, and a graph obtained from `shared_graph` before the binding is unchanged. | Test (TC-804) |
| FR-306-AC-6 | When a guarantee, or a step relation reading a bound frame, cannot be derived because the relation has no node (a Rust path with an empty segment), a binding and a check of a unit declaring that relation both refuse `invalid_model_binding`/`malformed-declaration` carrying that cause, not an internal fault, and the refused binding leaves the package unchanged. | Test (TC-804) |

## Dependencies

- **Upstream:** FSpec FR-353 and FR-290 own the relation's semantics and its
  absence of a capability kind; FSpec FR-451 spells the v2 node; ADR-017
  AR-4 and AR-5 fix the authority and the dispatch rules.
- **Downstream:** [FR-307](FR-307-export-the-bindings-each-item-references.md)
  exports bindings from the checked package; CG's Kani and Verus generators
  read the v2 node.

## References

- ADR-017 §3 AR-4 ("One authority", "Relation revision") and AR-5.
- FSpec FR-451 (specification ticket STD-116, Q-2).
- Linear QSL-388 (specification), QSL-36 (implementation).
