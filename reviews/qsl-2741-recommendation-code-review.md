---
id: SR-17952950000001
title: "QSL-2741 recommendation code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2741; PR3340 frozen434ae94acdf80c5f498d5bee56773f6ae217b6bb"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: references
  - target: ix://agent-ix/filament-specification/FR-822
    type: references
---

## Summary

Scoped Code/Rust review of the advisory recommendation repair found no new code defect. Preconditions-only application enumeration, canonical records, result-free constraints and the source example agree with the supplied 11 passing focused tests.

## Verdict

**PASS** for the scoped repair; this is not a whole-feature qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Frozen head 434ae94acdf80c5f498d5bee56773f6ae217b6bb, source base 29ac6eba2e40f167d68b6ca155564813de0e3723. All 15 changed files were inspected, including seven production files, both changed test areas and the authored example. No CI, manifest or lock change exists. Existing operation enumeration and selected-precondition logic are reused; the recommendation sink returns before frame, candidate post-state or result computation. Canonical operation text is the already-owned application identity; the instance and optional actor follow the actual protocol transition context. The code retains distinct false/undecided behavior and finite argument-vector charging. No adapter, network or process facility is added.

Actual current-head evidence: protocol_observe_only:: 4 passed / 0 failed / 0 ignored / 1558 filtered; protocol_recommendation:: 7 passed / 0 failed / 0 ignored / 182 filtered. Both completed exit 0; logs are /private/tmp/qsl-2741-main-eval-observe.log and /private/tmp/qsl-2741-main-command-recommendation.log. No Cargo invocation was made by this reviewer. Parent-authorized focused policy supersedes aggregate gates.

This is a partial delivery. Normal-driver post-change CLI qualification is pending; preserved DA01 output is not new canonical-field evidence. Receipt binding, run-wide boundary intake, protocol simulation and a public decoder for tampered recommendation transitions remain explicitly unmet; no full FR-822/FR-182 or QSL-2741 closure is claimed. The tests' external FSpec-prefixed tags name the external owner and are not treated as typos solely because the local matrix does not mint those IDs.

No applicable AssuranceProfile was found. Rust API, implementation, panic/resource, wire and testing references were applied; no unsafe, async task or lifecycle change is introduced. Helper/fixture search found reuse rather than copied upstream logic: frames::apply owns the one argument/precondition loop; the example and command harness reuse this repository's existing counter fixture. No new source or test stub occurs in the changed paths.

## Examined units and bindings

```json
{
  "scope": [
    {
      "id": "FR-206-AC-13",
      "path": "spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md",
      "role": "examined",
      "excerpt": "Over the FR-218-AC-45 role clause from an `M::Actor` with `ready = false`, the reachable graph has a `recommend` step at `Advise` whose transition carries a `recommendation` naming `Advise`, `Auditor` and `M::Actor::op` with its receiver and empty arguments, and every successor of a step at `Advise` or at the `receive` to `Auditor` has the predecessor's `semantic` key member and compensation registrations; an `attempt` by another role in the same clause, over an operation writing `ready`, reaches a state with `ready = true`. A run of a clause holding an `attempt` with `boundary` `observe-only`"
    },
    {
      "id": "FR-218-AC-47",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "examined",
      "excerpt": "A `recommend` over an unknown operation refuses as an `attempt` over it does; a `recommend` whose `contracts` names a clause of another operation refuses as for an attempt; a `while` guard or `visible` entry reading the recommend binder `r` refuses `ill_typed`/`type-mismatch` at the read with `expected` no result."
    },
    {
      "id": "FSpec-FR-822-AC-3",
      "path": "filament-specification:spec/functional/protocol/FR-822-declare-observe-only-protocol-boundaries-and-typed-recommendations.md",
      "role": "examined",
      "excerpt": "Exploring the AC-1 clause from an order with `total = 5` gives `recommend` steps whose transition identity has `step` `recommend` and a `recommendation` record with `type` `protocol-recommendation`, the control's `node`, `role` `Auditor`, `operation` `Orders::Order::refund`, the receiver and the arguments, one step per application whose selected precondition holds; every successor of a `recommend` step has the predecessor's `semantic` key member and compensation registrations; a counterexample through it carries the record in its step, and a replay of that counterexample with the record's `arg"
    },
    {
      "id": "FSpec-FR-822-AC-5",
      "path": "filament-specification:spec/functional/protocol/FR-822-declare-observe-only-protocol-boundaries-and-typed-recommendations.md",
      "role": "context_only",
      "excerpt": "A run of the AC-1 clause takes no step behind `Auditor` or `Review`, and no `recommend` step, whose successor differs from its predecessor in the model state or the compensation registrations, while the same run's `attempt` outside the boundary changes the model field it writes. A run with `boundary` `observe-only` over a clause whose `run` holds an `attempt` refuses before the first step with the AC-2 refusal at the attempt; the same run over a clause with only `receive`, `check` and `recommend` controls runs and every step keeps the model state; the two modes have different request identitie"
    },
    {
      "id": "FSpec-FR-182-AC-6",
      "path": "filament-specification:spec/functional/workflow/FR-182-drive-authorized-effects.md",
      "role": "examined",
      "excerpt": "Exploring and simulating a protocol whose `attempt` applies an operation that writes a model field changes only the returned model states; the run issues no adapter request, starts no process and opens no connection, and the language implementation's evaluation and analysis components have no dependency on an adapter, process or network facility."
    },
    {
      "id": "FSpec-FR-182-AC-7",
      "path": "filament-specification:spec/functional/workflow/FR-182-drive-authorized-effects.md",
      "role": "context_only",
      "excerpt": "Simulating a protocol whose observe-only role `Auditor` holds a `recommend` of an operation that writes a model field, under the default run mode, returns behaviours whose `recommend` steps carry recommendation records and keep the model state, while an `attempt` by another role of the same operation changes the returned model states; the run issues no adapter request for either; under `boundary` `observe-only` the same protocol refuses at the `attempt` before any step."
    }
  ],
  "bindings": [
    {
      "test_id": "a_recommendation_carries_its_precondition_enabled_arguments",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_false_precondition_disables_the_recommendation",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_recommendation_reaches_the_analyze_counterexample",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_recommendation_ignores_selected_postconditions",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_recommendation_constraint_has_no_operation_result",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_replicated_auditor_recommendation_names_its_actor",
      "ac_id": "FSpec-FR-822-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "an_observed_attempt_refuses_at_the_command_checker",
      "ac_id": "FSpec-FR-822-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "an_observe_only_step_leaves_state_unchanged",
      "ac_id": "FR-206-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "another_role_can_write_shared_state",
      "ac_id": "FR-206-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "an_observed_attempt_refuses_before_the_first_step",
      "ac_id": "FR-206-AC-13",
      "trace": "correct"
    }
  ]
}
```

