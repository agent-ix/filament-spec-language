// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-945, FR-1039: run a function of a selected checked package from its
//! bytes.
//!
//! The request selected an admitted `filament.checked-package/v2` artifact in
//! place of a program. The call resolves its function by the package's
//! `declaration.qualified_name`, reads each parameter and the result type
//! from the package's checked type nodes, admits the arguments in FR-650's
//! run value form and evaluates through the [`BarePackage`]'s
//! `CheckedPackageEvaluation`. No source is compiled and no `CheckedPackage`
//! is built; the package's `lock` supplies every law the call evaluates under,
//! the integer-division law included (FR-767).

use filament_eval::FamilyOutcome;
use filament_exact::{Cancel, Meter, Outcome, Value, ValueType};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::supplied_stop::SuppliedRun;
use fsl_eval::execution::SuppliedBackends;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::{BarePackage, BareSelectionFailure};
use fsl_semantics::model::object_environment::ObjectEnvironment;

use super::call::{
    admit_wire_arguments, bind_arguments, convert_call_failure, convert_outcome_resolving,
    is_identifier, Call, CallOutcome, CallValue, RunRefusal,
};
use super::run_value::{Forms, WriteStop};

/// Call `call`'s named function of the admitted `package` with its
/// arguments, under `call`'s accounting and supplied-value limits (FR-945).
/// Returns the outcome the same call over the package's source returns.
///
/// `cancel` is the caller's handle (ADR-029): a cancel observed before or
/// during the call is [`RunRefusal::Cancelled`].
///
/// # Errors
///
/// [`RunRefusal::MissingDeclaration`] for a `function` that is not one
/// identifier segment or that no function declaration of the package names;
/// [`RunRefusal::UnsupportedParameter`] and [`RunRefusal::UnsupportedResult`]
/// for a type with no run value form, a parameter checked before the result,
/// before any argument is admitted; the argument refusals of FR-650; and the
/// call's own failures, as [`super::run`] returns them.
pub fn run_package(
    package: &BarePackage<'_>,
    call: &Call,
    objects: &ObjectEnvironment,
    cancel: &Cancel,
) -> Result<CallOutcome, Box<RunRefusal>> {
    run_package_with_sources(package, call, objects, &[], cancel).outcome
}

/// A package call's original result beside any unavailable source attribution.
/// Attribution does not replace the evaluated code, cause or fields.
#[derive(Debug)]
pub struct PackageCallResult {
    /// The unchanged execution result, with a rendered locus when available.
    pub outcome: Result<CallOutcome, Box<RunRefusal>>,
    /// An admitted location whose occurrence attribution was unavailable.
    pub attribution: Option<PackageSourceAttributionFailure>,
}

/// A source-attribution failure retained beside the original evaluated result.
#[derive(Debug)]
pub struct PackageSourceAttributionFailure {
    /// The actual semantic location selected by evaluation.
    pub selector: filament_semantic_value::location::ExpressionLocation,
    /// The helper's original typed reason, without diagnostic normalization.
    pub reason: fsl_package::occurrence_source::OccurrenceSourceRefusal,
    /// The original budget denial, when a metered owner supplied one.
    pub budget_context: Option<fsl_foundation::diagnostic::BudgetDenial>,
}

/// Run the admitted package with caller-admitted rendering sources. Source
/// paths and text supply no package semantics or authentication.
pub fn run_package_with_sources(
    package: &BarePackage<'_>,
    call: &Call,
    objects: &ObjectEnvironment,
    sources: &[fsl_foundation::Source],
    cancel: &Cancel,
) -> PackageCallResult {
    let attribution = std::cell::RefCell::new(None);
    let outcome =
        run_package_resolving(package, call, objects, None, sources, cancel, &attribution);
    PackageCallResult {
        outcome,
        attribution: attribution.into_inner(),
    }
}

/// Run a package call using the effective views retained by its snapshot
/// admission. Population arguments bind against those same views, preserving
/// their declared maxima without normalizing the selected models again.
pub fn run_package_with_snapshot_views(
    package: &BarePackage<'_>,
    call: &Call,
    objects: &ObjectEnvironment,
    views: &[fsl_semantics::model::normalize::EffectiveView],
    sources: &[fsl_foundation::Source],
    cancel: &Cancel,
) -> PackageCallResult {
    let attribution = std::cell::RefCell::new(None);
    let outcome = run_package_resolving(
        package,
        call,
        objects,
        Some(views),
        sources,
        cancel,
        &attribution,
    );
    PackageCallResult {
        outcome,
        attribution: attribution.into_inner(),
    }
}

fn run_package_resolving(
    package: &BarePackage<'_>,
    call: &Call,
    objects: &ObjectEnvironment,
    views: Option<&[fsl_semantics::model::normalize::EffectiveView]>,
    sources: &[fsl_foundation::Source],
    cancel: &Cancel,
    attribution: &std::cell::RefCell<Option<PackageSourceAttributionFailure>>,
) -> Result<CallOutcome, Box<RunRefusal>> {
    // FR-296: a backend the caller did not supply refuses before any stage.
    SuppliedBackends::default()
        .member(call.backend)
        .map_err(|entry| Box::new(RunRefusal::Unsupported(entry)))?;
    if let Some(cause) = cancel.cause() {
        return Err(Box::new(RunRefusal::Cancelled(cause)));
    }
    let missing = || {
        Box::new(RunRefusal::MissingDeclaration {
            function: call.function.clone(),
        })
    };
    let segment = call.function.as_str();
    if segment.is_empty() || segment.contains('.') || !is_identifier(segment) {
        return Err(missing());
    }
    let name = QualifiedName::unqualified(segment).map_err(|_| missing())?;
    let projection = match package.project_function(segment) {
        Ok(Some(projection)) => projection,
        Ok(None) => return Err(missing()),
        Err(BareSelectionFailure::FunctionParameterType { position, .. }) => {
            return Err(Box::new(RunRefusal::UnsupportedParameter { position }));
        }
        Err(BareSelectionFailure::FunctionResultType { .. }) => {
            return Err(Box::new(RunRefusal::UnsupportedResult {
                function: call.function.clone(),
            }));
        }
        // Any other projection failure is the call's own refusal, as the
        // bare package's evaluation entry reports it.
        Err(_) => return Err(call_failure(package, &name, objects, call, cancel)),
    };
    let selected = projection.selected_function().ok_or_else(|| {
        Box::new(RunRefusal::Fault(InternalFault::new(
            "FR-945",
            "bare-call-has-selected-function",
        )))
    })?;
    let parameters: Vec<(String, ValueType)> = selected
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.value_type.clone()))
        .collect();
    let world = match package.world() {
        Ok(world) => world,
        Err(_) => return Err(call_failure(package, &name, objects, call, cancel)),
    };
    let bound_objects;
    let objects = match (call.snapshot.as_ref(), views) {
        (Some(_), Some(views)) => {
            bound_objects =
                super::call::bind_snapshot_populations(objects, views.iter(), &parameters, call)?;
            &bound_objects
        }
        _ => objects,
    };
    // FR-533 "Types the bytes cannot project": the bytes carry no declared
    // enumeration index, so the forms read no enumeration by name.
    let universe_of = |declared: &filament_exact::EffectiveId| {
        objects.population_universe_for_type(declared).cloned()
    };
    let object_world = call.snapshot.as_ref().map(|_| super::run_value::World {
        objects,
        universe_of: &universe_of,
    });
    let forms =
        Forms::new(world.types(), &[]).in_world(package.package().name(), object_world.as_ref());
    let supplied = admit_wire_arguments(&forms, &parameters, call)?;
    let arguments = bind_arguments(&parameters, &supplied)?;
    let mut meter = Meter::new(call.accounting).with_cancel(cancel.clone());
    // One run for the whole invocation: both helper budgets and the caller's
    // original handle (FR-277).
    let mut run = SuppliedRun::new(call.supplied, cancel.clone())
        .map_err(|stop| Box::new(RunRefusal::Supplied(stop)))?;
    let evaluation = package.call_supplied(&name, arguments, objects, &mut meter, &mut run);
    if let Some(cause) = cancel.tripped() {
        return Err(Box::new(RunRefusal::Cancelled(cause)));
    }
    let evaluation = evaluation.map_err(|failure| match failure {
        filament_eval::CallFailure::Cancelled(cause) => Box::new(RunRefusal::Cancelled(cause)),
        other => convert_call_failure(other),
    })?;
    match &evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value))
            if !matches!(
                value,
                Value::Boolean(_) | Value::Integer(_) | Value::Reference(_)
            ) =>
        {
            match forms.write(value) {
                Ok(form) => Ok(CallOutcome::Completed(CallValue::Written(form))),
                Err(WriteStop::NoForm) => Err(Box::new(RunRefusal::UnsupportedResult {
                    function: call.function.clone(),
                })),
                Err(WriteStop::Fault(invariant)) => Err(Box::new(RunRefusal::Fault(
                    InternalFault::new("call", invariant),
                ))),
            }
        }
        _ => convert_outcome_resolving(
            evaluation,
            |location| match projection.source_at_location(package, location) {
                Ok(region) => region,
                Err(reason) => {
                    *attribution.borrow_mut() = Some(PackageSourceAttributionFailure {
                        selector: location.clone(),
                        reason,
                        budget_context: None,
                    });
                    None
                }
            },
            sources,
        ),
    }
}

/// The refusal the bare package's evaluation entry gives a call its function
/// projection or world refused, so the refusal is the evaluator's own.
fn call_failure(
    package: &BarePackage<'_>,
    name: &QualifiedName,
    objects: &ObjectEnvironment,
    call: &Call,
    cancel: &Cancel,
) -> Box<RunRefusal> {
    let mut meter = Meter::new(call.accounting).with_cancel(cancel.clone());
    match package.call(name, Vec::new(), objects, &mut meter) {
        Err(filament_eval::CallFailure::Cancelled(cause)) => Box::new(RunRefusal::Cancelled(cause)),
        Err(failure) => convert_call_failure(failure),
        Ok(_) => Box::new(RunRefusal::Fault(InternalFault::new(
            "FR-945",
            "a-refused-projection-refuses-the-call",
        ))),
    }
}
