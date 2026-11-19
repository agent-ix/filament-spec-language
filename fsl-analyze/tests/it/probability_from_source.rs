// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2066 (FR-741): probability items run from source through
//! `fsl analyze` on a `native-analyze/1` request, over ADR-028 §15.4's
//! `Link` model (random losses on two sends, a `cost` reward and the
//! workload `Even`), which S4 emits as a checked package (FR-740).

use serde_json::{json, Value};

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

/// FR-741-AC-1: each statistical method settles `measured` with a
/// measurement naming its method and seed, and a rerun with four workers
/// gives the same item.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-1")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-1")]
#[test]
#[ignore = "QSL-2158: statistical probability items from source through fsl analyze not implemented yet"]
fn statistical_items_settle_measured_from_source() {
    for method in ["okamoto", "sprt", "clopper-pearson"] {
        let item = |workers: u64| {
            json!([{"claim": "Delivered", "probabilistic": {
                "evidence": "statistical", "method": {"type": method},
                "seed": "7", "max_witnesses": 8, "workers": workers}}])
        };
        let one = run(&format!("stat-{method}-1"), DELIVERED, item(1));
        let four = run(&format!("stat-{method}-4"), DELIVERED, item(4));
        let first = &one["items"][0];
        assert_eq!(label(first), "measured", "{first}");
        assert_eq!(first["measurement"]["method"], method, "{first}");
        assert_eq!(first["measurement"]["seed"], "7", "{first}");
        assert_eq!(first, &four["items"][0], "workers do not change the result");
    }
}

/// FR-741-AC-2: an exact unbounded reachability item settles `proved` with
/// the exact value `391/400` and carries its accepted `exact` certificate
/// document in the item result.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-2")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-2")]
#[ix_trace_rs::trace("TC-2065", "FR-203-AC-8")]
#[test]
#[ignore = "QSL-2158: exact probability items from source through fsl analyze not implemented yet"]
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
    let text = item.to_string();
    assert!(text.contains("391") && text.contains("400"), "{text}");
}

/// FR-741-AC-3 (negative): an exact item whose value misses its threshold
/// settles `refuted` with a counterexample whose replay reproduced it.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-3")]
#[ix_trace_rs::trace("TC-2065", "FR-203-AC-8")]
#[test]
#[ignore = "QSL-2158: refuted exact probability items from source through fsl analyze not implemented yet"]
fn an_exact_item_below_its_threshold_is_refuted_with_a_replayed_witness() {
    let document = run(
        "exact-refuted",
        TOO_LIKELY,
        json!([{"claim": "TooLikely", "probabilistic": {"evidence": "exact"}}]),
    );
    let item = &document["items"][0];
    assert_eq!(label(item), "refuted", "{item}");
    assert_eq!(
        item["counterexample"]["replay"]["reproduced"], true,
        "{item}"
    );
}

/// FR-741-AC-4 (negative): an unbounded exact-only claim requested with
/// statistical evidence settles `unsupported` `exact-only-form` with no
/// sample drawn.
#[ix_trace_rs::trace("TC-2066", "FR-741-AC-4")]
#[ix_trace_rs::trace("FSpec-TC-1564", "FSpec-FR-641-AC-3")]
#[test]
#[ignore = "QSL-2158: probability item refusals from source through fsl analyze not implemented yet"]
fn an_exact_only_claim_with_statistical_evidence_is_unsupported() {
    let document = run(
        "exact-only-statistical",
        EVENTUALLY,
        json!([{"claim": "Eventually", "probabilistic": {
            "evidence": "statistical", "method": {"type": "okamoto"},
            "seed": "0", "max_witnesses": 0}}]),
    );
    let item = &document["items"][0];
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
#[ignore = "QSL-2151: the `on each` activation of a probabilistic claim not checked yet"]
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
#[ignore = "QSL-2151: an on-each probabilistic claim over an empty population settling inconclusive no-instance not implemented yet"]
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
#[ignore = "QSL-2151: a never-activating on-each instance holding vacuously from source not implemented yet"]
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
#[ignore = "QSL-2151: naming the first failing on-each instance from source not implemented yet"]
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
    assert_eq!(
        item["counterexample"]["replay"]["reproduced"], true,
        "{item}"
    );
}

/// FR-876-AC-3 (TC-2488): with its `random` lines the same claim's minimum
/// over schedulers is `24/25` and the item is proved.
#[ix_trace_rs::trace("TC-2488", "FR-876-AC-3")]
#[ix_trace_rs::trace("FSpec-TC-1766", "FSpec-FR-724-AC-2")]
#[test]
#[ignore = "QSL-2159: every-scheduler claims with random lines settling from source not implemented yet"]
fn every_scheduler_with_random_is_proved() {
    let document = run_unit("every-random-proved", &unit(EVERY));
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    let text = item.to_string();
    assert!(text.contains("24") && text.contains("25"), "{text}");
}
