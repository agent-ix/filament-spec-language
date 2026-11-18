// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 and FR-751 (TC-2100, TC-2101), FSpec FR-652 (TC-1605) and FSpec
//! FR-193-AC-23 (TC-373): hybrid items requested through `native-analyze/1`
//! documents (FR-620) and settled through the command adapter's analyze
//! entry, `fsl_command::command::analyze_with_providers` (FR-635), with
//! fixture hybrid providers registered as the caller's providers. The outcome
//! is FR-621's document: each item's `record` (`result`, `basis` or `cause`,
//! `certification`, `backend`), its `counterexample` and `replay`, and its
//! `certificate` (`accepted`, `rule`, `at`, `form`).
//!
//! The fixture providers are made by `Engine::hybrid` and
//! `Engine::hybrid_process` (FR-750), which do not exist yet. Until they do,
//! [`hybrid_fixture`], [`process_fixture`] and [`hybrid_engine`] build a probe
//! engine in their place: it advertises no hybrid dynamics, so it is never a
//! candidate for a hybrid item, and every assertion below fails for real.

use std::path::PathBuf;

use fsl_analyze::analyze::Engine;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze_with_providers, Run, Workspace};

/// FR-447's `Braking` without `t`; `claims` follow the declaration.
pub(crate) fn braking(claims: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Braking using p {{\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ v' = -a; a' = 0; }}\n\
             transition to Stopped when {{ v = 0 }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ v' = 0; a' = 0; }}\n\
         }}\n{claims}"
    )
}

/// A `model-time` temporal claim `name` over `Braking` with `formula`.
pub(crate) fn claim(name: &str, formula: &str) -> String {
    format!(
        "temporal {name} using p over (h: Braking) clock \"model-time\" \
         on origin {{ {formula} }}\n"
    )
}

/// `SlowEnough` (holds within horizon 10) and `TooSlow` (violated).
pub(crate) fn braking_pair() -> String {
    braking(&format!(
        "{}{}",
        claim("SlowEnough", "always [0, 10] holds(h.v < 30)"),
        claim("TooSlow", "always [0, 10] holds(h.v > 5)")
    ))
}

/// Write `unit` and a `native-analyze/1` document over it with `items` and
/// the extra request members `extra`; returns the document's path.
pub(crate) fn request(ws: &Workspace, unit: &str, items: Value, extra: Value) -> PathBuf {
    ws.write("unit.fsl", unit.as_bytes());
    let mut request = json!({
        "program": {"source": {"file": "unit.fsl", "authority": "test",
            "identity": "hybrid-unit", "document": "HybridUnit", "formal_revision": 1}},
        "items": items,
    });
    for (key, value) in extra.as_object().into_iter().flatten() {
        request[key] = value.clone();
    }
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

/// Analyze `unit`'s `items` with `providers` beside the command adapter's
/// engine set and return the first outcome item.
pub(crate) fn first_item(
    name: &str,
    unit: &str,
    items: Value,
    extra: Value,
    providers: Vec<Engine>,
) -> (Run, Value) {
    let ws = Workspace::new(name);
    let run = analyze_with_providers(&request(&ws, unit, items, extra), providers);
    let item = run.item(0);
    (run, item)
}

/// The first value under `key` anywhere in `value`, depth first.
pub(crate) fn find<'v>(value: &'v Value, key: &str) -> Option<&'v Value> {
    match value {
        Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|inner| find(inner, key))),
        Value::Array(items) => items.iter().find_map(|inner| find(inner, key)),
        _ => None,
    }
}

/// The in-process hybrid fixture `id` whose provider answers every item with
/// the FSpec FR-331 result entry `entry`: `Engine::hybrid` over a descriptor
/// with `dynamics` `["hybrid"]` and a provider built by
/// `HybridAnswer::read_entry(&entry)` (FR-750). The probe stands in until
/// those exist.
pub(crate) fn hybrid_fixture(id: &str, entry: &Value) -> Engine {
    let _answer = entry;
    Engine::probe(id)
}

/// The process hybrid fixture `id` whose exchange returns `entry` as its
/// result frame: `Engine::hybrid_process` (FR-750). The probe stands in.
pub(crate) fn process_fixture(id: &str, entry: &Value) -> Engine {
    let _answer = entry;
    Engine::probe(id)
}

/// `Engine::hybrid` over the FSpec FR-331 descriptor `descriptor`, its error
/// rendered as text (FR-750). The probe stands in and never refuses.
pub(crate) fn hybrid_engine(descriptor: &Value) -> Result<Engine, String> {
    let id = descriptor["backend"].as_str().unwrap_or("hybrid-fixture");
    Ok(Engine::probe(id))
}

fn interval(lower: &str, upper: &str) -> Value {
    json!({"lower": lower, "upper": upper})
}

fn inductive_entry(moving_v_upper: &str) -> Value {
    json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "inductive-solver", "method": "inductive-enclosure"},
        "certificate": {"type": "inductive-enclosure", "boxes": {
            "Moving": {"v": interval("0", moving_v_upper), "a": interval("1", "2")},
            "Stopped": {"v": interval("0", "0"), "a": interval("1", "2")}}}})
}

fn named(claim: &str, backend: &str) -> Value {
    json!([{"claim": claim, "backend": backend}])
}

/// FR-750-AC-1: a fixture's covering `delta-complete` `unsat` settles
/// `proved`, `bounded-solver`, `uncertified`, naming the fixture.
#[test]
#[trace("TC-2100", "FR-750-AC-1")]
#[ignore = "QSL-2234: analyze dispatch of a hybrid item to its provider not implemented yet"]
fn a_covering_delta_complete_proof_settles_proved_bounded_solver() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": "10", "jumps": 1, "exhaustive": ["time"]}});
    let (_, item) = first_item(
        "hybrid-750-1",
        &braking_pair(),
        named("SlowEnough", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry)],
    );
    let record = &item["record"];
    assert_eq!(record["result"], "proved", "{item}");
    assert_eq!(record["basis"]["type"], "bounded-solver", "{item}");
    assert_eq!(record["certification"], "uncertified", "{item}");
    assert_eq!(record["backend"], "hybrid-fixture", "{item}");
}

/// FR-750-AC-2: `Engine::hybrid` refuses a descriptor whose `dynamics` lacks
/// `hybrid`, naming the backend and `invalid-dynamics`.
#[test]
#[trace("TC-2100", "FR-750-AC-2")]
#[ignore = "QSL-2234: hybrid provider registration (`Engine::hybrid`) not implemented yet"]
fn a_hybrid_provider_without_hybrid_dynamics_is_refused() {
    let refused = hybrid_engine(
        &json!({"backend": "discrete-only", "dynamics": ["discrete"],
        "advertises": [{"kind": "temporal-satisfaction", "mode": "bounded"}],
        "bounds": {"max_solver_steps": 1000, "max_solver_time": 1}}),
    );
    let error = refused.expect_err("no engine is made");
    assert!(error.contains("discrete-only"), "{error}");
    assert!(error.contains("invalid-dynamics"), "{error}");
}

fn point_trace(before_v: &str) -> Value {
    let state = |mode: &str, time: &str, v: &str| {
        json!({"mode": mode, "time": interval(time, time),
            "enclosure": {"v": interval(v, v), "a": interval("2", "2")}})
    };
    json!({"start": state("Moving", "0", "20"), "jumps": [{"transition": 0,
        "before": {"time": interval("10", "10"),
            "enclosure": {"v": interval(before_v, before_v), "a": interval("2", "2")}},
        "state": state("Stopped", "10", "0")}]})
}

fn refuted_entry(trace: Value) -> Value {
    json!({"request_index": 0, "result": "refuted", "counterexample": 0,
        "counterexamples": [{"type": "hybrid-counterexample", "kind": "formula",
            "trace": trace}]})
}

/// FR-750-AC-5: a process provider composed for a clause request returns a
/// point trace; one that replays settles `refuted`, one that does not
/// `inconclusive`, `replay-parity`.
#[test]
#[trace("TC-2100", "FR-750-AC-5")]
#[ignore = "QSL-2234: process-provider composition for hybrid clause requests not implemented yet"]
fn a_process_provider_refutation_settles_by_exact_replay() {
    let unit = braking(&claim("Positive", "always [0, 12] holds(h.v > 0)"));
    let (_, item) = first_item(
        "hybrid-750-5a",
        &unit,
        named("Positive", "hybrid-process"),
        json!({}),
        vec![process_fixture(
            "hybrid-process",
            &refuted_entry(point_trace("0")),
        )],
    );
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    assert!(
        item["counterexample"]
            .to_string()
            .contains("hybrid-counterexample"),
        "{item}"
    );
    let (_, item) = first_item(
        "hybrid-750-5b",
        &unit,
        named("Positive", "hybrid-process"),
        json!({}),
        vec![process_fixture(
            "hybrid-process",
            &refuted_entry(point_trace("1")),
        )],
    );
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["type"], "replay-parity", "{item}");
}

/// FR-751-AC-1 and FSpec FR-652-AC-1: an accepted `inductive-enclosure`
/// settles `certified`; one excluding the initial `v = 20` is rejected at
/// rule (a) at the `Moving` box and stays `uncertified`.
#[test]
#[trace("TC-2101", "FR-751-AC-1")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-1")]
#[ignore = "QSL-2234: core check of hybrid provider certificates not implemented yet"]
fn an_inductive_enclosure_certificate_is_checked_in_the_core() {
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let (_, accepted) = first_item(
        "hybrid-751-1a",
        &unit,
        named("Bounded", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &inductive_entry("20"))],
    );
    assert_eq!(accepted["record"]["result"], "proved", "{accepted}");
    assert_eq!(
        accepted["record"]["certification"], "certified",
        "{accepted}"
    );
    assert_eq!(accepted["certificate"]["accepted"], true, "{accepted}");
    let (_, rejected) = first_item(
        "hybrid-751-1b",
        &unit,
        named("Bounded", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &inductive_entry("19"))],
    );
    assert_eq!(
        rejected["record"]["certification"], "uncertified",
        "{rejected}"
    );
    assert_eq!(rejected["certificate"]["accepted"], false, "{rejected}");
    assert_eq!(rejected["certificate"]["rule"], "a", "{rejected}");
    assert!(
        rejected["certificate"]["at"]
            .as_str()
            .is_some_and(|at| at.contains("Moving")),
        "{rejected}"
    );
}

/// FR-751-AC-2 and FSpec FR-652-AC-2: a `flowpipe` step violating the Picard
/// condition is rejected at that step and the proof stays `uncertified`.
#[test]
#[trace("TC-2101", "FR-751-AC-2")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-2")]
#[ignore = "QSL-2234: core check of flowpipe certificates not implemented yet"]
fn a_flowpipe_step_violating_picard_is_rejected() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "flowpipe-outward-rounded",
            "horizon": "10", "jumps": 1, "exhaustive": ["time"]},
        "certificate": {"type": "flowpipe", "steps": [{"mode": "Moving",
            "time": interval("0", "10"), "step": "10",
            "box": {"v": interval("0", "20"), "a": interval("1", "2")},
            "apriori": {"v": interval("19", "20"), "a": interval("1", "2")}}]}});
    let (_, item) = first_item(
        "hybrid-751-2",
        &braking_pair(),
        named("SlowEnough", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry)],
    );
    assert_eq!(item["record"]["certification"], "uncertified", "{item}");
    assert_eq!(item["certificate"]["accepted"], false, "{item}");
    assert_eq!(item["certificate"]["at"], "/certificate/steps/0", "{item}");
}

/// FR-751-AC-3 and FSpec FR-652-AC-4: a check under a work limit of one unit
/// is rejected `work-limit`, never accepted.
#[test]
#[trace("TC-2101", "FR-751-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
#[ignore = "QSL-2234: work-limited certificate check not implemented yet"]
fn a_work_limited_certificate_check_is_rejected() {
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let (_, item) = first_item(
        "hybrid-751-3",
        &unit,
        named("Bounded", "hybrid-fixture"),
        json!({"limits": {"accounting": {"work_units": 1}}}),
        vec![hybrid_fixture("hybrid-fixture", &inductive_entry("20"))],
    );
    assert_eq!(item["certificate"]["accepted"], false, "{item}");
    assert_eq!(item["certificate"]["rule"], "work-limit", "{item}");
}

/// FSpec FR-652-AC-4: a `flowpipe` certificate on an `inductive-solver`
/// entry refuses `invalid_result`/`invalid-value` at `/certificate`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-4")]
#[ignore = "QSL-2234: certificate form check not implemented yet"]
fn a_mismatched_certificate_form_refuses_at_certificate() {
    let mut entry = inductive_entry("20");
    entry["certificate"] = json!({"type": "flowpipe", "steps": []});
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let (_, item) = first_item(
        "hybrid-652-4",
        &unit,
        named("Bounded", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry)],
    );
    let text = item.to_string();
    assert!(text.contains("invalid_result"), "{item}");
    assert!(text.contains("/certificate"), "{item}");
}

/// FSpec FR-652-AC-3 and FR-193-AC-22: a point trace through a mode whose
/// flow is not polynomial refutes with an accepted `validated-witness`, and
/// without one stays `inconclusive`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-193-AC-22")]
#[ignore = "QSL-2234: validated-witness refutation not implemented yet"]
fn a_validated_witness_refutes_through_a_non_polynomial_flow() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Grow using p {\n\
           mode Up invariant { v >= 1 and v <= 100 } flow { v' = v * v / (1 + v * v); }\n\
         }\n\
         temporal Below using p over (h: Grow) clock \"model-time\" \
         on origin { always [0, 40] holds(h.v < 30) }\n";
    let trace = json!({"start": {"mode": "Up", "time": interval("0", "0"),
        "enclosure": {"v": interval("1", "1")}}, "jumps": []});
    let witness = json!({"type": "validated-witness", "dwells": [[{"mode": "Up",
        "time": interval("0", "40"), "step": "40",
        "box": {"v": interval("1", "1")}, "apriori": {"v": interval("1", "41")}}]]});
    let entry = |certificate: Option<&Value>| {
        let mut entry = json!({"request_index": 0, "result": "inconclusive",
            "cause": {"type": "solver-inconclusive", "trace": trace.clone()}});
        if let Some(certificate) = certificate {
            entry["certificate"] = certificate.clone();
        }
        entry
    };
    let (_, with) = first_item(
        "hybrid-652-3a",
        unit,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(Some(&witness)))],
    );
    assert_eq!(with["record"]["result"], "refuted", "{with}");
    let (_, without) = first_item(
        "hybrid-652-3b",
        unit,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(None))],
    );
    assert_eq!(without["record"]["result"], "inconclusive", "{without}");
}

/// `Braking`'s flowpipe: 160 `Moving` steps of 1/16 s contracted to
/// `v >= 0`, then one `Stopped` jump step whose time is `jump_time`.
fn braking_flowpipe(jump_time: (&str, &str)) -> Value {
    let mut steps = Vec::new();
    for k in 0..160_u32 {
        let start = format!("{k}/16");
        let end = format!("{}/16", k + 1);
        let upper = format!("{}/16", 320 - k);
        steps.push(
            json!({"mode": "Moving", "time": interval(&start, &end), "step": "1/16",
            "box": {"v": interval("0", &upper), "a": interval("1", "2")},
            "apriori": {"v": interval("0", &upper), "a": interval("1", "2")}}),
        );
    }
    steps.push(
        json!({"mode": "Stopped", "time": interval(jump_time.0, jump_time.1),
        "step": "10", "box": {"v": interval("0", "0"), "a": interval("1", "2")},
        "apriori": {"v": interval("0", "0"), "a": interval("1", "2")}}),
    );
    json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "flowpipe-outward-rounded",
            "horizon": "10", "jumps": 1, "exhaustive": ["time"]},
        "certificate": {"type": "flowpipe", "steps": steps}})
}

/// FSpec FR-652-AC-5: a flowpipe whose `Moving` boxes stop at the boundary
/// invariant and whose one jump step carries the exact jump times is
/// accepted; a jump step omitting time 0 is rejected at that step; against
/// a violated claim it is rejected naming the claim rule.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-5")]
#[ignore = "QSL-2234: flowpipe guard and boundary-invariant check not implemented yet"]
fn a_flowpipe_jump_step_carries_the_guarded_part_and_its_jump_times() {
    let unit = braking_pair();
    let check = |name: &str, claim: &str, jump_time: (&str, &str)| {
        first_item(
            name,
            &unit,
            named(claim, "hybrid-fixture"),
            json!({}),
            vec![hybrid_fixture(
                "hybrid-fixture",
                &braking_flowpipe(jump_time),
            )],
        )
        .1
    };
    let accepted = check("hybrid-652-5a", "SlowEnough", ("0", "10"));
    assert_eq!(
        accepted["record"]["certification"], "certified",
        "{accepted}"
    );
    let late = check("hybrid-652-5b", "SlowEnough", ("1", "10"));
    assert_eq!(late["certificate"]["accepted"], false, "{late}");
    assert_eq!(
        late["certificate"]["at"], "/certificate/steps/160",
        "{late}"
    );
    let violated = check("hybrid-652-5c", "TooSlow", ("0", "10"));
    assert_eq!(violated["certificate"]["accepted"], false, "{violated}");
    assert_eq!(violated["certificate"]["rule"], "claim", "{violated}");
}

/// FSpec FR-193-AC-23: a trace cut by `end` inside its last dwell replays
/// exactly to the bad prefix; a wrong `end` does not reproduce; `end` with
/// `loop` refuses at `/end`.
#[test]
#[trace("FSpec-TC-373", "FSpec-FR-193-AC-23")]
#[ignore = "QSL-2234: hybrid trace `end` replay not implemented yet"]
fn a_trace_cut_by_end_replays_to_its_bad_prefix() {
    let unit = braking(&claim("TooSlow", "always [0, 10] holds(h.v > 5)"));
    let trace = |end_v: &str| {
        json!({"start": {"mode": "Moving", "time": interval("0", "0"),
            "enclosure": {"v": interval("20", "20"), "a": interval("2", "2")}},
            "jumps": [], "end": {"time": interval("8", "8"),
            "enclosure": {"v": interval(end_v, end_v), "a": interval("2", "2")}}})
    };
    let run = |name: &str, trace: Value| {
        first_item(
            name,
            &unit,
            named("TooSlow", "hybrid-fixture"),
            json!({}),
            vec![hybrid_fixture("hybrid-fixture", &refuted_entry(trace))],
        )
        .1
    };
    let reproduced = run("hybrid-193-23a", trace("4"));
    assert_eq!(reproduced["record"]["result"], "refuted", "{reproduced}");
    let wrong = run("hybrid-193-23b", trace("5"));
    assert_eq!(wrong["record"]["result"], "inconclusive", "{wrong}");
    let mut looped = trace("4");
    looped["loop"] = json!(0);
    let refused = run("hybrid-193-23c", looped);
    let text = refused.to_string();
    assert!(text.contains("invalid_result"), "{refused}");
    assert!(text.contains("/end"), "{refused}");
}
