---
id: FR-998
title: "Lower a population only to the catalogued collection-only operation"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-012
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: implements
  - target: ix://agent-ix/filament-specification/FR-376
    type: implements
---
# FR-998: Lower a population only to the catalogued collection-only operation

## Description

When S4 lowers a checked `only(x in P where c: e)` (the `Query{visit: Only}`
node FR-391 checks), it SHALL emit one FSpec FR-322 `application` of
`filament.op.collection.only`, operator `collection`, member
`type_argument` the type node of `e`, `result_type` that same node, and
arguments `[ref(F), binding{name: x, value: ref(e)}]`, `F` the population
value of `P` (FR-391's `all_instances` application) when `where` is omitted
and otherwise the `filament.op.collection.filter` application over it with
binder body `c`. Lowering SHALL NOT refuse the node in any position where it
checks.

## Rationale

FR-391 checks `only` in clause bodies and refinement rows and says it lowers
to a collection row, and FSpec FR-322 "Operation catalog" of the population
forms defines `filament.op.collection.only` with this shape. FR-093's table
had no row for it, so lowering refused a checked form with no S4
representation, which FR-391 forbids. The form lowers through `filter` and
not through `map`, because `map` over a set would collapse two objects with
equal `e` into one and turn an undefined `only` (two matching objects) into
a defined one (FSpec FR-376 `only-not-one`).

## Inputs

A checked `Query{visit: Only}` node over a population domain.

## Outputs

The `filament.op.collection.only` application node and, when `where` is
written, its `filament.op.collection.filter` operand.

## Behavior

- The node's identity SHALL be `filament.op.collection.only` and its member
  and `result_type` the type node of `e`.
- The filter SHALL take the binder `x` and body `c`; the `only` application
  SHALL take a binder of the same name and body `e`.
- Evaluation of the lowered node over a collection of exactly one element
  SHALL give `e` for that element, and over any other size SHALL be
  undefined with reason `only-not-one` (FSpec FR-376).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-998-AC-1 | Over FR-391-AC-10's `Counter` unit with `bound B on M::counters [3]`, `invariant O using v on M::Counter at current { only(o in M::counters where o.v = 2: o.v) = 2 }` checks and lowers to one `filament.op.collection.only` application whose first argument is a `filament.op.collection.filter` over the `filament.op.model.all_instances` application of `M::counters` and whose member and `result_type` name the type node of `o.v`; without `where`, the first argument is the `all_instances` application itself. Neither lowering refuses `unsupported_construct`. | Test (TC-2822) |
| FR-998-AC-2 | Evaluating the AC-1 invariant over a snapshot with exactly one counter at `v = 2` holds; over snapshots with no such counter or with two, the invariant is undefined with reason `only-not-one`, the same reason for both. | Test (TC-2823) |

## Dependencies

- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) owns
  the application table; [FR-391](FR-391-name-a-population-in-a-state-clause.md)
  checks the form.
- FSpec FR-322 "Operation catalog" of the population forms and FR-322-AC-455;
  FSpec FR-376 `only-not-one`.
