// SPDX-License-Identifier: AGPL-3.0-or-later
//! The corpus replay adapter: a corpus `replay-request` file, validated
//! against the published schema, is read by the one FSpec FR-336 replay
//! request reader (`fsl_command::replay_request`, FR-862) and replayed.

use std::path::{Component, Path};

use fsl_command::unique_json::{self, Deep};
use fsl_foundation::{SettingLimits, SettingName};
use fsl_replay::{OutcomeDiagnostic, OutcomeLocus, ReplayRefusal};
use serde_json::Value;

use super::{document, from_document, Execution, Failure, InputKind, Inputs};

impl From<fsl_command::replay_request::RequestDefect> for Failure {
    fn from(defect: fsl_command::replay_request::RequestDefect) -> Self {
        Failure::decode(defect.pointer(), defect.message())
    }
}

pub(super) fn execute(
    inputs: &Inputs<'_>,
    root: &Path,
    limits: &fsl_replay::CallerLimits,
    overrides: &[String],
    schema: &jsonschema::JSONSchema,
) -> Result<Execution, Failure> {
    let (request_file, bytes) = inputs.one(InputKind::ReplayRequest)?;
    drop(document(bytes, schema)?);
    let documents = inputs
        .files
        .iter()
        .filter(|(file, _)| {
            matches!(
                file.kind,
                InputKind::Model | InputKind::ObjectContext | InputKind::Population
            )
        })
        .map(|(_, bytes)| bytes.clone())
        .collect();
    let mut request = fsl_command::replay_request::read(
        bytes,
        limits.replay.input_bytes,
        |path, pointer| corpus_bytes(inputs, root, path, pointer),
        documents,
    )?;
    if let Some((path, error)) = inputs.unreadable_bytes.first() {
        return Err(Failure::io(path, error));
    }
    for operand in overrides {
        let Some((name, _)) = operand.split_once('=') else {
            continue;
        };
        let bound = match SettingName::from_name(name) {
            Some(SettingName::Accounting(setting)) => limits
                .accounting
                .bounds()
                .into_iter()
                .find(|(candidate, _)| *candidate == setting)
                .map(|(_, bound)| bound),
            Some(SettingName::Stage(setting)) => limits
                .spine
                .bounds()
                .into_iter()
                .find(|(candidate, _)| *candidate == setting)
                .map(|(_, bound)| bound),
            None => None,
        };
        if let Some(bound) = bound {
            if fsl_replay::StageLimits::default()
                .with_named(name, bound)
                .is_ok()
            {
                request.stage_limits.insert(name.to_owned(), bound);
            }
        }
    }
    let result = fsl_replay::replay(request, limits.replay);
    let actual = match &result {
        Ok(result) => Some(replay_value(result)?),
        Err(refusal @ (ReplayRefusal::Package(_) | ReplayRefusal::PackageWithIdentity { .. })) => {
            // Corpus diagnostics locate the embedded package in its request;
            // the retained operation bytes keep the native artifact locus.
            let locus = refusal.pointer().map(|pointer| OutcomeLocus::Document {
                document: None,
                path: Some(request_file.path.clone()),
                pointer: pointer.into_owned(),
                object: None,
                member: None,
            });
            let diagnostic = OutcomeDiagnostic::new(
                refusal.cause(),
                refusal.code().as_str(),
                locus,
                refusal.to_string(),
            );
            Some(Deep::new(
                serde_json::to_value(diagnostic)
                    .map_err(|error| Failure::decode("", error.to_string()))?,
            ))
        }
        Err(_) => None,
    };
    let mut execution = from_document(fsl_replay::OutcomeDocument::from_replay(&result), actual)?;
    if result.as_ref().is_ok_and(|result| {
        result.limit().is_some()
            || result.pre_call_limit().is_some_and(|stop| {
                matches!(
                    stop.cause,
                    filament_semantic_value::supplied_stop::SuppliedCause::Limit(_)
                        | filament_semantic_value::supplied_stop::SuppliedCause::Cancelled(_)
                )
            })
    }) {
        execution.partition = super::Partition::Incomplete;
    }
    Ok(execution)
}

/// A corpus byte provision row's file: a listed, readable bytes input whose
/// path is relative to, and resolves inside, the corpus root.
fn corpus_bytes(
    inputs: &Inputs<'_>,
    root: &Path,
    path: &str,
    pointer: &str,
) -> Result<Vec<u8>, Failure> {
    let relative = Path::new(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Failure::decode(
            pointer,
            "byte path is not relative to the corpus",
        ));
    }
    let bytes = inputs
        .files
        .iter()
        .find(|(file, _)| file.kind == InputKind::Bytes && file.path == path)
        .map(|(_, bytes)| bytes.clone())
        .ok_or_else(|| {
            Failure::decode(pointer, "byte path is not a listed, readable bytes input")
        })?;
    let resolved = std::fs::canonicalize(root.join(relative))
        .map_err(|error| Failure::decode(pointer, error.to_string()))?;
    if !resolved.starts_with(root) {
        return Err(Failure::decode(pointer, "byte path escapes the corpus"));
    }
    Ok(bytes)
}

fn replay_value(result: &fsl_replay::ReplayResult) -> Result<Deep, Failure> {
    let row = result
        .to_function_json()
        .map_err(|error| Failure::decode("", error.to_string()))?;
    let row = unique_json::read(row.as_bytes()).map_err(|error| Failure::decode("", error))?;
    // Moved, not written through `json!`, which would walk the row again.
    let results = Value::Array(vec![row.into_inner()]);
    Ok(Deep::new(Value::Object(
        std::iter::once(("results".to_owned(), results)).collect(),
    )))
}

#[cfg(test)]
mod depth_tests {
    use ix_trace_rs::trace;

    #[trace("TC-1325", "FR-350-AC-11")]
    #[test]
    fn malformed_typed_witnesses_at_depth_keep_the_decode_partition() {
        let mut schema: serde_json::Value = serde_json::from_slice(
            filament_specification::published_file("proposals/conformance-corpus-v1/schema.json")
                .expect("pinned FSpec publishes the corpus schema"),
        )
        .expect("the selected schema is JSON");
        let alternatives = schema["$defs"]["WitnessValue"]["oneOf"]
            .as_array()
            .expect("the published discriminated alternatives")
            .len();
        assert_eq!(
            super::super::guard_witness_alternatives(&mut schema),
            alternatives
        );
        let schema = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .with_document("urn:filament:conformance-corpus:v1".to_owned(), schema)
            .compile(
                &serde_json::json!({"$ref":"urn:filament:conformance-corpus:v1#/$defs/WitnessValue"}),
            )
            .expect("the actual published witness schema compiles");
        let mut valid = br#"{"kind":"sequence","elements":["#.repeat(192);
        valid.extend_from_slice(br#"{"kind":"boolean","value":true}"#);
        valid.extend_from_slice(&b"]}".repeat(192));
        let admitted = super::super::document(&valid, &schema)
            .unwrap_or_else(|_| panic!("a schema-valid deep witness remains admitted"));
        drop(admitted);
        let mut bytes = br#"{"kind":"sequence","elements":["#.repeat(192);
        bytes.extend_from_slice(br#"{"kind":"boolean","value":"not-a-boolean"}"#);
        bytes.extend_from_slice(&b"]}".repeat(192));
        let failure = match super::super::document(&bytes, &schema) {
            Ok(value) => {
                drop(value);
                panic!("a mistyped witness leaf must refuse after deep JSON admission");
            }
            Err(failure) => failure,
        };
        assert_eq!(failure.cause, Some("corpus-decode"));
        assert!(!failure.message.contains("recursion limit"));
    }
}
