// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-960 to FR-964 (TC-2700, TC-2701, TC-2703, TC-2704): a run call names
//! the backends of its requirement items, `run_with_providers` and plain
//! `command::run` settle them, and the run outcome reports them.

use std::path::{Path, PathBuf};

use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SQ: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }\n\
    function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n\
    function seven using v(): Int[0, 9] pure { 7 }\n";

const PROVIDER: &str = "fsl.test-recording-provider";

fn source() -> Value {
    json!({"file": "unit.native", "authority": "agent-ix", "identity": "test:requirements",
        "document": "Requirements", "formal_revision": 1})
}

/// A native-run/1 request calling `function` with `arguments` and, when
/// given, `requirements`.
fn write(
    directory: &Path,
    function: &str,
    arguments: Value,
    requirements: Option<Value>,
) -> PathBuf {
    std::fs::write(directory.join("unit.native"), SQ).unwrap();
    let mut call = json!({"function": function, "arguments": arguments});
    if let Some(requirements) = requirements {
        call["requirements"] = requirements;
    }
    let request = json!({"format": "native-run/1",
        "request": {"models": [], "program": {"source": source()}, "call": call}});
    let path = directory.join("run.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn sq(directory: &Path, requirements: Option<Value>) -> PathBuf {
    write(
        directory,
        "sq",
        json!([{"parameter": "x", "value": 3}]),
        requirements,
    )
}

/// The `spine-run-result/1` document of a run that completed, and its
/// `execute` outcome document.
fn documents(result: &command::RunResult) -> (Value, Value) {
    let value: Value = serde_json::from_str(&result.value.to_string()).expect("a JSON result");
    let outcome = result
        .outcome
        .as_ref()
        .map(|document| {
            serde_json::from_slice(&document.to_bytes().expect("the outcome serializes"))
                .expect("one JSON document")
        })
        .unwrap_or(Value::Null);
    (value, outcome)
}

fn entries(document: &Value) -> Vec<Value> {
    document["requirements"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("a `requirements` member: {document}"))
}

/// FR-960-AC-1, FR-960-AC-2, FR-960-AC-4, FR-963-AC-1, FR-963-AC-3,
/// FR-963-AC-4: `call.requirements` asks for settlement and names each
/// item's backend; the three outcome forms report the items.
#[trace("TC-2700", "FR-960-AC-1")]
#[trace("TC-2700", "FR-960-AC-2")]
#[trace("TC-2700", "FR-960-AC-4")]
#[trace("TC-2703", "FR-963-AC-1")]
#[trace("TC-2703", "FR-963-AC-3")]
#[trace("TC-2703", "FR-963-AC-4")]
#[test]
#[ignore = "QSL-2192: a run call does not read `requirements` or settle its requirement items yet"]
fn a_run_call_names_the_backend_of_its_requirement_items() {
    let directory = tempfile::tempdir().unwrap();
    // Settlement asked for, no backend named: three items on value-checker.
    let result = command::run(&sq(directory.path(), Some(json!({})))).expect("the run completes");
    let (value, outcome) = documents(&result);
    assert_eq!(
        value["outcome"],
        json!({"kind": "completed", "value": {"integer": "16"}})
    );
    let items = entries(&value);
    assert_eq!(items.len(), 3, "{value}");
    for item in &items {
        assert_eq!(item["claim_name"], "sq", "{item}");
        assert!(item["request"].get("backend").is_none(), "{item}");
        assert_eq!(
            item["request"]["candidates"]["backends"],
            json!(["value-checker"]),
            "{item}"
        );
        assert_eq!(item["record"]["result"], "proved", "{item}");
        assert_eq!(item["record"]["backend"], "value-checker", "{item}");
    }
    assert!(
        value["registry"].as_array().is_some_and(|r| !r.is_empty()),
        "{value}"
    );
    assert_eq!(
        outcome["items"],
        json!(items),
        "the execute items equal the entries"
    );
    assert_eq!(result.exit_code, 0);
    let first_claim = items[0]["request"]["claim"].clone();

    // The call-level backend names value-checker for every item.
    let named = documents(
        &command::run(&sq(
            directory.path(),
            Some(json!({"backend": "value-checker"})),
        ))
        .unwrap(),
    )
    .0;
    for item in entries(&named) {
        assert_eq!(item["request"]["backend"], "value-checker", "{item}");
    }

    // An unregistered backend settles each item unknown-backend; a per-item
    // entry with no backend falls back to no named backend for its item.
    let absent = command::run(&sq(
        directory.path(),
        Some(json!({"backend": "fsl.absent", "items": [{"claim": first_claim}]})),
    ))
    .unwrap();
    assert_eq!(absent.exit_code, 20, "category refusal");
    let absent_items = entries(&documents(&absent).0);
    assert_eq!(
        absent_items[0]["record"]["backend"], "value-checker",
        "{}",
        absent_items[0]
    );
    for item in &absent_items[1..] {
        assert!(item.to_string().contains("unknown-backend"), "{item}");
        assert!(item["record"].get("backend").is_none(), "{item}");
        assert!(item["record"].get("cause").is_none(), "{item}");
    }
    assert_eq!(
        documents(&absent).0["outcome"]["kind"],
        "completed",
        "the call's value is unchanged"
    );

    // An unpublished option refuses each item and leaves the value.
    let option = documents(
        &command::run(&sq(
            directory.path(),
            Some(json!({"backend": "value-checker", "options": {"unknown": "1"}})),
        ))
        .unwrap(),
    )
    .0;
    for item in entries(&option) {
        assert!(item.to_string().contains("unknown"), "{item}");
        assert!(
            item["record"].get("result").is_none(),
            "an invalid-request record: {item}"
        );
    }

    // Without `requirements`, no member and no items.
    let plain = command::run(&write(directory.path(), "seven", json!([]), None)).unwrap();
    let (value, outcome) = documents(&plain);
    assert!(
        value.get("requirements").is_none() && value.get("registry").is_none(),
        "{value}"
    );
    assert_eq!(outcome["items"], json!([]));
}

/// FR-960-AC-3: malformed `requirements` refuse at stage `request`, and a
/// claim of no item of the call at stage `call` with `missing_declaration`.
#[trace("TC-2700", "FR-960-AC-3")]
#[test]
#[ignore = "QSL-2192: a run call does not read `requirements` yet"]
fn malformed_requirements_refuse_before_the_call() {
    let directory = tempfile::tempdir().unwrap();
    let claim = json!({"node": {"package": "x", "path": [], "ordinal": 0}, "occurrence": 0});
    for (requirements, pointer) in [
        (json!([]), "/call/requirements"),
        (json!({"backend": ""}), "/call/requirements/backend"),
        (json!({"options": {"k": "v"}}), "/call/requirements/options"),
        (
            json!({"backend": "value-checker", "options": {"k": 1}}),
            "/call/requirements/options/k",
        ),
        (json!({"extra": 1}), "/call/requirements/extra"),
        (json!({"items": [{}]}), "/call/requirements/items/0"),
        (
            json!({"items": [{"claim": claim}, {"claim": claim}]}),
            "/call/requirements/items/1",
        ),
    ] {
        let error = command::run(&sq(directory.path(), Some(requirements.clone())))
            .expect_err("a malformed member refuses");
        let envelope = error.value().expect("refusal envelope");
        assert_eq!(envelope["stage"], "request", "{requirements}: {envelope}");
        assert!(
            envelope.to_string().contains(pointer),
            "{requirements}: {envelope}"
        );
        assert_eq!(error.category().exit_code(), 20);
    }
    let error = command::run(&sq(
        directory.path(),
        Some(json!({"items": [{"claim": claim}]})),
    ))
    .expect_err("a claim of no item of the call refuses");
    let envelope = error.value().expect("refusal envelope");
    assert_eq!(envelope["stage"], "call", "{envelope}");
    assert!(
        envelope.to_string().contains("missing_declaration"),
        "{envelope}"
    );
}

/// FR-961-AC-2: plain `command::run` settles the items with the linked
/// engines alone: an item named for an unregistered provider settles
/// unknown-backend, and the call's value is unchanged.
#[trace("TC-2701", "FR-961-AC-2")]
#[test]
#[ignore = "QSL-2192: plain command::run does not settle a call's requirement items yet"]
fn plain_run_settles_with_the_linked_engines_only() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let result = command::run(&request).expect("the run completes");
    let (value, _) = documents(&result);
    assert_eq!(
        value["outcome"],
        json!({"kind": "completed", "value": {"boolean": true}})
    );
    let items = entries(&value);
    assert!(!items.is_empty(), "Even has requirement items: {value}");
    for item in items {
        assert!(item.to_string().contains("unknown-backend"), "{item}");
        assert!(item.to_string().contains(PROVIDER), "{item}");
    }
}

/// What `run_with_providers` returned and what its recording provider saw.
struct ProviderRun {
    document: Value,
    settle_calls: usize,
    negotiations: usize,
    deadline_ms: u64,
    max_frame_bytes: u64,
}

/// `fsl_command::command::run_with_providers(request, negotiate, providers,
/// options, cancel)` (FR-961) with a recording negotiator and the recording
/// provider under `options`. The entry and `ProviderOptions` (FR-964) do not
/// exist yet, so this returns `None`; the code ticket replaces the body with
/// the call.
fn run_with_providers(_request: &Path, _scenario: Scenario) -> Option<ProviderRun> {
    None
}

/// How the recording provider and the run are set up.
#[allow(
    dead_code,
    reason = "read by the run_with_providers call the code ticket adds"
)]
enum Scenario {
    /// One recording provider under these limits.
    Limits {
        deadline_ms: u64,
        max_frame_bytes: u64,
    },
    /// Two providers with one `BackendId`.
    RepeatedProvider,
    /// The provider reports `refuted` at `n = 4`, which the evaluator does not violate.
    UnreproducedRefutation,
    /// The handle is cancelled before the run.
    Cancelled,
}

/// FR-961-AC-1, FR-961-AC-3, FR-963-AC-2, FR-964-AC-1, FR-964-AC-2: the run
/// entry negotiates every item, then calls the provider once with every item
/// under the run's limits, and reports each item proved on it.
#[trace("TC-2701", "FR-961-AC-1")]
#[trace("TC-2701", "FR-961-AC-3")]
#[trace("TC-2703", "FR-963-AC-2")]
#[trace("TC-2704", "FR-964-AC-1")]
#[trace("TC-2704", "FR-964-AC-2")]
#[test]
#[ignore = "QSL-2192: run_with_providers and ProviderOptions do not exist yet"]
fn run_with_providers_settles_the_call_s_items_after_negotiation() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let run = run_with_providers(
        &request,
        Scenario::Limits {
            deadline_ms: 250,
            max_frame_bytes: 1_048_576,
        },
    )
    .expect("the run-side provider entry");
    let items = entries(&run.document);
    assert_eq!(run.negotiations, items.len(), "one negotiation per item");
    assert_eq!(run.settle_calls, 1, "one request for the one provider");
    assert_eq!(run.deadline_ms, 250);
    assert_eq!(run.max_frame_bytes, 1_048_576);
    assert_eq!(
        run.document["outcome"],
        json!({"kind": "completed", "value": {"boolean": true}})
    );
    for item in items {
        assert_eq!(item["record"]["result"], "proved", "{item}");
        assert_eq!(item["record"]["backend"], PROVIDER, "{item}");
        assert_eq!(item["record"]["certification"], "trusted", "{item}");
    }
    assert!(run.document["registry"].to_string().contains(PROVIDER));

    // Without `requirements`, neither the negotiator nor the provider runs.
    let bare = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        None,
    );
    let run = run_with_providers(
        &bare,
        Scenario::Limits {
            deadline_ms: 250,
            max_frame_bytes: 1_048_576,
        },
    )
    .expect("the run-side provider entry");
    assert_eq!((run.negotiations, run.settle_calls), (0, 0));
}

/// FR-961-AC-4, FR-961-AC-5: negative cases of the run-side entry: a repeated
/// provider identity, a refused compile, a counterexample replay does not
/// reproduce, and a cancelled handle.
#[trace("TC-2701", "FR-961-AC-4")]
#[trace("TC-2701", "FR-961-AC-5")]
#[test]
#[ignore = "QSL-2192: run_with_providers does not exist yet"]
fn run_with_providers_refuses_and_settles_the_negative_cases() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let repeated =
        run_with_providers(&request, Scenario::RepeatedProvider).expect("the run-side entry");
    assert!(
        repeated.document.to_string().contains("providers"),
        "a repeated BackendId refuses naming `providers`: {}",
        repeated.document
    );
    assert_eq!((repeated.negotiations, repeated.settle_calls), (0, 0));

    let parity =
        run_with_providers(&request, Scenario::UnreproducedRefutation).expect("the run-side entry");
    for item in entries(&parity.document) {
        assert_eq!(item["record"]["result"], "inconclusive", "{item}");
        assert!(item.to_string().contains("replay-parity"), "{item}");
    }

    let cancelled = run_with_providers(&request, Scenario::Cancelled).expect("the run-side entry");
    assert_eq!(cancelled.settle_calls, 0, "no request after cancellation");
    for item in entries(&cancelled.document) {
        assert_eq!(item["record"]["result"], "incomplete", "{item}");
        assert!(item.to_string().contains("cancelled"), "{item}");
    }

    std::fs::write(directory.path().join("unit.native"), "not a program").unwrap();
    let refused = run_with_providers(
        &request,
        Scenario::Limits {
            deadline_ms: 600_000,
            max_frame_bytes: 67_108_864,
        },
    )
    .expect("the run-side entry");
    assert_eq!(
        (refused.negotiations, refused.settle_calls),
        (0, 0),
        "a refused compile settles no item"
    );
}

/// FR-962-AC-4: with the recording provider (advertising `value-validity`)
/// registered, an unnamed `sq` item has two candidates and no tie row, so it
/// settles `ambiguous-backend`; naming `value-checker` settles it there.
#[trace("TC-2702", "FR-962-AC-4")]
#[test]
#[ignore = "QSL-2192: run_with_providers does not exist yet"]
fn an_unnamed_run_item_with_two_value_validity_candidates_is_ambiguous() {
    let directory = tempfile::tempdir().unwrap();
    let limits = || Scenario::Limits {
        deadline_ms: 600_000,
        max_frame_bytes: 67_108_864,
    };
    let unnamed = run_with_providers(&sq(directory.path(), Some(json!({}))), limits())
        .expect("the run-side entry");
    for item in entries(&unnamed.document) {
        let text = item.to_string();
        assert!(text.contains("ambiguous-backend"), "{item}");
        assert!(
            text.contains("value-checker") && text.contains(PROVIDER),
            "{item}"
        );
    }
    let named = run_with_providers(
        &sq(directory.path(), Some(json!({"backend": "value-checker"}))),
        limits(),
    )
    .expect("the run-side entry");
    for item in entries(&named.document) {
        assert_eq!(item["record"]["backend"], "value-checker", "{item}");
    }
}
