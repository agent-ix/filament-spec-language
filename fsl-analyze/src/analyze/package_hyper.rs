// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal hyper analysis over authenticated package facts (FR-940).

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_exact::Cancel;
use fsl_eval::hyper::{CheckedStepMatch, ParameterBinding};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    CheckedHyperLetters, ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem,
    PackageModelInput,
};
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, CheckedHyperClause, DeclarationFamily};
use ix_trace_rs::implements;

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

    /// Each subject alias of the clause with the identity of the model it
    /// selects.
    pub(super) fn subject_models(&self) -> Option<&BTreeMap<String, Arc<str>>> {
        self.clause.package_subject_models()
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

    #[implements("FR-940-AC-17")]
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
            Err(fsl_replay::ProductClosureRefusal::Admission(failure)) => {
                return EngineReport::Refused(admission(&failure))
            }
            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        let clause = certificate.clause();
        let mut owned = Vec::with_capacity(clause.aliases().len());
        for alias in clause.aliases() {
            match ModelSystem::for_hyper_alias(simulation.clone(), clause, alias.as_str()) {
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
            .map(|quantifier| slot_of(quantifier.alias.as_str()).and_then(|slot| owned.get(slot)))
            .collect::<Option<Vec<_>>>()
        {
            Some(systems) => systems,
            None => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        if matches!(
            clause.form(),
            Some(fsl_semantics::check::HyperForm::SingleExistential)
        ) {
            return self.single_existential(
                &certificate,
                clause,
                simulation,
                &systems,
                form,
                limits,
                cancel,
            );
        }
        let letters =
            match CheckedHyperLetters::for_evaluator(self.input.evaluator, clause, systems.clone())
            {
                Ok(letters) => letters,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
        let matcher = CheckedStepMatch::for_evaluator(self.input.evaluator, clause);
        let retained = limits.engines.item_budget(limits.accounting, cancel);
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
                retained: Some(&retained),
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
                let instance = super::hyper_route::counterexample_instance(
                    clause,
                    &owned,
                    &slot_of,
                    &payload.instance,
                )
                .ok_or_else(|| fault("counterexample-instance"))?;
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

    #[implements("FR-940-AC-17", "FR-181-AC-10")]
    #[allow(clippy::too_many_arguments)]
    fn single_existential(
        &self,
        certificate: &fsl_replay::PackageHyperClosureRequest<'_>,
        clause: &CheckedHyperClause,
        simulation: ModelSimulationRequest<'_>,
        systems: &[&ModelSystem<'_>],
        form: &TemporalItem,
        limits: AnalyzeLimits,
        cancel: &Cancel,
    ) -> EngineReport {
        use fsl_replay::{StateGraphMethod, StateGraphPropertyForm, TerminalValue};

        if !clause.parameters().is_empty() {
            return EngineReport::Refused(Refusal::Unsupported(super::items::NOT_YET));
        }
        let [system] = systems else {
            return EngineReport::Refused(Refusal::Failed(BROKEN));
        };
        let mut budgets = form.limits;
        if let Some(samples) = form.witness_samples {
            budgets.witness_samples = samples;
        }
        let subject = ModelSubject {
            simulation: simulation.clone(),
            proof_bounds: form.proof_bounds.clone(),
        };
        let letters =
            match CheckedHyperLetters::for_evaluator(self.input.evaluator, clause, vec![*system]) {
                Ok(letters) => letters,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
        let run = match crate::model_check::check_single_existential(
            crate::model_check::SingleExistentialRequest {
                clause,
                subject,
                letters: &letters,
                limits: budgets,
                max_depth: form.max_depth,
                seed: form.seed,
                meter: limits.accounting,
            },
            cancel,
        ) {
            Ok(run) => run,
            Err(refusal) => return EngineReport::Refused(model_refusal(&refusal)),
        };
        let subjects = [ModelSubject {
            simulation,
            proof_bounds: form.proof_bounds.clone(),
        }];
        let max_depth = match form.max_depth.map(u64::try_from).transpose() {
            Ok(depth) => depth,
            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
        };
        let method = StateGraphMethod {
            limits: budgets,
            max_depth,
            seed: run.seed,
        };
        let mut reports = Vec::with_capacity(run.instances.len());
        for instance in &run.instances {
            let settled = match fsl_replay::settle_package_existential(
                certificate,
                instance,
                &method,
                cancel,
            ) {
                Ok(settled) => settled,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
            let wire = if let Some(evidence) = settled.evidence.as_ref() {
                let reference = match (clause.parameter_object_type(0), evidence.over().first()) {
                    (Some(declared), Some(instance)) => {
                        match system.written_reference(declared.clone(), instance) {
                            Ok(reference) => Some(reference),
                            Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                        }
                    }
                    (None, None) => None,
                    _ => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                };
                match fsl_replay::graph_evidence_wire(
                    evidence,
                    StateGraphPropertyForm::Possible,
                    clause
                        .parameters()
                        .first()
                        .map(|parameter| parameter.name.as_str()),
                    reference.as_ref(),
                    &[],
                    clause.assembled_occurrence(),
                ) {
                    Ok(wire) => Some(wire),
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                }
            } else {
                None
            };
            let refuted = matches!(settled.record.value, TerminalValue::Refuted);
            let proved = matches!(settled.record.value, TerminalValue::Proved { .. });
            reports.push(super::GraphReport {
                record: settled.record,
                instance: None,
                exploration: instance.stats,
                counterexample: if refuted { wire.clone() } else { None },
                evidence: if !refuted { wire } else { None },
                certificate_document: None,
                checker: proved.then_some(fsl_replay::CertificateChecker::ProductClosure),
            });
            if refuted {
                break;
            }
        }
        if reports.is_empty() {
            let cause = if subjects[0].simulation.initial.is_empty() {
                fsl_replay::InconclusiveCause::NoInitialState
            } else {
                fsl_replay::InconclusiveCause::NoInstance
            };
            return super::instance_records::undecided(cause, &method, budgets.witness_samples);
        }
        let records: Vec<_> = reports.iter().map(|report| report.record.clone()).collect();
        match reports
            .into_iter()
            .nth(super::instance_records::decisive(&records))
        {
            Some(report) => EngineReport::StateGraph(Box::new(report)),
            None => EngineReport::Refused(Refusal::Failed(BROKEN)),
        }
    }
}
