---
id: FR-753
title: "Build, check, emit and execute a verify plan"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-067
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-192
    type: depends_on
---
# FR-753: Build, check, emit and execute a verify plan

## Description

FSL SHALL carry a `verify` declaration through every stage: S2 builds a
`VerificationPlan` form, S3 resolves and checks it by FSpec FR-192 "Source
form", the compiler emits it as a linked `filament.verification-plan/v1`
document outside the checked package, and the plan coordinator executes it
by FSpec FR-192 "Plan execution graph".

## Inputs

A unit holding one or more `verify` declarations; the checked package; the
registered engines and the request's limits.

## Outputs

`DeclarationForm::VerificationPlan` at S2; a checked plan at S3; one emitted
plan document per plan; one plan result per execution.

## Behavior

- S2 SHALL build `DeclarationForm::VerificationPlan` holding the plan name,
  profile alias and, per `check` step in source order, the step name and its
  `claim`, `method` and `domain` names, its `depends` names and its `bound`,
  each with its span. S2 SHALL NOT refuse a `verify` declaration that S1
  admitted.
- S3 SHALL apply FSpec FR-192 "Source form": resolve each name, map each
  method to its FR-290 kind, extent and evidence kind, and refuse with the
  causes it states, each at the span it names.
- The checked package identity SHALL NOT depend on any `verify` declaration;
  S4 SHALL emit each plan as its own `filament.verification-plan/v1`
  document bound to the package identity.
- The coordinator SHALL dispatch each step's item to its engine (or to
  FR-752, FR-754 or FSpec FR-181 simulation for the language methods) in
  `requires` order, record one disposition per node, block only descendants
  of a failed node, and never substitute one evidence kind for another.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-753-AC-1 | S2 builds FSpec FR-192's `Release` plan into one `VerificationPlan` form with three steps in source order, their names, methods, bounds `20`, `500` and none, and `timely`'s `depends [safe]`, each with its span. | Test (TC-2103) |
| FR-753-AC-2 | S3 checks `Release` and refuses each of FSpec FR-192-AC-9's and AC-10's mutations with the cause and span stated, including the `a`/`b` cycle as `invalid_package`/`definition-cycle`. | Test (TC-2103) |
| FR-753-AC-3 | The unit's checked package identity is equal with and without `Release`; S4 emits its plan document with three nodes and one `requires` edge. | Test (TC-2103) |
| FR-753-AC-4 | Executing `Release` where `safe` settles `refuted` still runs the independent step `tested`, blocks `timely` with the disposition `blocked` naming `safe`, and records one disposition per node. | Test (TC-2103) |

## Dependencies

- FR-067 (S2 dispatch); FR-752, FR-754 (language methods); FSpec FR-192.
