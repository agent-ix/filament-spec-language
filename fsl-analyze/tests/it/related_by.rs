// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-731 (TC-2023): a `related by` clause is a premise of its event step.
//! The fixture extends `fill`'s model with the relationship `owns` from a
//! `Cell` to a `Cell`, and a snapshot in which `c` owns `d` and nothing else.
//! A protocol over `c` whose event `Pick` binds an `M::PeerAttempt` record
//! (one `p: M::Cell` field) and is `related by Owns (k, r.p)` may step only
//! with `p = d`; a replayed `Pick` with `p = c` matches no enabled step.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_analyze::model_check::{check_protocol, ProtocolCheckRequest};
use fsl_eval::model_subject::{ProtocolSubject, RoleBinding};
use fsl_eval::simulation::model::ExpansionLimits;
use fsl_eval::simulation::protocol::{ProtocolTransition, StepKind};
use fsl_eval::simulation::SubjectSystem;
use fsl_foundation::diagnostic::Code;
use fsl_package::CheckedPackage;
use fsl_replay::{
    replay_protocol_trace, ModelCheckItem, ModelCheckLimits, ModelTraceSubject,
    ProtocolTraceSubject, ReplayLimits, ReplayRefusal,
};
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::DocumentRef;
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::counter::Letters;
use crate::fill::{
    attempt, contracts, document, instance, json_of, model_subject, package_over, protocol,
    reachable, sc_system, source_memory, unit_text_with, universe, Documents, PACKAGE,
};
use crate::model_settle::{envelope_at, protocol_anchor, request};
use crate::protocol_system::violated;

const NO_ATOMS: Letters = Letters { atom: |_| None };

fn id(suffix: &str) -> String {
    format!("ix://{PACKAGE}/{suffix}")
}

/// `fill`'s model with the relationship `owns`, `Cell` to `Cell`.
fn related_document() -> Vec<u8> {
    let mut doc: Json = serde_json::from_slice(&document()).expect("the fill model is JSON");
    let cell = doc["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == json!(id("Cell")))
        .expect("the Cell type");
    let many = json!({"lower": 0, "upper": 2, "ordered": false, "unique": true});
    cell["relationships"] = json!([{
        "identity": id("relationship/owns"), "category": "structural", "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {"sourceIdentity": id("spec"), "path": "model.fspec",
            "startLine": 1, "startColumn": 1}},
        "sourceEnd": {"type": id("Cell"), "role": "owner", "multiplicity": many},
        "targetEnd": {"type": id("Cell"), "role": "owned", "multiplicity": many}
    }]);
    doc.to_string().into_bytes()
}

/// The related model with an operation that returns one finite `Cell` value.
fn related_attempt_document() -> Vec<u8> {
    let mut doc: Json =
        serde_json::from_slice(&related_document()).expect("the related model is JSON");
    let cell = doc["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == json!(id("Cell")))
        .expect("the Cell type");
    cell["operations"]
        .as_array_mut()
        .expect("operations")
        .push(json!({
            "identity": id("Cell/pickPeer"), "name": "pickPeer", "params": [],
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": id("spec"), "path": "model.fspec",
                "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": [], "modifies_receiver": [], "creates": [], "deletes": []},
            "returns": {"typeRef": id("Cell"), "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}}
        }));
    doc.to_string().into_bytes()
}

/// The documents: the related model and one snapshot of cells `c` and `d`
/// in which `c` owns `d`.
fn related_documents(model: &[u8]) -> (Documents, DocumentRef) {
    let mut documents = Documents {
        packages: package_input([model]).expect("the related model admits"),
        snapshots: BTreeMap::new(),
    };
    let cell = |key: &str| {
        json!({"key": key, "type": id("Cell"),
            "fields": {"v": {"integer": "0"}, "w": {"integer": "0"}}})
    };
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "owned"},
        "observation": "current", "model": {"identity": PACKAGE},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [
            {"population": id("cells"), "complete": true, "objects": [cell("c"), cell("d")]},
            {"population": id("idles"), "complete": true, "objects": []}
        ],
        "relationships": [{"relationship": id("relationship/owns"), "complete": true,
            "instances": [{"source": {"population": id("cells"), "key": "c"},
                "target": {"population": id("cells"), "key": "d"}}]}]
    });
    let bytes = snapshot.to_string().into_bytes();
    documents.snapshots.insert(
        fsl_semantics::model::observation::document_reference(&bytes)
            .expect("a document names itself"),
        bytes,
    );
    let start = DocumentRef {
        authority: "test".to_owned(),
        identity: "owned".to_owned(),
    };
    (documents, start)
}

/// The unit body: `Pick` (with `clause` after its binder) then an attempt
/// no record satisfies, so every behaviour deadlocks after `Pick`.
fn body(clause: &str) -> String {
    let run = format!(
        "sequence Main {{ event Pick by w as (r: M::PeerAttempt) {clause} {{ true }}; {} }}",
        attempt("Stuck", "touch", "TouchPre", "s", "false")
    );
    contracts("self.v = 0") + &protocol("P", "relationship Owns = M::owns;", &run, "d")
}

fn attempt_body(related: bool) -> String {
    let clause = if related {
        " related by Owns (k, r)"
    } else {
        ""
    };
    let run = format!(
        "sequence Main {{ attempt Pick by w on M::Cell::pickPeer contracts [PickPeerPre] as (r: M::Cell){clause} {{ true }}; {} }}",
        attempt("Stuck", "touch", "TouchPre", "s", "false")
    );
    contracts("self.v = 0")
        + "pre PickPeerPre using v on M::Cell::pickPeer { true }\n"
        + &protocol("P", "relationship Owns = M::owns;", &run, "true")
}

fn subject<'a>(
    checked: &'a CheckedPackage,
    documents: &'a Documents,
    start: &DocumentRef,
) -> ProtocolSubject<'a> {
    ProtocolSubject {
        model: model_subject(checked, documents, vec![start.clone()], &["c", "d"]),
        protocol: "P".to_owned(),
        over: instance("c"),
        roles: vec![RoleBinding {
            role: "w".to_owned(),
            model: PACKAGE.to_owned(),
        }],
        capacities: Vec::new(),
        memory: source_memory(checked, "P"),
    }
}

/// The `Pick` transitions of the initial state, with their identity JSON.
fn picks(subject: &ProtocolSubject<'_>) -> Vec<(ProtocolTransition, Json)> {
    let system = sc_system(subject);
    let reached = reachable(&system);
    reached[0]
        .steps
        .iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Event)
        .map(|(transition, _)| {
            let key = json_of(&system.transition_key(transition).expect("an identity"));
            (transition.clone(), key)
        })
        .collect()
}

/// FR-731-AC-1 and FR-731-AC-3: `Pick` steps only with the peer `c` owns,
/// and a replayed `Pick` binding the unrelated peer refuses
/// `invalid_runtime_input`/`invalid-value`.
#[trace(
    "TC-2023",
    "FR-731-AC-1",
    "FR-731-AC-3",
    "FSpec-TC-1522",
    "FSpec-FR-051-AC-9"
)]
#[test]
fn a_related_by_clause_enables_only_related_steps_and_refuses_an_unrelated_replay() {
    let model = related_document();
    let (documents, start) = related_documents(&model);

    let free_body = body("");
    let free = package_over(model.clone(), "package", "", &free_body);
    let free_picks = picks(&subject(&free, &documents, &start));
    assert_eq!(free_picks.len(), 2, "one Pick per peer: {free_picks:?}");

    let related_body = body("related by Owns (k, r.p)");
    let related = package_over(model.clone(), "package", "", &related_body);
    let related_subject = subject(&related, &documents, &start);
    let related_picks = picks(&related_subject);
    assert_eq!(
        related_picks.len(),
        1,
        "only the peer c owns: {related_picks:?}"
    );
    assert!(
        related_picks[0].1["record"].to_string().contains("\"d\""),
        "the related peer is d: {related_picks:?}"
    );

    // A deadlock counterexample through the related `Pick`, with that step
    // replaced by the free protocol's `Pick` of the unrelated peer `c`.
    let outcome = check_protocol(
        ProtocolCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(&related, &documents, &start),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the related protocol checks");
    let counterexample = violated(&outcome);
    let at = counterexample
        .prefix
        .iter()
        .position(|step| step.transition.kind() == StepKind::Event)
        .expect("the counterexample takes Pick");
    let (unrelated, _) = free_picks
        .iter()
        .find(|(_, key)| !key["record"].to_string().contains("\"d\""))
        .expect("the free protocol picks c");
    let mut forged = counterexample.clone();
    forged.prefix[at].transition = unrelated.clone();

    let unit = unit_text_with("", &related_body).into_bytes();
    let (node, occurrence) = protocol_anchor(&related, "P");
    let replayed = replay_protocol_trace(
        request(&related, &[], &model, documents.snapshots.iter(), &unit),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: vec![start.clone()],
                universes: universe(&["c", "d"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            protocol: "P".to_owned(),
            over: instance("c"),
            roles: related_subject.roles.clone(),
            capacities: Vec::new(),
            memory: related_subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope_at(forged, node, occurrence),
        &NO_ATOMS,
        ReplayLimits::default(),
    );
    let Err(ReplayRefusal::ModelTrace(refusal)) = replayed else {
        panic!("an unrelated Pick refuses: {replayed:?}");
    };
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::InvalidRuntimeInput, "invalid-value")
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "invalid_runtime_input/invalid-value: protocol step {at} at `Main/Pick` fails related-by clause `Owns`"
        )
    );
}

/// FR-731-AC-3: a node constraint refusal is not a relationship refusal.
#[trace("TC-2023", "FR-731-AC-3")]
#[test]
fn replay_does_not_call_a_false_node_constraint_a_related_by_failure() {
    let model = related_document();
    let (documents, start) = related_documents(&model);
    let related_body = body("related by Owns (k, r.p)");
    let related = package_over(model.clone(), "package", "", &related_body);
    let related_subject = subject(&related, &documents, &start);
    let outcome = check_protocol(
        ProtocolCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(&related, &documents, &start),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the related protocol checks");
    let counterexample = violated(&outcome);
    let at = counterexample
        .prefix
        .iter()
        .position(|step| step.transition.kind() == StepKind::Event)
        .expect("the counterexample takes Pick");

    let false_body = body("related by Owns (k, r.p)").replace("{ true }", "{ false }");
    let false_related = package_over(model.clone(), "package", "", &false_body);
    let (node, occurrence) = protocol_anchor(&false_related, "P");
    let replayed = replay_protocol_trace(
        request(
            &false_related,
            &[],
            &model,
            documents.snapshots.iter(),
            &unit_text_with("", &false_body).into_bytes(),
        ),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: vec![start.clone()],
                universes: universe(&["c", "d"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            protocol: "P".to_owned(),
            over: instance("c"),
            roles: related_subject.roles.clone(),
            capacities: Vec::new(),
            memory: related_subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope_at(counterexample, node, occurrence),
        &NO_ATOMS,
        ReplayLimits::default(),
    );
    let Err(ReplayRefusal::ModelTrace(refusal)) = replayed else {
        panic!("a false node constraint refuses: {replayed:?}");
    };
    assert!(
        matches!(*refusal, fsl_replay::ModelTraceRefusal::ProtocolNotEnabled { step, .. } if step == at),
        "the unrelated cause keeps the ordinary not-enabled refusal: {refusal:?}"
    );
}

/// FR-731-AC-3: the result binder of an attempt is the observed value for its
/// relationship premise during replay diagnosis, not its pre-step slot value.
#[trace("TC-2023", "FR-731-AC-3")]
#[test]
fn replay_names_a_failed_relation_on_an_attempt_result_binder() {
    let model = related_attempt_document();
    let (mut documents, start) = related_documents(&model);
    let snapshot = documents
        .snapshots
        .values_mut()
        .next()
        .expect("one snapshot");
    let mut snapshot_json: Json = serde_json::from_slice(snapshot).expect("snapshot is JSON");
    snapshot_json["relationships"][0]["instances"] = json!([]);
    *snapshot = snapshot_json.to_string().into_bytes();

    let free_body = attempt_body(false);
    let free = package_over(model.clone(), "package", "", &free_body);
    let outcome = check_protocol(
        ProtocolCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(&free, &documents, &start),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
            limits: ModelCheckLimits::default(),
            ignore_memory_constraints: false,
        },
        &Cancel::new(),
    )
    .expect("the free attempt protocol checks");
    let counterexample = violated(&outcome);
    let at = counterexample
        .prefix
        .iter()
        .position(|step| {
            step.transition.kind() == StepKind::Attempt
                && step.transition.node() == Some("Main/Pick")
        })
        .expect("the counterexample takes Pick");
    assert!(
        counterexample.prefix[at]
            .transition
            .binder_record()
            .is_some(),
        "the attempt result binder is carried into replay diagnosis"
    );

    let related_body = attempt_body(true);
    let related = package_over(model.clone(), "package", "", &related_body);
    let related_subject = subject(&related, &documents, &start);
    let (node, occurrence) = protocol_anchor(&related, "P");
    let replayed = replay_protocol_trace(
        request(
            &related,
            &[],
            &model,
            documents.snapshots.iter(),
            &unit_text_with("", &related_body).into_bytes(),
        ),
        &ProtocolTraceSubject {
            model: ModelTraceSubject {
                timed: None,
                initial: vec![start.clone()],
                universes: universe(&["c", "d"]),
                expansion: ExpansionLimits {
                    max_candidates: 1_000_000,
                },
            },
            protocol: "P".to_owned(),
            over: instance("c"),
            roles: related_subject.roles.clone(),
            capacities: Vec::new(),
            memory: related_subject.memory.clone(),
            max_store_buffer: None,
            max_messages: None,
            max_live_instances: None,
        },
        &envelope_at(counterexample, node, occurrence),
        &NO_ATOMS,
        ReplayLimits::default(),
    );
    let Err(ReplayRefusal::ModelTrace(refusal)) = replayed else {
        panic!("an attempt with an unrelated result refuses: {replayed:?}");
    };
    assert_eq!(
        refusal.to_string(),
        format!(
            "invalid_runtime_input/invalid-value: protocol step {at} at `Main/Pick` fails related-by clause `Owns`"
        )
    );
}

/// FR-731-AC-2: a second clause no peer satisfies leaves `Pick` with no
/// step, and the two clauses in either order give the same states.
#[trace("TC-2023", "FR-731-AC-2")]
#[test]
fn an_unsatisfiable_clause_leaves_no_step_in_either_order() {
    let model = related_document();
    let (documents, start) = related_documents(&model);
    let mut reached_counts = Vec::new();
    for clauses in [
        "related by Owns (k, r.p) related by Owns (r.p, k)",
        "related by Owns (r.p, k) related by Owns (k, r.p)",
    ] {
        let checked = package_over(model.clone(), "package", "", &body(clauses));
        let subject = subject(&checked, &documents, &start);
        assert!(picks(&subject).is_empty(), "{clauses}: no Pick step");
        reached_counts.push(reachable(&sc_system(&subject)).len());
    }
    assert_eq!(reached_counts[0], reached_counts[1], "{reached_counts:?}");
}
