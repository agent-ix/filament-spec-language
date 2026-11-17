// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-630-AC-2 (TC-1590): `refinement_internals` is forwarded as part of the
//! analyze request's compilation input, and omission supplies an empty list.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    Workspace, analyze, counter_request_with, label, request_members,
};

fn refinement(name: &str) -> String {
    format!(
        "refinement {name} using v of M as Spec {{\n\
         object Spec::Counter from M::Counter {{ value = self.value; }}\n\
         step M::Counter::inc as Spec::Counter::inc;\n\
         }}\n"
    )
}

fn run_with(name: &str, member: Option<Value>) -> Value {
    let ws = Workspace::new(name);
    let body = format!("model Spec = \"test/counter\";\n{}", refinement("Same"));
    let mut request = request_members(Some(json!([{
        "claim": "Same",
        "kind": "refinement",
        "temporal": {"item_kind": "refinement"}
    }])));
    if let Some(member) = member {
        request["refinement_internals"] = member;
    }
    let run = analyze(&counter_request_with(&ws, &body, request));
    assert_eq!(run.exit, 0, "{}", String::from_utf8_lossy(&run.stdout));
    run.document()
}

/// FR-630-AC-2: an explicit empty list for the program's refinement reaches
/// compilation and the selected refinement settles as supported.
#[trace("TC-1590", "FR-630-AC-2")]
#[test]
fn analyze_accepts_refinement_internals_for_the_program() {
    let result = run_with(
        "family-refinement-internals",
        Some(json!([{"refinement": "Same", "internal": []}])),
    );
    assert_eq!(label(&result["items"][0]), "proved");
}

/// FR-630-AC-2: omitting the member has the same result as supplying the
/// empty internal list.
#[trace("TC-1590", "FR-630-AC-2")]
#[test]
fn absent_refinement_internals_supplies_the_empty_list() {
    let absent = run_with("family-refinement-internals-absent", None);
    let empty = run_with("family-refinement-internals-empty", Some(json!([])));
    assert_eq!(absent["items"], empty["items"]);
}
