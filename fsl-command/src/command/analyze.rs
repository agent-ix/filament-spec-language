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
    analyze_document, resolve_items, AnalyzeDocumentError, AnalyzeRun, ItemSelection, Negotiate,
    SubjectSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply, Engines, SourceProvision};
use fsl_semantics::model::observation::DocumentProvision;

use super::{
    check_sources, intake_with_bytes, request, require_complete_edition, wire, FileCounts,
    FormalSource, NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
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
    read(directory, bytes, dependent_bytes).map(|(input, _)| PreparedAnalyze {
        input: Box::new(input),
    })
}

/// The request's inputs and the intake that read them, which still charges
/// the libraries and domain packages compilation reads.
fn read<'d>(
    directory: &'d Path,
    bytes: &[u8],
    dependent_bytes: usize,
) -> Result<(AnalyzeInput, super::Intake<'d>)> {
    let request: wire::AnalyzeRequest = request(bytes).map_err(|cause| match cause {
        RunCause::Json(error) => invalid("request", error.to_string()),
        cause => cause,
    })?;
    if request.package.is_some() {
        // FR-620 admits a checked-package selection, but the engines analyze
        // an in-process checked package that only a source compile builds:
        // an implementation gap (QSL-2271), not a refused request.
        return Err(RunCause::AnalyzeNotYetImplemented { member: "package" });
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
        },
        dependent_bytes,
    )?;
    let source = intake.source(&program.source)?;
    require_complete_edition(source.source())?;
    let mut documents = DocumentProvision::new();
    for document in &request.documents {
        let bytes = intake.file(&document.file, intake.remaining)?;
        let Some(reference) = fsl_semantics::model::observation::document_reference(&bytes) else {
            return Err(invalid(
                "documents",
                format!("{} declares no identity", document.file),
            ));
        };
        if documents.insert(reference, bytes).is_some() {
            return Err(invalid(
                "documents",
                format!("{} repeats a provisioned identity", document.file),
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
    };
    Ok((
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
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        let (input, mut intake) = read(directory, bytes, limits.dependent_bytes)?;
        run_input(&mut intake, input, negotiate, cancel)
    })
}

fn run_input(
    intake: &mut super::Intake<'_>,
    input: AnalyzeInput,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<RunResult> {
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
        |checked, packages, libraries, _| {
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
            let engines = Engines::fsl().map_err(|_| RunCause::AnalyzeFault)?;
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
