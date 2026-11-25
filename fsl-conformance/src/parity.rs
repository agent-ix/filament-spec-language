// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: compare every supplied non-interpreter backend with the
//! interpreter over selected corpus vectors and reproducible generated units.

use std::path::{Component, Path, PathBuf};
#[cfg(feature = "test-support")]
use std::sync::Arc;

use filament_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, FamilyOutcome, UnsupportedCause,
};
use filament_exact::{
    planned_equality, Cancel, Decimal, Integer, Meter, Outcome, ScalarLimits, Value,
};
use filament_semantic_value::catalog::Category;
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};
use fsl_eval::execution::coverage::entry_within_coverage;
use fsl_eval::execution::{
    BackendCoverage, CallLimits, EntrySelection, Interpreter, PrepareFailure, PrepareLimits,
    PreparedEntries, SuppliedBackend,
};
use fsl_eval::value::QualifiedName;
use fsl_foundation::diagnostic::{Code, LimitExceeded, StageFailure};
use fsl_foundation::SourceIdentity;
use fsl_package::CheckedPackage;
use fsl_replay::spine::{
    admit_parsed_unit_list, package, parse, preflight_unit_list, select_units, AdmittedModels,
    CheckedUnit, DependencyInput, FrontEndFailure, ParseRequest, SpineLimits,
};
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::AdmittedObservations;

use crate::call_input::CallSchemas;

/// FR-295's default seed, the ASCII bytes `FSL-JIT1` read as a big-endian
/// integer.
pub const DEFAULT_PARITY_SEED: u64 = 0x5153_4C2D_4A49_5431;

/// The generator input and the corpus read in place by [`run_parity`].
#[derive(Clone, Debug)]
pub struct ParityInputs {
    /// The corpus directory containing `complete-v1.json` and vector inputs.
    pub corpus: PathBuf,
    /// The generated-unit seed (FR-295 default: [`DEFAULT_PARITY_SEED`]).
    pub seed: u64,
    /// The number of admitted generated units (default 10,000).
    pub count: u64,
    /// Test-only admitted state-clause parity cases.
    #[cfg(feature = "test-support")]
    pub clauses: Vec<ParityClauseCase>,
}

impl ParityInputs {
    /// Inputs for `corpus`, using FR-295's default seed and unit count.
    #[must_use]
    pub fn new(corpus: impl Into<PathBuf>) -> Self {
        Self {
            corpus: corpus.into(),
            seed: DEFAULT_PARITY_SEED,
            count: 10_000,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        }
    }
}

/// One test-only, already checked state-clause parity case.
#[cfg(feature = "test-support")]
#[derive(Clone, Debug)]
pub struct ParityClauseCase {
    /// Reproducibility identifier.
    pub id: String,
    /// Exact source bytes used to create the checked package.
    pub source: Vec<u8>,
    /// Every source file forming the package.
    pub source_files: Vec<SourceFile>,
    /// The linked package whose clause was checked.
    pub package: Arc<CheckedPackage>,
    /// The selected clause name.
    pub clause: String,
    /// Admitted observations for the selected clause.
    pub observations: AdmittedObservations,
}

/// Accounting under which no budget is reached: comparing two results costs no
/// more than the values the bounded calls already built.
const UNMETERED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// Bounds for parity calls, unit generation and report retention.
#[derive(Clone, Copy, Debug)]
pub struct ParityLimits {
    /// Maximum number of retained findings per backend (default 100).
    pub max_reported: usize,
    /// Maximum number of generated draws checking may refuse (default 10,000).
    /// The next refused draw ends generation with a
    /// [`FindingKind::GeneratorExhausted`] finding.
    pub max_discarded: u64,
    /// Backend preparation bounds.
    pub prepare: PrepareLimits,
    /// Evaluation accounting bounds.
    pub accounting: ScalarLimits,
}

impl Default for ParityLimits {
    fn default() -> Self {
        Self {
            max_reported: 100,
            max_discarded: 10_000,
            prepare: PrepareLimits::default(),
            accounting: ScalarLimits {
                work_units: 1_000_000,
                ..UNMETERED
            },
        }
    }
}

/// The comparison member in which a backend differs from the interpreter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParityMember {
    /// The backend returned `Ok` where the interpreter returned `Err`, or vice versa.
    OkAgainstErr,
    /// The outcome kind.
    OutcomeKind,
    /// The completed value.
    CompletedValue,
    /// The typed outcome cause.
    TypedCause,
    /// The source location.
    Location,
    /// The operation loss records.
    Losses,
    /// An incomplete result's limit or charge point.
    IncompleteChargePoint,
    /// A `CallFailure` variant or payload.
    CallFailurePayload,
}

/// A parity finding kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FindingKind {
    /// A result member differs from the interpreter.
    Difference(ParityMember),
    /// Repeated calls to the backend returned different results.
    Nondeterministic,
    /// An unsupported entry's constructs all lie in the declared coverage.
    CoverageGap,
    /// Backend preparation failed for a unit prepared by the reference.
    PrepareFailed,
    /// The backend compared no entry.
    EmptyComparison,
    /// The interpreter returned an internal `Fault`, from a call or from
    /// preparing a unit.
    ReferenceFault,
    /// Checking refused more generated draws than
    /// [`ParityLimits::max_discarded`], so the generated units were not all
    /// run. The finding's unit and source are the last refused draw's.
    GeneratorExhausted,
    /// The interpreter's `prepare` of a unit reached the named bound: one
    /// finding for each entry of the unit.
    ReferencePrepareLimit(LimitExceeded),
    /// A selected corpus vector could not be built into a parity case, and
    /// its expected disposition does not excuse that.
    UnbuildableCase(UnbuildableCause),
}

/// One finding with the reproducibility fields FR-295 requires.
#[derive(Debug)]
pub struct Finding {
    /// The finding kind.
    pub kind: FindingKind,
    /// The non-interpreter backend.
    pub backend: ExecutionBackendChoice,
    /// Seed used for the generated corpus.
    pub seed: u64,
    /// Generated unit index or corpus vector identifier.
    pub unit: String,
    /// Exact unit source bytes.
    pub source: Vec<u8>,
    /// Every source file of a multi-file unit, including its corpus path.
    pub source_files: Vec<SourceFile>,
    /// Function entry.
    pub entry: String,
    /// The argument tuple.
    pub arguments: Vec<Value>,
    /// Typed reference result, absent only for findings that occur before a call.
    pub reference: Option<Result<Evaluation, CallFailure>>,
    /// Typed backend result, absent only for findings that occur before a call.
    pub result: Option<Result<Evaluation, CallFailure>>,
    /// Typed backend preparation failure when `kind` is `PrepareFailed`.
    pub prepare_failure: Option<PrepareFailure>,
}

/// One exact source file that forms a parity unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    /// The relative path named by the corpus, or generated-unit path.
    pub path: String,
    /// Exact source bytes.
    pub bytes: Vec<u8>,
}

/// One entry the backend listed unsupported.
#[derive(Clone, Debug)]
pub struct UnsupportedRecord {
    /// Corpus vector identifier or generated-unit index.
    pub unit: String,
    /// Function entry.
    pub entry: String,
    /// The backend's typed unsupported reason.
    pub reason: filament_eval::UnsupportedEntry,
}

/// One backend's parity totals and retained details.
#[derive(Debug)]
pub struct BackendReport {
    /// The non-interpreter backend.
    pub backend: ExecutionBackendChoice,
    /// Calls compared with the interpreter.
    pub compared: u64,
    /// Unsupported entries, including their typed reasons.
    pub unsupported: Vec<UnsupportedRecord>,
    /// Findings retained up to `max_reported`.
    pub findings: Vec<Finding>,
    /// All findings, including findings beyond the retained limit.
    pub total_findings: u64,
}

/// Why a selected corpus vector could not be built into a parity case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnbuildableCause {
    /// A named file could not be read.
    UnreadableFile(String),
    /// A named file's path is absolute or has a parent component.
    UnsafePath(String),
    /// The vector or its call file lacks the named member or file kind, or the
    /// member is not of its declared shape.
    MissingMember(&'static str),
    /// The vector names more than one call file, or names more than one source
    /// file and a later step of its build fails.
    DuplicateFile,
    /// The named call file is not a JSON document, is not an FR-336
    /// `CallInput` in a way no other cause names, or holds a refused
    /// accounting value.
    InvalidCall(String),
    /// An object context or population file is not an identified state
    /// snapshot.
    InvalidObservations,
    /// The input adapters cannot decode the argument at this position.
    UndecodableArgument(usize),
    /// The call supplies an argument at this position, the first for which
    /// the function declares no parameter.
    ExtraArgument(usize),
    /// A step of the build ended in an internal fault.
    Fault,
    /// Compiling the sources with the models ended in a typed refusal or a
    /// reached limit.
    Uncompilable,
    /// Admission of the object environment ended in a typed refusal or a
    /// reached limit.
    RefusedObservations,
    /// The sources declare no function of the called name.
    UnknownFunction(String),
    /// The owner's admission of the argument at this position ended in a typed
    /// refusal or a reached limit.
    RefusedArgument(usize),
}

impl UnbuildableCause {
    /// Whether a vector that expects a refusal may be unbuildable for this
    /// cause without a finding: only for a typed refusal FSL itself gives.
    /// Every other cause is a defect of the vector's input or of its
    /// decoding, or an internal fault, and is a finding whatever the vector
    /// expects.
    #[must_use]
    pub const fn excused_by_expected_refusal(&self) -> bool {
        match self {
            Self::Uncompilable
            | Self::RefusedObservations
            | Self::UnknownFunction(_)
            | Self::RefusedArgument(_) => true,
            Self::UnreadableFile(_)
            | Self::UnsafePath(_)
            | Self::MissingMember(_)
            | Self::DuplicateFile
            | Self::InvalidCall(_)
            | Self::InvalidObservations
            | Self::UndecodableArgument(_)
            | Self::ExtraArgument(_)
            | Self::Fault => false,
        }
    }
}

/// A selected corpus vector that is no parity case.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnbuildableCase {
    /// The vector's identifier.
    pub id: String,
    /// Why no case could be built.
    pub cause: UnbuildableCause,
    /// Whether the vector's own expected disposition is `refusal`. Such a
    /// vector is listed without a finding when its cause allows it
    /// ([`UnbuildableCause::excused_by_expected_refusal`]).
    pub expected_refusal: bool,
}

/// The report of [`run_parity`].
#[derive(Debug)]
pub struct ParityReport {
    /// The generated-unit seed.
    pub seed: u64,
    /// Generated units admitted plus corpus vectors built into parity cases.
    pub units: u64,
    /// Corpus vectors not selected as parity cases.
    pub not_selected: u64,
    /// Generated draws that checking refused.
    pub discarded: u64,
    /// Every selected corpus vector that could not be built into a parity
    /// case, in corpus order.
    pub unbuildable: Vec<UnbuildableCase>,
    /// One report per supplied backend, preserving caller order.
    pub backends: Vec<BackendReport>,
}

impl ParityReport {
    /// 0 when all backends have no finding, 1 when any finding exists.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        u8::from(
            self.backends
                .iter()
                .any(|backend| backend.total_findings != 0),
        )
    }
}

/// Why parity stopped without a verdict.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ParityRefusal {
    /// The caller supplied no non-interpreter backend.
    #[error("empty-scope: no non-interpreter backend was supplied")]
    EmptyScope,
    /// The corpus document was absent, unreadable, not JSON or without a
    /// `vectors` array, or a vector had no id or another vector's id. A
    /// problem confined to one vector's own members and files never refuses
    /// the run.
    #[error("unreadable-corpus: {0}")]
    UnreadableCorpus(PathBuf),
    /// The caller cancelled the run before it finished.
    #[error("cancelled")]
    Cancelled,
}

impl ParityRefusal {
    /// A refusal exits 2 under FR-353.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        2
    }
}

/// A supplied backend and its optional FR-560 construct coverage.
pub type BackendEntry<'a> = (
    ExecutionBackendChoice,
    &'a dyn SuppliedBackend,
    Option<&'a BackendCoverage>,
);

/// Run every backend against the interpreter over corpus and generated cases.
///
/// # Errors
/// Returns [`ParityRefusal`] when the backend set is empty, the corpus cannot
/// be read, preparing the reference reaches a limit, or `cancel` requests
/// cancellation before completion.
pub fn run_parity(
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    let interpreter = Interpreter;
    run_with_reference(&interpreter, backends, inputs, limits, cancel)
}

/// Test-only parity entry point that substitutes the reference evaluator.
#[cfg(feature = "test-support")]
pub fn run_parity_with_reference(
    reference: &dyn SuppliedBackend,
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    run_with_reference(reference, backends, inputs, limits, cancel)
}

struct ParityFunction {
    name: String,
    arguments: Vec<Vec<Value>>,
}

struct ParityClause {
    name: String,
    observations: AdmittedObservations,
}

enum UnitPackage {
    Checked(CheckedUnit),
    #[cfg(feature = "test-support")]
    Direct(Arc<CheckedPackage>),
}

impl UnitPackage {
    fn package(&self) -> &CheckedPackage {
        match self {
            Self::Checked(checked) => checked.package(),
            #[cfg(feature = "test-support")]
            Self::Direct(package) => package,
        }
    }
}

struct ParityUnit {
    id: String,
    source: Vec<u8>,
    source_files: Vec<SourceFile>,
    package: UnitPackage,
    environment: ObjectEnvironment,
    functions: Vec<ParityFunction>,
    clauses: Vec<ParityClause>,
}

struct CompiledSources {
    checked: CheckedUnit,
    models: AdmittedModels,
    packages: fsl_semantics::model::intake::PackageInput,
    wire_graph: fsl_replay::WireGraph,
}

fn run_with_reference(
    reference: &dyn SuppliedBackend,
    backends: &[BackendEntry<'_>],
    inputs: &ParityInputs,
    limits: &ParityLimits,
    cancel: &Cancel,
) -> Result<ParityReport, ParityRefusal> {
    if backends.is_empty() {
        return Err(ParityRefusal::EmptyScope);
    }
    if cancel.cause().is_some() {
        return Err(ParityRefusal::Cancelled);
    }

    let corpus = corpus_units(&inputs.corpus, cancel)?;
    let not_selected = corpus.not_selected;
    #[allow(unused_mut)]
    let mut units = corpus.units;
    #[cfg(feature = "test-support")]
    units.extend(inputs.clauses.iter().map(|clause| ParityUnit {
        id: clause.id.clone(),
        source: clause.source.clone(),
        source_files: clause.source_files.clone(),
        package: UnitPackage::Direct(Arc::clone(&clause.package)),
        environment: ObjectEnvironment::default(),
        functions: Vec::new(),
        clauses: vec![ParityClause {
            name: clause.clause.clone(),
            observations: clause.observations.clone(),
        }],
    }));
    let mut reports = backends
        .iter()
        .map(|(choice, _, _)| BackendReport {
            backend: *choice,
            compared: 0,
            unsupported: Vec::new(),
            findings: Vec::new(),
            total_findings: 0,
        })
        .collect::<Vec<_>>();
    let mut unbuildable = Vec::new();
    for (case, read) in corpus.unbuildable {
        if !(case.expected_refusal && case.cause.excused_by_expected_refusal()) {
            let source = read.source();
            let entry = read.entry.clone();
            let source_files = read.source_files();
            for report in &mut reports {
                record_finding(
                    report,
                    limits.max_reported,
                    make_finding(
                        FindingKind::UnbuildableCase(case.cause.clone()),
                        report.backend,
                        inputs.seed,
                        case.id.clone(),
                        source.clone(),
                        source_files.clone(),
                        entry.clone(),
                        Vec::new(),
                        None,
                        None,
                    ),
                );
            }
        }
        unbuildable.push(case);
    }
    let mut unit_count = 0_u64;
    for unit in units {
        check_cancel(cancel)?;
        unit_count = unit_count.saturating_add(1);
        compare_unit(
            reference,
            backends,
            &unit,
            inputs.seed,
            limits,
            cancel,
            &mut reports,
        )?;
    }

    let mut generator = ProgramGenerator::new(inputs.seed);
    let mut generated_index = 0_u64;
    let mut admitted = 0_u64;
    let mut discarded = 0_u64;
    while admitted < inputs.count {
        check_cancel(cancel)?;
        let index = generated_index;
        generated_index = generated_index.saturating_add(1);
        let (source, mut functions) = generator.unit(index);
        let checked = match compile_unit(&source, &format!("generated-{index}"), cancel) {
            Ok(checked) => checked,
            Err(_) => {
                check_cancel(cancel)?;
                discarded = discarded.saturating_add(1);
                if discarded <= limits.max_discarded {
                    continue;
                }
                for report in &mut reports {
                    record_finding(
                        report,
                        limits.max_reported,
                        make_finding(
                            FindingKind::GeneratorExhausted,
                            report.backend,
                            inputs.seed,
                            index.to_string(),
                            source.clone(),
                            vec![generated_source_file(index, &source)],
                            String::new(),
                            Vec::new(),
                            None,
                            None,
                        ),
                    );
                }
                break;
            }
        };
        materialize_composite_arguments(checked.package(), &mut functions);
        let unit = ParityUnit {
            id: index.to_string(),
            source_files: vec![generated_source_file(index, &source)],
            source,
            package: UnitPackage::Checked(checked),
            environment: ObjectEnvironment::default(),
            functions,
            clauses: Vec::new(),
        };
        unit_count = unit_count.saturating_add(1);
        compare_unit(
            reference,
            backends,
            &unit,
            inputs.seed,
            limits,
            cancel,
            &mut reports,
        )?;
        admitted = admitted.saturating_add(1);
    }

    for report in &mut reports {
        if report.compared == 0 {
            record_finding(
                report,
                limits.max_reported,
                make_finding(
                    FindingKind::EmptyComparison,
                    report.backend,
                    inputs.seed,
                    String::new(),
                    Vec::new(),
                    Vec::new(),
                    String::new(),
                    Vec::new(),
                    None,
                    None,
                ),
            );
        }
    }
    Ok(ParityReport {
        seed: inputs.seed,
        units: unit_count,
        not_selected,
        discarded,
        unbuildable,
        backends: reports,
    })
}

fn check_cancel(cancel: &Cancel) -> Result<(), ParityRefusal> {
    if cancel.cause().is_some() {
        Err(ParityRefusal::Cancelled)
    } else {
        Ok(())
    }
}

fn generated_source_file(index: u64, source: &[u8]) -> SourceFile {
    SourceFile {
        path: format!("generated-{index}.native"),
        bytes: source.to_vec(),
    }
}

/// The reference could not prepare `unit`, so none of its entries has an
/// oracle: each is a finding of `kind`, with the entry's first argument tuple,
/// for every backend, and none is compared.
fn record_unprepared_unit(
    reports: &mut [BackendReport],
    unit: &ParityUnit,
    seed: u64,
    max_reported: usize,
    kind: &FindingKind,
    reference: Option<&CallFailure>,
) {
    let functions = unit.functions.iter().map(|function| {
        let arguments = function.arguments.first().cloned().unwrap_or_default();
        (&function.name, arguments)
    });
    let clauses = unit
        .clauses
        .iter()
        .map(|clause| (&clause.name, clause_arguments(&clause.observations)));
    for (entry, arguments) in functions.chain(clauses) {
        for report in reports.iter_mut() {
            record_finding(
                report,
                max_reported,
                make_finding(
                    kind.clone(),
                    report.backend,
                    seed,
                    unit.id.clone(),
                    unit.source.clone(),
                    unit.source_files.clone(),
                    entry.clone(),
                    arguments.clone(),
                    reference.map(|failure| Err(failure.clone())),
                    None,
                ),
            );
        }
    }
}

fn compare_unit(
    reference: &dyn SuppliedBackend,
    backends: &[BackendEntry<'_>],
    unit: &ParityUnit,
    seed: u64,
    limits: &ParityLimits,
    cancel: &Cancel,
    reports: &mut [BackendReport],
) -> Result<(), ParityRefusal> {
    let package = unit.package.package();
    let selections = unit
        .functions
        .iter()
        .filter_map(|function| {
            QualifiedName::unqualified(&function.name)
                .ok()
                .map(EntrySelection::function)
        })
        .collect::<Vec<_>>();
    let mut selections = selections;
    selections.extend(unit.clauses.iter().filter_map(|clause| {
        QualifiedName::unqualified(&clause.name)
            .ok()
            .map(EntrySelection::clause)
    }));
    let reference_prepared =
        match reference.prepare_entries(package, &selections, &limits.prepare, cancel) {
            Ok(prepared) => prepared,
            Err(PrepareFailure::Cancelled(_)) => {
                return Err(ParityRefusal::Cancelled);
            }
            Err(PrepareFailure::Limit(limit)) => {
                check_cancel(cancel)?;
                record_unprepared_unit(
                    reports,
                    unit,
                    seed,
                    limits.max_reported,
                    &FindingKind::ReferencePrepareLimit(limit),
                    None,
                );
                return Ok(());
            }
            Err(PrepareFailure::Fault(fault)) => {
                check_cancel(cancel)?;
                record_unprepared_unit(
                    reports,
                    unit,
                    seed,
                    limits.max_reported,
                    &FindingKind::ReferenceFault,
                    Some(&CallFailure::Fault(fault)),
                );
                return Ok(());
            }
        };
    let call_limits = CallLimits {
        accounting: limits.accounting,
        session: Default::default(),
    };
    let environment = &unit.environment;

    // Find oracle faults before considering any backend's unsupported list.
    // A reference fault invalidates that entry for every backend, including
    // one which would otherwise skip it as unsupported.
    let mut reference_faults = std::collections::BTreeMap::new();
    for (function_index, function) in unit.functions.iter().enumerate() {
        let Some(name) = QualifiedName::unqualified(&function.name).ok() else {
            continue;
        };
        for arguments in &function.arguments {
            check_cancel(cancel)?;
            let expected = call(
                reference_prepared.as_ref(),
                &name,
                arguments,
                environment,
                &call_limits,
                cancel,
            );
            if matches!(expected, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                return Err(ParityRefusal::Cancelled);
            }
            if let Err(fault @ CallFailure::Fault(_)) = expected {
                reference_faults
                    .entry(function_index)
                    .or_insert_with(|| (fault, arguments.clone()));
                break;
            }
        }
    }
    let mut reference_clause_faults = std::collections::BTreeMap::new();
    for (clause_index, clause) in unit.clauses.iter().enumerate() {
        let Some(name) = QualifiedName::unqualified(&clause.name).ok() else {
            continue;
        };
        check_cancel(cancel)?;
        let expected = evaluate_clause(
            reference_prepared.as_ref(),
            &name,
            &clause.observations,
            limits.accounting,
            cancel,
        );
        if matches!(expected, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
            return Err(ParityRefusal::Cancelled);
        }
        if let Err(fault @ CallFailure::Fault(_)) = expected {
            reference_clause_faults.insert(clause_index, fault);
        }
    }

    for (slot, (choice, backend, coverage)) in backends.iter().enumerate() {
        check_cancel(cancel)?;
        let report = &mut reports[slot];
        let prepared = match backend.prepare_entries(package, &selections, &limits.prepare, cancel)
        {
            Ok(prepared) => prepared,
            Err(PrepareFailure::Cancelled(_)) => {
                return Err(ParityRefusal::Cancelled);
            }
            Err(failure) => {
                if cancel.cause().is_some() {
                    return Err(ParityRefusal::Cancelled);
                }
                let first = unit.functions.first();
                let first_clause = unit.clauses.first();
                let mut finding = make_finding(
                    FindingKind::PrepareFailed,
                    *choice,
                    seed,
                    unit.id.clone(),
                    unit.source.clone(),
                    unit.source_files.clone(),
                    first.map_or_else(
                        || first_clause.map_or_else(String::new, |clause| clause.name.clone()),
                        |function| function.name.clone(),
                    ),
                    first
                        .and_then(|function| function.arguments.first())
                        .cloned()
                        .or_else(|| {
                            first_clause.map(|clause| clause_arguments(&clause.observations))
                        })
                        .unwrap_or_default(),
                    None,
                    None,
                );
                finding.prepare_failure = Some(failure);
                record_finding(report, limits.max_reported, finding);
                continue;
            }
        };
        if cancel.cause().is_some() {
            return Err(ParityRefusal::Cancelled);
        }
        for (selection, reason) in prepared.unsupported() {
            let entry = selection.name.to_string();
            report.unsupported.push(UnsupportedRecord {
                unit: unit.id.clone(),
                entry: entry.clone(),
                reason: reason.clone(),
            });
            if let (Some(coverage), UnsupportedCause::Untranslatable { .. }) =
                (coverage, &reason.cause)
            {
                // The gap is decided on every construct of the entry, whichever
                // construct the backend names. An entry whose constructs cannot
                // be read is not shown to be outside the coverage, so it fails.
                if !matches!(
                    entry_within_coverage(package, selection, coverage),
                    Ok(false)
                ) {
                    record_finding(
                        report,
                        limits.max_reported,
                        make_finding(
                            FindingKind::CoverageGap,
                            *choice,
                            seed,
                            unit.id.clone(),
                            unit.source.clone(),
                            unit.source_files.clone(),
                            entry,
                            Vec::new(),
                            None,
                            None,
                        ),
                    );
                }
            }
        }

        for (function_index, function) in unit.functions.iter().enumerate() {
            let Some(name) = QualifiedName::unqualified(&function.name).ok() else {
                continue;
            };
            let unsupported = prepared.unsupported().iter().any(|(selection, _)| {
                selection.kind == fsl_eval::execution::EntryKind::Function && selection.name == name
            });
            if let Some((fault, fault_arguments)) = reference_faults.get(&function_index) {
                record_finding(
                    report,
                    limits.max_reported,
                    make_finding(
                        FindingKind::ReferenceFault,
                        *choice,
                        seed,
                        unit.id.clone(),
                        unit.source.clone(),
                        unit.source_files.clone(),
                        function.name.clone(),
                        fault_arguments.clone(),
                        Some(Err(fault.clone())),
                        None,
                    ),
                );
                continue;
            }
            for arguments in &function.arguments {
                check_cancel(cancel)?;
                if unsupported {
                    continue;
                }
                let expected = call(
                    reference_prepared.as_ref(),
                    &name,
                    arguments,
                    environment,
                    &call_limits,
                    cancel,
                );
                if matches!(expected, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                    return Err(ParityRefusal::Cancelled);
                }
                let first = call(
                    prepared.as_ref(),
                    &name,
                    arguments,
                    environment,
                    &call_limits,
                    cancel,
                );
                if matches!(first, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                    return Err(ParityRefusal::Cancelled);
                }
                let second = call(
                    prepared.as_ref(),
                    &name,
                    arguments,
                    environment,
                    &call_limits,
                    cancel,
                );
                if matches!(second, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                    return Err(ParityRefusal::Cancelled);
                }
                report.compared = report.compared.saturating_add(1);
                if difference(&first, &second, cancel)?.is_some() {
                    record_finding(
                        report,
                        limits.max_reported,
                        make_finding(
                            FindingKind::Nondeterministic,
                            *choice,
                            seed,
                            unit.id.clone(),
                            unit.source.clone(),
                            unit.source_files.clone(),
                            function.name.clone(),
                            arguments.clone(),
                            Some(first),
                            Some(second),
                        ),
                    );
                    continue;
                }
                if let Some(member) = difference(&expected, &first, cancel)? {
                    record_finding(
                        report,
                        limits.max_reported,
                        make_finding(
                            FindingKind::Difference(member),
                            *choice,
                            seed,
                            unit.id.clone(),
                            unit.source.clone(),
                            unit.source_files.clone(),
                            function.name.clone(),
                            arguments.clone(),
                            Some(expected),
                            Some(first),
                        ),
                    );
                }
            }
        }
        for (clause_index, clause) in unit.clauses.iter().enumerate() {
            let Some(name) = QualifiedName::unqualified(&clause.name).ok() else {
                continue;
            };
            let clause_arguments = clause_arguments(&clause.observations);
            if let Some(fault) = reference_clause_faults.get(&clause_index) {
                record_finding(
                    report,
                    limits.max_reported,
                    make_finding(
                        FindingKind::ReferenceFault,
                        *choice,
                        seed,
                        unit.id.clone(),
                        unit.source.clone(),
                        unit.source_files.clone(),
                        clause.name.clone(),
                        clause_arguments,
                        Some(Err(fault.clone())),
                        None,
                    ),
                );
                continue;
            }
            let unsupported = prepared.unsupported().iter().any(|(selection, _)| {
                selection.kind == fsl_eval::execution::EntryKind::Clause && selection.name == name
            });
            if unsupported {
                continue;
            }
            check_cancel(cancel)?;
            let expected = evaluate_clause(
                reference_prepared.as_ref(),
                &name,
                &clause.observations,
                limits.accounting,
                cancel,
            );
            if matches!(expected, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                return Err(ParityRefusal::Cancelled);
            }
            let first = evaluate_clause(
                prepared.as_ref(),
                &name,
                &clause.observations,
                limits.accounting,
                cancel,
            );
            if matches!(first, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                return Err(ParityRefusal::Cancelled);
            }
            let second = evaluate_clause(
                prepared.as_ref(),
                &name,
                &clause.observations,
                limits.accounting,
                cancel,
            );
            if matches!(second, Err(CallFailure::Cancelled(_))) || cancel.cause().is_some() {
                return Err(ParityRefusal::Cancelled);
            }
            report.compared = report.compared.saturating_add(1);
            if difference(&first, &second, cancel)?.is_some() {
                record_finding(
                    report,
                    limits.max_reported,
                    make_finding(
                        FindingKind::Nondeterministic,
                        *choice,
                        seed,
                        unit.id.clone(),
                        unit.source.clone(),
                        unit.source_files.clone(),
                        clause.name.clone(),
                        clause_arguments.clone(),
                        Some(first),
                        Some(second),
                    ),
                );
                continue;
            }
            if let Some(member) = difference(&expected, &first, cancel)? {
                record_finding(
                    report,
                    limits.max_reported,
                    make_finding(
                        FindingKind::Difference(member),
                        *choice,
                        seed,
                        unit.id.clone(),
                        unit.source.clone(),
                        unit.source_files.clone(),
                        clause.name.clone(),
                        clause_arguments,
                        Some(expected),
                        Some(first),
                    ),
                );
            }
        }
    }
    Ok(())
}

fn call(
    prepared: &dyn PreparedEntries,
    entry: &QualifiedName,
    arguments: &[Value],
    environment: &ObjectEnvironment,
    limits: &CallLimits,
    cancel: &Cancel,
) -> Result<Evaluation, CallFailure> {
    let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel.clone())
        .map_err(CallFailure::Supplied)?;
    prepared.call(entry, arguments, environment, limits, &mut run, cancel)
}

fn evaluate_clause(
    prepared: &dyn PreparedEntries,
    clause: &QualifiedName,
    observations: &AdmittedObservations,
    accounting: ScalarLimits,
    cancel: &Cancel,
) -> Result<Evaluation, CallFailure> {
    prepared.evaluate(clause, observations, &accounting, cancel)
}

fn clause_arguments(observations: &AdmittedObservations) -> Vec<Value> {
    let mut arguments = Vec::with_capacity(observations.parameters.len() + 2);
    arguments.push(Value::Reference(observations.self_object.clone()));
    arguments.extend(
        observations
            .parameters
            .iter()
            .map(|(_, value)| value.clone()),
    );
    if let Some(result) = &observations.result {
        arguments.push(result.clone());
    }
    arguments
}

fn difference(
    expected: &Result<Evaluation, CallFailure>,
    actual: &Result<Evaluation, CallFailure>,
    cancel: &Cancel,
) -> Result<Option<ParityMember>, ParityRefusal> {
    match (expected, actual) {
        (Err(_), Ok(_)) | (Ok(_), Err(_)) => Ok(Some(ParityMember::OkAgainstErr)),
        (Err(expected), Err(actual)) => {
            Ok((expected != actual).then_some(ParityMember::CallFailurePayload))
        }
        (Ok(expected), Ok(actual)) => compare_evaluations(expected, actual, cancel),
    }
}

fn compare_evaluations(
    expected: &Evaluation,
    actual: &Evaluation,
    cancel: &Cancel,
) -> Result<Option<ParityMember>, ParityRefusal> {
    match (&expected.outcome, &actual.outcome) {
        (
            FamilyOutcome::Evaluated(Outcome::Completed(_)),
            FamilyOutcome::Evaluated(Outcome::Completed(_)),
        )
        | (
            FamilyOutcome::Evaluated(Outcome::Undefined(_)),
            FamilyOutcome::Evaluated(Outcome::Undefined(_)),
        )
        | (
            FamilyOutcome::Evaluated(Outcome::Refused(_)),
            FamilyOutcome::Evaluated(Outcome::Refused(_)),
        )
        | (
            FamilyOutcome::Evaluated(Outcome::Incomplete(_)),
            FamilyOutcome::Evaluated(Outcome::Incomplete(_)),
        )
        | (FamilyOutcome::FamilyEvaluated(_), FamilyOutcome::FamilyEvaluated(_)) => {}
        _ => return Ok(Some(ParityMember::OutcomeKind)),
    }
    match (&expected.outcome, &actual.outcome) {
        (
            FamilyOutcome::Evaluated(Outcome::Completed(a)),
            FamilyOutcome::Evaluated(Outcome::Completed(b)),
        ) => {
            if cancel.cause().is_some() {
                return Err(ParityRefusal::Cancelled);
            }
            let equal = completed_values_equal(a, b, cancel)?;
            if !equal {
                return Ok(Some(ParityMember::CompletedValue));
            }
        }
        (
            FamilyOutcome::Evaluated(Outcome::Undefined(a)),
            FamilyOutcome::Evaluated(Outcome::Undefined(b)),
        ) if a != b => return Ok(Some(ParityMember::TypedCause)),
        (
            FamilyOutcome::Evaluated(Outcome::Refused(a)),
            FamilyOutcome::Evaluated(Outcome::Refused(b)),
        ) if a != b => return Ok(Some(ParityMember::TypedCause)),
        (
            FamilyOutcome::Evaluated(Outcome::Incomplete(a)),
            FamilyOutcome::Evaluated(Outcome::Incomplete(b)),
        ) if a != b => return Ok(Some(ParityMember::IncompleteChargePoint)),
        (FamilyOutcome::FamilyEvaluated(a), FamilyOutcome::FamilyEvaluated(b)) => {
            let equal = match (a, b) {
                (
                    filament_eval::FamilyResult::Refused(a),
                    filament_eval::FamilyResult::Refused(b),
                ) => {
                    a.catalog_code() == b.catalog_code() && a.catalog_fields() == b.catalog_fields()
                }
                (
                    filament_eval::FamilyResult::Undefined(a),
                    filament_eval::FamilyResult::Undefined(b),
                ) => {
                    a.catalog_code() == b.catalog_code()
                        && a.undefined_record() == b.undefined_record()
                }
                _ => false,
            };
            if !equal {
                return Ok(Some(ParityMember::TypedCause));
            }
        }
        _ => {}
    }
    if expected.location != actual.location {
        return Ok(Some(ParityMember::Location));
    }
    if expected.losses != actual.losses {
        return Ok(Some(ParityMember::Losses));
    }
    Ok(None)
}

fn completed_values_equal(
    left: &Value,
    right: &Value,
    cancel: &Cancel,
) -> Result<bool, ParityRefusal> {
    check_cancel(cancel)?;
    let outcome = planned_equality(
        left,
        right,
        &mut Meter::new(UNMETERED).with_cancel(cancel.clone()),
    );
    check_cancel(cancel)?;
    match outcome {
        Outcome::Completed(equal) => Ok(equal),
        // With no budget to reach and the run not cancelled, a comparison that
        // does not complete has not shown the values equal.
        Outcome::Incomplete(_) | Outcome::Undefined(_) | Outcome::Refused(_) => Ok(false),
    }
}

fn make_finding(
    kind: FindingKind,
    backend: ExecutionBackendChoice,
    seed: u64,
    unit: String,
    source: Vec<u8>,
    source_files: Vec<SourceFile>,
    entry: String,
    arguments: Vec<Value>,
    reference: Option<Result<Evaluation, CallFailure>>,
    result: Option<Result<Evaluation, CallFailure>>,
) -> Finding {
    Finding {
        kind,
        backend,
        seed,
        unit,
        source,
        source_files,
        entry,
        arguments,
        reference,
        result,
        prepare_failure: None,
    }
}

fn record_finding(report: &mut BackendReport, limit: usize, finding: Finding) {
    report.total_findings = report.total_findings.saturating_add(1);
    if report.findings.len() < limit {
        report.findings.push(finding);
    }
}

struct CorpusUnits {
    units: Vec<ParityUnit>,
    unbuildable: Vec<(UnbuildableCase, CaseInputs)>,
    not_selected: u64,
}

/// What was read of a selected vector, kept for the report whether or not the
/// vector builds into a case.
#[derive(Default)]
struct CaseInputs {
    sources: Vec<(String, Vec<u8>)>,
    entry: String,
}

impl CaseInputs {
    fn source(&self) -> Vec<u8> {
        let mut joined = Vec::new();
        for (index, (_, bytes)) in self.sources.iter().enumerate() {
            if index != 0 {
                joined.push(b'\n');
            }
            joined.extend_from_slice(bytes);
        }
        joined
    }

    fn source_files(self) -> Vec<SourceFile> {
        self.sources
            .into_iter()
            .map(|(path, bytes)| SourceFile { path, bytes })
            .collect()
    }
}

struct BuiltCase {
    checked: CheckedUnit,
    environment: ObjectEnvironment,
    arguments: Vec<Value>,
}

/// The parity cases of the corpus at `path`. Only the corpus document itself
/// refuses the run, its vector ids included: a selected vector that cannot be
/// built is listed.
fn corpus_units(path: &Path, cancel: &Cancel) -> Result<CorpusUnits, ParityRefusal> {
    let corpus_path = path.join("complete-v1.json");
    let corpus_bytes = std::fs::read(&corpus_path)
        .map_err(|_| ParityRefusal::UnreadableCorpus(corpus_path.clone()))?;
    let document = crate::deep_json::DeepJson::read(&corpus_bytes)
        .map_err(|_| ParityRefusal::UnreadableCorpus(corpus_path.clone()))?;
    let vectors = document
        .get("vectors")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ParityRefusal::UnreadableCorpus(corpus_path.clone()))?;
    // Every report row names its vector by id, so every vector has its own.
    let mut identified = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for vector in vectors {
        let id = vector
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| ids.insert(*id))
            .ok_or_else(|| ParityRefusal::UnreadableCorpus(corpus_path.clone()))?;
        identified.push((id, vector));
    }
    let mut units = Vec::new();
    let mut unbuildable = Vec::new();
    let mut not_selected = 0_u64;
    let schemas = CallSchemas::published();
    for (id, vector) in identified {
        check_cancel(cancel)?;
        let selected = vector
            .pointer("/subject/consumer_type")
            .and_then(|value| value.as_str())
            == Some("runtime")
            && vector
                .pointer("/subject/operation")
                .and_then(|value| value.as_str())
                == Some("run")
            && vector
                .pointer("/applicability/consumer_types")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|types| types.iter().any(|kind| kind.as_str() == Some("runtime")));
        if !selected {
            not_selected = not_selected.saturating_add(1);
            continue;
        }
        let id = id.to_owned();
        let mut inputs = CaseInputs::default();
        match build_corpus_case(path, schemas.as_ref(), vector, &mut inputs, cancel) {
            Ok(Some(case)) => units.push(ParityUnit {
                id,
                source: inputs.source(),
                functions: vec![ParityFunction {
                    name: inputs.entry.clone(),
                    arguments: vec![case.arguments],
                }],
                source_files: inputs.source_files(),
                package: UnitPackage::Checked(case.checked),
                environment: case.environment,
                clauses: Vec::new(),
            }),
            Ok(None) => not_selected = not_selected.saturating_add(1),
            Err(cause) => {
                // A build the caller cancelled says nothing about the vector.
                check_cancel(cancel)?;
                let expected_refusal = vector
                    .pointer("/expected/disposition")
                    .and_then(serde_json::Value::as_str)
                    == Some("refusal");
                unbuildable.push((
                    UnbuildableCase {
                        id,
                        cause,
                        expected_refusal,
                    },
                    inputs,
                ));
            }
        }
    }
    check_cancel(cancel)?;
    Ok(CorpusUnits {
        units,
        unbuildable,
        not_selected,
    })
}

/// The bytes of the file a vector names `relative` to the corpus at `root`.
/// An absolute or parent-traversing path is refused unread.
fn named_file(root: &Path, relative: &str) -> Result<Vec<u8>, UnbuildableCause> {
    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(UnbuildableCause::UnsafePath(relative.to_owned()));
    }
    std::fs::read(root.join(relative_path))
        .map_err(|_| UnbuildableCause::UnreadableFile(relative.to_owned()))
}

/// Whether the vector's call file can be read and names a clause. Such a
/// vector is no parity case, whatever else it names.
fn names_a_clause(root: &Path, files: &[serde_json::Value]) -> bool {
    files
        .iter()
        .find(|file| file.get("kind").and_then(serde_json::Value::as_str) == Some("call"))
        .and_then(|file| file.get("path").and_then(serde_json::Value::as_str))
        .and_then(|relative| named_file(root, relative).ok())
        .and_then(|bytes| crate::deep_json::DeepJson::read(&bytes).ok())
        .is_some_and(|call| {
            call.get("kind").and_then(serde_json::Value::as_str) == Some("clause")
                || call
                    .get("entry")
                    .and_then(|entry| entry.get("kind"))
                    .and_then(serde_json::Value::as_str)
                    == Some("clause")
        })
}

/// Build one selected vector's call, or `None` for a clause call. A vector
/// with several defects has the cause of the first met here.
fn build_corpus_case(
    root: &Path,
    schemas: Option<&CallSchemas>,
    vector: &serde_json::Value,
    inputs: &mut CaseInputs,
    cancel: &Cancel,
) -> Result<Option<BuiltCase>, UnbuildableCause> {
    use UnbuildableCause::{DuplicateFile, MissingMember};
    let files = vector
        .pointer("/input/files")
        .and_then(serde_json::Value::as_array)
        .ok_or(MissingMember("/input/files"))?;
    if names_a_clause(root, files) {
        return Ok(None);
    }
    let mut calls = Vec::new();
    let mut models = Vec::new();
    let mut observations = Vec::new();
    for file in files {
        let kind = file
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or(MissingMember("/input/files/*/kind"))?;
        let relative = file
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or(MissingMember("/input/files/*/path"))?;
        let bytes = named_file(root, relative)?;
        match kind {
            "source" => inputs.sources.push((relative.to_owned(), bytes)),
            "model" => models.push(bytes),
            "object-context" | "population" => observations.push(bytes),
            "call" => calls.push((relative, bytes)),
            _ => {}
        }
    }
    if calls.len() > 1 {
        return Err(DuplicateFile);
    }
    let call = calls.pop().ok_or(MissingMember("call file"))?;
    if inputs.sources.is_empty() {
        return Err(MissingMember("source file"));
    }
    let several_sources = inputs.sources.len() > 1;
    build_call(schemas, call, &models, &observations, inputs, cancel)
        .map(Some)
        .map_err(|cause| {
            if several_sources {
                DuplicateFile
            } else {
                cause
            }
        })
}

/// Read the call document, compile the sources, admit the observations and
/// adapt each argument, in that order.
fn build_call(
    schemas: Option<&CallSchemas>,
    (call_path, call_bytes): (&str, Vec<u8>),
    models: &[Vec<u8>],
    observations: &[Vec<u8>],
    inputs: &mut CaseInputs,
    cancel: &Cancel,
) -> Result<BuiltCase, UnbuildableCause> {
    use UnbuildableCause::MissingMember;
    let invalid_call = || UnbuildableCause::InvalidCall(call_path.to_owned());
    let call = crate::deep_json::DeepJson::read(&call_bytes).map_err(|_| invalid_call())?;
    let function = call
        .get("function")
        .and_then(serde_json::Value::as_str)
        .ok_or(MissingMember("/function"))?;
    inputs.entry = function.to_owned();
    let argument_values = call
        .get("arguments")
        .and_then(serde_json::Value::as_array)
        .ok_or(MissingMember("/arguments"))?
        .iter()
        .collect::<Vec<_>>();
    let schemas = schemas.ok_or(UnbuildableCause::Fault)?;
    if !schemas.admits_call(&call) {
        return Err(argument_values
            .iter()
            .position(|argument| !schemas.admits_argument(argument))
            .map_or_else(invalid_call, UnbuildableCause::UndecodableArgument));
    }
    // The schema bounds an accounting value's digits, not its size.
    let accounting_refused = call
        .get("accounting")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|counters| {
            counters.values().any(|value| {
                value
                    .as_str()
                    .and_then(|spelling| spelling.parse::<u64>().ok())
                    .is_none()
            })
        });
    if accounting_refused {
        return Err(invalid_call());
    }
    let compiled = compile_sources_with_models(&inputs.sources, models, cancel).map_err(
        |stop| match stop {
            CompileStop::Refused => UnbuildableCause::Uncompilable,
            CompileStop::Fault => UnbuildableCause::Fault,
        },
    )?;
    let mut environment =
        admit_corpus_object_environment(&compiled, observations, &argument_values)?;
    let signature = compiled
        .checked
        .package()
        .graph()
        .callable(function)
        .ok_or_else(|| UnbuildableCause::UnknownFunction(function.to_owned()))?;
    let mut population_meter = fsl_semantics::model::population::AdmissionMeter::new(
        fsl_semantics::model::population::PopulationAdmissionLimits::default(),
    );
    // A call may supply a leading part of the parameters: both sides are then
    // called with that prefix.
    let arguments = argument_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let (_, value_type) = signature
                .parameters
                .get(index)
                .ok_or(UnbuildableCause::ExtraArgument(index))?;
            corpus_input_value(
                value,
                value_type,
                &compiled,
                observations,
                &mut environment,
                &mut population_meter,
            )
            .map_err(|stop| stop.at(index))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BuiltCase {
        checked: compiled.checked,
        environment,
        arguments,
    })
}

fn materialize_composite_arguments(package: &CheckedPackage, functions: &mut [ParityFunction]) {
    for function in functions {
        let Some(signature) = package.graph().callable(&function.name) else {
            continue;
        };
        for (tuple_index, tuple) in function.arguments.iter_mut().enumerate() {
            for (position, argument) in tuple.iter_mut().enumerate() {
                let Some((_, value_type)) = signature.parameters.get(position) else {
                    continue;
                };
                let element = Value::Integer(Integer::from((tuple_index % 2) as i64));
                *argument = match value_type {
                    filament_exact::ValueType::Composite(declaration) => {
                        filament_exact::from_admitted_slots(
                            declaration.clone(),
                            vec![filament_exact::FieldValue::Present(element)].into_boxed_slice(),
                        )
                    }
                    filament_exact::ValueType::Collection(collection) => {
                        filament_exact::from_admitted(collection.as_ref().clone(), vec![element])
                    }
                    _ => continue,
                };
            }
        }
    }
}

/// Why compiling sources gave no unit.
#[derive(Clone, Copy, Debug)]
enum CompileStop {
    /// A typed refusal, a reached limit or a cancellation.
    Refused,
    /// An internal fault.
    Fault,
}

impl CompileStop {
    fn of_code(code: Code) -> Self {
        if code.category() == Category::InternalFailure {
            Self::Fault
        } else {
            Self::Refused
        }
    }

    fn of_stage(failure: FrontEndFailure) -> Self {
        match failure {
            StageFailure::Refused(refusal) => Self::of_code(refusal.code()),
            StageFailure::RefusedWithPayload(refusal, _) => Self::of_code(refusal.code()),
            StageFailure::Limit(_)
            | StageFailure::LimitWithPayload(..)
            | StageFailure::Cancelled(_) => Self::Refused,
            StageFailure::Fault(_) => Self::Fault,
        }
    }
}

fn compile_unit(source: &[u8], path: &str, cancel: &Cancel) -> Result<CheckedUnit, CompileStop> {
    compile_sources_with_models(&[(path.to_owned(), source.to_vec())], &[], cancel)
        .map(|compiled| compiled.checked)
}

fn compile_sources_with_models(
    sources: &[(String, Vec<u8>)],
    model_bytes: &[Vec<u8>],
    cancel: &Cancel,
) -> Result<CompiledSources, CompileStop> {
    compile_sources_with_mode(sources, model_bytes, None, cancel)
}

fn compile_sources_with_mode(
    sources: &[(String, Vec<u8>)],
    model_bytes: &[Vec<u8>],
    check_mode: Option<CheckMode>,
    cancel: &Cancel,
) -> Result<CompiledSources, CompileStop> {
    let limits = SpineLimits::default();
    let identities = sources
        .iter()
        .map(|(path, _)| SourceIdentity::new("fsl-conformance", path))
        .collect::<Vec<_>>();
    let requests = sources
        .iter()
        .zip(&identities)
        .map(|((path, bytes), identity)| ParseRequest {
            source: identity,
            path,
            bytes,
        })
        .collect::<Vec<_>>();
    let dependencies = DependencyInput::default();
    preflight_unit_list(&requests, &dependencies)
        .map_err(|refusal| CompileStop::of_code(refusal.code()))?;
    let parsed = requests
        .iter()
        .map(|request| {
            parse(request, limits.source, cancel)
                .map_err(CompileStop::of_stage)
                .map(|staged| staged.into_value())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admitted = admit_parsed_unit_list(parsed, &dependencies)
        .map_err(|refusal| CompileStop::of_code(refusal.code()))?;
    let packages = package_input(model_bytes.iter().map(Vec::as_slice))
        .map_err(|refusal| CompileStop::of_code(refusal.code))?;
    let admitted_models: AdmittedModels =
        select_units(&admitted, &packages, limits.model, limits.source, cancel)
            .map_err(CompileStop::of_stage)?
            .into_value();
    let checked = fsl_replay::spine::check_units_with_refinement_internals(
        &admitted,
        "parity".to_owned(),
        fsl_replay::spine::SourceInputs {
            check_mode,
            ..Default::default()
        },
        &admitted_models,
        &dependencies,
        limits,
        cancel,
    )
    .map_err(CompileStop::of_stage)?
    .into_value();
    let checked = checked.into_iter().next().ok_or(CompileStop::Fault)?;
    let emitted = package(&checked, limits.package, cancel)
        .map_err(CompileStop::of_stage)?
        .into_value();
    let wire_graph = fsl_replay::WireGraph::of_package_bytes(emitted.package().bytes())
        .map_err(|_| CompileStop::Fault)?;
    Ok(CompiledSources {
        checked,
        models: admitted_models,
        packages,
        wire_graph,
    })
}

fn admit_corpus_object_environment(
    compiled: &CompiledSources,
    observations: &[Vec<u8>],
    arguments: &[&serde_json::Value],
) -> Result<ObjectEnvironment, UnbuildableCause> {
    if observations.is_empty() {
        return Ok(ObjectEnvironment::default());
    }
    let identified = observations
        .iter()
        .map(|bytes| {
            fsl_semantics::model::observation::document_reference(bytes)
                .map(|reference| (reference, bytes.clone()))
                .ok_or(UnbuildableCause::InvalidObservations)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected = identified
        .first()
        .map(|(reference, _)| reference.clone())
        .ok_or(UnbuildableCause::InvalidObservations)?;
    let provision = identified
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let models = compiled
        .models
        .models()
        .iter()
        .map(|model| model.view.model_selection().clone())
        .collect::<Vec<_>>();
    let required = arguments
        .iter()
        .filter(|argument| {
            argument.get("kind").and_then(serde_json::Value::as_str) == Some("population")
        })
        .filter_map(|argument| argument.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    fsl_semantics::model::observation::admit_trace_snapshot(
        &models,
        compiled.checked.package().graph().scope().types(),
        &compiled.packages,
        SpineLimits::default().model,
        &provision,
        &selected,
        fsl_semantics::model::observation::ObservationLimits::default(),
        &required,
    )
    .map(|(environment, _)| environment)
    .map_err(|failure| match ArgumentStop::of_admission(&failure) {
        ArgumentStop::Refused => UnbuildableCause::RefusedObservations,
        ArgumentStop::Undecodable | ArgumentStop::Fault => UnbuildableCause::Fault,
    })
}

/// Why an argument gave no value.
#[derive(Clone, Copy, Debug)]
enum ArgumentStop {
    /// The input adapters cannot decode it.
    Undecodable,
    /// The owner's admission refused it or reached a limit.
    Refused,
    /// An internal fault.
    Fault,
}

impl ArgumentStop {
    fn of_admission(failure: &fsl_semantics::model::observation::AdmissionFailure) -> Self {
        use fsl_semantics::model::observation::AdmissionFailure;
        match failure {
            AdmissionFailure::Refused(_) | AdmissionFailure::Incomplete(_) => Self::Refused,
            AdmissionFailure::Fault(_) => Self::Fault,
        }
    }

    fn at(self, position: usize) -> UnbuildableCause {
        match self {
            Self::Undecodable => UnbuildableCause::UndecodableArgument(position),
            Self::Refused => UnbuildableCause::RefusedArgument(position),
            Self::Fault => UnbuildableCause::Fault,
        }
    }
}

/// Decode one corpus `CallInput` argument by its tag, as the FR-350 adapter
/// does. A value of another type than the parameter's is left to call
/// admission.
fn corpus_input_value(
    value: &serde_json::Value,
    value_type: &filament_exact::ValueType,
    compiled: &CompiledSources,
    observations: &[Vec<u8>],
    environment: &mut ObjectEnvironment,
    population_meter: &mut fsl_semantics::model::population::AdmissionMeter,
) -> Result<Value, ArgumentStop> {
    use ArgumentStop::Undecodable;
    match value.get("kind").and_then(serde_json::Value::as_str) {
        Some("boolean") => value
            .get("value")
            .and_then(serde_json::Value::as_bool)
            .map(Value::Boolean)
            .ok_or(Undecodable),
        Some("integer") => value
            .get("decimal")
            .and_then(serde_json::Value::as_str)
            .and_then(|spelling| spelling.parse::<Integer>().ok())
            .map(Value::Integer)
            .ok_or(Undecodable),
        Some("population") => {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .ok_or(Undecodable)?;
            if let filament_exact::ValueType::Population(maximum) = value_type {
                if environment.population_binding(name).is_none() {
                    admit_population(
                        name,
                        *maximum,
                        compiled,
                        observations,
                        environment,
                        population_meter,
                    )?;
                }
            }
            environment
                .population_binding(name)
                .map(|binding| Value::Population(binding.population_id()))
                .ok_or(Undecodable)
        }
        Some("reference") => {
            let universe = compiled
                .wire_graph
                .key_of_value(value.get("universe").ok_or(Undecodable)?)
                .map_err(|_| Undecodable)?;
            let object_type = compiled
                .wire_graph
                .key_of_value(value.get("type").ok_or(Undecodable)?)
                .map_err(|_| Undecodable)?;
            let identity = value
                .get("object_identity")
                .and_then(serde_json::Value::as_str)
                .ok_or(Undecodable)?;
            for reference in environment.references() {
                if reference.universe().root_type() != &universe
                    || reference.object().as_str() != identity
                {
                    continue;
                }
                let original = reference.object_type().original();
                let admitted_type = fsl_semantics::model::key::model_node_key(
                    original.package(),
                    "model",
                    "object_type",
                    original.node(),
                )
                .map_err(|_| Undecodable)?;
                if admitted_type == object_type {
                    return Ok(Value::Reference(reference.clone()));
                }
            }
            Err(Undecodable)
        }
        _ => Err(Undecodable),
    }
}

/// Bind the admitted population `name` as the argument of a parameter with
/// the member bound `maximum`.
fn admit_population(
    name: &str,
    maximum: Option<u64>,
    compiled: &CompiledSources,
    observations: &[Vec<u8>],
    environment: &mut ObjectEnvironment,
    population_meter: &mut fsl_semantics::model::population::AdmissionMeter,
) -> Result<(), ArgumentStop> {
    use fsl_semantics::model::population::AdmissionOutcome;
    use ArgumentStop::Undecodable;
    let node = environment.population_node(name).ok_or(Undecodable)?;
    let provenance = environment.population_provenance(node).ok_or(Undecodable)?;
    let declaration = provenance.declaration();
    let model = compiled
        .models
        .models()
        .iter()
        .find(|model| model.view.population(declaration).is_some())
        .ok_or(Undecodable)?;
    let snapshot = crate::deep_json::DeepJson::read(observations.first().ok_or(Undecodable)?)
        .map_err(|_| Undecodable)?;
    let row = snapshot
        .get("populations")
        .and_then(serde_json::Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("population").and_then(serde_json::Value::as_str) == Some(name))
        })
        .ok_or(Undecodable)?;
    let member_order = row
        .get("objects")
        .and_then(serde_json::Value::as_array)
        .ok_or(Undecodable)?
        .iter()
        .map(|member| {
            member
                .get("key")
                .and_then(serde_json::Value::as_str)
                .ok_or(Undecodable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let document = fsl_semantics::model::observation::admitted_population_document(
        &model.view,
        compiled.checked.package().graph().scope().types(),
        environment,
        name,
        &member_order,
    )
    .map_err(|failure| ArgumentStop::of_admission(&failure))?;
    let binding = match fsl_semantics::model::population::admit_binding(
        &model.view,
        &document,
        declaration,
        fsl_semantics::model::dispatch::GeneralizationClosure::Closed,
        maximum,
        population_meter,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        AdmissionOutcome::Refused(_)
        | AdmissionOutcome::UnknownClosure(_)
        | AdmissionOutcome::Incomplete(_) => return Err(ArgumentStop::Refused),
    };
    *environment = std::mem::take(environment)
        .with_population(binding)
        .map_err(|_| Undecodable)?;
    Ok(())
}

struct ProgramGenerator {
    state: u64,
}

impl ProgramGenerator {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit(&mut self, index: u64) -> (Vec<u8>, Vec<ParityFunction>) {
        let function_count = self.next() % 7;
        let mut source = String::from(
            "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nrecord Pair { x: Int[-9, 9]; }\n",
        );
        let helper = format!("parity_{index}_base");
        source.push_str(&format!(
            "function {helper} using v(): Int[-9, 9] pure {{ 1 }}\n"
        ));
        let typed_name = format!("parity_{index}_0");
        let (typed_declaration, typed_arguments) = match index % 7 {
            0 => (
                format!(
                    "function {typed_name} using v(x: Decimal[-100, 100; 0, 2; nearest-even]): Decimal[-100, 100; 0, 2; nearest-even] pure {{ x }}\n"
                ),
                [-9_i64, 9, -1, 1]
                    .into_iter()
                    .map(|coefficient| {
                        vec![Value::Decimal(Decimal::new(Integer::from(coefficient), 0))]
                    })
                    .collect(),
            ),
            1 => (
                format!(
                    "function {typed_name} using v(x: Boolean): Boolean pure {{ if x then false else true }}\n"
                ),
                [true, false, false, true]
                    .into_iter()
                    .map(|value| vec![Value::Boolean(value)])
                    .collect(),
            ),
            2 => (
                format!("function {typed_name} using v(x: Int[-9, 9]): Int[-9, 9] pure {{ x }}\n"),
                [-9_i64, 9, -1, 1]
                    .into_iter()
                    .map(|value| vec![Value::Integer(Integer::from(value))])
                    .collect(),
            ),
            3 => (
                format!(
                    "function {typed_name} using v(x: Rational[-9, 9; 1, 9]): Rational[-9, 9; 1, 9] pure {{ x }}\n"
                ),
                [(-1_i64, 1_i64), (1, 1), (-1, 2), (1, 2)]
                    .into_iter()
                    .map(|(numerator, denominator)| {
                        vec![Value::Rational(
                            filament_exact::Rational::new(
                                Integer::from(numerator),
                                Integer::from(denominator),
                            )
                            .expect("nonzero denominator"),
                        )]
                    })
                    .collect(),
            ),
            4 => (
                format!(
                    "function {typed_name} using v(x: Text[0, 8; nfc]): Text[0, 8; nfc] pure {{ x }}\n"
                ),
                ["", "a", "é", "hi"]
                    .into_iter()
                    .map(|value| vec![generated_text(value)])
                    .collect(),
            ),
            5 => (
                format!("function {typed_name} using v(): Pair pure {{ Pair{{x: 1}} }}\n"),
                vec![Vec::new(); 4],
            ),
            _ => (
                format!(
                    "function {typed_name} using v(): Sequence<Int[0, 1]>[0, 2] pure {{ sequence[0, 1] }}\n"
                ),
                vec![Vec::new(); 4],
            ),
        };
        source.push_str(&typed_declaration);
        let mut functions = vec![
            ParityFunction {
                name: helper.clone(),
                arguments: vec![Vec::new(); 4],
            },
            ParityFunction {
                name: typed_name,
                arguments: typed_arguments,
            },
        ];
        for function_index in 0..function_count {
            let name = format!("parity_{index}_{}", function_index + 1);
            let arity = (self.next() % 5) as usize;
            let parameters = (0..arity)
                .map(|parameter| format!("x{parameter}: Int[-9, 9]"))
                .collect::<Vec<_>>();
            let left = if arity == 0 { "1" } else { "x0" };
            let right = if arity < 2 { "2" } else { "x1" };
            let constant = (self.next() % 4) as i64;
            let expression = match self.next() % 5 {
                0 => format!("{left} + {right}"),
                1 => format!("let local = {left} in local - {right}"),
                2 => format!("if {left} <= 0 then {right} else {left}"),
                3 => format!("{}() + {left}", helper),
                _ => format!("{left} + {constant}"),
            };
            source.push_str(&format!(
                "function {name} using v({}): Int[-30, 30] pure {{ {expression} }}\n",
                parameters.join(", ")
            ));
            let arguments = (0..4)
                .map(|tuple| {
                    (0..arity)
                        .map(|parameter| {
                            let value = match tuple {
                                0 => -9,
                                1 => 9,
                                2 if parameter % 2 == 0 => -9,
                                2 => 9,
                                _ => (self.next() % 19) as i64 - 9,
                            };
                            Value::Integer(Integer::from(value))
                        })
                        .collect()
                })
                .collect();
            functions.push(ParityFunction { name, arguments });
        }
        (source.into_bytes(), functions)
    }
}

fn generated_text(text: &str) -> Value {
    use filament_exact::{admit_text, TextPayload, TextProfile, TextType};
    let payload = TextPayload::from_utf8(text.as_bytes()).expect("generated UTF-8 text");
    let text_type = TextType::new(0, 8, TextProfile::Nfc).expect("bounded NFC text type");
    match admit_text(&payload, &text_type, &mut Meter::new(UNMETERED)) {
        Outcome::Completed(value) => Value::Text(value),
        other => panic!("generated text is admitted: {other:?}"),
    }
}

#[cfg(test)]
mod generator_tests {
    use super::*;
    use fsl_eval::execution::{
        CallLimits, ExecutionBackend, InterpreterPrepared, PrepareLimits, PreparedEntries,
    };
    use fsl_semantics::model::observation::AdmittedObservations;
    use ix_trace_rs::trace;

    #[derive(Clone, Copy)]
    enum KernelMutation {
        TypedCause,
        Location,
    }

    #[derive(Clone, Copy)]
    struct KernelMutationBackend(KernelMutation);

    struct KernelMutationPrepared<'p> {
        inner: InterpreterPrepared<'p>,
        mutation: KernelMutation,
    }

    impl ExecutionBackend for KernelMutationBackend {
        type Prepared<'p> = KernelMutationPrepared<'p>;

        fn prepare<'p>(
            &self,
            package: &'p CheckedPackage,
            entries: &[EntrySelection],
            limits: &PrepareLimits,
            cancel: &Cancel,
        ) -> Result<Self::Prepared<'p>, fsl_eval::execution::PrepareFailure> {
            Ok(KernelMutationPrepared {
                inner: Interpreter.prepare(package, entries, limits, cancel)?,
                mutation: self.0,
            })
        }
    }

    impl PreparedEntries for KernelMutationPrepared<'_> {
        fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
            self.inner.unsupported()
        }

        fn call(
            &self,
            entry: &QualifiedName,
            arguments: &[Value],
            environment: &ObjectEnvironment,
            limits: &CallLimits,
            run: &mut SuppliedRun,
            cancel: &Cancel,
        ) -> Result<Evaluation, CallFailure> {
            let mut evaluation =
                self.inner
                    .call(entry, arguments, environment, limits, run, cancel)?;
            match self.mutation {
                KernelMutation::TypedCause => {
                    if matches!(
                        evaluation.outcome,
                        FamilyOutcome::Evaluated(Outcome::Undefined(_))
                    ) {
                        evaluation.outcome = FamilyOutcome::Evaluated(Outcome::Undefined(
                            filament_exact::Undefined::IeeeNotFinite,
                        ));
                    }
                }
                KernelMutation::Location => evaluation.location = None,
            }
            Ok(evaluation)
        }

        fn evaluate(
            &self,
            clause: &QualifiedName,
            input: &AdmittedObservations,
            accounting: &ScalarLimits,
            cancel: &Cancel,
        ) -> Result<Evaluation, CallFailure> {
            self.inner.evaluate(clause, input, accounting, cancel)
        }
    }

    /// TC-1386 FR-563-AC-2: a Kernel-checked Rational zero divisor reaches
    /// the real interpreter as Undefined; backend mutations are reported as
    /// typed-cause and location differences on the parity comparison path.
    #[trace("TC-1386", "FR-563-AC-2")]
    #[test]
    fn kernel_undefined_differences_are_compared_against_the_interpreter() {
        let source = b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction missing using v(a: Rational[-9, 9; 1, 9], b: Rational[-9, 9; 1, 9]): Rational[-10, 10; 1, 10] pure { a / b }\n";
        let compiled = compile_sources_with_mode(
            &[("undefined.native".to_owned(), source.to_vec())],
            &[],
            Some(CheckMode::Kernel),
            &Cancel::new(),
        )
        .expect("Kernel-mode Rational divisor source checks");
        let arguments = vec![
            Value::Rational(
                filament_exact::Rational::new(Integer::one(), Integer::one())
                    .expect("one is a rational"),
            ),
            Value::Rational(
                filament_exact::Rational::new(Integer::zero(), Integer::one())
                    .expect("zero is a rational"),
            ),
        ];
        let unit = ParityUnit {
            id: "kernel-rational-zero-divisor".to_owned(),
            source: source.to_vec(),
            source_files: vec![SourceFile {
                path: "undefined.native".to_owned(),
                bytes: source.to_vec(),
            }],
            package: UnitPackage::Checked(compiled.checked),
            environment: ObjectEnvironment::default(),
            functions: vec![ParityFunction {
                name: "missing".to_owned(),
                arguments: vec![arguments.clone()],
            }],
            clauses: Vec::new(),
        };

        for (mutation, expected_member) in [
            (KernelMutation::TypedCause, ParityMember::TypedCause),
            (KernelMutation::Location, ParityMember::Location),
        ] {
            let backend = KernelMutationBackend(mutation);
            let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &backend, None)];
            let mut reports = [BackendReport {
                backend: ExecutionBackendChoice::Jit,
                compared: 0,
                unsupported: Vec::new(),
                findings: Vec::new(),
                total_findings: 0,
            }];
            compare_unit(
                &Interpreter,
                &backends,
                &unit,
                DEFAULT_PARITY_SEED,
                &ParityLimits::default(),
                &Cancel::new(),
                &mut reports,
            )
            .expect("parity comparison completes");
            assert_eq!(reports[0].compared, 1);
            assert_eq!(reports[0].findings.len(), 1);
            let finding = &reports[0].findings[0];
            assert_eq!(finding.kind, FindingKind::Difference(expected_member));
            assert!(matches!(
                finding.arguments.as_slice(),
                [Value::Rational(numerator), Value::Rational(denominator)]
                    if numerator.numerator() == &Integer::one()
                        && numerator.denominator() == &Integer::one()
                        && denominator.numerator() == &Integer::zero()
                        && denominator.denominator() == &Integer::one()
            ));
            assert_eq!(finding.entry, "missing");
            assert_eq!(finding.unit, "kernel-rational-zero-divisor");
        }
    }

    /// TC-779 FR-295-AC-5: the seeded generator is deterministic, has bounded
    /// unit/function shape, reaches each required form/type, and includes
    /// both bounds in every generated Int parameter's tuple set.
    #[trace("TC-779", "FR-295-AC-5")]
    #[test]
    fn ten_thousand_units_cover_the_generator_contract() {
        for index in 0..100 {
            let (source, functions) = ProgramGenerator::new(DEFAULT_PARITY_SEED).unit(index);
            let checked =
                compile_unit(&source, &format!("generator-probe-{index}"), &Cancel::new())
                    .unwrap_or_else(|_| {
                        panic!(
                            "generated unit {index} must be admitted: {}",
                            String::from_utf8_lossy(&source)
                        )
                    });
            let graph = checked.package().graph();
            for function in functions {
                let callable = graph
                    .callable(&function.name)
                    .expect("generated signature is checked");
                let checked_function = graph
                    .function_by_identity(callable.identity.clone())
                    .expect("generated body is checked");
                let mut pending = vec![checked_function.body];
                let mut nodes = 0;
                while let Some(node) = pending.pop() {
                    nodes += 1;
                    pending.extend(node.children());
                }
                assert!(
                    nodes <= 64,
                    "generated body {} has {nodes} expression nodes",
                    function.name
                );
            }
        }
        let mut generator = ProgramGenerator::new(DEFAULT_PARITY_SEED);
        let mut repeated = ProgramGenerator::new(DEFAULT_PARITY_SEED);
        let mut forms = [0_u64; 8];
        let mut types = [0_u64; 7];
        for index in 0..10_000 {
            let (source, functions) = generator.unit(index);
            let (same_source, same_functions) = repeated.unit(index);
            assert_eq!(source, same_source, "unit {index} source reproducibility");
            assert_eq!(
                format!(
                    "{:?}",
                    functions.iter().map(|f| &f.arguments).collect::<Vec<_>>()
                ),
                format!(
                    "{:?}",
                    same_functions
                        .iter()
                        .map(|f| &f.arguments)
                        .collect::<Vec<_>>()
                ),
                "unit {index} tuple reproducibility"
            );
            let source = String::from_utf8(source).expect("generator emits UTF-8");
            assert!((1..=8).contains(&functions.len()));
            for function in &functions {
                assert_eq!(
                    function.arguments.len(),
                    4,
                    "four tuples for {}",
                    function.name
                );
                let arity = function.arguments[0].len();
                assert!(arity <= 4, "at most four parameters for {}", function.name);
                assert!(function.arguments.iter().all(|tuple| tuple.len() == arity));
            }
            assert!(source.matches("function ").count() <= 8);
            for (index, function) in functions.iter().enumerate() {
                let line = source
                    .lines()
                    .find(|line| line.starts_with(&format!("function {} ", function.name)))
                    .expect("every generated function has a declaration");
                for later in &functions[index + 1..] {
                    assert!(
                        !line.contains(&format!("{}(", later.name)),
                        "{} must not call later function {}",
                        function.name,
                        later.name
                    );
                }
            }
            assert!(source.lines().all(|line| line.len() <= 1024));
            if source.contains("{ 1 }") || source.contains("{ 1 ") {
                forms[0] += 1;
            }
            if source.contains(" + ") || source.contains(" / ") {
                forms[1] += 1;
            }
            if source.contains("let ") {
                forms[2] += 1;
            }
            if source.contains("if ") {
                forms[3] += 1;
            }
            if source.contains("() +") {
                forms[4] += 1;
            }
            if source.contains("Pair{") {
                forms[5] += 1;
            }
            if source.contains("sequence[") || source.contains("seq(") {
                forms[6] += 1;
            }
            if source.contains("function ") {
                forms[7] += 1;
            }
            let signatures = source
                .lines()
                .filter(|line| line.starts_with("function "))
                .collect::<Vec<_>>();
            if signatures.iter().any(|line| line.contains("Boolean")) {
                types[0] += 1;
            }
            if signatures.iter().any(|line| line.contains("Int[")) {
                types[1] += 1;
            }
            if signatures.iter().any(|line| line.contains("Rational[")) {
                types[2] += 1;
            }
            if signatures.iter().any(|line| line.contains("Decimal[")) {
                types[3] += 1;
            }
            if signatures.iter().any(|line| line.contains("Text[")) {
                types[4] += 1;
            }
            if signatures.iter().any(|line| line.contains(": Pair ")) {
                types[5] += 1;
            }
            if signatures.iter().any(|line| line.contains("Sequence<")) {
                types[6] += 1;
            }
            for function in &functions {
                for (position, first) in
                    function.arguments.first().into_iter().flatten().enumerate()
                {
                    if let Value::Integer(first) = first {
                        let Some(Value::Integer(last)) = function
                            .arguments
                            .get(1)
                            .and_then(|tuple| tuple.get(position))
                        else {
                            continue;
                        };
                        assert!(matches!(first.to_string().as_str(), "-9" | "0" | "1"));
                        assert!(matches!(last.to_string().as_str(), "1" | "9"));
                    }
                }
            }
        }
        assert!(
            forms.into_iter().all(|count| count >= 100),
            "forms: {forms:?}"
        );
        assert!(
            types.into_iter().all(|count| count >= 100),
            "types: {types:?}"
        );

        let mut other_seed = ProgramGenerator::new(DEFAULT_PARITY_SEED.wrapping_add(1));
        let mut default_seed = ProgramGenerator::new(DEFAULT_PARITY_SEED);
        assert_ne!(other_seed.unit(0).0, default_seed.unit(0).0);
    }
}
