// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898: evaluate a checked finite-hyperproperty over its request population.

use filament_exact::{Meter, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::hyper::{
    evaluate_finite_hyper, FiniteHyperOutcome, FinitePopulationError, FiniteQuantifier,
    FiniteTrace, FiniteTraceClosure, FiniteTracePopulation, FiniteTupleOutcome,
};
use fsl_eval::value::CheckedPackageEvaluation;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use serde::Deserialize;
use serde_json::Value as Json;

use super::{AnalyzeItem, EngineReport, FiniteHyperReport, Refusal, RunInput, SubjectBinding};
use crate::analyze::{AnalyzeLimits, ClaimItem, FiniteHyperItem};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PopulationDocument {
    format: String,
    record_type: String,
    closure: String,
    traces: Vec<PopulationTrace>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PopulationTrace {
    identity: String,
    states: Vec<Json>,
}

#[derive(Clone, Debug)]
struct FiniteRecord {
    values: Vec<Value>,
    source: Json,
}

impl PartialEq for FiniteRecord {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

pub(super) fn item<'a>(
    analysis: &'a RunInput<'a>,
    binding: &SubjectBinding<'a>,
    item: &ClaimItem,
    form: &FiniteHyperItem,
    limits: &AnalyzeLimits,
) -> AnalyzeItem<'a> {
    let package = analysis.package;
    let request = item.clone();
    let request_index = item.request_index;
    let binding = binding.clone();
    let form = form.clone();
    let limits = *limits;
    AnalyzeItem::new(
        item.request_index,
        item.kind,
        crate::claim_family::claim_family(
            package,
            fsl_route::request::ItemKind::Formula,
            &item.claim,
        ),
        fsl_route::Mode::Bounded,
        crate::model_check::manifest::BACKEND_ID,
        move |_| {
            let Some(reference) = request.options.traces.as_ref() else {
                return invalid_population(None);
            };
            let Some(bytes) = binding.snapshots.get(reference) else {
                return invalid_population(None);
            };
            let Ok(document) = serde_json::from_slice::<PopulationDocument>(bytes) else {
                return invalid_population(None);
            };
            if document.format != "filament.trace-population/1"
                || !matches!(document.closure.as_str(), "closed" | "open")
                || document.record_type != record_type_name(package, form.clause.record())
            {
                return invalid_population(None);
            }
            let expression = package.graph().finite_hyper_expression(form.clause.name());
            let Some(expression) = expression else {
                return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                    "runtime_invariant",
                    "established-invariant-broken",
                )));
            };
            let types = package.graph().scope().types();
            let record_fields: Vec<_> = expression
                .parameters()
                .iter()
                .map(|(name, field_type)| (name.clone(), field_type.clone()))
                .collect();
            let Some(fields_per_trace) = record_fields
                .len()
                .checked_div(form.clause.quantifiers().len())
            else {
                return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                    "runtime_invariant",
                    "established-invariant-broken",
                )));
            };
            let mut traces = Vec::with_capacity(document.traces.len());
            for trace in document.traces {
                let named = trace.identity.clone();
                let mut records = Vec::with_capacity(trace.states.len());
                for state in trace.states {
                    let Some(fields) = state.as_object() else {
                        return invalid_population(Some(&named));
                    };
                    let mut values = Vec::with_capacity(record_fields.len());
                    for (qualified_field, field_type) in record_fields.iter().take(fields_per_trace)
                    {
                        let Some((_, field)) = qualified_field.split_once('.') else {
                            return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                                "runtime_invariant",
                                "established-invariant-broken",
                            )));
                        };
                        let Some(json) = fields.get(field) else {
                            return invalid_population(Some(&named));
                        };
                        let Some(value) = crate::protocol_analysis::recording::recording_value(
                            json, field_type, types,
                        ) else {
                            return invalid_population(Some(&named));
                        };
                        values.push(value);
                    }
                    records.push(FiniteRecord {
                        values,
                        source: state,
                    });
                }
                traces.push(FiniteTrace {
                    identity: trace.identity,
                    records,
                });
            }
            let closure = if document.closure == "closed" {
                FiniteTraceClosure::Closed
            } else {
                FiniteTraceClosure::Open
            };
            let population = match FiniteTracePopulation::new(
                format!("{}:{}", reference.authority, reference.identity),
                document.record_type,
                closure,
                traces,
            ) {
                Ok(population) => population,
                Err(error) => {
                    return invalid_population(match &error {
                        FinitePopulationError::EmptyTraceIdentity => None,
                        FinitePopulationError::EmptyTrace { identity }
                        | FinitePopulationError::ConflictingTraceIdentity { identity } => {
                            Some(identity)
                        }
                    });
                }
            };
            let quantifiers: Vec<_> = form
                .clause
                .quantifiers()
                .iter()
                .map(|quantifier| {
                    (
                        match quantifier.kind {
                            fsl_semantics::check::QuantifierKind::Forall => {
                                FiniteQuantifier::Forall
                            }
                            fsl_semantics::check::QuantifierKind::Exists => {
                                FiniteQuantifier::Exists
                            }
                        },
                        quantifier.variable.clone(),
                    )
                })
                .collect();
            let mut meter = Meter::new(limits.accounting);
            let mut meter_stop = None;
            let evaluated =
                evaluate_finite_hyper(&population, form.clause.bound(), &quantifiers, |selected| {
                    let max_positions = selected
                        .iter()
                        .map(|trace| trace.records.len())
                        .max()
                        .unwrap_or(0);
                    for position in 0..max_positions {
                        let mut arguments = Vec::with_capacity(record_fields.len());
                        let mut aligned = true;
                        for trace in selected {
                            let Some(record) = trace.records.get(position) else {
                                aligned = false;
                                break;
                            };
                            arguments.extend(record.values.iter().cloned());
                        }
                        if !aligned {
                            return FiniteTupleOutcome::Incomplete;
                        }
                        let Some(evaluation) = package
                            .evaluate(
                                &expression,
                                arguments,
                                &ObjectEnvironment::default(),
                                &mut meter,
                            )
                            .ok()
                        else {
                            return FiniteTupleOutcome::Incomplete;
                        };
                        match evaluation.outcome {
                            filament_eval::FamilyOutcome::Evaluated(
                                filament_exact::Outcome::Completed(Value::Boolean(true)),
                            ) => {}
                            filament_eval::FamilyOutcome::Evaluated(
                                filament_exact::Outcome::Completed(Value::Boolean(false)),
                            ) => {
                                return FiniteTupleOutcome::False {
                                    position: position as u64,
                                };
                            }
                            filament_eval::FamilyOutcome::Evaluated(
                                filament_exact::Outcome::Incomplete(incomplete),
                            ) => {
                                meter_stop = Some(fsl_replay::IncompleteCause::LimitReached {
                                    limit: fsl_replay::ModelCheckLimit::EvaluationMeter {
                                        counter: incomplete.limit_kind,
                                    },
                                    value: incomplete.limit,
                                    required: None,
                                });
                                return FiniteTupleOutcome::Incomplete;
                            }
                            _ => return FiniteTupleOutcome::Incomplete,
                        }
                    }
                    FiniteTupleOutcome::True
                });
            let backend = fsl_foundation::BackendId::new(crate::model_check::manifest::BACKEND_ID);
            match evaluated {
                FiniteHyperOutcome::Proved => {
                    EngineReport::FiniteHyper(Box::new(FiniteHyperReport {
                        terminal: fsl_replay::TerminalRecord::proved(request_index, backend, 1),
                        counterexample: None,
                    }))
                }
                FiniteHyperOutcome::Inconclusive => {
                    EngineReport::FiniteHyper(Box::new(FiniteHyperReport {
                        terminal: match meter_stop {
                            Some(cause) => fsl_replay::TerminalRecord::incomplete(
                                request_index,
                                backend,
                                cause,
                            ),
                            None => fsl_replay::TerminalRecord::inconclusive(
                                request_index,
                                backend,
                                fsl_replay::InconclusiveCause::TemporalIndeterminate,
                            ),
                        },
                        counterexample: None,
                    }))
                }
                FiniteHyperOutcome::RequiresBound { bound, .. } => {
                    let found = population.traces.len();
                    EngineReport::Refused(Refusal::RequiresFiniteHyperBound { bound, found })
                }
                FiniteHyperOutcome::Refuted { false_tuples } => {
                    let replay_body = |selected: &[&FiniteTrace<FiniteRecord>]| {
                        let max_positions = selected
                            .iter()
                            .map(|trace| trace.records.len())
                            .max()
                            .unwrap_or(0);
                        for position in 0..max_positions {
                            let mut arguments = Vec::new();
                            for trace in selected {
                                let Some(record) = trace.records.get(position) else {
                                    return FiniteTupleOutcome::Incomplete;
                                };
                                arguments.extend(record.values.iter().cloned());
                            }
                            let Ok(evaluation) = package.evaluate(
                                &expression,
                                arguments,
                                &ObjectEnvironment::default(),
                                &mut Meter::new(limits.accounting),
                            ) else {
                                return FiniteTupleOutcome::Incomplete;
                            };
                            match evaluation.outcome {
                                filament_eval::FamilyOutcome::Evaluated(
                                    filament_exact::Outcome::Completed(Value::Boolean(true)),
                                ) => {}
                                filament_eval::FamilyOutcome::Evaluated(
                                    filament_exact::Outcome::Completed(Value::Boolean(false)),
                                ) => {
                                    return FiniteTupleOutcome::False {
                                        position: position as u64,
                                    };
                                }
                                _ => return FiniteTupleOutcome::Incomplete,
                            }
                        }
                        FiniteTupleOutcome::True
                    };
                    let Some(reproduced) = fsl_replay::reproduce_finite_hyper_refutation(
                        &population,
                        form.clause.bound(),
                        &quantifiers,
                        &false_tuples,
                        replay_body,
                    ) else {
                        return EngineReport::Refused(Refusal::Failed(CatalogCode::new(
                            "runtime_invariant",
                            "established-invariant-broken",
                        )));
                    };
                    let counterexample = serde_json::json!({
                        "false_tuples": false_tuples.iter().map(|tuple| serde_json::json!({
                            "traces": tuple.traces,
                            "position": tuple.position,
                        })).collect::<Vec<_>>(),
                    });
                    EngineReport::FiniteHyper(Box::new(FiniteHyperReport {
                        terminal: fsl_replay::TerminalRecord::refuted_by_finite_hyper(
                            request_index,
                            backend,
                            &reproduced,
                        ),
                        counterexample: Some(counterexample),
                    }))
                }
            }
        },
    )
    .at_claim(package, fsl_route::request::ItemKind::Formula, &item.claim)
}

fn record_type_name(
    package: &fsl_package::CheckedPackage,
    key: &filament_exact::NodeKey,
) -> String {
    package
        .graph()
        .semantic_graph()
        .node(key.clone())
        .and_then(|node| node.declaration())
        .map(|name| {
            name.iter()
                .map(|part| part.as_str())
                .collect::<Vec<_>>()
                .join("::")
        })
        .unwrap_or_default()
}

fn invalid_population_code() -> CatalogCode {
    CatalogCode::new("invalid_runtime_input", "invalid-value")
}

/// The FR-898 invalid population refusal, naming `trace` when one is at fault.
fn invalid_population(trace: Option<&str>) -> EngineReport {
    let cause = fsl_replay::ProofRefusalCause::InvalidInput;
    let decline = fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput);
    EngineReport::Refused(match trace {
        Some(trace) => {
            let mut detail = serde_json::Map::new();
            detail.insert("trace".to_owned(), serde_json::json!(trace));
            Refusal::DeclinedWithDetail(cause, decline, invalid_population_code(), detail)
        }
        None => Refusal::Declined(cause, decline, invalid_population_code()),
    })
}
