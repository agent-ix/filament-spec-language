// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2066 (FR-741): probability items run from source through
//! `fsl analyze` on a `native-analyze/1` request, over ADR-028 §15.4's
//! `Link` model (random losses on two sends, a `cost` reward and the
//! workload `Even`), which S4 emits as a checked package (FR-740).

use base64::Engine;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::prob_world::{self, World};
use filament_exact::Cancel;
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_analyze::exact_probabilistic::{
    build_product, decide_reachability, ExactProbLimits, ExtRational, ProbBuild, UnboundedObjective,
};
use fsl_eval::simulation::model::ModelSystem;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    replay_probabilistic_witness, ActivationPhase, ExactOrBounds, ModelCheckLimits, OnEachOutcome,
    ProbabilityProductKey, ProofBasis, TerminalValue,
};

use crate::analyze_cli_support::{
    analyze, assert_identity_names_claim, assert_no_nested_node_ids, label, request_over, Workspace,
};

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

/// Both native engines retain the actual zero/two/four-post refusal, its
/// complete interleaved argument vector and the first two real model keys.
#[trace("TC-622", "FR-187-AC-3", "TC-2066", "FR-741-AC-1", "FR-741-AC-3")]
#[test]
fn native_not_markov_preserves_actual_zero_two_and_four_post_payloads() {
    use fsl_eval::simulation::model::Unsteppable;
    use fsl_eval::simulation::TransitionSystem;
    use fsl_replay::outcome::{state_key_wire, task_source_transition_wire};

    const CLAIM: &str =
        "probabilistic Broken using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { probability >= 0.5 [ always[0,1] holds(n.healthy) ] }";
    for (post, count) in [
        ("false", 0_u64),
        ("self.healthy = not fault", 2),
        ("true", 4),
    ] {
        let mut model = prob_world::health(post, "false: 1", true, CLAIM);
        model.operations[0].parameters = vec![
            ("left", "ix://filament/native/Boolean".to_owned()),
            ("fault", "ix://filament/native/Boolean".to_owned()),
            ("right", "ix://filament/native/Boolean".to_owned()),
        ];
        model
            .body
            .push_str("\npre Fixed using v on M::Node::tick { left and not right }\n");
        let world = World::new(&model);
        let system = ModelSystem::new(world.subject().simulation).expect("the source model admits");
        let [initial] = <[_; 1]>::try_from(system.initial()).expect("one initial state");
        let workload = world
            .package
            .graph()
            .probabilistic_model()
            .workload("Steady")
            .unwrap();
        let Err(Unsteppable::NotMarkov(actual)) = system
            .weighted_steps(&initial, workload)
            .expect("the source model expands")
        else {
            panic!("the source fixture has {count} post-states per draw");
        };
        assert_eq!(actual.post_states, count);
        let expected_transition = task_source_transition_wire(&actual.drawn_transition).unwrap();
        assert_eq!(
            expected_transition["arguments"],
            json!([
                {"type": "boolean", "value": true},
                {"type": "boolean", "value": false},
                {"type": "boolean", "value": false},
            ])
        );
        assert!(expected_transition["operation"]
            .as_str()
            .unwrap()
            .contains("tick"));
        assert_eq!(expected_transition["receiver"]["type"], "reference");
        let expected_posts = actual
            .post_state_examples
            .iter()
            .map(|key| state_key_wire(key).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(expected_posts.len(), usize::try_from(count.min(2)).unwrap());
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.model.complete/v1\";\n\
             profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
             model M = \"{}\";\n{}\n",
            model.package, model.body,
        );
        let snapshot = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "start"},
            "observation": "current",
            "model": {"identity": model.package},
            "anchor": {"kind": "initialization", "name": "start"},
            "populations": [{"population": "ix://test/health/nodes", "complete": true,
                "objects": [{"key": "n", "type": "ix://test/health/Node",
                    "fields": model.initial.clone()}]}]
        })
        .to_string()
        .into_bytes();
        let document = model.document(false);
        for evidence in ["statistical", "exact"] {
            let ws = Workspace::new(&format!("not-markov-{evidence}-{count}"));
            let probabilistic = if evidence == "statistical" {
                json!({"evidence": evidence, "seed": "7"})
            } else {
                json!({"evidence": evidence})
            };
            let request = request_over(
                &ws,
                &source,
                &document,
                std::slice::from_ref(&snapshot),
                json!({"items": [{"claim": "Broken", "probabilistic": probabilistic}]}),
            );
            let result = analyze(&request).document();
            let item = &result["items"][0];
            assert_eq!(label(item), "unsupported", "{result}");
            let cause = &item["record"]["cause"];
            assert_eq!(cause["kind"], "not-markov", "{item}");
            assert_eq!(cause["state"], state_key_wire(&actual.state).unwrap());
            assert_eq!(cause["transition"], expected_transition);
            assert_eq!(cause["post_state_count"], count);
            assert_eq!(cause["post_states"], json!(expected_posts));
            assert_eq!(
                cause
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "kind",
                    "post_state_count",
                    "post_states",
                    "state",
                    "transition",
                ])
            );
            assert!(item.get("verdict").is_none(), "{item}");
            assert!(item["record"].get("basis").is_none(), "{item}");
        }
    }
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
    // FSpec FR-410-AC-12: the verdict states the bound it was judged against.
    let (form, numerator, denominator) = if method == "regenerative" {
        ("long-run-fraction", "3", "10")
    } else {
        ("probability", "9", "10")
    };
    assert_eq!(
        first["verdict"]["bound"],
        json!({"type": form, "comparison": "greater-or-equal", "threshold": {
            "numerator": numerator, "denominator": denominator}}),
        "{first}"
    );
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

#[trace("TC-2066", "FR-741-AC-1", "FR-192-AC-2", "FSpec-FR-410-AC-12")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_okamoto_from_source() {
    statistical_method_from_source("okamoto", json!({"type": "okamoto"}));
}

#[trace("TC-2066", "FR-741-AC-1", "FR-192-AC-2", "FSpec-FR-410-AC-12")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
fn statistical_sprt_from_source() {
    statistical_method_from_source("sprt", json!({"type": "sprt"}));
}

#[trace("TC-2066", "FR-741-AC-1", "FR-192-AC-2", "FSpec-FR-410-AC-12")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
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
#[trace("TC-2066", "FR-741-AC-3")]
#[test]
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
#[trace("TC-2066", "FR-741-AC-2")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-2")]
#[trace("TC-2065", "FR-203-AC-8")]
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
#[trace("TC-2066", "FR-741-AC-3")]
#[trace("TC-2065", "FR-203-AC-8")]
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
#[trace("TC-2066", "FR-741-AC-4")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-3")]
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
#[trace("TC-2167", "FR-186-AC-10")]
#[trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
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
    // `on origin` claim's, `391/400`. FR-741 "Binding": the on-each entry
    // names `m` as its `instance` with a null `binding`, where the
    // `on origin` entry names `m` as its `binding` with a null `instance`.
    let entry = |item: &Value| item["record"]["basis"]["entries"][0].clone();
    let (each, origin) = (entry(item), entry(&document["items"][1]));
    assert_eq!(each["value"], origin["value"], "{document}");
    assert_eq!(each["binding"], Value::Null, "{each}");
    assert_eq!(each["instance"]["key"], "m", "{each}");
    assert_eq!(origin["binding"]["key"], "m", "{origin}");
    assert_eq!(origin["instance"], Value::Null, "{origin}");
    assert!(each["value"].to_string().contains("391"), "{each}");
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
#[trace("TC-2485", "FR-875-AC-1")]
#[trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
fn probabilistic_on_each_empty_population() {
    let document = run_from(
        "on-each-empty",
        &on_each("rational(9, 10)"),
        link_start(&[]),
        json!([{"claim": "E"}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "no-instance", "{item}");
}

/// FR-875-AC-2 (TC-2486): an instance that never activates holds vacuously
/// and is never the failing instance.
#[trace("TC-2486", "FR-875-AC-2")]
#[trace("FSpec-TC-1650", "FSpec-FR-407-AC-17")]
#[test]
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
    assert_eq!(
        item["counterexample"]["instance"]["identity"], "m",
        "{item}"
    );
}

/// FR-875-AC-3 (TC-2487): a refuted `on each` item names its first failing
/// instance in key order, and a rerun gives the same item.
#[trace("TC-2487", "FR-875-AC-3")]
#[test]
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
    assert_eq!(
        item["counterexample"]["instance"]["identity"], "a",
        "{item}"
    );
    assert_eq!(item, &once()["items"][0], "a rerun names the same instance");
}

/// FR-741-AC-5 (TC-3062): an `on each` proof carries a `conditional`
/// certificate, and its refutation names the failing instance in the
/// counterexample's `instance` member.
#[trace("TC-3062", "FR-741-AC-5")]
#[test]
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
    // `m` is the one compared instance, with a null `binding`; `n` never
    // activates, is confirmed vacuous and has no entry (FR-741 "Binding").
    let entries = item["record"]["basis"]["entries"]
        .as_array()
        .expect("entries");
    assert_eq!(entries.len(), 1, "{item}");
    assert_eq!(entries[0]["instance"]["key"], "m", "{item}");
    assert_eq!(entries[0]["binding"], Value::Null, "{item}");
    assert!(entries[0]["value"].to_string().contains("391"), "{item}");
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

/// FR-196-AC-15 (TC-3060): the checked two-object Link source emits the
/// conditional masses for m, leaves n vacuous and waiting at every reachable
/// key, and preserves the distinct activation keys.
#[trace("TC-3060", "FR-196-AC-15")]
#[test]
fn probabilistic_on_each_product_preserves_multi_initial_keys() {
    let world_with = |threshold: &str| {
        let mut model = prob_world::link("");
        model
            .body
            .push_str(&format!("\n{}\n{}", on_each(threshold), EVENTUALLY));
        // LINK_UNIT's `self` contracts and operations are receiver-scoped;
        // the symmetric fixture emits those same per-object frames.
        World::symmetric_with_initials(
            &model,
            &["m", "n"],
            &[json!({"objects": {
                "m": {"delivered": {"boolean": false}, "attempts": {"integer": "0"}},
                "n": {"delivered": {"boolean": true}, "attempts": {"integer": "0"}}
            }})],
        )
    };
    let world = world_with("rational(9, 10)");
    let cancel = Cancel::new();
    let subject = world.subject();
    let system = ModelSystem::new(subject.simulation).expect("the checked Link model admits");
    let claims = world.package.graph().probabilistic_model().claims();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("E"))
        .expect("the on-each identity is canonical");
    let limits = fsl_replay::ProbabilityDocumentLimits::for_exact(
        &ExactProbLimits::default(),
        system.meter_limits(),
    );
    let OnEachOutcome::Holds { document } = fsl_replay::on_each_outcome(
        &system,
        claims,
        &item,
        limits,
        ExactProbLimits::default().max_witness_paths,
        &cancel,
    )
    .expect("the checked on-each claim settles") else {
        panic!("the nonempty on-each proof holds");
    };
    let proof: Value = serde_json::from_slice(&document).expect("canonical conditional JSON");
    assert_eq!(proof["kind"], "conditional", "{proof}");
    let conditional = &proof["conditional"];
    assert_eq!(conditional["mode"], "ratio", "{proof}");

    let initial = fsl_eval::simulation::TransitionSystem::initial(&system)
        .into_iter()
        .next()
        .expect("initial state");
    let (model_key, _) = system.state_key(&initial).expect("initial model key");
    let model_product = ProbabilityProductKey::for_model_state(None, &model_key)
        .expect("the initial model key forms a product key");
    let fsl_semantics::check::ClaimActivation::Each { value_type, .. } =
        &world.claim("E").activation
    else {
        panic!("E is an on-each claim");
    };
    let instances = system.instances(value_type).expect("the Msg instances");
    let reference = |key: &str| {
        let instance = instances
            .iter()
            .find(|instance| instance.key == key)
            .expect("the initial instance exists");
        let filament_exact::ValueType::Reference(over) = value_type.as_ref() else {
            panic!("the activation is over a reference type");
        };
        system
            .bound_reference(over.clone(), instance)
            .expect("the instance has its checked reference")
    };
    let product_key = |instance: &str, phase| {
        model_product
            .with_activation(
                &reference(instance),
                phase,
                fsl_replay::ProbabilityDocumentIoLimits::default(),
                &cancel,
            )
            .expect("the product key is canonical")
    };
    let active_m = product_key("m", ActivationPhase::Active);
    let waiting_n = product_key("n", ActivationPhase::Waiting);
    let encoded_m = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(active_m.as_bytes());
    let encoded_n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(waiting_n.as_bytes());
    let numerator_rows = conditional["numerator"]["values"]
        .as_array()
        .expect("numerator value rows");
    let denominator_rows = conditional["denominator"]["values"]
        .as_array()
        .expect("denominator value rows");
    let exact_value = |rows: &[Value], key: &str| {
        rows.iter()
            .find(|row| row["state"] == key)
            .map(|row| row["value"].clone())
            .expect("the initial instance key has an exact value")
    };
    assert_eq!(
        exact_value(numerator_rows, &encoded_m),
        json!({"numerator": "391", "denominator": "400"}),
        "m's active initial numerator equals on-origin"
    );
    assert_eq!(
        exact_value(denominator_rows, &encoded_m),
        json!({"numerator": "1", "denominator": "1"}),
        "m's active initial denominator is one"
    );
    assert!(
        numerator_rows
            .iter()
            .chain(denominator_rows)
            .any(|row| row["state"] == encoded_n),
        "the producer emits n's initial waiting key"
    );
    for row in numerator_rows.iter().chain(denominator_rows) {
        let Some(encoded) = row["state"].as_str() else {
            panic!("an exact row has a state key: {row}");
        };
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .expect("a canonical product key is base64url");
        let key: Value = serde_json::from_slice(&bytes).expect("a canonical product key is JSON");
        if key[2][1] == "n" {
            assert_eq!(key[3], "waiting", "every reachable n key remains waiting");
        }
    }
    assert_ne!(active_m, waiting_n, "m and n initial keys differ");
    assert!(matches!(
        fsl_replay::check_probability_certificate_document(
            &system, claims, &item, &document, limits, &cancel
        ),
        Ok(fsl_replay::ExactProbabilityCertificateCheck::Accepted(_))
    ));

    let analyze_claim = |world: &World, name: &str| {
        let engines = Engines::fsl().expect("the exact engine registers");
        analyze_items(
            &engines,
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request(name, ModelCheckLimits::default()),
            )],
            &Cancel::new(),
        )
        .remove(0)
    };
    let on_each_record = analyze_claim(&world, "E");
    let on_each_terminal = on_each_record.terminal().expect("E settles");
    let TerminalValue::Proved {
        basis:
            ProofBasis::ExactValue {
                entries: on_each_entries,
                ..
            },
        ..
    } = on_each_terminal.value()
    else {
        panic!("E has an exact value basis");
    };
    assert_eq!(on_each_entries.len(), 1, "n has no conditional value");
    let entry = &on_each_entries[0];
    assert_eq!(
        entry
            .instance
            .as_ref()
            .map(|instance| instance.key.as_str()),
        Some("m")
    );
    assert!(entry.binding.is_none());
    let ExactOrBounds::Exact { value } = &entry.value else {
        panic!("m has an exact value");
    };
    let on_each_value = value.value().clone();
    assert_eq!(
        on_each_value,
        filament_exact::Rational::new(
            filament_exact::Integer::from(391_i64),
            filament_exact::Integer::from(400_i64),
        )
        .expect("nonzero denominator"),
        "m's on-each value is exactly 391/400"
    );

    let origin_world = World::new(&prob_world::link(EVENTUALLY));
    let origin_product = match build_product(
        origin_world.request("Eventually", ModelCheckLimits::default()),
        &cancel,
    ) {
        Ok(ProbBuild::Built(product)) => product,
        other => panic!("the on-origin claim builds an exact product: {other:?}"),
    };
    let origin_target = origin_product
        .labels
        .iter()
        .map(|labels| labels[0])
        .collect::<Vec<_>>();
    let origin_values = decide_reachability(
        &origin_product,
        &origin_target,
        UnboundedObjective::Max,
        &ExactProbLimits::default(),
        &cancel,
    )
    .expect("the on-origin exact reachability value computes");
    let origin_initial = origin_product.initial[0].2.index();
    assert_eq!(
        origin_values.values[origin_initial],
        ExtRational::Finite(on_each_value.clone()),
        "m's on-each value equals the on-origin product value"
    );
    assert_eq!(
        origin_values.values[origin_initial],
        ExtRational::Finite(
            filament_exact::Rational::new(
                filament_exact::Integer::from(391_i64),
                filament_exact::Integer::from(400_i64),
            )
            .expect("nonzero denominator")
        ),
        "the independently computed on-origin value is 391/400"
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
#[trace("TC-2488", "FR-876-AC-1")]
#[test]
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
#[trace("TC-2488", "FR-876-AC-2")]
#[trace("FSpec-TC-1766", "FSpec-FR-724-AC-1")]
#[test]
fn every_scheduler_without_random_is_refuted() {
    let document = run_unit("every-no-random-refuted", &without_random());
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-876-AC-2: the refutation's value is 0. The replayed witness paths are
/// the scheduler's losses and carry the whole mass, so the claim's event has
/// probability 0.
#[trace("TC-2488", "FR-876-AC-2")]
#[test]
fn the_refutation_without_random_has_value_zero() {
    let mut model = prob_world::link(EVERY);
    model.body = model
        .body
        .lines()
        .filter(|line| !line.starts_with("random "))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let world = World::new(&model);
    let engines = Engines::fsl().expect("the linked engines register");
    let record = analyze_items(
        &engines,
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Every", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    let witness = record
        .probabilistic_counterexample
        .expect("the refutation retains replayed witness evidence");
    assert_eq!(
        witness.mass(),
        &filament_exact::Rational::from_integer(1_i64.into())
    );
}

/// FR-876-AC-2 for a state-predicate form: an every-scheduler `always` claim
/// without `random` lines is refuted with a replayed counterexample too.
#[trace("TC-2488", "FR-876-AC-2")]
#[test]
fn every_scheduler_always_without_random_is_refuted() {
    let claim = "probabilistic StaysUp using inf on M over (m: M::Msg) under every scheduler \
         on origin { probability >= rational(9, 10) [ always holds(m.attempts = 0 or m.delivered) ] }";
    let source = unit(claim)
        .lines()
        .filter(|line| !line.starts_with("random "))
        .collect::<Vec<_>>()
        .join("\n");
    let ws = Workspace::new("every-always-no-random-refuted");
    let request = request_over(
        &ws,
        &source,
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "StaysUp", "probabilistic": {"evidence": "exact"}}]}),
    );
    let document = analyze(&request).document();
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-741-AC-2: §15.3's long-run item through `fsl analyze` settles with an
/// accepted certificate of kind `long-run`.
#[trace("TC-2066", "FR-741-AC-2")]
#[test]
fn a_long_run_item_settles_with_a_long_run_certificate() {
    let claim = "probabilistic LongRun using inf on M over (m: M::Msg) under Even on origin \
         { long_run_fraction holds(m.delivered) >= rational(391, 400) }";
    let ws = Workspace::new("long-run-from-source");
    let request = request_over(
        &ws,
        &unit(claim),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "LongRun", "probabilistic": {"evidence": "exact"}}]}),
    );
    let document = analyze(&request).document();
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    assert_eq!(item["certificate"]["kind"], "long-run", "{item}");
}

/// FR-876-AC-3 (TC-2488): with its `random` lines the same claim's minimum
/// over schedulers is `24/25` and the item is proved.
#[trace("TC-2488", "FR-876-AC-3")]
#[trace("FSpec-TC-1766", "FSpec-FR-724-AC-2")]
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

#[trace("TC-2066", "FR-741-AC-1", "FR-192-AC-2", "FSpec-FR-410-AC-12")]
#[trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
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
#[trace("TC-2066", "FR-741-AC-4")]
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
    // The default Okamoto count exceeds the budget, so nothing is drawn.
    assert_eq!(item["record"]["cause"]["limit"], "max_samples", "{item}");
    assert_eq!(item["record"]["cause"]["required"], "922", "{item}");
    assert_eq!(item["record"]["cause"]["value"], 10, "{item}");
    assert_eq!(
        item["request"]["probabilistic"]["limits"]["max_samples"], 10,
        "{item}"
    );
}

/// FR-741-AC-4, FR-191-AC-1: a claim whose indifference region (0.00005)
/// makes the fixed sample count about 9.2e8, far above the default
/// `max_samples`, settles `incomplete` before any sample, naming the limit,
/// the configured value and the required count, with no draw.
// FR-192-AC-9's reader clause is deferred (QSL-2944), so it is not tagged.
#[trace("TC-2066", "FR-741-AC-4", "FR-191-AC-1")]
#[test]
fn a_tiny_indifference_region_settles_incomplete_before_sampling() {
    let claim = "probabilistic Delivered using ev on M over (m: M::Msg) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.00005 \
         { probability >= 0.9 [ eventually[0,2] holds(m.delivered) ] }";
    let document = run(
        "tiny-indifference",
        claim,
        json!([{"claim": "Delivered", "probabilistic": {"evidence": "statistical", "seed": "7"}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "incomplete", "{item}");
    assert_eq!(
        item["record"]["cause"],
        json!({"kind": "limit-reached", "limit": "max_samples",
            "value": 16_777_216, "setting": "statistical.max_samples",
            "required": "921034038"}),
        "{item}"
    );
}

/// FSpec FR-331-AC-67: the fixed-N preflight `limit-reached` cause the engine
/// writes validates against the pinned `LimitReached` definition, `required`
/// the exact decimal string; the item spells the cause's tag `kind` where the
/// provider envelope spells it `type`, so only that member is renamed.
#[trace("FSpec-FR-331-AC-67")]
#[test]
fn a_written_preflight_limit_reached_cause_validates_against_the_pinned_schema() {
    let claim = "probabilistic Delivered using ev on M over (m: M::Msg) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.00005 \
         { probability >= 0.9 [ eventually[0,2] holds(m.delivered) ] }";
    let document = run(
        "preflight-schema",
        claim,
        json!([{"claim": "Delivered", "probabilistic": {"evidence": "statistical", "seed": "7"}}]),
    );
    let cause = &document["items"][0]["record"]["cause"];
    assert_eq!(cause["required"], "921034038", "{cause}");
    let mut wire = cause.clone();
    let tag = wire
        .as_object_mut()
        .and_then(|members| members.remove("kind"))
        .expect("the cause's tag");
    wire["type"] = tag;
    assert_valid(
        &provider_definition("LimitReached"),
        &wire,
        "the written preflight cause",
    );
}

/// FSpec FR-410-AC-12, FR-413: the bound the engine writes for each claim
/// form validates against the pinned `ProbabilisticBound`, a quantile
/// carrying its separate `level` beside the claim's own comparison and
/// threshold, every other form having none.
#[trace("FSpec-FR-410-AC-12")]
#[test]
fn a_written_verdict_bound_validates_against_the_pinned_schema() {
    use fsl_replay::statistical::{BoundComparison, ClaimForm, VerdictBound};
    let ratio = |numerator: i64, denominator: i64| {
        filament_exact::Rational::new(
            filament_exact::Integer::from(numerator),
            filament_exact::Integer::from(denominator),
        )
        .expect("a ratio")
    };
    let bound = |form, comparison, threshold, level| VerdictBound {
        form,
        comparison,
        threshold,
        level,
    };
    let schema = provider_definition("ProbabilisticBound");
    let quantile = fsl_replay::verdict_bound_wire(&bound(
        ClaimForm::Quantile,
        BoundComparison::GreaterOrEqual,
        ratio(3, 1),
        Some(ratio(1, 2)),
    ));
    assert_eq!(quantile["type"], "quantile", "{quantile}");
    assert_eq!(quantile["comparison"], "greater-or-equal", "{quantile}");
    assert_eq!(
        quantile["level"],
        json!({"numerator": "1", "denominator": "2"}),
        "{quantile}"
    );
    assert_eq!(
        quantile["threshold"],
        json!({"numerator": "3", "denominator": "1"}),
        "{quantile}"
    );
    assert_valid(&schema, &quantile, "the quantile bound");
    for (form, name) in [
        (ClaimForm::Probability, "probability"),
        (ClaimForm::MeanOfFraction, "mean-of-fraction"),
        (ClaimForm::LongRunFraction, "long-run-fraction"),
    ] {
        let wire = fsl_replay::verdict_bound_wire(&bound(
            form,
            BoundComparison::LessOrEqual,
            ratio(9, 10),
            None,
        ));
        assert_eq!(wire["type"], name, "{wire}");
        assert_eq!(wire["comparison"], "less-or-equal", "{wire}");
        assert!(wire.get("level").is_none(), "{wire}");
        assert_valid(&schema, &wire, name);
    }
}

/// A probability item's finite expansion default preserves the caller's
/// accounting bound for both evidence kinds, including its reported value.
#[trace("TC-2066", "FR-741-AC-6", "FR-255-AC-23", "FR-192-AC-1", "FR-203-AC-7")]
#[test]
fn native_probability_items_preserve_caller_accounting_limits() {
    for work_units in [0, 1] {
        let ws = Workspace::new(&format!("probability-accounting-{work_units}"));
        let request = request_over(
            &ws,
            &unit(&format!("{DELIVERED}\n{EVENTUALLY}")),
            LINK_MODEL,
            &[LINK_START.to_vec()],
            json!({
                "items": [
                    {"claim": "Delivered", "probabilistic": {
                        "evidence": "statistical", "workers": 4}},
                    {"claim": "Eventually", "probabilistic": {"evidence": "exact"}}
                ],
                "limits": {"accounting": {"work_units": work_units}}
            }),
        );
        let run = analyze(&request);
        assert_eq!(run.exit, 22, "work_units={work_units}");
        let document = run.document();
        let items = document["items"].as_array().expect("settled items");
        assert_eq!(items.len(), 2, "{document}");
        for (item, backend) in items.iter().zip(["en-4", "en-5"]) {
            assert_eq!(label(item), "incomplete", "{item}");
            let record = &item["record"];
            assert_eq!(record["settled_by"], "result", "{item}");
            assert_eq!(record["backend"], backend, "{item}");
            assert_eq!(record["cause"]["kind"], "limit-reached", "{item}");
            assert_eq!(record["cause"]["limit"], "work_units", "{item}");
            assert_eq!(record["cause"]["value"], work_units, "{item}");
            assert_eq!(
                record["cause"]["setting"], "accounting.work_units",
                "{item}"
            );
            assert!(record.get("truth").is_none(), "{item}");
            assert!(item.get("verdict").is_none(), "{item}");
        }
        assert_eq!(items[0]["request"]["probabilistic"]["workers"], 4);
    }
}

#[trace("TC-2066", "FR-120-AC-35", "FR-192-AC-1", "FR-203-AC-13")]
#[test]
fn native_probability_items_share_work_across_expansions_and_start_fresh_per_item() {
    use fsl_eval::simulation::TransitionSystem;
    let claims = format!("{DELIVERED}\n{EVENTUALLY}");
    let world = World::new(&prob_world::link(&claims));
    let subject = world.subject();
    let scope = filament_exact::WorkScope::new(subject.simulation.meter);
    let system =
        ModelSystem::new_with_work_scope(subject.simulation, scope.clone(), &Cancel::new())
            .unwrap();
    let start = system.initial().remove(0);
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload("Even")
        .unwrap();
    let first = system.weighted_steps(&start, workload).unwrap().unwrap();
    let first_work = scope.consumed_work();
    let failed = first
        .iter()
        .find(|step| {
            matches!(
                step.random.as_slice(),
                [filament_exact::Value::Boolean(true)]
            )
        })
        .expect("an actual lost send");
    system
        .weighted_steps(&failed.post, workload)
        .unwrap()
        .unwrap();
    let second_work = scope.consumed_work() - first_work;
    assert!(second_work > 1);
    let bound = first_work + second_work / 2;
    // The same source model's first relation fits but its next relation
    // shares the original allowance rather than obtaining another bound.
    #[cfg(feature = "test-support")]
    {
        use fsl_analyze::exact_probabilistic::{
            build_product, product_construction_counts, reset_product_construction_counts,
            ProductRefusal,
        };
        let mut request = world.request("Eventually", ModelCheckLimits::default());
        request.subject.simulation.meter.work_units = bound;
        reset_product_construction_counts();
        let result = build_product(request, &Cancel::new());
        assert!(
            matches!(
                result,
                Err(ProductRefusal::Expansion(
                    fsl_eval::simulation::ExpansionStop {
                        limit: Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(
                            filament_exact::LimitKind::WorkUnits
                        )),
                        ..
                    }
                ))
            ),
            "{result:?}"
        );
        assert!(
            product_construction_counts().relations >= 2,
            "the engine reaches another real expansion before stopping"
        );
    }
    let ws = Workspace::new("probability-cumulative-items");
    let request = request_over(
        &ws,
        &unit(&claims),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({
            "items": [
                {"claim": "Delivered", "probabilistic": {"evidence": "statistical", "workers": 1}},
                {"claim": "Eventually", "probabilistic": {"evidence": "exact"}},
                {"claim": "Delivered", "probabilistic": {"evidence": "statistical", "workers": 4}},
                {"claim": "Eventually", "probabilistic": {"evidence": "exact"}}
            ],
            "limits": {"accounting": {"work_units": bound}}
        }),
    );
    let document = analyze(&request).document();
    let items = document["items"]
        .as_array()
        .expect("four settled native items");
    assert_eq!(items.len(), 4, "{document}");
    for item in items {
        assert_eq!(label(item), "incomplete", "{item}");
        assert_eq!(item["record"]["cause"]["limit"], "work_units", "{item}");
        assert_eq!(item["record"]["cause"]["value"], bound, "{item}");
        assert_eq!(
            item["record"]["cause"]["setting"], "accounting.work_units",
            "{item}"
        );
        assert!(item["record"].get("truth").is_none(), "{item}");
        assert!(item["record"].get("decision").is_none(), "{item}");
        assert!(item.get("verdict").is_none(), "{item}");
    }
    assert_eq!(
        items[0]["record"], items[2]["record"],
        "requested workers reproduce the same serial path, with a fresh allowance per item"
    );
    assert_eq!(
        items[1]["record"], items[3]["record"],
        "each exact item starts fresh"
    );
    assert_eq!(items[0]["request"]["probabilistic"]["workers"], 1);
    assert_eq!(items[2]["request"]["probabilistic"]["workers"], 4);
}

fn charged_link_rewards(source: &str) -> String {
    source
        .replace(
            "reward cost on Msg::send_a = 1;",
            "reward cost on Msg::send_a = 1 + 0;",
        )
        .replace(
            "reward cost on Msg::send_b = 1;",
            "reward cost on Msg::send_b = 1 + 0;",
        )
}

#[trace("TC-2066", "FR-120-AC-35", "FR-192-AC-1", "FR-203-AC-13")]
#[test]
fn native_probability_reward_admission_preserves_accounting_stop() {
    use fsl_analyze::statistical::{check_statistical, StatisticalRequest};
    use fsl_foundation::selection::DefinitionRef;
    use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod, StatisticalRefusal};
    let claims = format!("{DELIVERED}\n{EVENTUALLY}");
    let mut one_model = prob_world::link(&claims);
    one_model.body = one_model
        .body
        .replace("reward cost on Msg::send_b = 1;\n", "")
        .replace(
            "reward cost on Msg::send_a = 1;",
            "reward cost on Msg::send_a = 1 + 0;",
        );
    let one = World::new(&one_model);
    let request = one.subject().simulation;
    let scope = filament_exact::WorkScope::new(request.meter);
    ModelSystem::new_with_work_scope(request, scope.clone(), &Cancel::new())
        .expect("the first reward's vectors admit");
    let prefix = scope.consumed_work();
    assert!(prefix > 0);
    let mut model = prob_world::link(&claims);
    model.body = charged_link_rewards(&model.body);
    let world = World::new(&model);
    let source = charged_link_rewards(&unit(&claims));
    for bound in [0, prefix] {
        let mut subject = world.subject();
        subject.simulation.meter.work_units = bound;
        let raw_statistical = check_statistical(
            StatisticalRequest {
                workers: 1,
                subject,
                claim: world.claim("Delivered"),
                method: StatisticalMethod::Okamoto,
                seed: 7,
                sampler: DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").unwrap(),
                limits: StatisticalLimits::default(),
                max_witnesses: 0,
                symmetry: Vec::new(),
                timed_sampling: fsl_replay::statistical::TimedSampling::default(),
            },
            &Cancel::new(),
        );
        let Err(StatisticalRefusal::PredicateRefused(
            fsl_eval::simulation::model::ClauseOutcome::Incomplete(denied),
        )) = raw_statistical
        else {
            panic!("the EN-4 existing typed incomplete route must receive the reward stop: {raw_statistical:?}")
        };
        assert_eq!(denied.limit_kind, filament_exact::LimitKind::WorkUnits);
        assert_eq!(denied.limit, bound);
        assert_eq!(denied.consumed, bound);
        let mut request = world.request("Eventually", ModelCheckLimits::default());
        request.subject.simulation.meter.work_units = bound;
        let raw = fsl_analyze::exact_probabilistic::build_product(request, &Cancel::new());
        assert!(
            matches!(
                raw,
                Err(fsl_analyze::exact_probabilistic::ProductRefusal::Expansion(
                    fsl_eval::simulation::ExpansionStop {
                        limit: Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(
                            filament_exact::LimitKind::WorkUnits
                        )),
                        ..
                    }
                ))
            ),
            "reward admission must enter the existing EN-5 scalar-stop route: {raw:?}"
        );
        let ws = Workspace::new(&format!("probability-reward-admission-{bound}"));
        let request = request_over(
            &ws,
            &source,
            LINK_MODEL,
            &[LINK_START.to_vec()],
            json!({
                "items": [
                    {"claim": "Delivered", "probabilistic": {"evidence": "statistical"}},
                    {"claim": "Eventually", "probabilistic": {"evidence": "exact"}}
                ],
                "limits": {"accounting": {"work_units": bound}}
            }),
        );
        let document = analyze(&request).document();
        let items = document["items"]
            .as_array()
            .expect("both native results settle");
        assert_eq!(items.len(), 2, "{document}");
        for (item, backend) in items.iter().zip(["en-4", "en-5"]) {
            assert_eq!(label(item), "incomplete", "{item}");
            assert_eq!(item["record"]["settled_by"], "result", "{item}");
            assert_eq!(item["record"]["backend"], backend, "{item}");
            assert_eq!(item["record"]["cause"]["kind"], "limit-reached", "{item}");
            assert_eq!(item["record"]["cause"]["limit"], "work_units", "{item}");
            assert_eq!(item["record"]["cause"]["value"], bound, "{item}");
            assert_eq!(
                item["record"]["cause"]["setting"], "accounting.work_units",
                "{item}"
            );
            assert!(item["record"].get("truth").is_none(), "{item}");
            assert!(item["record"].get("decision").is_none(), "{item}");
            assert!(item.get("verdict").is_none(), "{item}");
        }
    }
}

#[trace("TC-2066", "FR-120-AC-36", "FR-192-AC-1", "FR-203-AC-13")]
#[test]
fn native_probability_reward_admission_cancellation_keeps_its_cause() {
    use filament_exact::CancelCause;
    use fsl_analyze::analyze::statistical_item;
    use fsl_analyze::statistical::StatisticalRequest;
    use fsl_foundation::selection::DefinitionRef;
    use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod};
    use fsl_replay::{IncompleteCause, TerminalValue};
    use std::sync::{Arc, OnceLock};
    let mut model = prob_world::link(&format!("{DELIVERED}\n{EVENTUALLY}"));
    model.body = charged_link_rewards(&model.body);
    let world = World::new(&model);
    let engines = Engines::fsl().unwrap();
    for exact in [false, true] {
        let slot = Arc::new(OnceLock::<Cancel>::new());
        let weak = Arc::downgrade(&slot);
        let cancel = Cancel::observing(move || {
            weak.upgrade()
                .unwrap()
                .get()
                .unwrap()
                .cancel(CancelCause::Requested);
        });
        slot.set(cancel.clone()).unwrap();
        let item = if exact {
            exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("Eventually", ModelCheckLimits::default()),
            )
        } else {
            statistical_item(
                RequestIndex::new(0),
                StatisticalRequest {
                    workers: 1,
                    subject: world.subject(),
                    claim: world.claim("Delivered"),
                    method: StatisticalMethod::Okamoto,
                    seed: 7,
                    sampler: DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1")
                        .unwrap(),
                    limits: StatisticalLimits::default(),
                    max_witnesses: 0,
                    symmetry: Vec::new(),
                    timed_sampling: fsl_replay::statistical::TimedSampling::default(),
                },
            )
        };
        let records = analyze_items(&engines, vec![item], &cancel);
        assert_eq!(cancel.tripped(), Some(CancelCause::Requested));
        let terminal = records[0].terminal().expect("native cancellation settles");
        assert_eq!(
            terminal.value(),
            &TerminalValue::Incomplete(IncompleteCause::Cancelled {
                source: CancelCause::Requested
            })
        );
        let wire = serde_json::to_value(fsl_replay::outcome::OutcomeItem::from_terminal(terminal))
            .unwrap();
        assert_eq!(wire["result"], "incomplete", "{wire}");
        assert!(wire.get("truth").is_none(), "{wire}");
        assert!(wire.get("decision").is_none(), "{wire}");
    }
}

/// FR-741 request members: an absent method of a long-run fraction is
/// `regenerative` with `min_cycles` 100, of a bounded form `okamoto`; FSL
/// writes every budget the caller leaves out at its published default.
#[trace("TC-2066", "FR-741-AC-1")]
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

/// FR-420 "Delay-weighted cycles": the `Link` unit's behaviours all end in a
/// terminal state, so the long-run delay-weighted fraction is the mean of
/// their terminal limits.
#[test]
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-11")]
fn delay_weighting_from_source_settles_the_limit_of_a_terminating_run() {
    let claim = "probabilistic Weighted using t on M over (m: M::Msg) under Even on origin \
         with alpha 0.4, beta 0.4, indifference 0.05 \
         { long_run_fraction holds(m.delivered) weighted by delay >= 0.1 }";
    let document = run_timed(
        "delay-weighting-statistical",
        claim,
        json!([{"claim": "Weighted", "probabilistic": {
            "evidence": "statistical", "method": {"type": "regenerative", "min_cycles": 1}, "seed": "7"}}]),
    );
    let item = &document["items"][0];
    assert!(item.is_object(), "{document}");
    assert_eq!(label(item), "measured", "{document}");
    assert_eq!(item["verdict"]["decision"]["type"], "accepted", "{item}");
}

/// Both sends have delay at least 1 ms, so the entire half-ms window is
/// spent at attempts = 0. Its held and total durations are both 1/2 ms.
#[trace("FR-254-AC-3", "FSpec-FR-407-AC-19")]
#[test]
fn bounded_delay_fraction_from_source_clips_the_first_leaving_step() {
    let claim = "probabilistic Weighted using t on M over (m: M::Msg) under Even on origin \
         with alpha 0.4, beta 0.4, indifference 0.05 \
         { mean of (fraction holds(m.attempts = 0) over [0, rational(1, 2) ms] weighted by delay) >= 0.5 }\n\
         probabilistic WeightedEvent using t on M over (m: M::Msg) under Even on origin \
         with alpha 0.4, beta 0.4, indifference 0.05 \
         { probability >= 0.5 [ fraction holds(m.attempts = 0) over [0, rational(1, 2) ms] weighted by delay >= rational(1, 2) ] }";
    let document = run_timed(
        "bounded-delay-fraction",
        claim,
        json!([{"claim": "Weighted", "probabilistic": {
            "evidence": "statistical", "method": {"type": "okamoto"}, "seed": "7"}},
            {"claim": "WeightedEvent", "probabilistic": {
            "evidence": "statistical", "method": {"type": "sprt"}, "seed": "7"}}]),
    );
    for index in 0..2 {
        let item = &document["items"][index];
        assert_eq!(label(item), "measured", "{document}");
        assert_eq!(item["verdict"]["decision"]["type"], "accepted", "{item}");
        assert_eq!(
            item["verdict"]["estimate"]["value"],
            json!({"numerator": "1", "denominator": "1"}),
            "{item}"
        );
    }
}

/// FR-741 "Request": an item with no `probabilistic` member at all reaches a
/// verdict, and its request echo carries every default FSL wrote.
#[trace("TC-2066", "FR-741-AC-1")]
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
pub(crate) fn provider_definition(name: &str) -> jsonschema::JSONSchema {
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

pub(crate) fn assert_valid(compiled: &jsonschema::JSONSchema, instance: &Value, what: &str) {
    if let Err(errors) = compiled.validate(instance) {
        let errors: Vec<_> = errors.map(|error| error.to_string()).collect();
        panic!("{what} violates the schema: {errors:?}\n{instance}");
    }
}

/// FR-410, FR-741, FR-322, FSpec FR-412: a real statistical `measured` item's
/// `verdict` validates against FSpec's backend-provider schema
/// (`StatisticalVerdict`), and the exact `proved` item's certificate identity
/// and document validate as `CertificateIdentity` and
/// `ProbabilityCertificate`: the identity's `node_id` is the structural id
/// with the owning model named, and every other node id is written in the
/// document-held index form.
#[trace("TC-2066", "FR-741-AC-1", "FR-741-AC-2")]
#[trace("TC-3154", "FR-621-AC-16")]
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
    let ws = Workspace::new("schema-proved");
    let request = request_over(
        &ws,
        &unit(EVENTUALLY),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Eventually", "probabilistic": {"evidence": "exact"}}]}),
    );
    let proved = analyze(&request).document();
    let item = &proved["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    let certificate = &item["certificate"];
    assert_valid(
        &provider_definition("CertificateIdentity"),
        &certificate["identity"],
        "the proved item's certificate identity",
    );
    assert_valid(
        &provider_definition("ProbabilityCertificate"),
        &certificate["document"],
        "the proved item's certificate document",
    );
    assert_eq!(
        certificate["document"]["identity"], certificate["identity"],
        "{item}"
    );
    assert_valid(
        &provider_definition("Locus"),
        &item["request"]["claim"],
        "the proved item's claim locus",
    );
    assert_identity_names_claim(&ws, item);
}

/// FR-741-AC-2 and FR-1042-AC-6: a `ValueBounds` item writes its `certificate`
/// as the claim side's `lower` document and a `counterpart` beside it at item
/// level with exactly `{kind, document, accepted}`, the `upper` document in
/// FSpec's `ProbabilityCertificate` spelling; the record carries no
/// `counterpart` member of its own.
#[trace("TC-2066", "FR-741-AC-2")]
#[trace("TC-2066", "FR-1042-AC-6")]
#[test]
fn a_value_bounds_item_writes_its_counterpart_at_item_level() {
    const INTERVAL: &str =
        "probabilistic Interval using ev on M over (m: M::Msg) under Even on origin \
         { probability >= 0.9 [ eventually[0,2] holds(m.delivered) ] }";
    let document = run(
        "value-bounds-item",
        INTERVAL,
        json!([{"claim": "Interval", "probabilistic": {
            "evidence": "exact", "limits": {"max_rational_bits": 4}}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["certificate"]["kind"], "lower", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    let counterpart = item["counterpart"].as_object().expect("a counterpart");
    let mut members: Vec<&str> = counterpart.keys().map(String::as_str).collect();
    members.sort_unstable();
    assert_eq!(members, ["accepted", "document", "kind"], "{item}");
    assert_eq!(counterpart["kind"], "upper", "{item}");
    assert_eq!(counterpart["accepted"], true, "{item}");
    assert_eq!(counterpart["document"]["kind"], "upper", "{item}");
    assert_eq!(
        counterpart["document"]["identity"], item["certificate"]["identity"],
        "{item}"
    );
    assert_valid(
        &provider_definition("ProbabilityCertificate"),
        &counterpart["document"],
        "the counterpart document",
    );
    assert!(item["record"].get("counterpart").is_none(), "{item}");
}

/// FR-322, FR-331: a real `refuted` item's claim locus is a schema-valid
/// `Locus` whose node id holds index segments, never a nested node id.
#[trace("TC-3154", "FR-621-AC-16")]
#[test]
fn a_refuted_item_names_its_claim_in_the_document_held_form() {
    let refuted = run(
        "schema-refuted",
        TOO_LIKELY,
        json!([{"claim": "TooLikely", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &refuted["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_valid(
        &provider_definition("Locus"),
        &item["request"]["claim"],
        "the refuted item's claim locus",
    );
    // The counterexample's typed `reference` values spell `object_type` as an
    // object where the schema's `TypedValue` spells a `NodeId`: the open
    // spelling gap `measured_and_proved_items_validate_against_the_backend_provider_schema`
    // records for a verdict's `binding`. Its node paths hold no nested id.
    assert_no_nested_node_ids(&item["counterexample"]);
    assert_no_nested_node_ids(&refuted);
    let path = item["request"]["claim"]["node"]["path"]
        .as_array()
        .expect("a node path");
    assert!(
        path.iter().any(|segment| segment.get("node").is_some()),
        "{item}"
    );
    assert!(
        path.iter().all(|segment| segment.get("package").is_none()),
        "{item}"
    );
}

/// FR-410: a `measured` verdict spells each test's `over` binding as a typed
/// reference and its `instance` as null; no witness is written until every
/// member of it has a schema-valid spelling.
#[trace("TC-2066", "FR-741-AC-1")]
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
#[trace("TC-2066", "FR-741-AC-2")]
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

/// FR-741 Budgets over FR-875: a state limit reached while an `on each` item
/// builds its phase graph settles `incomplete` naming `exact_prob.max_states`
/// like every other exact item, never `failed`.
#[test]
fn probabilistic_on_each_state_limit_settles_incomplete_naming_the_limit() {
    let document = run_from(
        "on-each-limit",
        &on_each("rational(9, 10)"),
        link_start(&[("m", false)]),
        json!([{"claim": "E", "probabilistic": {
            "evidence": "exact", "limits": {"max_states": 1}}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "incomplete", "{item}");
    assert!(item.to_string().contains("exact_prob.max_states"), "{item}");
}

/// FR-201-AC-80, FR-875 "Unbuilt on-each settlements (current)": an `on each`
/// claim with authored fairness, and one under every scheduler, are outside
/// the closed profile. Each settles `unsupported` with the
/// `not-yet-implemented` diagnostic and no `cause`, and writes no certificate.
#[trace("TC-3209", "FR-201-AC-80")]
#[test]
fn probabilistic_on_each_out_of_profile_form_is_unsupported_with_no_cause() {
    for (name, scheduler) in [
        ("fairness", "every scheduler fair weak Msg::send_a"),
        ("every-scheduler", "every scheduler"),
    ] {
        let claim = format!(
            "probabilistic E using inf on M under {scheduler} \
             on each (k: M::Msg) when (not k.delivered) \
             {{ probability >= rational(1, 2) [ eventually holds(k.delivered) ] }}"
        );
        let document = run_from(
            &format!("on-each-{name}"),
            &claim,
            link_start(&[("m", false)]),
            json!([{"claim": "E"}]),
        );
        let item = &document["items"][0];
        assert!(item.is_object(), "{name}: {document}");
        assert_eq!(label(item), "unsupported", "{name}: {item}");
        assert!(
            item.to_string().contains("not-yet-implemented"),
            "{name}: {item}"
        );
        assert!(item["record"].get("cause").is_none(), "{name}: {item}");
        assert!(item.get("certificate").is_none(), "{name}: {item}");
    }
}

/// `E`'s activation over `body`, as `name`, written `using profile` under
/// `scheduler`.
fn on_each_form(name: &str, profile: &str, scheduler: &str, body: &str) -> String {
    format!(
        "probabilistic {name} using {profile} on M under {scheduler} \
         on each (k: M::Msg) when (not k.delivered) {{ {body} }}"
    )
}

/// The `Link` unit whose sends no longer count attempts: a lost send
/// returns the message to its own undelivered state, so every instance's
/// phase graph is cyclic.
fn cyclic_unit(claims: &str) -> String {
    unit(claims)
        .replace(
            "self.attempts - pre(self.attempts) = 1 and self.delivered = (not lost)",
            "self.attempts = pre(self.attempts) and self.delivered = (not lost)",
        )
        .replace(
            "terminal Stop using v on M when { self.delivered or self.attempts = 2 };",
            "",
        )
}

/// FR-875's profile boundary: the item settles `unsupported` with
/// `not-yet-implemented`, no `cause`, value, measurement or certificate, and
/// does not name the activation or `no-instance` as its reason.
fn settles_at_the_profile_boundary(case: &str, item: &Value) {
    assert!(item.is_object(), "{case}: {item}");
    assert_eq!(label(item), "unsupported", "{case}: {item}");
    let text = item.to_string();
    assert!(text.contains("not-yet-implemented"), "{case}: {item}");
    assert!(!text.contains("no-instance"), "{case}: {item}");
    assert!(item["record"].get("cause").is_none(), "{case}: {item}");
    assert!(item["record"].get("basis").is_none(), "{case}: {item}");
    assert!(item.get("certificate").is_none(), "{case}: {item}");
    assert!(item.get("measurement").is_none(), "{case}: {item}");
}

/// Analyze `claim` under `source`'s unit text with one undelivered message,
/// and with an empty `msgs` population, requesting `request`.
fn on_each_both(case: &str, source: &str, request: Value) -> (Value, Value) {
    let over = |start: Vec<u8>, suffix: &str| {
        let ws = Workspace::new(&format!("on-each-{case}-{suffix}"));
        let path = request_over(
            &ws,
            source,
            LINK_MODEL,
            &[start],
            json!({"items": [request.clone()]}),
        );
        analyze(&path).document()["items"][0].clone()
    };
    (
        over(link_start(&[("m", false)]), "one"),
        over(link_start(&[]), "none"),
    )
}

/// The item settles `inconclusive` with cause `no-instance`.
fn settles_no_instance(case: &str, item: &Value) {
    assert_eq!(label(item), "inconclusive", "{case}: {item}");
    assert_eq!(
        item["record"]["cause"]["kind"], "no-instance",
        "{case}: {item}"
    );
}

/// [`on_each_form`] over one undelivered message, requested by `request`.
fn on_one_message(case: &str, source: &str, request: Value) -> Value {
    let ws = Workspace::new(&format!("on-each-{case}-one"));
    let path = request_over(
        &ws,
        source,
        LINK_MODEL,
        &[link_start(&[("m", false)])],
        json!({"items": [request]}),
    );
    analyze(&path).document()["items"][0].clone()
}

/// TC-3211: an on-each form outside the `on-each-reachability/v1` profile
/// settles `unsupported` at the profile boundary over a nonempty instance
/// set.
fn assert_profile_boundary(case: &str, source: &str, request: Value) {
    settles_at_the_profile_boundary(case, &on_one_message(case, source, request));
}

/// TC-3211 step 1: the admitted exact workload profile is the control. The
/// one instance `m` activates at position 0, so its value is the `on origin`
/// value `391/400`, carried by a `ratio` conditional certificate.
#[trace("TC-3211", "FR-875-AC-4")]
#[test]
fn on_each_profile_control_is_proved_with_its_ratio_certificate() {
    let source = unit(&on_each_form(
        "E",
        "inf",
        "Even",
        "probability >= rational(9, 10) [ eventually holds(k.delivered) ]",
    ));
    let (one, none) = on_each_both("control", &source, json!({"claim": "E"}));
    assert_eq!(label(&one), "proved", "{one}");
    assert_eq!(one["certificate"]["kind"], "conditional", "{one}");
    assert_eq!(
        one["certificate"]["document"]["conditional"]["mode"], "ratio",
        "{one}"
    );
    assert_eq!(
        one["record"]["basis"]["entries"][0]["value"],
        json!({"type": "exact", "value": {"numerator": "391", "denominator": "400"}}),
        "{one}"
    );
    settles_no_instance("control", &none);
}

/// FR-875 "Instances": an empty instance set settles `no-instance` before
/// the profile is admitted, so each out-of-profile form of TC-3211 over an
/// empty `msgs` population is `no-instance`, not `unsupported`. No
/// acceptance criterion states this precedence yet; the test is traced to
/// the requirement.
#[trace("FR-875")]
#[test]
fn on_each_empty_population_settles_no_instance_before_the_profile_boundary() {
    let exact = json!({"claim": "E", "probabilistic": {"evidence": "exact"}});
    let statistical = json!({"claim": "E", "probabilistic": {
        "evidence": "statistical", "method": {"type": "okamoto"},
        "seed": "0", "max_witnesses": 0}});
    let eventually = "probability >= rational(1, 2) [ eventually holds(k.delivered) ]";
    for (case, source, request) in [
        (
            "every-scheduler",
            unit(&on_each_form("E", "inf", "every scheduler", eventually)),
            exact.clone(),
        ),
        (
            "free-delay",
            timed_unit(&on_each_form("E", "inf", "Even", eventually)),
            exact.clone(),
        ),
        (
            "statistical",
            unit(&on_each_form(
                "E",
                "ev",
                "Even",
                "probability >= 0.9 [ eventually[0,2] holds(k.delivered) ]",
            ))
            .replace(
                "when (not k.delivered)",
                "when (not k.delivered) with alpha 0.01, beta 0.01, indifference 0.05",
            ),
            statistical,
        ),
        (
            "cyclic",
            cyclic_unit(&on_each_form("E", "inf", "Even", eventually)),
            exact.clone(),
        ),
        (
            "quantile",
            unit(&on_each_form(
                "E",
                "ev",
                "Even",
                "quantile 0.4 of (steps from holds(true) until holds(k.delivered) within 4) <= 3",
            )),
            exact.clone(),
        ),
        (
            "timed",
            timed_unit(&on_each_form(
                "E",
                "t",
                "Even",
                "probability >= rational(1, 2) [ eventually[0 ms, 3 ms] holds(k.delivered) ]",
            )),
            exact.clone(),
        ),
        (
            "fairness",
            unit(&on_each_form(
                "E",
                "inf",
                "every scheduler fair weak Msg::send_a",
                eventually,
            )),
            exact,
        ),
    ] {
        let (_, none) = on_each_both(case, &source, request);
        settles_no_instance(case, &none);
    }
}

/// FR-875-AC-4: every-scheduler and free-delay optima.
#[trace("TC-3211", "FR-875-AC-4")]
#[test]
fn on_each_optima_settle_at_the_profile_boundary() {
    let body = "probability >= rational(1, 2) [ eventually holds(k.delivered) ]";
    assert_profile_boundary(
        "every-scheduler",
        &unit(&on_each_form("E", "inf", "every scheduler", body)),
        json!({"claim": "E"}),
    );
    assert_profile_boundary(
        "free-delay",
        &timed_unit(&on_each_form("E", "inf", "Even", body)),
        json!({"claim": "E", "probabilistic": {"evidence": "exact"}}),
    );
}

/// FR-875-AC-5: statistical evidence, for a sampled form and for an
/// exact-only form alike.
#[trace("TC-3211", "FR-875-AC-5")]
#[test]
fn on_each_statistical_evidence_settles_at_the_profile_boundary() {
    let statistical = json!({"claim": "E", "probabilistic": {
        "evidence": "statistical", "method": {"type": "okamoto"},
        "seed": "0", "max_witnesses": 0}});
    for (case, body) in [
        (
            "statistical-bounded",
            "probability >= 0.9 [ eventually[0,2] holds(k.delivered) ]",
        ),
        (
            "statistical-unbounded",
            "probability >= 0.9 [ eventually holds(k.delivered) ]",
        ),
    ] {
        let claim = on_each_form("E", "ev", "Even", body).replace(
            "when (not k.delivered)",
            "when (not k.delivered) with alpha 0.01, beta 0.01, indifference 0.05",
        );
        let claim = if case == "statistical-unbounded" {
            claim.replace("using ev", "using inf")
        } else {
            claim
        };
        assert_profile_boundary(case, &unit(&claim), statistical.clone());
    }
}

/// FR-875-AC-6: a cyclic continuation graph.
#[trace("TC-3211", "FR-875-AC-6")]
#[test]
fn on_each_cyclic_phase_graph_settles_at_the_profile_boundary() {
    assert_profile_boundary(
        "cyclic",
        &cyclic_unit(&on_each_form(
            "E",
            "inf",
            "Even",
            "probability >= rational(1, 2) [ eventually holds(k.delivered) ]",
        )),
        json!({"claim": "E"}),
    );
}

/// FR-875-AC-7: property forms other than unbounded eventuality.
#[trace("TC-3211", "FR-875-AC-7")]
#[test]
fn on_each_other_forms_settle_at_the_profile_boundary() {
    for (case, profile, body) in [
        (
            "always",
            "ev",
            "probability >= rational(1, 2) [ always[0,2] holds(k.attempts >= 0) ]",
        ),
        (
            "until",
            "inf",
            "probability >= rational(1, 2) [ holds(k.attempts = 0) until holds(k.delivered) ]",
        ),
        (
            "bounded-eventually",
            "ev",
            "probability >= rational(1, 2) [ eventually[0,2] holds(k.delivered) ]",
        ),
        (
            "expected-reward",
            "inf",
            "expected accumulate cost until holds(k.delivered or k.attempts = 2) <= rational(2, 1)",
        ),
        (
            "long-run-fraction",
            "inf",
            "long_run_fraction holds(k.delivered) >= rational(1, 2)",
        ),
        (
            "measure",
            "ev",
            "mean of (fraction holds(k.delivered) over [0, 2]) >= rational(1, 5)",
        ),
        (
            "quantile",
            "ev",
            "quantile 0.4 of (steps from holds(true) until holds(k.delivered) within 4) <= 3",
        ),
    ] {
        assert_profile_boundary(
            case,
            &unit(&on_each_form("E", profile, "Even", body)),
            json!({"claim": "E", "probabilistic": {"evidence": "exact"}}),
        );
    }
}

/// FR-875-AC-8: a timed subject's eventuality and an authored fairness set.
#[trace("TC-3211", "FR-875-AC-8")]
#[test]
fn on_each_timed_and_fair_claims_settle_at_the_profile_boundary() {
    assert_profile_boundary(
        "timed",
        &timed_unit(&on_each_form(
            "E",
            "t",
            "Even",
            "probability >= rational(1, 2) [ eventually[0 ms, 3 ms] holds(k.delivered) ]",
        )),
        json!({"claim": "E", "probabilistic": {"evidence": "exact"}}),
    );
    assert_profile_boundary(
        "fairness",
        &unit(&on_each_form(
            "E",
            "inf",
            "every scheduler fair weak Msg::send_a",
            "probability >= rational(1, 2) [ eventually holds(k.delivered) ]",
        )),
        json!({"claim": "E"}),
    );
}

/// `E` over the `Link` model with statistical confidence parameters, for an
/// on-each item requested with statistical evidence.
const ON_EACH_SAMPLED: &str = "probabilistic Sampled using ev on M under Even \
     on each (k: M::Msg) when (not k.delivered) \
     with alpha 0.01, beta 0.01, indifference 0.05 \
     { probability >= 0.9 [ eventually[0,2] holds(k.delivered) ] }";

/// FR-875 with FR-203: a model-admission stop while an on-each item reads
/// its instances is classified like any product-admission refusal, naming
/// the exhausted resource, for statistical and exact evidence alike, and is
/// never `failed`.
#[trace("FR-875", "FR-203-AC-13")]
#[test]
fn on_each_admission_stop_is_not_an_internal_failure() {
    let claims = format!("{ON_EACH_SAMPLED}\n{}", on_each("rational(9, 10)"));
    let source = charged_link_rewards(&unit(&claims));
    let ws = Workspace::new("on-each-admission-stop");
    let request = request_over(
        &ws,
        &source,
        LINK_MODEL,
        &[link_start(&[("m", false)])],
        json!({
            "items": [
                {"claim": "Sampled", "probabilistic": {
                    "evidence": "statistical", "method": {"type": "okamoto"},
                    "seed": "0", "max_witnesses": 0}},
                {"claim": "E", "probabilistic": {"evidence": "exact"}}
            ],
            "limits": {"accounting": {"work_units": 0}}
        }),
    );
    let document = analyze(&request).document();
    let items = document["items"].as_array().expect("both items settle");
    assert_eq!(items.len(), 2, "{document}");
    for (item, backend) in items.iter().zip(["en-4", "en-5"]) {
        assert_eq!(label(item), "declined", "{item}");
        assert_eq!(item["record"]["backend"], backend, "{item}");
        assert_eq!(
            item["record"]["cause"],
            json!({"code": "resource_exhausted", "kind": "incomplete-input"}),
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["limit"], "evaluation_meter",
            "{item}"
        );
    }
}

/// FR-875 with FR-203: cancellation while a statistical on-each item reads
/// its instances keeps its cause and draws no sample.
#[trace("FR-875", "FR-203-AC-13")]
#[test]
fn on_each_statistical_cancellation_keeps_its_cause() {
    use filament_exact::CancelCause;
    use fsl_analyze::analyze::statistical_item;
    use fsl_analyze::statistical::StatisticalRequest;
    use fsl_foundation::selection::DefinitionRef;
    use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod};
    use fsl_replay::{IncompleteCause, TerminalValue};
    let world = World::new(&prob_world::link(ON_EACH_SAMPLED));
    let engines = Engines::fsl().unwrap();
    // Cancelled at the first poll, inside the item's own work.
    let slot = std::sync::Arc::new(std::sync::OnceLock::<Cancel>::new());
    let weak = std::sync::Arc::downgrade(&slot);
    let cancel = Cancel::observing(move || {
        weak.upgrade()
            .unwrap()
            .get()
            .unwrap()
            .cancel(CancelCause::Requested);
    });
    slot.set(cancel.clone()).unwrap();
    let item = statistical_item(
        RequestIndex::new(0),
        StatisticalRequest {
            workers: 1,
            subject: world.subject(),
            claim: world.claim("Sampled"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").unwrap(),
            limits: StatisticalLimits::default(),
            max_witnesses: 0,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
    );
    let records = analyze_items(&engines, vec![item], &cancel);
    let terminal = records[0].terminal().expect("cancellation settles");
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: CancelCause::Requested
        })
    );
}

/// FR-875-AC-5: a statistical on-each item over a nonempty instance set
/// settles at the profile boundary having built no product and started no
/// path-set search. EN-4 exposes no draw counter; the item carries no
/// `measurement`, so no sample is reported.
#[trace("TC-3211", "FR-875-AC-5")]
#[test]
fn on_each_statistical_evidence_builds_no_product() {
    use fsl_analyze::analyze::statistical_item;
    use fsl_analyze::exact_probabilistic::{
        product_construction_counts, reset_product_construction_counts,
    };
    use fsl_analyze::statistical::StatisticalRequest;
    use fsl_foundation::selection::DefinitionRef;
    use fsl_replay::statistical::{StatisticalLimits, StatisticalMethod};
    let world = World::new(&prob_world::link(ON_EACH_SAMPLED));
    let engines = Engines::fsl().unwrap();
    let item = statistical_item(
        RequestIndex::new(0),
        StatisticalRequest {
            workers: 1,
            subject: world.subject(),
            claim: world.claim("Sampled"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").unwrap(),
            limits: StatisticalLimits::default(),
            max_witnesses: 0,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
    );
    reset_product_construction_counts();
    let records = analyze_items(&engines, vec![item], &Cancel::new());
    assert_eq!(product_construction_counts(), Default::default());
    assert!(
        matches!(
            records[0].settlement,
            fsl_analyze::analyze::Settlement::Disposed(_)
        ),
        "unsupported at the profile boundary: {:?}",
        records[0].settlement
    );
}
