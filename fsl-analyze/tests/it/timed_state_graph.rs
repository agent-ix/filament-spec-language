// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-852 and FSpec FR-712 (TC-2412): `possible`, `always possible` and
//! `unique path` items over the timed `Rpc` subject route to EN-6 and are
//! decided over the timed state graph on zones, with exact-rational evidence
//! that replays and a zone certificate.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_cli::{rpc_snapshot, rpc_unit};

const CLAIMS: &str = "possible ReplyPossible using v over (c: M::Call) { c.phase = 2 }\n\
    possible LatePossible using v over (c: M::Call) { c.phase = 3 }\n\
    possible IdlePossible using v over (c: M::Call) { c.phase = 0 }\n\
    always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }\n\
    unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);\n";

/// The `Rpc` unit at timeout 4 with [`CLAIMS`]; without `reset`, the
/// `reset` precondition is `false`, so no step leaves `phase` 2 or 3.
fn unit(reset: bool) -> String {
    let text = rpc_unit(4, CLAIMS);
    if reset {
        text
    } else {
        text.replace(
            "not self.inflight and (self.phase = 2 or self.phase = 3)",
            "false",
        )
    }
}

fn run(name: &str, reset: bool, items: Value) -> Run {
    let ws = Workspace::new(name);
    analyze(&request_over(
        &ws,
        &unit(reset),
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": items}),
    ))
}

fn rpc_empty_snapshot() -> Vec<u8> {
    let mut snapshot: Value = serde_json::from_slice(&rpc_snapshot()).expect("snapshot JSON");
    snapshot["populations"][0]["objects"] = json!([]);
    snapshot.to_string().into_bytes()
}

/// FR-852-AC-1, FSpec FR-712-AC-1: `Replied` proves on `en-6` with a
/// replayed, certified witness `send` then `reply`; `Late` is unreachable
/// within the server deadline and refutes.
#[trace("TC-2412", "FR-852-AC-1")]
#[trace("TC-2412", "FSpec-FR-712-AC-1")]
#[test]
fn a_timed_possible_claim_proves_on_zones_and_refutes() {
    let run = run(
        "timed-sg-possible",
        true,
        json!([{"claim": "ReplyPossible"}, {"claim": "LatePossible"}]),
    );
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    assert_eq!(proved["record"]["backend"], "en-6", "{proved}");
    assert_eq!(
        proved["record"]["basis"]["type"], "zone-certified",
        "{proved}"
    );
    assert_eq!(proved["record"]["certification"], "certified", "{proved}");
    let path = &proved["record"]["evidence"]["paths"][0]["path"];
    assert_eq!(path["steps"].as_array().map(Vec::len), Some(2), "{proved}");
    assert_eq!(label(&run.item(1)), "refuted", "{}", run.item(1));
}

/// FR-852-AC-2, FSpec FR-712-AC-2: `BackToIdle` proves; with `reset`
/// disabled it refutes with a path ending at `phase` 2. The proof's accepted
/// certificate carries the `E(P)` zones in its `state_graph` part.
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FSpec-FR-712-AC-2")]
#[test]
fn a_timed_always_possible_claim_proves_and_refutes() {
    let items = json!([{"claim": "BackToIdle"}]);
    let proved = run("timed-sg-always", true, items.clone());
    let item = proved.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(
        item["certificate"]["document"]["state_graph"]["form"],
        "always-possible",
        "{item}"
    );
    assert!(
        item["certificate"]["document"]["state_graph"]["ef_nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "the proof certificate carries the E(P) zones: {item}"
    );
    let refuted = run("timed-sg-always-no-reset", false, items);
    let item = refuted.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    assert_eq!(item["certificate"]["accepted"], false, "{item}");
    assert_eq!(
        item["certificate"]["document"]["state_graph"]["form"],
        "always-possible",
        "{item}"
    );
}

/// A state-graph certificate establishes reachability and the claim's graph
/// predicate. It remains valid when the timed graph has only zero-time
/// behavior; FR-244's non-Zeno witness belongs to liveness certificates.
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FSpec-FR-712-AC-2")]
#[test]
fn a_zeno_only_timed_graph_accepts_state_graph_certificate() {
    let frozen = unit(true).replace(
        "time invariant ServerDeadline",
        "time invariant Frozen when { true } { self.x <= 0 ms }\n\
         time invariant ServerDeadline",
    );
    let ws = Workspace::new("timed-sg-zeno-only");
    let run = analyze(&request_over(
        &ws,
        &frozen,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": [{"claim": "IdlePossible"}]}),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-6", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(
        item["certificate"]["document"]["state_graph"]["form"],
        "possible",
        "{item}"
    );
    assert!(item["certificate"]["document"]["admitted"].is_null(), "{item}");
}

/// FR-852-AC-3, FSpec FR-712-AC-3: the `reset` cycle gives `OneReply`
/// infinitely many paths; without it the one step sequence `send`, `reply`
/// is one path whatever the delay before `reply`.
#[trace("TC-2412", "FR-852-AC-3")]
#[trace("TC-2412", "FSpec-FR-712-AC-3")]
#[test]
fn a_timed_unique_path_counts_step_sequences() {
    let items = json!([{"claim": "OneReply"}]);
    let cycle_unit = unit(true).replace("self.x <= 3 ms", "self.x <= 4 ms");
    let ws = Workspace::new("timed-sg-unique");
    let cyclic = analyze(&request_over(
        &ws,
        &cycle_unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": items.clone()}),
    ));
    assert_eq!(label(&cyclic.item(0)), "refuted", "{}", cyclic.item(0));
    let acyclic = run("timed-sg-unique-no-reset", false, items);
    assert_eq!(label(&acyclic.item(0)), "proved", "{}", acyclic.item(0));
}

/// FR-852-AC-4, FSpec FR-712-AC-4 (negative): EN-1 is no candidate for a
/// timed state-graph item, and a state budget settles `incomplete`.
#[trace("TC-2412", "FR-852-AC-4")]
#[trace("TC-2412", "FSpec-FR-712-AC-4")]
#[test]
fn en_1_is_no_candidate_and_a_budget_stops_the_zone_search() {
    let named = run(
        "timed-sg-en1",
        true,
        json!([{"claim": "ReplyPossible", "backend": "en-1"}]),
    );
    let item = named.item(0);
    assert_eq!(label(&item), "unsupported", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unsupported-requested-capability",
        "{item}"
    );
    assert_eq!(named.exit, 21);
    let limited = run(
        "timed-sg-budget",
        true,
        json!([{"claim": "ReplyPossible", "temporal": {"limits": {"max_states": 1}}}]),
    );
    let item = limited.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(
        item["record"]["cause"]["limit"],
        "max_states",
        "{item}"
    );
}

/// FR-852-AC-5: EN-6 advertises the three state-graph forms.
#[trace("TC-2412", "FR-852-AC-5")]
#[test]
fn en_6_advertises_the_state_graph_forms() {
    use fsl_route::StateGraphFormKind;
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    let en6 = engines
        .registrations()
        .find(|descriptor| descriptor.id().as_str() == "en-6")
        .expect("en-6 registers");
    for form in [
        StateGraphFormKind::Possible,
        StateGraphFormKind::AlwaysPossible,
        StateGraphFormKind::UniquePath,
    ] {
        assert!(en6.state_graph_forms().contains(&form), "{form:?}");
    }
}

/// FR-852-AC-6: with `options.method` `digital-clocks` the closed `Rpc`
/// subject's state-graph items settle on `en-1` over the digitized subject,
/// the proof with basis `exhaustive`; EN-1 lists the forms.
#[trace("TC-2412", "FR-852-AC-6")]
#[trace("TC-2412", "FSpec-FR-712-AC-4")]
#[test]
fn digital_clocks_decide_timed_state_graph_items_on_en_1() {
    let run = run(
        "timed-sg-digital",
        true,
        json!([
            {"claim": "ReplyPossible", "options": {"method": "digital-clocks"}},
            {"claim": "LatePossible", "options": {"method": "digital-clocks"}}
        ]),
    );
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    assert_eq!(proved["record"]["backend"], "en-1", "{proved}");
    assert_eq!(proved["record"]["basis"]["type"], "exhaustive", "{proved}");
    let evidence = &proved["record"]["evidence"];
    assert_eq!(evidence["arm"], "witness", "{proved}");
    let paths = evidence["paths"].as_array().expect("witness paths");
    assert!(!paths.is_empty(), "{proved}");
    assert!(paths.iter().all(|entry| {
        entry["path"]["steps"]
            .as_array()
            .is_some_and(|steps| steps.iter().all(|step| step["delay"]["denominator"] == "1"))
    }), "digital-clock witnesses use integral delays: {proved}");
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    assert_eq!(refuted["record"]["backend"], "en-1", "{refuted}");
    assert_eq!(refuted["replay"]["reproduced"], true, "{refuted}");

    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    let en1 = engines
        .registrations()
        .find(|descriptor| descriptor.id().as_str() == "en-1")
        .expect("en-1 registers");
    assert_eq!(en1.state_graph_forms().len(), 3);
}

/// FR-852-AC-6: EN-1 decides the other two state-graph forms on the integer
/// grid and replays their negative evidence.
#[trace("TC-2412", "FR-852-AC-6")]
#[trace("TC-2412", "FSpec-FR-712-AC-4")]
#[test]
fn digital_clocks_decide_always_possible_and_unique_path() {
    let method = json!({"method": "digital-clocks"});
    let always = run(
        "timed-sg-digital-always",
        true,
        json!([{"claim": "BackToIdle", "options": method.clone()}]),
    );
    let item = always.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(item["record"]["basis"]["type"], "exhaustive", "{item}");

    let always_refuted = run(
        "timed-sg-digital-always-refuted",
        false,
        json!([{"claim": "BackToIdle", "options": method.clone()}]),
    );
    let item = always_refuted.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");

    let unique = run(
        "timed-sg-digital-unique",
        false,
        json!([{"claim": "OneReply", "options": method.clone()}]),
    );
    let item = unique.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");

    let cyclic_unit = unit(true).replace("self.x <= 3 ms", "self.x <= 4 ms");
    let ws = Workspace::new("timed-sg-digital-unique-refuted");
    let unique_refuted = analyze(&request_over(
        &ws,
        &cyclic_unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": [{"claim": "OneReply", "options": method}]}),
    ));
    let item = unique_refuted.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-243: EN-1 refuses an open timed state-graph subject before search.
#[trace("TC-2412", "FR-852-AC-6")]
#[test]
fn digital_clocks_decline_a_strict_timed_subject() {
    let strict = unit(true).replace(
        "self.inflight and self.x >= 1 ms",
        "self.inflight and self.x > 1 ms",
    );
    let run = analyze(&request_over(
        &Workspace::new("timed-sg-digital-strict"),
        &strict,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": [{"claim": "ReplyPossible",
            "options": {"method": "digital-clocks"}}]}),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(item["record"]["settled_by"], "result", "{item}");
    assert!(item["record"].to_string().contains("strict-clock-constraint"), "{item}");
    assert!(item.get("exploration").is_none(), "no search runs: {item}");
}

/// FR-125: timed state-graph items settle NoInstance or NoInitialState
/// before exploring, through either timed route.
#[trace("TC-2412")]
#[trace("FSpec-FR-125-AC-7")]
#[test]
fn timed_state_graph_items_report_empty_instance_and_initial_causes() {
    for options in [json!({}), json!({"method": "digital-clocks"})] {
        let empty_universe = analyze(&request_over(
            &Workspace::new("timed-sg-no-instance"),
            &unit(true),
            &crate::zone_search::rpc_document(),
            &[rpc_empty_snapshot()],
            json!({"items": [{"claim": "ReplyPossible", "options": options.clone()}]}),
        ));
        let item = empty_universe.item(0);
        assert_eq!(label(&item), "inconclusive", "{item}");
        assert_eq!(item["record"]["cause"]["kind"], "no-instance", "{item}");
        assert_eq!(item["exploration"]["states"], "0", "{item}");

        let start = json!({"authority": "test", "identity": "s0"});
        let no_initial = analyze(&request_over(
            &Workspace::new("timed-sg-no-initial"),
            &unit(true),
            &crate::zone_search::rpc_document(),
            &[rpc_snapshot()],
            json!({"items": [{"claim": "ReplyPossible", "options": options}],
                "subject": {"initial_states": [], "universes": [start], "proof_bounds": []}}),
        ));
        let item = no_initial.item(0);
        assert_eq!(label(&item), "inconclusive", "{item}");
        assert_eq!(item["record"]["cause"]["kind"], "no-initial-state", "{item}");
        assert_eq!(item["exploration"]["states"], "0", "{item}");
    }
}

/// The Zeno-only `Rpc` variant of FR-244-AC-7: the time invariant
/// `x <= 0 ms` holds at every state and `reply` needs `x >= 0 ms`, so no
/// positive delay is ever admissible and the only infinite path is the
/// zero-delay `send`, `reply`, `reset` cycle.
fn zeno_only_unit() -> String {
    let claims = format!(
        "{CLAIMS}temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always holds(not c.late) }}\n"
    );
    crate::analyze_family_cli::rpc_unit_with(4, ">= 0", ">=", &claims).replace(
        "time invariant ServerDeadline when { self.inflight } { self.x <= 3 ms }",
        "time invariant ServerDeadline when { true } { self.x <= 0 ms }",
    )
}

/// FR-244-AC-7, FR-245-AC-11: a state-graph item's certificate certifies the
/// reachable timed graph with no `admitted` witness and is accepted over a
/// Zeno-only graph; a temporal claim over the same subject still settles
/// `NoAdmittedBehaviour` (negative: its witness requirement stays).
#[trace("TC-3030", "FR-244-AC-7")]
#[trace("TC-3030", "FR-245-AC-11")]
#[test]
#[ignore = "QSL-2305: a state-graph zone certificate over a Zeno-only graph is not emitted and accepted without the admitted witness yet"]
fn a_state_graph_certificate_over_a_zeno_only_graph_needs_no_divergence_witness() {
    let unit = zeno_only_unit();
    assert!(unit.contains("when { true } { self.x <= 0 ms }"), "{unit}");
    let ws = Workspace::new("timed-sg-zeno-only");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": [
            {"claim": "Replied"},
            {"claim": "BackToIdle"},
            {"claim": "NoLateReply"}
        ]}),
    ));
    for index in [0, 1] {
        let item = run.item(index);
        assert_eq!(label(&item), "proved", "{item}");
        assert_eq!(item["certificate"]["accepted"], true, "{item}");
        assert_eq!(item["record"]["certification"], "certified", "{item}");
        assert!(
            item["certificate"]["document"].get("admitted").is_none(),
            "a state-graph certificate carries no admitted witness: {item}"
        );
    }
    assert!(
        run.item(1)["certificate"]["document"]["state_graph"]["ef_nodes"]
            .as_array()
            .is_some_and(|nodes| !nodes.is_empty()),
        "{}",
        run.item(1)
    );
    let temporal = run.item(2);
    assert_eq!(label(&temporal), "inconclusive", "{temporal}");
    assert!(
        temporal.to_string().contains("no-admitted-behaviour"),
        "{temporal}"
    );
    assert!(temporal["certificate"].is_null(), "{temporal}");
}
