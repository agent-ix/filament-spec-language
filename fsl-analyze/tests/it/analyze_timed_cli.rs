// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-632 and FR-626 (TC-1592, TC-1586) over the timed `Rpc` subject through
//! `fsl analyze`: MITL claims decided on zones with a certificate, derived
//! deadlock-freedom and time-lock-freedom items, budgets, and the exact
//! rational delays of a refutation.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};
use crate::analyze_family_cli::{rpc_snapshot, rpc_unit, rpc_unit_with};

/// ADR-026 section 11's `Settles`: every `Waiting` position is followed within
/// `interval` by a position that is not `Waiting`.
fn settles(interval: &str) -> String {
    format!(
        "temporal Settles using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ always (holds(c.phase = 1) implies eventually{interval} holds(not (c.phase = 1))) }}\n"
    )
}

fn run(name: &str, unit: &str, items: Value) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": items}),
    ));
    let items = run.document()["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    (run, items)
}

/// FR-632-AC-1: the MITL claim proves with basis `zone-certified` and
/// certification `certified` when the reply fits its interval, and refutes
/// with a counterexample whose delays are exact rationals when the interval
/// is cut short; the counterexample replays.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn an_mitl_claim_proves_certified_and_refutes_with_exact_delays() {
    let (run, items) = run(
        "timed-mitl",
        &rpc_unit(
            4,
            &format!(
                "{}{}",
                settles("[0 ms, 3 ms]").replace("Settles", "Within"),
                settles("[0 ms, 3 ms)").replace("Settles", "Short")
            ),
        ),
        json!([{"claim": "Within"}, {"claim": "Short"}]),
    );
    assert_eq!(label(&items[0]), "proved");
    assert_eq!(items[0]["record"]["basis"]["type"], "zone-certified");
    assert_eq!(items[0]["record"]["certification"], "certified");
    assert_eq!(label(&items[1]), "refuted");
    assert_eq!(items[1]["replay"]["reproduced"], true);
    let steps = items[1]["counterexample"]["prefix"]
        .as_array()
        .expect("a finite prefix");
    assert!(
        steps
            .iter()
            .any(|step| step["delay"].is_object() || step["delay"].is_string()),
        "the steps carry exact delays: {}",
        items[1]["counterexample"]
    );
    assert_eq!(run.exit, 10);
}

/// FR-626-AC-3: the derived time-lock-freedom item of the timed subject
/// proves when every reachable state lets time diverge, and refutes with a
/// concretised, replayed counterexample when a strict guard stalls it.
#[trace("TC-1586", "FR-626-AC-3")]
#[test]
fn time_lock_freedom_proves_and_refutes_over_a_timed_subject() {
    let item = json!([{"claim": "M", "temporal": {"item_kind": "time-lock-freedom"}}]);
    let (_, items) = run("timed-lock-free", &rpc_unit(4, ""), item.clone());
    assert_eq!(label(&items[0]), "proved");
    assert_eq!(items[0]["record"]["certification"], "certified");
    let (run, items) = run("timed-lock", &rpc_unit_with(3, "> 3", ">", ""), item);
    assert_eq!(label(&items[0]), "refuted");
    assert_eq!(items[0]["counterexample"]["kind"], "time-lock");
    assert_eq!(items[0]["replay"]["reproduced"], true);
    assert_eq!(run.exit, 10);
}

/// FR-626: the derived deadlock-freedom item of the timed subject is decided
/// on zones as well.
#[trace("TC-1586", "FR-626-AC-1")]
#[test]
fn deadlock_freedom_is_decided_over_a_timed_subject() {
    let (_, items) = run(
        "timed-deadlock",
        &rpc_unit(4, ""),
        json!([{"claim": "M", "temporal": {"item_kind": "deadlock-freedom"}}]),
    );
    assert!(
        matches!(label(&items[0]), "proved" | "refuted"),
        "{}",
        items[0]
    );
    assert_eq!(items[0]["request"]["candidates"]["backends"][0], "en-6");
}

/// FR-625-AC-3 on zones: a symbolic-state budget returns `incomplete` naming
/// the limit, never a verdict.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn a_zone_state_budget_is_incomplete() {
    let (run, items) = run(
        "timed-budget",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        json!([{"claim": "Settles", "temporal": {"limits": {"max_states": 2}}}]),
    );
    assert_eq!(label(&items[0]), "incomplete");
    assert_eq!(run.exit, 22);
}

/// QSL-2129: a clock bound of the wrong dimension (a bare number against a
/// `Time` clock) is refused at check, before any item runs.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn a_wrong_dimension_clock_bound_is_refused_at_check() {
    let unit = rpc_unit(4, "").replace("self.x <= 3 ms", "self.x <= 3");
    let ws = Workspace::new("timed-dimension");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &crate::zone_search::rpc_document(),
        &[rpc_snapshot()],
        json!({"items": []}),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["items"], json!([]));
    assert_ne!(run.document()["last_stage"], Value::Null);
}
