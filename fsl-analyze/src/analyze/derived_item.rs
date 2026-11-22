// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-1 item over a derived `deadlock-freedom` request item (FR-626,
//! FR-124, FSpec FR-368), built from bytes as [`super::ClauseSubject`] builds
//! an authored clause's: the checked package, the request's subject binding
//! and the item's budgets are everything it needs.
//!
//! The item decides that every reachable state either enables a step or
//! satisfies a `terminal` declaration of the unit, so it reads no `holds`
//! atom; its counterexample replays through `replay_model_trace` over the
//! same source, named by the subject's `model_root` anchor (FR-128).

use std::collections::BTreeSet;

use filament_exact::{Identifier, Meter};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::model_subject::{AtomStop, ModelInstance, ModelLetters, ModelSubject};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimState, ModelSimulationRequest};
use fsl_eval::temporal::AtomReading;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    CertificateRequest, ModelCheckItem, QualifiedName, ReplayRefusal, ReplayTargetMembers,
    RequestTargetWire, TemporalCounterexample,
};
use fsl_route::request::ItemKind;
use fsl_semantics::check::footprint::{FootprintRefusal, SymbolicLocation};
use fsl_semantics::check::FormulaNodeId;

use super::clause_item::{replay_over, request_and_envelope, ReplayInputs, ReplayShape, WireParts};
use super::items::{joined_replay, run_model_item, CounterexampleReplayFn, ModelItem};
use super::{AnalyzeItem, AnalyzeLimits, SourceProvision, SubjectBinding, TemporalItem};
use crate::claim_family::claim_family;
use crate::model_check::manifest as en1;
use crate::model_check::ModelCheckRequest;

/// The witness transcript of a derived deadlock-freedom counterexample: it
/// carries no values, the payload being the finite prefix (FR-128).
pub(super) const DEADLOCK_TRANSCRIPT: &str = "<<<assertion|model_check|deadlock|>>>";

/// The replay target a derived item names: it has no declaration of its own,
/// and the replay reads only the item's node and occurrence key.
const DERIVED_TARGET: &str = "derived_item";

/// The letters of a derived item: it has no `holds` atom and no activation,
/// so a read of either is a fault of the engine, never a reading.
pub(crate) struct NoLetters;

impl NoLetters {
    fn fault() -> AtomStop {
        AtomStop::Fault(InternalFault::new(
            "analyze",
            "a-derived-item-reads-no-atom",
        ))
    }
}

impl ModelLetters for NoLetters {
    fn atom(
        &self,
        _atom: FormulaNodeId,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Err(Self::fault())
    }

    fn activation(
        &self,
        _instance: &ModelInstance,
        _state: &ModelSimState,
        _meter: &mut Meter,
    ) -> Result<AtomReading, AtomStop> {
        Err(Self::fault())
    }

    fn activation_reads(&self) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }

    fn reads(&self, _atom: FormulaNodeId) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Ok(BTreeSet::new())
    }
}

/// The replay target of a derived item.
pub(crate) fn derived_target() -> Option<QualifiedName> {
    QualifiedName::new(vec![Identifier::new(DERIVED_TARGET).ok()?]).ok()
}

/// The EN-1 item that decides the derived deadlock-freedom item anchored at
/// `claim` (the `model_root` occurrence of the subject's one selection) over
/// the binding, with the item's budgets. A derived item carries no `max_depth` (FSpec
/// FR-368-AC-20), so the search has no horizon.
pub(super) fn deadlock_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let package = analysis.package;
    let mut item = deadlock_model_item(analysis, binding, limits, provision, claim, form)?;
    item.replay = joined_replay(item.replay, package.graph().terminal_keys());
    let family = claim_family(package, ItemKind::DeadlockFreedom, claim);
    Some(run_model_item(index, family, item).claimed(claim.clone()))
}

/// The source route's EN-1 run of the derived deadlock-freedom item anchored
/// at `claim`, its certificate check and its replay over the source
/// provision, which [`super::derived_model_item`] settles against the
/// request item; `None` when the derived replay target is not formed.
pub fn deadlock_model_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    provision: &'a SourceProvision,
    claim: &OccurrenceKey,
    form: &TemporalItem,
) -> Option<ModelItem<'a>> {
    let (package, input) = (analysis.package, &analysis.model);
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
    let target = derived_target()?;
    let (binding, form, claim) = (binding.clone(), form.clone(), claim.clone());
    let replay_binding = binding.clone();
    let request = ModelCheckRequest {
        temporal_clock_selections: &[],
        subject: subject(&binding),
        item: ModelCheckItem::DeadlockFreedom,
        max_depth: None,
        limits: form.limits,
        reductions: form.reductions.clone(),
    };
    let certificate = CertificateRequest {
        temporal_clock_selections: &[],
        subject: subject(&binding),
        item: ModelCheckItem::DeadlockFreedom,
        smt_encoding: form.smt_encoding.unwrap_or_default(),
    };
    let replay: Box<CounterexampleReplayFn<'a>> =
        Box::new(move |payload: &TemporalCounterexample| {
            replay_derived(
                &ReplayInputs {
                    backend: en1::BACKEND_ID,
                    package,
                    binding: &replay_binding,
                    limits: &limits,
                    provision,
                    max_candidates: form.max_candidates,
                    timed: None,
                    stutter_closure: false,
                },
                &claim,
                &target,
                payload,
            )
        });
    Some(ModelItem {
        request,
        stutter_closure: false,
        certificate,
        replay,
    })
}

/// FR-128's replay of a derived deadlock-freedom counterexample, in the
/// envelope that names the item by its anchor.
fn replay_derived(
    inputs: &ReplayInputs<'_, '_>,
    claim: &OccurrenceKey,
    target: &QualifiedName,
    payload: &TemporalCounterexample,
) -> Result<fsl_replay::ModelTraceReplayResult, ReplayRefusal> {
    replay_over(
        inputs,
        ReplayShape {
            target: RequestTargetWire::function(target.clone(), claim.clone()),
            members: ReplayTargetMembers::function(target.clone()),
            node: claim.node().clone(),
            occurrence: claim.clone(),
            transcript: DEADLOCK_TRANSCRIPT,
        },
        payload,
        &NoLetters,
    )
}

/// FR-850, FR-940: the derived deadlock-freedom item anchored at a model
/// subject's `model_root` occurrence on the package route. The model system
/// is built from the package route's model-check input
/// ([`fsl_eval::simulation::model::PackageModelInput::of`]), and a
/// counterexample replays from the package bytes and the handoff supply
/// ([`fsl_replay::replay_model_trace_from_package`]), never from a source
/// provision. [`super::derived_model_item`] joins its anchor and terminal
/// declarations against the admitted package's model selections.
pub struct PackageDeadlockSubject<'a> {
    input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    claim: OccurrenceKey,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
}

impl<'a> PackageDeadlockSubject<'a> {
    /// The deadlock-freedom subject anchored at `claim` over `package`, the
    /// package route's model-check input, whose admitted bytes are
    /// `package_bytes` and whose handoff supply is `supply`, with the item's
    /// settings `form`.
    pub fn new(
        package: &'a fsl_eval::simulation::model::PackageModelInput<'a>,
        package_bytes: &'a [u8],
        supply: fsl_replay::HandoffSupply<'a>,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
        claim: &OccurrenceKey,
        form: &TemporalItem,
    ) -> Self {
        Self {
            input: package.input(),
            claim: claim.clone(),
            binding,
            limits,
            form: form.clone(),
            package_bytes,
            supply,
        }
    }

    /// The EN-1 run, its certificate check and its bytes-only replay, which
    /// [`super::derived_model_item`] settles against the request item; no
    /// `max_depth`, since a derived item is an exhaustive search (FSpec
    /// FR-368-AC-20).
    pub fn model_item(self) -> ModelItem<'a> {
        let subject = || ModelSubject {
            simulation: self.simulation(),
            proof_bounds: self.form.proof_bounds.clone(),
        };
        let request = ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: subject(),
            item: ModelCheckItem::DeadlockFreedom,
            max_depth: None,
            limits: self.form.limits,
            reductions: self.form.reductions.clone(),
        };
        let certificate = CertificateRequest {
            temporal_clock_selections: &[],
            subject: subject(),
            item: ModelCheckItem::DeadlockFreedom,
            smt_encoding: self.form.smt_encoding.unwrap_or_default(),
        };
        ModelItem {
            request,
            stutter_closure: false,
            certificate,
            replay: Box::new(move |payload: &TemporalCounterexample| self.replay(payload)),
        }
    }

    /// The `temporal-satisfaction` item that decides the derived
    /// deadlock-freedom item at `index` as the source route's does: its
    /// replay joined with the package's terminal declarations, keyed by
    /// their model's selection identity (FR-411).
    pub fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let claim = self.claim.clone();
        let keys = self.input.terminal_keys();
        let mut item = self.model_item();
        item.replay = joined_replay(item.replay, keys);
        run_model_item(
            index,
            crate::claim_family::kind_family(ItemKind::DeadlockFreedom),
            item,
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

    /// FR-128's replay of `payload` over the package bytes and the handoff
    /// supply, in the envelope that names the item by its anchor: the request
    /// carries the package and the subject's documents and no source
    /// provision.
    fn replay(
        &self,
        payload: &TemporalCounterexample,
    ) -> Result<fsl_replay::ModelTraceReplayResult, ReplayRefusal> {
        let fault =
            |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("analyze", cause));
        let target = derived_target().ok_or_else(|| fault("derived-target"))?;
        let package = super::clause_item::read_package(self.package_bytes, &self.limits)?;
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
                target: RequestTargetWire::function(target.clone(), self.claim.clone()),
                members: ReplayTargetMembers::function(target),
                node: self.claim.node().clone(),
                occurrence: self.claim.clone(),
                transcript: DEADLOCK_TRANSCRIPT,
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
                stutter_closure: false,
            },
            &envelope,
            &NoLetters,
            self.limits.replay,
        )
    }
}
