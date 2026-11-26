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
use fsl_analyze::analyze::SourceProvision;
use fsl_analyze::plan::{execute_plan, plan_consumers, PlanInputs, PlanRefusal};
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::Code;
use fsl_package::verification_plan::{
    emit_verification_plans, VerificationPlan, VerificationPlanRefusal,
};
use fsl_semantics::model::observation::DocumentProvision;
use serde_json::Value;

use super::settle::Settle;
use super::{
    check_sources, intake_with_bytes, request_integer_division, require_native_header, wire,
    FileCounts, NativeResult, RequestOverrides, Result, RunCause, RunLimits, RunResult,
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
    overrides: &RequestOverrides<'_>,
    settle: &Settle<'_>,
) -> Result<Option<RunResult>> {
    let Some((request::Route::Source, plan_request)) = member(bytes)? else {
        return Ok(None);
    };
    super::overrides::admit(super::overrides::RunRoute::VerificationPlan, overrides)?;
    let mut request: wire::Request = super::request_with_overrides(bytes, overrides)?;
    // A `synthesis` beside this member is refused by the synthesis run's own
    // mutual-exclusion check (FR-1037), which names every excluded member.
    if request.synthesis.is_some() {
        return Ok(None);
    }
    if request.call.is_some()
        || request.clause.is_some()
        || request.items.is_some()
        || request.tl_lowering.is_some()
    {
        return Err(RunCause::ClauseRequest {
            member: "verification_plan".to_owned(),
            reason: "a verification plan run is the request's one run selection".to_owned(),
        });
    }
    let requirements = request
        .requirements
        .as_ref()
        .map(|value| super::requirements::read(value, "/requirements"))
        .transpose()?
        .unwrap_or_default();
    if !requirements.items.is_empty() {
        return Err(RunCause::ClauseRequest {
            member: "/requirements/items".to_owned(),
            reason: "per-claim call selectors do not select verification-plan steps".to_owned(),
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
        call_site: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    let sources = std::slice::from_ref(&source);
    let result = check_sources(
        &compile,
        &mut intake,
        sources,
        false,
        overrides.spine_limits(),
        integer_division,
        None,
        |intake, checked, _, _, packages, units, _, _, _, _| {
            let package = checked.package();
            let plans = emit_verification_plans(package);
            let plan = select(&plans, plan_request.plan.as_deref())?;
            let mut documents = DocumentProvision::default();
            let mut initial = Vec::new();
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
                initial.push(read.reference.clone());
                documents.insert(read.reference, read.bytes);
            }
            // FSpec FR-824: `initial` is the state each consuming step starts
            // from, and its complete populations are the subject's universes.
            let mut binding =
                SubjectSelection::default_over(&documents).binding(&packages, &documents);
            binding.initial = initial;
            let mut provision = SourceProvision::default();
            for unit in std::iter::once(&source).chain(units) {
                provision.insert(
                    unit.source().reference(),
                    unit.source().text().as_bytes().to_vec(),
                );
            }
            let inputs = PlanInputs {
                backend: requirements.call.backend.clone(),
                backend_options: requirements.call.options.clone(),
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
            // The native-run request carries no analysis limits or deadline member
            // (FSpec FR-824 "no member sets a step's limits"), so the limits are
            // the analyze route's own over its empty `limits` member.
            let analyze_limits = super::overrides::analyze_limits(
                super::analyze::analyze_limits(&wire::AnalyzeLimits::default(), None),
                overrides,
            );
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
                Some(outcome::dispositions(
                    outcome,
                    package.graph().package_name(),
                )?)
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
                qualified
                    .iter()
                    .map(String::as_str)
                    .eq(path.iter().copied())
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
        refusal @ PlanRefusal::Initial(_) => {
            // FSpec FR-824: the snapshot admission's own refusal, at `/initial`.
            let catalog = filament_semantic_value::catalog::CatalogCoded::catalog_code(&refusal);
            refuse(
                Code::from_code(catalog.code()).unwrap_or(Code::InvalidRuntimeInput),
                "/initial",
                catalog.cause(),
                format!("no consuming step's model admits `initial`: {refusal}"),
            )
        }
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

#[cfg(test)]
mod tests {
    use fsl_foundation::Code;
    use fsl_package::verification_plan::VerificationPlan;
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{select, RunCause};

    /// A plan named `Release` declared in the scope `scope`.
    fn release_in(scope: &str) -> VerificationPlan {
        serde_json::from_value(json!({
            "format": "filament.verification-plan/v1",
            "package": "test:plans",
            "plan": {"name": "Release", "qualified_name": [scope, "Release"],
                "source_location": {
                    "source": {"authority": "agent-ix", "identity": "test:plans"},
                    "start": 0, "end": 1}},
            "profile": "v",
            "nodes": [],
            "edges": []
        }))
        .expect("a plan document")
    }

    /// FSpec FR-824-AC-2, its ambiguous selection: a bare `Release` matching
    /// `A::Release` and `B::Release` refuses at the `plan` pointer, naming
    /// both, with the refusal exit.
    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-2")]
    fn a_bare_name_matching_two_plans_refuses_ambiguous_naming_both() {
        let plans = [release_in("A"), release_in("B")];
        let refusal = select(&plans, Some("Release")).expect_err("two plans match");
        assert_eq!(refusal.category().exit_code(), 20, "{refusal}");
        let RunCause::PlanRequest {
            code,
            path,
            cause,
            reason,
        } = refusal
        else {
            panic!("a plan request refusal: {refusal}");
        };
        assert_eq!(code, Code::AmbiguousDeclaration);
        assert_eq!(cause, "ambiguous-name");
        assert_eq!(path, "/request/verification_plan/plan");
        assert_eq!(
            reason,
            "`Release` names several plans: A::Release, B::Release"
        );
    }
}
