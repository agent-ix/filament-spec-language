// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-564's `execute` tool-failure cell: a backend outside FSL's evaluator
//! that faults.

use filament_eval::{CallFailure, Evaluation, UnsupportedEntry};
use filament_exact::{Cancel, ScalarLimits, Value};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::execution::{
    CallLimits, EntrySelection, ExecutionBackend, PrepareFailure, PrepareLimits, PreparedEntries,
};
use fsl_eval::value::QualifiedName;
use fsl_package::CheckedPackage;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::AdmittedObservations;

/// A test backend whose `prepare` succeeds and whose `call` returns
/// [`CallFailure::Fault`] (FR-564, FR-565). The driver links it into `fsl`
/// as a supplied backend, behind its test-only feature, so the cell reaches
/// the CLI.
#[derive(Clone, Copy, Debug, Default)]
pub struct FaultInjectingBackend;

/// What [`FaultInjectingBackend`] prepared: nothing, and no unsupported
/// entry.
#[derive(Clone, Copy, Debug)]
pub struct FaultInjectingPrepared;

fn fault() -> CallFailure {
    CallFailure::Fault(InternalFault::new(
        "fsl-conformance",
        "the fault-injecting backend faults on every call",
    ))
}

impl ExecutionBackend for FaultInjectingBackend {
    type Prepared<'p> = FaultInjectingPrepared;

    fn prepare<'p>(
        &self,
        _package: &'p CheckedPackage,
        _entries: &[EntrySelection],
        _limits: &PrepareLimits,
        _cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        Ok(FaultInjectingPrepared)
    }
}

impl PreparedEntries for FaultInjectingPrepared {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &[]
    }

    fn call(
        &self,
        _entry: &QualifiedName,
        _arguments: &[Value],
        _environment: &ObjectEnvironment,
        _limits: &CallLimits,
        _cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        Err(fault())
    }

    fn evaluate(
        &self,
        _clause: &QualifiedName,
        _input: &AdmittedObservations,
        _accounting: &ScalarLimits,
        _cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        Err(fault())
    }
}
