---
id: FR-1049
title: "Classify alpha tutorial probability-analysis outcomes"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---
# FR-1049: Classify alpha tutorial probability-analysis outcomes

## Description

When `fsl analyze` or `fsl compile` runs a documented alpha tutorial probability
example, the command SHALL return the documented settled category and exit
status, including the distinction between an unsupported form and an
refuted claim.

This requirement covers the explicit unsupported and refuted examples and
variations in the alpha probability tutorial, including cases whose candidate
result conflicts with the tutorial's documented expected outcome.

## Inputs

- The literal FSL source and native request files identified in the acceptance
  criteria, run by `fsl analyze` or `fsl compile`.

## Outputs

- The command exit status and `filament-outcome/1` document produced by FSL.

## Behavior

- For each example, `fsl` SHALL emit the specified outcome category,
  exit status, result shape, and diagnostic code/cause specified below.
- For an unsupported example, `fsl` SHALL emit an `unsupported` outcome and
  any diagnostic code and cause named by the tutorial or its expected result.
- For a refuted example, `fsl` SHALL emit a `refuted` outcome with exit status
  10.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1049-AC-1 | Given the tutorial's bounded-profile source form `probabilistic StaysUp using ev on Arch over (s: Arch::Service) under every scheduler on origin { probability >= 0.9 [ always holds(s.up) ] }` from `examples/architecture/rules/probability/claims/claims-unbounded-event.fsl`, compiled by the `native-compile/1` request in `examples/architecture/requests/probability/claims/claims-unbounded-event.json`, the command exits 21 and emits an outcome with `category: "unsupported"`, `last_stage: "S3"`, `result: null`, a first diagnostic with `code: "unsupported_construct"` and `cause: "expression-form"`, and `items: []`. | Test |
| FR-1049-AC-2 | Given `examples/architecture/rules/probability/claims/claims-unbounded-event.fsl` changed to profile `inf` and the unwindowed claim `probability >= 0.9 [ holds(s.up) until holds(not s.up) ]`, plus `examples/architecture/requests/probability/claims/claims-unbounded-event.json` changed to `native-analyze/1` with an exact-evidence item for `StaysUp` and `examples/architecture/data/start-service-up.json`, the claim that holds exits 21 as `unsupported`. | Test |
| FR-1049-AC-3 | Given the same source and request paths as AC-2, with the request document set to `examples/architecture/data/start-service-down.json`, the failing unwindowed until claim exits 10 as `refuted` at S8. | Test |
| FR-1049-AC-4 | Given `examples/architecture/rules/probability/claims/claims-every-scheduler-without-random.fsl` changed to profile `inf` and the unwindowed claim `probability >= 0.99 [ eventually holds(s.up) ]`, plus `examples/architecture/requests/probability/claims/claims-every-scheduler-without-random.json` with an exact-evidence item for `NoRandomUp` and document `examples/architecture/data/start-service-down.json`, the command exits 21 as `unsupported`. | Test |
| FR-1049-AC-5 | Given the same source and request paths as AC-4 with the request document set to `examples/architecture/data/start-service-up.json`, the unwindowed every-scheduler/no-random eventually claim exits 21 as `unsupported`. | Test |
| FR-1049-AC-6 | Given `examples/architecture/rules/probability/claims/claims-unbounded-event.fsl` with `StaysUp` changed to use profile `inf` and `probability <= 0.5 [ always holds(s.up) ]`, plus `examples/architecture/requests/probability/claims/claims-unbounded-event.json` changed to `native-analyze/1` with an exact-evidence item for `StaysUp` and document `examples/architecture/data/start-service-up.json`, the unwindowed always claim exits 21 as `unsupported` when satisfied. | Test |
| FR-1049-AC-7 | Given the same source and request paths as AC-6 with the request document set to `examples/architecture/data/start-service-down.json`, the unwindowed always claim exits 21 as `unsupported` when satisfied. | Test |
| FR-1049-AC-8 | Given the same profile and analyze request path as AC-6, with claim `probability >= 0.9 [ always holds(s.up) ]` and request document `examples/architecture/data/start-service-up.json`, the failing unwindowed always claim exits 10 as `refuted` at S8. | Test |
| FR-1049-AC-9 | Given the same input and request as AC-8 with the request document set to `examples/architecture/data/start-service-down.json`, the failing unwindowed always claim exits 10 as `refuted` at S8. | Test |
| FR-1049-AC-10 | Given `examples/engines/rules/coin.fsl` and `examples/engines/requests/probability-atom-now.json`, `HeadsNow` with `probability >= 0.5 [ holds(c.heads) ]` and `examples/engines/data/coin-tails.json`, the unwindowed atom exits 21 as `unsupported`. | Test |
| FR-1049-AC-11 | Given `examples/engines/rules/coin.fsl` and `examples/engines/requests/probability-atom-now-at-most.json`, `HeadsNowAtMost` with `probability <= 0.5 [ holds(c.heads) ]` and `examples/engines/data/coin-tails.json`, the unwindowed atom exits 21 as `unsupported`. | Test |
| FR-1049-AC-12 | Given `examples/rover/rules/staged-vote-probability-missed.fsl`, `examples/rover/requests/staged-vote-probability-missed.json`, and the request's `examples/rover/data/start-driving.json` document, `fsl analyze` exits 21 with an unsupported item on backend `en-5` and cause kind `not-markov`, `post_state_count: 8004`, and a `post_states` array containing 8004 states. | Test |
| FR-1049-AC-13 | Given `examples/architecture/rules/probability/schedulers/schedulers-fair-weak.fsl` and `examples/architecture/requests/probability/schedulers/schedulers-fair-weak.json`, the `FairlyUp` claim with `fair weak Arch::Service::call` under every scheduler exits 21 with category and item result `unsupported`. | Test |

The alpha journey gate's recorded projection contains `category`,
`last_stage`, `result`, `first_diagnostic` code/cause, and `items`, and omits
`diagnostics[].message`. This is verification context only; the requirement
specifies CLI outcomes and adds no gate behavior. Where the tutorial does not
name a cause, the criteria do not add one.

## References

- [FR-286](./FR-286-serialize-every-outcome-as-one-json-outcome-document.md) defines the `filament-outcome/1` document that carries the command result.

## Dependencies

- The alpha tutorial probability page and the probability example sources and requests named in AC-1 through AC-13 define the literal inputs and documented outcomes.
- [FR-286](./FR-286-serialize-every-outcome-as-one-json-outcome-document.md) defines the command outcome document fields.
