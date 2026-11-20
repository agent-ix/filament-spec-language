// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1038 (FSpec FR-824): a `native-run/1` request's `verification_plan`
//! member. [`request`] is its strict reader and [`outcome`] the writer of the
//! `filament.verification-plan-outcome/v1` document; [`run_bytes`] compiles
//! the program, selects one emitted plan, hands its inputs to FR-753's
//! coordinator and writes the outcome.

pub mod outcome;
pub mod request;

use std::path::Path;

use fsl_analyze::analyze::document::SubjectSelection;
use fsl_analyze::analyze::{AnalyzeLimits, SourceProvision};
use fsl_analyze::plan::{PlanInputs, PlanRefusal, execute_plan, plan_consumers};
use fsl_foundation::Code;
use fsl_foundation::diagnostic::StageFailure;
use fsl_package::verification_plan::{
    VerificationPlan, VerificationPlanRefusal, emit_verification_plans,
};
use fsl_semantics::model::observation::DocumentProvision;
use serde_json::Value;

use super::settle::Settle;
use super::{
    FileCounts, NativeResult, Result, RunCause, RunLimits, RunResult, check_sources,
    intake_with_bytes, request_integer_division, require_native_header, wire,
};

/// Read the `verification_plan` member of the `native-run/1` request in
/// `bytes`, when it carries one, before any stage runs. A request with a
/// `package` member is a run over a selected checked package, which carries
/// the plan `document`; any other request is a source run.
///
/// # Errors
///
/// The reader's refusal of a malformed member.
pub(super) fn check_member(bytes: &[u8]) -> Result<()> {
    member(bytes).map(drop)
}

/// The request's route and its read `verification_plan` member, when the
/// document is a `native-run/1` request carrying one.
fn member(bytes: &[u8]) -> Result<Option<(request::Route, request::PlanRequest)>> {
    let document: Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(None);
    }
    let Some(body) = document.get("request") else {
        return Ok(None);
    };
    let route = if body.get("package").is_some() {
        request::Route::Package
    } else {
        request::Route::Source
    };
    match body.get("verification_plan") {
        Some(member) => request::read(member, route).map(|read| Some((route, read))),
        None => Ok(None),
    }
}

fn refuse(code: Code, pointer: &str, cause: &'static str, reason: impl Into<String>) -> RunCause {
    RunCause::PlanRequest {
        code,
        path: format!("/request/verification_plan{pointer}"),
        cause,
        reason: reason.into(),
    }
}

/// FR-1038: run a source request's `verification_plan` member. `None` when
/// the request carries no such member, or is a run over a selected checked
/// package, which the package route serves.
///
/// The program is compiled as FR-100 compiles it; the plan is selected from
/// the plan documents S4 emits for the package (FSpec FR-824 "Selection");
/// the member set is closed by the selected plan; `initial` is resolved
/// against the request file's directory; and FR-753's coordinator runs the
/// plan when `execute` holds.
///
/// # Errors
///
/// A refusal at the member's pointer, or the compile's refusal.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    limits: RunLimits,
    settle: &Settle<'_>,
) -> Result<Option<RunResult>> {
    let Some((request::Route::Source, plan_request)) = member(bytes)? else {
        return Ok(None);
    };
    let mut request: wire::Request = super::request(bytes)?;
    if request.call.is_some()
        || request.clause.is_some()
        || request.items.is_some()
        || request.tl_lowering.is_some()
        || request.synthesis.is_some()
    {
        return Err(RunCause::ClauseRequest {
            member: "verification_plan".to_owned(),
            reason: "a verification plan run is the request's one run selection".to_owned(),
        });
    }
    let integer_division = request_integer_division(request.integer_division.take())?;
    let program = super::required_program(&request)?;
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: usize::from(plan_request.initial.is_some()),
            extensions: request.extensions.len(),
        },
        limits.dependent_bytes,
    )?;
    let source = intake.source(&program.source, "program.source")?;
    require_native_header(source.source())?;
    let libraries = std::mem::take(&mut request.libraries)
        .into_iter()
        .enumerate()
        .map(|(index, library)| {
            if library.integer_division.is_some() {
                return Err(RunCause::ClauseRequest {
                    member: format!("libraries[{index}].integer_division"),
                    reason: "a verification plan run compiles each library under its own law"
                        .to_owned(),
                });
            }
            Ok(wire::Library {
                identity: library.identity,
                refinement_internals: library.refinement_internals,
                source: library.source,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let compile = wire::CompileRequest {
        models: Some(std::mem::take(&mut request.models)),
        program: wire::CompileProgram {
            sources: Vec::new(),
            extraction: false,
        },
        refinement_internals: None,
        libraries: Some(libraries),
        extensions: std::mem::take(&mut request.extensions),
        clocks: Some(request.clocks.clone()),
        output: None,
        handoff: None,
        integer_division: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    let sources = std::slice::from_ref(&source);
    let result = check_sources(
        &compile,
        &mut intake,
        sources,
        false,
        fsl_replay::spine::SpineLimits::default(),
        integer_division,
        None,
        |intake, checked, _, _, packages, units, _, _, _, _| {
            let package = checked.package();
            let plans = emit_verification_plans(package);
            let plan = select(&plans, plan_request.plan.as_deref())?;
            let mut documents = DocumentProvision::default();
            if let Some(file) = &plan_request.initial {
                let bytes = intake.file(file, intake.remaining)?;
                let read = fsl_replay::spine::SuppliedObservation::from_document(
                    "verification_plan.initial",
                    bytes,
                )
                .map_err(|error| {
                    refuse(
                        Code::InvalidRuntimeInput,
                        "/initial",
                        "invalid-value",
                        error.reason,
                    )
                })?;
                documents.insert(read.reference, read.bytes);
            }
            let binding = SubjectSelection::default_over(&documents).binding(&packages, &documents);
            let mut provision = SourceProvision::default();
            for unit in std::iter::once(&source).chain(units) {
                provision.insert(
                    unit.source().reference(),
                    unit.source().text().as_bytes().to_vec(),
                );
            }
            let inputs = PlanInputs {
                bindings: plan
                    .nodes
                    .iter()
                    .map(|node| (node.name.clone(), binding.clone()))
                    .collect(),
                provision,
                seed: plan_request.seed,
                max_trace_length: plan_request.max_trace_length,
                max_value_bits: plan_request.max_value_bits,
            };
            let analyze_limits = AnalyzeLimits::unbounded();
            let consumers =
                plan_consumers(package, plan, &inputs, &analyze_limits).map_err(plan_refusal)?;
            plan_request.close_by_plan(&request::Consumers {
                initial: first(&consumers.initial),
                seed: first(&consumers.seed),
                max_trace_length: first(&consumers.max_trace_length),
                max_value_bits: first(&consumers.max_value_bits),
            })?;
            let dispositions = if plan_request.execute {
                let outcome = execute_plan(
                    package,
                    plan,
                    &inputs,
                    settle.engines,
                    settle.negotiate,
                    &analyze_limits,
                    settle.cancel,
                )
                .map_err(plan_refusal)?
                .into_value();
                Some(outcome::dispositions(outcome)?)
            } else {
                None
            };
            let run = outcome::PlanRunOutcome {
                package: package.graph().package_name().to_owned(),
                plan: plan.plan.name.clone(),
                document: if plan_request.emit {
                    Some(serde_json::to_value(plan)?)
                } else {
                    None
                },
                initial: plan_request.initial.clone(),
                seed: plan_request.effective_seed(),
                max_trace_length: plan_request.max_trace_length,
                max_value_bits: plan_request.max_value_bits,
                dispositions,
            };
            Ok(RunResult {
                exit_code: run.exit_code(),
                value: NativeResult::plan_document(run.document()),
                outcome: None,
                requirements: None,
                sources: crate::Sources::default(),
            })
        },
    )?;
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(Some(RunResult { sources, ..result }))
}

/// The first step in source order of `steps`.
fn first(steps: &[String]) -> Option<&str> {
    steps.first().map(String::as_str)
}

/// FSpec FR-824 "Selection": the plan `name` selects among `plans` by the
/// last segment of a plan's `qualified_name` (a bare name) or its whole path
/// (a qualified name); an absent name selects the only plan.
fn select<'p>(plans: &'p [VerificationPlan], name: Option<&str>) -> Result<&'p VerificationPlan> {
    let names = || {
        plans
            .iter()
            .map(|plan| plan.plan.qualified_name.join("::"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let Some(name) = name else {
        return match plans {
            [plan] => Ok(plan),
            [] => Err(refuse(
                Code::MissingDeclaration,
                "/plan",
                "missing-name",
                "the program declares no verification plan",
            )),
            _ => Err(refuse(
                Code::InvalidRuntimeInput,
                "/plan",
                "missing-member",
                format!("the program declares several plans: {}", names()),
            )),
        };
    };
    let path: Vec<&str> = name.split("::").collect();
    let matched: Vec<&VerificationPlan> = plans
        .iter()
        .filter(|plan| {
            let qualified = &plan.plan.qualified_name;
            if path.len() == 1 {
                qualified.last().map(String::as_str) == Some(name)
            } else {
                qualified.iter().map(String::as_str).eq(path.iter().copied())
            }
        })
        .collect();
    match matched.as_slice() {
        [plan] => Ok(plan),
        [] => Err(refuse(
            Code::MissingDeclaration,
            "/plan",
            "missing-name",
            format!("`{name}` names no verification plan"),
        )),
        _ => Err(refuse(
            Code::AmbiguousDeclaration,
            "/plan",
            "ambiguous-name",
            format!(
                "`{name}` names several plans: {}",
                matched
                    .iter()
                    .map(|plan| plan.plan.qualified_name.join("::"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// A coordinator refusal at its request pointer (FSpec FR-824): a missing
/// consumed bound at its member, an `initial` no consuming model admits at
/// `/initial`, and a plan document of another package at `/document`.
fn plan_refusal(failure: StageFailure<PlanRefusal>) -> RunCause {
    let StageFailure::Refused(refusal) = failure else {
        return RunCause::AnalyzeFault;
    };
    match refusal {
        PlanRefusal::MissingMember { member, step } => refuse(
            Code::InvalidRuntimeInput,
            &format!("/{member}"),
            "missing-member",
            format!("the step `{step}` consumes `{member}`"),
        ),
        PlanRefusal::Initial(failure) => refuse(
            Code::InvalidRuntimeInput,
            "/initial",
            "invalid-value",
            format!("no consuming step's model admits `initial`: {failure:?}"),
        ),
        PlanRefusal::Reader(VerificationPlanRefusal::PackageIdentityMismatch {
            expected,
            actual,
        }) => refuse(
            Code::InvalidRuntimeInput,
            "/document",
            "invalid-value",
            format!("the plan document is bound to `{actual}`, not the selected `{expected}`"),
        ),
        PlanRefusal::Reader(VerificationPlanRefusal::InvalidDocument { reason }) => refuse(
            Code::InvalidRuntimeInput,
            "/document",
            "invalid-value",
            reason,
        ),
    }
}
