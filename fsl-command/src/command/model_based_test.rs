// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-190: decode a direct `model_based_test` request.

use serde_json::Value;

use super::{
    package_name, wire, FormalSource, Intake, PreparedRun, PreparedModelBasedTestRun, Result,
    RunCause,
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

fn model_test_error(error: fsl_eval::simulation::model::ModelBasedTestError) -> RunCause {
    use fsl_eval::simulation::model::ModelBasedTestError;

    match error {
        ModelBasedTestError::EmptyCaseBound => {
            invalid("max_cases", "must be a positive integer")
        }
        ModelBasedTestError::EmptyTraceBound => {
            invalid("max_trace_length", "must be a positive integer")
        }
        ModelBasedTestError::InvalidValueBits => {
            invalid("max_value_bits", "must be between 1 and 64")
        }
        ModelBasedTestError::MissingValueBits => {
            invalid("max_value_bits", "required for an unbounded integer parameter")
        }
        ModelBasedTestError::EmptyValueRoots => {
            invalid("clause", "value-clause request has no parameter roots")
        }
        other => route_error(other),
    }
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
    let parsed = fsl_replay::spine::parse(
        &parse_request,
        fsl_cst::Limits::default(),
        &cancel,
    )
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
    let input = fsl_eval::simulation::model::ModelCheckInput::source(checked.package());
    let is_state_clause = input
        .state_clauses
        .iter()
        .any(|clause| clause.name == selection.clause);
    if !is_state_clause {
        let callable = checked
            .package()
            .graph()
            .callable(&selection.clause)
            .ok_or_else(|| invalid(&selection.clause, "unknown clause"))?;
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
        let parameters = callable
            .parameters
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let package = checked.package();
        let function =
            fsl_eval::value::QualifiedName::unqualified(&selection.clause).map_err(route_error)?;
        let record = fsl_eval::simulation::model::run_model_based_test(
            fsl_eval::simulation::model::ModelBasedTestRequest::Values {
                clause: &selection.clause,
                domain: &package_name(source.source().identity()),
                parameters: &parameters,
                max_cases: selection.max_cases,
                seed: selection.seed,
                max_value_bits: selection.max_value_bits,
            },
            |case| {
                let fsl_eval::simulation::model::ModelBasedTestCase::Values(arguments) = case
                else {
                    return fsl_eval::simulation::model::ModelBasedTestOutcome::Discarded {
                        reason: "unexpected-model-case-kind".to_owned(),
                    };
                };
                use fsl_eval::value::CheckedPackageEvaluation;

                let values = arguments.to_vec();
                let mut meter = filament_exact::Meter::new(
                    fsl_replay::spine::default_accounting(10_000),
                );
                match package.call(
                    &function,
                    values,
                    &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
                    &mut meter,
                ) {
                    Ok(evaluation) => match evaluation.outcome {
                        filament_eval::FamilyOutcome::Evaluated(
                            filament_exact::Outcome::Completed(filament_exact::Value::Boolean(
                                true,
                            )),
                        ) => fsl_eval::simulation::model::ModelBasedTestOutcome::Passed,
                        filament_eval::FamilyOutcome::Evaluated(
                            filament_exact::Outcome::Completed(filament_exact::Value::Boolean(
                                false,
                            )),
                        ) => fsl_eval::simulation::model::ModelBasedTestOutcome::Failed {
                            disposition: "predicate-false".to_owned(),
                        },
                        _ => fsl_eval::simulation::model::ModelBasedTestOutcome::Discarded {
                            reason: "native-evaluation-incomplete".to_owned(),
                        },
                    },
                    Err(_) => fsl_eval::simulation::model::ModelBasedTestOutcome::Discarded {
                        reason: "native-evaluation-refused".to_owned(),
                    },
                }
            },
        )
        .map_err(model_test_error)?;
        let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        return Ok(PreparedRun::ModelBasedTest(PreparedModelBasedTestRun {
            record,
            sources,
        }));
    }
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
    let universes = snapshot
        .populations
        .into_iter()
        .filter(|population| population.complete)
        .map(|population| fsl_eval::simulation::model::PopulationUniverse {
            population: filament_checked::key::DeclarationKey {
                package: snapshot.model.identity.clone(),
                node: population.population,
            },
            keys: population.objects.into_iter().map(|object| object.key).collect(),
        })
        .collect();
    let snapshots = fsl_semantics::model::observation::DocumentProvision::from([(
        initial.clone(),
        initial_bytes,
    )]);
    let model_system = fsl_eval::simulation::model::ModelSystem::new(
        fsl_eval::simulation::model::ModelSimulationRequest {
            input,
            packages: &packages,
            normalization: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
            snapshots: &snapshots,
            initial: vec![initial],
            universes,
            observation: fsl_semantics::model::observation::ObservationLimits::default(),
            expansion: fsl_eval::simulation::model::ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: fsl_replay::spine::default_accounting(10_000),
            emit: fsl_replay::spine::SpineLimits::default().package.emit(),
        },
    )
    .map_err(route_error)?;
    let sampler = fsl_foundation::selection::DefinitionRef::new(
        "agent-ix",
        "filament.simulation.sampler/v1",
    )
    .map_err(route_error)?;
    let domain = model_system
        .populations()
        .first()
        .map(|population| population.key.package.clone())
        .unwrap_or_default();
    let max_trace_length = selection.max_trace_length.unwrap_or(1);
    let record = fsl_eval::simulation::model::run_model_based_test(
        fsl_eval::simulation::model::ModelBasedTestRequest::State {
            system: &model_system,
            sampler: &sampler,
            clause: &selection.clause,
            domain: &domain,
            max_cases: selection.max_cases,
            seed: selection.seed,
            max_trace_length,
            max_value_bits: selection.max_value_bits,
            position_limit: 10_000,
        },
        |case| {
            let fsl_eval::simulation::model::ModelBasedTestCase::State(trace) = case else {
                return fsl_eval::simulation::model::ModelBasedTestOutcome::Discarded {
                    reason: "unexpected-model-case-kind".to_owned(),
                };
            };
            let findings = trace
                .trace
                .findings
                .iter()
                .flat_map(|finding| finding.findings.iter())
                .collect::<Vec<_>>();
            if findings.iter().copied().any(|finding| {
                matches!(finding,
                    fsl_eval::simulation::model::ModelFinding::InvariantViolated { clause, .. }
                        if clause == &selection.clause)
            }) {
                fsl_eval::simulation::model::ModelBasedTestOutcome::Failed {
                    disposition: "invariant-violated".to_owned(),
                }
            } else if findings.iter().copied().any(|finding| {
                matches!(finding,
                    fsl_eval::simulation::model::ModelFinding::InvariantUndetermined { clause, .. }
                        if clause == &selection.clause)
            }) {
                fsl_eval::simulation::model::ModelBasedTestOutcome::Discarded {
                    reason: "invariant-undetermined".to_owned(),
                }
            } else {
                fsl_eval::simulation::model::ModelBasedTestOutcome::Passed
            }
        },
    )
    .map_err(model_test_error)?;
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(PreparedRun::ModelBasedTest(PreparedModelBasedTestRun {
        record,
        sources,
    }))
}
