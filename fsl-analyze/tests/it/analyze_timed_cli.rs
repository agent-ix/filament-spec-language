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
    let delays: Vec<(String, String)> = steps
        .iter()
        .map(|step| {
            let delay = &step["delay"];
            (
                delay["numerator"].as_str().unwrap_or("-").to_owned(),
                delay["denominator"].as_str().unwrap_or("-").to_owned(),
            )
        })
        .collect();
    let exact: Vec<(String, String)> = [("0", "1"), ("3", "1"), ("0", "1"), ("0", "1"), ("1", "1")]
        .iter()
        .map(|(n, d)| ((*n).to_owned(), (*d).to_owned()))
        .collect();
    assert_eq!(delays, exact, "{}", items[1]["counterexample"]);
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
    assert_eq!(label(&items[0]), "proved", "{}", items[0]);
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

fn run_over(
    name: &str,
    unit: &str,
    document: &[u8],
    items: Value,
) -> (crate::analyze_cli_support::Run, Vec<Value>) {
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        unit,
        document,
        &[rpc_snapshot()],
        json!({"items": items}),
    ));
    let items = run.document()["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    (run, items)
}

/// QSL-2129: a timed subject whose clock owner takes an unbounded `Integer`
/// parameter needs a proof bound, as zone search reports it; the request's
/// subject cannot make the item an engine fault.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn a_timed_subject_with_an_unbounded_root_requires_a_bound() {
    let document = crate::timed_world::document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let (run, items) = run_over(
        "timed-unbounded",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        &document,
        json!([{"claim": "Settles"}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["record"]["diagnostic"]["cause"], "unbounded-extent");
    assert_eq!(run.exit, 21);
}

/// `max_clocks` is read by a timed subject (FSpec FR-415) and echoed in the
/// item's request; on an untimed subject no route reads it, so the item is
/// `unsupported`.
#[trace("TC-1592", "FR-632-AC-3")]
#[test]
fn max_clocks_is_read_on_a_timed_subject_and_unrouted_on_an_untimed_one() {
    let (_, items) = run(
        "timed-max-clocks",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        json!([{"claim": "Settles", "temporal": {"limits": {"max_clocks": 1}}}]),
    );
    assert_eq!(label(&items[0]), "proved", "{}", items[0]);
    let ws = Workspace::new("untimed-max-clocks");
    let untimed = analyze(&crate::analyze_cli_support::counter_request(
        &ws,
        &crate::analyze_cli_support::bounded(),
        json!([{"claim": "Bounded", "temporal": {"limits": {"max_clocks": 1}}}]),
    ));
    assert_eq!(untimed.item(0)["record"]["result"], "unsupported");
}

/// A state-graph claim over a timed subject settles `unsupported`: EN-6
/// lists no state-graph form and EN-1 reads no clocks (QSL-2305).
#[trace("TC-1588", "FR-628-AC-1")]
#[test]
fn a_state_graph_claim_over_a_timed_subject_is_unsupported() {
    let (run, items) = run(
        "timed-state-graph",
        &rpc_unit(
            4,
            "possible Reaches using v over (c: M::Call) { c.phase = 1 }\n",
        ),
        json!([{"claim": "Reaches"}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert!(
        item["record"]["diagnostic"]["detail"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("QSL-2305")),
        "{item}"
    );
    assert_eq!(run.exit, 21);
}

/// An item carrying a member no route reads (`witness_samples` on a clause)
/// settles `unsupported` `unsupported-requested-capability` even where its
/// subject is unbounded: it is never told to add a bound that would still
/// leave it unsupported.
#[trace("TC-1592", "FR-632-AC-1")]
#[test]
fn an_unread_member_settles_unsupported_not_requires_bound() {
    let document = crate::timed_world::document_unbounded(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let (run, items) = run_over(
        "timed-unread",
        &rpc_unit(4, &settles("[0 ms, 3 ms]")),
        &document,
        json!([{"claim": "Settles", "temporal": {"witness_samples": 2}}]),
    );
    let item = &items[0];
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unsupported-requested-capability",
        "{item}"
    );
    assert_eq!(run.exit, 21);
}
