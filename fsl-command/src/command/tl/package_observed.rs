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
use fsl_eval::temporal::{PackagePositions, TemporalTrace};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::BarePackage;
use fsl_replay::spine::{
    read_clause_selection, ClauseDisposition, ClauseRunSelection, ObservedValuation,
    PackageBehaviours, SuppliedObservation, TraceInput,
};
use fsl_replay::BehaviourValuations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::PackageInput;

use super::{CrossEvaluate, TlLowering};
use crate::command::{wire, Result, RunCause};

/// What a package run reads beyond the package: the domain packages its
/// model selections bind to and the snapshot of an `initial` request.
pub(in crate::command) struct PackageRun<'a> {
    /// The domain package documents the request supplied.
    pub(in crate::command) packages: &'a PackageInput,
    /// The limits admission re-normalizes them under.
    pub(in crate::command) model_limits: ModelNormalizationLimits,
    /// The bytes of the `initial` snapshot file.
    pub(in crate::command) snapshot: Option<&'a [u8]>,
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

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

fn fault(message: &str) -> RunCause {
    RunCause::TlFault(message.to_owned())
}

/// The native valuation `lowering`'s `cross_evaluate` asks of the clause
/// declared at `claim`, whose node is `key`.
pub(super) fn observe(
    bare: &BarePackage<'_>,
    claim: &OccurrenceKey,
    key: &filament_exact::NodeKey,
    lowering: &TlLowering,
    run: &PackageRun<'_>,
) -> Result<Observed> {
    let Some(cross) = &lowering.cross else {
        return Ok(Observed::default());
    };
    let model = PackageModelInput::of(bare, run.packages, run.model_limits)
        .map_err(|refusal| not_read(&refusal))?;
    let clause = model
        .temporal_valuation_clause(
            claim,
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )
        .map_err(|refusal| clause_not_read(&refusal))?;
    match cross {
        CrossEvaluate::Trace { .. } => trace(bare, &model, &clause, key, lowering),
        CrossEvaluate::Initial { max_steps, .. } => {
            behaviours(&model, &clause, key, run, *max_steps)
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
    clause: &fsl_semantics::check::CheckedTemporalClause,
    key: &filament_exact::NodeKey,
    lowering: &TlLowering,
) -> Result<Observed> {
    let selection: wire::ClauseRun = serde_json::from_value(super::selection(lowering))?;
    let parts = selection
        .selection
        .into_parts()
        .map_err(|error| invalid(&error.member, error.reason))?;
    let ClauseRunSelection::SampledTemporal { trace, .. } =
        read_clause_selection(parts, &BTreeMap::new(), None)
            .map_err(|error| invalid(&error.member, error.reason))?
    else {
        return Err(invalid(
            "tl_lowering.cross_evaluate.over",
            "a package run reads record samples, which name no object",
        ));
    };
    let filament_exact::ValueType::Composite(record) = &clause.over().1 else {
        return Err(invalid(
            "tl_lowering.cross_evaluate.trace",
            "the clause reads objects of a model, not record samples",
        ));
    };
    let (documents, shape) = match &trace {
        TraceInput::Finite(positions) if positions.is_empty() => return Ok(Observed::default()),
        TraceInput::Finite(positions) => (
            positions.iter().collect::<Vec<_>>(),
            TemporalTrace::Finite {
                len: length(positions.len())?,
            },
        ),
        TraceInput::Lasso { prefix, repeat } => {
            if !fsl_replay::spine::profile_reads(clause.profile()).contains(&"lasso") {
                return Ok(Observed::default());
            }
            if repeat.is_empty() {
                return Err(invalid(
                    "tl_lowering.cross_evaluate.trace",
                    "a lasso's loop holds at least one position",
                ));
            }
            (
                prefix.iter().chain(repeat).collect(),
                TemporalTrace::Lasso {
                    prefix: length(prefix.len())?,
                    loop_len: length(repeat.len())?,
                },
            )
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
    };
    let types = model.input().types;
    let samples = documents
        .into_iter()
        .map(|document| {
            fsl_semantics::model::observation::admit_record_sample(
                types,
                record,
                document.identity.as_bytes(),
            )
            .map_err(|record| RunCause::TlSubject(Box::new(record)))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut meter = filament_exact::Meter::new(fsl_replay::AccountingLimits::default().0);
    let positions = fsl_eval::temporal::package_observed_valuation(
        bare,
        clause,
        key,
        PackagePositions::Samples(&samples),
        shape,
        &mut meter,
    )
    .map_err(|_| fault("the valuation reads every atom and activation"))?;
    Ok(Observed {
        positions: Some(positions),
        behaviours: None,
    })
}

fn length(len: usize) -> Result<u64> {
    u64::try_from(len).map_err(|_| fault("a trace's length fits 64 bits"))
}

/// The valuation of every behaviour of 1 to `max_steps` steps of the model
/// subject the one `initial` snapshot builds.
fn behaviours(
    model: &PackageModelInput<'_>,
    clause: &fsl_semantics::check::CheckedTemporalClause,
    key: &filament_exact::NodeKey,
    run: &PackageRun<'_>,
    max_steps: u64,
) -> Result<Observed> {
    let bytes = run.snapshot.ok_or_else(|| {
        invalid(
            "tl_lowering.cross_evaluate.initial",
            "expected one snapshot file",
        )
    })?;
    let observation = SuppliedObservation::from_document("initial", bytes.to_vec())
        .map_err(|error| invalid(&error.member, error.reason))?;
    let snapshots: fsl_semantics::model::observation::DocumentProvision =
        BTreeMap::from([(observation.reference.clone(), observation.bytes.clone())]);
    let reference = serde_json::json!({
        "authority": observation.reference.authority,
        "identity": observation.reference.identity,
    });
    let subject: fsl_analyze::analyze::document::SubjectSelection =
        serde_json::from_value(serde_json::json!({
            "initial_states": [reference],
            "universes": [reference],
            "proof_bounds": [],
        }))?;
    subject
        .check_universes(&snapshots)
        .map_err(RunCause::AnalyzeUniverse)?;
    let max_steps = usize::try_from(max_steps).map_err(|_| {
        invalid(
            "tl_lowering.cross_evaluate.max_steps",
            "the step bound does not fit this platform",
        )
    })?;
    let observation_limits = fsl_replay::spine::ObservationLimitsWire::resolve(None)
        .map_err(|error| invalid(&error.member, error.reason))?;
    let valuations = fsl_replay::spine::run_package_behaviours(PackageBehaviours {
        input: model.input(),
        clause,
        key,
        packages: run.packages,
        model_limits: run.model_limits,
        snapshots: &snapshots,
        initial: &observation.reference,
        observation_limits,
        accounting: fsl_replay::AccountingLimits::default().0,
        max_steps,
    })
    .map_err(|disposition| match disposition {
        ClauseDisposition::Admit(fsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) => RunCause::TlSubject(record),
        other => RunCause::TlFault(format!("the behaviours were not valued: {other:?}")),
    })?;
    Ok(Observed {
        positions: None,
        behaviours: Some(valuations),
    })
}
