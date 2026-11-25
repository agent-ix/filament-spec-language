// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared replay fixture APIs are consumed by separate integration binaries"
)]
//! Shared model replay helpers used by integration test targets.

use crate::counter::{
    document, package_with_mode, subject, unit_text, universe, Documents, Letters, UNLIMITED,
};
use filament_exact::{Cancel, Identifier, NodeKey, Origin, Role};
use filament_semantic_value::checking::CheckMode;
use fsl_analyze::model_check::{check_model, ModelCheckRequest};
use fsl_eval::simulation::model::ExpansionLimits;
use fsl_eval::simulation::SubjectTransition;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::CheckedPackage;
use fsl_replay::{
    replay_model_trace, CertificateRequest, EnvelopeSource, ModelCheckItem, ModelCheckLimits,
    ModelCheckOutcome, ModelTraceReplayResult, ModelTraceSubject, QualifiedName, ReplayLimits,
    ReplayRefusal, ReplayRequestWire, ReplaySource, StateEnvironment, TemporalCounterexample,
    Witness, WitnessEnvelope, WitnessPacket,
};

/// Typed allocator boundary failure, without forcing host OOM or mocking a run.
pub(crate) fn allocation_refusal(requested: usize) -> ReplayRefusal {
    use fsl_semantics::model::observation::{AdmissionFailure, AdmissionRecord};
    ReplayRefusal::Admission(AdmissionFailure::Incomplete(Box::new(AdmissionRecord {
        catalog: fsl_foundation::diagnostic::ALLOCATION_FAILED,
        fields: std::collections::BTreeMap::from([("requested", requested.to_string())]),
        path: Vec::new(),
        kernel: None,
    })))
}

pub(crate) fn assert_allocation_outcome(records: &[fsl_replay::TerminalRecord], requested: usize) {
    let document = fsl_analyze::analyze::analyze_document(records, &[]).unwrap();
    assert_eq!(document.category().exit_code(), 22);
    assert_eq!(
        records[0].diagnostic(),
        Some(fsl_foundation::diagnostic::ALLOCATION_FAILED)
    );
    assert_eq!(
        records[0].diagnostic_detail(),
        Some(&serde_json::Map::from_iter([(
            "requested".to_owned(),
            serde_json::json!(requested.to_string())
        )]))
    );
    let document = serde_json::to_value(document).unwrap();
    assert_eq!(
        document["items"][0]["diagnostic"]["cause"],
        "allocation-failed"
    );
    assert_eq!(
        document["items"][0]["diagnostic"]["detail"],
        serde_json::json!({"requested": requested.to_string()})
    );
}

fn witness_source() -> ReplaySource {
    ReplaySource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

fn envelope_witness_source() -> EnvelopeSource {
    EnvelopeSource::Witness(
        Witness::parse("<<<assertion|model_check|deadlock|>>>").expect("a transcript"),
    )
}

/// The letters of a replay that reads no atom: the deadlock-freedom item has
/// none.
pub(crate) const NO_ATOMS: Letters = Letters { atom: |_| None };

/// A compiled Counter unit with its documents, initial snapshots and the
/// engine's deadlock-freedom outcome over universe `{c}`.
pub(crate) struct Case {
    pub(crate) unit: Vec<u8>,
    pub(crate) documents: Documents,
    pub(crate) initial: Vec<fsl_semantics::model::observation::DocumentRef>,
    pub(crate) checked: CheckedPackage,
    pub(crate) outcome: ModelCheckOutcome,
}

impl Case {
    /// The deadlock-freedom item `analyze` runs (FR-281): the run, its
    /// closure checker's request, and the
    /// counterexample replay of [`replay`] over this case's source.
    pub(crate) fn model_item(&self) -> fsl_analyze::analyze::ModelItem<'_> {
        fsl_analyze::analyze::ModelItem {
            request: ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &[],
                subject: subject(
                    &self.checked,
                    &self.documents,
                    self.initial.clone(),
                    &["c"],
                    UNLIMITED,
                ),
                item: ModelCheckItem::DeadlockFreedom,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            stutter_closure: false,
            certificate: self.request(),
            replay: Box::new(|payload| replay(self, payload.clone())),
        }
    }

    /// The case's checked package.
    pub(crate) fn checked(&self) -> &CheckedPackage {
        &self.checked
    }

    /// The deadlock-freedom item's certificate request over the case's
    /// subject.
    pub(crate) fn request(&self) -> CertificateRequest<'_> {
        CertificateRequest {
            temporal_clock_selections: &[],
            subject: subject(
                &self.checked,
                &self.documents,
                self.initial.clone(),
                &["c"],
                UNLIMITED,
            ),
            item: ModelCheckItem::DeadlockFreedom,
            smt_encoding: fsl_replay::SmtEncodingLimits::default(),
        }
    }
}

pub(crate) fn case(body: &str, values: &[i64]) -> Case {
    case_in_mode(body, values, CheckMode::Linked)
}

/// [`case`] over a body whose `functions` (unit-level declarations) are checked
/// in `CheckMode::Kernel`, so a function whose divisor may be zero is checked
/// by the Kernel (FR-495-AC-3).
pub(crate) fn kernel_case(body: &str, functions: &str, values: &[i64]) -> Case {
    case_in_mode(&format!("{body}{functions}"), values, CheckMode::Kernel)
}

fn case_in_mode(body: &str, values: &[i64], mode: CheckMode) -> Case {
    let document = document(false);
    let unit = unit_text(body).into_bytes();
    let checked = package_with_mode(&document, body, mode);
    let mut documents = Documents::new(document);
    let initial: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(index, value)| documents.start(&format!("s{index}-{value}"), &[("c", *value)]))
        .collect();
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &[],
            subject: subject(&checked, &documents, initial.clone(), &["c"], UNLIMITED),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            smt_encoding: Default::default(),
            smt_run: Default::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("checked");
    Case {
        unit,
        documents,
        initial,
        checked,
        outcome,
    }
}

pub(crate) fn trace_subject(case: &Case) -> ModelTraceSubject {
    ModelTraceSubject {
        timed: None,
        initial: case.initial.clone(),
        universes: universe(&["c"]),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        stutter_closure: false,
    }
}

pub(crate) fn replay(
    case: &Case,
    payload: TemporalCounterexample,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    replay_model_trace(
        request(
            &case.checked,
            &[],
            &case.documents.document,
            case.documents.snapshot_bytes(),
            &case.unit,
        ),
        &trace_subject(case),
        &envelope(payload),
        &NO_ATOMS,
        ReplayLimits::default(),
    )
}

pub(crate) fn counterexample(case: &Case) -> TemporalCounterexample {
    match &case.outcome {
        ModelCheckOutcome::Violated { counterexample, .. } => counterexample.as_ref().clone(),
        other => panic!("expected a violation, got {other:?}"),
    }
}

fn selected() -> QualifiedName {
    QualifiedName::new(vec![Identifier::new("Noop").expect("an identifier")]).expect("a name")
}

/// FR-098's request retaining the original checked package's emission,
/// with its source, domain package and snapshots in the byte provision.
pub(crate) fn request<'a>(
    checked: &CheckedPackage,
    temporal_clock_selections: &[fsl_package::TemporalClockSelection],
    document: &[u8],
    snapshots: impl Iterator<
        Item = (
            &'a fsl_semantics::model::observation::DocumentRef,
            &'a Vec<u8>,
        ),
    >,
    unit: &[u8],
) -> ReplayRequestWire {
    let source = checked.graph().source();
    let byte_provision = vec![(
        source.authority().to_owned(),
        source.identity().to_owned(),
        unit.to_vec(),
    )];
    let mut documents = vec![document.to_vec()];
    for (_, bytes) in snapshots {
        documents.push(bytes.clone());
    }
    let emission = fsl_package::emit_checked_with_temporal_selections(
        checked,
        temporal_clock_selections,
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the original checked replay package emits");
    ReplayRequestWire {
        certificate: None,
        package: fsl_replay::ReplayPackage::read(
            emission.package().bytes(),
            ReplayLimits::default().input_bytes,
        )
        .expect("the emitted package is bounded JSON"),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: fsl_replay::RequestTargetWire::function(
            selected(),
            OccurrenceKey::new(
                crate::fixture_ids::node_key([7]),
                fsl_replay::Origin::new(fsl_replay::Role::new("declaration"), 0),
            ),
        ),
        source: witness_source(),
        obligation_identity: crate::fixture_ids::obligation_identity(),
        backend: "fsl-en-1".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: UNLIMITED,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents,
    }
}

/// The identity of a package-level freedom item over a model subject: the
/// selected model's `model_root` node of the domain package `identity`, with
/// the `generated` occurrence 0.
pub(crate) fn model_anchor(identity: &str) -> (NodeKey, OccurrenceKey) {
    let node = NodeKey::new(
        identity,
        vec![
            filament_exact::PathSegment::Text("model".to_owned()),
            filament_exact::PathSegment::Text("model_root".to_owned()),
        ],
        0,
    )
    .expect("a model root is admissible");
    let occurrence = OccurrenceKey::new(node.clone(), Origin::new(Role::new("generated"), 0));
    (node, occurrence)
}

/// The identity of a package-level freedom item over the protocol `protocol`
/// of `package`: its `protocol_clause` node with the `declaration`
/// occurrence 0.
pub(crate) fn protocol_anchor(
    package: &CheckedPackage,
    protocol: &str,
) -> (NodeKey, OccurrenceKey) {
    // A package that selects no protocol profile has no clause node: the
    // fixture node is then unknown to the replay, which refuses it.
    let node = package
        .graph()
        .protocol_node_keys(protocol)
        .and_then(|keys| keys.clause.clone())
        .unwrap_or_else(|| crate::fixture_ids::node_key("protocol-clause"));
    let occurrence = OccurrenceKey::new(node.clone(), Origin::new(Role::new("declaration"), 0));
    (node, occurrence)
}

/// The identity of the derived deadlock-freedom item over the counter unit's
/// model.
pub(crate) fn deadlock_freedom_clause() -> (NodeKey, OccurrenceKey) {
    model_anchor(crate::counter::PACKAGE)
}

/// The envelope carrying `payload`, naming the
/// item by `node` and `occurrence`.
pub(crate) fn envelope_at<T: SubjectTransition>(
    payload: TemporalCounterexample<T>,
    node: NodeKey,
    occurrence: OccurrenceKey,
) -> WitnessEnvelope<TemporalCounterexample<T>> {
    WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(crate::fixture_ids::obligation_identity()),
            occurrence_key: Some(occurrence),
            clause_node: Some(node),
            replay_target: fsl_replay::ReplayTargetMembers::function(selected()),
            profile_selections: Some(Vec::new()),
            run_limits: Some(UNLIMITED),
            declared_domains: Some(Vec::new()),
            backend: Some("fsl-en-1".to_owned()),
            trace_position: Some(None),
            source: Some(envelope_witness_source()),
            family_payload: Some(payload),
        },
        ReplayLimits::default(),
    )
    .expect("a complete packet reconstructs")
}

/// The envelope carrying `payload` under the derived deadlock-freedom item's
/// identity.
fn envelope(payload: TemporalCounterexample) -> WitnessEnvelope<TemporalCounterexample> {
    let (node, occurrence) = deadlock_freedom_clause();
    envelope_at(payload, node, occurrence)
}
