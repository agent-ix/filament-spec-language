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
    process_engine, rational, refuted, run, Workspace, BACKEND, PROVIDER,
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
#[trace("TC-2100", "FR-750-AC-5")]
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
#[trace("TC-2100", "FR-750-AC-4")]
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
#[trace("TC-2100", "FR-750-AC-5")]
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
    assert_eq!(request["items"][0]["options"], json!({"step": "1/16"}));
    assert_eq!(
        request["items"][0]["requirements"],
        json!(["hybrid-dynamics"])
    );
    assert_eq!(request["deadline_ms"], limits().deadline_ms);
    assert!(request["package"].is_string());
    let _ = (interval("0", "1"), PROVIDER);
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
