// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lowered checked expressions and their public run and call-site boundaries.

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use serde_json::Value;

mod calls;
mod lowering;
mod profiles;
mod replay_bindings;
mod state_clauses;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn source(body: &str) -> String {
    format!("{HEADER}{body}\n")
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:lowering-intake-boundaries")
}

fn package(body: &str) -> Value {
    let text = source(body);
    let emitted = crate::support::front_end::emitted(
        identity(),
        "boundaries.native",
        text.as_bytes(),
        &Default::default(),
        &DependencyInput::default(),
    )
    .unwrap_or_else(|error| panic!("fixture must emit: {error:?}"));
    let bytes = emitted.package().bytes();
    // The actual producer output also satisfies the independent public reader.
    crate::emit_checked_v2::read_back(bytes, &[]);
    serde_json::from_slice(bytes).expect("emitted checked package JSON")
}

fn nodes(package: &Value) -> &[Value] {
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
}

fn applications<'a>(package: &'a Value, operation: &str) -> Vec<&'a Value> {
    nodes(package)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == operation)
        .collect()
}

fn one_application<'a>(package: &'a Value, operation: &str) -> &'a Value {
    let found = applications(package, operation);
    assert_eq!(found.len(), 1, "one {operation} application: {found:?}");
    found[0]
}

fn reference_target(term: &Value) -> &Value {
    assert_eq!(term["term"], "reference", "{term}");
    term.get("target").expect("reference target")
}
