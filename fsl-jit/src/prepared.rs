// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-294, FR-560, FR-561, FR-568: the [`Jit`] backend and [`PreparedJit`],
//! the safe wrapper that upholds the contract of the `exec` module.

use std::collections::BTreeMap;

use filament_eval::{
    CallFailure, EnvironmentRefusal, Evaluation, ExecutionBackendChoice, FamilyOutcome,
    UnsupportedCause, UnsupportedEntry,
};
use filament_exact::{Cancel, Charge, ChargePoint, Integer, Meter, Outcome, Refusal, ScalarLimits, Value};
use filament_semantic_value::call::InputRefusal;
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::location::ExpressionLocation;
use filament_semantic_value::stop::{outcome_from_stop, Stop};
use filament_semantic_value::supplied_stop::SuppliedRun;
use fsl_eval::execution::{
    CallLimits, EntryKind, EntrySelection, ExecutionBackend, PrepareFailure, PrepareLimits,
    PrepareMeter, PreparedEntries,
};
use fsl_eval::value::{admit_arguments, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::AdmittedObservations;

use crate::coverage::coverage;
use crate::exec::{Entry, EntryFrame, EntryRefusal, Halt, Machine, Status};
use crate::translate::{register_kind, type_of, Compiler, Failure, Op, RegisterKind};
use fsl_eval::execution::coverage::OutsideCoverage as Outside;

/// The just-in-time backend (ADR-029 EB-1): it translates the entries inside
/// [`coverage`] and lists every other selected entry unsupported.
#[derive(Clone, Copy, Debug, Default)]
pub struct Jit;

/// One translated function.
struct Compiled {
    entry: Entry,
    /// The runtime representation of each parameter, or `None` when its
    /// checked type is outside the backend's value coverage.
    parameters: Vec<Option<RegisterKind>>,
    result_is_boolean: bool,
    result_is_integer: bool,
    /// Where an evaluation that stops before the body reports: the body root.
    site: ExpressionLocation,
}

/// What [`Jit`] prepared: the translated functions and the unsupported
/// entries, with the executable memory behind the former.
///
/// A callable entry borrows the value that holds its code:
///
/// ```compile_fail,E0505
/// use fsl_eval::value::QualifiedName;
/// use fsl_jit::PreparedJit;
/// fn held(prepared: PreparedJit<'_>, name: &QualifiedName) {
///     let entry = prepared.entry(name).expect("an entry");
///     drop(prepared);
///     let _ = entry.name();
/// }
/// ```
///
/// The control uses the entry within the borrow:
///
/// ```
/// use fsl_eval::value::QualifiedName;
/// use fsl_jit::PreparedJit;
/// fn within(prepared: PreparedJit<'_>, name: &QualifiedName) {
///     let entry = prepared.entry(name).expect("an entry");
///     let _ = entry.name();
///     drop(prepared);
/// }
/// ```
///
/// The frame type is private to `exec`:
///
/// ```compile_fail
/// use fsl_jit::exec::EntryFrame;
/// ```
pub struct PreparedJit<'p> {
    package: &'p CheckedPackage,
    compiled: BTreeMap<QualifiedName, Compiled>,
    compiled_clauses: BTreeMap<QualifiedName, Compiled>,
    ops: Vec<Op>,
    unsupported: Vec<(EntrySelection, UnsupportedEntry)>,
    // Declared last: it is dropped last, after nothing can call an entry.
    _machine: Option<Machine>,
}

/// A callable entry of a [`PreparedJit`], valid while the value is borrowed.
pub struct CallableEntry<'a> {
    prepared: &'a PreparedJit<'a>,
    name: &'a QualifiedName,
}

impl<'a> CallableEntry<'a> {
    /// The entry's function name.
    #[must_use]
    pub fn name(&self) -> &QualifiedName {
        self.name
    }

    /// Call the entry as [`PreparedJit::call_metered`] does.
    ///
    /// # Errors
    /// As [`PreparedEntries::call`].
    pub fn call(
        &self,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &mut Meter,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        self.prepared
            .call_metered(self.name, arguments, environment, meter, run, cancel)
    }
}

fn fault(invariant: &'static str) -> PrepareFailure {
    PrepareFailure::Fault(InternalFault::new("FR-560", invariant))
}

fn executable_memory_refusal(selection: &EntrySelection) -> (EntrySelection, UnsupportedEntry) {
    (
        selection.clone(),
        UnsupportedEntry {
            backend: ExecutionBackendChoice::Jit,
            cause: UnsupportedCause::EnvironmentRefused(
                EnvironmentRefusal::ExecutableMemoryRefused,
            ),
        },
    )
}

fn refused_prepared<'p>(
    package: &'p CheckedPackage,
    entries: &[EntrySelection],
) -> PreparedJit<'p> {
    PreparedJit {
        package,
        compiled: BTreeMap::new(),
        compiled_clauses: BTreeMap::new(),
        ops: Vec::new(),
        unsupported: entries.iter().map(executable_memory_refusal).collect(),
        _machine: None,
    }
}

impl Jit {
    fn prepare_using<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
        executable_memory_permitted: impl FnOnce() -> bool,
    ) -> Result<PreparedJit<'p>, PrepareFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(PrepareFailure::Cancelled(cause));
        }
        if !executable_memory_permitted() {
            return Ok(refused_prepared(package, entries));
        }
        let mut meter = PrepareMeter::new(limits, cancel);
        // Executable-memory refusal applies to every selected entry: the JIT
        // has no callable code for any of them, but this is an environment
        // refusal rather than an invariant fault.
        let (mut machine, shim) = match Machine::new() {
            Ok(machine) => machine,
            Err(crate::exec::MachineError::ExecutableMemoryRefused) => {
                return Ok(refused_prepared(package, entries));
            }
            Err(crate::exec::MachineError::Unavailable) => {
                return Err(fault("jit-host-code-generator-unavailable"));
            }
        };
        let mut ops = Vec::new();
        let mut unsupported = Vec::new();
        let mut translated = Vec::new();
        let mut translated_clauses = Vec::new();
        let graph = package.graph();
        {
            let mut compiler = Compiler::new(
                machine.module(),
                &shim,
                graph,
                coverage(),
                &mut ops,
                &mut meter,
            );
            let mut seen = std::collections::BTreeSet::new();
            let mut function_count = 0;
            while graph.function_identity_at(function_count).is_some() {
                function_count += 1;
            }
            let mut clause_index = 0;
            for selection in entries {
                if !seen.insert((selection.kind == EntryKind::Clause, selection.name.clone())) {
                    continue;
                }
                let name = selection.name.to_string();
                if selection.kind == EntryKind::Clause {
                    // The interpreter refuses an unknown clause at the call;
                    // so does this backend.
                    let Some(clause) = graph.state_clause(&name) else {
                        continue;
                    };
                    let body = clause.body();
                    match fsl_eval::execution::coverage::first_outside_body(
                        graph,
                        &[],
                        body,
                        coverage(),
                    ) {
                        Ok(Some(outside)) => {
                            unsupported.push((selection.clone(), untranslatable(outside)));
                            continue;
                        }
                        Err(fault) => return Err(PrepareFailure::Fault(fault)),
                        Ok(None) => {}
                    }
                    let index = function_count.saturating_add(clause_index);
                    clause_index = clause_index.saturating_add(1);
                    let parameters = clause
                        .parameters()
                        .iter()
                        .enumerate()
                        .map(|(position, (_, value_type))| {
                            if position == 0 {
                                None
                            } else {
                                type_of(value_type)
                            }
                        })
                        .collect::<Vec<_>>();
                    match compiler.clause_entry(index, &parameters, body) {
                        Ok(entry) => translated_clauses.push((selection.name.clone(), entry)),
                        Err(Failure::Unsupported(outside)) => {
                            unsupported.push((selection.clone(), untranslatable(outside)));
                        }
                        Err(Failure::Prepare(failure)) => return Err(failure),
                    }
                    continue;
                }
                let Some(callable) = graph.callable(&name) else {
                    continue;
                };
                let Some(index) = compiler.index_of(&callable.identity) else {
                    return Err(fault("jit-callable-without-index"));
                };
                match compiler.entry(index) {
                    Ok(entry) => translated.push((selection.name.clone(), index, entry)),
                    Err(Failure::Unsupported(outside)) => {
                        unsupported.push((selection.clone(), untranslatable(outside)));
                    }
                    Err(Failure::Prepare(failure)) => return Err(failure),
                }
            }
        }
        match machine.finalize() {
            Ok(()) => {}
            Err(crate::exec::MachineError::ExecutableMemoryRefused) => {
                return Ok(refused_prepared(package, entries));
            }
            Err(crate::exec::MachineError::Unavailable) => {
                return Err(fault("jit-finalize-definitions"));
            }
        }
        let mut compiled = BTreeMap::new();
        let mut compiled_clauses = BTreeMap::new();
        for (name, index, entry) in translated {
            let (Some(function), Some(callable)) = (
                graph.function_state(index),
                graph.callable(&name.to_string()),
            ) else {
                return Err(fault("jit-callable-vanished"));
            };
            compiled.insert(
                name,
                Compiled {
                    entry: machine.entry(entry.id, callable.parameters.len(), entry.signature),
                    parameters: callable
                        .parameters
                        .iter()
                        .map(|(_, ty)| register_kind(ty))
                        .collect(),
                    result_is_boolean: entry.result_is_boolean,
                    result_is_integer: entry.result_is_integer,
                    site: function.body.location().clone(),
                },
            );
        }
        for (name, entry) in translated_clauses {
            let Some(clause) = graph.state_clause(&name.to_string()) else {
                return Err(fault("jit-clause-vanished"));
            };
            compiled_clauses.insert(
                name,
                Compiled {
                    entry: machine.entry(entry.id, clause.parameters().len(), entry.signature),
                    parameters: clause
                        .parameters()
                        .iter()
                        .enumerate()
                        .map(|(position, (_, value_type))| {
                            if position == 0 {
                                None
                            } else {
                                register_kind(value_type)
                            }
                        })
                        .collect(),
                    result_is_boolean: entry.result_is_boolean,
                    result_is_integer: entry.result_is_integer,
                    site: clause.body().location().clone(),
                },
            );
        }
        Ok(PreparedJit {
            package,
            compiled,
            compiled_clauses,
            ops,
            unsupported,
            _machine: Some(machine),
        })
    }

    /// Prepare with a test-supplied executable-memory permission decision.
    /// A denied request follows FR-560's normal per-entry refusal path.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn prepare_with_executable_memory_provider_for_test<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
        provider: impl FnOnce() -> bool,
    ) -> Result<PreparedJit<'p>, PrepareFailure> {
        self.prepare_using(package, entries, limits, cancel, provider)
    }
}

impl ExecutionBackend for Jit {
    type Prepared<'p> = PreparedJit<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<PreparedJit<'p>, PrepareFailure> {
        self.prepare_using(package, entries, limits, cancel, || true)
    }
}

fn untranslatable(outside: Outside) -> UnsupportedEntry {
    UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause: UnsupportedCause::Untranslatable {
            construct: outside.construct,
            location: outside.location,
        },
    }
}

/// The evaluation a stop gives, as the interpreter gives it.
fn stopped(
    stop: Stop,
    location: Option<ExpressionLocation>,
    peak_live_occurrences: u64,
    frame_depth: u32,
) -> Result<Evaluation, CallFailure> {
    if let Stop::Refused(Refusal::CheckedInvariant { cause }) = stop {
        return Err(CallFailure::Fault(InternalFault::from_kernel(
            "S6a",
            "checked-program-invariant",
            cause,
        )));
    }
    Ok(Evaluation {
        outcome: FamilyOutcome::Evaluated(outcome_from_stop(Err(stop))),
        location,
        losses: Vec::new(),
        frame_depth,
        peak_live_occurrences,
    })
}

fn frame_occurrences(arguments: &[Value]) -> u64 {
    arguments
        .iter()
        .fold(Integer::one(), |sum, value| sum.add(&value.occ()))
        .to_u64()
        .unwrap_or(u64::MAX)
}

impl PreparedEntries for PreparedJit<'_> {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &self.unsupported
    }

    fn call(
        &self,
        function: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(CallFailure::Cancelled(cause));
        }
        let mut meter = Meter::new(limits.accounting).with_cancel(cancel.clone());
        self.call_metered(function, arguments, environment, &mut meter, run, cancel)
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some(cause) = cancel.cause() {
            return Err(CallFailure::Cancelled(cause));
        }
        if let Some((_, reason)) = self
            .unsupported
            .iter()
            .find(|(selection, _)| selection.kind == EntryKind::Clause && selection.name == *clause)
        {
            return Err(CallFailure::Unsupported(reason.clone()));
        }
        let Some(compiled) = self.compiled_clauses.get(clause) else {
            return Err(InputRefusal::UnknownClause(clause.to_string()).into());
        };
        let graph = self.package.graph();
        let checked_clause = graph
            .state_clause(&clause.to_string())
            .ok_or_else(|| InputRefusal::UnknownClause(clause.to_string()))?;
        if input.clause != checked_clause.identity() {
            return Err(InputRefusal::ObservationsMismatch.into());
        }
        let mut clause_arguments = Vec::with_capacity(checked_clause.parameters().len());
        for (position, (name, _)) in checked_clause.parameters().iter().enumerate() {
            let value = match (position, name.as_str()) {
                (0, _) => Value::Reference(input.self_object.clone()),
                (_, "result") => input
                    .result
                    .clone()
                    .ok_or(InputRefusal::ObservationsMismatch)?,
                _ => input
                    .parameters
                    .iter()
                    .find(|(parameter, _)| parameter == name)
                    .map(|(_, value)| value.clone())
                    .ok_or(InputRefusal::ObservationsMismatch)?,
            };
            clause_arguments.push(value);
        }
        let peak_live_occurrences = frame_occurrences(&clause_arguments);
        let mut meter = Meter::new(*accounting).with_cancel(cancel.clone());
        let mut frame = EntryFrame::new(
            &mut meter,
            &self.ops,
            graph,
            input.current.as_ref().map(|observation| &observation.environment),
            peak_live_occurrences,
        );
        let arguments = registers(compiled, &clause_arguments, &mut frame)?;
        let status = compiled.entry.call(&mut frame, &arguments);
        let value = result_value(compiled, &frame);
        let losses = frame.losses().to_vec();
        let peak_live_occurrences = frame.peak_live_occurrences();
        let frame_depth = frame.frame_depth();
        let halt = frame.into_halt();
        let status = status.map_err(|refusal| {
            CallFailure::Fault(InternalFault::new(
                "FR-568",
                match refusal {
                    EntryRefusal::Signature => "jit-entry-signature-mismatch",
                    EntryRefusal::Status => "jit-entry-status-out-of-range",
                    EntryRefusal::Arity => "jit-argument-count",
                },
            ))
        })?;
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        match (status, halt) {
            (Status::Completed, None) => Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Completed(value.ok_or_else(|| {
                    CallFailure::Fault(InternalFault::new(
                        "FR-560",
                        "jit-result-integer-handle-is-valid",
                    ))
                })?)),
                location: None,
                losses,
                frame_depth: 0,
                peak_live_occurrences,
            }),
            (Status::Stopped, Some(Halt::Stop(stop, location))) => {
                stopped(stop, Some(location), peak_live_occurrences, frame_depth)
            }
            (_, Some(Halt::Fault(invariant))) => {
                Err(CallFailure::Fault(InternalFault::new("FR-560", invariant)))
            }
            _ => Err(CallFailure::Fault(InternalFault::new(
                "FR-568",
                "jit-status-halt-disagree",
            ))),
        }
    }
}

impl<'p> PreparedJit<'p> {
    /// The callable entry of the translated function `name`, borrowed from
    /// this value: the code it enters is freed when the value drops.
    #[must_use]
    pub fn entry<'a>(&'a self, name: &'a QualifiedName) -> Option<CallableEntry<'a>>
    where
        'p: 'a,
    {
        // SAFETY-free: the borrow of `self` ties the entry to the memory.
        self.compiled.contains_key(name).then(|| CallableEntry {
            prepared: self.shrink(),
            name,
        })
    }

    fn shrink<'a>(&'a self) -> &'a PreparedJit<'a>
    where
        'p: 'a,
    {
        self
    }

    /// Record `parameters` and `returns_u32` as the signature of the entry
    /// `name` in place of the real one (a test hook for FR-568-AC-3): a call
    /// on it then returns `Fault` without entering the code.
    #[cfg(feature = "test-support")]
    pub fn record_signature_for_test(
        &mut self,
        name: &QualifiedName,
        parameters: u8,
        returns_u32: bool,
    ) {
        if let Some(compiled) = self.compiled.get_mut(name) {
            compiled.entry = compiled
                .entry
                .with_recorded_signature(crate::exec::Signature::for_test(parameters, returns_u32));
        }
    }

    /// Replace the entry `name` with one returning a status outside the
    /// closed status type (a test hook for FR-568-AC-3).
    #[cfg(feature = "test-support")]
    pub fn return_out_of_range_status_for_test(&mut self, name: &QualifiedName) {
        if let Some(compiled) = self.compiled.get_mut(name) {
            compiled.entry = compiled.entry.returning_out_of_range_status();
        }
    }

    /// The signature recorded at generation for the entry `name`, as its
    /// parameter count and whether it returns a `u32`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn recorded_signature_for_test(&self, name: &QualifiedName) -> Option<(u8, bool)> {
        self.compiled
            .get(name)
            .map(|compiled| compiled.entry.recorded().shape())
    }

    /// [`PreparedEntries::call`] under a caller-owned `meter` and supplied-value
    /// `run`, with the contract of `CheckedPackageEvaluation::call_supplied`:
    /// the meter then shows the charges of the call, which the parity gate
    /// compares with the interpreter's (FR-562).
    ///
    /// # Errors
    /// As [`PreparedEntries::call`].
    pub fn call_metered(
        &self,
        function: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        meter: &mut Meter,
        run: &mut SuppliedRun,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some((_, reason)) = self.unsupported.iter().find(|(selection, _)| {
            selection.kind == EntryKind::Function && selection.name == *function
        }) {
            return Err(CallFailure::Unsupported(reason.clone()));
        }
        let graph = self.package.graph();
        let callable = graph
            .callable(&function.to_string())
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        // A function the caller did not select at `prepare` has no generated
        // code, and this backend never runs what it did not translate
        // (FR-296): it is a caller condition, refused as an unknown entry of
        // this prepared value, not a backend fault.
        let Some(compiled) = self.compiled.get(function) else {
            return Err(InputRefusal::UnknownFunction(function.to_string()).into());
        };
        admit_arguments(
            callable.parameters,
            arguments,
            environment,
            graph.scope().types(),
            run,
        )?;
        let initial_live_occurrences = frame_occurrences(arguments);
        // FR-401: the scheduler charges `function.call` before the body.
        if let Err(denied) = meter.charge(Charge::new(ChargePoint::FunctionCall)) {
            if let Some(cause) = cancel.tripped() {
                return Err(CallFailure::Cancelled(cause));
            }
            return Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(denied)),
                location: Some(compiled.site.clone()),
                losses: Vec::new(),
                frame_depth: 0,
                peak_live_occurrences: 0,
            });
        }
        let mut frame = EntryFrame::new(
            meter,
            &self.ops,
            graph,
            Some(environment),
            initial_live_occurrences,
        );
        let registers = registers(compiled, arguments, &mut frame)?;
        let status = compiled.entry.call(&mut frame, &registers);
        let value = result_value(compiled, &frame);
        let losses = frame.losses().to_vec();
        let peak_live_occurrences = frame.peak_live_occurrences();
        let frame_depth = frame.frame_depth();
        let halt = frame.into_halt();
        let status = status.map_err(|refusal| {
            CallFailure::Fault(InternalFault::new(
                "FR-568",
                match refusal {
                    EntryRefusal::Signature => "jit-entry-signature-mismatch",
                    EntryRefusal::Status => "jit-entry-status-out-of-range",
                    EntryRefusal::Arity => "jit-argument-count",
                },
            ))
        })?;
        if let Some(cause) = cancel.tripped() {
            return Err(CallFailure::Cancelled(cause));
        }
        match (status, halt) {
            (Status::Completed, None) => Ok(Evaluation {
                outcome: FamilyOutcome::Evaluated(Outcome::Completed(value.ok_or_else(|| {
                    CallFailure::Fault(InternalFault::new(
                        "FR-560",
                        "jit-result-integer-handle-is-valid",
                    ))
                })?)),
                location: None,
                losses,
                frame_depth: 0,
                peak_live_occurrences,
            }),
            (Status::Stopped, Some(Halt::Stop(stop, location))) => {
                stopped(stop, Some(location), peak_live_occurrences, frame_depth)
            }
            (_, Some(Halt::Fault(invariant))) => {
                Err(CallFailure::Fault(InternalFault::new("FR-560", invariant)))
            }
            _ => Err(CallFailure::Fault(InternalFault::new(
                "FR-568",
                "jit-status-halt-disagree",
            ))),
        }
    }
}

/// The 64-bit register image of `arguments` for `compiled`.
fn registers(
    compiled: &Compiled,
    arguments: &[Value],
    frame: &mut EntryFrame<'_>,
) -> Result<Vec<i64>, CallFailure> {
    let broken = || CallFailure::Fault(InternalFault::new("FR-560", "jit-argument-kind"));
    compiled
        .parameters
        .iter()
        .zip(arguments)
        .map(|(kind, argument)| match (kind, argument) {
            (Some(RegisterKind::Boolean), Value::Boolean(value)) => Ok(i64::from(*value)),
            (Some(RegisterKind::Integer), Value::Integer(value)) => {
                crate::translate::integer_to_i64(value).ok_or_else(broken)
            }
            (Some(RegisterKind::ExactValue), value) => frame
                .store_value(value.clone())
                .map_err(|invariant| CallFailure::Fault(InternalFault::new("FR-560", invariant))),
            // The translator binds no slot for a parameter without a register
            // kind, so no translated body reads this word.
            (None, _) => Ok(0),
            _ => Err(broken()),
        })
        .collect()
}

fn result_value(compiled: &Compiled, frame: &EntryFrame<'_>) -> Option<Value> {
    if compiled.result_is_boolean {
        Some(Value::Boolean(frame.result() != 0))
    } else if compiled.result_is_integer {
        Some(Value::Integer(Integer::from(frame.result())))
    } else {
        frame.result_value()
    }
}
