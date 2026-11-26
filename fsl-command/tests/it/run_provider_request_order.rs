// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-961: prepare the run request before building its provider registry.

use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::json;

/// FR-1047-AC-2: `{}` is refused with the exact missing-members request
/// diagnostic, even when provider registration would also fail.
#[trace("FR-1047-AC-2")]
#[test]
fn run_prepares_the_request_before_registering_providers() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let request = directory.path().join("request.json");
    std::fs::write(&request, json!({}).to_string()).expect("the request is written");

    // `value_check` is already registered by Engines::fsl(), so adding it as
    // a provider would fail registry construction if that happened first.
    let duplicate = fsl_analyze::analyze::Engine::value_check().expect("the linked engine");
    let Err(error) = command::run_with_providers(
        &request,
        &crate::negotiation::Fr290,
        vec![duplicate],
        &fsl_analyze::provider::ProviderOptions::default(),
        &filament_exact::Cancel::new(),
    ) else {
        panic!("the request has no format or request member");
    };

    let refusal = error.value().expect("the refusal document");
    assert_eq!(error.category().exit_code(), 20);
    assert_eq!(refusal["code"], "invalid-request");
    assert_eq!(refusal["stage"], "request");
    assert_eq!(
        refusal["message"],
        "invalid request: missing required members /format, /request"
    );
    assert_eq!(
        refusal["details"],
        json!({"missing": ["/format", "/request"]})
    );
}
