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

fn evidence_name(evidence: fsl_eval::simulation::model::ModelBasedTestEvidence) -> &'static str {
    use fsl_eval::simulation::model::ModelBasedTestEvidence;
    match evidence {
        ModelBasedTestEvidence::Tested => "tested",
        ModelBasedTestEvidence::Refuted => "refuted",
        ModelBasedTestEvidence::Inconclusive => "inconclusive",
        ModelBasedTestEvidence::Incomplete => "incomplete",
        ModelBasedTestEvidence::Failed => "failed",
        ModelBasedTestEvidence::Unsupported => "unsupported",
    }
}

fn boundary_name(kind: fsl_eval::simulation::model::ModelBoundaryKind) -> &'static str {
    use fsl_eval::simulation::model::ModelBoundaryKind as Kind;
    match kind {
        Kind::TextEmpty => "text-empty",
        Kind::TextLowerEndpoint => "text-lower-endpoint",
        Kind::TextSingleton => "text-singleton",
        Kind::TextUpperEndpoint => "text-upper-endpoint",
        Kind::TextNormalization => "text-normalization",
        Kind::SizeLeast => "size-least",
        Kind::SizeUnit => "size-unit",
        Kind::SizeExtremeLow => "size-extreme-low",
        Kind::SizeExtremeHigh => "size-extreme-high",
        Kind::LowerRefusal => "lower-refusal",
        Kind::LowerEndpoint => "lower-endpoint",
        Kind::UpperEndpoint => "upper-endpoint",
        Kind::UpperRefusal => "upper-refusal",
    }
}

fn partition_json(partition: &fsl_eval::simulation::model::ModelBoundaryPartition) -> Value {
    use fsl_eval::simulation::model::ModelPartitionStatus;
    let status = partition.status.as_ref().map(|status| match status {
        ModelPartitionStatus::Covered { first_case } => {
            serde_json::json!({"status": "covered", "first_case": first_case})
        }
        ModelPartitionStatus::Refused { first_case } => {
            serde_json::json!({"status": "refused", "first_case": first_case})
        }
        ModelPartitionStatus::Uncovered { bounds, .. } => {
            serde_json::json!({"status": "uncovered", "bounds": bounds})
        }
    });
    serde_json::json!({
        "node": partition.node.to_string(),
        "path": partition.path,
        "kind": boundary_name(partition.kind),
        "value": {"type": "integer", "value": partition.value.to_string()},
        "text_value": partition.text_value,
        "status": status,
    })
}

fn counterexample_json(result: &fsl_eval::simulation::model::ModelBasedTestResult) -> Value {
    use fsl_eval::simulation::model::ModelBasedCounterexample;
    let Some(counterexample) = &result.counterexample else {
        return Value::Null;
    };
    match counterexample {
        ModelBasedCounterexample::Trace(trace) => {
            let disposition = trace
                .trace
                .findings
                .iter()
                .flat_map(|finding| finding.findings.iter())
                .find_map(|finding| match finding {
                    fsl_eval::simulation::model::ModelFinding::InvariantViolated {
                        clause, ..
                    } if clause == &result.clause => Some("invariant-violated"),
                    _ => None,
                })
                .unwrap_or("model-check-failed");
            serde_json::json!({
                "minimality": "1-minimal",
                "replay": "reproduced",
                "disposition": disposition,
                "initial": trace.trace.initial.hex(),
                "invocations": trace.trace.steps.iter().map(|step| serde_json::json!({
                    "operation": step.transition.operation(),
                    "receiver": step.transition.receiver(),
                    "arguments": step.transition.arguments(),
                })).collect::<Vec<_>>(),
            })
        }
        ModelBasedCounterexample::ValueAssignment(values) => serde_json::json!({
            "minimality": "1-minimal",
            "replay": "reproduced",
            "disposition": "predicate-false",
            "assignment": values.iter().map(|value| match value {
                filament_exact::Value::Integer(value) => serde_json::json!({
                    "type": "integer",
                    "value": value.to_string(),
                }),
                other => serde_json::json!({"debug": format!("{other:?}")}),
            }).collect::<Vec<_>>(),
        }),
    }
}

pub(super) fn result_json(
    result: &fsl_eval::simulation::model::ModelBasedTestResult,
    domain: &str,
) -> Value {
    use fsl_eval::simulation::model::{
        ModelBasedTestCause, ModelBasedTestDiscardReason, ModelOperationCoverageReasonKind,
        ModelOperationCoverageSummaryStatus,
    };
    let evidence = evidence_name(result.evidence);
    let discarded_by = result
        .discarded_by
        .iter()
        .map(|(reason, count)| {
            let name = match reason {
                ModelBasedTestDiscardReason::DomainRefusal => "domain-refusal",
                ModelBasedTestDiscardReason::PreconditionFalse => "precondition-false",
                ModelBasedTestDiscardReason::InvariantFalse => "invariant-false",
                ModelBasedTestDiscardReason::NoCheckPoint => "no-check-point",
            };
            (
                name.to_owned(),
                serde_json::json!({
                    "count": count.count,
                    "first_case": count.first_case,
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    let cause = result.cause.as_ref().map(|cause| match cause {
        ModelBasedTestCause::NoEvaluatedCase => serde_json::json!("no-evaluated-case"),
        ModelBasedTestCause::LimitReached {
            limit,
            value,
            setting,
        } => serde_json::json!({
            "kind": "limit-reached",
            "limit": limit.limit_name(),
            "value": value,
            "setting": setting,
        }),
        ModelBasedTestCause::ToolFailure { defect } => serde_json::json!({
            "kind": "tool-failure",
            "defect": match defect {
                fsl_eval::simulation::model::ModelBasedTestDefect::OracleDisagreement => {
                    "oracle-disagreement"
                }
                fsl_eval::simulation::model::ModelBasedTestDefect::InvalidShrink => {
                    "invalid-shrink"
                }
            },
        }),
        ModelBasedTestCause::NoContractLevelReading => {
            serde_json::json!("no-contract-level-reading")
        }
    });
    let operations = result
        .operations
        .as_ref()
        .map(|operations| serde_json::to_value(operations).expect("coverage entries serialize"));
    let coverage = result.coverage.as_ref().map(|coverage| {
        let status = match coverage.status {
            ModelOperationCoverageSummaryStatus::Complete => "complete",
            ModelOperationCoverageSummaryStatus::Partial => "partial",
        };
        let uncovered = coverage
            .uncovered
            .iter()
            .map(|(reason, count)| {
                let name = match reason {
                    ModelOperationCoverageReasonKind::Unreachable => "unreachable",
                    ModelOperationCoverageReasonKind::UnreachableWithinBound => {
                        "unreachable-within-bound"
                    }
                    ModelOperationCoverageReasonKind::NotGenerated => "not-generated",
                    ModelOperationCoverageReasonKind::Undetermined => "undetermined",
                };
                (name.to_owned(), serde_json::json!(count))
            })
            .collect::<serde_json::Map<String, Value>>();
        serde_json::json!({"status": status, "uncovered": uncovered})
    });
    serde_json::json!({
        "clause": result.clause,
        "domain": domain,
        "status": evidence,
        "evidence": evidence,
        "verdict": if evidence == "refuted" { serde_json::json!("refuted") } else { Value::Null },
        "cause": cause,
        "cases": {
            "generated": result.counts.generated,
            "passed": result.counts.passing,
            "failed": result.counts.failing,
            "discarded": result.counts.discarded,
            "discarded_by": discarded_by,
        },
        "checks": {
            "evaluated": result.checks.evaluated,
            "skipped": result.checks.skipped.len(),
        },
        "limits": {
            "max_cases": result.max_cases,
            "max_trace_length": result.max_trace_length,
            "max_value_bits": result.max_value_bits,
        },
        "provenance": {
            "seed": result.seed,
            "max_value_bits": result.provenance.max_value_bits,
            "size_bounded": result.provenance.size_bounded.iter().map(|root| serde_json::json!({
                "node": root.node.to_string(),
                "path": root.path,
                "values": root.values,
            })).collect::<Vec<_>>(),
            "value_bound_cuts": {"count": result.provenance.value_bound_cuts.count},
        },
        "partitions": result.partitions.iter().map(partition_json).collect::<Vec<_>>(),
        "partition_coverage": {
            "status": match result.partition_coverage.status {
                ModelOperationCoverageSummaryStatus::Complete => "complete",
                ModelOperationCoverageSummaryStatus::Partial => "partial",
            },
            "uncovered": result.partition_coverage.uncovered,
        },
        "operations": operations,
        "coverage": coverage,
        "counterexample": counterexample_json(result),
    })
}

/// The run-level inputs shared with every other `run` route.
pub(super) struct Context {
    pub(super) source: FormalSource,
    pub(super) integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    pub(super) library_integer_division: Vec<(String, fsl_replay::AdmittedIntegerDivision)>,
    pub(super) limits: fsl_replay::spine::SpineLimits,
    pub(super) observation_override: Option<fsl_semantics::model::observation::ObservationLimits>,
}

#[implements("FR-752-AC-16")]
pub(super) fn prepare(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    member: &Value,
    context: Context,
) -> Result<PreparedRun> {
    let Context {
        source,
        integer_division,
        library_integer_division,
        limits,
        observation_override,
    } = context;
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
    let observation = observation_override.unwrap_or_default();
    // The member is closed, so no request carries these: they run at the
    // published defaults.
    let published = fsl_analyze::analyze::AnalyzeLimits::published(
        fsl_analyze::analyze::DeadlineSupply::Unbounded,
    );
    let request = fsl_eval::simulation::model::ModelBasedTestRequest {
        clause: Some(clause_identity),
        profile: sampler,
        seed: selection.seed,
        max_cases: selection.max_cases,
        max_trace_length: selection.max_trace_length,
        max_value_bits: selection.max_value_bits.map(u64::from),
        subject: subject.as_ref().map(|(snapshots, initial, universes)| {
            fsl_eval::simulation::model::ModelBasedTestSubject {
                packages: &packages,
                snapshots,
                initial: initial.clone(),
                universes: universes.clone(),
            }
        }),
        expansion: published.expansion,
        accounting: published.accounting,
        normalization: limits.model,
        observation,
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
    let record = result_json(&record, &package_name(source.source().identity()));
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(PreparedRun::ModelBasedTest(PreparedModelBasedTestRun {
        record,
        exit_code,
        sources,
    }))
}
