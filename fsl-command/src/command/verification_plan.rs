// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strict request reading and declaration-name selection for FR-1038.

use fsl_foundation::Code;
use serde_json::Value;

const PREFIX: &str = "/verification_plan";

/// The closed request carried by `native-run/1`'s `verification_plan` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Request {
    /// The selected plan, if the caller named it.
    pub plan: Option<String>,
    /// Whether to include the emitted plan document.
    pub emit: bool,
    /// Whether to execute the plan.
    pub execute: bool,
    /// Optional state snapshot supplied to consuming steps.
    pub initial: Option<String>,
    /// Deterministic seed; absence is the same as zero.
    pub seed: u64,
    /// Whether the caller explicitly supplied `seed` (for closed-use checks).
    pub seed_supplied: bool,
    /// An optional checked-package plan document; source runs reject it.
    pub document: Option<Value>,
}

/// A typed refusal from the closed plan request reader or selector.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code}/{cause} at {pointer}: {message}")]
pub(super) struct Refusal {
    /// The stable catalog code.
    pub code: Code,
    /// The stable catalog cause.
    pub cause: &'static str,
    /// RFC 6901 pointer, prefixed with `/verification_plan`.
    pub pointer: String,
    /// Human-readable detail.
    pub message: String,
}

/// Read a plan request before any source or dependent file is admitted.
pub(super) fn read(value: &Value) -> Result<Request, Refusal> {
    let Some(object) = value.as_object() else {
        return Err(refusal(
            Code::InvalidRuntimeInput,
            "wrong-value-kind",
            PREFIX,
            "expected an object",
        ));
    };
    for name in object.keys() {
        if !matches!(
            name.as_str(),
            "plan" | "emit" | "execute" | "initial" | "seed" | "document"
        ) {
            return Err(refusal(
                Code::InvalidRuntimeInput,
                "unknown-member",
                &pointer(name),
                format!("unknown verification plan request member {name:?}"),
            ));
        }
    }
    let plan = optional_string(object.get("plan"), "plan")?;
    let emit = optional_bool(object.get("emit"), "emit", false)?;
    let execute = optional_bool(object.get("execute"), "execute", true)?;
    let initial = optional_string(object.get("initial"), "initial")?;
    let seed_value = object.get("seed");
    let seed = match seed_value {
        None => 0,
        Some(value) => value.as_u64().ok_or_else(|| {
            refusal(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
                &pointer("seed"),
                "expected an unsigned 64-bit integer",
            )
        })?,
    };
    let document = object.get("document").cloned();
    if let Some(document) = &document {
        let valid = document.as_object().is_some_and(|document| {
            document.len() == 1
                && document
                    .get("file")
                    .is_some_and(serde_json::Value::is_string)
        });
        if !valid {
            return Err(refusal(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
                &pointer("document"),
                "expected an object with only a string `file` member",
            ));
        }
    }
    if !emit && !execute {
        return Err(refusal(
            Code::InvalidRuntimeInput,
            "invalid-value",
            &pointer("execute"),
            "emit and execute cannot both be false",
        ));
    }
    Ok(Request {
        plan,
        emit,
        execute,
        initial,
        seed,
        seed_supplied: seed_value.is_some(),
        document,
    })
}

/// Read the source-run form of the request; `document` belongs only to the
/// checked-package route and is therefore an unknown member here.
pub(super) fn read_source(value: &Value) -> Result<Request, Refusal> {
    let request = read(value)?;
    if request.document.is_some() {
        return Err(unknown_member(
            "document",
            "document is accepted only when running a checked package",
        ));
    }
    Ok(request)
}

/// Select exactly one declaration, defaulting to the sole plan in the unit.
pub(super) fn select(request: &Request, plans: &[String]) -> Result<String, Refusal> {
    let Some(selected) = &request.plan else {
        return match plans {
            [only] => Ok(only.clone()),
            [] => Err(refusal(
                Code::MissingDeclaration,
                "missing-name",
                &pointer("plan"),
                "the unit declares no verification plan",
            )),
            _ => Err(refusal(
                Code::InvalidRuntimeInput,
                "missing-member",
                &pointer("plan"),
                format!("name one of the verification plans: {}", plans.join(", ")),
            )),
        };
    };
    let matches: Vec<&String> = plans
        .iter()
        .filter(|name| {
            if selected.contains("::") {
                name.as_str() == selected
            } else {
                name.rsplit("::").next() == Some(selected.as_str())
            }
        })
        .collect();
    match matches.as_slice() {
        [name] => Ok((*name).clone()),
        [] => Err(refusal(
            Code::MissingDeclaration,
            "missing-name",
            &pointer("plan"),
            format!("{selected:?} names no verification plan"),
        )),
        names => Err(refusal(
            Code::AmbiguousDeclaration,
            "ambiguous-name",
            &pointer("plan"),
            format!(
                "{selected:?} matches verification plans: {}",
                names
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// Reject caller inputs that the selected plan has no consumer for.
pub(super) fn validate_consumers(
    request: &Request,
    steps: &[fsl_forms::VerificationStepForm],
) -> Result<(), Refusal> {
    let consumes_initial = steps.iter().any(|step| {
        matches!(
            step.method.name.as_str(),
            "model_check" | "model_based_test" | "simulate" | "contract_check"
        )
    });
    if request.initial.is_some() && !consumes_initial {
        return Err(unknown_member(
            "initial",
            "the selected plan has no step that consumes an initial state",
        ));
    }
    let consumes_seed = steps.iter().any(|step| {
        matches!(
            step.method.name.as_str(),
            "model_based_test" | "probabilistic_statistical"
        )
    });
    if request.seed_supplied && !consumes_seed {
        return Err(unknown_member(
            "seed",
            "the selected plan has no step that consumes a seed",
        ));
    }
    Ok(())
}

fn optional_string(value: Option<&Value>, member: &str) -> Result<Option<String>, Refusal> {
    value
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                refusal(
                    Code::InvalidRuntimeInput,
                    "wrong-value-kind",
                    &pointer(member),
                    "expected a string",
                )
            })
        })
        .transpose()
}

fn optional_bool(value: Option<&Value>, member: &str, default: bool) -> Result<bool, Refusal> {
    value
        .map(|value| {
            value.as_bool().ok_or_else(|| {
                refusal(
                    Code::InvalidRuntimeInput,
                    "wrong-value-kind",
                    &pointer(member),
                    "expected a boolean",
                )
            })
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn pointer(member: &str) -> String {
    format!("{PREFIX}/{}", member.replace('~', "~0").replace('/', "~1"))
}

fn unknown_member(member: &str, message: &str) -> Refusal {
    refusal(
        Code::InvalidRuntimeInput,
        "unknown-member",
        &pointer(member),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn selection_uses_qualified_names_and_bare_suffixes() {
        let request = read(&json!({"plan": "Release"})).unwrap();
        assert_eq!(
            select(&request, &["A::Release".to_owned()]).unwrap(),
            "A::Release"
        );
        let request = read(&json!({"plan": "A::Release"})).unwrap();
        assert_eq!(
            select(&request, &["A::Release".to_owned()]).unwrap(),
            "A::Release"
        );
        assert_eq!(
            select(&request, &["B::Release".to_owned()])
                .unwrap_err()
                .cause,
            "missing-name"
        );
        let request = read(&json!({"plan": "Release"})).unwrap();
        assert_eq!(
            select(
                &request,
                &["A::Release".to_owned(), "B::Release".to_owned()]
            )
            .unwrap_err()
            .cause,
            "ambiguous-name"
        );
    }

    #[test]
    fn source_reader_refuses_a_package_document_at_its_pointer() {
        let refusal = read_source(&json!({"document": {"file": "plan.json"}})).unwrap_err();
        assert_eq!(refusal.cause, "unknown-member");
        assert_eq!(refusal.pointer, "/verification_plan/document");
    }
}

fn refusal(code: Code, cause: &'static str, pointer: &str, message: impl Into<String>) -> Refusal {
    Refusal {
        code,
        cause,
        pointer: pointer.to_owned(),
        message: message.into(),
    }
}
