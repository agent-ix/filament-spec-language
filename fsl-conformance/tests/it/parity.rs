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
        r#"{"function":"inc","arguments":[{"kind":"integer","decimal":"4"}]}"#,
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

fn bytes_corpus() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction acceptBytes using v(x: Bytes): Boolean pure { true }\n",
    )
    .expect("source input");
    fs::write(
        directory.path().join("call.json"),
        r#"{"function":"acceptBytes","arguments":[{"kind":"bytes","text":"00ff"}]}"#,
    )
    .expect("call input");
    fs::write(
        directory.path().join("complete-v1.json"),
        r#"{"vectors":[{"id":"bytes-identity","applicability":{"consumer_types":["runtime"]},"subject":{"consumer_type":"runtime","operation":"run"},"input":{"files":[{"kind":"source","path":"unit.native"},{"kind":"call","path":"call.json"}]}}]}"#,
    )
    .expect("corpus file");
    directory
}

fn aggregate_corpus() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("corpus directory");
    fs::write(
        directory.path().join("unit.native"),
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nrecord Pair { x: Integer; }\nfunction acceptSequence using v(s: Sequence<Integer>[1, 2]): Boolean pure { true }\nfunction acceptPair using v(p: Pair): Boolean pure { true }\nfunction acceptMap using v(m: Map<Integer, Text[0, 8; nfc]>[1, 2]): Boolean pure { true }\n",
    )
    .expect("source input");
    let calls = [
        (
            "sequence",
            "acceptSequence",
            r#"{"sequence":[{"integer":"7"}]}"#,
        ),
        (
            "record",
            "acceptPair",
            r#"{"record":{"type":"Pair","fields":{"x":{"integer":"1"}}}}"#,
        ),
        (
            "map",
            "acceptMap",
            r#"{"map":[{"key":{"integer":"1"},"value":{"kind":"text","text":"x"}}]}"#,
        ),
    ];
    let vectors = calls
        .iter()
        .map(|(id, function, argument)| {
            let call_path = format!("{id}.json");
            fs::write(
                directory.path().join(&call_path),
                format!(r#"{{"function":"{function}","arguments":[{argument}]}}"#),
            )
            .expect("call input");
            serde_json::json!({
                "id": format!("{id}-value"),
                "applicability": {"consumer_types":["runtime"]},
                "subject": {"consumer_type":"runtime", "operation":"run"},
                "input": {"files":[
                    {"kind":"source", "path":"unit.native"},
                    {"kind":"call", "path":call_path}
                ]}
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        directory.path().join("complete-v1.json"),
        serde_json::to_vec(&serde_json::json!({"vectors": vectors})).expect("corpus JSON"),
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
            format!(r#"{{"function":"{name}","arguments":[{{"kind":"integer","decimal":"4"}}]}}"#),
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
        r#"{"function":"twice","arguments":[{"kind":"integer","decimal":"4"}]}"#,
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
        r#"{"function":"inc","arguments":[{"kind":"boolean","value":true}]}"#,
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

/// Refuses every preparation with an internal fault.
#[derive(Clone, Copy)]
struct PrepareFaultBackend;

impl ExecutionBackend for PrepareFaultBackend {
    type Prepared<'p> = InterpreterPrepared<'p>;

    fn prepare<'p>(
        &self,
        _package: &'p CheckedPackage,
        _entries: &[EntrySelection],
        _limits: &PrepareLimits,
        _cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Err(PrepareFailure::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "parity-test",
                "injected prepare fault",
            ),
        ))
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

/// A generator whose draws checking keeps refusing stops at the discard bound
/// and fails the gate by name; a run that stays within the bound does not.
#[test]
fn refused_draws_beyond_the_bound_fail_the_gate() {
    let corpus = empty_corpus();
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
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
    assert!(unbounded.discarded > 0, "the control needs a refused draw");
    let within = run(unbounded.discarded);
    assert_eq!(within.units, 100);
    assert_eq!(within.exit_code(), 0);

    let exhausted = run(unbounded.discarded - 1);
    assert!(exhausted.units < 100);
    assert_eq!(exhausted.discarded, unbounded.discarded);
    let finding = exhausted.backends[0]
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::GeneratorExhausted)
        .expect("the exhausted generator is named");
    assert!(!finding.source.is_empty());
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

/// FR-563's corpus adapter reads the FR-350 `bytes` input form as raw octets.
#[trace("TC-1386", "FR-563-AC-9")]
#[test]
fn corpus_bytes_argument_is_decoded_for_selected_runtime_vector() {
    let corpus = bytes_corpus();
    let backend = UnsupportedBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &backend, None)];
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
    .expect("a selected bytes argument is decoded and compared");
    assert_eq!(report.units, 1);
    assert_eq!(report.not_selected, 0);
    assert_eq!(report.backends[0].unsupported.len(), 1);
    assert_eq!(report.backends[0].unsupported[0].unit, "bytes-identity");
    assert_eq!(report.backends[0].unsupported[0].entry, "acceptBytes");
}

/// TC-1386 FR-563-AC-9: the shared FR-350 adapter decodes native aggregate
/// value forms using the selected function's declared parameter type.
#[trace("TC-1386", "FR-563-AC-9")]
#[test]
fn corpus_aggregate_arguments_use_the_shared_fr350_value_adapter() {
    let corpus = aggregate_corpus();
    let backend = UnsupportedBackend;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &backend, None)];
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
    .expect("native aggregate arguments are decoded and compared");
    assert_eq!(report.units, 3);
    let unsupported = &report.backends[0].unsupported;
    assert_eq!(unsupported.len(), 3);
    assert_eq!(
        unsupported
            .iter()
            .map(|row| row.unit.as_str())
            .collect::<Vec<_>>(),
        ["sequence-value", "record-value", "map-value"]
    );
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

/// TC-1387 FR-563-AC-6: a reference that faults while preparing a unit gives a
/// `reference-fault` for its entry and the later units still run; a reference
/// preparation limit refuses.
#[trace("TC-1387", "FR-563-AC-6")]
#[test]
fn reference_preparation_failures_are_a_finding_or_a_refusal() {
    let corpus = inc_corpus_with_count(3);
    let interpreter = Interpreter;
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &interpreter, None)];
    let report = fsl_conformance::run_parity_with_reference(
        &PrepareFaultBackend,
        &backends,
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("a reference prepare fault is a reported result");
    assert_eq!(report.units, 3);
    assert_eq!(report.backends[0].compared, 0);
    assert_eq!(
        report.backends[0]
            .findings
            .iter()
            .filter(|finding| finding.kind == FindingKind::ReferenceFault)
            .map(|finding| (finding.unit.as_str(), finding.entry.as_str()))
            .collect::<Vec<_>>(),
        [
            ("run-inc-0", "inc"),
            ("run-inc-1", "inc"),
            ("run-inc-2", "inc")
        ]
    );
    assert_eq!(report.exit_code(), 1);

    let refusal = fsl_conformance::run_parity_with_reference(
        &PrepareLimitBackend,
        &backends,
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a reference prepare limit refuses");
    assert!(matches!(
        refusal,
        ParityRefusal::ReferencePrepareLimit(ref limit)
            if limit.setting() == fsl_foundation::Setting::PrepareCompileWorkUnits
    ));
    assert_eq!(refusal.exit_code(), 2);
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

/// TC-1387 FR-563-AC-6: a selected vector whose source does not compile refuses
/// before any unit runs, the ones ahead of it included.
#[trace("TC-1387", "FR-563-AC-6")]
#[test]
fn an_uncompilable_selected_vector_refuses_before_any_unit_runs() {
    let corpus = inc_corpus_with_count(2);
    fs::write(
        corpus.path().join("broken.native"),
        "language \"ix:native\" edition \"1-draft\";\nfunction broken using missing(x: Integer): Integer pure { x }\n",
    )
    .expect("broken source");
    let document_path = corpus.path().join("complete-v1.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&document_path).expect("corpus file"))
            .expect("corpus JSON");
    document["vectors"][1]["input"]["files"][0]["path"] = serde_json::json!("broken.native");
    fs::write(
        &document_path,
        serde_json::to_vec(&document).expect("corpus JSON"),
    )
    .expect("corpus file");

    let counting = CountingBackend::default();
    let backends: [BackendEntry<'_>; 1] = [(ExecutionBackendChoice::Jit, &counting, None)];
    let refusal = run_parity(
        &backends,
        &corpus_only(corpus.path()),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect_err("an uncompilable selected vector refuses");
    assert!(matches!(refusal, ParityRefusal::UnreadableCorpus(_)));
    assert_eq!(refusal.exit_code(), 2);
    assert_eq!(counting.prepares.load(Ordering::Relaxed), 0);
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
