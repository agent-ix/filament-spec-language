// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2472: `fsl analyze <request>` at the command surface the driver calls
//! (`analyze_with_providers` with no configured provider): proved, refuted,
//! incomplete and mixed items, whole-request refusals, cancellation, the
//! byte ceilings and provider limits, and odd input.
//!
//! The Counter domain package is the driver's `analyze-counter` fixture,
//! copied as `fsl-command/tests/fixtures/cli-contract/counter.semantic-ir.json`.

use filament_exact::Cancel;
use fsl_analyze::provider::ProviderOptions;
use fsl_command::command::RunLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::{analyze, analyze_under, Case, INVALID_UTF8};

const MODEL: &[u8] = include_bytes!("../../fixtures/cli-contract/counter.semantic-ir.json");

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile values = \"filament.value.complete/v1\";\n\
    profile paths = \"filament.temporal.infinite-trace/v1\";\n\
    model Queue = \"test/counter\";\n\
    pre SpaceAvailable using values on Queue::Counter::inc { self.value < 2 }\n\
    post AppendOne using values on Queue::Counter::inc { self.value - pre(self.value) = 1 }\n\
    temporal WithinCapacity using paths over (q: Queue::Counter) clock \"model-steps\" on origin {\n\
      always holds(q.value <= 2)\n\
    }\n\
    temporal NeverFull using paths over (q: Queue::Counter) clock \"model-steps\" on origin {\n\
      always holds(q.value < 2)\n\
    }\n";

fn start() -> Value {
    json!({"format": "filament.state.snapshot/v1",
        "identity": {"authority": "contracts", "identity": "queue-start"},
        "observation": "current", "model": {"identity": "test/counter"},
        "anchor": {"kind": "initialization", "name": "queue-start"},
        "populations": [{"population": "ix://test/counter/counters", "complete": true,
            "objects": [{"key": "queue-east", "type": "ix://test/counter/Counter",
                "fields": {"value": {"integer": "0"}}}]}]})
}

fn request(items: Value) -> Value {
    json!({"format": "native-analyze/1", "request": {
        "program": {"source": {"file": "workflow.fsl", "authority": "contracts",
            "identity": "queue-workflow", "document": "QueueWorkflow", "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "counter.json",
            "authority": "contracts", "identity": "counter-reference",
            "document": "CounterInput", "formal_revision": 1}}],
        "documents": [{"file": "start.json"}],
        "items": items}})
}

/// A case holding the workflow, its model and snapshot, and `request`.
fn counter(source: &[u8], request: &Value) -> (Case, std::path::PathBuf) {
    let case = Case::new();
    case.write("workflow.fsl", source);
    case.write("counter.json", MODEL);
    case.json("start.json", &start());
    let path = case.json("analyze.json", request);
    (case, path)
}

fn results(document: &Value) -> Vec<String> {
    document["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items: {document}"))
        .iter()
        .map(|item| item["record"]["result"].as_str().unwrap_or("?").to_owned())
        .collect()
}

/// FR-621, FR-286-AC-3: a proved item exits 0 with one `analyze` outcome.
#[test]
#[trace("FR-621", "FR-286-AC-3")]
fn a_proved_item_exits_0() {
    let (_case, path) = counter(
        SOURCE.as_bytes(),
        &request(json!([{"claim": "WithinCapacity"}])),
    );
    let outcome = analyze(&path);
    let document = outcome.expect("analyze", "success", 0);
    assert_eq!(results(document), ["proved"]);
}

/// FR-286-AC-14: a refuted item exits 10 with its counterexample.
#[test]
#[trace("FR-286-AC-14", "FR-285-AC-1")]
fn a_refuted_item_exits_10_with_a_counterexample() {
    let (_case, path) = counter(SOURCE.as_bytes(), &request(json!([{"claim": "NeverFull"}])));
    let outcome = analyze(&path);
    let document = outcome.expect("analyze", "violation", 10);
    assert_eq!(results(document), ["refuted"]);
    assert!(
        document["items"][0]["counterexample"].is_object(),
        "{document}"
    );
}

/// FR-620-AC-7 and FR-285-AC-2: an item over its `max_states` budget is
/// incomplete, and a refuted item beside it gives exit 22, not 10.
#[test]
#[trace("FR-620-AC-7", "FR-285-AC-2", "FR-286-AC-3")]
fn an_item_over_budget_is_incomplete_and_outranks_a_violation() {
    let items = json!([{"claim": "NeverFull"},
        {"claim": "WithinCapacity", "temporal": {"limits": {"max_states": 1}}}]);
    let (_case, path) = counter(SOURCE.as_bytes(), &request(items));
    let outcome = analyze(&path);
    assert_eq!(outcome.exit, 22, "{}", outcome.document);
    super::assert_outcome_schema(&outcome.document);
    assert_eq!(results(&outcome.document), ["refuted", "incomplete"]);
}

/// FR-624 (SIGINT/SIGTERM): a handle cancelled before the call settles every
/// item `incomplete`, exit 22.
#[test]
#[trace("FR-624")]
fn a_cancelled_run_settles_every_item_incomplete() {
    let items = json!([{"claim": "WithinCapacity"}, {"claim": "NeverFull"}]);
    let (_case, path) = counter(SOURCE.as_bytes(), &request(items));
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    let outcome = analyze_under(
        &path,
        RunLimits::default(),
        &ProviderOptions::default(),
        &cancel,
    );
    assert_eq!(outcome.exit, 22, "{}", outcome.document);
    super::assert_outcome_schema(&outcome.document);
    assert_eq!(results(&outcome.document), ["incomplete", "incomplete"]);
}

/// FR-621-AC-10: a claim the program lacks settles as one `invalid-request`
/// item (`record.settled_by` `disposition`), the document is a refusal, exit
/// 20, and the aggregate names the gap.
#[test]
#[trace("FR-621-AC-10")]
fn an_unknown_claim_is_an_invalid_request_item() {
    let (_case, path) = counter(SOURCE.as_bytes(), &request(json!([{"claim": "Absent"}])));
    let outcome = analyze(&path);
    let document = outcome.expect("analyze", "refusal", 20);
    assert_eq!(
        document["items"].as_array().map(Vec::len),
        Some(1),
        "{document}"
    );
    assert_eq!(
        document["items"][0]["record"]["settled_by"], "disposition",
        "{document}"
    );
    assert!(
        document["items"][0]["record"].get("backend").is_none(),
        "{document}"
    );
    assert_eq!(
        document["aggregate"]["gaps"][0]["result"], "invalid-request",
        "{document}"
    );
}

/// FR-620-AC-9, FR-624: a run request handed to analyze and an unknown
/// request member each refuse the whole request, exit 20, `invalid-request`.
#[test]
#[trace("FR-620-AC-9", "FR-624")]
fn whole_request_defects_refuse() {
    let mut wrong_format = request(json!([{"claim": "WithinCapacity"}]));
    wrong_format["format"] = json!("native-run/1");
    let mut unknown = request(json!([{"claim": "WithinCapacity"}]));
    unknown["request"]["extra"] = json!(true);
    for document in [wrong_format, unknown] {
        let (_case, path) = counter(SOURCE.as_bytes(), &document);
        let outcome = analyze(&path);
        outcome.refused("analyze", 20, "invalid-request");
    }
}

/// FR-286-AC-19: an ill-typed program refuses `ill_typed`, exit 20.
#[test]
#[trace("FR-286-AC-19")]
fn an_ill_typed_program_refuses() {
    let source = SOURCE.replace("q.value <= 2", "q.value + true");
    let (_case, path) = counter(
        source.as_bytes(),
        &request(json!([{"claim": "WithinCapacity"}])),
    );
    let outcome = analyze(&path);
    let diagnostic = outcome.refused("analyze", 20, "ill_typed");
    assert!(diagnostic["details"].is_null(), "{diagnostic}");
}

/// FR-624-AC-6: the request byte ceiling one byte short stops at intake, exit 22
/// naming `request_bytes` and `--request-bytes`; at the exact size the request
/// runs.
#[test]
#[trace("FR-624-AC-6")]
fn request_byte_ceiling_is_reached_and_raised() {
    let (_case, path) = counter(
        SOURCE.as_bytes(),
        &request(json!([{"claim": "WithinCapacity"}])),
    );
    let size = std::fs::metadata(&path).unwrap().len() as usize;
    let limits = |request_bytes| RunLimits {
        request_bytes,
        ..RunLimits::default()
    };
    let over = analyze_under(
        &path,
        limits(size - 1),
        &ProviderOptions::default(),
        &Cancel::new(),
    );
    over.expect("analyze", "incomplete", 22);
    let text = over.document.to_string();
    assert!(
        text.contains("request_bytes") && text.contains("--request-bytes"),
        "{text}"
    );
    let fits = analyze_under(
        &path,
        limits(size),
        &ProviderOptions::default(),
        &Cancel::new(),
    );
    fits.expect("analyze", "success", 0);
}

/// `--provider-limit` with no provider configured changes no FSL-settled
/// item (FR-964: limits are carried by provider requests only).
#[test]
#[trace("FR-964")]
fn provider_limits_without_a_provider_change_nothing() {
    let (_case, path) = counter(SOURCE.as_bytes(), &request(json!([{"claim": "NeverFull"}])));
    let mut options = ProviderOptions::default();
    options.provider_limits.max_frame_bytes = 1;
    options.provider_limits.deadline_ms = 1;
    let limited = analyze_under(&path, RunLimits::default(), &options, &Cancel::new());
    let plain = analyze(&path);
    assert_eq!(limited.exit, plain.exit);
    assert_eq!(limited.bytes, plain.bytes);
}

/// FR-001: invalid UTF-8 in the program, and a missing, a directory and an
/// empty request operand, each refuse, exit 20.
#[test]
#[trace("FR-001-AC-3", "FR-001")]
fn odd_input_refuses() {
    let (case, path) = counter(INVALID_UTF8, &request(json!([{"claim": "WithinCapacity"}])));
    analyze(&path).expect("analyze", "refusal", 20);
    std::fs::create_dir_all(case.path("folder")).unwrap();
    for path in [
        case.path("absent.json"),
        case.path("folder"),
        case.write("empty.json", b""),
    ] {
        let outcome = analyze(&path);
        outcome.expect("analyze", "refusal", 20);
        assert_eq!(
            outcome.document["diagnostics"].as_array().map(Vec::len),
            Some(1),
            "{}",
            outcome.document
        );
    }
}
