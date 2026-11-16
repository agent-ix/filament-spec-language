// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-501-AC-27 to AC-30, FR-526-AC-31 and FR-966 (TC-1546, TC-1547,
//! TC-1548, TC-2721 to TC-2723): the
//! bounded clock-uncertainty cover, per-observation interval stamps, the
//! late-data cutoff and the admission of a late observation by a superseding
//! result, each through `command::run` over a request file whose
//! `timestamped` trace carries FR-501's request-file spelling.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STAMPED: &str = "filament.temporal.timestamped-event.finite-window/v1";

/// One unit: `Ack` answers every request (`level = 1`) with an
/// acknowledgement (`level = 2`) within 10 ms, and `Seen` asks for a reading
/// of 2 within 5 ms of the origin.
fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal Ack using w over (r: Reading) clock \"plant-ms\" on each (q: Reading) \
         {{ holds(r.level = 1) implies eventually[0,10] holds(r.level = 2) }}\n\
         temporal Seen using w over (r: Reading) clock \"plant-ms\" on origin \
         {{ eventually[0,5] holds(r.level = 2) }}\n\
         temporal Until using w over (r: Reading) clock \"plant-ms\" on origin \
         {{ holds(r.level = 0) until[0,5] holds(r.level = 2) }}\n"
    )
}

fn clocks() -> Value {
    json!({"plant-ms": {"kind": "timestamped_event", "timestamp_unit": "ms"}})
}

fn program() -> Value {
    json!({"source": {"file": "unit.native", "authority": "agent-ix",
        "identity": "test:uncertain-late", "document": "UncertainLate", "formal_revision": 1}})
}

/// An observation of `level` at `stamp`.
fn observation(stamp: u64, level: i64) -> Value {
    json!({"position": {"sample": {"level": {"integer": level.to_string()}}}, "stamp": stamp})
}

/// A closed `timestamped` trace over `observations` with watermark and
/// completeness through `watermark`.
fn trace(observations: Vec<Value>, watermark: u64) -> Value {
    json!({
        "observations": observations,
        "progress": {"watermark": watermark,
            "completeness": {"complete": {"through": watermark}},
            "scope": "closed", "execution": "closed"},
        "lower_boundary": "authoritative-origin",
        "binding": {"profile": STAMPED, "clock": "plant-ms",
            "parameters": clocks()["plant-ms"].clone()},
    })
}

fn request(clause: &str, timestamped: Value) -> Value {
    json!({"format": "native-run/1", "request": {
        "models": [], "program": program(), "clocks": clocks(),
        "clause": {"selection": {"temporal": {"name": clause,
            "trace": {"timestamped": timestamped}}}, "observations": {}}}})
}

fn write(directory: &Path, name: &str, request: &Value) -> PathBuf {
    std::fs::write(directory.join("unit.native"), unit()).unwrap();
    let path = directory.join(name);
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

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

fn result(output: &Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "expected a result document, got the refusal {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a result document")
}

fn run(clause: &str, timestamped: Value) -> Value {
    let directory = tempfile::tempdir().unwrap();
    result(&run_file(&write(
        directory.path(),
        "run.json",
        &request(clause, timestamped),
    )))
}

/// The run's temporal verdict kind.
fn kind(document: &Value) -> String {
    find(document, "temporal")
        .and_then(|temporal| temporal["kind"].as_str())
        .unwrap_or_else(|| panic!("a temporal verdict: {document}"))
        .to_owned()
}

/// `Ack` over a request at 0 and an acknowledgement at `ack`, through 17.
fn ack_at(ack: u64, cover: Option<Value>) -> Value {
    let mut timestamped = trace(vec![observation(0, 1), observation(ack, 2)], 17);
    if let Some(cover) = cover {
        timestamped["cover"] = cover;
    }
    run("Ack", timestamped)
}

/// FR-501-AC-27: under `{"bounded": {"uncertainty": 2}}` the acknowledgement
/// at 5, 10 and 15 ms gives holds, indeterminate (the difference straddles
/// the bound) and violated; under `exact` holds, holds and violated; with no
/// acknowledgement the request is pending at watermark 11 and violated at 12.
#[trace("TC-1546", "FR-501-AC-27", "FSpec-TC-1044", "FSpec-FR-160-AC-9")]
#[test]
fn a_bounded_cover_widens_point_stamps_and_straddles_the_bound() {
    let bounded = || Some(json!({"bounded": {"uncertainty": 2}}));
    assert_eq!(kind(&ack_at(5, bounded())), "holds");
    let straddle = ack_at(10, bounded());
    assert_eq!(kind(&straddle), "indeterminate", "{straddle}");
    assert_eq!(
        find(&straddle, "temporal").unwrap()["reason"]["kind"],
        "straddles-bound",
        "{straddle}"
    );
    assert_eq!(kind(&ack_at(15, bounded())), "violated");
    assert_eq!(kind(&ack_at(5, None)), "holds");
    assert_eq!(kind(&ack_at(10, None)), "holds");
    assert_eq!(kind(&ack_at(15, None)), "violated");

    let mut silent = trace(vec![observation(0, 1)], 11);
    silent["cover"] = json!({"bounded": {"uncertainty": 2}});
    assert_eq!(kind(&run("Ack", silent.clone())), "pending-window");
    silent["progress"]["watermark"] = json!(12);
    silent["progress"]["completeness"] = json!({"complete": {"through": 12}});
    assert_eq!(kind(&run("Ack", silent)), "violated");
}

/// FR-501-AC-28: overlapping interval stamps with no order keys make `until`
/// refuse at the later position while `eventually` evaluates; malformed
/// intervals refuse at the observation; an interval overrides the cover
/// (the acknowledgement `[9, 10]` against the request `[0, 0]` is within 10
/// although the cover of 5 would straddle it).
#[trace("TC-1546", "FR-501-AC-28", "FSpec-TC-1044", "FSpec-FR-160-AC-10")]
#[test]
#[ignore = "QSL-2146: per-observation interval stamps are not implemented yet"]
fn interval_stamps_order_refuse_and_override_the_cover() {
    let interval = |stamp: u64, level: i64, earliest: u64, latest: u64, latest_open: bool| {
        let mut observed = observation(stamp, level);
        observed["interval"] =
            json!({"earliest": earliest, "latest": latest, "latest_open": latest_open});
        observed
    };
    let overlapping = || {
        trace(
            vec![
                observation(0, 0),
                interval(4, 0, 3, 5, false),
                interval(5, 2, 4, 6, true),
            ],
            9,
        )
    };
    let refused = run("Until", overlapping());
    assert_eq!(kind(&refused), "trace-refused", "{refused}");
    assert_eq!(find(&refused, "position"), Some(&json!(2)), "{refused}");
    assert_ne!(kind(&run("Seen", overlapping())), "trace-refused");

    for (earliest, latest, latest_open) in [(6, 5, false), (5, 5, true)] {
        let directory = tempfile::tempdir().unwrap();
        let output = run_file(&write(
            directory.path(),
            "run.json",
            &request(
                "Seen",
                trace(
                    vec![
                        observation(0, 0),
                        interval(5, 2, earliest, latest, latest_open),
                    ],
                    9,
                ),
            ),
        ));
        let document: Value = serde_json::from_slice(if output.stdout.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        })
        .expect("a document");
        assert_eq!(
            find(&document, "code").or_else(|| find(&document, "kind")),
            Some(&json!("invalid_runtime_input")),
            "[{earliest}, {latest}]: {document}"
        );
    }

    let mut narrowed = trace(
        vec![interval(0, 1, 0, 0, false), interval(9, 2, 9, 10, false)],
        17,
    );
    narrowed["cover"] = json!({"bounded": {"uncertainty": 5}});
    assert_eq!(kind(&run("Ack", narrowed)), "holds");
}

/// The observations of FR-501-AC-29: stamp 0 (arrival 0, level 0), stamp 4
/// (arrival 6, level `four`) and stamp 3 (arrival 6, level `three`).
fn late_trace(four: i64, three: i64, watermark: u64, cutoff: Option<Value>) -> Value {
    let arrive = |stamp: u64, level: i64, arrival: u64| {
        let mut observed = observation(stamp, level);
        observed["arrival"] = json!(arrival);
        observed
    };
    let mut timestamped = trace(
        vec![arrive(0, 0, 0), arrive(4, four, 6), arrive(3, three, 6)],
        watermark,
    );
    if let Some(cutoff) = cutoff {
        timestamped["late_data_cutoff"] = cutoff;
    }
    timestamped
}

/// FR-501-AC-29: under an allowed lateness of 2 the window with deadline 5 is
/// pending at watermark 6 and settles at 7; the stamp-3 observation arriving
/// at 6 is late, never read, and listed with its arrival and anchor; under a
/// fixed instant 5 it is late and a stamp-6 arrival at 6 is on time.
#[trace("TC-1547", "FR-501-AC-29", "FSpec-TC-1043", "FSpec-FR-094-AC-12")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn the_late_data_cutoff_delays_establishment_and_excludes_late_observations() {
    let lateness = || Some(json!({"allowed_lateness": 2}));
    assert_eq!(
        kind(&run("Seen", late_trace(2, 2, 6, lateness()))),
        "pending-window"
    );
    let settled = run("Seen", late_trace(2, 2, 7, lateness()));
    assert_eq!(kind(&settled), "holds", "{settled}");
    assert_eq!(
        settled["late"],
        json!([{"position": 2, "stamp": 3, "arrival": 6, "anchors": [0]}]),
        "{settled}"
    );
    let only_late = run("Seen", late_trace(0, 2, 7, lateness()));
    assert_eq!(kind(&only_late), "violated", "{only_late}");
    assert_eq!(only_late["late"], settled["late"]);

    let fixed = run(
        "Seen",
        late_trace(0, 2, 7, Some(json!({"fixed_instant": 5}))),
    );
    assert_eq!(kind(&fixed), "violated", "{fixed}");
    // Under the fixed instant 5 the stamp-4 observation arriving at 6 is late
    // too (FR-966-AC-4); the list is in input order.
    let stamps: Vec<_> = fixed["late"]
        .as_array()
        .expect("a late list")
        .iter()
        .map(|entry| entry["stamp"].clone())
        .collect();
    assert_eq!(stamps, [json!(4), json!(3)], "{fixed}");
}

/// FR-501-AC-30: with no cutoff every observation is read and no `late`
/// member is written; adding the cutoff changes the result identity.
#[trace("TC-1547", "FR-501-AC-30", "FSpec-TC-1043", "FSpec-FR-094-AC-13")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn without_a_cutoff_nothing_is_late_and_the_cutoff_enters_the_identity() {
    let without = run("Seen", late_trace(0, 2, 7, None));
    assert_eq!(kind(&without), "holds", "{without}");
    assert!(without.get("late").is_none(), "{without}");
    let with = run(
        "Seen",
        late_trace(0, 2, 7, Some(json!({"allowed_lateness": 2}))),
    );
    assert_ne!(
        find(&without, "result_identity"),
        find(&with, "result_identity")
    );
}

/// FR-526-AC-31: the late observation never rewrites `r1`; a superseding run
/// whose trace admits it on time reads it at lineage 1, and re-sending `r1`'s
/// own trace as a superseding run refuses `unchanged-input`.
#[trace("TC-1548", "FR-526-AC-31", "FSpec-TC-1043", "FSpec-FR-094-AC-13")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff and the run-request relation are not implemented yet"]
fn a_late_observation_is_admitted_only_by_a_superseding_result() {
    let directory = tempfile::tempdir().unwrap();
    let lateness = json!({"allowed_lateness": 2});
    let original_trace = late_trace(0, 2, 7, Some(lateness.clone()));
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &request("Seen", original_trace.clone()),
    )));
    assert_eq!(kind(&first), "violated", "{first}");
    let r1 = first["result_identity"].clone();
    std::fs::write(
        directory.path().join("evidence.json"),
        serde_json::to_vec(&first["evidence_ledger"]).unwrap(),
    )
    .unwrap();

    let superseding = |timestamped: Value| {
        let mut next = request("Seen", timestamped);
        let temporal = &mut next["request"]["clause"]["selection"]["temporal"];
        temporal["relation"] = json!({"superseding": r1.clone()});
        temporal["evidence_ledger"] = json!("evidence.json");
        next
    };
    let mut repaired = original_trace.clone();
    repaired["observations"][2]["arrival"] = json!(5);
    let second = result(&run_file(&write(
        directory.path(),
        "second.json",
        &superseding(repaired),
    )));
    assert_eq!(kind(&second), "holds", "{second}");
    assert_eq!(second["lineage"], 1, "{second}");
    assert_eq!(second["late"], json!([]), "{second}");

    let unchanged = run_file(&write(
        directory.path(),
        "third.json",
        &superseding(original_trace),
    ));
    let refusal: Value = serde_json::from_slice(if unchanged.stdout.is_empty() {
        &unchanged.stderr
    } else {
        &unchanged.stdout
    })
    .expect("a document");
    assert!(refusal.to_string().contains("unchanged-input"), "{refusal}");
}

/// `observation` at `stamp` carrying `interval` and, when given, `arrival`.
fn stamped(stamp: u64, level: i64, interval: Option<Value>, arrival: Option<u64>) -> Value {
    let mut observed = observation(stamp, level);
    if let Some(interval) = interval {
        observed["interval"] = interval;
    }
    if let Some(arrival) = arrival {
        observed["arrival"] = json!(arrival);
    }
    observed
}

/// The document a run wrote, a result on stdout or a refusal on stderr.
fn document_of(output: &Output) -> Value {
    serde_json::from_slice(if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    })
    .expect("a document")
}

/// The document of running `clause` over `timestamped`, refusal or result.
fn attempt(clause: &str, timestamped: Value) -> Value {
    let directory = tempfile::tempdir().unwrap();
    document_of(&run_file(&write(
        directory.path(),
        "run.json",
        &request(clause, timestamped),
    )))
}

/// FR-966-AC-1: a malformed interval, or a nominal stamp outside its
/// interval, refuses at stage `admit` naming member `interval` and the
/// input-order position 1; a stamp inside its interval is admitted.
#[trace("TC-2721", "FR-966-AC-1")]
#[test]
#[ignore = "QSL-2146: per-observation interval stamps are not implemented yet"]
fn a_malformed_or_foreign_interval_refuses_at_admission() {
    let interval = |earliest: u64, latest: u64, latest_open: bool| json!({"earliest": earliest, "latest": latest, "latest_open": latest_open});
    let second = |stamp: u64, interval: Value| {
        trace(
            vec![observation(0, 0), stamped(stamp, 2, Some(interval), None)],
            9,
        )
    };
    for (stamp, earliest, latest, open) in [
        (5, 6, 5, false),
        (5, 5, 5, true),
        (7, 4, 6, false),
        (6, 4, 6, true),
    ] {
        let document = attempt("Seen", second(stamp, interval(earliest, latest, open)));
        let text = document.to_string();
        assert!(text.contains("invalid_runtime_input"), "{document}");
        assert!(text.contains("invalid-value"), "{document}");
        assert!(text.contains("\"admit\""), "{document}");
        assert!(text.contains("\"interval\""), "{document}");
        assert_eq!(find(&document, "position"), Some(&json!(1)), "{document}");
    }
    for (stamp, earliest, latest, open) in [(6, 4, 6, false), (5, 4, 6, true)] {
        run("Seen", second(stamp, interval(earliest, latest, open)));
    }
}

/// FR-966-AC-2: interval stamps `[0, 0]` and `[9, 10]` override the cover
/// `Bounded { uncertainty: 5 }`, under which the point stamps straddle.
#[trace("TC-2721", "FR-966-AC-2")]
#[test]
#[ignore = "QSL-2146: per-observation interval stamps are not implemented yet"]
fn an_interval_stamp_overrides_the_bounded_cover() {
    let point = |interval: bool| {
        let at = |stamp: u64, level: i64, latest: u64| {
            let span = json!({"earliest": stamp, "latest": latest});
            stamped(stamp, level, interval.then_some(span), None)
        };
        let mut timestamped = trace(vec![at(0, 1, 0), at(9, 2, 10)], 17);
        timestamped["cover"] = json!({"bounded": {"uncertainty": 5}});
        timestamped
    };
    assert_eq!(kind(&run("Ack", point(true))), "holds");
    let straddle = run("Ack", point(false));
    assert_eq!(kind(&straddle), "indeterminate", "{straddle}");
    assert_eq!(
        find(&straddle, "temporal").unwrap()["reason"]["kind"],
        "straddles-bound"
    );
}

/// FR-966-AC-3: the cutoff classifies the upper end of an observation's
/// uncertainty, and the `late` entry's `stamp` is that instant.
#[trace("TC-2722", "FR-966-AC-3")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn the_cutoff_classifies_the_latest_possible_instant() {
    let late_stamps = |cover: Value, observed: Value| {
        let mut timestamped = trace(vec![stamped(0, 0, None, Some(0)), observed], 9);
        timestamped["cover"] = cover;
        timestamped["late_data_cutoff"] = json!({"allowed_lateness": 2});
        let result = run("Seen", timestamped);
        result["late"]
            .as_array()
            .unwrap_or_else(|| panic!("a late list: {result}"))
            .iter()
            .map(|entry| entry["stamp"].clone())
            .collect::<Vec<_>>()
    };
    let span = |earliest: u64, latest: u64| json!({"earliest": earliest, "latest": latest});
    let exact = json!("exact");
    assert!(late_stamps(exact.clone(), stamped(3, 2, Some(span(2, 4)), Some(6))).is_empty());
    assert_eq!(
        late_stamps(exact, stamped(3, 2, Some(span(2, 3)), Some(6))),
        [json!(3)]
    );
    for cover in [
        json!({"bounded": {"uncertainty": 1}}),
        json!({"counter": {"uncertainty": 1}}),
    ] {
        assert!(late_stamps(cover.clone(), stamped(3, 2, None, Some(6))).is_empty());
        assert_eq!(late_stamps(cover, stamped(2, 2, None, Some(6))), [json!(3)]);
    }
}

/// FR-966-AC-4: late observations take no position; under the fixed instant
/// 5 both later observations are late; an all-late trace refuses at `admit`.
#[trace("TC-2722", "FR-966-AC-4")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn late_observations_take_no_position() {
    let settled = run(
        "Seen",
        late_trace(2, 2, 7, Some(json!({"allowed_lateness": 2}))),
    );
    assert_eq!(kind(&settled), "holds", "{settled}");
    assert!(
        settled.to_string().contains("\"position\":1"),
        "support at position 1: {settled}"
    );
    let fixed = run(
        "Seen",
        late_trace(2, 2, 7, Some(json!({"fixed_instant": 5}))),
    );
    assert_eq!(kind(&fixed), "violated", "{fixed}");
    assert_eq!(
        fixed["late"],
        json!([{"position": 1, "stamp": 4, "arrival": 6, "anchors": [0]},
               {"position": 2, "stamp": 3, "arrival": 6, "anchors": [0]}]),
        "{fixed}"
    );
    let mut all_late = late_trace(2, 2, 7, Some(json!({"fixed_instant": 5})));
    all_late["observations"].as_array_mut().unwrap().remove(0);
    let refused = attempt("Seen", all_late);
    let text = refused.to_string();
    assert!(
        text.contains("invalid_runtime_input") && text.contains("\"admit\""),
        "{refused}"
    );
}

/// FR-966-AC-5: the cover, an interval stamp, an arrival watermark and the
/// late-data cutoff's rule and parameter each enter both identities.
#[trace("TC-2723", "FR-966-AC-5")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn uncertainty_and_lateness_inputs_enter_both_identities() {
    let base = late_trace(2, 2, 7, Some(json!({"allowed_lateness": 2})));
    let identities = |timestamped: Value| {
        let result = run("Seen", timestamped);
        (
            find(&result, "result_identity").cloned(),
            find(&result, "input_identity").cloned(),
        )
    };
    let original = identities(base.clone());
    let mut variants = Vec::new();
    let mut arrival = base.clone();
    arrival["observations"][1]["arrival"] = json!(7);
    variants.push(arrival);
    let mut rule = base.clone();
    rule["late_data_cutoff"] = json!({"fixed_instant": 2});
    variants.push(rule);
    let mut parameter = base.clone();
    parameter["late_data_cutoff"] = json!({"allowed_lateness": 3});
    variants.push(parameter);
    let mut cover = base.clone();
    cover["cover"] = json!({"bounded": {"uncertainty": 0}});
    variants.push(cover);
    let mut interval = base;
    interval["observations"][1]["interval"] = json!({"earliest": 4, "latest": 4});
    variants.push(interval);
    for variant in variants {
        let changed = identities(variant.clone());
        assert_ne!(changed.0, original.0, "{variant}");
        assert_ne!(changed.1, original.1, "{variant}");
    }
}

/// A unit with `Once`: `once[0,4]` of a reading of 2, anchored at each
/// position.
fn once_unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile w = \"{STAMPED}\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         temporal Once using w over (r: Reading) clock \"plant-ms\" on each (q: Reading) \
         {{ once[0,4] holds(r.level = 2) }}\n"
    )
}

/// FR-966-AC-6: the history cutoff yields `MissingHistory` at position 0
/// and never classifies an observation late; the late-data cutoff excludes
/// the stamp-3 observation, anchored at position 1.
#[trace("TC-2722", "FR-966-AC-6")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff is not implemented yet"]
fn a_history_cutoff_and_a_late_data_cutoff_stay_independent() {
    let once = |cutoff: Option<Value>| {
        let mut timestamped = late_trace(2, 2, 7, cutoff);
        timestamped["lower_boundary"] = json!("history-cutoff");
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("unit.native"), once_unit()).unwrap();
        let path = directory.path().join("run.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&request("Once", timestamped)).unwrap(),
        )
        .unwrap();
        result(&run_file(&path))
    };
    let with = once(Some(json!({"allowed_lateness": 2})));
    assert_eq!(kind(&with), "indeterminate", "{with}");
    assert_eq!(
        find(&with, "temporal").unwrap()["reason"]["kind"],
        "missing-history",
        "{with}"
    );
    assert_eq!(
        with["late"],
        json!([{"position": 2, "stamp": 3, "arrival": 6, "anchors": [1]}]),
        "{with}"
    );
    let without = once(None);
    assert!(without.get("late").is_none(), "{without}");
    assert!(!without.to_string().contains("contradiction"), "{without}");
}

/// FR-966-AC-7: a refused correction refuses the run at stage `evaluate`
/// with member `relation` and its cause, and writes no result.
#[trace("TC-2723", "FR-966-AC-7")]
#[test]
#[ignore = "QSL-2146: the late-data cutoff and the run-request relation are not implemented yet"]
fn a_refused_correction_writes_no_result() {
    let directory = tempfile::tempdir().unwrap();
    let original_trace = late_trace(0, 2, 7, Some(json!({"allowed_lateness": 2})));
    let first = result(&run_file(&write(
        directory.path(),
        "first.json",
        &request("Seen", original_trace.clone()),
    )));
    std::fs::write(
        directory.path().join("evidence.json"),
        serde_json::to_vec(&first["evidence_ledger"]).unwrap(),
    )
    .unwrap();
    for (named, cause) in [
        (first["result_identity"].clone(), "unchanged-input"),
        (json!("unknown"), "predecessor-unknown"),
    ] {
        let mut next = request("Seen", original_trace.clone());
        let temporal = &mut next["request"]["clause"]["selection"]["temporal"];
        temporal["relation"] = json!({"superseding": named});
        temporal["evidence_ledger"] = json!("evidence.json");
        let output = run_file(&write(directory.path(), "next.json", &next));
        assert!(output.stdout.is_empty(), "no result document");
        let refusal = document_of(&output);
        let text = refusal.to_string();
        for needle in [
            "invalid_runtime_input",
            "invalid-value",
            "\"evaluate\"",
            "\"relation\"",
            cause,
        ] {
            assert!(text.contains(needle), "{needle}: {refusal}");
        }
    }
}
