// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1039, FR-330: a temporal clause a checked package carries, run over a
//! trace of snapshots or a model-arm behaviour.
//!
//! The package route reads the clause from the package's own nodes and
//! evaluates its atoms through the package's evaluator
//! ([`fsl_eval::simulation::model::ModelEvaluator`]); everything else is the
//! source route's: the snapshots are admitted as FR-106 admits them, the
//! clause's `over` object is resolved in the first, a behaviour is
//! re-executed by the same replay, and the verdict, counterexample and usage
//! are built as [`super::run_temporal`] builds them. A package holds no
//! source text, so a refused condition names no region.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::node::NodeKind;
use filament_exact::{Meter, NodeKey, ScalarLimits, ValueType};
use fsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem};
use fsl_eval::temporal::{DecisiveAtom, PackagePositions, evaluate_package_positions_decisive};
use fsl_eval::value::{CheckedPackageEvaluation, HoldsAtom, HoldsBinders, ObservationIdentity};
use fsl_semantics::check::{CheckedTemporalClause, FormulaNode};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    AdmissionUsage, DocumentProvision, ModelSelections, ObservationLimits, admit_trace_snapshot,
};

use super::super::model_trace::{behaviour_payload, expansion_stop, replay_refusal, universes};
use super::super::trace_refusal::{PackageClauseSite, TraceSite};
use super::super::{ClauseDisposition, ClauseRunReport, ClauseRunUsage, TemporalVerdict};
use super::{OverObject, TraceInput, refused, resolve_over_in, with_counterexample};
use crate::model_check::replay::{ModelTraceEnd, ModelTraceRun, evaluate_model_trace};

/// A run the package route reads no verdict for, and why.
#[derive(Debug)]
pub enum PackageSelectionNotRead {
    /// A selection the package adapter cannot read.
    Selection {
        /// The request member.
        member: &'static str,
        /// The owning reader's reason.
        reason: String,
    },
    /// The original run refusal, retaining its call stage and source identity.
    Run(Box<super::RunRefusal>),
    /// Attribution is unavailable beside the original evaluated result.
    Attribution(Box<super::package_window::PackageAtomAttributionFailure>),
}

/// FR-330: one temporal clause a checked package carries, run over snapshots
/// of its model world.
pub struct PackageSnapshotRequest<'a, 'p> {
    /// Caller-admitted sources used only to render regions and atom labels.
    pub sources: &'a [fsl_foundation::Source],
    /// The existing covered-position limit for this whole run's ledger.
    pub progress_capacity: u64,
    /// The package whose nodes carry the clause.
    pub package: &'a fsl_package::BarePackage<'p>,
    /// The package's model-check input, which holds its evaluator, its types
    /// and its model selections.
    pub input: &'a crate::PackageModelCheckInput<'a>,
    /// The temporal clause's declared name.
    pub name: &'a str,
    /// The package form of the clause and its node's key; `None` when the
    /// package declares no clause of that name.
    pub clause: Option<(&'a CheckedTemporalClause, &'a NodeKey)>,
    /// The domain packages the package selects.
    pub packages: &'a PackageInput,
    /// The limits admission re-normalizes the domain packages under.
    pub model_limits: ModelNormalizationLimits,
    /// The snapshot documents the trace names.
    pub snapshots: &'a DocumentProvision,
    /// FR-106's observation limits.
    pub observation_limits: ObservationLimits,
    /// The evaluation meter's accounting limits.
    pub accounting: ScalarLimits,
    /// The trace: a finite or lasso trace of snapshots, or a model-arm
    /// behaviour.
    pub trace: &'a TraceInput,
    /// The object the clause's `over` parameter names.
    pub over: &'a OverObject,
    /// The progress ledger the request carries, which only a trace that
    /// asserts progress accepts.
    pub ledger: Option<&'a super::ProgressLedger>,
}

fn report(disposition: ClauseDisposition, usage: ClauseRunUsage) -> ClauseRunReport {
    *super::disposition_report(disposition, usage)
}

/// The declaration identities of the populations the clause's `holds` atoms
/// and guard name (FR-391), ascending and each once: each is required in every
/// snapshot of the trace (FR-106 check 7). Read from the projected conditions;
/// one that does not project is read as naming none, and the evaluation
/// reports its failure.
fn named_populations(
    package: &fsl_package::BarePackage<'_>,
    clause: &CheckedTemporalClause,
    key: &NodeKey,
) -> Vec<String> {
    let mut named = BTreeSet::new();
    let mut collect = |projection: &fsl_package::ReaderProtocolProjection| {
        let mut index = 0;
        while let Some(kind) = projection.kind(index) {
            if let NodeKind::NamedPopulation { population, .. } = kind {
                named.insert(population.node.clone());
            }
            index += 1;
        }
    };
    for (index, node) in clause.formula().nodes().iter().enumerate() {
        if matches!(node, FormulaNode::Holds { .. }) {
            if let Ok(term) = package.project_holds(key, index) {
                collect(term.projection());
            }
        }
    }
    if let Ok(Some(term)) = package.project_guard(key) {
        collect(term.projection());
    }
    named.into_iter().collect()
}

/// FR-330 "Admit" and "Evaluate" for a package clause over snapshots.
///
/// # Errors
///
/// [`PackageSelectionNotRead`] for a trace the package route does not read,
/// and for a violation, whose decisive atom a package cannot report.
pub fn run_package_snapshots(
    request: &PackageSnapshotRequest<'_, '_>,
) -> Result<ClauseRunReport, PackageSelectionNotRead> {
    let PackageSnapshotRequest {
        package,
        input,
        name,
        clause,
        packages,
        model_limits,
        snapshots,
        observation_limits,
        accounting,
        trace,
        over,
        ledger,
        sources,
        progress_capacity,
    } = *request;
    let input = input.as_inner();
    let early = |disposition| Ok(report(disposition, ClauseRunUsage::default()));
    let Some((clause, key)) = clause else {
        return early(ClauseDisposition::MissingName {
            name: name.to_owned(),
        });
    };
    let site = PackageClauseSite { clause };
    // FR-645: a trace kind the clause's profile does not read refuses at stage
    // `admit` before any position is admitted.
    if !super::super::trace_refusal::profile_reads(clause.profile()).contains(&trace.kind()) {
        return early(ClauseDisposition::TraceRefused(Box::new(
            site.kind_mismatch(trace.kind()),
        )));
    }
    let authenticated = match super::authenticate_bare_timed(package.package(), name, trace) {
        Ok(selection) => selection,
        Err(disposition) => return early(*disposition),
    };
    let screened = match super::package_window::screen(clause, trace) {
        Ok(screened) => screened,
        Err(disposition) => return early(*disposition),
    };
    let trace = screened.as_ref().map_or(trace, |screened| &screened.trace);
    // A ledger retains progress; a trace that asserts none has nothing to
    // retain (FR-502 Behavior).
    if ledger.is_some() && !trace.carries_progress() {
        return early(ClauseDisposition::TraceRefused(Box::new(
            site.ledger_without_progress(),
        )));
    }
    let (_, over_type) = clause.over();
    let ValueType::Reference(type_identity) = over_type else {
        return early(refused(
            "invalid-value",
            BTreeMap::from([("member", "over".to_owned())]),
        ));
    };
    if let TraceInput::Model { .. } = trace {
        return run_model(request, clause, key);
    }
    let Some(shape) = trace.shape() else {
        return early(refused("invalid-value", BTreeMap::new()));
    };
    // FR-645: a trace with no position, or a lasso with no loop position, has
    // no observation to evaluate over.
    let empty = super::empty_trace_refusal(&site, trace);
    if let Some(refusal) = empty {
        return early(ClauseDisposition::TraceRefused(Box::new(refusal)));
    }

    let mut meter = Meter::new(accounting);
    let progressed = match super::package_window::progressed(clause, trace, &mut meter) {
        Ok(progressed) => progressed,
        Err(disposition) => return early(*disposition),
    };
    let named = named_populations(package, clause, key);
    let mut admitted = AdmissionUsage::default();
    let mut environments = Vec::new();
    for document in trace.documents() {
        match admit_trace_snapshot(
            ModelSelections::new(input.model_selections, input.source_packages),
            input.types,
            packages,
            model_limits,
            snapshots,
            document,
            observation_limits,
            &named,
        ) {
            Ok((environment, usage)) => {
                admitted.document_bytes =
                    admitted.document_bytes.saturating_add(usage.document_bytes);
                admitted.objects = admitted.objects.saturating_add(usage.objects);
                admitted.values = admitted.values.saturating_add(usage.values);
                environments.push(environment);
            }
            Err(failure) => {
                return Ok(report(
                    ClauseDisposition::Admit(failure),
                    ClauseRunUsage {
                        admission_consumed: admitted,
                        ..ClauseRunUsage::default()
                    },
                ));
            }
        }
    }
    let resolved = match resolve_over_in(
        ModelSelections::new(input.model_selections, input.source_packages),
        packages,
        model_limits,
        type_identity,
        over,
        environments.first(),
    ) {
        Ok(reference) => reference,
        Err(miss) => {
            return Ok(report(
                miss.disposition(over),
                ClauseRunUsage {
                    admission_consumed: admitted,
                    ..ClauseRunUsage::default()
                },
            ));
        }
    };
    let positions = PackagePositions::Objects {
        environments: &environments,
        over: &resolved,
    };
    let context =
        super::package_window::Context::new(package, input.evaluator, clause, key, name, sources)?;
    if trace.carries_progress() {
        let mut finished = super::package_window::run(
            &context,
            positions,
            trace,
            screened.as_ref(),
            progressed.as_ref(),
            ledger,
            progress_capacity,
            admitted,
            &mut meter,
        )?;
        finished.authenticated_temporal_selection = authenticated;
        return Ok(finished);
    }
    let observed = if let TraceInput::Timed {
        prefix,
        repeat,
        period,
        through,
        ..
    } = trace
    {
        fsl_eval::temporal::evaluate_package_timed(
            input.evaluator,
            clause,
            key,
            positions,
            (
                prefix
                    .iter()
                    .chain(repeat)
                    .map(|p| p.stamp.clone())
                    .collect(),
                period.as_ref().map(|period| {
                    (
                        u64::try_from(prefix.len()).unwrap_or(u64::MAX),
                        period.clone(),
                    )
                }),
                through.clone(),
            ),
            &mut meter,
        )
    } else {
        evaluate_package_positions_decisive(
            input.evaluator,
            clause,
            key,
            positions,
            shape,
            &mut meter,
        )
    };
    let inactive = observed.inactive;
    // FR-330 Outputs: a violation carries the decisive record of the atom the
    // search found; the package reads a search that found none, and a search
    // that stopped, as the source route does.
    let decisive = observed.decisive;
    let (disposition, refused_at) = match observed.stopped {
        Some(fsl_eval::temporal::HoldsStop::Refused { at, evaluation }) => {
            (context.refused(evaluation)?, Some(at.index()))
        }
        stopped => super::package_disposition(stopped, observed.result, inactive),
    };
    let violated = matches!(
        disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
    );
    let (disposition, counterexample) =
        with_counterexample(clause, trace, disposition, observed.undefined);
    let (disposition, counterexample, decisive) = super::with_decisive_by(
        violated.then_some(decisive).flatten(),
        disposition,
        counterexample,
        |atom, meter| {
            let slot = usize::try_from(atom.position.index())
                .ok()
                .and_then(|at| super::observation_slot(trace, at))
                .ok_or_else(|| decisive_fault("decisive-position-has-an-observation"))?;
            let document = trace
                .documents()
                .nth(slot)
                .ok_or_else(|| decisive_fault("decisive-position-has-a-document"))?;
            let objects = environments
                .get(slot)
                .ok_or_else(|| decisive_fault("decisive-position-has-an-environment"))?;
            decisive_over(
                package,
                clause,
                key,
                atom,
                &resolved,
                objects,
                ObservationIdentity {
                    authority: document.authority.clone(),
                    identity: document.identity.clone(),
                },
                meter,
            )
        },
        &mut meter,
    );
    let mut finished = finished(disposition, (&meter, admitted), counterexample, refused_at);
    finished.decisive = decisive;
    finished.authenticated_temporal_selection = authenticated;
    Ok(finished)
}

fn finished(
    disposition: ClauseDisposition,
    (meter, admitted): (&Meter, AdmissionUsage),
    counterexample: Option<crate::ObservedTemporalCounterexample>,
    refused_at: Option<u64>,
) -> ClauseRunReport {
    let mut finished = report(disposition, ClauseRunUsage::from_meter(meter, admitted));
    finished.counterexample = counterexample.map(Into::into);
    finished.refused_at = refused_at;
    finished
}

/// FR-330 "Admit" and "Evaluate" for `TraceInput::Model` over a package: the
/// behaviour is re-executed over the model system the package and the
/// universe snapshots define, and the clause evaluated over the reconstructed
/// states through the package's evaluator.
fn run_model(
    request: &PackageSnapshotRequest<'_, '_>,
    clause: &CheckedTemporalClause,
    key: &NodeKey,
) -> Result<ClauseRunReport, PackageSelectionNotRead> {
    let PackageSnapshotRequest {
        input,
        packages,
        model_limits,
        snapshots,
        observation_limits,
        accounting,
        trace,
        over,
        ..
    } = *request;
    let input = input.as_inner();
    let TraceInput::Model {
        initial,
        universes: universe_documents,
        prefix,
        repeat,
        terminal_stutter,
        max_candidates,
    } = trace
    else {
        return Err(PackageSelectionNotRead::Selection {
            member: "clause.selection.trace",
            reason: "a model trace is required".to_owned(),
        });
    };
    let mut admitted = AdmissionUsage::default();
    let stopped = |disposition, admitted| {
        Ok(report(
            disposition,
            ClauseRunUsage {
                admission_consumed: admitted,
                ..ClauseRunUsage::default()
            },
        ))
    };
    // FR-645: a model behaviour whose loop holds no step is refused, as a
    // lasso with an empty loop is (FR-329-AC-3).
    if repeat.is_empty() && !*terminal_stutter {
        return stopped(
            ClauseDisposition::TraceRefused(Box::new(PackageClauseSite { clause }.empty_loop())),
            admitted,
        );
    }
    // FR-330: the universe snapshots are observations of the trace, admitted
    // by FR-106 before the behaviour is simulated over their populations.
    for document in universe_documents {
        match admit_trace_snapshot(
            ModelSelections::new(input.model_selections, input.source_packages),
            input.types,
            packages,
            model_limits,
            snapshots,
            document,
            observation_limits,
            &[],
        ) {
            Ok((_, usage)) => {
                admitted.document_bytes =
                    admitted.document_bytes.saturating_add(usage.document_bytes);
                admitted.objects = admitted.objects.saturating_add(usage.objects);
                admitted.values = admitted.values.saturating_add(usage.values);
            }
            Err(failure) => return stopped(ClauseDisposition::Admit(failure), admitted),
        }
    }
    let fault = |invariant: &'static str| {
        Ok(report(
            ClauseDisposition::EvaluateFault(filament_semantic_value::catalog::InternalFault::new(
                "temporal", invariant,
            )),
            ClauseRunUsage::default(),
        ))
    };
    let Some(universes) = universes(snapshots, universe_documents) else {
        return fault("admitted-universe-snapshot-reads");
    };
    let system = match ModelSystem::new(ModelSimulationRequest {
        input: input.clone(),
        packages,
        normalization: model_limits,
        snapshots,
        initial: vec![initial.clone()],
        universes,
        observation: observation_limits,
        expansion: ExpansionLimits {
            max_candidates: *max_candidates,
        },
        meter: accounting,
        emit: fsl_package::EmitLimits::new(u64::MAX),
    }) {
        Ok(system) => system,
        Err(failure) => return stopped(ClauseDisposition::Admit(failure), admitted),
    };
    let instance = match system.instances(&clause.over().1) {
        Ok(instances) => instances
            .into_iter()
            .find(|instance| instance.key == over.key),
        Err(_) => return fault("model-instances-are-listed"),
    };
    let Some(instance) = instance else {
        return stopped(
            refused(
                "wrong-role-mapping",
                BTreeMap::from([
                    ("population", over.population.clone()),
                    ("object", over.key.clone()),
                ]),
            ),
            admitted,
        );
    };
    let letters = match system.package_clause_letters(clause, key) {
        Ok(letters) => letters,
        Err(fsl_eval::simulation::model::LettersRefusal::ReferenceBindingsAdmission(failure)) => {
            return stopped(ClauseDisposition::Admit(failure), admitted);
        }
        Err(_) => return fault("model-clause-has-letters"),
    };
    let payload = behaviour_payload(prefix, repeat, *terminal_stutter, instance);
    let evaluated = match evaluate_model_trace(&system, &payload, clause, &letters, accounting) {
        Ok(ModelTraceEnd::Evaluated(evaluated)) => *evaluated,
        Ok(ModelTraceEnd::Stopped(stop)) => {
            return stopped(expansion_stop(&stop, accounting, *max_candidates), admitted);
        }
        Err(refusal) => return stopped(replay_refusal(refusal), admitted),
    };
    let ModelTraceRun {
        bound_over,
        terminal_state,
        call_failure,
        outcome,
        decisive,
        mut meter,
        observation,
        ..
    } = evaluated;
    if let Some(failure) = call_failure {
        return Ok(finished(
            super::package_call_disposition(failure),
            (&meter, admitted),
            None,
            None,
        ));
    }
    let (disposition, _) = super::package_disposition(None, outcome, false);
    let violated = matches!(
        disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
    );
    let (disposition, _, decisive) = super::with_decisive_by(
        violated.then_some(decisive).flatten(),
        disposition,
        None::<crate::ObservedTemporalCounterexample>,
        |atom, meter| {
            let observed = observation
                .as_ref()
                .ok_or_else(|| decisive_fault("decisive-atom-has-an-observation"))?;
            decisive_over(
                request.package,
                clause,
                key,
                atom,
                &observed.over,
                &observed.environment,
                ObservationIdentity {
                    authority: observed.identity.authority.clone(),
                    identity: observed.identity.identity.clone(),
                },
                meter,
            )
        },
        &mut meter,
    );
    let counterexample = super::super::model_trace::model_counterexample(
        clause,
        &disposition,
        payload,
        bound_over,
        terminal_state,
    );
    let mut report = finished(disposition, (&meter, admitted), None, None);
    report.counterexample = counterexample
        .map(Box::new)
        .map(crate::ClauseRunCounterexample::Model);
    report.decisive = decisive;
    Ok(report)
}

fn decisive_fault(invariant: &'static str) -> Box<ClauseDisposition> {
    Box::new(ClauseDisposition::EvaluateFault(
        filament_semantic_value::catalog::InternalFault::new("temporal", invariant),
    ))
}

/// Re-evaluate the selected false atom with the witness channel, then follow
/// the actual short-circuit decisions through its admitted expression.
#[allow(clippy::too_many_arguments)]
pub(super) fn decisive_over(
    package: &fsl_package::BarePackage<'_>,
    clause: &CheckedTemporalClause,
    key: &NodeKey,
    atom: DecisiveAtom,
    over: &filament_exact::ObjectReference,
    objects: &fsl_semantics::model::object_environment::ObjectEnvironment,
    observation: ObservationIdentity,
    meter: &mut Meter,
) -> Result<Option<crate::TemporalDecisive>, Box<ClauseDisposition>> {
    use filament_eval::FamilyOutcome;
    use filament_exact::{Outcome, Value};
    if !matches!(
        clause.formula().nodes().get(atom.atom.index()),
        Some(FormulaNode::Holds { .. })
    ) {
        return Ok(None);
    }
    let named = HoldsAtom {
        clause: key.clone(),
        position: atom.atom.index(),
    };
    let evaluation = package
        .evaluate_holds_traced(
            &named,
            HoldsBinders {
                over,
                captures: &[],
            },
            objects,
            observation,
            meter,
        )
        .map_err(|failure| Box::new(super::package_call_disposition(failure)))?;
    match &evaluation.evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {}
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            return Err(Box::new(ClauseDisposition::Temporal(
                TemporalVerdict::Incomplete(incomplete.clone()),
            )));
        }
        FamilyOutcome::Evaluated(_) | FamilyOutcome::FamilyEvaluated(_) => {
            return Err(decisive_fault("decisive-atom-reads-false-again"));
        }
    }
    let term = package
        .project_holds(key, atom.atom.index())
        .map_err(|_| decisive_fault("decisive-atom-has-a-condition"))?;
    let stop = crate::witness::decisive_package_stop(term.projection(), &evaluation)
        .map_err(|internal| Box::new(ClauseDisposition::EvaluateFault(internal)))?;
    let decisive = match stop {
        Some(report) => crate::TemporalDecisive::from_stop(report),
        None => crate::TemporalDecisive::undecided(
            evaluation
                .receiver_path()
                .cloned()
                .ok_or_else(|| decisive_fault("decisive-atom-has-a-root-path"))?,
        ),
    };
    Ok(Some(decisive.at_position(atom.position.index())))
}
