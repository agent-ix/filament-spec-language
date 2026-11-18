// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-990 (TC-2955): a recording-mode `global-conformance` item enumerates
//! each fan-out member from its `population` member, the activated role
//! instances the request supplies, never from the members the observations
//! name.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, Run, Workspace};

/// FR-990-AC-7's fan-out fixture: replicated role `r` over `cells`, with a
/// `parallel` of `left` and `right` joined `all`.
const FANOUT: &str = "protocol R using pp over (k: M::Core) on origin {\n\
     role w on M::Cell;\n\
     role r each M::Cell from cells max 2 lifetime workflow;\n\
     run sequence Main { parallel Both {\n\
       branch left sequence Left { event LeftWork by r as (left_worked: Boolean) { true }; }\n\
       branch right sequence Right { event RightWork by r as (right_worked: Boolean) { true }; }\n\
     } join all [left, right]; }\n\
     finish Done as (d: Boolean) { true };\n}\n";

const LEFT: &str = "R::Main::Both::left";
const RIGHT: &str = "R::Main::Both::right";

fn member(identity: &str) -> Value {
    json!({"authority": "test", "identity": identity})
}

fn work(node: &str, identity: &str) -> Value {
    let event = if node == LEFT {
        "Left::LeftWork"
    } else {
        "Right::RightWork"
    };
    json!({"kind": "event", "node": format!("{node}::{event}"), "role": "r",
           "role_instance": member(identity), "keys": {}, "record": true})
}

fn premise(node: &str, identity: &str) -> Value {
    json!({"kind": "branch", "node": node, "role_instance": member(identity)})
}

/// A recording of the members `observed`, with closure `state` stating the
/// branch premises of `premised`.
fn recording(observed: &[&str], premised: &[&str], state: &str) -> Value {
    let mut observations: Vec<Value> = observed
        .iter()
        .flat_map(|identity| [work(LEFT, identity), work(RIGHT, identity)])
        .collect();
    let finish_position = observations.len();
    observations.push(json!({"kind": "finish", "node": "R::Done", "role": "w",
                             "keys": {}, "record": true}));
    let order: Vec<usize> = (0..finish_position).collect();
    let mut premises = vec![json!({"kind": "execution", "node": "R::Done"})];
    for node in [LEFT, RIGHT] {
        premises.extend(premised.iter().map(|identity| premise(node, identity)));
    }
    json!({
        "format": "filament.protocol-recording/1", "protocol": "R",
        "observations": observations, "local_order": {"r": order, "w": [finish_position]}, "correlations": [],
        "closure": {"state": state, "premises": premises}
    })
}

fn known() -> Value {
    json!([{"role": "r", "members": [member("x"), member("y")]}])
}

/// Analyze `recording` with the item's `population` member, when given.
fn run(name: &str, recording: &Value, population: Option<Value>) -> Run {
    let ws = Workspace::new(name);
    ws.write("recording.json", recording.to_string().as_bytes());
    let mut item = json!({"claim": "R", "kind": "global-conformance",
                          "recording": {"authority": "test", "identity": "recording"}});
    if let Some(population) = population {
        item["population"] = population;
    }
    analyze(&litmus_request(
        &ws,
        FANOUT,
        json!({"items": [item],
               "documents": [{"file": "recording.json",
                              "identity": {"authority": "test", "identity": "recording"}}]}),
    ))
}

fn member_control<'a>(item: &'a Value, node: &str, identity: &str) -> &'a Value {
    item["obligations"]
        .as_array()
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry["obligation"]["kind"] == "control"
                    && entry["obligation"]["node"] == node
                    && entry["obligation"]["role_instance"] == member(identity)
            })
        })
        .unwrap_or_else(|| panic!("a control obligation for {identity} at {node}: {item}"))
}

/// FR-990-AC-7: every supplied member observed and premised settles `proved`.
#[trace("TC-2955", "FR-990-AC-7")]
#[test]
fn every_supplied_member_observed_and_premised_proves_the_recording() {
    let run = run(
        "population-all-present",
        &recording(&["x", "y"], &["x", "y"], "closed"),
        Some(known()),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["global"]["complete"], true, "{item}");
    for node in [LEFT, RIGHT] {
        for identity in ["x", "y"] {
            assert_eq!(
                member_control(&item, node, identity)["verdict"],
                "satisfied"
            );
        }
    }
}

/// FR-990-AC-8: a member absent from the recording is a refused premise set
/// or a `missing` violation, never incomplete closure.
#[trace("TC-2955", "FR-990-AC-8")]
#[test]
fn a_member_absent_from_the_recording_is_refused_or_refuted() {
    let omitted = run(
        "population-omitted-premise",
        &recording(&["x"], &["x"], "closed"),
        Some(known()),
    );
    let item = omitted.item(0);
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    let detail = item["record"]["diagnostic"]["detail"].to_string();
    assert!(detail.contains(LEFT) && detail.contains("\"y\""), "{item}");
    assert!(item.get("obligations").is_none(), "{item}");

    let premised = run(
        "population-unobserved-member",
        &recording(&["x"], &["x", "y"], "closed"),
        Some(known()),
    );
    let item = premised.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    for node in [LEFT, RIGHT] {
        let entry = member_control(&item, node, "y");
        assert_eq!(entry["verdict"], "violated", "{item}");
        assert_eq!(entry["cause"], "missing", "{item}");
    }

    let open = run(
        "population-unobserved-open",
        &recording(&["x"], &["x", "y"], "open"),
        Some(known()),
    );
    let item = open.item(0);
    assert_eq!(label(&item), "inconclusive", "{item}");
    assert_eq!(item["global"]["cause"], "open-history", "{item}");
    assert_eq!(member_control(&item, LEFT, "y")["verdict"], "pending");
}

/// FR-990-AC-9: without a population the member set is unknown, and the
/// item is `incomplete-closure-scope`, never `proved`.
#[trace("TC-2955", "FR-990-AC-9")]
#[test]
fn an_unknown_population_leaves_closure_scope_incomplete() {
    let run = run(
        "population-unknown",
        &recording(&["x", "y"], &["x", "y"], "closed"),
        None,
    );
    let item = run.item(0);
    assert_eq!(label(&item), "inconclusive", "{item}");
    assert_eq!(
        item["global"]["cause"], "incomplete-closure-scope",
        "{item}"
    );
    assert_eq!(item["global"]["complete"], false, "{item}");
    assert_eq!(item["closure"]["state"], "closed", "{item}");
    for node in [LEFT, RIGHT] {
        for identity in ["x", "y"] {
            assert_eq!(member_control(&item, node, identity)["verdict"], "pending");
        }
    }
}

/// FR-990-AC-10: defective populations refuse the item or the request.
#[trace("TC-2955", "FR-990-AC-10")]
#[test]
fn defective_populations_refuse_the_item_or_the_request() {
    let complete = recording(&["x", "y"], &["x", "y"], "closed");
    let item_refusals = [
        (
            json!([{"role": "q", "members": [member("x")]}]),
            "missing-name",
        ),
        (
            json!([{"role": "r", "instance": "w-1", "members": [member("x"), member("y")]}]),
            "invalid-value",
        ),
        (
            json!([{"role": "r", "members": [member("x"), member("x"), member("y")]}]),
            "invalid-value",
        ),
        (
            json!([{"role": "r", "members": [member("x")]}]),
            "invalid-value",
        ),
        (json!([]), "missing-member"),
    ];
    for (index, (population, subcode)) in item_refusals.into_iter().enumerate() {
        let run = run(
            &format!("population-item-refusal-{index}"),
            &complete,
            Some(population),
        );
        let item = run.item(0);
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert!(
            item["record"]["diagnostic"].to_string().contains(subcode),
            "{item}"
        );
        assert!(item.get("obligations").is_none(), "{item}");
    }

    for (index, population) in [
        json!({"role": "r"}),
        json!([{"role": "r", "members": [{"authority": "test"}]}]),
    ]
    .into_iter()
    .enumerate()
    {
        let run = run(
            &format!("population-request-refusal-{index}"),
            &complete,
            Some(population),
        );
        assert_eq!(run.exit, 20);
        let document = run.document();
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid-request",
            "{document}"
        );
        assert!(
            document.to_string().contains("/items/0/population"),
            "{document}"
        );
    }

    let ws = Workspace::new("population-without-recording");
    let run = analyze(&litmus_request(
        &ws,
        FANOUT,
        json!({"items": [{"claim": "R", "kind": "global-conformance", "population": known()}]}),
    ));
    assert_eq!(run.exit, 20);
    assert!(run.document().to_string().contains("/items/0/population"));
}
