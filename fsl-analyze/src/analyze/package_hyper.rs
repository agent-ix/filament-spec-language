// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal hyper analysis over authenticated package facts (FR-940).

use filament_exact::{Cancel, ValueType};
use fsl_eval::hyper::{CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    CheckedHyperLetters, ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem,
    PackageModelInput,
};
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, CheckedHyperClause, DeclarationFamily};

use super::items::{admission, model_refusal, BROKEN};
use super::{AnalyzeItem, AnalyzeLimits, EngineReport, Refusal, SubjectBinding, TemporalItem};
use crate::model_check::{check_hyper, manifest, HyperCheckRequest};

/// A package-only universal hyper subject. The selected clause is authenticated
/// again by the independent checker before its proof can settle.
pub(super) struct PackageHyperSubject<'a> {
    input: ModelCheckInput<'a>,
    package: &'a fsl_package::BarePackage<'a>,
    clause: CheckedHyperClause,
    binding: SubjectBinding<'a>,
    limits: AnalyzeLimits,
    form: TemporalItem,
}

impl<'a> PackageHyperSubject<'a> {
    pub(super) fn new(
        input: &'a PackageModelInput<'a>,
        clause: CheckedHyperClause,
        binding: SubjectBinding<'a>,
        limits: AnalyzeLimits,
    ) -> Self {
        Self {
            input: input.input(),
            package: input.package(),
            clause,
            binding,
            limits,
            form: TemporalItem::default(),
        }
    }

    pub(super) fn configured(mut self, form: TemporalItem) -> Self {
        self.form = form;
        self
    }

    pub(super) fn item(self, index: RequestIndex) -> AnalyzeItem<'a> {
        let key = self.clause.assembled_occurrence().cloned();
        let item = AnalyzeItem::new(
            index,
            Capability::TemporalSatisfaction,
            Some(DeclarationFamily::TemporalTrace),
            Mode::Bounded,
            manifest::BACKEND_ID,
            move |cancel| self.run(cancel),
        );
        match key {
            Some(key) => item.claimed(key),
            None => item,
        }
    }

    fn run(&self, cancel: &Cancel) -> EngineReport {
        let form = &self.form;
        let limits = self.limits;
        let simulation = ModelSimulationRequest {
            input: self.input.clone(),
            packages: self.binding.packages,
            normalization: limits.normalization,
            snapshots: self.binding.snapshots,
            initial: self.binding.initial.clone(),
            universes: self.binding.universes.clone(),
            observation: limits.observation,
            expansion: ExpansionLimits {
                max_candidates: form.max_candidates,
            },
            meter: limits.accounting,
            emit: limits.emit,
        };
        let subjects: Vec<_> = self
            .clause
            .aliases()
            .iter()
            .map(|_| ModelSubject {
                simulation: simulation.clone(),
                proof_bounds: form.proof_bounds.clone(),
            })
            .collect();
        let Some(occurrence) = self.clause.assembled_occurrence() else {
            return EngineReport::Refused(Refusal::Failed(BROKEN));
        };
        let certificate = match fsl_replay::PackageHyperClosureRequest::new(
            self.package,
            occurrence,
            &subjects,
            limits.accounting,
            crate::plan::MODEL_BASED_POSITIONS,
        ) {
            Ok(request) => request,
            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        let clause = certificate.clause();
        let mut owned = Vec::with_capacity(clause.aliases().len());
        for alias in clause.aliases() {
            match ModelSystem::for_alias(simulation.clone(), alias.as_str()) {
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
        let systems: Vec<_> = match clause
            .quantifiers()
            .iter()
            .map(|quantifier| slot_of(&quantifier.alias).and_then(|slot| owned.get(slot)))
            .collect::<Option<Vec<_>>>()
        {
            Some(systems) => systems,
            None => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        let letters =
            match CheckedHyperLetters::for_evaluator(self.input.evaluator, clause, systems.clone())
            {
                Ok(letters) => letters,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
        let matcher = CheckedStepMatch::for_evaluator(self.input.evaluator, clause);
        let outcome = match check_hyper(
            HyperCheckRequest {
                clause,
                subjects: subjects
                    .iter()
                    .map(|subject| ModelSubject {
                        simulation: subject.simulation.clone(),
                        proof_bounds: subject.proof_bounds.clone(),
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
        let settled = fsl_replay::settle_package_hyper_check(
            &outcome,
            &form.limits,
            &form.requested_limits,
            &certificate,
            cancel,
            |payload| {
                let fault = |cause| {
                    fsl_replay::ReplayRefusal::Fault(
                        filament_semantic_value::catalog::InternalFault::new(
                            "package-hyper-item",
                            cause,
                        ),
                    )
                };
                let mut instance = Vec::with_capacity(clause.parameters().len());
                for parameter in clause.parameters() {
                    let key = payload
                        .instance
                        .iter()
                        .find(|(name, _)| name == &parameter.name)
                        .map(|(_, key)| key);
                    let system = slot_of(&parameter.alias)
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
                fsl_replay::replay_hyper_trace_tuple(
                    &fsl_replay::HyperReplay {
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
            },
        );
        match settled {
            Ok(record) => EngineReport::Hyper(Box::new(record)),
            Err(_) => EngineReport::Refused(Refusal::Failed(BROKEN)),
        }
    }
}
