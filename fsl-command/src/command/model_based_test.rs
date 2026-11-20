// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-190: decode a direct `model_based_test` request.

use serde_json::Value;

use super::{
    FormalSource, Intake, PreparedModelBasedTestRun, PreparedRun, Result, RunCause, package_name,
    wire,
};

/// Limits and selection admitted by the direct model-based test member.
pub(super) struct Request {
    pub(super) clause: String,
    pub(super) seed: u64,
    pub(super) max_cases: u64,
    pub(super) max_trace_length: Option<usize>,
    pub(super) max_value_bits: Option<u8>,
    pub(super) initial: Option<String>,
}

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: format!("model_based_test.{member}"),
        reason: reason.into(),
    }
}

/// Read the closed member and validate its scalar bounds before source intake.
pub(super) fn decode(value: &Value) -> Result<Request> {
    let Value::Object(members) = value else {
        return Err(RunCause::ClauseRequest {
            member: "model_based_test".to_owned(),
            reason: "expected an object".to_owned(),
        });
    };
    if let Some(unknown) = members.keys().find(|key| {
        !matches!(
            key.as_str(),
            "clause" | "seed" | "max_cases" | "max_trace_length" | "max_value_bits" | "initial"
        )
    }) {
        return Err(invalid(unknown, "unknown member"));
    }
    let clause = members
        .get("clause")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| invalid("clause", "expected a non-empty string"))?
        .to_owned();
    let seed = members
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("seed", "expected a 64-bit unsigned integer"))?;
    let max_cases = members
        .get("max_cases")
        .and_then(Value::as_u64)
        .filter(|count| *count > 0)
        .ok_or_else(|| invalid("max_cases", "must be a positive integer"))?;
    let max_trace_length = members
        .get("max_trace_length")
        .map(|value| {
            let length = value
                .as_u64()
                .filter(|length| *length > 0)
                .ok_or_else(|| invalid("max_trace_length", "must be a positive integer"))?;
            usize::try_from(length)
                .map(Some)
                .map_err(|_| invalid("max_trace_length", "does not fit this platform"))
        })
        .transpose()?
        .flatten();
    let max_value_bits = members
        .get("max_value_bits")
        .map(|value| {
            let bits = value
                .as_u64()
                .filter(|bits| (1..=64).contains(bits))
                .ok_or_else(|| invalid("max_value_bits", "must be between 1 and 64"))?;
            u8::try_from(bits)
                .map(Some)
                .map_err(|_| invalid("max_value_bits", "must be between 1 and 64"))
        })
        .transpose()?
        .flatten();
    let initial = members
        .get("initial")
        .map(|value| {
            value
                .as_str()
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| invalid("initial", "expected a non-empty path"))
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
    complete: bool,
    #[serde(default)]
    objects: Vec<SnapshotObject>,
}

#[derive(serde::Deserialize)]
struct SnapshotObject {
    key: String,
}

fn route_error(error: impl std::fmt::Debug) -> RunCause {
    invalid("clause", format!("{error:?}"))
}

fn model_test_error(error: fsl_eval::simulation::model::ModelBasedTestRefusal) -> RunCause {
    invalid(error.member, error.cause)
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

fn result_json(result: &fsl_eval::simulation::model::ModelBasedTestResult, domain: &str) -> Value {
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
            "limit": limit.as_str(),
            "value": value,
            "setting": setting,
        }),
        ModelBasedTestCause::ToolFailure { defect } => serde_json::json!({
            "kind": "tool-failure",
            "defect": format!("{defect:?}"),
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

pub(super) fn prepare(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    member: &Value,
    source: FormalSource,
    _observation_override: Option<fsl_semantics::model::observation::ObservationLimits>,
) -> Result<PreparedRun> {
    let selection = decode(member)?;
    if request.package.is_some()
        || request.call.is_some()
        || request.clause.is_some()
        || request.items.is_some()
        || request.tl_lowering.is_some()
    {
        return Err(invalid(
            "clause",
            "model_based_test cannot be combined with another run selection",
        ));
    }

    let domain_sources = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| {
            if !super::is_domain_package(model) {
                return Err(invalid("models", "expected semantic-ir/2.0.0"));
            }
            intake.source(&model.source, &format!("models[{index}].source"))
        })
        .collect::<Result<Vec<_>>>()?;
    let packages = fsl_semantics::model::intake::package_input(
        domain_sources
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| invalid("models", refusal.detail))?;
    let dependencies = fsl_replay::spine::DependencyInput::default();
    let cancel = filament_exact::Cancel::new();
    let parse_request = fsl_replay::spine::ParseRequest {
        source: source.source().identity(),
        path: source.source().path(),
        bytes: source.source().text().as_bytes(),
    };
    let parsed = fsl_replay::spine::parse(&parse_request, fsl_cst::Limits::default(), &cancel)
        .map_err(route_error)?
        .into_value();
    let admitted = fsl_replay::spine::select(
        &parsed,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &cancel,
    )
    .map_err(route_error)?
    .into_value();
    let checked = fsl_replay::spine::check(
        &parsed,
        package_name(source.source().identity()),
        &admitted,
        &dependencies,
        fsl_replay::spine::SpineLimits::default(),
        &cancel,
    )
    .map_err(route_error)?
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
        return Err(invalid(&selection.clause, "unknown clause"));
    };
    if !is_state_clause {
        let callable = callable.expect("callable selected above");
        if callable.result != &filament_exact::ValueType::Boolean {
            return Err(invalid(
                &selection.clause,
                "value clause must return Boolean",
            ));
        }
        if selection.max_value_bits.is_some()
            && callable
                .parameters
                .iter()
                .any(|(_, value_type)| matches!(value_type, filament_exact::ValueType::Int(_)))
        {
            return Err(invalid(
                "max_value_bits",
                "not valid for a bounded integer parameter",
            ));
        }
        if callable
            .parameters
            .iter()
            .any(|(_, value_type)| value_type != &filament_exact::ValueType::Integer)
        {
            return Err(invalid(
                &selection.clause,
                "value-clause parameters must have Integer type",
            ));
        }
    }
    if is_state_clause && selection.max_trace_length.is_none() {
        return Err(invalid(
            "max_trace_length",
            "a state clause requires a positive length",
        ));
    }
    if !is_state_clause && selection.max_trace_length.is_some() {
        return Err(invalid("max_trace_length", "not valid for a value clause"));
    }
    if !is_state_clause && selection.initial.is_some() {
        return Err(invalid("initial", "not valid for a value clause"));
    }

    let subject = if is_state_clause {
        let initial_file = selection
            .initial
            .as_deref()
            .ok_or_else(|| invalid("initial", "a state clause requires an initial snapshot"))?;
        let initial_bytes = intake.file(initial_file, intake.remaining)?;
        let supplied = fsl_replay::spine::SuppliedObservation::from_document(
            "model_based_test.initial",
            initial_bytes.clone(),
        )
        .map_err(|error| invalid(&error.member, error.reason))?;
        let initial = supplied.reference.clone();
        let snapshot: Snapshot = serde_json::from_slice(&initial_bytes)
            .map_err(|error| invalid("initial", format!("invalid snapshot: {error}")))?;
        let universes: Vec<fsl_eval::simulation::model::PopulationUniverse> = snapshot
            .populations
            .into_iter()
            .filter(|population| population.complete)
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
        let snapshots = fsl_semantics::model::observation::DocumentProvision::from([(
            initial.clone(),
            initial_bytes,
        )]);
        Some((snapshots, vec![initial], universes))
    } else {
        None
    };
    let sampler =
        fsl_foundation::selection::DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
            .map_err(route_error)?;
    let observation = _observation_override.unwrap_or_default();
    let request = fsl_eval::simulation::model::ModelBasedTestRequest {
        clause: Some(clause_identity),
        profile: sampler,
        seed: selection.seed,
        max_cases: selection.max_cases,
        max_trace_length: selection
            .max_trace_length
            .map(|length| u64::try_from(length).unwrap_or(u64::MAX)),
        max_value_bits: selection.max_value_bits.map(u64::from),
        subject: subject.as_ref().map(|(snapshots, initial, universes)| {
            fsl_eval::simulation::model::ModelBasedTestSubject {
                packages: &packages,
                snapshots,
                initial: initial.clone(),
                universes: universes.clone(),
            }
        }),
        expansion: fsl_eval::simulation::model::ExpansionLimits {
            max_candidates: 1_000_000,
        },
        accounting: fsl_replay::spine::default_accounting(10_000),
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        observation,
        emit: fsl_replay::spine::SpineLimits::default().package.emit(),
        exploration: fsl_eval::simulation::Limits::default(),
        position_limit: 10_000,
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
