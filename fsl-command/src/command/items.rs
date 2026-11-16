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
    Clause(Option<Box<fsl_replay::spine::ClauseRunRequest>>),
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
    let observations = items
        .get("observations")
        .cloned()
        .unwrap_or_else(|| json!({}));
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
        if kind == "value_invariants" {
            let Some(members) = value.as_object() else {
                return Err(refused("value_invariants", "expected an object"));
            };
            if members.contains_key("parameters") {
                return Err(refused(
                    "parameters",
                    "not valid for a value_invariants selection",
                ));
            }
            let valid_members = members.len() == 1 && members.contains_key("snapshot")
                || members.len() == 2
                    && members.contains_key("snapshot")
                    && members.contains_key("invocation");
            if !valid_members || members.values().any(|member| member.as_str().is_none()) {
                return Err(refused(
                    "value_invariants",
                    "expected snapshot and optional invocation string members",
                ));
            }
        } else if kind == "context" {
            let Some(members) = value.as_object() else {
                return Err(refused("context", "expected an object"));
            };
            if members.len() != 3
                || !["name", "invocation", "side"]
                    .iter()
                    .all(|member| members.get(*member).is_some_and(Value::is_string))
            {
                return Err(refused(
                    "context",
                    "expected name, invocation and side string members",
                ));
            }
            if !matches!(
                members.get("side").and_then(Value::as_str),
                Some("pre" | "post")
            ) {
                return Err(refused("side", "expected `pre` or `post`"));
            }
        }
        let mut decoded = value.clone();
        if find_selector_for(kind, &decoded).is_some() {
            set_selector_reference(
                kind,
                &mut decoded,
                json!({"population": "ix://request-preflight/placeholder", "key": "placeholder"}),
            )?;
        }
        if kind == "call" {
            serde_json::from_value::<super::wire::Call>(decoded)
                .map_err(|error| refused("selections", error.to_string()))?;
        } else if kind != "value_invariants" {
            serde_json::from_value::<fsl_replay::spine::ClauseRunSelectionWire>(
                json!({kind: decoded}),
            )
            .map_err(|error| refused("selections", error.to_string()))?;
        }
        let observation = match kind.as_str() {
            "clause" => value.get("observation").and_then(Value::as_str),
            "context" | "frame" => value.get("invocation").and_then(Value::as_str),
            "call" | "function" => value.get("snapshot").and_then(Value::as_str),
            "value_invariants" => value.get("snapshot").and_then(Value::as_str),
            _ => None,
        };
        if kind == "value_invariants" {
            for member in ["snapshot", "invocation"] {
                if let Some(observation) = value.get(member).and_then(Value::as_str) {
                    if !observation_keys.contains(observation) {
                        return Err(refused(observation, "observation key is not present"));
                    }
                }
            }
        } else if let Some(observation) = observation {
            if !observation_keys.contains(observation) {
                return Err(refused(observation, "observation key is not present"));
            }
        }
        // FR-652 distinguishes a missing observation key from a key that
        // names the wrong document kind. Check the named bytes before any
        // program compilation so an invocation/snapshot swap is a request
        // refusal, never a later admission result.
        for (member, expected) in match kind.as_str() {
            "context" => vec![("invocation", "filament.state.invocation/v1")],
            "value_invariants" => {
                let mut expected = vec![("snapshot", "filament.state.snapshot/v1")];
                if value.get("invocation").is_some() {
                    expected.push(("invocation", "filament.state.invocation/v1"));
                }
                expected
            }
            _ => Vec::new(),
        } {
            let key = value
                .get(member)
                .and_then(Value::as_str)
                .expect("validated member");
            let file = observations
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| refused(key, "observation key is not present"))?;
            let bytes = super::read_file(&directory.join(file), limits.dependent_bytes)?;
            let document: Value = serde_json::from_slice(&bytes).map_err(RunCause::Json)?;
            if document.get("format").and_then(Value::as_str) != Some(expected) {
                return Err(refused(member, format!("expected a {expected} document")));
            }
        }
        normalized.push((index, kind.clone(), value.clone()));
    }

    // The clause batch below owns its full compile preflight. Call and
    // value-invariant paths still compile independently, so parse the shared
    // source first to make syntax failures request-level across those paths.
    if normalized
        .iter()
        .any(|(_, kind, _)| matches!(kind.as_str(), "call" | "value_invariants"))
    {
        preflight_program_parse(directory, &request, limits.dependent_bytes)?;
    }

    let accounting = items.get("accounting").cloned();
    let observation_limits = items.get("observation_limits").cloned();
    let mut expanded = Vec::<(usize, String, Value, Option<Value>, Option<String>)>::new();
    let mut selection_counts = Vec::with_capacity(normalized.len());
    for (index, kind, value) in normalized {
        let selector = find_selector_for(&kind, &value).map(str::to_owned);
        if selector.is_some()
            && !matches!(kind.as_str(), "clause" | "temporal" | "call" | "function")
        {
            return Err(refused(
                "each",
                "selector is not valid in this selection position",
            ));
        }
        if let Some(population) = selector {
            let observation_key = selector_observation(&kind, &value)
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
                    let mut selected_value = value.clone();
                    set_selector_reference(
                        &kind,
                        &mut selected_value,
                        json!({"population": &population, "key": key}),
                    )?;
                    expanded.push((
                        index,
                        kind.clone(),
                        selected_value,
                        Some(json!({"population": &population, "key": key})),
                        None,
                    ));
                    expanded_count += 1;
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
            let snapshot_key = value
                .get("snapshot")
                .and_then(Value::as_str)
                .ok_or_else(|| refused("snapshot", "expected a snapshot key"))?;
            let file = observations
                .get(snapshot_key)
                .and_then(Value::as_str)
                .ok_or_else(|| refused(snapshot_key, "observation key is not present"))?;
            let snapshot_bytes = super::read_file(&directory.join(file), limits.dependent_bytes)?;
            let result =
                prepare_run_bytes(directory, &single_bytes, limits.dependent_bytes, overrides)
                    .and_then(|prepared| match prepared {
                        super::PreparedRun::Clause(request) => {
                            let bytes = snapshot_bytes;
                            let reference =
                                fsl_semantics::model::observation::document_reference(&bytes)
                                    .ok_or_else(|| {
                                        refused(snapshot_key, "invalid snapshot identity")
                                    })?;
                            let invocation = value
                                .get("invocation")
                                .and_then(Value::as_str)
                                .map(|invocation_key| {
                                    let file = observations
                                        .get(invocation_key)
                                        .and_then(Value::as_str)
                                        .ok_or_else(|| {
                                            refused(
                                                invocation_key,
                                                "observation key is not present",
                                            )
                                        })?;
                                    let bytes = super::read_file(
                                        &directory.join(file),
                                        limits.dependent_bytes,
                                    )?;
                                    fsl_semantics::model::observation::document_reference(&bytes)
                                        .ok_or_else(|| {
                                            refused(invocation_key, "invalid invocation identity")
                                        })
                                })
                                .transpose()?;
                            run_value_invariants(*request, &reference, invocation.as_ref())
                        }
                        super::PreparedRun::Function(_) => Err(refused(
                            "value_invariants",
                            "expected a clause-run preparation",
                        )),
                    });
            prepared.push((index, object, ItemExecution::ValueInvariants(result)));
        } else {
            let execution = match prepare_run_bytes(
                directory,
                &single_bytes,
                limits.dependent_bytes,
                overrides,
            ) {
                Ok(super::PreparedRun::Clause(request)) => ItemExecution::Clause(Some(request)),
                Ok(prepared) => ItemExecution::Run(execute_prepared(prepared)),
                Err(cause) => ItemExecution::Run(Err(cause)),
            };
            prepared.push((index, object, execution));
        }
    }

    let mut clause_slots = Vec::new();
    let mut clause_requests = Vec::new();
    for (slot, (_, _, execution)) in prepared.iter_mut().enumerate() {
        if let ItemExecution::Clause(request) = execution {
            if let Some(request) = request.take() {
                clause_slots.push((slot, request.selection.clone()));
                clause_requests.push(*request);
            }
        }
    }
    if !clause_requests.is_empty() {
        match fsl_replay::spine::run_clause_batch(clause_requests) {
            Ok(fsl_replay::spine::ClauseRunBatchOutcome::CompileRefusal(report)) => {
                return Err(refused(
                    "items.selections",
                    format!(
                        "shared program compilation refused: {:?}",
                        report.disposition
                    ),
                ));
            }
            Ok(fsl_replay::spine::ClauseRunBatchOutcome::Reports(reports)) => {
                if reports.len() != clause_slots.len() {
                    return Err(RunCause::ItemsFault);
                }
                for ((slot, selection), report) in clause_slots.into_iter().zip(reports) {
                    let result = super::output::clause_report(&report, &selection);
                    prepared[slot].2 = ItemExecution::Run(result);
                }
            }
            Err(fsl_replay::spine::ClauseRunBatchRefusal::EmptySource) => {
                return Err(refused("program", "source is empty"));
            }
            Err(fsl_replay::spine::ClauseRunBatchRefusal::DifferentCompilation) => {
                return Err(refused(
                    "items.selections",
                    "clause selections did not share one compilation",
                ));
            }
        }
    }

    if let Some(result) = prepared.iter().find_map(|(_, _, item)| match item {
        ItemExecution::Run(Ok(result)) if is_compile_refusal(result.value.as_value()) => {
            Some(result)
        }
        _ => None,
    }) {
        return Err(RunCause::ClauseRequest {
            member: "items.selections".to_owned(),
            reason: format!("a selection failed compilation: {}", result.value),
        });
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
                    if let Some(count) = selection_counts
                        .iter_mut()
                        .find(|count| count["index"] == index)
                    {
                        count["expanded"] = json!(results.len());
                    }
                    for result in results {
                        let mut item = json!({
                            "selection": index,
                            "result": result["result"]
                        });
                        if let Some(invariant) = result.get("invariant") {
                            item["invariant"] = invariant.clone();
                        }
                        if let Some(path) = result.get("path") {
                            item["path"] = path.clone();
                        }
                        if let Some(object) = &object {
                            item["object"] = object.clone();
                        }
                        item_values.push(item);
                    }
                    continue;
                }
                Err(cause) => {
                    if whole_request_failure(&cause)
                        || matches!(
                            cause,
                            RunCause::Spine(_) | RunCause::Source(_) | RunCause::Native(_)
                        )
                    {
                        return Err(cause);
                    }
                    let error = RunError { cause };
                    let value = error.value().map_err(RunCause::Output)?;
                    (value, error.category().exit_code())
                }
            },
            ItemExecution::Run(result) => match result {
                Ok(result) if is_internal_item_failure(&result) => {
                    return Err(RunCause::ItemsFault)
                }
                Ok(result) => (result.value.as_value().clone(), result.exit_code),
                Err(cause) => {
                    if whole_request_failure(&cause)
                        || matches!(
                            cause,
                            RunCause::Spine(_) | RunCause::Source(_) | RunCause::Native(_)
                        )
                    {
                        return Err(cause);
                    }
                    let error = RunError { cause };
                    let value = error.value().map_err(RunCause::Output)?;
                    (value, error.category().exit_code())
                }
            },
            ItemExecution::Clause(_) => return Err(RunCause::ItemsFault),
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
        outcome: None,
    }))
}

fn preflight_program_parse(directory: &Path, request: &Value, byte_limit: usize) -> Result<()> {
    let program: super::wire::Program = serde_json::from_value(
        request
            .get("request")
            .and_then(|request| request.get("program"))
            .cloned()
            .ok_or_else(|| refused("program", "expected a program object"))?,
    )
    .map_err(|error| refused("program", error.to_string()))?;
    let identities = program
        .source
        .identity
        .bind()
        .map_err(RunCause::Identifier)?;
    let source_path = directory.join(&program.source.file);
    let bytes = super::read_file(&source_path, byte_limit)?;
    let path = program.source.file;
    let cancel = filament_exact::Cancel::new();
    match fsl_replay::spine::parse(
        &fsl_replay::spine::ParseRequest {
            source: &identities.native,
            path: &path,
            bytes: &bytes,
        },
        fsl_replay::spine::SpineLimits::default().source,
        &cancel,
    ) {
        Ok(_) => Ok(()),
        Err(failure) => match fsl_replay::spine::refusal_or_fault(failure) {
            Ok(refusal) => Err(RunCause::Spine(Box::new(super::SpineFailure {
                source: identities.native,
                path,
                span: None,
                refusal: *refusal,
            }))),
            Err(fault) => Err(RunCause::SpineRun(Box::new(
                fsl_replay::spine::RunRefusal::Fault(fault),
            ))),
        },
    }
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

fn find_selector_for<'a>(kind: &str, value: &'a Value) -> Option<&'a str> {
    match kind {
        "clause" => find_selector(value),
        "temporal" => value
            .get("over")
            .and_then(|over| over.get("reference"))
            .and_then(|reference| reference.get("each"))
            .and_then(|each| each.get("population"))
            .and_then(Value::as_str),
        "call" | "function" => {
            value
                .get("arguments")
                .and_then(Value::as_array)
                .and_then(|arguments| {
                    arguments.iter().find_map(|argument| {
                        argument
                            .get("value")
                            .and_then(|value| value.get("each"))
                            .and_then(|each| each.get("population"))
                            .and_then(Value::as_str)
                    })
                })
        }
        _ => None,
    }
}

fn selector_observation<'a>(kind: &str, value: &'a Value) -> Option<&'a str> {
    match kind {
        "clause" => value.get("observation").and_then(Value::as_str),
        "temporal" => {
            let trace = value.get("trace")?;
            trace
                .get("finite")
                .and_then(Value::as_array)
                .and_then(|positions| positions.first())
                .and_then(Value::as_str)
                .or_else(|| {
                    trace
                        .get("lasso")
                        .and_then(|lasso| lasso.get("prefix"))
                        .and_then(Value::as_array)
                        .and_then(|positions| positions.first())
                        .and_then(Value::as_str)
                })
                .or_else(|| {
                    trace
                        .get("timestamped")
                        .and_then(|trace| trace.get("observations"))
                        .and_then(Value::as_array)
                        .and_then(|positions| positions.first())
                        .and_then(|position| position.get("position"))
                        .and_then(Value::as_str)
                })
                .or_else(|| {
                    trace
                        .get("model")
                        .and_then(|trace| trace.get("initial"))
                        .and_then(Value::as_str)
                })
                .or_else(|| {
                    trace
                        .get("timed")
                        .and_then(|trace| trace.get("prefix"))
                        .and_then(Value::as_array)
                        .and_then(|positions| positions.first())
                        .and_then(|position| position.get("position"))
                        .and_then(Value::as_str)
                })
        }
        "call" | "function" => value.get("snapshot").and_then(Value::as_str),
        _ => None,
    }
}

fn set_selector_reference(kind: &str, value: &mut Value, reference: Value) -> Result<()> {
    if kind == "clause" {
        return set_clause_self(value, reference);
    }
    if matches!(kind, "call" | "function") {
        let arguments = value
            .get_mut("arguments")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| refused("arguments", "expected an argument array"))?;
        let argument = arguments
            .iter_mut()
            .find(|argument| {
                argument
                    .get("value")
                    .is_some_and(|value| value.get("each").is_some())
            })
            .ok_or_else(|| {
                refused(
                    "each",
                    "expected a population selector in an argument value",
                )
            })?;
        argument["value"] = if reference.get("each").is_some() {
            reference
        } else {
            json!({"reference": reference})
        };
        return Ok(());
    }
    let Some(over) = value.get_mut("over").and_then(Value::as_object_mut) else {
        return Err(refused(
            "over",
            "expected an object for a population selector",
        ));
    };
    over.insert("reference".to_owned(), reference);
    Ok(())
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

fn whole_request_failure(cause: &RunCause) -> bool {
    match cause {
        RunCause::SpineRun(refusal) => matches!(
            refusal.as_ref(),
            fsl_replay::spine::RunRefusal::Cancelled(_) | fsl_replay::spine::RunRefusal::Fault(_)
        ),
        RunCause::MonitorFault
        | RunCause::MonitorCancelled
        | RunCause::AnalyzeFault
        | RunCause::ItemsFault => true,
        _ => cause.category().exit_code() == 30,
    }
}

fn is_compile_refusal(value: &Value) -> bool {
    value.get("format").and_then(Value::as_str) == Some("native-run-result/2")
        && value
            .get("disposition")
            .and_then(|disposition| disposition.get("stage"))
            .and_then(Value::as_str)
            == Some("compile")
        && value
            .get("disposition")
            .and_then(|disposition| disposition.get("category"))
            .and_then(Value::as_str)
            == Some("refusal")
}

fn is_internal_item_failure(result: &RunResult) -> bool {
    result.exit_code == 30
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
    invocation: Option<&fsl_semantics::model::observation::DocumentRef>,
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
    let (observations, admission_usage) =
        match fsl_semantics::model::observation::admit_value_invariant_selection(
            graph.model_inputs(),
            graph.scope().types(),
            graph.scope().operations(),
            &request.packages,
            request.limits.model,
            &fsl_semantics::model::observation::Provisions {
                snapshots: &request.snapshots,
                invocations: &request.invocations,
            },
            snapshot,
            invocation,
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
                    vec![json!({"result": invariant_result(
                        category, "admit", None,
                        Some(json!({"code": code, "cause": cause, "fields": fields})),
                        &fsl_semantics::model::observation::AdmissionUsage::default(),
                    )})],
                    exit,
                ));
            }
        };
    let mut meter = filament_exact::Meter::new(request.accounting);
    let evaluation = package
        .evaluate_value_invariant_selection(&observations, &mut meter)
        .map_err(|failure| match failure {
            filament_eval::CallFailure::Fault(_) => RunCause::ItemsFault,
            other => refused("evaluation", other.to_string()),
        })?;
    let results = evaluation
        .results
        .iter()
        .map(|result| {
            let holds = result.holds();
            json!({
                "invariant": {"package": result.clause.package, "node": result.clause.node},
                "path": fsl_replay::runtime_value_path_json(&result.path),
                "result": invariant_result(
                    if holds { "success" } else { "violation" },
                    "evaluate", Some(holds), None, &admission_usage,
                ),
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

fn invariant_result(
    category: &str,
    stage: &str,
    truth: Option<bool>,
    refusal: Option<Value>,
    usage: &fsl_semantics::model::observation::AdmissionUsage,
) -> Value {
    let mut disposition = json!({"category": category, "stage": stage, "truth": truth});
    if stage == "evaluate" {
        if let Some(value) = truth {
            disposition["outcome"] = json!({
                "kind": "completed",
                "value": {"kind": "boolean", "value": value},
            });
        }
    }
    if let Some(refusal) = refusal {
        disposition["refusal"] = refusal;
    }
    let settled = truth.is_some();
    json!({
        "format": "native-run-result/2",
        "disposition": disposition,
        "usage": {
            "admission": {
                "document_bytes": usage.document_bytes,
                "objects": usage.objects,
                "values": usage.values,
            },
            "evaluation_admissions": 0,
            "evaluation_consumed": [],
        },
        "basis": if settled { "closed-scope" } else { "unavailable" },
        "axes": {
            "activation": if settled { "active" } else { "unavailable" },
            "execution": if stage == "evaluate" { Some("completed") } else { None::<&str> },
            "settlement": if settled { Some("closed-scope") } else { Some("unavailable") },
            "truth": truth.map(|truth| if truth { "true" } else { "false" }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_item_failure_is_request_fatal() {
        let result = RunResult {
            exit_code: 30,
            value: NativeResult::items_document(json!({"category": "internal-failure"})),
            outcome: None,
        };
        assert!(is_internal_item_failure(&result));
        let refusal = RunResult {
            exit_code: 20,
            value: NativeResult::items_document(json!({"category": "refusal"})),
            outcome: None,
        };
        assert!(!is_internal_item_failure(&refusal));
    }
}
