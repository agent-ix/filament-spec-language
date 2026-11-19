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
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-824
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
plan document per plan; one plan result per execution, selected by the run
request's `verification_plan` member and written as the
`filament.verification-plan-outcome/v1` document (FR-1038, FSpec FR-824).

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

### Decision: the coordinator is `fsl-analyze`'s `plan` module

The coordinator runs engines, and FSL's engines are in `fsl-analyze`
(ADR-011 §6.1 layer A, FR-281) or below it (`fsl-eval`'s simulator and
model-based tester, `fsl-replay`'s replay facade, the hybrid provider seam).
A coordinator in a lower crate could not reach them, and one in
`fsl-command` would put engine dispatch in the request adapter. So the
coordinator SHALL be the module `plan` of `fsl-analyze`, after `analyze`
and before `monitor`, with one public entry:

```rust
pub fn execute_plan(
    package: &fsl_package::CheckedPackage,
    plan: &fsl_package::verification_plan::VerificationPlan,
    inputs: &PlanInputs<'_>,          // initial snapshot per consuming step, seed
    engines: &Engines,
    negotiate: &dyn Negotiate,
    limits: &AnalyzeLimits,
    cancel: &filament_exact::Cancel,
) -> Result<Staged<PlanOutcome>, StageFailure<PlanRefusal>>
```

- `plan` is the admitted `filament.verification-plan/v1` document. Its
  emitter and strict reader are in `fsl-package` (layer 4), beside S4, and
  the reader refuses a document whose `package` is not `package`'s identity
  (FSpec FR-192 "Package form").
- `negotiate` is FR-281's `Negotiate`: the coordinator builds each step's
  FR-290 item, computes its candidates by FR-075 and asks the caller to
  settle it, exactly as `analyze` does, and never chooses an engine itself.
- `PlanOutcome` holds one `PlanDisposition` per node in source order, with
  the closed status of FSpec FR-824 "Outcome document", the method, the
  evidence kind, `blocked_by` and the step's terminal record. `fsl-command`
  writes it as the `filament.verification-plan-outcome/v1` document
  (FR-1038); the coordinator renders nothing.
- `fsl-command` serves the request member (FR-1038) and calls
  `execute_plan`, as it calls `fsl_analyze::analyze` for `native-analyze/1`
  (FR-621). That edge is the one ADR-011 §6.1 records for `fsl-command`.
- `PlanRefusal` is a whole-plan refusal before any step runs: the plan
  document's reader refusal, the package-identity mismatch, or an `initial`
  that no consuming step's model admits.

### Dispatch per method

Each step runs on the route its method names in FSpec FR-192's method table,
and on no other:

| Method | Route | Owning requirement |
| --- | --- | --- |
| `model_check` | a negotiated `temporal-satisfaction` item through `analyze` (EN-1) | FR-281, FR-625 |
| `contract_check` | a negotiated `operation-contract` item through `analyze` | FR-281 |
| `value_check` | a negotiated `value-validity` item through `analyze` | FR-281 |
| `probabilistic_exact` | a negotiated `probabilistic-satisfaction` item with evidence `exact` (EN-5) | FR-196, FR-201 |
| `probabilistic_statistical` | a negotiated `probabilistic-satisfaction` item with evidence `statistical` (EN-4), seeded by the request's `seed` | FR-189, FR-191, FR-192 |
| `hybrid_solver` | a negotiated `temporal-satisfaction` item carrying the hybrid-dynamics requirement, run by the negotiated hybrid provider | FR-750, FR-1019 |
| `schedulability` | a negotiated `schedulability` item through `analyze` | FR-246, FR-633 |
| `model_based_test` | `fsl-eval`'s model-based test entry, with the step's clause, domain, bound as `max_cases` and the request's `seed`; its FR-190 record is the step's result | FR-752 |
| `simulate` | a `simulation` item through `analyze`, not negotiated | FR-634, FR-120 |
| `replay` | a negotiated `finite-replay` item, run through the layer-6 replay facade | FR-098 |

A step whose item settles anything other than `supported` runs no engine and
settles as FSpec FR-824 "Outcome document" states; with no registered engine
for its kind it settles `unsupported` with
`unsupported_projection`/`unsupported-requested-capability`, as FR-281-AC-8
settles the same item outside a plan.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-753-AC-1 | S2 builds FSpec FR-192's `Release` plan into one `VerificationPlan` form with three steps in source order, their names, methods, bounds `20`, `500` and none, and `timely`'s `depends [safe]`, each with its span. | Test (TC-2103) |
| FR-753-AC-2 | S3 checks `Release` and refuses each of FSpec FR-192-AC-9's and AC-10's mutations with the cause and span stated, including the `a`/`b` cycle as `invalid_package`/`definition-cycle`. | Test (TC-2103) |
| FR-753-AC-3 | The unit's checked package identity is equal with and without `Release`; S4 emits its plan document with three nodes and one `requires` edge. | Test (TC-2103) |
| FR-753-AC-4 | Executing `Release` where `safe` settles `refuted` still runs the independent step `tested`, blocks `timely` with the disposition `blocked` naming `safe`, and records one disposition per node. | Test (TC-2103) |
| FR-753-AC-5 | `fsl_analyze::plan::execute_plan` over `Release` with engines holding no `temporal-satisfaction` engine returns `safe` `unsupported` with `unsupported_projection`/`unsupported-requested-capability`, `tested` `tested` from `fsl-eval`'s model-based test entry with `max_cases` 500 and the given seed, and `timely` `blocked` by `safe`; a recording `Negotiate` is asked for `safe` only, never for the blocked `timely` and never for `tested`, whose method is not negotiated; a plan document whose `package` is another identity refuses before any step, naming both identities. | Test (TC-3028) |

## Dependencies

- FR-067 (S2 dispatch); FR-752, FR-754 (language methods); FSpec FR-192.
