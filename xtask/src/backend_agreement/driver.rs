// SPDX-License-Identifier: AGPL-3.0-or-later
//! The explicit driver process and native corpus/compiler adapters.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use filament_exact::{Cancel, Value as RuntimeValue};
use fsl_analyze::analyze::document::{resolve_items, ResolvedItem, SubjectSelection};
use fsl_analyze::analyze::{AnalyzeLimits, Engines, SubjectBinding};
use fsl_foundation::{BackendId, SourceIdentity};
use fsl_replay::spine::{self, CheckedUnit, DependencyInput, ParseRequest};
use fsl_replay::value_check::{ValueTarget, Verdict};
use fsl_route::Registry;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::DocumentProvision;
use serde_json::{json, Value};

use super::super::{self as corpus, ConsumerType, InputKind, Vector};
use super::{CandidateRecord, Entry, Item, Negotiated, Provider, Recheck, Settlement};
use crate::error::{Error, Result};
use crate::Outcome;

pub(super) struct Driver {
    executable: PathBuf,
    registry: Registry,
    outcome_schema: jsonschema::JSONSchema,
}

fn refusal(message: impl Into<String>) -> Error {
    Error::QualificationRefusal(format!("backend-agreement: {}", message.into()))
}

impl Driver {
    pub(super) fn new(executable: &Path) -> Result<Self> {
        if !executable.is_absolute() {
            return Err(refusal("FSL must be an absolute executable path"));
        }
        let engines = Engines::fsl().map_err(|error| refusal(error.to_string()))?;
        let mut registry = Registry::new();
        for descriptor in engines.registrations() {
            registry
                .register(descriptor.clone())
                .map_err(|error| refusal(error.to_string()))?;
        }
        let schema: Value = serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1)
            .map_err(|error| refusal(error.to_string()))?;
        let outcome_schema = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&schema)
            .map_err(|error| refusal(error.to_string()))?;
        Ok(Self {
            executable: executable.to_path_buf(),
            registry,
            outcome_schema,
        })
    }
}

impl Provider for Driver {
    fn registry(&self) -> &Registry {
        &self.registry
    }

    fn preflight(&self, backends: &[BackendId]) -> Result<()> {
        let output = Command::new(&self.executable)
            .arg("--version")
            .output()
            .map_err(|error| {
                refusal(format!(
                    "backends {backends:?}: tool {}: {error}",
                    self.executable.display()
                ))
            })?;
        if !output.status.success() || output.stdout.is_empty() {
            return Err(refusal(format!(
                "backends {backends:?}: tool {} did not report its version",
                self.executable.display()
            )));
        }
        Ok(())
    }

    #[ix_trace_rs::string_edge]
    fn execute(&self, item: &Item<'_>, backend: &BackendId) -> Result<CandidateRecord> {
        let mut request = item.request.clone();
        request["request"]["items"] = json!([{
            "claim": item.claim.claim_name,
            "kind": item.claim.item.kind.to_wire(),
            "backend": backend.as_str(),
        }]);
        let directory = tempfile::tempdir().map_err(|error| refusal(error.to_string()))?;
        let path = directory.path().join("analyze.json");
        fs::write(
            &path,
            serde_json::to_vec(&request).map_err(|error| refusal(error.to_string()))?,
        )
        .map_err(|error| refusal(error.to_string()))?;
        let mut command = Command::new(&self.executable);
        command.arg("analyze").arg(path).args(["--format", "json"]);
        for setting in item.overrides {
            command.args(["--limit", setting]);
        }
        let output = command
            .output()
            .map_err(|error| refusal(error.to_string()))?;
        let document: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            refusal(format!(
                "{} emitted no analyze outcome ({:?}): {error}",
                backend.as_str(),
                output.status.code()
            ))
        })?;
        if let Err(mut errors) = self.outcome_schema.validate(&document) {
            let reason = errors
                .next()
                .map(|error| error.to_string())
                .unwrap_or_default();
            return Err(refusal(format!(
                "driver emitted an invalid outcome document: {reason}"
            )));
        }
        if document.get("format").and_then(Value::as_str) != Some("filament-outcome/1")
            || document.get("operation").and_then(Value::as_str) != Some("analyze")
        {
            return Err(refusal("driver emitted an unrelated outcome document"));
        }
        let entries = document
            .get("items")
            .and_then(Value::as_array)
            .filter(|items| items.len() == 1)
            .ok_or_else(|| refusal("driver did not return the one requested item"))?;
        let mut record = entries
            .first()
            .cloned()
            .ok_or_else(|| refusal("driver omitted the requested item"))?;
        let terminal = record.get("record").unwrap_or(&record);
        let disposition =
            if terminal.get("settled_by").and_then(Value::as_str) == Some("disposition") {
                match terminal.get("result").and_then(Value::as_str) {
                    Some("requires-bound") => Negotiated::RequiresBound,
                    Some("unsupported") => Negotiated::Unsupported,
                    Some("invalid-request" | "declined") => Negotiated::InvalidRequest,
                    _ => {
                        return Err(refusal(
                            "driver returned an unrecognized negotiated disposition",
                        ))
                    }
                }
            } else {
                if terminal.get("backend").and_then(Value::as_str) != Some(backend.as_str()) {
                    return Err(refusal("driver's terminal record names another backend"));
                }
                Negotiated::Supported
            };
        record["request_index"] = json!(item.claim.item.request_index.get());
        if let Some(terminal) = record.get_mut("record") {
            terminal["request_index"] = json!(item.claim.item.request_index.get());
        }
        let declared_domain = declared_domain(item)?;
        Ok(CandidateRecord {
            backend: backend.as_str().to_owned(),
            disposition,
            record: Some(record),
            declared_domain,
            replay: None,
            not_compared: None,
        })
    }

    fn replay(&self, item: &Item<'_>, record: &CandidateRecord) -> Recheck {
        recheck_value(item, record)
    }
}

fn declared_domain(item: &Item<'_>) -> Result<Option<Vec<super::DomainBound>>> {
    let Ok(target) = ValueTarget::select(
        item.checked.package(),
        &item.claim.claim_name,
        &item.claim.item.claim,
    ) else {
        return Ok(None);
    };
    let mut bounds = Vec::new();
    for parameter in target.parameters() {
        if let fsl_replay::value_check::ParameterDomain::Integers(Some(interval)) =
            parameter.domain()
        {
            bounds.push(super::DomainBound {
                parameter: fsl_foundation::node_wire::node_key_to_json(&parameter.wire())
                    .map_err(|error| refusal(error.to_string()))?,
                lower: interval.lower().to_string(),
                upper: interval.upper().to_string(),
            });
        }
    }
    Ok((!bounds.is_empty()).then_some(bounds))
}

fn recheck_value(item: &Item<'_>, record: &CandidateRecord) -> Recheck {
    let refused = |cause: &str| Recheck::Refused {
        code: "evaluator_unsupported".to_owned(),
        cause: cause.to_owned(),
    };
    let target = match ValueTarget::select(
        item.checked.package(),
        &item.claim.claim_name,
        &item.claim.item.claim,
    ) {
        Ok(target) => target.with_limits(item.limits.accounting),
        Err(fsl_replay::value_check::Selection::Fault(fault)) => {
            return Recheck::Fault {
                detail: format!("{fault:?}"),
            }
        }
        Err(_) => return refused("not-a-value-target"),
    };
    let Some(counterexample) = record
        .record
        .as_ref()
        .and_then(|record| record.get("counterexample"))
    else {
        return Recheck::Inconclusive;
    };
    let assignments =
        match fsl_replay::read_plugin_assignments(&json!({"counterexample": counterexample})) {
            Ok(assignments) => assignments,
            Err(_) => return refused("counterexample-not-assignments"),
        };
    let values = target
        .parameters()
        .iter()
        .map(|parameter| {
            let value = &assignments
                .iter()
                .find(|assignment| assignment.parameter == parameter.wire())?
                .value;
            match value {
                fsl_replay::WitnessValue::Integer(value) => {
                    Some(RuntimeValue::Integer((*value).into()))
                }
                fsl_replay::WitnessValue::ExactInteger(value) => {
                    value.to_string().parse().ok().map(RuntimeValue::Integer)
                }
                fsl_replay::WitnessValue::Boolean(value) => Some(RuntimeValue::Boolean(*value)),
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>();
    let Some(values) = values else {
        return refused("witness-value-not-supported");
    };
    let cancel = Cancel::new();
    match target.evaluate(&values, &cancel) {
        Ok(Verdict::Violated(expected)) => match target.replay(&values, &expected, &cancel) {
            Ok(Some(_)) => Recheck::ReproducedWithEvaluatedWitness,
            Ok(None) => Recheck::Inconclusive,
            Err(fault) => Recheck::Fault {
                detail: format!("{fault:?}"),
            },
        },
        Ok(Verdict::Holds | Verdict::NotDecisive) => Recheck::Inconclusive,
        Ok(Verdict::Stopped(_)) => Recheck::Refused {
            code: "resource_exhausted".to_owned(),
            cause: "replay-accounting-limit".to_owned(),
        },
        Ok(_) => refused("replay-undecided"),
        Err(fault) => Recheck::Fault {
            detail: format!("{fault:?}"),
        },
    }
}

struct CompiledVector {
    vector: String,
    capability: String,
    checked: CheckedUnit,
    claims: Vec<fsl_analyze::analyze::document::ResolvedClaim>,
    request: Value,
}

fn failed_vector(vector: &Vector, settlement: Settlement, failure: Value) -> Entry {
    Entry {
        vector: vector.id.clone(),
        request_index: None,
        capability: vector.capability_id.clone(),
        settlement,
        candidates: Vec::new(),
        failure: Some(failure),
    }
}

fn compile(
    vector: &Vector,
    root: &Path,
    limits: &fsl_replay::CallerLimits,
    registry: &Registry,
) -> std::result::Result<CompiledVector, Entry> {
    let mut inputs = Vec::new();
    for input in &vector.input.files {
        let path = fs::canonicalize(root.join(&input.path)).map_err(|error| {
            failed_vector(
                vector,
                Settlement::ToolFailure,
                json!({"path":input.path,"io":error.to_string()}),
            )
        })?;
        if !path.starts_with(root) {
            return Err(failed_vector(
                vector,
                Settlement::ToolFailure,
                json!({"path":input.path,"io":"input escapes corpus root"}),
            ));
        }
        let bytes = fs::read(&path).map_err(|error| {
            failed_vector(
                vector,
                Settlement::ToolFailure,
                json!({"path":input.path,"io":error.to_string()}),
            )
        })?;
        inputs.push((input, path, bytes));
    }
    let sources = inputs
        .iter()
        .filter(|(input, _, _)| input.kind == InputKind::Source)
        .collect::<Vec<_>>();
    let [source] = sources.as_slice() else {
        return Err(failed_vector(
            vector,
            Settlement::ToolFailure,
            json!({"input":"expected one source file"}),
        ));
    };
    let cancel = Cancel::new();
    let source_identity = SourceIdentity::new("agent-ix", &source.0.path);
    let compile_error = |error: spine::FrontEndFailure| {
        let settlement = if matches!(error, fsl_foundation::diagnostic::StageFailure::Fault(_)) {
            Settlement::ToolFailure
        } else {
            Settlement::NotCompiled
        };
        let detail = match corpus::adapters::front_end_failure(error) {
            Ok(execution) => execution.actual,
            Err(error) => serde_json::to_value(error).unwrap_or(Value::Null),
        };
        failed_vector(vector, settlement, detail)
    };
    let parsed = spine::parse(
        &ParseRequest {
            source: &source_identity,
            path: &source.0.path,
            bytes: &source.2,
        },
        limits.spine.source,
        &cancel,
    )
    .map_err(compile_error)?
    .into_value();
    let packages = package_input(
        inputs
            .iter()
            .filter(|(input, _, _)| input.kind == InputKind::Model)
            .map(|(_, _, bytes)| bytes.as_slice()),
    )
    .map_err(|error| {
        failed_vector(
            vector,
            Settlement::NotCompiled,
            json!({"model":format!("{error:?}")}),
        )
    })?;
    let models = spine::select(&parsed, &packages, limits.spine.model, &cancel)
        .map_err(compile_error)?
        .into_value();
    let checked = spine::check(
        &parsed,
        "qualification".to_owned(),
        &models,
        &DependencyInput::default(),
        limits.spine,
        &cancel,
    )
    .map_err(compile_error)?
    .into_value();
    let documents = DocumentProvision::default();
    let binding = SubjectBinding {
        packages: &packages,
        snapshots: &documents,
        initial: Vec::new(),
        universes: Vec::new(),
    };
    let subject = SubjectSelection::default_over(&documents);
    let resolved = resolve_items(
        checked.package(),
        &subject,
        None,
        registry,
        &[checked.source().reference().clone()],
        &binding,
        &AnalyzeLimits::unbounded(),
    );
    let claims = resolved
        .into_iter()
        .filter_map(|item| match item {
            ResolvedItem::Claim(claim) => Some(*claim),
            _ => None,
        })
        .collect();
    let models = inputs.iter().filter(|(input,_,_)| input.kind == InputKind::Model).map(|(input,path,_)| json!({"format":"semantic-ir/2.0.0","source":{"file":path,"authority":"agent-ix","identity":input.path,"document":input.path,"formal_revision":1}})).collect::<Vec<_>>();
    let request = json!({"format":"native-analyze/1","request":{
        "program":{"source":{"file":source.1,"authority":"agent-ix","identity":source.0.path,"document":source.0.path,"formal_revision":1}},"models":models,
    }});
    Ok(CompiledVector {
        vector: vector.id.clone(),
        capability: vector.capability_id.clone(),
        checked,
        claims,
        request,
    })
}

fn panic_detail(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
        })
        .unwrap_or_else(|| "backend operation panicked with a non-text payload".to_owned())
}

pub(super) fn execute(operands: &[OsString], provider: &dyn Provider) -> Result<Outcome> {
    let fspec = corpus::fspec_dir();
    let prepared = corpus::prepare(fspec.path(), operands)?;
    let mut compiled = Vec::new();
    let mut entries = Vec::new();
    for vector in &prepared.corpus.vectors {
        if !prepared.scope_ids.contains(&vector.capability_id)
            || !vector
                .applicability
                .consumer_types
                .contains(&ConsumerType::Language)
        {
            continue;
        }
        // The audit boundary records a panic and continues with a fresh vector;
        // it never repairs compiler state or reuses a partial result.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            compile(
                vector,
                &prepared.corpus_root,
                &prepared.limits,
                provider.registry(),
            )
        })) {
            Ok(Ok(vector)) => compiled.push(vector),
            Ok(Err(entry)) => entries.push(entry),
            Err(payload) => entries.push(failed_vector(
                vector,
                Settlement::ToolFailure,
                json!({"panic":panic_detail(payload)}),
            )),
        }
    }
    let backends = compiled
        .iter()
        .flat_map(|vector| &vector.claims)
        .filter(|claim| claim.item.candidates.len() >= 2)
        .flat_map(|claim| {
            claim
                .item
                .candidates
                .iter()
                .map(|candidate| candidate.id().clone())
        })
        .collect::<BTreeSet<_>>();
    if backends.is_empty() {
        return Err(refusal("no-comparable-item"));
    }
    provider.preflight(&backends.into_iter().collect::<Vec<_>>())?;
    for vector in compiled {
        for claim in &vector.claims {
            let item = Item {
                checked: &vector.checked,
                claim,
                request: &vector.request,
                limits: &prepared.limits,
                overrides: &prepared.args.limits,
            };
            let mut candidates = Vec::new();
            let mut failure = None;
            if claim.item.candidates.len() < 2 {
                entries.push(Entry {
                    vector: vector.vector.clone(),
                    request_index: Some(claim.item.request_index.get()),
                    capability: vector.capability.clone(),
                    settlement: Settlement::NotCompared,
                    candidates: Vec::new(),
                    failure: Some(json!({"reason":"fewer-than-two-candidates"})),
                });
                continue;
            }
            for backend in &claim.item.candidates {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    provider.execute(&item, backend.id())
                })) {
                    Ok(Ok(mut candidate)) => {
                        if candidate.record.as_ref().is_some_and(|record| {
                            record
                                .pointer("/record/result")
                                .or_else(|| record.get("result"))
                                == Some(&json!("refuted"))
                        }) {
                            candidate.replay = Some(
                                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    provider.replay(&item, &candidate)
                                })) {
                                    Ok(replay) => replay,
                                    Err(payload) => Recheck::Fault {
                                        detail: panic_detail(payload),
                                    },
                                },
                            );
                        }
                        candidates.push(candidate);
                    }
                    Ok(Err(error)) => {
                        failure = Some(
                            json!({"backend":backend.id().as_str(),"failure":error.to_string()}),
                        )
                    }
                    Err(payload) => {
                        failure = Some(
                            json!({"backend":backend.id().as_str(),"panic":panic_detail(payload)}),
                        )
                    }
                }
            }
            let mut settlement = super::compare(&mut candidates);
            if failure.is_some() && settlement != Settlement::Conflict {
                settlement = Settlement::ToolFailure;
            }
            entries.push(Entry {
                vector: vector.vector.clone(),
                request_index: Some(claim.item.request_index.get()),
                capability: vector.capability.clone(),
                settlement,
                candidates,
                failure,
            });
        }
    }
    super::report(entries, prepared.args.text)
}
