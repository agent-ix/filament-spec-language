// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 "Dispatch" for a process provider: the request FSL writes, the
//! result frame it reads and how each result settles, over the scripted
//! provider process.

use std::sync::{Arc, Mutex};

use fsl_analyze::analyze::Engine;
use fsl_analyze::provider::{HybridAnswer, HybridProvider, HybridSolveRequest};
use fsl_replay::{ExpressionLocus, ExpressionSite, TermLocus};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::support::{
    braking, braking_with, claim, engine_at, interval, limits, outcome, point_trace,
    process_engine, rational, refuted, run, run_with_cancel, run_with_library, Workspace, BACKEND,
};

fn named(claim: &str) -> Value {
    json!({"claim": claim, "backend": BACKEND})
}

fn answer(entry: &Value) -> Value {
    json!({"mode": "answer", "outcome": entry})
}

/// FR-750-AC-5: a process provider composed for a clause request returns a
/// point trace; one that replays settles `refuted`, one that does not
/// `inconclusive`, `replay-parity` with the `not-reproduced` disagreement.
#[test]
#[trace("TC-2100", "FR-750-AC-5")]
fn a_process_provider_refutation_settles_by_exact_replay() {
    let unit = braking(&claim("Positive", "always [0, 12] holds(h.v > 0)"));
    let ws = Workspace::new("ac5a");
    let engine = process_engine(&ws, &answer(&refuted(point_trace("0"))), limits());
    let item = run(&ws, &unit, named("Positive"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    assert_eq!(item["record"]["backend"], BACKEND, "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
    assert!(
        item["counterexample"]
            .to_string()
            .contains("hybrid-counterexample"),
        "{item}"
    );
    let ws = Workspace::new("ac5b");
    let engine = process_engine(&ws, &answer(&refuted(point_trace("1"))), limits());
    let item = run(&ws, &unit, named("Positive"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "replay-parity", "{item}");
    assert_eq!(
        item["record"]["cause"]["parity"]["reason"], "hybrid-trace",
        "{item}"
    );
    assert_eq!(
        item["record"]["cause"]["parity"]["disagreement"]["type"], "not-reproduced",
        "{item}"
    );
    assert_eq!(
        item["record"]["cause"]["parity"]["disagreement"]["counterexample"]["trace"],
        point_trace("1"),
        "{item}"
    );
    assert!(
        item.get("counterexample").is_none_or(Value::is_null),
        "{item}"
    );
}

/// FR-450-AC-9 and FR-450 "Settlement": a provider `refuted` trace exact
/// replay cannot decide settles `inconclusive`, `replay-parity`, and the
/// item's cause names the `not-exact` disagreement.
#[test]
#[trace("TC-1221", "FR-450-AC-9")]
fn a_not_exact_process_refutation_names_its_disagreement_type() {
    let unit = braking_with(
        "v' = -v;",
        &claim("Positive", "always [0, 12] holds(h.v > 0)"),
    );
    let ws = Workspace::new("ac450-9");
    let engine = process_engine(&ws, &answer(&refuted(point_trace("0"))), limits());
    let item = run(&ws, &unit, named("Positive"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "replay-parity", "{item}");
    assert_eq!(
        item["record"]["cause"]["parity"]["disagreement"]["type"], "not-exact",
        "{item}"
    );
    assert_eq!(
        item["record"]["cause"]["parity"]["disagreement"]["counterexample"]["type"],
        "hybrid-counterexample",
        "{item}"
    );
}

/// An in-process provider that records the emitted-package node id FR-331
/// writes for the construct at `locus` and answers nothing.
#[derive(Debug)]
struct LocusProbe(Arc<Mutex<Option<Value>>>);

impl HybridProvider for LocusProbe {
    fn solve(
        &self,
        request: &HybridSolveRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> HybridAnswer {
        let claim = request
            .graph
            .hybrid_claims()
            .iter()
            .position(|candidate| std::ptr::eq(candidate, request.claim))
            .expect("the claim belongs to its graph");
        // `Moving`'s rate of `v` is the division `-a / (1 + v)`.
        let locus = fsl_replay::hybrid_locus_wire(
            request.graph,
            request.wire,
            claim,
            &ExpressionLocus {
                site: ExpressionSite::Rate {
                    mode: 0,
                    variable: 0,
                },
                term: TermLocus::default(),
            },
        )
        .expect("the division has an emitted locus");
        *self.0.lock().expect("the probe is not poisoned") = Some(locus);
        HybridAnswer::solved(fsl_replay::SolverResult {
            method: fsl_replay::SolverMethod::Unclassified,
            outcome: fsl_replay::SolverOutcome::Unknown(None),
            coverage: fsl_replay::SolverCoverage::Inductive,
        })
    }
}

/// The division's emitted node id and `Locus` in the unit with the flow
/// `v' = -a / (1 + v)`, which the emitted package fixes.
fn division_locus(unit: &str) -> Value {
    let seen = Arc::new(Mutex::new(None));
    let probe = Engine::hybrid(
        crate::support::descriptor("locus-probe"),
        Box::new(LocusProbe(Arc::clone(&seen))),
    )
    .expect("the probe engine is made");
    let ws = Workspace::new("locus-probe");
    let item = run(
        &ws,
        unit,
        json!({"claim": "SlowEnough", "backend": "locus-probe"}),
        vec![probe],
        limits(),
    );
    let locus = seen.lock().expect("the probe is not poisoned").take();
    locus.unwrap_or_else(|| panic!("the probe ran: {item}"))
}

fn division_unit() -> String {
    braking_with(
        "v' = -a / (1 + v);",
        &claim("SlowEnough", "always [0, 10] holds(h.v < 30)"),
    )
}

fn unsupported_entry(locus: &Value, mode: &str) -> Value {
    outcome(
        json!({"result": "unsupported", "diagnostic": {
            "code": "unsupported_construct", "cause": "expression-form",
            "detail": {"form": "division", "declaration": "Braking", "mode": mode,
                "variable": "v", "backend": BACKEND, "locus": locus}}}),
        &[],
    )
}

/// FR-750-AC-7 (process half): a process fixture whose entry is
/// `unsupported` with `unsupported_construct`/`expression-form` settles the
/// same located `unsupported` as the in-process fixture, with no
/// counterexample and no replay; a mode the model lacks refuses the entry.
#[test]
#[trace("TC-3035", "FR-750-AC-7")]
#[trace("TC-3037", "FR-750-AC-13")]
fn a_process_unsupported_entry_settles_a_located_expression_form() {
    let unit = division_unit();
    let locus = division_locus(&unit);
    let ws = Workspace::new("ac7");
    let engine = process_engine(&ws, &answer(&unsupported_entry(&locus, "Moving")), limits());
    let item = run(&ws, &unit, named("SlowEnough"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "unsupported", "{item}");
    assert_eq!(item["record"]["backend"], BACKEND, "{item}");
    let text = item.to_string();
    for member in [
        "unsupported_construct",
        "expression-form",
        "Braking",
        "Moving",
    ] {
        assert!(text.contains(member), "{member}: {item}");
    }
    assert!(
        text.contains(&locus["node"].to_string()),
        "the locus: {item}"
    );
    assert!(
        item.get("counterexample").is_none_or(Value::is_null),
        "{item}"
    );
    assert!(item.get("replay").is_none_or(Value::is_null), "{item}");

    let ws = Workspace::new("ac7-mode");
    let engine = process_engine(&ws, &answer(&unsupported_entry(&locus, "Halted")), limits());
    let item = run(&ws, &unit, named("SlowEnough"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "failed", "{item}");
    let text = item.to_string();
    assert!(text.contains("invalid_result"), "{item}");
    assert!(text.contains("invalid-value"), "{item}");
    assert!(text.contains("/diagnostic/detail/mode"), "{item}");
}

/// FR-750-AC-11 (process half): a process `failed` `tool-failure` entry is
/// not a solver answer and the item settles `failed` by FR-290 with that
/// defect.
#[test]
#[trace("TC-3035", "FR-750-AC-11")]
fn a_process_failed_entry_settles_failed_by_fr_290() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let entry = outcome(
        json!({"result": "failed", "cause": {"type": "tool-failure", "defect": "crash"}}),
        &[],
    );
    let ws = Workspace::new("ac11");
    let engine = process_engine(&ws, &answer(&entry), limits());
    let item = run(&ws, &unit, named("SlowEnough"), vec![engine], limits());
    assert_eq!(item["record"]["result"], "failed", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "tool-failure", "{item}");
    assert_eq!(item["record"]["cause"]["defect"], "crash", "{item}");
}

fn failure_of(
    script: &Value,
    tweak: impl FnOnce(&mut fsl_analyze::provider::ProviderLimits),
) -> Value {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let ws = Workspace::new("failure");
    let mut run_limits = limits();
    tweak(&mut run_limits);
    let engine = process_engine(&ws, script, run_limits);
    run(&ws, &unit, named("SlowEnough"), vec![engine], run_limits)
}

fn assert_defect(item: &Value, defect: &str) {
    assert_eq!(item["record"]["result"], "failed", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "tool-failure", "{item}");
    assert_eq!(item["record"]["cause"]["defect"], defect, "{item}");
}

/// FR-750 "Dispatch" with FSpec FR-462 "Failures": a provider that crashes,
/// exits non-zero, ends without answering, or speaks garbage settles the
/// item `failed` with the typed defect; none is `proved` or `refuted`.
#[test]
fn a_misbehaving_process_provider_settles_failed_with_a_typed_defect() {
    let good = outcome(
        json!({"result": "proved", "certification": "trusted", "basis": {
            "type": "bounded-solver", "method": "delta-complete", "horizon": rational("10"),
            "jumps": 1, "exhaustive": ["time"]}}),
        &[],
    );
    for (script, defect) in [
        (json!({"mode": "crash", "stderr": "boom"}), "crash"),
        (json!({"mode": "exit-nonzero"}), "nonzero-exit"),
        (json!({"mode": "silent"}), "missing-terminal-record"),
        (json!({"mode": "garbage"}), "malformed-frame"),
        (
            json!({"mode": "noncanonical", "outcome": good}),
            "noncanonical-frame",
        ),
        (
            json!({"mode": "foreign-protocol", "outcome": good}),
            "unknown-protocol",
        ),
    ] {
        assert_defect(&failure_of(&script, |_| {}), defect);
    }
    let oversize = failure_of(&json!({"mode": "oversize", "size": 1_048_577}), |limits| {
        limits.max_frame_bytes = 1_048_576;
    });
    assert_defect(&oversize, "frame-length");
    assert_eq!(
        oversize["record"]["cause"]["limit"], 1_048_576,
        "{oversize}"
    );
    assert_eq!(
        oversize["record"]["cause"]["value"], 1_048_577,
        "{oversize}"
    );
    let ok = failure_of(
        &json!({"mode": "answer", "outcome": good, "stderr": "noise"}),
        |_| {},
    );
    assert_eq!(
        ok["record"]["result"], "proved",
        "stderr changes nothing: {ok}"
    );
}

/// A provider that is not there is a typed failure, never a verdict.
#[test]
fn a_missing_provider_executable_settles_failed() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let ws = Workspace::new("missing");
    let script = ws.write("script.json", b"{}");
    let engine = engine_at(BACKEND, &ws.path("no-such-provider"), &script, limits());
    let item = run(&ws, &unit, named("SlowEnough"), vec![engine], limits());
    assert_defect(&item, "crash");
}

/// A provider that overruns the run's `deadline_ms` is ended and the item
/// settles `incomplete`, `timed-out`, never `proved` or `refuted`.
#[test]
fn a_provider_over_the_deadline_settles_incomplete_timed_out() {
    let item = failure_of(&json!({"mode": "hang"}), |limits| limits.deadline_ms = 300);
    assert_eq!(item["record"]["result"], "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "timed-out", "{item}");
}

/// FR-750-AC-6 (process half): the item's `options` reach the provider as
/// the strings the request wrote, in the request the provider reads beside
/// the emitted package's v2 bytes and the run's deadline.
#[test]
#[trace("TC-3035", "FR-750-AC-6")]
fn item_options_and_the_deadline_reach_the_process_provider() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let ws = Workspace::new("ac6");
    let recorded = ws.path("request.json");
    let script = json!({"mode": "answer", "record_request": recorded,
        "outcome": outcome(json!({"result": "inconclusive",
            "cause": {"type": "solver-inconclusive"}}), &[])});
    let engine = process_engine(&ws, &script, limits());
    let item = run(
        &ws,
        &unit,
        json!({"claim": "SlowEnough", "backend": BACKEND, "options": {"step": "1/16"}}),
        vec![engine],
        limits(),
    );
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    let request: Value =
        serde_json::from_slice(&std::fs::read(recorded).expect("the request was recorded"))
            .expect("the request is JSON");
    assert_eq!(
        request["request"]["items"][0]["options"],
        json!({"step": "1/16"})
    );
    assert_eq!(
        request["request"]["items"][0]["requirements"],
        json!(["hybrid-dynamics"])
    );
    assert_eq!(request["request"]["deadline_ms"], limits().deadline_ms);
    assert!(request["request"]["package"].is_string());
}

/// FR-750 "Dispatch": a process provider's `proved` word is `trusted`; its
/// entry may not claim another certification, and an FR-751-accepted
/// certificate makes the record `certified`.
#[test]
#[trace("TC-2101", "FR-751-AC-1")]
fn a_process_proof_is_trusted_unless_its_certificate_is_accepted() {
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let inductive = |certification: &str, moving_upper: &str, certificate: bool| {
        let mut entry = json!({"result": "proved", "certification": certification,
            "basis": {"type": "inductive-solver", "method": "inductive-enclosure"}});
        if certificate {
            entry["certificate"] = json!({"type": "inductive-enclosure", "boxes": {
                "Moving": {"v": interval("0", moving_upper), "a": interval("1", "2")},
                "Stopped": {"v": interval("0", "0"), "a": interval("1", "2")}}});
        }
        outcome(entry, &[])
    };
    let settle = |name: &str, entry: Value| {
        let ws = Workspace::new(name);
        let engine = process_engine(&ws, &answer(&entry), limits());
        run(&ws, &unit, named("Bounded"), vec![engine], limits())
    };
    let bare = settle("trusted", inductive("trusted", "20", false));
    assert_eq!(bare["record"]["result"], "proved", "{bare}");
    assert_eq!(bare["record"]["certification"], "trusted", "{bare}");
    let accepted = settle("accepted", inductive("trusted", "20", true));
    assert_eq!(
        accepted["record"]["certification"], "certified",
        "{accepted}"
    );
    assert_eq!(accepted["certificate"]["accepted"], true, "{accepted}");
    let rejected = settle("rejected", inductive("trusted", "19", true));
    assert_eq!(rejected["record"]["certification"], "trusted", "{rejected}");
    assert_eq!(rejected["certificate"]["accepted"], false, "{rejected}");
    let claimed = settle("claimed", inductive("uncertified", "20", false));
    assert_eq!(claimed["record"]["result"], "failed", "{claimed}");
    assert!(
        claimed.to_string().contains("certification-mismatch"),
        "{claimed}"
    );
}

fn solver_proof(horizon: &str, certification: &str) -> Value {
    json!({"result": "proved", "certification": certification, "basis": {
        "type": "bounded-solver", "method": "delta-complete", "horizon": rational(horizon),
        "jumps": 1, "exhaustive": ["time"]}})
}

/// FR-750 "Dispatch" ("settle it by the same rules") with FSpec FR-193's
/// hybrid basis rule: only a `bounded-solver` or `inductive-solver` proof
/// is a hybrid provider's `proved`, and only that settles `proved`
/// (`trusted`); every other result an entry can spell is refuted after
/// replay, `incomplete`, `failed` or refused `invalid_result`, never
/// settled as the entry says.
#[test]
fn only_a_solver_basis_proof_settles_proved_and_it_is_trusted() {
    let refused = |pointer: &str| ("failed", Some(("invalid_result", pointer.to_owned())));
    let rows: Vec<(&str, Value, &str, Option<(&str, String)>)> = vec![
        (
            "solver proof",
            solver_proof("10", "trusted"),
            "proved",
            None,
        ),
        (
            "solver proof claiming uncertified",
            solver_proof("10", "uncertified"),
            refused("/certification").0,
            refused("/certification").1,
        ),
        (
            "solver proof claiming certified",
            solver_proof("10", "certified"),
            refused("/certification").0,
            refused("/certification").1,
        ),
        (
            "short horizon",
            solver_proof("1", "trusted"),
            "inconclusive",
            Some(("insufficient-horizon", String::new())),
        ),
        (
            "exhaustive basis",
            json!({"result": "proved", "certification": "trusted",
                "basis": {"type": "exhaustive"}}),
            refused("/basis/type").0,
            refused("/basis/type").1,
        ),
        (
            "bounded-complete basis",
            json!({"result": "proved", "certification": "trusted",
                "basis": {"type": "bounded-complete", "depth": 1}}),
            refused("/basis/type").0,
            refused("/basis/type").1,
        ),
        (
            "tested",
            json!({"result": "tested", "certification": "trusted"}),
            refused("/result").0,
            refused("/result").1,
        ),
        (
            "measured",
            json!({"result": "measured", "certification": "trusted"}),
            refused("/result").0,
            refused("/result").1,
        ),
        (
            "declined",
            json!({"result": "declined"}),
            refused("/result").0,
            refused("/result").1,
        ),
        (
            "refuted by a trace replay rejects",
            json!({"result": "refuted", "counterexample": 0}),
            "inconclusive",
            Some(("replay-parity", String::new())),
        ),
        (
            "failed",
            json!({"result": "failed", "cause": {"type": "tool-failure", "defect": "crash"}}),
            "failed",
            Some(("tool-failure", String::new())),
        ),
    ];
    for (name, entry, result, marker) in rows {
        let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
        let ws = Workspace::new("settlement-table");
        let counterexamples = if entry["result"] == "refuted" {
            vec![json!({"type": "hybrid-counterexample", "kind": "formula",
                "trace": point_trace("1")})]
        } else {
            Vec::new()
        };
        // A refuted trace that replays would settle `refuted`: this claim
        // holds, so the trace never reproduces.
        let script = answer(&outcome(entry, &counterexamples));
        let engine = process_engine(&ws, &script, limits());
        let item = run(&ws, &unit, named("SlowEnough"), vec![engine], limits());
        assert_eq!(item["record"]["result"], result, "{name}: {item}");
        let text = item.to_string();
        match marker {
            Some((marker, pointer)) => {
                assert!(text.contains(marker), "{name}: {marker}: {item}");
                assert!(text.contains(&pointer), "{name}: {pointer}: {item}");
            }
            None => {}
        }
        if result == "proved" {
            assert_eq!(item["record"]["certification"], "trusted", "{name}: {item}");
        } else {
            assert!(
                item["record"]
                    .get("certification")
                    .is_none_or(Value::is_null),
                "{name}: nothing but a solver proof is certified: {item}"
            );
        }
    }
}

/// FSpec FR-462 "Cancellation": a request cancelled while the provider
/// works is told so, and when the provider ends its answer a late `proved`
/// the host still settles the item `incomplete`, `cancelled`.
#[test]
fn a_cancelled_process_item_ignores_a_late_proof() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let ws = Workspace::new("cancel-late");
    let proof = outcome(solver_proof("10", "trusted"), &[]);
    let script = json!({"mode": "late-result", "outcome": proof});
    let mut run_limits = limits();
    run_limits.grace_ms = 5_000;
    let engine = process_engine(&ws, &script, run_limits);
    let cancel = filament_exact::Cancel::new();
    let trip = cancel.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        trip.cancel(filament_exact::CancelCause::Requested);
    });
    let item = run_with_cancel(
        &ws,
        &unit,
        named("SlowEnough"),
        vec![engine],
        run_limits,
        &cancel,
    );
    canceller.join().expect("the canceller ran");
    assert_eq!(item["record"]["result"], "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "cancelled", "{item}");
}

/// FSpec FR-462 "Cancellation": a provider that ignores `cancel` is ended
/// after the grace period and the item settles `incomplete`, `cancelled`;
/// so does a request cancelled before the provider has said hello.
#[test]
fn a_cancelled_process_item_ends_the_provider_after_the_grace() {
    for (name, mode) in [("cancel-hang", "hang"), ("cancel-no-hello", "no-hello")] {
        let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
        let ws = Workspace::new(name);
        let script = json!({"mode": mode});
        let mut run_limits = limits();
        run_limits.grace_ms = 100;
        let engine = process_engine(&ws, &script, run_limits);
        let cancel = filament_exact::Cancel::new();
        let trip = cancel.clone();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            trip.cancel(filament_exact::CancelCause::Requested);
        });
        let item = run_with_cancel(
            &ws,
            &unit,
            named("SlowEnough"),
            vec![engine],
            run_limits,
            &cancel,
        );
        canceller.join().expect("the canceller ran");
        assert_eq!(item["record"]["result"], "incomplete", "{name}: {item}");
        assert_eq!(
            item["record"]["cause"]["kind"], "cancelled",
            "{name}: {item}"
        );
    }
}

/// FSL FR-635 "How a process provider runs an item": the request carries the
/// unit's dependency closure as `dependencies`, each package by identity, and
/// a provider that requires it settles the item.
#[test]
#[trace("TC-3035", "FR-750-AC-6")]
fn the_request_carries_the_units_dependency_closure() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)")).replacen(
        "hybrid Braking",
        "import \"test/lib\" as l;\nhybrid Braking",
        1,
    );
    let library = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function Early using v(): Boolean pure { true }\n";
    let ws = Workspace::new("dependencies");
    let recorded = ws.path("request.json");
    let script = json!({"mode": "answer", "record_request": recorded,
        "require_dependencies": ["test/lib"],
        "outcome": outcome(json!({"result": "inconclusive",
            "cause": {"type": "solver-inconclusive"}}), &[])});
    let engine = process_engine(&ws, &script, limits());
    let item = run_with_library(
        &ws,
        &unit,
        ("test/lib", library),
        json!({"claim": "SlowEnough", "backend": BACKEND}),
        vec![engine],
        limits(),
    );
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    let request: Value =
        serde_json::from_slice(&std::fs::read(recorded).expect("the request was recorded"))
            .expect("the request is JSON");
    let dependencies = request["request"]["dependencies"]
        .as_array()
        .expect("the closure is sent");
    assert_eq!(dependencies.len(), 1, "{request}");
    assert_eq!(dependencies[0]["identity"], "test/lib", "{request}");
    assert!(dependencies[0]["package"].is_string(), "{request}");
}

/// FSL FR-635: a unit with no import sends no `dependencies` member.
#[test]
#[trace("TC-3035", "FR-750-AC-6")]
fn a_unit_without_imports_sends_no_dependencies() {
    let unit = braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"));
    let ws = Workspace::new("no-dependencies");
    let recorded = ws.path("request.json");
    let script = json!({"mode": "answer", "record_request": recorded,
        "outcome": outcome(json!({"result": "inconclusive",
            "cause": {"type": "solver-inconclusive"}}), &[])});
    let engine = process_engine(&ws, &script, limits());
    let item = run(
        &ws,
        &unit,
        json!({"claim": "SlowEnough", "backend": BACKEND}),
        vec![engine],
        limits(),
    );
    assert_eq!(item["record"]["result"], "inconclusive", "{item}");
    let request: Value =
        serde_json::from_slice(&std::fs::read(recorded).expect("the request was recorded"))
            .expect("the request is JSON");
    assert!(request["request"].get("dependencies").is_none(), "{request}");
}
