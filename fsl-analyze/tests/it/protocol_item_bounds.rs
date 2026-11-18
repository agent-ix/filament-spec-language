// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636-AC-5 to AC-7 (TC-2988): `local-projection` and `realizability`
//! items carry `temporal.profile`, `temporal.max_depth` and (realizability
//! only) `temporal.max_history` as key members with no default.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Run, Workspace};

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

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {"protocol": "Flow",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"Buyer": binding, "Seller": binding}}})
}

fn realizability(mut temporal: Value) -> Value {
    temporal["subject"] = subject();
    json!({"claim": "Flow", "kind": "realizability", "assessed_root": root(),
        "premises": {"role_knowledge": [], "environment": []}, "temporal": temporal})
}

fn projection(mut temporal: Value) -> Value {
    temporal["subject"] = subject();
    json!({"claim": "Flow", "kind": "local-projection", "assessed_root": root(),
        "options": {"role": "Seller"}, "temporal": temporal})
}

fn run_items(name: &str, items: Vec<Value>) -> Run {
    let ws = Workspace::new(name);
    analyze(&litmus_request(
        &ws,
        &order_protocol(true),
        json!({ "items": items }),
    ))
}

fn assert_refused(item: &Value, cause: &str, member: &str) {
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(item["record"]["diagnostic"]["cause"], cause, "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["member"], member,
        "{item}"
    );
}

/// FR-636-AC-5: complete items settle under both profile words.
#[trace("TC-2988", "FR-636-AC-5")]
#[test]
fn complete_protocol_items_settle_under_both_profiles() {
    for profile in ["bounded-static", "bounded"] {
        let run = run_items(
            &format!("protocol-bounds-{profile}"),
            vec![
                realizability(json!({"profile": profile, "max_depth": 16, "max_history": 16})),
                projection(json!({"profile": profile, "max_depth": 16})),
            ],
        );
        assert_eq!(run.exit, 0, "{}", run.document());
        assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
        assert_eq!(label(&run.item(1)), "proved", "{}", run.item(1));
    }
}

/// FR-636-AC-6: an absent key member settles its item `missing-member`; the
/// other items still run.
#[trace("TC-2988", "FR-636-AC-6")]
#[test]
fn an_absent_profile_or_bound_settles_missing_member() {
    let run = run_items(
        "protocol-bounds-absent",
        vec![
            realizability(json!({"profile": "bounded-static", "max_depth": 16})),
            realizability(json!({"max_depth": 16, "max_history": 16})),
            realizability(json!({"profile": "bounded-static", "max_history": 16})),
            projection(json!({"max_depth": 16})),
            realizability(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 16})),
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    for (index, member) in [
        (0, "temporal.max_history"),
        (1, "temporal.profile"),
        (2, "temporal.max_depth"),
        (3, "temporal.profile"),
    ] {
        assert_refused(&run.item(index), "missing-member", member);
    }
    assert_eq!(label(&run.item(4)), "proved", "{}", run.item(4));
}

/// FR-636-AC-7: an out-of-range profile or bound settles `invalid-value`,
/// and `max_history` on a projection settles `unknown-member`.
#[trace("TC-2988", "FR-636-AC-7")]
#[test]
fn an_out_of_range_profile_or_bound_is_refused_naming_it() {
    let run = run_items(
        "protocol-bounds-range",
        vec![
            realizability(json!({"profile": "static", "max_depth": 16, "max_history": 16})),
            realizability(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 0})),
            realizability(json!({"profile": "bounded-static", "max_depth": 0, "max_history": 16})),
            projection(json!({"profile": "bounded-static", "max_depth": 16, "max_history": 16})),
        ],
    );
    assert_eq!(run.exit, 20, "{}", run.document());
    assert_refused(&run.item(0), "invalid-value", "temporal.profile");
    assert_refused(&run.item(1), "invalid-value", "temporal.max_history");
    assert_refused(&run.item(2), "invalid-value", "temporal.max_depth");
    assert_refused(&run.item(3), "unknown-member", "temporal.max_history");
}
