// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-960 to FR-964 (TC-2700, TC-2701, TC-2703, TC-2704): a run call names
//! the backends of its requirement items, `run_with_providers` and plain
//! `command::run` settle them, and the run outcome reports them.

use std::path::{Path, PathBuf};

use fsl_command::command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::negotiation::Fr290;
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

/// The `execute` document with FR-963 requirement items (FR-621 item spelling,
/// no top-level item `result`) validates against the published schema
/// (FR-286-AC-31).
fn assert_execute_document_valid(document: &Value) {
    let schema: Value = serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("schema compiles");
    let result = compiled.validate(document);
    if let Err(errors) = result {
        let errors: Vec<String> = errors
            .map(|error| format!("{error} at {}", error.instance_path))
            .collect();
        panic!("the execute document fails the published schema: {errors:?}\n{document}");
    }
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
#[trace("TC-3110", "FR-286-AC-31")]
#[test]
fn a_run_call_names_the_backend_of_its_requirement_items() {
    let directory = tempfile::tempdir().unwrap();
    // Settlement asked for, no backend named: three items on value-checker.
    let result =
        command::run(&sq(directory.path(), Some(json!({}))), &Fr290).expect("the run completes");
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
    assert_execute_document_valid(&outcome);
    assert_eq!(result.exit_code, 0);
    let first_claim = items[0]["request"]["claim"].clone();

    // The call-level backend names value-checker for every item.
    let named = documents(
        &command::run(
            &sq(directory.path(), Some(json!({"backend": "value-checker"}))),
            &Fr290,
        )
        .unwrap(),
    )
    .0;
    for item in entries(&named) {
        assert_eq!(item["request"]["backend"], "value-checker", "{item}");
    }

    // An unregistered backend settles each item unknown-backend; a per-item
    // entry naming value-checker settles its item there.
    let absent = command::run(&sq(
        directory.path(),
        Some(json!({"backend": "fsl.absent", "items": [{"claim": first_claim, "backend": "value-checker"}]})),
    ), &Fr290)
    .unwrap();
    assert_eq!(absent.exit_code, 20, "category refusal");
    let absent_items = entries(&documents(&absent).0);
    assert_eq!(
        absent_items[0]["record"]["backend"], "value-checker",
        "{}",
        absent_items[0]
    );
    assert_eq!(absent_items[0]["request"]["backend"], "value-checker");
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
        &command::run(
            &sq(
                directory.path(),
                Some(json!({"backend": "value-checker", "options": {"unknown": "1"}})),
            ),
            &Fr290,
        )
        .unwrap(),
    )
    .0;
    for item in entries(&option) {
        let text = item.to_string();
        assert!(text.contains("invalid_capability"), "{item}");
        assert!(text.contains("unknown-option"), "{item}");
        assert!(!text.contains("invalid_runtime_input"), "{item}");
        assert!(
            item["record"].get("result").is_none(),
            "an invalid-request record: {item}"
        );
    }

    // Without `requirements`, no member and no items.
    let plain = command::run(&write(directory.path(), "seven", json!([]), None), &Fr290).unwrap();
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
        let error = command::run(&sq(directory.path(), Some(requirements.clone())), &Fr290)
            .expect_err("a malformed member refuses");
        let envelope = error.value().expect("refusal envelope");
        assert_eq!(envelope["stage"], "request", "{requirements}: {envelope}");
        assert!(
            envelope.to_string().contains(pointer),
            "{requirements}: {envelope}"
        );
        assert_eq!(error.category().exit_code(), 20);
    }
    let error = command::run(
        &sq(directory.path(), Some(json!({"items": [{"claim": claim}]}))),
        &Fr290,
    )
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
fn plain_run_settles_with_the_linked_engines_only() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let result = command::run(&request, &Fr290).expect("the run completes");
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
    options: Vec<Value>,
}

/// What the recording provider saw: its `settle` calls and the limits of
/// the last request.
#[derive(Debug, Default)]
struct Seen {
    settle_calls: usize,
    deadline_ms: u64,
    max_frame_bytes: u64,
    /// The FR-331 `options` of every request item the provider received.
    options: Vec<Value>,
}

/// A process provider that records each request and answers every item
/// with `verdict`.
#[derive(Debug)]
struct Recording {
    seen: Arc<Mutex<Seen>>,
    verdict: fsl_analyze::provider::ProviderVerdict,
}

impl fsl_analyze::provider::ExternalProvider for Recording {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        let mut seen = self.seen.lock().expect("the record lock");
        seen.settle_calls += 1;
        seen.deadline_ms = request.deadline_ms();
        seen.max_frame_bytes = request.limits().max_frame_bytes;
        let wire = serde_json::to_value(request).expect("the request serializes");
        seen.options.extend(
            wire["items"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| item.get("options").cloned().unwrap_or(Value::Null)),
        );
        vec![self.verdict.clone(); request.items().len()]
    }
}

/// The recording provider, advertising `value-validity` `unbounded` as a
/// process plugin.
fn recording_provider(
    seen: &Arc<Mutex<Seen>>,
    verdict: fsl_analyze::provider::ProviderVerdict,
) -> fsl_analyze::analyze::Engine {
    publishing_provider(seen, verdict, &[])
}

/// The recording provider whose manifest publishes the option `published`.
fn publishing_provider(
    seen: &Arc<Mutex<Seen>>,
    verdict: fsl_analyze::provider::ProviderVerdict,
    published: &[&str],
) -> fsl_analyze::analyze::Engine {
    use fsl_semantics::check::Capability;
    let descriptor = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(PROVIDER)),
        fsl_route::ProviderOrigin::Process,
        vec![(
            Some(Capability::ValueValidity.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises the external value arm")
    .with_options(
        published
            .iter()
            .map(|name| ((*name).to_owned(), "1".to_owned())),
    );
    fsl_analyze::analyze::Engine::external(
        descriptor,
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(Recording {
            seen: Arc::clone(seen),
            verdict,
        }),
    )
    .expect("the descriptor is unbounded")
}

/// FR-290's negotiation, counting the items it negotiates.
struct Counting {
    negotiations: AtomicUsize,
}

impl fsl_analyze::analyze::document::Negotiate for Counting {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        self.negotiations.fetch_add(1, Ordering::SeqCst);
        fsl_analyze::analyze::document::Negotiate::negotiate(&Fr290, item, candidates)
    }
}

/// `fsl_command::command::run_with_providers(request, negotiate, providers,
/// options, cancel)` (FR-961) with a counting negotiator and the recording
/// provider set up as `scenario` says. The document is the run's
/// `spine-run-result/1` document, or its refusal envelope.
fn run_with_providers(request: &Path, scenario: Scenario) -> Option<ProviderRun> {
    use fsl_analyze::provider::ProviderVerdict;
    let seen = Arc::new(Mutex::new(Seen::default()));
    let mut options = fsl_analyze::provider::ProviderOptions::default();
    let cancel = filament_exact::Cancel::new();
    let providers = match scenario {
        Scenario::Limits {
            deadline_ms,
            max_frame_bytes,
        } => {
            options.provider_limits.deadline_ms = deadline_ms;
            options.provider_limits.max_frame_bytes = max_frame_bytes;
            vec![recording_provider(&seen, ProviderVerdict::Proved)]
        }
        Scenario::RepeatedProvider => vec![
            recording_provider(&seen, ProviderVerdict::Proved),
            recording_provider(&seen, ProviderVerdict::Proved),
        ],
        Scenario::UnreproducedRefutation => vec![recording_provider(
            &seen,
            ProviderVerdict::Refuted {
                counterexample: vec![fsl_replay::Value::Integer(filament_exact::Integer::from(
                    4_i64,
                ))],
            },
        )],
        Scenario::FrameFailure => vec![recording_provider(
            &seen,
            ProviderVerdict::Failed {
                limit: fsl_analyze::provider::ProviderLimit::MaxFrameBytes,
                cause: fsl_replay::ToolFailure::FrameLength {
                    limit: 1_048_576,
                    value: 1_048_577,
                },
            },
        )],
        Scenario::TotalBytesFailure => vec![recording_provider(
            &seen,
            ProviderVerdict::Failed {
                limit: fsl_analyze::provider::ProviderLimit::MaxTotalBytes,
                cause: fsl_replay::ToolFailure::TotalBytes {
                    limit: 2_097_152,
                    value: 2_097_153,
                },
            },
        )],
        Scenario::Cancelled => {
            cancel.cancel(filament_exact::CancelCause::Requested);
            vec![recording_provider(&seen, ProviderVerdict::Proved)]
        }
        Scenario::PublishesDepth => vec![publishing_provider(
            &seen,
            ProviderVerdict::Proved,
            &["depth"],
        )],
    };
    let negotiate = Counting {
        negotiations: AtomicUsize::new(0),
    };
    let document =
        match command::run_with_providers(request, &negotiate, providers, &options, &cancel) {
            Ok(result) => documents(&result).0,
            Err(error) => error.value().expect("refusal envelope"),
        };
    let seen = seen.lock().expect("the record lock");
    Some(ProviderRun {
        document,
        settle_calls: seen.settle_calls,
        negotiations: negotiate.negotiations.load(Ordering::SeqCst),
        deadline_ms: seen.deadline_ms,
        max_frame_bytes: seen.max_frame_bytes,
        options: seen.options.clone(),
    })
}

/// How the recording provider and the run are set up.
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
    /// The provider reports an oversize transport frame.
    FrameFailure,
    /// The provider reports an oversize transport run total.
    TotalBytesFailure,
    /// The handle is cancelled before the run.
    Cancelled,
    /// The provider's manifest publishes the option `depth`.
    PublishesDepth,
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

/// FR-950 / QSL-2375: the provider-backed entry uses caller-selected byte
/// ceilings and applies the same request overrides as plain `run`.
#[trace("TC-2701", "FR-950-AC-5")]
#[test]
fn provider_run_accepts_run_limits_and_request_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let request_bytes = std::fs::metadata(&request).unwrap().len() as usize;
    let dependent_bytes = std::fs::metadata(directory.path().join("unit.native"))
        .unwrap()
        .len() as usize;
    let limits = command::RunLimits {
        request_bytes,
        dependent_bytes,
        ..command::RunLimits::default()
    };
    let clocks = serde_json::value::RawValue::from_string("{}".to_owned()).unwrap();
    let overrides = command::RequestOverrides {
        clocks: Some(&clocks),
        ..command::RequestOverrides::default()
    };
    let seen = Arc::new(Mutex::new(Seen::default()));
    let options = fsl_analyze::provider::ProviderOptions::default();
    let cancel = filament_exact::Cancel::new();
    let result = command::run_with_providers_and_limits_and_overrides(
        &request,
        limits,
        &overrides,
        &Fr290,
        vec![recording_provider(
            &seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect("the exact request and dependent-file ceilings admit the provider run");
    assert_eq!(
        documents(&result).0["outcome"]["value"],
        json!({"boolean": true})
    );
    assert_eq!(seen.lock().unwrap().settle_calls, 1);

    let below_request_size = command::RunLimits {
        request_bytes: request_bytes - 1,
        dependent_bytes,
        ..command::RunLimits::default()
    };
    let error = command::run_with_providers_and_limits_and_overrides(
        &request,
        below_request_size,
        &overrides,
        &Fr290,
        vec![recording_provider(
            &seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect_err("the provider run enforces the caller's request byte ceiling");
    assert!(error.to_string().contains("request_bytes"), "{error}");
    assert_eq!(
        seen.lock().unwrap().settle_calls,
        1,
        "refusal sends no request"
    );

    let below_dependent_size = command::RunLimits {
        request_bytes,
        dependent_bytes: dependent_bytes - 1,
        ..command::RunLimits::default()
    };
    let error = command::run_with_providers_and_limits_and_overrides(
        &request,
        below_dependent_size,
        &overrides,
        &Fr290,
        vec![recording_provider(
            &seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect_err("the provider run enforces the caller's dependent-file byte ceiling");
    assert!(matches!(
        error.cause,
        command::RunCause::Limit(command::LimitKind::DependentBytes { maximum })
            if maximum == dependent_bytes - 1
    ));
    assert_eq!(
        seen.lock().unwrap().settle_calls,
        1,
        "a dependent-file limit refusal sends no request"
    );

    let unused_clock = serde_json::value::RawValue::from_string(
        json!({"spare": {"kind": "event_position", "sequence_authority": "spare"}}).to_string(),
    )
    .unwrap();
    let clock_override = command::RequestOverrides {
        clocks: Some(&unused_clock),
        ..command::RequestOverrides::default()
    };
    let error = command::run_with_providers_and_limits_and_overrides(
        &request,
        limits,
        &clock_override,
        &Fr290,
        vec![recording_provider(
            &seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect_err("the non-default clock override is applied to the provider run");
    assert!(
        error
            .to_string()
            .contains("names a clock no temporal declaration uses"),
        "{error}"
    );
    assert_eq!(
        seen.lock().unwrap().settle_calls,
        1,
        "the unused clock refuses before provider settlement"
    );
}

/// FR-961-AC-6: a provider-backed run applies the caller's S1 node limit to
/// compilation, so an incomplete compile settles no requirement items.
#[trace("TC-2701", "FR-961-AC-6")]
#[test]
fn provider_run_applies_s1_node_limit_before_requirement_settlement() {
    use fsl_foundation::{Setting, SettingLimits};
    use fsl_replay::Operation;

    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let mut stage_limits = fsl_replay::spine::SpineLimits::default();
    assert!(stage_limits.set_bound(Setting::S1Nodes, 1));
    let overrides = command::RequestOverrides {
        limits: Some(&stage_limits),
        ..command::RequestOverrides::default()
    };
    let limits = command::RunLimits::default();

    let plain_error = command::run_with_limits_and_overrides(&request, limits, &overrides, &Fr290)
        .expect_err("the caller's S1 node limit prevents compilation");

    let seen = Arc::new(Mutex::new(Seen::default()));
    let negotiate = Counting {
        negotiations: AtomicUsize::new(0),
    };
    let options = fsl_analyze::provider::ProviderOptions::default();
    let cancel = filament_exact::Cancel::new();
    let provider_error = command::run_with_providers_and_limits_and_overrides(
        &request,
        limits,
        &overrides,
        &negotiate,
        vec![recording_provider(
            &seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect_err("the provider-backed run applies the caller's S1 node limit");

    assert_eq!(
        provider_error.cause.code(),
        fsl_foundation::Code::StageLimitExceeded
    );
    assert!(provider_error.cause.is_incomplete());
    assert_eq!(provider_error.category().exit_code(), 22);
    let outcome =
        serde_json::to_value(provider_error.outcome_document(Operation::Execute)).unwrap();
    let plain_outcome =
        serde_json::to_value(plain_error.outcome_document(Operation::Execute)).unwrap();
    assert_eq!(
        outcome, plain_outcome,
        "the typed limit outcome matches plain run"
    );
    assert_eq!(outcome["category"], "incomplete");
    assert_eq!(outcome["diagnostics"][0]["code"], "stage_limit_exceeded");
    assert!(outcome["diagnostics"][0].to_string().contains("s1.nodes"));
    assert!(
        outcome["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("limit 1 reached")),
        "the typed limit names configured bound 1: {outcome}"
    );
    assert_eq!(negotiate.negotiations.load(Ordering::SeqCst), 0);
    assert_eq!(seen.lock().unwrap().settle_calls, 0);

    let default_limits = fsl_replay::spine::SpineLimits::default();
    let default_overrides = command::RequestOverrides {
        limits: Some(&default_limits),
        ..command::RequestOverrides::default()
    };
    let default_seen = Arc::new(Mutex::new(Seen::default()));
    let default_result = command::run_with_providers_and_limits_and_overrides(
        &request,
        limits,
        &default_overrides,
        &Fr290,
        vec![recording_provider(
            &default_seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &options,
        &cancel,
    )
    .expect("the provider run with default caller limits settles its items");
    let (default_document, default_outcome) = documents(&default_result);

    let ac1_seen = Arc::new(Mutex::new(Seen::default()));
    let ac1_result = command::run_with_providers(
        &request,
        &Fr290,
        vec![recording_provider(
            &ac1_seen,
            fsl_analyze::provider::ProviderVerdict::Proved,
        )],
        &fsl_analyze::provider::ProviderOptions::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("FR-961-AC-1 result");
    let (ac1_document, ac1_outcome) = documents(&ac1_result);
    assert_eq!(
        default_document, ac1_document,
        "the default-limit result equals AC-1"
    );
    assert_eq!(
        default_outcome, ac1_outcome,
        "the default-limit outcome equals AC-1"
    );
    assert_eq!(
        default_seen.lock().unwrap().settle_calls,
        1,
        "the default-bound provider run settles every item"
    );
}

/// FR-961-AC-4, FR-961-AC-5: negative cases of the run-side entry: a repeated
/// provider identity, a refused compile, a counterexample replay does not
/// reproduce, and a cancelled handle.
#[trace("TC-2701", "FR-961-AC-4")]
#[trace("TC-2701", "FR-961-AC-5")]
#[test]
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

    let failed = run_with_providers(&request, Scenario::FrameFailure).expect("the run-side entry");
    for item in entries(&failed.document) {
        assert_eq!(item["record"]["result"], "failed", "{item}");
        assert_eq!(item["record"]["cause"]["kind"], "tool-failure", "{item}");
        assert_eq!(item["record"]["cause"]["defect"], "frame-length", "{item}");
        assert_eq!(item["record"]["cause"]["limit"], 1_048_576, "{item}");
        assert_eq!(item["record"]["cause"]["value"], 1_048_577, "{item}");
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

/// FR-287-AC-12: a provider's over-limit run total settles as `failed` with
/// the exact typed `total-bytes` transport cause.
#[trace("TC-2701", "FR-291-AC-1")]
#[test]
fn provider_total_byte_limit_settles_failed_with_typed_cause() {
    let directory = tempfile::tempdir().unwrap();
    let request = write(
        directory.path(),
        "Even",
        json!([{"parameter": "n", "value": 4}]),
        Some(json!({"backend": PROVIDER})),
    );
    let result = run_with_providers(&request, Scenario::TotalBytesFailure)
        .expect("the provider-backed run returns a settled result");
    let items = entries(&result.document);
    assert!(!items.is_empty(), "the call has requirement items");
    for item in items {
        assert_eq!(item["record"]["result"], "failed", "{item}");
        assert_eq!(
            item["record"]["cause"],
            json!({
                "kind": "tool-failure",
                "defect": "total-bytes",
                "limit": 2_097_152,
                "value": 2_097_153
            }),
            "{item}"
        );
    }
    assert_eq!(result.settle_calls, 1);
}

/// FR-962-AC-4: with the recording provider (advertising `value-validity`)
/// registered, an unnamed `sq` item has two candidates and no tie row, so it
/// settles `ambiguous-backend`; naming `value-checker` settles it there.
#[trace("TC-2702", "FR-962-AC-4")]
#[test]
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

/// FR-967-AC-2, its published-option half, through the run's requirement
/// items: with the recording provider's manifest publishing `depth`, an
/// item naming it with `{"depth": "3"}` reaches the provider with that
/// option in its FSpec FR-331 request item, and `{"depth": "3", "width":
/// "1"}` settles `invalid_capability`/`unknown-option` naming `width` and
/// sends the provider no request.
#[trace("TC-2724", "FR-967-AC-2")]
#[test]
fn a_published_option_reaches_the_provider_and_an_unpublished_one_settles() {
    let directory = tempfile::tempdir().unwrap();
    let even = |options: Value| {
        write(
            directory.path(),
            "Even",
            json!([{"parameter": "n", "value": 4}]),
            Some(json!({"backend": PROVIDER, "options": options})),
        )
    };
    let published = run_with_providers(&even(json!({"depth": "3"})), Scenario::PublishesDepth)
        .expect("the run-side entry");
    let items = entries(&published.document);
    assert_eq!(published.settle_calls, 1, "{}", published.document);
    assert_eq!(published.options.len(), items.len());
    for options in &published.options {
        assert_eq!(options, &json!({"depth": "3"}));
    }
    for item in &items {
        assert_eq!(item["record"]["result"], "proved", "{item}");
    }

    let unpublished = run_with_providers(
        &even(json!({"depth": "3", "width": "1"})),
        Scenario::PublishesDepth,
    )
    .expect("the run-side entry");
    assert_eq!(unpublished.settle_calls, 0, "no request is sent");
    assert_eq!(unpublished.negotiations, 0, "no item is negotiated");
    for item in entries(&unpublished.document) {
        let text = item.to_string();
        assert!(text.contains("invalid_capability"), "{item}");
        assert!(text.contains("unknown-option"), "{item}");
        assert!(text.contains("\"width\""), "{item}");
        assert!(text.contains(PROVIDER), "{item}");
        let detail = &item["record"]["diagnostic"]["detail"];
        assert_eq!(detail["request_index"], item["request_index"], "{item}");
        assert_eq!(detail["option"], "width", "{item}");
    }
    assert_eq!(
        unpublished.document["outcome"],
        json!({"kind": "completed", "value": {"boolean": true}}),
        "the call's value is unchanged"
    );
}

/// The claims a result wrote for the items of `sq`, in the document-held form
/// FSL FR-621 gives (operands as `{"node": i}` segments).
fn held_claims(directory: &Path) -> Vec<Value> {
    let result = command::run(&sq(directory, Some(json!({}))), &Fr290).expect("the run completes");
    let claims: Vec<Value> = entries(&documents(&result).0)
        .iter()
        .map(|item| item["request"]["claim"].clone())
        .collect();
    for claim in &claims {
        assert!(
            claim["node"]["path"]
                .as_array()
                .is_some_and(|path| path.iter().any(|segment| segment.get("node").is_some())),
            "the claim is held: {claim}"
        );
    }
    claims
}

fn held_first_claim(directory: &Path) -> Value {
    held_claims(directory).remove(0)
}

/// `held` in FSpec FR-331's structural spelling, resolved through the graph of
/// the package the request's source compiles to.
fn structural(directory: &Path, held: &Value) -> Value {
    let request = json!({"models": [], "program": {"source": source()},
        "handoff": {"directory": "handoff", "package_name": "test:requirements",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}});
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    command::compile(&path).expect("the unit compiles");
    let bytes = std::fs::read(directory.join("handoff/checked-package.json")).unwrap();
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(&bytes).unwrap();
    let mut claim = held.clone();
    claim["node"] = graph.structural_value(&held["node"]).unwrap();
    assert!(
        !claim["node"]["path"]
            .as_array()
            .unwrap()
            .iter()
            .any(|segment| segment.get("node").is_some()),
        "the claim is structural: {claim}"
    );
    claim
}

/// Run `sq` with `requirements` and return its item entries.
fn settled(directory: &Path, requirements: Value) -> Vec<Value> {
    let result = command::run(&sq(directory, Some(requirements)), &Fr290).unwrap();
    entries(&documents(&result).0)
}

/// The refusal envelope of `sq` run with `requirements`.
fn refusal(directory: &Path, requirements: Value) -> (Value, u8) {
    let error = command::run(&sq(directory, Some(requirements)), &Fr290)
        .expect_err("the request refuses");
    (
        error.value().expect("refusal envelope").clone(),
        error.category().exit_code(),
    )
}

/// FR-960 "Inputs", FSL FR-621 "Node ids in an item": a claim a result wrote
/// names its item when supplied back, and settles it on the named backend.
#[trace("TC-2700", "FR-960-AC-5")]
#[test]
fn a_claim_written_by_a_result_names_its_item_when_supplied_back() {
    let directory = tempfile::tempdir().unwrap();
    let claim = held_first_claim(directory.path());
    let items = settled(
        directory.path(),
        json!({"backend": "fsl.absent", "items": [{"claim": claim, "backend": "value-checker"}]}),
    );
    assert_eq!(items[0]["record"]["backend"], "value-checker", "{}", items[0]);
    assert_eq!(items[0]["record"]["result"], "proved", "{}", items[0]);
}

/// FR-960 "Inputs": a claim in FSpec FR-331's structural spelling names its
/// item and settles it on its named backend.
#[trace("TC-2700", "FR-960-AC-5")]
#[test]
fn a_structural_claim_names_its_item() {
    let directory = tempfile::tempdir().unwrap();
    let claim = structural(directory.path(), &held_first_claim(directory.path()));
    let items = settled(
        directory.path(),
        json!({"backend": "fsl.absent", "items": [{"claim": claim, "backend": "value-checker"}]}),
    );
    assert_eq!(items[0]["record"]["backend"], "value-checker", "{}", items[0]);
    assert!(items[1].to_string().contains("unknown-backend"), "{}", items[1]);
}

/// FR-960 "Inputs": a held entry and a structural entry naming two different
/// items each settle on their own backend.
#[trace("TC-2700", "FR-960-AC-5")]
#[test]
fn a_held_entry_and_a_structural_entry_name_their_own_items() {
    let directory = tempfile::tempdir().unwrap();
    let claims = held_claims(directory.path());
    let second = structural(directory.path(), &claims[1]);
    let items = settled(
        directory.path(),
        json!({"backend": "fsl.absent", "items": [
            {"claim": claims[0], "backend": "value-checker"},
            {"claim": second, "backend": "value-checker"}]}),
    );
    assert_eq!(items[0]["record"]["backend"], "value-checker", "{}", items[0]);
    assert_eq!(items[1]["record"]["backend"], "value-checker", "{}", items[1]);
    assert!(items[2].to_string().contains("unknown-backend"), "{}", items[2]);
}

/// FR-960 "Refusals before the call": one claim written in both forms is two
/// entries naming one claim, refused at the later entry.
#[trace("TC-2700", "FR-960-AC-6")]
#[test]
fn one_claim_written_in_both_forms_refuses_as_a_duplicate() {
    let directory = tempfile::tempdir().unwrap();
    let held = held_first_claim(directory.path());
    let structural = structural(directory.path(), &held);
    let (envelope, exit) = refusal(
        directory.path(),
        json!({"items": [{"claim": held}, {"claim": structural}]}),
    );
    assert_eq!(envelope["stage"], "request", "{envelope}");
    assert_eq!(envelope["code"], "invalid-request", "{envelope}");
    assert!(
        envelope.to_string().contains("/call/requirements/items/1"),
        "{envelope}"
    );
    assert_eq!(exit, 20);
}

/// A held claim whose index segment the package graph does not have, or that
/// is no index, refuses `invalid-request` at stage `request`, naming that
/// segment's `node` member.
#[trace("TC-2700", "FR-960-AC-7")]
#[test]
fn a_held_claim_with_a_bad_index_refuses_at_its_node_member() {
    let directory = tempfile::tempdir().unwrap();
    let held = held_first_claim(directory.path());
    let position = held["node"]["path"]
        .as_array()
        .unwrap()
        .iter()
        .position(|segment| segment.get("node").is_some())
        .unwrap();
    for bad in [json!(999_999), json!("3"), json!(-1)] {
        let mut claim = held.clone();
        claim["node"]["path"][position] = json!({"node": bad});
        let (envelope, exit) = refusal(directory.path(), json!({"items": [{"claim": claim}]}));
        assert_eq!(envelope["stage"], "request", "{bad}: {envelope}");
        assert_eq!(envelope["code"], "invalid-request", "{bad}: {envelope}");
        assert!(
            envelope.to_string().contains(&format!(
                "/call/requirements/items/0/claim/node/path/{position}/node"
            )),
            "{bad}: {envelope}"
        );
        assert_eq!(exit, 20);
    }
}

/// A held claim that resolves but names no item of the call stays
/// `missing_declaration` at stage `call`, and the diagnostic names the claim
/// as the caller supplied it.
#[trace("TC-2700", "FR-960-AC-8")]
#[test]
fn a_held_claim_naming_no_item_of_the_call_is_an_unknown_claim() {
    let directory = tempfile::tempdir().unwrap();
    let mut claim = held_first_claim(directory.path());
    claim["occurrence"]["ordinal"] = json!(99);
    let (envelope, exit) = refusal(directory.path(), json!({"items": [{"claim": claim}]}));
    assert_eq!(envelope["stage"], "call", "{envelope}");
    assert_eq!(envelope["code"], "missing_declaration", "{envelope}");
    assert_eq!(envelope["details"]["claim"], claim, "{envelope}");
    assert_eq!(exit, 20);
}
