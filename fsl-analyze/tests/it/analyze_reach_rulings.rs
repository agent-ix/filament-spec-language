// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsl analyze` stubs for the analysis-reach rulings: the digital-clock
//! route (FR-632, FR-243), state-graph evidence and budget stops (FR-628),
//! instance order, replay targets and the record's `backend` (FR-621),
//! simulation traces and coverage (FR-634), process providers (FR-635) and
//! the linked registry (FR-489). TC-2290 to TC-2294, TC-2296, TC-1272.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, counter_request, counter_snapshot, label, request_over,
    staying_counter_request, Workspace,
};
use crate::counter::{document, unit_text, CONTRACT};

/// The first value under `key` anywhere in `value`, depth first.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(members) => members
            .get(key)
            .or_else(|| members.values().find_map(|member| find(member, key))),
        Value::Array(items) => items.iter().find_map(|item| find(item, key)),
        _ => None,
    }
}

/// A JSON string or number read as its decimal text.
fn decimal(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

/// The Counter request from `objects`' snapshot with `body` and `items`.
fn counters_request(
    ws: &Workspace,
    objects: &[(&str, i64)],
    body: &str,
    items: Value,
) -> std::path::PathBuf {
    request_over(
        ws,
        &unit_text(&format!("{CONTRACT}{body}")),
        &document(false),
        &[counter_snapshot(objects)],
        json!({"items": items}),
    )
}

/// FR-632-AC-4, FR-243-AC-4: the digital-clock route proves the closed
/// `Rpc` on EN-1 and refuses a strict guard or a half-open interval.
#[trace("TC-2290", "FR-632-AC-4")]
#[trace("TC-2290", "FR-243-AC-4")]
#[test]
#[ignore = "QSL-2129: the digital-clock route is not reachable through fsl analyze yet"]
fn digital_clocks_refuse_strict_constraints_and_intervals() {
    let claim = "temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
                 { always holds(not c.late) }\n\
                 temporal Soon using tt over (c: M::Call) clock \"model-time\" on origin \
                 { eventually[0 ms, 2 ms) holds(not c.late) }\n";
    let digital = json!({"options": {"method": "digital-clocks"}});
    let items = |name: &str| {
        let mut item = digital.clone();
        item["claim"] = json!(name);
        json!({"items": [item]})
    };
    let ws = Workspace::new("rulings-digital");
    let run = analyze(&crate::analyze_family_cli::rpc_request(
        &ws,
        4,
        claim,
        items("NoLateReply"),
    ));
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    assert_eq!(
        proved["record"]["basis"]["type"], "digitized-explicit-state",
        "{proved}"
    );
    assert_eq!(proved["record"]["backend"], "en-1", "{proved}");

    let ws = Workspace::new("rulings-digital-interval");
    let run = analyze(&crate::analyze_family_cli::rpc_request(
        &ws,
        4,
        claim,
        items("Soon"),
    ));
    let declined = run.item(0);
    assert_eq!(label(&declined), "declined", "{declined}");
    assert!(
        declined["record"].to_string().contains("invalid-input"),
        "{declined}"
    );
    // EN-1's digital-clock route declines before any search.
    assert_eq!(declined["record"]["settled_by"], "result", "{declined}");
    assert_eq!(declined["record"]["backend"], "en-1", "{declined}");

    let strict = crate::analyze_family_cli::rpc_unit(4, claim).replace(
        "self.inflight and self.x >= 1 ms",
        "self.inflight and self.x > 1 ms",
    );
    let ws = Workspace::new("rulings-digital-strict");
    let run = analyze(&request_over(
        &ws,
        &strict,
        &crate::zone_search::rpc_document(),
        &[crate::analyze_family_cli::rpc_snapshot()],
        items("NoLateReply"),
    ));
    let declined = run.item(0);
    assert_eq!(label(&declined), "declined", "{declined}");
    assert!(
        declined["record"].to_string().contains("invalid-input"),
        "{declined}"
    );
    // EN-1's digital-clock route declines before any search.
    assert_eq!(declined["record"]["settled_by"], "result", "{declined}");
    assert_eq!(declined["record"]["backend"], "en-1", "{declined}");
}

/// FR-628-AC-4, AC-5: the evidence's instance key is the typed canonical
/// universe key, the undefined arm carries its locus, and a budget stop
/// settles by the claim's form.
#[trace("TC-2291", "FR-628-AC-4")]
#[trace("TC-2291", "FR-628-AC-5")]
#[test]
#[ignore = "QSL-2125: state-graph evidence keys, loci and budget stops are not written as specified yet"]
fn state_graph_evidence_keys_loci_and_budget_stops() {
    let body = "possible ReachThree using t0 over (c: M::Counter) { c.value = 3 }\n\
                possible Ratio using t0 over (c: M::Counter) { 3 / (3 - c.value) = 1 }\n\
                always possible BackToZero using t0 over (c: M::Counter) { c.value = 0 }\n";
    let ws = Workspace::new("rulings-evidence");
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "ReachThree"}, {"claim": "Ratio"}]),
    ));
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    let instance = find(&proved, "instance").expect("the evidence names its instance");
    assert_eq!(instance["type"], "instance", "{instance}");
    assert_eq!(instance["parameter"], "c", "{instance}");
    assert!(
        instance["key"].get("type").is_some(),
        "the key is in typed canonical form: {instance}"
    );
    let undefined = run.item(1);
    assert_eq!(label(&undefined), "refuted", "{undefined}");
    let place = find(&undefined, "where").expect("the undefined arm's where");
    for member in ["position", "state_key", "locus"] {
        assert!(place.get(member).is_some(), "{member}: {place}");
    }
    assert_eq!(place["predicate"], "P", "{place}");

    let ws = Workspace::new("rulings-budget");
    let limited = json!({"limits": {"max_states": 2}});
    let run = analyze(&counter_request(
        &ws,
        body,
        json!([{"claim": "ReachThree", "temporal": limited.clone()},
            {"claim": "BackToZero", "temporal": limited}]),
    ));
    let witnessed = run.item(0);
    assert_eq!(label(&witnessed), "inconclusive", "{witnessed}");
    assert!(
        witnessed["record"]
            .to_string()
            .contains("well-definedness-unchecked"),
        "{witnessed}"
    );
    let stopped = run.item(1);
    assert_eq!(label(&stopped), "incomplete", "{stopped}");
    assert_eq!(
        stopped["record"]["cause"]["limit"], "max_states",
        "{stopped}"
    );
}

/// FR-621-AC-9: the first refuted instance in key order is reported, and a
/// refuted state-graph item names its claim as the replay target.
#[trace("TC-2292", "FR-621-AC-9")]
#[test]
#[ignore = "QSL-2125: instance order and the state-graph replay target are not written as specified yet"]
fn several_instances_report_the_first_refuted_and_state_graph_replay_names_the_claim() {
    let body = "temporal AtLeastOne using t0 over (c: M::Counter) clock \"model-steps\" on origin \
                { always holds(c.value >= 1) }\n";
    let ws = Workspace::new("rulings-instances");
    let path = counters_request(
        &ws,
        &[("a", 1), ("b", 0), ("d", 0)],
        body,
        json!([{"claim": "AtLeastOne"}]),
    );
    let first = analyze(&path);
    let refuted = first.item(0);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    assert!(
        refuted["counterexample"]["over_binding"]
            .to_string()
            .contains("\"b\""),
        "{refuted}"
    );
    assert_eq!(first.item(0), analyze(&path).item(0));

    let ws = Workspace::new("rulings-replay-target");
    let run = analyze(&counter_request(
        &ws,
        "always possible BackToZero using t0 over (c: M::Counter) { c.value = 0 }\n",
        json!([{"claim": "BackToZero"}]),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert!(
        item["replay"]["target"]["state_clause"]
            .as_str()
            .is_some_and(|name| name.ends_with("BackToZero")),
        "{item}"
    );
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-634-AC-4, AC-5: enumeration reports one trace per discovery-tree
/// leaf and counts choice points; sampling reports one trace per sample.
#[trace("TC-2293", "FR-634-AC-4")]
#[trace("TC-2293", "FR-634-AC-5")]
#[test]
#[ignore = "QSL-2132: simulation items are not reachable through fsl analyze yet"]
fn simulation_reports_discovery_tree_traces_and_choice_points() {
    let ws = Workspace::new("rulings-simulation");
    let run = analyze(&staying_counter_request(
        &ws,
        "",
        json!([{"kind": "simulation", "simulation": {"mode": "enumerate"}},
            {"kind": "simulation", "simulation": {"mode": "sample", "seed": 5, "samples": 3}}]),
    ));
    let enumerated = run.item(0);
    assert_eq!(enumerated["category"], "success", "{enumerated}");
    assert_eq!(enumerated["coverage"]["states"], "4", "{enumerated}");
    assert_eq!(enumerated["coverage"]["transitions"], "4", "{enumerated}");
    assert_eq!(enumerated["coverage"]["choices"], "1", "{enumerated}");
    assert_eq!(
        enumerated["traces"].as_array().map(Vec::len),
        Some(1),
        "{enumerated}"
    );
    let sampled = run.item(1);
    assert_eq!(
        sampled["traces"].as_array().map(Vec::len),
        Some(3),
        "{sampled}"
    );
    assert_eq!(decimal(&sampled["seed"]), "5", "{sampled}");
    // Coverage counts exactly what the three sampled traces visited.
    let traces = sampled["traces"].as_array().expect("traces");
    let mut states = std::collections::BTreeSet::new();
    let mut steps = std::collections::BTreeSet::new();
    for trace in traces {
        let visited = trace["states"].as_array().expect("trace states");
        for state in visited {
            states.insert(state.to_string());
        }
        for pair in visited.windows(2) {
            steps.insert(format!("{}->{}", pair[0], pair[1]));
        }
    }
    assert_eq!(
        sampled["coverage"]["states"],
        states.len().to_string(),
        "{sampled}"
    );
    assert_eq!(
        sampled["coverage"]["transitions"],
        steps.len().to_string(),
        "{sampled}"
    );
}

/// FR-635-AC-4: no process plugin joins the linked set FR-489's layering
/// snapshot is built from, and an analyze run with no configured plugin
/// lists only linked backends. The misadvertising-plugin half needs
/// `analyze_with_providers` and the workspace test plugin, which the coder
/// adds.
#[trace("TC-2294", "FR-635-AC-4")]
#[test]
#[ignore = "QSL-2132: process providers are not reachable through fsl analyze yet"]
fn a_misadvertising_plugin_is_refused_and_never_joins_the_layering_snapshot() {
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    assert!(engines
        .registrations()
        .all(|descriptor| !descriptor.id().as_str().starts_with("fsl.test-plugin")));
    let ws = Workspace::new("rulings-plugin");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"}]),
    ));
    let registry: Vec<String> = run.document()["registry"]
        .as_array()
        .expect("registry")
        .iter()
        .map(|entry| decimal(&entry["backend"]))
        .collect();
    assert!(
        registry.iter().any(|backend| backend == "en-1"),
        "{registry:?}"
    );
    assert!(
        registry
            .iter()
            .all(|backend| !backend.starts_with("fsl.test-plugin")),
        "{registry:?}"
    );
}

/// FR-621-AC-10: `backend` is present exactly when a result entry settles
/// the record.
#[trace("TC-2296", "FR-621-AC-10")]
#[test]
#[ignore = "QSL-2304: the analyze record does not write backend by settled_by yet"]
fn a_record_names_its_backend_exactly_when_a_result_settles_it() {
    let ws = Workspace::new("rulings-backend");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"}, {"claim": "Missing"}]),
    ));
    for index in [0, 1] {
        let record = &run.item(index)["record"];
        assert_eq!(record["settled_by"], "result", "{record}");
        assert_eq!(record["backend"], "en-1", "{record}");
    }
    let record = &run.item(2)["record"];
    assert_eq!(record["settled_by"], "disposition", "{record}");
    assert!(record.get("backend").is_none(), "{record}");
}

/// FR-489-AC-5: the linked registrations are exactly the EN-1, EN-4, EN-5,
/// EN-6, EN-7, check-arm and protocol-analysis descriptors.
#[trace("TC-1272", "FR-489-AC-5")]
#[test]
#[ignore = "QSL-2293: EN-7 is not registered in Engines::fsl() yet"]
fn the_linked_registrations_include_en7() {
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    let mut ids: Vec<&str> = engines
        .registrations()
        .map(|descriptor| descriptor.id().as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        [
            "contract-checker",
            "en-1",
            "en-4",
            "en-5",
            "en-6",
            "en-7",
            "protocol-analysis",
            "value-checker"
        ]
    );
}
