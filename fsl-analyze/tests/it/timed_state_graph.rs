// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-852 and FSpec FR-712 (TC-2412): `possible`, `always possible` and
//! `unique path` items over the timed `Rpc` subject route to EN-6 and are
//! decided over the timed state graph on zones, with exact-rational evidence
//! that replays and a zone certificate.

use fsl_eval::simulation::TransitionSystem;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::analyze_family_support::{rpc_snapshot, rpc_unit};

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
        item["certificate"]["document"]["state_graph"]["form"], "always-possible",
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
        item["certificate"]["document"]["state_graph"]["form"], "always-possible",
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
        item["certificate"]["document"]["state_graph"]["form"], "possible",
        "{item}"
    );
    assert!(
        item["certificate"]["document"]["admitted"].is_null(),
        "{item}"
    );
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
    assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
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
    assert!(
        paths.iter().all(|entry| {
            entry["path"]["steps"]
                .as_array()
                .is_some_and(|steps| steps.iter().all(|step| step["delay"]["denominator"] == "1"))
        }),
        "digital-clock witnesses use integral delays: {proved}"
    );
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

/// `item` is EN-1's decline of the state-graph claim `declared` in `unit`,
/// carrying `rule` and the declaration's span, with no search.
fn assert_not_digitisable(item: &Value, unit: &str, declared: &str, rule: &str) {
    let start = unit.find(declared).expect("the claim is declared");
    assert_eq!(label(item), "declined", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(item["record"]["settled_by"], "result", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["diagnostic"],
        json!({"code": "invalid_runtime_input", "cause": "invalid-value",
            "detail": {"not_digitisable": {"rule": rule,
                "span": {"start": start, "end": start + declared.len()}}}}),
        "{item}"
    );
    assert!(item.get("exploration").is_none(), "no search runs: {item}");
}

const BACK_TO_IDLE: &str = "always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }";
const ONE_REPLY: &str =
    "unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);";

/// FR-243-AC-6, TC-2412 step 7: with `options.method` `digital-clocks`,
/// `always possible` and `unique path` are declined naming the form, whatever
/// their verdict, and no search runs.
#[trace("TC-2412", "FR-243-AC-6")]
#[test]
fn digital_clocks_decline_always_possible_and_unique_path() {
    let method = json!({"method": "digital-clocks"});
    for (name, reset) in [
        ("timed-sg-digital-forms", true),
        ("timed-sg-digital-forms-no-reset", false),
    ] {
        let run = run(
            name,
            reset,
            json!([
                {"claim": "BackToIdle", "options": method.clone()},
                {"claim": "OneReply", "options": method.clone()}
            ]),
        );
        let unit = unit(reset);
        assert_not_digitisable(&run.item(0), &unit, BACK_TO_IDLE, "always-possible");
        assert_not_digitisable(&run.item(1), &unit, ONE_REPLY, "unique-path");
    }
}

/// FR-243-AC-6, TC-2412 step 8: `first` runs at a time `t` in [0, 1] ms and
/// resets `x`; `y` reads the time. From phase 1, `zero` needs `t = 0` and
/// `one` needs `t = 1`, so only a fractional `t` is trapped, which no
/// integer-clock reading visits. EN-1 declines both forms instead of proving
/// them.
#[trace("TC-2412", "FR-243-AC-6")]
#[test]
fn digital_clocks_decline_the_forms_over_a_fractional_trap() {
    let always = "always possible Done using v over (c: M::Call) { c.phase >= 2 }";
    let unique = "unique path Done using v over (c: M::Call) from (c.phase = 1) to (c.phase >= 2);";
    for (name, declared, rule) in [
        ("timed-sg-fraction-trap", always, "always-possible"),
        ("timed-sg-fraction-unique", unique, "unique-path"),
    ] {
        let body = format!(
            "{}\
             pre First using v on M::Call::first {{ self.phase = 0 }}\n\
             post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
             pre Zero using v on M::Call::zero {{ self.phase = 1 and self.y >= 2 ms and self.x >= 2 ms }}\n\
             post ZeroDone using v on M::Call::zero {{ self.phase = 2 }}\n\
             pre One using v on M::Call::one {{ self.phase = 1 and self.y >= 2 ms and self.x <= 1 ms }}\n\
             post OneDone using v on M::Call::one {{ self.phase = 3 }}\n\
             time invariant Early when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
             time invariant Late when {{ self.phase = 1 }} {{ self.y <= 2 ms }}\n\
             {declared}\n",
            crate::timed_world::DENSE,
        );
        let unit = crate::timed_world::unit_text(
            crate::timed_world::PACKAGE,
            crate::zone_search::TIMED,
            &body,
        );
        let document = crate::timed_world::document(
            &["x", "y"],
            &[
                ("first", &["phase", "x"]),
                ("zero", &["phase"]),
                ("one", &["phase"]),
            ],
        );
        let run = analyze(&request_over(
            &Workspace::new(name),
            &unit,
            &document,
            &[rpc_snapshot()],
            json!({"items": [{"claim": "Done", "options": {"method": "digital-clocks"}}]}),
        ));
        assert_not_digitisable(&run.item(0), &unit, declared, rule);
    }
}

/// FR-852-AC-9 and FR-852-AC-13: both timed state-graph routes settle an
/// undefined reachable reset guard as the first undefined-evaluation
/// refutation, including when `possible` has a witness.
#[trace("TC-2412", "FR-852-AC-9")]
#[trace("TC-2412", "FR-852-AC-13")]
#[trace("TC-2412", "FR-852-AC-8")]
#[trace("TC-2412", "FR-852-AC-12")]
#[trace("TC-2412", "FR-852-AC-17")]
#[trace("TC-2412", "FR-852-AC-25")]
#[test]
fn timed_state_graph_routes_refute_an_undefined_guard() {
    let guard = "q(2 - self.phase) >= 0";
    let mut source = unit(true)
        .replace(
            "not self.inflight and (self.phase = 2 or self.phase = 3)",
            guard,
        )
        .replacen(
            "pre SendIdle",
            "function q using v(x: Integer): Integer pure { 6 mod x }\npre SendIdle",
            1,
        )
        .replacen(
            "model M = ",
            "profile d = \"filament.value.integer-division.truncating/v1\";\nmodel M = ",
            1,
        );
    source.push_str(
        "\npossible Never using v over (c: M::Call) { false }\n\
         possible UndefinedTarget using v over (c: M::Call) { q(2 - c.phase) >= 0 }\n\
         unique path UndefinedUnique using v over (c: M::Call) from (c.phase = 2) to (c.phase = 2);\n",
    );
    let body = source
        .split_once("model M = \"test/timed\";\n")
        .map(|(_, body)| body)
        .expect("the timed fixture has a model declaration");
    let world = crate::timed_world::world_with_mode(
        &crate::zone_search::rpc_document(),
        &format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        ),
        body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    assert!(
        String::from_utf8_lossy(&world.unit).contains("q(2 - self.phase) >= 0"),
        "the checked fixture carries the undefined guard: {}",
        String::from_utf8_lossy(&world.unit)
    );
    let system = world.system();
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, "M")
        .expect("the source selects a timed model");
    let zones = fsl_analyze::zone_check::explore_timed_state_graph(
        &timed,
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    );
    let zones = zones.expect("the zone graph keeps the undefined finding");
    assert_eq!(zones.undefined.len(), 1, "{zones:?}");
    assert_eq!(
        zones.undefined[0].reading.site,
        fsl_eval::simulation::model::TimedSite::Guard {
            clause: "Reset".to_owned()
        }
    );
    assert!(format!("{:?}", zones.undefined[0].reading.cause).contains("DivisionByZero"));
    assert!(zones.undefined[0].reading.transition.is_some(), "{zones:?}");
    assert!(zones.undefined[0].reading.location.is_some(), "{zones:?}");
    let claim = world
        .package
        .graph()
        .state_graph_claim("ReplyPossible")
        .expect("the source package has the state-graph claim");
    let parameter = claim.over().expect("the claim is over a call");
    let instance = system
        .instances(&filament_exact::ValueType::Reference(
            parameter.object.clone(),
        ))
        .expect("enumerate claim instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .bound_reference(parameter.object.clone(), &instance)
        .expect("bind the claim instance");
    let decision = fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &zones,
        claim,
        Some(&over),
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("decide the claim over the graph");
    assert!(
        matches!(
            decision,
            fsl_analyze::zone_check::TimedStateGraphDecision::Undefined { .. }
        ),
        "undefined subject evaluation outranks this claim's graph decision: {decision:?}"
    );
    for name in ["BackToIdle", "UndefinedUnique"] {
        let claim = world
            .package
            .graph()
            .state_graph_claim(name)
            .unwrap_or_else(|| panic!("the source package has the {name} claim"));
        let parameter = claim.over().expect("the claim is over a call");
        let over = system
            .bound_reference(parameter.object.clone(), &instance)
            .expect("bind the claim instance");
        let decision = fsl_analyze::zone_check::decide_timed_state_graph(
            &timed,
            &zones,
            claim,
            Some(&over),
            fsl_analyze::zone_check::ZoneCheckLimits::default(),
            &filament_exact::Cancel::new(),
        )
        .unwrap_or_else(|error| panic!("decide {name}: {error:?}"));
        assert!(
            matches!(
                decision,
                fsl_analyze::zone_check::TimedStateGraphDecision::Undefined { .. }
            ),
            "the undefined guard also settles {name}: {decision:?}"
        );
    }
    let never = world
        .package
        .graph()
        .state_graph_claim("Never")
        .expect("the fixture has an earlier trap claim");
    let never_parameter = never.over().expect("the claim is over a call");
    let never_over = system
        .bound_reference(never_parameter.object.clone(), &instance)
        .expect("bind the never claim instance");
    let never_decision = fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &zones,
        never,
        Some(&never_over),
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("decide the earlier trap");
    assert!(
        matches!(
            never_decision,
            fsl_analyze::zone_check::TimedStateGraphDecision::Refuted { node: 0, .. }
        ),
        "the initial trap precedes the later undefined node: {never_decision:?}"
    );
    let undefined_target = world
        .package
        .graph()
        .state_graph_claim("UndefinedTarget")
        .expect("the fixture has a claim predicate undefined at the same node");
    let target_parameter = undefined_target.over().expect("the claim is over a call");
    let target_over = system
        .bound_reference(target_parameter.object.clone(), &instance)
        .expect("bind the undefined target instance");
    let target_decision = fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &zones,
        undefined_target,
        Some(&target_over),
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("decide the undefined predicate");
    assert!(
        matches!(&target_decision, fsl_analyze::zone_check::TimedStateGraphDecision::Undefined { reading, .. }
            if reading.predicate == Some(fsl_semantics::check::PredicateRole::Target)
                && reading.transition.is_none()),
        "same-node claim predicate undefined takes precedence over the guard: {target_decision:?}"
    );
    assert_eq!(zones.undefined.len(), 1, "{zones:?}");
    let phase_two = zones
        .nodes
        .iter()
        .find(|node| {
            node.state
                .state()
                .populations
                .values()
                .flat_map(|objects| objects.values())
                .flat_map(|object| object.fields.values())
                .any(|value| matches!(value, filament_exact::Value::Integer(value) if value.to_string() == "2"))
        })
        .expect("phase 2 is explored");
    let clocks = fsl_eval::simulation::model::ClockValuation::new(vec![
            filament_exact::Rational::from_integer(filament_exact::Integer::zero());
            timed.places().len()
        ]);
    let phase_two_timed =
        fsl_eval::simulation::model::TimedState::new(phase_two.state.clone(), clocks);
    let expansion = system
        .successors(&phase_two.state)
        .expect("expand phase two");
    assert!(expansion
        .findings
        .iter()
        .any(|finding| finding.undefined_contract().is_some()));
    let phase_two_steps = timed.steps(&phase_two_timed);
    assert!(
        matches!(
            phase_two_steps,
            Err(fsl_eval::simulation::model::TimedStop::Undecided(_))
        ),
        "the reset guard has the undefined division: {phase_two_timed:?}; {phase_two_steps:?}"
    );
    assert!(
        zones.nodes.iter().any(|node| {
            node.state
                .state()
                .populations
                .values()
                .flat_map(|objects| objects.values())
                .flat_map(|object| object.fields.values())
                .any(|value| matches!(value, filament_exact::Value::Integer(value) if value.to_string() == "2"))
        }),
        "phase 2 is explored: {zones:?}"
    );
    let digital = fsl_replay::explore_digital_state_graph(
        &timed,
        10_000,
        100_000,
        &filament_exact::Cancel::new(),
    );
    let digital = digital.expect("the digital graph keeps the undefined finding");
    assert!(!digital.undefined.is_empty(), "{digital:?}");
    assert!(
        digital.undefined[0].reading.location.is_some(),
        "{digital:?}"
    );
    assert!(
        digital.nodes.iter().any(|node| {
            node.discrete()
                .state()
                .populations
                .values()
                .flat_map(|objects| objects.values())
                .flat_map(|object| object.fields.values())
                .any(|value| matches!(value, filament_exact::Value::Integer(value) if value.to_string() == "2"))
        }),
        "phase 2 is explored: {digital:?}"
    );
}

/// FR-852-AC-10 and FR-852-AC-14: both timed routes retain an undefined
/// `terminal when` reading at the reachable terminal model state.
#[trace("TC-2412", "FR-852-AC-10")]
#[trace("TC-2412", "FR-852-AC-14")]
#[test]
fn timed_state_graph_routes_retain_undefined_terminal_readings() {
    let source = unit(false).replacen(
        "pre SendIdle",
        "function q using v(x: Integer): Integer pure { 6 mod x }\npre SendIdle",
        1,
    ) + "\nterminal Done using v on M when { q(2 - self.phase) >= 0 };\n";
    let body = source
        .split_once("model M = \"test/timed\";\n")
        .map(|(_, body)| body)
        .expect("the timed fixture has a model declaration");
    let world = crate::timed_world::world_with_mode(
        &crate::zone_search::rpc_document(),
        &format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        ),
        body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let system = world.system();
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, "M")
        .expect("the source selects a timed model");
    let zones = fsl_analyze::zone_check::explore_timed_state_graph(
        &timed,
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("zone exploration retains the terminal undefined reading");
    assert!(
        zones.undefined.iter().any(|entry| {
            matches!(
                entry.reading.site,
                fsl_eval::simulation::model::TimedSite::Terminal { .. }
            ) && entry.reading.location.is_some()
                && format!("{:?}", entry.reading.cause).contains("DivisionByZero")
        }),
        "{zones:?}"
    );
    let claim = world
        .package
        .graph()
        .state_graph_claim("ReplyPossible")
        .expect("the source package has the state-graph claim");
    let parameter = claim.over().expect("the claim is over a call");
    let instance = system
        .instances(&filament_exact::ValueType::Reference(
            parameter.object.clone(),
        ))
        .expect("enumerate claim instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .bound_reference(parameter.object.clone(), &instance)
        .expect("bind the claim instance");
    let decision = fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &zones,
        claim,
        Some(&over),
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("decide the claim over terminal undefined evidence");
    assert!(
        matches!(&decision, fsl_analyze::zone_check::TimedStateGraphDecision::Undefined { reading, .. }
            if reading.transition.is_none() && reading.locus.is_some()),
        "{decision:?}"
    );
    let digital = fsl_replay::explore_digital_state_graph(
        &timed,
        10_000,
        100_000,
        &filament_exact::Cancel::new(),
    )
    .expect("digital exploration retains the terminal undefined reading");
    assert!(
        digital.undefined.iter().any(|entry| {
            matches!(
                entry.reading.site,
                fsl_eval::simulation::model::TimedSite::Terminal { .. }
            ) && entry.reading.location.is_some()
                && format!("{:?}", entry.reading.cause).contains("DivisionByZero")
        }),
        "{digital:?}"
    );
}

/// FR-852-AC-11 and FR-852-AC-15: the selected state constraint's undefined
/// reading remains a refutation on both timed graph routes.
#[trace("TC-2412", "FR-852-AC-11")]
#[trace("TC-2412", "FR-852-AC-15")]
#[test]
fn timed_state_graph_routes_refute_an_undefined_state_constraint() {
    let source = unit(true).replacen(
        "pre SendIdle",
        "function q using v(x: Integer): Integer pure { 6 mod x }\n\
         constraint C using v on M::Call at current { q(2 - self.phase) >= 0 }\n\
         pre SendIdle",
        1,
    );
    let body = source
        .split_once("model M = \"test/timed\";\n")
        .map(|(_, body)| body)
        .expect("the timed fixture has a model declaration");
    let world = crate::timed_world::world_with_mode(
        &crate::zone_search::rpc_document(),
        &format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        ),
        body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let system = world.system();
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, "M")
        .expect("the source selects a timed model");
    let zones = fsl_analyze::zone_check::explore_timed_state_graph(
        &timed,
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("zone exploration reaches the undefined constraint state");
    let claim = world
        .package
        .graph()
        .state_graph_claim("ReplyPossible")
        .expect("the source package has the state-graph claim");
    let parameter = claim.over().expect("the claim is over a call");
    let instance = system
        .instances(&filament_exact::ValueType::Reference(
            parameter.object.clone(),
        ))
        .expect("enumerate claim instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .bound_reference(parameter.object.clone(), &instance)
        .expect("bind the claim instance");
    let decision = fsl_analyze::zone_check::decide_timed_state_graph_with_constraint(
        &timed,
        &zones,
        claim,
        Some(&over),
        Some("C"),
        fsl_analyze::zone_check::ZoneCheckLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("decide the selected constraint");
    assert!(
        matches!(&decision, fsl_analyze::zone_check::TimedStateGraphDecision::Undefined { reading, .. }
            if reading.locus == system.state_clause_locus("C")
                && format!("{:?}", reading.cause).contains("DivisionByZero")),
        "{decision:?}"
    );
    let digital = fsl_replay::explore_digital_state_graph(
        &timed,
        10_000,
        100_000,
        &filament_exact::Cancel::new(),
    )
    .expect("digital exploration reaches the undefined constraint state");
    assert!(digital.nodes.iter().any(|node| {
        matches!(
            system.constraint_value(node.discrete(), "C"),
            Ok(fsl_eval::simulation::ConstraintValue::Undefined { .. })
        )
    }));
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
    assert!(
        item["record"]
            .to_string()
            .contains("strict-clock-constraint"),
        "{item}"
    );
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
        assert_eq!(
            item["record"]["cause"]["kind"], "no-initial-state",
            "{item}"
        );
        assert_eq!(item["exploration"]["states"], "0", "{item}");
    }
}

/// The Zeno-only `Rpc` variant of FR-244-AC-7: the time invariant
/// `x <= 0 ms` holds at every state and `reply` needs `x >= 0 ms`, so no
/// positive delay is ever admissible and the only infinite path is the
/// zero-delay `send`, `reply`, `reset` cycle.
fn zeno_only_unit() -> String {
    let claims = format!(
        "{CLAIMS}possible Replied using v over (c: M::Call) {{ c.phase = 2 }}\n\
         temporal NoLateReply using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always holds(not c.late) }}\n"
    );
    let text = crate::analyze_family_support::rpc_unit_with(4, ">= 0", ">=", &claims);
    // FR-244-AC-7 names the possible claim `Replied`; this fixture gives the
    // unit's `post Replied` clause another name so the two do not clash.
    let renamed = text.replace("post Replied using", "post ReplyEffect using");
    assert_ne!(renamed, text, "the unit declares `post Replied`");
    let zeno = renamed.replace(
        "time invariant ServerDeadline when { self.inflight } { self.x <= 3 ms }",
        "time invariant ServerDeadline when { true } { self.x <= 0 ms }",
    );
    assert_ne!(zeno, renamed, "the unit declares the server deadline");
    zeno
}

/// FR-244-AC-7, FR-245-AC-11: a state-graph item's certificate certifies the
/// reachable timed graph with no `admitted` witness and is accepted over a
/// Zeno-only graph; a temporal claim over the same subject still settles
/// `NoAdmittedBehaviour` (negative: its witness requirement stays).
#[trace("TC-3030", "FR-244-AC-7")]
#[trace("TC-3030", "FR-245-AC-11")]
#[test]
fn a_state_graph_certificate_over_a_zeno_only_graph_needs_no_divergence_witness() {
    let unit = zeno_only_unit();
    assert!(unit.contains("when { true } { self.x <= 0 ms }"), "{unit}");
    let ws = Workspace::new("timed-sg-zeno-certificate");
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

/// The document item `analyze` writes for `item` over a timed subject whose
/// object types the unit declares: `Derived` extends `Base` and the base
/// population holds `c` as a `Derived`, so a binding of `over (c: Base)` has
/// an admitted type other than its declared type. A domain package declaring
/// both types cannot name one of them as its time source, and the `analyze`
/// request reader names no population of a unit's own, so the binding is
/// given directly. The selected domain package holds no object.
fn settle_over_base(claim: &str, item: Value) -> Value {
    let limits = fsl_analyze::analyze::AnalyzeLimits::published(
        fsl_analyze::analyze::DeadlineSupply::Unbounded,
    );
    let (document, _) = settle_over_base_with(claim, item, limits, false).expect("a settled run");
    document["items"][0].clone()
}

/// [`settle_over_base`] under `limits`, the whole document or the refusal;
/// `stated` hands the run the item as the caller stated it.
fn settle_over_base_with(
    claim: &str,
    item: Value,
    limits: fsl_analyze::analyze::AnalyzeLimits,
    stated: bool,
) -> Result<(Value, u8), fsl_analyze::analyze::document::AnalyzeDocumentError> {
    use crate::timed_world::{world_over, DENSE};
    use fsl_analyze::analyze::document::{
        analyze_document, resolve_items, AnalyzeRun, ItemSelection, SubjectSelection,
    };
    use fsl_analyze::analyze::{Engines, SourceProvision, SubjectBinding};
    use fsl_eval::simulation::model::PopulationUniverse;
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    use fsl_semantics::model::observation::document_reference;

    let world = world_over(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        &format!(
            "{DENSE}\
             time Base dense unit ms;\n\
             object Base {{ x: Clock; population bases; }}\n\
             object Derived extends Base {{ }}\n\
             {claim}"
        ),
        &[],
    );
    let bases = world
        .package
        .graph()
        .source_packages()
        .iter()
        .flat_map(|source| &source.records)
        .find_map(|record| match record {
            DomainPackageRecord::Population(population) => Some(population.key.clone()),
            _ => None,
        })
        .expect("the unit declares one population");
    let mut snapshot: Value =
        serde_json::from_slice(world.snapshots.values().next().expect("one snapshot"))
            .expect("snapshot JSON");
    snapshot["populations"]
        .as_array_mut()
        .expect("snapshot populations")
        .push(json!({"population": "bases", "complete": true,
            "objects": [{"key": "c", "type": "Derived", "fields": {}}]}));
    let bytes = snapshot.to_string().into_bytes();
    let snapshots = std::collections::BTreeMap::from([(
        document_reference(&bytes).expect("a document names itself"),
        bytes,
    )]);
    let request = world.request();
    let mut universes = request.universes.clone();
    universes.push(PopulationUniverse {
        population: bases,
        keys: vec!["c".to_owned()],
    });
    let binding = SubjectBinding {
        packages: request.packages,
        snapshots: &snapshots,
        initial: request.initial.clone(),
        universes,
    };
    let engines = Engines::fsl().expect("the manifests register");
    let mut registry = fsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a unique backend");
    }
    let selections: Vec<ItemSelection> =
        serde_json::from_value(json!([item])).expect("an item selection");
    let source = world.package.graph().source().clone();
    let items = resolve_items(
        &world.package,
        &SubjectSelection::default_over(&snapshots),
        Some(&selections),
        &registry,
        std::slice::from_ref(&source),
        &binding,
        &limits,
    );
    analyze_document(
        AnalyzeRun {
            selections: if stated { &selections } else { &[] },
            package: &world.package,
            binding,
            provision: SourceProvision::single(&source, &world.unit),
            items,
            engines: &engines,
            limits,
        },
        &crate::analyze_cli_support::Fr290,
        &filament_exact::Cancel::new(),
    )
}

/// FR-125 "Instancing"'s written binding of the held `Derived` under `c` for
/// a parameter of type `Base`: `Base`'s universe, the admitted type `Derived`
/// and the key.
fn the_admitted_derived() -> Value {
    json!({
        "identity": "c",
        "object_type": {
            "original": {"node": "ix://source/package/Derived", "package": "source/package"},
            "owner_effective_type": null
        },
        "type": "reference",
        "universe": {
            "ordinal": 0,
            "package": "source/package",
            "path": ["model", "object_type", "ix://source/package/Base"]
        }
    })
}

/// QSL-3553 (FR-255-AC-25): the library path refuses a timed item that states
/// `temporal.limits.max_states` when the caller also set
/// `zone_check.max_symbolic_states`, as the command does, and with the same
/// item position and setting.
#[trace("FR-255-AC-25")]
#[test]
fn the_library_refuses_a_timed_item_limit_the_caller_also_set() {
    use fsl_analyze::analyze::document::AnalyzeDocumentError;
    use fsl_analyze::analyze::{AnalyzeLimits, AnalyzeRefusal, DeadlineSupply};
    let item = json!({"claim": "AnyBase", "temporal": {"limits": {"max_states": 2}}});
    let mut limits = AnalyzeLimits::published(DeadlineSupply::Unbounded);
    assert!(limits
        .engines
        .set(fsl_foundation::Setting::ZoneCheckMaxSymbolicStates, 5));
    let refused = settle_over_base_with(BASE_POSSIBLE, item.clone(), limits, true)
        .expect_err("the item and the caller state one bound twice");
    let AnalyzeDocumentError::Refused(AnalyzeRefusal::EngineConflict { position, setting }) =
        refused
    else {
        panic!("an engine conflict, not {refused:?}");
    };
    assert_eq!(position, 0);
    assert_eq!(setting, fsl_foundation::Setting::ZoneCheckMaxSymbolicStates);
    // The caller's bound alone, or the item's alone, settles.
    let mut caller_only = AnalyzeLimits::published(DeadlineSupply::Unbounded);
    caller_only
        .engines
        .set(fsl_foundation::Setting::ZoneCheckMaxSymbolicStates, 5);
    settle_over_base_with(
        BASE_POSSIBLE,
        json!({"claim": "AnyBase"}),
        caller_only,
        true,
    )
    .expect("a caller bound alone settles");
}

const BASE_POSSIBLE: &str = "possible AnyBase using v over (c: Base) { true }\n";

/// FR-125 "Instancing": the zone route's state-graph evidence of
/// `over (c: Base)` names the admitted `Derived`, and its replay and zone
/// certificate accept that binding.
#[trace("TC-520")]
#[test]
fn zone_state_graph_evidence_binds_the_admitted_member_type() {
    let item = settle_over_base(BASE_POSSIBLE, json!({"claim": "AnyBase"}));
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-6", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(
        item["record"]["evidence"]["instance"],
        json!({"type": "instance", "parameter": "c", "key": the_admitted_derived()}),
        "{item}"
    );
}

/// FR-125 "Instancing": the digital-clocks route's state-graph evidence of
/// `over (c: Base)` names the admitted `Derived`, and its replay accepts that
/// binding.
#[trace("TC-520")]
#[test]
fn digital_state_graph_evidence_binds_the_admitted_member_type() {
    let item = settle_over_base(
        BASE_POSSIBLE,
        json!({"claim": "AnyBase", "options": {"method": "digital-clocks"}}),
    );
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert_eq!(
        item["record"]["evidence"]["instance"],
        json!({"type": "instance", "parameter": "c", "key": the_admitted_derived()}),
        "{item}"
    );
}

/// FR-125 "Instancing": a refuted zone temporal `over_binding` of
/// `over (c: Base)` names the admitted `Derived`.
#[trace("TC-520", "FR-125-AC-9")]
#[test]
fn a_zone_temporal_over_binding_names_the_admitted_member_type() {
    let item = settle_over_base(
        "temporal NeverBase using tt over (c: Base) clock \"model-time\" on origin \
            { always holds(false) }\n",
        json!({"claim": "NeverBase"}),
    );
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["record"]["backend"], "en-6", "{item}");
    assert_eq!(
        item["counterexample"]["over_binding"],
        the_admitted_derived(),
        "{item}"
    );
}

const PHASE_OPS: [(&str, &[&str]); 3] = [
    ("first", &["phase", "x"]),
    ("zero", &["phase"]),
    ("one", &["phase"]),
];

/// The declarations of the trap subject. `first` runs at a time `t` in
/// [0, 1] ms and resets `x`; `y` reads the time. Phase 1 ends by `y = 2 ms`,
/// where `zero` needs `x >= 2 ms` (`t` is 0) and `one` has the guard
/// `one_guard`; there `x` reads `2 ms - t`.
fn trap_body(one_guard: &str, claim: &str) -> String {
    format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Zero using v on M::Call::zero {{ self.phase = 1 and self.y >= 2 ms and self.x >= 2 ms }}\n\
         post ZeroDone using v on M::Call::zero {{ self.phase = 2 }}\n\
         pre One using v on M::Call::one {{ self.phase = 1 and {one_guard} }}\n\
         post OneDone using v on M::Call::one {{ self.phase = 3 }}\n\
         time invariant Early when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
         time invariant Late when {{ self.phase = 1 }} {{ self.y <= 2 ms }}\n\
         {claim}\n",
        crate::timed_world::DENSE,
    )
}

/// `analyze` of the claim `Done` over a dense subject of `body`.
fn timed_run(
    name: &str,
    body: &str,
    clocks: &[&str],
    ops: &[(&str, &[&str])],
    items: Value,
) -> Run {
    let unit =
        crate::timed_world::unit_text(crate::timed_world::PACKAGE, crate::zone_search::TIMED, body);
    analyze(&request_over(
        &Workspace::new(name),
        &unit,
        &crate::timed_world::document(clocks, ops),
        &[rpc_snapshot()],
        json!({"items": items}),
    ))
}

fn trapped_with(name: &str, one_guard: &str, claim: &str) -> Value {
    let run = timed_run(
        name,
        &trap_body(one_guard, claim),
        &["x", "y"],
        &PHASE_OPS,
        json!([{"claim": "Done"}]),
    );
    let item = run.item(0);
    assert_eq!(item["record"]["backend"], "en-6", "{item}");
    item
}

const ALWAYS_DONE: &str = "always possible Done using v over (c: M::Call) { c.phase >= 2 }";
const UNIQUE_DONE: &str =
    "unique path Done using v over (c: M::Call) from (c.phase = 1) to (c.phase >= 2);";
const POSSIBLE_THREE: &str = "possible Done using v over (c: M::Call) { c.phase = 3 }";

fn assert_refuted_and_replayed(item: &Value) {
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    assert_eq!(item["certificate"]["accepted"], false, "{item}");
}

fn assert_proved_and_certified(item: &Value) {
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
}

/// With `one` never enabled, every `first` after time 0 leaves a reachable
/// state from which no run reaches phase 2, so the claim is false.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_always_possible_when_part_of_a_zone_is_trapped() {
    let item = trapped_with("timed-sg-part-zone-trap", "false", ALWAYS_DONE);
    assert_refuted_and_replayed(&item);
}

/// `unique path` counts the ways from every reachable `X` state: a trapped
/// state has none, so the claim is false although every other state has one.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_unique_path_when_part_of_a_zone_has_no_path() {
    let item = trapped_with("timed-sg-part-zone-unique", "false", UNIQUE_DONE);
    assert_refuted_and_replayed(&item);
}

/// `zero` takes `t = 0` and `one` exactly the other times, so every state has
/// a way, and one way, although two steps leave the node of phase 1.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_prove_over_steps_that_split_a_zone_between_them() {
    let guard = "self.y >= 2 ms and self.x < 2 ms";
    assert_proved_and_certified(&trapped_with("timed-sg-split-always", guard, ALWAYS_DONE));
    assert_proved_and_certified(&trapped_with("timed-sg-split-unique", guard, UNIQUE_DONE));
}

/// At `t = 0` both `zero` and `one` run: two sequences from one state.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_unique_path_with_two_sequences_from_part_of_a_zone() {
    let item = trapped_with(
        "timed-sg-split-overlap",
        "self.y >= 2 ms and self.x <= 2 ms",
        UNIQUE_DONE,
    );
    assert_refuted_and_replayed(&item);
}

/// `one` takes `t` strictly inside (0, 1) ms only, so the one time `t = 1`
/// is trapped: a strict bound's boundary.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_when_only_a_strict_boundary_is_trapped() {
    let guard = "self.y >= 2 ms and self.x < 2 ms and self.x > 1 ms";
    assert_refuted_and_replayed(&trapped_with("timed-sg-edge-always", guard, ALWAYS_DONE));
    assert_refuted_and_replayed(&trapped_with("timed-sg-edge-unique", guard, UNIQUE_DONE));
}

/// TC-2412 step 8's subject on EN-6: with `first` strictly between 0 and
/// 1 ms, neither `zero` nor `one` is ever taken, so both claims are false.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_the_trap_between_two_integer_times() {
    let guard = "self.y >= 2 ms and self.x <= 1 ms";
    assert_refuted_and_replayed(&trapped_with(
        "timed-sg-fraction-always",
        guard,
        ALWAYS_DONE,
    ));
    assert_refuted_and_replayed(&trapped_with(
        "timed-sg-fraction-unique",
        guard,
        UNIQUE_DONE,
    ));
}

/// One step through two overlapping guard parts is one transition sequence.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_count_one_sequence_for_a_step_through_two_guard_parts() {
    let item = trapped_with(
        "timed-sg-two-parts",
        "self.y >= 2 ms and self.x < 2 ms and (self.x <= 1 ms or self.x >= 1 ms)",
        UNIQUE_DONE,
    );
    assert_proved_and_certified(&item);
}

/// `possible` is read from the initial state alone: phase 3 is reachable
/// when `one` can run and not when it never can.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_decide_possible_from_the_initial_valuation() {
    let never = trapped_with("timed-sg-possible-never", "false", POSSIBLE_THREE);
    assert_eq!(label(&never), "refuted", "{never}");
    let some = trapped_with(
        "timed-sg-possible-some",
        "self.y >= 2 ms and self.x <= 1 ms",
        POSSIBLE_THREE,
    );
    assert_proved_and_certified(&some);
}

/// `renew` leaves `x` behind the time, where `finish`'s lower bound is out of
/// reach before phase 1 ends. The aLU simulation covers that state by
/// `keep`'s and would hide it.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_a_trap_that_lower_bound_coverage_would_hide() {
    let body = format!(
        "{}\
         pre Keep using v on M::Call::keep {{ self.phase = 0 }}\n\
         post KeepDone using v on M::Call::keep {{ self.phase = 1 }}\n\
         pre Renew using v on M::Call::renew {{ self.phase = 0 }}\n\
         post RenewDone using v on M::Call::renew {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Finish using v on M::Call::finish {{ self.phase = 1 and self.x >= 2 ms }}\n\
         post FinishDone using v on M::Call::finish {{ self.phase = 2 }}\n\
         time invariant Early when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
         time invariant Late when {{ self.phase = 1 }} {{ self.y <= 2 ms }}\n\
         {ALWAYS_DONE}\n",
        crate::timed_world::DENSE,
    );
    let run = timed_run(
        "timed-sg-lower-cover",
        &body,
        &["x", "y"],
        &[
            ("keep", &["phase"]),
            ("renew", &["phase", "x"]),
            ("finish", &["phase"]),
        ],
        json!([{"claim": "Done"}]),
    );
    assert_refuted_and_replayed(&run.item(0));
}

/// `enter` reaches phase 1 with `x >= 5 ms`, and `join` goes on to a phase 2
/// whose node `skip` already stored with any `x`. The edge into that wider
/// node leads on to phase 3; no run from phase 1 does.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn zones_refute_where_a_covering_edge_leads_further_than_any_run() {
    for (name, claim) in [
        (
            "timed-sg-covered-always",
            "always possible Done using v over (c: M::Call) from (c.phase = 1) { c.phase = 3 }",
        ),
        (
            "timed-sg-covered-unique",
            "unique path Done using v over (c: M::Call) from (c.phase = 1) to (c.phase = 3);",
        ),
    ] {
        let body = format!(
            "{}\
             pre Skip using v on M::Call::skip {{ self.phase = 0 }}\n\
             post SkipDone using v on M::Call::skip {{ self.phase = 2 }}\n\
             pre Enter using v on M::Call::enter {{ self.phase = 0 and self.x >= 5 ms }}\n\
             post EnterDone using v on M::Call::enter {{ self.phase = 1 }}\n\
             pre Join using v on M::Call::join {{ self.phase = 1 }}\n\
             post JoinDone using v on M::Call::join {{ self.phase = 2 }}\n\
             pre Last using v on M::Call::done {{ self.phase = 2 and self.x <= 1 ms }}\n\
             post LastDone using v on M::Call::done {{ self.phase = 3 }}\n\
             {claim}\n",
            crate::timed_world::DENSE,
        );
        let run = timed_run(
            name,
            &body,
            &["x"],
            &[
                ("skip", &["phase"]),
                ("enter", &["phase"]),
                ("join", &["phase"]),
                ("done", &["phase"]),
            ],
            json!([{"claim": "Done"}]),
        );
        assert_refuted_and_replayed(&run.item(0));
    }
}

/// `first` enters phase 1 resetting `x`, where `escape` runs only at once.
/// Every later time in phase 1 is trapped, and only waiting gets there.
/// The `late` step takes some of those times on to phase 3, a state that is
/// trapped as soon as it is entered. The `direct` step is a second way from
/// phase 0 to phase 2.
fn escape_run(name: &str, extra: Option<&str>, claim: &str) -> Value {
    let extra_step = match extra {
        Some("late") => {
            "pre Late using v on M::Call::slide { self.phase = 1 and self.x > 1 ms }\n\
             post LateDone using v on M::Call::slide { self.phase = 3 }\n"
        }
        Some("direct") => {
            "pre Direct using v on M::Call::direct { self.phase = 0 }\n\
             post DirectDone using v on M::Call::direct { self.phase = 2 }\n"
        }
        _ => "",
    };
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Escape using v on M::Call::escape {{ self.phase = 1 and self.x <= 0 ms }}\n\
         post EscapeDone using v on M::Call::escape {{ self.phase = 2 }}\n\
         {extra_step}\
         time invariant Stay when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
         {claim}\n",
        crate::timed_world::DENSE,
    );
    let mut ops: Vec<(&str, &[&str])> = vec![("first", &["phase", "x"]), ("escape", &["phase"])];
    if let Some(extra) = extra {
        // `late` is a field of the subject, so its step is named `slide`.
        let op = if extra == "late" { "slide" } else { extra };
        ops.push((op, &["phase"]));
    }
    let run = timed_run(name, &body, &["x"], &ops, json!([{"claim": "Done"}]));
    let item = run.item(0);
    assert_eq!(
        item["record"]["backend"],
        "en-6",
        "{item} in {}",
        run.document()
    );
    item
}

/// A trap stem ends at a trapped state. Here every trapped state of the one
/// violating node is reached only by waiting, which the stem cannot say, so
/// the item is inconclusive: never a counterexample that ends where the
/// target is still reachable.
#[trace("TC-2412", "FR-852-AC-34")]
#[trace("TC-1742", "FSpec-FR-712-AC-12")]
#[test]
fn zones_report_no_trap_stem_that_ends_where_the_target_is_reachable() {
    for (name, claim) in [
        (
            "timed-sg-wait-always",
            "always possible Done using v over (c: M::Call) from (c.phase = 1) { c.phase = 2 }",
        ),
        (
            "timed-sg-wait-unique",
            "unique path Done using v over (c: M::Call) from (c.phase = 1) to (c.phase = 2);",
        ),
    ] {
        let item = escape_run(name, None, claim);
        assert_eq!(label(&item), "inconclusive", "{item}");
        assert_eq!(
            item["record"]["cause"]["kind"], "lasso-not-concretized",
            "{item}"
        );
        assert!(
            item.get("counterexample").is_none_or(Value::is_null),
            "{item}"
        );
    }
}

/// A later violating node whose trapped states a step lands in gives the
/// stem, although an earlier node's are reached only by waiting.
#[trace("TC-2412", "FR-852-AC-34")]
#[trace("TC-1742", "FSpec-FR-712-AC-12")]
#[test]
fn zones_take_the_trap_stem_from_a_node_a_step_lands_in() {
    for (name, claim) in [
        (
            "timed-sg-late-always",
            "always possible Done using v over (c: M::Call) \
             from (c.phase = 1 or c.phase = 3) { c.phase = 2 }",
        ),
        (
            "timed-sg-late-unique",
            "unique path Done using v over (c: M::Call) \
             from (c.phase = 1 or c.phase = 3) to (c.phase = 2);",
        ),
    ] {
        let item = escape_run(name, Some("late"), claim);
        assert_refuted_and_replayed(&item);
        assert_eq!(item["counterexample"]["arm"], "trap", "{item}");
        let stem = &item["counterexample"]["stem"]["steps"];
        assert_eq!(stem.as_array().map(Vec::len), Some(2), "{item}");
        // The last step is `slide`, whose post-state is phase 3, a trapped
        // state: the stem ends there, not at a point before a delay.
        let last = stem[1]["transition"]["operation"].as_str().unwrap_or("");
        assert!(last.contains("Call/slide"), "{item}");
    }
}

/// Phase 1 has a trap no stem reaches, and phase 0 has two sequences to
/// phase 2: the second violation refutes.
#[trace("TC-2412", "FR-852-AC-34")]
#[trace("TC-1742", "FSpec-FR-712-AC-12")]
#[test]
fn zones_refute_by_two_sequences_beside_a_trap_with_no_stem() {
    let item = escape_run(
        "timed-sg-wait-pair",
        Some("direct"),
        "unique path Done using v over (c: M::Call) \
         from (c.phase = 0 or c.phase = 1) to (c.phase = 2);",
    );
    assert_refuted_and_replayed(&item);
    assert_eq!(item["counterexample"]["arm"], "path-pair", "{item}");
}

/// A certificate over the explored graph of the trap subject, listing
/// `ef_nodes`, and the checker's verdict on it.
fn check_listing(
    one_guard: &str,
    claim: &str,
    ef_nodes: &[usize],
    max_certificate_edges: u64,
) -> fsl_replay::ZoneCertificateVerdict {
    use filament_exact::{Cancel, Integer};
    use fsl_analyze::zone_check::{explore_timed_state_graph, Bound, ZoneCheckLimits};
    use fsl_eval::simulation::model::TimedSystem;
    use fsl_route::deadlock::{ItemObligation, ObligationKind};

    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x", "y"], &PHASE_OPS),
        crate::zone_search::TIMED,
        &trap_body(one_guard, claim),
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed subject");
    let graph = explore_timed_state_graph(&timed, ZoneCheckLimits::default(), &Cancel::new())
        .expect("closed graph");
    // Phase 0 holds every time up to 1 ms, in units of 1 ms.
    for clock in 1..3 {
        assert_eq!(
            graph.nodes[0].zone.bound(clock, 0).expect("a clock"),
            &Bound::le(1.into())
        );
    }
    let nodes = graph
        .nodes
        .iter()
        .map(|node| {
            let mut bounds = Vec::new();
            for i in 0..3 {
                for j in 0..3 {
                    bounds.push(match node.zone.bound(i, j).expect("a clock") {
                        Bound::Finite { value, strict } => {
                            Some((Integer::from_big(value.clone()), *strict))
                        }
                        Bound::Infinite => None,
                    });
                }
            }
            fsl_replay::CertNode {
                state_key: fsl_eval::simulation::StateKey::from_canonical_bytes(
                    system
                        .state_key(&node.state, fsl_foundation::IdentityLimits::default())
                        .expect("state key")
                        .0,
                ),
                automaton_state: 0,
                zone: fsl_replay::Zone::from_matrix(3, bounds).expect("a node zone"),
                locked: false,
                product: None,
            }
        })
        .collect();
    let edges = graph
        .edges
        .iter()
        .map(|edge| fsl_replay::CertEdge {
            from: edge.from,
            to: edge.to,
            transition: Some(edge.transition.clone()),
            part: Some(edge.part_index),
            task_transition: None,
            horizon: None,
            silent: None,
        })
        .collect();
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("Done");
    let instance = fsl_eval::model_subject::ModelInstance {
        population: world.request().universes[0].population.clone(),
        key: "c".to_owned(),
    };
    let over = system
        .bound_reference(claim.over().expect("over").object.clone(), &instance)
        .expect("bound c");
    let certificate = fsl_replay::ZoneCertificate {
        obligation: ItemObligation {
            subject: (),
            kind: ObligationKind::StateGraphAuthored(
                claim.occurrence().expect("occurrence").clone(),
            ),
            reductions: Default::default(),
        },
        scale: Integer::one(),
        lu_bounds: fsl_replay::LuBounds {
            lower: vec![Some(Integer::from_big(10.into())); 2],
            upper: vec![Some(Integer::from_big(10.into())); 2],
        },
        nodes,
        edges,
        initials: graph.initials.clone(),
        components: None,
        non_zeno: None,
        divergence: None,
        admitted: None,
        state_graph: matches!(
            claim.form(),
            fsl_semantics::check::StateGraphForm::AlwaysPossible { .. }
        )
        .then(|| fsl_replay::StateGraphPart {
            form: fsl_replay::StateGraphCertificateForm::AlwaysPossible,
            ef_nodes: ef_nodes.to_vec(),
        }),
        refinement: None,
        task_product: None,
    };
    fsl_replay::check_zone_certificate(
        fsl_replay::ZoneCertificateRequest {
            simulation: world.request(),
            model: "M",
            max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
            item: fsl_replay::CertifiedItem::StateGraph {
                claim,
                over: Some(&over),
            },
        },
        &certificate.obligation,
        |obligation| match &obligation.kind {
            ObligationKind::StateGraphAuthored(occurrence) => {
                fsl_replay::CertificateItem::StateGraph(occurrence)
            }
            _ => fsl_replay::CertificateItem::Other,
        },
        &certificate,
        fsl_replay::CertificateCheckLimits {
            max_certificate_edges,
            ..fsl_replay::CertificateCheckLimits::default()
        },
        &Cancel::new(),
    )
    .expect("a checked certificate")
}

fn rule(verdict: fsl_replay::ZoneCertificateVerdict) -> fsl_replay::CertificateRule {
    match verdict {
        fsl_replay::ZoneCertificateVerdict::Rejected(rejection) => rejection.rule,
        other => panic!("a rejection: {other:?}"),
    }
}

/// The checker, on certificates written here. Nodes 0, 1 and 2 are phases 0,
/// 1 and 2 when `one` never runs; closing over whole nodes lists all three,
/// but only phase 2's zone lies in `E(P)`.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn the_checker_rejects_a_node_level_closure() {
    use fsl_replay::CertificateRule;
    assert_eq!(
        rule(check_listing("false", ALWAYS_DONE, &[0, 1, 2], u64::MAX)),
        CertificateRule::RankBroken
    );
    assert_eq!(
        rule(check_listing("false", ALWAYS_DONE, &[2], u64::MAX)),
        CertificateRule::SuccessorMissing
    );
    assert_eq!(
        check_listing(
            "self.y >= 2 ms and self.x < 2 ms",
            ALWAYS_DONE,
            &[0, 1, 2, 3],
            u64::MAX
        ),
        fsl_replay::ZoneCertificateVerdict::Accepted
    );
}

/// The checker's own `unique path` and `possible`, and its budget.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn the_checker_decides_unique_path_and_possible_on_valuations() {
    use fsl_replay::{CertificateRule, ZoneCertificateVerdict};
    assert_eq!(
        rule(check_listing("false", UNIQUE_DONE, &[], u64::MAX)),
        CertificateRule::CountBroken
    );
    assert_eq!(
        rule(check_listing(
            "self.y >= 2 ms and self.x <= 2 ms",
            UNIQUE_DONE,
            &[],
            u64::MAX
        )),
        CertificateRule::CountBroken
    );
    assert_eq!(
        check_listing(
            "self.y >= 2 ms and self.x < 2 ms",
            UNIQUE_DONE,
            &[],
            u64::MAX
        ),
        ZoneCertificateVerdict::Accepted
    );
    assert_eq!(
        rule(check_listing("false", POSSIBLE_THREE, &[], u64::MAX)),
        CertificateRule::InitialMissing
    );
    let stopped = check_listing("self.y >= 2 ms and self.x < 2 ms", UNIQUE_DONE, &[], 6);
    assert!(
        matches!(stopped, ZoneCertificateVerdict::Stopped(_)),
        "{stopped:?}"
    );
}

/// The producer's own result on the trap subject: only the phase-2 node's
/// whole zone lies in `E(P)`. The initial node's trapped states are reached
/// by waiting, so the stem is the step that lands in a trapped state of
/// phase 1.
#[trace("TC-2412", "FR-852-AC-32")]
#[trace("TC-2412", "FSpec-FR-712-AC-10")]
#[test]
fn the_decision_lists_whole_nodes_of_e_p_and_a_trapped_node() {
    use filament_exact::Cancel;
    use fsl_analyze::zone_check::{
        decide_timed_state_graph, explore_timed_state_graph, TimedStateGraphDecision,
        ZoneCheckLimits,
    };
    use fsl_eval::simulation::model::TimedSystem;

    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x", "y"], &PHASE_OPS),
        crate::zone_search::TIMED,
        &trap_body("false", ALWAYS_DONE),
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed subject");
    let graph = explore_timed_state_graph(&timed, ZoneCheckLimits::default(), &Cancel::new())
        .expect("closed graph");
    assert_eq!((graph.nodes.len(), graph.edges.len()), (3, 2));
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("Done");
    let instance = fsl_eval::model_subject::ModelInstance {
        population: world.request().universes[0].population.clone(),
        key: "c".to_owned(),
    };
    let over = system
        .bound_reference(claim.over().expect("over").object.clone(), &instance)
        .expect("bound c");
    let decision = decide_timed_state_graph(
        &timed,
        &graph,
        claim,
        Some(&over),
        ZoneCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("decided");
    let TimedStateGraphDecision::Refuted {
        stem,
        node,
        ef_nodes,
        paths,
    } = decision
    else {
        panic!("refuted: {decision:?}");
    };
    assert_eq!(ef_nodes, [2]);
    assert_eq!(node, 1);
    assert_eq!(stem.steps.len(), 1, "{stem:?}");
    // `first` at time 0 still leads on to phase 2.
    assert!(
        stem.steps[0]
            .delay
            .as_ref()
            .is_some_and(|delay| delay != &filament_exact::Rational::from_integer(0_i64.into())),
        "{stem:?}"
    );
    assert_eq!(paths, None);
}

/// The decision draws on what the exploration left: a budget the explored
/// graph fills exactly lets the search finish and stops the decision, and the
/// item settles `incomplete` naming it.
#[trace("TC-2412", "FR-852-AC-33")]
#[test]
fn a_budget_the_graph_fills_stops_the_decision() {
    use filament_exact::Cancel;
    use fsl_analyze::zone_check::{
        decide_timed_state_graph, explore_timed_state_graph, ZoneCheckLimit, ZoneCheckLimits,
        ZoneCheckRefusal, ZoneStop,
    };
    use fsl_eval::simulation::model::TimedSystem;

    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x", "y"], &PHASE_OPS),
        crate::zone_search::TIMED,
        &trap_body("false", ALWAYS_DONE),
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed subject");
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("Done");
    let instance = fsl_eval::model_subject::ModelInstance {
        population: world.request().universes[0].population.clone(),
        key: "c".to_owned(),
    };
    let over = system
        .bound_reference(claim.over().expect("over").object.clone(), &instance)
        .expect("bound c");
    // The graph holds three nodes and two edges.
    for (limits, limit) in [
        (
            ZoneCheckLimits {
                max_symbolic_states: 3,
                ..ZoneCheckLimits::default()
            },
            ZoneCheckLimit::MaxSymbolicStates(3),
        ),
        (
            ZoneCheckLimits {
                max_symbolic_transitions: 2,
                ..ZoneCheckLimits::default()
            },
            ZoneCheckLimit::MaxSymbolicTransitions(2),
        ),
    ] {
        let graph =
            explore_timed_state_graph(&timed, limits, &Cancel::new()).expect("the graph fits");
        assert_eq!((graph.nodes.len(), graph.edges.len()), (3, 2));
        let decided =
            decide_timed_state_graph(&timed, &graph, claim, Some(&over), limits, &Cancel::new());
        assert_eq!(
            decided,
            Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(limit)))
        );
    }
    let run = timed_run(
        "timed-sg-decision-budget",
        &trap_body("false", ALWAYS_DONE),
        &["x", "y"],
        &PHASE_OPS,
        json!([{"claim": "Done", "temporal": {"limits": {"max_states": 3}}}]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
}

/// Each zone pair a subtraction reads is charged to
/// `max_symbolic_transitions` by the producer: on the trap subject the
/// decision completes at `TOTAL` transitions, the two of the graph's edges
/// and the work of the decision, and stops at one fewer.
#[trace("TC-2412", "FR-852-AC-33")]
#[test]
fn the_producer_charges_each_pair_a_subtraction_reads() {
    use filament_exact::Cancel;
    use fsl_analyze::zone_check::{
        decide_timed_state_graph, explore_timed_state_graph, ZoneCheckLimit, ZoneCheckLimits,
        ZoneCheckRefusal, ZoneStop,
    };
    use fsl_eval::simulation::model::TimedSystem;

    const TOTAL: u64 = 9;

    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x", "y"], &PHASE_OPS),
        crate::zone_search::TIMED,
        &trap_body("false", ALWAYS_DONE),
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed subject");
    let graph = explore_timed_state_graph(&timed, ZoneCheckLimits::default(), &Cancel::new())
        .expect("closed graph");
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("Done");
    let instance = fsl_eval::model_subject::ModelInstance {
        population: world.request().universes[0].population.clone(),
        key: "c".to_owned(),
    };
    let over = system
        .bound_reference(claim.over().expect("over").object.clone(), &instance)
        .expect("bound c");
    let decide = |budget: u64| {
        let limits = ZoneCheckLimits {
            max_symbolic_transitions: budget,
            ..ZoneCheckLimits::default()
        };
        decide_timed_state_graph(&timed, &graph, claim, Some(&over), limits, &Cancel::new())
    };
    assert!(decide(TOTAL).is_ok());
    assert_eq!(
        decide(TOTAL - 1),
        Err(ZoneCheckRefusal::Stopped(ZoneStop::LimitReached(
            ZoneCheckLimit::MaxSymbolicTransitions(TOTAL - 1)
        )))
    );
}

/// The subject of the trap-before-pair order: phase 0 has two steps, `a` and
/// `b`, to phase 1 (a path pair to phase 1); phase 1 takes `c` to phase 2
/// and `d` to phase 3; phase 3 has no step, a trap. `e` at phase 2 has the
/// guard `guard`, which is undefined there when it divides by `2 - phase`.
/// The nodes are in that order: phase 0 is node 0, phase 2 is node 2 and
/// phase 3 is node 3.
fn trap_and_pair_decision(guard: &str) -> fsl_analyze::zone_check::TimedStateGraphDecision {
    let body = format!(
        "{}\
         pre A using v on M::Call::a {{ self.phase = 0 }}\n\
         post ADone using v on M::Call::a {{ self.phase = 1 }}\n\
         pre B using v on M::Call::b {{ self.phase = 0 }}\n\
         post BDone using v on M::Call::b {{ self.phase = 1 }}\n\
         pre C using v on M::Call::c {{ self.phase = 1 }}\n\
         post CDone using v on M::Call::c {{ self.phase = 2 }}\n\
         pre D using v on M::Call::d {{ self.phase = 1 }}\n\
         post DDone using v on M::Call::d {{ self.phase = 3 }}\n\
         pre E using v on M::Call::e {{ self.phase = 2 and {guard} }}\n\
         post EDone using v on M::Call::e {{ self.phase = 2 }}\n\
         function q using v(x: Integer): Integer pure {{ 6 mod x }}\n\
         unique path Done using v over (c: M::Call) \
         from (c.phase = 0 or c.phase = 3) to (c.phase = 1);\n",
        crate::timed_world::DENSE,
    );
    let ops: [(&str, &[&str]); 5] = [
        ("a", &["phase"]),
        ("b", &["phase"]),
        ("c", &["phase"]),
        ("d", &["phase"]),
        ("e", &["phase"]),
    ];
    let world = crate::timed_world::world_with_mode(
        &crate::timed_world::document(&["x"], &ops),
        &format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        ),
        &body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let system = world.system();
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, "M")
        .expect("the source selects a timed model");
    let limits = fsl_analyze::zone_check::ZoneCheckLimits::default();
    let graph = fsl_analyze::zone_check::explore_timed_state_graph(
        &timed,
        limits,
        &filament_exact::Cancel::new(),
    )
    .expect("the zone graph is explored");
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("the source package has the claim");
    let parameter = claim.over().expect("the claim is over a call");
    let instance = system
        .instances(&filament_exact::ValueType::Reference(
            parameter.object.clone(),
        ))
        .expect("enumerate claim instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .bound_reference(parameter.object.clone(), &instance)
        .expect("bind the claim instance");
    fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &graph,
        claim,
        Some(&over),
        limits,
        &filament_exact::Cancel::new(),
    )
    .expect("the claim is decided")
}

/// For `unique path` a trap is taken before a path pair, and the evidence is
/// compared with an undefined reading by its node: with the pair at node 0, an
/// undefined guard at node 2 and the trap at node 3, the undefined reading
/// decides; without it the trap at node 3 does, not the pair at node 0.
#[trace("TC-2412", "FR-852-AC-34")]
#[test]
fn a_unique_path_trap_precedes_a_pair_and_is_compared_by_its_node() {
    use fsl_analyze::zone_check::TimedStateGraphDecision;

    let decision = trap_and_pair_decision("q(2 - self.phase) >= 0");
    assert!(
        matches!(&decision, TimedStateGraphDecision::Undefined { node: 2, reading, .. }
            if format!("{:?}", reading.cause).contains("DivisionByZero")),
        "{decision:?}"
    );
    let decision = trap_and_pair_decision("true");
    let TimedStateGraphDecision::Refuted {
        stem, node, paths, ..
    } = decision
    else {
        panic!("refuted by the trap: {decision:?}");
    };
    assert_eq!((node, paths), (3, None), "{stem:?}");
    // `a` or `b`, `d`: the last step lands in phase 3.
    assert_eq!(stem.steps.len(), 2, "{stem:?}");
}

/// The subject of the pair order: `a` takes phase 0 (node 0) to phase 1; `c`
/// and `d` take phase 1 to phase 2, and `h` takes it to phase 3 (node 3); `f`
/// and `g` take phase 3 to phase 2 at once; `e` at phase 2 has the guard
/// `guard`, undefined there when it divides by `2 - phase`. The nodes are in
/// that order. With `unique path` from `from` to phase 2 there is no trap;
/// node 0 has its shortest pair (`a c`, `a d`) with sequences of two steps,
/// and node 3 has its pair (`f`, `g`) with sequences of one step.
fn pair_order_decision(
    guard: &str,
    from: &str,
) -> fsl_analyze::zone_check::TimedStateGraphDecision {
    let d_step = "pre D using v on M::Call::d { self.phase = 1 }\n\
                  post DDone using v on M::Call::d { self.phase = 2 }\n";
    let body = format!(
        "{}\
         pre A using v on M::Call::a {{ self.phase = 0 }}\n\
         post ADone using v on M::Call::a {{ self.phase = 1 }}\n\
         pre C using v on M::Call::c {{ self.phase = 1 }}\n\
         post CDone using v on M::Call::c {{ self.phase = 2 }}\n\
         {d_step}\
         pre H using v on M::Call::h {{ self.phase = 1 }}\n\
         post HDone using v on M::Call::h {{ self.phase = 3 }}\n\
         pre F using v on M::Call::f {{ self.phase = 3 }}\n\
         post FDone using v on M::Call::f {{ self.phase = 2 }}\n\
         pre G using v on M::Call::g {{ self.phase = 3 }}\n\
         post GDone using v on M::Call::g {{ self.phase = 2 }}\n\
         pre E using v on M::Call::e {{ self.phase = 2 and {guard} }}\n\
         post EDone using v on M::Call::e {{ self.phase = 2 }}\n\
         function q using v(x: Integer): Integer pure {{ 6 mod x }}\n\
         unique path Done using v over (c: M::Call) \
         from ({from}) to (c.phase = 2);\n",
        crate::timed_world::DENSE,
    );
    let ops: [(&str, &[&str]); 7] = [
        ("a", &["phase"]),
        ("c", &["phase"]),
        ("d", &["phase"]),
        ("e", &["phase"]),
        ("f", &["phase"]),
        ("g", &["phase"]),
        ("h", &["phase"]),
    ];
    let world = crate::timed_world::world_with_mode(
        &crate::timed_world::document(&["x"], &ops),
        &format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        ),
        &body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let system = world.system();
    let timed = fsl_eval::simulation::model::TimedSystem::new(&system, "M")
        .expect("the source selects a timed model");
    let limits = fsl_analyze::zone_check::ZoneCheckLimits::default();
    let graph = fsl_analyze::zone_check::explore_timed_state_graph(
        &timed,
        limits,
        &filament_exact::Cancel::new(),
    )
    .expect("the zone graph is explored");
    let claim = world
        .package
        .graph()
        .state_graph_claim("Done")
        .expect("the source package has the claim");
    let parameter = claim.over().expect("the claim is over a call");
    let instance = system
        .instances(&filament_exact::ValueType::Reference(
            parameter.object.clone(),
        ))
        .expect("enumerate claim instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .bound_reference(parameter.object.clone(), &instance)
        .expect("bind the claim instance");
    fsl_analyze::zone_check::decide_timed_state_graph(
        &timed,
        &graph,
        claim,
        Some(&over),
        limits,
        &filament_exact::Cancel::new(),
    )
    .expect("the claim is decided")
}

/// A `unique path` pair is taken at the earliest node that has one, with that
/// node's shortest sequences, though a later node has a pair of shorter
/// sequences; an undefined reading at node 2 outranks a pair only at a later
/// node.
#[trace("TC-2412", "FR-852-AC-34")]
#[test]
fn a_unique_path_pair_is_taken_at_the_earliest_node() {
    use fsl_analyze::zone_check::TimedStateGraphDecision;

    let undefined_guard = "q(2 - self.phase) >= 0";
    let both = "c.phase = 0 or c.phase = 3";
    let late = "c.phase = 3";
    for (guard, from, node, undefined) in [
        // Pairs at nodes 0 and 3: node 0's, with two-step sequences.
        ("true", both, 0, false),
        // The same with an undefined reading at node 2: the pair at node 0
        // is earlier, so it wins.
        (undefined_guard, both, 0, false),
        // Only node 3 is a source, and no undefined reading: node 3's pair,
        // with one-step sequences.
        ("true", late, 3, false),
        // Only node 3 is a source, and node 2 reads undefined: node 2.
        (undefined_guard, late, 2, true),
    ] {
        let decision = pair_order_decision(guard, from);
        match (&decision, undefined) {
            (TimedStateGraphDecision::Undefined { node: found, .. }, true) => {
                assert_eq!(*found, node, "{decision:?}");
            }
            (
                TimedStateGraphDecision::Refuted {
                    node: found,
                    stem,
                    paths: Some((first, second)),
                    ..
                },
                false,
            ) => {
                assert_eq!(*found, node, "{decision:?}");
                // From node 0 each sequence has two steps, from node 3 one;
                // node 0 is reached by no step.
                let steps = if node == 0 { 2 } else { 1 };
                assert_eq!((first.len(), second.len()), (steps, steps), "{decision:?}");
                assert_eq!(
                    stem.steps.len(),
                    if node == 0 { 0 } else { 2 },
                    "{decision:?}"
                );
            }
            _ => panic!("guard {guard}, from {from}: {decision:?}"),
        }
    }
}

/// FR-125 "Instancing": on the digital probabilistic route, the proved entry
/// and the refuted binding of `over (c: Base)` name the admitted `Derived`,
/// and the proof's certificate accepts that binding.
#[trace("TC-520", "FR-125-AC-9")]
#[test]
fn a_digital_probability_binding_names_the_admitted_member_type() {
    for (comparison, expected) in [("<= 0.5", "proved"), (">= 0.5", "refuted")] {
        let item = settle_over_base(
            &format!(
                "probabilistic OverBase using tt on M over (c: Base) under every scheduler \
                 on origin {{ probability {comparison} [ eventually[0 ms, 1 ms] holds(false) ] }}\n"
            ),
            json!({"claim": "OverBase"}),
        );
        assert_eq!(label(&item), expected, "{item}");
        let binding = if expected == "proved" {
            assert_eq!(item["certificate"]["accepted"], true, "{item}");
            &item["record"]["basis"]["entries"][0]["binding"]
        } else {
            &item["counterexample"]["binding"]
        };
        assert_eq!(*binding, the_admitted_derived(), "{item}");
    }
}
/// A timed RPC unit with one undefined source. `U-P` makes the claim target
/// undefined at phase 2; the other variants make a subject-side predicate
/// undefined at the same reachable state.
fn fr852_gap_undefined_unit(source: &str) -> String {
    let q = "function q using v(x: Integer): Integer pure { 6 mod x }\n";
    let profile = "profile d = \"filament.value.integer-division.truncating/v1\";\n";
    let base = match source {
        "U-G" => unit(true).replace(
            "not self.inflight and (self.phase = 2 or self.phase = 3)",
            "q(2 - self.phase) >= 0",
        ),
        "U-T" => unit(false) + "\nterminal Done using v on M when { q(2 - self.phase) >= 0 };\n",
        "U-C" => unit(true).replacen(
            "pre SendIdle",
            "constraint C using v on M::Call at current { q(2 - self.phase) >= 0 }\npre SendIdle",
            1,
        ),
        "U-P" => unit(true),
        _ => panic!("known undefined source variant: {source}"),
    };
    let with_function = base.replacen("pre SendIdle", &format!("{q}pre SendIdle"), 1);
    let with_profile = with_function.replacen("model M = ", &format!("{profile}model M = "), 1);
    if source == "U-P" {
        with_profile + "\npossible UndefinedTarget using v over (c: M::Call) { c.phase = 2 and q(2 - c.phase) > 0 }\n\
         always possible UndefinedAlways using v over (c: M::Call) { q(2 - c.phase) >= 0 }\n\
         unique path UndefinedUnique using v over (c: M::Call) from (c.phase = 3) to (q(2 - c.phase) >= 0);\n"
    } else {
        with_profile
            + "\nalways possible UndefinedAlways using v over (c: M::Call) { c.phase = 2 }\n\
         unique path UndefinedUnique using v over (c: M::Call) from (c.phase = 3) to (c.phase = 2);\n"
    }
}

fn fr852_gap_request_item(variant: &str, claim: &str, options: Option<Value>) -> Value {
    let mut item = json!({"claim": claim});
    if let Some(options) = options {
        item["options"] = options;
    }
    if variant == "U-C" {
        item["reductions"] = json!({"constraint": "C"});
    }
    item
}

struct Fr852GapRun {
    items: Vec<Value>,
    records: Vec<fsl_analyze::analyze::AnalyzeRecord>,
}

impl Fr852GapRun {
    fn item(&self, index: usize) -> Value {
        self.items[index].clone()
    }

    fn record(&self, index: usize) -> &fsl_analyze::analyze::AnalyzeRecord {
        &self.records[index]
    }
}

fn fr852_gap_analyze(_name: &str, source: &str, items: Value) -> Fr852GapRun {
    use fsl_analyze::analyze::{
        test_support_state_graph_item, AnalyzeLimits, DeadlineSupply, Engines, RunInput,
        SourceProvision, SubjectBinding, TemporalItem,
    };
    use fsl_foundation::RequestIndex;

    let body = source
        .split_once("model M = \"test/timed\";\n")
        .map(|(_, body)| body)
        .expect("the fixture carries its model declaration");
    let profiles = if source.contains("profile d =") {
        format!(
            "{}profile d = \"filament.value.integer-division.truncating/v1\";\n",
            crate::zone_search::TIMED
        )
    } else {
        crate::zone_search::TIMED.to_owned()
    };
    let world = crate::timed_world::world_with_mode(
        &crate::zone_search::rpc_document(),
        &profiles,
        body,
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let request = world.request();
    let package = &world.package;
    let phase_two_initial = items.as_array().is_some_and(|selections| {
        selections.len() == 1
            && selections[0]["claim"] == "UndefinedTarget"
            && selections[0]["options"]["method"] == "digital-clocks"
    });
    let mut snapshots = world.snapshots.clone();
    if phase_two_initial {
        let snapshot = snapshots
            .values_mut()
            .next()
            .expect("the U-P fixture has one initial snapshot");
        let mut document: Value =
            serde_json::from_slice(snapshot).expect("the initial snapshot is valid JSON");
        document["populations"][0]["objects"][0]["fields"]["phase"] = json!({"integer": "2"});
        *snapshot = serde_json::to_vec(&document).expect("the phase-2 snapshot serializes");
    }
    let input = RunInput::source(package);
    let binding = SubjectBinding {
        packages: request.packages,
        snapshots: &snapshots,
        initial: request.initial,
        universes: request.universes,
    };
    let source_ref = package.graph().source();
    let provision = SourceProvision::single(source_ref, &world.unit);
    let limits = AnalyzeLimits::published(DeadlineSupply::Unbounded);
    let engines = Engines::fsl().expect("the layer-A engines register");
    let selections = items
        .as_array()
        .expect("the scenario supplies item selections");
    let records = selections
        .iter()
        .enumerate()
        .map(|(index, selection)| {
            let name = selection["claim"]
                .as_str()
                .expect("each selection names its claim");
            let claim = package
                .graph()
                .state_graph_claim(name)
                .expect("the checked package contains the selected claim");
            let key = claim.occurrence().expect("the claim has an occurrence");
            let mut form = TemporalItem::default();
            if let Some(constraint) = selection["reductions"]["constraint"].as_str() {
                form.reductions.constraint = Some(constraint.to_owned());
            }
            let method = selection["options"]["method"].as_str();
            test_support_state_graph_item(
                &engines,
                &input,
                &binding,
                limits,
                &provision,
                key,
                RequestIndex::new(index),
                &form,
                method,
                &filament_exact::Cancel::new(),
            )
            .expect("the selected state-graph claim builds an item")
        })
        .collect::<Vec<_>>();
    let items = records
        .iter()
        .map(|record| {
            let outcome = if let Some(graph) = &record.state_graph {
                let terminal = record.terminal().expect("the graph item is terminal");
                let backend = terminal
                    .backend()
                    .expect("the graph item names its backend")
                    .clone();
                fsl_analyze::analyze::analyze_item(
                    terminal,
                    &[fsl_analyze::analyze::ItemCounterexample {
                        request_index: record.request_index,
                        backend,
                        counterexample: fsl_analyze::analyze::Counterexample::StateGraph(
                            graph.clone(),
                        ),
                        wire: Ok(Value::Null),
                    }],
                )
                .expect("the analyzed graph writes FR-286 and FR-393 evidence")
            } else {
                record
                    .outcome_item(package)
                    .expect("the analyzed claim writes an outcome")
                    .unwrap_or_else(|error| {
                        panic!(
                            "the outcome carries FR-286 and FR-393 evidence: {error}; {record:?}"
                        )
                    })
            };
            serde_json::to_value(outcome).expect("the outcome serializes")
        })
        .collect();
    Fr852GapRun { items, records }
}

fn fr852_gap_assert_undefined_record(
    item: &Value,
    expected_locus: &str,
    expected_position: u64,
    record: &fsl_analyze::analyze::AnalyzeRecord,
) {
    assert_eq!(item["result"], "refuted", "{item}; {record:?}");
    let graph = record
        .state_graph
        .as_ref()
        .expect("the state-graph item settled its full record");
    assert_eq!(
        graph.record.label,
        fsl_replay::ResultLabel::Refuted,
        "{graph:?}"
    );
    assert_eq!(
        graph.record.basis,
        fsl_replay::SettlementBasis::DecisiveCounterexample,
        "{graph:?}"
    );
    assert_eq!(
        graph.record.execution,
        fsl_replay::Execution::Completed,
        "a completed refutation includes successful replay: {graph:?}"
    );
    assert!(graph.counterexample.is_some(), "FR-393 evidence: {graph:?}");
    let evidence = &item["counterexample"]["undefined"];
    assert_eq!(evidence["type"], "undefined-evaluation", "{item}");
    assert_eq!(evidence["cause"], "division-by-zero", "{item}");
    assert_eq!(evidence["where"]["position"], expected_position, "{item}");
    assert!(evidence["where"]["state_key"]["key"].is_string(), "{item}");
    assert!(
        evidence["where"]["locus"]["occurrence"].is_object(),
        "{item}"
    );
    match expected_locus {
        "target" => assert!(evidence["where"]["predicate"].is_string(), "{item}"),
        "guard" => assert!(evidence["where"]["transition"].is_object(), "{item}"),
        "terminal" | "constraint" => {
            assert!(evidence["where"].get("transition").is_none(), "{item}")
        }
        _ => panic!("known undefined locus: {expected_locus}"),
    }
}

/// FR-852-AC-16 and AC-24: at adapter level, not the command route, EN-6
/// reports U-P target undefinedness for both forms with FR-393 evidence.
#[trace("TC-2412", "FR-852-AC-16")]
#[trace("TC-2412", "FR-852-AC-24")]
#[test]
fn fr852_gap_zone_route_refutes_undefined_target_for_always_and_unique() {
    let source = fr852_gap_undefined_unit("U-P");
    let run = fr852_gap_analyze(
        "fr852-gap-zone-target",
        &source,
        json!([
            {"claim": "UndefinedAlways"},
            {"claim": "UndefinedUnique"}
        ]),
    );
    for (index, item) in [run.item(0), run.item(1)].into_iter().enumerate() {
        fr852_gap_assert_undefined_record(&item, "target", 2, run.record(index));
        assert!(
            run.record(index)
                .terminal()
                .and_then(|terminal| terminal.backend())
                .is_some_and(|backend| backend.as_str() == "en-6"),
            "{:?}",
            run.record(index)
        );
        assert!(
            run.record(index)
                .state_graph
                .as_ref()
                .is_some_and(|graph| graph.counterexample.is_some()),
            "{:?}",
            run.record(index)
        );
    }
}

/// FR-852-AC-18/19/26/27: at adapter level, not the command route, EN-6
/// settles U-T and U-C for both forms with FR-393 undefined evidence.
#[trace("TC-2412", "FR-852-AC-18")]
#[trace("TC-2412", "FR-852-AC-19")]
#[trace("TC-2412", "FR-852-AC-26")]
#[trace("TC-2412", "FR-852-AC-27")]
#[test]
fn fr852_gap_zone_route_refutes_undefined_terminal_and_constraint() {
    for (variant, claim, locus, name) in [
        ("U-T", "UndefinedAlways", "terminal", "terminal-always"),
        ("U-C", "UndefinedAlways", "constraint", "constraint-always"),
        ("U-T", "UndefinedUnique", "terminal", "terminal-unique"),
        ("U-C", "UndefinedUnique", "constraint", "constraint-unique"),
    ] {
        let run = fr852_gap_analyze(
            &format!("fr852-gap-zone-{name}"),
            &fr852_gap_undefined_unit(variant),
            json!([fr852_gap_request_item(variant, claim, None)]),
        );
        let item = run.item(0);
        fr852_gap_assert_undefined_record(&item, locus, 2, run.record(0));
        assert!(
            run.record(0)
                .terminal()
                .and_then(|terminal| terminal.backend())
                .is_some_and(|backend| backend.as_str() == "en-6"),
            "{:?}",
            run.record(0)
        );
        assert!(
            run.record(0)
                .state_graph
                .as_ref()
                .is_some_and(|graph| graph.counterexample.is_some()),
            "{:?}",
            run.record(0)
        );
    }
}

/// FR-852-AC-12..15: at adapter level, not the command route, EN-1's
/// digital-clocks method settles each source with FR-393 evidence and replay.
#[trace("TC-2412", "FR-852-AC-12")]
#[trace("TC-2412", "FR-852-AC-13")]
#[trace("TC-2412", "FR-852-AC-14")]
#[trace("TC-2412", "FR-852-AC-15")]
#[test]
fn fr852_gap_digital_clocks_refutes_all_undefined_sources() {
    for (variant, claim, locus, name, position) in [
        ("U-P", "UndefinedTarget", "target", "target", 0),
        ("U-G", "ReplyPossible", "guard", "guard", 2),
        ("U-T", "ReplyPossible", "terminal", "terminal", 2),
        ("U-C", "ReplyPossible", "constraint", "constraint", 2),
    ] {
        let run = fr852_gap_analyze(
            &format!("fr852-gap-digital-{name}"),
            &fr852_gap_undefined_unit(variant),
            json!([fr852_gap_request_item(
                variant,
                claim,
                Some(json!({"method": "digital-clocks"}))
            )]),
        );
        let item = run.item(0);
        fr852_gap_assert_undefined_record(&item, locus, position, run.record(0));
        assert!(
            run.record(0)
                .terminal()
                .and_then(|terminal| terminal.backend())
                .is_some_and(|backend| backend.as_str() == "en-1"),
            "{:?}",
            run.record(0)
        );
        assert!(
            run.record(0)
                .state_graph
                .as_ref()
                .is_some_and(|graph| graph.counterexample.is_some()),
            "{:?}",
            run.record(0)
        );
    }
}

/// FR-852-AC-20..23 and AC-28..31: at adapter level, not the command route,
/// EN-1 returns the exact digital-clocks decline for each source and form.
#[trace("TC-2412", "FR-852-AC-20")]
#[trace("TC-2412", "FR-852-AC-21")]
#[trace("TC-2412", "FR-852-AC-22")]
#[trace("TC-2412", "FR-852-AC-23")]
#[trace("TC-2412", "FR-852-AC-28")]
#[trace("TC-2412", "FR-852-AC-29")]
#[trace("TC-2412", "FR-852-AC-30")]
#[trace("TC-2412", "FR-852-AC-31")]
#[test]
fn fr852_gap_digital_clocks_declines_unsupported_forms_for_all_sources() {
    for (variant, claim, requested_form, expected_name) in [
        (
            "U-P",
            "UndefinedAlways",
            "always-possible",
            "always-possible",
        ),
        (
            "U-G",
            "UndefinedAlways",
            "always-possible",
            "always-possible",
        ),
        (
            "U-T",
            "UndefinedAlways",
            "always-possible",
            "always-possible",
        ),
        (
            "U-C",
            "UndefinedAlways",
            "always-possible",
            "always-possible",
        ),
        ("U-P", "UndefinedUnique", "unique-path", "unique-path"),
        ("U-G", "UndefinedUnique", "unique-path", "unique-path"),
        ("U-T", "UndefinedUnique", "unique-path", "unique-path"),
        ("U-C", "UndefinedUnique", "unique-path", "unique-path"),
    ] {
        let run = fr852_gap_analyze(
            &format!("fr852-gap-decline-{variant}-{requested_form}"),
            &fr852_gap_undefined_unit(variant),
            json!([fr852_gap_request_item(
                variant,
                claim,
                Some(json!({"method": "digital-clocks"}))
            )]),
        );
        let item = run.item(0);
        assert!(item.to_string().contains("not_digitisable"), "{item}");
        assert!(item.to_string().contains(expected_name), "{item}");
        assert!(
            matches!(
                &run.record(0).settlement,
                fsl_analyze::analyze::Settlement::Terminal(_)
            ),
            "the EN-1 request settles its decline: {:?}",
            run.record(0)
        );
        assert!(
            run.record(0)
                .terminal()
                .and_then(|terminal| terminal.backend())
                .is_some_and(|backend| backend.as_str() == "en-1"),
            "the decline uses the EN-1 backend: {:?}",
            run.record(0)
        );
        assert!(
            run.record(0).state_graph.is_none(),
            "a declined form does not search: {:?}",
            run.record(0)
        );
    }
}
