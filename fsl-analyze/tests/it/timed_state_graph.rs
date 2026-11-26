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
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FSpec-FR-712-AC-2")]
#[test]
fn zones_refute_always_possible_when_part_of_a_zone_is_trapped() {
    let item = trapped_with("timed-sg-part-zone-trap", "false", ALWAYS_DONE);
    assert_refuted_and_replayed(&item);
}

/// `unique path` counts the ways from every reachable `X` state: a trapped
/// state has none, so the claim is false although every other state has one.
#[trace("TC-2412", "FR-852-AC-3")]
#[trace("TC-2412", "FSpec-FR-712-AC-3")]
#[test]
fn zones_refute_unique_path_when_part_of_a_zone_has_no_path() {
    let item = trapped_with("timed-sg-part-zone-unique", "false", UNIQUE_DONE);
    assert_refuted_and_replayed(&item);
}

/// `zero` takes `t = 0` and `one` exactly the other times, so every state has
/// a way, and one way, although two steps leave the node of phase 1.
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FR-852-AC-3")]
#[test]
fn zones_prove_over_steps_that_split_a_zone_between_them() {
    let guard = "self.y >= 2 ms and self.x < 2 ms";
    assert_proved_and_certified(&trapped_with("timed-sg-split-always", guard, ALWAYS_DONE));
    assert_proved_and_certified(&trapped_with("timed-sg-split-unique", guard, UNIQUE_DONE));
}

/// At `t = 0` both `zero` and `one` run: two sequences from one state.
#[trace("TC-2412", "FR-852-AC-3")]
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
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FR-852-AC-3")]
#[test]
fn zones_refute_when_only_a_strict_boundary_is_trapped() {
    let guard = "self.y >= 2 ms and self.x < 2 ms and self.x > 1 ms";
    assert_refuted_and_replayed(&trapped_with("timed-sg-edge-always", guard, ALWAYS_DONE));
    assert_refuted_and_replayed(&trapped_with("timed-sg-edge-unique", guard, UNIQUE_DONE));
}

/// TC-2412 step 8's subject on EN-6: with `first` strictly between 0 and
/// 1 ms, neither `zero` nor `one` is ever taken, so both claims are false.
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FR-852-AC-3")]
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
#[trace("TC-2412", "FR-852-AC-3")]
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
#[trace("TC-2412", "FR-852-AC-1")]
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
#[trace("TC-2412", "FR-852-AC-2")]
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
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FR-852-AC-3")]
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
#[trace("TC-2412", "FR-852-AC-2")]
#[trace("TC-2412", "FSpec-FR-712-AC-2")]
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
#[trace("TC-2412", "FR-852-AC-1")]
#[trace("TC-2412", "FR-852-AC-3")]
#[trace("TC-2412", "FSpec-FR-712-AC-3")]
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
/// whole zone lies in `E(P)`, and the initial node holds a trapped state,
/// reached by waiting.
#[trace("TC-2412", "FR-852-AC-2")]
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
    assert_eq!(node, 0);
    assert!(stem.steps.is_empty(), "{stem:?}");
    assert_eq!(paths, None);
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
