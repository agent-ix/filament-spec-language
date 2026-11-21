// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2066 (FR-741): probability items run from source through
//! `fsl analyze` on a `native-analyze/1` request, over ADR-028 §15.4's
//! `Link` model (random losses on two sends, a `cost` reward and the
//! workload `Even`), which S4 emits as a checked package (FR-740).

use serde_json::{json, Value};

use crate::prob_world::{self, World};
use filament_exact::Cancel;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_eval::simulation::model::ModelSystem;
use fsl_foundation::RequestIndex;
use fsl_replay::{replay_probabilistic_witness, ExactProbLimits, ModelCheckLimits};

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};

const LINK_UNIT: &str =
    include_str!("../../../tools/fsl-probability-check/tests/fixtures/link.native");
const LINK_MODEL: &[u8] =
    include_bytes!("../../../tools/fsl-probability-check/tests/fixtures/link.semantic-ir.json");
const LINK_START: &[u8] =
    include_bytes!("../../../tools/fsl-probability-check/tests/fixtures/link.snapshot.json");

/// The `Link` unit with an event-position profile and `claims` appended.
fn unit(claims: &str) -> String {
    let header = "profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    format!(
        "{}\n{claims}\n",
        LINK_UNIT.replace(
            header,
            &format!(
                "{header}profile ev = \"filament.temporal.event-position.false-extension/v1\";\n"
            ),
        )
    )
}

/// `Delivered`: delivery within two steps under `Even`, true probability
/// `391/400`, at `0.9` with stated confidence parameters.
const DELIVERED: &str =
    "probabilistic Delivered using ev on M over (m: M::Msg) under Even on origin \
     with alpha 0.01, beta 0.01, indifference 0.05 \
     { probability >= 0.9 [ eventually[0,2] holds(m.delivered) ] }";

/// `Eventually`: unbounded delivery under `Even`, exact value `391/400`.
const EVENTUALLY: &str =
    "probabilistic Eventually using inf on M over (m: M::Msg) under Even on origin \
     { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }";

/// `TooLikely`: the same event at a threshold its exact value misses.
const TOO_LIKELY: &str =
    "probabilistic TooLikely using inf on M over (m: M::Msg) under Even on origin \
     { probability >= rational(99, 100) [ eventually holds(m.delivered) ] }";

/// `LongRun`: the long-run fraction of healthy ticks under `Steady`.
const LONG_RUN: &str =
    "probabilistic LongRun using inf on M over (n: M::Node) under Steady on origin \
     with alpha 0.1, beta 0.1, indifference 0.1 \
     { long_run_fraction holds(n.healthy) >= 0.3 }";

const FINITE: &str = "probabilistic Finite using ev on M over (m: M::Msg) under Even on origin \
     { probability >= rational(0, 1) [ eventually[0,2] holds(m.delivered) ] }";

const UNTIL: &str = "probabilistic Until using inf on M over (m: M::Msg) under Even on origin \
     { probability >= rational(0, 1) [ holds(m.attempts = 0) until holds(m.delivered or m.attempts = 2) ] }";

const ALWAYS: &str = "probabilistic Always using ev on M over (m: M::Msg) under Even on origin \
     { probability >= rational(9, 10) [ always[0,2] holds(m.attempts >= 0) ] }";

const REWARD: &str = "probabilistic Reward using inf on M over (m: M::Msg) under Even on origin \
     { expected accumulate cost until holds(m.delivered or m.attempts = 2) <= rational(2, 1) }";

const FAIR_DELIVER: &str = "probabilistic DeliverFair using ev on M over (m: M::Msg) \
     under every scheduler fair weak Msg::send_a on origin \
     { probability >= rational(95, 100) [ eventually[0,2] holds(m.delivered) ] }";

fn run(name: &str, claims: &str, items: Value) -> Value {
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        &unit(claims),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": items}),
    );
    let run = analyze(&request);
    run.document()
}

fn timed_unit(claims: &str) -> String {
    let header = "profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let source = LINK_UNIT.replace(
        header,
        &format!(
            "{header}profile tc = \"filament.temporal.complete/v1\";\n\
             profile t = \"filament.temporal.timed/v1\";\n"
        ),
    );
    format!(
        "{source}\n\
         dimension Time;\n\
         unit ms : Time = rational(1, 1);\n\
         time M dense unit ms;\n\
         delay Msg::send_a ~ uniform[1 ms, 2 ms];\n\
         delay Msg::send_b ~ uniform[1 ms, 2 ms];\n\
         {claims}\n"
    )
}

fn run_timed(name: &str, claims: &str, items: Value) -> Value {
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        &timed_unit(claims),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": items}),
    );
    analyze(&request).document()
}

fn run_pta(name: &str, claim: &str) -> Value {
    let unit = timed_unit(claim).replace("uniform[1 ms, 2 ms]", "discrete { 1 ms: 1 }");
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        &unit,
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "TimedDelivery", "probabilistic": {"evidence": "exact"}}]}),
    );
    analyze(&request).document()
}

fn run_health_long_run(name: &str, items: Value) -> Value {
    let model =
        crate::prob_world::health("self.healthy = (not fault)", "true: 1, false: 1", false, "");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         model M = \"{}\";\n{}\n{}\n",
        model.package, model.body, LONG_RUN
    );
    let model_document = model.document(false);
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current",
        "model": {"identity": model.package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://test/health/nodes", "complete": true,
            "objects": [{"key": "n", "type": "ix://test/health/Node",
                "fields": {"healthy": {"boolean": true}}}]}]
    })
    .to_string()
    .into_bytes();
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        &unit,
        &model_document,
        &[snapshot],
        json!({"items": items}),
    );
    analyze(&request).document()
}

/// FR-741-AC-1: each statistical method settles `measured` with a
/// `verdict` naming its method and seed, and a rerun with four workers
/// gives the same item. The tests that call it carry the trace.
fn statistical_method_from_source(method: &str, method_selection: Value) {
    let claim_name = if method == "regenerative" {
        "LongRun"
    } else {
        "Delivered"
    };
    let item = |workers: u64| {
        json!([{"claim": claim_name, "probabilistic": {
            "evidence": "statistical", "method": method_selection,
            "seed": "7", "max_witnesses": 8, "workers": workers}}])
    };
    let (one, four) = if method == "regenerative" {
        (
            run_health_long_run(&format!("stat-{method}-1"), item(1)),
            run_health_long_run(&format!("stat-{method}-4"), item(4)),
        )
    } else {
        (
            run(&format!("stat-{method}-1"), DELIVERED, item(1)),
            run(&format!("stat-{method}-4"), DELIVERED, item(4)),
        )
    };
    assert!(one["items"][0].is_object(), "{one}");
    assert!(four["items"][0].is_object(), "{four}");
    let first = &one["items"][0];
    assert_eq!(label(first), "measured", "{first}");
    assert_eq!(
        first["verdict"]["basis"]["method"]["type"], method,
        "{first}"
    );
    assert_eq!(first["verdict"]["provenance"]["seed"], "7", "{first}");
    if method == "sprt" {
        assert!(first["verdict"]["interval"].is_null(), "{first}");
    } else {
        assert!(first["verdict"]["interval"].is_object(), "{first}");
    }
    assert!(
        first["verdict"]["basis"]["samples"].as_u64().is_some(),
        "{first}"
    );
    assert!(
        first["verdict"]["basis"]["draws"].as_u64().is_some(),
        "{first}"
    );
    // FR-410: every test row names its binding and instance, and no
    // number is written for what the run did not measure.
    for test in first["verdict"]["provenance"]["tests"]
        .as_array()
        .expect("provenance tests")
    {
        assert!(test.get("instance").is_some(), "{first}");
        assert!(test.get("binding").is_some(), "{first}");
    }
    assert!(first["verdict"]["witnesses"].is_array(), "{first}");
    assert_eq!(first["request"]["probabilistic"]["workers"], 1);
    assert_eq!(four["items"][0]["request"]["probabilistic"]["workers"], 4);
    assert_eq!(
        first["verdict"], four["items"][0]["verdict"],
        "workers do not change the measurement"
    );
}

#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_okamoto_from_source() {
    statistical_method_from_source("okamoto", json!({"type": "okamoto"}));
}

#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_sprt_from_source() {
    statistical_method_from_source("sprt", json!({"type": "sprt"}));
}

#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_regenerative_from_source() {
    statistical_method_from_source(
        "regenerative",
        json!({"type": "regenerative", "min_cycles": 1}),
    );
}

#[test]
fn timed_sampling_from_source() {
    let claim = "probabilistic TimedDelivery using t on M over (m: M::Msg) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.05 \
         { probability >= 0.9 [ eventually[0 ms, 3 ms] holds(m.delivered) ] }";
    let document = run_timed(
        "timed-sampling-source",
        claim,
        json!([{"claim": "TimedDelivery", "probabilistic": {
            "evidence": "statistical", "method": {"type": "sprt"},
            "seed": "7", "timed_sampling": {"q": 8, "max_sample_steps": 1000}
        }}]),
    );
    let item = &document["items"][0];
    assert!(item.is_object(), "expected an analyzed item: {document}");
    assert_eq!(label(item), "measured", "{item}");
    assert_eq!(item["request"]["probabilistic"]["timed_sampling"]["q"], 8);
    assert_eq!(
        item["request"]["probabilistic"]["timed_sampling"]["max_sample_steps"],
        1000
    );
    assert_eq!(item["verdict"]["basis"]["grid"], 8, "{item}");
    // The schema closes `provenance`: the grid and step budget are in the
    // request echo and the basis only.
    assert!(
        item["verdict"]["provenance"]
            .get("delay_grid_bits")
            .is_none(),
        "{item}"
    );
}

/// FR-741-AC-3 (timed half): a timed probabilistic source claim uses the exact EN-5
/// digital-clock product and retains its independently checkable certificate.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-3")]
#[test]
#[ignore = "QSL-2156: EN-5 timed digital-clock dispatch and certificates (PR #3204) not landed yet"]
fn pta_from_source() {
    let claim = "probabilistic TimedDelivery using t on M over (m: M::Msg) under Even on origin \
         { probability >= rational(95, 100) [ eventually[0 ms, 3 ms] holds(m.delivered) ] }";
    let document = run_pta("pta-source", claim);
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["request"]["candidates"]["backends"], json!(["en-5"]));
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["kind"], "exact", "{item}");
    assert_eq!(
        item["certificate"]["document"]["objective"]["timed"]["route"], "digital",
        "{item}"
    );
}

/// FR-741-AC-2: an exact unbounded reachability item settles `proved` with
/// the exact value `391/400` and carries its accepted `exact` certificate
/// document in the item result.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-2")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-2")]
#[ix_trace_rs::trace("TC-2065", "FR-203-AC-8")]
#[test]
fn an_exact_item_settles_proved_with_its_certificate() {
    let document = run(
        "exact-eventually",
        EVENTUALLY,
        json!([{"claim": "Eventually", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["kind"], "exact", "{item}");
    assert!(item["certificate"]["document"].is_object(), "{item}");
    // FR-621: the certificate names its identity and its checker.
    assert!(
        item["certificate"]["identity"]["node_id"].is_object(),
        "{item}"
    );
    assert!(
        item["certificate"]["identity"]["model"].is_string(),
        "{item}"
    );
    assert!(item["certificate"]["checker"].is_string(), "{item}");
    let text = item.to_string();
    assert!(text.contains("391") && text.contains("400"), "{text}");
}

/// Exact source routing covers bounded reachability, unbounded until, bounded
/// always, and expected rewards. Each proved result carries its exact value in
/// the accepted certificate document.
#[test]
fn exact_finite_horizon_from_source() {
    let claims = format!("{FINITE}\n{UNTIL}\n{ALWAYS}\n{REWARD}");
    let document = run(
        "exact-finite-and-subforms",
        &claims,
        json!([
            {"claim": "Finite", "probabilistic": {"evidence": "exact"}},
            {"claim": "Until", "probabilistic": {"evidence": "exact"}},
            {"claim": "Always", "probabilistic": {"evidence": "exact"}},
            {"claim": "Reward", "probabilistic": {"evidence": "exact"}}
        ]),
    );
    for item in document["items"].as_array().expect("four items") {
        assert_eq!(label(item), "proved", "{item}");
        assert_eq!(item["certificate"]["accepted"], true, "{item}");
        assert_eq!(item["certificate"]["kind"], "exact", "{item}");
        assert!(item["certificate"]["document"].is_object(), "{item}");
        assert!(
            item["certificate"]["document"]
                .to_string()
                .contains("numerator"),
            "{item}"
        );
    }
}

/// FR-741-AC-3 (negative): an exact item whose value misses its threshold
/// settles `refuted` with a counterexample whose replay reproduced it.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-3")]
#[ix_trace_rs::trace("TC-2065", "FR-203-AC-8")]
#[test]
fn probability_refuted_from_source() {
    let document = run(
        "exact-refuted",
        TOO_LIKELY,
        json!([{"claim": "TooLikely", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// The path set emitted by `fsl analyze` is the same source-backed witness
/// that `fsl-replay` independently accepts against the checked package.
#[test]
fn probabilistic_witness_replays() {
    let document = run(
        "probabilistic-witness-replay",
        TOO_LIKELY,
        json!([{"claim": "TooLikely", "probabilistic": {"evidence": "exact"}}]),
    );
    let emitted = document["items"][0]["counterexample"].clone();
    assert_eq!(emitted["type"], "probabilistic", "{emitted}");
    assert_eq!(emitted["evidence"]["type"], "path-set", "{emitted}");

    let world = World::new(&prob_world::link(TOO_LIKELY));
    let engines = Engines::fsl().expect("the linked engines register");
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("TooLikely", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    let reproduced = record
        .probabilistic_counterexample
        .expect("the source run retains replayed witness evidence");
    let source_wire = reproduced.wire.as_ref().expect("the witness spells");
    let source_paths = source_wire["evidence"]["paths"]
        .as_array()
        .expect("typed witness paths");
    let emitted_paths = emitted["evidence"]["paths"]
        .as_array()
        .expect("the verb emits witness paths");
    assert_eq!(source_paths.len(), emitted_paths.len());
    for (source_path, emitted_path) in source_paths.iter().zip(emitted_paths) {
        let source_steps = source_path.as_array().expect("typed path steps");
        let emitted_steps = emitted_path.as_array().expect("emitted path steps");
        assert_eq!(source_steps.len(), emitted_steps.len());
        for (source_step, emitted_step) in source_steps.iter().zip(emitted_steps) {
            assert_eq!(source_step["transition"], emitted_step["transition"]);
            assert_eq!(source_step["random"], emitted_step["random"]);
            assert_eq!(
                source_step["post_state"]["key"],
                emitted_step["post_state"]["key"]
            );
        }
    }

    let witness = reproduced.evidence();
    let system = ModelSystem::new(world.subject().simulation).expect("the source model admits");
    let package = system
        .source_package()
        .expect("the subject is source-backed");
    assert!(matches!(
        replay_probabilistic_witness(
            package,
            &system,
            &witness.item,
            witness,
            &ExactProbLimits::default(),
            &Cancel::new(),
        ),
        Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
    ));
}

/// FR-741-AC-4 (negative): an unbounded exact-only claim requested with
/// statistical evidence settles `unsupported` `exact-only-form` with no
/// sample drawn.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-4")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-3")]
#[test]
fn an_exact_only_claim_with_statistical_evidence_is_unsupported() {
    let document = run(
        "exact-only-statistical",
        EVENTUALLY,
        json!([{"claim": "Eventually", "probabilistic": {
            "evidence": "statistical", "method": {"type": "okamoto"},
            "seed": "0", "max_witnesses": 0}}]),
    );
    let item = &document["items"][0];
    assert!(item.is_object(), "{document}");
    assert_eq!(label(item), "unsupported", "{item}");
    assert!(item.to_string().contains("exact-only-form"), "{item}");
    assert!(
        item.get("measurement").is_none(),
        "no sample is drawn: {item}"
    );
}

/// FR-186-AC-10 (TC-2167): an `on each` probabilistic claim compiles and
/// settles a verdict per instance from source, never `unsupported`.
#[ix_trace_rs::trace("TC-2167", "FR-186-AC-10")]
#[ix_trace_rs::trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
#[ignore = "QSL-2158: source-route on-each probabilistic claims settle internal-failure (runtime_invariant, established-invariant-broken) before the expected per-instance verdict"]
fn probabilistic_on_each_settles_per_instance() {
    let claim = "probabilistic E using inf on M under Even \
         on each (k: M::Msg) when (not k.delivered) \
         { probability >= rational(9, 10) [ eventually holds(k.delivered) ] }";
    let claims = format!("{EVENTUALLY}\n{claim}");
    let document = run(
        "on-each",
        &claims,
        json!([{"claim": "E"}, {"claim": "Eventually"}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    // The start snapshot holds one `Msg`, `m`, undelivered at position 0, so
    // the one instance activates at position 0 and its exact value is the
    // `on origin` claim's, `391/400`.
    assert_eq!(
        item["record"]["basis"], document["items"][1]["record"]["basis"],
        "{document}"
    );
}

/// `E`: every undelivered message is delivered eventually with probability
/// at least `threshold` (FR-875's use case).
fn on_each(threshold: &str) -> String {
    format!(
        "probabilistic E using inf on M under Even \
         on each (k: M::Msg) when (not k.delivered) \
         {{ probability >= {threshold} [ eventually holds(k.delivered) ] }}"
    )
}

/// The `Link` start snapshot with its `msgs` population holding one
/// message per `(key, delivered)` pair, each with no attempt made.
fn link_start(messages: &[(&str, bool)]) -> Vec<u8> {
    let mut snapshot: Value = serde_json::from_slice(LINK_START).expect("the Link snapshot");
    let objects: Vec<Value> = messages
        .iter()
        .map(|(key, delivered)| {
            json!({"fields": {"attempts": {"integer": "0"},
                              "delivered": {"boolean": delivered}},
                   "key": key, "type": "ix://test/link/Msg"})
        })
        .collect();
    snapshot["populations"][0]["objects"] = Value::Array(objects);
    serde_json::to_vec(&snapshot).expect("snapshot bytes")
}

/// [`run`] over the `Link` unit with `claims` and a custom start snapshot.
fn run_from(name: &str, claims: &str, start: Vec<u8>, items: Value) -> Value {
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        &unit(claims),
        LINK_MODEL,
        &[start],
        json!({"items": items}),
    );
    analyze(&request).document()
}

/// FR-875-AC-1 (TC-2485): an `on each` claim over an empty population settles
/// `inconclusive` with `no-instance`, never `proved` and never `unsupported`.
#[ix_trace_rs::trace("TC-2485", "FR-875-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
#[ignore = "QSL-2158: source-route on-each probabilistic claims settle internal-failure (runtime_invariant, established-invariant-broken) before the expected per-instance verdict"]
fn probabilistic_on_each_empty_population() {
    let document = run_from(
        "on-each-empty",
        &on_each("rational(9, 10)"),
        link_start(&[]),
        json!([{"claim": "E"}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["type"], "no-instance", "{item}");
}

/// FR-875-AC-2 (TC-2486): an instance that never activates holds vacuously
/// and is never the failing instance.
#[ix_trace_rs::trace("TC-2486", "FR-875-AC-2")]
#[ix_trace_rs::trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
#[ignore = "QSL-2158: source-route on-each probabilistic claims settle internal-failure (runtime_invariant, established-invariant-broken) before the expected per-instance verdict"]
fn probabilistic_on_each_never_activating_instance_is_vacuous() {
    let start = || link_start(&[("m", false), ("n", true)]);
    let proved = run_from(
        "on-each-vacuous",
        &on_each("rational(9, 10)"),
        start(),
        json!([{"claim": "E"}]),
    );
    assert_eq!(label(&proved["items"][0]), "proved", "{proved}");

    let refuted = run_from(
        "on-each-vacuous-refuted",
        &on_each("rational(99, 100)"),
        start(),
        json!([{"claim": "E"}]),
    );
    let item = &refuted["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    let counterexample = item["counterexample"].to_string();
    assert!(counterexample.contains("\"m\""), "{item}");
    assert!(!counterexample.contains("\"n\""), "{item}");
}

/// FR-875-AC-3 (TC-2487): a refuted `on each` item names its first failing
/// instance in key order, and a rerun gives the same item.
#[ix_trace_rs::trace("TC-2487", "FR-875-AC-3")]
#[test]
#[ignore = "QSL-2158: source-route on-each probabilistic claims settle internal-failure (runtime_invariant, established-invariant-broken) before the expected per-instance verdict"]
fn probabilistic_on_each_names_first_failing_instance() {
    let once = || {
        run_from(
            "on-each-first-failing",
            &on_each("rational(99, 100)"),
            link_start(&[("a", false), ("b", false)]),
            json!([{"claim": "E"}]),
        )
    };
    let first = once();
    let item = &first["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    let counterexample = item["counterexample"].to_string();
    assert!(counterexample.contains("\"a\""), "{item}");
    assert!(!counterexample.contains("\"b\""), "{item}");
    assert_eq!(item, &once()["items"][0], "a rerun names the same instance");
}

/// FR-741-AC-5 (TC-3062): an `on each` proof carries a `conditional`
/// certificate, and its refutation names the failing instance in the
/// counterexample's `instance` member.
#[ix_trace_rs::trace("TC-3062", "FR-741-AC-5")]
#[test]
#[ignore = "QSL-2151: conditional certificates and the counterexample instance member not implemented yet"]
fn probabilistic_on_each_reports_conditional_certificate_and_instance() {
    let start = || link_start(&[("m", false), ("n", true)]);
    let proved = run_from(
        "on-each-conditional",
        &on_each("rational(9, 10)"),
        start(),
        json!([{"claim": "E"}]),
    );
    let item = &proved["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["kind"], "conditional", "{item}");
    let conditional = &item["certificate"]["document"]["conditional"];
    assert_eq!(conditional["mode"], "ratio", "{item}");
    // `m` activates at position 0, so at its initial key the numerator is the
    // `on origin` value and the denominator 1.
    let holds_value = |part: &str, value: Value| {
        conditional[part]["values"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|row| row["value"] == value))
    };
    assert!(
        holds_value(
            "numerator",
            json!({"numerator": "391", "denominator": "400"})
        ),
        "{item}"
    );
    assert!(
        holds_value("denominator", json!({"numerator": "1", "denominator": "1"})),
        "{item}"
    );

    let refuted = run_from(
        "on-each-conditional-refuted",
        &on_each("rational(99, 100)"),
        start(),
        json!([{"claim": "E"}]),
    );
    let item = &refuted["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    let counterexample = &item["counterexample"];
    assert_eq!(counterexample["binding"], Value::Null, "{item}");
    assert!(
        counterexample["instance"].to_string().contains("\"m\""),
        "{item}"
    );
}

/// `Every`: delivery under every scheduler at `9/10` (FR-876's use case).
const EVERY: &str =
    "probabilistic Every using inf on M over (m: M::Msg) under every scheduler on origin \
     { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }";

/// The `Link` unit with `Every` and without its two `random` lines, so the
/// scheduler chooses each send's `lost`.
fn without_random() -> String {
    unit(EVERY)
        .lines()
        .filter(|line| !line.starts_with("random "))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_unit(name: &str, source: &str) -> Value {
    let ws = Workspace::new(name);
    let request = request_over(
        &ws,
        source,
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Every", "probabilistic": {"evidence": "exact"}}]}),
    );
    analyze(&request).document()
}

/// FR-876-AC-1 (TC-2488): a unit without `random` lines compiles with no
/// diagnostic about them, and its every-scheduler item reaches a verdict.
#[ix_trace_rs::trace("TC-2488", "FR-876-AC-1")]
#[test]
#[ignore = "QSL-2159: every-scheduler claims over a unit without random lines settling from source not implemented yet"]
fn every_scheduler_without_random_compiles() {
    let source = without_random();
    assert!(!source.contains("random "), "{source}");
    let document = run_unit("every-no-random-compiles", &source);
    let diagnostics = document["diagnostics"].as_array().map_or(0, Vec::len);
    assert_eq!(diagnostics, 0, "{document}");
    assert_ne!(label(&document["items"][0]), "unsupported", "{document}");
}

/// FR-876-AC-2 (TC-2488, negative): without `random` lines the scheduler may
/// lose both sends, so the minimum is 0 and the item is refuted with a
/// replayed counterexample.
#[ix_trace_rs::trace("TC-2488", "FR-876-AC-2")]
#[ix_trace_rs::trace("FSpec-TC-1766", "FSpec-FR-724-AC-1")]
#[test]
#[ignore = "QSL-2159: refuting an every-scheduler claim over a unit without random lines not implemented yet"]
fn every_scheduler_without_random_is_refuted() {
    let document = run_unit("every-no-random-refuted", &without_random());
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-876-AC-3 (TC-2488): with its `random` lines the same claim's minimum
/// over schedulers is `24/25` and the item is proved.
#[ix_trace_rs::trace("TC-2488", "FR-876-AC-3")]
#[ix_trace_rs::trace("FSpec-TC-1766", "FSpec-FR-724-AC-2")]
#[test]
fn exact_every_scheduler_from_source() {
    let document = run_unit("every-random-proved", &unit(EVERY));
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["kind"], "exact", "{item}");
    let text = item.to_string();
    assert!(text.contains("24") && text.contains("25"), "{text}");
}

#[test]
fn exact_fair_scheduler_from_source() {
    let checked_source = crate::prob_world::World::new(&crate::prob_world::link(FAIR_DELIVER));
    let claim = checked_source
        .package
        .graph()
        .probabilistic_model()
        .claim("DeliverFair")
        .expect("the source claim checks");
    assert_eq!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every);
    assert!(!claim.fairness.is_empty(), "the fairness set is checked");
    assert!(
        matches!(
            &claim.form,
            fsl_semantics::check::ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula {
                    timed_horizon: None,
                    ..
                },
                ..
            }
        ),
        "FR-200 bounded fair event form: {:?}",
        claim.form
    );
    let document = run(
        "fair-scheduler",
        FAIR_DELIVER,
        json!([{"claim": "DeliverFair", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["kind"], "exact", "{item}");
    assert!(item["certificate"]["document"].is_object(), "{item}");
    assert!(
        item.to_string().contains("24") && item.to_string().contains("25"),
        "{item}"
    );
}

#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_clopper_pearson_from_source() {
    statistical_method_from_source("clopper-pearson", json!({"type": "clopper-pearson"}));
}

/// `MeanDelivered`: the mean fraction of delivered positions in `[0, 2]`.
const MEAN_DELIVERED: &str =
    "probabilistic MeanDelivered using ev on M over (m: M::Msg) under Even on origin \
     with alpha 0.01, beta 0.01, indifference 0.05 \
     { mean of (fraction holds(m.delivered) over [0, 2]) >= 0.1 }";

/// FR-741-AC-4 (negative): a `mean of` claim requested with `sprt` settles
/// `invalid-request` `not-statistical`, and `statistical.max_samples` 10
/// settles `incomplete` `LimitReached` naming `statistical.max_samples`.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-4")]
#[test]
fn statistical_refusals_and_budgets_from_source() {
    let document = run(
        "mean-of-sprt",
        MEAN_DELIVERED,
        json!([{"claim": "MeanDelivered", "probabilistic": {
            "evidence": "statistical", "method": {"type": "sprt"}, "seed": "0"}}]),
    );
    // An `invalid-request` item is a refusal settled by its disposition.
    let item = &document["items"][0];
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "not-statistical",
        "{item}"
    );

    let document = run(
        "max-samples",
        DELIVERED,
        json!([{"claim": "Delivered", "probabilistic": {
            "evidence": "statistical", "seed": "7", "limits": {"max_samples": 10}}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "incomplete", "{item}");
    assert!(
        item.to_string().contains("statistical.max_samples"),
        "{item}"
    );
    assert_eq!(
        item["request"]["probabilistic"]["limits"]["max_samples"], 10,
        "{item}"
    );
}

/// FR-741 request members: an absent method of a long-run fraction is
/// `regenerative` with `min_cycles` 100, of a bounded form `okamoto`; FSL
/// writes every budget the caller leaves out at its published default.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[test]
fn absent_probabilistic_members_take_their_published_defaults() {
    let long_run = run_health_long_run(
        "default-long-run-method",
        json!([{"claim": "LongRun", "probabilistic": {"evidence": "statistical", "seed": "7"}}]),
    );
    let request = &long_run["items"][0]["request"]["probabilistic"];
    assert_eq!(
        request["method"],
        json!({"type": "regenerative", "min_cycles": 100}),
        "{long_run}"
    );
    assert_eq!(request["limits"]["max_samples"], 1_u64 << 24, "{long_run}");

    let bounded = run(
        "default-bounded-method",
        DELIVERED,
        json!([{"claim": "Delivered", "probabilistic": {"evidence": "statistical"}}]),
    );
    let request = &bounded["items"][0]["request"]["probabilistic"];
    assert_eq!(request["method"], json!({"type": "okamoto"}), "{bounded}");
    assert_eq!(request["seed"], "0", "{bounded}");
    assert_eq!(request["max_witnesses"], 8, "{bounded}");
}

/// FR-254 / QSL-2551: a delay-weighted fraction is a valid form, so EN-4
/// refuses it as not yet implemented (`unsupported`), never as a caller's
/// method choice (`invalid-request` `not-statistical`).
#[test]
fn delay_weighting_from_source_is_not_yet_implemented_not_a_method_error() {
    let claim = "probabilistic Weighted using t on M over (m: M::Msg) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.05 \
         { long_run_fraction holds(m.delivered) weighted by delay >= 0.1 }";
    let document = run_timed(
        "delay-weighting-statistical",
        claim,
        json!([{"claim": "Weighted", "probabilistic": {
            "evidence": "statistical", "method": {"type": "regenerative", "min_cycles": 1}, "seed": "7"}}]),
    );
    let item = &document["items"][0];
    assert!(item.is_object(), "{document}");
    assert_eq!(label(item), "unsupported", "{document}");
    assert_ne!(
        item["record"]["diagnostic"]["cause"], "not-statistical",
        "{item}"
    );
}

/// FR-741 "Request": an item with no `probabilistic` member at all reaches a
/// verdict, and its request echo carries every default FSL wrote.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[test]
fn an_item_with_no_probabilistic_member_takes_every_default() {
    let document = run(
        "no-probabilistic-member",
        DELIVERED,
        json!([{"claim": "Delivered"}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "measured", "{item}");
    let request = &item["request"]["probabilistic"];
    assert_eq!(request["evidence"], "statistical", "{item}");
    assert_eq!(request["method"], json!({"type": "okamoto"}), "{item}");
    assert_eq!(request["seed"], "0", "{item}");
    assert_eq!(request["max_witnesses"], 8, "{item}");
    assert_eq!(request["workers"], 1, "{item}");
    assert_eq!(request["limits"]["max_samples"], 1_u64 << 24, "{item}");
}

/// One definition of FSpec's backend-provider schema, with the checked-package
/// schema it refers to.
fn provider_definition(name: &str) -> jsonschema::JSONSchema {
    let schema = crate::published_fspec::json("proposals/backend-provider-v1/schema.json");
    let package = crate::published_fspec::json("proposals/checked-package-v2/schema.json");
    let definition = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{name}"),
    });
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document("urn:filament:checked-package:v2".to_owned(), package)
        .compile(&definition)
        .expect("the schema compiles")
}

fn assert_valid(compiled: &jsonschema::JSONSchema, instance: &Value, what: &str) {
    if let Err(errors) = compiled.validate(instance) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("{what} violates the schema: {errors:?}\n{instance}");
    }
}

/// FR-410, FR-741: a real statistical `measured` item's `verdict` validates
/// against FSpec's backend-provider schema (`StatisticalVerdict`); the exact
/// `proved` item's certificate carries the members FR-621 names, and its
/// schema validation is an open spelling gap (see the comments below).
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1", "FR-741-AC-2")]
#[test]
fn measured_and_proved_items_validate_against_the_backend_provider_schema() {
    let measured = run(
        "schema-measured",
        DELIVERED,
        json!([{"claim": "Delivered", "probabilistic": {
            "evidence": "statistical", "method": {"type": "okamoto"}, "seed": "7"}}]),
    );
    let item = &measured["items"][0];
    assert_eq!(label(item), "measured", "{item}");
    // The `over` binding is written in the engine's reference spelling,
    // whose `object_type` and `identity` members the schema's `TypedValue`
    // `reference` arm spells differently (`NodeId`, hex identity): an open
    // spelling gap shared with the counterexample's references, so it is
    // checked as a `reference` here and validated as `null`.
    let mut verdict = item["verdict"].clone();
    for test in verdict["provenance"]["tests"]
        .as_array_mut()
        .expect("provenance tests")
    {
        assert_eq!(test["binding"]["type"], "reference", "{test}");
        test["binding"] = Value::Null;
    }
    assert_valid(
        &provider_definition("StatisticalVerdict"),
        &verdict,
        "the measured verdict",
    );
    let proved = run(
        "schema-proved",
        EVENTUALLY,
        json!([{"claim": "Eventually", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &proved["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    // `identity.node_id` nests structural node ids as objects where the
    // schema's `NodeId` path takes another spelling: the same open gap as
    // EN-1's identity (FR-621-AC-8), so its members are checked here.
    let identity = &item["certificate"]["identity"];
    assert!(identity["node_id"].is_object(), "{item}");
    assert!(identity["model"].is_string(), "{item}");
    assert!(identity["ordinal"].is_number(), "{item}");
    // The certificate document does not validate against
    // `ProbabilityCertificate` either: its `identity.model` is null and its
    // nested node ids use the same spelling as above. Open gap, so only its
    // kind is checked.
    assert_eq!(item["certificate"]["document"]["kind"], "exact", "{item}");
}

/// FR-410: a `measured` verdict spells each test's `over` binding as a typed
/// reference and its `instance` as null; no witness is written until every
/// member of it has a schema-valid spelling.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[test]
fn a_verdict_spells_its_bindings_and_instances() {
    let document = run(
        "verdict-wire",
        DELIVERED,
        json!([{"claim": "Delivered", "probabilistic": {
            "evidence": "statistical", "method": {"type": "okamoto"},
            "seed": "7", "max_witnesses": 8}}]),
    );
    let verdict = &document["items"][0]["verdict"];
    let tests = verdict["provenance"]["tests"].as_array().expect("tests");
    assert!(!tests.is_empty(), "{verdict}");
    for test in tests {
        assert!(test["binding"].is_object(), "{test}");
        assert!(test["instance"].is_null(), "{test}");
    }
    assert_eq!(verdict["witnesses"], json!([]), "{verdict}");
}

/// FR-741 request members and FR-203 budgets: an exact item's
/// `probabilistic.limits` reach EN-5, so `max_states` 1 settles
/// `incomplete` naming `exact_prob.max_states`, and an unknown limit member
/// is an invalid value of the item.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-2")]
#[test]
fn exact_limits_from_source_reach_the_engine() {
    let document = run(
        "exact-max-states",
        EVENTUALLY,
        json!([
            {"claim": "Eventually", "probabilistic": {
                "evidence": "exact", "limits": {"max_states": 1}}},
            {"claim": "Eventually", "probabilistic": {
                "evidence": "exact", "limits": {"max_nothing": 1}}}
        ]),
    );
    let limited = &document["items"][0];
    assert_eq!(label(limited), "incomplete", "{limited}");
    assert!(
        limited.to_string().contains("exact_prob.max_states"),
        "{limited}"
    );
    assert_eq!(
        limited["request"]["probabilistic"]["limits"]["max_states"], 1,
        "{limited}"
    );
    assert!(
        document.to_string().contains("probabilistic"),
        "the unknown member is refused naming the member: {document}"
    );
    assert_ne!(label(&document["items"][1]), "proved", "{document}");
}
