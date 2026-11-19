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
//! In-process and process fixture providers exercise both FR-750 dispatch
//! paths.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use filament_exact::{Integer, Rational, ScalarLimits};
use fsl_analyze::analyze::{BackendTool, Engine, HybridProcessExchange};
use fsl_analyze::provider::{HybridAnswer, HybridProvider, HybridSolveRequest};
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::ModeIndex;
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

/// What an [`answering`] provider was asked: how often, and the options and
/// deadline of each request.
type Asked = (BTreeMap<String, String>, Option<u64>);

#[derive(Debug, Default)]
pub(crate) struct Seen {
    pub(crate) calls: AtomicUsize,
    pub(crate) requests: Mutex<Vec<Asked>>,
}

impl Seen {
    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    /// The options and deadline of every request, in order.
    pub(crate) fn requests(&self) -> Vec<Asked> {
        self.requests
            .lock()
            .expect("the log is not poisoned")
            .clone()
    }
}

type Answer = Box<dyn Fn(&HybridSolveRequest<'_>) -> HybridAnswer + Send + Sync>;

/// A hybrid provider that answers by `answer` and records what it was asked.
struct AnsweringProvider {
    seen: Arc<Seen>,
    answer: Answer,
}

impl std::fmt::Debug for AnsweringProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AnsweringProvider")
    }
}

impl HybridProvider for AnsweringProvider {
    fn solve(
        &self,
        request: &HybridSolveRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> HybridAnswer {
        self.seen.calls.fetch_add(1, Ordering::SeqCst);
        self.seen
            .requests
            .lock()
            .expect("the log is not poisoned")
            .push((request.options.clone(), request.deadline_ms));
        (self.answer)(request)
    }
}

/// The in-process hybrid fixture `id` whose provider answers every item with
/// the FSpec FR-331 result entry `entry`: `Engine::hybrid` over a descriptor
/// with `dynamics` `["hybrid"]`.
pub(crate) fn hybrid_fixture(id: &str, entry: &Value) -> Engine {
    answering(id, entry_answer(entry.clone())).0
}

/// The answer that reads `entry` against its own `counterexamples`.
pub(crate) fn entry_answer(entry: Value) -> impl Fn(&HybridSolveRequest<'_>) -> HybridAnswer {
    move |request| {
        let counterexamples = entry["counterexamples"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        HybridAnswer::read_entry(
            &entry,
            counterexamples,
            request.graph,
            request.claim,
            request.wire,
        )
        .unwrap_or_else(|refusal| panic!("the fixture entry is read: {refusal}"))
    }
}

/// The in-process hybrid fixture `id` answering by `answer`, with the log of
/// what it was asked.
pub(crate) fn answering(
    id: &str,
    answer: impl Fn(&HybridSolveRequest<'_>) -> HybridAnswer + Send + Sync + 'static,
) -> (Engine, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let engine = build_hybrid(
        id,
        Box::new(AnsweringProvider {
            seen: Arc::clone(&seen),
            answer: Box::new(answer),
        }),
    )
    .unwrap_or_else(|error| panic!("the fixture engine is made: {error}"));
    (engine, seen)
}

fn build_hybrid(id: &str, provider: Box<dyn HybridProvider>) -> Result<Engine, String> {
    let descriptor = descriptor_of(&json!({"backend": id, "dynamics": ["hybrid"],
        "advertises": [{"kind": "temporal-satisfaction", "mode": "bounded"},
            {"kind": "temporal-satisfaction", "mode": "unbounded"}],
        "options": ["step"],
        "bounds": {"max_solver_steps": 1000, "max_solver_time": 1}}))?;
    Engine::hybrid(descriptor, provider).map_err(|error| error.to_string())
}

/// The routed descriptor of the FSpec FR-331 descriptor `descriptor`.
pub(crate) fn descriptor_of(descriptor: &Value) -> Result<fsl_route::BackendDescriptor, String> {
    let id = descriptor["backend"].as_str().unwrap_or("hybrid-fixture");
    let pairs: Vec<(String, String)> = descriptor["advertises"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|pair| {
            (
                pair["kind"].as_str().unwrap_or_default().to_owned(),
                pair["mode"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let labels: Vec<&str> = descriptor["dynamics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let bound = |name: &str| descriptor["bounds"][name].as_u64();
    fsl_route::descriptor_from_manifest_with_dynamics(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(id)),
        fsl_route::ProviderOrigin::Linked,
        pairs
            .iter()
            .map(|(kind, mode)| (Some(kind.as_str()), Some(mode.as_str()))),
        fsl_route::ManifestMembers {
            domains: Some(&["integer"]),
            fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
            ..fsl_route::ManifestMembers::default()
        },
        Some(fsl_route::DynamicsMember::List(&labels)),
        Some(fsl_route::SolverBounds {
            max_solver_steps: bound("max_solver_steps"),
            max_solver_time: bound("max_solver_time"),
        }),
    )
    .map(|routed| {
        routed.with_options(
            descriptor["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name.as_str().map(str::to_owned)),
        )
    })
    .map_err(|refusal| format!("{refusal:?}"))
}

#[derive(Debug)]
struct FixedHybridExchange(Vec<u8>);

impl HybridProcessExchange for FixedHybridExchange {
    fn exchange(
        &self,
        _request: &[u8],
        _cancel: &filament_exact::Cancel,
    ) -> Result<Vec<u8>, fsl_replay::ToolFailure> {
        Ok(self.0.clone())
    }
}

/// The process hybrid fixture `id` whose exchange returns `entry` as its
/// result body: `Engine::hybrid_process` (FR-750).
pub(crate) fn process_fixture(id: &str, entry: &Value) -> Engine {
    let descriptor = descriptor_of(&json!({"backend": id, "dynamics": ["hybrid"],
        "advertises": [{"kind": "temporal-satisfaction", "mode": "bounded"},
            {"kind": "temporal-satisfaction", "mode": "unbounded"}],
        "options": [], "bounds": {"max_solver_steps": 1000, "max_solver_time": 1}}))
    .expect("the process fixture descriptor is valid");
    let tool = BackendTool::new("fixture-hybrid", "1").expect("the process fixture tool is valid");
    Engine::hybrid_process(
        descriptor,
        tool,
        Box::new(FixedHybridExchange(
            serde_json::to_vec(entry).expect("the fixture entry serializes"),
        )),
    )
    .expect("the process fixture advertises hybrid dynamics")
}

/// `Engine::hybrid` over the FSpec FR-331 descriptor `descriptor`, its error
/// rendered as text (FR-750).
pub(crate) fn hybrid_engine(descriptor: &Value) -> Result<Engine, String> {
    Engine::hybrid(
        descriptor_of(descriptor)?,
        Box::new(AnsweringProvider {
            seen: Arc::default(),
            answer: Box::new(|_| panic!("the engine is only built, never asked")),
        }),
    )
    .map_err(|error| error.to_string())
}

/// The exact rational `text` (`n` or `n/d`) as FSpec FR-193's
/// `{numerator, denominator}` object.
pub(crate) fn rational(text: &str) -> Value {
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    json!({"numerator": numerator, "denominator": denominator})
}

pub(crate) fn interval(lower: &str, upper: &str) -> Value {
    json!({"lower": rational(lower), "upper": rational(upper)})
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
fn a_covering_delta_complete_proof_settles_proved_bounded_solver() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": {"numerator": "10", "denominator": "1"}, "jumps": 1,
            "exhaustive": ["time"]}});
    let (engine, seen) = answering("hybrid-fixture", entry_answer(entry));
    let (_, item) = first_item(
        "hybrid-750-1",
        &braking_pair(),
        named("SlowEnough", "hybrid-fixture"),
        json!({}),
        vec![engine],
    );
    let record = &item["record"];
    assert_eq!(record["result"], "proved", "{item}");
    assert_eq!(record["basis"]["type"], "bounded-solver", "{item}");
    assert_eq!(record["certification"], "uncertified", "{item}");
    assert_eq!(record["backend"], "hybrid-fixture", "{item}");
    assert_eq!(seen.calls(), 1, "the provider is dispatched once");
}

/// FR-750-AC-6: an item's `options` reach the provider as the strings the
/// request wrote, and its exact numbers are read only as FSpec FR-193
/// `{numerator, denominator}` objects.
#[test]
#[trace("TC-3035", "FR-750-AC-6")]
fn item_options_reach_the_provider_as_strings() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "delta-complete",
            "horizon": rational("10"), "jumps": 1, "exhaustive": ["time"]}});
    let (engine, seen) = answering("hybrid-fixture", entry_answer(entry));
    let (_, item) = first_item(
        "hybrid-750-6",
        &braking_pair(),
        json!([{"claim": "SlowEnough", "backend": "hybrid-fixture",
            "options": {"step": "1/16"}}]),
        json!({}),
        vec![engine],
    );
    assert_eq!(item["record"]["result"], "proved", "{item}");
    let requests = seen.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].0,
        BTreeMap::from([("step".to_owned(), "1/16".to_owned())])
    );
}

/// FR-750-AC-2: `Engine::hybrid` refuses a descriptor whose `dynamics` lacks
/// `hybrid`, naming the backend and `invalid-dynamics`.
#[test]
#[trace("TC-2100", "FR-750-AC-2")]
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

pub(crate) fn point_trace(before_v: &str) -> Value {
    let state = |mode: &str, time: &str, v: &str| {
        json!({"mode": mode, "time": interval(time, time),
            "enclosure": {"v": interval(v, v), "a": interval("2", "2")}})
    };
    json!({"start": state("Moving", "0", "20"), "jumps": [{"transition": 0,
        "before": {"time": interval("10", "10"),
            "enclosure": {"v": interval(before_v, before_v), "a": interval("2", "2")}},
        "state": state("Stopped", "10", "0")}]})
}

pub(crate) fn refuted_entry(trace: Value) -> Value {
    json!({"request_index": 0, "result": "refuted", "counterexample": 0,
        "counterexamples": [{"type": "hybrid-counterexample", "kind": "formula",
            "trace": trace}]})
}

fn exact(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn point_counterexample() -> fsl_replay::HybridCounterexample {
    point_counterexample_with_acceleration(2, 10)
}

fn point_counterexample_with_acceleration(
    acceleration: i64,
    transition_time: i64,
) -> fsl_replay::HybridCounterexample {
    use fsl_replay::{
        ExactInterval, HybridCounterexample, HybridCounterexampleKind, HybridTrace, TraceBefore,
        TraceJump, TraceState,
    };
    let interval = |value| ExactInterval::point(exact(value));
    let state = |mode, time, v| TraceState {
        mode: ModeIndex::new(mode),
        time: interval(time),
        enclosure: vec![interval(v), interval(acceleration)],
    };
    HybridCounterexample {
        kind: HybridCounterexampleKind::Formula,
        trace: HybridTrace {
            start: state(0, 0, 20),
            jumps: vec![TraceJump {
                transition: 0,
                before: Some(TraceBefore {
                    time: interval(transition_time),
                    enclosure: vec![interval(0), interval(acceleration)],
                }),
                state: state(1, transition_time, 0),
            }],
            delta: None,
            loop_to: None,
        },
    }
}

fn hybrid_replay_request(
    unit: &str,
    counterexample: fsl_replay::HybridCounterexample,
) -> fsl_replay::ReplayRequestWire {
    let source = SourceIdentity::new("agent-ix", "hybrid-replay");
    let compiled = fsl_replay::compile_package(
        fsl_replay::CompileUnit {
            source: source.clone(),
            package_name: "hybrid-package".to_owned(),
            path: "hybrid.native",
            bytes: unit.as_bytes(),
        },
        [],
        &fsl_replay::DependencyInput::default(),
        fsl_replay::StageLimits::default(),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("the hybrid unit compiles");
    fsl_replay::ReplayRequestWire {
        package: fsl_replay::ReplayPackage::read(
            compiled.bytes(),
            fsl_replay::ReplayLimits::default().input_bytes,
        )
        .expect("the emitted package is a replay package"),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: fsl_replay::RequestTargetWire::state_clause(
            fsl_replay::QualifiedName::new(vec![filament_exact::Identifier::new("TooSlow")
                .expect("the claim name is an identifier")])
            .expect("the claim name is qualified"),
        ),
        source: fsl_replay::ReplaySource::Hybrid(Box::new(counterexample)),
        obligation_identity: crate::fixture_ids::obligation_identity(),
        backend: "hybrid-fixture".to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: ScalarLimits {
            work_units: 1_000,
            ..crate::counter::UNLIMITED
        },
        stage_limits: std::collections::BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![(source.authority, source.identity, unit.as_bytes().to_vec())],
        documents: Vec::new(),
    }
}

/// FR-750-AC-5: a process provider composed for a clause request returns a
/// point trace; one that replays settles `refuted`, one that does not
/// `inconclusive`, `replay-parity`.
#[test]
#[trace("TC-2100", "FR-750-AC-5")]
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
#[ignore = "QSL-2531: core check of hybrid provider certificates not implemented yet"]
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
#[ignore = "QSL-2531: core check of flowpipe certificates not implemented yet"]
fn a_flowpipe_step_violating_picard_is_rejected() {
    let entry = json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "flowpipe-outward-rounded",
            "horizon": rational("10"), "jumps": 1, "exhaustive": ["time"]},
        "certificate": {"type": "flowpipe", "steps": [{"mode": "Moving",
            "time": interval("0", "10"), "step": rational("10"),
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
#[ignore = "QSL-2531: work-limited certificate check not implemented yet"]
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
#[ignore = "QSL-2531: certificate form check not implemented yet"]
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
#[ignore = "QSL-2531: validated-witness refutation not implemented yet"]
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
        "time": interval("0", "40"), "step": rational("40"),
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
            json!({"mode": "Moving", "time": interval(&start, &end), "step": rational("1/16"),
            "box": {"v": interval("0", &upper), "a": interval("1", "2")},
            "apriori": {"v": interval("0", &upper), "a": interval("1", "2")}}),
        );
    }
    steps.push(
        json!({"mode": "Stopped", "time": interval(jump_time.0, jump_time.1),
        "step": rational("10"), "box": {"v": interval("0", "0"), "a": interval("1", "2")},
        "apriori": {"v": interval("0", "0"), "a": interval("1", "2")}}),
    );
    json!({"request_index": 0, "result": "proved", "certification": "uncertified",
        "basis": {"type": "bounded-solver", "method": "flowpipe-outward-rounded",
            "horizon": rational("10"), "jumps": 1, "exhaustive": ["time"]},
        "certificate": {"type": "flowpipe", "steps": steps}})
}

/// FSpec FR-652-AC-5: a flowpipe whose `Moving` boxes stop at the boundary
/// invariant and whose one jump step carries the narrowed jump time
/// `[0, 10]` is accepted; a jump step omitting time 0 is rejected with rule
/// `jump`; one with time `[0, 20]` contains the narrowed jump time and is
/// accepted; against a violated claim it is rejected with rule `claim`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-5")]
#[ignore = "QSL-2531: flowpipe guard and boundary-invariant check not implemented yet"]
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
    assert_eq!(late["certificate"]["rule"], "jump", "{late}");
    let wide = check("hybrid-652-5d", "SlowEnough", ("0", "20"));
    assert_eq!(wide["record"]["certification"], "certified", "{wide}");
    let violated = check("hybrid-652-5c", "TooSlow", ("0", "10"));
    assert_eq!(violated["certificate"]["accepted"], false, "{violated}");
    assert_eq!(violated["certificate"]["rule"], "claim", "{violated}");
}

/// FSpec FR-193-AC-23: a trace cut by `end` inside its last dwell replays
/// exactly to the bad prefix; a wrong `end` does not reproduce at `/end`; an
/// `end` outside the invariant fails static check 6; `end` with `loop`
/// refuses at `/end` (FR-450-AC-10).
#[test]
#[trace("FSpec-TC-373", "FSpec-FR-193-AC-23")]
#[trace("TC-1221", "FR-450-AC-10")]
#[ignore = "QSL-2533: hybrid trace `end` replay not implemented yet"]
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
    assert!(wrong.to_string().contains("/end"), "{wrong}");
    let outside = first_item(
        "hybrid-193-23d",
        &unit,
        named("TooSlow", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture(
            "hybrid-fixture",
            &json!({"request_index": 0, "result": "inconclusive",
                "cause": {"type": "solver-inconclusive", "trace": trace("25")}}),
        )],
    )
    .1;
    let static_outcome = find(&outside["record"], "static").expect("a static replay outcome");
    assert_eq!(static_outcome["outcome"], "inconsistent", "{outside}");
    assert_eq!(static_outcome["check"], 6, "{outside}");
    let mut looped = trace("4");
    looped["loop"] = json!(0);
    let refused = run("hybrid-193-23c", looped);
    let text = refused.to_string();
    assert!(text.contains("invalid_result"), "{refused}");
    assert!(text.contains("/end"), "{refused}");
}

/// FR-750-AC-7 (process half): a process fixture whose entry is
/// `unsupported` with `unsupported_construct`/`expression-form` settles the
/// same located `unsupported` as the in-process fixture.
#[test]
#[trace("TC-3035", "FR-750-AC-7")]
#[ignore = "QSL-2532: `Engine::hybrid_process` does not exist yet"]
fn a_process_unsupported_entry_settles_a_located_expression_form() {
    todo!("QSL-2532: process half of FR-750-AC-7");
}

/// FR-750-AC-9 (reader and irrational instant): the plugin reader reads a
/// written undefined-evaluation counterexample back to a `Refuted` answer
/// FR-450 reproduces, and an irrational instant is written as a rational
/// interval that contains it.
#[test]
#[trace("TC-3035", "FR-750-AC-9")]
#[ignore = "QSL-2533: the reader's read-back of `where`/`locus` and the irrational-instant solver are not implemented yet"]
fn an_undefined_evaluation_counterexample_reads_back() {
    todo!("QSL-2533: AC-9 read-back and irrational instant");
}

/// FR-750-AC-10: the refuted `TooSlow` item's `replay` resolves to the
/// hybrid claim and FR-098 writes the FSpec FR-323 `hybrid` row
/// `reproduced-with-evaluated-witness`; a non-reproducing trace writes
/// `inconclusive` with the `hybrid-trace` disagreement.
#[test]
#[trace("TC-3035", "FR-750-AC-10")]
fn a_refuted_hybrid_item_replays_through_fr_098() {
    let request = hybrid_replay_request(&braking_pair(), point_counterexample());
    let result = fsl_replay::replay(request, fsl_replay::ReplayLimits::default())
        .expect("the hybrid counterexample replays");
    let fsl_replay::ReplayResult::Hybrid(row) = &result else {
        panic!("FR-323 dispatches the hybrid source to the hybrid arm: {result:?}");
    };
    assert_eq!(
        result.category(),
        filament_semantic_value::catalog::Category::Success
    );
    let wire: Value = serde_json::from_str(
        &result
            .to_function_json()
            .expect("the hybrid result serializes"),
    )
    .expect("the result row is JSON");
    assert_eq!(wire["arm"], "hybrid");
    assert_eq!(wire["settlement"], "reproduced-with-evaluated-witness");
    assert_eq!(wire["category"], "success");
    assert_eq!(wire["replay"]["exact"]["outcome"], "reproduced");
    assert_eq!(wire["counterexample"]["kind"], "formula");
    assert!(matches!(
        row.settlement,
        fsl_replay::HybridReplaySettlement::Reproduced { .. }
    ));

    // With acceleration 1, the same path reaches Stopped only at t=20.
    // `TooSlow` holds throughout [0, 10], so exact replay must report that
    // the provider's refuted trace does not reproduce the claim violation.
    let non_reproducing = fsl_replay::replay(
        hybrid_replay_request(
            &braking_pair(),
            point_counterexample_with_acceleration(1, 20),
        ),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("a valid trace that does not violate TooSlow settles as a row");
    let non_reproducing_wire: Value = serde_json::from_str(
        &non_reproducing
            .to_function_json()
            .expect("the inconclusive hybrid result serializes"),
    )
    .expect("the result row is JSON");
    assert_eq!(
        non_reproducing.category(),
        filament_semantic_value::catalog::Category::Inconclusive
    );
    assert_eq!(non_reproducing_wire["settlement"], "inconclusive");
    assert_eq!(
        non_reproducing_wire["replay"]["exact"]["outcome"],
        "not-reproduced"
    );
    assert_eq!(
        non_reproducing_wire["disagreement"]["cause"],
        "hybrid-trace"
    );
    assert_eq!(
        non_reproducing_wire["disagreement"]["disagreement"]["type"],
        "not-reproduced"
    );
}

/// FR-750-AC-11 (process half): a process `failed` `tool-failure` entry is
/// `NotRead` and the item settles `failed` by FR-290.
#[test]
#[trace("TC-3035", "FR-750-AC-11")]
fn a_process_failed_entry_settles_failed_by_fr_290() {
    let entry = json!({"request_index": 0, "result": "failed",
        "cause": {"kind": "tool-failure", "defect": "crash"}});
    let unit = braking(&claim("Positive", "always [0, 12] holds(h.v > 0)"));
    let (_, item) = first_item(
        "hybrid-750-11",
        &unit,
        named("Positive", "hybrid-process"),
        json!({}),
        vec![process_fixture("hybrid-process", &entry)],
    );
    assert_eq!(item["record"]["result"], "failed", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "tool-failure", "{item}");
    assert_eq!(item["record"]["cause"]["defect"], "crash", "{item}");
}

/// FR-1021-AC-3: the plan's `Fill`/`Drain` validation item reaches the
/// reference provider with its dwell and horizon as `Rational` objects, and a
/// trace bound written as `"8"` refuses `invalid_result`/`wrong-value-kind`.
#[test]
#[trace("TC-3035", "FR-1021-AC-3")]
#[ignore = "QSL-2534: the plan's validation items over the reference provider land with the reference solver"]
fn a_plan_validation_item_carries_rational_objects() {
    todo!("QSL-2534: FR-1021-AC-3");
}

/// FR-750-AC-12 (exact-replay half): over `Shuttle`, a lasso trace for an
/// `on each` claim that reads its activation state, and for `fair` claims
/// over a transition with a non-deterministic reset, reproduces or does not
/// reproduce by exact replay instead of refusing.
#[test]
#[trace("TC-3037", "FR-750-AC-12")]
#[ignore = "QSL-2529: exact replay of activation-state and non-deterministic-reset claims (tau as a second real variable, quantifier elimination) is not implemented; those gaps still settle `NotExact`"]
fn exact_replay_decides_activation_state_and_nondeterministic_reset_claims() {
    todo!("QSL-2529: FR-450 exact replay of the Shuttle claims");
}

/// FR-750-AC-12 (replay-request half): the same refusals on FR-098's replay
/// path: `ReplayRefusal::HybridTrace` at `/loop` and exit 30 with no row for
/// the three package faults.
#[test]
#[trace("TC-3037", "FR-750-AC-12")]
fn a_replay_request_settles_each_hybrid_refusal_by_its_row() {
    let unit = braking_pair();
    let mut invalid = point_counterexample();
    invalid.trace.loop_to = Some(1);
    let error = fsl_replay::replay(
        hybrid_replay_request(&unit, invalid),
        fsl_replay::ReplayLimits::default(),
    )
    .expect_err("a malformed caller trace refuses before a result row");
    assert_eq!(error.code(), fsl_foundation::Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
    assert_eq!(error.pointer().as_deref(), Some("/loop"));
    let fsl_replay::ReplayRefusal::HybridTrace(trace) = error else {
        panic!("the FR-193 trace refusal has its typed FR-098 mapping: {error}");
    };
    assert_eq!(trace.pointer.to_string(), "/loop");
    assert_eq!(trace.code(), "invalid_result");
    assert_eq!(trace.cause(), "invalid-value");
}

/// FR-750-AC-14 (wire half): `to_wire` writes the four undefined causes and
/// the reader reads each back to the same variant with its payload taken from
/// the term `locus` names; `undefined` refuses at `/kind/cause`; an
/// irrational instant is claimed through its isolating interval.
#[test]
#[trace("TC-3037", "FR-750-AC-14")]
#[trace("FSpec-TC-2051", "FSpec-FR-193-AC-26")]
#[ignore = "QSL-2533: the plugin reader does not read an undefined-evaluation counterexample back yet"]
fn the_four_undefined_causes_round_trip_through_the_wire() {
    todo!("QSL-2533: undefined-evaluation read-back");
}
