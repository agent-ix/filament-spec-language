// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's classes, trusted base, exception rows and routes, and the
//! item-level lists the requirement leaves to this check.

use super::{BaseRow, Classes, ExceptionRow, Extent, Policy, Rule};

mod entries;
mod lists;

const CLASSES: Classes = Classes {
    by_file: "fsl-replay",
    checker_files: &[
        "probability_certificate.rs",
        "probability_certificate/**",
        "certificate.rs",
        "certificate/**",
        "hybrid_replay.rs",
        "hybrid_replay/**",
        "model_check/replay.rs",
        "model_check/replay/**",
        "model_check/race_replay.rs",
        "timed/explore.rs",
        "model_check/graph_replay.rs",
        "model_check/graph_hyper.rs",
        "model_check/hyper_replay.rs",
        "model_check/finite_hyper_replay.rs",
        "refinement.rs",
        "refinement/**",
        "model_check/refinement_replay.rs",
        "contract_check.rs",
    ],
    checker_items: &[
        (
            "refinement_query.rs",
            "RefinementQueryRequest::check_smt_proof",
        ),
        ("outcome/counterexample.rs", "replay_task_product_trace"),
        ("outcome/counterexample.rs", "TaskProductTraceReplayRefusal"),
    ],
    checker_crates: &["tools/fsl-probability-check"],
    settling_files: &[
        "model_check/settle.rs",
        "model_check/graph_settle.rs",
        "model_check/hyper_settle.rs",
        "model_check/existential_settle.rs",
        "model_check/refinement_settle.rs",
        "model_check/step_settle.rs",
        "model_check/step_safety.rs",
        "hybrid.rs",
        "proof_result.rs",
        "proof_result/**",
    ],
    out_of_scope_files: &["statistical/**", "protocol_analysis/**"],
    producer_files: &[
        "admitted.rs",
        "application.rs",
        "axes.rs",
        "backends.rs",
        "bounds.rs",
        "build_inventory.rs",
        "call_replay_context.rs",
        "call_requirements.rs",
        "call_site.rs",
        "canonical_atom.rs",
        "clause_evaluation.rs",
        "compile.rs",
        "composite.rs",
        "exact_probabilistic.rs",
        "execute/argument.rs",
        "execute/composite_domain.rs",
        "execute/composite_parity.rs",
        "execute/composite_site.rs",
        "execute/frame.rs",
        "execute/object_environment.rs",
        "execute/operator_parity.rs",
        "execute/parity_identity.rs",
        "execute.rs",
        "execute/scalar_site.rs",
        "execute/state_clause.rs",
        "execute/value_parity.rs",
        "hybrid/locus.rs",
        "hybrid/process_request.rs",
        "hybrid/provider.rs",
        "identity.rs",
        "id_text.rs",
        "lib.rs",
        "limits.rs",
        "model_check/counterexample.rs",
        "model_check/digital_state_graph.rs",
        "model_check/existential_outcome.rs",
        "model_check/hyper_counterexample.rs",
        "model_check/hyper_outcome.rs",
        "model_check/hyper_smt_outcome.rs",
        "model_check/hyper_smt_query.rs",
        "model_check/identity.rs",
        "model_check/item.rs",
        "model_check/limits.rs",
        "model_check/outcome.rs",
        "model_check.rs",
        "model_check/smt_query.rs",
        "model_check/state_graph.rs",
        "model_check/subject_identity.rs",
        "monitor_plan.rs",
        "object_environment.rs",
        "outcome/counterexample.rs",
        "outcome/hint.rs",
        "outcome.rs",
        "planner/admitted_declaration.rs",
        "planner/compose.rs",
        "planner/composition_graph.rs",
        "planner/declaration.rs",
        "planner/entry.rs",
        "planner/from_admitted.rs",
        "planner/from_package.rs",
        "planner/item.rs",
        "planner/json.rs",
        "planner/key_build.rs",
        "planner/key.rs",
        "planner/mismatch.rs",
        "planner/request.rs",
        "planner.rs",
        "planner/settle.rs",
        "planner/transfer.rs",
        "product_action.rs",
        "protocol_analysis.rs",
        "reference_budget.rs",
        "reference_codec/json.rs",
        "reference_codec.rs",
        "reference_codec/values.rs",
        "refinement_query/handoff.rs",
        "refinement_query.rs",
        "replay_model_input.rs",
        "request/package.rs",
        "request.rs",
        "result.rs",
        "result/wire/replay/arms.rs",
        "result/wire/replay.rs",
        "result/wire.rs",
        "result/wire/value.rs",
        "scalar.rs",
        "source_inventory.rs",
        "statistical.rs",
        "spine/bind.rs",
        "spine/call.rs",
        "spine/clause/clause_state/progress.rs",
        "spine/clause/clause_state.rs",
        "spine/clause/clause_state/snapshot.rs",
        "spine/clause/correction.rs",
        "spine/clause/late_data.rs",
        "spine/clause/ledger_wire.rs",
        "spine/clause/model_trace.rs",
        "spine/clause/package_authentication.rs",
        "spine/clause/progress.rs",
        "spine/clause.rs",
        "spine/clause/temporal/package_selection.rs",
        "spine/clause/temporal/package_window.rs",
        "spine/clause/temporal.rs",
        "spine/clause/temporal/source_render.rs",
        "spine/clause/trace_refusal.rs",
        "spine/clause/trace_subject.rs",
        "spine/clocks.rs",
        "spine_eval_api.rs",
        "spine/identity_form.rs",
        "spine/lifecycle.rs",
        "spine/native_fences.rs",
        "spine/native_result/protocol.rs",
        "spine/native_result.rs",
        "spine/native_result/temporal/model_read.rs",
        "spine/native_result/temporal.rs",
        "spine/package_call.rs",
        "spine/protocol_premises.rs",
        "spine/protocol_proof.rs",
        "spine/protocol_structure.rs",
        "spine/query_proof.rs",
        "spine/refinement.rs",
        "spine/relationships.rs",
        "spine/request.rs",
        "spine.rs",
        "spine/run_value/canonical.rs",
        "spine/run_value/references.rs",
        "spine/run_value.rs",
        "spine/subject.rs",
        "supplied_limits.rs",
        "target.rs",
        "task_transition.rs",
        "timed.rs",
        "value_check.rs",
        "witness/derivation.rs",
        "witness_entry_name.rs",
        "witness/frame.rs",
        "witness.rs",
        "witness/state_clause.rs",
        "witness/value.rs",
        "witness/value_text.rs",
        "written_operation.rs",
    ],
    trusted_crates: &[
        "fsl-foundation",
        "fsl-cst",
        "fsl-source",
        "fsl-forms",
        "fsl-semantics",
        "fsl-package",
        "fsl-route",
        "fsl-walk-grow",
        "fsl-eval",
    ],
    producer_crates: &[
        "fsl-analyze",
        "fsl-hybrid-reference",
        "fsl-command",
        "fsl-inspect",
        "fsl-conformance",
        "fsl-jit",
        "tools/fsl-hybrid-test-provider",
        "filament-spec-language",
    ],
    tool_crates: &[
        "xtask",
        "tools/arch-lint",
        "fsl-string-edge",
        "fsl-proptest-policy",
        "fsl-bench",
        "tests/fixtures/**",
        "fuzz",
    ],
};

const fn whole(id: &'static str, modules: &'static [&'static str]) -> BaseRow {
    BaseRow {
        id,
        modules,
        prefix: "",
        extent: Extent::WholeCrate,
        except: &[],
        recorded: &[],
    }
}

const fn module(
    id: &'static str,
    modules: &'static [&'static str],
    except: &'static [&'static str],
) -> BaseRow {
    BaseRow {
        id,
        modules,
        prefix: "",
        extent: Extent::Module,
        except,
        recorded: &[],
    }
}

const BASE: &[BaseRow] = &[
    whole(
        "B-1",
        &[
            "fsl_foundation",
            "fsl_cst",
            "fsl_source",
            "fsl_forms",
            "fsl_semantics",
            "fsl_package",
            "filament_checked",
            "filament_contract_model",
            "filament_semantic_value",
        ],
    ),
    whole(
        "B-2",
        &[
            "filament_eval",
            "filament_walk",
            "filament_observation",
            "fsl_route",
            "fsl_walk_grow",
            "thiserror",
            "ix_trace_rs",
            "std",
        ],
    ),
    whole(
        "B-3",
        &[
            "quire_canonical",
            "serde",
            "serde_json",
            "serde_path_to_error",
            "base64",
            "sha2",
        ],
    ),
    whole(
        "B-4",
        &["filament_exact", "num_bigint", "num_integer", "num_traits"],
    ),
    module("B-5", &["fsl_eval::simulation"], &["S-5", "S-10"]),
    module("B-6", &["fsl_eval::simulation::model"], &["S-5", "S-8"]),
    module("B-7", &["fsl_eval::simulation::protocol"], &["S-9"]),
    module("B-8", &["fsl_eval::model_subject"], &[]),
    BaseRow {
        id: "B-9",
        modules: &["fsl_eval::automaton"],
        prefix: "",
        extent: Extent::Recorded,
        except: &[],
        recorded: lists::B9,
    },
    BaseRow {
        id: "B-10",
        modules: &["fsl-eval"],
        prefix: "",
        extent: Extent::DataItems,
        except: &[],
        recorded: &[],
    },
    BaseRow {
        id: "B-11",
        modules: &[
            "request",
            "bounds",
            "limits",
            "identity",
            "execute",
            "spine",
            "application",
            "model_check::identity",
            "model_check::limits",
            "spine_eval_api",
        ],
        prefix: "fsl_replay::",
        extent: Extent::TypeItems,
        except: &[],
        recorded: lists::B11,
    },
    BaseRow {
        id: "B-12",
        modules: &[
            "reference_codec",
            "product_action",
            "outcome",
            "outcome::counterexample",
            "result",
            "result::wire",
            "proof_result",
            "witness",
            "task_transition",
            "exact_probabilistic",
            "hybrid",
            "protocol_analysis",
            "model_check::counterexample",
            "model_check::hyper_counterexample",
            "model_check::graph_settle",
            "model_check::outcome",
            "model_check::state_graph",
            "model_check::item",
            "written_operation",
        ],
        prefix: "fsl_replay::",
        extent: Extent::TypeItems,
        except: &[],
        recorded: lists::B12,
    },
];

const fn row(
    id: &'static str,
    rule: Rule,
    concerns: &'static [(&'static str, &'static [&'static str])],
) -> ExceptionRow {
    ExceptionRow {
        id,
        rule,
        concerns,
        planned: false,
    }
}

const EXCEPTIONS: &[ExceptionRow] = &[
    row(
        "S-1",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::automaton", &["fsl_eval::automaton"]),
            ("model_check::item", &["fsl_replay::model_check::item"]),
        ],
    ),
    row(
        "S-2",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::probabilistic", &[]),
            (
                "advance_nested_timed_deadline",
                &["fsl_eval::probabilistic::advance_nested_timed_deadline"],
            ),
        ],
    ),
    row(
        "S-3",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::probabilistic", &["fsl_eval::probabilistic"]),
            (
                "fsl_eval::probabilistic::key",
                &["fsl_eval::probabilistic::key"],
            ),
        ],
    ),
    row(
        "S-4",
        Rule::SharedSemantics,
        &[("fsl_eval::fairness", &["fsl_eval::fairness"])],
    ),
    row(
        "S-5",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::hyper", &["fsl_eval::hyper"]),
            ("fsl_eval::simulation", &[]),
            ("reduction.rs", &["fsl-eval/src/simulation/reduction.rs"]),
            ("fsl_eval::simulation::model", &[]),
            (
                "hyper_letters.rs",
                &["fsl-eval/src/simulation/model/hyper_letters.rs"],
            ),
        ],
    ),
    row(
        "S-6",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::temporal", &["fsl_eval::temporal"]),
            ("fsl_eval::value", &["fsl_eval::value"]),
        ],
    ),
    row(
        "S-7",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::smt", &["fsl_eval::smt"]),
            (
                "refinement_query.rs",
                &["fsl-replay/src/refinement_query.rs"],
            ),
        ],
    ),
    row(
        "S-8",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::simulation::model", &[]),
            (
                "task_automaton.rs",
                &["fsl-eval/src/simulation/model/task_automaton.rs"],
            ),
            (
                "task_key.rs",
                &["fsl-eval/src/simulation/model/task_key.rs"],
            ),
            (
                "task_product.rs",
                &["fsl-eval/src/simulation/model/task_product.rs"],
            ),
        ],
    ),
    row(
        "S-9",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::simulation::protocol", &[]),
            (
                "fairness.rs",
                &["fsl-eval/src/simulation/protocol/fairness.rs"],
            ),
            ("protocol.rs", lists::LOOP_PROOF_TYPES),
            (
                "activates_on_each",
                &["fsl_eval::simulation::protocol::activates_on_each"],
            ),
        ],
    ),
    row(
        "S-10",
        Rule::SharedSemantics,
        &[
            ("fsl_eval::simulation", &[]),
            ("progress.rs", &["fsl-eval/src/simulation/progress.rs"]),
        ],
    ),
    row(
        "S-11",
        Rule::SharedSemantics,
        &[(
            "model_check/digital_state_graph.rs",
            &["fsl-replay/src/model_check/digital_state_graph.rs"],
        )],
    ),
    row(
        "S-12",
        Rule::SharedSemantics,
        &[("product_action.rs", &["fsl-replay/src/product_action.rs"])],
    ),
    row(
        "S-13",
        Rule::SharedSemantics,
        &[(
            "model_check/counterexample.rs",
            &["fsl-replay/src/model_check/counterexample.rs"],
        )],
    ),
    row(
        "S-14",
        Rule::SharedSemantics,
        &[(
            "model_check/identity.rs",
            &["fsl-replay/src/model_check/identity.rs"],
        )],
    ),
    row(
        "P-1",
        Rule::ProducerCall,
        &[(
            "probability_certificate/document/measure.rs",
            &["fsl-replay/src/probability_certificate/document/measure.rs"],
        )],
    ),
    row(
        "P-2",
        Rule::ProducerCall,
        &[
            (
                "probability_certificate.rs",
                &["fsl-replay/src/probability_certificate.rs"],
            ),
            (
                "probability_certificate/key_admission.rs",
                &["fsl-replay/src/probability_certificate/key_admission.rs"],
            ),
            (
                "probability_certificate/document/timed.rs",
                &["fsl-replay/src/probability_certificate/document/timed.rs"],
            ),
        ],
    ),
    row(
        "P-3",
        Rule::ProducerCall,
        &[
            (
                "probability_certificate/document.rs",
                &["fsl-replay/src/probability_certificate/document.rs"],
            ),
            (
                "probability_certificate/document/{measure,mdp,partial,long_run}.rs",
                &[
                    "fsl-replay/src/probability_certificate/document/measure.rs",
                    "fsl-replay/src/probability_certificate/document/mdp.rs",
                    "fsl-replay/src/probability_certificate/document/partial.rs",
                    "fsl-replay/src/probability_certificate/document/long_run.rs",
                ],
            ),
            (
                "probability_certificate/witness.rs",
                &["fsl-replay/src/probability_certificate/witness.rs"],
            ),
        ],
    ),
    row(
        "P-4",
        Rule::ProducerCall,
        &[(
            "probability_certificate/document/instances.rs",
            &["fsl-replay/src/probability_certificate/document/instances.rs"],
        )],
    ),
    row(
        "P-5",
        Rule::ProducerCall,
        &[
            (
                "probability_certificate/bounded.rs",
                &["fsl-replay/src/probability_certificate/bounded.rs"],
            ),
            (
                "probability_certificate/cyclic.rs",
                &["fsl-replay/src/probability_certificate/cyclic.rs"],
            ),
            (
                "probability_certificate/document.rs",
                &["fsl-replay/src/probability_certificate/document.rs"],
            ),
            (
                "probability_certificate.rs",
                &["fsl-replay/src/probability_certificate.rs"],
            ),
        ],
    ),
    row(
        "P-6",
        Rule::ProducerCall,
        &[
            ("refinement/**", &["fsl-replay/src/refinement/**"]),
            ("refinement.rs", &["fsl-replay/src/refinement.rs"]),
        ],
    ),
    row(
        "P-7",
        Rule::ProducerCall,
        &[(
            "model_check/replay.rs",
            &["fsl-replay/src/model_check/replay.rs"],
        )],
    ),
    row(
        "P-8",
        Rule::ProducerCall,
        &[(
            "certificate/closed_form/task_set.rs",
            &["fsl-replay/src/certificate/closed_form/task_set.rs"],
        )],
    ),
    row(
        "P-9",
        Rule::ProducerCall,
        &[
            (
                "certificate/task_product.rs",
                &["fsl-replay/src/certificate/task_product.rs"],
            ),
            (
                "certificate/task_product/**",
                &["fsl-replay/src/certificate/task_product/**"],
            ),
        ],
    ),
    row(
        "P-10",
        Rule::ProducerCall,
        &[(
            "certificate/product.rs",
            &["fsl-replay/src/certificate/product.rs"],
        )],
    ),
    row(
        "P-11",
        Rule::ProducerCall,
        &[("hybrid_replay/**", &["fsl-replay/src/hybrid_replay/**"])],
    ),
    row(
        "P-12",
        Rule::ProducerCall,
        &[("contract_check.rs", &["fsl-replay/src/contract_check.rs"])],
    ),
    row(
        "P-13",
        Rule::ProducerCall,
        &[(
            "certificate/zone.rs",
            &["fsl-replay/src/certificate/zone.rs"],
        )],
    ),
    row(
        "P-14",
        Rule::ProducerCall,
        &[(
            "certificate/zone_certificate/task_check.rs",
            &["fsl-replay/src/certificate/zone_certificate/task_check.rs"],
        )],
    ),
    row(
        "O-1",
        Rule::ProducerObject,
        &[
            (
                "probability_certificate.rs",
                &["fsl-replay/src/probability_certificate.rs"],
            ),
            (
                "probability_certificate/**",
                &["fsl-replay/src/probability_certificate/**"],
            ),
        ],
    ),
    row(
        "O-2",
        Rule::ProducerObject,
        &[
            (
                "model_check/replay.rs",
                &["fsl-replay/src/model_check/replay.rs"],
            ),
            (
                "certificate/zone_certificate.rs",
                &["fsl-replay/src/certificate/zone_certificate.rs"],
            ),
            (
                "certificate/closure.rs",
                &["fsl-replay/src/certificate/closure.rs"],
            ),
            ("certificate/smt.rs", &["fsl-replay/src/certificate/smt.rs"]),
        ],
    ),
    row(
        "O-3",
        Rule::ProducerObject,
        &[(
            "outcome/counterexample.rs",
            &["fsl-replay/src/outcome/counterexample.rs"],
        )],
    ),
    row(
        "O-4",
        Rule::ProducerObject,
        &[(
            "model_check/refinement_replay.rs",
            &["fsl-replay/src/model_check/refinement_replay.rs"],
        )],
    ),
    row(
        "O-5",
        Rule::ProducerObject,
        &[
            (
                "model_check/hyper_replay.rs",
                &["fsl-replay/src/model_check/hyper_replay.rs"],
            ),
            (
                "model_check/finite_hyper_replay.rs",
                &["fsl-replay/src/model_check/finite_hyper_replay.rs"],
            ),
            (
                "certificate/product.rs",
                &["fsl-replay/src/certificate/product.rs"],
            ),
        ],
    ),
    row(
        "O-6",
        Rule::ProducerObject,
        &[("contract_check.rs", &["fsl-replay/src/contract_check.rs"])],
    ),
    row(
        "O-7",
        Rule::ProducerObject,
        &[
            (
                "probability_certificate/document.rs",
                &["fsl-replay/src/probability_certificate/document.rs"],
            ),
            (
                "probability_certificate/document/mdp.rs",
                &["fsl-replay/src/probability_certificate/document/mdp.rs"],
            ),
            (
                "probability_certificate/document/long_run.rs",
                &["fsl-replay/src/probability_certificate/document/long_run.rs"],
            ),
            (
                "probability_certificate/witness.rs",
                &["fsl-replay/src/probability_certificate/witness.rs"],
            ),
        ],
    ),
    row(
        "O-8",
        Rule::ProducerObject,
        &[(
            "certificate/zone_certificate.rs",
            &["fsl-replay/src/certificate/zone_certificate.rs"],
        )],
    ),
    row(
        "O-9",
        Rule::ProducerObject,
        &[
            (
                "certificate/closure.rs",
                &["fsl-replay/src/certificate/closure.rs"],
            ),
            (
                "certificate/product.rs",
                &["fsl-replay/src/certificate/product.rs"],
            ),
            (
                "certificate/simulation.rs",
                &["fsl-replay/src/certificate/simulation.rs"],
            ),
            ("certificate/smt.rs", &["fsl-replay/src/certificate/smt.rs"]),
            (
                "certificate/state_graph.rs",
                &["fsl-replay/src/certificate/state_graph.rs"],
            ),
            (
                "certificate/zone_certificate.rs",
                &["fsl-replay/src/certificate/zone_certificate.rs"],
            ),
        ],
    ),
];

const ROUTES: &[&str] = &[
    "R-1", "R-2", "R-3", "R-4", "R-5", "R-6", "R-7", "R-8", "R-9", "R-10", "R-11", "R-12", "R-13",
    "R-14", "R-15", "R-16", "R-17", "R-18", "R-19", "R-20", "R-21", "R-22", "R-23", "R-24", "R-25",
    "R-26", "R-27", "R-28", "R-29", "R-30", "R-31", "R-32", "R-33", "R-34", "R-35", "R-36", "R-37",
    "R-38", "R-39", "R-40", "R-41", "R-42", "R-43", "R-44", "R-45", "R-46", "R-47", "R-48", "R-49",
    "R-50", "R-51", "R-52", "R-53", "R-54", "R-55", "R-56", "R-57", "R-58", "R-59", "R-60", "R-61",
    "R-62",
];

/// The check's data for this repository.
pub(crate) const FSL: Policy = Policy {
    classes: CLASSES,
    base: BASE,
    exceptions: EXCEPTIONS,
    routes: ROUTES,
    producer_types: &[
        "ModelSystem",
        "TimedSystem",
        "ProtocolSystem",
        "TaskProductSystem",
        "AbstractProtocolSystem",
        "RefinementSystems",
        "FiniteTracePopulation",
        "ModelLetters",
        "TemporalLetters",
        "HyperLetters",
        "TupleLetters",
        "StepMatch",
        "ModelEvaluator",
        "BoundedTranslation",
        "SafetyTranslation",
        "PropertyAutomaton",
        "Monitor",
        "QualifiedBoundedMonitor",
        "MeasurePlan",
        "HyperProduct",
        "ProjectedProduct",
        "WitnessProduct",
        "CheckedStepMatch",
        "CheckedHyperLetters",
        "FiniteQuantifier",
    ],
    check_entries: lists::CHECK_ENTRIES,
    codec: lists::CODEC,
    codec_classes: &["C-1", "C-2", "C-3", "C-4"],
    entries: entries::ENTRIES,
};
