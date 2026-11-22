// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 (TC-1596): the protocol deadlock verdict `fsl analyze` gives agrees
//! with local projection and realizability of the same protocol.

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Workspace};

/// FR-636-AC-2: global deadlock freedom agrees with projection.
#[trace("TC-1596", "FR-636-AC-2")]
#[test]
fn protocol_deadlock_agrees_with_projection() {
    let root = root();
    for receive in [true, false] {
        let ws = Workspace::new(&format!("protocol-differential-{receive}"));
        let binding = json!({"type": "model-binding", "selection": "test/litmus"});
        let subject = json!({"protocol": {"protocol": "Flow",
            "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
            "roles": {"Buyer": binding, "Seller": binding}}});
        let run = analyze(&litmus_request(
            &ws,
            &order_protocol(receive),
            json!({"items": [
                {"claim": "Flow", "kind": "global-conformance",
                 "temporal": {"item_kind": "deadlock-freedom", "subject": subject}},
                {"claim": "Flow", "kind": "local-projection", "assessed_root": root,
                 "options": {"role": "Seller"},
                 "temporal": {"profile": "bounded-static", "max_depth": 16, "subject": subject}},
                {"claim": "Flow", "kind": "realizability", "assessed_root": root,
                 "premises": {"role_knowledge": [], "environment": []},
                 "temporal": {"profile": "bounded-static", "max_depth": 16, "max_history": 16, "subject": subject}},
            ]}),
        ));
        let global = label(&run.item(0)) == "proved";
        let projection_realizable =
            label(&run.item(1)) == "proved" && label(&run.item(2)) == "proved";
        assert_eq!(global, projection_realizable, "{}", run.document());
    }
}

fn root() -> serde_json::Value {
    let universe = crate::fixture_ids::universe_id("u");
    let effective = crate::fixture_ids::effective_id("e");
    let universe = fsl_foundation::node_wire::node_key_to_text(universe.root_type())
        .expect("universe identity encodes");
    let effective = serde_json::Value::Array(
        effective
            .owner_chain()
            .iter()
            .map(|original| json!({"package": original.package(), "node": original.node()}))
            .collect(),
    );
    json!({"observation": {"authority": "test", "identity": "protocol-claim"},
        "subject": {"tag": "object", "universe": universe, "type": effective.to_string(), "object_identity": "flow-1"}})
}

/// FR-891-AC-2 (TC-2523): a symbolic capacity takes its value from the
/// subject's FSpec FR-785 `capacities` binding. Unbound, realizability is
/// `inconclusive`, `premise-not-stated`, whose FR-331 wire spelling carries
/// no payload; bound to 1 it settles as the literal capacity 1 does.
#[trace("TC-2523", "FR-891-AC-2", "FSpec-FR-785-AC-6")]
#[test]
#[ignore = "QSL-2885: the Flow claim is refused invalid_runtime_input/missing-member `temporal.profile` before the capacities binding is read"]
fn a_symbolic_capacity_settles_from_the_capacities_binding() {
    let root = json!({"observation": {"authority": "test", "identity": "protocol-claim"},
        "subject": {"tag": "object", "universe": "u", "type": "e", "object_identity": "flow-1"}});
    let item = json!({"claim": "Flow", "kind": "realizability", "assessed_root": root});
    let symbolic = order_protocol(true).replace("capacity 4", "capacity symbolic(Bound)");
    let run = |name: &str, unit: &str, extra: serde_json::Value| {
        let ws = Workspace::new(name);
        let mut members = json!({"items": [item.clone()]});
        for (key, value) in extra.as_object().expect("extra members") {
            members[key] = value.clone();
        }
        analyze(&litmus_request(&ws, unit, members)).item(0)
    };
    let unbound = run("capacity-unbound", &symbolic, json!({}));
    assert_eq!(label(&unbound), "inconclusive", "{unbound}");
    let text = unbound.to_string();
    assert!(text.contains("premise-not-stated"), "{text}");
    assert!(!text.contains("requires-bound"), "{text}");
    let bound = run(
        "capacity-bound",
        &symbolic,
        json!({"subject": {"capacities": {"Bound": "1"}}}),
    );
    let literal = run(
        "capacity-literal",
        &order_protocol(true).replace("capacity 4", "capacity 1"),
        json!({}),
    );
    assert_eq!(label(&bound), label(&literal), "{bound} {literal}");
}
