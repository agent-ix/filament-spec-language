// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty command items over the request's admitted model subject
//! (FR-631, FR-640-AC-12): one EN-1 run of the clause, settled through the
//! replay gate.

use filament_exact::{Cancel, ValueType};
use fsl_eval::hyper::{CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{CheckedHyperLetters, ExpansionLimits, ModelSimulationRequest};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_replay::{replay_hyper_trace_tuple, settle_hyper_check, HyperReplay};
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily};

use super::items::{admission, model_refusal, BROKEN};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::model_check::{
    alias_provisions, alias_system, check_hyper, manifest, HyperCheckRequest,
};

/// The item that runs the checked hyper clause `key` names, or none when the
/// occurrence names no hyper clause. Every alias ranges over the one admitted
/// request subject: its populations carry each model's objects, and each
/// object parameter names its own model's population.
pub(super) fn hyper_item<'a>(
    analysis: &super::RunInput<'a>,
    binding: &SubjectBinding<'a>,
    limits: AnalyzeLimits,
    key: &OccurrenceKey,
    index: RequestIndex,
    form: &TemporalItem,
) -> Option<AnalyzeItem<'a>> {
    let clause = analysis
        .package
        .graph()
        .hyper_clauses()
        .iter()
        .find(|clause| clause.assembled_occurrence() == Some(key))?;
    let package = analysis.package;
    let input = analysis.model.clone();
    let binding = binding.clone();
    let form = form.clone();
    Some(
        AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            Mode::Bounded,
            manifest::BACKEND_ID,
            move |cancel: &Cancel| {
                let simulation = ModelSimulationRequest {
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
                // One system per alias, over its own model (FR-173 "Event-position
                // form": a trace is read over its own model), and the system of each
                // trace variable out of them, in quantifier order.
                let provisions = alias_provisions(&simulation, clause);
                let mut owned = Vec::with_capacity(clause.aliases().len());
                for (index, alias) in clause.aliases().iter().enumerate() {
                    let provision = provisions.get(index).and_then(Option::as_ref);
                    match alias_system(&simulation, provision, alias.as_str()) {
                        Ok(system) => owned.push(system),
                        Err(failure) => return EngineReport::Refused(admission(&failure)),
                    }
                }
                let slot_of = |name: &str| {
                    clause
                        .aliases()
                        .iter()
                        .position(|alias| alias.as_str() == name)
                };
                let mut systems = Vec::with_capacity(clause.quantifiers().len());
                for quantifier in clause.quantifiers() {
                    match slot_of(quantifier.alias.as_str()).and_then(|slot| owned.get(slot)) {
                        Some(system) => systems.push(system),
                        None => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                    }
                }
                let letters = match CheckedHyperLetters::new(package, clause, systems.clone()) {
                    Ok(letters) => letters,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                };
                let matcher = CheckedStepMatch::new(package, clause);
                let outcome = match check_hyper(
                    HyperCheckRequest {
                        clause,
                        subjects: clause
                            .aliases()
                            .iter()
                            .map(|_| ModelSubject {
                                simulation: simulation.clone(),
                                proof_bounds: form.proof_bounds.clone(),
                            })
                            .collect(),
                        letters: &letters,
                        matcher: &matcher,
                        limits: form.limits,
                        max_depth: form.max_depth,
                        meter: limits.accounting,
                        reductions: form.reductions.clone(),
                    },
                    cancel,
                ) {
                    Ok(outcome) => outcome,
                    Err(refusal) => return EngineReport::Refused(model_refusal(&refusal)),
                };
                let settled = settle_hyper_check(&outcome, &form.limits, &[], |payload| {
                    let mut instance = Vec::with_capacity(clause.parameters().len());
                    for parameter in clause.parameters() {
                        let key = payload
                            .instance
                            .iter()
                            .find(|(name, _)| name == &parameter.name)
                            .map(|(_, key)| key);
                        let fault = |cause| {
                            fsl_replay::ReplayRefusal::Fault(
                                filament_semantic_value::catalog::InternalFault::new(
                                    "hyper-item",
                                    cause,
                                ),
                            )
                        };
                        let system = slot_of(parameter.alias.as_str())
                            .and_then(|slot| owned.get(slot))
                            .ok_or_else(|| fault("parameter-alias"))?;
                        let admitted = system
                            .instances(&ValueType::Reference(parameter.population.object()))
                            .map_err(|_| fault("parameter-universe"))?;
                        let bound = admitted
                            .into_iter()
                            .find(|instance| Some(&instance.key) == key)
                            .ok_or_else(|| fault("counterexample-instance"))?;
                        instance.push(ParameterBinding {
                            parameter: parameter.name.clone(),
                            instance: bound,
                        });
                    }
                    replay_hyper_trace_tuple(
                        &HyperReplay {
                            clause,
                            systems: systems.clone(),
                            instance: &instance,
                            letters: &letters,
                            matcher: &matcher,
                            meter: limits.accounting,
                            max_witness_set: form.limits.max_witness_set,
                            max_states: u64::try_from(form.limits.limits.max_states)
                                .unwrap_or(u64::MAX),
                            cancel,
                        },
                        payload,
                    )
                });
                match settled {
                    Ok(record) => EngineReport::Hyper(Box::new(record)),
                    Err(_) => EngineReport::Refused(Refusal::Failed(BROKEN)),
                }
            },
        )
        .claimed(key.clone()),
    )
}
