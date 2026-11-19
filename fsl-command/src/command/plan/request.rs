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
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(kind_error(name, "a string")),
    }
}

fn flag(object: &Map<String, Value>, name: &str, default: bool) -> Result<bool> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(_) => Err(kind_error(name, "a boolean")),
    }
}

fn seed(object: &Map<String, Value>) -> Result<Option<u64>> {
    match object.get("seed") {
        None | Some(Value::Null) => Ok(None),
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
    let Some(member) = object.get("document").filter(|value| !value.is_null()) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn cause(member: &Value, route: Route) -> (String, &'static str) {
        match read(member, route) {
            Err(RunCause::PlanRequest { path, cause, .. }) => (path, cause),
            other => panic!("expected a plan request refusal, got {other:?}"),
        }
    }

    #[test]
    #[trace("TC-3018", "FR-1038-AC-2")]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-3")]
    fn a_seed_on_a_plan_no_step_consumes_it_in_refuses_unknown_member() {
        let request = read(
            &json!({"plan": "P", "seed": 3, "initial": "s.json"}),
            Route::Source,
        )
        .unwrap();
        assert!(request.close_by_plan(true, true).is_ok());
        for (initial, seed, pointer) in [
            (true, false, "/request/verification_plan/seed"),
            (false, true, "/request/verification_plan/initial"),
        ] {
            match request.close_by_plan(initial, seed) {
                Err(RunCause::PlanRequest { path, cause, .. }) => {
                    assert_eq!((path.as_str(), cause), (pointer, "unknown-member"));
                }
                other => panic!("expected unknown-member, got {other:?}"),
            }
        }
    }

    #[test]
    #[trace("FSpec-TC-2039", "FSpec-FR-824-AC-3")]
    fn the_package_route_requires_a_document_and_the_source_route_refuses_one() {
        let document = "/request/verification_plan/document";
        assert_eq!(
            cause(&json!({"plan": "P"}), Route::Package),
            (document.to_owned(), "missing-member")
        );
        assert_eq!(
            cause(&json!({"document": {"file": "p.json"}}), Route::Source),
            (document.to_owned(), "unknown-member")
        );
        let request = read(&json!({"document": {"file": "p.json"}}), Route::Package).unwrap();
        assert_eq!(request.document.as_deref(), Some("p.json"));
        assert_eq!(
            cause(&json!({"document": {"file": 1}}), Route::Package),
            (format!("{document}/file"), "wrong-value-kind")
        );
    }

    #[test]
    #[trace("TC-3032", "FR-1038-AC-3")]
    #[trace("FSpec-TC-2046", "FSpec-FR-824-AC-6")]
    fn a_null_member_is_absent_and_a_seed_that_is_not_a_u64_is_invalid() {
        let request = read(
            &json!({"plan": "Release", "seed": null, "emit": null, "initial": null}),
            Route::Source,
        )
        .unwrap();
        assert_eq!(request.seed, None);
        assert_eq!(request.effective_seed(), 0);
        assert_eq!(request.initial, None);
        assert!(!request.emit && request.execute);
        for seed in [
            json!(-1),
            json!(7.5),
            json!(1e3),
            Value::from(18_446_744_073_709_551_616_f64),
        ] {
            assert_eq!(
                cause(&json!({"seed": seed}), Route::Source),
                (
                    "/request/verification_plan/seed".to_owned(),
                    "invalid-value"
                ),
                "{seed}"
            );
        }
        assert_eq!(
            cause(&json!({"seed": "7"}), Route::Source),
            (
                "/request/verification_plan/seed".to_owned(),
                "wrong-value-kind"
            )
        );
    }
}
