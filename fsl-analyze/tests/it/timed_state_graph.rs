// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-852 and FSpec FR-712 (TC-2412): `possible`, `always possible` and
//! `unique path` items over the timed `Rpc` subject route to EN-6 and are
//! decided over the timed state graph on zones, with exact-rational evidence
//! that replays and a zone certificate.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_cli::{rpc_snapshot, rpc_unit};

const CLAIMS: &str = "possible Replied using v over (c: M::Call) { c.phase = 2 }\n\
    possible Late using v over (c: M::Call) { c.phase = 3 }\n\
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

/// FR-852-AC-1, FSpec FR-712-AC-1: `Replied` proves on `en-6` with a
/// replayed, certified witness `send` then `reply`; `Late` is unreachable
/// within the server deadline and refutes.
#[trace("TC-2412", "FR-852-AC-1")]
#[trace("TC-2412", "FSpec-FR-712-AC-1")]
#[test]
#[ignore = "QSL-2305: EN-6 lists no state-graph form, so a timed state-graph item settles unsupported"]
fn a_timed_possible_claim_proves_on_zones_and_refutes() {
    let run = run(
        "timed-sg-possible",
        true,
        json!([{"claim": "Replied"}, {"claim": "Late"}]),
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
#[ignore = "QSL-2305: EN-6 lists no state-graph form, so a timed state-graph item settles unsupported"]
fn a_timed_always_possible_claim_proves_and_refutes() {
    let items = json!([{"claim": "BackToIdle"}]);
    let proved = run("timed-sg-always", true, items.clone());
    let item = proved.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
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
}

/// FR-852-AC-3, FSpec FR-712-AC-3: the `reset` cycle gives `OneReply`
/// infinitely many paths; without it the one step sequence `send`, `reply`
/// is one path whatever the delay before `reply`.
#[trace("TC-2412", "FR-852-AC-3")]
#[trace("TC-2412", "FSpec-FR-712-AC-3")]
#[test]
#[ignore = "QSL-2305: EN-6 lists no state-graph form, so a timed state-graph item settles unsupported"]
fn a_timed_unique_path_counts_step_sequences() {
    let items = json!([{"claim": "OneReply"}]);
    let cyclic = run("timed-sg-unique", true, items.clone());
    assert_eq!(label(&cyclic.item(0)), "refuted", "{}", cyclic.item(0));
    let acyclic = run("timed-sg-unique-no-reset", false, items);
    assert_eq!(label(&acyclic.item(0)), "proved", "{}", acyclic.item(0));
}

/// FR-852-AC-4, FSpec FR-712-AC-4 (negative): EN-1 is no candidate for a
/// timed state-graph item, and a state budget settles `incomplete`.
#[trace("TC-2412", "FR-852-AC-4")]
#[trace("TC-2412", "FSpec-FR-712-AC-4")]
#[test]
#[ignore = "QSL-2305: EN-6 lists no state-graph form, so a timed state-graph item settles unsupported"]
fn en_1_is_no_candidate_and_a_budget_stops_the_zone_search() {
    let named = run(
        "timed-sg-en1",
        true,
        json!([{"claim": "Replied", "backend": "en-1"}]),
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
        json!([{"claim": "Replied", "temporal": {"limits": {"max_states": 1}}}]),
    );
    let item = limited.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
}

/// FR-852-AC-5: EN-6 advertises the three state-graph forms.
#[trace("TC-2412", "FR-852-AC-5")]
#[test]
#[ignore = "QSL-2305: EN-6 lists no state-graph form"]
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
/// the proof with basis `digitized-explicit-state`; EN-1 lists the forms.
#[trace("TC-2412", "FR-852-AC-6")]
#[trace("TC-2412", "FSpec-FR-712-AC-4")]
#[test]
#[ignore = "QSL-2305: a timed state-graph item has no digital-clock route"]
fn digital_clocks_decide_timed_state_graph_items_on_en_1() {
    let run = run(
        "timed-sg-digital",
        true,
        json!([
            {"claim": "Replied", "options": {"method": "digital-clocks"}},
            {"claim": "Late", "options": {"method": "digital-clocks"}}
        ]),
    );
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    assert_eq!(proved["record"]["backend"], "en-1", "{proved}");
    assert_eq!(
        proved["record"]["basis"]["type"], "digitized-explicit-state",
        "{proved}"
    );
    assert_eq!(proved["replay"]["reproduced"], true, "{proved}");
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    assert_eq!(refuted["record"]["backend"], "en-1", "{refuted}");

    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    let en1 = engines
        .registrations()
        .find(|descriptor| descriptor.id().as_str() == "en-1")
        .expect("en-1 registers");
    assert_eq!(en1.state_graph_forms().len(), 3);
}
