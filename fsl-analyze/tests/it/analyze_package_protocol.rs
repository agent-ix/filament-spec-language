// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940-AC-6: protocol litmus and memory-model items settle equally from a
//! source-checked package and its admitted handoff bytes.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, package_selection, Run, Workspace,
};
use crate::litmus::{sb_run, sb_unit};

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {
        "protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding, "c": binding},
    }})
}

fn request_extra(items: Value) -> Value {
    json!({
        "subject": subject(),
        "memory": [{"parallel": "SB::Both", "model": "tso"}],
        "items": items,
    })
}

fn request_without_memory(items: Value) -> Value {
    json!({
        "subject": subject(),
        "items": items,
    })
}

fn package_request(
    ws: &Workspace,
    request_path: &std::path::Path,
) -> (std::path::PathBuf, Vec<u8>) {
    let document: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("the source request exists"))
            .expect("the source request is JSON");
    let (members, bytes) = package_selection(ws, &document["request"]);
    let package_document = json!({"format": "native-analyze/1", "request": members});
    (
        ws.write(
            "analyze-package.json",
            package_document.to_string().as_bytes(),
        ),
        bytes,
    )
}

fn package_run(ws: &Workspace, request_path: &std::path::Path) -> Run {
    analyze(&package_request(ws, request_path).0)
}

fn repeat_attempt(name: &str, binder: &str, constraint: &str) -> String {
    crate::fill::attempt(name, "touch", "", binder, constraint).replace(" by w ", " by Machining ")
}

fn workflow_roles_request(
    ws: &Workspace,
    maximum: u64,
    lifetime: &str,
    empty_universe: bool,
    controls: &str,
    finish: &str,
) -> std::path::PathBuf {
    let path = channel_request(
        ws,
        finite_channel_model(),
        if empty_universe {
            "ordering unordered delivery exactly-once-premise capacity 2 overflow reject"
        } else {
            "ordering unordered delivery unknown capacity 2 overflow reject"
        },
        controls,
        finish,
    );
    let (object, population) = if empty_universe {
        ("Idle", "idles")
    } else {
        ("Cell", "cells")
    };
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("the role unit");
    let unit = unit
        .replace(
            "role Machining on M::Cell;",
            &format!(
                "role Worker each M::{object} from {population} max {maximum} lifetime {lifetime};"
            ),
        )
        .replace("to Machining", "to Worker");
    ws.write("program.fsl", unit.as_bytes());
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
    request["request"]["items"][0]["temporal"]["subject"]["protocol"]["roles"]
        .as_object_mut()
        .expect("role bindings")
        .remove("Machining");
    ws.write("analyze.json", request.to_string().as_bytes());
    ws.write(
        "snapshot-0.json",
        &channel_snapshots(&[("c", 0), ("d", 0)])[0],
    );
    path
}

/// Explore both routes from the same external snapshot and universe. Only the
/// source input uses the checked graph; the other input has admitted supply.
fn workflow_role_graph(ws: &Workspace, path: &std::path::Path) -> Vec<crate::fill::Reached> {
    package_protocol_graph(
        ws,
        path,
        fsl_eval::simulation::protocol::Sc,
        &["Planning"],
        false,
        false,
        |_, _| {},
    )
}

fn package_protocol_graph<M: crate::fill::Explored + Clone>(
    ws: &Workspace,
    path: &std::path::Path,
    memory: M,
    roles: &[&str],
    source_absent: bool,
    cycle_keys: bool,
    inspect: impl FnOnce(
        &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
        &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
    ),
) -> Vec<crate::fill::Reached<M>> {
    package_protocol_graph_with_snapshot(
        ws,
        path,
        memory,
        PackageProtocolGraphOptions {
            roles,
            source_absent,
            cycle_keys,
            snapshot: None,
        },
        inspect,
    )
}

struct PackageProtocolGraphOptions<'a> {
    roles: &'a [&'a str],
    source_absent: bool,
    cycle_keys: bool,
    snapshot: Option<Vec<u8>>,
}

fn package_protocol_graph_with_snapshot<M: crate::fill::Explored + Clone>(
    ws: &Workspace,
    path: &std::path::Path,
    memory: M,
    options: PackageProtocolGraphOptions<'_>,
    inspect: impl FnOnce(
        &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
        &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
    ),
) -> Vec<crate::fill::Reached<M>> {
    use fsl_eval::model_subject::{ProtocolSubject, RoleBinding};
    use fsl_eval::simulation::model::{ModelSystem, PackageModelInput};
    use fsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory};
    use fsl_package::{AnalysisView, BarePackage, BareSupply, EmitLimits, V2ReadLimits};
    use fsl_semantics::model::accounting::ModelNormalizationLimits;

    let PackageProtocolGraphOptions {
        roles,
        source_absent,
        cycle_keys,
        snapshot,
    } = options;

    let _ = package_request(ws, path);
    let compiled =
        fsl_command::checked_handoff::compile(&ws.path(crate::analyze_cli_support::HANDOFF))
            .expect("the role fixture compiles");
    let mut documents = crate::fill::Documents::new();
    documents.packages = compiled.packages().clone();
    let cells: &[&str] = if snapshot.is_some() {
        &["c"]
    } else {
        &["c", "d"]
    };
    let start = if let Some(bytes) = snapshot {
        let start = fsl_semantics::model::observation::document_reference(&bytes)
            .expect("the fixture snapshot names itself");
        documents.snapshots.insert(start.clone(), bytes);
        start
    } else {
        documents.start("s0", &[("c", 0), ("d", 0)])
    };
    let request: Value =
        serde_json::from_slice(&std::fs::read(path).expect("request")).expect("request JSON");
    let capacities = request["request"]["items"][0]["temporal"]["subject"]["protocol"]
        ["capacities"]
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .map(|(name, value)| fsl_eval::model_subject::CapacityBinding {
            name: name.clone(),
            value: value
                .as_str()
                .expect("capacity text")
                .parse()
                .expect("natural capacity"),
        })
        .collect();
    let source = ProtocolSubject {
        model: crate::fill::model_subject(compiled.checked(), &documents, vec![start], cells),
        protocol: "Flow".to_owned(),
        over: crate::fill::instance("c"),
        roles: roles
            .iter()
            .map(|role| RoleBinding {
                role: (*role).to_owned(),
                model: crate::fill::PACKAGE.to_owned(),
            })
            .collect(),
        capacities,
        memory: crate::fill::source_memory(compiled.checked(), "Flow"),
    };
    let emission =
        fsl_package::emit_checked(compiled.checked(), EmitLimits::fixture()).expect("emission");
    let admitted = compiled.admitted();
    let identities = emission
        .identity_rows(admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("identities");
    let equality = emission
        .equality_witness(admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("equality");
    let bare = BarePackage::new(admitted)
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
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("package model");
    let mut model = crate::fill::model_subject(
        compiled.checked(),
        &documents,
        source.model.simulation.initial.clone(),
        cells,
    );
    model.simulation.input = input.input();
    assert!(model.simulation.source_package().is_none());
    let package = ProtocolSubject {
        model,
        protocol: source.protocol.clone(),
        over: source.over.clone(),
        roles: source.roles.clone(),
        capacities: source.capacities.clone(),
        memory: ResolvedMemory::resolve_package(
            &bare
                .protocol_execution_named("Flow")
                .expect("protocol selection")
                .expect("Flow"),
            &[],
        )
        .expect("package memory"),
    };
    let mut source_system = ProtocolSystem::new(
        ModelSystem::new(source.model.simulation.clone()).expect("source model"),
        &source,
        memory.clone(),
    )
    .expect("source roles");
    if cycle_keys {
        source_system = source_system.with_cycle_keys(std::collections::BTreeSet::new());
    }
    let workers = std::num::NonZeroUsize::new(1).expect("one worker");
    let source = crate::fill::reachable_with_workers(&source_system, workers).0;
    if source_absent {
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("source removed before package execution");
    }
    let mut package_system = ProtocolSystem::new_package(
        ModelSystem::new(package.model.simulation.clone()).expect("package model"),
        &package,
        memory,
    )
    .expect("package roles");
    if cycle_keys {
        package_system = package_system.with_cycle_keys(std::collections::BTreeSet::new());
    }
    inspect(&source_system, &package_system);
    let package = crate::fill::reachable_with_workers(&package_system, workers).0;
    let canonical = |states: &[crate::fill::Reached<M>]| {
        let mut rows: Vec<_> = states
            .iter()
            .map(|state| {
                let mut steps: Vec<_> = state
                    .steps
                    .iter()
                    .map(|(step, next)| {
                        (
                            quire_canonical::to_vec(step, quire_canonical::Limits::new(u64::MAX))
                                .expect("transition identity"),
                            states[*next].key.as_bytes().to_vec(),
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
    assert_eq!(canonical(&package), canonical(&source));
    source
}

fn live_workflow_roles(state: &crate::fill::Reached) -> usize {
    let key: Value = serde_json::from_slice(state.key.as_bytes()).expect("state key JSON");
    key["roles"]["entries"]
        .as_array()
        .expect("role map")
        .iter()
        .filter(|entry| entry["key"]["role"] == "Worker")
        .flat_map(|entry| entry["value"]["entries"].as_array().expect("instance map"))
        .filter(|entry| entry["value"]["value"] == "live")
        .count()
}

fn workflow_role_parity(ws: &Workspace, path: &std::path::Path, expected: &str) -> Value {
    // Graph comparison already wrote this authentic handoff and its request.
    let package_path = ws.path("analyze-package.json");
    assert!(
        package_path.is_file(),
        "the graph prepared the package request"
    );
    channel_send_parity_using(ws, path, expected, || package_path)
}

/// The complete replicated-role example has an empty run and an unused channel;
/// its two spawn orders meet at the same state, and finish tears down both actors.
#[trace(
    "TC-2958",
    "FR-940-AC-6",
    "FR-531",
    "FR-208-AC-2",
    "FR-208-AC-3",
    "FR-850"
)]
#[test]
fn package_workflow_roles_runs_the_complete_body_and_tears_down() {
    use fsl_eval::simulation::protocol::StepKind;
    let ws = Workspace::new("package-workflow-role-body");
    let path = workflow_roles_request(&ws, 2, "workflow", false, "", "closing");
    let graph = workflow_role_graph(&ws, &path);
    assert_eq!(graph.iter().map(live_workflow_roles).max(), Some(2));
    let both = graph
        .iter()
        .position(|state| live_workflow_roles(state) == 2)
        .expect("both workers live");
    assert_eq!(
        graph
            .iter()
            .flat_map(|state| &state.steps)
            .filter(|(step, next)| step.kind() == StepKind::Spawn && *next == both)
            .count(),
        2,
        "the two spawn orders converge"
    );
    let finished: Vec<_> = graph
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(step, _)| step.kind() == StepKind::Finish)
        .collect();
    assert!(!finished.is_empty());
    for (_, next) in finished {
        assert_eq!(live_workflow_roles(&graph[*next]), 0);
        assert!(graph[*next].steps.is_empty());
    }
    workflow_role_parity(&ws, &path, "proved");
}

/// The bound changes real reachable actors; owned attempts retain their actor
/// identity in both graphs, and a blocked continuation replays without source.
#[trace("TC-2958", "FR-940-AC-6", "FR-208", "FR-208-AC-2", "FR-206", "FR-850")]
#[test]
fn package_workflow_roles_bounds_instances_and_executes_owned_attempts() {
    use fsl_eval::simulation::protocol::StepKind;
    let controls = repeat_attempt("Work", "worked", "true").replace("by Machining", "by Worker");
    for maximum in [1, 2] {
        let ws = Workspace::new(&format!("package-workflow-role-bound-{maximum}"));
        let path = workflow_roles_request(&ws, maximum, "workflow", false, &controls, "false");
        let graph = workflow_role_graph(&ws, &path);
        assert_eq!(
            graph.iter().map(live_workflow_roles).max(),
            Some(usize::try_from(maximum).expect("small bound"))
        );
        let actors: std::collections::BTreeSet<_> = graph
            .iter()
            .flat_map(|state| &state.steps)
            .filter(|(step, _)| step.kind() == StepKind::Attempt)
            .map(|(step, _)| {
                let actor = step.actor().expect("a replicated attempt has an actor");
                assert_eq!(actor.role, "Worker");
                actor.object.clone()
            })
            .collect();
        assert_eq!(actors.len(), 2, "either cell can execute Work");
        workflow_role_parity(&ws, &path, "refuted");
    }
}

/// An empty selected universe cannot manufacture an actor, and malformed caller
/// role bindings retain the source route's typed refusal before exploration.
#[trace("TC-2958", "FR-940-AC-6", "FR-208-AC-5", "FR-531", "FR-850")]
#[test]
fn package_workflow_roles_empty_universe_and_binding_refusal() {
    let ws = Workspace::new("package-workflow-role-empty");
    let controls = channel_send("Released", "order", "order.n = k.v")
        + "receive Heard via Release of Released as (heard: M::Attempt) { true };";
    let path = workflow_roles_request(&ws, 2, "workflow", true, &controls, "closing");
    let graph = workflow_role_graph(&ws, &path);
    assert!(graph.iter().all(|state| live_workflow_roles(state) == 0));
    assert!(graph
        .iter()
        .flat_map(|state| &state.steps)
        .all(|(step, _)| step.actor().is_none()));
    let item = workflow_role_parity(&ws, &path, "refuted");
    let blocked = item["counterexample"]["blocked"]
        .as_array()
        .expect("blocked worker receive");
    assert_eq!(blocked.len(), 1, "{item}");
    assert_eq!(blocked[0]["cause"]["type"], "no-role-instance", "{item}");
    assert_eq!(blocked[0]["cause"]["role"], "Worker", "{item}");

    let ws = Workspace::new("package-workflow-role-invalid-binding");
    let path = workflow_roles_request(&ws, 2, "workflow", false, "", "closing");
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
    let roles = request["request"]["items"][0]["temporal"]["subject"]["protocol"]["roles"]
        .as_object_mut()
        .expect("bindings");
    let binding = roles.remove("Planning").expect("static role binding");
    roles.insert("Unexpected".to_owned(), binding);
    ws.write("analyze.json", request.to_string().as_bytes());
    let item = repeat_parity(&ws, &path);
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "wrong-role-mapping",
        "{item}"
    );
    assert!(item.get("exploration").is_none(), "{item}");
}

/// A recorded actor must belong to the live role instances at that prefix,
/// even when its transition came from a real branch of an admitted system.
#[trace("TC-2958", "FR-940-AC-6", "FR-208-AC-2", "FR-850")]
#[test]
fn package_workflow_roles_replay_refuses_forged_actors() {
    use fsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
    use fsl_eval::model_subject::RoleBinding;
    use fsl_eval::simulation::model::ExpansionLimits;
    use fsl_eval::simulation::protocol::{ProtocolTransition, StepKind};
    use fsl_foundation::diagnostic::Code;
    use fsl_replay::{
        replay_protocol_trace, replay_protocol_trace_from_package, HandoffSupply, ModelCheckItem,
        ModelCheckLimits, ModelCheckOutcome, ModelTraceRefusal, ModelTraceSettlement,
        ProtocolTraceSubject, ReplayLimits, ReplayRefusal,
    };

    let controls = repeat_attempt("Work", "worked", "true").replace("by Machining", "by Worker");
    let ws = Workspace::new("package-workflow-forged-actor");
    let path = workflow_roles_request(&ws, 1, "workflow", false, &controls, "false");
    let graph = workflow_role_graph(&ws, &path);
    let directory = ws.path(crate::analyze_cli_support::HANDOFF);
    let compiled =
        fsl_command::checked_handoff::compile(&directory).expect("the workflow compiles");
    let mut documents = crate::fill::Documents::new();
    documents.packages = compiled.packages().clone();
    let initial = documents.start("s0", &[("c", 0), ("d", 0)]);
    let mut subject =
        crate::fill::subject(compiled.checked(), &documents, vec![initial], "Flow", "c");
    subject.model.simulation.universes = crate::fill::universe(&["c", "d"]);
    subject.roles = vec![RoleBinding {
        role: "Planning".to_owned(),
        model: crate::fill::PACKAGE.to_owned(),
    }];
    let trace_subject = ProtocolTraceSubject {
        model: fsl_replay::ModelTraceSubject {
            timed: None,
            initial: subject.model.simulation.initial.clone(),
            universes: subject.model.simulation.universes.clone(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            stutter_closure: false,
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
    let outcome = check_protocol(
        ProtocolCheckRequest {
            subject,
            workers: 1,
            temporal_clock_selections: &[],
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &filament_exact::Cancel::new(),
    )
    .expect("the real deadlock checks");
    let ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("Work followed by a false finish must deadlock");
    };
    let valid = *counterexample;
    let at = valid
        .prefix
        .iter()
        .position(|step| step.transition.kind() == StepKind::Attempt)
        .expect("the witness executes Work");
    assert_eq!(at, 1, "the real witness spawns once before Work");
    let work = &valid.prefix[at].transition;
    let actor = work.actor().expect("Work has a spawned actor");
    assert_eq!(
        valid.prefix[0]
            .transition
            .role_instance()
            .expect("spawned worker")
            .1,
        &actor.object
    );
    let without_actor = |transition: &ProtocolTransition| {
        let bytes = quire_canonical::to_vec(transition, quire_canonical::Limits::new(u64::MAX))
            .expect("transition encodes");
        let mut value: Value = serde_json::from_slice(&bytes).expect("transition JSON");
        value
            .as_object_mut()
            .expect("transition members")
            .remove("actor");
        value
    };
    let other = graph
        .iter()
        .flat_map(|state| &state.steps)
        .map(|(step, _)| step)
        .find(|step| {
            step.kind() == StepKind::Attempt
                && step
                    .actor()
                    .is_some_and(|other| other.role == actor.role && other.object != actor.object)
                && without_actor(step) == without_actor(work)
        })
        .expect("the other real spawned worker can take the same Work")
        .clone();

    // The public transition is immutable. A second checked fixture provides
    // a genuine Work with another role; no actor or graph is fabricated.
    let renamed_ws = Workspace::new("package-workflow-other-role");
    let renamed_path =
        workflow_roles_request(&renamed_ws, 1, "workflow", false, &controls, "false");
    let unit = std::fs::read_to_string(renamed_ws.path("program.fsl")).expect("role source");
    renamed_ws.write(
        "program.fsl",
        unit.replace("Worker", "UnknownWorker").as_bytes(),
    );
    let renamed = workflow_role_graph(&renamed_ws, &renamed_path);
    let unknown = renamed
        .iter()
        .flat_map(|state| &state.steps)
        .map(|(step, _)| step)
        .find(|step| {
            step.kind() == StepKind::Attempt
                && step.actor().is_some_and(|other| {
                    other.role == "UnknownWorker" && other.object == actor.object
                })
                && without_actor(step) == without_actor(work)
        })
        .expect("the admitted other role takes the same Work")
        .clone();
    assert_ne!(other.actor(), work.actor());
    assert_ne!(unknown.actor(), work.actor());
    for replacement in [&other, &unknown] {
        assert_eq!(without_actor(replacement), without_actor(work));
        assert_ne!(replacement, work);
    }
    let unit = std::fs::read(ws.path("program.fsl")).expect("source bytes");
    let model = std::fs::read(ws.path("model.json")).expect("model bytes");
    let wire = crate::model_settle_support::request(
        compiled.checked(),
        &[],
        &model,
        documents.snapshots.iter(),
        &unit,
    );
    let (node, occurrence) =
        crate::model_settle_support::protocol_anchor(compiled.checked(), "Flow");
    let envelope = |payload| {
        crate::model_settle_support::envelope_at(payload, node.clone(), occurrence.clone())
    };
    let letters = crate::counter::Letters { atom: |_| None };
    let legitimate = replay_protocol_trace(
        wire.clone(),
        &trace_subject,
        &envelope(valid.clone()),
        &letters,
        ReplayLimits::default(),
    )
    .expect("the real source witness replays");
    assert!(matches!(
        legitimate.settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));
    let forged: Vec<_> = [other, unknown]
        .into_iter()
        .map(|replacement| {
            let mut payload = valid.clone();
            payload.prefix[at].transition = replacement;
            payload
        })
        .collect();
    let refuse = |result: Result<_, ReplayRefusal>| {
        let Err(ReplayRefusal::ModelTrace(refusal)) = result else {
            panic!("a forged actor must refuse");
        };
        assert!(
            matches!(&*refusal, ModelTraceRefusal::ProtocolNotEnabled { step, .. } if *step == at),
            "{refusal:?}"
        );
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::InvalidRuntimeInput, "invalid-value")
        );
        *refusal
    };
    let source_refusals: Vec<_> = forged
        .iter()
        .map(|payload| {
            refuse(replay_protocol_trace(
                wire.clone(),
                &trace_subject,
                &envelope(payload.clone()),
                &letters,
                ReplayLimits::default(),
            ))
        })
        .collect();
    let identities = std::fs::read(directory.join("identities.json")).expect("identities");
    let equality = std::fs::read(directory.join("equality.json")).expect("equality");
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package replay");
    let mut package_wire = wire;
    package_wire.byte_provision.clear();
    assert!(!ws.path("program.fsl").exists());
    assert!(package_wire.byte_provision.is_empty());
    let replay_package = |payload| {
        replay_protocol_trace_from_package(
            package_wire.clone(),
            HandoffSupply {
                dependencies: &[],
                identities: &identities,
                equality: Some(&equality),
            },
            &trace_subject,
            &envelope(payload),
            &letters,
            ReplayLimits::default(),
        )
    };
    assert!(matches!(
        replay_package(valid)
            .expect("the source-absent package witness replays")
            .settlement,
        ModelTraceSettlement::Reproduced { .. }
    ));
    for (payload, source_refusal) in forged.into_iter().zip(source_refusals) {
        assert_eq!(refuse(replay_package(payload)), source_refusal);
    }
}

/// Other declared lifetimes remain an explicit package capability boundary.
#[trace("TC-2958", "FR-940-AC-6", "FR-531", "FR-208")]
#[test]
fn package_workflow_roles_other_lifetimes_remain_named_refusals() {
    for lifetime in ["scope", "until(true)"] {
        let ws = Workspace::new(&format!("package-workflow-role-lifetime-{lifetime}"));
        let controls =
            repeat_attempt("Work", "worked", "true").replace("by Machining", "by Worker");
        let path = workflow_roles_request(&ws, 2, lifetime, false, &controls, "closing");
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "{}",
            source.document()
        );
        assert!(
            matches!(label(&source.item(0)), "proved" | "refuted"),
            "{}",
            source.document()
        );
        let (path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source");
        let package = analyze(&path);
        let item = package.item(0);
        let diagnostic = &item["record"]["diagnostic"];
        assert_eq!(diagnostic["code"], "unsupported_construct", "{item}");
        assert_eq!(diagnostic["cause"], "not-yet-implemented", "{item}");
        assert_eq!(
            diagnostic["detail"]["fact"], "replicated-role-lifetime",
            "{item}"
        );
        assert_eq!(
            diagnostic["detail"]["lifetime"],
            lifetime.split('(').next().expect("lifetime name"),
            "{item}"
        );
        assert!(item.get("exploration").is_none(), "{item}");
    }
}

fn repeat_request(ws: &Workspace, controls: &str, finish: &str) -> std::path::PathBuf {
    channel_request(
        ws,
        finite_channel_model(),
        "ordering unordered delivery exactly-once-premise capacity 4 overflow block",
        controls,
        finish,
    )
}

fn decimal_capture_request(ws: &Workspace, controls: &str, finish: &str) -> std::path::PathBuf {
    channel_capture_request(
        ws,
        controls,
        finish,
        "capture zero: Decimal[0, 0; 0, 0; exact] = decimal(0, 0);",
    )
}

fn channel_capture_request(
    ws: &Workspace,
    controls: &str,
    finish: &str,
    capture: &str,
) -> std::path::PathBuf {
    let path = repeat_request(ws, controls, finish);
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("the Decimal capture unit");
    let unit = unit.replace(
        "role Planning on M::Cell;",
        &format!("{capture} role Planning on M::Cell;"),
    );
    ws.write("program.fsl", unit.as_bytes());
    path
}

fn repeat_parity(ws: &Workspace, path: &std::path::Path) -> Value {
    let source = analyze(path);
    let source_document = source.document();
    assert!(source.item(0).get("record").is_some(), "{source_document}");
    assert!(
        matches!(source_document["last_stage"].as_str(), Some("S6c" | "S8")),
        "{source_document}"
    );
    let (package_path, _) = package_request(ws, path);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove repeat source before package run");
    let package = analyze(&package_path);
    let package_document = package.document();
    for member in ["items", "aggregate", "last_stage", "category"] {
        assert_eq!(
            package_document[member], source_document[member],
            "{member}: {package_document}"
        );
    }
    assert_eq!(package.exit, source.exit, "{package_document}");
    source.item(0)
}

/// A received payload remains live across the repeat's entry and back edge.
#[trace(
    "TC-2958",
    "FR-940-AC-6",
    "FR-206-AC-5",
    "FR-383-AC-6",
    "FR-218",
    "FR-531",
    "FR-390",
    "FR-850"
)]
#[test]
fn package_repeat_reads_the_earlier_receive_on_entry_and_revisit() {
    for (name, guard, expected) in [
        ("false", "seen.n != 0", "refuted"),
        ("true", "seen.n = 0", "proved"),
    ] {
        let ws = Workspace::new(&format!("package-repeat-received-{name}"));
        let controls = channel_send("Handed", "handed", "handed.n = k.v")
            + "receive Inspected via Release of Handed as (seen: M::Attempt) { seen.n = 0 };"
            + &format!(
                "repeat Again by Machining visible (seen.n) while {{ {guard} }} {}",
                repeat_attempt("Remachined", "again", "true")
            );
        let path = channel_request(
            &ws,
            finite_channel_model(),
            "ordering unordered delivery at-least-once capacity 4 overflow block",
            &controls,
            "false",
        );
        let item = repeat_parity(&ws, &path);
        assert_eq!(label(&item), expected, "{item}");
        if name == "false" {
            let prefix = item["counterexample"]["prefix"]
                .as_array()
                .expect("false finish prefix");
            let steps: Vec<_> = prefix
                .iter()
                .map(|step| step["transition"]["step"].as_str().expect("step kind"))
                .collect();
            assert_eq!(steps, ["send", "receive"], "{item}");
            assert_eq!(item["replay"]["reproduced"], true, "{item}");
        }
    }
}

/// A Decimal capture's exact type projects independently of Repeat execution.
#[trace("TC-2958", "FR-531", "FR-390", "FR-940-AC-6")]
#[test]
fn package_decimal_capture_empty_run_preserves_source_result() {
    let ws = Workspace::new("package-decimal-capture-empty-run");
    let item = repeat_parity(&ws, &decimal_capture_request(&ws, "", "closing"));
    assert_eq!(label(&item), "proved", "{item}");
}

/// An enclosing optional Decimal keeps its original unsupported type projection.
#[trace("TC-2958", "FR-531", "FR-390", "FR-940-AC-6")]
#[test]
fn package_decimal_capture_nested_type_preserves_projection_refusal() {
    let ws = Workspace::new("package-decimal-capture-nested-type");
    let path = channel_capture_request(
        &ws,
        "",
        "closing",
        "capture nested: Option<Decimal[0, 0; 0, 0; exact]> = absent;",
    );
    let source = analyze(&path);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let (package_path, _) = package_request(&ws, &path);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before type refusal");
    let package = analyze(&package_path);
    let item = package.item(0);
    assert_eq!(package.exit, 21, "{}", package.document());
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(diagnostic["code"], "unknown_required_feature", "{item}");
    assert_eq!(diagnostic["cause"], "unsupported-feature", "{item}");
    assert_eq!(diagnostic["detail"]["fact"], "parameter-type", "{item}");
    assert_eq!(
        diagnostic["detail"]["reason"], "missing-type-representation",
        "{item}"
    );
    assert!(!diagnostic["detail"]["node"].is_null(), "{item}");
    assert!(!diagnostic["detail"]["locus"].is_null(), "{item}");
    assert!(item.get("exploration").is_none(), "{item}");
    assert!(item.get("counterexample").is_none(), "{item}");
}

/// A bounded repeat executes its body at most max times and then its authored exhaustion.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-5", "FR-383-AC-6")]
#[test]
fn package_repeat_maximum_enters_the_stepful_exhausted_continuation() {
    for maximum in [0, 2] {
        let ws = Workspace::new(&format!("package-repeat-maximum-{maximum}"));
        let controls = format!(
            "repeat Again by Machining visible () max {maximum} while {{ true }} {} exhausted {}",
            repeat_attempt("Remachined", "again", "true"),
            repeat_attempt("Exhausted", "out", "true")
        );
        let item = repeat_parity(&ws, &repeat_request(&ws, &controls, "false"));
        assert_eq!(label(&item), "refuted", "{item}");
        let prefix = item["counterexample"]["prefix"]
            .as_array()
            .expect("exhausted finish prefix");
        let nodes: Vec<_> = prefix
            .iter()
            .map(|step| step["transition"]["node"].as_str().expect("step node"))
            .collect();
        let mut expected = vec!["Main/Again/Remachined"; maximum];
        expected.push("Main/Again/Exhausted");
        assert_eq!(nodes, expected, "{item}");
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
    }
}

/// A body-bound guard has the native post-test mode and executes once when false.
#[trace("TC-2958", "FR-940-AC-6", "FR-880-AC-8", "FR-882-AC-6")]
#[test]
fn package_repeat_post_test_false_guard_executes_once() {
    let ws = Workspace::new("package-repeat-post-test");
    let controls = format!(
        "repeat Again by Machining visible (again) while {{ again }} {}",
        crate::fill::attempt(
            "Remachined",
            "flag",
            "FlagPre, FlagPost",
            "again",
            "not again"
        )
        .replace(" by w ", " by Machining ")
    );
    let path = repeat_request(&ws, &controls, "false");
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("the post-test unit");
    ws.write(
        "program.fsl",
        format!("{unit}\n{}", crate::fill::contracts("false")).as_bytes(),
    );
    let item = repeat_parity(&ws, &path);
    assert_eq!(label(&item), "refuted", "{item}");
    let prefix = item["counterexample"]["prefix"]
        .as_array()
        .expect("post-test finish prefix");
    assert_eq!(prefix.len(), 1, "{item}");
    assert_eq!(prefix[0]["transition"]["step"], "attempt", "{item}");
    assert_eq!(
        prefix[0]["transition"]["node"], "Main/Again/Remachined",
        "{item}"
    );
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// A blocked repeat body preserves its real deadlock and bounded replay refusal.
#[trace("TC-2958", "FR-940-AC-6", "FR-940-AC-22", "FR-206-AC-5")]
#[test]
fn package_repeat_blocked_attempt_preserves_replay_refusal() {
    for (scenario, guard, constraint) in [
        ("blocked", "true", "false"),
        ("undefined", "zero / zero = zero", "true"),
    ] {
        let controls = format!(
            "repeat Again by Machining visible () while {{ {guard} }} {}",
            repeat_attempt("Remachined", "again", constraint)
        );
        let request = |ws: &Workspace| {
            if scenario == "undefined" {
                decimal_capture_request(ws, &controls, "closing")
            } else {
                repeat_request(ws, &controls, "closing")
            }
        };
        let ws = Workspace::new(&format!("package-repeat-{scenario}"));
        let item = repeat_parity(&ws, &request(&ws));
        assert_eq!(label(&item), "refuted", "{item}");
        if scenario == "blocked" {
            assert_eq!(
                item["counterexample"]["blocked"][0]["rest"]["node"], "Main/Again/Remachined",
                "{item}"
            );
            assert_eq!(
                item["counterexample"]["blocked"][0]["cause"]["type"], "constraint",
                "{item}"
            );
        } else {
            assert_eq!(
                item["record"]["cause"]["kind"], "undefined-evaluation",
                "{item}"
            );
            assert_eq!(
                item["counterexample"]["kind"]["type"], "undefined-evaluation",
                "{item}"
            );
            assert_eq!(
                item["counterexample"]["kind"]["cause"], "division-by-zero",
                "{item}"
            );
        }
        assert_eq!(item["replay"]["reproduced"], true, "{item}");

        let bounded = Workspace::new(&format!("package-repeat-replay-bound-{scenario}"));
        let path = request(&bounded);
        let (package_path, bytes) = package_request(&bounded, &path);
        let limit = u64::try_from(bytes.len() / 2).expect("package byte bound");
        let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, limit);
        assert!(
            source.item(0).get("record").is_some(),
            "{}",
            source.document()
        );
        let source_item = source.item(0);
        let cause = &source_item["record"]["cause"];
        assert_eq!(cause["kind"], "replay-refused", "{cause}");
        assert_eq!(cause["code"], "stage_limit_exceeded", "{cause}");
        assert_eq!(cause["cause"], "input-bytes-exceeded", "{cause}");
        std::fs::remove_file(bounded.path("program.fsl"))
            .expect("remove source before bounded replay");
        let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, limit);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// Application-bearing proofs retain their explicit unsupported disposition.
#[trace("TC-2958", "FR-940-AC-6", "FR-730")]
#[test]
fn package_repeat_proof_refuses_before_execution() {
    let ws = Workspace::new("package-repeat-proof");
    let controls = format!(
        "repeat Again by Machining visible () invariant {{ 1 = 1 }} variant {{ 1 }} while {{ false }} {}",
        repeat_attempt("Remachined", "again", "true")
    );
    let path = repeat_request(&ws, &controls, "closing");
    let source = analyze(&path);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let (package_path, _) = package_request(&ws, &path);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before proof refusal");
    let package = analyze(&package_path);
    let item = package.item(0);
    assert_eq!(package.exit, 21, "{}", package.document());
    assert_eq!(
        item["record"]["diagnostic"]["code"], "unsupported_construct",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "not-yet-implemented",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["fact"], "repeat-proof",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["node"]["path"]
            .as_array()
            .expect("repeat node path")
            .last(),
        Some(&json!("Again")),
        "{item}"
    );
    assert!(item.get("exploration").is_none(), "{item}");
}

fn await_request(
    ws: &Workspace,
    delivery: &str,
    matched: &str,
    then: &str,
    timeout: &str,
) -> std::path::PathBuf {
    await_event_request(
        ws,
        delivery,
        &receive_fifo_receive("Inspected", "Handed", "seen", matched),
        then,
        timeout,
    )
}

fn await_event_request(
    ws: &Workspace,
    delivery: &str,
    event: &str,
    then: &str,
    timeout: &str,
) -> std::path::PathBuf {
    let controls = await_controls(event, then, timeout);
    let ordering = format!("ordering unordered delivery {delivery} capacity 4 overflow block");
    let path = channel_request_with_profiles(
        ws,
        finite_channel_model(),
        &ordering,
        &controls,
        "closing",
        concat!(
            "profile pp = \"filament.protocol.finite-global/v1\";\n",
            "profile t0 = \"filament.temporal.event-position.false-extension/v1\";\n"
        ),
    );
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the Await request"))
            .expect("the request is JSON");
    document["request"]["clocks"] = json!({"line-clock": {
        "kind": "event_position", "sequence_authority": "caller-events"
    }});
    std::fs::write(&path, document.to_string()).expect("retain the supplied Await clock");
    path
}

fn await_controls(event: &str, then: &str, timeout: &str) -> String {
    channel_send("Handed", "handed", "handed.n = k.v")
        + &format!(
            "await Wait after Handed using t0 clock \"line-clock\" within [0, 120]
                match {event}
                then {then}
                timeout {timeout}"
        )
}

/// The complete await body explores both the receive and timeout continuations.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-5", "FR-383-AC-7")]
#[test]
fn package_await_runs_the_complete_matched_receive_body() {
    for delivery in ["exactly-once-premise", "at-least-once"] {
        let ws = Workspace::new(&format!("package-await-complete-{delivery}"));
        let path = await_request(
            &ws,
            delivery,
            "seen.n = handed.n",
            "sequence InTime {}",
            "sequence TooSlow {}",
        );
        let item = channel_send_parity(&ws, &path, "proved");
        if delivery == "exactly-once-premise" {
            // The two continuations reach different finish states: timeout
            // retains the queued entry, while the match consumes it.
            assert_eq!(item["exploration"]["states"], "5", "{item}");
            assert_eq!(item["exploration"]["transitions"], "5", "{item}");
        } else {
            let transitions = item["exploration"]["transitions"]
                .as_str()
                .expect("the explored transition count")
                .parse::<u64>()
                .expect("a natural count");
            assert!(transitions > 5, "duplicate delivery is explored: {item}");
        }
    }
}

/// A blocked timeout continuation retains its counterexample and replay refusals.
#[trace(
    "TC-2958",
    "FR-940-AC-6",
    "FR-940-AC-22",
    "FR-206-AC-5",
    "FR-517-AC-33"
)]
#[test]
fn package_await_timeout_deadlock_preserves_replay_and_refusal() {
    let timeout = format!(
        "sequence TooSlow {{ {} }}",
        receive_fifo_receive("Never", "Handed", "late", "false")
    );
    let request = |ws: &Workspace| {
        await_request(
            ws,
            "exactly-once-premise",
            "seen.n = handed.n",
            "sequence InTime {}",
            &timeout,
        )
    };
    let ws = Workspace::new("package-await-timeout-deadlock");
    let path = request(&ws);
    let item = channel_send_parity(&ws, &path, "refuted");
    let prefix = item["counterexample"]["prefix"]
        .as_array()
        .expect("the timeout deadlock prefix");
    let timed_out = prefix
        .iter()
        .find(|step| step["transition"]["step"] == "timeout")
        .expect("the timeout continuation was taken");
    assert_eq!(timed_out["transition"]["node"], "Main/Wait", "{item}");
    assert!(
        prefix
            .iter()
            .all(|step| step["transition"]["step"] != "receive"),
        "the matched receive leads to the healthy continuation: {item}"
    );
    let blocked = item["counterexample"]["blocked"]
        .as_array()
        .expect("the blocked timeout receive");
    assert_eq!(blocked.len(), 1, "{item}");
    assert_eq!(
        blocked[0]["rest"]["node"], "Main/Wait/TooSlow/Never",
        "{item}"
    );
    assert_eq!(blocked[0]["cause"]["type"], "constraint", "{item}");

    let bounded_ws = Workspace::new("package-await-replay-bound");
    let bounded_path = request(&bounded_ws);
    let (package_path, bytes) = package_request(&bounded_ws, &bounded_path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source_limited =
        crate::analyze_cli_support::analyze_under_replay_bytes(&bounded_path, below);
    assert!(
        source_limited.item(0).get("record").is_some(),
        "{}",
        source_limited.document()
    );
    assert_eq!(
        label(&source_limited.item(0)),
        "inconclusive",
        "{}",
        source_limited.document()
    );
    let cause = &source_limited.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused", "{cause}");
    assert_eq!(cause["code"], "stage_limit_exceeded", "{cause}");
    assert_eq!(cause["cause"], "input-bytes-exceeded", "{cause}");
    std::fs::remove_file(bounded_ws.path("program.fsl"))
        .expect("remove source before package replay refusal");
    let package_limited =
        crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(
        package_limited.item(0),
        source_limited.item(0),
        "{}",
        package_limited.document()
    );
    assert_eq!(package_limited.exit, source_limited.exit);
}

/// A false match keeps timeout enabled and never enters the blocked success arm.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-5", "FR-211-AC-5")]
#[test]
fn package_await_false_match_keeps_timeout_enabled() {
    let ws = Workspace::new("package-await-false-match");
    let then = format!(
        "sequence InTime {{ {} }}",
        receive_fifo_receive("Never", "Handed", "early", "false")
    );
    let path = await_request(
        &ws,
        "exactly-once-premise",
        "not (seen.n = handed.n)",
        &then,
        "sequence TooSlow {}",
    );
    let item = channel_send_parity(&ws, &path, "proved");
    assert_eq!(item["exploration"]["states"], "4", "{item}");
    assert_eq!(item["exploration"]["transitions"], "3", "{item}");
}

/// Source checking and strict package admission reject ineligible Await matches.
#[trace("TC-2958", "FR-218-AC-35", "FR-383-AC-7", "FR-850")]
#[test]
fn package_await_ineligible_matches_refuse_in_both_routes() {
    use filament_contract_model::{
        CheckedPackageRefusalCause, CheckedPackageRefusalCode, CheckedProtocolRefusalDetail,
    };
    let attempt = crate::fill::attempt("Template", "touch", "", "attempted", "true")
        .replace(" by w ", " by Machining ");
    for kind in ["send", "attempt"] {
        let ws = Workspace::new(&format!("package-await-ineligible-{kind}"));
        let then = format!("sequence InTime {{ {attempt} }}");
        let path = await_request(
            &ws,
            "exactly-once-premise",
            "seen.n = handed.n",
            &then,
            "sequence TooSlow {}",
        );
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "{}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
        let (package_path, bytes) = package_request(&ws, &path);
        let baseline = analyze(&package_path);
        assert_eq!(baseline.item(0), source.item(0), "{}", baseline.document());
        assert_eq!(baseline.exit, source.exit);
        let mut wire: Value = serde_json::from_slice(&bytes).expect("the emitted package");
        let original = wire.clone();
        fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
            .expect("resolve node-held IDs through their actual operand rows");
        let mut roundtrip = wire.clone();
        fsl_semantics::check::wire_id::write_operand_rows(&mut roundtrip);
        assert_eq!(
            roundtrip, original,
            "the official conversion preserves every unedited wire fact"
        );
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wire)
            .expect("the emitted identities");
        let nodes = wire["semantic_graph"]["nodes"]
            .as_array_mut()
            .expect("nodes");
        let position = |name: &str| {
            nodes
                .iter()
                .position(|node| {
                    node["semantic_form"] == "control"
                        && node["declaration"]["qualified_name"]
                            .as_array()
                            .and_then(|path| path.last())
                            == Some(&json!(name))
                })
                .unwrap_or_else(|| panic!("the {name} control"))
        };
        let inspected = position("Inspected");
        let template = position("Template");
        if kind == "send" {
            let control = &mut nodes[inspected];
            control["body"]["operation"]["member"]["operator"] = json!("send");
            let arguments = control["body"]["arguments"]
                .as_array_mut()
                .expect("arguments");
            let of = arguments
                .iter()
                .position(|arg| arg["name"] == "of")
                .expect("the receive's send reference");
            let send = arguments.remove(of)["value"]["target"].clone();
            control["dependencies"]
                .as_array_mut()
                .expect("dependencies")
                .retain(|target| target != &send);
        } else {
            // Change only untrusted wire. Keep the declared event identity,
            // sidecars and every other node exactly as emitted.
            let mut body = nodes[template]["body"].clone();
            let mut dependencies = nodes[template]["dependencies"]
                .as_array()
                .expect("template dependencies")
                .clone();
            for name in ["binder", "related", "constraint"] {
                let original = nodes[inspected]["body"]["arguments"]
                    .as_array()
                    .expect("receive arguments")
                    .iter()
                    .find(|arg| arg["name"] == name)
                    .expect("the receive member")
                    .clone();
                let member = body["arguments"]
                    .as_array_mut()
                    .expect("attempt arguments")
                    .iter_mut()
                    .find(|arg| arg["name"] == name)
                    .expect("the attempt member");
                if let Some(target) = member["value"].get("target") {
                    dependencies.retain(|dependency| dependency != target);
                }
                if let Some(target) = original["value"].get("target") {
                    dependencies.push(target.clone());
                }
                *member = original;
            }
            nodes[inspected]["body"] = body;
            nodes[inspected]["dependencies"] = json!(dependencies);
        }
        nodes[inspected]["dependencies"]
            .as_array_mut()
            .expect("dependencies")
            .sort_by_key(|target| graph.key_of_value(target).expect("an emitted identity"));
        fsl_semantics::check::wire_id::write_operand_rows(&mut wire);
        let event = if kind == "send" {
            channel_send("Inspected", "seen", "seen.n = handed.n")
        } else {
            crate::fill::attempt("Inspected", "touch", "", "attempted", "true")
                .replace(" by w ", " by Machining ")
        };
        let invalid_path = await_event_request(
            &ws,
            "exactly-once-premise",
            &event,
            "sequence InTime {}",
            "sequence TooSlow {}",
        );
        let invalid_source = analyze(&invalid_path).document();
        assert_eq!(
            invalid_source["diagnostics"][0]["code"], "unsupported_construct",
            "{invalid_source}"
        );
        assert_eq!(
            invalid_source["diagnostics"][0]["cause"], "declaration-form",
            "{invalid_source}"
        );
        assert_eq!(invalid_source["items"], json!([]), "{invalid_source}");
        std::fs::write(
            ws.path("handoff/checked-package.json"),
            serde_json::to_vec(&wire).expect("the edited untrusted wire"),
        )
        .expect("replace the package input");
        std::fs::remove_file(ws.path("program.fsl")).expect("source is absent");
        let refusal = fsl_command::checked_handoff::verify(&ws.path("handoff"))
            .expect_err("the strict reader rejects the ineligible event");
        let fsl_command::checked_handoff::Error::ReaderRefused(refusal) = refusal else {
            panic!("the strict protocol reader must decide: {refusal:?}");
        };
        assert_eq!(
            refusal.code,
            CheckedPackageRefusalCode::UnsupportedConstruct
        );
        assert_eq!(
            refusal.cause,
            Some(CheckedPackageRefusalCause::DeclarationForm)
        );
        assert!(
            matches!(refusal.protocol_detail.as_deref(),
                Some(CheckedProtocolRefusalDetail::AwaitedEvent { form, .. })
                    if form.as_ref() == kind),
            "{refusal:?}"
        );
        let native_path = refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .expect("the native refusal names its Await control")
            .as_str();
        let package_run = analyze(&package_path);
        assert_eq!(package_run.exit, 20);
        let invalid_package = package_run.document();
        // Source checking and the native reader retain their semantic refusal.
        // FR-850's artifact boundary classifies rejected package bytes while
        // preserving that reader's cause and original pointer.
        assert_eq!(invalid_package["category"], "refusal", "{invalid_package}");
        assert_eq!(
            invalid_package["diagnostics"][0]["code"], "invalid_package",
            "{invalid_package}"
        );
        assert_eq!(
            invalid_package["diagnostics"][0]["cause"], "declaration-form",
            "{invalid_package}"
        );
        assert_eq!(
            invalid_package["diagnostics"][0]["details"]["path"], native_path,
            "{invalid_package}"
        );
        assert_eq!(invalid_package["items"], json!([]), "{invalid_package}");
        assert_eq!(invalid_package["registry"], json!([]), "{invalid_package}");
    }
}

fn receive_fifo_request(
    ws: &Workspace,
    key: &str,
    capacity: u64,
    overflow: &str,
    delivery: &str,
    controls: &str,
    finish: &str,
) -> std::path::PathBuf {
    let mut model = finite_channel_model();
    let payload = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Attempt")
        .expect("the finite payload record");
    let field = payload["fields"][0].clone();
    for (name, value_type) in [
        ("ok", "ix://filament/native/Boolean"),
        ("target", "ix://test/fill/Cell"),
    ] {
        let mut member = field.clone();
        let identity = format!("ix://test/fill/Attempt/{name}");
        member["identity"] = json!(identity);
        member["name"] = json!(name);
        member["typeRef"] = json!(value_type);
        member["origin"] = json!({"generated": {
            "generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]
        }});
        payload["fields"]
            .as_array_mut()
            .expect("payload fields")
            .push(member);
    }
    let ordering = format!(
        "ordering fifo by (m: M::Attempt) {{ {key} }}
                delivery {delivery} capacity {capacity} overflow {overflow}"
    );
    channel_request(ws, model, &ordering, controls, finish)
}

fn receive_fifo_send(name: &str, binder: &str, n: u64) -> String {
    let body =
        format!("\n            {binder}.n = {n} and {binder}.ok and {binder}.target = k\n        ");
    channel_send_body(name, binder, &body)
}

fn receive_fifo_receive(name: &str, send: &str, binder: &str, constraint: &str) -> String {
    format!("receive {name} via Release of {send} as ({binder}: M::Attempt) {{ {constraint} }};")
}

/// The FIFO example's receive evaluates both scalar and reference-valued keys.
#[trace("TC-2958", "FR-940-AC-6", "FR-383-AC-4", "FR-383-AC-7", "FR-390")]
#[test]
fn package_receive_fifo_runs_the_complete_example_body() {
    for (name, key, delivery, finish, expected) in [
        (
            "reference",
            "m.target",
            "at-least-once",
            "closing",
            "proved",
        ),
        ("scalar", "m.n", "exactly-once-premise", "false", "refuted"),
    ] {
        let ws = Workspace::new(&format!("package-receive-fifo-complete-{name}"));
        let controls = receive_fifo_send("Released", "order", 0)
            + &receive_fifo_receive("Received", "Released", "got", "got.ok");
        let path = receive_fifo_request(&ws, key, 4, "block", delivery, &controls, finish);
        let item = channel_send_parity(&ws, &path, expected);
        if expected == "refuted" {
            let prefix = item["counterexample"]["prefix"]
                .as_array()
                .expect("a deadlock prefix");
            let received = prefix
                .iter()
                .find(|step| step["transition"]["step"] == "receive")
                .expect("the payload was received before the false finish");
            assert_eq!(received["transition"]["node"], "Main/Received", "{item}");
            let sent = prefix
                .iter()
                .find(|step| step["transition"]["step"] == "send")
                .expect("the payload's send");
            assert_eq!(
                received["transition"]["delivery"]["message"], sent["transition"]["message"],
                "{item}"
            );
            let fields = sent["send_record"]["payload"]["fields"]
                .as_array()
                .expect("payload fields");
            let ok = fields
                .iter()
                .find(|field| field["name"] == "ok")
                .expect("the ok field");
            assert_eq!(ok["value"]["value"], true, "{item}");
        } else {
            let states = item["exploration"]["states"]
                .as_str()
                .expect("state count")
                .parse::<u64>()
                .expect("a natural count");
            assert!(states > 3, "send, receive and finish were explored: {item}");
        }
    }
}

/// A receive of the second send waits only behind another send with the same key.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-4", "FR-211-AC-5")]
#[test]
fn package_receive_fifo_preserves_key_partition_and_head_blocking() {
    for (name, second_key, expected, cause) in [
        ("same", 0, "refuted", Some("head-blocked")),
        ("different", 1, "proved", None),
        ("ne-same-reference", 1, "proved", None),
        ("ne-distinct-reference", 1, "proved", None),
        ("eq-distinct-reference", 1, "refuted", Some("constraint")),
    ] {
        let ws = Workspace::new(&format!("package-receive-fifo-head-{name}"));
        let distinct = matches!(name, "ne-distinct-reference" | "eq-distinct-reference");
        let mut second = receive_fifo_send("Second", "sentTwo", second_key);
        if distinct {
            second = second.replace("sentTwo.target = k", "sentTwo.target != k");
        }
        let receive = match name {
            "ne-same-reference" => "got.ok and not (got.target != k)",
            "ne-distinct-reference" => "got.ok and got.target != k",
            "eq-distinct-reference" => "got.ok and got.target = k",
            _ => "got.ok",
        };
        let controls = receive_fifo_send("First", "sentOne", 0)
            + &second
            + &receive_fifo_receive("Received", "Second", "got", receive);
        let path = receive_fifo_request(
            &ws,
            "m.n",
            2,
            "block",
            "exactly-once-premise",
            &controls,
            "closing",
        );
        if distinct {
            let snapshots = channel_snapshots(&[("c", 0), ("d", 0)]);
            ws.write("snapshot-0.json", &snapshots[0]);
        }
        let item = channel_send_parity(&ws, &path, expected);
        if let Some(cause) = cause {
            let blocked = item["counterexample"]["blocked"]
                .as_array()
                .expect("blocked receive");
            assert_eq!(blocked.len(), 1, "{item}");
            assert_eq!(blocked[0]["rest"]["node"], "Main/Received", "{item}");
            assert_eq!(blocked[0]["cause"]["type"], cause, "{item}");
            if cause == "head-blocked" {
                assert_eq!(blocked[0]["cause"]["channel"], "Flow::Release", "{item}");
            }
        }
    }
}

/// Constraint, empty-queue and displaced evidence retain their own wait causes.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-7", "FR-211-AC-5")]
#[test]
fn package_receive_fifo_preserves_receive_wait_causes() {
    for cause in ["constraint", "empty-queue", "displaced"] {
        let ws = Workspace::new(&format!("package-receive-fifo-cause-{cause}"));
        let mut controls = receive_fifo_send("Released", "order", 0);
        let (capacity, overflow) = match cause {
            "constraint" => {
                controls += &receive_fifo_receive("Received", "Released", "got", "not got.ok");
                (2, "block")
            }
            "empty-queue" => {
                controls += &receive_fifo_receive("Received", "Released", "got", "got.ok");
                controls += &receive_fifo_receive("Again", "Released", "again", "again.ok");
                (2, "block")
            }
            "displaced" => {
                controls += &receive_fifo_send("Other", "other", 1);
                controls += &receive_fifo_receive("Received", "Released", "got", "got.ok");
                (1, "loss head")
            }
            _ => unreachable!("the listed wait causes"),
        };
        let path = receive_fifo_request(
            &ws,
            "true",
            capacity,
            overflow,
            "exactly-once-premise",
            &controls,
            "closing",
        );
        let item = channel_send_parity(&ws, &path, "refuted");
        let blocked = item["counterexample"]["blocked"]
            .as_array()
            .expect("blocked receive");
        assert_eq!(blocked.len(), 1, "{item}");
        assert_eq!(blocked[0]["cause"]["type"], cause, "{item}");
        let rest = if cause == "empty-queue" {
            "Main/Again"
        } else {
            "Main/Received"
        };
        assert_eq!(blocked[0]["rest"]["node"], rest, "{item}");
    }
}

/// Refused replay of a blocked FIFO receive retains the typed limit code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-4")]
#[test]
fn package_receive_fifo_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-receive-fifo-replay-bound");
    let controls = receive_fifo_send("First", "sentOne", 0)
        + &receive_fifo_send("Second", "sentTwo", 0)
        + &receive_fifo_receive("Received", "Second", "got", "got.ok");
    let path = receive_fifo_request(
        &ws,
        "m.n",
        2,
        "block",
        "exactly-once-premise",
        &controls,
        "closing",
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused", "{}", source.document());
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package replay");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// FIFO correlation filters the delivered payload on both intake routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-731-AC-1", "FSpec-FR-831")]
#[test]
fn package_receive_fifo_related_premises_enforce_source_package_parity() {
    for (related, expected) in [(false, "refuted"), (true, "proved")] {
        let ws = Workspace::new(&format!("package-receive-fifo-related-{related}"));
        let controls = receive_fifo_send("Released", "order", 0)
            + "receive Correlated via Release of Released as (got: M::Attempt) related by Owns (k, got.target) { got.ok };";
        let path = receive_fifo_request(
            &ws,
            "m.target",
            1,
            "block",
            "exactly-once-premise",
            &controls,
            "closing",
        );
        channel_related_premise(&ws, related);
        channel_related_graph(&ws, &path, related, true);
        let item = workflow_role_parity(&ws, &path, expected);
        if !related {
            assert_eq!(
                item["counterexample"]["blocked"][0]["rest"]["node"], "Main/Correlated",
                "{item}"
            );
            let prefix = item["counterexample"]["prefix"]
                .as_array()
                .expect("the send precedes the blocked receive");
            assert!(
                prefix
                    .iter()
                    .any(|step| step["transition"]["step"] == "send"),
                "{item}"
            );
            assert!(
                !prefix
                    .iter()
                    .any(|step| step["transition"]["step"] == "receive"),
                "{item}"
            );
        }
    }
}

fn finite_channel_model() -> Value {
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    // The record's field ranges over Level so the three sent payloads can
    // distinguish which queue entry a head or tail policy displaced.
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some(
                    "ix://test/fill/Level"
                        | "ix://test/fill/Cell"
                        | "ix://test/fill/Idle"
                        | "ix://test/fill/Attempt"
                )
            )
        });
    let payload = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Attempt")
        .expect("the payload record");
    payload["fields"][0]["typeRef"] = json!("ix://test/fill/Level");
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population" | "record_value_type")
            )
        });
    model
}

fn channel_snapshots(cells: &[(&str, i64)]) -> Vec<Vec<u8>> {
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", cells);
    documents.snapshots.into_values().collect()
}

fn channel_request(
    ws: &Workspace,
    model: Value,
    ordering: &str,
    controls: &str,
    finish: &str,
) -> std::path::PathBuf {
    channel_request_with_profiles(
        ws,
        model,
        ordering,
        controls,
        finish,
        crate::fill::PROTOCOL_PROFILE,
    )
}

fn channel_body(ordering: &str, controls: &str, finish: &str) -> String {
    format!(
        "protocol Flow using pp over (k: M::Cell) on origin {{
            role Planning on M::Cell;
            role Machining on M::Cell;
            channel Release from Planning to Machining carries M::Attempt
                {ordering};
            run sequence Main {{ {controls} }}
            finish Over as (closing: Boolean) {{ {finish} }};
        }}"
    )
}

fn channel_request_with_profiles(
    ws: &Workspace,
    model: Value,
    ordering: &str,
    controls: &str,
    finish: &str,
    profiles: &str,
) -> std::path::PathBuf {
    let unit = crate::fill::unit_text_with(profiles, &channel_body(ordering, controls, finish));
    let model = serde_json::to_vec(&model).expect("the channel model serializes");
    let snapshots = channel_snapshots(&[("c", 0)]);
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {
                "Planning": {"type": "model-binding", "selection": "test/fill"},
                "Machining": {"type": "model-binding", "selection": "test/fill"}
            }
        }}}}]}),
    )
}

fn channel_send_request(
    ws: &Workspace,
    capacity: u64,
    overflow: &str,
    delivery: &str,
    sends: &str,
    finish: &str,
) -> std::path::PathBuf {
    let ordering =
        format!("ordering unordered delivery {delivery} capacity {capacity} overflow {overflow}");
    channel_request(ws, finite_channel_model(), &ordering, sends, finish)
}

fn channel_send_body(name: &str, binder: &str, body: &str) -> String {
    format!("send {name} via Release as ({binder}: M::Attempt) {{{body}}};")
}

fn channel_send(name: &str, binder: &str, constraint: &str) -> String {
    channel_send_body(name, binder, &format!(" {constraint} "))
}

fn channel_send_parity(ws: &Workspace, path: &std::path::Path, expected: &str) -> Value {
    channel_send_parity_using(ws, path, expected, || package_request(ws, path).0)
}

fn channel_send_parity_using(
    ws: &Workspace,
    path: &std::path::Path,
    expected: &str,
    package_request_path: impl FnOnce() -> std::path::PathBuf,
) -> Value {
    let source = analyze(path);
    assert!(
        source.item(0).get("record").is_some(),
        "source channel analysis: {}",
        source.document()
    );
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let package_path = package_request_path();
    std::fs::remove_file(ws.path("program.fsl"))
        .expect("remove source before package analysis and replay");
    let package = analyze(&package_path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
    if expected == "refuted" {
        assert_eq!(package.item(0)["replay"]["reproduced"], true);
    }
    source.item(0)
}

fn symbolic_capacity_request(
    ws: &Workspace,
    capacity: Option<u64>,
    second: bool,
) -> std::path::PathBuf {
    let sends = |channel: &str, suffix: &str| {
        format!("send First{suffix} via {channel} as (sent_a{suffix}: M::Attempt) {{ sent_a{suffix}.n = 0 }};
                 send Second{suffix} via {channel} as (second{suffix}: M::Attempt) {{ second{suffix}.n = 1 }};")
    };
    let receives = |channel: &str, suffix: &str| {
        format!("receive GotFirst{suffix} via {channel} of First{suffix} as (gotfirst{suffix}: M::Attempt) {{ true }};
                 receive GotSecond{suffix} via {channel} of Second{suffix} as (gotsecond{suffix}: M::Attempt) {{ true }};")
    };
    let mut controls = sends("Release", "");
    if second {
        controls += &sends("Alpha", "Other");
    }
    controls += &receives("Release", "");
    if second {
        controls += &receives("Alpha", "Other");
    }
    let ordering =
        "ordering unordered delivery exactly-once-premise capacity symbolic(Bound) overflow block";
    let path = channel_request(ws, finite_channel_model(), ordering, &controls, "closing");
    if second {
        let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("source");
        ws.write("program.fsl", unit.replace("run sequence Main", &format!(
            "channel Alpha from Planning to Machining carries M::Attempt {ordering}; run sequence Main"
        )).as_bytes());
    }
    if let Some(capacity) = capacity {
        let mut request: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
        request["request"]["items"][0]["temporal"]["subject"]["protocol"]["capacities"] =
            json!({"Bound": capacity.to_string()});
        ws.write("analyze.json", request.to_string().as_bytes());
    }
    path
}

/// Bound symbolic queues use the same native states, steps and message identities
/// as Source, including zero capacity and one value shared by two channels.
#[trace(
    "TC-2958",
    "FR-940-AC-6",
    "FSpec-TC-1911",
    "FSpec-FR-785-AC-1",
    "FSpec-FR-785-AC-3"
)]
#[test]
fn package_symbolic_capacity_steps_at_zero_one_and_two() {
    for (capacity, second) in [(0, false), (1, false), (2, false), (2, true)] {
        let ws = Workspace::new(&format!("package-symbolic-capacity-{capacity}-{second}"));
        let path = symbolic_capacity_request(&ws, Some(capacity), second);
        let unit = std::fs::read(ws.path("program.fsl")).expect("source before graph comparison");
        let graph = package_protocol_graph(
            &ws,
            &path,
            fsl_eval::simulation::protocol::Sc,
            &["Planning", "Machining"],
            true,
            false,
            |_, _| {},
        );
        assert!(
            !ws.path("program.fsl").exists(),
            "native graph runs without source"
        );
        let peak = graph
            .iter()
            .map(|state| {
                let key: Value = serde_json::from_slice(state.key.as_bytes()).expect("state key");
                key["queues"]["entries"]
                    .as_array()
                    .expect("queue map")
                    .iter()
                    .map(|entry| {
                        entry["value"]["elements"]
                            .as_array()
                            .expect("queue entries")
                            .len()
                    })
                    .sum::<usize>()
            })
            .max()
            .expect("initial state");
        assert_eq!(
            peak,
            usize::try_from(capacity).expect("tiny bound") * if second { 2 } else { 1 }
        );
        ws.write("program.fsl", &unit);
        let item =
            workflow_role_parity(&ws, &path, if capacity < 2 { "refuted" } else { "proved" });
        assert!(
            !ws.path("program.fsl").exists(),
            "package analysis ran without source"
        );
        if capacity < 2 {
            assert_eq!(
                item["counterexample"]["subject"]["capacities"],
                json!({"Bound": capacity.to_string()})
            );
            assert_eq!(
                item["counterexample"]["blocked"][0]["cause"]["type"], "full-queue",
                "{item}"
            );
            assert_eq!(
                item["counterexample"]["blocked"][0]["cause"]["channel"], "Flow::Release",
                "{item}"
            );
        }
    }
}

fn copy_capacity_subject<'a>(
    subject: &fsl_eval::model_subject::ProtocolSubject<'a>,
) -> fsl_eval::model_subject::ProtocolSubject<'a> {
    fsl_eval::model_subject::ProtocolSubject {
        model: fsl_eval::model_subject::ModelSubject {
            simulation: subject.model.simulation.clone(),
            proof_bounds: subject.model.proof_bounds.clone(),
        },
        protocol: subject.protocol.clone(),
        over: subject.over.clone(),
        roles: subject.roles.clone(),
        capacities: subject.capacities.clone(),
        memory: subject.memory.clone(),
    }
}

fn with_capacity_subjects(
    ws: &Workspace,
    path: &std::path::Path,
    run: impl FnOnce(
        fsl_eval::model_subject::ProtocolSubject<'_>,
        fsl_eval::model_subject::ProtocolSubject<'_>,
        &fsl_package::CheckedPackage,
        &crate::fill::Documents,
    ),
) {
    use fsl_eval::simulation::model::PackageModelInput;
    use fsl_package::{AnalysisView, BarePackage, BareSupply, EmitLimits, V2ReadLimits};
    let _ = package_request(ws, path);
    let compiled =
        fsl_command::checked_handoff::compile(&ws.path(crate::analyze_cli_support::HANDOFF))
            .expect("the capacity fixture compiles");
    let mut documents = crate::fill::Documents::new();
    documents.packages = compiled.packages().clone();
    let initial = documents.start("s0", &[("c", 0)]);
    let mut source =
        crate::fill::subject(compiled.checked(), &documents, vec![initial], "Flow", "c");
    source.roles = ["Planning", "Machining"]
        .into_iter()
        .map(|role| fsl_eval::model_subject::RoleBinding {
            role: role.to_owned(),
            model: crate::fill::PACKAGE.to_owned(),
        })
        .collect();
    let emission =
        fsl_package::emit_checked(compiled.checked(), EmitLimits::fixture()).expect("emission");
    let identities = emission
        .identity_rows(
            compiled.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("identities");
    let equality = emission
        .equality_witness(
            compiled.admitted(),
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("equality");
    let bare = BarePackage::new(compiled.admitted())
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("strict native supply");
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("native input");
    let mut package = copy_capacity_subject(&source);
    package.model.simulation.input = input.input();
    assert!(package.model.simulation.source_package().is_none());
    package.memory = fsl_eval::simulation::protocol::ResolvedMemory::resolve_package(
        &bare
            .protocol_execution_named("Flow")
            .expect("protocol view")
            .expect("Flow"),
        &[],
    )
    .expect("native memory");
    run(source, package, compiled.checked(), &documents);
}

fn capacity_outcome(
    subject: fsl_eval::model_subject::ProtocolSubject<'_>,
) -> Result<
    fsl_replay::ModelCheckOutcome<fsl_eval::simulation::protocol::ProtocolTransition>,
    fsl_analyze::model_check::ModelCheckRefusal,
> {
    fsl_analyze::model_check::check_protocol(
        fsl_analyze::model_check::ProtocolCheckRequest {
            subject,
            workers: 1,
            temporal_clock_selections: &[],
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
}

/// Unbound names aggregate with other roots before exploration; channel users
/// retain declaration order rather than node-id or alphabetical order.
#[trace(
    "TC-2958",
    "FR-940-AC-6",
    "FR-215-AC-4",
    "FSpec-TC-1911",
    "FSpec-FR-785-AC-2",
    "FSpec-FR-785-AC-3"
)]
#[test]
fn package_symbolic_capacity_precheck_retains_typed_roots_and_binding_refusals() {
    use fsl_analyze::model_check::ModelCheckRefusal;
    use fsl_eval::model_subject::CapacityBinding;
    use fsl_eval::simulation::protocol::{CapacityRefusal, ProtocolRefusal, UnboundCapacity};
    for (wide, separate) in [(false, false), (true, false), (false, true)] {
        let ws = Workspace::new(&format!(
            "package-symbolic-capacity-unbound-{wide}-{separate}"
        ));
        let path = symbolic_capacity_request(&ws, None, true);
        if separate {
            let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("source");
            ws.write(
                "program.fsl",
                unit.replacen("symbolic(Bound)", "symbolic(Other)", 1)
                    .as_bytes(),
            );
        }
        if wide {
            let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("source");
            ws.write(
                "program.fsl",
                unit.replace(
                    "(closing: Boolean) { closing }",
                    "(closing: Integer) { closing = 0 }",
                )
                .as_bytes(),
            );
        }
        with_capacity_subjects(&ws, &path, |source, package, _, _| {
            let names = if separate {
                vec![
                    UnboundCapacity {
                        name: "Bound".to_owned(),
                        channels: vec!["Flow::Alpha".to_owned()],
                    },
                    UnboundCapacity {
                        name: "Other".to_owned(),
                        channels: vec!["Flow::Release".to_owned()],
                    },
                ]
            } else {
                vec![UnboundCapacity {
                    name: "Bound".to_owned(),
                    channels: vec!["Flow::Release".to_owned(), "Flow::Alpha".to_owned()],
                }]
            };
            let inspect = |refusal| match refusal {
                ModelCheckRefusal::UnboundCapacity(actual) if !wide => assert_eq!(actual, names),
                ModelCheckRefusal::RequiresBound(actual) if wide => {
                    assert_eq!(actual.capacities, names);
                    assert!(
                        actual.domains.is_some(),
                        "the Integer finish is also unbounded"
                    );
                }
                other => panic!("expected typed requires-bound, got {other:?}"),
            };
            inspect(
                capacity_outcome(copy_capacity_subject(&source)).expect_err("no source expansion"),
            );
            std::fs::remove_file(ws.path("program.fsl"))
                .expect("remove source before native precheck");
            inspect(
                capacity_outcome(copy_capacity_subject(&package))
                    .expect_err("no package expansion"),
            );
            for (bindings, expected) in [
                (
                    vec![CapacityBinding {
                        name: "Unknown".to_owned(),
                        value: 1,
                    }],
                    CapacityRefusal::UnknownName {
                        name: "Unknown".to_owned(),
                    },
                ),
                (
                    vec![
                        CapacityBinding {
                            name: "Bound".to_owned(),
                            value: 1
                        };
                        2
                    ],
                    CapacityRefusal::Duplicate {
                        name: "Bound".to_owned(),
                    },
                ),
            ] {
                for mut subject in [
                    copy_capacity_subject(&source),
                    copy_capacity_subject(&package),
                ] {
                    subject.capacities = bindings.clone();
                    let refusal =
                        capacity_outcome(subject).expect_err("binding defect precedes roots");
                    assert!(
                        matches!(refusal, ModelCheckRefusal::Protocol(ProtocolRefusal::Capacity(ref actual))
                        if actual == &expected),
                        "{refusal:?}"
                    );
                }
            }
        });
    }
}

/// A real deadlock's package replay keeps the capacity binding, disagrees at
/// another capacity and refuses missing, unknown and duplicate bindings by type.
#[trace("TC-2958", "FR-940-AC-6", "FSpec-TC-1911", "FSpec-FR-785-AC-5")]
#[test]
fn package_symbolic_capacity_replay_readmits_the_recorded_binding() {
    use fsl_eval::model_subject::CapacityBinding;
    use fsl_eval::simulation::protocol::{CapacityRefusal, ProtocolRefusal};
    use fsl_replay::{ModelCheckOutcome, ModelTraceRefusal, ModelTraceSettlement, ReplayRefusal};
    let ws = Workspace::new("package-symbolic-capacity-replay");
    let path = symbolic_capacity_request(&ws, Some(1), false);
    with_capacity_subjects(&ws, &path, |mut source, _, checked, documents| {
        source.capacities = vec![CapacityBinding {
            name: "Bound".to_owned(),
            value: 1,
        }];
        let ModelCheckOutcome::Violated { counterexample, .. } =
            capacity_outcome(copy_capacity_subject(&source)).expect("bound source checks")
        else {
            panic!("one-slot queue deadlocks");
        };
        let mut wire = crate::model_settle_support::request(
            checked,
            &[],
            &std::fs::read(ws.path("model.json")).expect("model"),
            documents.snapshots.iter(),
            &std::fs::read(ws.path("program.fsl")).expect("source"),
        );
        let (node, occurrence) = crate::model_settle_support::protocol_anchor(checked, "Flow");
        let envelope = crate::model_settle_support::envelope_at(*counterexample, node, occurrence);
        let mut subject = fsl_replay::ProtocolTraceSubject {
            model: fsl_replay::ModelTraceSubject {
                timed: None,
                initial: source.model.simulation.initial.clone(),
                universes: source.model.simulation.universes.clone(),
                expansion: source.model.simulation.expansion,
                stutter_closure: false,
            },
            protocol: source.protocol.clone(),
            over: source.over.clone(),
            roles: source.roles.clone(),
            capacities: source.capacities.clone(),
            memory: source.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        };
        let directory = ws.path(crate::analyze_cli_support::HANDOFF);
        let identities = std::fs::read(directory.join("identities.json")).expect("identity rows");
        let equality = std::fs::read(directory.join("equality.json")).expect("equality rows");
        std::fs::remove_file(ws.path("program.fsl")).expect("source absent during native replay");
        wire.byte_provision.clear();
        assert!(wire.byte_provision.is_empty());
        assert!(!ws.path("program.fsl").exists());
        let replay = |subject: &fsl_replay::ProtocolTraceSubject| {
            fsl_replay::replay_protocol_trace_from_package(
                wire.clone(),
                fsl_replay::HandoffSupply {
                    dependencies: &[],
                    identities: &identities,
                    equality: Some(&equality),
                },
                subject,
                &envelope,
                &crate::counter::Letters { atom: |_| None },
                fsl_replay::ReplayLimits::default(),
            )
        };
        assert!(matches!(
            replay(&subject).expect("native replay").settlement,
            ModelTraceSettlement::Reproduced { .. }
        ));
        subject.capacities[0].value = 2;
        assert!(matches!(
            replay(&subject).expect("changed bound rechecks").settlement,
            ModelTraceSettlement::Inconclusive(_)
        ));
        for (bindings, expected) in [
            (
                vec![],
                CapacityRefusal::Missing {
                    name: "Bound".to_owned(),
                },
            ),
            (
                vec![CapacityBinding {
                    name: "Other".to_owned(),
                    value: 1,
                }],
                CapacityRefusal::UnknownName {
                    name: "Other".to_owned(),
                },
            ),
            (
                vec![
                    CapacityBinding {
                        name: "Bound".to_owned(),
                        value: 1
                    };
                    2
                ],
                CapacityRefusal::Duplicate {
                    name: "Bound".to_owned(),
                },
            ),
        ] {
            subject.capacities = bindings;
            let Err(ReplayRefusal::ModelTrace(refusal)) = replay(&subject) else {
                panic!("native replay must refuse the defective binding");
            };
            assert!(
                matches!(&*refusal, ModelTraceRefusal::Protocol(ProtocolRefusal::Capacity(actual))
                if actual == &expected),
                "{refusal:?}"
            );
        }
    });
}

/// The complete loss-head example's single constrained send uses model payloads.
#[trace("TC-2958", "FR-940-AC-6", "FR-383-AC-4", "FR-383-AC-7")]
#[test]
fn package_channel_send_runs_the_complete_loss_head_body() {
    let ws = Workspace::new("package-channel-send-complete");
    let path = channel_send_request(
        &ws,
        2,
        "loss head",
        "at-most-once",
        &channel_send("Released", "order", "order.n = k.v"),
        "closing",
    );
    let item = channel_send_parity(&ws, &path, "proved");
    let states = item["exploration"]["states"]
        .as_str()
        .expect("the explored state count")
        .parse::<u64>()
        .expect("a natural count");
    assert!(states > 2, "the send and finish were explored: {item}");
}

/// Every unordered overflow policy preserves the recorded payload and displacement.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-7", "FR-206-AC-12")]
#[test]
fn package_channel_send_preserves_unordered_overflow_records() {
    let sends = channel_send("First", "sent_a", "sent_a.n = k.v")
        + &channel_send("Second", "sent_b", "sent_b.n = sent_a.n + 1")
        + &channel_send("Third", "sent_c", "sent_c.n = sent_b.n + 1");
    for (overflow, outcome, displaced) in [
        ("reject", "rejected", None),
        ("loss new", "lost", None),
        ("loss head", "accepted", Some("0")),
        ("loss tail", "accepted", Some("1")),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-{overflow}"));
        let path = channel_send_request(&ws, 2, overflow, "exactly-once-premise", &sends, "false");
        let item = channel_send_parity(&ws, &path, "refuted");
        let records: Vec<_> = item["counterexample"]["prefix"]
            .as_array()
            .expect("a deadlock prefix")
            .iter()
            .filter_map(|step| step.get("send_record"))
            .collect();
        assert_eq!(records.len(), 3, "three sends are reached: {item}");
        for record in &records {
            assert_eq!(record["channel"], "Flow::Release");
        }
        assert_eq!(records[0]["outcome"], "accepted");
        assert_eq!(records[1]["outcome"], "accepted");
        assert_eq!(records[2]["outcome"], outcome);
        assert_eq!(records[2]["payload"]["fields"][0]["value"]["value"], "2");
        match displaced {
            Some(value) => {
                assert_eq!(records[2]["displaced"]["type"], "queue-entry");
                assert_eq!(
                    records[2]["displaced"]["payload"]["fields"][0]["value"]["value"], value,
                    "{item}"
                );
            }
            None => assert_eq!(records[2]["displaced"], Value::Null, "{item}"),
        }
    }
}

/// A send's constraint is checked before capacity, including in blocked evidence.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-4", "FR-211-AC-5")]
#[test]
fn package_channel_send_preserves_capacity_and_constraint_causes() {
    for (name, overflow, constraint, expected, cause) in [
        (
            "full",
            "block",
            "sent_b.n = 1",
            "refuted",
            Some("full-queue"),
        ),
        (
            "constraint",
            "block",
            "false",
            "refuted",
            Some("constraint"),
        ),
        ("loss", "loss head", "sent_b.n = 1", "proved", None),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-{name}"));
        let sends = channel_send("First", "sent_a", "sent_a.n = 0")
            + &channel_send("Second", "sent_b", constraint);
        let path =
            channel_send_request(&ws, 1, overflow, "exactly-once-premise", &sends, "closing");
        let item = channel_send_parity(&ws, &path, expected);
        if let Some(cause) = cause {
            let blocked = item["counterexample"]["blocked"]
                .as_array()
                .expect("the blocked send");
            assert_eq!(blocked.len(), 1, "{item}");
            assert_eq!(blocked[0]["rest"]["node"], "Main/Second");
            assert_eq!(blocked[0]["cause"]["type"], cause, "{item}");
        }
    }
}

/// Refused replay of a full queue preserves the typed limit code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-4")]
#[test]
fn package_channel_send_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-channel-send-replay-bound");
    let sends = channel_send("First", "sent_a", "sent_a.n = 0")
        + &channel_send("Second", "sent_b", "sent_b.n = 1");
    let path = channel_send_request(&ws, 1, "block", "exactly-once-premise", &sends, "closing");
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "{}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl"))
        .expect("remove source before package analysis and replay");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Extra or malformed capacity bindings refuse with identical typed records.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_channel_send_invalid_capacity_records_match_source() {
    for (name, value, cause) in [
        ("negative", "-1", "invalid-value"),
        ("unknown", "1", "unknown-member"),
    ] {
        let ws = Workspace::new(&format!("package-channel-send-capacity-{name}"));
        let path = channel_send_request(
            &ws,
            1,
            "block",
            "exactly-once-premise",
            &channel_send("Only", "only", "only.n = 0"),
            "closing",
        );
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
        document["request"]["items"][0]["temporal"]["subject"]["protocol"]["capacities"] =
            json!({"extra": value});
        std::fs::write(&path, document.to_string()).expect("write malformed capacity selection");
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(item["category"], "refusal", "{}", source.document());
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(item["record"]["diagnostic"]["cause"], cause, "{item}");
        if name == "unknown" {
            assert_eq!(
                item["record"]["diagnostic"]["detail"]["capacity"], "extra",
                "{item}"
            );
        }
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package refusal");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), item, "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// The original inside/after and SC/TSO combinations execute on both routes.
#[trace("TC-2958", "FR-940", "FR-894", "FR-205", "FR-533")]
#[test]
fn package_channel_send_parallel_combinations_match_both_routes() {
    use fsl_package::{
        AnalysisView, BarePackage, BareSupply, EmitLimits, ProtocolSlotDescriptor, V2ReadLimits,
    };
    for memory in ["sc", "tso"] {
        for position in ["inside", "after"] {
            let ws = Workspace::new(&format!("package-channel-send-{memory}-{position}"));
            let send = channel_send("Only", "only", "only.n = 0");
            let (inside, after) = if position == "inside" {
                (send.as_str(), "")
            } else {
                ("", send.as_str())
            };
            let run = format!(
                "parallel Both memory {memory} {{
                    branch left sequence Left {{ {inside} }}
                    branch right sequence Right {{ }}
                }} join all [left, right]; {after}"
            );
            let path =
                channel_send_request(&ws, 1, "block", "exactly-once-premise", &run, "closing");
            let item = channel_send_parity_using(&ws, &path, "proved", || {
                let (package_path, _) = package_request(&ws, &path);
                let compiled = fsl_command::checked_handoff::compile(
                    &ws.path(crate::analyze_cli_support::HANDOFF),
                )
                .expect("the original mixed fixture compiles");
                let emission = fsl_package::emit_checked(compiled.checked(), EmitLimits::fixture())
                    .expect("genuine mixed emission");
                let identities = emission
                    .identity_rows(
                        compiled.admitted(),
                        &mut V2ReadLimits::fixture().native_join_budget(),
                    )
                    .expect("native identity rows");
                let equality = emission
                    .equality_witness(
                        compiled.admitted(),
                        &mut V2ReadLimits::fixture().native_join_budget(),
                    )
                    .expect("native equality rows");
                let bare = BarePackage::new(compiled.admitted())
                    .with_supply(
                        BareSupply {
                            identities: &identities,
                            equality: Some(&equality),
                        },
                        &mut V2ReadLimits::fixture().native_join_budget(),
                    )
                    .expect("native mixed package");
                let form = bare
                    .protocol_execution_named("Flow")
                    .expect("native protocol view")
                    .expect("Flow");
                let sends = form
                    .controls
                    .iter()
                    .filter(|control| {
                        control.operation == fsl_package::protocol_view::ControlOperation::Send
                    })
                    .collect::<Vec<_>>();
                assert_eq!(sends.len(), 1, "the original fixture has exactly one send");
                let slot = bare
                    .select_protocol_slot(&ProtocolSlotDescriptor::new(
                        bare.node_key(&sends[0].node.node_id)
                            .expect("the native send key"),
                        "constraint".to_owned(),
                        None,
                    ))
                    .expect("native constraint selection");
                let origin = if position == "inside" {
                    "protocol Flow run Main Both left Left Only body"
                } else {
                    "protocol Flow run Main Only body"
                };
                assert_eq!(
                    slot.origin(),
                    Some(origin),
                    "the authored branch label remains in the site path"
                );
                bare.project_protocol_slot(&slot)
                    .expect("the real constraint pairs with its complete origin rows");
                package_path
            });
            assert!(item.get("exploration").is_some(), "{item}");
        }
    }
}

/// Equal payloads from nested branches retain their authored static paths
/// in queues, transition identities and identity-label canonical graphs.
#[trace("TC-2958", "FR-940-AC-6", "FR-894", "FR-205", "FSpec-FR-425")]
#[test]
fn package_parallel_send_preserves_message_identity() {
    use fsl_eval::simulation::protocol::{Sc, StepKind};
    use std::collections::BTreeSet;
    let sends = format!(
        "parallel Outer memory sc {{
            branch left sequence Left {{
                parallel Inner memory sc {{
                    branch inner sequence In {{ {} }}
                    branch empty sequence Empty {{ }}
                }} join all [inner, empty];
            }}
            branch right sequence Right {{ {} }}
        }} join all [left, right];",
        channel_send("Nested", "nested_payload", "nested_payload.n = 0"),
        channel_send("Other", "other_payload", "other_payload.n = 0"),
    );
    for cycle in [false, true] {
        let ws = Workspace::new(&format!("package-parallel-send-identity-{cycle}"));
        let path = channel_send_request(&ws, 2, "block", "exactly-once-premise", &sends, "closing");
        let source = analyze(&path);
        assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
        let graph = package_protocol_graph(
            &ws,
            &path,
            Sc,
            &["Planning", "Machining"],
            true,
            cycle,
            |_, _| {},
        );
        let mut nodes = BTreeSet::new();
        let mut payloads = BTreeSet::new();
        for transition in graph
            .iter()
            .flat_map(|state| &state.steps)
            .map(|(step, _)| step)
        {
            if transition.kind() != StepKind::Send {
                continue;
            }
            let message = transition.message().expect("send occurrence");
            assert_eq!(
                Some(message.node()),
                transition.node(),
                "the message names its actual send path"
            );
            assert_eq!(message.ordinal(), 0);
            assert!(!message.path().steps.is_empty());
            nodes.insert(message.node().to_owned());
            let payload = fsl_eval::simulation::protocol::send_record_json(
                transition.send_record().expect("send record"),
            )
            .expect("record JSON");
            payloads.insert(payload["payload"].to_string());
        }
        assert_eq!(nodes.len(), 2, "both equal-payload sends execute");
        assert_eq!(
            payloads.len(),
            1,
            "payload equality does not merge occurrences"
        );
        fn queued(value: &Value, nodes: &BTreeSet<String>) -> usize {
            match value {
                Value::Object(members) if value["type"] == "message-occurrence" => {
                    let node = value["node"].as_str().expect("static declaration path");
                    assert!(
                        nodes.contains(node),
                        "queue message uses authored path {node}"
                    );
                    1 + members
                        .values()
                        .map(|value| queued(value, nodes))
                        .sum::<usize>()
                }
                Value::Object(members) => members.values().map(|value| queued(value, nodes)).sum(),
                Value::Array(values) => values.iter().map(|value| queued(value, nodes)).sum(),
                _ => 0,
            }
        }
        assert!(
            graph
                .iter()
                .map(|state| queued(
                    &serde_json::from_slice(state.key.as_bytes()).expect("key JSON"),
                    &nodes
                ))
                .sum::<usize>()
                > 0
        );
        assert!(!ws.path("program.fsl").exists());
        let package = analyze(&ws.path("analyze-package.json"));
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// A projection of the model's singleton bounded Tally payload survives strict
/// admission and has the same analysis record through both public routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-093", "FR-094", "FSpec-FR-322-AC-31")]
#[test]
fn package_send_preserves_bounded_tally_projection() {
    let ws = Workspace::new("package-send-bounded-tally-projection");
    let original: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the original Fill model");
    let tally = original["types"]
        .as_array()
        .expect("model types")
        .iter()
        .find(|node| node["identity"] == "ix://test/fill/Tally")
        .expect("the original singleton Tally type")
        .clone();
    let mut model = finite_channel_model();
    let types = model["types"].as_array_mut().expect("model types");
    types.push(tally);
    let attempt = types
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Attempt")
        .expect("the original payload record");
    attempt["fields"][0]["typeRef"] = json!("ix://test/fill/Tally");
    let writer = crate::fill::attempt("Write", "set", "SetPost", "written", "true")
        .replace("by w", "by Planning");
    let reader = channel_send("Read", "observed_payload", "observed_payload.n <= k.v");
    let controls = format!(
        "parallel Both memory tso {{
            branch writer sequence Writer {{ {writer} }}
            branch reader sequence Reader {{ {reader} }}
        }} join all [writer, reader];"
    );
    let path = channel_request(
        &ws,
        model,
        "ordering unordered delivery exactly-once-premise capacity 1 overflow block",
        &controls,
        "closing",
    );
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("source unit");
    ws.write(
        "program.fsl",
        unit.replace(
            "protocol Flow",
            &format!("{} protocol Flow", crate::fill::contracts("true")),
        )
        .as_bytes(),
    );
    channel_send_parity_using(&ws, &path, "proved", || {
        let (package_path, _) = package_request(&ws, &path);
        fsl_command::checked_handoff::verify(&ws.path(crate::analyze_cli_support::HANDOFF))
            .expect("the strict reader admits the original Attempt.n: Tally (0..0) projection");
        package_path
    });
}

/// A compact package frame treats only the real over binder as receiver;
/// an aliased capture instead reads every matching object, even at slot 0.
#[trace("TC-2958", "FR-940-AC-6", "FR-220", "FR-155", "FSpec-FR-425")]
#[test]
fn package_parallel_send_preserves_weak_model_reads() {
    use fsl_eval::simulation::protocol::{Ra, Tso};
    for memory in ["tso", "ra"] {
        for capture in [false, true] {
            let ws = Workspace::new(&format!("package-parallel-send-reads-{memory}-{capture}"));
            let binder = if capture { "j" } else { "k" };
            let writer = crate::fill::attempt("Write", "set", "SetPost", "written", "true")
                .replace("by w", "by Planning");
            let run = format!(
                "parallel Both memory {memory} {{
                    branch writer sequence Writer {{ {writer} }}
                    branch reader sequence Reader {{ {} }}
                }} join all [writer, reader];",
                channel_send(
                    "Read",
                    "observed_payload",
                    &format!("observed_payload.n <= {binder}.v")
                ),
            );
            let model = finite_channel_model();
            let path = channel_request(
                &ws,
                model,
                "ordering unordered delivery exactly-once-premise capacity 1 overflow block",
                &run,
                "closing",
            );
            let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("source unit");
            let captures = if capture {
                "capture j: Reference<M::Cell> = k;"
            } else {
                ""
            };
            ws.write(
                "program.fsl",
                unit.replace("role Planning", &format!("{captures} role Planning"))
                    .replace(
                        "protocol Flow",
                        &format!("{} protocol Flow", crate::fill::contracts("true")),
                    )
                    .as_bytes(),
            );
            let source = analyze(&path);
            assert!(
                source.item(0).get("record").is_some(),
                "Source weak fixture {memory}/{capture}: {}",
                source.document(),
            );
            assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
            let expected = if capture { 2 } else { 1 };
            fn inspect<M: fsl_eval::simulation::protocol::MemoryModel>(
                source: &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
                package: &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
                expected: usize,
            ) {
                let path = "Main/Both/reader/Reader/Read";
                let reads = source.weak_accesses().node_reads(path);
                assert_eq!(
                    reads.len(),
                    expected,
                    "Source reads retain over versus capture distinction: {reads:?}"
                );
                assert_eq!(package.weak_accesses().node_reads(path), reads);
                assert_eq!(
                    package.weak_accesses().shared(),
                    source.weak_accesses().shared()
                );
            }
            if memory == "tso" {
                package_protocol_graph(
                    &ws,
                    &path,
                    Tso::default(),
                    &["Planning", "Machining"],
                    true,
                    false,
                    |s, p| inspect(s, p, expected),
                );
            } else {
                package_protocol_graph(
                    &ws,
                    &path,
                    Ra::default(),
                    &["Planning", "Machining"],
                    true,
                    false,
                    |s, p| inspect(s, p, expected),
                );
            }
            let package = analyze(&ws.path("analyze-package.json"));
            assert_eq!(package.item(0), source.item(0), "{}", package.document());
            assert_eq!(package.exit, source.exit);
        }
    }
}

/// A genuine mixed deadlock reproduces, and refusing its replay byte budget
/// retains the same typed code/cause on both intake routes.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-4", "FR-894")]
#[test]
fn package_parallel_send_preserves_replay_and_refusal() {
    let run = format!(
        "parallel Both memory sc {{
            branch left sequence Left {{ {} }}
            branch right sequence Right {{ {} }}
        }} join all [left, right];",
        channel_send("LeftSend", "left_payload", "left_payload.n = 0"),
        channel_send("RightSend", "right_payload", "right_payload.n = 0"),
    );
    let ws = Workspace::new("package-parallel-send-replay");
    let path = channel_send_request(&ws, 1, "block", "exactly-once-premise", &run, "closing");
    let item = channel_send_parity(&ws, &path, "refuted");
    assert_eq!(item["replay"]["reproduced"], true);
    assert!(item["counterexample"]["prefix"]
        .as_array()
        .expect("real deadlock trace")
        .iter()
        .any(|step| step["transition"]["step"] == "send"));

    let ws = Workspace::new("package-parallel-send-replay-refusal");
    let path = channel_send_request(&ws, 1, "block", "exactly-once-premise", &run, "closing");
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("bounded package bytes");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    assert_eq!(source.item(0)["record"]["cause"]["kind"], "replay-refused");
    assert_eq!(
        source.item(0)["record"]["cause"]["code"],
        "stage_limit_exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["cause"],
        "input-bytes-exceeded"
    );
    std::fs::remove_file(ws.path("program.fsl")).expect("source removed before package replay");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn channel_related_premise(ws: &Workspace, related: bool) {
    let unit = std::fs::read_to_string(ws.path("program.fsl")).expect("the source unit");
    std::fs::write(
        ws.path("program.fsl"),
        unit.replace(
            "channel Release",
            "relationship Owns = M::owns; channel Release",
        ),
    )
    .expect("select the relationship premise");
    let mut model: Value =
        serde_json::from_slice(&std::fs::read(ws.path("model.json")).expect("model document"))
            .expect("model JSON");
    crate::related_by_support::add_owns_relationship(&mut model);
    std::fs::write(ws.path("model.json"), model.to_string()).expect("write the related model");
    let mut snapshot: Value = serde_json::from_slice(
        &std::fs::read(ws.path("snapshot-0.json")).expect("the initial snapshot"),
    )
    .expect("snapshot JSON");
    let object = json!({"population": "ix://test/fill/cells", "key": "c"});
    let instances = if related {
        vec![json!({"source": object, "target": object})]
    } else {
        Vec::new()
    };
    snapshot["relationships"] = json!([{
        "relationship": "ix://test/fill/relationship/owns", "complete": true,
        "instances": instances
    }]);
    std::fs::write(ws.path("snapshot-0.json"), snapshot.to_string())
        .expect("write the relationship instances");
}

/// Compare real states and steps using the same relationship observation as the CLI.
fn channel_related_graph(ws: &Workspace, path: &std::path::Path, related: bool, receive: bool) {
    use fsl_eval::simulation::protocol::{Sc, StepKind};

    let snapshot = std::fs::read(ws.path("snapshot-0.json")).expect("the related snapshot");
    let graph = package_protocol_graph_with_snapshot(
        ws,
        path,
        Sc,
        PackageProtocolGraphOptions {
            roles: &["Planning", "Machining"],
            source_absent: false,
            cycle_keys: false,
            snapshot: Some(snapshot),
        },
        |_, _| {},
    );
    let queue_len = |at: usize| {
        let state: Value = serde_json::from_slice(graph[at].key.as_bytes()).expect("state key");
        state["queues"]["entries"]
            .as_array()
            .expect("queue map")
            .iter()
            .map(|entry| {
                entry["value"]["elements"]
                    .as_array()
                    .expect("queued messages")
                    .len()
            })
            .sum::<usize>()
    };
    let sends: Vec<_> = graph
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(step, _)| step.kind() == StepKind::Send)
        .collect();
    assert_eq!(
        !sends.is_empty(),
        receive || related,
        "a false send premise disables every candidate"
    );
    for (send, queued) in &sends {
        assert_eq!(
            send.node(),
            Some(if receive {
                "Main/Released"
            } else {
                "Main/Correlated"
            })
        );
        assert_eq!(
            queue_len(*queued),
            1,
            "the accepted send retains one message"
        );
        let record = fsl_eval::simulation::protocol::send_record_json(
            send.send_record().expect("send record"),
        )
        .expect("send record JSON");
        assert_eq!(record["outcome"], "accepted", "{record}");
        let fields = record["payload"]["fields"]
            .as_array()
            .expect("payload fields");
        assert_eq!(
            fields
                .iter()
                .find(|field| field["name"] == "n")
                .expect("n field")["value"]["value"],
            "0",
            "{record}"
        );
        if receive {
            assert_eq!(
                fields
                    .iter()
                    .find(|field| field["name"] == "ok")
                    .expect("ok field")["value"]["value"],
                true,
                "{record}"
            );
            assert_eq!(
                fields
                    .iter()
                    .find(|field| field["name"] == "target")
                    .expect("target field")["value"]["identity"],
                "c",
                "{record}"
            );
        }
    }
    let receives: Vec<_> = graph
        .iter()
        .flat_map(|state| &state.steps)
        .filter(|(step, _)| step.kind() == StepKind::Receive)
        .collect();
    assert_eq!(
        !receives.is_empty(),
        receive && related,
        "only a true receive premise permits delivery"
    );
    for (received, drained) in receives {
        assert_eq!(received.node(), Some("Main/Correlated"));
        let (sent, _) = sends
            .iter()
            .find(|(sent, _)| {
                sent.message() == received.delivery().map(|delivery| delivery.message())
            })
            .expect("delivery names an actual send occurrence");
        assert_eq!(
            filament_exact::compare_keys(
                received.binder_record().expect("received payload"),
                sent.binder_record().expect("sent payload"),
            ),
            Some(std::cmp::Ordering::Equal),
            "receive preserves the exact sent payload"
        );
        assert_eq!(queue_len(*drained), 0, "the receive drains the FIFO queue");
    }
    let finishes = graph
        .iter()
        .flat_map(|state| &state.steps)
        .any(|(step, _)| step.kind() == StepKind::Finish);
    assert_eq!(
        finishes, related,
        "the run finishes exactly when its correlation holds"
    );
    if !related {
        assert!(
            graph
                .iter()
                .enumerate()
                .any(|(at, state)| state.steps.is_empty() && queue_len(at) == usize::from(receive)),
            "a false premise waits with the original queue intact"
        );
    }
}

/// Send correlation is an enabling premise on both intake routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-731-AC-1", "FSpec-FR-831")]
#[test]
fn package_channel_send_related_premises_enforce_source_package_parity() {
    for (related, expected) in [(false, "refuted"), (true, "proved")] {
        let ws = Workspace::new(&format!("package-channel-send-related-{related}"));
        let path = channel_send_request(
            &ws,
            1,
            "block",
            "exactly-once-premise",
            "send Correlated via Release as (sent: M::Attempt) related by Owns (k, k) { sent.n = 0 };",
            "closing",
        );
        channel_related_premise(&ws, related);
        channel_related_graph(&ws, &path, related, false);
        let item = workflow_role_parity(&ws, &path, expected);
        if !related {
            assert_eq!(
                item["counterexample"]["blocked"][0]["rest"]["node"], "Main/Correlated",
                "{item}"
            );
            assert!(
                item["counterexample"]["prefix"]
                    .as_array()
                    .expect("blocked send witness")
                    .is_empty(),
                "{item}"
            );
        }
    }
}

fn parallel_request(ws: &Workspace, run: &str) -> std::path::PathBuf {
    let body = format!(
        "{}{}",
        crate::fill::contracts("false"),
        crate::fill::protocol("Flow", "role a on M::Cell;", run, "d")
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    // Both roles and every attempted operation use the Cell/Level closure.
    // Idle remains because the shared model declares its empty population.
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
            )
        });
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population")
            )
        });
    let model = serde_json::to_vec(&model).expect("the parallel model serializes");
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {
                "w": {"type": "model-binding", "selection": "test/fill"},
                "a": {"type": "model-binding", "selection": "test/fill"}
            }
        }}}}]}),
    )
}

fn parallel_attempts(join: &str) -> String {
    format!(
        "parallel Both {{ branch Machine {} branch Assemble {} }} {join};",
        crate::fill::attempt("Reserved", "setTwo", "TwoPre, TwoPost", "reserved", "true"),
        "attempt Assembled by a on M::Cell::touch contracts [TouchPre] as (assembled: Boolean) { true };"
    )
}

/// The complete two-attempt Any/cancel control-flow example, with one blocked
/// contract, distinguishes cancellation from leaving the other branch running.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-3", "FR-206-AC-6")]
#[test]
fn package_parallel_any_preserves_outstanding_and_joined_labels() {
    for (name, join, expected) in [
        (
            "cancel",
            "join any [Machine, Assemble] outstanding cancel",
            "proved",
        ),
        (
            "continue",
            "join any [Machine, Assemble] outstanding continue",
            "refuted",
        ),
        (
            "blocked-label",
            "join any [Machine] outstanding cancel",
            "refuted",
        ),
        (
            "ready-label",
            "join any [Assemble] outstanding cancel",
            "proved",
        ),
        ("all", "join all [Machine, Assemble]", "refuted"),
        ("all-ready", "join all [Machine, Assemble]", "proved"),
    ] {
        let ws = Workspace::new(&format!("package-parallel-any-{name}"));
        let mut run = parallel_attempts(join);
        if name == "all-ready" {
            run = run.replace("TwoPre, TwoPost", "TwoPost");
        }
        let path = parallel_request(&ws, &run);
        let source = analyze(&path);
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Cancelling an outer branch also cancels its inner running branches; keeping
/// them running prevents finish and leaves the same deadlock on both routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-206-AC-9", "FR-206-AC-6")]
#[test]
fn package_parallel_any_cancels_nested_running_branches() {
    for (outstanding, expected) in [("cancel", "proved"), ("continue", "refuted")] {
        let ws = Workspace::new(&format!("package-parallel-any-nested-{outstanding}"));
        let blocked = crate::fill::attempt("Blocked", "setTwo", "TwoPre, TwoPost", "b", "true");
        let ready = crate::fill::attempt("Ready", "touch", "TouchPre", "r", "true");
        let run = format!(
            "parallel Outer {{ branch fast {ready} branch nested parallel Inner {{ \
             branch left {blocked} branch right {} }} join all [left, right]; \
             }} join any [fast] outstanding {outstanding};",
            crate::fill::attempt("Other", "setTwo", "TwoPre, TwoPost", "o", "true")
        );
        let path = parallel_request(&ws, &run);
        let source = analyze(&path);
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Any/continue deadlocks retain the replay refusal's typed code and cause.
#[trace("TC-2958", "FR-940-AC-22", "FR-206-AC-3")]
#[test]
fn package_parallel_any_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-parallel-any-replay-bound");
    let path = parallel_request(
        &ws,
        &parallel_attempts("join any [Machine, Assemble] outstanding continue"),
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package analysis");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn terminal_request(
    ws: &Workspace,
    captures: &str,
    terminal: &str,
    blocked: bool,
) -> std::path::PathBuf {
    terminal_request_over(ws, captures, terminal, blocked, "Cell")
}

/// [`terminal_request`] with the protocol over `M::<over>`: `Cell`, or the
/// supertype `Base` that `Cell` then extends.
fn terminal_request_over(
    ws: &Workspace,
    captures: &str,
    terminal: &str,
    blocked: bool,
    over: &str,
) -> std::path::PathBuf {
    let run = if blocked {
        format!(
            "sequence Main {{ {} }}",
            crate::fill::attempt("Blocked", "setTwo", "TwoPre, TwoPost", "attempted", "true")
        )
    } else {
        "sequence Main {}".to_owned()
    };
    let body = format!(
        "{}protocol Flow using pp over (k: M::{over}) on origin {{
            {captures}
            role w on M::Cell;
            {terminal}
            run {run}
            finish Done as (d: Boolean) {{ d }};
        }}",
        crate::fill::contracts("false")
    );
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    // Select the terminal fixture's domain closure; the shared Fill corpus also
    // contains unrelated variant and compensation record declarations.
    let mut model: Value =
        serde_json::from_slice(&crate::fill::document()).expect("the Fill model is JSON");
    model["types"]
        .as_array_mut()
        .expect("model types")
        .retain(|node| {
            matches!(
                node["identity"].as_str(),
                Some("ix://test/fill/Level" | "ix://test/fill/Cell" | "ix://test/fill/Idle")
            )
        });
    model["constructs"]
        .as_array_mut()
        .expect("model constructs")
        .retain(|construct| {
            matches!(
                construct["kind"]["name"].as_str(),
                Some("value_type" | "object_type" | "population")
            )
        });
    if over == "Base" {
        add_base_supertype(&mut model);
    }
    let model = serde_json::to_vec(&model).expect("the terminal model serializes");
    crate::analyze_cli_support::request_over(
        ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "temporal-satisfaction",
        "temporal": {"item_kind": "deadlock-freedom", "subject": {"protocol": {
            "protocol": "Flow",
            "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
            "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}
        }}}}]}),
    )
}

/// Terminal declarations classify stopped live instances identically on both routes.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11", "FR-211-AC-2")]
#[test]
fn package_terminal_classifies_blocked_live_instances() {
    for (name, terminal, expected) in [
        ("absent", "", "refuted"),
        ("any", "terminal any;", "proved"),
        ("true", "terminal when { k.v = 0 };", "proved"),
        ("false", "terminal when { k.v = 1 };", "refuted"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-{name}"));
        let path = terminal_request(&ws, "", terminal, true);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source terminal {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert!(
                source.item(0)["counterexample"]["blocked"]
                    .as_array()
                    .is_some_and(|blocked| !blocked.is_empty()),
                "{}",
                source.document()
            );
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// A finished instance is intended even when its terminal predicate is false.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_finished_instances() {
    for (name, terminal) in [
        ("any", "terminal any; scheduling adversarial;"),
        ("false", "terminal when { k.v = 1 };"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-finished-{name}"));
        let path = terminal_request(&ws, "", terminal, false);
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source finished {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
        let package = package_run(&ws, &path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// Origin captures initialize in order from the admitted observation and feed terminal predicates.
#[trace("TC-2958", "FR-940-AC-6", "FR-211-AC-2", "FR-383-AC-5")]
#[test]
fn package_terminal_reads_origin_captures_in_authored_order() {
    for (name, predicate, expected) in [("true", "saved", "proved"), ("false", "later", "refuted")]
    {
        let ws = Workspace::new(&format!("package-terminal-captures-{name}"));
        let members = format!("terminal when {{ {predicate} }};");
        let path = terminal_request(
            &ws,
            "capture saved: Boolean = k.v = 0; capture later: Boolean = not saved;",
            &members,
            true,
        );
        let source = analyze(&path);
        assert!(
            source.item(0).get("record").is_some(),
            "source capture {name}: {}",
            source.document()
        );
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl"))
            .expect("remove source before package analysis");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), source.item(0), "{}", package.document());
        assert_eq!(package.exit, source.exit);
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Refusing replay retains the same typed cause for a terminal deadlock on both routes.
#[trace("TC-2958", "FR-940-AC-22", "FR-211-AC-2")]
#[test]
fn package_terminal_preserves_replay_refusal_code_and_cause() {
    let ws = Workspace::new("package-terminal-replay-bound");
    let path = terminal_request(
        &ws,
        "capture saved: Boolean = k.v = 1;",
        "terminal when { saved };",
        true,
    );
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert!(
        source.item(0).get("record").is_some(),
        "source replay refusal: {}",
        source.document()
    );
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let cause = &source.item(0)["record"]["cause"];
    assert_eq!(cause["kind"], "replay-refused");
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["cause"], "input-bytes-exceeded");
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Invalid deadlock subjects refuse before exploration with identical typed records.
#[trace("TC-2958", "FR-940-AC-6", "FR-850-AC-11")]
#[test]
fn package_terminal_invalid_subject_records_match_source() {
    for (name, member) in [
        ("missing", "temporal.subject.protocol"),
        ("mismatched", "temporal.subject.protocol"),
        ("capacity", "capacities"),
    ] {
        let ws = Workspace::new(&format!("package-terminal-invalid-{name}"));
        let path = terminal_request(&ws, "", "terminal any;", false);
        let mut document: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("source request"))
                .expect("request JSON");
        let subject = &mut document["request"]["items"][0]["temporal"]["subject"];
        match name {
            "missing" => {
                subject
                    .as_object_mut()
                    .expect("subject object")
                    .remove("protocol");
            }
            "mismatched" => subject["protocol"]["protocol"] = json!("Other"),
            "capacity" => subject["protocol"]["capacities"] = json!({"unused": "-1"}),
            _ => unreachable!("the listed malformed cases"),
        }
        std::fs::write(&path, document.to_string()).expect("write malformed selection");
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(item["category"], "refusal", "{}", source.document());
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["cause"], "invalid-value",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["member"], member,
            "{item}"
        );
        assert!(
            item.get("exploration").is_none(),
            "invalid selection explored states: {item}"
        );
        assert_eq!(source.exit, 20, "{}", source.document());
        let (package_path, _) = package_request(&ws, &path);
        std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package refusal");
        let package = analyze(&package_path);
        assert_eq!(package.item(0), item, "{}", package.document());
        assert_eq!(package.exit, source.exit);
    }
}

/// FR-940-AC-6: the store-buffering litmus claim under TSO has equal records,
/// counterexamples and package-byte replay on both analyze routes.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-tso");
    let source_request =
        litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// Release/acquire uses the same package protocol, effective memory model
/// and bytes-only counterexample replay as the source route.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_ra_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-ra");
    let mut extra = request_extra(json!([{"claim": "NoBothZero"}]));
    extra["memory"][0]["model"] = json!("ra");
    let source_request = litmus_request(&ws, &body, extra);
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0), source.item(0));
    assert_eq!(package.exit, source.exit);
}

/// FR-940-AC-6: the protocol's race-freedom result reads the same admitted
/// protocol and memory-model facts on the package route.
#[trace("TC-2958", "FR-940-AC-6", "TC-3050", "FR-850-AC-11")]
#[test]
fn the_tso_race_freedom_item_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-race-tso");
    let source_request = litmus_request(
        &ws,
        &body,
        request_extra(json!([{
            "claim": "SB::Both",
            "temporal": {"item_kind": "race-freedom"},
        }])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// FR-882-AC-5: alongside the source fallback oracle in
/// `weak_memory::an_access_footprint_is_stepped`, package execution uses the
/// same helper-call footprint and TSO verdict as source.
#[trace("TC-2506", "FR-882-AC-5")]
#[test]
fn a_helper_call_attempt_has_equal_tso_results_and_footprints() {
    use fsl_eval::model_subject::ProtocolSubject;
    use fsl_eval::simulation::model::{ModelSystem, PackageModelInput};
    use fsl_eval::simulation::protocol::{MemorySelection, ProtocolSystem, ResolvedMemory, Tso};
    use fsl_foundation::MemoryModelName;
    use fsl_package::{AnalysisView, BarePackage, BareSupply, EmitLimits, V2ReadLimits};
    use fsl_semantics::model::accounting::ModelNormalizationLimits;

    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} }} \
         branch right sequence R {{ {} }} }} join all [left, right];",
        crate::litmus::attempt("Sx", "c", "Core::look", "", "LooksY, Looked"),
        crate::litmus::attempt("Sy", "c", "Core::look", "", "LooksY, Looked"),
    );
    let direct_contracts = crate::litmus::contracts().replace(
        "pre LooksY using v on M::Core::look { c.id = 1 }",
        "pre LooksY using v on M::Core::look { c.id = 0 }",
    );
    let helper_contracts = direct_contracts.replace(
        "pre LooksY using v on M::Core::look { c.id = 0 }",
        "pre LooksY using v on M::Core::look { is_zero(c.id) }",
    );
    let body = format!(
        "function is_zero using v(x: Int[0, 3]): Boolean pure {{ x = 0 }}\n{helper_contracts}{}{}",
        crate::litmus::protocol("SB", &run),
        crate::litmus::NO_BOTH_ZERO,
    );
    let ws = Workspace::new("package-helper-attempt-footprint-tso");
    let selection = request_extra(json!([{"claim": "NoBothZero"}]));
    let source_request = litmus_request(&ws, &body, selection);
    let source_result = analyze(&source_request);
    assert!(
        source_result.item(0)["record"].is_object(),
        "source helper result: {}",
        source_result.document()
    );
    let package_result = package_run(&ws, &source_request);
    assert_eq!(
        package_result.item(0),
        source_result.item(0),
        "{}",
        package_result.document()
    );
    assert_eq!(package_result.exit, source_result.exit);

    let compiled =
        fsl_command::checked_handoff::compile(&ws.path(crate::analyze_cli_support::HANDOFF))
            .expect("the helper-call litmus fixture compiles");
    let documents = crate::litmus::Documents::new();
    let selection = [MemorySelection {
        parallel: "SB::Both".to_owned(),
        model: MemoryModelName::Tso,
    }];
    let source = documents.subject(compiled.checked(), "SB", &selection);
    let emission = fsl_package::emit_checked(compiled.checked(), EmitLimits::fixture())
        .expect("the helper-call package emits");
    let admitted = compiled.admitted();
    let identities = emission
        .identity_rows(admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("package identities admit");
    let equality = emission
        .equality_witness(admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("package equality admits");
    let bare = BarePackage::new(admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the package supply admits");
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package model admits");
    let mut model = documents
        .subject(compiled.checked(), "SB", &selection)
        .model;
    model.simulation.input = input.input();
    let execution = bare
        .protocol_execution_named("SB")
        .expect("the package protocol reads")
        .expect("the package carries SB");
    let package = ProtocolSubject {
        model,
        protocol: source.protocol.clone(),
        over: source.over.clone(),
        roles: source.roles.clone(),
        capacities: source.capacities.clone(),
        memory: ResolvedMemory::resolve_package(&execution, &selection)
            .expect("the package TSO selection resolves"),
    };
    let source_system = ProtocolSystem::new(
        ModelSystem::new(source.model.simulation.clone()).expect("source model admits"),
        &source,
        Tso::default(),
    )
    .expect("source protocol builds");
    let package_system = ProtocolSystem::new_package(
        ModelSystem::new(package.model.simulation.clone()).expect("package model admits"),
        &package,
        Tso::default(),
    )
    .expect("package protocol builds");
    for step in ["Both/left/L/Sx", "Both/right/R/Sy"] {
        let source_attempt = source_system
            .weak_accesses()
            .attempt(step)
            .expect("source helper-call attempt is classified");
        let package_attempt = package_system
            .weak_accesses()
            .attempt(step)
            .expect("package helper-call attempt is classified");
        assert_eq!(
            package_attempt.instances, source_attempt.instances,
            "package helper-call footprint for {step}",
        );
    }
}

/// FR-850-AC-11: the same SB protocol with no request memory member uses its
/// source `sc` default on both analyze routes, under which the litmus claim
/// holds.
#[trace("TC-3050", "FR-850-AC-11")]
#[test]
fn the_default_sc_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-litmus-default-sc");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
}

/// A later attempt retains the earlier attempt's result binder while its
/// constraint reads it, on both protocol routes.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_later_package_attempt_can_read_an_earlier_result_binder() {
    let run = "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { not a }; \
        }";
    let body = sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    );
    let ws = Workspace::new("package-protocol-binder-lifetime");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(source.exit, 0, "{}", source.document());

    let package = package_run(&ws, &source_request);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

fn true_claim(run: &str) -> String {
    sb_unit(run).replace(
        crate::litmus::NO_BOTH_ZERO,
        "temporal BinderLifetime using t0 over (k: M::Core) clock \
         \"model-steps\" on origin { always holds(true) }\n",
    )
}

/// Finish retains a result binder and observes the current model state.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_finish_reads_the_previous_result_and_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        }",
    )
    .replace("{ d };", "{ d and not a and k.r1 = 0 };");
    let ws = Workspace::new("package-protocol-finish-binders");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// An attempt's object reference is admitted against its current observation.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_attempt_constraint_reads_the_current_model() {
    let body = true_claim(
        "sequence Main { \
        attempt A by c on M::Core::look contracts [LooksY, Looked] as (a: Boolean) { true }; \
        attempt B by c on M::Core::copy contracts [CopyR] as (b: Boolean) { k.r1 = 0 }; \
        }",
    );
    let ws = Workspace::new("package-protocol-attempt-observation");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "BinderLifetime"}])),
    );
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// The protocol replay shares the request's package-byte ceiling on both routes.
#[trace("TC-2958", "FR-940-AC-22")]
#[test]
fn package_protocol_replay_obeys_the_request_input_bytes_bound() {
    let ws = Workspace::new("package-protocol-replay-input-bytes");
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let path = litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let (package_path, bytes) = package_request(&ws, &path);
    let below = u64::try_from(bytes.len() / 2).expect("the package length");
    let source = crate::analyze_cli_support::analyze_under_replay_bytes(&path, below);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    assert_eq!(source.item(0)["record"]["cause"]["kind"], "replay-refused");
    assert_eq!(
        source.item(0)["record"]["cause"]["code"],
        "stage_limit_exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["cause"],
        "input-bytes-exceeded"
    );
    assert_eq!(
        source.item(0)["record"]["cause"]["setting"],
        "replay.input_bytes"
    );
    let package = crate::analyze_cli_support::analyze_under_replay_bytes(&package_path, below);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// Every memory entry is admitted before items run, including unused protocols.
#[trace("TC-2959", "FR-1033-AC-4", "FR-850-AC-11")]
#[test]
fn bad_package_memory_entries_refuse_the_whole_request() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    for (index, memory) in [
        json!([{"parallel": "Nope::Both", "model": "tso"}]),
        json!([{"parallel": "SB::Both::left::L::Sx", "model": "tso"}]),
        json!([{"parallel": "SB::Both", "model": "tso"}, {"parallel": "SB::Both", "model": "ra"}]),
    ]
    .into_iter()
    .enumerate()
    {
        let ws = Workspace::new(&format!("package-invalid-memory-{index}"));
        let path = litmus_request(
            &ws,
            &body,
            json!({"subject": subject(), "memory": memory, "items": [{"claim": "NoBothZero"}]}),
        );
        let source = analyze(&path);
        assert_eq!(source.exit, 20, "{}", source.document());
        assert_eq!(source.document()["items"], json!([]));
        let package = package_run(&ws, &path);
        assert_eq!(package.exit, 20, "{}", package.document());
        assert_eq!(package.document()["items"], json!([]));
        assert_eq!(
            package.document()["diagnostics"],
            source.document()["diagnostics"]
        );
        assert_eq!(
            package.document()["category"],
            source.document()["category"]
        );
    }
}

/// A valid memory entry for another protocol leaves this subject's identity alone.
#[trace("TC-2959", "FR-1033", "FR-850-AC-11")]
#[test]
fn another_protocols_memory_does_not_change_the_package_subject() {
    let run = sb_run("", "release", "acquire");
    let body = sb_unit(&run) + &crate::litmus::protocol("Other", &run);
    let ws = Workspace::new("package-other-protocol-memory");
    let path = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let baseline = analyze(&path);
    assert_eq!(
        label(&baseline.item(0)),
        "proved",
        "{}",
        baseline.document()
    );
    let path = litmus_request(
        &ws,
        &body,
        json!({"subject": subject(), "memory": [{"parallel": "Other::Both", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    );
    let source = analyze(&path);
    assert_eq!(source.item(0), baseline.item(0), "{}", source.document());
    let package = package_run(&ws, &path);
    assert_eq!(package.item(0), source.item(0), "{}", package.document());
    assert_eq!(package.exit, source.exit);
}

/// `Cell` extends a `Base` with no fields.
fn add_base_supertype(model: &mut Value) {
    const BASE: &str = "ix://test/fill/Base";
    let types = model["types"].as_array_mut().expect("model types");
    let cell = types
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/fill/Cell")
        .expect("Cell is declared");
    cell["supertypes"] = json!([BASE]);
    let mut base = cell.clone();
    base["identity"] = json!(BASE);
    base["displayName"] = json!(BASE);
    base["supertypes"] = json!([]);
    base["fields"] = json!([]);
    base["operations"] = json!([]);
    base["origin"] = json!({"generated": {"generatorIdentity": BASE,
        "generatorVersion": "1.0.0", "inputIdentities": [BASE]}});
    types.push(base);
}

/// FR-125 "Instancing": on the source and the package route, a refuted
/// protocol claim over `M::Base` names its `over` object with the admitted
/// type `Cell`.
#[trace("TC-520", "FR-125-AC-9")]
#[test]
fn a_protocol_over_subject_names_the_admitted_member_type_on_both_routes() {
    let ws = Workspace::new("package-over-admitted-type");
    let path = terminal_request_over(&ws, "", "", true, "Base");
    let source = analyze(&path);
    let (package_path, _) = package_request(&ws, &path);
    std::fs::remove_file(ws.path("program.fsl")).expect("remove source before package analysis");
    let package = analyze(&package_path);
    for run in [&source, &package] {
        let item = run.item(0);
        assert_eq!(label(&item), "refuted", "{}", run.document());
        let over = &item["counterexample"]["subject"]["over"];
        assert_eq!(over["type"], "reference", "{}", run.document());
        assert_eq!(
            over["object_type"]["original"],
            json!({"node": "ix://test/fill/Cell", "package": "test/fill"}),
            "{}",
            run.document()
        );
        assert!(over.get("population").is_none(), "{over}");
        assert_eq!(item["replay"]["reproduced"], true);
    }
}

/// FR-125 "Instancing": on the source and the package constructor, a
/// replicated role over `M::Base` spawns objects of the admitted type `Cell`.
#[trace("TC-520")]
#[test]
fn a_replicated_role_object_names_the_admitted_member_type_on_both_routes() {
    use fsl_eval::simulation::protocol::{Sc, StepKind};

    let ws = Workspace::new("package-role-admitted-type");
    let path = workflow_roles_request(&ws, 1, "workflow", false, "", "closing");
    let unit = std::fs::read_to_string(ws.path("program.fsl"))
        .expect("the role unit")
        .replace("over (k: M::Cell)", "over (k: M::Base)")
        .replace("each M::Cell from", "each M::Base from");
    ws.write("program.fsl", unit.as_bytes());
    let mut model: Value =
        serde_json::from_slice(&std::fs::read(ws.path("model.json")).expect("the model"))
            .expect("the model is JSON");
    add_base_supertype(&mut model);
    // A role takes a type its population names, so `cells` names `Base` too.
    for population in model["populations"].as_array_mut().expect("populations") {
        population["members"]
            .as_array_mut()
            .expect("members")
            .push(json!("ix://test/fill/Base"));
    }
    ws.write("model.json", model.to_string().as_bytes());
    package_protocol_graph(
        &ws,
        &path,
        Sc,
        &["Planning"],
        false,
        false,
        |source, package| {
            let one = std::num::NonZeroUsize::new(1).expect("one worker");
            for system in [source, package] {
                let reached = crate::fill::reachable_with_workers(system, one).0;
                let spawned: Vec<String> = reached
                    .iter()
                    .flat_map(|state| &state.steps)
                    .filter(|(step, _)| step.kind() == StepKind::Spawn)
                    .map(|(step, _)| {
                        let bytes =
                            quire_canonical::to_vec(step, quire_canonical::Limits::new(u64::MAX))
                                .expect("transition identity");
                        String::from_utf8(bytes).expect("canonical JSON")
                    })
                    .collect();
                assert!(!spawned.is_empty(), "a worker spawns");
                for step in spawned {
                    let step: Value = serde_json::from_str(&step).expect("canonical JSON");
                    assert_eq!(
                        step["object"]["object_type"]["original"],
                        json!({"node": "ix://test/fill/Cell", "package": "test/fill"}),
                        "{step}"
                    );
                }
            }
        },
    );
}
