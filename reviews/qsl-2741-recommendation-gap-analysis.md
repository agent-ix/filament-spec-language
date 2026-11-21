---
id: SR-17952950000002
title: "QSL-2741 recommendation gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2741; PR3340 frozen434ae94acdf80c5f498d5bee56773f6ae217b6bb"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-206
    type: references
  - target: ix://agent-ix/filament-specification/FR-822
    type: references
---

## Summary

Repository-driven, planless gap audit retains a global assurance FAIL while finding no new unowned behavior or hollow evidence in the scoped recommendation repair. Plan completion: not assessed.

## Verdict

**FAIL** globally; scoped code delivery has no new blocking implementation finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The whole-repository static matrix retains 1123 untagged rows and 81 rows tagged only by ignored tests; the live FR-001-AC-7 is an unchanged untagged example. This existing assurance backlog prevents a repository gap-analysis PASS; the scoped recommendation repair does not close it. | spec/functional/FR-001-read-exact-source.md:273 |

## Coverage

Plan completion: not assessed

Reconciliation: quire 0.52.0 (engine 0.62.2) static matrix fallback — no run evidence read. quoin 0.36.7 at /Users/peter/.nvm/versions/node/v24.15.0/bin/quoin refused `quoin matrix --repo /Users/peter/dev/qsl-2741-recommendation-code --json` with exit 1: `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. The documented static matrix fallback succeeded; quire coverage also succeeded. This tool limitation was reported to the lead and the architecture lane explicitly accepted the documented fallback.

Raw static matrix rows: tagged 4868; untagged 1123; tagged-by-ignored-test 81; method-without-symbol 105; retired 13. Of the raw untagged rows, 199 contain textual RETIRED language; these were not silently relabeled as live or as engine-retired. The concrete live example FR-001-AC-7 remains untagged and is byte-identical at the source base. Coverage census: 13,648 evidence symbols, 12,557 bound, 45,011 production symbols and 5,826 untracked-symbol records; external FSpec references and existing collision/marker diagnostics are kept distinct from local new-tag defects. The complete global reverse audit remains incomplete; zero globally untraced behaviors is not claimed.

Changed behavior inventory: precondition-only applications; result-free advisory constraint; canonical recommendation operation/receiver/arguments; protocol instance/replicated actor; memory gate; replayed analysis prefix; state-preserving recommendation vs a different role's model write. These eight scoped behaviors map to FR-206, FR-218 and external FSpec FR-822/182. New unowned scoped behaviors: 0. New changed-path source stubs: 0. New changed-path hollow tests: 0. A global source/test scan was inspected; it cannot establish global absence of pre-existing stubs.

Actual current-head evidence: protocol_observe_only:: 4 passed / 0 failed / 0 ignored / 1558 filtered; protocol_recommendation:: 7 passed / 0 failed / 0 ignored / 182 filtered. Both completed exit 0; logs are /private/tmp/qsl-2741-main-eval-observe.log and /private/tmp/qsl-2741-main-command-recommendation.log. No Cargo invocation was made by this reviewer. Parent-authorized focused policy supersedes aggregate gates.

This is a partial delivery. Normal-driver post-change CLI qualification is pending; preserved DA01 output is not new canonical-field evidence. Receipt binding, run-wide boundary intake, protocol simulation and a public decoder for tampered recommendation transitions remain explicitly unmet; no full FR-822/FR-182 or QSL-2741 closure is claimed. The tests' external FSpec-prefixed tags name the external owner and are not treated as typos solely because the local matrix does not mint those IDs.

## Dispositions

Round 1, frozen head 434ae94acdf80c5f498d5bee56773f6ae217b6bb; no source change or Cargo run.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The dispatching lead explicitly accepts the unchanged repository assurance backlog for the scoped FR-822 recommendation repair only. The 1123 untagged / 81 ignored-only rows and incomplete global reverse audit remain open; FR-001-AC-7 is unchanged at the source base. Global gap FAIL remains. This disposition makes no driver, receipt, run-boundary, simulation, decoder or whole-QSL-2741 completion claim. |

Scoped mergeability: the recommendation repair is mergeable after this explicit baseline disposition; Code/Rust PASS and the 11 actual focused passes remain unchanged. Global gap FAIL remains.

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
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key."
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
