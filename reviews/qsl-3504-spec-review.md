---
id: SR-96723
title: "spec review of FR-1051 certificate checker trusted base"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md; PR4035 d7cf6ea955160efebaeba7d19ad42ff5f0db500c on main ef9ea586b9"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3504. Review of the trusted-base requirement against code on main ef9ea586b9. The rules and the evidence obligations are the right shape and agree with FR-282, FR-921 item 7, ADR-026 CF-6 and ADR-028 CE-7. Every exception X-1 to X-16 that was checked exists on main. But both lists are incomplete. Producer code calls more checker-code functions than X-7 to X-13 name, including document builders that derive ranks, keys and objectives. Checker code also shares more fsl-eval semantics than the base rows and X-1, X-3 and X-5 name. As written, the requirement's own claim ("No other exception is allowed") is false on main, and FR-1051-AC-5 would fail on its first run. Review cut short by the dispatcher: the parts not checked are listed under Scope and evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Producer calls checker-code document builders that derive members (ranks, keys, objective, fairness), and no exception lists them | fsl-replay/src/probability_certificate/document/measure.rs:1949-1953; document.rs:1361-1386, 1869-1872; document/mdp.rs:146; document/long_run.rs:1193-1195; fsl-analyze/src/analyze/exact.rs:26,834,1286; fsl-analyze/src/analyze/exact/measure.rs:8-10,128; fsl-analyze/src/analyze/exact/long_run.rs:11-12,264 |
| FND-002 | high | The product-closure checker and the hyper producer share certificate writing, key canonicalisation and symmetry reduction beyond X-5's three product types | fsl-replay/src/certificate/product.rs:358,618-624,794,1075; fsl-eval/src/hyper/copy_swap.rs:48,82; fsl-analyze/src/model_check/hyper.rs:339-343,810,827 |
| FND-003 | high | Other producer calls into checker code are missing from X-7 to X-13: protocol trace evaluation, the refinement liveness and protocol step judges, the closed-form reader, digital key clocks and subject admission | fsl-replay/src/model_check/replay.rs:3343; fsl-replay/src/refinement/liveness.rs:139,343; fsl-analyze/src/protocol_analysis/finite_replay.rs:26; fsl-analyze/src/model_check/refine.rs:55-57,3643-3678; fsl-analyze/src/protocol_analysis/step_view.rs:60; fsl-analyze/src/analyze/schedule_item.rs:139,151; fsl-analyze/src/analyze/exact.rs:1453,2470,4494 |
| FND-004 | high | The trusted-base rows do not say whether they cover whole modules or only the items named. Checker code uses semantic procedures that fall in neither the named items nor an exception: task-automaton lowering, protocol fairness and loop proof, zero progress, symmetry reduction, the frame judge, automaton monitor and context items outside X-1, probabilistic items outside X-3, and crates not listed | fsl-replay/src/certificate/zone_certificate/task_check.rs; certificate/closure.rs; model_check/replay.rs:48,3050; refinement/step.rs; refinement/protocol_system.rs; certificate/product.rs:19; fsl-replay/Cargo.toml |
| FND-005 | medium | Checker code omits replay checkers that producers call (state-graph evidence, hyper, finite-hyper, refinement and race replay, hybrid exact replay, the refinement-query SMT check), and the `replay_model_trace*` entry pattern matches a function outside checker code | fsl-replay/src/model_check/graph_replay.rs:1; hyper_replay.rs; finite_hyper_replay.rs; refinement_replay.rs; race_replay.rs; refinement_query.rs:250; outcome/counterexample.rs:1549; fsl-analyze/src/analyze/state_graph_item.rs:1562 |
| FND-006 | medium | Check entry and document codec are defined by example lists, so FR-1051-AC-3, AC-4 and AC-5 have no closed set to check | FR-1051 Terms |
| FND-007 | medium | The rule that a hand-written rejection test may not be removed is stated only in Status prose; no SHALL and no criterion enforces it | FR-1051 Status |
| FND-008 | low | FR-1051 overrides ADR-028 CE-7's shared EN-1 product, but relates to ADR-028 only as `depends_on`, and no statement says CE-7 is to be amended when X-1 is removed | FR-1051 frontmatter; Recorded exceptions X-1 |

### FND-001 detail

`measure_probability_certificate_document` (doc comment: "Ranks, objective and reachable coverage come from independent reconstruction"). `probability_certificate_document` ("independently derived target keys and acyclic height ranks"). `probability_proof_document`, `long_run_probability_certificate_document` ("independently derived predicate keys"), `weighted_long_run_probability_certificate_document`, `partial_lower_measure_probability_certificate_document`, `partial_lower_probability_proof_document`, `vacuous_measure_probability_certificate_document` and `conditional_measure_probability_certificate`. Also `write_timed_fair_probability_certificate`, which runs the checker's `fairness::checked` to derive the fair objective, and `write_value_bounds_measure_certificate`, which edits `ranking`. All of these are in checker code and called by `fsl-analyze`. Under the FR's own rule ("A writer in checker code that derives a member value ... is not document codec"), each one breaks "Producer code SHALL call checker code only through a check entry or the document codec". They are the self-comparing rank checks of the design review. Fix: add an exception row (Producer call into checker, QSL-3501) that names each builder, or one row per family with an exhaustive name list.

### FND-002 detail

The producer writes its certificate through the checker's `product_certificate` and `product_state_key`. Both sides take `fairness_preserving_copy_group` and `copy_symmetry` from fsl-eval. The checker canonicalises states under the same `SymmetryGroup` through the product's `canonical` hook, and both sides use `CheckedStepMatch` and `CheckedHyperLetters`. A defect in canonicalisation merges states the same way on both sides. Fix: extend X-5 to name these items, and add a producer-call row for `product_certificate` and `product_state_key`.

### FND-003 detail

These producer calls have no exception row:

- `evaluate_protocol_trace`: the producer decides with the checker's evaluator.
- `candidate_lasso_has_fair_run` and `abstract_classes`.
- `check_protocol_initial`, `check_protocol_step`, `check_protocol_state_relations`, `check_step_for_model` and `open_protocol_systems`. X-11 names only `check_initial`, `check_step` and `open_systems`.
- `step_view_over`.
- `ScheduleSubject::new` and `TaskSet::new`.
- `digital_key_clocks`.
- `admit_acyclic_probabilistic_refutation_subject` and `admit_expected_reward_certificate_subject`.

`claim_binding` and `probabilistic_claim_identity` derive identity. They need classifying, either as the "identity" service of the base or as an exception. Fix: add each to an exception row, or name it in the base with a reason.

### FND-004 detail

These items are in use and need a place in the base or in an exception:

- `fsl_eval::simulation::model::{TaskAutomaton, TaskProductSystem}`, also used by the producer.
- `fsl_eval::simulation::protocol::{ProtocolFairness, ProtocolLoopProofKind, check_protocol_bounds, activates_on_each}`.
- `fsl_eval::simulation::{ZeroProgress, SymmetryGroup}`.
- `fsl_eval::simulation::model::Judgement` (frames judge).
- `fsl_eval::automaton::{Monitor, ContextActivation, ContextBinding, ContextRoot, StateContext, SafetyItem, SafetyState, SafetyStates, BoundedState, BoundedKind, AutomatonStateKey, invariant_atom, bounded_shape}`.
- `fsl_eval::probabilistic::{DeadlineKind, MeasureComparison, MeasureError, key}`.
- The crates `fsl-route`, `filament-eval` (replay.rs:3050), `filament-observation`, `filament-walk` and `thiserror`, which are fsl-replay dependencies outside the listed crates.

Read module-wide, the base silently trusts decision procedures. Read item-wise, AC-5 fails on main. Fix: state at the top of the table that a row's parenthetical is the whole allowed set. Move the semantic procedures above (task lowering, protocol fairness and loop proof, zero progress, symmetry, the frame judge) to exception rows with tickets, and extend X-1 and X-3 to every item in use. Add the plain-data crates and `thiserror` to the base.

### FND-005 detail

Fix: add `model_check/graph_replay.rs`, `hyper_replay.rs`, `finite_hyper_replay.rs`, `refinement_replay.rs`, `race_replay.rs`, `refinement_query.rs` and the hybrid exact replay to checker code, or exclude each one with a stated reason. Restrict the replay entry pattern to checker code.

### FND-006 detail

Fix: require each check entry and codec function to be declared, for example by a marker attribute or by a list in the lint's configuration, and have the lint fail on a declared entry that does not exist. AC-3 should test against the declared set.

### FND-007 detail

Fix: add a criterion. Each route's rejection criterion keeps at least one tagged test until the route's harness registration exists, and the trace or matrix check fails when the last one is removed.

### FND-008 detail

Fix: state in the Description that FR-1051 narrows ADR-028 CE-7 and ADR-026 CF-6, and require the change that removes X-1 to amend CE-7. Consider a `refines` relationship to ADR-028.

## Verdict

Changes requested. The rule set, the mutation and false-claim obligations and AC-1 to AC-11 are testable, do not contradict each other, and agree with FR-282 (including AC-13's monitor reconstruction through fsl-eval, recorded as X-1 and X-3), FR-921 item 7 (X-2), ADR-026 CF-6 and ADR-028 CE-7. They pin no harness design beyond the mutation kinds, which the design review also names. Confirmed on main:

- X-2: `fsl-eval/src/probabilistic.rs:147`, called from digital.rs:1513,1595 and timed.rs:1753.
- X-7: measure.rs:64,313.
- X-8: digital.rs:1294,1352.
- X-9: instances.rs, called from fsl-analyze exact/instances.rs:13.
- X-10: exact.rs:2211,2284.
- X-11: refine.rs:55.
- X-12: task_symbolic.rs.
- X-13: `HybridModelReader` is the alias of `ModelReader` (lib.rs:283), used in fsl-hybrid-reference solver.rs:142.
- X-14: `ptr::eq` at document.rs:1419 and measure.rs:2405, and also in mdp.rs, long_run.rs, refutation.rs and instances.rs.
- X-15: replay.rs:3088.
- X-16: counterexample.rs:171 takes a `TaskProductSystem`.
- X-1, X-3, X-4, X-5 and X-6: the named items are imported by checker code.

The blocking problem is completeness. FND-001 to FND-004 are sharing routes that neither list names, which is the false-proof risk this requirement exists to close.

## Scope and evidence

- FR-1051 (statement, Terms, Trusted base, rules, X-1 to X-16, AC-1 to AC-11): examined.
- FR-282, FR-921, ADR-026 CF-6, ADR-028 CE-7: context only.
- Method: the `use` trees of all 82 non-test files in the declared checker paths were expanded, and every `fsl_replay` item imported by fsl-analyze and fsl-hybrid-reference was mapped to its defining file. Spot reads confirmed each cited line.
- Not checked (review stopped by the dispatcher): imports of `tools/fsl-probability-check`; checker calls into non-checker `crate::` modules (the Services row); full classification of producer-imported types as document data types versus semantic types; the existence of relationship targets; and a separate EARS or code-review pass. The diff is spec-only, and no cargo was run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Named builders are rows or assemble-only codec; the further unrecorded calls are SR-96741 |
| FND-002 | fixed | Checked against code at 47b83d02c1 |
| FND-003 | fixed | Checked against code at 47b83d02c1 |
| FND-004 | fixed | Extents stated and named items placed; further unlisted items are SR-96741 FND-005 |
| FND-005 | fixed | Paths and explicit entry list added; Routes still uses wildcards |
| FND-006 | fixed | Checked against code at 47b83d02c1 |
| FND-007 | fixed | Checked against code at 47b83d02c1 |
| FND-008 | fixed | Narrowing and amendment stated; the specifies edges are SR-96743 FND-001 |
