// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620: the `native-analyze/1` request reader and the analyze run over
//! it. The reader applies FR-026's envelope and intake rules, compiles the
//! program as FR-027 compiles it, provisions the supplied documents under
//! their own identities, and hands the items to `fsl-analyze`'s resolver
//! and document writer. The caller negotiates (FR-281), so the run takes the
//! caller's [`Negotiate`].

use std::path::Path;

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::analyze::document::{
    AnalyzeDocumentError, AnalyzeRun, ItemSelection, Negotiate, SubjectSelection, analyze_document,
    resolve_items, resolve_package_items, settle_package_items,
};
use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply, Engine, Engines, SourceProvision};
use fsl_analyze::provider::ProviderOptions;
use fsl_semantics::model::observation::DocumentProvision;

use super::{
    FileCounts, FormalSource, NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
    check_sources, intake_with_bytes, request, require_complete_edition, wire,
};

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::AnalyzeRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

/// A read `native-analyze/1` request: its compiled-from sources, supplied
/// documents, subject, items and limits, with no compiler, evaluator or
/// engine call made.
pub struct PreparedAnalyze {
    input: Box<AnalyzeInput>,
}

/// The owned inputs of one analyze run (FR-620 "Library surface").
pub struct AnalyzeInput {
    /// The program source.
    pub source: FormalSource,
    /// The caller's domain-package selections, compile request members and
    /// libraries, compiled as FR-027 compiles them.
    compile: wire::CompileRequest,
    /// The supplied documents, each under its own identity.
    pub documents: DocumentProvision,
    /// The request's default model subject.
    pub subject: SubjectSelection,
    /// The caller's items; `None` for FR-620's default items.
    pub items: Option<Vec<ItemSelection>>,
    /// The limits every item of the run shares.
    pub limits: AnalyzeLimits,
    /// The run's deadline in milliseconds.
    pub deadline_ms: Option<u64>,
}

impl PreparedAnalyze {
    /// Transfer the read inputs without running a stage.
    pub fn into_input(self) -> AnalyzeInput {
        *self.input
    }
}

/// Read a `native-analyze/1` request and every file it selects (FR-620,
/// FR-026), performing no compiler, evaluator, engine or provider call.
///
/// # Errors
///
/// `invalid-request` for a malformed, wrong-format or unknown-member
/// request; FR-026's resource causes for a byte or file limit; the source
/// refusal of an unreadable program.
pub fn prepare_analyze(
    path: &Path,
    limits: RunLimits,
) -> std::result::Result<PreparedAnalyze, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        prepare_bytes(directory, bytes, limits.dependent_bytes)
    })
}

fn prepare_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
) -> Result<PreparedAnalyze> {
    match read(directory, bytes, dependent_bytes)? {
        Readout::Source(input, _) => Ok(PreparedAnalyze {
            input: Box::new(input),
        }),
        // A prepared run holds the source it compiles; a package request
        // has none to prepare (FR-850).
        Readout::Package(_) => Err(RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: ": a prepared run holds a program source".to_owned(),
        }),
    }
}

/// What [`read`] read: a `program` request's inputs with the intake that
/// still charges compilation, or a `package` request's admitted package.
enum Readout<'d> {
    Source(AnalyzeInput, super::Intake<'d>),
    Package(PackageSelection),
}

/// A `package` request (FR-850): the package the strict reader admitted
/// from the request's bytes, with the request's items.
struct PackageSelection {
    admitted: AdmittedHandoff,
    items: Option<Vec<ItemSelection>>,
}

/// FR-850, FR-940: analyze a `package` request over the `BarePackage` of
/// its admitted package, read through `AnalysisView`, never compiling
/// source. Items whose engine family has no package form yet refuse the
/// request whole, as not yet implemented, rather than settle `unsupported`.
fn run_package(
    selection: PackageSelection,
    providers: Vec<Engine>,
    options: &ProviderOptions,
) -> Result<RunResult> {
    let engines = engines_with(providers, options)?;
    let AdmittedHandoff {
        package: admitted,
        identities,
        equality,
        mut budget,
    } = selection.admitted;
    let package = fsl_package::BarePackage::new(&admitted)
        .with_supply(
            fsl_package::BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .map_err(|failure| {
            // FR-531 "Denial is its own typed refusal": a denied identity
            // join names the `i2.*` setting, never a witness defect.
            let setting = failure
                .setting()
                .map_or_else(|| "i2.work_units".to_owned(), |setting| setting.to_string());
            RunCause::AnalyzePackage {
                code: fsl_foundation::Code::StageLimitExceeded,
                cause: None,
                pointer: None,
                message: format!("stage_limit_exceeded: {setting}: {failure:?}"),
            }
        })?;
    let items = resolve_package_items(&package, selection.items.as_deref())
        .map_err(|pending| RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: match pending.request_index {
                Some(index) => format!(
                    ": items[{}] names the {} family, which has no package form yet",
                    index.get(),
                    pending.family.name()
                ),
                None => format!(": the {} rule has no package form yet", pending.family.name()),
            },
        })?;
    let settled = settle_package_items(items, &engines).map_err(|_| RunCause::AnalyzeFault)?;
    Ok(RunResult {
        exit_code: settled.exit_code,
        value: NativeResult::analyze_document(settled.document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    })
}

/// The request's inputs and the intake that read them, which still charges
/// the libraries and domain packages compilation reads.
fn read<'d>(
    directory: &'d Path,
    bytes: &[u8],
    dependent_bytes: usize,
) -> Result<Readout<'d>> {
    let request: wire::AnalyzeRequest = request(bytes).map_err(|cause| match cause {
        RunCause::Json(error) => invalid("request", error.to_string()),
        cause => cause,
    })?;
    for (position, item) in request.items.iter().flatten().enumerate() {
        if item.recording.is_some()
            && item.kind.as_deref()
                != Some(fsl_semantics::check::Capability::GlobalConformance.to_wire())
        {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                "recording is only valid on a global-conformance item",
            ));
        }
    }
    // FR-634: a `simulation` item names no claim and carries its
    // `simulation` member, and only it does; `seed` and `samples` belong to
    // `sample` only.
    for (position, item) in request.items.iter().flatten().enumerate() {
        let simulation =
            item.kind.as_deref() == Some(fsl_analyze::analyze::document::SIMULATION_KIND);
        match (&item.simulation, simulation) {
            (Some(_), false) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "simulation is only valid on a simulation item",
                ))
            }
            (None, true) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "a simulation item carries its simulation member",
                ))
            }
            (Some(settings), true) => {
                if item.claim.is_some() {
                    return Err(invalid(
                        &format!("/items/{position}/claim"),
                        "a simulation item settles no claim",
                    ));
                }
                if let Some(member) = settings.misplaced_member() {
                    return Err(invalid(
                        &format!("/items/{position}/simulation/{member}"),
                        format!("{member} belongs to a sample simulation only"),
                    ));
                }
            }
            (None, false) => {}
        }
    }
    if let Some(package) = &request.package {
        let admitted = admit_package(directory, dependent_bytes, &request, package)?;
        return Ok(Readout::Package(PackageSelection {
            admitted,
            items: request.items,
        }));
    }
    if request.integer_division.is_some() {
        return Err(invalid(
            "integer_division",
            "an integer-division law is not read by this build",
        ));
    }
    let Some(program) = &request.program else {
        return Err(invalid("program", "a program or package is required"));
    };
    if !request.memory.is_empty() {
        return Err(invalid(
            "memory",
            "weak-memory selections are not read by this build",
        ));
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let source = intake.source(&program.source, "program.source")?;
    require_complete_edition(source.source())?;
    let mut documents = DocumentProvision::new();
    let mut recording_documents = std::collections::BTreeSet::new();
    for (position, document) in request.documents.iter().enumerate() {
        let bytes = intake.file(&document.file, intake.remaining)?;
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
            invalid(
                &format!("/documents/{position}"),
                format!("{} is not a JSON document: {error}", document.file),
            )
        })?;
        let format = parsed.get("format").and_then(serde_json::Value::as_str);
        let declared_identity = match parsed.get("identity") {
            Some(value) => Some(
                serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                    value.clone(),
                )
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?,
            ),
            None => None,
        };
        let stated_identity = match &document.identity {
            Some(value) => Some(
                serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                    value.clone(),
                )
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?,
            ),
            None => None,
        };
        if format == Some("filament.state.snapshot/v1") && document.identity.is_some() {
            return Err(invalid(
                &format!("/documents/{position}/identity"),
                "a supplied snapshot uses its declared identity and cannot state a wrapper identity",
            ));
        }
        if let (Some(declared), Some(stated)) = (&declared_identity, &stated_identity) {
            if declared != stated {
                return Err(invalid(
                    &format!("/documents/{position}/identity"),
                    format!(
                        "stated identity {{{}, {}}} contradicts declared identity {{{}, {}}}",
                        stated.authority, stated.identity, declared.authority, declared.identity
                    ),
                ));
            }
        }
        let Some(identity) = declared_identity.as_ref().or(stated_identity.as_ref()) else {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} declares no identity and none was stated", document.file),
            ));
        };
        let reference = fsl_semantics::model::observation::DocumentRef {
            authority: identity.authority.clone(),
            identity: identity.identity.clone(),
        };
        if format == Some("filament.protocol-recording/1") {
            recording_documents.insert((reference.authority.clone(), reference.identity.clone()));
        }
        if documents.insert(reference, bytes).is_some() {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} repeats a provisioned document identity", document.file),
            ));
        }
    }
    // FR-636: a `trace` naming no provisioned document refuses the request.
    for (position, trace) in request
        .items
        .iter()
        .flatten()
        .enumerate()
        .filter_map(|(position, item)| item.trace.as_ref().map(|trace| (position, trace)))
    {
        if !documents.contains_key(&trace.reference()) {
            return Err(invalid(
                "items",
                format!(
                    "items[{position}].trace {{{}, {}}} names no provisioned document",
                    trace.authority, trace.identity
                ),
            ));
        }
    }
    for (position, recording) in
        request
            .items
            .iter()
            .flatten()
            .enumerate()
            .filter_map(|(position, item)| {
                item.recording
                    .as_ref()
                    .map(|recording| (position, recording))
            })
    {
        let identity = recording.reference();
        if !recording_documents.contains(&(identity.authority.clone(), identity.identity.clone())) {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                format!(
                    "recording {{{}, {}}} does not name a provisioned protocol recording",
                    recording.authority, recording.identity,
                ),
            ));
        }
    }
    let mut limits = AnalyzeLimits::unbounded();
    let mut accounting = fsl_replay::AccountingLimits(limits.accounting);
    for (setting, bound) in request.limits.accounting.entries() {
        accounting.set_bound(setting, bound);
    }
    limits.accounting = accounting.0;
    limits.observation = request.limits.observation.resolve();
    limits.normalization = request.limits.normalization.resolve();
    limits.deadline = request
        .deadline_ms
        .map_or(DeadlineSupply::Unbounded, DeadlineSupply::Milliseconds);
    // FR-620: an explicit `subject.universes` refuses an unprovisioned or
    // incomplete reference; the default subject's universes are the complete
    // populations its snapshots declare, so it refuses neither.
    let subject = match request.subject {
        Some(subject) => {
            subject
                .check_universes(&documents)
                .map_err(RunCause::AnalyzeUniverse)?;
            subject
        }
        None => SubjectSelection::default_over(&documents),
    };
    let compile = wire::CompileRequest {
        models: request.models,
        program: wire::CompileProgram {
            sources: Vec::new(),
            extraction: false,
        },
        refinement_internals: request.refinement_internals,
        libraries: request.libraries,
        extensions: Vec::new(),
        clocks: Default::default(),
        output: None,
        handoff: None,
    };
    Ok(Readout::Source(
        AnalyzeInput {
            source,
            compile,
            documents,
            subject,
            items: request.items,
            limits,
            deadline_ms: request.deadline_ms,
        },
        intake,
    ))
}

/// Read the `native-analyze/1` request at `path`, compile its program,
/// resolve its items, have `negotiate` settle them, run them under `cancel`
/// and its deadline, and return FR-621's document with its FR-285 exit code.
///
/// # Errors
///
/// A whole-request refusal (FR-624): the request stage's, a compile stage's,
/// or the subject binding's and source provision's; the caller writes its
/// `analyze` refusal document.
pub fn analyze(
    path: &Path,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_limits(path, RunLimits::default(), negotiate, cancel)
}

/// [`analyze`] under the caller's request and dependent-file byte ceilings.
///
/// # Errors
///
/// As [`analyze`].
pub fn analyze_with_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        limits,
        negotiate,
        Vec::new(),
        &ProviderOptions::default(),
        cancel,
    )
}

/// [`analyze`] with the caller's started process providers (FR-635 "One rule
/// for every provider-routing entry"): each an [`Engine::external`] engine
/// over its `Process` descriptor and the handle its items are sent over.
/// They register beside `Engines::fsl()` before any item is negotiated, so
/// an item reaches one only by negotiation. An empty `providers` is
/// [`analyze`]. `options` carries the run's FSpec FR-462 limits, which every
/// provider request carries (FR-964).
///
/// # Errors
///
/// As [`analyze`]; and `invalid-request` naming `providers` when a provider
/// repeats a registered `BackendId`, before any item runs.
pub fn analyze_with_providers(
    path: &Path,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        RunLimits::default(),
        negotiate,
        providers,
        options,
        cancel,
    )
}

/// [`analyze_with_providers`] under the caller's request and dependent-file
/// byte ceilings.
///
/// # Errors
///
/// As [`analyze_with_providers`].
pub fn analyze_with_providers_and_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        let (input, mut intake) = match read(directory, bytes, limits.dependent_bytes)? {
            Readout::Source(input, intake) => (input, intake),
            Readout::Package(selection) => {
                return run_package(selection, providers, options);
            }
        };
        let mut result = run_input(&mut intake, input, negotiate, providers, options, cancel)?;
        result.sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        Ok(result)
    })
}

/// FSL's engines with the caller's process providers registered beside them,
/// under the run's provider `options`.
pub(super) fn engines_with(providers: Vec<Engine>, options: &ProviderOptions) -> Result<Engines> {
    let mut engines = Engines::fsl()
        .map_err(|_| RunCause::AnalyzeFault)?
        .under(*options);
    for provider in providers {
        engines.with(provider).map_err(|duplicate| {
            invalid(
                "providers",
                format!(
                    "a configured provider repeats the registered backend {}",
                    duplicate.backend.as_str()
                ),
            )
        })?;
    }
    Ok(engines)
}

fn run_input(
    intake: &mut super::Intake<'_>,
    input: AnalyzeInput,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult> {
    // The registry is fixed before the program is read, so a provider that
    // cannot register refuses the request before any stage runs.
    let engines = engines_with(providers, options)?;
    let AnalyzeInput {
        source,
        compile,
        documents,
        subject,
        items,
        limits,
        deadline_ms,
    } = input;
    let sources = std::slice::from_ref(&source);
    check_sources(
        &compile,
        intake,
        sources,
        false,
        fsl_replay::spine::SpineLimits::default(),
        None,
        None,
        |checked, _, _, packages, libraries, _, _, _, _| {
            let package = checked.package();
            let mut provision = SourceProvision::default();
            // FR-620: the request's units in request order, the program then
            // each library as listed, rank the default items.
            let mut units = Vec::new();
            for read in std::iter::once(&source).chain(libraries) {
                units.push(read.source().reference().clone());
                provision.insert(
                    read.source().reference(),
                    read.source().text().as_bytes().to_vec(),
                );
            }
            let mut registry = fsl_route::Registry::new();
            for descriptor in engines.registrations() {
                registry
                    .register(descriptor.clone())
                    .map_err(|_| RunCause::AnalyzeFault)?;
            }
            let resolved = resolve_items(package, &subject, items.as_deref(), &registry, &units);
            let run = AnalyzeRun {
                package,
                binding: subject.binding(&packages, &documents),
                provision,
                items: resolved,
                engines: &engines,
                limits,
            };
            let written = with_deadline(cancel, deadline_ms, || {
                analyze_document(run, negotiate, cancel)
            });
            match written {
                Ok((document, exit_code)) => Ok(RunResult {
                    exit_code,
                    value: NativeResult::analyze_document(document),
                    outcome: None,
                    requirements: None,
                    sources: crate::Sources::default(),
                }),
                Err(AnalyzeDocumentError::Refused(refusal)) => {
                    Err(RunCause::AnalyzeRefused(Box::new(refusal)))
                }
                Err(AnalyzeDocumentError::Fault) => Err(RunCause::AnalyzeFault),
            }
        },
    )
}

/// Run `work` with `cancel` cancelled for `deadline` once `deadline_ms`
/// elapses (FR-287): a deadline of 0 has passed before any item runs.
fn with_deadline<T>(cancel: &Cancel, deadline_ms: Option<u64>, work: impl FnOnce() -> T) -> T {
    match deadline_ms {
        None => work(),
        Some(0) => {
            cancel.cancel(CancelCause::Deadline);
            work()
        }
        Some(milliseconds) => std::thread::scope(|scope| {
            let (done, finished) = std::sync::mpsc::channel::<()>();
            let timer = cancel.clone();
            scope.spawn(move || {
                let wait = std::time::Duration::from_millis(milliseconds);
                if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = finished.recv_timeout(wait)
                {
                    timer.cancel(CancelCause::Deadline);
                }
            });
            let value = work();
            // The timer thread ends on this message or on its timeout; a
            // closed channel also ends it.
            let _ = done.send(());
            value
        }),
    }
}

/// FR-850 "Admission": a request that selects `package` is read by the strict
/// checked-package reader against the supplied domain packages, and refuses
/// before any item resolves when a compile member sits beside it, the reader
/// refuses the bytes, a recorded domain package is not supplied, or a
/// supplied one is not recorded.
fn admit_package(
    directory: &Path,
    dependent_bytes: usize,
    request: &wire::AnalyzeRequest,
    package: &wire::HandoffSelection,
) -> Result<AdmittedHandoff> {
    if request.program.is_some() {
        return Err(invalid(
            "program",
            "a program cannot be selected beside a package",
        ));
    }
    if !request.libraries.is_empty() {
        return Err(invalid(
            "libraries",
            "a package already holds its dependency selections",
        ));
    }
    if request.refinement_internals.is_some() {
        return Err(invalid(
            "refinement_internals",
            "a package is already compiled",
        ));
    }
    if request.integer_division.is_some() {
        return Err(invalid("integer_division", "a package is already compiled"));
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            // The handoff's package, identity and equality files.
            programs: 3,
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let handoff = handoff_directory(&package.directory)?;
    // FR-850 "Admission": an absent handoff directory or package file is a
    // package refusal, not an I/O failure.
    if !directory.join(&handoff).is_dir() {
        return Err(witness_refusal(
            &handoff,
            "missing-file",
            "the request names no handoff directory there",
        ));
    }
    if !directory.join(&handoff).join(PACKAGE_FILE).is_file() {
        return Err(witness_refusal(
            PACKAGE_FILE,
            "missing-file",
            "the handoff directory holds no checked package",
        ));
    }
    let bytes = intake.file(
        &format!("{handoff}/{PACKAGE_FILE}"),
        intake.remaining,
    )?;
    if request
        .models
        .iter()
        .any(|model| !super::is_domain_package(model))
    {
        return Err(RunCause::CompleteSelection(
            super::CompleteSelection::NativeModels,
        ));
    }
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let supplied = fsl_semantics::model::intake::package_input(
        documents
            .iter()
            .map(|document| document.source().text().as_bytes()),
    )
    .map_err(|refusal| match refusal.cause {
        fsl_semantics::model::refusal::ModelRefusalCause::ConflictingPackage { identity } => {
            conflicting_model(&identity)
        }
        _ => RunCause::ClauseRequest {
            member: "models".to_owned(),
            reason: refusal.detail,
        },
    })?;
    let limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let admitted =
        fsl_package::admit_checked_package(&bytes, &supplied, limits).map_err(package_refusal)?;
    for identity in supplied.keys() {
        if !admitted
            .selected_model_packages()
            .any(|recorded| recorded == identity)
        {
            return Err(conflicting_model(identity));
        }
    }
    // FR-850 "Admission": the handoff's identity rows are required, its
    // equality rows read when the producer wrote them (FR-531).
    if !directory.join(&handoff).join(IDENTITIES_FILE).is_file() {
        return Err(witness_refusal(
            IDENTITIES_FILE,
            "missing-file",
            "the handoff directory holds no identity file",
        ));
    }
    let identity_bytes = intake.file(
        &format!("{handoff}/{IDENTITIES_FILE}"),
        intake.remaining,
    )?;
    // One native budget for the pass: the identity read, then the supply's
    // join (FR-531 "One join, then borrowing").
    let mut budget = limits.native_join_limits().budget();
    let identities = fsl_package::ReadIdentityRows::read(&identity_bytes, &admitted, &mut budget)
        .map_err(|refusal| {
            let cause = match &refusal {
                fsl_package::IdentityFileRefusal::Malformed { .. } => "malformed-file",
                fsl_package::IdentityFileRefusal::Format(_) => "unknown-format",
                fsl_package::IdentityFileRefusal::Unjoined { .. } => "unjoined-row",
                fsl_package::IdentityFileRefusal::Native(_)
                | fsl_package::IdentityFileRefusal::Rows(_)
                | fsl_package::IdentityFileRefusal::SourceObject(_)
                | fsl_package::IdentityFileRefusal::SourceOperation(_) => "native-witness",
            };
            witness_refusal(IDENTITIES_FILE, cause, &refusal.to_string())
        })?;
    let equality = if directory.join(&handoff).join(EQUALITY_FILE).is_file() {
        let equality_bytes =
            intake.file(&format!("{handoff}/{EQUALITY_FILE}"), intake.remaining)?;
        Some(
            fsl_package::ReadEqualityRows::read(&equality_bytes, &admitted).map_err(
                |refusal| witness_refusal(EQUALITY_FILE, "refused-file", &refusal.to_string()),
            )?,
        )
    } else {
        None
    };
    Ok(AdmittedHandoff {
        package: admitted,
        identities,
        equality,
        budget,
    })
}

/// The FR-517 files a `package` selection reads from its handoff directory.
const PACKAGE_FILE: &str = "checked-package.json";
const IDENTITIES_FILE: &str = "identities.json";
const EQUALITY_FILE: &str = "equality.json";

/// The admitted handoff of a `package` request: the package the strict
/// reader admitted, its identity rows and its equality rows when present.
struct AdmittedHandoff {
    package: filament_contract_model::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: Option<fsl_package::ReadEqualityRows>,
    budget: filament_contract_model::NativeJoinBudget,
}

/// The handoff directory a `package` selection names, beneath the request's
/// directory.
fn handoff_directory(directory: &str) -> Result<String> {
    use std::path::Component;
    let path = Path::new(directory);
    if directory.is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(
            "package.directory",
            "must name a directory beneath the request directory",
        ));
    }
    Ok(directory.trim_end_matches('/').to_owned())
}

/// FR-850 "Admission": a handoff witness file that is absent or refused.
fn witness_refusal(file: &str, cause: &str, detail: &str) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some(cause.to_owned()),
        pointer: None,
        message: format!("invalid_package/{cause}: {file}: {detail}"),
    }
}

fn conflicting_model(identity: &str) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some("conflicting-definition".to_owned()),
        pointer: None,
        message: format!(
            "invalid_package/conflicting-definition: the models entry {identity} is not a domain \
             package the package records, or repeats one"
        ),
    }
}

/// The request-stage refusal of a package the strict reader refused.
fn package_refusal(refusal: fsl_package::PackageAdmissionRefusal) -> RunCause {
    let fsl_package::PackageAdmissionRefusal {
        code,
        cause,
        pointer,
        identity,
    } = refusal;
    let named = match (&identity, &pointer) {
        (Some(identity), _) => format!(": no models entry supplies {identity}"),
        (None, Some(pointer)) => format!(" at {pointer}"),
        (None, None) => String::new(),
    };
    let label = match &cause {
        Some(cause) => format!("{}/{cause}", code.as_str()),
        None => code.as_str().to_owned(),
    };
    RunCause::AnalyzePackage {
        code,
        cause,
        pointer,
        message: format!("{label}{named}"),
    }
}
