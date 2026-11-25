// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: exercise the corpus adapter and default seeded generator through
//! the public parity operation.

use std::collections::BTreeSet;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use filament_eval::{CallFailure, Evaluation, ExecutionBackendChoice};
use filament_exact::{Cancel, Integer, Outcome, ScalarLimits, Value};
use filament_semantic_value::supplied_stop::SuppliedRun;
use fsl_conformance::{
    run_parity, BackendEntry, FindingKind, ParityInputs, ParityLimits, ParityMember, ParityRefusal,
    ParityReport, UnbuildableCause,
};
use fsl_eval::execution::{
    BackendCoverage, CallLimits, CoverageOperation, CoverageOperator, CoverageValueType,
    EntrySelection, ExecutionBackend, Interpreter, InterpreterPrepared, PrepareFailure,
    PrepareLimits, PreparedEntries,
};
use fsl_eval::value::QualifiedName;
use fsl_package::CheckedPackage;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::AdmittedObservations;
use ix_trace_rs::trace;

fn empty_corpus() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("complete-v1.json"),
        "{\"vectors\":[]}",
    )
    .expect("empty corpus file");
    directory
}

fn inc_corpus() -> tempfile::TempDir {
    inc_corpus_with_count(1)
}

fn inc_corpus_with_count(count: usize) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n",
    )
    .expect("source input");
    fs::write(
        directory.path().join("call.json"),
        r#"{"function":"inc","arguments":[{"kind":"integer","decimal":"4"}],"accounting":{}}"#,
    )
    .expect("call input");
    let vector = |id: &str, operation: &str| {
        let mut files = vec![serde_json::json!({"kind":"source", "path":"unit.native"})];
        if operation == "run" {
            files.push(serde_json::json!({"kind":"call", "path":"call.json"}));
        }
        serde_json::json!({
            "id":id,
            "applicability":{"consumer_types":["runtime"]},
            "subject":{"consumer_type":"runtime", "operation":operation},
            "input":{"files":files}
        })
    };
    let mut vectors = (0..count)
        .map(|index| vector(&format!("run-inc-{index}"), "run"))
        .collect::<Vec<_>>();
    if count == 1 {
        vectors.push(vector("compile-inc", "compile"));
        vectors.push(vector("format-inc", "format"));
    }
    let corpus = serde_json::json!({"vectors":vectors});
    fs::write(
        directory.path().join("complete-v1.json"),
        serde_json::to_vec(&corpus).expect("corpus JSON"),
    )
    .expect("corpus file");
    directory
}

fn let_corpus() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction usesLet using v(x: Int[0, 9]): Int[0, 10] pure { let y = x in y + 1 }\nfunction simple using v(x: Int[0, 9]): Int[0, 9] pure { x }\n",
    )
    .expect("source input");
    for name in ["usesLet", "simple"] {
        fs::write(
            directory.path().join(format!("{name}.json")),
            format!(
                r#"{{"function":"{name}","arguments":[{{"kind":"integer","decimal":"4"}}],"accounting":{{}}}}"#
            ),
        )
        .expect("call input");
    }
    let vector = |id: &str, function: &str| {
        serde_json::json!({
            "id":id,
            "applicability":{"consumer_types":["runtime"]},
            "subject":{"consumer_type":"runtime", "operation":"run"},
            "input":{"files":[
                {"kind":"source", "path":"unit.native"},
                {"kind":"call", "path":format!("{function}.json")}
            ]}
        })
    };
    let corpus = serde_json::json!({
        "vectors":[vector("let-vector", "usesLet"), vector("simple-vector", "simple")]
    });
    fs::write(
        directory.path().join("complete-v1.json"),
        serde_json::to_vec(&corpus).expect("corpus JSON"),
    )
    .expect("corpus file");
    directory
}

fn two_source_corpus() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("base.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n",
    )
    .expect("base source");
    fs::write(
        directory.path().join("consumer.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction twice using v(x: Int[0, 9]): Int[0, 20] pure { inc(x) + inc(x) }\n",
    )
    .expect("consumer source");
    fs::write(
        directory.path().join("call.json"),
        r#"{"function":"twice","arguments":[{"kind":"integer","decimal":"4"}],"accounting":{}}"#,
    )
    .expect("call input");
    let corpus = serde_json::json!({"vectors":[{
        "id":"two-source-run",
        "applicability":{"consumer_types":["runtime"]},
        "subject":{"consumer_type":"runtime", "operation":"run"},
        "input":{"files":[
            {"kind":"source", "path":"base.native"},
            {"kind":"source", "path":"consumer.native"},
            {"kind":"call", "path":"call.json"}
        ]}
    }]});
    fs::write(
        directory.path().join("complete-v1.json"),
        serde_json::to_vec(&corpus).expect("corpus JSON"),
    )
    .expect("corpus file");
    directory
}

fn invalid_argument_corpus() -> tempfile::TempDir {
    let directory = inc_corpus();
    fs::write(
        directory.path().join("call.json"),
        r#"{"function":"inc","arguments":[{"kind":"boolean","value":true}],"accounting":{}}"#,
    )
    .expect("invalid argument input");
    directory
}

#[derive(Clone, Copy)]
struct WrongValueBackend;

struct WrongValuePrepared<'p>(InterpreterPrepared<'p>);

impl ExecutionBackend for WrongValueBackend {
    type Prepared<'p> = WrongValuePrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Interpreter
            .prepare(package, entries, limits, cancel)
            .map(WrongValuePrepared)
    }
}

impl PreparedEntries for WrongValuePrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
        self.0.unsupported()
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
        let mut evaluation = self
            .0
            .call(entry, arguments, environment, limits, run, cancel)?;
        if matches!(
            &evaluation.outcome,
            filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(_)))
        ) {
            evaluation.outcome = filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(
                Value::Integer(Integer::from(6_i64)),
            ));
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
        self.0.evaluate(clause, input, accounting, cancel)
    }
}

#[derive(Clone, Copy)]
struct AlternatingBackend;

struct AlternatingPrepared<'p> {
    inner: InterpreterPrepared<'p>,
    calls: AtomicUsize,
}

impl ExecutionBackend for AlternatingBackend {
    type Prepared<'p> = AlternatingPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Ok(AlternatingPrepared {
            inner: Interpreter.prepare(package, entries, limits, cancel)?,
            calls: AtomicUsize::new(0),
        })
    }
}

impl PreparedEntries for AlternatingPrepared<'_> {
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
        let mut result = self
            .inner
            .call(entry, arguments, environment, limits, run, cancel)?;
        if matches!(
            &result.outcome,
            filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(_)))
        ) {
            let value = if self.calls.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
                5_i64
            } else {
                6_i64
            };
            result.outcome = filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(
                Value::Integer(Integer::from(value)),
            ));
        }
        Ok(result)
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

#[derive(Clone, Copy)]
struct PrepareLimitBackend;

impl ExecutionBackend for PrepareLimitBackend {
    type Prepared<'p> = InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        _package: &'p CheckedPackage,
        _entries: &[EntrySelection],
        _limits: &PrepareLimits,
        _cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Err(PrepareFailure::Limit(
            fsl_foundation::diagnostic::LimitExceeded::new(
                fsl_foundation::Setting::PrepareCompileWorkUnits,
                0,
                1,
            ),
        ))
    }
}

#[derive(Clone, Copy)]
struct UnsupportedBackend;

struct UnsupportedPrepared<'p> {
    inner: InterpreterPrepared<'p>,
    unsupported: Vec<(EntrySelection, filament_eval::UnsupportedEntry)>,
}

#[derive(Clone, Copy)]
struct LetUnsupportedBackend;

struct LetUnsupportedPrepared<'p> {
    inner: InterpreterPrepared<'p>,
    unsupported: Vec<(EntrySelection, filament_eval::UnsupportedEntry)>,
}

impl ExecutionBackend for LetUnsupportedBackend {
    type Prepared<'p> = LetUnsupportedPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        let without_let = BackendCoverage {
            expression_forms: ["literal", "name", "operator"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            value_types: [CoverageValueType::Integer].into_iter().collect(),
            operations: BTreeSet::from([CoverageOperation {
                operator: CoverageOperator::Add,
                operand_types: vec![CoverageValueType::Integer, CoverageValueType::Integer],
                result_type: CoverageValueType::Integer,
            }]),
        };
        let unsupported = entries
            .iter()
            .filter(|selection| selection.name.to_string() == "usesLet")
            .cloned()
            .map(|entry| {
                let outside = fsl_eval::execution::coverage::first_outside(
                    package,
                    &entry.name.to_string(),
                    &without_let,
                )
                .expect("coverage scan succeeds")
                .expect("usesLet begins with the omitted form");
                (
                    entry,
                    filament_eval::UnsupportedEntry {
                        backend: ExecutionBackendChoice::Jit,
                        cause: filament_eval::UnsupportedCause::Untranslatable {
                            construct: outside.construct,
                            location: outside.location,
                        },
                    },
                )
            })
            .collect();
        Ok(LetUnsupportedPrepared { inner, unsupported })
    }
}

impl PreparedEntries for LetUnsupportedPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
        &self.unsupported
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
        self.inner
            .call(entry, arguments, environment, limits, run, cancel)
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

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Fault,
    FaultOnNine,
    OutcomeKind,
    CompletedValue,
    Losses,
    IncompleteChargePoint,
    CallFailurePayload,
}

#[derive(Clone, Copy)]
struct MutationBackend(Mutation);

struct MutationPrepared<'p> {
    inner: InterpreterPrepared<'p>,
    mutation: Mutation,
}

impl ExecutionBackend for MutationBackend {
    type Prepared<'p> = MutationPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Ok(MutationPrepared {
            inner: Interpreter.prepare(package, entries, limits, cancel)?,
            mutation: self.0,
        })
    }
}

impl PreparedEntries for MutationPrepared<'_> {
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
        use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
        if matches!(self.mutation, Mutation::Fault)
            || matches!(self.mutation, Mutation::FaultOnNine)
                && arguments
                    .iter()
                    .any(|value| matches!(value, Value::Integer(integer) if integer == &Integer::from(9_i64)))
        {
            return Err(CallFailure::Fault(
                filament_semantic_value::catalog::InternalFault::new(
                    "parity-test",
                    "injected backend fault",
                ),
            ));
        }
        let mut adjusted_limits = *limits;
        if matches!(self.mutation, Mutation::IncompleteChargePoint) {
            adjusted_limits.accounting.work_units =
                adjusted_limits.accounting.work_units.saturating_add(1);
        }
        let mut evaluation =
            match self
                .inner
                .call(entry, arguments, environment, &adjusted_limits, run, cancel)
            {
                Err(_) if matches!(self.mutation, Mutation::CallFailurePayload) => {
                    return Err(CallFailure::Fault(
                        filament_semantic_value::catalog::InternalFault::new(
                            "parity-test",
                            "different call failure payload",
                        ),
                    ));
                }
                result => result?,
            };
        match self.mutation {
            Mutation::Fault
            | Mutation::FaultOnNine
            | Mutation::IncompleteChargePoint
            | Mutation::CallFailurePayload => {}
            Mutation::OutcomeKind => {
                evaluation.outcome = filament_eval::FamilyOutcome::Evaluated(Outcome::Undefined(
                    filament_exact::Undefined::DivisionByZero,
                ));
            }
            Mutation::CompletedValue => {
                if matches!(
                    evaluation.outcome,
                    filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(_)))
                ) {
                    evaluation.outcome = filament_eval::FamilyOutcome::Evaluated(
                        Outcome::Completed(Value::Integer(Integer::from(6_i64))),
                    );
                }
            }
            Mutation::Losses => {
                evaluation
                    .losses
                    .push(filament_semantic_value::loss::LocatedLoss {
                        location: ExpressionLocation::at(ExpressionOwner::Expression, &[3, 2]),
                        loss: filament_semantic_value::loss::ValueLoss::IeeeFlags(
                            [filament_exact::IeeeFlag::Inexact].into_iter().collect(),
                        ),
                    });
            }
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

impl ExecutionBackend for UnsupportedBackend {
    type Prepared<'p> = UnsupportedPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        let unsupported = entries
            .iter()
            .cloned()
            .map(|entry| {
                (
                    entry,
                    filament_eval::UnsupportedEntry {
                        backend: ExecutionBackendChoice::Jit,
                        cause: filament_eval::UnsupportedCause::NotSupplied,
                    },
                )
            })
            .collect();
        Ok(UnsupportedPrepared { inner, unsupported })
    }
}

impl PreparedEntries for UnsupportedPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
        &self.unsupported
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
        self.inner
            .call(entry, arguments, environment, limits, run, cancel)
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

/// Lists `entry` unsupported, naming `construct` whatever the entry holds.
#[derive(Clone, Copy)]
struct NamingBackend {
    entry: &'static str,
    construct: &'static str,
}

impl ExecutionBackend for NamingBackend {
    type Prepared<'p> = UnsupportedPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        let unsupported = entries
            .iter()
            .filter(|selection| selection.name.to_string() == self.entry)
            .cloned()
            .map(|entry| {
                (
                    entry,
                    filament_eval::UnsupportedEntry {
                        backend: ExecutionBackendChoice::Jit,
                        cause: filament_eval::UnsupportedCause::Untranslatable {
                            construct: filament_eval::UntranslatableConstruct::ExpressionForm(
                                self.construct.into(),
                            ),
                            location: ExpressionLocation::at(ExpressionOwner::Expression, &[0]),
                        },
                    },
                )
            })
            .collect();
        Ok(UnsupportedPrepared { inner, unsupported })
    }
}

/// The interpreter with no work bound on a call, whatever the runner supplies.
#[derive(Clone, Copy)]
struct UnmeteredCallBackend;

struct UnmeteredCallPrepared<'p>(InterpreterPrepared<'p>);

impl ExecutionBackend for UnmeteredCallBackend {
    type Prepared<'p> = UnmeteredCallPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Interpreter
            .prepare(package, entries, limits, cancel)
            .map(UnmeteredCallPrepared)
    }
}

impl PreparedEntries for UnmeteredCallPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
        self.0.unsupported()
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
        let mut unmetered = *limits;
        unmetered.accounting.work_units = u64::MAX;
        self.0
            .call(entry, arguments, environment, &unmetered, run, cancel)
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        self.0.evaluate(clause, input, accounting, cancel)
    }
}

/// The interpreter, counting the units it is asked to prepare.
#[derive(Default)]
struct CountingBackend {
    prepares: AtomicUsize,
}

impl ExecutionBackend for CountingBackend {
    type Prepared<'p> = InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        self.prepares.fetch_add(1, Ordering::Relaxed);
        Interpreter.prepare(package, entries, limits, cancel)
    }
}

/// The corpus at `path` alone, with no generated unit.
fn corpus_only(path: &std::path::Path) -> ParityInputs {
    ParityInputs {
        count: 0,
        ..ParityInputs::new(path)
    }
}

/// Runs a generated decimal division unit and counts only admitted units.
#[test]
fn generated_decimal_probe_is_admitted_and_compared() {
    let corpus = empty_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 1,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the generated unit is admitted");
    assert_eq!(report.units, 1);
    assert!(report.backends[0].compared > 0);
}

/// TC-1386 FR-295-AC-5: the default seed admits generated cases,
/// and a fixed seed reproduces their source and argument tuples.
#[trace("TC-1386", "FR-295-AC-5")]
#[test]
fn generated_units_are_reproducible_for_the_default_seed() {
    let corpus = empty_corpus();
    let wrong = WrongValueBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &wrong, None)];
    let inputs = ParityInputs {
        corpus: corpus.path().to_path_buf(),
        seed: fsl_conformance::DEFAULT_PARITY_SEED,
        count: 1,
        #[cfg(feature = "test-support")]
        clauses: Vec::new(),
    };
    let first = run_parity(&backends, &inputs, &ParityLimits::default(), &Cancel::new())
        .expect("generated source is admitted");
    let second = run_parity(&backends, &inputs, &ParityLimits::default(), &Cancel::new())
        .expect("same generated source is admitted again");
    assert_eq!(first.seed, 0x5153_4C2D_4A49_5431);
    assert_eq!(first.units, 1);
    assert!(first.backends[0].total_findings > 0);
    let first = first.backends[0]
        .findings
        .iter()
        .find(|finding| finding.unit == "0")
        .expect("generated finding carries its unit");
    let second = second.backends[0]
        .findings
        .iter()
        .find(|finding| finding.unit == "0")
        .expect("repeated generated finding carries its unit");
    assert_eq!(first.source, second.source);
    assert_eq!(first.source_files, second.source_files);
    assert_eq!(
        format!("{:?}", first.arguments),
        format!("{:?}", second.arguments)
    );
    assert!(first.reference.is_some());
    assert!(first.result.is_some());
}

/// TC-1386 FR-563-AC-1: an interpreter-delegating backend passes every
/// default generated unit with at least one comparison.
#[trace("TC-1386", "FR-563-AC-1")]
#[test]
fn interpreter_backend_passes_default_generated_scope() {
    let corpus = empty_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
    let report = run_parity(
        &backends,
        &ParityInputs::new(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the default generated scope is admitted");
    assert_eq!(report.units, 10_000);
    assert_eq!(report.discarded, 1_263, "seeded rejected draws are counted");
    assert!(report.backends[0].compared > 0);
    assert_eq!(report.backends[0].total_findings, 0);
    assert_eq!(report.exit_code(), 0);
}

/// TC-1386 FR-563-AC-1: an interpreter-delegating backend passes a selected
/// corpus vector, which is compared and not skipped.
#[trace("TC-1386", "FR-563-AC-1")]
#[test]
fn interpreter_backend_passes_a_selected_corpus_vector() {
    let corpus = inc_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
    let report = run_parity(
        &backends,
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the corpus vector is admitted");
    assert_eq!(report.units, 1);
    assert_eq!(report.backends[0].compared, 1);
    assert!(report.backends[0].unsupported.is_empty());
    assert_eq!(report.backends[0].total_findings, 0);
    assert_eq!(report.exit_code(), 0);
}

/// TC-1387 FR-563-AC-13: a generator whose draws checking keeps refusing stops
/// at the discard bound and fails every backend by name; a run that stays within
/// the bound does not.
#[trace("TC-1387", "FR-563-AC-13")]
#[test]
fn refused_draws_beyond_the_bound_fail_the_gate() {
    assert_eq!(ParityLimits::default().max_discarded, 10_000);
    let corpus = empty_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 2] = [
        (ExecutionBackendChoice::Jit, &interpreter, None),
        (ExecutionBackendChoice::Jit, &interpreter, None),
    ];
    let inputs = ParityInputs {
        count: 100,
        ..ParityInputs::new(corpus.path())
    };
    let run = |max_discarded| {
        let limits = ParityLimits {
            max_discarded,
            ..ParityLimits::default()
        };
        run_parity(&backends, &inputs, &limits, &Cancel::new())
            .expect("the discard bound is a report, not a refusal")
    };

    let unbounded = run(u64::MAX);
    let refused = unbounded.discarded;
    assert!(refused > 0, "the control needs a refused draw");
    let within = run(refused);
    assert_eq!(within.units, 100);
    assert!(within
        .backends
        .iter()
        .all(|backend| backend.total_findings == 0));
    assert_eq!(within.exit_code(), 0);

    let exhausted = run(refused - 1);
    assert!(exhausted.units < 100);
    assert_eq!(exhausted.discarded, refused);
    // Draws are indexed from zero: those before the last refused one were the
    // admitted units and the other refused draws.
    let index = exhausted.units + refused - 1;
    let mut sources = Vec::new();
    for backend in &exhausted.backends {
        assert_eq!(backend.total_findings, 1);
        let finding = &backend.findings[0];
        assert_eq!(finding.kind, FindingKind::GeneratorExhausted);
        assert_eq!(finding.unit, index.to_string());
        assert!(
            String::from_utf8_lossy(&finding.source)
                .contains(&format!("function parity_{index}_base")),
            "the finding carries the source of draw {index}"
        );
        assert!(finding.entry.is_empty() && finding.arguments.is_empty());
        assert!(finding.reference.is_none() && finding.result.is_none());
        sources.push(&finding.source);
    }
    assert_eq!(sources[0], sources[1]);
    assert_eq!(exhausted.exit_code(), 1);
}

/// TC-1386 FR-563-AC-9: only the function `runtime`/`run` vector is selected;
/// an incorrect result carries its vector id and decoded argument tuple.
#[trace("TC-1386", "FR-563-AC-9")]
#[test]
fn run_vector_is_selected_with_its_single_decoded_tuple() {
    let corpus = inc_corpus();
    let wrong = WrongValueBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &wrong, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the corpus vector is admitted");
    assert_eq!(report.units, 1);
    assert_eq!(report.not_selected, 2);
    assert_eq!(report.backends[0].compared, 1);
    assert_eq!(report.exit_code(), 1);
    let finding = report.backends[0]
        .findings
        .first()
        .expect("difference finding");
    assert_eq!(
        finding.kind,
        FindingKind::Difference(ParityMember::CompletedValue)
    );
    assert_eq!(finding.unit, "run-inc-0");
    assert_eq!(finding.entry, "inc");
    assert_eq!(format!("{:?}", finding.arguments), "[Integer(Integer(4))]");
}

#[test]
fn multiple_source_files_are_checked_as_distinct_units_and_retained() {
    let corpus = two_source_corpus();
    let wrong = WrongValueBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &wrong, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("all source units are composed");
    assert_eq!(report.backends[0].compared, 1);
    assert_eq!(report.backends[0].total_findings, 1);
    assert_eq!(report.units, 1);
    let finding = &report.backends[0].findings[0];
    assert_eq!(finding.source_files.len(), 2);
    assert_eq!(finding.source_files[0].path, "base.native");
    assert_eq!(finding.source_files[1].path, "consumer.native");
    assert!(String::from_utf8_lossy(&finding.source_files[0].bytes).contains("function inc"));
    assert!(String::from_utf8_lossy(&finding.source_files[1].bytes).contains("function twice"));
}

/// TC-1386 FR-563-AC-3: repeated calls with identical inputs must agree.
#[trace("TC-1386", "FR-563-AC-3")]
#[test]
fn alternating_backend_is_reported_nondeterministic() {
    let corpus = inc_corpus();
    let alternating = AlternatingBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &alternating, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("alternation is a reported finding");
    let finding = report.backends[0]
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::Nondeterministic)
        .expect("nondeterministic call finding");
    assert_eq!(finding.entry, "inc");
    assert_eq!(finding.arguments.len(), 1);
    assert_eq!(report.exit_code(), 1);
}

/// TC-1386 FR-563-AC-2: every specified comparison member maps to its own
/// difference finding and retains reproducible case evidence.
#[trace("TC-1386", "FR-563-AC-2")]
#[test]
fn comparison_member_differences_are_named_and_reproducible() {
    let invalid = invalid_argument_corpus();
    let cases = [
        (Mutation::Fault, ParityMember::OkAgainstErr, inc_corpus(), 0),
        (
            Mutation::OutcomeKind,
            ParityMember::OutcomeKind,
            inc_corpus(),
            0,
        ),
        (
            Mutation::CompletedValue,
            ParityMember::CompletedValue,
            inc_corpus(),
            0,
        ),
        (Mutation::Losses, ParityMember::Losses, inc_corpus(), 0),
        (
            Mutation::IncompleteChargePoint,
            ParityMember::IncompleteChargePoint,
            inc_corpus(),
            0,
        ),
        (
            Mutation::CallFailurePayload,
            ParityMember::CallFailurePayload,
            invalid,
            0,
        ),
    ];
    for (mutation, member, corpus, count) in cases {
        let backend = MutationBackend(mutation);
        let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &backend, None)];
        let mut limits = ParityLimits::default();
        if matches!(mutation, Mutation::IncompleteChargePoint) {
            limits.accounting.work_units = 0;
        }
        let inputs = ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        };
        let cancel = Cancel::new();
        let report = run_parity(&backends, &inputs, &limits, &cancel)
            .unwrap_or_else(|error| panic!("mutation {mutation:?}: {error:?}"));
        let findings = &report.backends[0].findings;
        assert_eq!(findings.len(), 1, "member {member:?}: {findings:#?}");
        let finding = &findings[0];
        assert_eq!(finding.kind, FindingKind::Difference(member));
        assert_eq!(finding.seed, fsl_conformance::DEFAULT_PARITY_SEED);
        assert_eq!(finding.unit, "run-inc-0");
        assert!(!finding.source.is_empty());
        assert_eq!(finding.entry, "inc");
        let arguments_match = if matches!(mutation, Mutation::CallFailurePayload) {
            matches!(finding.arguments.as_slice(), [Value::Boolean(true)])
        } else {
            matches!(
                finding.arguments.as_slice(),
                [Value::Integer(value)] if value == &Integer::from(4_i64)
            )
        };
        assert!(arguments_match, "arguments: {:?}", finding.arguments);
        assert!(finding.reference.is_some());
        assert!(finding.result.is_some());
    }
}

/// TC-1387 FR-563-AC-5: a prepare limit is a finding; an all-unsupported
/// backend produces an empty-comparison finding.
#[trace("TC-1387", "FR-563-AC-5")]
#[test]
fn prepare_failure_and_all_unsupported_are_reported() {
    let corpus = empty_corpus();
    let failed = PrepareLimitBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &failed, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 1,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("prepare limit is a finding");
    let prepare_finding = report.backends[0]
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::PrepareFailed && finding.unit == "0")
        .expect("prepare failure is retained");
    assert!(matches!(
        prepare_finding.prepare_failure,
        Some(PrepareFailure::Limit(ref limit))
            if limit.setting() == fsl_foundation::Setting::PrepareCompileWorkUnits
    ));
    assert!(!prepare_finding.source.is_empty());
    assert!(!prepare_finding.entry.is_empty());

    let corpus = inc_corpus();
    let unsupported = UnsupportedBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &unsupported, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("unsupported entries are a report, not refusal");
    assert_eq!(report.backends[0].compared, 0);
    assert!(!report.backends[0].unsupported.is_empty());
    assert!(report.backends[0]
        .findings
        .iter()
        .any(|finding| finding.kind == FindingKind::EmptyComparison));
}

/// A coverage every construct of `let_corpus`'s entries lies in: `usesLet`
/// narrows its sum to the declared result, so the conversion is listed too.
fn let_coverage() -> BackendCoverage {
    BackendCoverage {
        expression_forms: ["literal", "name", "operator", "let"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        value_types: [CoverageValueType::Integer].into_iter().collect(),
        operations: BTreeSet::from([
            CoverageOperation {
                operator: CoverageOperator::Add,
                operand_types: vec![CoverageValueType::Integer, CoverageValueType::Integer],
                result_type: CoverageValueType::Integer,
            },
            CoverageOperation {
                operator: CoverageOperator::Other("coerce".to_owned()),
                operand_types: vec![CoverageValueType::Integer],
                result_type: CoverageValueType::Integer,
            },
        ]),
    }
}

/// TC-1387 FR-563-AC-4: an unsupported `let` entry is a coverage gap only
/// when the backend declares `let` in its coverage; other entries still run.
#[trace("TC-1387", "FR-563-AC-4")]
#[test]
fn coverage_gap_depends_on_declared_forms() {
    let corpus = let_corpus();
    let unsupported = LetUnsupportedBackend;
    let mut coverage = let_coverage();
    let backends: [BackendEntry<'_>; 1] =
        [(ExecutionBackendChoice::Jit, &unsupported, Some(&coverage))];
    let gap = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("coverage gap is reported");
    assert!(
        gap.backends[0]
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::CoverageGap && finding.entry == "usesLet"),
        "findings: {:#?}",
        (&gap.backends[0].findings, &gap.backends[0].unsupported)
    );
    assert_eq!(gap.backends[0].compared, 1);

    coverage.expression_forms.remove("let");
    let backends: [BackendEntry<'_>; 1] =
        [(ExecutionBackendChoice::Jit, &unsupported, Some(&coverage))];
    let no_gap = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("outside coverage unsupported entry is retained");
    assert_eq!(no_gap.exit_code(), 0);
    assert_eq!(no_gap.backends[0].compared, 1);
    assert_eq!(no_gap.backends[0].unsupported.len(), 1);
    assert_eq!(no_gap.backends[0].unsupported[0].entry, "usesLet");
}

/// TC-1387 FR-563-AC-4: the gap is decided on all of the entry's constructs,
/// not on the one the backend names.
#[trace("TC-1387", "FR-563-AC-4")]
#[test]
fn coverage_gap_ignores_which_construct_the_backend_names() {
    let corpus = let_corpus();
    let mut coverage = let_coverage();
    let gaps = |backend: NamingBackend, coverage: &BackendCoverage| {
        let backends: [BackendEntry<'_>; 1] =
            [(ExecutionBackendChoice::Jit, &backend, Some(coverage))];
        let report = run_parity(
            &backends,
            &corpus_only(corpus.path()),
            &ParityLimits::default(),
            &Cancel::new(),
        )
        .expect("an unsupported entry is a report");
        assert_eq!(report.backends[0].unsupported.len(), 1);
        report.backends[0]
            .findings
            .iter()
            .filter(|finding| finding.kind == FindingKind::CoverageGap)
            .map(|finding| finding.entry.clone())
            .collect::<Vec<_>>()
    };

    // A covered entry is a gap although the named form is not covered.
    let covered_entry = NamingBackend {
        entry: "simple",
        construct: "case",
    };
    assert_eq!(gaps(covered_entry, &coverage), ["simple"]);

    // An entry outside the coverage is no gap although the named form is
    // covered.
    coverage.expression_forms.remove("let");
    let uncovered_entry = NamingBackend {
        entry: "usesLet",
        construct: "operator",
    };
    assert!(gaps(uncovered_entry, &coverage).is_empty());
}

/// A reference whose first preparation fails, with a fault or with a reached
/// limit, and whose later ones are the interpreter's.
struct FirstPrepareFails {
    fault: bool,
    prepares: AtomicUsize,
}

impl FirstPrepareFails {
    fn new(fault: bool) -> Self {
        Self {
            fault,
            prepares: AtomicUsize::new(0),
        }
    }
}

impl ExecutionBackend for FirstPrepareFails {
    type Prepared<'p> = InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        if self.prepares.fetch_add(1, Ordering::Relaxed) > 0 {
            return Interpreter.prepare(package, entries, limits, cancel);
        }
        Err(if self.fault {
            PrepareFailure::Fault(filament_semantic_value::catalog::InternalFault::new(
                "parity-test",
                "injected prepare fault",
            ))
        } else {
            PrepareFailure::Limit(fsl_foundation::diagnostic::LimitExceeded::new(
                fsl_foundation::Setting::PrepareCompileWorkUnits,
                0,
                1,
            ))
        })
    }
}

/// TC-1387 FR-563-AC-14: a reference preparation limit or fault is a finding on
/// every backend for each entry of its unit, and the later units still run.
#[trace("TC-1387", "FR-563-AC-14")]
#[test]
fn reference_preparation_failures_are_findings() {
    let corpus = inc_corpus_with_count(2);
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 2] = [
        (ExecutionBackendChoice::Jit, &interpreter, None),
        (ExecutionBackendChoice::Jit, &interpreter, None),
    ];
    for fault in [false, true] {
        let report = fsl_conformance::run_parity_with_reference(
            &FirstPrepareFails::new(fault),
            &backends,
            &corpus_only(corpus.path()),
            &ParityLimits::default(),
            &Cancel::new(),
        )
        .expect("a reference preparation failure is a reported result");
        assert_eq!(report.units, 2);
        for backend in &report.backends {
            assert_eq!(backend.compared, 1, "the second vector is compared");
            assert_eq!(backend.total_findings, 1);
            let finding = &backend.findings[0];
            assert_eq!(
                (finding.unit.as_str(), finding.entry.as_str()),
                ("run-inc-0", "inc")
            );
            assert_eq!(format!("{:?}", finding.arguments), "[Integer(Integer(4))]");
            assert!(finding.result.is_none());
            match &finding.kind {
                FindingKind::ReferenceFault => assert!(fault),
                FindingKind::ReferencePrepareLimit(limit) => {
                    assert!(!fault);
                    assert!(finding.reference.is_none());
                    assert_eq!(
                        limit.setting(),
                        fsl_foundation::Setting::PrepareCompileWorkUnits
                    );
                }
                other => panic!("unexpected finding {other:?}"),
            }
        }
        assert_eq!(report.exit_code(), 1);
    }

    // A generated unit has several entries: one finding for each.
    let generated = empty_corpus();
    let report = fsl_conformance::run_parity_with_reference(
        &FirstPrepareFails::new(false),
        &backends,
        &ParityInputs {
            count: 1,
            ..ParityInputs::new(generated.path())
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a reference preparation limit is a reported result");
    for backend in &report.backends {
        let limited = backend
            .findings
            .iter()
            .filter(|finding| matches!(finding.kind, FindingKind::ReferencePrepareLimit(_)))
            .map(|finding| finding.entry.as_str())
            .collect::<Vec<_>>();
        assert!(backend
            .findings
            .iter()
            .filter(|finding| matches!(finding.kind, FindingKind::ReferencePrepareLimit(_)))
            .all(|finding| finding.reference.is_none() && finding.result.is_none()));
        let entries = limited.iter().collect::<BTreeSet<_>>();
        assert!(entries.len() >= 2, "{limited:?}");
        assert_eq!(entries.len(), limited.len());
        assert_eq!(backend.compared, 0);
        let typed = backend
            .findings
            .iter()
            .find(|finding| finding.entry == "parity_0_0")
            .expect("the typed entry");
        assert!(
            matches!(typed.arguments.as_slice(), [Value::Decimal(_)]),
            "the entry's first argument tuple: {:?}",
            typed.arguments
        );
    }
    assert_eq!(report.exit_code(), 1);
}

/// Comparing two completed values draws on no call budget, so a run with an
/// exhausted accounting bound still reaches a verdict.
#[test]
fn comparison_is_not_charged_to_the_call_budget() {
    let corpus = inc_corpus();
    let backends: [BackendEntry<'_>; 1] =
        [(ExecutionBackendChoice::Jit, &UnmeteredCallBackend, None)];
    let mut limits = ParityLimits::default();
    limits.accounting.work_units = 0;
    let report = fsl_conformance::run_parity_with_reference(
        &UnmeteredCallBackend,
        &backends,
        &corpus_only(corpus.path()),
        &limits,
        &Cancel::new(),
    )
    .expect("the comparison completes");
    assert_eq!(report.backends[0].compared, 1);
    assert_eq!(report.backends[0].total_findings, 0);
    assert_eq!(report.exit_code(), 0);
}

/// A `runtime`/`run` corpus vector over `files`, given as kind and path.
fn run_vector(id: &str, files: &[(&str, &str)]) -> serde_json::Value {
    let files = files
        .iter()
        .map(|(kind, path)| serde_json::json!({"kind":kind, "path":path}))
        .collect::<Vec<_>>();
    serde_json::json!({
        "id":id,
        "class":"positive",
        "applicability":{"consumer_types":["runtime"]},
        "subject":{"consumer_type":"runtime", "operation":"run"},
        "input":{"files":files},
        "expected":{"disposition":"value", "value":null}
    })
}

fn write_vectors(corpus: &std::path::Path, vectors: &[serde_json::Value]) {
    fs::write(
        corpus.join("complete-v1.json"),
        serde_json::to_vec(&serde_json::json!({"vectors":vectors})).expect("corpus JSON"),
    )
    .expect("corpus file");
}

/// AC-9's `inc` vector, then sixteen vectors with one defect each, in the
/// corpus directory `corpus` below a root that also holds a valid source.
struct DefectiveCorpus {
    _root: tempfile::TempDir,
    corpus: std::path::PathBuf,
    /// The `inc` vector.
    inc: serde_json::Value,
    /// Each defective vector with the cause the report gives it.
    defective: Vec<(serde_json::Value, UnbuildableCause)>,
}

const INC_SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n";

const SEEN_SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel M = \"acme/orders\";\nfunction seen using v(p: Population<M::Gadget>[1], ok: Boolean): Boolean pure { ok }\n";

/// A call file naming `function` with the `arguments` list.
fn call_file(function: &str, arguments: &str) -> String {
    format!(r#"{{"function":"{function}","arguments":[{arguments}],"accounting":{{}}}}"#)
}

/// The domain package `acme/orders` with a population `Fleet` of `extent`.
fn fleet_model(extent: &str) -> String {
    let mut model: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .expect("the model fixture");
    model["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(serde_json::json!({
            "kind":{"module":"acme/orders", "name":"population"}, "moduleVersion":"1.0.0",
            "construct":{"identity":"none", "shape":"record", "members":{}, "meaning":"filament.meaning.model.population/v1"}
        }));
    model["populations"] = serde_json::json!([{
        "identity":"ix://acme/orders/Fleet", "displayName":"Fleet", "kind":{"module":"acme/orders", "name":"population"},
        "members":["ix://acme/orders/Gadget"], "extent":extent,
        "origin":{"generated":{"generatorIdentity":"ix://acme/orders/Fleet", "generatorVersion":"1.0.0", "inputIdentities":["ix://acme/orders/Fleet"]}}
    }]);
    model.to_string()
}

/// A current state snapshot listing one `Fleet` object of `object_type`.
fn fleet_snapshot(object_type: &str) -> String {
    serde_json::json!({
        "format":"filament.state.snapshot/v1", "identity":{"authority":"fixture", "identity":"current"}, "observation":"current",
        "model":{"identity":"acme/orders"},
        "populations":[{"population":"ix://acme/orders/Fleet", "complete":true,
            "objects":[{"key":"g1", "type":object_type, "fields":{"code":{"integer":"7"}}}]}]
    })
    .to_string()
}

fn defective_corpus() -> DefectiveCorpus {
    let root = tempfile::tempdir().expect("fixture root");
    let corpus = root.path().join("corpus");
    fs::create_dir(&corpus).expect("corpus directory");
    let outside = root.path().join("outside.native");
    fs::write(&outside, INC_SOURCE).expect("source outside the corpus");
    let outside = outside.to_str().expect("a UTF-8 path").to_owned();
    let four = r#"{"kind":"integer","decimal":"4"}"#;
    let fleet =
        r#"{"kind":"population","name":"ix://acme/orders/Fleet"},{"kind":"boolean","value":true}"#;
    for (name, text) in [
        ("unit.native", INC_SOURCE.to_owned()),
        ("call.json", call_file("inc", four)),
        ("bad-call.json", "not JSON".to_owned()),
        (
            "extra-member.json",
            r#"{"function":"inc","arguments":[],"accounting":{},"extra":1}"#.to_owned(),
        ),
        (
            "broken.native",
            "language \"ix:native\" edition \"1-draft\";\nfunction broken using missing(x: Integer): Integer pure { x }\n".to_owned(),
        ),
        ("model.json", "not a model document".to_owned()),
        ("context.json", "not JSON".to_owned()),
        ("unknown.json", call_file("absent", four)),
        ("two.json", call_file("inc", &format!("{four},{four}"))),
        (
            "spelling.json",
            call_file("inc", r#"{"kind":"integer","decimal":"four"}"#),
        ),
        ("seen.native", SEEN_SOURCE.to_owned()),
        ("seen-call.json", call_file("seen", fleet)),
        ("closed.json", fleet_model("closed")),
        ("open.json", fleet_model("open")),
        (
            "listed.json",
            fleet_snapshot("ix://acme/orders/Gadget"),
        ),
        (
            "undeclared.json",
            fleet_snapshot("ix://acme/orders/Absent"),
        ),
    ] {
        fs::write(corpus.join(name), text).expect("fixture input");
    }
    let unit = ("source", "unit.native");
    let good = ("call", "call.json");
    let seen = ("source", "seen.native");
    let seen_call = ("call", "seen-call.json");
    let owned = |path: &str| path.to_owned();
    let defective = vec![
        (
            run_vector("absent-source", &[("source", "absent.native"), good]),
            UnbuildableCause::UnreadableFile(owned("absent.native")),
        ),
        (
            run_vector("absent-call", &[unit, ("call", "absent.json")]),
            UnbuildableCause::UnreadableFile(owned("absent.json")),
        ),
        (
            run_vector("parent-path", &[("source", "../outside.native"), good]),
            UnbuildableCause::UnsafePath(owned("../outside.native")),
        ),
        (
            run_vector("absolute-path", &[("source", &outside), good]),
            UnbuildableCause::UnsafePath(outside.clone()),
        ),
        (
            run_vector("no-call", &[unit]),
            UnbuildableCause::MissingMember("call file"),
        ),
        (
            run_vector("two-calls", &[unit, good, good]),
            UnbuildableCause::DuplicateFile,
        ),
        (
            run_vector("call-not-json", &[unit, ("call", "bad-call.json")]),
            UnbuildableCause::InvalidCall(owned("bad-call.json")),
        ),
        (
            run_vector("unknown-member", &[unit, ("call", "extra-member.json")]),
            UnbuildableCause::InvalidCall(owned("extra-member.json")),
        ),
        (
            run_vector(
                "bad-context",
                &[unit, ("object-context", "context.json"), good],
            ),
            UnbuildableCause::InvalidObservations,
        ),
        (
            run_vector("bad-integer", &[unit, ("call", "spelling.json")]),
            UnbuildableCause::UndecodableArgument(0),
        ),
        (
            run_vector("two-arguments", &[unit, ("call", "two.json")]),
            UnbuildableCause::ExtraArgument(1),
        ),
        (
            run_vector("broken-source", &[("source", "broken.native"), good]),
            UnbuildableCause::Uncompilable,
        ),
        (
            run_vector("bad-model", &[unit, ("model", "model.json"), good]),
            UnbuildableCause::Uncompilable,
        ),
        (
            run_vector(
                "refused-snapshot",
                &[
                    seen,
                    ("model", "closed.json"),
                    ("population", "undeclared.json"),
                    seen_call,
                ],
            ),
            UnbuildableCause::RefusedObservations,
        ),
        (
            run_vector("unknown-function", &[unit, ("call", "unknown.json")]),
            UnbuildableCause::UnknownFunction(owned("absent")),
        ),
        (
            run_vector(
                "open-population",
                &[
                    seen,
                    ("model", "open.json"),
                    ("population", "listed.json"),
                    seen_call,
                ],
            ),
            UnbuildableCause::RefusedArgument(0),
        ),
    ];
    DefectiveCorpus {
        inc: run_vector("inc", &[unit, good]),
        corpus,
        defective,
        _root: root,
    }
}

fn expecting_refusal(mut vector: serde_json::Value) -> serde_json::Value {
    vector["expected"] = serde_json::json!({"disposition":"refusal"});
    vector
}

/// The vector ids of one backend's findings, all of them `unbuildable-case`
/// findings carrying the cause the report lists for that vector.
fn unbuildable_findings<'a>(report: &'a ParityReport, backend: usize) -> Vec<&'a str> {
    report.backends[backend]
        .findings
        .iter()
        .map(|finding| {
            let listed = report
                .unbuildable
                .iter()
                .find(|case| case.id == finding.unit)
                .expect("the finding's vector is listed");
            assert_eq!(
                finding.kind,
                FindingKind::UnbuildableCase(listed.cause.clone())
            );
            finding.unit.as_str()
        })
        .collect()
}

/// TC-1387 FR-563-AC-10: only a defect of the corpus document refuses the run,
/// before any unit is prepared.
#[trace("TC-1387", "FR-563-AC-10")]
#[test]
fn a_document_level_defect_refuses_before_any_unit_runs() {
    let counting = CountingBackend::default();
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &counting, None)];
    let run = |corpus: &std::path::Path| {
        run_parity(
            &backends,
            &ParityInputs {
                count: 1,
                ..ParityInputs::new(corpus)
            },
            &ParityLimits::default(),
            &Cancel::new(),
        )
    };
    let valid = inc_corpus_with_count(2);
    let document_path = valid.path().join("complete-v1.json");
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&document_path).expect("corpus file"))
            .expect("corpus JSON");
    let mut unidentified = document.clone();
    unidentified["vectors"][1]
        .as_object_mut()
        .expect("a vector object")
        .remove("id");
    let mut duplicated = document.clone();
    duplicated["vectors"][1]["id"] = duplicated["vectors"][0]["id"].clone();
    let defects = [
        None,
        Some("not JSON".to_owned()),
        Some("{}".to_owned()),
        Some(unidentified.to_string()),
        Some(duplicated.to_string()),
    ];
    for defect in defects {
        match &defect {
            Some(text) => fs::write(&document_path, text).expect("defective corpus"),
            None => fs::remove_file(&document_path).expect("remove the corpus"),
        }
        let refusal = run(valid.path()).expect_err("a document-level defect refuses");
        assert_eq!(
            refusal,
            ParityRefusal::UnreadableCorpus(document_path.clone()),
            "{defect:?}"
        );
        assert_eq!(refusal.exit_code(), 2);
    }
    fs::remove_file(&document_path).expect("remove the corpus");
    fs::create_dir(&document_path).expect("a directory in the corpus's place");
    let refusal = run(valid.path()).expect_err("a directory is no corpus");
    assert_eq!(
        refusal,
        ParityRefusal::UnreadableCorpus(document_path.clone())
    );
    assert_eq!(counting.prepares.load(Ordering::Relaxed), 0);

    fs::remove_dir(&document_path).expect("remove the directory");
    fs::write(&document_path, document.to_string()).expect("the valid corpus");
    let report = run(valid.path()).expect("the valid corpus gives a report");
    assert_eq!(report.units, 3);
}

/// TC-1387 FR-563-AC-11: a vector no case can be built from is listed with its
/// cause and found on every backend, and every other case still runs.
#[trace("TC-1387", "FR-563-AC-11")]
#[test]
fn unbuildable_vectors_are_listed_and_found_while_the_rest_run() {
    let fixture = defective_corpus();
    let mut vectors = vec![fixture.inc.clone()];
    vectors.extend(fixture.defective.iter().map(|(vector, _)| vector.clone()));
    write_vectors(&fixture.corpus, &vectors);
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 2] = [
        (ExecutionBackendChoice::Jit, &interpreter, None),
        (ExecutionBackendChoice::Jit, &interpreter, None),
    ];
    let report = run_parity(
        &backends,
        &ParityInputs {
            count: 3,
            ..ParityInputs::new(&fixture.corpus)
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a defective vector refuses nothing");
    let expected = fixture
        .defective
        .iter()
        .map(|(vector, cause)| (vector["id"].as_str().expect("an id"), cause))
        .collect::<Vec<_>>();
    assert_eq!(
        report
            .unbuildable
            .iter()
            .map(|case| (case.id.as_str(), &case.cause))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(report.not_selected, 0);
    assert_eq!(report.units, 4, "inc and the three generated units");
    let ids = expected.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    for backend in 0..2 {
        assert_eq!(unbuildable_findings(&report, backend), ids);
        assert_eq!(report.backends[backend].total_findings, 16);
        assert!(report.backends[backend].compared > 1);
        for finding in &report.backends[backend].findings {
            assert_eq!(finding.seed, fsl_conformance::DEFAULT_PARITY_SEED);
            assert!(finding.arguments.is_empty());
            assert!(finding.reference.is_none() && finding.result.is_none());
        }
        let finding = |id: &str| {
            report.backends[backend]
                .findings
                .iter()
                .find(|finding| finding.unit == id)
                .expect("a finding for the vector")
        };
        let broken = finding("broken-source");
        assert!(String::from_utf8_lossy(&broken.source).contains("function broken"));
        assert_eq!(broken.entry, "inc");
        let absent = finding("absent-source");
        assert!(absent.source.is_empty() && absent.entry.is_empty());
    }
    assert_eq!(report.exit_code(), 1);
}

/// TC-1387 FR-563-AC-12: an expected refusal excuses only what FSL itself
/// refuses; a vector's missing, unsafe, unreadable or undecodable input is a
/// finding whatever it expects.
#[trace("TC-1387", "FR-563-AC-12")]
#[test]
fn an_expected_refusal_excuses_only_what_fsl_refuses() {
    let fixture = defective_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 2] = [
        (ExecutionBackendChoice::Jit, &interpreter, None),
        (ExecutionBackendChoice::Jit, &interpreter, None),
    ];
    let run = |vectors: &[serde_json::Value]| {
        write_vectors(&fixture.corpus, vectors);
        run_parity(
            &backends,
            &corpus_only(&fixture.corpus),
            &ParityLimits::default(),
            &Cancel::new(),
        )
        .expect("a defective vector refuses nothing")
    };
    let refused_by_fsl = [
        "broken-source",
        "bad-model",
        "refused-snapshot",
        "unknown-function",
        "open-population",
    ];

    let mut every = vec![fixture.inc.clone()];
    every.extend(
        fixture
            .defective
            .iter()
            .map(|(vector, _)| expecting_refusal(vector.clone())),
    );
    let report = run(&every);
    assert_eq!(report.unbuildable.len(), 16);
    for backend in 0..2 {
        assert_eq!(
            unbuildable_findings(&report, backend),
            [
                "absent-source",
                "absent-call",
                "parent-path",
                "absolute-path",
                "no-call",
                "two-calls",
                "call-not-json",
                "unknown-member",
                "bad-context",
                "bad-integer",
                "two-arguments",
            ]
        );
    }
    assert_eq!(report.exit_code(), 1);

    let mut excused = vec![fixture.inc.clone()];
    excused.extend(
        fixture
            .defective
            .iter()
            .filter(|(vector, _)| {
                refused_by_fsl
                    .iter()
                    .any(|id| vector["id"].as_str() == Some(id))
            })
            .map(|(vector, _)| expecting_refusal(vector.clone())),
    );
    let report = run(&excused);
    assert_eq!(report.unbuildable.len(), 5);
    assert_eq!(report.backends[0].total_findings, 0);
    assert_eq!(report.backends[1].total_findings, 0);
    assert_eq!(report.exit_code(), 0);

    // The expected disposition decides, not the vector's class.
    excused[1]["class"] = serde_json::json!("refusal");
    excused[1]["expected"] = serde_json::json!({"disposition":"value", "value":null});
    let report = run(&excused);
    assert_eq!(unbuildable_findings(&report, 0), ["broken-source"]);

    // The first defect met gives the cause: a malformed argument before
    // compilation, and a second source file in place of a later step's cause.
    let first_cause = [
        expecting_refusal(run_vector(
            "malformed-and-broken",
            &[("source", "broken.native"), ("call", "spelling.json")],
        )),
        expecting_refusal(run_vector(
            "one-of-two-sources-broken",
            &[
                ("source", "unit.native"),
                ("source", "broken.native"),
                ("call", "call.json"),
            ],
        )),
    ];
    let report = run(&first_cause);
    assert_eq!(
        report
            .unbuildable
            .iter()
            .map(|case| &case.cause)
            .collect::<Vec<_>>(),
        [
            &UnbuildableCause::UndecodableArgument(0),
            &UnbuildableCause::DuplicateFile,
        ]
    );
    assert_eq!(report.backends[0].total_findings, 2);

    // A vector that expects a refusal and builds is compared.
    let wrong = WrongValueBackend;
    write_vectors(&fixture.corpus, &[expecting_refusal(fixture.inc.clone())]);
    let report = run_parity(
        &[(ExecutionBackendChoice::Jit, &wrong, None)],
        &corpus_only(&fixture.corpus),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a built vector is compared");
    assert_eq!(
        report.backends[0].findings[0].kind,
        FindingKind::Difference(ParityMember::CompletedValue)
    );
    assert_eq!(report.exit_code(), 1);
}

/// The interpreter, completing every call it makes, whatever it was given.
#[derive(Clone, Copy)]
struct CompletingBackend;

struct CompletingPrepared<'p>(InterpreterPrepared<'p>);

impl ExecutionBackend for CompletingBackend {
    type Prepared<'p> = CompletingPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Interpreter
            .prepare(package, entries, limits, cancel)
            .map(CompletingPrepared)
    }
}

impl PreparedEntries for CompletingPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, filament_eval::UnsupportedEntry)] {
        self.0.unsupported()
    }

    fn call(
        &self,
        entry: &QualifiedName,
        _arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        let arguments = [1_i64, 2].map(|value| Value::Integer(Integer::from(value)));
        self.0
            .call(entry, &arguments, environment, limits, run, cancel)
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        self.0.evaluate(clause, input, accounting, cancel)
    }
}

/// TC-1387 FR-563-AC-15: a call supplying a leading part of the parameters is
/// no unbuildable vector: both sides are called with that prefix.
#[trace("TC-1387", "FR-563-AC-15")]
#[test]
fn a_prefix_call_is_compared() {
    let corpus = inc_corpus();
    fs::write(
        corpus.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }\n",
    )
    .expect("a two-parameter source");
    let interpreter = Interpreter;
    let completing = CompletingBackend;
    let report = run_parity(
        &[
            (ExecutionBackendChoice::Jit, &interpreter, None),
            (ExecutionBackendChoice::Jit, &completing, None),
        ],
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a prefix call is a parity case");
    assert!(report.unbuildable.is_empty(), "{:?}", report.unbuildable);
    assert_eq!(report.backends[0].compared, 1);
    assert_eq!(report.backends[0].total_findings, 0);
    assert_eq!(report.backends[1].total_findings, 1);
    let finding = &report.backends[1].findings[0];
    assert_eq!(
        finding.kind,
        FindingKind::Difference(ParityMember::OkAgainstErr)
    );
    assert_eq!(format!("{:?}", finding.arguments), "[Integer(Integer(4))]");
    let reference = format!("{:?}", finding.reference);
    assert!(
        reference.contains("Arity { declared: 2, supplied: 1 }"),
        "{reference}"
    );
}

/// TC-1387 FR-563-AC-16: a cancellation while a vector is built stops the run
/// with no report.
#[trace("TC-1387", "FR-563-AC-16")]
#[test]
fn cancellation_while_a_case_is_built_gives_no_report() {
    let fixture = defective_corpus();
    write_vectors(
        &fixture.corpus,
        &[run_vector(
            "broken-source",
            &[("source", "broken.native"), ("call", "call.json")],
        )],
    );
    let handle = std::sync::Arc::new(std::sync::OnceLock::<Cancel>::new());
    let charges = std::sync::Arc::new(AtomicUsize::new(0));
    let (observed, counted) = (handle.clone(), charges.clone());
    let cancel = Cancel::observing(move || {
        counted.fetch_add(1, Ordering::Relaxed);
        if let Some(cancel) = observed.get() {
            cancel.cancel(filament_exact::CancelCause::Requested);
        }
    });
    assert!(handle.set(cancel.clone()).is_ok(), "the handle is unset");
    let counting = CountingBackend::default();
    let refusal = run_parity(
        &[(ExecutionBackendChoice::Jit, &counting, None)],
        &corpus_only(&fixture.corpus),
        &ParityLimits::default(),
        &cancel,
    )
    .expect_err("a cancelled run gives no report");
    assert_eq!(refusal, ParityRefusal::Cancelled);
    assert!(charges.load(Ordering::Relaxed) > 0);
    assert_eq!(counting.prepares.load(Ordering::Relaxed), 0);
}

/// TC-1387: a vector whose readable call file names a clause is not selected,
/// whatever else it names; a path with a leading `./` is read.
#[trace("TC-1387")]
#[test]
fn a_clause_call_is_unselected_and_a_current_directory_path_is_read() {
    let corpus = inc_corpus();
    fs::write(corpus.path().join("clause.json"), r#"{"kind":"clause"}"#).expect("clause call");
    write_vectors(
        corpus.path(),
        &[
            run_vector(
                "dotted",
                &[("source", "./unit.native"), ("call", "./call.json")],
            ),
            run_vector(
                "clause",
                &[
                    ("source", "absent.native"),
                    ("model", "../outside.json"),
                    ("call", "clause.json"),
                ],
            ),
        ],
    );
    let interpreter = Interpreter;
    let report = run_parity(
        &[(ExecutionBackendChoice::Jit, &interpreter, None)],
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the corpus document is valid");
    assert_eq!(report.units, 1);
    assert_eq!(report.not_selected, 1);
    assert!(report.unbuildable.is_empty(), "{:?}", report.unbuildable);
    assert_eq!(report.exit_code(), 0);
}

/// TC-1387 FR-563-AC-6: reference faults are reported, while an empty backend
/// set and an unreadable corpus refuse without a verdict.
#[trace("TC-1387", "FR-563-AC-6")]
#[test]
fn reference_fault_and_preflight_refusals_are_distinct() {
    let corpus = inc_corpus();
    let reference = fsl_conformance::FaultInjectingBackend;
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
    let report = fsl_conformance::run_parity_with_reference(
        &reference,
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("test reference fault is a reported result");
    assert!(report.backends[0]
        .findings
        .iter()
        .any(|finding| { finding.kind == FindingKind::ReferenceFault && finding.entry == "inc" }));
    assert!(report.backends[0]
        .findings
        .iter()
        .any(|finding| finding.kind == FindingKind::EmptyComparison));
    assert_eq!(report.exit_code(), 1);

    // Generated entries carry four tuples. A reference Fault on the first
    // tuple excludes that whole entry, with one entry-level report and no
    // comparisons against the backend for its remaining tuples.
    let generated_corpus = empty_corpus();
    let multi_tuple = fsl_conformance::run_parity_with_reference(
        &reference,
        &backends,
        &ParityInputs {
            corpus: generated_corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 1,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("generated reference faults are reported");
    let entry_faults = &multi_tuple.backends[0].findings;
    let fault_findings = entry_faults
        .iter()
        .filter(|finding| finding.kind == FindingKind::ReferenceFault)
        .collect::<Vec<_>>();
    let entries = fault_findings
        .iter()
        .map(|finding| finding.entry.as_str())
        .collect::<BTreeSet<_>>();
    assert!(!entries.is_empty());
    assert_eq!(entries.len(), fault_findings.len());
    assert_eq!(multi_tuple.backends[0].compared, 0);

    let selective_reference = MutationBackend(Mutation::FaultOnNine);
    let selective = fsl_conformance::run_parity_with_reference(
        &selective_reference,
        &backends,
        &ParityInputs {
            corpus: generated_corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 1,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a later-tuple reference fault is reported");
    let later_faults = selective.backends[0]
        .findings
        .iter()
        .filter(|finding| finding.kind == FindingKind::ReferenceFault)
        .collect::<Vec<_>>();
    assert!(!later_faults.is_empty());
    assert!(later_faults
        .iter()
        .all(|finding| finding.arguments.iter().any(
            |value| matches!(value, Value::Integer(integer) if *integer == Integer::from(9_i64))
        )));
    let later_fault_entries = later_faults
        .iter()
        .map(|finding| finding.entry.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(later_fault_entries.len(), later_faults.len());

    let no_backends: [BackendEntry<'_>; 0] = [];
    let refusal = run_parity(
        &no_backends,
        &ParityInputs::new(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect_err("empty backend set refuses");
    assert_eq!(refusal, ParityRefusal::EmptyScope);
    assert_eq!(refusal.exit_code(), 2);

    let missing = corpus.path().join("does-not-exist");
    let refusal = run_parity(
        &backends,
        &ParityInputs::new(missing),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect_err("missing corpus refuses");
    assert!(matches!(refusal, ParityRefusal::UnreadableCorpus(_)));
    assert_eq!(refusal.exit_code(), 2);
}

/// TC-1386 FR-563-AC-7: findings are capped while all later corpus units run.
#[trace("TC-1386", "FR-563-AC-7")]
#[test]
fn finding_retention_is_bounded_without_stopping_the_run() {
    let corpus = inc_corpus_with_count(150);
    let wrong = WrongValueBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &wrong, None)];
    let report = run_parity(
        &backends,
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            count: 0,
            #[cfg(feature = "test-support")]
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("all corpus units are processed");
    assert_eq!(report.units, 150);
    assert_eq!(report.backends[0].compared, 150);
    assert_eq!(report.backends[0].findings.len(), 100);
    assert_eq!(report.backends[0].total_findings, 150);
}
