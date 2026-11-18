// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over an authored temporal clause, built from bytes
//! (FR-281, QSL-1819): the checked package, the request's subject binding,
//! the clause's identity and the caller's budgets are everything it needs.
//! No closure, letters or verdict is supplied by the caller: the letters are
//! the production `ClauseLetters` of the admitted `ModelSystem`, and a
//! counterexample replays through `replay_model_trace` over the same source.
//!
//! Owner design: [`ClauseSubject`] holds only borrows of the caller's inputs
//! (`'a`) and the validated clause identity, so the [`AnalyzeItem<'a>`] it
//! makes outlives it. The `ModelSystem` and the `ClauseLetters` borrowing it
//! are built inside the item's work, where they live exactly as long as the
//! run, its settlement and its replay.
//!
//! The replay request names the clause as FR-128 records: a `state_clause`
//! target of the clause's declared name, the clause node's obligation
//! identity, this engine as backend and the canonical [`CLAUSE_TRANSCRIPT`].

use filament_exact::Identifier;
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_eval::simulation::Limits;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_package::CheckedPackage;
use fsl_replay::{
    replay_model_trace, CertificateRequest, EnvelopeSource, ModelCheckItem, ModelCheckLimits,
    ModelTraceReplayResult, ModelTraceSubject, ObligationIdentity, QualifiedName, ReplayLimits,
    ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource, ReplayTargetMembers,
    RequestTargetWire, StateEnvironment, TemporalCounterexample, Witness, WitnessEnvelope,
    WitnessPacket,
};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, TemporalClauseNode};

use super::items::{admission, settle_model_item, ModelItem, BROKEN, CLAUSE_FAMILY};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding};
use crate::model_check::{manifest as en1, ModelCheckRequest};

/// The replay source an EN-1 authored-clause counterexample carries: a
/// witness transcript with no values, since the clause's lasso is the
/// envelope's payload (FR-128).
pub const CLAUSE_TRANSCRIPT: &str = "<<<assertion|model_check|clause|>>>";

/// The default ceiling on the successor candidates one expansion enumerates:
/// the bound the crate's own EN-1 fixtures run under. `analyze` admission
/// itself is unbounded because it expands nothing; a run that expands states
/// takes a finite default, and a caller with a larger subject raises it.
pub const DEFAULT_MAX_CANDIDATES: u64 = 1_000_000;

/// The budgets of one authored-clause run.
#[derive(Clone, Copy, Debug)]
pub struct ClauseLimits {
    /// FR-101's `max_states` and `max_transitions`.
    pub limits: Limits,
    /// The search horizon `k`; `None` sets none.
    pub max_depth: Option<usize>,
    /// The most successor candidates one expansion enumerates.
    pub max_candidates: u64,
}

impl Default for ClauseLimits {
    fn default() -> Self {
        Self {
            limits: Limits::default(),
            max_depth: None,
            max_candidates: DEFAULT_MAX_CANDIDATES,
        }
    }
}

/// Why no clause item was made.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClauseItemRefusal {
    /// The package holds no temporal clause at the key (node and claim
    /// occurrence).
    #[error("the package holds no temporal clause at the given occurrence")]
    UnknownClause,
    /// The clause's declared name is no identifier, so no `state_clause`
    /// target can name it.
    #[error("the clause's declared name is not an identifier")]
    UnnamedClause,
}

/// The inputs of one authored-clause item.
#[derive(Clone)]
pub struct ClauseSubject<'a> {
    package: &'a CheckedPackage,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a super::SourceProvision,
    node: &'a TemporalClauseNode,
    run: ClauseLimits,
    proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    temporal: Option<super::TemporalItem>,
}

impl<'a> ClauseSubject<'a> {
    /// The subject of the clause at `key` (its node and `claim` occurrence)
    /// of `package`. `provision` is the source bytes `package` was checked from,
    /// which replay recompiles. A source reference carries no digest, so
    /// its bytes are not compared with the package here: bytes of another unit
    /// make the replay of a counterexample refuse (never settle `refuted`),
    /// not the construction.
    ///
    /// # Errors
    ///
    /// [`ClauseItemRefusal`] when the package holds no such named clause.
    pub fn new(
        package: &'a CheckedPackage,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        provision: &'a super::SourceProvision,
        key: &OccurrenceKey,
        run: ClauseLimits,
    ) -> Result<Self, ClauseItemRefusal> {
        Self::from_provision(package, binding, limits, provision, key, run, Vec::new())
    }

    /// Build a clause subject over the operation's complete source provision
    /// and the item's explicit proof-bound substitution.
    pub fn from_provision(
        package: &'a CheckedPackage,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        provision: &'a super::SourceProvision,
        key: &OccurrenceKey,
        run: ClauseLimits,
        proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    ) -> Result<Self, ClauseItemRefusal> {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.identity() == *key.node() && node.claim() == key.origin())
            .ok_or(ClauseItemRefusal::UnknownClause)?;
        clause_name(node)?;
        Ok(Self {
            package,
            binding,
            limits,
            provision,
            node,
            run,
            proof_bounds,
            temporal: None,
        })
    }

    pub(super) fn configured(mut self, temporal: super::TemporalItem) -> Self {
        self.temporal = Some(temporal);
        self
    }

    /// The `temporal-satisfaction` item that runs the clause on EN-1 and
    /// settles it by `settle_model_check` (FR-127), replaying a
    /// counterexample through the production `replay_model_trace`.
    pub fn item(&self, index: RequestIndex) -> AnalyzeItem<'a> {
        let subject = self.clone();
        let claim = OccurrenceKey::new(self.node.identity(), self.node.claim().clone());
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            CLAUSE_FAMILY,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                if let Some(temporal) = &subject.temporal {
                    if temporal.item_kind != fsl_route::request::ItemKind::Formula
                        || temporal.abstract_subject.is_some()
                        || temporal.stutter_closure.is_some()
                        || temporal.fairness != subject.node.clause().fairness()
                    {
                        return EngineReport::Refused(Refusal::Unsupported(super::items::NOT_YET));
                    }
                }
                let system = match ModelSystem::new(subject.simulation()) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let Ok(letters) = system.clause_letters(subject.node) else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let item = || ModelCheckItem::Clause {
                    clause: subject.node.clause(),
                    letters: &letters,
                };
                let report = settle_model_item(
                    ModelItem {
                        request: ModelCheckRequest {
                            temporal_clock_selections: &[],
                            subject: subject.model_subject(),
                            item: item(),
                            max_depth: subject.run.max_depth,
                            limits: subject
                                .temporal
                                .as_ref()
                                .map(|temporal| temporal.limits)
                                .unwrap_or(ModelCheckLimits {
                                    limits: subject.run.limits,
                                    ..ModelCheckLimits::default()
                                }),
                            reductions: subject
                                .temporal
                                .as_ref()
                                .map(|temporal| temporal.reductions.clone())
                                .unwrap_or_default(),
                        },
                        certificate: CertificateRequest {
                            temporal_clock_selections: &[],
                            subject: subject.model_subject(),
                            item: item(),
                            smt_encoding: subject
                                .temporal
                                .as_ref()
                                .and_then(|temporal| temporal.smt_encoding)
                                .unwrap_or_default(),
                        },
                        replay: Box::new(|payload| subject.replay(payload, &letters)),
                    },
                    cancel,
                );
                match report {
                    EngineReport::Model(model) => {
                        let over = model
                            .counterexample
                            .as_ref()
                            .and_then(|found| found.over_binding.as_ref())
                            .and_then(|instance| letters.instance_reference(instance));
                        EngineReport::Clause(model, over)
                    }
                    other => other,
                }
            },
        )
        .claimed(claim)
    }

    /// Replays `payload`, a counterexample of this clause, through
    /// `replay_model_trace` over the same source and the production letters:
    /// the replay `item` settles `refuted` by, run again for a caller that
    /// holds the record's counterexample.
    ///
    /// # Errors
    ///
    /// [`ReplayRefusal`] as `replay_model_trace` refuses it.
    pub fn replay_counterexample(
        &self,
        payload: &TemporalCounterexample,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let broken = || ReplayRefusal::Fault(InternalFault::new("analyze", "clause-letters"));
        let system = ModelSystem::new(self.simulation()).map_err(ReplayRefusal::Admission)?;
        let letters = system.clause_letters(self.node).map_err(|_| broken())?;
        self.replay(payload, &letters)
    }

    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            package: self.package,
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: self.run.max_candidates,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }

    fn model_subject(&self) -> fsl_eval::model_subject::ModelSubject<'a> {
        fsl_eval::model_subject::ModelSubject {
            simulation: self.simulation(),
            proof_bounds: self.proof_bounds.clone(),
        }
    }
    /// FR-128's replay of `payload` over the same source and subject.
    fn replay(
        &self,
        payload: &TemporalCounterexample,
        letters: &dyn fsl_eval::model_subject::ModelLetters,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let fault =
            |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
        let name = clause_name(self.node).map_err(|_| fault("clause-name"))?;
        let identity = self.node.identity();
        replay_over(
            &ReplayInputs {
                backend: en1::BACKEND_ID,
                package: self.package,
                binding: &self.binding,
                limits: &self.limits,
                provision: self.provision,
                max_candidates: self.run.max_candidates,
                timed: None,
            },
            ReplayShape {
                target: RequestTargetWire::state_clause(name.clone()),
                members: ReplayTargetMembers::state_clause(name),
                node: identity,
                occurrence: OccurrenceKey::new(self.node.identity(), self.node.claim().clone()),
                transcript: CLAUSE_TRANSCRIPT,
            },
            payload,
            letters,
        )
    }
}

/// What a replay of an EN-1 counterexample reads of the run: the checked
/// package, the subject binding, the shared limits, the source bytes and the
/// expansion ceiling.
pub(super) struct ReplayInputs<'r, 'a> {
    pub(super) package: &'a CheckedPackage,
    pub(super) binding: &'r SubjectBinding<'a>,
    pub(super) limits: &'r AnalyzeLimits,
    pub(super) provision: &'r super::SourceProvision,
    pub(super) max_candidates: u64,
    /// FR-237: the timed reading the counterexample was found under.
    pub(super) timed: Option<fsl_replay::TimedReplay>,
    /// The engine whose run the counterexample came from, which the replay
    /// request and envelope name.
    pub(super) backend: &'static str,
}

/// How a counterexample names its item (FR-128 Inputs): the replay target,
/// the item's node and occurrence, and the witness transcript.
pub(crate) struct ReplayShape {
    pub(crate) target: RequestTargetWire,
    pub(crate) members: ReplayTargetMembers,
    pub(crate) node: filament_exact::NodeKey,
    pub(crate) occurrence: OccurrenceKey,
    pub(crate) transcript: &'static str,
}

/// FR-128's replay of `payload` over the same source and subject, for the
/// item `shape` names.
pub(super) fn replay_over(
    inputs: &ReplayInputs<'_, '_>,
    shape: ReplayShape,
    payload: &TemporalCounterexample,
    letters: &dyn fsl_eval::model_subject::ModelLetters,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
    let ReplayShape {
        target,
        members,
        node: identity,
        occurrence,
        transcript: text,
    } = shape;
    let emission =
        fsl_package::emit_checked_with_temporal_selections(inputs.package, &[], inputs.limits.emit)
            .map_err(|_| fault("package-emission"))?;
    let package = ReplayPackage::read(
        emission.package().bytes(),
        ReplayLimits::default().input_bytes,
    )
    .map_err(|_| fault("package-emission"))?;
    let transcript = || Witness::parse(text).map_err(|_| fault("transcript"));
    let wire = ReplayRequestWire {
        package,
        profile_selections: Vec::new(),
        dependencies: super::request::dependency_wires(inputs.package),
        replay_target: target,
        source: ReplaySource::Witness(transcript()?),
        obligation_identity: ObligationIdentity::new(identity.clone(), None, 0),
        backend: inputs.backend.to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: inputs.limits.accounting,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: inputs.provision.wire_entries(),
        documents: inputs
            .binding
            .packages
            .values()
            .chain(inputs.binding.snapshots.values())
            .cloned()
            .collect(),
    };
    let envelope = WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(ObligationIdentity::new(identity.clone(), None, 0)),
            occurrence_key: Some(occurrence),
            clause_node: Some(identity),
            replay_target: members,
            profile_selections: Some(Vec::new()),
            run_limits: Some(inputs.limits.accounting),
            declared_domains: Some(Vec::new()),
            backend: Some(inputs.backend.to_owned()),
            trace_position: Some(None),
            source: Some(EnvelopeSource::Witness(transcript()?)),
            family_payload: Some(payload.clone()),
        },
        ReplayLimits::default(),
    )
    .map_err(|_| fault("envelope"))?;
    replay_model_trace(
        wire,
        &ModelTraceSubject {
            timed: inputs.timed.clone(),
            initial: inputs.binding.initial.clone(),
            universes: inputs.binding.universes.clone(),
            expansion: ExpansionLimits {
                max_candidates: inputs.max_candidates,
            },
        },
        &envelope,
        letters,
        ReplayLimits::default(),
    )
}

/// The `state_clause` target of a temporal clause: its declared name.
pub(super) fn clause_name(node: &TemporalClauseNode) -> Result<QualifiedName, ClauseItemRefusal> {
    let segment =
        Identifier::new(node.clause().name()).map_err(|_| ClauseItemRefusal::UnnamedClause)?;
    QualifiedName::new(vec![segment]).map_err(|_| ClauseItemRefusal::UnnamedClause)
}

/// What FR-217's replay of a protocol counterexample reads of the run: the
/// checked package, the subject binding, the shared limits, the source bytes,
/// and the protocol subject the counterexample was found over.
pub(crate) struct ProtocolReplayInputs<'r, 'a> {
    pub(crate) package: &'a CheckedPackage,
    pub(crate) binding: &'r SubjectBinding<'a>,
    pub(crate) limits: &'r AnalyzeLimits,
    pub(crate) provision: &'r super::SourceProvision,
    pub(crate) subject: fsl_replay::ProtocolTraceSubject,
    /// The engine whose run the counterexample came from.
    pub(crate) backend: &'static str,
}

/// FR-217's replay of the protocol counterexample `payload` over the same
/// source and subject, for the item `shape` names: the protocol analogue of
/// [`replay_over`].
pub(crate) fn replay_protocol_over(
    inputs: &ProtocolReplayInputs<'_, '_>,
    shape: ReplayShape,
    payload: &TemporalCounterexample<fsl_eval::simulation::protocol::ProtocolTransition>,
    letters: &dyn fsl_eval::model_subject::ModelLetters,
) -> Result<ModelTraceReplayResult, ReplayRefusal> {
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
    let ReplayShape {
        target,
        members,
        node: identity,
        occurrence,
        transcript: text,
    } = shape;
    let emission =
        fsl_package::emit_checked_with_temporal_selections(inputs.package, &[], inputs.limits.emit)
            .map_err(|_| fault("package-emission"))?;
    let package = ReplayPackage::read(
        emission.package().bytes(),
        ReplayLimits::default().input_bytes,
    )
    .map_err(|_| fault("package-emission"))?;
    let transcript = || Witness::parse(text).map_err(|_| fault("transcript"));
    let wire = ReplayRequestWire {
        package,
        profile_selections: Vec::new(),
        dependencies: super::request::dependency_wires(inputs.package),
        replay_target: target,
        source: ReplaySource::Witness(transcript()?),
        obligation_identity: ObligationIdentity::new(identity.clone(), None, 0),
        backend: inputs.backend.to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: inputs.limits.accounting,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: inputs.provision.wire_entries(),
        documents: inputs
            .binding
            .packages
            .values()
            .chain(inputs.binding.snapshots.values())
            .cloned()
            .collect(),
    };
    let envelope = WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(ObligationIdentity::new(identity.clone(), None, 0)),
            occurrence_key: Some(occurrence),
            clause_node: Some(identity),
            replay_target: members,
            profile_selections: Some(Vec::new()),
            run_limits: Some(inputs.limits.accounting),
            declared_domains: Some(Vec::new()),
            backend: Some(inputs.backend.to_owned()),
            trace_position: Some(None),
            source: Some(EnvelopeSource::Witness(transcript()?)),
            family_payload: Some(payload.clone()),
        },
        ReplayLimits::default(),
    )
    .map_err(|_| fault("envelope"))?;
    fsl_replay::replay_protocol_trace(
        wire,
        &inputs.subject,
        &envelope,
        letters,
        ReplayLimits::default(),
    )
}
