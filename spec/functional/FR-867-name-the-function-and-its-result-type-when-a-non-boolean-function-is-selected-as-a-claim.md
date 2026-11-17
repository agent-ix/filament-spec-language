---
id: FR-867
title: "Name the function and its result type when a non-Boolean function is selected as a claim"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-272
    type: references
---
# FR-867: Name the function and its result type when a non-Boolean function is selected as a claim

## Description

A clause-run request may select a function as a claim over a snapshot
(FR-312's `{"function": {"name", "arguments", "snapshot"}}`, FR-109's
`Function` selection). Only a function with a `Boolean` result is a claim;
FR-109 refuses any other at stage `select` with `ill_typed`/`type-mismatch`,
before any call. That check runs at `run`, because only the request knows
the selection: `fsl check` of the unit cannot know which function a later
request selects, so it never reports this refusal.

The refusal SHALL tell the reader which function was selected and what it
returns. Its FSpec FR-272 `type-mismatch` payload SHALL carry `function`,
the selected function's qualified name, `expected` `Boolean`, and `actual`,
the function's declared result type as the checker spells it; its locus is
the function's declared result type in the source; and its message names
the function, `Boolean` and the actual type, for example
"function `seven` returns `Integer`; a claim selection needs a function
whose result is `Boolean`".

A selection naming a function the package does not declare keeps FR-109's
`missing_declaration`/`missing-name` refusal at stage `select`, with the
name in its payload and message.

## Inputs

A clause-run request with a `function` selection.

## Outputs

The run's result document and FR-285's exit code; the refusal record named
above for a non-Boolean or unknown function.

## Behavior

- If a `Function` selection's function does not declare a `Boolean`
  result, then the clause-run entry SHALL refuse at stage `select`
  `ill_typed`/`type-mismatch` with payload `function`, `expected` `Boolean`
  and `actual` the declared result type, at the result type's locus, before
  any call.
- If a `Function` selection names no function of the package, then the
  entry SHALL refuse at stage `select` `missing_declaration`/`missing-name`
  naming the selected name.
- When a `Function` selection's function declares a `Boolean` result, the
  entry SHALL run it as a claim as FR-109 states.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-867-AC-1 | Through `command::run`, a clause request over FR-108's ConfigVersion unit extended with `function seven using v(): Integer pure { 7 }`, selecting `{"function": {"name": "seven", "arguments": [], "snapshot": "current"}}`, exits 20 with stage `select`, `ill_typed`/`type-mismatch`, payload `function` `seven`, `expected` `Boolean` and `actual` `Integer`, and a message naming `seven`, `Boolean` and `Integer`. | Test (TC-2459) |
| FR-867-AC-2 | The same request selecting `function yes using v(): Boolean pure { true }` exits 0 with `truth` true; selecting the name `nope` exits 20 with stage `select`, `missing_declaration`/`missing-name`, naming `nope`. | Test (TC-2459) |
| FR-867-AC-3 | The filament-examples suite holds a function-selection case selecting a Boolean function, expected exit 0, and one selecting an Integer function, expected exit 20 with `ill_typed`, and the examples gate runs both. | Inspection (TC-2459): the cases live in the filament-examples repository and run under the driver's examples gate (filament-driver FR-021). |

## Dependencies

- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the `Function`
  selection and its `select` refusals.
- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): the request's
  `function` selection.
- FSpec FR-272: the `type-mismatch` payload (expected and actual named type).
