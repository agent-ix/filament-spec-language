// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1022: closed admission of an export evidence selection. Settlement
//! remains unsupported until the original generated package can be bound.

use super::super::{output::NativeResult, RequestOverrides, Result, RunCause, RunResult};
use fsl_foundation::Code;
use fsl_replay::{Operation, OutcomeDiagnostic, OutcomeDocument};
use serde_json::Value;

const MEMBERS: [&str; 4] = ["package", "obligation", "tool", "artifact"];

fn refuse(member: &str, cause: &'static str, reason: impl Into<String>) -> RunCause {
    RunCause::ExportRequest {
        path: format!("/export_evidence{member}"),
        cause,
        reason: reason.into(),
    }
}

/// Validate only the declared evidence input. No artifact or source is read,
/// and no package identity or proof claim is inferred from this admission.
#[ix_trace_rs::string_edge]
pub(super) fn run(
    bytes: &[u8],
    member: &Value,
    overrides: &RequestOverrides<'_>,
) -> Result<RunResult> {
    let body = member
        .as_object()
        .ok_or_else(|| refuse("", "wrong-value-kind", "expected an object"))?;
    if let Some(name) = body.keys().find(|name| !MEMBERS.contains(&name.as_str())) {
        return Err(refuse(
            &format!("/{name}"),
            "unknown-member",
            "unknown evidence member",
        ));
    }
    let package = body
        .get("package")
        .ok_or_else(|| refuse("/package", "missing-member", "required member is absent"))?;
    if !package.is_object() {
        return Err(refuse("/package", "wrong-value-kind", "expected an object"));
    }
    for name in ["obligation", "tool", "artifact"] {
        let path = format!("/{name}");
        let value = body
            .get(name)
            .ok_or_else(|| refuse(&path, "missing-member", "required member is absent"))?;
        let text = value
            .as_str()
            .ok_or_else(|| refuse(&path, "wrong-value-kind", "expected a string"))?;
        if text.is_empty() {
            return Err(refuse(&path, "invalid-value", "must not be empty"));
        }
        if name == "tool" && !["tlc", "apalache", "tlaps", "lean"].contains(&text) {
            return Err(refuse(
                &path,
                "invalid-value",
                "expected tlc, apalache, tlaps or lean",
            ));
        }
    }
    let request: super::super::wire::Request = super::super::request(bytes)?;
    super::super::refuse_transfer_selection_conflict(&request)?;
    for (name, present) in super::super::TRANSFER_EXCLUSIVE {
        if name != "export_evidence" && name != "package" && present(&request) {
            return Err(RunCause::ClauseRequest {
                member: "export_evidence".to_owned(),
                reason: format!("export_evidence is exclusive with {name}"),
            });
        }
    }
    if overrides.output.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "output".to_owned(),
            reason: "an export_evidence selection does not write a target file".to_owned(),
        });
    }
    let code = Code::UnimplementedCapability;
    let document = OutcomeDocument::new(Operation::Export, None, code.category())
        .with_diagnostics(vec![OutcomeDiagnostic::new(
            Some("unimplemented-meaning"),
            code.as_str(),
            None,
            "export_evidence settlement is not implemented: original generated-package binding and target-result verification are unavailable",
        )]);
    let value = serde_json::to_value(&document).map_err(RunCause::Output)?;
    Ok(RunResult {
        exit_code: code.category().exit_code(),
        value: NativeResult::capability_document(value),
        outcome: Some(document),
        requirements: None,
        sources: crate::Sources::default(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn member() -> Value {
        json!({"package": {}, "obligation": "Low", "tool": "lean", "artifact": "proof.lean"})
    }

    fn run_member(member: &Value) -> Result<RunResult> {
        let bytes = serde_json::to_vec(&json!({
            "format": "native-run/1", "request": {"models": [], "export_evidence": member}
        }))
        .unwrap();
        run(&bytes, member, &RequestOverrides::default())
    }

    /// Admission defects must name their own member before the pending capability.
    #[test]
    #[trace("FR-1022", "TC-2872")]
    fn malformed_evidence_refuses_at_its_member() {
        for (name, value, cause) in [
            ("package", json!(false), "wrong-value-kind"),
            ("obligation", json!(""), "invalid-value"),
            ("tool", json!("observer"), "invalid-value"),
            ("artifact", Value::Null, "wrong-value-kind"),
            ("log", json!("proof accepted"), "unknown-member"),
        ] {
            let mut input = member();
            input[name] = value;
            let error = run_member(&input).unwrap_err();
            match error {
                RunCause::ExportRequest {
                    path,
                    cause: actual,
                    ..
                } => {
                    assert_eq!(path, format!("/export_evidence/{name}"));
                    assert_eq!(actual, cause);
                }
                other => panic!("expected evidence member refusal: {other}"),
            }
        }
        for name in MEMBERS {
            let mut input = member();
            input.as_object_mut().unwrap().remove(name);
            let error = run_member(&input).unwrap_err();
            assert!(
                matches!(error, RunCause::ExportRequest { ref path, cause: "missing-member", .. }
                if path == &format!("/export_evidence/{name}")),
                "{error}"
            );
        }
    }

    /// Shape admission supplies a lifecycle refusal, never a proof or a tool observation.
    #[test]
    #[trace("FR-1022", "TC-2872")]
    fn admitted_evidence_remains_an_unimplemented_capability() {
        let result = run_member(&member()).unwrap();
        assert_eq!(result.exit_code, 21);
        let outcome = result
            .outcome
            .expect("every selected export has a lifecycle outcome");
        let document = serde_json::to_value(outcome).unwrap();
        assert_eq!(document["operation"], "export");
        assert_eq!(document["category"], "unsupported");
        assert!(document["last_stage"].is_null());
        assert!(document["result"].is_null());
        assert_eq!(
            document["diagnostics"][0]["code"],
            "unimplemented_capability"
        );
        assert_eq!(document["diagnostics"][0]["cause"], "unimplemented-meaning");
        assert!(document["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("export_evidence"));
        assert!(!document.to_string().contains("tool-unavailable"));
        assert_eq!(result.value.as_value(), &document);
    }
}
