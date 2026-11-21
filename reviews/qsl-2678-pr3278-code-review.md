---
id: SR-202611210022442313144108502
title: "QSL-2678 PR 3278 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3278 semantic Rust changes and authorized formatting pass; QSL-2678"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Ticket: QSL-2678 (Done, existing has-review label retained). PR: filament-spec-language#3278, public. Base `01572e0da58c6e3ef0e4e12768dc38d45da93208`; semantic/lint head `9802f800a34e220aa5490b7a0f5f6f6f1f2a7394`; frozen head `195707cf790ee26e50285ccff81e8723ae1bf320`. GitHub reports MERGEABLE. Reviewed semantic diff base through semantic head, then final user-required cargo fmt --all commit independently, including git diff -w. No source/branch mutation, Cargo invocation, test run, push, relay, issue creation or status change by reviewer.

Installed tools on terra: /home/peter/.npm-global/bin/quoin 0.36.8; /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2); /home/peter/.cargo/bin/rustc 1.98.1 (48a229cea 2026-09-01). spec-artifacts-process@v0.36.0. These installed releases are newer than the supplied release notice; no old binary was selected. Supplied Clippy reports 0.1.98 (48a229ceae 2026-09-01). quire schema loaded successfully; loading only the process module emitted missing cross-module role/edge warnings, so artifact validation uses ambient complete module discovery.

Verification evidence: reviewed all eleven single-crate Clippy logs named in qsl-2678-clippy-checks.md, each ending in Finished dev profile; exit-0 results are supplied by the coder. Foundation/source retain inherited baseline evidence and no substantive change. The formatting log is quiet apart from locked-build diagnostics, with exit 0 recorded in the coder handoff. git diff --check base..head passed. Runtime manifest contains nine focused single-crate commands, 23 expected tests; no designated-runner results were available at review time. Do not count the expected 23 as passes. No workspace, all-targets, multi-package, whole-crate, benchmark, optional-feature or no-std validation was run or claimed.

## Verdict

**PASS** — no code defects found in the assigned scope. Merge still requires designated-runner results for the nine focused commands.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Box stores the same owned values: reference mismatch construction and extraction preserve ValueType, location, code/cause, derived equality/hash and Debug-backed Display; no Serde contract exists for CheckCause/ValueType. Task/timed payload constructors and pattern accesses retain the original identities/trace. BoundedMeasure predicates, timed constants and SeqStage binder annotations are boxed at construction and borrowed/dereferenced without changing their logical values. ProgramBatchSelection still compiles once and executes each selection with its original inputs and limits. ReplayOutput projects the same all-or-nothing OutcomeDocument; synthesis Refuse boxes and unboxes the same RunCause. CauseWire's serde Box is transparent; Extraction's renamed _document field retains serde(rename = "document"). No reader/writer schema, catalog pair, tag, numeric conversion, limit or ordering contract was changed.

Every added lint allow is local with a reason: independent pipeline/budget arguments, exact rationals whose representation caches cannot mutate ordering, stable direct EngineReport return APIs, or the shared TC-1070 fixture module. No workspace/crate-level blanket suppression or gate reduction appears. Removed helpers/fields are unused; test modules/functions were relocated without deleting tests. Collection/control-flow substitutions preserve assertions and side effects. The new refusal regression asserts exact Display formatting, catalog pair, actual payload, equality and Box/value hash transparency with real production types; it does not mock the unit under review.

Public Rust constructor signatures now require Box values for the exposed variants/fields. This intentional prerelease API change is explicitly disclosed in the PR, and the default-feature library/test Clippy checks cover assigned in-repository constructor sites, including analyze task-product patterns that work by deref. Optional features, benches/examples, external downstream Rust callers and no-std remain outside this evidence. No promise of Rust source compatibility is made.

The final formatting pass spans 246 Rust files, including 191 outside the lint edits. Whitespace-insensitive review shows rustfmt line wrapping, import/module ordering, trailing commas and equivalent match-arm/closure braces; the breadth is explicitly owner-authorized. It does not justify reverting or rejecting the pass. Additional token inventory is recorded separately as a review aid, not as a proof of semantic equivalence.


## Examined semantic files

- `fsl-analyze/src/analyze/document/package_run.rs`
- `fsl-analyze/src/analyze/exact.rs`
- `fsl-analyze/src/analyze/hybrid_item.rs`
- `fsl-analyze/src/analyze/hybrid_process.rs`
- `fsl-analyze/src/analyze/request.rs`
- `fsl-analyze/src/analyze/state_graph_item.rs`
- `fsl-analyze/src/analyze/task_product_item.rs`
- `fsl-analyze/src/exact_probabilistic/finite_horizon.rs`
- `fsl-analyze/src/exact_probabilistic/plan.rs`
- `fsl-analyze/src/model_check/state_graph/explore.rs`
- `fsl-analyze/src/plan.rs`
- `fsl-analyze/src/protocol_analysis/conformance.rs`
- `fsl-analyze/src/provider.rs`
- `fsl-analyze/src/statistical/method.rs`
- `fsl-analyze/src/zone_check/live_certificate.rs`
- `fsl-analyze/src/zone_check/task_product.rs`
- `fsl-analyze/tests/it/analyze_package_selection.rs`
- `fsl-analyze/tests/it/analyze_timed_cli.rs`
- `fsl-analyze/tests/it/hybrid_requests.rs`
- `fsl-analyze/tests/it/hyper_closed.rs`
- `fsl-analyze/tests/it/hyper_reductions.rs`
- `fsl-analyze/tests/it/hyper_witness.rs`
- `fsl-analyze/tests/it/prob_world.rs`
- `fsl-analyze/tests/it/relation_claims.rs`
- `fsl-analyze/tests/it/task_automaton_crosscheck.rs`
- `fsl-analyze/tests/it/task_product_certificate.rs`
- `fsl-analyze/tests/it/tied_item_resolution.rs`
- `fsl-command/src/command.rs`
- `fsl-command/src/command/export/lower/temporal.rs`
- `fsl-command/src/command/items.rs`
- `fsl-command/src/command/replay.rs`
- `fsl-command/src/command/synthesis.rs`
- `fsl-command/src/command/wire.rs`
- `fsl-command/src/lib.rs`
- `fsl-command/tests/it/check_imports.rs`
- `fsl-eval/src/simulation/model.rs`
- `fsl-eval/src/simulation/model/encode.rs`
- `fsl-eval/src/simulation/model/frames/narrowing_tests.rs`
- `fsl-eval/src/simulation/model/input.rs`
- `fsl-eval/src/simulation/model/model_based_test.rs`
- `fsl-eval/src/simulation/model/reference_bindings.rs`
- `fsl-eval/src/simulation/model/task_product.rs`
- `fsl-eval/src/simulation/model/timed.rs`
- `fsl-eval/src/simulation/protocol/steps.rs`
- `fsl-eval/src/simulation/subject.rs`
- `fsl-eval/src/value/expression/bare.rs`
- `fsl-eval/src/value/expression/evaluate.rs`
- `fsl-eval/src/value/expression/evaluate/text_conversion.rs`
- `fsl-eval/src/value/expression/family.rs`
- `fsl-eval/tests/it/machine_contexts.rs`
- `fsl-eval/tests/it/protocol_observe_only.rs`
- `fsl-eval/tests/it/protocol_population.rs`
- `fsl-eval/tests/it/protocol_retry_bound.rs`
- `fsl-eval/tests/it/protocol_slots/acceptance.rs`
- `fsl-eval/tests/it/reference_adapter.rs`
- `fsl-forms/tests/it/synthesis_verify_forms.rs`
- `fsl-package/src/emit.rs`
- `fsl-package/src/emit/protocol_locations.rs`
- `fsl-package/src/emit/tests/admission_corpus.rs`
- `fsl-package/src/emit/tests/golden.rs`
- `fsl-package/src/emit/tests/probabilistic.rs`
- `fsl-package/src/emit/tests/protocol_reference.rs`
- `fsl-package/src/reader_protocol_projection.rs`
- `fsl-replay/src/certificate/task_product/wire.rs`
- `fsl-replay/src/certificate/zone_certificate.rs`
- `fsl-replay/src/certificate/zone_certificate/product.rs`
- `fsl-replay/src/hybrid_replay/certificate/flowpipe.rs`
- `fsl-replay/src/hybrid_replay/tests/certificate_tests.rs`
- `fsl-replay/src/model_check/graph_replay.rs`
- `fsl-replay/src/model_check/replay.rs`
- `fsl-replay/src/outcome/counterexample.rs`
- `fsl-replay/src/probability_certificate/document/timed.rs`
- `fsl-replay/src/result/wire.rs`
- `fsl-replay/src/spine/clause.rs`
- `fsl-replay/src/spine/clause/tests.rs`
- `fsl-replay/src/spine/clause/tests/state_clause_replay.rs`
- `fsl-replay/src/spine/clause/tests/witness.rs`
- `fsl-replay/src/spine/clocks.rs`
- `fsl-replay/src/spine/lifecycle/tests.rs`
- `fsl-replay/src/witness/derivation.rs`
- `fsl-replay/tests/it/checked_body_failure.rs`
- `fsl-replay/tests/it/intake_refusal_outcome.rs`
- `fsl-semantics/src/check/check/typing.rs`
- `fsl-semantics/src/check/footprint.rs`
- `fsl-semantics/src/check/lowering.rs`
- `fsl-semantics/src/check/mod.rs`
- `fsl-semantics/src/check/probabilistic/claim.rs`
- `fsl-semantics/src/check/protocol_clause/tests/observe_only.rs`
- `fsl-semantics/src/check/protocol_clause/tests/source_contracts.rs`
- `fsl-semantics/src/check/refusal.rs`
- `fsl-semantics/src/check/timed_model/reading.rs`
- `fsl-semantics/src/model/intake.rs`
- `fsl-semantics/src/model/observation/document.rs`
- `fsl-semantics/tests/it/model_unit_input.rs`
- `tests/it/export_temporal_lowering.rs`
- `tests/it/forms_package_contracts.rs`
- `tests/it/request_surface_s1.rs`
- `tests/it/run_wire.rs`
- `tests/it/spine_retained_criteria.rs`
- `xtask/src/consumer_facades.rs`
- `xtask/tests/qualification_adapters.rs`
- `xtask/tests/qualify_parts.rs`

## Scope records

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "CheckCause::ReferenceLiteralTypeMismatch",
      "path": "fsl-semantics/src/check/refusal.rs",
      "role": "examined",
      "excerpt": "actual: Box<ValueType>,"
    },
    {
      "id": "TaskProductTransition::Task",
      "path": "fsl-eval/src/simulation/model/task_product.rs",
      "role": "examined",
      "excerpt": "Task(Box<TaskTransitionIdentity>),"
    },
    {
      "id": "BehaviourReading::Timed",
      "path": "fsl-eval/src/simulation/model/timed.rs",
      "role": "examined",
      "excerpt": "Timed(Box<crate::temporal::TimedTrace>),"
    },
    {
      "id": "BoundedMeasure",
      "path": "fsl-semantics/src/check/probabilistic/claim.rs",
      "role": "examined",
      "excerpt": "from: Box<CheckedPredicate>,"
    },
    {
      "id": "SeqStage",
      "path": "fsl-semantics/src/check/check/typing.rs",
      "role": "examined",
      "excerpt": "binder_type: Box<ValueType>,"
    },
    {
      "id": "Argument::Value",
      "path": "fsl-semantics/src/check/timed_model/reading.rs",
      "role": "examined",
      "excerpt": "constant: Box<Result<Rational, TemporalCause>>,"
    },
    {
      "id": "ProgramBatchSelection",
      "path": "fsl-replay/src/spine/clause.rs",
      "role": "examined",
      "excerpt": "ValueInvariants(Box<ValueInvariantsBatchRequest>),"
    },
    {
      "id": "RunCause::ReplayOutput",
      "path": "fsl-command/src/command.rs",
      "role": "examined",
      "excerpt": "outcome: Box<fsl_replay::OutcomeDocument>,"
    },
    {
      "id": "Halt::Refuse",
      "path": "fsl-command/src/command/synthesis.rs",
      "role": "examined",
      "excerpt": "Refuse(Box<super::RunCause>),"
    },
    {
      "id": "CauseWire::HybridTrace",
      "path": "fsl-replay/src/result/wire.rs",
      "role": "examined",
      "excerpt": "disagreement: Box<HybridDisagreementWire>,"
    },
    {
      "id": "ConformanceReplayRefusal",
      "path": "fsl-analyze/src/protocol_analysis/conformance.rs",
      "role": "examined",
      "excerpt": "bound: Option<Box<fsl_replay::BoundExceeded>>,"
    },
    {
      "id": "DeadlockEvent",
      "path": "fsl-analyze/src/model_check/state_graph/explore.rs",
      "role": "examined",
      "excerpt": "Violated(Box<CounterexampleKind>, usize),"
    },
    {
      "id": "reference_literal_refusal_preserves_display_equality_and_payload",
      "path": "fsl-semantics/src/check/refusal.rs",
      "role": "examined",
      "excerpt": "fn reference_literal_refusal_preserves_display_equality_and_payload() {"
    },
    {
      "id": "FR-652-AC-3",
      "path": "spec/functional/FR-652-select-state-machine-contexts-and-value-invariants-from-a-run-request.md",
      "role": "examined",
      "excerpt": "Through `run`, over FR-106-AC-48's TC-1051 K2 fixture, an `items` request with the selection `{\"value_invariants\": {\"snapshot\": \"s1\", \"invocation\": \"i1\"}}` returns one item per `evaluate_value_invariants` result in FR-366's order, each with `invariant` and `path`; the item for `Money`'s non-negative invariant at `i1`'s parameter `amount` (`amount_minor` `-7`) reports `violation`, and the document's `category` is `violation`, exit 10. The same selection with a `pre` snapshot as `snapshot` gives one item refused `wrong_snapshot`/`wrong-observation` at stage `admit`, with no `invariant` or `path`"
    },
    {
      "id": "FR-862-AC-3",
      "path": "spec/functional/FR-862-read-a-replay-request-file-and-replay-it.md",
      "role": "examined",
      "excerpt": "A replay whose results row at position 0 fails to encode writes an outcome document with `category` `internal-failure`, exit 30, `last_stage` `null` and one `output-failure` diagnostic naming position 0, and writes no `results` document."
    },
    {
      "id": "FR-1000-AC-2",
      "path": "spec/functional/FR-1000-keep-a-reference-literal-pending-until-subject-binding.md",
      "role": "examined",
      "excerpt": "Over TC-1076's `Counter` unit, `invariant R using v on M::Counter at current { self = M::Counter#\"a\" }` checks with the literal typed `Reference<M::Counter>`; `M::Nothing#\"a\"` refuses `missing_declaration`/`missing-name` at `Nothing`, `M::Level#\"a\"` refuses `ill_typed`/`type-mismatch` at `M::Level` with expected `object type`, and `M::Counter#\"\"` refuses `invalid_package`/`invalid-value` at the string. A protocol `choice` guard comparing a `receive` record's reference field `got.target` (record `Pick { target: Reference<M::Counter>; }`, received before the choice) to `M::Counter#\"z\"` checks at"
    },
    {
      "id": "FR-231-AC-2",
      "path": "spec/functional/FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md",
      "role": "examined",
      "excerpt": "Over the `Rpc` unit with `urgent when self.phase = Waiting` added, no positive delay is admissible after `send`. A behaviour of `Rpc` that ends at `Replied` reads under infinite-trace as the stutter step repeated after positive delays, and `always holds(c.phase != Waiting)` evaluated from position 2 over it is `true`; under event-position false-extension the execution closes at position 2."
    },
    {
      "id": "FR-651-AC-1",
      "path": "spec/functional/FR-651-run-several-selections-and-population-expansions-in-one-request.md",
      "role": "examined",
      "excerpt": "Through `run`, an `items` request over FR-108's ConfigVersion unit with two `clause` selections of `ParentOrder`, one over the healthy-parent current snapshot and one over the violating-parent snapshot, returns one `native-run-items/1` document with two items whose `result`s equal, byte for byte, the documents FR-312-AC-1's two single runs return (`success`, then `violation`), `selections` `[{\"index\": 0, \"expanded\": 1}, {\"index\": 1, \"expanded\": 1}]`, `category` `violation`, exit 10."
    },
    {
      "id": "FR-249-AC-3",
      "path": "spec/functional/FR-249-lower-a-task-automaton-to-a-timed-subject.md",
      "role": "examined",
      "excerpt": "The lowered EN-6 subject preserves FSpec FR-419's task-product state and FR-364's derived-step identity across a preemption/resumption sequence; FR-237 replays the resulting existing `ModelTrace` arm against the rebuilt product."
    }
  ],
  "findings": [],
  "bindings": []
}
```
