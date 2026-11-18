// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100: `fsl_replay::spine::run`, the spine `run` entry (ADR-011 §5).
//!
//! Compiles a `1-draft` program through the spine ([`super::compile`]) and
//! calls the one function the caller names through S6a
//! (`CheckedPackage::call`), with the arguments the caller supplies. This
//! module's public items name no `fsl_eval` path (FR-100-AC-8): the S6a
//! call itself, and the conversion of its `fsl_eval`-shaped result into the
//! `fsl_foundation`/`fsl_semantics`/`filament_exact` vocabulary below, are both
//! private to this module.
//!
//! FR-096 (PR #465/#469, #490) governs how a refused outcome
//! renders: a refusal with an FR-096 record renders its `code`, `cause`,
//! `fields` and `locus` -- every kernel refusal but `CheckedInvariant` has
//! one (`kernel_refusal_record`); a family
//! refusal with no record renders `code`/`cause` alone; `CheckedInvariant`
//! and a broken S6a invariant are internal failures ([`RunRefusal::Fault`]),
//! never a `refused` outcome.

use std::collections::BTreeMap;

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{
    Cancel, CancelCause, Incomplete, Integer, Outcome, Refusal, ScalarLimits, Undefined, Value,
    ValueType,
};
use filament_semantic_value::catalog::{CatalogCode, Category, InternalFault};
use filament_semantic_value::declaration::EnvironmentFailure;
use filament_semantic_value::location::ExpressionLocation as Location;
use fsl_eval::execution::{
    execute, ExecuteLimits, ExecuteRequest, ExecuteSelection, PrepareLimits, SuppliedBackends,
};
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::diagnostic::{Code, Locus, RefusalRecord};
use fsl_foundation::source::Source;
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedGraph;
use fsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::refusal::OfferedSelection;
use fsl_semantics::value::environment_stage::membership_failure_code;

use super::lifecycle as front_end;
use super::run_value::{ArgumentDefect, Forms, ReadStop};
use super::{
    AdmittedModels, CheckedUnit, CompileRefusal, DependencyInput, EmittedUnit, ProfileCatalogInput,
    SpineLimits,
};
use fsl_foundation::SourceIdentity;

/// FR-100: the default `work_units` limit when a request omits it.
pub const DEFAULT_WORK_UNITS: u64 = 1_000_000;

/// FR-100's default `work_units` with every other accounting counter at
/// `u64::MAX`.
pub fn default_accounting(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units,
        result_units: u64::MAX,
    }
}

/// One `{parameter, value}` argument of a [`Call`], keyed by the parameter's
/// declared name. File request adapters lower their canonical integer
/// assignments to this shared typed carrier before entering the spine.
#[derive(Clone, Debug)]
pub struct CallArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// The shared typed value.
    pub value: Value,
}

/// One `{parameter, value}` argument as the request wrote it: FR-650's run
/// value form, read against the parameter's declared type once the program
/// is checked.
#[derive(Clone, Debug)]
pub struct WireArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// A JSON integer in the signed 64-bit range, or a JSON object.
    pub value: serde_json::Value,
}

/// A call's snapshot document, as the command path admitted its bytes, and
/// the limits its admission runs under.
#[derive(Clone)]
pub struct CallSnapshot {
    /// The snapshot document.
    pub observation: super::SuppliedObservation,
    /// FR-106's admission limits.
    pub limits: fsl_semantics::model::observation::ObservationLimits,
}

impl std::fmt::Debug for CallSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallSnapshot")
            .field("reference", &self.observation.reference)
            .finish_non_exhaustive()
    }
}

/// The `call` member of a 1-draft native-run/1 request (FR-100 Inputs):
/// which function to call, its arguments, and the S6a accounting limits the
/// call runs under.
#[derive(Clone, Debug)]
pub struct Call {
    /// The called function's name: `.`-separated segments, each an
    /// identifier.
    pub function: String,
    /// The call's arguments, one per parameter, in any order.
    pub arguments: Vec<CallArgument>,
    /// The call's arguments as a request wrote them (FR-650); they bind with
    /// [`Self::arguments`], one per parameter.
    pub wire_arguments: Vec<WireArgument>,
    /// FR-650 "Arguments": the one current snapshot whose object world a
    /// Reference or Population argument resolves against; `None` for a call
    /// with an empty object world.
    pub snapshot: Option<CallSnapshot>,
    /// The S6a `filament.value.accounting/v1` limits the call charges under.
    pub accounting: ScalarLimits,
    /// FR-277: the caller's supplied-value helper ceilings. An omitted
    /// ceiling is its published default.
    pub supplied: filament_semantic_value::supplied_stop::SuppliedLimits,
    /// FR-296: the execution backend that runs the call, `Interpreter` by
    /// default. A backend the caller did not supply refuses; no other
    /// backend runs in its place.
    pub backend: filament_eval::ExecutionBackendChoice,
    /// FR-640, FR-100: the clock configurations of the program's
    /// clock-bearing declarations by clock name; empty when the request
    /// supplied none.
    pub clocks: super::RequestClocks,
}

impl Call {
    /// A call on the interpreter, the default backend (FR-296).
    #[must_use]
    pub fn new(
        function: String,
        arguments: Vec<CallArgument>,
        accounting: ScalarLimits,
        supplied: filament_semantic_value::supplied_stop::SuppliedLimits,
    ) -> Self {
        Self {
            function,
            arguments,
            wire_arguments: Vec::new(),
            snapshot: None,
            accounting,
            supplied,
            backend: filament_eval::ExecutionBackendChoice::default(),
            clocks: super::RequestClocks::default(),
        }
    }
}

/// A value [`run`] completed. [`run`] refuses `unknown_required_feature`
/// before any call for a function whose declared result has no run value form
/// (FR-650).
#[derive(Clone, Debug)]
pub enum CallValue {
    /// `{"kind": "boolean", "value": ...}`.
    Boolean(bool),
    /// `{"kind": "integer", "decimal": ...}` (FR-038's spelling is
    /// `Integer`'s own `Display`).
    Integer(Integer),
    /// An admitted model reference, retaining its exact identity triple.
    Reference(filament_exact::ObjectReference),
    /// Any other kind, in FR-650's run value form.
    Written(serde_json::Value),
}

impl CallValue {
    /// The value in FR-650's run value form.
    pub fn form(&self) -> serde_json::Value {
        match self {
            Self::Boolean(value) => serde_json::json!({"boolean": value}),
            Self::Integer(value) => serde_json::json!({"integer": value.to_string()}),
            Self::Reference(reference) => super::run_value::reference_form(reference),
            Self::Written(form) => form.clone(),
        }
    }
}

/// A refusal's [`Locus::Region`], resolved to the path of the source it
/// points into and its span rendered over that source's bytes (FR-096,
/// FR-100).
#[derive(Clone, Debug)]
pub struct CallLocus {
    /// Authored path of the source containing the region (FR-100's wire).
    pub file: String,
    /// The region's span, rendered over that source.
    pub span: fsl_foundation::LocatedSpan,
}

/// FR-100's `refused` outcome, one variant per refusal-table row (ADR-013
/// O-17, FR-096).
#[derive(Debug)]
pub enum CallRefusal {
    /// FR-096 builds a `RefusalRecord` for this refusal, kernel or family
    /// (`kernel_refusal_record`/`CatalogCoded::refusal_record`).
    Record {
        /// The record's catalog code and cause.
        code: CatalogCode,
        /// The record's catalog fields, keyed by the catalog's payload
        /// names.
        fields: BTreeMap<&'static str, String>,
        /// The record's locus, resolved over a supplied source, when it
        /// carries one.
        locus: Option<CallLocus>,
        /// Where the outcome arose, when known.
        location: Option<Location>,
    },
    /// A family cause with no FR-096 key-table row: its catalog code and
    /// cause, no fields.
    Family {
        /// The cause's catalog code and cause.
        code: CatalogCode,
        /// Where the outcome arose, when known.
        location: Option<Location>,
    },
}

impl CallRefusal {
    /// The ADR-013 O-16 category of the record's or cause's typed code
    /// (FR-100): refusal, unsupported for a profile-gated construct, or
    /// incomplete for an exhausted resource. A code naming no native `Code`
    /// is a refusal.
    pub fn category(&self) -> Category {
        let (Self::Record { code, .. } | Self::Family { code, .. }) = self;
        Code::from_code(code.code()).map_or(Category::Refusal, Code::category)
    }
}

/// FR-100's outcome mapping (ADR-013 O-16), for every category
/// `fsl_replay::spine::run` returns as `Ok`. `CheckedInvariant` and a broken
/// S6a invariant are never carried here (see [`RunRefusal::Fault`]).
#[derive(Debug)]
pub enum CallOutcome {
    /// `Outcome::Completed(v)`: `completed`, exit 0.
    Completed(CallValue),
    /// `refused`, one of the refusal table's three rows, exit by the
    /// mapping's own rule.
    Refused(CallRefusal),
    /// `Outcome::Undefined(u)` (kebab case) or `FamilyResult::Undefined(u)`
    /// (the catalog's own `UndefinedReason` spelling): `undefined`, exit 10.
    Undefined {
        /// The reason's tabled spelling.
        reason: &'static str,
    },
    /// `Outcome::Incomplete(i)`: `incomplete`, exit 22.
    Incomplete(CallIncomplete),
}

/// FR-277's accounting-limit outcome for `execute`: the counter that could
/// not take its charge, with everything the caller needs to raise it.
#[derive(Debug)]
pub struct CallIncomplete {
    /// The kernel's incomplete record: the limit kind, the configured
    /// value, the counter before the denied charge, the denied amount and
    /// the charge point.
    pub record: Incomplete,
    /// Where the outcome arose, when the evaluation knows.
    pub location: Option<Location>,
}

impl CallIncomplete {
    /// The `Call::accounting` field that raises this bound: the exhausted
    /// counter's `filament.value.accounting/v1` member name, which is no setting
    /// name (see [`Self::setting`]).
    pub fn limits_field(&self) -> &'static str {
        self.record.limit_kind.as_str()
    }

    /// The setting that raises this bound at every entry point (FR-255
    /// Behavior 9): the exhausted counter's.
    pub fn setting(&self) -> fsl_foundation::AccountingSetting {
        crate::limits::AccountingLimits::setting_of(self.record.limit_kind)
    }

    /// The setting name, including `evaluation.live_occurrences`, which is
    /// configured outside the ten scalar accounting settings.
    #[must_use]
    pub fn setting_name(&self) -> &'static str {
        crate::limits::AccountingLimits::setting_name(self.record.limit_kind)
    }

    /// The counter at the failed charge: what the counter would have
    /// reached had the charge been admitted.
    pub fn counter(&self) -> Integer {
        let consumed = Integer::from(self.record.consumed);
        if self.record.limit_kind.is_cumulative() {
            consumed.add(&self.record.next_charge)
        } else {
            self.record.next_charge.clone()
        }
    }
}

impl CallOutcome {
    /// The outcome's ADR-013 O-16 category as `run` reports it (FR-100): a
    /// completed call is a success whatever value it completes. A clause
    /// run reads a `false` completion as a violation itself.
    pub fn category(&self) -> Category {
        match self {
            Self::Completed(_) => Category::Success,
            Self::Refused(refusal) => refusal.category(),
            Self::Undefined { .. } => Category::Undefined,
            Self::Incomplete(_) => Category::Incomplete,
        }
    }
}

/// Why [`run`] returned no outcome: a spine compile refusal, every
/// stage-`call` refusal FR-100 names, or an internal failure (a broken S6a
/// invariant, the kernel `CheckedInvariant` refusal, or a record's locus
/// naming no supplied source).
#[derive(Debug, thiserror::Error)]
pub enum RunRefusal {
    /// The spine compile refused, at its own stage and cause code.
    #[error("{0}")]
    Compile(Box<CompileRefusal>),
    /// `function` is empty, holds an empty segment or a non-identifier
    /// segment, has more than one segment, or names no function of the
    /// compiled package (`missing_declaration`).
    #[error("no function named {function} is declared")]
    MissingDeclaration {
        /// The request's `function` string.
        function: String,
    },
    /// The named function's declared result type has no run value form
    /// (`unknown_required_feature`, FR-650).
    #[error("the result of the function {function} has no run value form")]
    UnsupportedResult {
        /// The request's `function` string.
        function: String,
    },
    /// An argument names no parameter of the called function
    /// (`invalid_runtime_input`).
    #[error("no parameter is named {parameter}")]
    UnknownParameter {
        /// The argument's `parameter` string.
        parameter: String,
    },
    /// Two arguments name the same parameter (`invalid_runtime_input`).
    #[error("the parameter {parameter} is bound twice")]
    DuplicateArgument {
        /// The parameter name bound twice.
        parameter: String,
    },
    /// A parameter of the called function has no argument
    /// (`invalid_runtime_input`).
    #[error("the parameter {parameter} has no argument")]
    UnboundParameter {
        /// The unbound parameter's declared name.
        parameter: String,
    },
    /// An argument is not a value of its parameter's declared type, with the
    /// cause and the JSON pointer FR-650 names.
    #[error("the argument at position {position} is refused: {} at {:?}", .defect.cause, .defect.path)]
    Argument {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The defect.
        defect: ArgumentDefect,
    },
    /// A parameter's declared type has no run value form
    /// (`unknown_required_feature`, FR-650).
    #[error("the parameter at position {position} has no run value form")]
    UnsupportedParameter {
        /// The parameter's zero-based declared position.
        position: usize,
    },
    /// An argument's value is not of its parameter's declared type, found
    /// either before the call or by S6a admission (`invalid_runtime_input`).
    #[error("the argument at position {position} is not a value of its parameter's declared type")]
    WrongValueKind {
        /// The parameter's zero-based declared position.
        position: usize,
    },
    /// A model-aware argument admission refusal retained from the owner API.
    #[error("the argument at position {position} was refused: {refusal:?}")]
    Admission {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The owning model refusal.
        refusal: ModelRefusal,
    },
    /// An argument's membership in its parameter's declared type was not
    /// decided because the type environment could not reserve storage
    /// (`resource_exhausted`): never a wrong-value-kind refusal.
    #[error("the argument at position {position} was not decided: {failure:?}")]
    Storage {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The kernel's storage failure.
        failure: EnvironmentFailure,
    },
    /// The reference is absent from the complete admitted object world.
    #[error("the argument at position {position} names no admitted object")]
    DanglingReference {
        /// The parameter's zero-based declared position.
        position: usize,
    },
    /// An internal failure at S6a (FR-100 "Internal failure at S6a"): the
    /// kernel `CheckedInvariant` refusal, `CallFailure::Fault`, or a
    /// record's locus naming no supplied source. Category internal failure,
    /// exit 30.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
    /// A record's locus names a source that the caller's `sources` does not
    /// hold, or whose bytes do not hold the region's span on UTF-8
    /// boundaries (FR-283 "Outputs"). A run over sources the spine compiled
    /// never reaches it; `monitor` takes `sources` as input and refuses with
    /// it.
    #[error("the source {reference:?} is not supplied for the region it is cited at")]
    SourceNotSupplied {
        /// The cited source's identity labels.
        reference: SourceIdentity,
    },
    /// The caller's [`Cancel`] stopped the run at one of its stages or at
    /// S6a (ADR-029 LC-3): category incomplete, exit 22, no call outcome.
    #[error("the run was cancelled: {0:?}")]
    Cancelled(CancelCause),
    /// FR-277: the supplied-value admission phase stopped before the call, at
    /// a helper ceiling, on invalid input, on the original cancellation, on
    /// a failed reservation or on an unrepresentable size, with the phase
    /// context. No evaluation ran: zero meter consumption, no `FunctionCall`
    /// charge and no partial value. Not a declaration environment failure.
    #[error("supplied-value {phase} stopped before the call: {cause:?}", phase = .0.context.phase.as_str(), cause = .0.cause)]
    Supplied(Box<filament_semantic_value::supplied_stop::SuppliedStop>),
    /// FR-277: the admission walk reached a helper family FSV has not
    /// qualified, before that helper ran. Distinct from every other
    /// refusal: never invalid input, never a limit.
    #[error("supplied-value {phase} reached the unqualified {family:?} helper", phase = .0.context.phase.as_str(), family = .0.family)]
    UnsupportedHelper(Box<filament_semantic_value::supplied_stop::UnsupportedHelperStop>),
    /// FR-049-AC-2: the call's entry refused the caller's object environment
    /// against the package's declarations (an object of an undeclared type,
    /// or a missing, surplus or ill-typed attribute), before any charge. The
    /// record names the code, cause, object and attribute.
    #[error("the object environment is refused: {0:?}")]
    ObjectEnvironment(Box<filament_semantic_value::admission::AdmissionRecord>),
    /// FR-296, FR-294: the chosen backend is not supplied, or did not
    /// prepare the entry (category unsupported, `unimplemented_capability`).
    #[error(transparent)]
    Unsupported(filament_eval::UnsupportedEntry),
    /// FR-279, FR-294: `prepare` reached a `PrepareLimits` bound (category
    /// incomplete, `stage_limit_exceeded`).
    #[error(transparent)]
    PrepareLimit(filament_eval::PrepareLimitReached),
}

impl RunRefusal {
    /// The stage FR-100's refusal table names: the wrapped compile
    /// refusal's own stage, or `call` for every refusal this module raises
    /// itself, including [`Self::Fault`] (the internal-failure envelope's
    /// own stage is `call`, FR-100). A cancel has no stage (FR-286: its
    /// `last_stage` is null whichever stage it stopped), so this is `None`
    /// for [`Self::Cancelled`].
    pub fn stage(&self) -> Option<&'static str> {
        Some(match self {
            Self::Compile(refusal) => refusal.stage().as_str(),
            Self::MissingDeclaration { .. }
            | Self::UnsupportedResult { .. }
            | Self::UnknownParameter { .. }
            | Self::DuplicateArgument { .. }
            | Self::UnboundParameter { .. }
            | Self::Argument { .. }
            | Self::UnsupportedParameter { .. }
            | Self::WrongValueKind { .. }
            | Self::Admission { .. }
            | Self::Storage { .. }
            | Self::DanglingReference { .. }
            | Self::Supplied(_)
            | Self::UnsupportedHelper(_)
            | Self::ObjectEnvironment(_)
            | Self::Unsupported(_)
            | Self::PrepareLimit(_)
            | Self::SourceNotSupplied { .. }
            | Self::Fault(_) => "call",
            Self::Cancelled(_) => return None,
        })
    }

    /// The original cancellation cause, whether the handle stopped a stage,
    /// the call, or a supplied-value phase.
    pub fn cancel_cause(&self) -> Option<CancelCause> {
        match self {
            Self::Cancelled(cause) => Some(*cause),
            Self::Supplied(stop) => match stop.cause {
                filament_semantic_value::supplied_stop::SuppliedCause::Cancelled(cause) => {
                    Some(cause)
                }
                filament_semantic_value::supplied_stop::SuppliedCause::Limit(_)
                | filament_semantic_value::supplied_stop::SuppliedCause::Input { .. }
                | filament_semantic_value::supplied_stop::SuppliedCause::Storage(_)
                | filament_semantic_value::supplied_stop::SuppliedCause::Capacity(_) => None,
            },
            Self::Compile(_)
            | Self::MissingDeclaration { .. }
            | Self::UnsupportedResult { .. }
            | Self::UnknownParameter { .. }
            | Self::DuplicateArgument { .. }
            | Self::UnboundParameter { .. }
            | Self::Argument { .. }
            | Self::UnsupportedParameter { .. }
            | Self::WrongValueKind { .. }
            | Self::Admission { .. }
            | Self::Storage { .. }
            | Self::DanglingReference { .. }
            | Self::UnsupportedHelper(_)
            | Self::ObjectEnvironment(_)
            | Self::Unsupported(_)
            | Self::PrepareLimit(_)
            | Self::SourceNotSupplied { .. }
            | Self::Fault(_) => None,
        }
    }

    /// The catalog code. For [`Self::Fault`] this is always
    /// `runtime_invariant`.
    pub fn code(&self) -> Code {
        match self {
            Self::Compile(refusal) => refusal.code(),
            Self::MissingDeclaration { .. } => Code::MissingDeclaration,
            Self::UnsupportedResult { .. } | Self::UnsupportedParameter { .. } => {
                Code::UnknownRequiredFeature
            }
            Self::Argument { defect, .. } => defect.code,
            Self::UnknownParameter { .. }
            | Self::DuplicateArgument { .. }
            | Self::UnboundParameter { .. }
            | Self::WrongValueKind { .. } => Code::InvalidRuntimeInput,
            Self::Admission { refusal, .. } => refusal.code,
            Self::Storage { failure, .. } => membership_failure_code(failure),
            Self::DanglingReference { .. } => Code::DanglingReference,
            Self::SourceNotSupplied { .. } => Code::InvalidRuntimeInput,
            Self::Fault(_) => Code::RuntimeInvariant,
            Self::Cancelled(_) => Code::Cancelled,
            Self::Supplied(stop) => fsl_eval::value::supplied_stop_code(stop),
            Self::UnsupportedHelper(_) => Code::UnsupportedConstruct,
            Self::Unsupported(_) => Code::UnimplementedCapability,
            Self::PrepareLimit(_) => Code::StageLimitExceeded,
            Self::ObjectEnvironment(record) => {
                Code::from_code(record.catalog.code()).unwrap_or(Code::InvalidRuntimeInput)
            }
        }
    }

    /// FR-285: the refusal's ADR-013 O-16 category, read from its code;
    /// [`Self::Fault`] is an internal failure.
    pub fn category(&self) -> Category {
        self.code().category()
    }

    /// FR-286 "Diagnostic details" and FR-100's refusal table: the one closed
    /// object that locates this refusal inside the request and the call, or
    /// `None` for a refusal whose row has none. The command-error envelope and
    /// the outcome diagnostic carry this same object.
    #[must_use]
    pub fn details(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        use serde_json::{json, Value};
        let object = |value: Value| match value {
            Value::Object(members) => Some(members),
            _ => None,
        };
        match self {
            Self::MissingDeclaration { function } | Self::UnsupportedResult { function } => {
                object(json!({"function": function}))
            }
            Self::UnknownParameter { parameter }
            | Self::DuplicateArgument { parameter }
            | Self::UnboundParameter { parameter } => object(json!({"parameter": parameter})),
            Self::Argument { position, defect } => {
                object(json!({"position": position, "path": defect.path}))
            }
            Self::UnsupportedParameter { position }
            | Self::WrongValueKind { position }
            | Self::DanglingReference { position }
            | Self::Storage { position, .. } => object(json!({"position": position})),
            Self::Admission { position, refusal } => match &refusal.cause {
                ModelRefusalCause::ForeignUniverse { actual, expected } => object(json!({
                    "position": position,
                    "required": super::node_key_form(expected.root_type()),
                    "supplied": super::node_key_form(actual),
                })),
                ModelRefusalCause::ForeignModelSelection { actual, expected } => {
                    let expected = json!({"identity": expected.identity});
                    match actual {
                        OfferedSelection::Document(identity) => object(json!({
                            "position": position,
                            "model_identity": identity,
                            "expected": expected,
                        })),
                        OfferedSelection::Population(population) => object(json!({
                            "position": position,
                            "population": {"package": population.package, "node": population.node},
                            "expected": expected,
                        })),
                    }
                }
                ModelRefusalCause::ForeignType { member, type_name } => object(json!({
                    "position": position,
                    "member": member,
                    "type": {"package": type_name.package, "node": type_name.node},
                })),
                _ => object(json!({"position": position})),
            },
            Self::Supplied(stop) => stop
                .context
                .parameter
                .and_then(|position| object(json!({"position": position}))),
            Self::UnsupportedHelper(stop) => stop
                .context
                .parameter
                .and_then(|position| object(json!({"position": position}))),
            Self::Fault(fault) => object(json!({
                "stage": fault.stage(),
                "invariant": fault.invariant(),
            })),
            Self::Compile(_)
            | Self::SourceNotSupplied { .. }
            | Self::ObjectEnvironment(_)
            | Self::Cancelled(_)
            | Self::Unsupported(_)
            | Self::PrepareLimit(_) => None,
        }
    }
}

/// Compile `bytes` through the spine and call `call`'s named function with
/// its arguments, under `call`'s accounting limits. Returns the call's
/// outcome (FR-100).
///
/// `cancel` is the caller's handle (ADR-029: every operation takes one): a
/// cancel stops the run at its next charge, whichever stage it is in, as
/// [`RunRefusal::Cancelled`].
#[allow(
    clippy::too_many_arguments,
    reason = "the request's fields are the signature FR-100 names; the caller's object context and cancel handle are explicit"
)]
pub fn run(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    run_with_optional_profile_catalog(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        None,
        None,
        cancel,
    )
}

/// Compile and run `bytes` with request-supplied extension definitions.
#[allow(
    clippy::too_many_arguments,
    reason = "FR-100's run inputs plus the request-owned profile catalog are explicit"
)]
pub fn run_with_profile_catalog(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    profile_catalog: ProfileCatalogInput<'_>,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    run_with_optional_profile_catalog(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        Some(profile_catalog),
        None,
        cancel,
    )
}

/// Compile and run `bytes` with a caller-selected integer-division law and
/// request-supplied extension definitions (FR-767).
#[allow(
    clippy::too_many_arguments,
    reason = "FR-100 run inputs plus the request-owned catalog and division selection"
)]
pub fn run_with_profile_catalog_and_integer_division(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    profile_catalog: ProfileCatalogInput<'_>,
    integer_division: crate::AdmittedIntegerDivision,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    run_with_optional_profile_catalog(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        Some(profile_catalog),
        Some(integer_division),
        cancel,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the run inputs and optional request-owned catalog are explicit"
)]
fn run_with_optional_profile_catalog(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    profile_catalog: Option<ProfileCatalogInput<'_>>,
    integer_division: Option<crate::AdmittedIntegerDivision>,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    run_reading_with_integer_division(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        &SuppliedBackends::default(),
        &PrepareLimits::default(),
        profile_catalog,
        integer_division,
        cancel,
        |_, _, _| Ok(()),
    )
    .map(|(outcome, ())| outcome)
}

/// [`run`] over one compile, and, once the call has run, `read` over the
/// compile's checked unit, emitted unit and admitted models (FR-395 "One
/// checked graph", FR-404 "One compile"): the value `read` returns comes from
/// the same compile as the call. A refusal before the call, including a cancel, returns
/// before `read` runs.
#[allow(
    clippy::too_many_arguments,
    reason = "run's signature (FR-100) plus the one read of the compile's units"
)]
pub(crate) fn run_reading<T>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    backends: &SuppliedBackends<'_>,
    prepare: &PrepareLimits,
    profile_catalog: Option<ProfileCatalogInput<'_>>,
    cancel: &Cancel,
    read: impl FnOnce(&CheckedUnit, &EmittedUnit, &AdmittedModels) -> Result<T, Box<RunRefusal>>,
) -> Result<(CallOutcome, T), Box<RunRefusal>> {
    run_reading_with_integer_division(
        source,
        package_name,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        call,
        objects,
        backends,
        prepare,
        profile_catalog,
        None,
        cancel,
        read,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "run's explicit source, package, evaluation, backend, catalog and law inputs"
)]
pub(crate) fn run_reading_with_integer_division<T>(
    source: SourceIdentity,
    package_name: String,
    path: &str,
    bytes: &[u8],
    packages: &fsl_semantics::model::intake::PackageInput,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
    objects: &ObjectEnvironment,
    backends: &SuppliedBackends<'_>,
    prepare: &PrepareLimits,
    profile_catalog: Option<ProfileCatalogInput<'_>>,
    integer_division: Option<crate::AdmittedIntegerDivision>,
    cancel: &Cancel,
    read: impl FnOnce(&CheckedUnit, &EmittedUnit, &AdmittedModels) -> Result<T, Box<RunRefusal>>,
) -> Result<(CallOutcome, T), Box<RunRefusal>> {
    // FR-296: a backend the caller did not supply refuses before any stage.
    backends
        .member(call.backend)
        .map_err(|entry| Box::new(RunRefusal::Unsupported(entry)))?;
    let refusal = |failure| match failure {
        StageFailure::Cancelled(cause) => Box::new(RunRefusal::Cancelled(cause)),
        other => match front_end::refusal_or_fault(other) {
            Ok(refusal) => Box::new(RunRefusal::Compile(refusal)),
            Err(fault) => Box::new(RunRefusal::Fault(fault)),
        },
    };
    let own_package = package_name.clone();
    let parse_request = front_end::ParseRequest {
        source: &source,
        path,
        bytes,
    };
    let parsed = match profile_catalog {
        Some(catalog) => {
            front_end::parse_selecting_extensions(&parse_request, catalog, limits.source, cancel)
        }
        None => front_end::parse(&parse_request, limits.source, cancel),
    }
    .map_err(refusal)?
    .into_value();
    let models = front_end::select(&parsed, packages, limits.model, cancel)
        .map_err(refusal)?
        .into_value();
    let check_selections = call.clocks.check_selections(&parsed);
    let checked = front_end::check_with_refinement_internals_and_integer_division(
        &parsed,
        package_name,
        super::SourceInputs {
            refinement_internals: None,
            temporal_selections: &check_selections,
            clocks: Some(&call.clocks),
        },
        &models,
        dependencies,
        integer_division,
        limits,
        cancel,
    )
    .map_err(refusal)?
    .into_value();
    let selections = call
        .clocks
        .package_selections(&checked, &[&parsed])
        .map_err(|compile| Box::new(RunRefusal::Compile(compile)))?;
    let emitted =
        front_end::package_with_temporal_selections(&checked, &selections, limits.package, cancel)
            .map_err(refusal)?
            .into_value();
    let outcome = run_compiled(
        &checked,
        &models,
        CallSources {
            package_name: &own_package,
            packages,
            model_limits: limits.model,
        },
        call,
        objects,
        backends,
        prepare,
        cancel,
    )?;
    let read = read(&checked, &emitted, &models)?;
    Ok((outcome, read))
}

/// The compile inputs a call over an already compiled program still reads
/// (FR-650): the package it was compiled as, and the model packages and
/// limits its call snapshot and population universes are admitted against.
#[derive(Clone, Copy)]
pub(crate) struct CallSources<'a> {
    pub(crate) package_name: &'a str,
    pub(crate) packages: &'a fsl_semantics::model::intake::PackageInput,
    pub(crate) model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
}

/// Run a call against one already compiled program. The call gets fresh
/// argument admission and evaluation accounting while reusing the checked
/// package, emitted package and admitted model set.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_compiled(
    checked: &super::CheckedUnit,
    models: &super::AdmittedModels,
    sources: CallSources<'_>,
    call: &Call,
    objects: &ObjectEnvironment,
    backends: &SuppliedBackends<'_>,
    prepare: &PrepareLimits,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    backends
        .member(call.backend)
        .map_err(|entry| Box::new(RunRefusal::Unsupported(entry)))?;
    let package = checked.package();
    let CallSources {
        package_name: own_package,
        packages,
        model_limits,
    } = sources;
    let selected = select(package, &call.function)?;
    let admitted_world;
    let objects = match &call.snapshot {
        Some(snapshot) => {
            admitted_world = admit_call_snapshot(package, packages, model_limits, snapshot)?;
            &admitted_world
        }
        None => objects,
    };
    let bound_world;
    let objects = if call.snapshot.is_some() {
        bound_world =
            bind_snapshot_populations(objects, models.models(), selected.parameters, call)?;
        &bound_world
    } else {
        objects
    };
    let universe_of = |declared: &filament_exact::EffectiveId| {
        fsl_semantics::model::observation::population_universe_for(
            package.graph().model_inputs(),
            packages,
            model_limits,
            declared.clone(),
        )
        .ok()
    };
    let world = call.snapshot.as_ref().map(|_| super::run_value::World {
        objects,
        universe_of: &universe_of,
    });
    let forms = Forms::new(
        package.graph().scope().types(),
        package.graph().scope().enums(),
    )
    .in_world(own_package, world.as_ref());
    let supplied = admit_wire_arguments(&forms, selected.parameters, call)?;
    let arguments = bind_arguments(selected.parameters, &supplied)?;
    admit_model_arguments(
        models.models(),
        package.graph().scope().types(),
        selected.parameters,
        &arguments,
        objects,
    )?;
    let evaluation = execute(
        package,
        &ExecuteRequest {
            selection: ExecuteSelection::Function {
                name: selected.name.clone(),
                arguments,
                environment: objects.clone(),
            },
            backend: call.backend,
        },
        backends,
        &ExecuteLimits {
            accounting: call.accounting,
            supplied: call.supplied,
            prepare: *prepare,
        },
        cancel,
    )
    .map_err(|failure| match failure {
        filament_eval::CallFailure::Cancelled(cause) => Box::new(RunRefusal::Cancelled(cause)),
        other => convert_call_failure(other),
    })?;
    // FND-016: the unit's and every resolved library's source, reused
    // exactly as the check already read them (FR-100: a locus is resolved
    // over "the `Source` whose reference equals the region's reference").
    // Never re-read: a completed call must never turn into a fault because
    // of I/O that a second read of already-admitted bytes could raise.
    let mut sources = Vec::with_capacity(1 + checked.libraries().len());
    sources.push(checked.source().clone());
    sources.extend(checked.libraries().iter().cloned());
    let outcome = match &evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value))
            if !matches!(
                value,
                Value::Boolean(_) | Value::Integer(_) | Value::Reference(_)
            ) =>
        {
            match forms.write(value) {
                Ok(form) => CallOutcome::Completed(CallValue::Written(form)),
                Err(super::run_value::WriteStop::NoForm) => {
                    return Err(Box::new(RunRefusal::UnsupportedResult {
                        function: call.function.clone(),
                    }));
                }
                Err(super::run_value::WriteStop::Fault(invariant)) => {
                    return Err(Box::new(RunRefusal::Fault(InternalFault::new(
                        "call", invariant,
                    ))));
                }
            }
        }
        _ => convert_outcome(evaluation, package.graph(), &sources)?,
    };
    Ok(outcome)
}

/// FR-650 "Population argument": FR-084 admission binds each population the
/// call names for a `Population<T>[N]` parameter, with the parameter's
/// declared maximum, and FR-089 records its identity in the object context.
fn bind_snapshot_populations(
    objects: &ObjectEnvironment,
    models: &[fsl_semantics::model::intake::SelectedModel],
    parameters: &[(String, ValueType)],
    call: &Call,
) -> Result<ObjectEnvironment, Box<RunRefusal>> {
    use fsl_semantics::model::dispatch::GeneralizationClosure;
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    use fsl_semantics::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    let mut environment = objects.clone();
    for (position, (name, value_type)) in parameters.iter().enumerate() {
        let ValueType::Population(maximum) = value_type else {
            continue;
        };
        let Some(named) = call
            .wire_arguments
            .iter()
            .find(|argument| argument.parameter == *name)
            .and_then(|argument| argument.value.get("population"))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(members) = objects.population_members(named) else {
            continue;
        };
        let wrong = || Box::new(RunRefusal::WrongValueKind { position });
        let Some((model, key)) = models.iter().find_map(|model| {
            model
                .view
                .domain_package()
                .records
                .iter()
                .find_map(|record| match record {
                    DomainPackageRecord::Population(population) if population.key.node == named => {
                        Some((model, population.key.clone()))
                    }
                    _ => None,
                })
        }) else {
            continue;
        };
        let documents = members
            .iter()
            .map(|member| {
                model
                    .view
                    .type_identities()
                    .iter()
                    .find(|(_, identity)| *identity == member.object_type())
                    .map(|(declaration, _)| PopulationMember {
                        object: member.object().as_str().to_owned(),
                        type_identity: declaration.clone(),
                        field_values: Vec::new(),
                    })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(wrong)?;
        let document = PopulationDocument {
            model_identity: model.view.model_selection().identity.clone(),
            members: documents,
        };
        let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::default());
        let AdmissionOutcome::Admitted(binding) = admit_binding(
            &model.view,
            &document,
            &key,
            GeneralizationClosure::Closed,
            *maximum,
            &mut meter,
        ) else {
            return Err(wrong());
        };
        environment = environment.with_population(binding).map_err(|_| wrong())?;
    }
    Ok(environment)
}

/// FR-650 "Arguments": the object world of the call's snapshot, admitted
/// as FR-109's `Function` selection admits its snapshot.
fn admit_call_snapshot(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    model_limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
    snapshot: &CallSnapshot,
) -> Result<ObjectEnvironment, Box<RunRefusal>> {
    use fsl_semantics::model::observation::AdmissionFailure;
    let provision = BTreeMap::from([(
        snapshot.observation.reference.clone(),
        snapshot.observation.bytes.clone(),
    )]);
    match fsl_semantics::model::observation::admit_current_snapshot(
        package.graph().model_inputs(),
        package.graph().scope().types(),
        packages,
        model_limits,
        &provision,
        &snapshot.observation.reference,
        snapshot.limits,
    ) {
        Ok((environment, _)) => Ok(environment),
        Err(AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record)) => {
            Err(Box::new(RunRefusal::ObjectEnvironment(record)))
        }
        Err(AdmissionFailure::Fault(fault)) => Err(Box::new(RunRefusal::Fault(fault))),
    }
}

/// The selected function: its S6a name and declared parameters.
pub(crate) struct Selected<'a> {
    pub(crate) name: fsl_eval::value::QualifiedName,
    pub(crate) parameters: &'a [(String, ValueType)],
}

/// Resolve `function` by one-segment name lookup in `package`'s
/// declarations (OQ-5, as `fsl_replay::replay` does), refusing shapes FR-100
/// names before ever looking one up.
pub(crate) fn select<'a>(
    package: &'a CheckedPackage,
    function: &str,
) -> Result<Selected<'a>, Box<RunRefusal>> {
    let missing = || {
        Box::new(RunRefusal::MissingDeclaration {
            function: function.to_owned(),
        })
    };
    let mut segments = function.split('.');
    let Some(first) = segments.next() else {
        return Err(missing());
    };
    if segments.next().is_some() {
        return Err(missing());
    }
    if first.is_empty() || !is_identifier(first) {
        return Err(missing());
    }
    let callable = package.graph().callable(first).ok_or_else(missing)?;
    let graph_scope = package.graph().scope();
    let result_has_form =
        Forms::new(graph_scope.types(), graph_scope.enums()).has_form(callable.result);
    if !result_has_form {
        return Err(Box::new(RunRefusal::UnsupportedResult {
            function: function.to_owned(),
        }));
    }
    let name = fsl_eval::value::QualifiedName::unqualified(first).map_err(|_| missing())?;
    Ok(Selected {
        name,
        parameters: callable.parameters,
    })
}

/// An ASCII identifier: a letter or `_`, then letters, digits or `_`.
pub(crate) fn is_identifier(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    match bytes.next() {
        Some(byte) if byte.is_ascii_alphabetic() || byte == b'_' => {}
        _ => return false,
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Read each request-written argument (FR-650) as the value of its
/// parameter's declared type, after the binding-name checks of FR-100 and in
/// declared parameter order, stopping at the first defect. Returns every
/// argument as a typed one.
pub(crate) fn admit_wire_arguments(
    forms: &Forms<'_>,
    parameters: &[(String, ValueType)],
    call: &Call,
) -> Result<Vec<CallArgument>, Box<RunRefusal>> {
    if call.wire_arguments.is_empty() {
        return Ok(call.arguments.clone());
    }
    let mut bound = vec![false; parameters.len()];
    let names = call
        .arguments
        .iter()
        .map(|argument| &argument.parameter)
        .chain(
            call.wire_arguments
                .iter()
                .map(|argument| &argument.parameter),
        );
    for name in names {
        let position = parameters
            .iter()
            .position(|(parameter, _)| parameter == name)
            .ok_or_else(|| {
                Box::new(RunRefusal::UnknownParameter {
                    parameter: name.clone(),
                })
            })?;
        if std::mem::replace(&mut bound[position], true) {
            return Err(Box::new(RunRefusal::DuplicateArgument {
                parameter: name.clone(),
            }));
        }
    }
    if let Some((_, (name, _))) = bound.iter().zip(parameters).find(|(bound, _)| !**bound) {
        return Err(Box::new(RunRefusal::UnboundParameter {
            parameter: name.clone(),
        }));
    }
    let mut supplied = call.arguments.clone();
    for (position, (name, value_type)) in parameters.iter().enumerate() {
        let Some(wire) = call
            .wire_arguments
            .iter()
            .find(|argument| argument.parameter == *name)
        else {
            continue;
        };
        // A parameter with no run value form refuses whatever the value is.
        if !forms.has_form(value_type) {
            return Err(Box::new(RunRefusal::UnsupportedParameter { position }));
        }
        let read = match &wire.value {
            serde_json::Value::Number(number) => match number.as_i64() {
                Some(number) => forms.bare_integer(number, value_type),
                None => Err(ReadStop::Defect(ArgumentDefect {
                    code: Code::InvalidRuntimeInput,
                    cause: "wrong-value-kind",
                    path: String::new(),
                })),
            },
            form => forms.read(form, value_type),
        };
        let value = read.map_err(|stop| {
            Box::new(match stop {
                ReadStop::Defect(defect) => RunRefusal::Argument { position, defect },
                ReadStop::Fault(invariant) => {
                    RunRefusal::Fault(InternalFault::new("call", invariant))
                }
            })
        })?;
        supplied.push(CallArgument {
            parameter: name.clone(),
            value,
        });
    }
    Ok(supplied)
}

/// Join each argument to the parameter of the same declared name (refusing
/// an unknown or twice-bound parameter, and an unbound one, before any value
/// is converted -- FND-010: binding order is checked in full before any
/// value's kind is), then convert each bound value to its parameter's
/// declared type, ordered by declared parameter position.
pub(crate) fn bind_arguments(
    parameters: &[(String, ValueType)],
    arguments: &[CallArgument],
) -> Result<Vec<Value>, Box<RunRefusal>> {
    let mut slots: Vec<Option<Value>> = vec![None; parameters.len()];
    for argument in arguments {
        let position = parameters
            .iter()
            .position(|(name, _)| *name == argument.parameter)
            .ok_or_else(|| {
                Box::new(RunRefusal::UnknownParameter {
                    parameter: argument.parameter.clone(),
                })
            })?;
        let slot = slots.get_mut(position).ok_or_else(|| {
            Box::new(RunRefusal::UnknownParameter {
                parameter: argument.parameter.clone(),
            })
        })?;
        if slot.replace(argument.value.clone()).is_some() {
            return Err(Box::new(RunRefusal::DuplicateArgument {
                parameter: argument.parameter.clone(),
            }));
        }
    }
    // Every parameter is bound before any is converted: an unbound
    // parameter at a later position refuses before a wrong-kind value at an
    // earlier one is ever inspected (FND-010).
    let bound: Vec<Value> = slots
        .into_iter()
        .zip(parameters)
        .map(|(value, (name, _))| {
            value.ok_or_else(|| {
                Box::new(RunRefusal::UnboundParameter {
                    parameter: name.clone(),
                })
            })
        })
        .collect::<Result<_, _>>()?;
    bound
        .into_iter()
        .zip(parameters)
        .enumerate()
        .map(|(position, (value, (_, value_type)))| argument_value(position, value, value_type))
        .collect()
}

/// Convert an input value to the parameter's declared type. Integer inputs
/// retain FR-098's `0`/`1` Boolean shorthand; typed model values already
/// carry their admission identity and are passed to S6a for model-aware
/// validation.
pub(crate) fn argument_value(
    position: usize,
    value: Value,
    value_type: &ValueType,
) -> Result<Value, Box<RunRefusal>> {
    let wrong = || Box::new(RunRefusal::WrongValueKind { position });
    match value_type {
        ValueType::Boolean => match value {
            Value::Boolean(value) => Ok(Value::Boolean(value)),
            Value::Integer(value) if value == Integer::from(0_i64) => Ok(Value::Boolean(false)),
            Value::Integer(value) if value == Integer::from(1_i64) => Ok(Value::Boolean(true)),
            _ => Err(wrong()),
        },
        ValueType::Integer | ValueType::Int(_) => match value {
            Value::Integer(value) => Ok(Value::Integer(value)),
            _ => Err(wrong()),
        },
        ValueType::String => match value {
            Value::String(value) => Ok(Value::String(value)),
            _ => Err(wrong()),
        },
        ValueType::Bytes => match value {
            Value::Bytes(value) => Ok(Value::Bytes(value)),
            _ => Err(wrong()),
        },
        ValueType::JsonObject => match value {
            Value::JsonObject(value) => Ok(Value::JsonObject(value)),
            _ => Err(wrong()),
        },
        ValueType::Uuid => match value {
            Value::Uuid(value) => Ok(Value::Uuid(value)),
            _ => Err(wrong()),
        },
        ValueType::Timestamp => match value {
            Value::Timestamp(value) => Ok(Value::Timestamp(value)),
            _ => Err(wrong()),
        },
        ValueType::Reference(_) => match value {
            Value::Reference(value) => Ok(Value::Reference(value)),
            _ => Err(wrong()),
        },
        ValueType::Population(_) => match value {
            Value::Population(value) => Ok(Value::Population(value)),
            _ => Err(wrong()),
        },
        // A value of these types arrives admitted against its declared type
        // (FR-650) or is checked by S6a admission; only its kind is read here.
        ValueType::Rational(_) => match value {
            Value::Rational(value) => Ok(Value::Rational(value)),
            _ => Err(wrong()),
        },
        ValueType::Decimal(_) => match value {
            Value::Decimal(value) => Ok(Value::Decimal(value)),
            _ => Err(wrong()),
        },
        ValueType::Float(_) => match value {
            Value::Float(value) => Ok(Value::Float(value)),
            _ => Err(wrong()),
        },
        ValueType::Text(_) => match value {
            Value::Text(value) => Ok(Value::Text(value)),
            _ => Err(wrong()),
        },
        ValueType::Enum(_) => match value {
            Value::Enum(value) => Ok(Value::Enum(value)),
            _ => Err(wrong()),
        },
        ValueType::Option(_) => match value {
            Value::Option(value) => Ok(Value::Option(value)),
            _ => Err(wrong()),
        },
        ValueType::Composite(_) => match value {
            Value::Composite(value) => Ok(Value::Composite(value)),
            Value::Union(value) => Ok(Value::Union(value)),
            _ => Err(wrong()),
        },
        ValueType::Collection(_) => match value {
            Value::Collection(value) => Ok(Value::Collection(value)),
            _ => Err(wrong()),
        },
        ValueType::Quantity(_) | ValueType::BoundedQuantity(_) | ValueType::CarriedQuantity(_) => {
            match value {
                Value::Quantity(value) => Ok(Value::Quantity(value)),
                _ => Err(wrong()),
            }
        }
        ValueType::Map(_) => match value {
            Value::Map(value) => Ok(Value::Map(value)),
            _ => Err(wrong()),
        },
    }
}

/// Check the parts of model-reference admission that the public call facade
/// can verify from the selected model views and the supplied object closure.
/// S6a repeats the shared type and closure checks at its evaluator boundary;
/// keeping this guard here also covers unused arguments and preserves the
/// typed facade refusal arms before evaluation starts.
pub(crate) fn admit_model_arguments(
    models: &[fsl_semantics::model::intake::SelectedModel],
    types: &filament_semantic_value::declaration::TypeEnvironment,
    parameters: &[(String, ValueType)],
    arguments: &[Value],
    objects: &ObjectEnvironment,
) -> Result<(), Box<RunRefusal>> {
    for (position, ((parameter, value_type), argument)) in
        parameters.iter().zip(arguments).enumerate()
    {
        if let (ValueType::Population(maximum), Value::Population(population_id)) =
            (value_type, argument)
        {
            let Some(binding) = objects.resolve_population(population_id.clone()) else {
                return Err(Box::new(RunRefusal::WrongValueKind { position }));
            };
            if binding.declared_maximum() != *maximum {
                return Err(Box::new(RunRefusal::WrongValueKind { position }));
            }
            let Some(expected) = models.first().map(|model| model.view.model_selection()) else {
                return Err(Box::new(RunRefusal::WrongValueKind { position }));
            };
            if !models
                .iter()
                .any(|model| model.view.model_selection() == binding.model_selection())
            {
                return Err(Box::new(RunRefusal::Admission {
                    position,
                    refusal: ModelRefusal {
                        code: Code::ForeignReference,
                        cause: ModelRefusalCause::ForeignModelSelection {
                            actual: OfferedSelection::Population(binding.population_key().clone()),
                            expected: expected.clone(),
                        },
                        detail: format!(
                            "population {}/{} names model {}, expected {}",
                            binding.population_key().package,
                            binding.population_key().node,
                            binding.model_identity(),
                            expected.identity
                        ),
                    },
                }));
            }
            continue;
        }
        let (ValueType::Reference(declared), Value::Reference(reference)) = (value_type, argument)
        else {
            continue;
        };
        let expected_model = models.iter().find_map(|model| {
            model
                .view
                .type_identities()
                .iter()
                .find_map(|(key, identity)| {
                    (identity == declared)
                        .then(|| {
                            model
                                .view
                                .object_universe_of(key)
                                .and_then(|universe| universe.identity().ok())
                        })
                        .flatten()
                })
                .map(|expected| (model, expected))
        });
        let supplied_model = models.iter().find(|model| {
            model
                .view
                .type_identities()
                .values()
                .any(|identity| identity == reference.object_type())
        });
        let Some((expected_model, expected)) = expected_model else {
            let Some(supplied_model) = supplied_model else {
                return Err(Box::new(RunRefusal::WrongValueKind { position }));
            };
            let Some((type_name, _)) = supplied_model
                .view
                .type_identities()
                .iter()
                .find(|(_, identity)| *identity == reference.object_type())
            else {
                return Err(Box::new(RunRefusal::WrongValueKind { position }));
            };
            return Err(Box::new(RunRefusal::Admission {
                position,
                refusal: ModelRefusal {
                    code: Code::ForeignReference,
                    cause: ModelRefusalCause::ForeignType {
                        member: parameter.clone(),
                        type_name: type_name.clone(),
                    },
                    detail: format!(
                        "reference parameter {} names undeclared type {}/{}",
                        parameter, type_name.package, type_name.node
                    ),
                },
            }));
        };
        if let Some(supplied_model) = supplied_model {
            if supplied_model.view.model_selection() != expected_model.view.model_selection() {
                return Err(Box::new(RunRefusal::Admission {
                    position,
                    refusal: ModelRefusal {
                        code: Code::ForeignReference,
                        cause: ModelRefusalCause::ForeignModelSelection {
                            actual: OfferedSelection::Document(
                                supplied_model.view.model_selection().identity.clone(),
                            ),
                            expected: expected_model.view.model_selection().clone(),
                        },
                        detail: format!(
                            "reference names model {}, expected {}",
                            supplied_model.view.model_selection().identity,
                            expected_model.view.model_selection().identity
                        ),
                    },
                }));
            }
        }
        if *reference.universe() != expected {
            return Err(Box::new(RunRefusal::Admission {
                position,
                refusal: ModelRefusal {
                    code: Code::ForeignReference,
                    cause: ModelRefusalCause::ForeignUniverse {
                        actual: reference.universe().root_type().clone(),
                        expected: expected.clone(),
                    },
                    detail: format!(
                        "reference names universe {}, expected {}",
                        reference.universe().root_type(),
                        expected.root_type()
                    ),
                },
            }));
        }
        let admitted = types
            .admits(value_type, argument)
            .map_err(|failure| Box::new(RunRefusal::Storage { position, failure }))?;
        if !admitted {
            return Err(Box::new(RunRefusal::WrongValueKind { position }));
        }
        if objects.references_are_unresolved() || !objects.objects().contains(reference) {
            return Err(Box::new(RunRefusal::DanglingReference { position }));
        }
    }
    Ok(())
}

/// `CheckedPackage::call`'s admission failure, converted with no `fsl_eval`
/// path in the conversion's own signature (only in this function body).
pub(crate) fn convert_call_failure(failure: filament_eval::CallFailure) -> Box<RunRefusal> {
    use filament_eval::CallFailure;
    use filament_semantic_value::call::InputRefusal;
    match failure {
        CallFailure::Input(InputRefusal::WrongValueKind { parameter }) => {
            Box::new(RunRefusal::WrongValueKind {
                position: parameter,
            })
        }
        CallFailure::Input(InputRefusal::DanglingReference { parameter }) => {
            Box::new(RunRefusal::DanglingReference {
                position: parameter,
            })
        }
        // `select`/`bind_arguments` already admit the name and arity this
        // module supplies, so no other `InputRefusal` is reachable; treated
        // as an internal fault rather than papered over with a fabricated
        // catalog code.
        CallFailure::Input(_) => Box::new(RunRefusal::Fault(InternalFault::new(
            "call",
            "spine-run-supplies-admitted-name-and-arity",
        ))),
        // FR-049-AC-2: the entry refused the object environment, caller
        // input. The spine's function call path supplies a checked
        // name/argument vector and selects no protocol slot or binder map,
        // so every other admission failure is a broken invariant.
        CallFailure::Admission(failure) => {
            match fsl_eval::value::object_environment_refusal(&failure) {
                Some(record) => Box::new(RunRefusal::ObjectEnvironment(Box::new(record.clone()))),
                None => Box::new(RunRefusal::Fault(InternalFault::new(
                    "call",
                    "spine-function-call-has-no-protocol-slot-admission",
                ))),
            }
        }
        CallFailure::Fault(fault) => Box::new(RunRefusal::Fault(fault)),
        // `run` uses the checked interpreter, not a registered host body;
        // the public application facade returns body failures unchanged.
        CallFailure::Body(_) => unreachable!("spine run registers no host bodies"),
        CallFailure::Storage { parameter, failure } => Box::new(RunRefusal::Storage {
            position: parameter,
            failure,
        }),
        CallFailure::Supplied(stop) => Box::new(RunRefusal::Supplied(stop)),
        CallFailure::UnsupportedHelper(stop) => Box::new(RunRefusal::UnsupportedHelper(stop)),
        // A clause run makes its own handle and shares it with nobody, so
        // nothing cancels it: a cancellation here is a broken invariant.
        // `run` takes the caller's handle and maps a cancel itself.
        CallFailure::Cancelled(_) => Box::new(RunRefusal::Fault(InternalFault::new(
            "call",
            "cancelled-without-a-shared-handle",
        ))),
        // FR-296, FR-279: an unsupported entry or a reached prepare limit
        // reaches the outcome with its own catalog code.
        CallFailure::Unsupported(entry) => Box::new(RunRefusal::Unsupported(entry)),
        CallFailure::Limit(limit) => Box::new(RunRefusal::PrepareLimit(limit)),
    }
}

/// The kernel's own kebab-case `Undefined` spelling (FR-100).
pub(crate) fn kernel_undefined_reason(reason: Undefined) -> &'static str {
    match reason {
        Undefined::DivisionByZero => "division-by-zero",
        Undefined::IeeeNotFinite => "ieee-not-finite",
        Undefined::EmptyReduction => "empty-reduction",
        Undefined::NoneValue => "none-value",
        Undefined::NoElement => "no-element",
        Undefined::SumOutOfDomain => "sum-out-of-domain",
    }
}

/// Resolve an FR-096 record's [`Locus`] over `sources`: a region locus
/// renders as the path and span of the one supplied source whose
/// reference equals the region's (FR-100). Every other locus shape is an
/// internal failure; a region naming no supplied source, or one the matching
/// source does not hold on UTF-8 boundaries, is
/// [`RunRefusal::SourceNotSupplied`] (FR-283 "Outputs").
pub(crate) fn resolve_locus(
    locus: &Locus,
    sources: &[Source],
) -> Result<CallLocus, Box<RunRefusal>> {
    let Locus::Region(region) = locus else {
        return Err(Box::new(RunRefusal::Fault(InternalFault::new(
            "spine-run",
            "locus-source-supplied",
        ))));
    };
    let unsupplied = || {
        Box::new(RunRefusal::SourceNotSupplied {
            reference: SourceIdentity::new(region.source().authority(), region.source().identity()),
        })
    };
    let source = sources
        .iter()
        .find(|source| source.reference() == region.source())
        .ok_or_else(unsupplied)?;
    let span = source.render(region).ok_or_else(unsupplied)?;
    Ok(CallLocus {
        file: source.path().to_owned(),
        span,
    })
}

/// Build [`CallRefusal`] from a refusal's FR-096 record (when FR-096 built
/// one), its fallback catalog code (a family cause with no record), and its
/// location.
///
/// `record` is `None` with `fallback` also `None` only for a kernel refusal
/// with no catalog code; since [`convert_outcome`] never calls this for
/// `CheckedInvariant` (the only kernel refusal `kernel_refusal_record`
/// builds no record for), that combination cannot occur today. If it ever
/// did, it is an internal failure (FR-100 "Internal failure at S6a"), the
/// same typed fault [`CheckedInvariant`](Refusal::CheckedInvariant) itself
/// takes, never a panic.
fn convert_refusal(
    record: Option<RefusalRecord>,
    fallback: Option<CatalogCode>,
    location: Option<Location>,
    sources: &[Source],
) -> Result<CallRefusal, Box<RunRefusal>> {
    if let Some(record) = record {
        let locus = record
            .locus()
            .map(|locus| resolve_locus(locus, sources))
            .transpose()?;
        return Ok(CallRefusal::Record {
            code: record.code(),
            fields: record.fields().clone(),
            locus,
            location,
        });
    }
    let Some(code) = fallback else {
        return Err(Box::new(RunRefusal::Fault(InternalFault::new(
            "call",
            "kernel-refusal-with-no-record",
        ))));
    };
    Ok(CallRefusal::Family { code, location })
}

/// FR-100's outcome mapping, for every category `run` returns as `Ok`
/// (ADR-013 O-16). `CheckedInvariant` and a broken S6a invariant return
/// `Err(RunRefusal::Fault(_))` instead (FR-100 "Internal failure at S6a").
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`.
#[deny(clippy::wildcard_enum_match_arm)]
pub(crate) fn convert_outcome(
    evaluation: filament_eval::Evaluation,
    graph: &CheckedGraph,
    sources: &[Source],
) -> Result<CallOutcome, Box<RunRefusal>> {
    let record = fsl_eval::value::refusal_record(&evaluation, |location| graph.region(location));
    let filament_eval::Evaluation {
        outcome, location, ..
    } = evaluation;
    match outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => {
            Ok(CallOutcome::Completed(CallValue::Boolean(value)))
        }
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
            Ok(CallOutcome::Completed(CallValue::Integer(value)))
        }
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) => {
            Ok(CallOutcome::Completed(CallValue::Reference(reference)))
        }
        // `select` admits only a function declared `Boolean` or an integer
        // type.
        FamilyOutcome::Evaluated(Outcome::Completed(_)) => Err(Box::new(RunRefusal::Fault(
            InternalFault::new("call", "boolean-or-integer-function-completes-that-kind"),
        ))),
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant { cause })) => {
            Err(Box::new(RunRefusal::Fault(InternalFault::from_kernel(
                "S6a",
                "checked-program-invariant",
                cause,
            ))))
        }
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => Ok(CallOutcome::Refused(convert_refusal(
            record, None, location, sources,
        )?)),
        FamilyOutcome::Evaluated(Outcome::Undefined(reason)) => Ok(CallOutcome::Undefined {
            reason: kernel_undefined_reason(reason),
        }),
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            Ok(CallOutcome::Incomplete(CallIncomplete {
                record: incomplete,
                location,
            }))
        }
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
            let fallback = cause.catalog_code();
            Ok(CallOutcome::Refused(convert_refusal(
                record,
                Some(fallback),
                location,
                sources,
            )?))
        }
        FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => {
            Ok(CallOutcome::Undefined {
                reason: cause.undefined_record().reason.as_str(),
            })
        }
        // FR-063: no arm for the probe variant under `--cfg seam_probe`
        // alone -- this crate's own seam over `FamilyOutcome` (mirroring
        // `execute::replay`'s). Do not add a catch-all to make it compile.
        #[cfg(seam_probe_replay_downstream)]
        FamilyOutcome::__SeamProbe => {
            unreachable!("never constructed outside the probe build")
        }
    }
}

#[cfg(test)]
pub(in crate::spine) mod tests;

#[cfg(test)]
mod execution_tests;
