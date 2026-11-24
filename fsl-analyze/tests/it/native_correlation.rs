// SPDX-License-Identifier: AGPL-3.0-or-later
//! SC Send/Receive/Attempt correlation over one object and immutable relationship extents.

use std::collections::BTreeMap;

use filament_contract_model::{CheckedPackageDispatchResult, CheckedPackageReadLimits};
use fsl_eval::model_subject::ProtocolSubject;
use fsl_eval::simulation::model::{ModelSystem, PackageModelInput};
use fsl_eval::simulation::protocol::{
    MemorySelection, PackageProgramRefusal, PackageProtocolProgram, ProtocolSystem,
    ProtocolTransition, ResolvedMemory, Sc, StepKind,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use fsl_foundation::MemoryModelName;
use fsl_package::{
    AnalysisView, BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::DocumentRef;
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::fill::{self, Documents, Reached, PACKAGE};

struct Fixture {
    model: Vec<u8>,
    body: String,
    checked: CheckedPackage,
    documents: Documents,
    initial: DocumentRef,
}

fn controls(send: &str, receive: &str) -> String {
    format!(
        "send Tx via ch as (tx: Boolean) {send} {{ tx }};
        receive Rx via ch of Tx as (rx: Boolean) {receive} {{ rx }};"
    )
}

impl Fixture {
    fn new(captures: &str, run: &str, owns: bool) -> Self {
        Self::with_channels(captures, "", run, owns)
    }

    fn with_channels(captures: &str, channels: &str, run: &str, owns: bool) -> Self {
        Self::with_prelude(
            captures,
            channels,
            run,
            owns,
            "",
            filament_semantic_value::checking::CheckMode::Linked,
        )
    }

    fn with_prelude(
        captures: &str,
        channels: &str,
        run: &str,
        owns: bool,
        prelude: &str,
        mode: filament_semantic_value::checking::CheckMode,
    ) -> Self {
        let mut model: Json = serde_json::from_slice(&fill::document()).expect("model JSON");
        model["types"]
            .as_array_mut()
            .expect("types")
            .retain(|node| {
                matches!(
                    node["identity"].as_str(),
                    Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
                )
            });
        model["constructs"]
            .as_array_mut()
            .expect("constructs")
            .retain(|construct| {
                matches!(
                    construct["kind"]["name"].as_str(),
                    Some("value_type" | "object_type" | "population")
                )
            });
        crate::related_by::add_owns_relationship(&mut model);
        let cell = model["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|node| node["identity"] == "ix://test/fill/Cell")
            .expect("Cell");
        cell["operations"]
            .as_array_mut()
            .expect("operations")
            .retain(|operation| matches!(operation["name"].as_str(), Some("touch" | "flag")));
        let mut other = cell["relationships"][0].clone();
        other["identity"] = json!("ix://test/fill/relationship/other");
        cell["relationships"]
            .as_array_mut()
            .expect("relationships")
            .push(other);
        let model = serde_json::to_vec(&model).expect("model bytes");
        let body = format!(
            "{prelude}protocol P using pp over (k: M::Cell) on origin {{
            {captures}
            role w on M::Cell;
            relationship Owns = M::owns;
            relationship owns = M::owns;
            relationship Other = M::other;
            channel ch from w to w carries Boolean ordering unordered
                delivery exactly-once-premise capacity 1 overflow block;
            {channels}
            run sequence Main {{ {run} }}
            finish Done as (done: Boolean) {{ false }};
        }}"
        );
        let checked = fill::package_over_with_mode(model.clone(), "package", "", &body, mode);
        let reference = json!({"population": "ix://test/fill/cells", "key": "c"});
        let snapshot = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "correlation"},
            "observation": "current", "model": {"identity": PACKAGE},
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [
                {"population": "ix://test/fill/cells", "complete": true,
                 "objects": [{"key": "c", "type": "ix://test/fill/Cell",
                    "fields": {"v": {"integer": "0"}, "w": {"integer": "0"}}}]},
                {"population": "ix://test/fill/idles", "complete": true, "objects": []}
            ],
            "relationships": [
                {"relationship": "ix://test/fill/relationship/owns", "complete": true,
                 "instances": if owns {vec![json!({"source": reference, "target": reference})]} else {vec![]}},
                {"relationship": "ix://test/fill/relationship/other", "complete": true, "instances": []}
            ]
        });
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot bytes");
        let initial = fsl_semantics::model::observation::document_reference(&bytes)
            .expect("snapshot identity");
        let documents = Documents {
            packages: package_input([model.as_slice()]).expect("model admits"),
            snapshots: BTreeMap::from([(initial.clone(), bytes)]),
        };
        Self {
            model,
            body,
            checked,
            documents,
            initial,
        }
    }

    fn subject(&self) -> ProtocolSubject<'_> {
        fill::subject(
            &self.checked,
            &self.documents,
            vec![self.initial.clone()],
            "P",
            "c",
        )
    }

    fn with_routes(
        &self,
        carrier: bool,
        inspect: impl FnOnce(&ProtocolSubject<'_>, &ProtocolSubject<'_>),
    ) {
        let emission =
            fsl_package::emit_checked(&self.checked, EmitLimits::fixture()).expect("emission");
        let mut evidence = emission.evidence().clone();
        evidence
            .insert_domain_package_document(PACKAGE, self.model.as_slice())
            .expect("domain evidence");
        let mut wire: Json =
            serde_json::from_slice(emission.package().bytes()).expect("package JSON");
        if !carrier {
            wire.as_object_mut()
                .expect("package")
                .remove("protocol_correlation_occurrences");
        }
        let bytes = serde_json::to_vec(&wire).expect("package bytes");
        let admitted = match filament_contract_model::read_checked_package(
            &bytes,
            CheckedPackageReadLimits::bounded(),
            &evidence,
        ) {
            CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("strict package read: {other:?}"),
        };
        let identities = emission
            .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
            .expect("identities");
        let equality = emission
            .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
            .expect("equality");
        let bare = BarePackage::new(&admitted)
            .with_supply(
                BareSupply {
                    identities: &identities,
                    equality: Some(&equality),
                },
                &mut V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("supply");
        let input = PackageModelInput::of(
            &bare,
            &self.documents.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("native input");
        let source = self.subject();
        let mut native = self.subject();
        native.model.simulation.input = input.input();
        native.memory = ResolvedMemory::resolve_package(
            &bare
                .protocol_execution_named("P")
                .expect("selection")
                .expect("P"),
            &[],
        )
        .expect("native memory");
        assert!(native.model.simulation.source_package().is_none());
        inspect(&source, &native);
    }

    fn graph(&self) -> Vec<Reached> {
        let mut result = None;
        self.with_routes(true, |source, native| {
            let source = fill::concrete_sc_system(source);
            let native = ProtocolSystem::new_package(
                ModelSystem::new(native.model.simulation.clone()).expect("native model"),
                native,
                Sc,
            )
            .expect("native correlation");
            assert_eq!(
                native.offered_reductions(),
                fsl_eval::simulation::OfferedReductions::NONE
            );
            let workers = std::num::NonZeroUsize::new(1).expect("one worker");
            let source = fill::reachable_with_workers(&source, workers).0;
            let native = fill::reachable_with_workers(&native, workers).0;
            let canonical = |graph: &[Reached]| {
                let mut rows: Vec<_> = graph
                    .iter()
                    .map(|state| {
                        let mut steps: Vec<_> = state
                            .steps
                            .iter()
                            .map(|(step, next)| {
                                (
                                    quire_canonical::to_vec(
                                        step,
                                        quire_canonical::Limits::new(u64::MAX),
                                    )
                                    .expect("step"),
                                    graph[*next].key.as_bytes().to_vec(),
                                )
                            })
                            .collect();
                        steps.sort();
                        (state.key.as_bytes().to_vec(), steps)
                    })
                    .collect();
                rows.sort();
                rows
            };
            assert_eq!(
                canonical(&native),
                canonical(&source),
                "all states, binders, queues and successors agree"
            );
            result = Some(native);
        });
        result.expect("both routes inspected")
    }
}

fn step(graph: &[Reached], kind: StepKind) -> (&ProtocolTransition, usize, usize) {
    graph
        .iter()
        .enumerate()
        .flat_map(|(at, state)| {
            state
                .steps
                .iter()
                .map(move |(step, next)| (step, at, *next))
        })
        .find(|(step, _, _)| step.kind() == kind)
        .expect("the required step is enabled")
}

fn has_step(graph: &[Reached], kind: StepKind) -> bool {
    graph
        .iter()
        .flat_map(|state| &state.steps)
        .any(|(step, _)| step.kind() == kind)
}

fn attempt(related: &str, constraint: &str) -> String {
    format!("attempt Tx by w on M::Cell::flag contracts [FlagPost] as (worked: Boolean) {related} {{ {constraint} }};")
}

fn attempt_fixture(captures: &str, run: &str, owns: bool) -> Fixture {
    Fixture::with_prelude(
        captures,
        "",
        run,
        owns,
        "post FlagPost using v on M::Cell::flag { self.v = 1 }\n",
        filament_semantic_value::checking::CheckMode::Linked,
    )
}

// Attempt application keeps the first satisfying result for each poststate.
// Distinct poststates therefore force both Boolean results to be candidates.
const FLAG_RESULT_POSTSTATES: &str = "post FlagPost using v on M::Cell::flag { (self.v = 1 or self.v = 2) and result = (self.v = 1) }\n";

#[trace("FR-731-AC-1", "FR-940-AC-6")]
#[test]
fn sc_attempt_correlation_preserves_candidate_results_poststates_and_both_end_readers() {
    let run = format!(
        "send First via ch as (prior: Boolean) {{ prior }}; check Between using v {{ true }}; {}",
        attempt(
            "related by Owns ((if worked then saved else k), (if prior then saved else k))",
            "true"
        )
    );
    let fixture = Fixture::with_prelude(
        "capture saved: Reference<M::Cell> = k;",
        "",
        &run,
        true,
        FLAG_RESULT_POSTSTATES,
        filament_semantic_value::checking::CheckMode::Linked,
    );
    let graph = fixture.graph();
    let attempts: Vec<_> = graph
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(step, _)| step.kind() == StepKind::Attempt)
        .collect();
    assert!(attempts.iter().any(|(step, _)| matches!(
        step.binder_record(),
        Some(filament_exact::Value::Boolean(true))
    )));
    assert!(attempts.iter().any(|(step, _)| matches!(
        step.binder_record(),
        Some(filament_exact::Value::Boolean(false))
    )));
    for (attempt, next) in attempts {
        let expected = match attempt.binder_record() {
            Some(filament_exact::Value::Boolean(true)) => 1,
            Some(filament_exact::Value::Boolean(false)) => 2,
            other => panic!("attempt must bind its Boolean result: {other:?}"),
        };
        assert_eq!(fill::v(graph[*next].state.model(), "c"), expected);
    }
    fixture.with_routes(true, |source, native| {
        let source = fill::concrete_sc_system(source);
        let native = ProtocolSystem::new_package(
            ModelSystem::new(native.model.simulation.clone()).expect("model"),
            native,
            Sc,
        )
        .expect("native");
        for system in [&source, &native] {
            let reached = fill::reachable(system);
            let (_, at, _) = step(&reached, StepKind::Attempt);
            assert!(system
                .binders(&reached[at].state)
                .expect("binders")
                .iter()
                .any(|(node, _)| node == "Main/First"));
            assert!(!system
                .capture_bytes(&reached[at].state)
                .expect("captures")
                .is_empty());
        }
    });
    assert!(!has_step(
        &attempt_fixture("", &attempt("related by Owns (k, k)", "true"), false).graph(),
        StepKind::Attempt
    ));
    assert!(has_step(
        &attempt_fixture("", &attempt("", "true"), false).graph(),
        StepKind::Attempt
    ));
}

#[trace("FR-731-AC-1", "FR-940-AC-6")]
#[test]
fn sc_attempt_undefined_end_filters_only_its_candidate_and_retains_findings() {
    let fixture = Fixture::with_prelude(
        "",
        "",
        &attempt(
            "related by Owns ((if q(if worked then 1 else 0) >= 0 then k else k), k)",
            "true",
        ),
        true,
        &format!(
            "function q using v(x: Integer): Integer pure {{ 6 mod x }}\n{FLAG_RESULT_POSTSTATES}"
        ),
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    fixture.with_routes(true, |source, native| {
        let source = fill::concrete_sc_system(source);
        let native = ProtocolSystem::new_package(
            ModelSystem::new(native.model.simulation.clone()).expect("model"),
            native,
            Sc,
        )
        .expect("native");
        for system in [&source, &native] {
            let state = system.initial().into_iter().next().expect("initial");
            let (expansion, _) = system.successors_bounded(&state).expect("expansion");
            assert_eq!(expansion.successors.len(), 1);
            assert!(matches!(
                expansion.successors[0].0.binder_record(),
                Some(filament_exact::Value::Boolean(true))
            ));
            assert_eq!(fill::v(expansion.successors[0].1.model(), "c"), 1);
            assert!(
                expansion.findings.iter().any(|finding| matches!(finding,
                    fsl_eval::simulation::model::ModelFinding::ProtocolExpressionUndetermined { node, .. }
                        if node == "Main/Tx"
                )),
                "undefined false-result candidate retains its finding"
            );
        }
    });
}

#[trace("FR-731-AC-1", "FR-940-AC-6", "FSpec-FR-831")]
#[test]
fn sc_send_receive_uses_real_extents_and_preserves_queue_and_binder_values() {
    let fixture = Fixture::new(
        "",
        &controls("related by Owns (k, k)", "related by Owns (k, k)"),
        true,
    );
    let graph = fixture.graph();
    let (send, _, queued) = step(&graph, StepKind::Send);
    let (receive, receiving, drained) = step(&graph, StepKind::Receive);
    assert_eq!(queued, receiving);
    assert!(matches!(
        send.binder_record(),
        Some(filament_exact::Value::Boolean(true))
    ));
    assert!(matches!(
        receive.binder_record(),
        Some(filament_exact::Value::Boolean(true))
    ));
    assert!(graph[queued].key.as_bytes() != graph[drained].key.as_bytes());
    assert!(
        !has_step(&graph, StepKind::Finish),
        "false finish retains a real deadlock witness"
    );
}

#[trace("FR-731-AC-1", "FR-940-AC-6")]
#[test]
fn false_send_and_receive_premises_disable_only_their_candidate() {
    let filtered = Fixture::new("", &controls("related by Owns (k, k)", ""), false).graph();
    assert!(!has_step(&filtered, StepKind::Send));
    let free = Fixture::new("", &controls("", ""), false).graph();
    assert!(has_step(&free, StepKind::Send));
    assert!(has_step(&free, StepKind::Receive));
    let waiting = Fixture::new(
        "",
        &controls("related by Owns (k, k)", "related by Other (k, k)"),
        true,
    )
    .graph();
    let (_, _, queued) = step(&waiting, StepKind::Send);
    assert!(!has_step(&waiting, StepKind::Receive));
    assert!(
        waiting[queued].steps.is_empty(),
        "the queued message is retained while Rx cannot take it"
    );
    let queued: Json = serde_json::from_slice(waiting[queued].key.as_bytes()).expect("state JSON");
    let entries = queued["queues"]["entries"].as_array().expect("queue map");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["key"]["channel"], "P::ch");
    assert_eq!(
        entries[0]["value"]["elements"]
            .as_array()
            .expect("queued entries")
            .len(),
        1
    );
    // The complete-value helper admits modulo; Kernel leaves its nonzero
    // obligation to runtime. The end calls it without widening the protocol profile.
    let undefined = Fixture::with_prelude(
        "",
        "",
        &controls(
            "related by Owns ((if q(if tx then 0 else 1) >= 0 then k else k), k)",
            "",
        ),
        true,
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    undefined.with_routes(true, |source, native| {
        let source = fill::concrete_sc_system(source);
        let native = ProtocolSystem::new_package(
            ModelSystem::new(native.model.simulation.clone()).expect("model"),
            native,
            Sc,
        )
        .expect("native undefined end");
        for system in [&source, &native] {
            let state = system
                .initial()
                .into_iter()
                .next()
                .expect("one initial state");
            let (expansion, _) = system.successors_bounded(&state).expect("expansion");
            assert!(expansion.successors.is_empty());
            assert!(
                !expansion.findings.is_empty(),
                "undefined end retains its finding"
            );
        }
    });
}

#[trace("FR-731-AC-1", "FR-940-AC-6")]
#[test]
fn correlation_only_capture_and_prior_binder_reads_remain_live_after_an_earlier_step() {
    let run = format!(
        "send First via spare as (prior: Boolean) {{ prior }}; check Between using v {{ true }}; {}",
        controls("related by Owns ((if prior then saved else k), saved)", "related by Owns (saved, (if prior then saved else k))")
    );
    let fixture = Fixture::with_channels("capture saved: Reference<M::Cell> = k;", "channel spare from w to w carries Boolean ordering unordered delivery exactly-once-premise capacity 1 overflow block;", &run, true);
    let graph = fixture.graph();
    let prior = graph
        .iter()
        .flat_map(|state| &state.steps)
        .find(|(step, _)| step.node() == Some("Main/First"))
        .expect("prior Boolean send");
    assert!(matches!(
        prior.0.binder_record(),
        Some(filament_exact::Value::Boolean(true))
    ));
    let tx = graph
        .iter()
        .flat_map(|state| &state.steps)
        .find(|(step, _)| step.node() == Some("Main/Tx"))
        .expect("Tx reads the preserved capture");
    assert!(matches!(
        tx.0.binder_record(),
        Some(filament_exact::Value::Boolean(true))
    ));
    assert!(has_step(&graph, StepKind::Receive));
    fixture.with_routes(true, |source, native| {
        let source = fill::concrete_sc_system(source);
        let native = ProtocolSystem::new_package(ModelSystem::new(native.model.simulation.clone()).expect("model"), native, Sc).expect("native");
        for system in [&source, &native] {
            let reached = fill::reachable(system);
            let (tx, at, _) = reached.iter().enumerate().flat_map(|(at, state)| state.steps.iter().map(move |(step, next)| (step, at, next)))
                .find(|(step, _, _)| step.node() == Some("Main/Tx")).expect("Tx");
            assert!(system.binders(&reached[at].state).expect("live binders").iter().any(|(node, _)| node == "Main/First"));
            let footprint = system.footprint(&reached[at].state, tx).expect("footprint");
            assert!(footprint.reads.iter().any(|location| matches!(location,
                fsl_eval::simulation::Location::Protocol(fsl_eval::simulation::protocol::ProtocolLocation::Binder { node, .. }) if node == "Main/First"
            )), "correlation-only prior binder is an enabling read: {footprint:?}");
            assert!(!system.capture_bytes(&reached[at].state).expect("captures").is_empty());
        }
    });
}

#[trace("FR-940-AC-6", "FSpec-FR-831")]
#[test]
fn missing_authored_carrier_is_typed_unavailable_but_empty_legacy_group_executes() {
    let correlated_attempt = attempt_fixture("", &attempt("related by Owns (k, k)", "true"), true);
    correlated_attempt.with_routes(false, |_, native| {
        assert!(matches!(PackageProtocolProgram::new(&native.model.simulation.input, "P", &[]), Err(PackageProgramRefusal::CorrelationProvenance(cause)) if cause.as_str() == "unavailable_correlation_provenance"));
    });
    attempt_fixture("", &attempt("", "true"), true).with_routes(false, |_, native| {
        let system = ProtocolSystem::new_package(
            ModelSystem::new(native.model.simulation.clone()).expect("model"),
            native,
            Sc,
        )
        .expect("empty legacy attempt");
        assert!(has_step(&fill::reachable(&system), StepKind::Attempt));
    });
    let correlated = Fixture::new("", &controls("related by Owns (k, k)", ""), true);
    correlated.with_routes(false, |_, native| {
        let result = PackageProtocolProgram::with_memory(&native.model.simulation.input,
            match native.model.simulation.input.protocol_definition("P").expect("P") {
                fsl_eval::simulation::model::ProtocolDefinition::Package(form) => *form,
                _ => panic!("native route"),
            }, native.memory.clone());
        assert!(matches!(result, Err(PackageProgramRefusal::CorrelationProvenance(cause)) if cause.as_str() == "unavailable_correlation_provenance"));
    });
    let free = Fixture::new("", &controls("", ""), true);
    free.with_routes(false, |_, native| {
        let system = ProtocolSystem::new_package(
            ModelSystem::new(native.model.simulation.clone()).expect("model"),
            native,
            Sc,
        )
        .expect("empty legacy group");
        assert!(has_step(&fill::reachable(&system), StepKind::Receive));
    });
    let emission =
        fsl_package::emit_checked(&correlated.checked, EmitLimits::fixture()).expect("emission");
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(PACKAGE, correlated.model.as_slice())
        .expect("evidence");
    let mut malformed: Json = serde_json::from_slice(emission.package().bytes()).expect("JSON");
    malformed["protocol_correlation_occurrences"][0]["canonical_index"] = json!(999);
    assert!(
        !matches!(
            filament_contract_model::read_checked_package(
                &serde_json::to_vec(&malformed).expect("bytes"),
                CheckedPackageReadLimits::bounded(),
                &evidence,
            ),
            CheckedPackageDispatchResult::AdmittedV2(_)
        ),
        "malformed carrier remains a strict reader refusal"
    );
}

#[trace("FR-940-AC-6", "FR-731-AC-1")]
#[test]
fn correlated_recommend_remains_named_unsupported() {
    let fixture = Fixture::new("", "recommend Work by w on M::Cell::touch contracts [] as (advice: Boolean) related by Other (k, k) { true };", true);
    fixture.with_routes(true, |_, native| {
        assert!(matches!(
            PackageProtocolProgram::new(&native.model.simulation.input, "P", &[]),
            Err(PackageProgramRefusal::Correlation { .. })
        ));
    });
}

#[trace("FR-940-AC-6", "FR-731-AC-1")]
#[test]
fn weak_package_profile_refuses_correlation_outside_parallel_and_sc_override_executes() {
    for control in [controls("related by Owns (k, k)", ""), "attempt Tx by w on M::Cell::touch contracts [] as (worked: Boolean) related by Owns (k, k) { true };".to_owned()] {
    let run = format!("{control} parallel Background memory sc {{ branch idle sequence Idle {{ check OK using v {{ true }}; }} branch other sequence Other {{ check AlsoOK using v {{ true }}; }} }} join all [idle, other];");
    let fixture = Fixture::new("", &run, true);
    fixture.with_routes(true, |_, native| {
        let definition = native
            .model
            .simulation
            .input
            .protocol_definition("P")
            .expect("P");
        let fsl_eval::simulation::model::ProtocolDefinition::Package(form) = definition else {
            panic!("native")
        };
        for model in [MemoryModelName::Tso, MemoryModelName::Ra] {
            let selection = [MemorySelection {
                parallel: "P::Main::Background".to_owned(),
                model,
            }];
            assert!(matches!(
                PackageProtocolProgram::new(&native.model.simulation.input, "P", &selection),
                Err(PackageProgramRefusal::Correlation { .. })
            ));
        }
        let sc = ResolvedMemory::resolve_package(
            &form,
            &[MemorySelection {
                parallel: "P::Main::Background".to_owned(),
                model: MemoryModelName::Sc,
            }],
        )
        .expect("SC override");
        assert!(
            PackageProtocolProgram::with_memory(&native.model.simulation.input, *form, sc).is_ok()
        );
    });
    }
}

fn witness(fixture: &Fixture) -> fsl_replay::TemporalCounterexample<ProtocolTransition> {
    let outcome = fsl_analyze::model_check::check_protocol(
        fsl_analyze::model_check::ProtocolCheckRequest {
            workers: 1,
            temporal_clock_selections: &[],
            subject: fixture.subject(),
            item: fsl_replay::ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: fsl_replay::ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &filament_exact::Cancel::new(),
    )
    .expect("bounded check");
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("false finish has a real deadlock");
    };
    *counterexample
}

#[derive(Clone, Copy)]
enum ReplayExpectation<'a> {
    Reproduced,
    Related(&'a str),
    NotEnabled,
}

fn replay_both(
    fixture: &Fixture,
    payload: fsl_replay::TemporalCounterexample<ProtocolTransition>,
    expected: ReplayExpectation<'_>,
) {
    use fsl_replay::{ModelTraceRefusal, ModelTraceSettlement, ReplayLimits, ReplayRefusal};
    let unit = fill::unit_text_with("", &fixture.body);
    let wire = crate::model_settle::request(
        &fixture.checked,
        &[],
        &fixture.model,
        fixture.documents.snapshots.iter(),
        unit.as_bytes(),
    );
    let (node, occurrence) = crate::model_settle::protocol_anchor(&fixture.checked, "P");
    let envelope = crate::model_settle::envelope_at(payload, node, occurrence);
    let subject = fixture.subject();
    let trace = fsl_replay::ProtocolTraceSubject {
        model: fsl_replay::ModelTraceSubject {
            timed: None,
            stutter_closure: false,
            initial: subject.model.simulation.initial.clone(),
            universes: subject.model.simulation.universes.clone(),
            expansion: subject.model.simulation.expansion,
        },
        protocol: subject.protocol.clone(),
        over: subject.over.clone(),
        roles: subject.roles.clone(),
        capacities: Vec::new(),
        memory: subject.memory.clone(),
        max_store_buffer: None,
        max_messages: None,
        max_live_instances: None,
    };
    let letters = crate::counter::Letters { atom: |_| None };
    let check = |result: Result<fsl_replay::ModelTraceReplayResult, ReplayRefusal>| match (
        expected, result,
    ) {
        (ReplayExpectation::Reproduced, Ok(result)) => assert!(matches!(
            result.settlement,
            ModelTraceSettlement::Reproduced { .. }
        )),
        (ReplayExpectation::Related(alias), Err(ReplayRefusal::ModelTrace(refusal))) => {
            assert!(
                matches!(&*refusal, ModelTraceRefusal::ProtocolRelatedBy { step: 0, node, relationship }
                    if node == "Main/Tx" && relationship == alias),
                "{refusal:?}"
            );
        }
        (ReplayExpectation::NotEnabled, Err(ReplayRefusal::ModelTrace(refusal))) => {
            assert!(
                matches!(
                    &*refusal,
                    ModelTraceRefusal::ProtocolNotEnabled { step: 0, .. }
                ),
                "{refusal:?}"
            );
        }
        (_, other) => panic!("replay result: {other:?}"),
    };
    check(fsl_replay::replay_protocol_trace(
        wire.clone(),
        &trace,
        &envelope,
        &letters,
        ReplayLimits::default(),
    ));
    let emission =
        fsl_package::emit_checked(&fixture.checked, EmitLimits::fixture()).expect("emission");
    let admitted = crate::protocol_projection::admitted(&fixture.checked, PACKAGE, &fixture.model);
    let identities = emission
        .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("identity rows")
        .to_bytes(&admitted)
        .expect("identity bytes");
    let equality = emission
        .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("equality rows")
        .to_bytes(&admitted)
        .expect("equality bytes");
    let mut native = wire;
    native.byte_provision.clear();
    check(fsl_replay::replay_protocol_trace_from_package(
        native,
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &identities,
            equality: Some(&equality),
        },
        &trace,
        &envelope,
        &letters,
        ReplayLimits::default(),
    ));
}

#[trace("FR-731-AC-2", "FR-731-AC-3", "FR-940-AC-6", "FSpec-FR-831")]
#[test]
fn sc_attempt_replay_preserves_authored_alias_order_and_failure_precedence() {
    let run = attempt(
        "related by owns (k, k) related by Owns (k, k) related by owns (k, k)",
        "true",
    );
    let passing = attempt_fixture("", &run, true);
    let valid = witness(&passing);
    assert_eq!(valid.prefix[0].transition.kind(), StepKind::Attempt);
    replay_both(&passing, valid.clone(), ReplayExpectation::Reproduced);
    for (run, owns, expected) in [
        (run.clone(), false, ReplayExpectation::Related("owns")),
        (
            attempt(
                "related by owns (k, k) related by Other (k, k) related by owns (k, k)",
                "true",
            ),
            true,
            ReplayExpectation::Related("Other"),
        ),
        (
            run.replace("{ true }", "{ false }"),
            false,
            ReplayExpectation::NotEnabled,
        ),
        (
            format!("check Stop using v {{ false }}; {run}"),
            false,
            ReplayExpectation::NotEnabled,
        ),
    ] {
        let fixture = attempt_fixture("", &run, owns);
        let mut forged = witness(&fixture);
        assert!(forged.prefix.is_empty());
        forged.prefix.push(valid.prefix[0].clone());
        replay_both(&fixture, forged, expected);
    }
}

#[trace("FR-731-AC-1", "FR-940-AC-6")]
#[test]
fn sc_attempt_package_analysis_executes_after_source_removal() {
    use crate::analyze_cli_support::{analyze, label, package_selection, request_over, Workspace};
    let fixture = attempt_fixture("", &attempt("related by Owns (k, k)", "true"), true);
    let ws = Workspace::new("attempt-correlation-source-absent");
    let request = request_over(
        &ws,
        &fill::unit_text_with("", &fixture.body),
        &fixture.model,
        &fixture
            .documents
            .snapshots
            .values()
            .cloned()
            .collect::<Vec<_>>(),
        json!({"items": [{"claim": "P", "kind": "temporal-satisfaction", "temporal": {
            "item_kind": "deadlock-freedom", "subject": {"protocol": {
                "protocol": "P", "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
                "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}
            }}
        }}]}),
    );
    let source = analyze(&request);
    assert_eq!(label(&source.item(0)), "refuted");
    let document: Json =
        serde_json::from_slice(&std::fs::read(&request).expect("request")).expect("JSON");
    let (members, _) = package_selection(&ws, &document["request"]);
    let request = ws.write(
        "package-analyze.json",
        &serde_json::to_vec(&json!({"format": "native-analyze/1", "request": members}))
            .expect("request bytes"),
    );
    std::fs::remove_file(ws.path("program.fsl")).expect("source removed before package read");
    let sources = ws.path("handoff/sources");
    if sources.exists() {
        std::fs::remove_dir_all(sources).expect("source copies removed");
    }
    let native = analyze(&request);
    assert_eq!(native.exit, source.exit);
    assert_eq!(label(&native.item(0)), "refuted");
}

#[trace("FR-731-AC-2", "FR-731-AC-3", "FR-940-AC-6", "FSpec-FR-831")]
#[test]
fn replay_reports_first_authored_alias_including_repeated_canonical_premises() {
    let run = controls(
        "related by owns (k, k) related by Owns (k, k) related by owns (k, k)",
        "related by Owns (k, k)",
    );
    let passing = Fixture::new("", &run, true);
    passing.with_routes(true, |_, native| {
        let admitted = native
            .model
            .simulation
            .input
            .admitted_package()
            .expect("admitted");
        let fsl_eval::simulation::model::ProtocolDefinition::Package(form) = native
            .model
            .simulation
            .input
            .protocol_definition("P")
            .expect("P")
        else {
            panic!("native")
        };
        let tx = form
            .controls
            .iter()
            .find(|control| control.path.last().is_some_and(|name| name == "Tx"))
            .expect("Tx");
        let clauses = admitted
            .protocol_correlation_clauses(&tx.node.node_id)
            .expect("authored rows");
        assert_eq!(
            clauses
                .iter()
                .map(|row| (row.alias.as_ref(), row.source_ordinal))
                .collect::<Vec<_>>(),
            vec![("owns", 0), ("Owns", 1), ("owns", 2)]
        );
        assert!(clauses
            .iter()
            .all(|row| row.canonical_index == clauses[0].canonical_index));
    });
    let valid = witness(&passing);
    assert_eq!(valid.prefix[0].transition.kind(), StepKind::Send);
    assert!(valid
        .prefix
        .iter()
        .any(|step| step.transition.kind() == StepKind::Receive));
    replay_both(&passing, valid.clone(), ReplayExpectation::Reproduced);
    let failing = Fixture::new("", &run, false);
    let mut forged = witness(&failing);
    assert!(forged.prefix.is_empty());
    replay_both(&failing, forged.clone(), ReplayExpectation::Reproduced);
    forged.prefix.push(valid.prefix[0].clone());
    replay_both(&failing, forged, ReplayExpectation::Related("owns"));
    for blocked in [
        run.replace("{ tx }", "{ false }"),
        format!("check Stop using v {{ false }}; {run}"),
    ] {
        let fixture = Fixture::new("", &blocked, false);
        let mut forged = witness(&fixture);
        assert!(forged.prefix.is_empty());
        forged.prefix.push(valid.prefix[0].clone());
        replay_both(&fixture, forged, ReplayExpectation::NotEnabled);
    }
    let mixed = controls(
        "related by owns (k, k) related by Other (k, k) related by owns (k, k)",
        "related by Owns (k, k)",
    );
    let fixture = Fixture::new("", &mixed, true);
    fixture.with_routes(true, |_, native| {
        let admitted = native
            .model
            .simulation
            .input
            .admitted_package()
            .expect("admitted");
        let fsl_eval::simulation::model::ProtocolDefinition::Package(form) = native
            .model
            .simulation
            .input
            .protocol_definition("P")
            .expect("P")
        else {
            panic!("native")
        };
        let tx = form
            .controls
            .iter()
            .find(|control| control.path.last().is_some_and(|name| name == "Tx"))
            .expect("Tx");
        let clauses = admitted
            .protocol_correlation_clauses(&tx.node.node_id)
            .expect("authored rows");
        assert_eq!(clauses.len(), 3);
        assert_eq!(clauses[0].alias.as_ref(), "owns");
        assert_eq!(clauses[1].alias.as_ref(), "Other");
        assert_ne!(clauses[0].canonical_index, clauses[1].canonical_index);
        assert_eq!(clauses[0].canonical_index, clauses[2].canonical_index);
    });
    assert!(
        !has_step(&fixture.graph(), StepKind::Send),
        "the later false premise disables Tx after the true Owns premise"
    );
    let mut forged = witness(&fixture);
    assert!(forged.prefix.is_empty());
    forged.prefix.push(valid.prefix[0].clone());
    replay_both(&fixture, forged, ReplayExpectation::Related("Other"));
    let swapped = controls(
        "related by Other (k, k) related by owns (k, k) related by owns (k, k)",
        "related by Owns (k, k)",
    );
    let swapped = Fixture::new("", &swapped, true);
    assert!(!has_step(&swapped.graph(), StepKind::Send));
}
