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
use filament_semantic_value::fairness::PropertyForm;
use fsl_eval::model_subject::{ModelLetters, ModelSubject, ProtocolSubject};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PackageModelInput,
};
use fsl_eval::simulation::protocol::{PackageProtocolProgram, ProtocolTransition};
use fsl_eval::simulation::Limits;
use fsl_foundation::{source::provenance::OccurrenceKey, RequestIndex};
use fsl_package::{CheckedPackage, ObligationIdentity};
use fsl_replay::{
    replay_model_trace_with_temporal_selections, replay_protocol_trace_from_package,
    CertificateRequest, EnvelopeSource, ModelCheckItem, ModelCheckLimits, ModelTraceReplayResult,
    ModelTraceSubject, PackageReadRefusal, ProtocolCertificateRequest, ProtocolTraceSubject,
    QualifiedName, ReplayPackage, ReplayRefusal, ReplayRequestWire, ReplaySource,
    ReplayTargetMembers, RequestTargetWire, StateEnvironment, TemporalCounterexample, Witness,
    WitnessEnvelope, WitnessPacket,
};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, TemporalClauseNode};

use super::items::{
    admission, settle_model_item, settle_protocol_item, ModelItem, ProtocolItem, BROKEN,
    CLAUSE_FAMILY,
};
use super::request::ProtocolBinding;
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::analyze::derived_item::{derived_target, NoLetters};
use crate::analyze::protocol_item::over_instance;
use crate::model_check::{manifest as en1, ModelCheckRequest, ProtocolCheckRequest};

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
    /// On the package route (FR-940): the clause carries a fact its package
    /// form does not read yet (a fairness set, an interval, an aggregate
    /// atom, a shared formula node, a profile other than the infinite-trace
    /// one or a capture), at the node that carries it.
    #[error(
        "unsupported_construct/not-yet-implemented: the clause's package form does not read its {fact} at {node:?} yet"
    )]
    Pending {
        /// The fact not read yet.
        fact: &'static str,
        /// The node that carries it.
        node: fsl_package::CheckedNodeId,
    },
}

/// The inputs of one authored-clause item.
#[derive(Clone)]
pub struct ClauseSubject<'a> {
    package: &'a CheckedPackage,
    input: ModelCheckInput<'a>,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a super::SourceProvision,
    node: &'a TemporalClauseNode,
    run: ClauseLimits,
    proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    temporal: Option<super::TemporalItem>,
    /// The run's clock selections of the declarations over a model subject,
    /// which the checker's S4 emit reads (FR-640).
    selections: Vec<fsl_package::TemporalClockSelection>,
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
        analysis: &super::RunInput<'a>,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        provision: &'a super::SourceProvision,
        key: &OccurrenceKey,
        run: ClauseLimits,
    ) -> Result<Self, ClauseItemRefusal> {
        Self::from_provision(analysis, binding, limits, provision, key, run, Vec::new())
    }

    /// Build a clause subject over the operation's complete source provision
    /// and the item's explicit proof-bound substitution.
    pub fn from_provision(
        analysis: &super::RunInput<'a>,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        provision: &'a super::SourceProvision,
        key: &OccurrenceKey,
        run: ClauseLimits,
        proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    ) -> Result<Self, ClauseItemRefusal> {
        let (package, input) = (analysis.package, &analysis.model);
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.identity() == *key.node() && node.claim() == key.origin())
            .ok_or(ClauseItemRefusal::UnknownClause)?;
        clause_name(node)?;
        Ok(Self {
            package,
            input: input.clone(),
            binding,
            limits,
            provision,
            node,
            run,
            proof_bounds,
            temporal: None,
            selections: clock_selections(package, provision),
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
                let stutter_closure = subject
                    .temporal
                    .as_ref()
                    .is_some_and(|temporal| temporal.stutter_closure == Some(true));
                if let Some(temporal) = &subject.temporal {
                    if temporal.item_kind != fsl_route::request::ItemKind::Formula
                        || temporal.abstract_subject.is_some()
                        || (stutter_closure
                            && subject.node.clause().property_form()
                                != Some(PropertyForm::Liveness))
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
                            temporal_clock_selections: &subject.selections,
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
                        stutter_closure,
                        certificate: CertificateRequest {
                            temporal_clock_selections: &subject.selections,
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
                        EngineReport::Clause(model, over, None)
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
            input: self.input.clone(),
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
        let inputs = ReplayInputs {
            backend: en1::BACKEND_ID,
            package: self.package,
            binding: &self.binding,
            limits: &self.limits,
            provision: self.provision,
            max_candidates: self.run.max_candidates,
            timed: None,
        };
        let shape = ReplayShape {
            target: RequestTargetWire::state_clause(name.clone()),
            members: ReplayTargetMembers::state_clause(name),
            node: identity,
            occurrence: OccurrenceKey::new(self.node.identity(), self.node.claim().clone()),
            transcript: CLAUSE_TRANSCRIPT,
        };
        if clause_has_reference_literal(self.package.graph(), self.node) {
            let (_, envelope) = replay_wire(&inputs, shape, payload)?;
            let system = ModelSystem::new(self.simulation()).map_err(ReplayRefusal::Admission)?;
            let letters = system
                .clause_letters(self.node)
                .map_err(|_| fault("clause-letters"))?;
            return fsl_replay::replay_checked_source_model_trace(
                self.package,
                &system,
                self.node,
                &envelope,
                &letters,
                self.limits.accounting,
            );
        }
        replay_over(&inputs, shape, payload, letters)
    }
}

fn clause_has_reference_literal(
    graph: &fsl_semantics::check::CheckedGraph,
    node: &TemporalClauseNode,
) -> bool {
    let clause = node.clause();
    let contains = |root: fsl_semantics::check::CheckedNode<'_>| {
        let mut visited_functions = std::collections::BTreeSet::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if let filament_checked::NodeKind::Call { function, .. } = node.kind() {
                if visited_functions.insert(*function) {
                    if let Some(function) = graph.function_state(*function) {
                        pending.push(function.body);
                    }
                }
            }
            if matches!(
                node.kind(),
                filament_checked::NodeKind::ReferenceLiteral { .. }
            ) {
                return true;
            }
            pending.extend(node.children());
        }
        false
    };
    let formula = clause.formula();
    (0..formula.nodes().len()).any(|index| {
        let Some(atom) = formula.node_id(index) else {
            return false;
        };
        node.condition(atom).is_some_and(|(root, _)| contains(root))
            || node.aggregate_conditions(atom).is_some_and(|conditions| {
                [conditions.value, conditions.filter, conditions.predicate]
                    .into_iter()
                    .flatten()
                    .chain([conditions.threshold])
                    .any(|(root, _)| contains(root))
            })
    }) || node.guard().is_some_and(|(root, _)| contains(root))
        || (0..clause.captures().len())
            .any(|index| node.capture(index).is_some_and(|(root, _)| contains(root)))
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
    let (wire, envelope) = replay_wire(inputs, shape, payload)?;
    replay_model_trace_with_temporal_selections(
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
        &clock_selections(inputs.package, inputs.provision),
        inputs.limits.replay,
    )
}

/// The refusal of a witness envelope the replay could not reconstruct: the
/// run's `replay.input_bytes` bound is the replay refusal of that item, any
/// other failure the internal fault `fallback`.
fn envelope_refusal(error: fsl_replay::WitnessRefusal, fallback: ReplayRefusal) -> ReplayRefusal {
    match error {
        fsl_replay::WitnessRefusal::BoundExceeded(bound) => {
            ReplayRefusal::Request(fsl_replay::ReplayRequestRefusal::BoundExceeded(bound))
        }
        _ => fallback,
    }
}

/// The replay's read of the package `bytes` under the run's
/// `replay.input_bytes` (FR-940-AC-22): bytes over the bound refuse as the
/// replay request's `stage_limit_exceeded`/`input-bytes-exceeded`, which
/// settles the item `inconclusive` `replay_refused`. A read that fails otherwise is the
/// replay refusal of that item alone: `resource_exhausted` for an allocation
/// it cannot make, `invalid_package` for bytes that are no package document.
pub(super) fn read_package(
    bytes: &[u8],
    limits: &AnalyzeLimits,
) -> Result<ReplayPackage, ReplayRefusal> {
    let bound = limits.replay.input_bytes;
    let actual = bytes.len() as u128;
    if actual > u128::from(bound) {
        return Err(ReplayRefusal::Request(
            fsl_replay::ReplayRequestRefusal::BoundExceeded(fsl_replay::BoundExceeded {
                bound,
                actual,
            }),
        ));
    }
    ReplayPackage::read(bytes, bound).map_err(|error| {
        ReplayRefusal::PackageRead(match error {
            quire_canonical::ReadError::Allocation { .. } => PackageReadRefusal::Allocation,
            _ => PackageReadRefusal::Malformed,
        })
    })
}

/// The clock selections of the declarations of `package` over a model
/// subject, which the model's binding and the declaration's profile derive
/// (FR-640): every S4 emit of the source route reads them, as the compile
/// does, so an event-position clause is emitted with its clock. A declaration
/// whose traces need different configurations derives none, and its emit
/// then refuses as it names.
pub(super) fn model_selections(
    package: &CheckedPackage,
) -> Vec<fsl_package::TemporalClockSelection> {
    fsl_replay::spine::RequestClocks::model_selections(package).unwrap_or_default()
}

/// Preserve the checked acquisition's selections, deriving only model-owned rows
/// not already supplied by that acquisition.
pub(super) fn clock_selections(
    package: &CheckedPackage,
    provision: &super::SourceProvision,
) -> Vec<fsl_package::TemporalClockSelection> {
    let mut selections = provision.temporal_selections().to_vec();
    for derived in model_selections(package) {
        if !selections.iter().any(|held| {
            held.source == derived.source
                && held.declaration_span == derived.declaration_span
                && held.trace == derived.trace
        }) {
            selections.push(derived);
        }
    }
    selections
}

/// FR-128's replay request and witness envelope for `payload`, the
/// counterexample of the item `shape` names.
pub(super) fn replay_wire<T: Clone + fsl_eval::simulation::SubjectTransition>(
    inputs: &ReplayInputs<'_, '_>,
    shape: ReplayShape,
    payload: &TemporalCounterexample<T>,
) -> Result<
    (
        ReplayRequestWire,
        WitnessEnvelope<TemporalCounterexample<T>>,
    ),
    ReplayRefusal,
> {
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
    let emission = fsl_package::emit_checked_with_temporal_selections(
        inputs.package,
        &clock_selections(inputs.package, inputs.provision),
        inputs.limits.emit,
    )
    .map_err(|_| fault("package-emission"))?;
    let package = read_package(emission.package().bytes(), inputs.limits)?;
    request_and_envelope(
        WireParts {
            package,
            dependencies: super::request::dependency_wires(inputs.package),
            binding: inputs.binding,
            limits: inputs.limits,
            byte_provision: inputs.provision.wire_entries(),
            backend: inputs.backend,
        },
        shape,
        payload,
    )
}

/// What a replay request is written over besides its item: the package, its
/// dependency entries, the subject, the shared limits, the byte provision
/// and the backend.
pub(super) struct WireParts<'r, 'a> {
    pub(super) package: ReplayPackage,
    pub(super) dependencies: Vec<fsl_replay::DependencyEntryWire>,
    pub(super) binding: &'r SubjectBinding<'a>,
    pub(super) limits: &'r AnalyzeLimits,
    pub(super) byte_provision: Vec<(String, String, Vec<u8>)>,
    pub(super) backend: &'static str,
}

/// FR-128's replay request over `package` and the witness envelope of
/// `payload`, the counterexample of the item `shape` names, with the
/// subject's documents and `byte_provision`: the source entries on the
/// source route, none on the package route (FR-850).
pub(super) fn request_and_envelope<T: Clone + fsl_eval::simulation::SubjectTransition>(
    parts: WireParts<'_, '_>,
    shape: ReplayShape,
    payload: &TemporalCounterexample<T>,
) -> Result<
    (
        ReplayRequestWire,
        WitnessEnvelope<TemporalCounterexample<T>>,
    ),
    ReplayRefusal,
> {
    let WireParts {
        package,
        dependencies,
        binding,
        limits,
        byte_provision,
        backend,
    } = parts;
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
    let ReplayShape {
        target,
        members,
        node: identity,
        occurrence,
        transcript: text,
    } = shape;
    let transcript = || Witness::parse(text).map_err(|_| fault("transcript"));
    let wire = ReplayRequestWire {
        certificate: None,
        package,
        profile_selections: Vec::new(),
        dependencies,
        replay_target: target,
        source: ReplaySource::Witness(transcript()?),
        obligation_identity: ObligationIdentity::new(identity.clone(), None, 0),
        backend: backend.to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: limits.accounting,
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents: binding.replay_documents().cloned().collect(),
    };
    let envelope = WitnessEnvelope::reconstruct(
        WitnessPacket {
            obligation_identity: Some(ObligationIdentity::new(identity.clone(), None, 0)),
            occurrence_key: Some(occurrence),
            clause_node: Some(identity),
            replay_target: members,
            profile_selections: Some(Vec::new()),
            run_limits: Some(limits.accounting),
            declared_domains: Some(Vec::new()),
            backend: Some(backend.to_owned()),
            trace_position: Some(None),
            source: Some(EnvelopeSource::Witness(transcript()?)),
            family_payload: Some(payload.clone()),
        },
        limits.replay,
    )
    .map_err(|error| envelope_refusal(error, fault("envelope")))?;
    Ok((wire, envelope))
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
    let emission = fsl_package::emit_checked_with_temporal_selections(
        inputs.package,
        &clock_selections(inputs.package, inputs.provision),
        inputs.limits.emit,
    )
    .map_err(|_| fault("package-emission"))?;
    let package = read_package(emission.package().bytes(), inputs.limits)?;
    let transcript = || Witness::parse(text).map_err(|_| fault("transcript"));
    let wire = ReplayRequestWire {
        certificate: None,
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
        documents: inputs.binding.replay_documents().cloned().collect(),
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
        inputs.limits.replay,
    )
    .map_err(|error| envelope_refusal(error, fault("envelope")))?;
    fsl_replay::replay_protocol_trace_with_temporal_selections(
        wire,
        &inputs.subject,
        &envelope,
        letters,
        &clock_selections(inputs.package, inputs.provision),
        inputs.limits.replay,
    )
}

/// FR-850, FR-940: the inputs of one authored-clause item on the package
/// route. The clause is its package form, read from the `BarePackage` by its
/// node key ([`fsl_eval::simulation::model::PackageModelInput::temporal_clause`]),
/// the model system is built from the package route's model-check input,
/// and a counterexample replays from the package bytes and the handoff
/// supply ([`fsl_replay::replay_model_trace_from_package`]), never from a
/// source provision.
pub struct PackageClauseSubject<'a> {
    input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    clause: fsl_semantics::check::CheckedTemporalClause,
    key: filament_exact::NodeKey,
    occurrence: OccurrenceKey,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    run: ClauseLimits,
    proof_bounds: Vec<fsl_foundation::bound::ProofBound>,
    temporal: Option<super::TemporalItem>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
}

impl<'a> PackageClauseSubject<'a> {
    /// The subject of the clause declared at the `claim` occurrence `claim`
    /// (its node key and occurrence) over `package`,
    /// the package route's model-check input, whose admitted bytes are
    /// `package_bytes` and whose handoff supply is `supply`.
    ///
    /// # Errors
    ///
    /// [`ClauseItemRefusal::Pending`] when the clause carries a fact its
    /// package form does not read yet, naming the fact and its node;
    /// [`ClauseItemRefusal::UnknownClause`] when the package holds no such
    /// clause; [`ClauseItemRefusal::UnnamedClause`] when its name is no
    /// identifier.
    pub fn new(
        package: &'a fsl_eval::simulation::model::PackageModelInput<'a>,
        package_bytes: &'a [u8],
        supply: fsl_replay::HandoffSupply<'a>,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        claim: &OccurrenceKey,
        run: ClauseLimits,
    ) -> Result<Self, ClauseItemRefusal> {
        let key = claim.node();
        let input = package.input();
        let admitted = input
            .admitted_package()
            .ok_or(ClauseItemRefusal::UnknownClause)?;
        let clause = package
            .temporal_clause(
                claim,
                filament_semantic_value::checking::CheckingLimits::default().nodes(),
            )
            .map_err(|refusal| match refusal.pending_fact() {
                Some((fact, node)) => ClauseItemRefusal::Pending {
                    fact,
                    node: node.clone(),
                },
                None => ClauseItemRefusal::UnknownClause,
            })?;
        let identity = fsl_replay::ClauseIdentity::of_admitted(admitted, &clause)
            .map_err(|_| ClauseItemRefusal::UnknownClause)?;
        named(&clause)?;
        Ok(Self {
            input,
            clause,
            key: key.clone(),
            occurrence: identity.occurrence().clone(),
            binding,
            limits,
            run,
            proof_bounds: Vec::new(),
            temporal: None,
            package_bytes,
            supply,
        })
    }

    /// The package form of the clause the item checks.
    pub fn clause(&self) -> &fsl_semantics::check::CheckedTemporalClause {
        &self.clause
    }

    /// The clause's claim occurrence, read from its `temporal_clause_names`
    /// row.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The subject with the request item's settings `temporal` and its
    /// explicit proof bounds, which the item runs under as
    /// [`ClauseSubject`]'s configured item does.
    pub(super) fn configured(mut self, temporal: super::TemporalItem) -> Self {
        self.proof_bounds.clone_from(&temporal.proof_bounds);
        self.temporal = Some(temporal);
        self
    }

    /// Run this authored clause over its package protocol form when the
    /// request's subject selects a protocol.
    pub(super) fn protocol_item(
        self,
        index: RequestIndex,
        protocol: ProtocolBinding,
    ) -> AnalyzeItem<'a> {
        PackageProtocolWork {
            input: self.input,
            clause: Some(self.clause),
            key: self.key,
            occurrence: self.occurrence,
            binding: self.binding,
            limits: self.limits,
            form: self.temporal.unwrap_or_default(),
            package_bytes: self.package_bytes,
            supply: self.supply,
            protocol,
        }
        .item(index)
    }

    /// The `temporal-satisfaction` item that runs the package-form clause
    /// on EN-1 and settles it as [`ClauseSubject::item`] settles the source
    /// form, replaying a counterexample from the package bytes.
    pub fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let claim = self.occurrence.clone();
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            CLAUSE_FAMILY,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                let subject = &self;
                if let Some(temporal) = &subject.temporal {
                    if temporal.item_kind != fsl_route::request::ItemKind::Formula
                        || temporal.abstract_subject.is_some()
                        || temporal.stutter_closure.is_some()
                        || temporal.fairness != subject.clause.fairness()
                    {
                        return EngineReport::Refused(Refusal::Unsupported(super::items::NOT_YET));
                    }
                }
                let system = match ModelSystem::new(subject.simulation()) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let Ok(letters) = system.package_clause_letters(&subject.clause, &subject.key)
                else {
                    return EngineReport::Refused(Refusal::Failed(BROKEN));
                };
                let item = || ModelCheckItem::Clause {
                    clause: &subject.clause,
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
                        stutter_closure: false,
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
                        EngineReport::Clause(model, over, None)
                    }
                    other => other,
                }
            },
        )
        .claimed(claim)
    }

    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            input: self.input.clone(),
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

    /// FR-128's replay of `payload` over the package bytes and the handoff
    /// supply: the request carries the package and the subject's documents
    /// and no source provision.
    fn replay(
        &self,
        payload: &TemporalCounterexample,
        letters: &dyn fsl_eval::model_subject::ModelLetters,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let fault =
            |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
        let name = named(&self.clause).map_err(|_| fault("clause-name"))?;
        let package = read_package(self.package_bytes, &self.limits)?;
        let (wire, envelope) = request_and_envelope(
            WireParts {
                package,
                dependencies: Vec::new(),
                binding: &self.binding,
                limits: &self.limits,
                byte_provision: Vec::new(),
                backend: en1::BACKEND_ID,
            },
            ReplayShape {
                target: RequestTargetWire::state_clause(name.clone()),
                members: ReplayTargetMembers::state_clause(name),
                node: self.key.clone(),
                occurrence: self.occurrence.clone(),
                transcript: CLAUSE_TRANSCRIPT,
            },
            payload,
        )?;
        fsl_replay::replay_model_trace_from_package(
            wire,
            self.supply,
            &ModelTraceSubject {
                timed: None,
                initial: self.binding.initial.clone(),
                universes: self.binding.universes.clone(),
                expansion: ExpansionLimits {
                    max_candidates: self.run.max_candidates,
                },
            },
            &envelope,
            letters,
            self.limits.replay,
        )
    }
}

/// The package-native protocol item over either an authored temporal clause
/// or a derived protocol freedom item.
struct PackageProtocolWork<'a> {
    input: ModelCheckInput<'a>,
    clause: Option<fsl_semantics::check::CheckedTemporalClause>,
    key: filament_exact::NodeKey,
    occurrence: OccurrenceKey,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    protocol: ProtocolBinding,
}

/// FR-850/FR-940: a derived protocol freedom item anchored at the package
/// protocol declaration, with its resolved request form.
pub(super) struct PackageProtocolSelection<'a> {
    /// The declaration occurrence that anchors this item.
    pub(super) claim: &'a OccurrenceKey,
    /// The requested temporal item and its bounds.
    pub(super) form: &'a TemporalItem,
    /// The resolved protocol subject and memory bindings.
    pub(super) protocol: ProtocolBinding,
    /// This item's position in the request.
    pub(super) index: RequestIndex,
}

/// FR-850/FR-940: a derived protocol freedom item anchored at the package
/// protocol declaration, to be run and replayed through the package protocol
/// transition system.
pub(super) fn package_protocol_freedom_item<'a>(
    package: &'a PackageModelInput<'a>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    selection: PackageProtocolSelection<'_>,
) -> AnalyzeItem<'a> {
    let PackageProtocolSelection {
        claim,
        form,
        protocol,
        index,
    } = selection;
    PackageProtocolWork {
        input: package.input(),
        clause: None,
        key: claim.node().clone(),
        occurrence: claim.clone(),
        binding,
        limits,
        form: form.clone(),
        package_bytes,
        supply,
        protocol,
    }
    .item(index)
}

impl<'a> PackageProtocolWork<'a> {
    fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let claim = self.occurrence.clone();
        let family = match self.form.item_kind {
            fsl_route::request::ItemKind::Formula => CLAUSE_FAMILY,
            fsl_route::request::ItemKind::DeadlockFreedom
            | fsl_route::request::ItemKind::RaceFreedom => {
                Some(fsl_semantics::check::DeclarationFamily::Protocol)
            }
            _ => None,
        };
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            family,
            Mode::Bounded,
            en1::BACKEND_ID,
            move |cancel| {
                let unsupported = || {
                    EngineReport::Refused(Refusal::Unsupported(
                        filament_semantic_value::catalog::CatalogCode::new(
                            "unsupported_construct",
                            "not-yet-implemented",
                        ),
                    ))
                };
                if !matches!(
                    self.form.item_kind,
                    fsl_route::request::ItemKind::Formula
                        | fsl_route::request::ItemKind::DeadlockFreedom
                        | fsl_route::request::ItemKind::RaceFreedom
                ) || !self.form.reductions.symmetry.is_empty()
                    || self.form.reductions.partial_order
                {
                    return unsupported();
                }
                let Some(over) = self
                    .protocol
                    .over
                    .as_ref()
                    .map(|over| over_instance(&self.binding, over))
                else {
                    let mut detail = serde_json::Map::new();
                    detail.insert("member".to_owned(), serde_json::json!("over"));
                    return EngineReport::Refused(Refusal::DeclinedWithDetail(
                        fsl_replay::ProofRefusalCause::InvalidInput,
                        fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        filament_semantic_value::catalog::CatalogCode::new(
                            "invalid_runtime_input",
                            "missing-member",
                        ),
                        detail,
                    ));
                };
                let program = match PackageProtocolProgram::new(
                    &self.input,
                    &self.protocol.protocol,
                    &self.protocol.memory,
                ) {
                    Ok(program) => program,
                    Err(fsl_eval::simulation::protocol::PackageProgramRefusal::Unsupported(_)) => {
                        return unsupported();
                    }
                    Err(fsl_eval::simulation::protocol::PackageProgramRefusal::Correlation {
                        node,
                    }) => {
                        let mut detail = serde_json::Map::new();
                        detail.insert("fact".to_owned(), serde_json::json!("correlation"));
                        detail.insert("node".to_owned(), serde_json::json!(node));
                        return EngineReport::Refused(Refusal::UnsupportedWithDetail(
                            filament_semantic_value::catalog::CatalogCode::new(
                                "unsupported_construct",
                                "not-yet-implemented",
                            ),
                            detail,
                        ));
                    }
                    Err(fsl_eval::simulation::protocol::PackageProgramRefusal::RepeatProof {
                        node,
                    }) => {
                        let mut detail = serde_json::Map::new();
                        detail.insert("fact".to_owned(), serde_json::json!("repeat-proof"));
                        detail.insert("node".to_owned(), serde_json::json!(node));
                        return EngineReport::Refused(Refusal::UnsupportedWithDetail(
                            filament_semantic_value::catalog::CatalogCode::new(
                                "unsupported_construct",
                                "not-yet-implemented",
                            ),
                            detail,
                        ));
                    }
                    Err(fsl_eval::simulation::protocol::PackageProgramRefusal::RoleLifetime {
                        node,
                        lifetime,
                    }) => {
                        let mut detail = serde_json::Map::new();
                        detail.insert(
                            "fact".to_owned(),
                            serde_json::json!("replicated-role-lifetime"),
                        );
                        detail.insert("node".to_owned(), serde_json::json!(node));
                        detail.insert("lifetime".to_owned(), serde_json::json!(lifetime));
                        return EngineReport::Refused(Refusal::UnsupportedWithDetail(
                            filament_semantic_value::catalog::CatalogCode::new(
                                "unsupported_construct",
                                "not-yet-implemented",
                            ),
                            detail,
                        ));
                    }
                    Err(fsl_eval::simulation::protocol::PackageProgramRefusal::ParameterType {
                        package,
                        node,
                        failure,
                    }) => {
                        return EngineReport::Refused(
                            super::items::package_parameter_type_refusal(&package, &node, &failure),
                        );
                    }
                    Err(_) => {
                        return EngineReport::Refused(Refusal::Declined(
                            fsl_replay::ProofRefusalCause::InvalidInput,
                            fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                            filament_semantic_value::catalog::CatalogCode::new(
                                "invalid_runtime_input",
                                "invalid-value",
                            ),
                        ));
                    }
                };
                let resolved_memory = program.memory.clone();
                drop(program);
                let simulation = self.simulation();
                let system = match ModelSystem::new(simulation.clone()) {
                    Ok(system) => system,
                    Err(failure) => return EngineReport::Refused(admission(&failure)),
                };
                let clause_letters;
                let letters: &dyn ModelLetters = match self.clause.as_ref() {
                    Some(clause) => {
                        clause_letters = match system.package_clause_letters(clause, &self.key) {
                            Ok(letters) => letters,
                            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                        };
                        &clause_letters
                    }
                    None => &NoLetters,
                };
                let make_item = || match self.clause.as_ref() {
                    Some(clause) => ModelCheckItem::Clause { clause, letters },
                    None if self.form.item_kind == fsl_route::request::ItemKind::RaceFreedom => {
                        ModelCheckItem::RaceFreedom
                    }
                    None if self.form.item_kind
                        == fsl_route::request::ItemKind::DeadlockFreedom =>
                    {
                        ModelCheckItem::DeadlockFreedom
                    }
                    None => ModelCheckItem::DeadlockFreedom,
                };
                let make_subject = || ProtocolSubject {
                    model: ModelSubject {
                        simulation: self.simulation(),
                        proof_bounds: self.form.proof_bounds.clone(),
                    },
                    protocol: self.protocol.protocol.clone(),
                    over: over.clone(),
                    roles: self.protocol.roles.clone(),
                    capacities: self.protocol.capacities.clone(),
                    memory: resolved_memory.clone(),
                };
                let run_subject = make_subject();
                let certificate_subject = make_subject();
                let request = ProtocolCheckRequest {
                    temporal_clock_selections: &[],
                    subject: run_subject,
                    item: make_item(),
                    max_depth: self.clause.as_ref().and(self.form.max_depth),
                    max_store_buffer: self.form.max_store_buffer,
                    max_messages: self.form.max_messages,
                    ignore_memory_constraints: false,
                    max_live_instances: None,
                    limits: self.form.limits,
                };
                let certificate = ProtocolCertificateRequest {
                    temporal_clock_selections: &[],
                    subject: certificate_subject,
                    item: make_item(),
                    max_store_buffer: self.form.max_store_buffer,
                    max_messages: self.form.max_messages,
                    max_live_instances: None,
                };
                let trace = ProtocolTraceSubject {
                    model: ModelTraceSubject {
                        timed: None,
                        initial: self.binding.initial.clone(),
                        universes: self.binding.universes.clone(),
                        expansion: ExpansionLimits {
                            max_candidates: self.form.max_candidates,
                        },
                    },
                    protocol: self.protocol.protocol.clone(),
                    over,
                    roles: self.protocol.roles.clone(),
                    capacities: self.protocol.capacities.clone(),
                    memory: resolved_memory,
                    max_store_buffer: self.form.max_store_buffer,
                    max_messages: self.form.max_messages,
                    max_live_instances: None,
                };
                let constraint = self.form.reductions.constraint.clone();
                let replay = Box::new(|payload: &TemporalCounterexample<ProtocolTransition>| {
                    self.replay(&trace, letters, payload)
                });
                settle_protocol_item(
                    ProtocolItem {
                        request,
                        certificate,
                        replay,
                    },
                    constraint.as_deref(),
                    cancel,
                )
            },
        )
        .claimed(claim)
    }

    fn simulation(&self) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            input: self.input.clone(),
            packages: self.binding.packages,
            normalization: self.limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: self.limits.observation,
            expansion: ExpansionLimits {
                max_candidates: self.form.max_candidates,
            },
            meter: self.limits.accounting,
            emit: self.limits.emit,
        }
    }

    fn replay(
        &self,
        trace: &ProtocolTraceSubject,
        letters: &dyn ModelLetters,
        payload: &TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal> {
        let fault =
            |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
        let shape = match self.clause.as_ref() {
            Some(clause) => {
                let name = named(clause).map_err(|_| fault("clause-name"))?;
                ReplayShape {
                    target: RequestTargetWire::state_clause(name.clone()),
                    members: ReplayTargetMembers::state_clause(name),
                    node: self.key.clone(),
                    occurrence: self.occurrence.clone(),
                    transcript: CLAUSE_TRANSCRIPT,
                }
            }
            None => {
                let target = derived_target().ok_or_else(|| fault("derived-target"))?;
                ReplayShape {
                    target: RequestTargetWire::function(target.clone(), self.occurrence.clone()),
                    members: ReplayTargetMembers::function(target),
                    node: self.key.clone(),
                    occurrence: self.occurrence.clone(),
                    transcript: if self.form.item_kind == fsl_route::request::ItemKind::RaceFreedom
                    {
                        "<<<assertion|model_check|race|>>>"
                    } else {
                        "<<<assertion|model_check|deadlock|>>>"
                    },
                }
            }
        };
        let package = read_package(self.package_bytes, &self.limits)?;
        let (wire, envelope) = request_and_envelope(
            WireParts {
                package,
                dependencies: Vec::new(),
                binding: &self.binding,
                limits: &self.limits,
                byte_provision: Vec::new(),
                backend: en1::BACKEND_ID,
            },
            shape,
            payload,
        )?;
        replay_protocol_trace_from_package(
            wire,
            self.supply,
            trace,
            &envelope,
            letters,
            self.limits.replay,
        )
    }
}

/// The `state_clause` target of a checked temporal clause: its name.
fn named(
    clause: &fsl_semantics::check::CheckedTemporalClause,
) -> Result<QualifiedName, ClauseItemRefusal> {
    let segment = Identifier::new(clause.name()).map_err(|_| ClauseItemRefusal::UnnamedClause)?;
    QualifiedName::new(vec![segment]).map_err(|_| ClauseItemRefusal::UnnamedClause)
}

#[cfg(test)]
mod tests {
    use fsl_foundation::Code;
    use fsl_replay::{InconclusiveCause, TerminalValue};
    use ix_trace_rs::trace;

    use super::*;

    /// Read the actual package produced at Source replay's emission seam. An
    /// untimed replay outcome alone cannot detect a substituted clock authority.
    #[trace("TC-2958", "FR-517-AC-33")]
    #[test]
    fn replay_wire_preserves_the_supplied_await_clock_authority() {
        use fsl_replay::spine::{check, parse, select, DependencyInput, ParseRequest, SpineLimits};
        let model = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/spine-model.semantic-ir.json"
        ));
        let packages = fsl_semantics::model::intake::package_input([model.as_slice()])
            .expect("the existing Widget model admits");
        let text = r#"language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
profile p = "filament.protocol.finite-global/v1";
profile t = "filament.temporal.event-position.false-extension/v1";
model M = "acme/orders";
protocol Flow using p over (input: M::Widget) on origin {
    role R on M::Widget;
    run sequence Main {
        event Started by R as (started: Boolean) { true };
        await Wait after Started using t clock "line-clock" within [0,1]
            match event Seen by R as (seen: Boolean) { true };
            then sequence Good {} timeout sequence Late {}
    }
    finish End as (outcome: Boolean) { true };
}
"#;
        let source = fsl_foundation::SourceIdentity::new("agent-ix", "replay-clock-selection");
        let cancel = filament_exact::Cancel::new();
        let spine = SpineLimits::default();
        let parsed = parse(
            &ParseRequest {
                source: &source,
                path: "replay-clock.native",
                bytes: text.as_bytes(),
            },
            spine.source,
            &cancel,
        )
        .expect("the configured Await source parses")
        .into_value();
        let models = select(&parsed, &packages, spine.model, &cancel)
            .expect("the fixture model is selected")
            .into_value();
        let checked = check(
            &parsed,
            "replay-clock".to_owned(),
            &models,
            &DependencyInput::default(),
            spine,
            &cancel,
        )
        .expect("the real source passes checking")
        .into_value();
        let package = checked.package();
        let protocol = package
            .graph()
            .protocol("Flow")
            .expect("the checked protocol");
        let awaited = &protocol.awaits[0];
        let clock = fsl_package::ClockConfiguration::EventPosition {
            sequence_authority: "caller-events".into(),
        };
        let selection = fsl_package::TemporalClockSelection::new(
            package.graph().source().clone(),
            awaited.span,
            clock.clone(),
        );
        let provision =
            super::super::SourceProvision::single(package.graph().source(), text.as_bytes())
                .with_temporal_selections(vec![selection]);
        let snapshots = fsl_semantics::model::observation::DocumentProvision::new();
        let binding = SubjectBinding {
            packages: &packages,
            snapshots: &snapshots,
            initial: vec![],
            universes: vec![],
        };
        let limits = AnalyzeLimits::unbounded();
        let inputs = ReplayInputs {
            package,
            binding: &binding,
            limits: &limits,
            provision: &provision,
            max_candidates: 1024,
            timed: None,
            backend: en1::BACKEND_ID,
        };
        let keys = package
            .graph()
            .protocol_node_keys("Flow")
            .expect("the checked protocol identities");
        let anchor = keys.clause.clone().expect("the protocol clause identity");
        let occurrence = OccurrenceKey::declaration(anchor.clone());
        let target =
            super::super::derived_item::derived_target().expect("the existing derived target");
        // Only the emission is under test here; behavioral replay remains the
        // clocked timeout integration test's assertion over its real witness.
        let payload = TemporalCounterexample::<ProtocolTransition> {
            initial_state: 0,
            prefix: vec![],
            over_binding: None,
            fairness: vec![],
            kind: fsl_replay::CounterexampleKind::Deadlock,
            lasso: None,
            blocked: vec![],
            refinement: None,
            final_delay: None,
        };
        let (wire, _) = replay_wire(
            &inputs,
            ReplayShape {
                target: RequestTargetWire::function(target.clone(), occurrence.clone()),
                members: ReplayTargetMembers::function(target),
                node: anchor,
                occurrence,
                transcript: "<<<assertion|model_check|deadlock|>>>",
            },
            &payload,
        )
        .expect("the Source replay request emits its actual package");
        let bytes = wire.package.bytes();
        let admitted = fsl_package::admit_checked_package(
            bytes,
            &packages,
            fsl_package::V2ReadLimits::bounded(1 << 24),
        )
        .expect("the strict reader admits the actual replay emission");
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(bytes)
            .expect("the replay package identities");
        let node = graph
            .wire_id(
                keys.nodes
                    .get(&awaited.node)
                    .expect("the Await's own checked identity"),
            )
            .expect("that Await is in the actual emission");
        let row = admitted
            .temporal_bindings()
            .expect("the emitted clock facts")
            .iter()
            .find(|row| row.declaration == node)
            .expect("the Await's actual emitted binding");
        assert_eq!(row.clock_name.as_ref(), "line-clock");
        let fsl_package::ClockConfiguration::EventPosition { sequence_authority } = &row.clock
        else {
            panic!(
                "Source replay must retain the event-position clock: {:?}",
                row.clock
            );
        };
        assert_eq!(
            sequence_authority.as_ref(),
            "caller-events",
            "the actual replay emission must not substitute a default authority"
        );
        assert!(row.source_unit.is_none() && row.trace.is_none());
    }

    /// FR-940-AC-22 (TC-2958): a replay read of bytes that are no package
    /// document is the replay refusal of that item alone, `invalid_package`,
    /// which settles `inconclusive` `replay_refused` and never the internal
    /// fault; a read the run's `replay.input_bytes` bound refuses is
    /// `stage_limit_exceeded`.
    #[trace("TC-2958", "FR-940-AC-22")]
    #[test]
    fn an_unreadable_replay_package_is_that_items_replay_refusal() {
        let limits = AnalyzeLimits::unbounded();
        let refusal = read_package(b"{not a package", &limits).expect_err("malformed bytes");
        assert!(matches!(
            refusal,
            ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)
        ));
        assert_eq!(refusal.code(), Code::InvalidPackage);
        assert_eq!(
            TerminalValue::from_replay_refusal(&refusal),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::InvalidPackage))
        );

        let bounded = AnalyzeLimits {
            replay: fsl_replay::ReplayLimits::default().with_input_bytes(4),
            ..limits
        };
        let refusal = read_package(b"{\"a\":1}", &bounded).expect_err("over the bound");
        assert_eq!(refusal.code(), Code::StageLimitExceeded);
    }
}
