---
id: FR-623
title: "Select state-space reductions in the analyze item over what the unit declares"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-150
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-151
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-153
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-159
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-160
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-382
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-385
    type: depends_on
---
# FR-623: Select state-space reductions in the analyze item over what the unit declares

## Description

A user SHALL select a symmetry reduction, partial-order reduction or a state
constraint for one model-check item through that item's `reductions` member
in a `native-analyze/1` document (FR-620). What a selection may use is
declared in the unit: a population annotated `symmetric` (FR-150) and a
`constraint` state clause (FSpec FR-382 "Constraint clause syntax").

### Decision: the unit declares, the item selects

A reduction is both a property of the model and a choice of method. The
unit states what is true of the model, such as which objects are
interchangeable and which states are of interest; the request chooses
whether a given check uses that. So one declared symmetry serves a reduced
and an unreduced check of the same claim, which are different obligations
(FR-159, FSpec FR-382 "Identity"), and no reduction is ever applied because
a unit happens to declare one. No reduction is selected by default.

A selection that the preservation table does not admit for the item's claim
form and fairness set settles that item `inconclusive` with
`ReductionNotPreserving{reduction, row}` before any expansion (FR-159,
FR-160), exit 22. This is a typed refusal of the one item: the item never
runs a different method than requested, and the request's other items still
run.

## Inputs

An analyze item's `reductions` member.

## Outputs

FSpec FR-382's admitted reduction selection for the engine, or a settled
item.

## Behavior

- **Spelling.** `reductions` has three optional members:
  - `symmetry`: either the string `"declared"`, meaning one class holding
    every key of each population the unit annotates `symmetric`, or FSpec
    FR-382's list of `{population, classes}` declarations, `population`
    written by its declared population name;
  - `partial_order`: `true` selects partial-order reduction with FSpec
    FR-382's `breadth-first-revisit` proviso; absent or `false` selects none;
  - `constraint`: the declared name of one `constraint` clause of the unit.
- **Admission.** FR-620's reader writes the FSpec FR-382 wire form: each
  population by its universe key, the classes sorted as FR-382 requires, and
  the constraint as the node key of its `state`/`state_clause` node (FR-158
  "Wire form").
- **Refusals.**
  - A selection the preservation table does not admit: `inconclusive`,
    `ReductionNotPreserving{reduction, row}`, wire cause
    `reduction-not-preserving` (FSpec FR-385).
  - A malformed symmetry declaration, or `"declared"` when the unit annotates
    no population `symmetric`: `declined`, cause `invalid-input`, diagnostic
    `invalid_runtime_input`/`invalid-value` (FR-281 "Engine refusals").
  - A `constraint` name that names no `constraint` clause:
    `invalid_runtime_input`/`invalid-value`, naming the name (FSpec FR-382).
  - A selection that no candidate advertises: negotiation settles the item
    `unsupported` with `unsupported-requested-capability` (FR-159 RV-6).
- **Result.** A proof under a reduction has proof basis
  `reduced{reductions}` with certification `uncertified` for a symmetry or
  partial-order reduction, and `exhaustive` with certification `uncertified`
  under a state constraint (FSpec FR-331, FR-385). The item's accounting
  records the constraint.

## Acceptance Criteria

The symmetric fixture: a population of three interchangeable counters
annotated `symmetric`, each `inc` up to 2, and the clause
`always holds(c.value <= 2)`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-623-AC-1 | The symmetric fixture's item settles the same verdict with no reduction, with `symmetry` `"declared"`, with `partial_order` `true`, and with both, for a holding and for a refuted clause. | Test (TC-1583) |
| FR-623-AC-2 | `symmetry` `"declared"` and the explicit declaration of one class of all three keys give equal admitted requests. | Test (TC-1583) |
| FR-623-AC-3 | `partial_order` `true` on a bounded MLTL clause settles `inconclusive` with `reduction-not-preserving` naming `partial-order`, exit 22, and a second item in the same request settles as it does alone. | Test (TC-1583) |
| FR-623-AC-4 | `constraint` naming no constraint clause settles `invalid_runtime_input`/`invalid-value` naming it; `symmetry` `"declared"` over a unit with no `symmetric` population settles `declined` `invalid-input`. | Test (TC-1583) |
| FR-623-AC-5 | The proved item under `symmetry` has proof basis `reduced` listing `symmetry` and certification `uncertified`. | Test (TC-1583) |

## Dependencies

- FR-159 and FR-160 own the preservation table and reduced settlement;
  FSpec FR-382 owns the request member and its identity rule.
