// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-651: read several independent selections through the public run command.

use super::{
    execute_prepared, output::NativeResult, prepare_run_bytes, RequestOverrides, Result, RunCause,
    RunError, RunLimits, RunResult,
};
use serde_json::{json, Map, Value};
use std::path::Path;

const FORMAT: &str = "native-run-items/1";

enum ItemExecution {
    Run(Result<RunResult>),
    ValueInvariants(Result<(Vec<Value>, u8)>),
    PopulationNotHeld(String),
}

pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<RunResult>> {
    let mut request: Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    let Some(body) = request.get_mut("request").and_then(Value::as_object_mut) else {
        return Ok(None);
    };
    let Some(items) = body.remove("items") else {
        return Ok(None);
    };
    if body.contains_key("call") || body.contains_key("clause") {
        return Err(refused(
            "items",
            "items is mutually exclusive with call and clause",
        ));
    }
    let Some(items) = items.as_object() else {
        return Err(refused("items", "expected an object"));
    };
    if let Some(unknown) = items.keys().find(|key| {
        ![
            "selections",
            "observations",
            "accounting",
            "observation_limits",
        ]
        .contains(&key.as_str())
    }) {
        return Err(refused(unknown, "unknown items member"));
    }
    let Some(selections) = items.get("selections").and_then(Value::as_array) else {
        return Err(refused("selections", "expected a non-empty array"));
    };
    if selections.is_empty() {
        return Err(refused("selections", "must not be empty"));
    }
    let observation_keys = items
        .get("observations")
        .and_then(Value::as_object)
        .map(|observations| {
            observations
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut normalized = Vec::with_capacity(selections.len());
    for (index, selection) in selections.iter().enumerate() {
        let Some(selection) = selection.as_object() else {
            return Err(refused("selections", "each selection must be an object"));
        };
        if selection.len() != 1 {
            return Err(refused(
                "selections",
                "each selection must have exactly one member",
            ));
        }
        let (kind, value) = selection.iter().next().expect("one member");
        if ![
            "call",
            "clause",
            "frame",
            "function",
            "temporal",
            "context",
            "value_invariants",
        ]
        .contains(&kind.as_str())
        {
            return Err(refused(
                "selections",
                format!("unknown selection member {kind}"),
            ));
        }
        if each_count(value) > 1 {
            return Err(refused(
                "each",
                "a selection can carry at most one population selector",
            ));
        }
        if kind == "value_invariants"
            && (value
                .as_object()
                .is_none_or(|object| object.len() != 1 || !object.contains_key("snapshot"))
                || value.get("snapshot").and_then(Value::as_str).is_none())
        {
            return Err(refused(
                "value_invariants",
                "expected exactly one string snapshot member",
            ));
        }
        let observation = match kind.as_str() {
            "clause" => value.get("observation").and_then(Value::as_str),
            "context" | "frame" => value.get("invocation").and_then(Value::as_str),
            "function" => value.get("snapshot").and_then(Value::as_str),
            "value_invariants" => value.get("snapshot").and_then(Value::as_str),
            _ => None,
        };
        if let Some(observation) = observation {
            if !observation_keys.contains(observation) {
                return Err(refused(observation, "observation key is not present"));
            }
        }
        normalized.push((index, kind.clone(), value.clone()));
    }

    let observations = items
        .get("observations")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let accounting = items.get("accounting").cloned();
    let observation_limits = items.get("observation_limits").cloned();
    let mut expanded = Vec::<(usize, String, Value, Option<Value>, Option<String>)>::new();
    let mut selection_counts = Vec::with_capacity(normalized.len());
    for (index, kind, mut value) in normalized {
        let selector = find_selector(&value).map(str::to_owned);
        if selector.is_some() && kind != "clause" {
            return Err(refused(
                "each",
                "selector is not valid in this selection position",
            ));
        }
        if let Some(population) = selector {
            let observation_key = value
                .get("observation")
                .and_then(Value::as_str)
                .ok_or_else(|| refused("observation", "required for a population selector"))?;
            let file = observations
                .get(observation_key)
                .and_then(Value::as_str)
                .ok_or_else(|| refused(observation_key, "observation key is not present"))?;
            let bytes = super::read_file(&directory.join(file), limits.dependent_bytes)?;
            let snapshot: Value = serde_json::from_slice(&bytes).map_err(RunCause::Json)?;
            let population_entry = snapshot
                .get("populations")
                .and_then(Value::as_array)
                .and_then(|populations| {
                    populations.iter().find(|entry| {
                        entry.get("population").and_then(Value::as_str) == Some(population.as_str())
                    })
                });
            let objects = population_entry
                .and_then(|entry| entry.get("objects"))
                .and_then(Value::as_array);
            if let Some(objects) = objects {
                let mut expanded_count = 0;
                for object in objects {
                    let Some(key) = object.get("key").and_then(Value::as_str) else {
                        continue;
                    };
                    set_clause_self(&mut value, json!({"population": &population, "key": key}))?;
                    expanded.push((
                        index,
                        kind.clone(),
                        value.clone(),
                        Some(json!({"population": &population, "key": key})),
                        None,
                    ));
                    expanded_count += 1;
                    set_clause_self(&mut value, json!({"each": {"population": &population}}))?;
                }
                selection_counts.push(json!({"index": index, "expanded": expanded_count}));
            } else {
                expanded.push((index, kind.clone(), value, None, Some(population)));
                selection_counts.push(json!({"index": index, "expanded": 1}));
            }
        } else {
            if contains_each(&value) {
                return Err(refused(
                    "each",
                    "selector is not valid in this selection position",
                ));
            }
            expanded.push((index, kind.clone(), value, None, None));
            selection_counts.push(json!({"index": index, "expanded": 1}));
        }
    }

    let mut prepared = Vec::with_capacity(expanded.len());
    for (index, kind, value, object, missing_population) in expanded {
        if let Some(population) = missing_population {
            prepared.push((index, object, ItemExecution::PopulationNotHeld(population)));
            continue;
        }
        let mut single = request.clone();
        let single_body = single
            .get_mut("request")
            .and_then(Value::as_object_mut)
            .expect("request object was checked");
        if kind == "call" {
            let mut call = value
                .as_object()
                .cloned()
                .ok_or_else(|| refused("call", "expected a call object"))?;
            if let Some(accounting) = accounting.clone() {
                call.entry("accounting".to_owned()).or_insert(accounting);
            }
            single_body.insert("call".to_owned(), Value::Object(call));
        } else {
            let mut clause = Map::new();
            let selection = if kind == "value_invariants" {
                let snapshot = value.get("snapshot").cloned().unwrap_or(Value::Null);
                json!({"function": {"name": "items_value_invariants_placeholder",
                    "arguments": [], "snapshot": snapshot}})
            } else {
                json!({kind.clone(): value})
            };
            clause.insert("selection".to_owned(), selection);
            clause.insert("observations".to_owned(), observations.clone());
            if let Some(accounting) = accounting.clone() {
                clause.insert("accounting".to_owned(), accounting);
            }
            if let Some(limits) = observation_limits.clone() {
                clause.insert("observation_limits".to_owned(), limits);
            }
            single_body.insert("clause".to_owned(), Value::Object(clause));
        }
        let single_bytes = serde_json::to_vec(&single).map_err(RunCause::Json)?;
        if kind == "value_invariants" {
            let result =
                prepare_run_bytes(directory, &single_bytes, limits.dependent_bytes, overrides)
                    .and_then(|prepared| match prepared {
                        super::PreparedRun::Clause(request) => {
                            let snapshot_key = value
                                .get("snapshot")
                                .and_then(Value::as_str)
                                .ok_or_else(|| refused("snapshot", "expected a snapshot key"))?;
                            let file = observations
                                .get(snapshot_key)
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    refused(snapshot_key, "observation key is not present")
                                })?;
                            let bytes =
                                super::read_file(&directory.join(file), limits.dependent_bytes)?;
                            let reference =
                                fsl_semantics::model::observation::document_reference(&bytes)
                                    .ok_or_else(|| {
                                        refused(snapshot_key, "invalid snapshot identity")
                                    })?;
                            run_value_invariants(*request, &reference)
                        }
                        super::PreparedRun::Function(_) => Err(refused(
                            "value_invariants",
                            "expected a clause-run preparation",
                        )),
                    });
            prepared.push((index, object, ItemExecution::ValueInvariants(result)));
        } else {
            let result =
                prepare_run_bytes(directory, &single_bytes, limits.dependent_bytes, overrides)
                    .and_then(execute_prepared);
            prepared.push((index, object, ItemExecution::Run(result)));
        }
    }

    let mut item_values = Vec::with_capacity(prepared.len());
    let source = source_from_request(&request);
    let mut exit_code = 0;
    for (index, object, execution) in prepared {
        let (value, item_exit) = match execution {
            ItemExecution::PopulationNotHeld(population) => (population_refusal(&population), 20),
            ItemExecution::ValueInvariants(result) => match result {
                Ok((results, item_exit)) => {
                    exit_code = more_severe(exit_code, item_exit);
                    selection_counts
                        .iter_mut()
                        .find(|count| count["index"] == index)
                        .map(|count| count["expanded"] = json!(results.len()));
                    for result in results {
                        let mut item = json!({"selection": index,
                            "invariant": result["invariant"],
                            "path": result["path"],
                            "result": result["result"]});
                        if let Some(object) = &object {
                            item["object"] = object.clone();
                        }
                        item_values.push(item);
                    }
                    continue;
                }
                Err(cause) => {
                    if cause.category().exit_code() == 30 {
                        return Err(cause);
                    }
                    let error = RunError { cause };
                    let value = error.value().map_err(RunCause::Output)?;
                    (value, error.category().exit_code())
                }
            },
            ItemExecution::Run(result) => match result {
                Ok(result) => (result.value.as_value().clone(), result.exit_code),
                Err(cause) => {
                    if cause.category().exit_code() == 30 {
                        return Err(cause);
                    }
                    let error = RunError { cause };
                    let value = error.value().map_err(RunCause::Output)?;
                    (value, error.category().exit_code())
                }
            },
        };
        exit_code = more_severe(exit_code, item_exit);
        let mut item = json!({"selection": index, "result": value});
        if let Some(object) = object {
            item["object"] = object;
        }
        item_values.push(item);
    }
    let category = category_for_exit(exit_code);
    let document = NativeResult::items_document(json!({
        "format": FORMAT,
        "source": source,
        "selections": selection_counts,
        "items": item_values,
        "category": category,
    }));
    Ok(Some(RunResult {
        exit_code,
        value: document,
    }))
}

fn contains_each(value: &Value) -> bool {
    match value {
        Value::Object(object) => object
            .iter()
            .any(|(key, child)| key == "each" || contains_each(child)),
        Value::Array(values) => values.iter().any(contains_each),
        _ => false,
    }
}

fn each_count(value: &Value) -> usize {
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, child)| usize::from(key == "each") + each_count(child))
            .sum(),
        Value::Array(values) => values.iter().map(each_count).sum(),
        _ => 0,
    }
}

fn find_selector(value: &Value) -> Option<&str> {
    value
        .get("self")
        .and_then(|self_object| self_object.get("each"))
        .and_then(|each| each.get("population"))
        .and_then(Value::as_str)
}

fn set_clause_self(value: &mut Value, self_object: Value) -> Result<()> {
    let Some(object) = value.as_object_mut() else {
        return Err(refused("clause", "expected a clause selection object"));
    };
    object.insert("self".to_owned(), self_object);
    Ok(())
}

fn source_from_request(request: &Value) -> Value {
    let Some(source) = request
        .get("request")
        .and_then(|request| request.get("program"))
        .and_then(|program| program.get("source"))
    else {
        return Value::Null;
    };
    json!({
        "authority": source.get("authority"),
        "identity": source.get("identity"),
        "path": source.get("file"),
    })
}

fn refused(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

fn population_refusal(population: &str) -> Value {
    json!({
        "format": "native-run-result/2",
        "disposition": {
            "category": "refusal",
            "refusal": {
                "code": "invalid_runtime_input",
                "cause": "wrong-role-mapping",
                "fields": {"population": population},
            },
            "stage": "admit",
            "truth": null,
        },
        "basis": "unavailable",
        "axes": {
            "activation": "unavailable",
            "execution": null,
            "settlement": "unavailable",
            "truth": null,
        },
        "usage": {
            "admission": {"document_bytes": 0, "objects": 0, "values": 0},
            "evaluation_admissions": 0,
            "evaluation_consumed": [],
        },
    })
}

fn more_severe(left: u8, right: u8) -> u8 {
    fn rank(code: u8) -> u8 {
        match code {
            30 => 7,
            20 => 6,
            21 => 5,
            22 => 4,
            10 => 3,
            11 => 2,
            _ => 0,
        }
    }
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

fn category_for_exit(exit_code: u8) -> &'static str {
    match exit_code {
        30 => "internal-failure",
        20 => "refusal",
        21 => "unsupported",
        22 => "incomplete",
        10 => "violation",
        11 => "indeterminate",
        _ => "success",
    }
}

fn run_value_invariants(
    request: fsl_replay::spine::ClauseRunRequest,
    snapshot: &fsl_semantics::model::observation::DocumentRef,
) -> Result<(Vec<Value>, u8)> {
    use fsl_eval::value::CheckedPackageEvaluation;
    use fsl_replay::spine::{ClauseRunSource, ParseRequest};

    let ClauseRunSource::Program {
        identity,
        path,
        bytes,
    } = request.source
    else {
        return Err(refused("program", "expected a program source"));
    };
    let cancel = filament_exact::Cancel::new();
    let parsed = fsl_replay::spine::parse(
        &ParseRequest {
            source: &identity,
            path: &path,
            bytes: &bytes,
        },
        request.limits.source,
        &cancel,
    )
    .map_err(|error| refused("program", format!("compile failed: {error:?}")))?
    .into_value();
    let models =
        fsl_replay::spine::select(&parsed, &request.packages, request.limits.model, &cancel)
            .map_err(|error| refused("models", format!("model selection failed: {error:?}")))?
            .into_value();
    let temporal_selections = request.clocks.check_selections(&parsed);
    let checked = fsl_replay::spine::check_with_refinement_internals(
        &parsed,
        request.package_name,
        fsl_replay::spine::SourceInputs {
            refinement_internals: None,
            temporal_selections: &temporal_selections,
            clocks: Some(&request.clocks),
        },
        &models,
        &request.dependencies,
        request.limits,
        &cancel,
    )
    .map_err(|error| refused("program", format!("check failed: {error:?}")))?
    .into_value();
    let package = checked.package();
    let graph = package.graph();
    let (current, admission_usage) =
        match fsl_semantics::model::observation::admit_value_invariants_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            &request.packages,
            request.limits.model,
            &request.snapshots,
            snapshot,
            request.observation_limits,
        ) {
            Ok(admitted) => admitted,
            Err(failure) => {
                let (category, exit, code, cause, fields) = match failure {
                    fsl_semantics::model::observation::AdmissionFailure::Refused(record) => (
                        "refusal",
                        20,
                        record.catalog.code().to_owned(),
                        record.catalog.cause().to_owned(),
                        serde_json::to_value(record.fields).unwrap_or(Value::Null),
                    ),
                    fsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => (
                        "incomplete",
                        22,
                        record.catalog.code().to_owned(),
                        record.catalog.cause().to_owned(),
                        serde_json::to_value(record.fields).unwrap_or(Value::Null),
                    ),
                    fsl_semantics::model::observation::AdmissionFailure::Fault(_) => {
                        return Err(RunCause::ClauseRequest {
                            member: "snapshot".to_owned(),
                            reason: "observation admission invariant failed".to_owned(),
                        })
                    }
                };
                return Ok((
                    vec![json!({"result": {
                        "format": "native-run-result/2",
                        "disposition": {"category": category, "stage": "admit",
                            "truth": null, "refusal": {"code": code,
                                "cause": cause, "fields": fields}},
                    }})],
                    exit,
                ));
            }
        };
    let mut meter = filament_exact::Meter::new(request.accounting);
    let evaluation = package
        .evaluate_current_value_invariants(&current, &mut meter)
        .map_err(|failure| refused("evaluation", failure.to_string()))?;
    let results = evaluation
        .results
        .iter()
        .map(|result| {
            let holds = result.holds();
            json!({
                "invariant": {"package": result.clause.package, "node": result.clause.node},
                "path": fsl_replay::runtime_value_path_json(&result.path),
                "result": {"category": if holds { "success" } else { "violation" },
                    "truth": holds},
            })
        })
        .collect::<Vec<_>>();
    let exit = match evaluation.outcome {
        fsl_eval::value::ValueInvariantOutcome::True => 0,
        fsl_eval::value::ValueInvariantOutcome::False => 10,
        fsl_eval::value::ValueInvariantOutcome::Incomplete(_) => 22,
    };
    let _ = admission_usage;
    Ok((results, exit))
}
