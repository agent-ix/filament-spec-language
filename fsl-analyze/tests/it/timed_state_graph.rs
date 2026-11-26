// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-852 and FSpec FR-712 (TC-2412): `possible`, `always possible` and
//! `unique path` items over the timed `Rpc` subject route to EN-6 and are
//! decided over the timed state graph on zones, with exact-rational evidence
//! that replays and a zone certificate.

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
    use crate::timed_world::{world_over, DENSE};
    use fsl_analyze::analyze::document::{
        analyze_document, resolve_items, AnalyzeRun, ItemSelection, SubjectSelection,
    };
    use fsl_analyze::analyze::{AnalyzeLimits, Engines, SourceProvision, SubjectBinding};
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
        &AnalyzeLimits::unbounded(),
    );
    let (document, _) = analyze_document(
        AnalyzeRun {
            package: &world.package,
            binding,
            provision: SourceProvision::single(&source, &world.unit),
            items,
            engines: &engines,
            limits: AnalyzeLimits::unbounded(),
        },
        &crate::analyze_cli_support::Fr290,
        &filament_exact::Cancel::new(),
    )
    .expect("a settled run");
    document["items"][0].clone()
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
