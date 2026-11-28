// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-190: decode a direct `model_based_test` request.

use ix_trace_rs::implements;
use serde_json::Value;

use fsl_foundation::Code;

use super::{
    package_name, wire, FormalSource, Intake, PreparedModelBasedTestRun, PreparedRun, Result,
    RunCause,
};

/// Limits and selection admitted by the direct model-based test member.
pub(super) struct Request {
    pub(super) clause: String,
    pub(super) seed: u64,
    pub(super) max_cases: u64,
    pub(super) max_trace_length: Option<u64>,
    pub(super) max_value_bits: Option<u8>,
    pub(super) initial: Option<String>,
}

/// A refusal of the `model_based_test` member `member` with its catalog cause.
fn refuse(member: &str, cause: &'static str) -> RunCause {
    RunCause::ModelBasedTestRequest {
        code: Code::InvalidRuntimeInput,
        path: format!("/model_based_test/{member}"),
        cause,
    }
}

/// A malformed part of the request outside the `model_based_test` member.
fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// The scalar a member must hold, refused `missing-member` when absent and
/// `invalid-value` when present but unusable.
fn scalar<'a, T>(
    members: &'a serde_json::Map<String, Value>,
    member: &str,
    read: impl FnOnce(&'a Value) -> Option<T>,
) -> Result<T> {
    let value = members
        .get(member)
        .ok_or_else(|| refuse(member, "missing-member"))?;
    read(value).ok_or_else(|| refuse(member, "invalid-value"))
}

/// Read the closed member and validate its scalar bounds before source intake.
pub(super) fn decode(value: &Value) -> Result<Request> {
    let Value::Object(members) = value else {
        return Err(invalid("model_based_test", "expected an object"));
    };
    if let Some(unknown) = members.keys().find(|key| {
        !matches!(
            key.as_str(),
            "clause" | "seed" | "max_cases" | "max_trace_length" | "max_value_bits" | "initial"
        )
    }) {
        return Err(refuse(unknown, "unknown-member"));
    }
    let clause = scalar(members, "clause", |value| {
        value.as_str().filter(|name| !name.is_empty())
    })?
    .to_owned();
    let seed = scalar(members, "seed", Value::as_u64)?;
    let max_cases = scalar(members, "max_cases", |value| {
        value.as_u64().filter(|count| *count > 0)
    })?;
    let max_trace_length = members
        .contains_key("max_trace_length")
        .then(|| {
            scalar(members, "max_trace_length", |value| {
                value.as_u64().filter(|length| *length > 0)
            })
        })
        .transpose()?;
    let max_value_bits = members
        .contains_key("max_value_bits")
        .then(|| {
            scalar(members, "max_value_bits", |value| {
                value.as_u64().and_then(|bits| u8::try_from(bits).ok())
            })
        })
        .transpose()?;
    let initial = members
        .contains_key("initial")
        .then(|| {
            scalar(members, "initial", |value| {
                value.as_str().filter(|path| !path.is_empty())
            })
            .map(str::to_owned)
        })
        .transpose()?;
    Ok(Request {
        clause,
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        initial,
    })
}

#[derive(serde::Deserialize)]
struct Snapshot {
    model: SnapshotModel,
    #[serde(default)]
    populations: Vec<SnapshotPopulation>,
}

#[derive(serde::Deserialize)]
struct SnapshotModel {
    identity: String,
}

#[derive(serde::Deserialize)]
struct SnapshotPopulation {
    population: String,
    #[serde(default)]
    objects: Vec<SnapshotObject>,
}

#[derive(serde::Deserialize)]
struct SnapshotObject {
    key: String,
}

fn model_test_error(error: fsl_eval::simulation::model::ModelBasedTestRefusal) -> RunCause {
    RunCause::ModelBasedTestRequest {
        code: Code::from_code(error.code).unwrap_or(Code::RuntimeInvariant),
        path: format!("/model_based_test/{}", error.member),
        cause: error.cause,
    }
}

fn partition_record(
    partition: &fsl_eval::simulation::model::ModelBoundaryPartition,
) -> wire::model_based_test_result::Partition {
    use fsl_eval::simulation::model::ModelPartitionStatus;
    use wire::model_based_test_result::{IntegerTag, IntegerValue, Partition, PartitionStatus};
    Partition {
        node: partition.node.to_string(),
        path: partition.path.clone(),
        kind: partition.kind,
        value: IntegerValue {
            kind: IntegerTag::Integer,
            value: partition.value.to_string(),
        },
        text_value: partition.text_value.clone(),
        status: partition.status.as_ref().map(|status| match status {
            ModelPartitionStatus::Covered { first_case } => PartitionStatus::Covered {
                first_case: *first_case,
            },
            ModelPartitionStatus::Refused { first_case } => PartitionStatus::Refused {
                first_case: *first_case,
            },
            ModelPartitionStatus::Uncovered { bounds, .. } => PartitionStatus::Uncovered {
                bounds: bounds.clone(),
            },
        }),
    }
}

fn counterexample_record(
    result: &fsl_eval::simulation::model::ModelBasedTestResult,
) -> Option<wire::model_based_test_result::Counterexample> {
    use fsl_eval::simulation::model::ModelBasedCounterexample;
    use wire::model_based_test_result::{
        AssignedValue, AssignmentCounterexample, AssignmentDisposition, Counterexample, IntegerTag,
        IntegerValue, Invocation, Minimality, Replay, TraceCounterexample, TraceDisposition,
    };
    Some(match result.counterexample.as_ref()? {
        ModelBasedCounterexample::Trace(trace) => {
            let disposition = trace
                .trace
                .findings
                .iter()
                .flat_map(|finding| finding.findings.iter())
                .find_map(|finding| match finding {
                    fsl_eval::simulation::model::ModelFinding::InvariantViolated {
                        clause, ..
                    } if clause == &result.clause => Some(TraceDisposition::InvariantViolated),
                    _ => None,
                })
                .unwrap_or(TraceDisposition::ModelCheckFailed);
            Counterexample::Trace(TraceCounterexample {
                minimality: Minimality::OneMinimal,
                replay: Replay::Reproduced,
                disposition,
                initial: trace.trace.initial.hex(),
                invocations: trace
                    .trace
                    .steps
                    .iter()
                    .map(|step| Invocation {
                        operation: step.transition.operation().to_owned(),
                        receiver: step.transition.receiver().map(str::to_owned),
                        arguments: step.transition.arguments().to_vec(),
                    })
                    .collect(),
            })
        }
        ModelBasedCounterexample::ValueAssignment(values) => {
            Counterexample::Assignment(AssignmentCounterexample {
                minimality: Minimality::OneMinimal,
                replay: Replay::Reproduced,
                disposition: AssignmentDisposition::PredicateFalse,
                assignment: values
                    .iter()
                    .map(|value| match value {
                        filament_exact::Value::Integer(value) => {
                            AssignedValue::Integer(IntegerValue {
                                kind: IntegerTag::Integer,
                                value: value.to_string(),
                            })
                        }
                        other => AssignedValue::Debug {
                            debug: format!("{other:?}"),
                        },
                    })
                    .collect(),
            })
        }
    })
}

fn cause_record(
    cause: &fsl_eval::simulation::model::ModelBasedTestCause,
) -> wire::model_based_test_result::Cause {
    use fsl_eval::simulation::model::{ModelBasedTestCause, ModelBasedTestDefect};
    use wire::model_based_test_result::{
        Cause, CauseName, Defect, LimitCause, LimitReachedKind, ToolFailureCause, ToolFailureKind,
    };
    match cause {
        ModelBasedTestCause::NoEvaluatedCase => Cause::Name(CauseName::NoEvaluatedCase),
        ModelBasedTestCause::LimitReached {
            limit,
            value,
            setting,
        } => Cause::Limit(LimitCause {
            kind: LimitReachedKind::LimitReached,
            limit: limit.limit_name().to_owned(),
            value: *value,
            setting: setting.clone(),
        }),
        ModelBasedTestCause::TraceStepLimit { value } => Cause::Limit(LimitCause {
            kind: LimitReachedKind::LimitReached,
            limit: "max_steps".to_owned(),
            value: *value,
            setting: "simulation.max_steps".to_owned(),
        }),
        ModelBasedTestCause::ToolFailure { defect } => Cause::ToolFailure(ToolFailureCause {
            kind: ToolFailureKind::ToolFailure,
            defect: match defect {
                ModelBasedTestDefect::OracleDisagreement => Defect::OracleDisagreement,
                ModelBasedTestDefect::InvalidShrink => Defect::InvalidShrink,
            },
        }),
        ModelBasedTestCause::NoContractLevelReading => {
            Cause::Name(CauseName::NoContractLevelReading)
        }
    }
}

/// The FR-752 result record of `result`, over the package `domain`.
pub(super) fn result_record(
    result: &fsl_eval::simulation::model::ModelBasedTestResult,
    domain: &str,
) -> wire::ModelBasedTestResultRecord {
    use fsl_eval::simulation::model::ModelBasedTestEvidence;
    use wire::model_based_test_result::{
        Cases, Checks, Provenance, RunLimits, SizeBounded, ValueBoundCuts, Verdict,
    };
    wire::ModelBasedTestResultRecord {
        clause: result.clause.clone(),
        domain: domain.to_owned(),
        status: result.evidence,
        evidence: result.evidence,
        verdict: (result.evidence == ModelBasedTestEvidence::Refuted).then_some(Verdict::Refuted),
        cause: result.cause.as_ref().map(cause_record),
        cases: Cases {
            generated: result.counts.generated,
            passed: result.counts.passing,
            failed: result.counts.failing,
            discarded: result.counts.discarded,
            discarded_by: result.discarded_by.clone(),
        },
        checks: Checks {
            evaluated: result.checks.evaluated,
            skipped: result.checks.skipped.len(),
        },
        limits: RunLimits {
            max_cases: result.max_cases,
            max_trace_length: result.max_trace_length,
            max_value_bits: result.max_value_bits,
        },
        provenance: Provenance {
            seed: result.seed,
            max_value_bits: result.provenance.max_value_bits,
            size_bounded: result
                .provenance
                .size_bounded
                .iter()
                .map(|root| SizeBounded {
                    node: root.node.to_string(),
                    path: root.path.clone(),
                    values: root.values.clone(),
                })
                .collect(),
            value_bound_cuts: ValueBoundCuts {
                count: result.provenance.value_bound_cuts.count,
            },
        },
        partitions: result.partitions.iter().map(partition_record).collect(),
        partition_coverage: result.partition_coverage.clone(),
        operations: result.operations.clone(),
        coverage: result.coverage.clone(),
        counterexample: counterexample_record(result),
    }
}

/// The run-level inputs shared with every other `run` route.
pub(super) struct Context<'a> {
    pub(super) source: FormalSource,
    pub(super) integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    pub(super) library_integer_division: Vec<(String, fsl_replay::AdmittedIntegerDivision)>,
    pub(super) overrides: &'a super::RequestOverrides<'a>,
}

#[implements("FR-752-AC-16", "FR-752-AC-15")]
pub(super) fn prepare(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    member: &Value,
    context: Context<'_>,
) -> Result<PreparedRun> {
    let Context {
        source,
        integer_division,
        library_integer_division,
        overrides,
    } = context;
    // The member is closed, so no request carries these: they run at the
    // published defaults and the caller's values.
    let published = fsl_analyze::analyze::AnalyzeLimits::published(
        fsl_analyze::analyze::DeadlineSupply::Unbounded,
    );
    let effective = super::overrides::effective_limits(
        super::overrides::RunRoute::ModelBasedTest,
        super::overrides::RequestLimits {
            accounting: published.accounting,
            observation: None,
        },
        overrides,
    )?;
    let limits = effective.spine;
    let selection = decode(member)?;
    if request.package.is_some()
        || request.call.is_some()
        || request.clause.is_some()
        || request.items.is_some()
    {
        return Err(invalid(
            "model_based_test",
            "model_based_test cannot be combined with another run selection",
        ));
    }

    if request
        .models
        .iter()
        .any(|model| !super::is_model_input_format(model))
    {
        return Err(invalid("models", "expected semantic-ir/2.0.0"));
    }
    let domain_sources = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = super::package_input_from_selected_models(&request.models, &domain_sources)?;
    let libraries = request
        .libraries
        .iter()
        .enumerate()
        .map(|(index, library)| {
            Ok((
                library,
                intake.source(&library.source, &format!("libraries[{index}].source"))?,
            ))
        })
        .collect::<Result<Vec<(&wire::RunLibrary, FormalSource)>>>()?;
    let extension_catalog = super::extension_catalog(&request.extensions, intake)?;
    let dependencies =
        fsl_replay::spine::DependencyInput::new(libraries.iter().map(|(library, read)| {
            fsl_replay::spine::SuppliedLibrary {
                identity: library.identity.clone(),
                source: read.source().identity().clone(),
                path: read.source().path().to_owned(),
                bytes: read.source().text().as_bytes().to_vec(),
            }
        }))
        .and_then(|dependencies| {
            dependencies.with_refinement_internals(libraries.iter().filter_map(|(library, _)| {
                library
                    .refinement_internals
                    .as_ref()
                    .map(|value| (library.identity.clone(), value.clone()))
            }))
        })
        .and_then(|dependencies| dependencies.with_integer_division(library_integer_division))
        .map_err(|refusal| {
            super::spine_run_failure(
                &source,
                libraries.iter().map(|(_, source)| source),
                fsl_replay::spine::CompileRefusal::DependencyInput(refusal),
            )
        })?;
    let failure = |failure| match fsl_replay::spine::refusal_or_fault(failure) {
        Ok(refusal) => super::spine_run_failure(
            &source,
            libraries.iter().map(|(_, source)| source),
            *refusal,
        ),
        Err(fault) => RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault))),
    };
    let cancel = filament_exact::Cancel::new();
    let parse_request = fsl_replay::spine::ParseRequest {
        source: source.source().identity(),
        path: source.source().path(),
        bytes: source.source().text().as_bytes(),
    };
    let parsed = fsl_replay::spine::parse_selecting_extensions(
        &parse_request,
        fsl_replay::spine::ProfileCatalogInput {
            catalog: &extension_catalog,
            limits: fsl_semantics::library::bundle::PackageLimits::default(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)?
    .into_value();
    let admitted = fsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)?
        .into_value();
    let checked = match &integer_division {
        Some(law) => fsl_replay::spine::check_with_integer_division(
            &parsed,
            package_name(source.source().identity()),
            &admitted,
            &dependencies,
            law,
            limits,
            &cancel,
        ),
        None => fsl_replay::spine::check(
            &parsed,
            package_name(source.source().identity()),
            &admitted,
            &dependencies,
            limits,
            &cancel,
        ),
    }
    .map_err(failure)?
    .into_value();
    let package = checked.package();
    let graph = package.graph();
    let state_clause = graph.state_clause(&selection.clause);
    let callable = graph.callable(&selection.clause);
    let is_state_clause = state_clause.is_some();
    let clause_identity = if let Some(clause) = state_clause {
        clause.identity().clone()
    } else if let Some(callable) = callable.as_ref() {
        callable.identity.clone()
    } else {
        return Err(refuse("clause", "unknown-member"));
    };
    if !is_state_clause {
        let callable = callable.expect("callable selected above");
        if callable.result != &filament_exact::ValueType::Boolean {
            return Err(refuse("clause", "invalid-value"));
        }
        if callable.parameters.iter().any(|(_, value_type)| {
            value_type != &filament_exact::ValueType::Integer
                && !matches!(value_type, filament_exact::ValueType::Int(_))
        }) {
            return Err(refuse("clause", "invalid-value"));
        }
    }
    if is_state_clause && selection.max_trace_length.is_none() {
        return Err(refuse("max_trace_length", "missing-member"));
    }
    if !is_state_clause && selection.initial.is_some() {
        return Err(refuse("initial", "unknown-member"));
    }

    let subject = if is_state_clause {
        let initial_file = selection
            .initial
            .as_deref()
            .ok_or_else(|| refuse("initial", "missing-member"))?;
        let initial_bytes = intake.file(initial_file, intake.remaining)?;
        let supplied = fsl_replay::spine::SuppliedObservation::from_document(
            "model_based_test.initial",
            initial_bytes.clone(),
        )
        .map_err(|error| invalid(&error.member, error.reason))?;
        let initial = supplied.reference.clone();
        let snapshot: Snapshot = serde_json::from_slice(&initial_bytes)
            .map_err(|error| invalid("initial", format!("invalid snapshot: {error}")))?;
        let snapshots = fsl_semantics::model::observation::DocumentProvision::from([(
            initial.clone(),
            initial_bytes,
        )]);
        let reference =
            serde_json::json!({"authority": initial.authority, "identity": initial.identity});
        let universe_subject: fsl_analyze::analyze::document::SubjectSelection =
            serde_json::from_value(serde_json::json!({
                "initial_states": [reference],
                "universes": [reference],
                "proof_bounds": [],
            }))?;
        universe_subject
            .check_universes(&snapshots)
            .map_err(RunCause::AnalyzeUniverse)?;
        let universes: Vec<fsl_eval::simulation::model::PopulationUniverse> = snapshot
            .populations
            .into_iter()
            .map(
                |population| fsl_eval::simulation::model::PopulationUniverse {
                    population: filament_checked::key::DeclarationKey {
                        package: snapshot.model.identity.clone(),
                        node: population.population,
                    },
                    keys: population
                        .objects
                        .into_iter()
                        .map(|object| object.key)
                        .collect(),
                },
            )
            .collect();
        Some((snapshots, vec![initial], universes))
    } else {
        None
    };
    let sampler =
        fsl_foundation::selection::DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
            .map_err(|error| invalid("model_based_test", format!("{error:?}")))?;
    let request = fsl_eval::simulation::model::ModelBasedTestRequest {
        clause: Some(clause_identity),
        profile: sampler,
        seed: selection.seed,
        max_cases: selection.max_cases,
        max_trace_length: selection.max_trace_length,
        max_steps: super::overrides::analyze_limits(published, overrides)
            .simulation
            .max_steps,
        max_value_bits: selection.max_value_bits.map(u64::from),
        subject: subject.as_ref().map(|(snapshots, initial, universes)| {
            fsl_eval::simulation::model::ModelBasedTestSubject {
                packages: &packages,
                snapshots,
                initial: initial.clone(),
                universes: universes.clone(),
            }
        }),
        expansion: super::overrides::analyze_limits(published, overrides).expansion,
        accounting: effective.accounting.0,
        normalization: limits.model,
        observation: effective.observation,
        emit: limits.package.emit(),
        exploration: fsl_eval::simulation::Limits::default(),
        position_limit: published.position_limit,
    };
    let record = fsl_eval::simulation::model::run_model_based_test(
        package,
        &request,
        &fsl_eval::simulation::model::NativeAdmission,
        &cancel,
    )
    .map_err(model_test_error)?;
    let exit_code = record.exit_code();
    let record = result_record(&record, &package_name(source.source().identity()));
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(PreparedRun::ModelBasedTest(PreparedModelBasedTestRun {
        record,
        exit_code,
        sources,
    }))
}
