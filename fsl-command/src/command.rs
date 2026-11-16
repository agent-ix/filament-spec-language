// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local orchestration for the `1-draft` spine compiler and runner.

mod analyze;
mod clause;
mod monitor;
mod output;
mod output_file;
mod wire;

pub use analyze::{analyze, analyze_with_limits, prepare_analyze, AnalyzeInput, PreparedAnalyze};
pub use monitor::{monitor, MonitorEvent, MonitorSummary};
pub use output::NativeResult;

use crate::formal_source::FormalSource;
use filament_contract_model as ir;
use fsl_foundation::serde_object::Object;
use fsl_foundation::{Code, Diagnostic, Source};
use fsl_semantics::model::domain_package::DOMAIN_PACKAGE_PROFILE;
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, RunCause>;
const REQUEST_BYTES: usize = 1_048_576;
const TOTAL_BYTES: usize = 8_388_608;
const FILES: usize = 64;

/// Caller byte ceilings for one `run` invocation (FR-312).
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
}

impl FileCategory {
    /// Stable category spelling in command intake diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Program => "programs",
            Self::Model => "models",
            Self::Observation => "observations",
        }
    }
}

struct FileCounts {
    programs: usize,
    models: usize,
    observations: usize,
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
}

/// Successful compilation with the core's document and exact emitted bytes.
pub struct CompileOutcome {
    package_bytes: Vec<u8>,
    document: fsl_replay::OutcomeDocument,
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
}

/// Prepared function input retained with its native source correspondence.
pub struct PreparedFunctionRun {
    data: Box<FunctionRunData>,
}

struct FunctionRunData {
    source: FormalSource,
    libraries: Vec<FormalSource>,
    packages: fsl_semantics::model::intake::PackageInput,
    dependencies: fsl_replay::spine::DependencyInput,
    call: fsl_replay::spine::Call,
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
    #[error("invalid request: clocks is given both in the request and as a clocks override (--clocks); give it once")]
    ClocksGivenTwice,
    /// FR-810: the request carries `output` and the caller also supplied an
    /// output override (the driver's `--output`).
    #[error("invalid request: output is given both in the request and as an output override (--output); give it once")]
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
    #[error("invalid request identifier: {0:?}")]
    Identifier(ir::Diagnostic),
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
    /// The request's `libraries` are refused before any library is read.
    #[error("the request's libraries are refused: {0}")]
    Libraries(LibrarySelection),
    /// Spine compile of a `1-draft` program refused at one stage.
    #[error("{}", .0.refusal)]
    Spine(Box<SpineFailure>),
    /// FR-100: a `1-draft` native-run/1 request carries a member the run
    /// command does not admit for it, or carries no `call`.
    #[error("a 1-draft program runs through the spine; the request selects {0}")]
    CompleteRunSelection(CompleteRunSelection),
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
    /// FR-620: a request member FSL admits but does not implement yet
    /// (`unsupported_construct`/`not-yet-implemented`, FR-763), such as a
    /// checked-package selection (QSL-2271).
    #[error("unsupported_construct/not-yet-implemented: analyzing a {member} selection is not implemented yet")]
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
        Self::Identifier(value)
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
            Self::Identifier(_) => Code::InvalidIdentifier,
            Self::Native(error) => error.code,
            Self::Source(error) => error.diagnostic.code,
            Self::CompleteSelection(_)
            | Self::Libraries(_)
            | Self::CompleteRunSelection(_)
            | Self::EmptyCompileProgram => Code::InvalidRequest,
            Self::ExtractionRequiresRun => Code::ExtractionRequiresRun,
            Self::Spine(failure) => failure.refusal.code(),
            Self::SpineRun(refusal) => refusal.code(),
            Self::AnalyzeRequest { .. } => Code::InvalidRequest,
            Self::AnalyzeNotYetImplemented { .. } => Code::UnsupportedConstruct,
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
            Self::AnalyzeFault | Self::MonitorFault | Self::MonitorCancelled => {
                Code::RuntimeInvariant
            }
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
            Self::Output(_) | Self::ClauseOutput(_) => {
                filament_semantic_value::catalog::Category::InternalFailure
            }
            Self::SpineRun(refusal) => refusal.category(),
            Self::AnalyzeRefused(refusal) => refusal.document().category(),
            Self::MonitorRefused(refusal) => refusal.document().category(),
            Self::AnalyzeFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorCancelled => filament_semantic_value::catalog::Category::Incomplete,
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
}

impl RunError {
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
            .with_diagnostics(vec![OutcomeDiagnostic::from_compile_refusal(
                &failure.refusal,
            )]),
            RunCause::SpineRun(refusal) => OutcomeDocument::from_run_refusal(operation, refusal),
            RunCause::AnalyzeRefused(refusal) => refusal.document(),
            RunCause::MonitorRefused(refusal) => refusal.document(),
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
            RunCause::Io { .. }
            | RunCause::Json(_)
            | RunCause::ClauseRequest { .. }
            | RunCause::ClocksGivenTwice
            | RunCause::OutputGivenTwice
            | RunCause::OutputExists { .. }
            | RunCause::Output(_)
            | RunCause::ClauseOutput(_)
            | RunCause::Format
            | RunCause::Limit(_)
            | RunCause::Identifier(_)
            | RunCause::CompleteSelection(_)
            | RunCause::ExtractionRequiresRun
            | RunCause::EmptyCompileProgram
            | RunCause::Libraries(_)
            | RunCause::CompleteRunSelection(_)
            | RunCause::AnalyzeRequest { .. }
            | RunCause::AnalyzeUniverse(_)
            | RunCause::AnalyzeFault
            | RunCause::MonitorFault
            | RunCause::MonitorCancelled => OutcomeDocument::new(operation, None, self.category())
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    None,
                    self.cause.code().as_str(),
                    None,
                    self.outcome_message(),
                )]),
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
                format!("observation.document_bytes limit {maximum} exceeded; raise --limit observation.document_bytes")
            }
            RunCause::Limit(LimitKind::SelectedFiles {
                category,
                requested,
                remaining,
                maximum,
            }) => {
                format!("selected files limit {maximum} exceeded: {category:?} requested {requested}, remaining {remaining}")
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
}

#[derive(Clone, Copy)]
enum DependentLimit {
    Fixed,
    CallerConfigured,
}

impl Intake<'_> {
    fn file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        self.bounded_file(path, ceiling, false)
    }

    fn observation_file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        self.bounded_file(path, ceiling, true)
    }

    fn bounded_file(&mut self, path: &str, ceiling: usize, observation: bool) -> Result<Vec<u8>> {
        let bytes = read_file(&self.directory.join(path), ceiling.min(self.remaining)).map_err(
            |cause| match cause {
                RunCause::Limit(LimitKind::FileBytes)
                    if observation && ceiling <= self.remaining =>
                {
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
            },
        )?;
        self.remaining -= bytes.len();
        Ok(bytes)
    }

    fn source(&mut self, selected: &wire::SourceFile) -> Result<FormalSource> {
        let identity = selected.identity.bind()?;
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
        Ok(FormalSource::new(source, identity.formal))
    }
}

/// Read a native-run/1 job and execute it locally with fresh, bounded stage state.
pub fn run(path: &Path) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits(path, RunLimits::default())
}

/// Run with request members supplied beside the request file (FR-640).
pub fn run_with_overrides(
    path: &Path,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits_and_overrides(path, RunLimits::default(), overrides)
}

/// Run with the caller's explicit request and dependent-file byte ceilings.
pub fn run_with_limits(
    path: &Path,
    limits: RunLimits,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_with_limits_and_overrides(path, limits, &RequestOverrides::default())
}

/// Run with explicit byte ceilings and request members supplied beside the
/// request file (FR-640).
pub fn run_with_limits_and_overrides(
    path: &Path,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<RunResult, Box<RunError>> {
    // FR-644: a `native-monitor/1` request runs its record file as the batch
    // stream.
    let monitored = with_request_limits(path, limits.request_bytes, |directory, bytes| {
        if monitor::is_monitor_request(bytes) {
            monitor::run_bytes(directory, bytes, limits.dependent_bytes).map(Some)
        } else {
            Ok(None)
        }
    })?;
    if let Some(result) = monitored {
        return Ok(result);
    }
    let prepared = prepare_run_with_overrides(path, limits, overrides)?;
    execute_prepared(prepared).map_err(|cause| Box::new(RunError { cause }))
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
            },
        )?;
        let source = intake.source(&request.program.source)?;
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
    compile_outcome_with_overrides(path, &RequestOverrides::default())
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
    with_request(path, |directory, bytes| {
        let request: wire::CompileRequest = request_with_overrides(bytes, overrides)?;
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
        let mut intake = intake_with_fixed_bytes(
            directory,
            FileCounts {
                // A library source is a program source a `1-draft` program
                // imports, and counts toward the same file limits (FR-027).
                programs: request
                    .libraries
                    .len()
                    .saturating_add(request.program.sources.len()),
                models: request.models.len(),
                observations: 0,
            },
            TOTAL_BYTES,
        )?;
        // Every unit's edition is decided before any model is read: a
        // `1-draft` program takes domain packages only, and a request
        // selecting a native rule model for it refuses whatever state the
        // model files are in.
        let mut sources = Vec::with_capacity(request.program.sources.len());
        for selection in &request.program.sources {
            let source = intake.source(selection)?;
            require_complete_edition(source.source())?;
            sources.push(source);
        }
        let outcome = complete(&request, &mut intake, &sources)?;
        if let Some(output) = output {
            output_file::write_new_file(&output, &outcome.package_bytes)?;
        }
        Ok(outcome)
    })
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
                    "{} declares edition {:?}; compile reads {:?}",
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
) -> Result<CompileOutcome> {
    if sources.len() == 1 {
        return complete_single(request, intake, &sources[0]);
    }
    complete_units(request, intake, sources)
}

fn complete_single(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    source: &FormalSource,
) -> Result<CompileOutcome> {
    complete_sources(request, intake, std::slice::from_ref(source), false)
}

/// Compile FR-480's ordered program-unit list as one checked package.
fn complete_units(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
) -> Result<CompileOutcome> {
    complete_sources(request, intake, sources, true)
}

/// Shared request, dependency, parsing, and emission pipeline for one source
/// and an ordered source-unit list. Only the spine's single-unit versus
/// multi-unit entry points vary by `unit_list`.
fn complete_sources(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    unit_list: bool,
) -> Result<CompileOutcome> {
    check_sources(
        request,
        intake,
        sources,
        unit_list,
        |checked, selections, _, _, failure| {
            let cancel = filament_exact::Cancel::new();
            let limits = fsl_replay::spine::SpineLimits::default();
            let result = fsl_replay::spine::package_with_temporal_selections(
                checked,
                selections,
                limits.package,
                &cancel,
            );
            let document = fsl_replay::OutcomeDocument::from_package(&result);
            let emitted = result.map_err(failure)?.into_value();
            Ok(CompileOutcome {
                package_bytes: emitted.package().bytes().to_vec(),
                document,
            })
        },
    )
}

/// The pipeline of [`complete_sources`] through S4: the checked unit is
/// handed to `finish` with the admitted domain packages, the library
/// sources and the map from a spine failure to its located cause.
fn check_sources<T>(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    unit_list: bool,
    finish: impl FnOnce(
        &fsl_replay::spine::CheckedUnit,
        &[fsl_replay::spine::TemporalClockSelection],
        fsl_semantics::model::intake::PackageInput,
        Vec<&FormalSource>,
        &dyn Fn(fsl_replay::spine::FrontEndFailure) -> RunCause,
    ) -> Result<T>,
) -> Result<T> {
    if request.models.iter().any(|model| !is_domain_package(model)) {
        return Err(RunCause::CompleteSelection(CompleteSelection::NativeModels));
    }
    let documents = request
        .models
        .iter()
        .map(|model| intake.source(&model.source))
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
    let limits = fsl_replay::spine::SpineLimits::default();
    let failure = |failure| match fsl_replay::spine::refusal_or_fault(failure) {
        Ok(refusal) => spine_failure(*refusal),
        Err(fault) => RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault))),
    };
    let parsed = parse_requests
        .iter()
        .map(|parse_request| {
            fsl_replay::spine::parse(parse_request, limits.source, &cancel)
                .map(|stage| stage.into_value())
                .map_err(failure)
        })
        .collect::<Result<Vec<_>>>()?;
    let check_selections: Vec<_> = parsed
        .iter()
        .flat_map(|unit| request.clocks.check_selections(unit))
        .collect();
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
        let checked = fsl_replay::spine::check_with_refinement_internals(
            &parsed,
            package_name(first_source.identity()),
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
        let selections = request
            .clocks
            .package_selections(&checked, &[&parsed])
            .map_err(|refusal| spine_failure(*refusal))?;
        (checked, selections)
    };
    finish(
        &checked,
        &package_selections,
        packages,
        libraries.iter().map(|(_, source)| source).collect(),
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
        .map(|library| Ok((library, intake.source(&library.source)?)))
        .collect()
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
        Box::new(RunError { cause })
    })?;
    action(path.parent().unwrap_or(Path::new(".")), &bytes)
        .map_err(|cause| Box::new(RunError { cause }))
}

/// Request members a caller supplies beside the request file rather than inside
/// it (FR-640). Each is read exactly as the request's own member would be.
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestOverrides<'a> {
    /// The `clocks` object, as the driver's `--clocks` file carries it. A request
    /// that also carries `clocks` is refused `invalid-request` at stage `request`.
    pub clocks: Option<&'a serde_json::value::RawValue>,
    /// The file the checked package is written to, as the driver's `--output`
    /// names it (FR-810). Only a compile reads it; a request that also carries
    /// `output` is refused `invalid-request` at stage `request`.
    pub output: Option<&'a Path>,
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
    })
}

fn intake_with_bytes(directory: &Path, counts: FileCounts, maximum: usize) -> Result<Intake<'_>> {
    check_file_count(counts)?;
    Ok(Intake {
        directory,
        remaining: maximum,
        maximum,
        dependent_limit: Some(DependentLimit::CallerConfigured),
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
    })
}

fn prepare_run_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    overrides: &RequestOverrides<'_>,
) -> Result<PreparedRun> {
    let request: wire::Request = request_with_overrides(bytes, overrides)?;
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
            }),
        },
        dependent_bytes,
    )?;
    // The program source is read exactly once; its declared edition decides
    // the runner before any model or library source is read (FR-100).
    let source = intake.source(&request.program.source)?;
    require_complete_edition(source.source())?;
    prepare_complete_run(&mut intake, &request, source)
}

/// Prepare FR-100's function or clause selection without executing a stage.
/// `source` is the program source, already read by bounded intake.
fn prepare_complete_run(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    source: FormalSource,
) -> Result<PreparedRun> {
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
    let documents = request
        .models
        .iter()
        .map(|model| intake.source(&model.source))
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
        .map(|library| Ok((library, intake.source(&library.source)?)))
        .collect::<Result<Vec<(&wire::Library, FormalSource)>>>()?;
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
    let arguments = call
        .arguments
        .iter()
        .map(|argument| fsl_replay::spine::CallArgument {
            parameter: argument.parameter.clone(),
            value: filament_exact::Value::Integer(filament_exact::Integer::from(argument.value)),
        })
        .collect();
    let mut spine_call = fsl_replay::spine::Call::new(
        call.function.clone(),
        arguments,
        call_accounting(&call.accounting),
        request.limits.supplied.unwrap_or_default(),
    );
    spine_call.clocks = request.clocks.clone();
    Ok(PreparedRun::Function(PreparedFunctionRun {
        data: Box::new(FunctionRunData {
            source,
            libraries: libraries.into_iter().map(|(_, source)| source).collect(),
            packages,
            dependencies,
            call: spine_call,
        }),
    }))
}

fn execute_prepared(prepared: PreparedRun) -> Result<RunResult> {
    let prepared = match prepared {
        PreparedRun::Function(prepared) => prepared,
        PreparedRun::Clause(request) => {
            let selection = request.selection.clone();
            let report = fsl_replay::spine::run_clause(*request).map_err(|error| {
                RunCause::ClauseRequest {
                    member: "clause".to_owned(),
                    reason: error.to_string(),
                }
            })?;
            return output::clause_report(&report, &selection);
        }
    };
    let prepared = *prepared.data;
    let source = &prepared.source;
    let outcome = fsl_replay::spine::run(
        source.source().identity().clone(),
        package_name(source.source().identity()),
        source.source().path(),
        source.source().text().as_bytes(),
        &prepared.packages,
        &prepared.dependencies,
        fsl_replay::spine::SpineLimits::default(),
        &prepared.call,
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &filament_exact::Cancel::new(),
    )
    .map_err(|refusal| match *refusal {
        fsl_replay::spine::RunRefusal::Compile(compile_refusal) => {
            spine_run_failure(source, prepared.libraries.iter(), *compile_refusal)
        }
        other => RunCause::SpineRun(Box::new(other)),
    })?;
    output::spine_run_result(source, &prepared.call.function, outcome)
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

    fn fault_error(stage: &'static str, invariant: &'static str) -> RunError {
        RunError {
            cause: RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(
                InternalFault::new(stage, invariant),
            ))),
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
