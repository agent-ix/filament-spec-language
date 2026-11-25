// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local orchestration for the `1-draft` spine compiler and runner.

mod analyze;
mod clause;
mod dependency_manifest;
mod export;
mod flow;
mod handoff;
mod items;
pub mod limits;
mod model_based_test;
pub mod monitor;
mod output;
mod output_file;
mod package_run;
pub mod plan;
mod relationships;
mod request_shape;
mod runtime_sources;
pub use package_run::PreparedPackageRun;
pub mod plan_request;
mod plan_synthesis;
mod replay;
mod requirements;
mod settle;
mod synthesis;
mod tl;
mod transfer;
mod wire;

pub use analyze::{
    analyze, analyze_with_limits, analyze_with_providers, analyze_with_providers_and_limits,
    prepare_analyze, AnalyzeInput, PreparedAnalyze,
};
pub use flow::{analyze_flow_report, FlowAnalyzeError};
pub use limits::{
    CommandLimits, LimitedOperation, MonitorPreparationLimits, OperationMismatch, PreparationBound,
    PreparationRefusal, SettingUnavailable,
};
pub use monitor::{
    monitor, monitor_with_target_preparer, MonitorEvent, MonitorPreparation, MonitorProviders,
    MonitorSummary, TargetPreparer,
};
pub use output::NativeResult;
pub use replay::replay_with_limits;
pub use request_shape::{request_example, RequestExampleError};
pub use tl::{record as tl_record, AtomMapping, TlRecord};

use crate::formal_source::FormalSource;
use filament_contract_model as ir;
use fsl_analyze::analyze::document::Negotiate;
use fsl_analyze::provider::ProviderOptions;
use fsl_foundation::serde_object::Object;
use fsl_foundation::{Code, Diagnostic, Source};
use fsl_replay::spine::SpineLimits;
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

fn request_integer_division(
    value: Option<serde_json::Value>,
) -> Result<Option<fsl_replay::AdmittedIntegerDivision>> {
    read_integer_division(value.as_ref(), "integer_division")
}

/// FR-027: the request's `call_site`, an object of exactly `kind`
/// `"imported-function"`, `library` (a library identity) and `function` (a
/// function identifier). Anything else, and a program that is an FR-480 array
/// of units, refuses `invalid-request` naming `call_site`.
fn request_call_site(
    value: Option<&serde_json::Value>,
    program_sources: usize,
) -> Result<Option<fsl_replay::ImportedFunctionName>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let refuse = |reason: &str| RunCause::ClauseRequest {
        member: "call_site".to_owned(),
        reason: reason.to_owned(),
    };
    let serde_json::Value::Object(members) = value else {
        return Err(refuse("must be an object"));
    };
    if let Some(extra) = members
        .keys()
        .find(|name| !matches!(name.as_str(), "kind" | "library" | "function"))
    {
        return Err(refuse(&format!("has the unknown member {extra}")));
    }
    let text = |name: &str| match members.get(name) {
        Some(serde_json::Value::String(text)) => Ok(text.as_str()),
        Some(_) => Err(refuse(&format!("member {name} must be a string"))),
        None => Err(refuse(&format!("lacks the member {name}"))),
    };
    let kind = text("kind")?;
    let library = text("library")?;
    let function = text("function")?;
    if kind != "imported-function" {
        return Err(refuse("kind must be `imported-function`"));
    }
    let library = (!library.is_empty())
        .then(|| fsl_semantics::library::LibraryName::new(library).ok())
        .flatten()
        .ok_or_else(|| refuse("library must be a library identity"))?;
    let function = filament_exact::Identifier::new(function)
        .map_err(|_| refuse("function must be an identifier"))?;
    if program_sources > 1 {
        return Err(refuse("is not supported for an array of program units"));
    }
    Ok(Some(fsl_replay::ImportedFunctionName { library, function }))
}

/// Caller byte ceilings for one command invocation (FR-312, FR-869).
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

#[derive(Clone, Copy, Default)]
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
    /// Aggregate ceiling for the selected model, library and extension definition
    /// files; the checked source is bounded by `s1.input_bytes`, not this.
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
/// Dependent documents (models, libraries, extensions) are charged to
/// `dependent_bytes`; the checked source is read under the fixed intake ceiling
/// and bounded by `s1.input_bytes` at parse, as in compile.
pub fn check_outcome(source_path: &Path, inputs: &CheckInputs) -> CheckOutcome {
    let result = check_outcome_inner(source_path, inputs);
    match result {
        Ok((document, sources)) => CheckOutcome { document, sources },
        Err(failure) => {
            let (cause, sources) = *failure;
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
) -> std::result::Result<
    (fsl_replay::OutcomeDocument, crate::Sources),
    Box<(RunCause, crate::Sources)>,
> {
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
    .map_err(|cause| Box::new((cause, crate::Sources::default())))?;
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

        let source_identity = source_path.to_string_lossy();
        let source = intake.source_outside_budget(
            &check_source_file(source_path, "filament", &source_identity, "CheckSource"),
            "source",
            check_source_read_ceiling(inputs.limits.source.source_bytes),
        )?;
        if fsl_semantics::model::intake::is_model_unit_source(source.source().text().as_bytes()) {
            compile_formal_model_unit(&source)?;
            return Ok(fsl_replay::OutcomeDocument::new(
                fsl_replay::Operation::Check,
                Some(fsl_replay::OutcomeStage::I1),
                filament_semantic_value::catalog::Category::Success,
            ));
        }
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

        let mut declared_models: Vec<&str> = Vec::new();
        for selection in &parsed.unit().selections().models {
            if !declared_models.contains(&selection.identity.as_str()) {
                declared_models.push(selection.identity.as_str());
            }
        }
        let declared_libraries: std::collections::BTreeSet<_> = parsed
            .unit()
            .selections()
            .imports
            .iter()
            .map(|selection| selection.identity.as_str())
            .collect();
        reject_unselected_check_models(&inputs.models, &declared_models)?;
        reject_unselected_check_inputs(&inputs.libraries, "--library", &declared_libraries)?;

        let models = inputs
            .models
            .iter()
            .map(|(identity, path)| {
                Ok(wire::Model {
                    format: DOMAIN_PACKAGE_PROFILE.to_owned(),
                    source: check_source_file(path, "fsl-command", identity, "CheckModel"),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let model_documents = models
            .iter()
            .map(|model| intake.source(&model.source, "models"))
            .collect::<Result<Vec<_>>>()?;
        let packages = package_input_from_formal_sources(&inputs.models, &model_documents)?;
        let admitted_models =
            match fsl_replay::spine::select(&parsed, &packages, inputs.limits.model, &cancel) {
                Ok(models) => models.into_value(),
                Err(failure) => {
                    return Ok(fsl_replay::OutcomeDocument::from_check(&Err(failure)));
                }
            };

        let libraries = inputs
            .libraries
            .iter()
            .map(|(identity, path)| {
                let source = intake.source(
                    &check_source_file(path, "fsl-command", identity, "CheckLibrary"),
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
        .map_err(|cause| Box::new((cause, sources)))
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

/// FR-866: a `--model` identity no `model` declaration selects refuses, naming
/// the identities the source selects in source order.
fn reject_unselected_check_models(
    selections: &[(String, PathBuf)],
    declared: &[&str],
) -> Result<()> {
    if let Some((identity, _)) = selections
        .iter()
        .find(|(identity, _)| !declared.contains(&identity.as_str()))
    {
        let selected = if declared.is_empty() {
            "the source selects no models".to_owned()
        } else {
            format!("the source selects: {}", declared.join(", "))
        };
        return Err(RunCause::ClauseRequest {
            member: "--model".to_owned(),
            reason: format!("--model {identity} is not selected by the source; {selected}"),
        });
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

fn check_source_file(
    path: &Path,
    authority: &str,
    identity: &str,
    document: &str,
) -> wire::SourceFile {
    wire::SourceFile {
        file: path.to_string_lossy().into_owned(),
        identity: wire::Identity {
            authority: authority.to_owned(),
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
    limits: fsl_replay::spine::SpineLimits,
    call: fsl_replay::spine::Call,
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
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
    /// Caller-supplied `div` and `rem` law for the source program, when one was
    /// present in the request (FR-767).
    pub integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
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
            integer_division: data.integer_division,
            call: data.call,
        }
    }
}

/// A decoded function or clause request; preparation performs no execution.
pub enum PreparedRun {
    /// A function selection for the existing execution pipeline.
    Function(PreparedFunctionRun),
    /// A clause selection for the existing clause runner.
    Clause(Box<PreparedClauseRun>),
    /// FR-931: a `tl_lowering` request: the clause's valuation run, the
    /// lowering it asks for, and the program text its atoms are named in.
    TlLowering(Box<PreparedLowering>),
    /// FR-100, FR-1039: a call over a selected checked package, run from the
    /// package's own records without compiling source.
    Package(Box<package_run::PreparedPackageRun>),
    /// FSpec FR-190: the result of the direct model-based testing request.
    ModelBasedTest(PreparedModelBasedTestRun),
    /// FR-1030: a refinement transfer, settled by the native run executor.
    RefinementTransfer(Box<transfer::PreparedTransfer>),
}

/// The direct model-based test record and sources admitted to produce it.
pub struct PreparedModelBasedTestRun {
    record: serde_json::Value,
    exit_code: u8,
    sources: crate::Sources,
}

/// A typed clause request together with every source admitted to prepare it.
pub struct PreparedClauseRun {
    request: Box<fsl_replay::spine::ClauseRunRequest>,
    sources: crate::Sources,
}

impl PreparedClauseRun {
    /// Transfer the typed clause request to the existing runner.
    pub fn into_request(self) -> Box<fsl_replay::spine::ClauseRunRequest> {
        self.request
    }
}

impl std::ops::Deref for PreparedClauseRun {
    type Target = fsl_replay::spine::ClauseRunRequest;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl std::ops::DerefMut for PreparedClauseRun {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
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
            Self::Clause(request) => request.sources.clone(),
            Self::TlLowering(prepared) => prepared.sources.clone(),
            Self::Package(prepared) => prepared.sources(),
            Self::ModelBasedTest(prepared) => prepared.sources.clone(),
            Self::RefinementTransfer(prepared) => {
                crate::Sources::from_sources([prepared.source.source().clone()])
            }
        }
    }
}

/// Original intake failure, without recovering typed data from display messages.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RunCause {
    /// Local file could not be opened, read or written.
    #[error("cannot access {path}: {error}")]
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
    #[error("invalid request: output {path} already exists; FSL never overwrites an existing path", path = path.display())]
    OutputExists {
        /// The existing output path.
        path: PathBuf,
    },
    /// FSpec FR-817 / FR-286: the export was made but its file could not be
    /// written: an output failure after S4.
    #[error("cannot write the export to {path}: {error}")]
    ExportOutput {
        /// The file the export was to create.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        error: io::Error,
    },
    /// FSpec FR-817: the export mapper failed on a construct it does not lower.
    #[error("the export mapper does not lower {construct}: {reason}")]
    ExportMapper {
        /// The construct the mapper stopped at.
        construct: String,
        /// Why it stopped.
        reason: String,
    },
    /// FR-1015: an `export` member the request reader refuses, with its
    /// FR-805 cause (`missing-member`, `invalid-value`, `wrong-value-kind`,
    /// `unknown-member`) apart from the message.
    #[error("invalid export request member {path}: {cause}: {reason}")]
    ExportRequest {
        /// The JSON pointer of the member at fault.
        path: String,
        /// The FR-805 cause.
        cause: &'static str,
        /// Why the member is refused.
        reason: String,
    },
    /// FR-1015-AC-17: a protocol binding has the same invalid-request
    /// classification as the analyze reader, while retaining its export
    /// request pointer for the caller.
    #[error("invalid analyze request member {path}: {reason}")]
    ExportProtocolRequest {
        /// The export protocol binding at fault.
        path: String,
        /// Why the binding is refused.
        reason: String,
    },
    /// FR-1038: a `verification_plan` member the request reader refuses, with
    /// its FR-805 cause apart from the message.
    #[error("invalid verification plan request member {path}: {cause}: {reason}")]
    PlanRequest {
        /// The JSON pointer of the member at fault.
        path: String,
        /// The FR-805 code: `invalid_runtime_input` for a member defect, the
        /// selection's `missing_declaration` or `ambiguous_declaration`.
        code: Code,
        /// The FR-805 cause.
        cause: &'static str,
        /// Why the member is refused.
        reason: String,
    },
    /// FR-1037: a `synthesis` member the request reader refuses, with the
    /// FSpec FR-823 catalog code and FR-805 cause apart from the message.
    #[error("invalid synthesis request member {path}: {cause}: {reason}")]
    SynthesisRequest {
        /// The JSON pointer of the member at fault.
        path: String,
        /// The catalog code of the refusal.
        code: Code,
        /// The FR-805 cause.
        cause: &'static str,
        /// Why the member is refused.
        reason: String,
    },
    /// A well-typed source objective whose numeric planning family is unavailable.
    #[error("{}", .diagnostic.message)]
    SynthesisUnsupported {
        /// The refusal at the authored objective expression.
        diagnostic: Box<Diagnostic>,
    },
    /// The original typed admission defect of the authored initial snapshot.
    #[error("initial snapshot {file} refused: {}/{}", .record.catalog.code(), .record.catalog.cause())]
    SynthesisSnapshot {
        /// The supplied file.
        file: String,
        /// Unchanged catalog, named fields, kernel cause and typed path.
        record: Box<fsl_semantics::model::observation::AdmissionRecord>,
    },
    /// FR-752: a typed model-based-test request refusal with its catalog code
    /// and cause.
    #[error("invalid model_based_test request member {path}: {cause}")]
    ModelBasedTestRequest {
        /// The catalog code of the refusal.
        code: Code,
        /// The JSON pointer of the member at fault.
        path: String,
        /// The FR-805 cause retained by the evaluator.
        cause: &'static str,
    },
    /// FR-312: a clause request member cannot form a typed selection.
    #[error("invalid clause request member {member}: {reason}")]
    ClauseRequest {
        /// The wire member or observation key at fault.
        member: String,
        /// Why this member is invalid.
        reason: String,
    },
    /// FR-644: an invalid monitor target has one stable request diagnostic.
    #[error("invalid request member /request/target: {reason}")]
    MonitorTargetRequest {
        /// Why the target is invalid.
        reason: String,
    },
    /// FR-1039: a member whose run builds states over the package's model
    /// world, selected over a `{file}` package that carries no handoff rows.
    #[error("invalid clause request member {member}: it needs a package selected as a directory")]
    RequiresDirectory {
        /// The wire member that needs the handoff rows.
        member: String,
    },
    /// FR-1031: a native-run transfer declaration failed after compilation
    /// while names, relationships, and clause families were being selected.
    #[error("refinement transfer {stage} refused at {member}: {message}")]
    TransferSelection {
        /// The spine stage the refusal belongs to: `select` or `admit`.
        stage: &'static str,
        /// The selected request member.
        member: String,
        /// The stable selection cause.
        cause: &'static str,
        /// The catalogued diagnostic code.
        code: Code,
        /// Why this member was refused.
        message: String,
        /// The refusal's `details` members.
        details: serde_json::Map<String, serde_json::Value>,
    },
    /// Typed output serialization failed.
    #[error("cannot serialize native result: {0}")]
    Output(#[source] serde_json::Error),
    /// A typed clause report failed canonical wire production.
    #[error("cannot serialize clause-run result: {0}")]
    ClauseOutput(#[source] fsl_replay::spine::NativeRunResultError),
    /// FR-1047: required request members the document lacks, all at once.
    #[error("{message}")]
    MissingMembers {
        /// The refusal's message.
        message: String,
        /// `details.missing`: a pointer, or the pointers of a one-of group.
        missing: Vec<serde_json::Value>,
    },
    /// FR-1047: a request format the entry does not read.
    #[error("{message}")]
    FormatNotRead {
        /// The refusal's message.
        message: String,
        /// The formats the entry reads, in FR-1047's order.
        accepted: Vec<&'static str>,
    },
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
    /// A model-unit compiler refusal at I1, retaining its exact source cause.
    #[error("{0}")]
    ModelUnit(#[source] Box<ModelUnitFailure>),
    /// A package-input refusal while parsing selected Semantic IR documents.
    #[error("model package input refusal: {0:?}")]
    ModelPackageInput(Box<fsl_semantics::model::normalize::ModelRefusal>),
    /// A `1-draft` program's request selects what spine compile does not
    /// take.
    #[error("a complete-V1 program compiles alone; the request selects {0}")]
    CompleteSelection(CompleteSelection),
    /// FR-031: an extracted program is a run input, not a source-only compile.
    #[error("an extracted program requires run")]
    ExtractionRequiresRun,
    /// FR-031-AC-6: the request cannot select a package artifact and extracted source.
    #[error("an extracted program cannot select a package artifact")]
    ExtractionPackageConflict,
    /// FR-031-AC-6: exactly one authored clause binding is required.
    #[error("extracted program has {actual} clause bindings; expected exactly one")]
    ExtractionClauseCount {
        /// Number of bindings supplied in the request.
        actual: usize,
    },
    /// FR-031-AC-7: Quire rejected the clause-only semantic context.
    #[cfg(feature = "quire-extraction")]
    #[error("Quire rejected the clause-only context")]
    InvalidQuireContext {
        /// Unchanged diagnostics returned by Quire's semantic-block validator.
        diagnostics: Vec<fsl_source::SemanticFailure>,
    },
    /// FR-031: extraction failed after the request's Quire context validated.
    #[cfg(feature = "quire-extraction")]
    #[error("{message}")]
    ExtractionFailure {
        /// FR-030's catalog code.
        code: Code,
        /// The command-error stage for this extraction failure.
        stage: ExtractionFailureStage,
        /// Original identity, selected binding and any completed Quire outcome.
        details: serde_json::Map<String, serde_json::Value>,
        /// The upstream extraction diagnostic.
        message: String,
    },
    /// FR-027/FR-480: a source-unit list must not be empty.
    #[error("the compile program has no source units")]
    EmptyCompileProgram,
    /// FR-931: `tl_lowering.correspondence` names no TL correspondence.
    #[error(
        "unknown_profile/unsupported-selection: tl_lowering.correspondence {value:?} is none of mltl.closed-trace/v1, mltl.online-prefix/v1, mltl.origin-complete-history/v1 and mltl.infinite-trace/v1"
    )]
    TlCorrespondence {
        /// The value given.
        value: String,
    },
    /// FR-767: `integer_division` names no supported division law.
    #[error(
        "unknown_profile/unsupported-selection: integer-division law `{value}` supplied by `{member}` is not one of euclidean, floor and truncating"
    )]
    IntegerDivisionLaw {
        /// The member that supplied the value.
        member: String,
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
    /// FR-931: the model subject builder refused the
    /// `cross_evaluate.initial` snapshot (FR-120's admission refusals).
    #[error("tl_lowering.cross_evaluate.initial is refused: {}/{}", .0.catalog.code(), .0.catalog.cause())]
    TlSubject(Box<fsl_semantics::model::observation::AdmissionRecord>),
    /// FR-931: the lowering of a supported clause broke an invariant.
    #[error("the TL lowering faulted: {0}")]
    TlFault(String),
    /// The request's `libraries` are refused before any library is read.
    #[error("the request's libraries are refused: {0}")]
    Libraries(LibrarySelection),
    /// The request's extension definition catalog is invalid.
    #[error("the request's extension definitions are refused: {0}")]
    ExtensionCatalog(String),
    /// An extension names a capability outside the closed complete-V1
    /// inventory; it cannot be dropped, so the request is unsupported rather
    /// than malformed.
    #[error("the request's extension names an unknown required capability: {0}")]
    UnknownExtensionCapability(String),
    /// FR-027: the request's `call_site` selection is refused over the checked
    /// package, or locating it broke an invariant.
    #[error("{0}")]
    CallSite(Box<fsl_replay::CallSiteRefusal>),
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
    /// FSL FR-1033: a `memory` entry names no outermost `parallel` of a
    /// protocol of the package, or two entries name one
    /// (`invalid_runtime_input`/`invalid-value`).
    #[error("invalid analyze request member {member}: {reason}")]
    AnalyzeMemory {
        /// The entry at fault, by its pointer.
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
        "unsupported_construct/not-yet-implemented: analyzing a {member} selection is not implemented yet{detail}"
    )]
    AnalyzeNotYetImplemented {
        /// The request member the gap is in.
        member: &'static str,
        /// What of the request is pending: empty, or the item family and
        /// request index the gap is at.
        detail: String,
        /// The package node the gap is at, in words, when the gap has one.
        locus: Option<String>,
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
    /// FSL FR-621 "Node ids in an item": the request's package, or a dependency
    /// an item is analyzed over, has no admitted emission, so the document's
    /// node ids cannot be held. The run stops with the limit or the reader's
    /// own category and writes no document, never an internal failure.
    #[error("the result's node ids cannot be held: {0:?}")]
    AnalyzeEmission(fsl_analyze::analyze::GraphUnavailable),
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
    #[error("cannot serialize replay result: {error}")]
    ReplayOutput {
        /// The codec failure.
        #[source]
        error: fsl_replay::CauseCodecError,
        /// The all-or-nothing outcome document for this failed row.
        outcome: Box<fsl_replay::OutcomeDocument>,
    },
    /// An item run violated an internal runtime invariant.
    #[error("an items run faulted")]
    ItemsFault,
}

#[cfg(feature = "quire-extraction")]
/// Stage that refused a Markdown extraction request.
#[derive(Clone, Copy, Debug)]
pub enum ExtractionFailureStage {
    /// The authored binding did not identify an extracted clause.
    Selection,
    /// Quire rejected or could not provide the clause context.
    Quire,
}

/// The two owner projections of the same source-intake refusal.
#[derive(Debug, thiserror::Error)]
#[error("{native}")]
pub struct SourceFailure {
    #[source]
    native: Box<Diagnostic>,
    diagnostic: Box<fsl_cst::diagnostic::CompleteDiagnostic>,
}

/// A model-unit refusal projected from the forms-owned S1 wrapper.
#[derive(Debug, thiserror::Error)]
#[error("{native}")]
pub struct ModelUnitFailure {
    #[source]
    native: Box<Diagnostic>,
    diagnostic: Box<fsl_forms::cst::CompleteDiagnostic>,
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
    /// FR-1047: the `details` of a missing-members or format refusal.
    fn request_details(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        match self {
            Self::MissingMembers { missing, .. } => Some(serde_json::Map::from_iter([(
                "missing".to_owned(),
                serde_json::Value::Array(missing.clone()),
            )])),
            Self::FormatNotRead { accepted, .. } => Some(serde_json::Map::from_iter([
                (
                    "path".to_owned(),
                    serde_json::Value::String("/format".to_owned()),
                ),
                ("accepted".to_owned(), serde_json::json!(accepted)),
            ])),
            _ => None,
        }
    }

    /// Stable catalogued code for this cause, independent of its display message.
    pub fn code(&self) -> Code {
        match self {
            Self::Io { .. } => Code::IoError,
            Self::Json(_) => Code::InvalidRequest,
            Self::ClauseRequest { .. }
            | Self::MonitorTargetRequest { .. }
            | Self::RequiresDirectory { .. }
            | Self::ClocksGivenTwice
            | Self::OutputGivenTwice
            | Self::OutputExists { .. }
            | Self::ExportProtocolRequest { .. } => Code::InvalidRequest,
            Self::ExportMapper { .. } => Code::RuntimeInvariant,
            Self::ExportRequest { .. } => Code::InvalidRuntimeInput,
            Self::PlanRequest { code, .. }
            | Self::SynthesisRequest { code, .. }
            | Self::ModelBasedTestRequest { code, .. } => *code,
            Self::Output(_) | Self::ClauseOutput(_) | Self::ExportOutput { .. } => {
                Code::OutputFailure
            }
            Self::TransferSelection { code, .. } => *code,
            Self::MissingMembers { .. } | Self::FormatNotRead { .. } => Code::InvalidRequest,
            Self::Limit(_) => Code::ResourceExhausted,
            Self::Identifier { .. } => Code::InvalidIdentifier,
            Self::Native(error) => error.code,
            Self::SynthesisUnsupported { .. } => Code::UnsupportedConstruct,
            Self::Source(error) => error.diagnostic.code,
            Self::ModelUnit(error) => {
                Code::from_code(error.diagnostic.code.as_str()).unwrap_or(Code::InvalidModelBinding)
            }
            Self::ModelPackageInput(refusal) => refusal.code,
            Self::CompleteSelection(_)
            | Self::Libraries(_)
            | Self::CompleteRunSelection(_)
            | Self::EmptyCompileProgram => Code::InvalidRequest,
            Self::ExtractionRequiresRun => Code::ExtractionRequiresRun,
            Self::ExtractionPackageConflict => Code::ExtractionPackageConflict,
            Self::ExtractionClauseCount { .. } => Code::ExtractionClauseCount,
            #[cfg(feature = "quire-extraction")]
            Self::InvalidQuireContext { .. } => Code::InvalidQuireContext,
            #[cfg(feature = "quire-extraction")]
            Self::ExtractionFailure { code, .. } => *code,
            Self::Spine(failure) => failure.refusal.code(),
            Self::CallSite(refusal) => refusal.code(),
            Self::ExtensionCatalog(_) => Code::InvalidPackage,
            Self::UnknownExtensionCapability(_) => Code::UnknownRequiredFeature,
            Self::SpineRun(refusal) => refusal.code(),
            Self::UnknownRequirementClaim { .. } => Code::MissingDeclaration,
            Self::TemporalCorrection(_) => Code::InvalidRuntimeInput,
            Self::AnalyzeRequest { .. } => Code::InvalidRequest,
            Self::AnalyzeMemory { .. } => Code::InvalidRuntimeInput,
            Self::AnalyzePackage { code, .. } => *code,
            Self::AnalyzeNotYetImplemented { .. } => Code::UnsupportedConstruct,
            Self::TlCorrespondence { .. } | Self::IntegerDivisionLaw { .. } => Code::UnknownProfile,
            Self::TlClause { .. } => Code::MissingDeclaration,
            Self::SynthesisSnapshot { record, .. } | Self::TlSubject(record) => {
                Code::from_code(record.catalog.code()).unwrap_or(Code::InvalidRuntimeInput)
            }
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
            Self::AnalyzeEmission(cause) => cause.code(),
            Self::AnalyzeFault | Self::MonitorFault | Self::ItemsFault | Self::MonitorCancelled => {
                Code::RuntimeInvariant
            }
            Self::ReplayRequest(_) => Code::InvalidRequest,
            Self::ReplayBytes { cause, .. } => cause.code(),
            Self::Replay(failure) => failure.refusal.code(),
            Self::ReplayOutput { .. } => Code::OutputFailure,
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
            Self::Output(_)
            | Self::ClauseOutput(_)
            | Self::ExportOutput { .. }
            | Self::TlFault(_) => filament_semantic_value::catalog::Category::InternalFailure,
            Self::CallSite(refusal) => match **refusal {
                fsl_replay::CallSiteRefusal::Fault(_) => {
                    filament_semantic_value::catalog::Category::InternalFailure
                }
                _ => self.code().category(),
            },
            Self::SpineRun(refusal) => refusal.category(),
            // FR-663/TC-1746: an unbounded plan parameter is a check-time
            // admission refusal. Other unsupported projections retain their
            // existing catalog category.
            Self::Spine(failure) if failure.refusal.unbounded_parameter_origin().is_some() => {
                filament_semantic_value::catalog::Category::Refusal
            }
            Self::AnalyzeRefused(refusal) => refusal.document().category(),
            Self::MonitorRefused(refusal) => refusal.document().category(),
            Self::AnalyzeFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::ItemsFault => filament_semantic_value::catalog::Category::InternalFailure,
            Self::MonitorCancelled => filament_semantic_value::catalog::Category::Incomplete,
            Self::ReplayBytes { cause, .. } => cause.category(),
            Self::Replay(failure) => failure.outcome.category(),
            Self::ReplayOutput { .. } => {
                filament_semantic_value::catalog::Category::InternalFailure
            }
            _ => self.code().category(),
        }
    }
}

/// The run cause of a result whose node ids could not be held: the typed
/// emission cause when its package has no graph, an internal failure when an
/// id has no held form.
pub(crate) fn identity_cause(error: fsl_analyze::analyze::document::HeldIdentityError) -> RunCause {
    match error {
        fsl_analyze::analyze::document::HeldIdentityError::NoGraph(cause) => {
            RunCause::AnalyzeEmission(cause)
        }
        fsl_analyze::analyze::document::HeldIdentityError::NotInGraph(_) => RunCause::AnalyzeFault,
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
        self.refusal_document(operation)
            .with_empty_analyze_members()
    }

    fn refusal_document(&self, operation: fsl_replay::Operation) -> fsl_replay::OutcomeDocument {
        use fsl_replay::{OutcomeDiagnostic, OutcomeDocument, OutcomeLocus};

        match &self.cause {
            RunCause::Spine(failure) => OutcomeDocument::new(
                operation,
                Some(failure.refusal.stage().into()),
                self.category(),
            )
            .with_diagnostics(failure.refusal.outcome_diagnostics()),
            RunCause::CallSite(refusal) => OutcomeDocument::new(
                operation,
                Some(fsl_replay::OutcomeStage::I1),
                self.category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::from_call_site_refusal(refusal)]),
            RunCause::SpineRun(refusal) => OutcomeDocument::from_run_refusal(operation, refusal),
            RunCause::AnalyzeRefused(refusal) => refusal.document(),
            RunCause::MonitorRefused(refusal) => refusal.document(),
            RunCause::Replay(failure) => failure.outcome.clone(),
            RunCause::ReplayOutput { outcome, .. } => outcome.as_ref().clone(),
            RunCause::Source(failure) => OutcomeDocument::new(operation, None, self.category())
                .with_diagnostics(vec![OutcomeDiagnostic::from_source_diagnostic(
                    &failure.diagnostic,
                )]),
            RunCause::ModelUnit(failure) => OutcomeDocument::new(
                operation,
                Some(fsl_replay::OutcomeStage::I1),
                self.category(),
            )
            .with_diagnostics(vec![model_unit_outcome_diagnostic(&failure.diagnostic)]),
            RunCause::ModelPackageInput(refusal) => OutcomeDocument::new(
                operation,
                Some(fsl_replay::OutcomeStage::I1),
                self.category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                Some(refusal.cause.as_str()),
                refusal.code.as_str(),
                None,
                refusal.detail.clone(),
            )
            .with_selection(
                fsl_replay::outcome::hint::MissingSelection::of_model_refusal(refusal),
            )]),
            RunCause::SynthesisSnapshot { file, record } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some(record.catalog.cause()),
                        record.catalog.code(),
                        Some(OutcomeLocus::Input {
                            operand: file.clone(),
                            member: None,
                            pointer: Some("/request/synthesis/initial".to_owned()),
                        }),
                        self.to_string(),
                    ),
                ])
            }
            RunCause::SynthesisUnsupported { diagnostic, .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some("not-yet-implemented"),
                        Code::UnsupportedConstruct.as_str(),
                        Some(OutcomeLocus::Located {
                            file: diagnostic.path.clone(),
                            span: diagnostic.span,
                        }),
                        diagnostic.message.clone(),
                    ),
                ])
            }
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
            RunCause::ExportMapper { .. } | RunCause::ExportOutput { .. } => OutcomeDocument::new(
                operation,
                Some(fsl_replay::OutcomeStage::S4),
                self.category(),
            )
            .with_diagnostics(vec![OutcomeDiagnostic::new(
                matches!(self.cause, RunCause::ExportOutput { .. }).then_some("output-failure"),
                self.cause.code().as_str(),
                None,
                self.outcome_message(),
            )]),
            RunCause::ExportRequest { path, cause, .. }
            | RunCause::PlanRequest { path, cause, .. }
            | RunCause::SynthesisRequest { path, cause, .. }
            | RunCause::ModelBasedTestRequest { path, cause, .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some(*cause),
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String(path.clone()),
                    )]))),
                ])
            }
            RunCause::MissingMembers { .. } | RunCause::FormatNotRead { .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(self.cause.request_details()),
                ])
            }
            RunCause::ExportProtocolRequest { path, .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String(path.clone()),
                    )]))),
                ])
            }
            RunCause::ClauseRequest { member, .. } if member == "/request/target" => {
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
            RunCause::MonitorTargetRequest { .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String("/request/target".to_owned()),
                    )]))),
                ])
            }
            RunCause::ClauseRequest { member, .. }
                if member.starts_with("/call/arguments/")
                    || member == "/call/relationships"
                    || member.starts_with("/call/relationships/")
                    || member.starts_with("/call/snapshot/relationships/") =>
            {
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
            | RunCause::RequiresDirectory { .. }
            | RunCause::IntegerDivisionLaw { .. }
            | RunCause::TransferSelection { .. }
            | RunCause::TlCorrespondence { .. }
            | RunCause::TlClause { .. }
            | RunCause::TlSubject(_)
            | RunCause::TlFault(_)
            | RunCause::ClocksGivenTwice
            | RunCause::OutputGivenTwice
            | RunCause::OutputExists { .. }
            | RunCause::Output(_)
            | RunCause::ClauseOutput(_)
            | RunCause::Limit(_)
            | RunCause::Identifier { .. }
            | RunCause::CompleteSelection(_)
            | RunCause::ExtractionRequiresRun
            | RunCause::ExtractionPackageConflict
            | RunCause::ExtractionClauseCount { .. }
            | RunCause::EmptyCompileProgram
            | RunCause::Libraries(_)
            | RunCause::ExtensionCatalog(_)
            | RunCause::UnknownExtensionCapability(_)
            | RunCause::CompleteRunSelection(_)
            | RunCause::AnalyzeRequest { .. }
            | RunCause::AnalyzeUniverse(_)
            | RunCause::AnalyzeEmission(_)
            | RunCause::AnalyzeFault
            | RunCause::MonitorFault
            | RunCause::MonitorCancelled
            | RunCause::ReplayRequest(_)
            | RunCause::ReplayBytes { .. }
            | RunCause::UnknownRequirementClaim { .. }
            | RunCause::TemporalCorrection(_)
            | RunCause::ItemsFault => OutcomeDocument::new(operation, None, self.category())
                .with_diagnostics(vec![OutcomeDiagnostic::new(
                    match &self.cause {
                        RunCause::IntegerDivisionLaw { .. } => Some("unsupported-selection"),
                        RunCause::UnknownExtensionCapability(_) => Some("unknown-feature"),
                        _ => None,
                    },
                    self.cause.code().as_str(),
                    None,
                    self.outcome_message(),
                )]),
            RunCause::AnalyzeMemory { .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some("invalid-value"),
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    ),
                ])
            }
            #[cfg(feature = "quire-extraction")]
            RunCause::ExtractionFailure { stage, details, .. } => {
                let mut details = details.clone();
                details.insert(
                    "stage".to_owned(),
                    serde_json::Value::String(
                        match stage {
                            ExtractionFailureStage::Selection => "selection",
                            ExtractionFailureStage::Quire => "quire",
                        }
                        .to_owned(),
                    ),
                );
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
                        self.cause.code().as_str(),
                        None,
                        self.outcome_message(),
                    )
                    .with_details(Some(details)),
                ])
            }
            RunCause::AnalyzePackage { cause, pointer, .. } => {
                let code = self.cause.code();
                let cause = cause
                    .as_deref()
                    .or((code == Code::MissingImport).then_some("missing-selection"));
                let diagnostic =
                    OutcomeDiagnostic::new(cause, code.as_str(), None, self.outcome_message())
                        .with_selection(
                            (code == Code::MissingImport && cause == Some("missing-selection"))
                                .then_some(
                                    fsl_replay::outcome::hint::MissingSelection::PackageReference,
                                ),
                        );
                // Manifest and strict-reader refusals retain their original pointer.
                let diagnostic = match pointer {
                    Some(pointer) => diagnostic.with_details(Some(serde_json::Map::from_iter([(
                        "path".to_owned(),
                        serde_json::Value::String(pointer.clone()),
                    )]))),
                    _ => diagnostic,
                };
                OutcomeDocument::new(operation, None, self.category())
                    .with_diagnostics(vec![diagnostic])
            }
            RunCause::AnalyzeNotYetImplemented { locus, .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        Some("not-yet-implemented"),
                        self.cause.code().as_str(),
                        locus.as_ref().map(|occurrence| OutcomeLocus::Occurrence {
                            occurrence: occurrence.clone(),
                        }),
                        self.outcome_message(),
                    ),
                ])
            }
            #[cfg(feature = "quire-extraction")]
            RunCause::InvalidQuireContext { .. } => {
                OutcomeDocument::new(operation, None, self.category()).with_diagnostics(vec![
                    OutcomeDiagnostic::new(
                        None,
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

/// Read ceiling for check's source: the fixed intake ceiling, raised to the
/// S1 limit so `s1.input_bytes` is always the bound that refuses.
fn check_source_read_ceiling(s1_source_bytes: usize) -> usize {
    TOTAL_BYTES.max(s1_source_bytes)
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
    additional_files: usize,
}

#[derive(Clone, Copy)]
enum DependentLimit {
    Fixed,
    CallerConfigured,
}

impl Intake<'_> {
    fn additional_file(&mut self) -> Result<()> {
        if self.additional_files == 0 {
            return Err(RunCause::Limit(LimitKind::SelectedFiles {
                category: FileCategory::Program,
                requested: 1,
                remaining: 0,
                maximum: FILES,
            }));
        }
        self.additional_files -= 1;
        Ok(())
    }
    fn file(&mut self, path: &str, ceiling: usize) -> Result<Vec<u8>> {
        if let Some(bytes) = self.prefetched_observations.remove(path) {
            return Ok(bytes);
        }
        self.bounded_file(path, ceiling)
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
        self.admit_source(selected, identity, bytes, self.maximum)
    }

    /// Reads a source under `ceiling` (see `check_source_read_ceiling`) without
    /// charging the intake's budget (check's `dependent_bytes`, FR-866); the stage-1
    /// `s1.input_bytes` ceiling is then enforced by the parse, as in compile.
    fn source_outside_budget(
        &mut self,
        selected: &wire::SourceFile,
        request_member: &str,
        ceiling: usize,
    ) -> Result<FormalSource> {
        let identity = selected.identity.bind(request_member)?;
        let bytes = read_file(&self.directory.join(&selected.file), ceiling)?;
        self.admit_source(selected, identity, bytes, ceiling)
    }

    fn admit_source(
        &mut self,
        selected: &wire::SourceFile,
        identity: crate::formal_source::SourceIdentities,
        bytes: Vec<u8>,
        byte_limit: usize,
    ) -> Result<FormalSource> {
        let source = Source::read_typed(identity.native, &selected.file, &bytes, byte_limit)
            .map_err(|refusal| {
                RunCause::Source(Box::new(SourceFailure {
                    native: fsl_foundation::diagnostic::source_refusal(
                        &refusal, &bytes, byte_limit,
                    ),
                    diagnostic: fsl_cst::diagnostic::source_refusal(*refusal, byte_limit),
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
/// Function calls use FR-181's canonical argument and completed-value forms.
/// Verification plans and independent selections run through their existing
/// coordinators, preserving their dispositions and aggregate exit category.
/// Drivers dispatch the run verb here rather than through function-only
/// preparation and execution.
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

/// The request member an explicit export invocation requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportSelection {
    /// Generate target text from the request's `export` member.
    Ordinary,
    /// Admit the request's `export_evidence` member.
    Evidence,
}

/// Read an authored export request exactly once under the caller's byte
/// ceilings, require its explicit selection, and execute those admitted
/// bytes. Relative dependent paths use the request operand's parent.
///
/// Raw duplicate names and a non-closed envelope refuse before selection or
/// dependent file reads. Every admitted selection returns a lifecycle
/// outcome. Evidence settlement remains an unsupported capability.
pub fn export_with_limits_and_overrides(
    path: &Path,
    selection: ExportSelection,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
) -> std::result::Result<RunResult, Box<RunError>> {
    with_checked_request(
        path,
        limits.request_bytes,
        export::require_object_request,
        |directory, bytes| {
            export::run_selected_bytes(directory, bytes, selection, limits, overrides)
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
    run_with_providers_and_limits_and_overrides(
        path,
        RunLimits::default(),
        &RequestOverrides::default(),
        negotiate,
        providers,
        options,
        cancel,
    )
}

/// Run with providers, caller-selected request/dependent byte ceilings, and
/// request members supplied beside the request file (FR-312, FR-640).
/// This is the provider-backed counterpart of
/// [`run_with_limits_and_overrides`].
pub fn run_with_providers_and_limits_and_overrides(
    path: &Path,
    limits: RunLimits,
    overrides: &RequestOverrides<'_>,
    negotiate: &dyn Negotiate,
    providers: Vec<fsl_analyze::analyze::Engine>,
    options: &ProviderOptions,
    cancel: &filament_exact::Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    run_configured(
        path,
        limits,
        overrides,
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
        request_shape::admit(request_shape::Family::Run, bytes)?;
        if monitor::is_monitor_request(bytes) {
            monitor::run_bytes(directory, bytes, limits.dependent_bytes, &settle).map(Some)
        } else if replay::is_replay_request(bytes) {
            // FR-862: a `native-replay/1` request replays its FR-336 request.
            replay::run_bytes(
                directory,
                path.to_str(),
                bytes,
                limits.dependent_bytes,
                &CommandLimits::replay(),
            )
            .map(Some)
        } else {
            // FR-1045 closes the relationship selection. Check duplicate
            // names before special request readers (notably export's unique
            // JSON admission) normalize the raw request and lose the later
            // member's JSON Pointer.
            if let Some(member) =
                relationships::duplicate_selection_member(bytes).map_err(RunCause::Json)?
            {
                return Err(RunCause::ClauseRequest {
                    member,
                    reason: "member is repeated".to_owned(),
                });
            }
            if let Some(result) = export::run_bytes(directory, bytes, limits, overrides)? {
                return Ok(Some(result));
            }
            plan::check_member(bytes)?;
            if let Some(result) = plan::run_bytes(directory, bytes, limits, overrides, &settle)? {
                return Ok(Some(result));
            }
            if let Some(result) = version_argv(bytes)? {
                Ok(Some(result))
            } else if let Some(result) = check_argv(directory, bytes)? {
                Ok(Some(result))
            } else if let Some(result) = synthesis::run_bytes(directory, bytes, limits, overrides)?
            {
                Ok(Some(result))
            } else {
                if let Some(result) =
                    items::run_bytes(directory, bytes, limits, overrides, &settle)?
                {
                    Ok(Some(result))
                } else {
                    #[cfg(feature = "quire-extraction")]
                    if let Some(result) = run_extracted_clause_bytes(
                        directory,
                        bytes,
                        limits.dependent_bytes,
                        overrides,
                    )? {
                        return Ok(Some(result));
                    }
                    Ok(None)
                }
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

#[cfg(feature = "quire-extraction")]
fn run_extracted_clause_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    overrides: &RequestOverrides<'_>,
) -> Result<Option<RunResult>> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    if !value
        .pointer("/request/program/extraction")
        .is_some_and(|extraction| !extraction.is_null())
    {
        return Ok(None);
    }
    let request =
        read_clause_run_request_bytes(directory, bytes, Some(dependent_bytes), overrides)?;
    let (extraction, map) = extraction_record(&request, &value)?;
    let selection = request.selection.clone();
    let report =
        fsl_replay::spine::run_clause(request).map_err(|error| RunCause::ClauseRequest {
            member: "clause".to_owned(),
            reason: error.to_string(),
        })?;
    output::clause_report_with_extraction(report, &selection, extraction, &map).map(Some)
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
    let mut markdown = false;
    let mut format = None;
    let mut extensions_file = None;
    let mut positionals = Vec::new();
    let mut index = 1;
    let mut options_enabled = true;
    while index < arguments.len() {
        if options_enabled && arguments[index] == "--" {
            options_enabled = false;
            index += 1;
            continue;
        }
        if !options_enabled {
            positionals.push(arguments[index]);
            index += 1;
            continue;
        }
        match arguments[index] {
            "--markdown" => {
                if markdown {
                    return Err(check_option_refusal(
                        "--markdown",
                        "is given more than once",
                    ));
                }
                markdown = true;
                index += 1;
            }
            option @ ("--format" | "--extensions") => {
                let slot = if option == "--format" {
                    &mut format
                } else {
                    &mut extensions_file
                };
                if slot.is_some() {
                    return Err(check_option_refusal(option, "is given more than once"));
                }
                match arguments.get(index + 1) {
                    Some(value) if !value.starts_with("--") => *slot = Some(*value),
                    _ => return Err(check_option_refusal(option, "expects a value")),
                }
                index += 2;
            }
            argument if argument.starts_with("--") => {
                return Err(check_option_refusal(argument, "is not recognized"));
            }
            argument => {
                positionals.push(argument);
                index += 1;
            }
        }
    }
    if format.is_some_and(|format| format != "json") {
        return Err(RunCause::ClauseRequest {
            member: "format".to_owned(),
            reason: "check supports only json".to_owned(),
        });
    }
    let (identity, source) = match positionals.as_slice() {
        [source] if !markdown => (None, *source),
        [authority, name, source] => (Some((*authority, *name)), *source),
        _ => return Err(check_argv_refusal()),
    };
    let extensions = match extensions_file {
        Some(set_path) => {
            let set_bytes = read_file(&directory.join(set_path), TOTAL_BYTES)?;
            let set: ExtensionSetFile = serde_json::from_slice(&set_bytes)?;
            if set.format != "filament.extension-set/v1" {
                return Err(RunCause::ClauseRequest {
                    member: "format".to_owned(),
                    reason: "expected filament.extension-set/v1".to_owned(),
                });
            }
            set.extensions
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
                .collect()
        }
        None => Vec::new(),
    };
    if markdown {
        let (authority, name) = identity.expect("markdown requires the document labels");
        return check_markdown_argv(directory, authority, name, source, extensions).map(Some);
    }
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

#[cfg(feature = "quire-extraction")]
fn check_markdown_argv(
    directory: &Path,
    authority: &str,
    name: &str,
    source: &str,
    extensions: Vec<CheckExtensionInput>,
) -> Result<RunResult> {
    let document = check_markdown_document(directory, authority, name, source, extensions)?;
    let exit_code = document.category().exit_code();
    let document = serde_json::to_value(&document).map_err(RunCause::Output)?;
    Ok(RunResult {
        exit_code,
        value: NativeResult::check_document(document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    })
}

#[cfg(not(feature = "quire-extraction"))]
fn check_markdown_argv(
    _directory: &Path,
    _authority: &str,
    _name: &str,
    _source: &str,
    _extensions: Vec<CheckExtensionInput>,
) -> Result<RunResult> {
    Err(RunCause::ClauseRequest {
        member: "--markdown".to_owned(),
        reason: "this build of fsl-command has no Markdown extraction (feature quire-extraction)"
            .to_owned(),
    })
}

fn check_option_refusal(option: &str, reason: &str) -> RunCause {
    RunCause::ClauseRequest {
        member: option.to_owned(),
        reason: format!("check option {option} {reason}"),
    }
}

fn check_argv_refusal() -> RunCause {
    RunCause::ClauseRequest {
        member: "argv".to_owned(),
        reason: "check expects [--markdown] [--extensions <file>] [--format json] a source, or <authority> <name> <source> (--markdown requires the labels)"
            .to_owned(),
    }
}

/// FR-662, FSpec FR-563: check every `ix:native` fence of one Markdown
/// document, naming each body `<identity>#<clause id>`.
#[cfg(feature = "quire-extraction")]
fn check_markdown_document(
    directory: &Path,
    authority: &str,
    name: &str,
    source: &str,
    extensions: Vec<CheckExtensionInput>,
) -> Result<fsl_replay::OutcomeDocument> {
    let wire_extensions = extensions
        .iter()
        .map(|extension| wire::Extension {
            authority: extension.authority.clone(),
            identity: extension.identity.clone(),
            file: extension
                .file
                .strip_prefix(directory)
                .unwrap_or(&extension.file)
                .to_string_lossy()
                .into_owned(),
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
    validate_extension_entries(&wire_extensions)?;
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: 1,
            models: 0,
            observations: 0,
            extensions: extensions.len(),
        },
        CheckInputs::default().dependent_bytes,
    )?;
    let document_source = intake.source_outside_budget(
        &check_source_file(Path::new(source), authority, name, "CheckSource"),
        "source",
        TOTAL_BYTES,
    )?;
    let catalog = if wire_extensions.is_empty() {
        None
    } else {
        Some(extension_catalog(&wire_extensions, &mut intake)?)
    };
    let packages = fsl_semantics::model::intake::PackageInput::new();
    let dependencies = fsl_replay::spine::DependencyInput::new(Vec::new()).map_err(|refusal| {
        RunCause::ClauseRequest {
            member: "libraries".to_owned(),
            reason: format!("{refusal:?}"),
        }
    })?;
    let request = fsl_replay::spine::NativeFenceRequest {
        original: document_source.source(),
        packages: &packages,
        dependencies: &dependencies,
        extensions: catalog
            .as_ref()
            .map(|catalog| fsl_replay::spine::ProfileCatalogInput {
                catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            }),
        integer_division: None,
    };
    Ok(
        match fsl_replay::spine::check_native_fences(
            &request,
            fsl_replay::spine::CheckLimits::default(),
            &filament_exact::Cancel::new(),
        ) {
            Ok(report) => fsl_replay::OutcomeDocument::from_native_fences(&report.into_value()),
            Err(failure) => fsl_replay::OutcomeDocument::from_native_fence_failure(&failure),
        },
    )
}

/// Inspect a checked-package handoff and return the command's successful
/// `inspect` outcome. A refusal retains the handoff reader's typed error.
pub fn inspect_handoff(directory: &Path) -> fsl_replay::OutcomeDocument {
    inspect_handoff_with_operand(directory, &directory.to_string_lossy())
}

/// Inspect a checked-package handoff while retaining the caller's exact
/// directory operand for refusal loci.
pub fn inspect_handoff_with_operand(
    directory: &Path,
    operand: &str,
) -> fsl_replay::OutcomeDocument {
    inspect_handoff_result(
        crate::checked_handoff::verify(directory),
        directory,
        operand,
    )
}

fn inspect_handoff_result(
    result: std::result::Result<
        Box<filament_contract_model::CheckedPackageV2>,
        crate::checked_handoff::Error,
    >,
    directory: &Path,
    operand: &str,
) -> fsl_replay::OutcomeDocument {
    use filament_semantic_value::catalog::Category;
    use fsl_replay::{Operation, OutcomeDiagnostic, OutcomeDocument, OutcomeLocus};

    match result {
        Ok(_) => OutcomeDocument::new(Operation::Inspect, None, Category::Success),
        Err(error) => {
            let (member, pointer) = inspect_handoff_position(directory, &error);
            let locus = Some(OutcomeLocus::Input {
                operand: operand.to_owned(),
                member,
                pointer,
            });
            let category = inspect_handoff_category(&error);
            OutcomeDocument::new(Operation::Inspect, None, category).with_diagnostics(vec![
                OutcomeDiagnostic::new(None, error.code_name(), locus, error.to_string()),
            ])
        }
    }
}

fn inspect_handoff_category(
    error: &crate::checked_handoff::Error,
) -> filament_semantic_value::catalog::Category {
    use filament_semantic_value::catalog::Category;

    if matches!(
        error,
        crate::checked_handoff::Error::ReaderHostAllocationFailed(_)
    ) {
        Category::InternalFailure
    } else {
        Category::Refusal
    }
}

fn inspect_handoff_position(
    directory: &Path,
    error: &crate::checked_handoff::Error,
) -> (Option<String>, Option<String>) {
    use crate::checked_handoff::{Error, SelectionReadError};

    match error {
        Error::ReaderRefused(refusal) => (
            Some("checked-package.json".to_owned()),
            refusal
                .loci
                .first()
                .and_then(|locus| locus.path.as_ref())
                .map(ToString::to_string),
        ),
        Error::ReaderIncomplete(_)
        | Error::ReaderCatalogUnavailable(_)
        | Error::ReaderHostAllocationFailed(_) => (Some("checked-package.json".to_owned()), None),
        Error::Read { path, .. } => {
            let path = Path::new(path);
            if path == directory {
                return (None, None);
            }
            let member = path
                .strip_prefix(directory)
                .ok()
                .map(|member| member.to_string_lossy().replace('\\', "/"));
            (member, None)
        }
        Error::Label { label } => (Some(label.clone()), None),
        Error::Sidecar(SelectionReadError::Format(_)) => (
            Some("selection.json".to_owned()),
            Some("/format".to_owned()),
        ),
        Error::Sidecar(SelectionReadError::Malformed(_)) => {
            (Some("selection.json".to_owned()), None)
        }
        Error::SelectionMismatch(_) => (Some("selection.json".to_owned()), None),
        _ => (None, None),
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
    let mut sources = crate::Sources::default();
    with_request_limits(path, limits.request_bytes, |directory, bytes| {
        request_shape::admit(request_shape::Family::RunReader, bytes)?;
        prepare_run_bytes_with_sources(
            directory,
            bytes,
            limits.dependent_bytes,
            overrides,
            &mut sources,
        )
    })
    .map_err(|mut error| {
        error.sources = sources;
        error
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
        request_shape::admit(request_shape::Family::RunReader, bytes)?;
        read_clause_run_request_bytes(directory, bytes, None, &RequestOverrides::default())
    })
}

fn read_clause_run_request_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: Option<usize>,
    overrides: &RequestOverrides<'_>,
) -> Result<fsl_replay::spine::ClauseRunRequest> {
    let observation_override = overrides.observation.copied();
    let mut request: wire::Request = request_with_overrides(bytes, overrides)?;
    apply_accounting_override(&mut request, overrides);
    let program = required_program(&request)?;
    #[cfg(feature = "quire-extraction")]
    if program.extraction.is_some() && request.package.is_some() {
        return Err(RunCause::ExtractionPackageConflict);
    }
    #[cfg(feature = "quire-extraction")]
    if program.extraction.is_some() && request.call.is_some() {
        return Err(RunCause::ExtractionRequiresRun);
    }
    #[cfg(feature = "quire-extraction")]
    if program.extraction.is_some() && program.clauses.len() != 1 {
        return Err(RunCause::ExtractionClauseCount {
            actual: program.clauses.len(),
        });
    }
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
    let counts = FileCounts {
        programs: request.libraries.len().saturating_add(1),
        models: request.models.len(),
        observations: clause
            .observations
            .values()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        extensions: request.extensions.len(),
    };
    let mut intake = match dependent_bytes {
        Some(maximum) => intake_with_bytes(directory, counts, maximum)?,
        None => intake(directory, counts)?,
    };
    reject_current_parameters_before_sources(&mut intake, request.clause.as_ref())?;
    let source = intake.source(&program.source, "program.source")?;
    #[cfg(feature = "quire-extraction")]
    if let Some(extraction) = &program.extraction {
        let binding = &program.clauses[0];
        let context = fsl_source::clause_context(&binding.package, source.source())
            .map_err(|diagnostics| RunCause::InvalidQuireContext { diagnostics })?;
        let body_identity = fsl_foundation::SourceIdentity::new(
            extraction.body.authority.clone(),
            extraction.body.identity.clone(),
        );
        let extracted = match fsl_source::extract(
            source.source().clone(),
            &context,
            fsl_source::Selection {
                clause_id: binding.clause.clone(),
                package: binding.package.clone(),
                body: body_identity,
            },
            fsl_source::Limits::default(),
        ) {
            Ok(extracted) => extracted,
            Err(error) => return Err(extraction_failure(error)?),
        };
        return clause::read_extracted(
            &mut intake,
            &request,
            clause,
            source,
            extracted,
            overrides.limits.copied().unwrap_or_default(),
            observation_override,
        );
    } else {
        require_native_header(source.source())?;
    }
    #[cfg(not(feature = "quire-extraction"))]
    require_native_header(source.source())?;
    clause::read(
        &mut intake,
        &request,
        clause,
        source,
        overrides.limits.copied().unwrap_or_default(),
        observation_override,
    )
}

#[cfg(feature = "quire-extraction")]
fn extraction_failure(error: Box<fsl_source::Error>) -> Result<RunCause> {
    let code = error.code();
    let stage = match &error.cause {
        fsl_source::Cause::Preflight(_) => ExtractionFailureStage::Selection,
        fsl_source::Cause::Join(_)
            if error.extraction().is_some_and(|outcome| {
                outcome.availability.state != fsl_source::AvailabilityState::Available
            }) =>
        {
            ExtractionFailureStage::Quire
        }
        fsl_source::Cause::Join(_) => ExtractionFailureStage::Selection,
    };
    let original = error.original();
    let identity = original.identity();
    let selection = error.selection();
    let mut details = serde_json::Map::from_iter([
        (
            "original".to_owned(),
            serde_json::json!({
                "authority": identity.authority,
                "identity": identity.identity,
            }),
        ),
        (
            "path".to_owned(),
            serde_json::Value::String(original.path().to_owned()),
        ),
        (
            "body".to_owned(),
            serde_json::json!({
                "authority": selection.body.authority,
                "identity": selection.body.identity,
            }),
        ),
        (
            "binding".to_owned(),
            serde_json::json!({
                "clause": selection.clause_id,
                "package": selection.package,
            }),
        ),
    ]);
    if let Some(outcome) = error.extraction() {
        let mut value = serde_json::Map::from_iter([
            (
                "availability".to_owned(),
                serde_json::to_value(&outcome.availability).map_err(RunCause::Output)?,
            ),
            (
                "clause_text".to_owned(),
                serde_json::to_value(&outcome.clause_text).map_err(RunCause::Output)?,
            ),
            (
                "diagnostics".to_owned(),
                serde_json::to_value(&outcome.diagnostics).map_err(RunCause::Output)?,
            ),
        ]);
        if let Some(clauses) = &outcome.clauses {
            value.insert(
                "clauses".to_owned(),
                serde_json::to_value(clauses).map_err(RunCause::Output)?,
            );
        }
        details.insert("outcome".to_owned(), serde_json::Value::Object(value));
    }
    Ok(RunCause::ExtractionFailure {
        code,
        stage,
        details,
        message: error.to_string(),
    })
}

#[cfg(feature = "quire-extraction")]
fn extraction_record(
    request: &fsl_replay::spine::ClauseRunRequest,
    envelope: &serde_json::Value,
) -> Result<(serde_json::Value, fsl_foundation::source_map::SourceMap)> {
    let fsl_replay::spine::ClauseRunSource::Extracted(extracted) = &request.source else {
        return Err(RunCause::ClauseRequest {
            member: "program.extraction".to_owned(),
            reason: "extraction request did not produce an extracted source".to_owned(),
        });
    };
    let body_document = envelope
        .pointer("/request/program/extraction/body/document")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "program.extraction.body.document".to_owned(),
            reason: "missing body document identity".to_owned(),
        })?;
    let binding = envelope
        .pointer("/request/program/clauses/0")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "program.clauses[0]".to_owned(),
            reason: "missing extraction binding".to_owned(),
        })?;
    let clause = binding
        .get("clause")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "program.clauses[0].clause".to_owned(),
            reason: "missing extraction clause".to_owned(),
        })?;
    let package = binding
        .get("package")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "program.clauses[0].package".to_owned(),
            reason: "missing extraction package".to_owned(),
        })?;
    let map = extracted.map();
    let original = map.original();
    let body = map.body();
    let outcome = extracted.extraction();
    let mut outcome_value = serde_json::Map::from_iter([
        (
            "availability".to_owned(),
            serde_json::to_value(&outcome.availability).map_err(RunCause::Output)?,
        ),
        (
            "clause_text".to_owned(),
            serde_json::to_value(&outcome.clause_text).map_err(RunCause::Output)?,
        ),
        (
            "diagnostics".to_owned(),
            serde_json::to_value(&outcome.diagnostics).map_err(RunCause::Output)?,
        ),
    ]);
    if let Some(clauses) = &outcome.clauses {
        outcome_value.insert(
            "clauses".to_owned(),
            serde_json::to_value(clauses).map_err(RunCause::Output)?,
        );
    }
    let segments: Vec<_> = map
        .segments()
        .iter()
        .map(|segment| {
            serde_json::json!({
                "body_start": segment.body.start,
                "body_end": segment.body.end,
                "original_start": segment.original.start,
                "original_end": segment.original.end,
            })
        })
        .collect();
    let record = serde_json::json!({
        "original": {
            "authority": original.identity().authority,
            "identity": original.identity().identity,
            "path": original.path(),
            "bytes": original.text().as_bytes(),
        },
        "body": {
            "authority": body.identity().authority,
            "identity": body.identity().identity,
            "document": body_document,
            "bytes": body.text().as_bytes(),
        },
        "binding": {"clause": clause, "package": package},
        "outcome": outcome_value,
        "map": {
            "region": {"start": map.region().start, "end": map.region().end},
            "segments": segments,
        }
    });
    Ok((record, map.clone()))
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
        request_shape::admit(request_shape::Family::Compile, bytes)?;
        let request: wire::CompileRequest = request_with_overrides(bytes, overrides)?;
        // FR-767: a malformed or unknown law refuses before any file is read.
        let integer_division = request_integer_division(request.integer_division.clone())?;
        if !request.program.sources.is_empty()
            && request_call_site(request.call_site.as_ref(), request.program.sources.len())?
                .is_some()
            && fsl_semantics::library::LibraryName::new(
                request.program.sources[0].identity.identity.as_str(),
            )
            .is_err()
        {
            return Err(RunCause::ClauseRequest {
                member: "call_site".to_owned(),
                reason: "program identity must be a library identity".to_owned(),
            });
        }
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
        if let (Some(handoff), Some(output)) = (&request.handoff, &output) {
            let handoff_directory = directory.join(&handoff.directory);
            if resolved_path_for_comparison(output)
                .zip(resolved_path_for_comparison(&handoff_directory))
                .is_some_and(|(output, handoff)| output.starts_with(&handoff))
            {
                return Err(RunCause::ClauseRequest {
                    member: "output and handoff".to_owned(),
                    reason: format!(
                        "output path {} is inside handoff directory {}",
                        output.display(),
                        handoff_directory.display()
                    ),
                });
            }
        }
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
                .as_ref()
                .map_or(0, Vec::len)
                .saturating_add(request.program.sources.len()),
            models: request.models.as_ref().map_or(0, Vec::len),
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
            require_native_header(source.source())?;
            sources.push(source);
        }
        if sources.len() == 1
            && fsl_semantics::model::intake::is_model_unit_source(
                sources[0].source().text().as_bytes(),
            )
        {
            for (member, present) in [
                ("models", request.models.is_some()),
                ("libraries", request.libraries.is_some()),
                ("clocks", request.clocks.is_some()),
                ("handoff", request.handoff.is_some()),
                (
                    "refinement_internals",
                    request.refinement_internals.is_some(),
                ),
                ("integer_division", request.integer_division.is_some()),
                ("call_site", request.call_site.is_some()),
            ] {
                if present {
                    return Err(RunCause::ClauseRequest {
                        member: member.to_owned(),
                        reason: "a model unit compiles only its own declarations".to_owned(),
                    });
                }
            }
            let compiled = compile_formal_model_unit(&sources[0])?;
            let document = fsl_replay::OutcomeDocument::new(
                fsl_replay::Operation::Package,
                Some(fsl_replay::OutcomeStage::I1),
                filament_semantic_value::catalog::Category::Success,
            );
            let mut outcome = CompileOutcome {
                package_bytes: compiled.bytes().to_vec(),
                document,
                handoff_compiled: None,
                handoff_temporal_selections: Vec::new(),
                sources: crate::Sources::default(),
            };
            outcome.sources = crate::Sources::from_sources(intake.sources.iter().cloned());
            let written = output
                .as_deref()
                .map(|output| output.to_string_lossy().into_owned());
            if let Some(output) = &output {
                output_file::write_new_file(output, &outcome.package_bytes)?;
            }
            outcome.document =
                outcome
                    .document
                    .with_packaged(outcome.package_bytes.len(), written, None);
            return Ok(outcome);
        }
        if request.models.is_none() {
            return Err(RunCause::ClauseRequest {
                member: "models".to_owned(),
                reason: "is required for a complete-V1 program".to_owned(),
            });
        }
        let mut outcome = complete(
            &request,
            &mut intake,
            &sources,
            overrides.limits,
            integer_division,
        )?;
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
        let written = output
            .as_deref()
            .map(|output| output.to_string_lossy().into_owned());
        if let Some(output) = &output {
            output_file::write_new_file(output, &outcome.package_bytes)?;
        }
        let handoff = request.handoff.as_ref().map(|handoff| {
            directory
                .join(&handoff.directory)
                .to_string_lossy()
                .into_owned()
        });
        outcome.document =
            outcome
                .document
                .with_packaged(outcome.package_bytes.len(), written, handoff);
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

/// Resolve existing path components before normalizing `..`, so a symlinked
/// parent keeps the same meaning it will have when the output is opened. The
/// final output path may not exist yet, so unresolved components remain lexical.
fn resolved_path_for_comparison(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut resolved = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().ok()?
    };
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => resolved.push(prefix.as_os_str()),
            Component::RootDir => resolved.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                let candidate = resolved.join(name);
                resolved = match std::fs::canonicalize(&candidate) {
                    Ok(canonical) => canonical,
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                        ) =>
                    {
                        candidate
                    }
                    Err(_) => return None,
                };
            }
        }
    }
    Some(resolved)
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
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|model| directory.join(&model.source.file))
        .collect();
    let libraries = request
        .libraries
        .as_deref()
        .unwrap_or_default()
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

/// Whether a request's selected model uses a supported source or package format.
#[ix_trace_rs::string_edge]
fn is_model_input_format(model: &wire::Model) -> bool {
    matches!(
        model.format.as_str(),
        DOMAIN_PACKAGE_PROFILE | "native-model-unit/1"
    )
}

/// Refuse a header language or edition the spine does not admit, at the declared literal.
#[ix_trace_rs::string_edge]
fn require_native_header(source: &Source) -> Result<()> {
    if let Some(declared) = fsl_cst::declared_language(source.text()) {
        if declared.language != "ix:native" {
            return Err(RunCause::Native(fsl_foundation::diagnostic::error(
                source,
                Code::UnknownLanguage,
                fsl_foundation::Phase::Source,
                declared.span.start,
                declared.span.end,
                format!(
                    "{} declares language {:?}; this command reads \"ix:native\"",
                    source.path(),
                    declared.language
                ),
            )));
        }
    }
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

fn model_unit_refusal(source: &Source, diagnostic: fsl_forms::cst::CompleteDiagnostic) -> RunCause {
    let span = diagnostic
        .byte_span()
        .unwrap_or(fsl_foundation::Span { start: 0, end: 0 });
    let code = Code::from_code(diagnostic.code.as_str()).unwrap_or(Code::InvalidModelBinding);
    let native = fsl_foundation::diagnostic::error(
        source,
        code,
        diagnostic.phase,
        span.start,
        span.end,
        diagnostic.message.clone(),
    );
    RunCause::ModelUnit(Box::new(ModelUnitFailure {
        native,
        diagnostic: Box::new(diagnostic),
    }))
}

fn model_unit_outcome_diagnostic(
    diagnostic: &fsl_forms::cst::CompleteDiagnostic,
) -> fsl_replay::OutcomeDiagnostic {
    let locus = diagnostic.source_region().map(|region| {
        fsl_replay::OutcomeLocus::from(&fsl_foundation::diagnostic::Locus::Region(
            region.clone(),
        ))
    });
    fsl_replay::OutcomeDiagnostic::new(
        Some(diagnostic.cause().as_str()),
        diagnostic.code.as_str(),
        locus,
        diagnostic.message.clone(),
    )
    .with_source_label(diagnostic.cause().identity_label())
}

fn compile_formal_model_unit(
    source: &FormalSource,
) -> std::result::Result<fsl_semantics::model::intake::CompiledModelUnit, RunCause> {
    let parsed =
        fsl_forms::cst::parse_source(source.source().clone(), fsl_forms::cst::Limits::default())
            .map_err(|diagnostic| model_unit_refusal(source.source(), *diagnostic))?;
    fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .next()
            .map(|diagnostic| model_unit_refusal(source.source(), diagnostic))
            .unwrap_or_else(|| RunCause::ClauseRequest {
                member: "models".to_owned(),
                reason: "model-unit compilation returned no diagnostic".to_owned(),
            })
    })
}

fn malformed_model_unit_format(source: &Source) -> RunCause {
    model_unit_format_refusal(
        source,
        "a native model unit was selected as a semantic-ir/2.0.0 document",
    )
}

fn package_input_from_formal_sources(
    selections: &[(String, PathBuf)],
    sources: &[FormalSource],
) -> Result<fsl_semantics::model::intake::PackageInput> {
    let mut packages = fsl_semantics::model::intake::PackageInput::new();
    for ((selected_identity, selected_path), source) in selections.iter().zip(sources) {
        let bytes = if fsl_semantics::model::intake::is_model_unit_source(
            source.source().text().as_bytes(),
        ) {
            let relative_path = selected_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| RunCause::ClauseRequest {
                    member: "models".to_owned(),
                    reason: "a model-unit source needs a relative file name".to_owned(),
                })?;
            let model_source = Source::read_typed(
                source.source().identity().clone(),
                relative_path,
                source.source().text().as_bytes(),
                fsl_cst::Limits::default().source_bytes,
            )
            .map_err(|_refusal| RunCause::ClauseRequest {
                member: "models".to_owned(),
                reason: "model-unit source could not be re-read with a relative path".to_owned(),
            })?;
            fsl_semantics::model::intake::package_input_from_sources([(
                selected_identity.as_str(),
                &model_source,
            )])
            .map_err(|refusal| RunCause::ClauseRequest {
                member: "models".to_owned(),
                reason: refusal.detail,
            })?
        } else {
            fsl_semantics::model::intake::package_input([source.source().text().as_bytes()])
                .map_err(|refusal| RunCause::ClauseRequest {
                    member: "models".to_owned(),
                    reason: refusal.detail,
                })?
        };
        for (identity, document) in bytes {
            if packages.insert(identity.clone(), document).is_some() {
                return Err(RunCause::ClauseRequest {
                    member: "models".to_owned(),
                    reason: format!("two supplied model documents select {identity:?}"),
                });
            }
        }
    }
    Ok(packages)
}

fn package_input_from_selected_models(
    models: &[wire::Model],
    sources: &[FormalSource],
) -> Result<fsl_semantics::model::intake::PackageInput> {
    let documents = models
        .iter()
        .zip(sources)
        .map(|(model, source)| {
            let bytes = source.source().text().as_bytes();
            match model.format.as_str() {
                "native-model-unit/1" => {
                    if !fsl_semantics::model::intake::is_model_unit_source(bytes) {
                        return Err(model_unit_format_refusal(
                            source.source(),
                            "native-model-unit/1 requires a model unit with a domain identity",
                        ));
                    }
                    compile_formal_model_unit(source).map(|compiled| compiled.bytes().to_vec())
                }
                DOMAIN_PACKAGE_PROFILE => {
                    if fsl_semantics::model::intake::is_model_unit_source(bytes) {
                        return Err(malformed_model_unit_format(source.source()));
                    }
                    Ok(bytes.to_vec())
                }
                _ => Err(RunCause::CompleteSelection(CompleteSelection::NativeModels)),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    fsl_semantics::model::intake::package_input(documents.iter().map(Vec::as_slice))
        .map_err(|refusal| RunCause::ModelPackageInput(Box::new(refusal)))
}

fn model_unit_format_refusal(source: &Source, message: &str) -> RunCause {
    let text = source.text();
    let start = text.find("domain").unwrap_or(0);
    let end = text[start..]
        .find(';')
        .map_or(start, |offset| start + offset + 1);
    let diagnostic = *fsl_forms::cst::diagnostic::error(
        source,
        fsl_forms::cst::CompleteCode::InvalidModelBinding,
        fsl_forms::cst::CompleteCause::MalformedDeclaration,
        fsl_foundation::Phase::Parse,
        start,
        end,
        message,
    );
    model_unit_refusal(source, diagnostic)
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
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
) -> Result<CompileOutcome> {
    let limits = supplied_limits.copied().unwrap_or_default();
    if sources.len() == 1 {
        return complete_single(request, intake, &sources[0], limits, integer_division);
    }
    complete_units(request, intake, sources, limits, integer_division)
}

fn complete_single(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    source: &FormalSource,
    limits: fsl_replay::spine::SpineLimits,
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
) -> Result<CompileOutcome> {
    complete_sources(
        request,
        intake,
        std::slice::from_ref(source),
        false,
        limits,
        integer_division,
    )
}

/// Compile FR-480's ordered program-unit list as one checked package.
fn complete_units(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    limits: fsl_replay::spine::SpineLimits,
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
) -> Result<CompileOutcome> {
    complete_sources(request, intake, sources, true, limits, integer_division)
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
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
) -> Result<CompileOutcome> {
    check_sources(
        request,
        intake,
        sources,
        unit_list,
        limits,
        integer_division,
        None,
        |_intake,
         checked,
         selections,
         points,
         packages,
         _,
         libraries,
         models,
         inventory,
         failure| {
            let cancel = filament_exact::Cancel::new();
            let result = fsl_replay::spine::package_with_temporal_selections(
                &checked,
                selections,
                limits.package,
                &cancel,
            );
            let document = fsl_replay::OutcomeDocument::from_package(&result);
            let emitted = result.map_err(failure)?.into_value();
            if let Some(selection) = request_call_site(request.call_site.as_ref(), sources.len())? {
                let program_identity = package_name(sources[0].source().identity());
                fsl_replay::locate_in_checked(&checked, &program_identity, &selection)
                    .map_err(RunCause::CallSite)?;
            }
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
    let graph = checked.package().graph();
    // A selection the model derives is derived again on read, so it is not
    // written (FR-640): a hyper clause over two models has one for each trace.
    let derived =
        fsl_replay::spine::RequestClocks::model_selections(checked.package()).unwrap_or_default();
    let mut named: Vec<crate::emit_checked_v2::ClockSelection> = Vec::new();
    for selection in selections {
        if selection.trace.is_some()
            && derived.iter().any(|held| {
                held.declaration_span == selection.declaration_span
                    && held.trace == selection.trace
                    && held.clock == selection.clock
            })
        {
            continue;
        }
        let name = graph
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().span() == selection.declaration_span)
            .map(|node| node.clause().name().to_owned())
            .or_else(|| {
                graph
                    .protocols()
                    .iter()
                    .flat_map(|protocol| &protocol.obligations.compensations)
                    .find(|template| template.span == selection.declaration_span)
                    .map(|template| template.name.clone())
            })
            .or_else(|| {
                graph
                    .hyper_clauses()
                    .iter()
                    .find(|hyper| hyper.span() == selection.declaration_span)
                    .map(|hyper| hyper.name().to_owned())
            });
        let Some(name) = name else {
            let awaited = graph
                .protocols()
                .iter()
                .flat_map(|protocol| {
                    protocol
                        .awaits
                        .iter()
                        .map(move |awaited| (protocol, awaited))
                })
                .find(|(_, awaited)| awaited.span == selection.declaration_span);
            if let Some((protocol, awaited)) = awaited {
                let found = crate::emit_checked_v2::ClockSelection {
                    clause: format!("await {}::{}", protocol.name, awaited.name),
                    clock: selection.clock.clone(),
                };
                if !named.contains(&found) {
                    named.push(found);
                }
            }
            continue;
        };
        let found = crate::emit_checked_v2::ClockSelection {
            clause: name,
            clock: selection.clock.clone(),
        };
        if !named.contains(&found) {
            named.push(found);
        }
    }
    named
}

/// The pipeline of [`complete_sources`] through S4: the checked unit is
/// handed to `finish` with the admitted domain packages, the library
/// sources and the map from a spine failure to its located cause.
#[allow(
    clippy::too_many_arguments,
    reason = "mirrors the existing wide signature; a parameter struct is a separate refactor"
)]
fn check_sources<T>(
    request: &wire::CompileRequest,
    intake: &mut Intake<'_>,
    sources: &[FormalSource],
    unit_list: bool,
    limits: fsl_replay::spine::SpineLimits,
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    extension_catalog_override: Option<fsl_semantics::library::bundle::DefinitionCatalog>,
    finish: impl FnOnce(
        &mut Intake<'_>,
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
    let selected_models = request.models.as_deref().unwrap_or_default();
    if selected_models
        .iter()
        .any(|model| !is_model_input_format(model))
    {
        return Err(RunCause::CompleteSelection(CompleteSelection::NativeModels));
    }
    let documents = selected_models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = package_input_from_selected_models(selected_models, &documents)?;
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
    let default_clocks = fsl_replay::spine::RequestClocks::default();
    let clocks = request.clocks.as_ref().unwrap_or(&default_clocks);
    let check_selections: Vec<_> = parsed
        .iter()
        .flat_map(|unit| clocks.check_selections(unit))
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
                clocks: Some(clocks),
                check_mode: Some(request.check_mode),
                integer_division: integer_division.as_ref(),
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
        let selections = clocks
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
                clocks: Some(clocks),
                check_mode: Some(request.check_mode),
                integer_division: integer_division.as_ref(),
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
        // An absent `clocks` member selects like an empty one: a clause whose
        // clock needs no configuration still binds its default selection.
        let selections = clocks
            .package_selections(&checked, &[&parsed])
            .map_err(|refusal| spine_failure(*refusal))?;
        (checked, selections)
    };
    finish(
        intake,
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
    let libraries = request.libraries.as_deref().unwrap_or_default();
    for library in libraries {
        if library.identity.is_empty() {
            return Err(RunCause::Libraries(LibrarySelection::EmptyIdentity));
        }
    }
    libraries
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
            .map(|capability| CapabilityId::complete(capability).map_err(extension_catalog_cause))
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
        .map_err(extension_catalog_cause)
}

/// The refusal of an extension catalogue failure: a capability outside the
/// closed inventory is an unknown required feature, every other failure a
/// malformed catalogue.
fn extension_catalog_cause(error: fsl_semantics::library::bundle::PackageError) -> RunCause {
    match error {
        fsl_semantics::library::bundle::PackageError::UnknownCapability(_) => {
            RunCause::UnknownExtensionCapability(error.to_string())
        }
        other => RunCause::ExtensionCatalog(other.to_string()),
    }
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
    with_checked_request(path, request_bytes, |_| Ok(()), action)
}

/// [`with_request_limits`] with an entry's own check of the bytes, run before
/// the shared provision validation.
fn with_checked_request<T>(
    path: &Path,
    request_bytes: usize,
    check: impl FnOnce(&[u8]) -> Result<()>,
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
    check(&bytes)
        .and_then(|()| runtime_sources::validate_bytes(&bytes))
        .and_then(|()| action(path.parent().unwrap_or(Path::new(".")), &bytes))
        .map_err(|cause| {
            Box::new(RunError {
                cause,
                sources: crate::Sources::default(),
            })
        })
}

/// Request settings a caller supplies beside the request file rather than
/// inside it. Each request member is read exactly as its in-file counterpart
/// would be (FR-640); `limits` configures run and compile stages (FR-869,
/// FR-961).
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestOverrides<'a> {
    /// The caller's fully resolved scalar evaluation budget. When supplied,
    /// this effective budget replaces request accounting for calls and clause
    /// selections, and configures each verification-plan step.
    pub accounting: Option<&'a filament_exact::ScalarLimits>,
    /// The `clocks` object, as the driver's `--clocks` file carries it. A request
    /// that also carries `clocks` is refused `invalid-request` at stage `request`.
    pub clocks: Option<&'a serde_json::value::RawValue>,
    /// The file the checked package is written to, as the driver's `--output`
    /// names it (FR-810). Only a compile reads it; a request that also carries
    /// `output` is refused `invalid-request` at stage `request`.
    pub output: Option<&'a Path>,
    /// Run or compile stage limits supplied by the caller (FR-869, FR-961).
    /// `None` uses the published defaults.
    pub limits: Option<&'a fsl_replay::spine::SpineLimits>,
    /// Clause observation limits supplied by the caller (FR-312). A request
    /// that also carries `clause.observation_limits` is refused at request
    /// stage before any observation file is read.
    pub observation: Option<&'a fsl_semantics::model::observation::ObservationLimits>,
}

/// The caller's effective budget replaces request accounting on a call or a
/// clause selection, never a fresh allowance.
fn apply_accounting_override(request: &mut wire::Request, overrides: &RequestOverrides<'_>) {
    if let Some(accounting) = overrides.accounting {
        if let Some(call) = &mut request.call {
            call.accounting = wire::Accounting::from_limits(*accounting);
        }
        if let Some(clause) = &mut request.clause {
            clause.accounting = wire::Accounting::from_limits(*accounting);
        }
    }
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
    if envelope.format != T::FORMAT.as_str() {
        return Err(request_shape::not_read(T::FORMAT, &envelope.format));
    }
    let Object(request): Object<T> = serde_json::from_str(envelope.request.get())?;
    request.validate()?;
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
    let additional_files =
        FILES - counts.programs - counts.models - counts.observations - counts.extensions;
    Ok(Intake {
        additional_files,
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
    let additional_files =
        FILES - counts.programs - counts.models - counts.observations - counts.extensions;
    Ok(Intake {
        additional_files,
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
    let additional_files =
        FILES - counts.programs - counts.models - counts.observations - counts.extensions;
    Ok(Intake {
        additional_files,
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
    let mut sources = crate::Sources::default();
    prepare_run_bytes_with_sources(directory, bytes, dependent_bytes, overrides, &mut sources)
}

fn prepare_run_bytes_with_sources(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
    overrides: &RequestOverrides<'_>,
    admitted_sources: &mut crate::Sources,
) -> Result<PreparedRun> {
    if let Some(member) =
        relationships::duplicate_selection_member(bytes).map_err(RunCause::Json)?
    {
        return Err(RunCause::ClauseRequest {
            member,
            reason: "member is repeated".to_owned(),
        });
    }
    let mut request: wire::Request = request_with_overrides(bytes, overrides)?;
    apply_accounting_override(&mut request, overrides);
    // FR-1031: the first refusal in table order is the selection conflict.
    refuse_transfer_selection_conflict(&request)?;
    // FR-1045: reject malformed correlation selection before any selected
    // program, package, model or snapshot file is opened.
    relationships::validate_request(request.call.as_ref())?;
    // FR-100, FR-1039: a selected checked package runs from its own records,
    // refusing every member that would restate them before any file is read.
    if let Some(package) = &request.package {
        let progress_capacity = overrides.limits.map_or_else(
            || {
                fsl_replay::spine::SpineLimits::default()
                    .progress
                    .covered_positions
            },
            |limits| limits.progress.covered_positions,
        );
        let prepared = package_run::prepare(
            directory,
            &request,
            package,
            dependent_bytes,
            progress_capacity,
        );
        if let Ok(prepared) = &prepared {
            *admitted_sources = prepared.sources();
        }
        return prepared.map(|prepared| PreparedRun::Package(Box::new(prepared)));
    }
    let program = required_program(&request)?;
    let integer_division =
        read_integer_division(request.integer_division.as_ref(), "integer_division")?;
    let library_integer_division = request
        .libraries
        .iter()
        .enumerate()
        .map(|(index, library)| {
            read_integer_division(
                library.integer_division.as_ref(),
                &format!("libraries[{index}].integer_division"),
            )
            .map(|law| law.map(|law| (library.identity.clone(), law)))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    validate_extension_entries(&request.extensions)?;
    // FR-931: a malformed `tl_lowering` member refuses before any source
    // file is read.
    if let Some(member) = &request.tl_lowering {
        tl::decode(member)?;
    }
    if let Some(member) = &request.model_based_test {
        model_based_test::decode(member)?;
    }
    if request.tl_lowering.is_some() && request.model_based_test.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "model_based_test".to_owned(),
            reason: "model_based_test and tl_lowering are mutually exclusive".to_owned(),
        });
    }
    if let Some(transfer) = &request.refinement_transfer {
        validate_transfer_member(&request, transfer)?;
        let mut intake = intake_with_bytes(
            directory,
            FileCounts {
                programs: request.libraries.len().saturating_add(1),
                models: request.models.len(),
                observations: transfer
                    .observations
                    .values()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                extensions: request.extensions.len(),
            },
            dependent_bytes,
        )?;
        let source = intake.source(&program.source, "program.source")?;
        require_native_header(source.source())?;
        return Ok(PreparedRun::RefinementTransfer(Box::new(
            transfer::PreparedTransfer {
                directory: directory.to_path_buf(),
                dependent_bytes,
                request,
                source,
                integer_division,
            },
        )));
    }
    if request.items.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "items".to_owned(),
            reason: "items is handled by the multi-selection runner".to_owned(),
        });
    }
    if request.verification_plan.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "verification_plan".to_owned(),
            reason: "a verification plan is run by the plan coordinator route".to_owned(),
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
            ) + usize::from(
                request
                    .model_based_test
                    .as_ref()
                    .and_then(|member| member.get("initial"))
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
            ),
            extensions: request.extensions.len(),
        },
        dependent_bytes,
    )?;
    let prepared = (|| {
        reject_current_parameters_before_sources(&mut intake, request.clause.as_ref())?;
        // The program source is read exactly once; its declared edition decides
        // the runner before any model or library source is read (FR-100).
        let source = intake.source(&program.source, "program.source")?;
        require_native_header(source.source())?;
        prepare_complete_run(
            &mut intake,
            &request,
            source,
            integer_division,
            library_integer_division,
            overrides.limits.copied().unwrap_or_default(),
            overrides.observation.copied(),
        )
    })();
    *admitted_sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    prepared
}

type SelectionPresence = fn(&wire::Request) -> bool;

/// The request members FR-1031's first row names beside `export`, in the
/// order a refusal lists them, each with its presence in a typed request.
const TRANSFER_EXCLUSIVE: [(&str, SelectionPresence); 9] = [
    ("package", |request| request.package.is_some()),
    ("call", |request| request.call.is_some()),
    ("clause", |request| request.clause.is_some()),
    ("items", |request| request.items.is_some()),
    ("tl_lowering", |request| request.tl_lowering.is_some()),
    ("model_based_test", |request| {
        request.model_based_test.is_some()
    }),
    ("verification_plan", |request| {
        request.verification_plan.is_some()
    }),
    ("synthesis", |request| request.synthesis.is_some()),
    ("export_evidence", |request| {
        request.export_evidence.is_some()
    }),
];

/// FR-1031's first row: `refinement_transfer` beside any other selection
/// member refuses at `request`, naming every selection member present.
fn refuse_transfer_selection_conflict(request: &wire::Request) -> Result<()> {
    if request.refinement_transfer.is_none() {
        return Ok(());
    }
    let present: Vec<_> = TRANSFER_EXCLUSIVE
        .into_iter()
        .filter_map(|(name, is_present)| is_present(request).then_some(name))
        .collect();
    if present.is_empty() {
        return Ok(());
    }
    Err(transfer_conflict(&present))
}

/// The refusal of `refinement_transfer` beside the selection members
/// `present`.
fn transfer_conflict(present: &[&str]) -> RunCause {
    RunCause::ClauseRequest {
        member: "refinement_transfer".to_owned(),
        reason: format!(
            "refinement_transfer is exclusive with {}",
            present.join(", ")
        ),
    }
}

fn validate_transfer_member(
    request: &wire::Request,
    member: &wire::RefinementTransfer,
) -> Result<()> {
    if member.refinement.is_empty() {
        return Err(RunCause::ClauseRequest {
            member: "/refinement_transfer/refinement".to_owned(),
            reason: "expected a non-empty refinement name".to_owned(),
        });
    }
    if member.clauses.is_empty() {
        return Err(RunCause::ClauseRequest {
            member: "/refinement_transfer/clauses".to_owned(),
            reason: "expected a non-empty clause list".to_owned(),
        });
    }
    let mut clauses = std::collections::BTreeSet::new();
    for (index, clause) in member.clauses.iter().enumerate() {
        if clause.is_empty() {
            return Err(RunCause::ClauseRequest {
                member: format!("/refinement_transfer/clauses/{index}"),
                reason: "expected a non-empty clause name".to_owned(),
            });
        }
        if !clauses.insert(clause) {
            return Err(RunCause::ClauseRequest {
                member: format!("/refinement_transfer/clauses/{index}"),
                reason: "a clause name occurs more than once".to_owned(),
            });
        }
    }
    for (side, subject) in [
        ("abstract", &member.abstract_subject),
        ("concrete", &member.concrete),
    ] {
        if subject.model.is_empty() {
            return Err(RunCause::ClauseRequest {
                member: format!("/refinement_transfer/{side}/model"),
                reason: "expected a non-empty model alias".to_owned(),
            });
        }
        if subject.initial.is_empty() {
            return Err(RunCause::ClauseRequest {
                member: format!("/refinement_transfer/{side}/initial"),
                reason: "expected a non-empty initial snapshot list".to_owned(),
            });
        }
        for (index, initial) in subject.initial.iter().enumerate() {
            if initial.is_empty() {
                return Err(RunCause::ClauseRequest {
                    member: format!("/refinement_transfer/{side}/initial/{index}"),
                    reason: "expected a non-empty snapshot key".to_owned(),
                });
            }
            if !member.observations.contains_key(initial) {
                return Err(RunCause::ClauseRequest {
                    member: format!("/refinement_transfer/{side}/initial/{index}"),
                    reason: "initial snapshot key is absent from observations".to_owned(),
                });
            }
        }
    }
    if let Some(premises) = &member.premises {
        for name in premises.claims.keys() {
            if !member.clauses.contains(name) {
                return Err(RunCause::ClauseRequest {
                    member: format!("/refinement_transfer/premises/claims/{name}"),
                    reason: "premise clause is absent from clauses".to_owned(),
                });
            }
        }
    }
    if request
        .models
        .iter()
        .any(|model| !is_model_input_format(model))
    {
        return Err(RunCause::CompleteRunSelection(
            CompleteRunSelection::NativeModel,
        ));
    }
    Ok(())
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
    stage_limits: SpineLimits,
    observation_override: Option<fsl_semantics::model::observation::ObservationLimits>,
) -> Result<PreparedRun> {
    if request.call.is_some() || request.clause.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "tl_lowering".to_owned(),
            reason: "tl_lowering, call and clause are mutually exclusive".to_owned(),
        });
    }
    let lowering = tl::decode(member)?;
    let clause: wire::ClauseRun = serde_json::from_value(tl::selection(&lowering))?;
    let text = source.source().text().to_owned();
    let run = clause::read(
        intake,
        request,
        &clause,
        source,
        stage_limits,
        observation_override,
    )?;
    let behaviours = match &lowering.cross {
        Some(tl::CrossEvaluate::Initial { max_steps, .. }) => {
            Some(initial_subject(&run.snapshots, *max_steps)?)
        }
        Some(tl::CrossEvaluate::Trace { .. }) | None => None,
    };
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(PreparedRun::TlLowering(Box::new(PreparedLowering {
        request: run,
        lowering,
        behaviours,
        source: text,
        sources,
    })))
}

/// FR-931 `cross_evaluate.initial`: the snapshot the lowering's model
/// subject is built from, and the longest behaviour. A universe is a
/// complete population, so a population the snapshot marks incomplete
/// refuses `incomplete_population` naming it, as FR-620 refuses a universe
/// snapshot's.
fn initial_subject(
    provision: &fsl_semantics::model::observation::DocumentProvision,
    max_steps: u64,
) -> Result<(fsl_semantics::model::observation::DocumentRef, usize)> {
    let mut snapshots = provision.keys();
    let (Some(initial), None) = (snapshots.next(), snapshots.next()) else {
        return Err(RunCause::ClauseRequest {
            member: "tl_lowering.cross_evaluate.initial".to_owned(),
            reason: "expected one snapshot file".to_owned(),
        });
    };
    let reference =
        serde_json::json!({"authority": initial.authority, "identity": initial.identity});
    let subject: fsl_analyze::analyze::document::SubjectSelection =
        serde_json::from_value(serde_json::json!({
            "initial_states": [reference],
            "universes": [reference],
            "proof_bounds": [],
        }))?;
    subject
        .check_universes(provision)
        .map_err(RunCause::AnalyzeUniverse)?;
    let max_steps = usize::try_from(max_steps).map_err(|_| RunCause::ClauseRequest {
        member: "tl_lowering.cross_evaluate.max_steps".to_owned(),
        reason: "the step bound does not fit this platform".to_owned(),
    })?;
    Ok((initial.clone(), max_steps))
}

/// Prepare FR-100's function or clause selection without executing a stage.
/// `source` is the program source, already read by bounded intake.
fn read_integer_division(
    value: Option<&serde_json::Value>,
    member: &str,
) -> Result<Option<fsl_replay::AdmittedIntegerDivision>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let law = value.as_str().ok_or_else(|| RunCause::ClauseRequest {
        member: member.to_owned(),
        reason: "expected one of euclidean, floor and truncating".to_owned(),
    })?;
    let role = match law {
        "euclidean" => fsl_replay::DefinitionRole::IntegerDivisionEuclidean,
        "floor" => fsl_replay::DefinitionRole::IntegerDivisionFloor,
        "truncating" => fsl_replay::DefinitionRole::IntegerDivisionTruncating,
        _ => {
            return Err(RunCause::IntegerDivisionLaw {
                member: member.to_owned(),
                value: law.to_owned(),
            });
        }
    };
    fsl_replay::admit_integer_division(&[role.reference()], None)
        .map(Some)
        .map_err(|_| RunCause::ClauseRequest {
            member: member.to_owned(),
            reason: "the selected law could not be admitted".to_owned(),
        })
}

fn prepare_complete_run(
    intake: &mut Intake<'_>,
    request: &wire::Request,
    source: FormalSource,
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
    library_integer_division: Vec<(String, fsl_replay::AdmittedIntegerDivision)>,
    stage_limits: SpineLimits,
    observation_override: Option<fsl_semantics::model::observation::ObservationLimits>,
) -> Result<PreparedRun> {
    if let Some(member) = &request.tl_lowering {
        return prepare_lowering(
            intake,
            request,
            member,
            source,
            stage_limits,
            observation_override,
        );
    }
    if let Some(member) = &request.model_based_test {
        return model_based_test::prepare(
            intake,
            request,
            member,
            model_based_test::Context {
                source,
                integer_division,
                library_integer_division,
                limits: stage_limits,
                observation_override,
            },
        );
    }
    if let Some(clause) = &request.clause {
        if request.call.is_some() {
            return Err(RunCause::ClauseRequest {
                member: "call".to_owned(),
                reason: "call and clause are mutually exclusive".to_owned(),
            });
        }
        let request = clause::read(
            intake,
            request,
            clause,
            source,
            stage_limits,
            observation_override,
        )?;
        let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        return Ok(PreparedRun::Clause(Box::new(PreparedClauseRun {
            request: Box::new(request),
            sources,
        })));
    }
    if request
        .models
        .iter()
        .any(|model| !is_model_input_format(model))
    {
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
        .map(|value| requirements::read(value, "/call/requirements"))
        .transpose()?;
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let packages = package_input_from_selected_models(&request.models, &documents)?;
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
        .collect::<Result<Vec<(&wire::RunLibrary, FormalSource)>>>()?;
    // FR-660: the same request-owned catalog is available to run compilation.
    let extension_catalog = extension_catalog(&request.extensions, intake)?;
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
                    .as_ref()
                    .map(|value| (library.identity.clone(), value.clone()))
            }))
        })
        .and_then(|dependencies| dependencies.with_integer_division(library_integer_division))
        .map_err(|refusal| {
            spine_run_failure(
                &source,
                libraries.iter().map(|(_, source)| source),
                fsl_replay::spine::CompileRefusal::DependencyInput(refusal),
            )
        })?;
    let relationship_navigation = if let Some(selection) = call.relationships.as_ref() {
        let parse_request = fsl_replay::spine::ParseRequest {
            source: source.source().identity(),
            path: source.source().path(),
            bytes: source.source().text().as_bytes(),
        };
        let cancel = filament_exact::Cancel::new();
        let parsed = fsl_replay::spine::parse_selecting_extensions(
            &parse_request,
            fsl_replay::spine::ProfileCatalogInput {
                catalog: &extension_catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            },
            stage_limits.source,
            &cancel,
        )
        .map(|stage| stage.into_value())
        .map_err(
            |failure| match fsl_replay::spine::refusal_or_fault(failure) {
                Ok(refusal) => spine_run_failure(
                    &source,
                    libraries.iter().map(|(_, source)| source),
                    *refusal,
                ),
                Err(fault) => {
                    RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(fault)))
                }
            },
        )?;
        Some(relationships::validate_package_mappings(
            selection,
            parsed
                .unit()
                .selections()
                .models
                .iter()
                .map(|model| model.identity.clone()),
            &packages,
        )?)
    } else {
        None
    };
    let mut spine_call = read_call(call, request)?;
    if let Some(file) = &call.snapshot {
        let limits = fsl_replay::spine::ObservationLimitsWire::resolve(None).map_err(|error| {
            RunCause::ClauseRequest {
                member: error.member,
                reason: error.reason,
            }
        })?;
        let bytes = intake.file(file, intake.remaining)?;
        let (snapshot_bytes, relationship_extents) =
            relationships::snapshot_document(&bytes, call.relationships.as_ref())?;
        let observation =
            fsl_replay::spine::SuppliedObservation::from_document("snapshot", snapshot_bytes)
                .map_err(|error| RunCause::ClauseRequest {
                    member: error.member,
                    reason: error.reason,
                })?;
        spine_call.snapshot = Some(fsl_replay::spine::CallSnapshot {
            observation,
            limits,
            relationship_selection: call.relationships.clone(),
            relationship_extents,
            relationship_navigation,
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
            extension_catalog: Some(extension_catalog),
            limits: stage_limits,
            call: spine_call,
            integer_division,
            requirements,
        }),
    }))
}

/// A prepared FR-931 lowering: the valuation run of its clause and what the
/// record is built from.
pub struct PreparedLowering {
    request: fsl_replay::spine::ClauseRunRequest,
    lowering: tl::TlLowering,
    /// `cross_evaluate.initial`'s snapshot and longest behaviour.
    behaviours: Option<(fsl_semantics::model::observation::DocumentRef, usize)>,
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

/// Execute a prepared FR-1039 package call and retain the standard run error
/// envelope used by the command API.
///
/// Request and dependent-file limits are enforced while constructing the
/// `PreparedPackageRun` through [`prepare_run`] or
/// [`prepare_run_with_overrides`]; this function executes only that admitted
/// package.
pub fn execute_package_run(
    prepared: PreparedPackageRun,
) -> std::result::Result<RunResult, Box<RunError>> {
    let sources = prepared.sources();
    package_run::execute(prepared).map_err(|cause| Box::new(RunError { cause, sources }))
}

fn execute_lowering(prepared: PreparedLowering) -> Result<RunResult> {
    let PreparedLowering {
        request,
        lowering,
        behaviours,
        source,
        sources,
    } = prepared;
    let selection = request.selection.clone();
    let report = match &behaviours {
        Some((initial, max_steps)) => {
            fsl_replay::spine::run_temporal_behaviours(request, initial, *max_steps)
        }
        None => fsl_replay::spine::run_temporal_valuation(request),
    }
    .map_err(|error| RunCause::ClauseRequest {
        member: "tl_lowering".to_owned(),
        reason: error.to_string(),
    })?;
    let valuation = match &report.disposition {
        fsl_replay::spine::ClauseDisposition::TemporalValuation(valuation) => valuation,
        fsl_replay::spine::ClauseDisposition::MissingName { name } => {
            return Err(RunCause::TlClause { name: name.clone() });
        }
        // FR-931: a snapshot the subject builder refuses refuses the request.
        fsl_replay::spine::ClauseDisposition::Admit(
            fsl_semantics::model::observation::AdmissionFailure::Refused(record),
        ) if behaviours.is_some() => {
            return Err(RunCause::TlSubject(record.clone()));
        }
        _ => return output::clause_report(report, &selection),
    };
    let record = tl::lower(valuation, &source, &lowering)?;
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
        PreparedRun::Package(prepared) => {
            return package_run::execute_with_settle(*prepared, settle)
        }
        PreparedRun::ModelBasedTest(prepared) => {
            return Ok(RunResult {
                exit_code: prepared.exit_code,
                value: NativeResult::model_based_test_document(prepared.record),
                outcome: None,
                requirements: None,
                sources: prepared.sources,
            });
        }
        PreparedRun::RefinementTransfer(prepared) => {
            return transfer::execute(*prepared, settle);
        }
        PreparedRun::Clause(prepared) => {
            let PreparedClauseRun { request, sources } = *prepared;
            let selection = request.selection.clone();
            // The source map is kept on every clause run: it is an Arc clone of the
            // emission's map, and JSON rendering never reads it, so only text
            // witness lines pay for it.
            let (report, source_map) = fsl_replay::spine::run_clause_with_source_map(*request)
                .map_err(|error| RunCause::ClauseRequest {
                    member: "clause".to_owned(),
                    reason: error.to_string(),
                })?;
            let mut result = output::clause_report(report, &selection)?;
            result.sources = sources.with_source_map(source_map);
            return Ok(result);
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
    let run = |catalog: Option<&fsl_semantics::library::bundle::DefinitionCatalog>| match (
        catalog,
        prepared.integer_division,
    ) {
        (Some(catalog), Some(integer_division)) => {
            fsl_replay::spine::run_with_profile_catalog_and_integer_division(
                source.source().identity().clone(),
                package_name(source.source().identity()),
                source.source().path(),
                source.source().text().as_bytes(),
                &prepared.packages,
                &prepared.dependencies,
                prepared.limits,
                &prepared.call,
                &objects,
                fsl_replay::spine::ProfileCatalogInput {
                    catalog,
                    limits: fsl_semantics::library::bundle::PackageLimits::default(),
                },
                integer_division,
                &call_cancel,
            )
        }
        (Some(catalog), None) => fsl_replay::spine::run_with_profile_catalog(
            source.source().identity().clone(),
            package_name(source.source().identity()),
            source.source().path(),
            source.source().text().as_bytes(),
            &prepared.packages,
            &prepared.dependencies,
            prepared.limits,
            &prepared.call,
            &objects,
            fsl_replay::spine::ProfileCatalogInput {
                catalog,
                limits: fsl_semantics::library::bundle::PackageLimits::default(),
            },
            &call_cancel,
        ),
        (None, _) => fsl_replay::spine::run(
            source.source().identity().clone(),
            package_name(source.source().identity()),
            source.source().path(),
            source.source().text().as_bytes(),
            &prepared.packages,
            &prepared.dependencies,
            prepared.limits,
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
    let (ran, settled) =
        fsl_replay::run_with_requirements_reading_and_profile_catalog_and_integer_division(
            &fsl_replay::RequirementsRun {
                source: source.source().identity().clone(),
                package_name: package_name(source.source().identity()),
                path: source.source().path(),
                bytes: source.source().text().as_bytes(),
                packages: &prepared.packages,
                dependencies: &prepared.dependencies,
                limits: prepared.limits,
                call: &prepared.call,
                objects: &objects,
                cancel: &call_cancel,
            },
            prepared.extension_catalog.as_ref().map(|catalog| {
                fsl_replay::spine::ProfileCatalogInput {
                    catalog,
                    limits: fsl_semantics::library::bundle::PackageLimits::default(),
                }
            }),
            prepared.integer_division,
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

/// FR-100, FR-650: the spine call a request's `call` names, its arguments in
/// the run value form, before any snapshot is read.
fn read_call(call: &wire::Call, request: &wire::Request) -> Result<fsl_replay::spine::Call> {
    // Canonical run values are objects; the typed reader closes their
    // member sets against each parameter after the program is checked.
    let wire_arguments = call
        .arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| {
            if !argument.value.is_object() {
                return Err(RunCause::ClauseRequest {
                    member: format!("/call/arguments/{index}/value"),
                    reason: "expected a typed canonical value object".to_owned(),
                });
            }
            Ok(fsl_replay::spine::WireArgument {
                parameter: argument.parameter.clone(),
                value: argument.value.clone(),
                selected_object: None,
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
    spine_call.canonical_values = true;
    spine_call.live_occurrences = call.evaluation.live_occurrences();
    Ok(spine_call)
}

/// FR-100: the request's `program`; a request selecting neither `program`
/// nor `package` refuses `invalid-request` naming `program`.
fn required_program(request: &wire::Request) -> Result<&wire::Program> {
    request
        .program
        .as_ref()
        .ok_or_else(|| RunCause::ClauseRequest {
            member: "program".to_owned(),
            reason: "a request selects exactly one of program and package".to_owned(),
        })
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

    /// FR-286-AC-15: the command's forms wrapper preserves a blank source
    /// label without inventing a region, and retains the exact region for a
    /// located model-unit source diagnostic.
    #[test]
    #[trace("TC-3112", "FR-286-AC-15")]
    fn model_unit_source_projection_preserves_optional_region_and_blank_label() {
        let blank = fsl_forms::cst::diagnostic::read_source(
            fsl_foundation::SourceIdentity::new("", "model"),
            "model.fsl",
            b"",
            usize::MAX,
        )
        .expect_err("a blank authority is refused before reading source");
        let blank_json = serde_json::to_value(model_unit_outcome_diagnostic(&blank))
            .expect("the blank-label outcome diagnostic serializes");
        assert_eq!(blank_json["cause"], "blank-label");
        assert_eq!(blank_json["label"], "authority");
        assert!(blank_json["locus"].is_null(), "{blank_json}");

        let parsed = fsl_forms::cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "test:model"),
            "model.fsl",
            b"@",
            fsl_forms::cst::Limits::default(),
        )
        .expect("the source is read");
        let diagnostics = parsed.diagnostics();
        let [located] = diagnostics.as_slice() else {
            panic!("one invalid-token diagnostic is retained");
        };
        let located_json = serde_json::to_value(model_unit_outcome_diagnostic(located))
            .expect("the located outcome diagnostic serializes");
        assert_eq!(located_json["locus"]["kind"], "region");
        assert_eq!(located_json["locus"]["start"], 0);
        assert_eq!(located_json["locus"]["end"], 1);
        assert!(located_json.get("label").is_none(), "{located_json}");
    }

    /// FR-621-AC-17: a result whose package has no graph for its node ids ends
    /// the run with the emission cause and a category that is no internal
    /// failure; an id with no held form stays an analyze fault.
    #[test]
    #[trace("TC-3154", "FR-621-AC-17")]
    fn a_package_without_a_graph_ends_the_run_with_the_typed_emission_cause() {
        use fsl_analyze::analyze::document::HeldIdentityError;
        use fsl_analyze::analyze::GraphUnavailable;
        let cause = identity_cause(HeldIdentityError::NoGraph(GraphUnavailable::EmitRefused(
            Box::new(fsl_package::EmitRefusal::OutputLimit(
                fsl_foundation::diagnostic::LimitExceeded::new(
                    fsl_foundation::Setting::EmitOutputBytes,
                    1,
                    2,
                ),
            )),
        )));
        assert!(matches!(
            cause,
            RunCause::AnalyzeEmission(GraphUnavailable::EmitRefused(_))
        ));
        assert_eq!(cause.code(), Code::StageLimitExceeded);
        assert_ne!(
            cause.category(),
            filament_semantic_value::catalog::Category::InternalFailure
        );
        assert!(matches!(
            identity_cause(HeldIdentityError::NotInGraph(String::new())),
            RunCause::AnalyzeFault
        ));
    }

    /// FR-866 / FR-869: a raised `s1.input_bytes` raises check's source read
    /// ceiling, so the S1 limit is the bound that refuses.
    #[test]
    #[trace("FR-866", "TC-2457")]
    fn check_source_read_ceiling_never_undercuts_the_s1_limit() {
        assert_eq!(check_source_read_ceiling(0), TOTAL_BYTES);
        assert_eq!(check_source_read_ceiling(TOTAL_BYTES), TOTAL_BYTES);
        assert_eq!(check_source_read_ceiling(TOTAL_BYTES + 1), TOTAL_BYTES + 1);
    }

    fn fault_error(stage: &'static str, invariant: &'static str) -> RunError {
        RunError {
            cause: RunCause::SpineRun(Box::new(fsl_replay::spine::RunRefusal::Fault(
                InternalFault::new(stage, invariant),
            ))),
            sources: crate::Sources::default(),
        }
    }

    #[test]
    #[trace("FR-862-AC-3", "TC-2443")]
    fn replay_row_output_failure_projects_internal_failure_without_partial_result() {
        let codec = fsl_replay::CauseCodecError::Empty("result row");
        let error = RunError {
            cause: RunCause::ReplayOutput {
                outcome: Box::new(fsl_replay::OutcomeDocument::from_replay_output_failure(
                    1, &codec,
                )),
                error: codec,
            },
            sources: crate::Sources::default(),
        };
        let outcome: serde_json::Value = serde_json::from_slice(
            &error
                .outcome_document(fsl_replay::Operation::Replay)
                .to_bytes()
                .expect("failure outcome JSON"),
        )
        .expect("outcome document");
        assert_eq!(error.category().exit_code(), 30);
        assert_eq!(outcome["category"], "internal-failure");
        assert_eq!(outcome["last_stage"], serde_json::Value::Null);
        assert_eq!(outcome["result"], serde_json::Value::Null);
        assert_eq!(outcome["diagnostics"][0]["cause"], "output-failure");
        assert!(
            outcome["diagnostics"][0]["message"]
                .as_str()
                .expect("diagnostic message")
                .contains("row at position 1"),
            "{outcome}"
        );
        let command = error.value().expect("command error document");
        assert_eq!(command["stage"], "output");
        assert_eq!(command["code"], "output-failure");
    }

    /// A package the strict reader refuses, found on the analyze route, stops
    /// the run with the reader's own code, cause and pointer and that code's
    /// exit, never as a runtime invariant (exit 30).
    #[test]
    #[trace("FR-286-AC-2")]
    fn a_refused_analyze_package_keeps_its_own_code_cause_pointer_and_exit() {
        let error = RunError {
            cause: RunCause::AnalyzeRefused(Box::new(
                fsl_analyze::analyze::AnalyzeRefusal::Package(
                    fsl_package::PackageAdmissionRefusal {
                        limit: None,
                        code: Code::InvalidPackage,
                        cause: Some("invalid-value".to_owned()),
                        pointer: Some("/claim_names/2".to_owned()),
                        identity: None,
                        host_allocation_failure: None,
                    },
                ),
            )),
            sources: crate::Sources::default(),
        };
        let outcome: serde_json::Value = serde_json::from_slice(
            &error
                .outcome_document(fsl_replay::Operation::Analyze)
                .to_bytes()
                .expect("refusal outcome JSON"),
        )
        .expect("outcome document");
        assert_eq!(error.cause.code(), Code::InvalidPackage);
        assert_eq!(error.category().exit_code(), 20);
        let diagnostic = &outcome["diagnostics"][0];
        assert_eq!(diagnostic["code"], "invalid_package");
        assert_eq!(diagnostic["cause"], "invalid-value");
        assert_eq!(diagnostic["details"]["path"], "/claim_names/2");
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

#[cfg(test)]
mod host_allocation_tests {
    use super::inspect_handoff_result;
    use crate::checked_handoff;
    use filament_contract_model::{
        CheckedPackageDispatchResult, CheckedPackageHostAllocationFailed, NativeJoinLocus,
        NativeJoinLocusSource, PackageReadStage,
    };
    use filament_semantic_value::catalog::Category;
    use ix_trace_rs::trace;
    use std::path::Path;

    /// FSpec FR-330-AC-17 and FSL FR-096-AC-24: an injected reader host
    /// allocation failure reaches the inspect command as an internal tool
    /// failure with stage and requested size.
    #[trace("FSpec-FR-330-AC-17", "FR-096-AC-24")]
    #[test]
    fn inspect_preserves_an_injected_package_reader_host_allocation_failure() {
        let failure = CheckedPackageHostAllocationFailed {
            stage: PackageReadStage::Envelope,
            locus: NativeJoinLocus {
                source: NativeJoinLocusSource::PackageRead(PackageReadStage::Envelope, 0),
                form: "package_bytes",
            },
            requested: 32,
            consumed: 64,
        };
        let error =
            checked_handoff::admit(CheckedPackageDispatchResult::HostAllocationFailed(failure))
                .expect_err("injected reader failure must be surfaced");
        let outcome = inspect_handoff_result(Err(error), Path::new("handoff"), "handoff");

        assert_eq!(outcome.category(), Category::InternalFailure);
        let diagnostic = &outcome.diagnostics()[0];
        assert_eq!(diagnostic.code(), "runtime_invariant");
        assert!(diagnostic.message().contains("Envelope"));
        assert!(diagnostic.message().contains("32 bytes"));
    }
}
