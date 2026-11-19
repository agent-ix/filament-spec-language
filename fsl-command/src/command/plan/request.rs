// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-824 "Request": the strict reader of a run request's
//! `verification_plan` member (FR-1038). The member list is closed; every
//! refusal is `invalid_runtime_input` at the member's pointer with its FR-805
//! cause.

use super::super::{Result, RunCause};
use serde_json::{Map, Value};

/// The members of a verification plan request (FSpec FR-824).
const MEMBERS: [&str; 6] = ["plan", "emit", "execute", "initial", "seed", "document"];

/// Which run route carries the request: FSpec FR-824 "Selection" lets only a
/// run over a selected checked package carry `document`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    /// A source run: the program is compiled and the plan documents emitted.
    Source,
    /// A run over a selected checked package: no source is compiled, so the
    /// request carries the plan document to run.
    Package,
}

/// One verification plan request, read strictly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRequest {
    /// The bare or qualified declared plan name, when given.
    pub plan: Option<String>,
    /// Whether the outcome carries the plan document (default `false`).
    pub emit: bool,
    /// Whether the coordinator runs the plan (default `true`).
    pub execute: bool,
    /// The `filament.state.snapshot/v1` file, resolved by the caller against
    /// the request file's directory.
    pub initial: Option<String>,
    /// The generation seed, when given.
    pub seed: Option<u64>,
    /// The plan document file of a package-route run.
    pub document: Option<String>,
}

impl PlanRequest {
    /// The seed in force: the request's, else `0` (FSpec FR-824).
    #[must_use]
    pub fn effective_seed(&self) -> u64 {
        self.seed.unwrap_or(0)
    }

    /// Close the member set by the selected plan (FSpec FR-824 "Request"):
    /// `initial` on a plan with no consuming step, and `seed` on a plan with
    /// no consuming step, refuse `unknown-member` at their pointers.
    ///
    /// # Errors
    ///
    /// `unknown-member` at `/verification_plan/initial` or `/seed`.
    pub fn close_by_plan(&self, consumes_initial: bool, consumes_seed: bool) -> Result<()> {
        if self.initial.is_some() && !consumes_initial {
            return Err(refuse(
                "/initial",
                "unknown-member",
                "no step of the selected plan consumes `initial`",
            ));
        }
        if self.seed.is_some() && !consumes_seed {
            return Err(refuse(
                "/seed",
                "unknown-member",
                "no step of the selected plan consumes `seed`",
            ));
        }
        Ok(())
    }
}

fn refuse(pointer: &str, cause: &'static str, reason: impl Into<String>) -> RunCause {
    RunCause::PlanRequest {
        path: format!("/request/verification_plan{pointer}"),
        cause,
        reason: reason.into(),
    }
}

fn kind_error(name: &str, expected: &str) -> RunCause {
    refuse(
        &format!("/{name}"),
        "wrong-value-kind",
        format!("expected {expected}"),
    )
}

fn text(object: &Map<String, Value>, name: &str) -> Result<Option<String>> {
    match object.get(name) {
        None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(kind_error(name, "a string")),
    }
}

fn flag(object: &Map<String, Value>, name: &str, default: bool) -> Result<bool> {
    match object.get(name) {
        None => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(kind_error(name, "a boolean")),
    }
}

fn seed(object: &Map<String, Value>) -> Result<Option<u64>> {
    match object.get("seed") {
        None => Ok(None),
        Some(Value::Number(number)) => number.as_u64().map(Some).ok_or_else(|| {
            refuse(
                "/seed",
                "invalid-value",
                "the seed is an unsigned 64-bit integer",
            )
        }),
        Some(_) => Err(kind_error("seed", "an unsigned 64-bit integer")),
    }
}

fn document(object: &Map<String, Value>, route: Route) -> Result<Option<String>> {
    let Some(member) = object.get("document") else {
        return match route {
            Route::Source => Ok(None),
            Route::Package => Err(refuse(
                "/document",
                "missing-member",
                "a run over a selected package names the plan document to run",
            )),
        };
    };
    if route == Route::Source {
        return Err(refuse(
            "/document",
            "unknown-member",
            "a source run emits its plan documents and takes none",
        ));
    }
    let Some(inner) = member.as_object() else {
        return Err(kind_error("document", "an object"));
    };
    if let Some(unknown) = inner.keys().find(|key| key.as_str() != "file") {
        return Err(refuse(
            &format!("/document/{unknown}"),
            "unknown-member",
            "unknown document member",
        ));
    }
    match inner.get("file") {
        Some(Value::String(file)) => Ok(Some(file.clone())),
        Some(_) => Err(refuse(
            "/document/file",
            "wrong-value-kind",
            "expected a string",
        )),
        None => Err(refuse(
            "/document/file",
            "missing-member",
            "the document names its file",
        )),
    }
}

/// Read `member`, the run request's `verification_plan` value.
///
/// # Errors
///
/// `invalid_runtime_input` at the first defective member's pointer:
/// `unknown-member`, `wrong-value-kind`, `invalid-value` (`emit` and
/// `execute` both `false`, at `/execute`; a `seed` outside the unsigned
/// 64-bit range), or `missing-member` (`document` on a package route).
pub fn read(member: &Value, route: Route) -> Result<PlanRequest> {
    let Some(object) = member.as_object() else {
        return Err(refuse("", "wrong-value-kind", "expected an object"));
    };
    if let Some(unknown) = object.keys().find(|key| !MEMBERS.contains(&key.as_str())) {
        return Err(refuse(
            &format!("/{unknown}"),
            "unknown-member",
            "unknown verification plan member",
        ));
    }
    let plan = text(object, "plan")?;
    let emit = flag(object, "emit", false)?;
    let execute = flag(object, "execute", true)?;
    if !emit && !execute {
        return Err(refuse(
            "/execute",
            "invalid-value",
            "`emit` and `execute` are both false, so the request asks for nothing",
        ));
    }
    let initial = text(object, "initial")?;
    let seed = seed(object)?;
    let document = document(object, route)?;
    Ok(PlanRequest {
        plan,
        emit,
        execute,
        initial,
        seed,
        document,
    })
}
