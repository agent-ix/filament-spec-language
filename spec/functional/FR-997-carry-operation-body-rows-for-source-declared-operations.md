---
id: FR-997
title: "Carry operation-body rows for source-declared operations"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-576
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/filament-specification/FR-322
    type: implements
  - target: ix://agent-ix/filament-specification/FR-790
    type: implements
---
# FR-997: Carry operation-body rows for source-declared operations

## Description

When S4 emits the FR-576 `operation_bodies` carrier for a package whose unit
declares source object types, the carrier SHALL hold one row
`{document, declaring, operation, body}` per operation each source-declared
`model`/`object_type` node declares, beside the rows of the selected
models: `document` the declaring node's `SourceOwner` `identity` (as its
`operation_effects` row has it), `declaring` that node, `operation` the
source operation name, and `body` `{kind: "function", node}` naming the
source body clause function `[<type>, <operation>, "body"]` exactly when the
unit gives the operation a `body`, and `{kind: "no_body"}` otherwise.

## Rationale

FSpec FR-322-AC-397 requires a row for every operation member of every
admitted `object_type` node and joins each dispatch candidate to the row of
the type that declares its effective operation. A source-declared node is an
admitted `object_type` node, and its dispatch candidates carry source body
clause functions (FR-569 "Dispatch tables"), so without source rows a
package with a dispatched source call could not satisfy the candidate join,
and a carrier that left source operations out would be partial, which
FR-576 forbids. The body is derived from the graph for a model operation (a
`function` whose owner is the operation member and whose clause is `body`);
a source clause function carries the unit's `SourceOwner` and its
`declaration` instead of a `ModelOwner`, so the same derivation reads its
`declaration`.

## Inputs

The checked source model's operations and their clause functions (FR-569).

## Outputs

The source rows of `operation_bodies`, in the carrier's
`(document, declaring, operation)` order.

## Behavior

- One row SHALL exist per operation a source object type declares; an
  operation only inherited has no row under the subtype.
- `body` SHALL be a `function` exactly when the package holds a `function`
  node with the declaring node's `SourceOwner` and `declaration`
  `[<type>, <operation>, "body"]`, and SHALL name that node.
- Rows SHALL be sorted with the model rows by `(document, declaring,
  operation)` UTF-8 bytes; a source row and a model row whose `document`
  strings are equal are ordered by `declaring`.
- The carrier is emitted whole or not at all (FR-576).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-997-AC-1 | Over FR-569's fixture S, `operation_bodies` holds six source rows, four of `Order` (`bump`, `count`, `scaled`, `size`) and two of `Sub` (`scaled`, `size`), each with `document` the `SourceOwner` `identity` of its node; `Order.size`'s `body` names `C`'s clause function `["Order", "size", "body"]`, `Sub.size`'s names `B`'s `["Sub", "size", "body"]`, and the other four are `no_body`. The dispatch candidates `Order.size` and `Sub.size` of `Sized` each join their own row. | Test (TC-2820) |
| FR-997-AC-2 | FSL's strict I2 read admits the FR-997-AC-1 package. Each refuses with FSpec FR-790's code, cause and locus: `Order.size`'s row with `body` `no_body` (`invalid_package`/`invalid-value` at `body`); `Order.bump`'s row with a `function` body; `Order.size`'s `body.node` naming `Qs`'s precondition function (`invalid_package`/`invalid-value` at `body/node`); `Sub.size`'s row removed (`invalid_package`/`invalid-value` at `/operation_bodies`); and `Order.size`'s `document` replaced by a string that is no `SourceOwner` identity of the package (`missing_declaration`/`missing-selection` at `document`). | Test (TC-2821) |

## Dependencies

- [FR-576](FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md) owns the carrier's
  emission; FR-569 "Operation bodies" states the source rows.
- FSpec FR-322 "Operation-body carrier", FR-322-AC-397 and FSpec FR-790.
