// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local orchestration for the `1-draft` spine compiler and runner.

mod analyze;
mod clause;
mod flow;
mod items;
mod monitor;
mod output;
mod output_file;
mod replay;
mod requirements;
mod settle;
mod tl;
mod wire;

pub use analyze::{
    AnalyzeInput, PreparedAnalyze, analyze, analyze_with_limits, analyze_with_providers,
    analyze_with_providers_and_limits, prepare_analyze,
};
pub use flow::{FlowAnalyzeError, analyze_flow_report};
pub use monitor::{MonitorEvent, MonitorSummary, monitor};
pub use output::NativeResult;
pub use tl::{AtomMapping, TlRecord, record as tl_record};

use crate::formal_source::FormalSource;
use filament_contract_model as ir;
use fsl_analyze::analyze::document::Negotiate;
use fsl_analyze::provider::ProviderOptions;
use fsl_foundation::serde_object::Object;
use fsl_foundation::{Code, Diagnostic, Source};
use fsl_semantics::model::domain_package::DOMAIN_PACKAGE_PROFILE;
use settle::Settle;
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, RunCause>;
const REQUEST_BYTES: usize = 1_048_576;
const TOTAL_BYTES: usize = 8_388_608;
const FILES: usize = 64;

/// Caller byte ceilings for one `run` or `compile` invocation (FR-312, FR-869).
#[derive(Clone, Copy, Debug)]
pub struct RunLimits {
    /// Maximum bytes in the request file.
    pub request_bytes: usize,
    /// Maximum aggregate bytes in selected dependent files.
    pub dependent_bytes: usize,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            request_bytes: REQUEST_BYTES,
            dependent_bytes: TOTAL_BYTES,
        }
    }
}

/// Selected file category charged by command preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileCategory {
    /// Native program source.
    Program,
    /// Rule-model sources.
    Model,
    /// Clause observation artifacts.
    Observation,
    /// Extension definition documents.
    Extension,
}

impl FileCategory {
    /// Stable category spelling in command intake diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Program => "programs",
            Self::Model => "models",
            Self::Observation => "observations",
            Self::Extension => "extensions",
        }
    }
}

#[derive(Default)]
struct FileCounts {
    programs: usize,
    models: usize,
    observations: usize,
    extensions: usize,
}

/// Command intake ceiling that stopped selected-file processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LimitKind {
    /// Per-file or remaining aggregate byte ceiling.
    FileBytes,
    /// A request file exceeded its caller's configured ceiling.
    RequestBytes {
        /// The caller's effective request byte ceiling.
        maximum: usize,
    },
    /// A command without caller-configurable request limits exceeded its fixed ceiling.
    FixedRequestBytes {
        /// The effective request byte ceiling.
        maximum: usize,
    },
    /// Dependent files exceeded their aggregate configured ceiling.
    DependentBytes {
        /// The caller's effective aggregate dependent byte ceiling.
        maximum: usize,
    },
    /// A compile request exceeded the fixed aggregate dependent-file ceiling.
    FixedDependentBytes {
        /// The effective aggregate dependent byte ceiling.
        maximum: usize,
    },
    /// One selected observation exceeded its document byte ceiling.
    ObservationDocumentBytes {
        /// Effective observation document byte ceiling.
        maximum: usize,
    },
    /// Total selected file count.
    SelectedFiles {
        /// Group that could not be charged.
        category: FileCategory,
        /// Files requested by the group.
        requested: usize,
        /// Remaining slots before this group.
        remaining: usize,
        /// Effective total file ceiling.
        maximum: usize,
    },
}

impl LimitKind {
    /// Stable field spelling in command results.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileBytes => "file bytes",
            Self::RequestBytes { .. } | Self::FixedRequestBytes { .. } => "request_bytes",
            Self::DependentBytes { .. } | Self::FixedDependentBytes { .. } => "dependent_bytes",
            Self::ObservationDocumentBytes { .. } => "observation.document_bytes",
            Self::SelectedFiles { .. } => "selected files",
        }
    }
}

impl std::fmt::Display for LimitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// JSON result of the local command, distinct from a portable evidence envelope.
#[derive(Debug)]
pub struct RunResult {
    /// FR-285's exit code of the outcome's category, or the most severe
    /// item code of a multi-item outcome.
    pub exit_code: u8,
    /// Structured `native-run-result/2` or current spine call observations.
    pub value: NativeResult,
    /// The FR-286 lifecycle outcome of a function run (category and last
    /// stage); `None` for the other run families.
    pub outcome: Option<fsl_replay::OutcomeDocument>,
    /// FR-963: the settled requirement items of a call that carried
    /// `requirements`; `None` exactly when it carried none.
    pub requirements: Option<RequirementResults>,
    sources: crate::Sources,
}

impl RunResult {
    /// Sources admitted while producing this run outcome (FR-865).
    pub fn sources(&self) -> &crate::Sources {
        &self.sources
    }
}

/// The settled requirement items of a run call (FR-963), one record per
/// item of the call in FR-395's package order.
#[derive(Debug)]
pub struct RequirementResults {
    /// One terminal record per item, in package order.
    pub records: Vec<fsl_replay::TerminalRecord>,
    /// Exactly one reproduced counterexample per `refuted` record.
    pub counterexamples: Vec<fsl_analyze::analyze::ItemCounterexample>,
    /// FR-635's sorted `{backend, tool}` list of every backend the items
    /// were negotiated against: the document's `registry` member.
    pub registry: Vec<fsl_analyze::analyze::document::RegistryEntry>,
}

/// Successful compilation with the core's document and exact emitted bytes.
pub struct CompileOutcome {
    package_bytes: Vec<u8>,
    document: fsl_replay::OutcomeDocument,
    handoff_compiled: Option<crate::emit_checked_v2::Compiled>,
    handoff_temporal_selections: Vec<crate::emit_checked_v2::ClockSelection>,
    sources: crate::Sources,
}

/// Caller-selected model/library files and stage ceilings for `check` (FR-866).
#[derive(Clone, Debug)]
pub struct CheckInputs {
    /// Model selections in command-line order.
    pub models: Vec<(String, PathBuf)>,
    /// Library selections in command-line order.
    pub libraries: Vec<(String, PathBuf)>,
    /// Request-supplied extension definition documents for `check`.
    pub extensions: Vec<CheckExtensionInput>,
    /// Optional caller-admitted integer-division law.
    pub integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    /// Check-stage limits.
    pub limits: fsl_replay::spine::SpineLimits,
    /// Aggregate ceiling for the selected model and library files.
    pub dependent_bytes: usize,
}

impl Default for CheckInputs {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            libraries: Vec::new(),
            extensions: Vec::new(),
            integer_division: None,
            limits: fsl_replay::spine::SpineLimits::default(),
            dependent_bytes: TOTAL_BYTES,
        }
    }
}

/// One extension definition selected by the source-only `check` API.
#[derive(Clone, Debug)]
pub struct CheckExtensionInput {
    /// Definition authority.
    pub authority: String,
    /// Definition identity.
    pub identity: String,
    /// Definition document path, resolved from the current directory.
    pub file: PathBuf,
    /// Exact dependency authority and identity pairs.
    pub depends_on: Vec<(String, String)>,
    /// Complete-V1 capabilities declared by this definition.
    pub capabilities: Vec<String>,
}

/// Check document and the source values admitted while producing it (FR-865, FR-866).
#[derive(Debug)]
pub struct CheckOutcome {
    document: fsl_replay::OutcomeDocument,
    sources: crate::Sources,
}

impl CheckOutcome {
    /// The FR-286 check outcome document.
    pub fn document(&self) -> &fsl_replay::OutcomeDocument {
        &self.document
    }

    /// Sources admitted while producing this check outcome (FR-865).
    pub fn sources(&self) -> &crate::Sources {
        &self.sources
    }
}

/// Check one source against the model and library files it selects (FR-866).
/// All selected files pass through the same bounded admission used by compile.
pub fn check_outcome(source_path: &Path, inputs: &CheckInputs) -> CheckOutcome {
    let result = check_outcome_inner(source_path, inputs);
    match result {
        Ok((document, sources)) => CheckOutcome { document, sources },
        Err((cause, sources)) => {
            let document = RunError {
                cause,
                sources: sources.clone(),
            }
            .outcome_document(fsl_replay::Operation::Check);
            CheckOutcome { document, sources }
        }
    }
}

fn check_outcome_inner(
    source_path: &Path,
    inputs: &CheckInputs,
) -> std::result::Result<(fsl_replay::OutcomeDocument, crate::Sources), (RunCause, crate::Sources)>
{
    let mut intake = intake_with_bytes(
        Path::new("."),
        FileCounts {
            programs: inputs.libraries.len().saturating_add(1),
            models: inputs.models.len(),
            observations: 0,
            extensions: inputs.extensions.len(),
        },
        inputs.dependent_bytes,
    )
    .map_err(|cause| (cause, crate::Sources::default()))?;
    let run = (|| -> Result<fsl_replay::OutcomeDocument> {
        validate_check_selections(&inputs.models, "--model")?;
        validate_check_selections(&inputs.libraries, "--library")?;
        let extensions = inputs
            .extensions
            .iter()
            .map(|extension| wire::Extension {
                authority: extension.authority.clone(),
                identity: extension.identity.clone(),
                file: extension.file.to_string_lossy().into_owned(),
                depends_on: extension
                    .depends_on
                    .iter()
                    .map(|(authority, identity)| wire::DefinitionReference {
                        authority: authority.clone(),
                        identity: identity.clone(),
                    })
                    .collect(),
                capabilities: extension.capabilities.clone(),
            })
            .collect::<Vec<_>>();
        validate_extension_entries(&extensions)?;

        let source = intake.source(
            &check_source_file(source_path, "check-source", "CheckSource"),
            "source",
        )?;
        let extension_catalog = extension_catalog(&extensions, &mut intake)?;
        let cancel = filament_exact::Cancel::new();
        let parsed = match fsl_replay::spine::parse_selecting_extensions(
            &fsl_replay::spine::ParseRequest {
                source: source.source().identity(),
                path: source.source().path(),
                bytes: source.source().text().as_bytes(),
            },
            fsl_replay::spine::ProfileCatalogInput {
                catalog: &extension_catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            },
            inputs.limits.source,
            &cancel,
        ) {
            Ok(parsed) => parsed.into_value(),
            Err(failure) => {
                return Ok(fsl_replay::OutcomeDocument::from_check(&Err(failure)));
            }
        };

        let declared_models: std::collections::BTreeSet<_> = parsed
            .unit()
            .selections()
            .models
            .iter()
            .map(|selection| selection.identity.as_str())
            .collect();
        let declared_libraries: std::collections::BTreeSet<_> = parsed
            .unit()
            .selections()
            .imports
            .iter()
            .map(|selection| selection.identity.as_str())
            .collect();
        reject_unselected_check_inputs(&inputs.models, "--model", &declared_models)?;
        reject_unselected_check_inputs(&inputs.libraries, "--library", &declared_libraries)?;

        let models = inputs
            .models
            .iter()
            .map(|(identity, path)| {
                Ok(wire::Model {
                    format: DOMAIN_PACKAGE_PROFILE.to_owned(),
                    source: check_source_file(path, identity, "CheckModel"),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let model_documents = models
            .iter()
            .map(|model| intake.source(&model.source, "models"))
            .collect::<Result<Vec<_>>>()?;
        let packages = fsl_semantics::model::intake::package_input(
            model_documents
                .iter()
                .map(|document| document.source().text().as_bytes()),
        )
        .map_err(|refusal| RunCause::ClauseRequest {
            member: "models".to_owned(),
            reason: refusal.detail,
        })?;
        let admitted_models =
            match fsl_replay::spine::select(&parsed, &packages, inputs.limits.model, &cancel) {
                Ok(models) => models.into_value(),
                Err(failure) => {
                    let document = fsl_replay::OutcomeDocument::from_check(&Err(failure));
                    return Ok(check_missing_model_help(document, &parsed, &inputs.models));
                }
            };

        let libraries = inputs
            .libraries
            .iter()
            .map(|(identity, path)| {
                let source = intake.source(
                    &check_source_file(path, identity, "CheckLibrary"),
                    "libraries",
                )?;
                Ok(fsl_replay::spine::SuppliedLibrary {
                    identity: identity.clone(),
                    source: source.source().identity().clone(),
                    path: source.source().path().to_owned(),
                    bytes: source.source().text().as_bytes().to_vec(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let dependencies = match fsl_replay::spine::DependencyInput::new(libraries) {
            Ok(dependencies) => dependencies,
            Err(refusal) => {
                let failure = fsl_replay::spine::FrontEndFailure::Refused(Box::new(
                    fsl_replay::spine::CompileRefusal::DependencyInput(refusal),
                ));
                return Ok(fsl_replay::OutcomeDocument::from_check(&Err(failure)));
            }
        };
        let checked = if let Some(integer_division) = &inputs.integer_division {
            fsl_replay::spine::check_with_integer_division(
                &parsed,
                package_name(source.source().identity()),
                &admitted_models,
                &dependencies,
                integer_division,
                inputs.limits,
                &cancel,
            )
        } else {
            fsl_replay::spine::check(
                &parsed,
                package_name(source.source().identity()),
                &admitted_models,
                &dependencies,
                inputs.limits,
                &cancel,
            )
        };
        Ok(fsl_replay::OutcomeDocument::from_check(&checked))
    })();
    let sources = crate::Sources::from_sources(intake.sources);
    run.map(|document| (document, sources.clone()))
        .map_err(|cause| (cause, sources))
}

fn check_missing_model_help(
    document: fsl_replay::OutcomeDocument,
    parsed: &fsl_replay::spine::ParsedUnitSource,
    supplied: &[(String, PathBuf)],
) -> fsl_replay::OutcomeDocument {
    let Some(diagnostic) = document.diagnostics().first() else {
        return document;
    };
    if diagnostic.code() != Code::MissingImport.as_str()
        || diagnostic.cause() != Some("missing-selection")
    {
        return document;
    }
    let Some(missing) = parsed.unit().selections().models.iter().find(|model| {
        !supplied
            .iter()
            .any(|(identity, _)| identity == &model.identity)
    }) else {
        return document;
    };
    let message = format!(
        "{}; supply --model {}=<file> or the compile request's models member",
        diagnostic.message(),
        missing.identity
    );
    fsl_replay::OutcomeDocument::new(
        fsl_replay::Operation::Check,
        Some(fsl_replay::OutcomeStage::I1),
        document.category(),
    )
    .with_diagnostics(vec![fsl_replay::OutcomeDiagnostic::new(
        diagnostic.cause(),
        diagnostic.code(),
        diagnostic.locus().cloned(),
        message,
    )])
}

fn validate_check_selections(selections: &[(String, PathBuf)], option: &str) -> Result<()> {
    let mut identities = std::collections::BTreeSet::new();
    for (identity, path) in selections {
        if identity.is_empty() || path.as_os_str().is_empty() {
            return Err(RunCause::ClauseRequest {
                member: option.to_owned(),
                reason: format!("invalid selection {identity}={}", path.display()),
            });
        }
        if !identities.insert(identity) {
            return Err(RunCause::ClauseRequest {
                member: option.to_owned(),
                reason: format!("identity {identity} was selected more than once"),
            });
        }
    }
    Ok(())
}

fn reject_unselected_check_inputs(
    selections: &[(String, PathBuf)],
    option: &str,
    declared: &std::collections::BTreeSet<&str>,
) -> Result<()> {
    if let Some((identity, _)) = selections
        .iter()
        .find(|(identity, _)| !declared.contains(identity.as_str()))
    {
        return Err(RunCause::ClauseRequest {
            member: option.to_owned(),
            reason: format!("{option} {identity} is not selected by the source"),
        });
    }
    Ok(())
}

fn check_source_file(path: &Path, identity: &str, document: &str) -> wire::SourceFile {
    wire::SourceFile {
        file: path.to_string_lossy().into_owned(),
        identity: wire::Identity {
            authority: "fsl-command".to_owned(),
            identity: identity.to_owned(),
            document: document.to_owned(),
            formal_revision: 1,
        },
    }
}

impl CompileOutcome {
    /// The completed package artifact, with no command wrapper.
    pub fn package_bytes(&self) -> &[u8] {
        &self.package_bytes
    }

    /// The outcome projected from the actual staged package result.
    pub fn document(&self) -> &fsl_replay::OutcomeDocument {
        &self.document
    }

    /// Sources admitted while compiling this outcome (FR-865).
    pub fn sources(&self) -> &crate::Sources {
        &self.sources
    }
}

/// Prepared function input retained with its native source correspondence.
pub struct PreparedFunctionRun {
    data: Box<FunctionRunData>,
}

struct FunctionRunData {
    source: FormalSource,
    models: Vec<FormalSource>,
    libraries: Vec<FormalSource>,
    packages: fsl_semantics::model::intake::PackageInput,
    dependencies: fsl_replay::spine::DependencyInput,
    extension_catalog: Option<fsl_semantics::library::bundle::DefinitionCatalog>,
    call: fsl_replay::spine::Call,
    /// FR-960: the call's `requirements`; `None` settles no item.
    requirements: Option<requirements::Requirements>,
}

/// Owned inputs passed to the driver's existing execution pipeline.
pub struct FunctionRunInput {
    /// Admitted source with its authored labels and path.
    pub source: Source,
    /// Package name selected from the authored identity.
    pub package_name: String,
    /// Admitted domain packages.
    pub packages: fsl_semantics::model::intake::PackageInput,
    /// Explicit library dependencies.
    pub dependencies: fsl_replay::spine::DependencyInput,
    /// Function, arguments, accounting and supplied-phase limits.
    pub call: fsl_replay::spine::Call,
}

impl PreparedFunctionRun {
    pub(in crate::command) fn requirements(&self) -> Option<&requirements::Requirements> {
        self.data.requirements.as_ref()
    }

    /// Transfer the prepared typed values without executing a stage.
    pub fn into_input(self) -> FunctionRunInput {
        let data = *self.data;
        FunctionRunInput {
            package_name: package_name(data.source.source().identity()),
            source: data.source.source().clone(),
            packages: data.packages,
            dependencies: data.dependencies,
            call: data.call,
        }
    }
}

/// A decoded function or clause request; preparation performs no execution.
pub enum PreparedRun {
    /// A function selection for the existing execution pipeline.
    Function(PreparedFunctionRun),
    /// A clause selection for the existing clause runner.
    Clause(Box<fsl_replay::spine::ClauseRunRequest>),
    /// FR-931: a `tl_lowering` request: the clause's valuation run, the
    /// lowering it asks for, and the program text its atoms are named in.
    TlLowering(Box<PreparedLowering>),
}

impl PreparedRun {
    fn admitted_sources(&self) -> crate::Sources {
        match self {
            Self::Function(prepared) => crate::Sources::from_sources(
                std::iter::once(prepared.data.source.source().clone())
                    .chain(
                        prepared
                            .data
                            .models
                            .iter()
                            .map(|source| source.source().clone()),
                    )
                    .chain(
                        prepared
                            .data
                            .libraries
                            .iter()
                            .map(|source| source.source().clone()),
                    ),
            ),
            Self::Clause(request) => {
                crate::Sources::from_sources(source_from_clause(&request.source))
            }
            Self::TlLowering(prepared) => prepared.sources.clone(),
        }
    }
}

fn source_from_clause(source: &fsl_replay::spine::ClauseRunSource) -> Option<Source> {
    match source {
        fsl_replay::spine::ClauseRunSource::Program {
            identity,
            path,
            bytes,
        } => Source::read_typed(identity.clone(), path, bytes, bytes.len()).ok(),
        // ClauseRunSource is non-exhaustive; future source variants need an
        // admitted Source projection before they can be rendered here.
        _ => None,
    }
}

/// Original intake failure, without recovering typed data from display messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RunCause {
    /// Local file could not be opened or read.
    #[error("cannot read {path}: {error}")]
    Io {
        /// Actual selected filesystem path.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        error: io::Error,
    },
    /// Closed request decoding failed.
    #[error("invalid request: {0}")]
    Json(#[from] serde_json::Error),
    /// FR-640: the request carries `clocks` and the caller also supplied a
    /// clocks override (the driver's `--clocks`).
    #[error(
        "invalid request: clocks is given both in the request and as a clocks override (--clocks); give it once"
    )]
    ClocksGivenTwice,
    /// FR-810: the request carries `output` and the caller also supplied an
    /// output override (the driver's `--output`).
    #[error(
        "invalid request: output is given both in the request and as an output override (--output); give it once"
    )]
    OutputGivenTwice,
    /// FR-810: a file or directory already exists at the output path.
    #[error("invalid request: output {path} already exists; a compile never overwrites it", path = path.display())]
    OutputExists {
        /// The existing output path.
        path: PathBuf,
    },
    /// FR-312: a clause request member cannot form a typed selection.
    #[error("invalid clause request member {member}: {reason}")]
    ClauseRequest {
        /// The wire member or observation key at fault.
        member: String,
        /// Why this member is invalid.
        reason: String,
    },
    /// Typed output serialization failed.
    #[error("cannot serialize native result: {0}")]
    Output(#[source] serde_json::Error),
    /// A typed clause report failed canonical wire production.
    #[error("cannot serialize clause-run result: {0}")]
    ClauseOutput(#[source] fsl_replay::spine::NativeRunResultError),
    /// Request format was not selected.
    #[error("unsupported native command format")]
    Format,
    /// A command intake ceiling stopped the request.
    #[error("{0} limit exceeded")]
    Limit(LimitKind),
    /// Existing IR identifier validation failed.
    #[error("{message}")]
    Identifier {
        /// The shared diagnostic retained in the structured JSON details.
        diagnostic: ir::Diagnostic,
        /// Plain request-facing sentence, without Rust's `Debug` rendering.
        message: String,
    },
    /// Existing source/parser/profile failure with no linking or checking context.
    #[error("{0}")]
    Native(#[from] Box<Diagnostic>),
    /// One source refusal retained in native and canonical typed projections.
    #[error("{0}")]
    Source(#[source] Box<SourceFailure>),
    /// A `1-draft` program's request selects what spine compile does not
    /// take.
    #[error("a complete-V1 program compiles alone; the request selects {0}")]
    CompleteSelection(CompleteSelection),
    /// FR-031: an extracted program is a run input, not a source-only compile.
    #[error("an extracted program requires run")]
    ExtractionRequiresRun,
    /// FR-027/FR-480: a source-unit list must not be empty.
    #[error("the compile program has no source units")]
    EmptyCompileProgram,
    /// FR-931: `tl_lowering.correspondence` names no TL correspondence.
    #[error(
        "unknown_profile/unsupported-selection: tl_lowering.correspondence {value:?} is none of mltl.closed-trace/v1, mltl.online-prefix/v1 and mltl.origin-complete-history/v1"
    )]
    TlCorrespondence {
        /// The value given.
        value: String,
    },
    /// FR-931: `tl_lowering.clause` names no temporal clause of the program.
    #[error(
        "missing_declaration/missing-name: tl_lowering.clause {name:?} names no temporal clause"
    )]
    TlClause {
        /// The name given.
        name: String,
    },
    /// FR-931: a `tl_lowering` form this build does not run yet
    /// (`unsupported_construct`/`not-yet-implemented`).
    #[error("unsupported_construct/not-yet-implemented: {member} is not implemented yet")]
    TlNotYetImplemented {
        /// The request member the gap is in.
        member: &'static str,
    },
    /// FR-931: the lowering of a supported clause broke an invariant.
    #[error("the TL lowering faulted: {0}")]
    TlFault(String),
    /// The request's `libraries` are refused before any library is read.
    #[error("the request's libraries are refused: {0}")]
    Libraries(LibrarySelection),
    /// The request's extension definition catalog is invalid.
    #[error("the request's extension definitions are refused: {0}")]
    ExtensionCatalog(String),
    /// Spine compile of a `1-draft` program refused at one stage.
    #[error("{}", .0.refusal)]
    Spine(Box<SpineFailure>),
    /// FR-100: a `1-draft` native-run/1 request carries a member the run
    /// command does not admit for it, or carries no `call`.
    #[error("a 1-draft program runs through the spine; the request selects {0}")]
    CompleteRunSelection(CompleteRunSelection),
    /// FR-960: a `call.requirements.items` entry names a claim that is no
    /// item of the call; refused at stage `call` before any item is
    /// negotiated.
    #[error(
        "missing_declaration: call.requirements names the claim {claim}, which is no item of the call"
    )]
    UnknownRequirementClaim {
        /// The claim as the request spells it.
        claim: serde_json::Value,
    },
    /// FR-526, FR-966: a timestamped run's correction relation or
    /// admitted late observations are refused at stage `evaluate`, and no
    /// result document or ledger is written.
    #[error("invalid_runtime_input: the run's correction is refused: {0}")]
    TemporalCorrection(fsl_replay::spine::CorrectionRefusal),
    /// FR-100: the spine run entry refused, at its own stage and cause code.
    #[error("{0}")]
    SpineRun(Box<fsl_replay::spine::RunRefusal>),
    /// FR-620: a `native-analyze/1` request member is unknown, forbidden,
    /// mistyped or inconsistent, or a document of another format was
    /// handed to the analyze reader or an analyze document to another
    /// reader.
    #[error("invalid analyze request member {member}: {reason}")]
    AnalyzeRequest {
        /// The member at fault.
        member: String,
        /// Why it is refused.
        reason: String,
    },
    /// FR-620: a `subject.universes` entry names no provisioned snapshot
    /// (`invalid-request`), or a population of a referenced snapshot is
    /// incomplete (`incomplete_population`).
    #[error("{0}")]
    AnalyzeUniverse(fsl_analyze::analyze::document::UniverseRefusal),
    /// FR-850: the strict checked-package reader or the request's model
    /// supply refused the selected package before any item resolved.
    #[error("{message}")]
    AnalyzePackage {
        /// The catalogued code of the refusal.
        code: Code,
        /// The reader's typed cause tag.
        cause: Option<String>,
        /// The RFC 6901 pointer the reader names.
        pointer: Option<String>,
        /// The refusal, naming the identity or pointer at fault.
        message: String,
    },
    /// FR-620: a request member FSL admits but does not implement yet
    /// (`unsupported_construct`/`not-yet-implemented`, FR-763), such as a
    /// checked-package selection (QSL-2271).
    #[error(
        "unsupported_construct/not-yet-implemented: analyzing a {member} selection is not implemented yet"
    )]
    AnalyzeNotYetImplemented {
        /// The request member the gap is in.
        member: &'static str,
    },
    /// FR-624: the shared subject binding or source provision of an
    /// analyze request was refused before any item ran.
    #[error("{0}")]
    AnalyzeRefused(Box<fsl_analyze::analyze::AnalyzeRefusal>),
    /// FR-644: the monitor session refused the request, with FR-643's record.
    #[error("{0:?}")]
    MonitorRefused(Box<fsl_analyze::monitor::MonitorRefusal>),
    /// An analyze run faulted while writing its document.
    #[error("the analyze run faulted")]
    AnalyzeFault,
    /// FR-644: a monitor run faulted.
    #[error("the monitor run faulted")]
    MonitorFault,
    /// FR-644: the caller cancelled a monitor run.
    #[error("the monitor run was cancelled")]
    MonitorCancelled,
    /// FR-862: a `native-replay/1` request member is outside FSpec
    /// FR-336's replay request mapping, missing, or does not decode.
    #[error("invalid replay request: {0}")]
    ReplayRequest(crate::replay_request::RequestDefect),
    /// FR-862: a `byte_provision` file could not be read, or exceeds the
    /// request's file-count or byte ceilings.
    #[error("replay byte_provision file {bytes_path}: {cause}")]
    ReplayBytes {
        /// The row's `bytes_path`, as the request spells it.
        bytes_path: String,
        /// Why the file was not read.
        cause: Box<RunCause>,
    },
    /// FR-862: FR-071 or FR-098 refused the decoded replay request.
    #[error("{}", .0.refusal)]
    Replay(Box<replay::ReplayFailure>),
    /// FR-862: the settled replay has no FR-323 per-item result encoding.
    #[error("cannot serialize replay result: {0}")]
    ReplayOutput(#[source] fsl_replay::CauseCodecError),
    /// An item run violated an internal runtime invariant.
    #[error("an items run faulted")]
    ItemsFault,
}

/// The two owner projections of the same source-intake refusal.
#[derive(Debug, thiserror::Error)]
#[error("{native}")]
pub struct SourceFailure {
    #[source]
    native: Box<Diagnostic>,
    diagnostic: Box<fsl_cst::diagnostic::CompleteDiagnostic>,
}

/// A request selection FR-100 does not admit for a `1-draft` program's
/// native-run/1 request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompleteRunSelection {
    /// The request selects a model source in a native rule-model format.
    NativeModel,
    /// The request carries no `call`.
    NoCall,
}

impl std::fmt::Display for CompleteRunSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NativeModel => "a native rule-model source",
            Self::NoCall => "no call",
        })
    }
}

/// A spine refusal of a `1-draft` program, with the program it concerns.
#[derive(Debug)]
pub struct SpineFailure {
    /// The program's four source labels.
    pub source: fsl_foundation::SourceIdentity,
    /// The program's request-selected path.
    pub path: String,
    /// The refused region rendered over the program, when the stage records
    /// one.
    pub span: Option<fsl_foundation::LocatedSpan>,
    /// The stage's typed refusal.
    pub refusal: fsl_replay::spine::CompileRefusal,
}

impl From<ir::Diagnostic> for RunCause {
    fn from(value: ir::Diagnostic) -> Self {
        let message = if value.path.ends_with(".formal_revision") {
            value.message.clone()
        } else {
            format!("invalid request identifier: {}", value.message)
        };
        Self::Identifier {
            message,
            diagnostic: value,
        }
    }
}

impl RunCause {
    /// Stable catalogued code for this cause, independent of its display message.
    pub fn code(&self) -> Code {
        match self {
            Self::Io { .. } => Code::IoError,
            Self::Json(_) => Code::InvalidRequest,
            Self::ClauseRequest { .. }
            | Self::ClocksGivenTwice
            | Self::OutputGivenTwice
            | Self::OutputExists { .. } => Code::InvalidRequest,
            Self::Output(_) | Self::ClauseOutput(_) => Code::OutputFailure,
            Self::Format => Code::UnknownWire,
            Self::Limit(_) => Code::ResourceExhausted,
            Self::Identifier { .. } => Code::InvalidIdentifier,
            Self::Native(error) => error.code,
            Self::Source(error) => error.diagnostic.code,
            Self::CompleteSelection(_)
            | Self::Libraries(_)
            | Self::CompleteRunSelection(_)
            | Self::EmptyCompileProgram => Code::InvalidRequest,
            Self::ExtractionRequiresRun => Code::ExtractionRequiresRun,
            Self::Spine(failure) => failure.refusal.code(),
            Self::ExtensionCatalog(_) => Code::InvalidPackage,
            Self::SpineRun(refusal) => refusal.code(),
            Self::UnknownRequirementClaim { .. } => Code::MissingDeclaration,
            Self::TemporalCorrection(_) => Code::InvalidRuntimeInput,
            Self::AnalyzeRequest { .. } => Code::InvalidRequest,
            Self::AnalyzePackage { code, .. } => *code,
            Self::AnalyzeNotYetImplemented { .. } => Code::UnsupportedConstruct,
            Self::TlCorrespondence { .. } => Code::UnknownProfile,
            Self::TlClause { .. } => Code::MissingDeclaration,
            Self::TlNotYetImplemented { .. } => Code::UnsupportedConstruct,
            Self::TlFault(_) => Code::RuntimeInvariant,
            Self::AnalyzeUniverse(
                fsl_analyze::analyze::document::UniverseRefusal::Unprovisioned { .. },
            ) => Code::InvalidRequest,
            Self::AnalyzeUniverse(
                fsl_analyze::analyze::document::UniverseRefusal::Incomplete { .. },
            ) => Code::IncompletePopulation,
            Self::AnalyzeRefused(refusal) => refusal
                .document()
                .diagnostics()
                .first()
                .and_then(|diagnostic| Code::from_code(diagnostic.code()))
                .unwrap_or(Code::RuntimeInvariant),
            Self::MonitorRefused(refusal) => refusal
                .document()
                .diagnostics()
                .first()
                .and_then(|diagnostic| Code::from_code(diagnostic.code()))
                .unwrap_or(Code::RuntimeInvariant),
            Self::AnalyzeFault | Self::MonitorFault | Self::ItemsFault | Self::MonitorCancelled => {
                Code::RuntimeInvariant
            }
            Self::ReplayRequest(_) => Code::InvalidRequest,
            Self::ReplayBytes { cause, .. } => cause.code(),
            Self::Replay(failure) => failure.refusal.code(),
            Self::ReplayOutput(_) => Code::OutputFailure,
        }
    }

    /// Whether the retained cause prevented completion.
    pub fn is_incomplete(&self) -> bool {
        self.code().is_incomplete()
    }

    /// The cause's ADR-013 O-16 category, whose
    /// [`Category::exit_code`](filament_semantic_value::catalog::Category::exit_code)
    /// is the command's exit status (FR-285): writing the outcome out is an
    /// internal failure, as is a broken S6a invariant (FR-100 "Internal
    /// failure at S6a"); every other cause takes its code's category.
    pub fn category(&self) -> filament_semantic_value::catalog::Category {
        match self {
            Self::Output(_) | Self::ClauseOutput(_) | Self::TlFault(_) => {
                filament_semantic_value::catalog::Category::InternalFailure
            }
            Self::SpineRun(refusal) => refusal.category(),
            Self::AnalyzeRefused(refusal) => refusal.document().category(),
            Self::MonitorRefused(refusal) => refusal.document().category(),
            Self::AnalyzeFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::ItemsFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorCancelled => filament_semantic_value::catalog::Category::Incomplete,
            Self::ReplayBytes { cause, .. } => cause.category(),
            Self::Replay(failure) => failure.outcome.category(),
            Self::ReplayOutput(_) => filament_semantic_value::catalog::Category::InternalFailure,
            _ => self.code().category(),
        }
    }
}

/// Failed command with its typed cause.
#[derive(Debug, thiserror::Error)]
#[error("{cause}")]
pub struct RunError {
    /// Original typed failure.
    #[source]
    pub cause: RunCause,
    /// Sources admitted while producing this failure (FR-865).
    pub sources: crate::Sources,
}

impl RunError {
    /// Sources admitted while producing this failure (FR-865).
    pub fn sources(&self) -> &crate::Sources {
        &self.sources
    }
    /// The cause's ADR-013 O-16 category (FR-285).
    pub fn category(&self) -> filament_semantic_value::catalog::Category {
        self.cause.category()
    }

    /// Project the typed command failure through the core's outcome producers.
    /// Intake failures have no reached compiler stage. This operation reads no
    /// file and never reads back the native result's JSON.
    pub fn outcome_document(
        &self,
        operation: fsl_replay::Operation,
    ) -> fsl_replay::OutcomeDocument {
        use fsl_replay::{OutcomeDiagnostic, OutcomeDocument, OutcomeLocus};

        match &self.cause {
            RunCause::Spine(failure) => OutcomeDocument::new(
                operation,
                Some(failure.refusal.stage().into()),
                self.category(),
            )
            .with_diagnostics(failure.refusal.outcome_diagnostics()),
            RunCause::SpineRun(refusal) => OutcomeDocument::from_run_refusal(operation, refusal),
            RunCause::AnalyzeRefused(refusal) => refusal.document(),
            RunCause::MonitorRefused(refusal) => refusal.document(),
            RunCause::Replay(failure) => failure.outcome.clone(),
            RunCause::Source(failure) => OutcomeDocument::new(operation, None, self.category())
                .with_diagnostics(vec![OutcomeDiagnostic::from_source_diagnostic(
                    &failure.diagnostic,
                )]),
            RunCause::Native(diagnostic) => {
                let projected = if let Some(cause) = diagnostic.identity_cause() {
                    let source = fsl_cst::diagnostic::source_refusal(
                        fsl_foundation::source::SourceReadRefusal {
                            cause,
                            error: fsl_foundation::source::SourceReadError {
                                source: diagnostic.source.clone(),
                                path: diagnostic.path.clone(),
                                region: None,
                                message: diagnostic.message.clone(),
                            },
                        },
                        0,
                    );
                    OutcomeDiagnostic::from_source_diagnostic(&source)
                } else {
                    OutcomeDiagnostic::new(
                        None,
                        diagnostic.code.as_str(),
                        Some(OutcomeLocus::Located {
                            file: diagnostic.path.clone(),
                            span: diagnostic.span,
                        }),
                        diagnostic.message.clone(),
                    )
                };
                OutcomeDocument::new(operation, None, self.category())
                    .with_diagnostics(vec![projected])
            }
            RunCause::ClauseRequest { member, .. } if member.starts_with("/call/arguments/") => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String(member.clone()),
                    )]))),
                ])
            }
            RunCause::Io { .. }
            | RunCause::Json(_)
            | RunCause::ClauseRequest { .. }
            | RunCause::TlCorrespondence { .. }
            | RunCause::TlClause { .. }
            | RunCause::TlNotYetImplemented { .. }
            | RunCause::TlFault(_)
            | RunCause::ClocksGivenTwice
            | RunCause::OutputGivenTwice
            | RunCause::OutputExists { .. }
            | RunCause::Output(_)
            | RunCause::ClauseOutput(_)
            | RunCause::Format
            | RunCause::Limit(_)
            | RunCause::Identifier { .. }
            | RunCause::CompleteSelection(_)
            | RunCause::ExtractionRequiresRun
            | RunCause::EmptyCompileProgram
            | RunCause::Libraries(_)
            | RunCause::ExtensionCatalog(_)
            | RunCause::CompleteRunSelection(_)
            | RunCause::AnalyzeRequest { .. }
            | RunCause::AnalyzeUniverse(_)
            | RunCause::AnalyzeFault
            | RunCause::MonitorFault
            | RunCause::MonitorCancelled
            | RunCause::ReplayRequest(_)
            | RunCause::ReplayBytes { .. }
            | RunCause::UnknownRequirementClaim { .. }
            | RunCause::TemporalCorrection(_)
            | RunCause::ReplayOutput(_)
            | RunCause::ItemsFault => OutcomeDocument::new(operation, None, self.category())
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    None,
                    self.cause.code().as_str(),
                    None,
                    self.outcome_message(),
                )]),
            RunCause::AnalyzePackage { cause, .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        cause.as_deref(),
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    ),
                ])
            }
            RunCause::AnalyzeNotYetImplemented { .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some("not-yet-implemented"),
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    ),
                ])
            }
        }
    }

    fn outcome_message(&self) -> String {
        match &self.cause {
            RunCause::Limit(LimitKind::RequestBytes { maximum }) => {
                format!("request_bytes limit {maximum} exceeded; raise --request-bytes")
            }
            RunCause::Limit(LimitKind::DependentBytes { maximum }) => {
                format!("dependent_bytes limit {maximum} exceeded; raise --dependent-bytes")
            }
            RunCause::Limit(LimitKind::FixedRequestBytes { maximum }) => {
                format!("request_bytes limit {maximum} exceeded")
            }
            RunCause::Limit(LimitKind::FixedDependentBytes { maximum }) => {
                format!("dependent_bytes limit {maximum} exceeded")
            }
            RunCause::Limit(LimitKind::ObservationDocumentBytes { maximum }) => {
                format!(
                    "observation.document_bytes limit {maximum} exceeded; raise --limit observation.document_bytes"
                )
            }
            RunCause::Limit(LimitKind::SelectedFiles {
                category,
                requested,
                remaining,
                maximum,
            }) => {
                format!(
                    "selected files limit {maximum} exceeded: {category:?} requested {requested}, remaining {remaining}"
                )
            }
            _ => self.to_string(),
        }
    }

    /// JSON error with original stage/code and available source/reference details.
    pub fn value(&self) -> std::result::Result<serde_json::Value, serde_json::Error> {
        output::error(self, false)
    }

    /// JSON error for an artifact command, retaining its exact request
    /// identity and refusal/incomplete status.
    pub fn artifact_value(&self) -> std::result::Result<serde_json::Value, serde_json::Error> {
        output::error(self, true)
    }
}

fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>> {
    let read = || -> io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        // Limits are internal constants or their checked remainder, well below u64.
        let ceiling = u64::try_from(limit).map_err(io::Error::other)?;
        file.take(ceiling.saturating_add(1))
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let bytes = read().map_err(|error| RunCause::Io {
        path: path.into(),
        error,
    })?;
    if bytes.len() > limit {
        return Err(RunCause::Limit(LimitKind::FileBytes));
    }
    Ok(bytes)
}

struct Intake<'a> {
    directory: &'a Path,
    remaining: usize,
    maximum: usize,
    dependent_limit: Option<DependentLimit>,
    prefetched_observations: std::collections::BTreeMap<String, Vec<u8>>,
    sources: Vec<Source>,
}

#[derive(Clone, Copy)]
enum DependentLimit {
    Fixed,
    CallerConfigured,
}

impl Intake<'_> {
    fn file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        if let Some(bytes) = self.prefetched_observations.remove(path) {
            return Ok(bytes);
        }
        self.bounded_file(path, ceiling)
    }

    fn observation_file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        if let Some(bytes) = self.prefetched_observations.remove(path) {
            if bytes.len() > ceiling {
                return Err(RunCause::Limit(LimitKind::ObservationDocumentBytes {
                    maximum: ceiling,
                }));
            }
            return Ok(bytes);
        }
        let remaining = self.remaining;
        let bytes =
            read_file(&self.directory.join(path), ceiling.min(remaining)).map_err(|cause| {
                match cause {
                    RunCause::Limit(LimitKind::FileBytes) if ceiling <= remaining => {
                        RunCause::Limit(LimitKind::ObservationDocumentBytes { maximum: ceiling })
                    }
                    RunCause::Limit(LimitKind::FileBytes) => match self.dependent_limit {
                        Some(DependentLimit::CallerConfigured) => {
                            RunCause::Limit(LimitKind::DependentBytes {
                                maximum: self.maximum,
                            })
                        }
                        Some(DependentLimit::Fixed) => {
                            RunCause::Limit(LimitKind::FixedDependentBytes {
                                maximum: self.maximum,
                            })
                        }
                        None => RunCause::Limit(LimitKind::FileBytes),
                    },
                    cause => cause,
                }
            })?;
        self.remaining -= bytes.len();
        Ok(bytes)
    }

    fn bounded_file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        let bytes = read_file(&self.directory.join(path), ceiling.min(self.remaining)).map_err(
            |cause| match cause {
                RunCause::Limit(LimitKind::FileBytes) => match self.dependent_limit {
                    Some(DependentLimit::CallerConfigured) => {
                        RunCause::Limit(LimitKind::DependentBytes {
                            maximum: self.maximum,
                        })
                    }
                    Some(DependentLimit::Fixed) => {
                        RunCause::Limit(LimitKind::FixedDependentBytes {
                            maximum: self.maximum,
                        })
                    }
                    None => RunCause::Limit(LimitKind::FileBytes),
                },
                cause => cause,
            },
        )?;
        self.remaining -= bytes.len();
        Ok(bytes)
    }

    fn source(
        &mut self,
        selected: &wire::SourceFile,
        request_member: &str,
    ) -> Result<FormalSource> {
        let identity = selected.identity.bind(request_member)?;
        let bytes = self.file(&selected.file, self.remaining)?;
        let source = Source::read_typed(identity.native, &selected.file, &bytes, self.maximum)
            .map_err(|refusal| {
                RunCause::Source(Box::new(SourceFailure {
                    native: fsl_foundation::diagnostic::source_refusal(
                        &refusal,
                        &bytes,
                        self.maximum,
                    ),
                    diagnostic: fsl_cst::diagnostic::source_refusal(*refusal, self.maximum),
                }))
            })?;
        self.sources.push(source.clone());
        Ok(FormalSource::new(source, identity.formal))
    }
}

/// FR-664: refuse a current-snapshot `parameters` member before reading the
/// program or model sources. The selected snapshot is cached for the normal
/// clause reader so it is still consumed once.
fn reject_current_parameters_before_sources(
    intake: &mut Intake<'_>,
    clause: Option<&wire::ClauseRun>,
) -> Result<()> {
    let Some(selection) = clause
        .and_then(|clause| clause.selection.clause.as_ref())
        .filter(|selection| selection.parameters.is_some())
    else {
        return Ok(());
    };
    let clause = clause.expect("the selection comes from the clause");
    let _observation_limits =
        fsl_replay::spine::ObservationLimitsWire::resolve(clause.observation_limits.as_ref())
            .map_err(|error| RunCause::ClauseRequest {
                member: error.member,
                reason: error.reason,
            })?;
    let file = clause
        .observations
        .get(&selection.observation)
        .ok_or_else(|| RunCause::ClauseRequest {
            member: selection.observation.clone(),
            reason: "observation key is not present".to_owned(),
        })?;
    let bytes = intake.file(file, intake.remaining)?;
    let document: serde_json::Value = serde_json::from_slice(&bytes).map_err(RunCause::Json)?;
    if document["format"] == "filament.state.snapshot/v1" && document["observation"] == "current" {
        return Err(RunCause::ClauseRequest {
            member: "parameters".to_owned(),
            reason: "forbidden for current snapshot".to_owned(),
        });
    }
    intake.prefetched_observations.insert(file.clone(), bytes);
    Ok(())
}

/// Read a native-run/1 job and execute it locally with fresh, bounded stage
/// state. A call carrying `requirements` settles its items with the linked
/// engines alone, each negotiated by `negotiate` (FR-961 "plain
/// `command::run` settles with the linked engines").
pub fn run(
    path: &Path,
    negotiate: &dyn Negotiate,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits(path, RunLimits::default(), negotiate)
}

/// Run with request members supplied beside the request file (FR-640).
pub fn run_with_overrides(
    path: &Path,
    overrides: &RequestOverrides<'_>,
    negotiate: &dyn Negotiate,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits_and_overrides(path, RunLimits::default(), overrides, negotiate)
}

/// Run with the caller's explicit request and dependent-file byte ceilings.
pub fn run_with_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits_and_overrides(path, limits, &RequestOverrides::default(), negotiate)
}

/// Run with explicit byte ceilings and request members supplied beside the
/// request file (FR-640): [`run_with_providers`] with no provider, the
/// default [`ProviderOptions`] and a handle that is never cancelled.
pub fn run_with_limits_and_overrides(
    path: &Path,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
    negotiate: &dyn Negotiate,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_configured(
        path,
        limits,
        overrides,
        Settlement {
            negotiate,
            providers: Vec::new(),
            options: &ProviderOptions::default(),
            cancel: &filament_exact::Cancel::new(),
        },
    )
}

/// FSL's run-side provider entry (FR-961): run the call the native-run/1
/// request at `path` names as FR-100 does and, when the call carries
/// `requirements` (FR-960), settle its requirement items against
/// `Engines::fsl()` with `providers` registered beside it, each item
/// negotiated by `negotiate` before any provider is called, and every
/// provider request carrying `options`' limits (FR-964). `cancel` governs
/// the settlement: once it fires no further request is sent and every
/// unsettled item settles `incomplete`. The call's own evaluation is the
/// same with and without `requirements`, `providers` and `options`.
///
/// # Errors
///
/// As [`run`]; and `invalid-request` naming `providers` when a provider
/// repeats a registered `BackendId`, before any stage runs.
pub fn run_with_providers(
    path: &Path,
    negotiate: &dyn Negotiate,
    providers: Vec<fsl_analyze::analyze::Engine>,
    options: &ProviderOptions,
    cancel: &filament_exact::Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_configured(
        path,
        RunLimits::default(),
        &RequestOverrides::default(),
        Settlement {
            negotiate,
            providers,
            options,
            cancel,
        },
    )
}

/// How a run settles its call's requirement items (FR-961).
struct Settlement<'a> {
    negotiate: &'a dyn Negotiate,
    providers: Vec<fsl_analyze::analyze::Engine>,
    options: &'a ProviderOptions,
    cancel: &'a filament_exact::Cancel,
}

fn run_configured(
    path: &Path,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
    settlement: Settlement<'_>,
) -> std::result::Result<RunResult, Box<RunError>> {
    let Settlement {
        negotiate,
        providers,
        options,
        cancel,
    } = settlement;
    // FR-961 step 2: the registry is fixed before any stage runs, so a
    // provider that cannot register refuses the request first.
    let engines = analyze::engines_with(providers, options).map_err(|cause| {
        Box::new(RunError {
            cause,
            sources: crate::Sources::default(),
        })
    })?;
    let settle = Settle {
        negotiate,
        engines: &engines,
        cancel,
    };
    // FR-644: a `native-monitor/1` request runs its record file as the batch
    // stream.
    let special = with_request_limits(path, limits.request_bytes, |directory, bytes| {
        if monitor::is_monitor_request(bytes) {
            monitor::run_bytes(directory, bytes, limits.dependent_bytes).map(Some)
        } else if replay::is_replay_request(bytes) {
            // FR-862: a `native-replay/1` request replays its FR-336 request.
            replay::run_bytes(directory, bytes, limits.dependent_bytes).map(Some)
        } else {
            if let Some(result) = version_argv(bytes)? {
                Ok(Some(result))
            } else if let Some(result) = check_argv(directory, bytes)? {
                Ok(Some(result))
            } else {
                items::run_bytes(directory, bytes, limits, overrides, &settle)
            }
        }
    })?;
    if let Some(result) = special {
        return Ok(result);
    }
    let prepared = prepare_run_with_overrides(path, limits, overrides)?;
    let sources = prepared.admitted_sources();
    execute_prepared(prepared, &settle).map_err(|cause| Box::new(RunError { cause, sources }))
}

/// FR-661: handle the driver's in-process `version` invocation used by the
/// command harness. The binary driver owns argv parsing at its process
/// boundary; this entry accepts the exact JSON adapter shape used by FSL's
/// tests and returns the same inventory document.
fn version_argv(bytes: &[u8]) -> Result<Option<RunResult>> {
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(arguments) = value.get("argv") else {
        return Ok(None);
    };
    let arguments: Vec<&str> = arguments
        .as_array()
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "argv".to_owned(),
            reason: "expected an array of strings".to_owned(),
        })?
        .iter()
        .map(|argument| {
            argument.as_str().ok_or_else(|| RunCause::ClauseRequest {
                member: "argv".to_owned(),
                reason: "expected an array of strings".to_owned(),
            })
        })
        .collect::<Result<_>>()?;
    if arguments.first().copied() != Some("version") {
        return Ok(None);
    }

    let lists = ["editions", "profiles", "families", "features", "consumers"];
    let mut format = None;
    let mut kind = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index] {
            "--format" if format.is_none() && index + 1 < arguments.len() => {
                format = Some(arguments[index + 1]);
                index += 2;
            }
            "--kind" if kind.is_none() && index + 1 < arguments.len() => {
                kind = Some(arguments[index + 1]);
                index += 2;
            }
            member => {
                return Err(RunCause::ClauseRequest {
                    member: member.to_owned(),
                    reason:
                        "version accepts --format json and --kind followed by one inventory list"
                            .to_owned(),
                });
            }
        }
    }
    if format.is_some_and(|format| format != "json") {
        return Err(RunCause::ClauseRequest {
            member: "format".to_owned(),
            reason: "version supports only json".to_owned(),
        });
    }
    if let Some(kind) = kind {
        if !lists.contains(&kind) {
            return Err(RunCause::ClauseRequest {
                member: kind.to_owned(),
                reason: format!(
                    "unknown inventory list; expected one of {}",
                    lists.join(", ")
                ),
            });
        }
    }

    let inventory = fsl_replay::build_inventory(&fsl_route::Registry::new());
    let mut document = serde_json::to_value(inventory).map_err(RunCause::Output)?;
    if let Some(kind) = kind {
        let object = document
            .as_object_mut()
            .expect("inventory serializes as object");
        object.retain(|name, _| name == "application" || name == "format" || name == kind);
    }
    Ok(Some(RunResult {
        exit_code: 0,
        value: NativeResult::version_document(document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    }))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionSetFile {
    format: String,
    #[serde(deserialize_with = "wire::extensions")]
    extensions: Vec<wire::Extension>,
}

/// FR-562: route the in-process `check --extensions` command surface to the
/// source-only check API used by FSL's integration harness.
fn check_argv(directory: &Path, bytes: &[u8]) -> Result<Option<RunResult>> {
    let value: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(arguments) = value.get("argv") else {
        return Ok(None);
    };
    let arguments = arguments
        .as_array()
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "argv".to_owned(),
            reason: "expected an array of strings".to_owned(),
        })?
        .iter()
        .map(|argument| {
            argument.as_str().ok_or_else(|| RunCause::ClauseRequest {
                member: "argv".to_owned(),
                reason: "expected an array of strings".to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if arguments.first().copied() != Some("check") {
        return Ok(None);
    }
    let (extensions, source) = match arguments.as_slice() {
        ["check", source] => (Vec::new(), *source),
        ["check", _authority, _name, source] => (Vec::new(), *source),
        ["check", "--extensions", set_path, _authority, _name, source] => {
            let set_bytes = read_file(&directory.join(set_path), TOTAL_BYTES)?;
            let set: ExtensionSetFile = serde_json::from_slice(&set_bytes)?;
            if set.format != "filament.extension-set/v1" {
                return Err(RunCause::ClauseRequest {
                    member: "format".to_owned(),
                    reason: "expected filament.extension-set/v1".to_owned(),
                });
            }
            let extensions = set
                .extensions
                .into_iter()
                .map(|extension| CheckExtensionInput {
                    authority: extension.authority,
                    identity: extension.identity,
                    file: directory.join(extension.file),
                    depends_on: extension
                        .depends_on
                        .into_iter()
                        .map(|dependency| (dependency.authority, dependency.identity))
                        .collect(),
                    capabilities: extension.capabilities,
                })
                .collect();
            (extensions, *source)
        }
        _ => {
            return Err(RunCause::ClauseRequest {
                member: "argv".to_owned(),
                reason: "check expects a source or --extensions <file> <authority> <name> <source>"
                    .to_owned(),
            });
        }
    };
    let source_path = Path::new(source);
    let source_path = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        directory.join(source_path)
    };
    let outcome = check_outcome(
        &source_path,
        &CheckInputs {
            extensions,
            ..CheckInputs::default()
        },
    );
    let exit_code = outcome.document().category().exit_code();
    let document = serde_json::to_value(outcome.document()).map_err(RunCause::Output)?;
    Ok(Some(RunResult {
        exit_code,
        value: NativeResult::check_document(document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    }))
}

/// Inspect a checked-package handoff and return the command's successful
/// `inspect` outcome. A refusal retains the handoff reader's typed error.
pub fn inspect_handoff(directory: &Path) -> fsl_replay::OutcomeDocument {
    use filament_semantic_value::catalog::Category;
    use fsl_replay::{Operation, OutcomeDiagnostic, OutcomeDocument, OutcomeLocus};

    match crate::checked_handoff::verify(directory) {
        Ok(_) => OutcomeDocument::new(Operation::Inspect, None, Category::Success),
        Err(error) => OutcomeDocument::new(Operation::Inspect, None, Category::Refusal)
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                None,
                error.code_name(),
                Some(OutcomeLocus::Artifact {
                    pointer: "/".to_owned(),
                }),
                error.to_string(),
            )]),
    }
}

/// Read and validate a request and its selected files without executing it.
pub fn prepare_run(
    path: &Path,
    limits: RunLimits,
) -> std::result::Result<PreparedRun, Box<RunError>> {
    prepare_run_with_overrides(path, limits, &RequestOverrides::default())
}

/// Read and validate a request, with members supplied beside the request
/// file, and its selected files without executing it (FR-640).
pub fn prepare_run_with_overrides(
    path: &Path,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<PreparedRun, Box<RunError>> {
    with_request_limits(path, limits.request_bytes, |directory, bytes| {
        prepare_run_bytes(directory, bytes, limits.dependent_bytes, overrides)
    })
}

/// FR-312's request reader, returning the exact typed input the spine runs.
/// A refusal retains its located command cause.
pub type ClauseRunRequestRefusal = Box<RunError>;

/// Read a closed clause-run request and every selected file exactly once.
pub fn read_clause_run_request(
    path: &Path,
) -> std::result::Result<fsl_replay::spine::ClauseRunRequest, ClauseRunRequestRefusal> {
    with_request(path, |directory, bytes| {
        let request: wire::Request = request(bytes)?;
        let clause = request
            .clause
            .as_ref()
            .ok_or_else(|| RunCause::ClauseRequest {
                member: "clause".to_owned(),
                reason: "missing clause run".to_owned(),
            })?;
        if request.call.is_some() {
            return Err(RunCause::ClauseRequest {
                member: "call".to_owned(),
                reason: "call and clause are mutually exclusive".to_owned(),
            });
        }
        let mut intake = intake(
            directory,
            FileCounts {
                programs: request.libraries.len().saturating_add(1),
                models: request.models.len(),
                observations: clause
                    .observations
                    .values()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                extensions: request.extensions.len(),
            },
        )?;
        reject_current_parameters_before_sources(&mut intake, request.clause.as_ref())?;
        let source = intake.source(&request.program.source, "program.source")?;
        require_complete_edition(source.source())?;
        clause::read(&mut intake, &request, clause, source)
    })
}

/// Compile a `1-draft` source-only job through the spine (FR-027).
/// Failures expose no partial artifact.
pub fn compile(path: &Path) -> std::result::Result<Vec<u8>, Box<RunError>> {
    compile_outcome(path).map(|outcome| outcome.package_bytes)
}

/// Compile once, retaining the actual core outcome document and package bytes.
pub fn compile_outcome(path: &Path) -> std::result::Result<CompileOutcome, Box<RunError>> {
    compile_outcome_inner(
        path,
        RunLimits::default(),
        &RequestOverrides::default(),
        false,
    )
}

/// Compile, taking request members supplied beside the request file (FR-640).
pub fn compile_with_overrides(
    path: &Path,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<Vec<u8>, Box<RunError>> {
    compile_outcome_with_overrides(path, overrides).map(|outcome| outcome.package_bytes)
}

/// Compile once, taking request members supplied beside the request file
/// (FR-640), retaining the core outcome document and package bytes.
pub fn compile_outcome_with_overrides(
    path: &Path,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<CompileOutcome, Box<RunError>> {
    compile_outcome_inner(path, RunLimits::default(), overrides, false)
}

/// Compile with caller-configured request/dependent byte ceilings and request
/// overrides, including stage limits (FR-869).
pub fn compile_outcome_with_limits_and_overrides(
    path: &Path,
    run_limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<CompileOutcome, Box<RunError>> {
    compile_outcome_inner(path, run_limits, overrides, true)
}

fn compile_outcome_inner(
    path: &Path,
    run_limits: RunLimits,
    overrides: &RequestOverrides<'_>,
    request_limit_is_configured: bool,
) -> std::result::Result<CompileOutcome, Box<RunError>> {
    let result = with_request_limits(path, run_limits.request_bytes, |directory, bytes| {
        let request: wire::CompileRequest = request_with_overrides(bytes, overrides)?;
        validate_extension_entries(&request.extensions)?;
        if let Some(handoff) = &request.handoff {
            validate_handoff_request(handoff, request.program.sources.len())?;
            let destination = directory.join(&handoff.directory);
            if destination.exists() {
                return Err(RunCause::ClauseRequest {
                    member: "handoff.directory".to_owned(),
                    reason: format!("{} already exists", destination.display()),
                });
            }
        }
        let output = match (&request.output, overrides.output) {
            (Some(_), Some(_)) => return Err(RunCause::OutputGivenTwice),
            (Some(member), None) => Some(directory.join(&member.package)),
            (None, supplied) => supplied.map(Path::to_path_buf),
        };
        if request.program.extraction {
            return Err(RunCause::ExtractionRequiresRun);
        }
        if request.program.sources.is_empty() {
            return Err(RunCause::EmptyCompileProgram);
        }
        let counts = FileCounts {
            // A library source is a program source a `1-draft` program
            // imports, and counts toward the same file limits (FR-027).
            programs: request
                .libraries
                .len()
                .saturating_add(request.program.sources.len()),
            models: request.models.len(),
            observations: 0,
            extensions: request.extensions.len(),
        };
        let mut intake = if request_limit_is_configured {
            intake_with_bytes(directory, counts, run_limits.dependent_bytes)?
        } else {
            intake_with_fixed_bytes(directory, counts, run_limits.dependent_bytes)?
        };
        // Every unit's edition is decided before any model is read: a
        // `1-draft` program takes domain packages only, and a request
        // selecting a native rule model for it refuses whatever state the
        // model files are in.
        let mut sources = Vec::with_capacity(request.program.sources.len());
        for (index, selection) in request.program.sources.iter().enumerate() {
            let source = intake.source(selection, &format!("program.sources[{index}].source"))?;
            require_complete_edition(source.source())?;
            sources.push(source);
        }
        let mut outcome = complete(&request, &mut intake, &sources, overrides.limits)?;
        outcome.sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        if let Some(handoff) = &request.handoff {
            let compiled =
                outcome
                    .handoff_compiled
                    .take()
                    .ok_or_else(|| RunCause::ClauseRequest {
                        member: "handoff".to_owned(),
                        reason: "requires a single program object".to_owned(),
                    })?;
            write_compile_handoff(
                directory,
                &request,
                handoff,
                &outcome.package_bytes,
                compiled,
                std::mem::take(&mut outcome.handoff_temporal_selections),
            )?;
        }
        if let Some(output) = output {
            output_file::write_new_file(&output, &outcome.package_bytes)?;
        }
        Ok(outcome)
    });
    if request_limit_is_configured {
        result
    } else {
        result.map_err(|mut error| {
            if let RunCause::Limit(LimitKind::RequestBytes { maximum }) = &error.cause {
                error.cause = RunCause::Limit(LimitKind::FixedRequestBytes { maximum: *maximum });
            }
            error
        })
    }
}

/// Reject malformed FR-665 handoff selections before opening selected inputs.
fn validate_handoff_request(handoff: &wire::Handoff, source_count: usize) -> Result<()> {
    use std::path::Component;

    let path = Path::new(&handoff.directory);
    if handoff.directory.is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RunCause::ClauseRequest {
            member: "handoff.directory".to_owned(),
            reason: "must name a directory beneath the compile request directory".to_owned(),
        });
    }
    if handoff.package_name.is_empty() {
        return Err(RunCause::ClauseRequest {
            member: "handoff.package_name".to_owned(),
            reason: "must not be empty".to_owned(),
        });
    }
    if source_count != 1 {
        return Err(RunCause::ClauseRequest {
            member: "handoff".to_owned(),
            reason: "requires exactly one program source".to_owned(),
        });
    }
    if handoff.native_join.is_none() {
        return Err(RunCause::ClauseRequest {
            member: "handoff.native_join".to_owned(),
            reason: "is required".to_owned(),
        });
    }
    decode_handoff_execution_points(handoff)?;
    Ok(())
}

fn decode_handoff_execution_points(
    handoff: &wire::Handoff,
) -> Result<fsl_replay::spine::StateClauseExecutionPoints> {
    let mut points = fsl_replay::spine::StateClauseExecutionPoints::new();
    for (name, value) in &handoff.execution_points {
        let point: filament_contract_model::ExecutionPoint = serde_json::from_value(value.clone())
            .map_err(|error| RunCause::ClauseRequest {
                member: format!("handoff.execution_points/{name}"),
                reason: error.to_string(),
            })?;
        points.insert(name.clone(), point);
    }
    Ok(points)
}

/// Write FR-665's checked-package handoff from the exact compilation result.
fn write_compile_handoff(
    directory: &Path,
    request: &wire::CompileRequest,
    handoff: &wire::Handoff,
    package_bytes: &[u8],
    compiled: crate::emit_checked_v2::Compiled,
    temporal_selections: Vec<crate::emit_checked_v2::ClockSelection>,
) -> Result<()> {
    let Some(native_join) = handoff.native_join.as_ref() else {
        return Err(RunCause::ClauseRequest {
            member: "handoff.native_join".to_owned(),
            reason: "is required".to_owned(),
        });
    };
    let source_path = directory.join(&request.program.sources[0].file);
    let models = request
        .models
        .iter()
        .map(|model| directory.join(&model.source.file))
        .collect();
    let libraries = request
        .libraries
        .iter()
        .map(|library| {
            (
                library.identity.clone(),
                directory.join(&library.source.file),
            )
        })
        .collect();
    let emit = crate::emit_checked_v2::Request {
        source: source_path,
        package_name: handoff.package_name.clone(),
        models,
        libraries,
        execution_points: None,
        temporal_selections,
        out: None,
    };
    let corpus = match handoff.corpus {
        wire::HandoffCorpus::None => crate::checked_handoff::Corpus::None,
        wire::HandoffCorpus::Standard => crate::checked_handoff::Corpus::Standard,
    };
    let writer_request = crate::checked_handoff::CheckedHandoffRequest {
        emit,
        corpus,
        native_join: fsl_package::NativeJoinLimits::new(
            native_join.work_units,
            native_join.storage,
        ),
    };
    crate::checked_handoff::write_compiled_from_command(
        &writer_request,
        &directory.join(&handoff.directory),
        compiled,
        package_bytes,
        &request.program.sources[0].file,
    )
    .map_err(|error| RunCause::ClauseRequest {
        member: "handoff".to_owned(),
        reason: error.to_string(),
    })?;
    Ok(())
}

/// Whether a request's selected model declares the one supported domain-package
/// format (ADR-012 section 9): the one reader of a model selection's wire
/// `format` field.
#[ix_trace_rs::string_edge]
fn is_domain_package(model: &wire::Model) -> bool {
    model.format == DOMAIN_PACKAGE_PROFILE
}

/// Refuse an edition the spine does not admit at the declared token.
#[ix_trace_rs::string_edge]
fn require_complete_edition(source: &Source) -> Result<()> {
    if let Some(declared) = fsl_cst::declared_edition(source.text()) {
        if declared.edition != fsl_cst::EDITION {
            return Err(RunCause::Native(fsl_foundation::diagnostic::error(
                source,
                Code::UnknownEdition,
                fsl_foundation::Phase::Profile,
                declared.span.start,
                declared.span.end,
                format!(
                    "{} declares edition {:?}; this command reads {:?}",
                    source.path(),
                    declared.edition,
                    fsl_cst::EDITION
                ),
            )));
        }
    }
    Ok(())
}

/// Spine-compile a `1-draft` program. Each selected model is a domain
/// package document (format [`DOMAIN_PACKAGE_PROFILE`], the Semantic IR
/// contract it is admitted under), read under its source reference and handed
/// to the spine as FR-056's package input; the program's `model`
/// declarations select from it. Native rule models and clause bindings
/// select native constructs, which a complete-V1 program does not take. The
/// program's `document` and `formal_revision` were validated when intake
/// read it; the v2 wire has no member for them, so they do not reach the
/// bytes (FR-027 Outputs).
fn complete(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    supplied_limits: Option<&fsl_replay::spine::SpineLimits>,
) -> Result<CompileOutcome> {
    let limits = supplied_limits.copied().unwrap_or_default();
    if sources.len() == 1 {
        return complete_single(request, intake, &sources[0], limits);
    }
    complete_units(request, intake, sources, limits)
}

fn complete_single(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    source: &FormalSource,
    limits: fsl_replay::spine::SpineLimits,
) -> Result<CompileOutcome> {
    complete_sources(request, intake, std::slice::from_ref(source), false, limits)
}

/// Compile FR-480's ordered program-unit list as one checked package.
fn complete_units(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    limits: fsl_replay::spine::SpineLimits,
) -> Result<CompileOutcome> {
    complete_sources(request, intake, sources, true, limits)
}

/// Shared request, dependency, parsing, and emission pipeline for one source
/// and an ordered source-unit list. Only the spine's single-unit versus
/// multi-unit entry points vary by `unit_list`.
fn complete_sources(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    unit_list: bool,
    limits: fsl_replay::spine::SpineLimits,
) -> Result<CompileOutcome> {
    check_sources(
        request,
        intake,
        sources,
        unit_list,
        limits,
        None,
        None,
        |checked, selections, points, packages, _, libraries, models, inventory, failure| {
            let cancel = filament_exact::Cancel::new();
            let result = fsl_replay::spine::package_with_temporal_selections(
                &checked,
                selections,
                limits.package,
                &cancel,
            );
            let document = fsl_replay::OutcomeDocument::from_package(&result);
            let emitted = result.map_err(failure)?.into_value();
            let package_bytes = emitted.package().bytes().to_vec();
            let handoff_temporal_selections = if request.handoff.is_some() {
                handoff_clock_selections(&checked, selections)
            } else {
                Vec::new()
            };
            let handoff_compiled =
                request
                    .handoff
                    .as_ref()
                    .map(|_| crate::emit_checked_v2::Compiled {
                        source: sources[0].source().text().as_bytes().to_vec(),
                        models,
                        libraries,
                        packages,
                        points,
                        checked,
                        inventory: inventory
                            .expect("handoff inventory is captured for this request"),
                        emit: limits.package.emit(),
                        emitted,
                    });
            Ok(CompileOutcome {
                package_bytes,
                document,
                handoff_compiled,
                handoff_temporal_selections,
                sources: crate::Sources::default(),
            })
        },
    )
}

/// Rebuild the emit command's clause-name clock selections from the exact
/// checked declarations selected by FR-640's clock-name map.
fn handoff_clock_selections(
    checked: &fsl_replay::spine::CheckedUnit,
    selections: &[fsl_package::TemporalClockSelection],
) -> Vec<crate::emit_checked_v2::ClockSelection> {
    selections
        .iter()
        .filter_map(|selection| {
            let name = checked
                .package()
                .graph()
                .temporal_clauses()
                .iter()
                .find(|node| node.clause().span() == selection.declaration_span)
                .map(|node| node.clause().name().to_owned())
                .or_else(|| {
                    checked
                        .package()
                        .graph()
                        .protocols()
                        .iter()
                        .flat_map(|protocol| &protocol.obligations.compensations)
                        .find(|template| template.span == selection.declaration_span)
                        .map(|template| template.name.clone())
                })?;
            Some(crate::emit_checked_v2::ClockSelection {
                clause: name,
                clock: selection.clock.clone(),
            })
        })
        .collect()
}

/// The pipeline of [`complete_sources`] through S4: the checked unit is
/// handed to `finish` with the admitted domain packages, the library
/// sources and the map from a spine failure to its located cause.
fn check_sources<T>(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    unit_list: bool,
    limits: fsl_replay::spine::SpineLimits,
    _integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    extension_catalog_override: Option<fsl_semantics::library::bundle::DefinitionCatalog>,
    finish: impl FnOnce(
        fsl_replay::spine::CheckedUnit,
        &[fsl_replay::spine::TemporalClockSelection],
        fsl_replay::spine::StateClauseExecutionPoints,
        fsl_semantics::model::intake::PackageInput,
        Vec<&FormalSource>,
        Vec<fsl_replay::spine::SuppliedLibrary>,
        Vec<Vec<u8>>,
        Option<
            std::result::Result<
                filament_contract_model::source_obligation::SourceInventoryOutcome,
                String,
            >,
        >,
        &dyn Fn(fsl_replay::spine::FrontEndFailure) -> RunCause,
    ) -> Result<T>,
) -> Result<T> {
    if request.models.iter().any(|model| !is_domain_package(model)) {
        return Err(RunCause::CompleteSelection(CompleteSelection::NativeModels));
    }
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = fsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| RunCause::ClauseRequest {
        member: "models".to_owned(),
        reason: refusal.detail,
    })?;
    let libraries = library_sources(request, intake)?;
    // FR-660: read definitions after the program and library sources, under
    // the same aggregate dependent-file ceiling.
    let extension_catalog = match extension_catalog_override {
        Some(catalog) => catalog,
        None => extension_catalog(&request.extensions, intake)?,
    };
    if request.handoff.is_some() && unit_list {
        return Err(RunCause::ClauseRequest {
            member: "handoff".to_owned(),
            reason: "requires a single program object".to_owned(),
        });
    }
    let handoff_models = documents
        .iter()
        .map(|document| document.source().text().as_bytes().to_vec())
        .collect();
    let handoff_libraries = libraries
        .iter()
        .map(|(library, read)| fsl_replay::spine::SuppliedLibrary {
            identity: library.identity.clone(),
            source: read.source().identity().clone(),
            path: read.source().path().to_owned(),
            bytes: read.source().text().as_bytes().to_vec(),
        })
        .collect();
    let execution_points = request
        .handoff
        .as_ref()
        .map(decode_handoff_execution_points)
        .transpose()?
        .unwrap_or_default();
    let program_sources = sources.iter().map(FormalSource::source).collect::<Vec<_>>();
    let first_source = program_sources[0];
    let spine_failure = |refusal: fsl_replay::spine::CompileRefusal| {
        let located = refusal
            .region()
            .and_then(|region| {
                program_sources
                    .iter()
                    .copied()
                    .chain(libraries.iter().map(|(_, read)| read.source()))
                    .find(|candidate| candidate.reference() == region.source())
            })
            .unwrap_or(first_source);
        RunCause::Spine(Box::new(SpineFailure {
            source: located.identity().clone(),
            path: located.path().to_owned(),
            span: refusal.region().and_then(|region| located.render(region)),
            refusal,
        }))
    };
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
                    .clone()
                    .map(|value| (library.identity.clone(), value))
            }))
        })
        .map_err(|refusal| {
            spine_failure(fsl_replay::spine::CompileRefusal::DependencyInput(refusal))
        })?;
    let parse_requests: Vec<_> = program_sources
        .iter()
        .map(|source| fsl_replay::spine::ParseRequest {
            source: source.identity(),
            path: source.path(),
            bytes: source.text().as_bytes(),
        })
        .collect();
    if unit_list {
        fsl_replay::spine::preflight_unit_list(&parse_requests, &dependencies)
            .map_err(|refusal| spine_failure(*refusal))?;
    }
    let cancel = filament_exact::Cancel::new();
    let failure = |failure| match fsl_replay::spine::refusal_or_fault(failure) {
        Ok(refusal) => spine_failure(*refusal),
        Err(fault) => RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault))),
    };
    let parsed = parse_requests
        .iter()
        .map(|parse_request| {
            fsl_replay::spine::parse_selecting_extensions(
                parse_request,
                fsl_replay::spine::ProfileCatalogInput {
                    catalog: &extension_catalog,
                    limits: fsl_semantics::library::bundle::PackageLimits::default(),
                },
                limits.source,
                &cancel,
            )
            .map(|stage| stage.into_value())
            .map_err(failure)
        })
        .collect::<Result<Vec<_>>>()?;
    let check_selections: Vec<_> = parsed
        .iter()
        .flat_map(|unit| request.clocks.check_selections(unit))
        .collect();
    let mut handoff_inventory = None;
    let (checked, package_selections) = if unit_list {
        let admitted = fsl_replay::spine::admit_parsed_unit_list(parsed, &dependencies)
            .map_err(|refusal| spine_failure(*refusal))?;
        let models = fsl_replay::spine::select_units(
            &admitted,
            &packages,
            limits.model,
            limits.source,
            &cancel,
        )
        .map_err(failure)?
        .into_value();
        let checked = fsl_replay::spine::check_units_with_refinement_internals(
            &admitted,
            package_name(canonical_unit_source(sources).source().identity()),
            fsl_replay::spine::SourceInputs {
                refinement_internals: request.refinement_internals.clone(),
                temporal_selections: &check_selections,
                clocks: Some(&request.clocks),
            },
            &models,
            &dependencies,
            limits,
            &cancel,
        )
        .map_err(failure)?
        .into_value();
        let checked = checked
            .into_iter()
            .next()
            .ok_or(RunCause::EmptyCompileProgram)?;
        let units: Vec<_> = admitted.units().iter().collect();
        let selections = request
            .clocks
            .package_selections(&checked, &units)
            .map_err(|refusal| spine_failure(*refusal))?;
        (checked, selections)
    } else {
        let parsed = parsed
            .into_iter()
            .next()
            .ok_or(RunCause::EmptyCompileProgram)?;
        let models = fsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
            .map_err(failure)?
            .into_value();
        let checked_result = fsl_replay::spine::check_with_execution_points_and_source_inputs(
            &parsed,
            request.handoff.as_ref().map_or_else(
                || package_name(first_source.identity()),
                |handoff| handoff.package_name.clone(),
            ),
            &execution_points,
            fsl_replay::spine::SourceInputs {
                refinement_internals: request.refinement_internals.clone(),
                temporal_selections: &check_selections,
                clocks: Some(&request.clocks),
            },
            &models,
            &dependencies,
            limits,
            &cancel,
        );
        if request.handoff.is_some() {
            handoff_inventory = Some(
                fsl_replay::source_inventory::fsl_inventory(
                    Some(&parsed),
                    &checked_result,
                    sources[0].identity(),
                )
                .map_err(|refusal| refusal.to_string()),
            );
        }
        let checked = checked_result.map_err(failure)?.into_value();
        let selections = request
            .clocks
            .package_selections(&checked, &[&parsed])
            .map_err(|refusal| spine_failure(*refusal))?;
        (checked, selections)
    };
    finish(
        checked,
        &package_selections,
        execution_points,
        packages,
        libraries.iter().map(|(_, source)| source).collect(),
        handoff_libraries,
        handoff_models,
        handoff_inventory,
        &failure,
    )
}

/// The request's `libraries`, each with its source read under its caller-named
/// identity (FR-027, ADR-015 D-1). An empty identity refuses as
/// `invalid-request` before any library file is read.
fn library_sources<'r>(
    request: &'r wire::CompileRequest,
    intake: &mut Intake<'_>,
) -> Result<Vec<(&'r wire::Library, FormalSource)>> {
    for library in &request.libraries {
        if library.identity.is_empty() {
            return Err(RunCause::Libraries(LibrarySelection::EmptyIdentity));
        }
    }
    request
        .libraries
        .iter()
        .enumerate()
        .map(|(index, library)| {
            Ok((
                library,
                intake.source(&library.source, &format!("libraries[{index}].source"))?,
            ))
        })
        .collect()
}

/// Read the request's extension documents and bind them to a reader-owned
/// definition catalog (FR-660).
fn extension_catalog(
    extensions: &[wire::Extension],
    intake: &mut Intake<'_>,
) -> Result<fsl_semantics::library::bundle::DefinitionCatalog> {
    use fsl_foundation::selection::DefinitionRef;
    use fsl_semantics::library::bundle::{CapabilityId, ExtensionDefinitionSource};

    validate_extension_entries(extensions)?;

    let mut definitions = Vec::with_capacity(extensions.len());
    for extension in extensions {
        let dependencies = extension
            .depends_on
            .iter()
            .map(|dependency| {
                DefinitionRef::new(dependency.authority.clone(), dependency.identity.clone())
                    .map_err(|error| RunCause::ClauseRequest {
                        member: "extensions.depends_on".to_owned(),
                        reason: format!("{error:?}"),
                    })
            })
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        let capabilities = extension
            .capabilities
            .iter()
            .map(|capability| {
                CapabilityId::complete(capability)
                    .map_err(|error| RunCause::ExtensionCatalog(error.to_string()))
            })
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        definitions.push(ExtensionDefinitionSource {
            authority: extension.authority.clone(),
            identity: extension.identity.clone(),
            exact_bytes: intake.file(&extension.file, intake.remaining)?,
            dependencies,
            capabilities,
        });
    }
    fsl_semantics::library::bundle::DefinitionCatalog::read_extensions(definitions)
        .map_err(|error| RunCause::ExtensionCatalog(error.to_string()))
}

/// Validate extension labels and logical identity uniqueness before intake
/// reads any program, model, library or extension document.
fn validate_extension_entries(extensions: &[wire::Extension]) -> Result<()> {
    let mut identities = std::collections::BTreeMap::new();
    for (index, extension) in extensions.iter().enumerate() {
        for (member, value) in [
            ("authority", extension.authority.as_str()),
            ("identity", extension.identity.as_str()),
            ("file", extension.file.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(RunCause::ClauseRequest {
                    member: format!("extensions[{index}].{member}"),
                    reason: "must be a non-blank string".to_owned(),
                });
            }
        }
        let key = (extension.authority.as_str(), extension.identity.as_str());
        if let Some(previous) = identities.insert(key, index) {
            return Err(RunCause::ExtensionCatalog(format!(
                "conflicting-definition: extensions[{previous}] and extensions[{index}] both name {}:{}",
                extension.authority, extension.identity
            )));
        }
        for dependency in &extension.depends_on {
            if dependency.authority.trim().is_empty() || dependency.identity.trim().is_empty() {
                return Err(RunCause::ClauseRequest {
                    member: format!("extensions[{index}].depends_on"),
                    reason: "authority and identity must be non-blank strings".to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// A request selection a complete-V1 (`1-draft`) program does not take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompleteSelection {
    /// The request selects a model source in a native rule-model format.
    NativeModels,
}

impl std::fmt::Display for CompleteSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NativeModels => "native rule-model sources",
        })
    }
}

/// Why a `1-draft` request's `libraries` refuse (FR-027).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LibrarySelection {
    /// A library has an empty identity.
    EmptyIdentity,
}

impl std::fmt::Display for LibrarySelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a library has an empty identity")
    }
}

fn with_request<T>(
    path: &Path,
    action: impl FnOnce(&Path, &[u8]) -> Result<T>,
) -> std::result::Result<T, Box<RunError>> {
    with_request_limits(path, REQUEST_BYTES, action).map_err(|mut error| {
        if let RunCause::Limit(LimitKind::RequestBytes { maximum }) = &error.cause {
            error.cause = RunCause::Limit(LimitKind::FixedRequestBytes { maximum: *maximum });
        }
        error
    })
}

fn with_request_limits<T>(
    path: &Path,
    request_bytes: usize,
    action: impl FnOnce(&Path, &[u8]) -> Result<T>,
) -> std::result::Result<T, Box<RunError>> {
    let bytes = read_file(path, request_bytes).map_err(|cause| {
        let cause = match cause {
            RunCause::Limit(LimitKind::FileBytes) => RunCause::Limit(LimitKind::RequestBytes {
                maximum: request_bytes,
            }),
            cause => cause,
        };
        Box::new(RunError {
            cause,
            sources: crate::Sources::default(),
        })
    })?;
    action(path.parent().unwrap_or(Path::new(".")), &bytes).map_err(|cause| {
        Box::new(RunError {
            cause,
            sources: crate::Sources::default(),
        })
    })
}

/// Request settings a caller supplies beside the request file rather than
/// inside it. Each request member is read exactly as its in-file counterpart
/// would be (FR-640); `limits` configures compile stages (FR-869).
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestOverrides<'a> {
    /// The `clocks` object, as the driver's `--clocks` file carries it. A request
    /// that also carries `clocks` is refused `invalid-request` at stage `request`.
    pub clocks: Option<&'a serde_json::value::RawValue>,
    /// The file the checked package is written to, as the driver's `--output`
    /// names it (FR-810). Only a compile reads it; a request that also carries
    /// `output` is refused `invalid-request` at stage `request`.
    pub output: Option<&'a Path>,
    /// Compile-stage limits supplied by the caller (FR-869). `None` uses the
    /// published defaults.
    pub limits: Option<&'a fsl_replay::spine::SpineLimits>,
}

fn request_with_overrides<T: wire::ClocksRequest>(
    bytes: &[u8],
    overrides: &RequestOverrides<'_>,
) -> Result<T> {
    let mut request: T = request(bytes)?;
    if let Some(supplied) = overrides.clocks {
        let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
        let members: std::collections::BTreeMap<String, &serde_json::value::RawValue> =
            serde_json::from_str(envelope.request.get())?;
        if members.contains_key("clocks") {
            return Err(RunCause::ClocksGivenTwice);
        }
        *request.clocks_mut() = wire::read_clocks(supplied)?;
    }
    Ok(request)
}

fn request<T: wire::RequestKind>(bytes: &[u8]) -> Result<T> {
    let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
    let analyze = fsl_foundation::wire_format::WireFormat::AnalyzeRequest.as_str();
    if (envelope.format == analyze) != (T::FORMAT.as_str() == analyze) {
        // FR-620: an analyze document handed to another reader, or another
        // document handed to the analyze reader, is an invalid request.
        return Err(RunCause::AnalyzeRequest {
            member: "format".to_owned(),
            reason: format!("{} is not read here", envelope.format),
        });
    }
    if envelope.format != T::FORMAT.as_str() {
        return Err(RunCause::Format);
    }
    let Object(request) = serde_json::from_str(envelope.request.get())?;
    Ok(request)
}

fn check_file_count(counts: FileCounts) -> Result<()> {
    let mut remaining_files = FILES;
    for (category, requested) in [
        (FileCategory::Program, counts.programs),
        (FileCategory::Model, counts.models),
        (FileCategory::Observation, counts.observations),
        (FileCategory::Extension, counts.extensions),
    ] {
        remaining_files = remaining_files
            .checked_sub(requested)
            .ok_or(RunCause::Limit(LimitKind::SelectedFiles {
                category,
                requested,
                remaining: remaining_files,
                maximum: FILES,
            }))?;
    }
    Ok(())
}

fn intake(directory: &Path, counts: FileCounts) -> Result<Intake<'_>> {
    check_file_count(counts)?;
    Ok(Intake {
        directory,
        remaining: TOTAL_BYTES,
        maximum: TOTAL_BYTES,
        dependent_limit: None,
        prefetched_observations: std::collections::BTreeMap::new(),
        sources: Vec::new(),
    })
}

fn intake_with_bytes(directory: &Path, counts: FileCounts, maximum: usize) -> Result<Intake<'_>> {
    check_file_count(counts)?;
    Ok(Intake {
        directory,
        remaining: maximum,
        maximum,
        dependent_limit: Some(DependentLimit::CallerConfigured),
        prefetched_observations: std::collections::BTreeMap::new(),
        sources: Vec::new(),
    })
}

fn intake_with_fixed_bytes(
    directory: &Path,
    counts: FileCounts,
    maximum: usize,
) -> Result<Intake<'_>> {
    check_file_count(counts)?;
    Ok(Intake {
        directory,
        remaining: maximum,
        maximum,
        dependent_limit: Some(DependentLimit::Fixed),
        prefetched_observations: std::collections::BTreeMap::new(),
        sources: Vec::new(),
    })
}

fn prepare_run_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    overrides: &RequestOverrides<'_>,
) -> Result<PreparedRun> {
    let request: wire::Request = request_with_overrides(bytes, overrides)?;
    validate_extension_entries(&request.extensions)?;
    // FR-931: a malformed `tl_lowering` member refuses before any source
    // file is read.
    if let Some(member) = &request.tl_lowering {
        tl::decode(member)?;
    }
    if request.items.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "items".to_owned(),
            reason: "items is handled by the multi-selection runner".to_owned(),
        });
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            // A library source is a program source a `1-draft` program
            // imports, and counts toward the same file limits (FR-100,
            // mirroring FR-027).
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: request.clause.as_ref().map_or(0, |clause| {
                clause
                    .observations
                    .values()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            }) + usize::from(
                request
                    .call
                    .as_ref()
                    .is_some_and(|call| call.snapshot.is_some()),
            ),
            extensions: request.extensions.len(),
        },
        dependent_bytes,
    )?;
    reject_current_parameters_before_sources(&mut intake, request.clause.as_ref())?;
    // The program source is read exactly once; its declared edition decides
    // the runner before any model or library source is read (FR-100).
    let source = intake.source(&request.program.source, "program.source")?;
    require_complete_edition(source.source())?;
    prepare_complete_run(&mut intake, &request, source)
}

/// FR-931: prepare a `tl_lowering` request. Its clause is read through
/// FR-312's clause-run reader as a `temporal` selection over the trace to
/// cross-evaluate (or an empty one), so the compile, the trace's admission
/// and every refusal before evaluation are the clause run's own.
fn prepare_lowering(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    member: &serde_json::Value,
    source: FormalSource,
) -> Result<PreparedRun> {
    if request.call.is_some() || request.clause.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "tl_lowering".to_owned(),
            reason: "tl_lowering, call and clause are mutually exclusive".to_owned(),
        });
    }
    let lowering = tl::decode(member)?;
    if matches!(lowering.cross, Some(tl::CrossEvaluate::Initial)) {
        return Err(RunCause::TlNotYetImplemented {
            member: "tl_lowering.cross_evaluate.initial",
        });
    }
    let clause: wire::ClauseRun = serde_json::from_value(tl::selection(&lowering))?;
    let text = source.source().text().to_owned();
    let run = clause::read(intake, request, &clause, source)?;
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(PreparedRun::TlLowering(Box::new(PreparedLowering {
        request: run,
        lowering,
        source: text,
        sources,
    })))
}

/// Prepare FR-100's function or clause selection without executing a stage.
/// `source` is the program source, already read by bounded intake.
fn prepare_complete_run(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    source: FormalSource,
) -> Result<PreparedRun> {
    if let Some(member) = &request.tl_lowering {
        return prepare_lowering(intake, request, member, source);
    }
    if let Some(clause) = &request.clause {
        if request.call.is_some() {
            return Err(RunCause::ClauseRequest {
                member: "call".to_owned(),
                reason: "call and clause are mutually exclusive".to_owned(),
            });
        }
        return clause::read(intake, request, clause, source)
            .map(|request| PreparedRun::Clause(Box::new(request)));
    }
    if request.models.iter().any(|model| !is_domain_package(model)) {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::NativeModel,
        ));
    }
    let Some(call) = &request.call else {
        return Err(RunCause::CompleteRunSelection(CompleteRunSelection::NoCall));
    };
    // FR-960: a malformed `requirements` refuses before any model,
    // library or compile is read.
    let requirements = call
        .requirements
        .as_ref()
        .map(requirements::read)
        .transpose()?;
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = fsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| RunCause::ClauseRequest {
        member: "models".to_owned(),
        reason: refusal.detail,
    })?;
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
        .collect::<Result<Vec<(&wire::Library, FormalSource)>>>()?;
    // FR-660: the same request-owned catalog is available to run compilation.
    let extension_catalog = Some(extension_catalog(&request.extensions, intake)?);
    let dependencies =
        fsl_replay::spine::DependencyInput::new(libraries.iter().map(|(library, read)| {
            fsl_replay::spine::SuppliedLibrary {
                identity: library.identity.clone(),
                source: read.source().identity().clone(),
                path: read.source().path().to_owned(),
                bytes: read.source().text().as_bytes().to_vec(),
            }
        }))
        .map_err(|refusal| {
            spine_run_failure(
                &source,
                libraries.iter().map(|(_, source)| source),
                fsl_replay::spine::CompileRefusal::DependencyInput(refusal),
            )
        })?;
    // FR-650: a `value` is a JSON integer in the signed 64-bit range or a JSON
    // object; the run reads it against its parameter's declared type.
    let wire_arguments = call
        .arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            if !(argument.value.is_object() || argument.value.as_i64().is_some()) {
                return Err(RunCause::ClauseRequest {
                    member: format!("/call/arguments/{index}/value"),
                    reason: "expected a JSON integer in the signed 64-bit range or an object"
                        .to_owned(),
                });
            }
            Ok(fsl_replay::spine::WireArgument {
                parameter: argument.parameter.clone(),
                value: argument.value.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut spine_call = fsl_replay::spine::Call::new(
        call.function.clone(),
        Vec::new(),
        call_accounting(&call.accounting),
        request.limits.supplied.unwrap_or_default(),
    );
    spine_call.wire_arguments = wire_arguments;
    if let Some(file) = &call.snapshot {
        let limits = fsl_replay::spine::ObservationLimitsWire::resolve(None).map_err(|error| {
            RunCause::ClauseRequest {
                member: error.member,
                reason: error.reason,
            }
        })?;
        let bytes = intake.file(file, intake.remaining)?;
        let observation = fsl_replay::spine::SuppliedObservation::from_document("snapshot", bytes)
            .map_err(|error| RunCause::ClauseRequest {
                member: error.member,
                reason: error.reason,
            })?;
        spine_call.snapshot = Some(fsl_replay::spine::CallSnapshot {
            observation,
            limits,
        });
    }
    spine_call.clocks = request.clocks.clone();
    Ok(PreparedRun::Function(PreparedFunctionRun {
        data: Box::new(FunctionRunData {
            source,
            models: documents,
            libraries: libraries.into_iter().map(|(_, source)| source).collect(),
            packages,
            dependencies,
            extension_catalog,
            call: spine_call,
            requirements,
        }),
    }))
}

/// A prepared FR-931 lowering: the valuation run of its clause and what the
/// record is built from.
pub struct PreparedLowering {
    request: fsl_replay::spine::ClauseRunRequest,
    lowering: tl::TlLowering,
    source: String,
    sources: crate::Sources,
}

/// Execute a prepared FR-931 TL lowering and retain the standard run error
/// envelope used by the command API.
///
/// Request and dependent-file limits are enforced while constructing the
/// `PreparedLowering` through [`prepare_run`] or
/// [`prepare_run_with_overrides`]. This function executes only that admitted
/// value.
pub fn execute_tl_lowering(
    prepared: PreparedLowering,
) -> std::result::Result<RunResult, Box<RunError>> {
    let sources = prepared.sources.clone();
    execute_lowering(prepared).map_err(|cause| Box::new(RunError { cause, sources }))
}

fn execute_lowering(prepared: PreparedLowering) -> Result<RunResult> {
    let PreparedLowering {
        request,
        lowering,
        source,
        sources,
    } = prepared;
    let selection = request.selection.clone();
    let report = fsl_replay::spine::run_temporal_valuation(request).map_err(|error| {
        RunCause::ClauseRequest {
            member: "tl_lowering".to_owned(),
            reason: error.to_string(),
        }
    })?;
    let valuation = match &report.disposition {
        fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) => valuation,
        fsl_replay::spine::ClauseDisposition::MissingName { name } => {
            return Err(RunCause::TlClause { name: name.clone() });
        }
        _ => return output::clause_report(report, &selection),
    };
    let record = tl::lower(valuation, &source, &lowering).map_err(RunCause::TlFault)?;
    Ok(RunResult {
        exit_code: record.exit_code,
        value: NativeResult::lowering_document(tl::document(record.record)),
        outcome: None,
        requirements: None,
        sources,
    })
}

fn execute_prepared(prepared: PreparedRun, settle: &settle::Settle<'_>) -> Result<RunResult> {
    let prepared = match prepared {
        PreparedRun::Function(prepared) => prepared,
        PreparedRun::TlLowering(prepared) => return execute_lowering(*prepared),
        PreparedRun::Clause(request) => {
            let selection = request.selection.clone();
            let report = fsl_replay::spine::run_clause(*request).map_err(|error| {
                RunCause::ClauseRequest {
                    member: "clause".to_owned(),
                    reason: error.to_string(),
                }
            })?;
            return output::clause_report(report, &selection);
        }
    };
    let prepared = *prepared.data;
    let source = &prepared.source;
    let compile_failure = |refusal: Box<fsl_replay::spine::RunRefusal>| match *refusal {
        fsl_replay::spine::RunRefusal::Compile(compile_refusal) => {
            spine_run_failure(source, prepared.libraries.iter(), *compile_refusal)
        }
        other => RunCause::SpineRun(Box::new(other)),
    };
    // The call runs under a handle that is never cancelled: `settle.cancel`
    // governs only the settlement of its items (FR-961), so the call's own
    // evaluation is the same with and without them.
    let call_cancel = filament_exact::Cancel::new();
    let objects = fsl_semantics::model::object_environment::ObjectEnvironment::default();
    let run = |catalog: Option<&fsl_semantics::library::bundle::DefinitionCatalog>| match catalog {
        Some(catalog) => fsl_replay::spine::run_with_profile_catalog(
            source.source().identity().clone(),
            package_name(source.source().identity()),
            source.source().path(),
            source.source().text().as_bytes(),
            &prepared.packages,
            &prepared.dependencies,
            fsl_replay::spine::SpineLimits::default(),
            &prepared.call,
            &objects,
            fsl_replay::spine::ProfileCatalogInput {
                catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            },
            &call_cancel,
        ),
        None => fsl_replay::spine::run(
            source.source().identity().clone(),
            package_name(source.source().identity()),
            source.source().path(),
            source.source().text().as_bytes(),
            &prepared.packages,
            &prepared.dependencies,
            fsl_replay::spine::SpineLimits::default(),
            &prepared.call,
            &objects,
            &call_cancel,
        ),
    };
    let Some(selection) = &prepared.requirements else {
        // FR-960: a call without `requirements` settles no item, and its run
        // is exactly FR-100's.
        let outcome = run(prepared.extension_catalog.as_ref()).map_err(compile_failure)?;
        let mut result = output::spine_run_result(source, &prepared.call.function, outcome, None)?;
        result.sources = crate::Sources::from_sources(
            std::iter::once(source.source().clone())
                .chain(prepared.models.iter().map(|model| model.source().clone()))
                .chain(
                    prepared
                        .libraries
                        .iter()
                        .map(|library| library.source().clone()),
                ),
        );
        return Ok(result);
    };
    // FR-961 step 3 to 5: the call runs through FR-395's
    // `run_with_requirements`, and its items are settled on the same compile.
    let (ran, settled) = fsl_replay::run_with_requirements_reading_and_profile_catalog(
        &fsl_replay::RequirementsRun {
            source: source.source().identity().clone(),
            package_name: package_name(source.source().identity()),
            path: source.source().path(),
            bytes: source.source().text().as_bytes(),
            packages: &prepared.packages,
            dependencies: &prepared.dependencies,
            limits: fsl_replay::spine::SpineLimits::default(),
            call: &prepared.call,
            objects: &objects,
            cancel: &call_cancel,
        },
        prepared
            .extension_catalog
            .as_ref()
            .map(|catalog| fsl_replay::spine::ProfileCatalogInput {
                catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            }),
        |checked, members| {
            settle::settle_call(checked, members, selection, &prepared.packages, settle)
        },
    )
    .map_err(compile_failure)?;
    let (outcome, _) = ran.into_parts();
    let mut result =
        output::spine_run_result(source, &prepared.call.function, outcome, Some(settled?))?;
    result.sources = crate::Sources::from_sources(
        std::iter::once(source.source().clone())
            .chain(prepared.models.iter().map(|model| model.source().clone()))
            .chain(
                prepared
                    .libraries
                    .iter()
                    .map(|library| library.source().clone()),
            ),
    );
    Ok(result)
}

/// The package name a program compiles under: its source identity's
/// `identity` label.
fn package_name(identity: &fsl_foundation::source::SourceIdentity) -> String {
    identity
        .label(fsl_foundation::source::SourceLabel::Identity)
        .to_owned()
}

fn canonical_unit_source(sources: &[FormalSource]) -> &FormalSource {
    sources
        .iter()
        .min_by(|left, right| {
            use fsl_foundation::source::SourceLabel;
            left.source()
                .identity()
                .label(SourceLabel::Authority)
                .as_bytes()
                .cmp(
                    right
                        .source()
                        .identity()
                        .label(SourceLabel::Authority)
                        .as_bytes(),
                )
                .then_with(|| {
                    left.source()
                        .identity()
                        .label(SourceLabel::Identity)
                        .as_bytes()
                        .cmp(
                            right
                                .source()
                                .identity()
                                .label(SourceLabel::Identity)
                                .as_bytes(),
                        )
                })
        })
        .expect("the compile program has at least one source")
}

/// The call's accounting limits (FR-100): each named counter uses the
/// supplied bound and every omitted counter retains its accounting default.
fn call_accounting(accounting: &wire::Accounting) -> filament_exact::ScalarLimits {
    let mut limits = fsl_replay::AccountingLimits::default();
    for (setting, bound) in accounting.entries() {
        limits.set_bound(setting, bound);
    }
    limits.0
}

/// A spine compile refusal from a `1-draft` `run`, located over the source
/// its region is in (the program's or one library's), the same rendering
/// `complete` uses for `compile` (FR-100 mirrors FR-027).
fn spine_run_failure<'a>(
    source: &'a FormalSource,
    libraries: impl Iterator<Item = &'a FormalSource>,
    refusal: fsl_replay::spine::CompileRefusal,
) -> RunCause {
    let located = refusal
        .region()
        .and_then(|region| {
            std::iter::once(source)
                .chain(libraries)
                .find(|candidate| candidate.source().reference() == region.source())
        })
        .unwrap_or(source);
    RunCause::Spine(Box::new(SpineFailure {
        source: located.source().identity().clone(),
        path: located.source().path().to_owned(),
        span: refusal
            .region()
            .and_then(|region| located.source().render(region)),
        refusal,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_semantic_value::catalog::InternalFault;
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-2455", "FR-865-AC-5")]
    fn prepared_clause_source_is_available_to_outcome_rendering() {
        let source = fsl_replay::spine::ClauseRunSource::Program {
            identity: fsl_foundation::SourceIdentity::new("tests", "prepared-clause"),
            path: "src/clause.fsl".to_owned(),
            bytes: b"language \"ix:native\" edition \"1-draft\";\n".to_vec(),
        };
        let admitted =
            source_from_clause(&source).unwrap_or_else(|| panic!("prepared source is admitted"));
        assert_eq!(admitted.path(), "src/clause.fsl");
        assert_eq!(
            admitted.text(),
            "language \"ix:native\" edition \"1-draft\";\n"
        );
    }

    fn fault_error(stage: &'static str, invariant: &'static str) -> RunError {
        RunError {
            cause: RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(
                InternalFault::new(stage, invariant),
            ))),
            sources: crate::Sources::default(),
        }
    }

    /// FR-100 "Internal failure at S6a" (FND-013): the `CheckedInvariant`
    /// kernel refusal's own fault envelope -- stage `call`, code
    /// `runtime_invariant`, `details {stage, invariant}` naming
    /// `S6a`/`checked-program-invariant`, exit 30.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn checked_invariant_fault_envelope_exits_30() {
        let error = fault_error("S6a", "checked-program-invariant");
        assert_eq!(error.category().exit_code(), 30);
        let value = error.value().unwrap();
        assert_eq!(value["stage"], "call");
        assert_eq!(value["code"], "runtime_invariant");
        assert_eq!(
            value["details"],
            serde_json::json!({"stage": "S6a", "invariant": "checked-program-invariant"})
        );
    }

    /// FR-100 "Internal failure at S6a" (FND-013): `CallFailure::Fault`'s
    /// forwarded envelope (`convert_call_failure`) carries whatever
    /// stage/invariant the S6a caller named, still stage `call`, code
    /// `runtime_invariant`, exit 30.
    #[test]
    #[trace("TC-452", "FR-100-AC-9", "TC-786", "FR-100-AC-11")]
    fn call_failure_fault_envelope_exits_30() {
        let error = fault_error("call", "spine-run-supplies-admitted-name-and-arity");
        assert_eq!(error.category().exit_code(), 30);
        let value = error.value().unwrap();
        assert_eq!(value["stage"], "call");
        assert_eq!(value["code"], "runtime_invariant");
        assert_eq!(
            value["details"],
            serde_json::json!({
                "stage": "call",
                "invariant": "spine-run-supplies-admitted-name-and-arity",
            })
        );
    }

    /// FR-100-AC-13: `call.accounting` reaches the call as its accounting
    /// limits, with omitted counters retaining their defaults.
    #[test]
    #[trace("TC-921", "FR-100-AC-13")]
    fn call_accounting_reaches_the_call_as_its_limits() {
        let limits = |text: &str| {
            let call: wire::Call = serde_json::from_str(text).expect("a call");
            call_accounting(&call.accounting)
        };
        let all = limits(
            r#"{"function":"f","arguments":[],"accounting":{"integer_bits":1,
            "decimal_digits":2,"scale_expansion":3,"text_input_bytes":4,
            "text_scalars":5,"normalized_scalars":6,"unit_edges":7,
            "value_occurrences":8,"work_units":9,"result_units":10}}"#,
        );
        assert_eq!(
            all,
            filament_exact::ScalarLimits {
                integer_bits: 1,
                decimal_digits: 2,
                scale_expansion: 3,
                text_input_bytes: 4,
                text_scalars: 5,
                normalized_scalars: 6,
                unit_edges: 7,
                value_occurrences: 8,
                work_units: 9,
                result_units: 10,
            }
        );
        let one = limits(r#"{"function":"f","arguments":[],"accounting":{"work_units":7}}"#);
        assert_eq!(one, fsl_replay::spine::default_accounting(7));
        let defaults = fsl_replay::spine::default_accounting(fsl_replay::spine::DEFAULT_WORK_UNITS);
        assert_eq!(
            limits(r#"{"function":"f","arguments":[],"accounting":{}}"#),
            defaults
        );
        assert_eq!(limits(r#"{"function":"f","arguments":[]}"#), defaults);
    }

    /// FSpec FR-323-AC-10 (TC-278 RE-13): a request's `limits.supplied` is
    /// closed and complete with each ceiling an exact `u64`, or it refuses
    /// at decode; absent, the published defaults apply.
    #[test]
    #[trace("TC-721", "FR-255-AC-4")]
    fn limits_supplied_is_complete_and_u64_or_the_request_refuses() {
        use filament_semantic_value::supplied_stop::SuppliedLimits;
        let read = |text: &str| serde_json::from_str::<wire::Limits>(text);
        assert_eq!(read("{}").expect("no limits").supplied, None);
        assert_eq!(
            read(r#"{"supplied":{"admission_work_units":0,"conversion_work_units":7,"numeric_magnitude_bits":9}}"#)
                .expect("a complete object")
                .supplied,
            Some(
                SuppliedLimits::default()
                    .with_admission_work_units(0)
                    .with_conversion_work_units(7)
                    .with_numeric_magnitude_bits(9)
            )
        );
        for malformed in [
            r#"{"supplied":{"admission_work_units":1}}"#,
            r#"{"supplied":{"conversion_work_units":1}}"#,
            r#"{"supplied":{"admission_work_units":1,"conversion_work_units":1}}"#,
            r#"{"supplied":{"admission_work_units":1,"conversion_work_units":1,"numeric_magnitude_bits":-1}}"#,
            r#"{"supplied":{"admission_work_units":1,"conversion_work_units":1,"numeric_magnitude_bits":null}}"#,
            r#"{"supplied":{"admission_work_units":1,"conversion_work_units":1,"numeric_magnitude_bits":"64"}}"#,
            r#"{"supplied":{"admission_work_units":1,"conversion_work_units":1,"numeric_magnitude_bits":1,"depth":1}}"#,
            r#"{"supplied":{"admission_work_units":18446744073709551616,"conversion_work_units":1,"numeric_magnitude_bits":1}}"#,
            r#"{"supplied":{"admission_work_units":-1,"conversion_work_units":1}}"#,
            r#"{"supplied":{"admission_work_units":1.5,"conversion_work_units":1}}"#,
            r#"{"supplied":{"admission_work_units":null,"conversion_work_units":1}}"#,
            r#"{"supplied":null}"#,
            r#"{"supplied":[1,2]}"#,
        ] {
            assert!(read(malformed).is_err(), "{malformed} must refuse");
        }
    }
}
