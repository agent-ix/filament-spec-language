// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-931 "Cross-evaluation" over a selected checked package: the native
//! valuation of the clause a `{directory}` selection names, read from the
//! package's own nodes and evaluator as the source route reads it from the
//! checked clause.
//!
//! The `trace` form reads record samples, each position's atoms evaluated
//! through the package's slot call; the `initial` form enumerates the
//! behaviours of the model subject the package route builds from the one
//! snapshot, through the same enumerator the source route uses.

use std::collections::BTreeMap;

use fsl_eval::simulation::model::{PackageModelInput, PendingClauseForm, PendingPackageForm};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::BarePackage;
use fsl_replay::spine::{
    read_clause_selection, ClauseDisposition, ClauseRunReport, ClauseRunSelection,
    ObservedValuation, PackageBehaviours, PackageSampledRequest, SuppliedObservation, TraceInput,
};
use fsl_replay::BehaviourValuations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::DocumentProvision;

use super::{CrossEvaluate, TlLowering};
use crate::command::{wire, Result, RunCause};

/// What a package run reads beyond the package: the domain packages its
/// model selections bind to and the snapshot of an `initial` request.
pub(in crate::command) struct PackageRun<'a> {
    /// Explicit caller sources for any refused condition's source rendering.
    pub(in crate::command) sources: &'a [fsl_foundation::Source],
    /// The domain package documents the request supplied.
    pub(in crate::command) packages: &'a PackageInput,
    /// The limits admission re-normalizes them under.
    pub(in crate::command) model_limits: ModelNormalizationLimits,
    /// The bytes of the `initial` snapshot file.
    pub(in crate::command) snapshot: Option<&'a [u8]>,
    /// The accounting limits each valuation runs under.
    pub(in crate::command) accounting: filament_exact::ScalarLimits,
    /// The limits the `initial` snapshot is admitted under.
    pub(in crate::command) observation_limits: fsl_semantics::model::observation::ObservationLimits,
}

/// The snapshot a `cross_evaluate.initial` request builds its model subject
/// from, checked as the source route checks it.
pub(in crate::command) struct Initial {
    /// The snapshot, read as an observation named `initial`.
    observation: SuppliedObservation,
    /// The snapshot document, by reference.
    snapshots: DocumentProvision,
    /// The longest behaviour, in steps.
    max_steps: usize,
}

/// The native valuation of a package's clause: the one trace of a `trace`
/// request, or the behaviours of an `initial` request. A request that asks
/// for the clause alone, or whose trace the clause's profile does not read,
/// holds neither.
#[derive(Debug, Default)]
pub(in crate::command) struct Observed {
    /// The valuation of the supplied trace.
    pub(in crate::command) positions: Option<ObservedValuation>,
    /// The valuation of every behaviour of the snapshot's subject.
    pub(in crate::command) behaviours: Option<BehaviourValuations>,
}

/// A run that stopped before it had a valuation: the clause report the source
/// route gives for the same stop, and the selection it echoes.
pub(in crate::command) struct Reported {
    /// The clause report.
    pub(in crate::command) report: Box<ClauseRunReport>,
    /// The selection the report echoes.
    pub(in crate::command) selection: ClauseRunSelection,
}

/// What reading a package's clause came to.
pub(in crate::command) enum Observation {
    /// The valuation the lowering compares with.
    Valued(Observed),
    /// A stop the source route reports as a clause report.
    Reported(Box<Reported>),
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// The `initial` snapshot of `lowering`, when it asks for behaviours: read,
/// and its universes checked, before the clause is resolved, as the source
/// route's `prepare_lowering` does.
pub(in crate::command) fn initial(
    lowering: &TlLowering,
    run: &PackageRun<'_>,
) -> Result<Option<Initial>> {
    let Some(CrossEvaluate::Initial { initial, max_steps }) = &lowering.cross else {
        return Ok(None);
    };
    let bytes = run.snapshot.ok_or_else(|| {
        invalid(
            "tl_lowering.cross_evaluate.initial",
            "expected one snapshot file",
        )
    })?;
    let mut observation = SuppliedObservation::from_document(super::INITIAL_KEY, bytes.to_vec())
        .map_err(|error| invalid(&error.member, error.reason))?;
    observation.path = Some(initial.clone());
    let snapshots: DocumentProvision =
        BTreeMap::from([(observation.reference.clone(), observation.bytes.clone())]);
    let (_, max_steps) = crate::command::initial_subject(&snapshots, *max_steps)?;
    Ok(Some(Initial {
        observation,
        snapshots,
        max_steps,
    }))
}

/// The selection the source route reads `lowering`'s clause run from.
fn clause_selection(
    lowering: &TlLowering,
    initial: Option<&Initial>,
) -> Result<ClauseRunSelection> {
    let selection: wire::ClauseRun = serde_json::from_value(super::selection(lowering))?;
    let parts = selection
        .selection
        .into_parts()
        .map_err(|error| invalid(&error.member, error.reason))?;
    let observations = initial
        .map(|initial| {
            BTreeMap::from([(super::INITIAL_KEY.to_owned(), initial.observation.clone())])
        })
        .unwrap_or_default();
    read_clause_selection(parts, &observations, None)
        .map_err(|error| invalid(&error.member, error.reason))
}

/// The native valuation `lowering`'s `cross_evaluate` asks of the clause
/// declared at `claim`, whose node is `key`.
pub(super) fn observe(
    bare: &BarePackage<'_>,
    claim: &OccurrenceKey,
    key: &filament_exact::NodeKey,
    lowering: &TlLowering,
    run: &PackageRun<'_>,
    initial: Option<&Initial>,
) -> Result<Observation> {
    let Some(cross) = &lowering.cross else {
        return Ok(Observation::Valued(Observed::default()));
    };
    let model = PackageModelInput::of(bare, run.packages, run.model_limits)
        .map_err(|refusal| not_read(&refusal))?;
    let clause = model
        .temporal_run_clause(
            claim,
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )
        .map_err(|refusal| clause_not_read(&refusal))?;
    let selection = clause_selection(lowering, initial)?;
    match cross {
        CrossEvaluate::Trace { .. } => trace(bare, &model, (&clause, key), run, selection),
        CrossEvaluate::Initial { .. } => {
            let initial = initial.ok_or_else(|| {
                RunCause::TlFault("an initial request reads its snapshot".to_owned())
            })?;
            behaviours(&model, (&clause, key), run, initial, selection)
        }
    }
}

/// The refusal of a package whose model world the package route does not read.
fn not_read(refusal: &PendingPackageForm) -> RunCause {
    invalid("tl_lowering.cross_evaluate", refusal.to_string())
}

/// The refusal of a clause whose package form the route does not read.
fn clause_not_read(refusal: &PendingClauseForm) -> RunCause {
    invalid("tl_lowering.cross_evaluate", refusal.to_string())
}

/// The valuation of the record samples of a `trace` request.
fn trace(
    bare: &BarePackage<'_>,
    model: &PackageModelInput<'_>,
    (clause, key): (
        &fsl_semantics::check::CheckedTemporalClause,
        &filament_exact::NodeKey,
    ),
    run: &PackageRun<'_>,
    selection: ClauseRunSelection,
) -> Result<Observation> {
    let ClauseRunSelection::SampledTemporal { name, trace, .. } = &selection else {
        return Err(invalid(
            "tl_lowering.cross_evaluate.over",
            "a package run reads record samples, which name no object",
        ));
    };
    let filament_exact::ValueType::Composite(_) = &clause.over().1 else {
        return Err(invalid(
            "tl_lowering.cross_evaluate.trace",
            "the clause reads objects of a model, not record samples",
        ));
    };
    match trace {
        TraceInput::Finite(positions) if positions.is_empty() => {
            return Ok(Observation::Valued(Observed::default()))
        }
        TraceInput::Finite(_) => {}
        TraceInput::Lasso { repeat, .. } => {
            if !fsl_replay::spine::profile_reads(clause.profile()).contains(&"lasso") {
                return Ok(Observation::Valued(Observed::default()));
            }
            if repeat.is_empty() {
                return Err(invalid(
                    "tl_lowering.cross_evaluate.trace",
                    "a lasso's loop holds at least one position",
                ));
            }
        }
        TraceInput::Progressed { .. }
        | TraceInput::Timestamped { .. }
        | TraceInput::Timed { .. }
        | TraceInput::Model { .. } => {
            return Err(invalid(
                "tl_lowering.cross_evaluate.trace",
                "only a finite trace or a lasso is cross-evaluated",
            ))
        }
    }
    let request = PackageSampledRequest {
        package: bare,
        sources: run.sources,
        ledger: None,
        progress_capacity: fsl_replay::spine::SpineLimits::default()
            .progress
            .covered_positions,
        evaluator: bare.into(),
        name,
        clause: Some((clause, key)),
        types: model.input().types,
        trace,
        accounting: run.accounting,
    };
    match fsl_replay::spine::run_package_valuation(&request) {
        Ok(positions) => Ok(Observation::Valued(Observed {
            positions: Some(positions),
            behaviours: None,
        })),
        Err(report) => Ok(Observation::Reported(Box::new(Reported {
            report,
            selection,
        }))),
    }
}

/// The valuation of every behaviour of 1 to `max_steps` steps of the model
/// subject the one `initial` snapshot builds.
fn behaviours(
    model: &PackageModelInput<'_>,
    (clause, key): (
        &fsl_semantics::check::CheckedTemporalClause,
        &filament_exact::NodeKey,
    ),
    run: &PackageRun<'_>,
    initial: &Initial,
    selection: ClauseRunSelection,
) -> Result<Observation> {
    let valuations = fsl_replay::spine::run_package_behaviours(PackageBehaviours {
        input: model.input().into(),
        clause,
        key,
        packages: run.packages,
        model_limits: run.model_limits,
        snapshots: &initial.snapshots,
        initial: &initial.observation.reference,
        observation_limits: run.observation_limits,
        accounting: run.accounting,
        max_steps: initial.max_steps,
    });
    match valuations {
        Ok(valuations) => Ok(Observation::Valued(Observed {
            positions: None,
            behaviours: Some(valuations),
        })),
        // FR-931: a snapshot the subject builder refuses refuses the request.
        Err(disposition) => match *disposition {
            ClauseDisposition::Admit(
                fsl_semantics::model::observation::AdmissionFailure::Refused(record),
            ) => Err(RunCause::TlSubject(record)),
            other => Ok(Observation::Reported(Box::new(Reported {
                report: fsl_replay::spine::disposition_report(
                    other,
                    fsl_replay::spine::ClauseRunUsage::default(),
                ),
                selection,
            }))),
        },
    }
}
