// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 (FSpec FR-817): a `native-run/1` request's `export` member asks for
//! a TLA+ or Lean text of the program's checked package. This module reads the
//! member, compiles the program, runs the mapper, writes the
//! `filament.export-outcome/v1` document and, when a file is named and a
//! package was assembled, the target file.

mod map;
mod request;

use super::{
    check_sources, intake_with_bytes, output::NativeResult, output_file, wire, FileCounts,
    RequestOverrides, Result, RunCause, RunLimits, RunResult,
};
use filament_semantic_value::catalog::Category;
use fsl_replay::{OutcomeDocument, OutcomeStage};
use request::{EffectiveLimits, Request};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The generator identity the `package` member records (FSpec FR-817).
const GENERATOR_IDENTITY: &str = "filament-spec-language/export";

/// FSpec FR-298's disposition of one record, as FR-269 names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Disposition {
    Preserved,
    Unrepresented,
}

impl Disposition {
    const fn name(self) -> &'static str {
        match self {
            Self::Preserved => "preserved",
            Self::Unrepresented => "unrepresented",
        }
    }
}

/// One FSpec FR-298 record: the obligation's disposition, its conditions and
/// causes, and the half-open byte regions of the text that represent it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Record {
    pub(super) obligation: String,
    pub(super) disposition: Disposition,
    pub(super) conditions: Vec<String>,
    pub(super) causes: Vec<Value>,
    pub(super) regions: Vec<(usize, usize)>,
}

impl Record {
    fn value(&self) -> Value {
        json!({
            "obligation": self.obligation,
            "disposition": self.disposition.name(),
            "conditions": self.conditions,
            "causes": self.causes,
            "regions": self.regions.iter().map(|(start, end)| json!([start, end])).collect::<Vec<_>>(),
        })
    }

    /// Whether this record fails `floor` (FSpec FR-811 "The fidelity floor").
    fn fails(&self, floor: request::Floor) -> bool {
        match floor {
            request::Floor::Any => false,
            request::Floor::Conditional => self.disposition == Disposition::Unrepresented,
            request::Floor::Preserved => self.disposition != Disposition::Preserved,
        }
    }
}

/// What the mapper made of the request.
pub(super) enum Mapped {
    /// Text, its name map and one record per obligation.
    Assembled {
        text: String,
        name_map: Vec<Value>,
        records: Vec<Record>,
    },
    /// A denied export charge: the setting that stopped it.
    Incomplete { setting: &'static str },
}

/// The facts of the compiled program the outcome's `package` member names.
struct Program {
    identity: String,
    version: String,
}

/// Run the request at `directory`/`bytes` when it carries an `export` member.
///
/// # Errors
///
/// A refusal of the request, of its program, or a mapper failure.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<RunResult>> {
    let mut document: Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(None);
    }
    let Some(body) = document.get_mut("request").and_then(Value::as_object_mut) else {
        return Ok(None);
    };
    let Some(member) = body.remove("export") else {
        return Ok(None);
    };
    let given = overrides
        .limits
        .map(|limits| limits.export)
        .unwrap_or_default();
    let request = request::read(&member, &given)?;
    for name in ["output", "handoff"] {
        if body.contains_key(name) {
            return Err(RunCause::ClauseRequest {
                member: name.to_owned(),
                reason: "a compile member is not an export member; name the file in export.output"
                    .to_owned(),
            });
        }
    }
    let output = target_path(directory, &request, overrides)?;
    document["format"] = json!(fsl_foundation::wire_format::WireFormat::CompileRequest.as_str());
    let compile = serde_json::to_vec(&document).map_err(RunCause::Output)?;
    let compile: wire::CompileRequest = super::request(&compile)?;
    let mapped = compile_and_map(directory, &compile, &request, limits, overrides)?;
    finish(&request, &mapped.0, mapped.1, output.as_deref())
}

/// The file the export writes: the member's, the caller's, or none. A path
/// that exists refuses before any stage runs (FR-1015).
fn target_path(
    directory: &Path,
    request: &Request,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<PathBuf>> {
    let path = match (&request.output, overrides.output) {
        (Some(_), Some(_)) => return Err(RunCause::OutputGivenTwice),
        (Some(member), None) => Some(directory.join(member)),
        (None, supplied) => supplied.map(Path::to_path_buf),
    };
    if let Some(path) = &path {
        if path.symlink_metadata().is_ok() {
            return Err(RunCause::OutputExists { path: path.clone() });
        }
    }
    Ok(path)
}

fn compile_and_map(
    directory: &Path,
    compile: &wire::CompileRequest,
    request: &Request,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<(Mapped, Program)> {
    if compile.program.extraction {
        return Err(RunCause::ExtractionRequiresRun);
    }
    if compile.program.sources.len() != 1 {
        return Err(RunCause::ClauseRequest {
            member: "program".to_owned(),
            reason: "an export names one program source".to_owned(),
        });
    }
    let counts = FileCounts {
        programs: compile.libraries.len().saturating_add(1),
        models: compile.models.len(),
        observations: usize::from(request.initial.is_some()),
        extensions: compile.extensions.len(),
    };
    let mut intake = intake_with_bytes(directory, counts, limits.dependent_bytes)?;
    let selection = &compile.program.sources[0];
    let source = intake.source(selection, "program.sources[0].source")?;
    super::require_complete_edition(source.source())?;
    let program = Program {
        identity: super::package_name(source.source().identity()),
        version: selection.identity.formal_revision.to_string(),
    };
    let initial = request
        .initial
        .as_deref()
        .map(|file| {
            let bytes = intake.file(file, intake.remaining)?;
            serde_json::from_slice::<Value>(&bytes).map_err(|error| RunCause::ClauseRequest {
                member: "/request/export/initial".to_owned(),
                reason: format!("`{file}` is not a JSON document: {error}"),
            })
        })
        .transpose()?;
    let spine_limits = overrides.limits.copied().unwrap_or_default();
    let effective = request.limits.effective();
    let mapped = check_sources(
        compile,
        &mut intake,
        std::slice::from_ref(&source),
        false,
        spine_limits,
        None,
        None,
        |checked, selections, _points, packages, _, _libraries, _models, _inventory, failure| {
            let cancel = filament_exact::Cancel::new();
            let result = fsl_replay::spine::package_with_temporal_selections(
                &checked,
                selections,
                spine_limits.package,
                &cancel,
            );
            let emitted = result.map_err(failure)?.into_value();
            let admitted = fsl_package::admit_checked_package(
                emitted.package().bytes(),
                &packages,
                fsl_package::V2ReadLimits::bounded(
                    fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT,
                ),
            )
            .map_err(|refusal| RunCause::ClauseRequest {
                member: "program".to_owned(),
                reason: format!("the compiled package is not admitted: {refusal:?}"),
            })?;
            let _ = (&admitted, &initial);
            let clauses: Vec<map::ClauseFact> = checked
                .package()
                .graph()
                .state_clauses()
                .iter()
                .map(|clause| map::ClauseFact {
                    name: clause.name().to_owned(),
                    kind: match clause.kind() {
                        fsl_forms::StateClauseKind::Invariant => "invariant",
                        fsl_forms::StateClauseKind::Constraint => "constraint",
                        fsl_forms::StateClauseKind::Precondition => "precondition",
                        fsl_forms::StateClauseKind::Postcondition => "postcondition",
                    },
                })
                .chain(
                    checked
                        .package()
                        .graph()
                        .temporal_clauses()
                        .iter()
                        .map(|node| map::ClauseFact {
                            name: node.clause().name().to_owned(),
                            kind: "temporal",
                        }),
                )
                .collect();
            map::run(&program.identity, &clauses, request, &effective)
        },
    )?;
    Ok((mapped, program))
}

fn finish(
    request: &Request,
    mapped: &Mapped,
    program: Program,
    output: Option<&Path>,
) -> Result<Option<RunResult>> {
    let effective = request.limits.effective();
    let mut export = json!({
        "format": "filament.export-outcome/v1",
        "mapping_profile_id": request.target.profile(),
        "target_family": request.target.family(),
    });
    let (status, category) = match mapped {
        Mapped::Assembled {
            text,
            name_map,
            records,
        } => assembled(
            &mut export,
            request,
            records,
            text,
            name_map,
            &program,
            effective,
        ),
        Mapped::Incomplete { setting } => {
            export["status"] = json!("incomplete");
            export["incomplete"] = json!({"limit": setting});
            export["records"] = json!([]);
            ("incomplete", Category::Incomplete)
        }
    };
    debug_assert!(!status.is_empty());
    if let (Some(path), Mapped::Assembled { text, .. }) = (output, mapped) {
        if category == Category::Success {
            output_file::write_new_file(path, text.as_bytes())?;
        }
    }
    if let Some(named) = &request.output {
        export["output"] = json!({
            "path": named,
            "bytes_written": if category == Category::Success {
                mapped_len(mapped)
            } else {
                0
            },
        });
    } else if let Some(path) = output {
        export["output"] = json!({
            "path": path.display().to_string(),
            "bytes_written": if category == Category::Success { mapped_len(mapped) } else { 0 },
        });
    }
    let document = OutcomeDocument::new(
        fsl_replay::Operation::Export,
        Some(OutcomeStage::S4),
        category,
    )
    .with_export(export.clone());
    Ok(Some(RunResult {
        exit_code: category.exit_code(),
        value: NativeResult::export_document(json!({"export": export})),
        outcome: Some(document),
        requirements: None,
        sources: crate::Sources::default(),
    }))
}

fn mapped_len(mapped: &Mapped) -> usize {
    match mapped {
        Mapped::Assembled { text, .. } => text.len(),
        Mapped::Incomplete { .. } => 0,
    }
}

/// Fill `export` for an assembled mapping, applying the floor: below it the
/// request is `refused` with no package and no text.
fn assembled(
    export: &mut Value,
    request: &Request,
    records: &[Record],
    text: &str,
    name_map: &[Value],
    program: &Program,
    effective: EffectiveLimits,
) -> (&'static str, Category) {
    if u64::try_from(text.len()).map_or(true, |length| length > effective.output_bytes) {
        export["status"] = json!("incomplete");
        export["incomplete"] = json!({"limit": "export.output_bytes"});
        export["records"] = json!([]);
        return ("incomplete", Category::Incomplete);
    }
    let failing: Vec<Value> = records
        .iter()
        .filter(|record| record.fails(request.floor))
        .map(Record::value)
        .collect();
    export["records"] = json!(records.iter().map(Record::value).collect::<Vec<_>>());
    if !failing.is_empty() {
        export["status"] = json!("refused");
        export["refusal"] = json!({
            "code": "unsupported_projection",
            "cause": "below-fidelity-floor",
            "failing": failing,
        });
        return ("refused", Category::Unsupported);
    }
    export["status"] = json!("assembled");
    export["package"] = json!({
        "source_package": {"identity": program.identity, "version": program.version},
        "target_profile": request.target.profile(),
        "generator": {"identity": GENERATOR_IDENTITY, "version": env!("CARGO_PKG_VERSION")},
        "limits": {
            "export.output_bytes": effective.output_bytes,
            "export.nodes": effective.nodes,
            "export.work_units": effective.work_units,
        },
    });
    export["module"] = json!(map::module_name(&program.identity));
    export["target_text"] = json!(text);
    export["name_map"] = json!(name_map);
    export["observer"] = Value::Null;
    ("assembled", Category::Success)
}
