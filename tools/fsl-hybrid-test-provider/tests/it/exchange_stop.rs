// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: typed exchange stops, cancellation precedence and strict bytes.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::analyze::{BackendTool, Engine, HybridExchangeError, HybridProcessExchange};
use fsl_replay::ToolFailure;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::host::StdioHost;
use crate::support::{
    self, braking, claim, limits, outcome, rational, run_with_cancel, Workspace, BACKEND, PROVIDER,
};

#[derive(Debug)]
struct Exchange {
    result: Result<Vec<u8>, HybridExchangeError>,
    cancel: Option<CancelCause>,
    calls: Arc<AtomicUsize>,
}

impl HybridProcessExchange for Exchange {
    fn exchange(&self, _: &[u8], cancel: &Cancel) -> Result<Vec<u8>, HybridExchangeError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if let Some(source) = self.cancel {
            cancel.cancel(source);
        }
        self.result.clone()
    }
}

fn engine(
    id: &str,
    result: Result<Vec<u8>, HybridExchangeError>,
    cancel: Option<CancelCause>,
) -> (Engine, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let engine = Engine::hybrid_process(
        support::descriptor(id),
        BackendTool::new("exchange-stop", "1").expect("tool identity"),
        Box::new(Exchange {
            result,
            cancel,
            calls: calls.clone(),
        }),
    )
    .expect("process descriptor");
    (engine, calls)
}

fn proof() -> Value {
    outcome(
        json!({"result": "proved", "certification": "trusted", "basis": {
        "type": "bounded-solver", "method": "delta-complete", "horizon": rational("10"),
        "jumps": 1, "exhaustive": ["time"]}}),
        &[],
    )
}

fn unit() -> String {
    braking(&claim("SlowEnough", "always [0, 10] holds(h.v < 30)"))
}

fn settle(
    result: Result<Vec<u8>, HybridExchangeError>,
    trip: Option<CancelCause>,
    cancel: &Cancel,
) -> Value {
    let ws = Workspace::new("exchange-stop");
    let (engine, calls) = engine(BACKEND, result, trip);
    let item = run_with_cancel(
        &ws,
        &unit(),
        json!({"claim": "SlowEnough", "backend": BACKEND}),
        vec![engine],
        limits(),
        cancel,
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    item
}

fn assert_stop(item: &Value, kind: &str) {
    assert_eq!(item["record"]["result"], "incomplete", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], kind, "{item}");
    for field in [
        "counterexample",
        "replay",
        "witness",
        "certificate",
        "certification",
        "basis",
    ] {
        assert!(
            item.get(field).is_none_or(Value::is_null),
            "{field}: {item}"
        );
        assert!(
            item["record"].get(field).is_none_or(Value::is_null),
            "record {field}: {item}"
        );
    }
}

#[test]
#[trace("FR-750-AC-17", "FR-750-AC-18")]
fn exchange_stop_timeout_keeps_caller_live_and_runs_other_backend() {
    let ws = Workspace::new("independent-backends");
    let cancel = Cancel::new();
    let (a, calls_a) = engine("a", Err(HybridExchangeError::TimedOut), None);
    let mut response = proof();
    response["results"][0]["backend"] = json!("b");
    let (b, calls_b) = engine("b", Ok(response.to_string().into_bytes()), None);
    let document = run_with_cancel(
        &ws,
        &braking(
            &(claim("SlowEnough", "always [0, 10] holds(h.v < 30)")
                + &claim("AlsoEnough", "always [0, 10] holds(h.v < 30)")),
        ),
        json!([
            {"claim": "SlowEnough", "backend": "a"},
            {"claim": "AlsoEnough", "backend": "b"}
        ]),
        vec![a, b],
        limits(),
        &cancel,
    );
    let items = document["items"].as_array().expect("two outcome items");
    assert_eq!(items.len(), 2, "{document}");
    assert_stop(&items[0], "timed-out");
    assert_eq!(items[0]["record"]["backend"], "a");
    assert_eq!(items[1]["record"]["result"], "proved", "{document}");
    assert_eq!(calls_a.load(Ordering::Relaxed), 1);
    assert_eq!(calls_b.load(Ordering::Relaxed), 1);
    assert_eq!(cancel.cause(), None);
}

#[test]
#[trace("FR-750-AC-19")]
fn exchange_stop_preserves_requested_and_caller_deadline_sources() {
    for source in [CancelCause::Requested, CancelCause::Deadline] {
        let item = settle(
            Err(HybridExchangeError::Cancelled { source }),
            Some(source),
            &Cancel::new(),
        );
        assert_stop(&item, "cancelled");
        assert_eq!(
            item["record"]["cause"]["source"],
            match source {
                CancelCause::Requested => "requested",
                CancelCause::Deadline => "deadline",
            }
        );
    }
}

#[test]
#[trace("FR-750-AC-20", "FR-750-AC-23")]
fn exchange_stop_cancellation_wins_over_timeout_and_late_proof() {
    for result in [
        Err(HybridExchangeError::TimedOut),
        Ok(proof().to_string().into_bytes()),
    ] {
        let item = settle(result, Some(CancelCause::Requested), &Cancel::new());
        assert_stop(&item, "cancelled");
        assert_eq!(item["record"]["cause"]["source"], "requested");
    }
    let cancel = Cancel::new();
    let item = settle(Ok(proof().to_string().into_bytes()), None, &cancel);
    cancel.cancel(CancelCause::Requested);
    assert_eq!(item["record"]["result"], "proved", "{item}");
}

#[test]
#[trace("FR-750-AC-21")]
fn exchange_stop_preserves_genuine_tool_defects_and_byte_payloads() {
    for (failure, defect) in [
        (ToolFailure::Crash, "crash"),
        (
            ToolFailure::FrameLength {
                limit: 1048576,
                value: 1048577,
            },
            "frame-length",
        ),
    ] {
        let item = settle(
            Err(HybridExchangeError::ToolFailure(failure)),
            None,
            &Cancel::new(),
        );
        assert_eq!(item["record"]["result"], "failed", "{item}");
        assert_eq!(item["record"]["cause"]["kind"], "tool-failure", "{item}");
        assert_eq!(item["record"]["cause"]["defect"], defect, "{item}");
        if defect == "frame-length" {
            assert_eq!(item["record"]["cause"]["limit"], 1048576);
            assert_eq!(item["record"]["cause"]["value"], 1048577);
        }
    }
}

#[test]
#[trace("FR-750-AC-22")]
fn exchange_stop_successful_bytes_keep_strict_provider_readers() {
    let live = Cancel::new();
    assert_eq!(
        settle(Ok(proof().to_string().into_bytes()), None, &live)["record"]["result"],
        "proved"
    );
    let timed = outcome(
        json!({"result": "incomplete", "cause": {"type": "timed-out"}}),
        &[],
    );
    assert_stop(
        &settle(Ok(timed.to_string().into_bytes()), None, &live),
        "timed-out",
    );
    let malformed = settle(Ok(b"not-json".to_vec()), None, &live);
    assert_eq!(
        malformed["record"]["cause"]["defect"], "malformed-frame",
        "{malformed}"
    );
    let mut bad = proof();
    bad["results"][0]["basis"]["horizon"] = json!("invalid");
    let malformed = settle(Ok(bad.to_string().into_bytes()), None, &live);
    assert_eq!(malformed["record"]["result"], "failed", "{malformed}");
    assert_eq!(
        malformed["record"]["diagnostic"]["code"], "invalid_result",
        "{malformed}"
    );
    assert_eq!(
        malformed["record"]["diagnostic"]["detail"]["pointer"], "/basis/horizon",
        "{malformed}"
    );
}

#[test]
#[trace("FR-750-AC-23", "FR-750-AC-24")]
fn exchange_stop_real_host_timeout_reaps_without_result_frame() {
    let ws = Workspace::new("typed-timeout");
    let script = ws.write("script.json", br#"{"mode":"hang"}"#);
    let host = StdioHost::new(Path::new(PROVIDER).to_owned(), script, limits());
    let cancel = Cancel::new();
    assert_eq!(
        host.exchange(br#"{"request":{"deadline_ms":50}}"#, &cancel),
        Err(HybridExchangeError::TimedOut)
    );
    assert!(host.reaped.load(Ordering::Acquire));
    assert_eq!(cancel.cause(), None);
}

#[test]
#[trace("FR-750-AC-19", "FR-750-AC-23", "FR-750-AC-24")]
fn exchange_stop_real_host_discards_late_evidence_and_reaps_cancelled_processes() {
    for source in [CancelCause::Requested, CancelCause::Deadline] {
        for mode in ["late-result", "hang"] {
            for response in [proof(), support::refuted(support::point_trace("0")), {
                outcome(
                    json!({"result": "proved", "certification": "trusted",
                    "basis": {"type": "inductive-solver", "method": "inductive-enclosure"},
                    "certificate": {"type": "inductive-enclosure", "boxes": {
                        "Moving": {"v": support::interval("0", "20"), "a": support::interval("1", "2")},
                        "Stopped": {"v": support::interval("0", "0"), "a": support::interval("1", "2")}}}}),
                    &[],
                )
            }] {
                let ws = Workspace::new("typed-cancel");
                let request_seen = ws.path("request.json");
                let script =
                    json!({"mode": mode, "outcome": response, "record_request": request_seen});
                let script = ws.write("script.json", script.to_string().as_bytes());
                let mut bounds = limits();
                bounds.grace_ms = if mode == "late-result" { 5_000 } else { 10 };
                let host = StdioHost::new(Path::new(PROVIDER).to_owned(), script, bounds);
                let reaped = host.reaped.clone();
                let results = host.results.clone();
                let cancel = Cancel::new();
                let trip = cancel.clone();
                let canceller = std::thread::spawn(move || {
                    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    while !request_seen.exists() {
                        assert!(
                            std::time::Instant::now() < until,
                            "provider never received request"
                        );
                        std::thread::yield_now();
                    }
                    trip.cancel(source);
                });
                let engine = Engine::hybrid_process(
                    support::descriptor(BACKEND),
                    BackendTool::new("host", "1").expect("tool"),
                    Box::new(host),
                )
                .expect("process engine");
                let formula = if response["results"][0]["result"] == "refuted" {
                    "always [0, 12] holds(h.v > 0)"
                } else {
                    "always [0, 10] holds(h.v < 30)"
                };
                let document = run_with_cancel(
                    &ws,
                    &braking(&claim("SlowEnough", formula)),
                    json!([{ "claim": "SlowEnough", "backend": BACKEND }]),
                    vec![engine],
                    bounds,
                    &cancel,
                );
                canceller.join().expect("cancellation sent");
                assert_eq!(document["items"].as_array().expect("items").len(), 1);
                assert_stop(&document["items"][0], "cancelled");
                assert_eq!(
                    document["items"][0]["record"]["cause"]["source"],
                    match source {
                        CancelCause::Requested => "requested",
                        CancelCause::Deadline => "deadline",
                    }
                );
                assert!(reaped.load(Ordering::Acquire));
                assert_eq!(
                    results.load(Ordering::Relaxed),
                    usize::from(mode == "late-result"),
                    "the late result was observed before settlement"
                );
            }
        }
    }
}
