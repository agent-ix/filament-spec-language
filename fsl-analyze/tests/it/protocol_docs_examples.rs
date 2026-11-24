// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-570-AC-1, FR-575-AC-2 (TC-1410, TC-1415): projection and realizability
//! requests over protocols shaped like the docs examples settle with real
//! verdicts, and a refusal names its cause and location.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{Workspace, analyze, label, litmus_request};

fn root() -> Value {
    let universe = crate::fixture_ids::universe_id("u");
    let effective = crate::fixture_ids::effective_id("e");
    let universe = fsl_foundation::node_wire::node_key_to_text(universe.root_type())
        .expect("universe identity encodes");
    let effective = Value::Array(
        effective
            .owner_chain()
            .iter()
            .map(|original| json!({"package": original.package(), "node": original.node()}))
            .collect(),
    );
    json!({"observation": {"authority": "test", "identity": "protocol-claim"},
        "subject": {"tag": "object", "universe": universe, "type": effective.to_string(), "object_identity": "flow-1"}})
}

/// One `Buyer`-to-`Seller` protocol whose channel is `channel` and whose
/// message record is `record` with the field `field`.
fn protocol(channel: &str, record: &str, field: &str) -> String {
    format!(
        "record {record} {{ {field}; }}\n\
         protocol Flow using pp over (k: M::Core) on origin {{\n\
         role Buyer on M::Cell;\nrole Seller on M::Cell;\n\
         channel {channel} from Buyer to Seller carries {record} ordering unordered \
         delivery exactly-once-premise capacity 4 overflow reject;\n\
         run sequence Main {{\n\
         send Placed via {channel} as (placed: {record}) {{ true }};\n\
         receive Got via {channel} of Placed as (got: {record}) {{ true }};\n}}\n\
         finish Done as (zz: Boolean) {{ true }};\n}}\n"
    )
}

fn items() -> Vec<Value> {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    let subject = json!({"protocol": {"protocol": "Flow",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"Buyer": binding, "Seller": binding}}});
    vec![
        json!({"claim": "Flow", "kind": "realizability", "assessed_root": root(),
            "premises": {"role_knowledge": [], "environment": []},
            "temporal": {"profile": "bounded", "max_depth": 16, "max_history": 16,
                "subject": subject}}),
        json!({"claim": "Flow", "kind": "local-projection", "assessed_root": root(),
            "options": {"role": "Seller"},
            "temporal": {"profile": "bounded", "max_depth": 16, "subject": subject}}),
    ]
}

fn run(name: &str, text: &str) -> crate::analyze_cli_support::Run {
    let ws = Workspace::new(name);
    analyze(&litmus_request(&ws, text, json!({"items": items()})))
}

/// FR-570-AC-1: a role whose name another declaration of the unit shares
/// (here the channel) projects as the protocol's role.
#[trace("TC-1410", "FR-570-AC-1")]
#[test]
fn a_role_sharing_its_name_with_a_channel_projects() {
    let result = run(
        "docs-role-name-shared",
        &protocol("Seller", "Ack", "ok: Boolean"),
    );
    assert_eq!(label(&result.item(0)), "proved", "{}", result.item(0));
    assert_eq!(label(&result.item(1)), "proved", "{}", result.item(1));
}

/// FR-575-AC-2: an unenumerable step binder settles `evaluator-unsupported`
/// naming the `fsl-eval` cause and the clause it was found in.
#[trace("TC-1415", "FR-575-AC-2")]
#[test]
fn an_evaluator_refusal_names_its_cause_and_location() {
    let result = run(
        "docs-unenumerable-binder",
        &protocol("Order", "Txt", "s: String"),
    );
    let item = result.item(0);
    assert_eq!(item["category"], "unsupported", "{item}");
    let cause = &item["record"]["cause"];
    assert_eq!(cause["kind"], "evaluator-unsupported", "{item}");
    assert_eq!(cause["code"], "unsupported_projection", "{item}");
    assert_eq!(
        cause["message"], "step binder of an unenumerable record type",
        "{item}"
    );
    assert!(cause["where"].is_string(), "{item}");
}
