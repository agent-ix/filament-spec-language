---
id: FR-630
title: "Request a refinement item, its internals and its history and nested-reference rows through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-140
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-143
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-144
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-146
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-147
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-149
    type: depends_on
---
# FR-630: Request a refinement item, its internals and its history and nested-reference rows through analyze

## Description

A `native-analyze/1` item naming a refinement declaration SHALL request
FR-144's refinement item: the mapping is evaluated at every reachable
concrete state (FR-140), its safety half is decided on the explicit-state
product (FR-142) and its liveness half under the abstract fairness
(FR-143), with auxiliary and history variables (FR-138), a simulation
certificate (FR-149) and counterexample replay (FR-145). A wrong mapping
compiles, and only this item finds it.

### Decisions

- **Refinement internals are a request member.** The document carries
  `refinement_internals` at its top level for the program's own refinements
  and on each `libraries` entry for that library's, exactly as FR-027 reads
  them for compile; the reader hands them to compilation unchanged (FR-147).
  The same internals reach every replay the item runs.
- **History rows of every finite value type are checked and computed.** An
  enum, decimal, or option-of-enum or option-of-decimal history row is
  checked and computed as an integer row is; its value enters the history key
  by its canonical literal. A history row is auxiliary state the user writes
  to make a mapping expressible; restricting its type would make some
  correct refinements unstatable.
- **Nested-reference rows are lifted.** A field row whose type nests a
  `Reference<T_C>` inside an option, record, union or collection value lifts
  each nested reference to the `Reference<T_A>` of the object row that maps
  `T_A` from `T_C`, element by element at every depth, as a top-level
  reference is lifted (FR-135). A nested reference whose concrete type no
  object row maps refuses at S3 as a top-level one does.

## Inputs

### Request item

- `claim`: the refinement declaration's name; `kind` `refinement`;
  `item_kind` `refinement`.
- `temporal.subject`: the concrete subject; `temporal.abstract_subject`: the
  abstract subject's initial states and universes, defaulting to the
  abstract states the mapping gives the concrete initial states (FR-144).
- `temporal.limits` as FR-625, applied to the product.

## Outputs

- `record`: `proved` with the simulation certificate, `certified` once
  FR-149's core checker accepts it; `refuted` with the refinement failure
  part (`unmapped-step`, `mapping-undefined`, `abstract-step-disabled`,
  `liveness`) of FSpec FR-331's `RefinementFailure`.
- `counterexample`: the concrete path with the abstract image of each state,
  replayed by FR-145; the per-step simulation records of FR-146 travel in it.

## Behavior

- **Budgets and cancellation** are FR-625's over the product.
- **Refusals.** A refinement whose abstract side is a protocol runs FR-147's
  protocol arm; an abstract subject that does not admit settles `declined`
  `invalid-input`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-630-AC-1 | A register refined by a one-place queue with a correct mapping settles `proved` with certification `certified`; with a mapping that sends the empty register to a non-empty queue it settles `refuted` with `mapping-undefined` or `abstract-step-disabled` and a replayed counterexample carrying per-step records. | Test (TC-1590) |
| FR-630-AC-2 | A request carrying `refinement_internals` naming the program's refinement compiles with them and settles as FR-147 states; without the member it supplies the empty list. | Test (TC-1590) |
| FR-630-AC-3 | A history row typed `Decimal[0, 100; 0, 2; nearest-even]`, and one typed as an enum, check, and the refinement item over each settles `proved` for a correct mapping. | Test (TC-1590) |
| FR-630-AC-4 | A field row of type `Sequence<Reference<Ring>>[0, 2]` mapped to `Sequence<Reference<Queue>>[0, 2]` checks and its refinement item settles `proved`. | Test (TC-1590) |

## Dependencies

- FR-135 to FR-149 own refinement checking; FR-027 owns the
  `refinement_internals` reading.
