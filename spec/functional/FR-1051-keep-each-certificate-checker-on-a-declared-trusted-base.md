---
id: FR-1051
title: "Keep each certificate checker on a declared trusted base"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-170
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: specifies
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-284
    type: references
---
# FR-1051: Keep each certificate checker on a declared trusted base

## Description

A certificate checker (FR-282) accepts a `proved` only when its own code
establishes the claim from the certificate. A function that the checker and
the producer of the certificate both call is trusted without being checked:
a defect in it writes a wrong certificate that the checker then accepts. This
requirement names the code a checker may rely on (its trusted base), forbids
every other sharing between producer and checker, and requires two kinds of
evidence that a checker rejects what it should: single mutations of accepted
certificates, and false claims whose truth a definitional reference computes.

This requirement narrows ADR-028 CE-7 and ADR-026 CF-6. CE-7 lets the EN-5
checker share the EN-1 product construction; here that sharing is a recorded
exception (X-1 to X-3 and X-5) to be removed, and the change that removes the
last of those rows SHALL amend CE-7 to say the product is no longer shared.
CF-6 makes only the zone checker's DBM operations separate code; here every
function outside the trusted base is separate. It generalises FR-921 item 7
(no monitor shared by the timed-product producer and checker) to every
checker of FR-282's table and to the replay checkers below, including
FR-170's trap replay; FR-098's recompilation is a trusted-base service.

### Derived members and the one-type rule

The project rule "one type per interface, and each derived field from one
function both sides call" applies here to data types, and to functions used
on one side only. A certificate document's data types are one type shared by
its writer and its checker. A function that derives a member value (a key, a
rank, an objective, a fairness record, a value, an identity) SHALL NOT be
called by both a producer and a checker: the checker SHALL re-derive every
derived member it compares, with its own code. Sharing the function would
make the comparison agree with itself.

### Terms

- **Checker code**: exactly the following, excluding items under
  `#[cfg(test)]`, `#[cfg(kani)]` or `#[cfg(any(test, kani))]` and the files
  reached only through them (`certificate/zone/proofs.rs`,
  `certificate/zone/reference.rs`, `certificate/zone/tests.rs`,
  `certificate/zone/region_tests.rs`, `hybrid_replay/tests.rs`,
  `hybrid_replay/tests/**`, `model_check/replay/handoff_supply_tests.rs`).
  Code under `#[cfg(feature = "test-support")]` is included. Paths are
  relative to `fsl-replay/src` unless they name another crate:

  | Unit | Paths |
  | --- | --- |
  | EN-5 checker and probability witness replay | `probability_certificate.rs`, `probability_certificate/**` |
  | Zone, EN-1 closure, SCC, SMT, simulation, product-closure, state-graph, closed-form and task-product checkers | `certificate.rs`, `certificate/**` |
  | Hybrid certificate checker and hybrid trace replay | `hybrid_replay.rs`, `hybrid_replay/**` |
  | Counterexample replay | `model_check/replay.rs`, `model_check/replay/**`, `model_check/race_replay.rs`, `timed/explore.rs` |
  | State-graph evidence and trap replay (FR-170) | `model_check/graph_replay.rs`, `model_check/graph_hyper.rs` |
  | Hyperproperty replay | `model_check/hyper_replay.rs`, `model_check/finite_hyper_replay.rs` |
  | Refinement checking and replay | `refinement.rs`, `refinement/**`, `model_check/refinement_replay.rs` |
  | Operation-contract replay | `contract_check.rs` |
  | Single functions | `RefinementQueryRequest::check_smt_proof`, `RefinementQueryRequest::with_certificate_request` and `RefinementQuerySubject::model_subject` in `refinement_query.rs`; `replay_task_product_trace` and `TaskProductTraceReplayRefusal` in `outcome/counterexample.rs` |
  | Fresh-process checker | `tools/fsl-probability-check/src/**` |

- **Settling code**: `model_check/settle.rs`, `model_check/graph_settle.rs`,
  `model_check/hyper_settle.rs`, `model_check/existential_settle.rs`,
  `model_check/refinement_settle.rs`, `model_check/step_settle.rs`,
  `model_check/step_safety.rs`, `hybrid.rs`, `proof_result.rs` and
  `proof_result/**`. It settles an engine record on a check entry's verdict
  and is neither checker code nor producer code.
- **Producer code**: `fsl-analyze/src/**`, `fsl-hybrid-reference/src/**`,
  and these `fsl-replay/src` paths outside checker code:
  `refinement_query.rs`, `refinement_query/**`, `model_check/smt_query.rs`,
  `model_check/hyper_smt_query.rs`, `model_check/digital_state_graph.rs`,
  `model_check/state_graph.rs`, `model_check/counterexample.rs`,
  `model_check/outcome.rs`, `model_check/hyper_outcome.rs`,
  `model_check/existential_outcome.rs`, `exact_probabilistic.rs`,
  `product_action.rs`, `outcome/counterexample.rs`, `witness.rs` and
  `result/wire.rs`.
- **Not in scope**: `statistical/**` (sampled results, settled by no
  checker of FR-282), `execute/**` and `spine/**` (the TK-01 value replay
  and the front-end spine), and `protocol_analysis/**` (protocol-analysis
  findings). Their use as trusted-base services is listed below.
- **Check entry**: a function of the Check entries table. Each returns a
  checker's verdict on a certificate, witness or counterexample.
- **Document codec**: a function of the Codec table, or a type item
  (struct, enum, type alias or trait) defined in checker code other than
  these semantic types: `AbstractProtocolSystem`, `AbstractClass`,
  `ModelReader` (`HybridModelReader`), `HybridBox`, `Interval`
  (`HybridInterval`) and `Tri` (`HybridTri`). A codec
  function converts between a typed value and its canonical bytes, builds a
  document from members the producer already computed, or reads a member
  back; it derives no member value.
- **Route**: one row of the Routes table: a checker profile or form
  together with the abstraction it checks under.
- **Names**: code names an item when a `use` tree leaf, a path expression,
  a path type or a trait bound resolves to it (through `use` aliases,
  `extern crate` aliases and the `pub use` re-exports of `fsl-replay` and
  `fsl-eval`), or when a method call `x.m(..)` occurs in a file that names a
  type `T` and `T::m` is an item of a table below.

## Behavior

### Trusted base

Checker code SHALL name, outside checker code, only items of this table. A
row whose extent is "whole crate" allows every item of the crate; "module"
allows every public item of the module and its submodules except the items
of the exception rows it lists; "only" allows exactly the items named. Text
in parentheses in a "module" row describes the module and does not narrow
it.

| Part | Paths | Extent |
| --- | --- | --- |
| Front end and checked package | `fsl_foundation`, `fsl_cst`, `fsl_source`, `fsl_forms`, `fsl_semantics`, `fsl_package`, `filament_checked`, `filament_contract_model`, `filament_semantic_value` | whole crate |
| Data and support crates | `filament_eval`, `filament_walk`, `filament_observation`, `fsl_route`, `thiserror`, `ix_trace_rs`, `std` | whole crate |
| Canonical codec | `quire_canonical`, `serde`, `serde_json`, `serde_path_to_error`, `base64`, `sha2` | whole crate |
| Exact arithmetic | `filament_exact`, `num_bigint`, `num_integer`, `num_traits` | whole crate |
| Model stepping, frames and model keys | `fsl_eval::simulation` | module, except X-8 and X-13 |
| | `fsl_eval::simulation::model` (`ModelSystem`, `TimedSystem` and their steps, guards, invariants, urgency and frame judgement; `PredicateReading`; `ClauseLetters`; model and reference key encoding; `canonical_json`) | module, except X-7 and X-11 |
| | `fsl_eval::simulation::protocol` (`ProtocolSystem` stepping, memory models, `Capacities`, `check_protocol_bounds`) | module, except X-12 |
| Predicate and formula reading | `fsl_eval::model_subject`, `fsl_eval::temporal` (the ADR-018 SM-1 evaluator), `fsl_eval::value` | module |
| Position automaton | `fsl_eval::automaton::{position_automaton, PositionAutomaton, ClockedEdge, Read, Stamp, WindowClaim, WindowRefusal, Atom, Letter, StateId, AutomatonError}` | only |
| Replay services | `request::{ByteProvision, ReplayPackage, ReplayRequest, ReplayRequestWire, ReplayRequestRefusal}`; `bounds::{ReplayLimits, BoundExceeded}`; `limits::{CallerLimits, AccountingLimits}`; `identity::{DefinitionSourceRef, QualifiedName}`; `execute::{ReplayRefusal, PackageReadRefusal, request_limits, recompile, recompile_with_temporal_selections}`; `spine::{parse, select, check, package, ParseRequest, DependencyInput, SourceComponent, SpineLimits, FrontEndFailure, CompileRefusal, I2_NATIVE_JOIN_STORAGE_DEFAULT, ObservedValuation, is_effective_form}`; `application::CheckedClosureHandle`; `model_check::identity::{ClauseIdentity, StateGraphIdentity, HyperIdentity, ClauseRootRefusal, node_occurrence}`; `model_check::limits` (whole module) | only |
| Replay codecs | `reference_codec::{ReferenceCodecError, ReferenceScope}`; `product_action::{ProductAction, Schedule, FreeDelayRow}` (`ProductAction::parse`, `ProductAction::to_bytes`, `Schedule::read_transition`); `outcome::{TimedStateWire, TimedTransitionWire}`; `outcome::counterexample::{state_key_wire, task_source_transition_wire}`; `result::wire::{ProtocolWitnessWire, CauseCodecError}` | only |
| Result and evidence data types | The type items, and `Verdict::from_category`, of `exact_probabilistic::ExactProbBuildIssue`; `task_transition::{TaskTransitionAction, TaskProductTransition}`; `proof_result` and `proof_result::plugin`; `result`; `witness::{ReplaySource, WitnessEnvelope}`; `hybrid::{BoundDimension, UnsupportedConstruct}`; `protocol_analysis::{AssessedRoot, MissingMember}`, `protocol_analysis::arena::ArenaNames`; `model_check::counterexample`; `model_check::hyper_counterexample`; `model_check::graph_settle::{GraphReplayResult, GraphReplaySettlement}`; `model_check::outcome::{ModelCheckOutcome, RunStats}`; `model_check::state_graph::{GraphEvidence, ModelPath, TrapClosure, WitnessPath, GraphUndefined}`; `model_check::item::{ModelCheckItem, ItemRefusal}` | only (type items; no function except those named) |

These parts are shared with producer code by design. A defect in them is not
caught by mutation; the false-claim corpus is the evidence against them.
`fsl_eval` has no other part in the base: every other module of it
(`automaton` outside the row above, `probabilistic`, `fairness`, `hyper`,
`smt`, `execution`, `bare`, `policy_budget`) is outside it.

### No other shared semantics

- Outside the trusted base, no function or method SHALL be named both by
  checker code and by producer code.
- Producer code SHALL name checker code only through a check entry, a codec
  function or a type item of checker code.
- A function of checker code that derives a member value SHALL NOT be in the
  Codec table.

### No producer objects

- A check entry SHALL take as evidence only the certificate (bytes or typed
  document), the checked package or byte provision, the claim and subject
  binding, the checker's limits and `&Cancel`.
- A check entry SHALL NOT take, as a parameter or through a field of a
  request type of checker code, a value of a producer-built type:
  `ModelSystem`, `TimedSystem`, `ProtocolSystem`, `TaskProductSystem`,
  `AbstractProtocolSystem`, `RefinementSystems`, a `dyn` object or
  implementation of `ModelLetters`, `TemporalLetters`, `HyperLetters`,
  `TupleLetters`, `StepMatch` or `ModelEvaluator`, or a type of rows X-1 to
  X-5 or X-11. The checker SHALL build its own from the checked package.

### Check entries

| Unit | Check entries |
| --- | --- |
| EN-5 checker | `check_probability_certificate_source`, `check_probability_certificate_package`, `check_probability_certificate_document`, `check_probability_certificate_document_as`, `check_bounded_fair_probability_certificate_observing`, `check_cyclic_reward_probability_certificate_observing`, `check_cyclic_mdp_probability_certificate_observing`, `check_cyclic_mdp_probability_certificate_with_identity`, `check_long_run_probability_certificate_observing`, `check_long_run_probability_certificate_with_identity`, `check_claim_exact_reachability` |
| Probability witness replay | `replay_probabilistic_witness`, `replay_probabilistic_witness_observing`, `replay_probabilistic_witness_from_package`, `replay_probabilistic_undefined`, `replay_on_each_witness` |
| Zone and state graph | `check_zone_certificate`, `check_free_delay_zone_certificate`, `check_state_graph` |
| EN-1 closure and SCC | `check_closure`, `check_components`, `check_components_over_stutter_closure`, `check_protocol_closure`, `check_protocol_components` |
| Product closure | `check_product_closure`, `check_model_relation_closure`, `check_package_hyper_closure` |
| SMT and simulation | `check_smt_proof`, `check_refinement_smt_proof`, `RefinementQueryRequest::check_smt_proof`, `check_simulation_certificate` |
| Closed form | `settle` (`certificate::closed_form::settle`) |
| Hybrid | `check_certificate` (`check_hybrid_certificate`), `exact_replay`, `static_replay` |
| Counterexample replay | `replay_model_trace`, `replay_model_trace_from_package`, `replay_model_trace_with_temporal_selections`, `replay_checked_source_model_trace`, `replay_protocol_trace`, `replay_protocol_trace_from_package`, `replay_protocol_trace_with_temporal_selections`, `replay_timed_counterexample`, `replay_task_product_trace` |
| State-graph evidence and trap replay | `replay_model_graph`, `replay_model_graph_from_package`, `replay_model_graph_with_cancel`, `replay_model_graph_with_cancel_and_temporal_selections`, `replay_model_graph_with_temporal_selections`, `graph_hyper::replay_evidence` |
| Hyperproperty replay | `check_match_witness`, `replay_hyper_trace_tuple`, `replay_model_relation_input`, `replay_model_relation_tuple`, `replay_finite_hyper_refutation`, `reproduce_finite_hyper_refutation` |
| Refinement replay | `replay_refinement_trace`, `replay_timed_refinement_trace`, `replay_free_delay_refinement_trace`, `replay_protocol_assumption_witness` |
| Operation contract | `ContractTarget::replay` |

### Codec

The Codec table lists every codec function that code outside checker code
may name. An associated function whose only parameter is `&self` and that
returns a member, or a reference to one, without computing it, is a codec
accessor and need not be listed.

| Document | Codec functions |
| --- | --- |
| EN-5 | `write_probability_certificate`, `write_value_bounds_certificate`, `write_measure_probability_certificate`, `conditional_measure_probability_certificate`, `write_long_run_probability_certificate`, `write_weighted_long_run_probability_certificate`, `read_long_run_probability_certificate`, `write_probability_proof_document`, `write_value_bounds_proof_document`, `ProbabilitySubsystemCertificate::{from_long_run, from_weighted_long_run, from_document, as_bytes}`, `ProbabilityProductKey::{as_bytes, from_canonical_bytes}`, `ProbabilityDocumentLimits::{for_exact, for_interval, interval_scalar_setting}`, `ProbabilityDocumentIoLimits::default`, `ProbabilityDocumentLimit::model_limit`, `ReproducedProbabilityWitness::evidence` |
| Zone and task product | `ZoneCertificate::{to_wire, to_wire_with_source, from_wire, from_wire_with_source}`, `TaskProductCertificate::to_wire`, `Zone::{zero, from_matrix, dim, bound}`, `CertificateCheckLimits::default`, `TaskProductKey::{control, octets}`, `StateGraphCertificateForm::as_str`, `NonZeno::as_str`, `TaskClosureMethod::as_str` |
| Hybrid | `read_certificate` (`read_hybrid_certificate`), `HybridCounterexample::to_wire`, `HybridTrace::{to_wire, admit}`, `HybridReplayMeter::new`, `ExactInterval::{new, point}` |
| Closed form | `TaskSet::{name, tasks, task}`, `Arrival::interval`, `TaskSetRefusal::{code, cause}` |
| Replay and refinement | `ModelTraceRefusal::{code, cause}`, `RefinementFailure::position`, `Undetermined::cause`, `SafetyUndecided::cause`, `CertifiedPosition::candidate_clock_block_count`, `RefinementPosition::{candidate_clock_block_count, candidate_clock_block}` |

### Recorded exceptions

The sharing below exists on main and violates the rules above. Each row is
allowed only until its ticket removes it. The change that removes a sharing
SHALL delete its row, or the named items of it, in the same change. No other
exception is allowed. The Matches column is what the arch-lint check matches
the row by; Paths are as under "Names".

| ID | Rule broken | Matches | Sharing | Removed by |
| --- | --- | --- | --- | --- |
| X-1 | Shared semantics | `fsl_eval::automaton::{translate, translate_counted, translate_negation, BoundedTranslation, SafetyTranslation, SafetyStates, SafetyState, SafetyItem, PropertyAutomaton, Monitor, BoundedState, BoundedKind, AutomatonStateKey, SubjectKind, bounded_shape, invariant_atom}` | ADR-028 CE-7's shared EN-1 product: the property-automaton and monitor translations, named by the closure, SMT, zone, product-closure, hyper-replay and probability checkers and by `fsl-analyze` | QSL-3501 |
| X-2 | Shared semantics | `model_check::item::{Translation, LivenessTranslation, Instance}`, `ModelCheckItem::instances_for`, `Translation::admits` | The per-instance translations of a model-check item, built by the same code for `fsl-analyze` model checking and for the EN-1 closure checker | QSL-3501 |
| X-3 | Shared semantics | `fsl_eval::automaton::{QualifiedBoundedMonitor, QualifiedBoundedError, ContextActivation, ContextBinding, ContextRoot, StateContext}` | The qualified bounded monitor and its key contexts, stepped by `fsl-analyze` exact analysis and by the EN-5 checker | QSL-3501 |
| X-4 | Shared semantics | `fsl_eval::probabilistic::advance_nested_timed_deadline` | The nested timed deadline step, called by `fsl-analyze` digital stepping and the timed-product checker (FR-921 item 7 already forbids it) | QSL-3324 |
| X-5 | Shared semantics | every item of `fsl_eval::probabilistic` and `fsl_eval::probabilistic::key` other than X-4 (`MeasurePlan`, `MeasurePayoff`, `MeasureComparison`, `MeasureError`, `Accumulator`, `MonitorState`, `DeadlineKind`, `DeadlinePhase`, `TimedOperatorState`, `MeasureProductKey`, `RewardKey`) | The measure and monitor step | QSL-3501 |
| X-6 | Shared semantics | every item of `fsl_eval::fairness` | The fairness graph analysis (`analyze`, `components`, `is_live`, `check_loop` and their graph types), named by the zone, SCC, refinement and counterexample-replay checkers and by `fsl-analyze` | QSL-3501 |
| X-7 | Shared semantics | every item of `fsl_eval::hyper` other than X-8; `fsl_eval::simulation::model::CheckedHyperLetters` | The hyperproperty product and closed-execution search (`HyperProduct`, `ProjectedProduct`, `WitnessProduct`, `closed_executions`, `judge_closed_tuple`, `search_partner`, `replays_finite_refutation`, `CheckedStepMatch`), named by the product-closure and hyper-replay checkers and by `fsl-analyze` | QSL-3501 |
| X-8 | Shared semantics | `fsl_eval::hyper::{copy_symmetry, fairness_preserving_copy_group}`, `fsl_eval::simulation::{SymmetryGroup, Permutation}` | Copy symmetry reduction, applied by the product-closure checker and by `fsl-analyze` hyper search | QSL-3501 |
| X-9 | Shared semantics | every item of `fsl_eval::smt` | The SMT query encoders (`encode_smt_query`, `encode_refinement_query`), used by the query producers `model_check/smt_query.rs` and `refinement_query.rs` and re-run by the SMT proof checker | QSL-3501 |
| X-10 | Shared semantics | `refinement_query::refinement_step_rows` | The refinement step-row count, named by the SMT proof checker and the refinement query encoder | QSL-3501 |
| X-11 | Shared semantics | `fsl_eval::simulation::model::{TaskAutomaton, TaskAutomatonRefusal, TaskEdge, TaskLocation, TaskTemplate, TaskProductSystem, TaskProductPlan, TaskProductPlanRefusal, TaskProductTrace, TaskProductTraceStep, TaskProductTransition, TaskProductReplayRefusal, Successors, AllowancePhase, CertificationRefusal, ClockAtom, ClockRole, EdgeKind, JobPhase, LocationInfo, TaskKeyControl, TaskKeyJob, TaskKeyPending, CriticalityMode, write_task_control}` | Task lowering and the task product, built by the same code for `fsl-analyze` task search and the task-product checker and replay | QSL-3501 |
| X-12 | Shared semantics | `fsl_eval::simulation::protocol::{ProtocolFairness, ProtocolFairnessError, FairnessOrigin, ProtocolLoopProofFailure, ProtocolLoopProofKind, ProtocolLoopProofTarget, protocol_constraints, activates_on_each}` | Protocol fairness constraints and loop proof, named by counterexample replay, the closure checker and refinement and by `fsl-analyze` | QSL-3501 |
| X-13 | Shared semantics | `fsl_eval::simulation::{ZeroProgress, ZeroProgressError}` | The zero-progress fair-loop analysis, named by the closure checker and `fsl-analyze` model checking | QSL-3501 |
| X-14 | Shared semantics | `model_check::digital_state_graph::{explore_digital_state_graph, DigitalStateGraph}` | Digital state-graph exploration, called by `fsl-analyze` state-graph analysis and by graph replay | QSL-3501 |
| X-15 | Shared semantics | `Schedule::of`, `ProductAction::free_delays` | The canonical schedule and free-delay action keys, derived by the same functions for `fsl-analyze` exact analysis and the EN-5 checker | QSL-3501 |
| X-16 | Shared semantics | `model_check::counterexample::undefined_finding` | The choice of the first undefined finding of an expansion, made by the same function in `fsl-analyze` model checking and counterexample replay | QSL-3501 |
| X-17 | Producer call into checker | `measure_probability_objective`, `measure_probability_product_key`, `MeasurePayoffKind::initial_control` | The checker's own objective plan, measure key and measure control, called by `fsl-analyze` exact measure and undefined-witness code | QSL-3501 |
| X-18 | Producer call into checker | `digital_probability_product_key`, `region_probability_product_key`, `ProbabilityProductKey::{for_model_state_with_limits, for_formula_state, for_formula_state_with_budget}`, `digital_key_clocks`, `ProbabilityKeyAdmission::{new, refusal}` | Product-key encoding, clock decoding and key accounting, called by `fsl-analyze` exact and digital analysis | QSL-3501 |
| X-19 | Producer call into checker | `measure_probability_certificate_document`, `partial_lower_measure_probability_certificate_document`, `vacuous_measure_probability_certificate_document`, `write_value_bounds_measure_certificate` | Measure writers that derive ranks, the objective, coverage or the bound ranking, called by `fsl-analyze` exact measure | QSL-3501 |
| X-20 | Producer call into checker | `write_timed_fair_probability_certificate`, `probability_proof_document`, `partial_lower_probability_proof_document` | Writers that derive the fair objective (through the checker's fairness code), fairness records, cyclic-MDP ranks and policy, or partial ranks, called by `fsl-analyze` exact analysis | QSL-3501 |
| X-21 | Producer call into checker | `long_run_probability_certificate_document`, `weighted_long_run_probability_certificate_document`, `ProbabilitySubsystemCertificate::value_bound` | Long-run writers that derive predicate keys, the optimisation, fairness or the weight objective, and the subsystem writer that derives its base objective | QSL-3501 |
| X-22 | Producer call into checker | `on_each_outcome`, `on_each_has_instance` | The on-each producer lives in the checker file `probability_certificate/document/instances.rs` and shares its helpers with the on-each checker | QSL-3501 |
| X-23 | Producer call into checker | `probabilistic_formula_monitor_context` | The checker's monitor context, called by `fsl-analyze` exact analysis | QSL-3501 |
| X-24 | Producer call into checker | `admit_acyclic_probabilistic_refutation_subject`, `admit_expected_reward_certificate_subject` | The checker's subject admission (acyclicity, reward subject), called by `fsl-analyze` before it writes | QSL-3501 |
| X-25 | Producer call into checker | `probabilistic_claim_identity`, `claim_binding` | The claim identity and binding derivation, called by `fsl-analyze` to write the certificate identity the checker compares | QSL-3501 |
| X-26 | Producer call into checker | `check_initial`, `check_step`, `check_step_for_model`, `check_protocol_initial_with_transition_limit`, `check_protocol_step_with_transition_limit`, `check_protocol_state_relations` | The refinement step judge, called by the simulation and zone refinement checkers and by `fsl-analyze` refinement search | QSL-3501 |
| X-27 | Producer call into checker | `open_systems`, `open_systems_with_initial_limit`, `open_protocol_systems_with_initial_limit`, `PackageRefinement::read`, `AbstractProtocolSystem`, `protocol_transition_class`, `step_view_over` | Refinement subject admission, the abstract protocol system (stepping, keys and fairness) and the protocol step view, called by `fsl-analyze` refinement, contract and protocol analysis | QSL-3501 |
| X-28 | Producer call into checker | `candidate_lasso_has_fair_run`, `candidate_lasso_has_fair_run_with_recurring_edge`, `abstract_classes`, `AbstractClass` | The refinement liveness fairness check and abstract classes, called by `fsl-analyze` refinement liveness | QSL-3501 |
| X-29 | Producer call into checker | `evaluate_protocol_trace` | The protocol trace evaluator of counterexample replay, called by `fsl-analyze` finite protocol replay to decide | QSL-3501 |
| X-30 | Producer call into checker | `TaskSet::new`, `ScheduleSubject::new`, `ScheduleSubject::{by_priority, higher_priority, bound, analysed, analysis}` | Closed-form subject admission and priority order, used by `fsl-analyze` schedulability analysis | QSL-3501 |
| X-31 | Producer call into checker | `TaskProductKey::new`, `same_source_transition`, `TaskTransitionTemplate::same_identity`, `TaskQueueBound::from_checked_task_set`, `TaskProductCertificate::{closed_finite_product, closed_no_release_product}` | Task-product key order, transition identity, queue bounds and closure method, derived by checker functions for `fsl-analyze` task search | QSL-3501 |
| X-32 | Producer call into checker | `product_certificate`, `product_state_key`, `PackageHyperClosureRequest::new` | The product-closure writer (fairness components and canonical closure), its key and subject binding, called by `fsl-analyze` hyper search | QSL-3501 |
| X-33 | Producer call into checker | `HybridModelReader` (`ModelReader`), `HybridBox`, `HybridInterval` (`Interval`), `HybridTri` (`Tri`), `box_advance`, `box_within`, `box_hull`, `box_tube`, `probe_hybrid_flowpipe_claim`, `polynomial_class_gap` | The hybrid checker's model reader, interval machine, flowpipe rule probe and class admission, used by `fsl-hybrid-reference` to build boxes | QSL-3501 |
| X-34 | Producer call into checker | `ContractTarget::evaluate` | The operation-contract arm in `fsl-analyze` decides each case with the evaluator its replay uses | QSL-3501 |
| X-35 | Producer object | `check_probability_certificate_document`, `check_probability_certificate_document_as`, `check_bounded_fair_probability_certificate_observing`, `check_cyclic_reward_probability_certificate_observing`, `check_cyclic_mdp_probability_certificate_observing`, `check_cyclic_mdp_probability_certificate_with_identity`, `check_long_run_probability_certificate_observing`, `check_long_run_probability_certificate_with_identity`, `check_claim_exact_reachability`, `replay_probabilistic_witness`, `replay_probabilistic_witness_observing`, `replay_probabilistic_witness_from_package`, `replay_probabilistic_undefined`, `replay_on_each_witness` (parameter `ModelSystem`) | The checked-subject form of the EN-5 checker and the probability witness replays take the producer's `ModelSystem` (compared by pointer), as `fsl-analyze` exact analysis calls them | QSL-3501 |
| X-36 | Producer object | `replay_model_trace`, `replay_model_trace_from_package`, `replay_model_trace_with_temporal_selections`, `replay_protocol_trace`, `replay_protocol_trace_from_package`, `replay_protocol_trace_with_temporal_selections` (parameter `dyn ModelLetters`); `replay_checked_source_model_trace` (parameter `ModelSystem`); `replay_timed_counterexample` (parameters `TimedSystem`, `dyn ModelLetters`) | Counterexample replay reads atoms through the producer's letters (`ClauseLetters`) and, on the reference-literal path, steps the producer's system | QSL-3501 |
| X-37 | Producer object | `check_zone_certificate`, `check_free_delay_zone_certificate` (field `ZoneCertificateRequest::item`, `CertifiedItem::Clause::letters: dyn ModelLetters`) | The zone checker reads clause atoms through the producer's letters | QSL-3501 |
| X-38 | Producer object | `replay_task_product_trace` (parameter `TaskProductSystem`) | Task-product trace replay checks keys of the producer's own product object | QSL-3501 |
| X-39 | Producer object | `replay_refinement_trace`, `replay_timed_refinement_trace`, `replay_free_delay_refinement_trace`, `replay_protocol_assumption_witness` (parameter `RefinementSystems`) | Refinement replay steps the systems the producer opened | QSL-3501 |
| X-40 | Producer object | `check_product_closure`, `check_model_relation_closure`, `check_package_hyper_closure`, `replay_model_relation_input` (fields `evaluator: dyn ModelEvaluator`, `ModelRelationReplayInput::systems: ModelSystem`) | Product closure and model-relation replay evaluate through the producer's evaluator and systems | QSL-3501 |

### Routes

| Route | Check entry |
| --- | --- |
| EN-5 profiles `reachability/v1`, `state-predicate/v1`, `expected-reward/v1`, `dtmc-expected-reward/v1`, `cyclic-dtmc-eventual/v1`, `cyclic-mdp-eventual/v1`, `cyclic-dtmc-state-predicate/v1`, `cyclic-mdp-state-predicate/v1`, `cyclic-mdp-expected-reward/v1`, `bounded-fair-eventual/v1`, `cyclic-mdp-fair-eventual/v1`, `finite-horizon-measure/v1`, `value-bounds/v1`, `on-each-reachability/v1`, `dtmc-long-run/v1`, `dtmc-weighted-long-run/v1`, one route each | `check_probability_certificate_source` |
| EN-5 `timed-product/v1`, digital route and region route, one route each | `check_probability_certificate_source` |
| Zone: clause safety and TT-2, deadlock freedom, time-lock freedom, refinement, state-graph `possible`, `always possible`, `unique path`, liveness, free delay, task product, one route each | `check_zone_certificate`, `check_free_delay_zone_certificate` |
| Explicit state graph | `check_state_graph` |
| EN-1 closure; SCC components; protocol closure; protocol components | `check_closure`, `check_components`, `check_protocol_closure`, `check_protocol_components` |
| Product closure; model-relation closure; package hyper closure | `check_product_closure`, `check_model_relation_closure`, `check_package_hyper_closure` |
| SMT proof; refinement SMT proof | `check_smt_proof`, `check_refinement_smt_proof` |
| Simulation relation | `check_simulation_certificate` |
| Closed form | `settle` |
| Hybrid certificate; hybrid exact replay | `check_hybrid_certificate`, `exact_replay` |
| Counterexample replay, source route and byte route; protocol trace; timed counterexample; task-product trace | `replay_model_trace*`, `replay_protocol_trace*`, `replay_timed_counterexample`, `replay_task_product_trace` |
| State-graph evidence and trap replay | `replay_model_graph*` |
| Hyper trace tuple; model relation; finite hyper refutation | `replay_hyper_trace_tuple`, `replay_model_relation_input`, `reproduce_finite_hyper_refutation` |
| Refinement trace; protocol assumption witness | `replay_refinement_trace`, `replay_protocol_assumption_witness` |
| Probability witness; undefined witness; on-each witness | `replay_probabilistic_witness*`, `replay_probabilistic_undefined`, `replay_on_each_witness` |
| Operation-contract counterexample | `ContractTarget::replay` |

### Rejection evidence by mutation

- Each route SHALL register at least one accepted certificate fixture with a
  mutation harness. The harness is a Rust test-support crate that is only a
  dev-dependency, has no `fsl-replay` dependency, and takes each
  registration's check as a closure from the fixture to a verdict.
- A route whose certificate has a canonical byte form SHALL register its
  canonical document; the harness mutates the canonical tree and the closure
  calls the route's byte or source check entry (for EN-5, the source or
  package entry).
- A route whose certificate has only a typed form (state graph, EN-1
  closure, SCC components, closed form, simulation, SMT proof) SHALL register
  a typed adapter that applies each mutation kind to the typed value's
  members and returns the mutated value, and a closure that calls the
  route's typed check entry. The harness SHALL treat both kinds alike.
- For every registered fixture, every single mutation the harness makes
  SHALL be refused or rejected by the route's check entry, except a
  mutation on the route's allowed-to-survive list.
- The mutations SHALL include, wherever the certificate has such a member:
  a value changed, a row removed, a row duplicated, a key changed, a rank
  changed by one, a scheduler choice changed, a bound side flipped and the
  identity changed.
- Each allowed-to-survive entry SHALL name the member, the mutation and the
  reason the mutated certificate still establishes the claim (for example a
  value raised on an `upper` row that still satisfies its inequality and the
  threshold).
- Until a route has a harness registration, its hand-written rejection tests
  (tests that assert a check entry of the route rejects or refuses a changed
  certificate) SHALL NOT be removed.

### Rejection evidence by false claims

- Each route SHALL have a corpus of false claims over small models.
- The truth of each corpus claim SHALL be computed by a definitional
  reference: test code in its own crate, a dev-dependency only, that depends
  on `fsl-package`, `fsl-semantics`, `fsl-foundation` and the data crates
  `filament-exact`, `filament-checked`, `filament-contract-model` and
  `filament-semantic-value`, and on no other crate of this workspace (never
  `fsl-eval`, `fsl-analyze`, `fsl-replay` or `fsl-hybrid-reference`).
- The reference SHALL read the model from the `CheckedPackage`'s
  `CheckedGraph` (operation bodies and effects, the timed model's guards,
  resets and invariants, probabilistic supports, hybrid modes) with its own
  interpreter. When package checking needs an evaluator port, such as
  `RewardEvaluator`, the reference SHALL pass its own implementation, never
  one from `fsl-eval`.
- The reference SHALL accept only this fragment, and SHALL refuse, naming
  the construct, a corpus model or claim outside it:
  - state: finitely many fields of Boolean, enumeration or integer type with
    declared bounds, at most 2^16 valuations;
  - operations: conjunctive guards of comparisons between a field and a
    literal or another field; effects that assign a literal, a field, or a
    field plus or minus a literal;
  - probabilistic choice: a finite support of literal rational weights;
    rewards as literal rationals per operation;
  - clocks: rational valuations, guards and invariants that are
    conjunctions of `x ⋈ c` and `x - y ⋈ c` with integer `c`, and resets to
    zero; dense or digital time as the claim's route declares;
  - hybrid modes: constant rational rates, linear guards and invariants,
    resets to literals;
  - claims: Boolean combinations of field comparisons; `next`, `until`,
    `always`, `eventually` and their bounded windows; two-copy hyper
    formulas of these; probability, expected-reward and quantile bounds over
    every or some scheduler; `always possible` and `unique path`; task-set
    deadlines over a hyperperiod.
- The reference SHALL decide falsity definitionally: a bounded claim by
  enumerating every path to its horizon; an unbounded probability claim over
  a finite chain by its own exact rational solution of the defining
  equations; a claim over every scheduler by one scheduler, with its delays
  where timed, that the corpus entry gives and the reference evaluates; a
  claim over some scheduler by enumerating every memoryless deterministic
  scheduler (reachability and expected-reward objectives only); and a timed
  claim that needs every continuation from a state by exploring its own
  Alur-Dill region quotient from that concrete state.
- For every corpus claim, analysis through that route SHALL NOT settle
  `proved`, and the route's check entry SHALL NOT accept a certificate for it.

The corpus seeds are the false claims that reached `proved`. Each needs this
part of the fragment:

| Seed | Route | Model fragment | Claim and falsity witness |
| --- | --- | --- | --- |
| QSL-923 | Product closure, two copies | finite fields, untimed, no probability | `forall a forall b { (not holds(l@a=1)) and eventually holds(l@a=1) }`; witness the lasso `step(0)` forever from `(0,0)` |
| QSL-3324 | Timed product, digital | one clock, digital time, a probabilistic race with a 3-unit delay | `always[0,0] eventually[2,2] P` from a not-P state; witness the scheduler taking the 3-unit race delay |
| QSL-3414 | Value bounds (`value-bounds/v1`) | finite fields, untimed, acyclic probabilistic choice, two initial states | a `>= c` quantile with initial value 0 and a reachable tight activation; exact value by path enumeration from the second initial state |
| QSL-3444 | Timed product, region | two clocks `x`, `y`, dense time, strict guards | `eventually[0 ms, 4 ms] holds(c.late)` with probability `<= 0.999` over every scheduler; witness delays `1/2` ms then `6/5` ms, value 1 |
| QSL-3445 | Timed product, digital | two clocks, dense time, closed guards, a 1:1 draw | elapsed quantile `0.4 ... <= 1 ms` over every scheduler; witness `first` at `1/2` ms, value 0 |
| QSL-3478 | Zone, `always possible` and `unique path` | two clocks, dense time | `always possible Done { c.phase >= 2 }` and `unique path Done from (c.phase = 1) to (c.phase >= 2)`; witness `first` at `1/2` ms, then the region quotient shows no run reaches phase 2 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1051-AC-1 | An arch-lint check reads a declared configuration whose sets are exactly this requirement's checker-code paths, producer-code paths, settling-code paths, Trusted base rows, Check entries, Codec functions and exception rows. Over fixture crates it fails when checker code names an item outside the trusted base and in no exception row, naming the calling file, line and the named path (for example a checker module that names `fsl_analyze::...` or `fsl_eval::probabilistic::MeasurePlan`), and passes when every named path is in the trusted base. | Test |
| FR-1051-AC-2 | Over fixture crates, the same check fails when a function or method outside the trusted base is named both by checker code and by producer code, naming the item and one site on each side; with the function named on one side only, it reports nothing for it. | Test |
| FR-1051-AC-3 | Over fixture crates, the same check fails when producer code names a checker-code function that is neither a check entry nor a codec function nor a codec accessor, naming the function and the site; naming `check_probability_certificate_document`, `write_probability_certificate` or a checker type item passes. A method call `x.m()` is attributed to `T::m` only in a file that names `T`. | Test |
| FR-1051-AC-4 | Over fixture crates, the same check fails when a check entry has a parameter, or a field of a checker-code request type it takes, of a producer-built type of "No producer objects", naming the entry, the parameter or field and the type. | Test |
| FR-1051-AC-5 | On the repository, every finding of the check matches an exception row by its Matches column and the rule it breaks, and every exception row matches at least one finding; a finding that matches no row fails, and a row (or a named item of a row) that matches no finding fails as stale, naming its ID and item. | Test |
| FR-1051-AC-6 | The same check fails as stale, naming the entry, when a check entry, codec function, trusted-base item or exception item of its configuration names no item that exists in the repository, or when a checker-code path names no file. | Test |
| FR-1051-AC-7 | For every registered fixture of every route, every single mutation that is not on the route's allowed-to-survive list is refused or rejected by the route's check entry; a mutation the entry accepts fails the harness, naming the route, the fixture, the member path and the mutation. Verified by the harness's own tests, including one where a deliberately weakened check closure accepts a changed value, run once over a canonical document and once through a typed adapter. | Test |
| FR-1051-AC-8 | An allowed-to-survive entry without a reason, or one whose mutation the harness no longer makes or the check entry now rejects, fails the harness as stale, naming the entry. | Test |
| FR-1051-AC-9 | The harness fails, naming each route, when a route of the Routes table has no registered accepted fixture, or when a registered fixture is not accepted by its check entry. The harness crate's manifest has no `fsl-replay` dependency and it is a dev-dependency only. The first registered routes are the finite-horizon measure profile, the value-bounds profile, the cyclic-MDP fair profile, zone `always possible` and `unique path`, and counterexample replay on the byte route; a typed-only route (`check_state_graph`) registers through a typed adapter. | Test |
| FR-1051-AC-10 | A repository test holds, for each route without a harness registration, the names of its hand-written rejection tests as of this requirement. It fails, naming the route and the test, when a named test is no longer defined; it fails as stale, naming the route, when a route that has a harness registration still has a list. | Test |
| FR-1051-AC-11 | A test fails, naming each route, when a route of the Routes table has no false-claim corpus entry. For every corpus entry the reference's truth value is false, analysis through that route does not settle `proved`, and the route's check entry does not accept the certificate a producer writes for it. | Test |
| FR-1051-AC-12 | The corpus holds the six seeds of the seed table, each on its route with the model and witness given there; none settles `proved` and no check entry accepts a certificate for any of them. | Test |
| FR-1051-AC-13 | The reference crate's manifest has no direct dependency on a workspace crate other than `fsl-package`, `fsl-semantics` and `fsl-foundation`, and is a dev-dependency only; the arch-lint check fails on a reference fixture that names `fsl_eval::simulation::model::ModelSystem` or passes an `fsl-eval` `RewardEvaluator` to package checking. | Test |
| FR-1051-AC-14 | The reference refuses, naming the construct, a corpus model with an unbounded integer field, a guard that is a disjunction, a non-literal probability weight, a clock compared with a non-integer constant, or a hybrid mode with a non-constant rate, and returns no truth value for it. | Test |

## Status

PLANNED. No criterion is met on main. The exception rows record the sharing
on origin/main as of this requirement. The arch-lint check, the restructuring
that removes X-1 to X-3 and X-5 to X-40 (QSL-3501), the separate timed
monitor that removes X-4 (QSL-3324), the mutation harness (QSL-3505) and the
definitional reference with its corpus (QSL-3500) are not yet built.

## Dependencies

The frontmatter relationships are the links: FR-282 (the checkers, their
entries and inputs), FR-921 (item 7, generalised here), FR-170 (trap replay,
in checker code), FR-098 (recompilation, a trusted-base service), ADR-028
CE-7 and ADR-026 CF-6 (narrowed here) and FR-284 (the qualified-core crates
the base is drawn from).
