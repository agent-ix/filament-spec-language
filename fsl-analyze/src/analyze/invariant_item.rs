// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-625 and FR-951: the EN-1 item over an `invariant` state clause in a
//! request that names a model subject, built from bytes as the derived
//! deadlock-freedom item is: the checked package, the request's subject
//! binding and the item's budgets are everything it needs.
//!
//! The item decides `always holds(I)`, `I` the invariant's body over every
//! object of a state whose type conforms to its context type. Its proof is
//! certified under the invariant key-context root, and its counterexample,
//! a finite prefix to the first state where `I` is false or undefined,
//! replays through `replay_model_trace` naming the invariant as its
//! `state_clause` target (FR-128).

use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    CertificateRequest, ModelCheckItem, QualifiedName, ReplayTargetMembers, RequestTargetWire,
    TemporalCounterexample,
};
use fsl_semantics::check::DeclarationFamily;

use super::clause_item::{replay_over, ReplayInputs, ReplayShape, CLAUSE_TRANSCRIPT};
use super::derived_item::NoLetters;
use super::items::{run_model_item, CounterexampleReplayFn, ModelItem};
use super::{AnalyzeItem, AnalyzeLimits, SourceProvision, SubjectBinding, TemporalItem};
use crate::model_check::manifest as en1;
use crate::model_check::ModelCheckRequest;

/// The EN-1 item that decides the `invariant` state clause at `claim` (its
/// `state_clause` node and `claim` occurrence) over the binding, with the
/// budgets of `form`; `None` when `claim` names no checked invariant.
pub(super) fn invariant_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let (package, input) = (analysis.package, &analysis.model);
    let clause = package.graph().state_clauses().iter().find(|clause| {
        clause.is_invariant()
            && clause.identity() == *claim.node()
            && clause.claim() == claim.origin()
    })?;
    let name =
        QualifiedName::new(vec![filament_exact::Identifier::new(clause.name()).ok()?]).ok()?;
    let input = input.clone();
    let simulation = move |binding: &SubjectBinding<'a>| ModelSimulationRequest {
        input: input.clone(),
        packages: binding.packages,
        normalization: limits.normalization,
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: limits.observation,
        expansion: ExpansionLimits {
            max_candidates: form.max_candidates,
        },
        meter: limits.accounting,
        emit: limits.emit,
    };
    let subject = |binding: &SubjectBinding<'a>| ModelSubject {
        simulation: simulation(binding),
        proof_bounds: form.proof_bounds.clone(),
    };
    let item = ModelCheckItem::Invariant {
        clause: fsl_replay::InvariantClause::of(clause),
    };
    let request = ModelCheckRequest {
        temporal_clock_selections: &[],
        subject: subject(binding),
        item,
        max_depth: form.max_depth,
        limits: form.limits,
        reductions: form.reductions.clone(),
    };
    let certificate = CertificateRequest {
        temporal_clock_selections: &[],
        subject: subject(binding),
        item,
        smt_encoding: form.smt_encoding.unwrap_or_default(),
    };
    let replay_binding = binding.clone();
    let max_candidates = form.max_candidates;
    let occurrence = claim.clone();
    let replay: Box<CounterexampleReplayFn<'a>> =
        Box::new(move |payload: &TemporalCounterexample| {
            replay_over(
                &ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &replay_binding,
                    limits: &limits,
                    provision,
                    max_candidates,
                    timed: None,
                },
                ReplayShape {
                    target: RequestTargetWire::state_clause(name.clone()),
                    members: ReplayTargetMembers::state_clause(name.clone()),
                    node: occurrence.node().clone(),
                    occurrence: occurrence.clone(),
                    transcript: CLAUSE_TRANSCRIPT,
                },
                payload,
                &NoLetters,
            )
        });
    Some(
        run_model_item(
            index,
            Some(DeclarationFamily::StateModel),
            ModelItem {
                request,
                certificate,
                replay,
            },
        )
        .claimed(claim.clone()),
    )
}

/// FR-850, FR-940: the EN-1 item over an `invariant` state clause on the
/// package route. The clause is the one its `claim_names` row names
/// ([`fsl_eval::simulation::model::PackageModelInput::state_clause`]), the
/// model system is built from the package route's model-check input, and a
/// counterexample replays from the package bytes and the handoff supply
/// ([`fsl_replay::replay_model_trace_from_package`]), never from a source
/// provision.
pub struct PackageInvariantSubject<'a> {
    input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    clause: &'a fsl_eval::simulation::model::StateClauseEntry,
    claim: OccurrenceKey,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
}

/// What a package-route item reads beside its claim: the package route's
/// model-check input, its admitted bytes and handoff supply, the request's
/// binding and the shared limits.
pub struct PackageItemInputs<'a> {
    /// The package route's model-check input.
    pub input: &'a fsl_eval::simulation::model::PackageModelInput<'a>,
    /// The admitted `checked-package.json` bytes.
    pub package_bytes: &'a [u8],
    /// The handoff's identity and equality file bytes.
    pub supply: fsl_replay::HandoffSupply<'a>,
    /// The request's subject binding.
    pub binding: SubjectBinding<'a>,
    /// The shared limits.
    pub limits: AnalyzeLimits,
}

impl<'a> PackageInvariantSubject<'a> {
    /// The invariant subject of the state clause at `claim`, with the item's
    /// settings `form`; `None` when no state clause row names `claim`.
    pub fn new(
        inputs: PackageItemInputs<'a>,
        claim: &OccurrenceKey,
        form: &TemporalItem,
    ) -> Option<Self> {
        let clause = inputs.input.state_clause(claim.node(), claim.origin())?;
        Some(Self {
            input: inputs.input.input(),
            clause,
            claim: claim.clone(),
            binding: inputs.binding,
            limits: inputs.limits,
            form: form.clone(),
            package_bytes: inputs.package_bytes,
            supply: inputs.supply,
        })
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

    /// The item that decides the invariant at `index` as the source route's
    /// [`invariant_item`] does.
    pub fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let clause = fsl_replay::InvariantClause {
            name: &self.clause.name,
            node: &self.clause.identity,
            claim: &self.clause.claim,
        };
        let subject = || ModelSubject {
            simulation: self.simulation(),
            proof_bounds: self.form.proof_bounds.clone(),
        };
        let request = ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(),
            item: ModelCheckItem::Invariant { clause },
            max_depth: self.form.max_depth,
            limits: self.form.limits,
            reductions: self.form.reductions.clone(),
        };
        let certificate = CertificateRequest {
            temporal_clock_selections: &[],
            subject: subject(),
            item: ModelCheckItem::Invariant { clause },
            smt_encoding: self.form.smt_encoding.unwrap_or_default(),
        };
        let claim = self.claim.clone();
        run_model_item(
            index,
            Some(DeclarationFamily::StateModel),
            ModelItem {
                request,
                certificate,
                replay: Box::new(move |payload: &TemporalCounterexample| self.replay(payload)),
            },
        )
        .claimed(claim)
    }

    /// FR-128's replay of `payload` over the package bytes and the handoff
    /// supply, naming the invariant as its `state_clause` target.
    fn replay(
        &self,
        payload: &TemporalCounterexample,
    ) -> Result<fsl_replay::ModelTraceReplayResult, fsl_replay::ReplayRefusal> {
        let fault = |cause: &'static str| {
            fsl_replay::ReplayRefusal::Fault(filament_semantic_value::catalog::InternalFault::new(
                "analyze", cause,
            ))
        };
        let identifier = filament_exact::Identifier::new(&self.clause.name)
            .map_err(|_| fault("invariant-name"))?;
        let name = QualifiedName::new(vec![identifier]).map_err(|_| fault("invariant-name"))?;
        let package =
            super::clause_item::read_package(self.package_bytes, &self.limits, "package-bytes")?;
        let (wire, envelope) = super::clause_item::request_and_envelope(
            super::clause_item::WireParts {
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
                node: self.claim.node().clone(),
                occurrence: self.claim.clone(),
                transcript: CLAUSE_TRANSCRIPT,
            },
            payload,
        )?;
        fsl_replay::replay_model_trace_from_package(
            wire,
            self.supply,
            &fsl_replay::ModelTraceSubject {
                timed: None,
                initial: self.binding.initial.clone(),
                universes: self.binding.universes.clone(),
                expansion: ExpansionLimits {
                    max_candidates: self.form.max_candidates,
                },
            },
            &envelope,
            &NoLetters,
            self.limits.replay,
        )
    }
}
