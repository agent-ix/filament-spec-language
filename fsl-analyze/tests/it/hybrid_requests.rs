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
//! The in-process fixture providers are made by `Engine::hybrid` (FR-750).
//! The process providers (`Engine::hybrid_process`) run as a real child process
//! and are tested in `tools/fsl-hybrid-test-provider`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use filament_exact::{Integer, Rational, ScalarLimits};
use fsl_analyze::analyze::Engine;
use fsl_analyze::provider::{HybridAnswer, HybridProvider, HybridSolveRequest, HybridVerdict};
use fsl_foundation::SourceIdentity;
use fsl_replay::{ExactReplay, HybridReplayMeter};
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
                .filter_map(|name| name.as_str().map(|name| (name.to_owned(), String::new()))),
        )
    })
    .map_err(|refusal| format!("{refusal:?}"))
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
#[trace("TC-1187", "FR-447-AC-4")]
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
            end: None,
        },
    }
}

fn hybrid_replay_request(
    unit: &str,
    counterexample: fsl_replay::HybridCounterexample,
) -> fsl_replay::ReplayRequestWire {
    hybrid_replay_request_for(unit, "TooSlow", counterexample)
}

/// The replay request of the claim `claim` of `unit` carrying
/// `counterexample`.
fn hybrid_replay_request_for(
    unit: &str,
    claim: &str,
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
        certificate: None,
        package: fsl_replay::ReplayPackage::read(
            compiled.bytes(),
            fsl_replay::ReplayLimits::default().input_bytes,
        )
        .expect("the emitted package is a replay package"),
        profile_selections: Vec::new(),
        dependencies: Vec::new(),
        replay_target: fsl_replay::RequestTargetWire::state_clause(
            fsl_replay::QualifiedName::new(vec![
                filament_exact::Identifier::new(claim).expect("the claim name is an identifier")
            ])
            .expect("the claim name is qualified"),
        ),
        source: fsl_replay::ReplaySource::Hybrid(Box::new(counterexample)),
        obligation_identity: crate::fixture_ids::obligation_identity(),
        backend: "hybrid-fixture".to_owned(),
        state_environment: fsl_replay::StateEnvironment::new(Vec::new()),
        accounting_limits: ScalarLimits {
            work_units: 100_000,
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

/// FR-751-AC-1 and FSpec FR-652-AC-1: an accepted `inductive-enclosure`
/// settles `certified`; one excluding the initial `v = 20` is rejected at
/// rule (a) at the `Moving` box and stays `uncertified`.
#[test]
#[trace("TC-2101", "FR-751-AC-1")]
#[trace("FSpec-TC-2049", "FSpec-FR-652-AC-6")]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-1")]
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
fn a_mismatched_certificate_form_refuses_at_certificate() {
    let mut entry = inductive_entry("20");
    entry["certificate"] = json!({"type": "flowpipe", "steps": []});
    let unit = braking(&claim("Bounded", "always holds(h.v < 30)"));
    let (fixture, _) = answering("hybrid-fixture", move |request| {
        let read =
            HybridAnswer::read_entry(&entry, &[], request.graph, request.claim, request.wire);
        assert!(
            matches!(
                &read,
                Err(fsl_replay::HybridEntryRefusal::Entry(
                    fsl_replay::PluginResultRefusal::Malformed { pointer, .. }
                )) if pointer == "/certificate"
            ),
            "{read:?}"
        );
        let mut base = entry.clone();
        base.as_object_mut().expect("object").remove("certificate");
        let mut answer = entry_answer(base)(request);
        answer.certificate = entry.get("certificate").cloned();
        answer
    });
    let (_, item) = first_item(
        "hybrid-652-4",
        &unit,
        named("Bounded", "hybrid-fixture"),
        json!({}),
        vec![fixture],
    );
    let text = item.to_string();
    assert!(text.contains("invalid_result"), "{item}");
    assert!(text.contains("/certificate"), "{item}");
}

/// FSpec FR-652-AC-3's `Grow`: `v' = 1 - 1 / (1 + v * v)` is not a
/// polynomial flow, and its interval value over `[a, b]` with `a >= 1` is
/// exactly `[f(a), f(b)]`, so narrow boxes stay narrow.
const GROW: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Grow using p {\n\
       initial { mode Up when { v = 1 }; }\n\
       mode Up invariant { v >= 1 and v <= 100 } flow { v' = 1 - 1 / (1 + v * v); }\n\
     }\n\
     temporal Below using p over (h: Grow) clock \"model-time\" \
     on origin { always [0, 40] holds(h.v < 30) }\n";

/// The nine-step `validated-witness` of FSpec FR-652-AC-3, cut to its
/// first `steps` steps: step `k` spans `[4k, 4k + 4]` with `box`
/// `v [l_k, 1 + 4k]` and a-priori box `v [l_k, 5 + 4k]`. Each `l_{k+1}` is
/// at most `l_k + 4 * f(l_k)`, `f(v) = 1 - 1 / (1 + v * v)` being increasing
/// and below 1, so every step satisfies the Picard condition and its
/// successor lies in the next box; step 8's `box` `v [30, 33]` at time 32 is
/// the decisive box.
fn grow_witness(steps: usize) -> Value {
    const LOWER: [&str; 9] = [
        "1", "3", "33/5", "21/2", "72/5", "183/10", "111/5", "261/10", "30",
    ];
    let dwell: Vec<Value> = LOWER
        .iter()
        .take(steps)
        .enumerate()
        .map(|(k, lower)| {
            let k = u32::try_from(k).expect("nine steps");
            json!({"mode": "Up",
                "time": interval(&(4 * k).to_string(), &(4 * k + 4).to_string()),
                "step": rational("4"),
                "box": {"v": interval(lower, &(1 + 4 * k).to_string())},
                "apriori": {"v": interval(lower, &(5 + 4 * k).to_string())}})
        })
        .collect();
    json!({"type": "validated-witness", "dwells": [dwell]})
}

/// FSpec FR-652-AC-3 and FR-193-AC-22: a point trace through a mode whose
/// flow is not polynomial refutes with an accepted `validated-witness` whose
/// decisive box makes the claim definitely false, and stays `inconclusive`
/// without one or with a witness whose boxes only straddle `v = 30`.
#[test]
#[trace("FSpec-TC-1605", "FSpec-FR-652-AC-3")]
#[trace("FSpec-TC-1605", "FSpec-FR-193-AC-22")]
fn a_validated_witness_refutes_through_a_non_polynomial_flow() {
    let trace = json!({"start": {"mode": "Up", "time": interval("0", "0"),
        "enclosure": {"v": interval("1", "1")}}, "jumps": []});
    let witness = grow_witness(9);
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
        GROW,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(Some(&witness)))],
    );
    assert_eq!(with["record"]["result"], "refuted", "{with}");
    assert_eq!(with["certificate"]["accepted"], true, "{with}");
    let (_, without) = first_item(
        "hybrid-652-3b",
        GROW,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(None))],
    );
    assert_eq!(without["record"]["result"], "inconclusive", "{without}");
    // Cut after the step at [28, 32]: its `B` `v [261/10, 33]` straddles 30.
    let cut = grow_witness(8);
    // One step at [0, 40]: `B` `v [1, 41]` satisfies Picard but straddles 30.
    let single = json!({"type": "validated-witness", "dwells": [[{"mode": "Up",
        "time": interval("0", "40"), "step": rational("40"),
        "box": {"v": interval("1", "1")}, "apriori": {"v": interval("1", "41")}}]]});
    for (name, straddling, at) in [
        ("hybrid-652-3c", &cut, "/certificate/dwells/0/7"),
        ("hybrid-652-3d", &single, "/certificate/dwells/0/0"),
    ] {
        let (_, item) = first_item(
            name,
            GROW,
            named("Below", "hybrid-fixture"),
            json!({}),
            vec![hybrid_fixture("hybrid-fixture", &entry(Some(straddling)))],
        );
        assert_eq!(item["record"]["result"], "inconclusive", "{item}");
        assert_eq!(item["certificate"]["accepted"], false, "{item}");
        assert_eq!(item["certificate"]["rule"], "witness-violation", "{item}");
        assert_eq!(item["certificate"]["at"], at, "{item}");
    }
}

/// FR-450-AC-11: a provider's `refuted` entry whose trace exact replay
/// cannot decide settles `refuted` when its `validated-witness` is accepted,
/// and `inconclusive`, `replay-parity` `not-exact`, when it is rejected.
#[test]
#[trace("TC-1221", "FR-450-AC-11")]
fn an_accepted_witness_settles_a_not_exact_provider_refutation() {
    let trace = json!({"start": {"mode": "Up", "time": interval("0", "0"),
        "enclosure": {"v": interval("1", "1")}}, "jumps": []});
    let entry = |witness: Value| {
        let mut entry = refuted_entry(trace.clone());
        entry["certificate"] = witness;
        entry
    };
    let (_, accepted) = first_item(
        "hybrid-450-11a",
        GROW,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(grow_witness(9)))],
    );
    assert_eq!(accepted["record"]["result"], "refuted", "{accepted}");
    assert_eq!(accepted["certificate"]["accepted"], true, "{accepted}");
    assert!(
        accepted["counterexample"]
            .to_string()
            .contains("hybrid-counterexample"),
        "{accepted}"
    );
    let (_, rejected) = first_item(
        "hybrid-450-11b",
        GROW,
        named("Below", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry(grow_witness(8)))],
    );
    assert_eq!(rejected["record"]["result"], "inconclusive", "{rejected}");
    assert_eq!(
        rejected["certificate"]["rule"], "witness-violation",
        "{rejected}"
    );
    let text = rejected.to_string();
    assert!(text.contains("replay-parity"), "{rejected}");
    assert!(text.contains("hybrid-trace"), "{rejected}");
    assert!(text.contains("not-exact"), "{rejected}");
    assert_eq!(
        rejected["record"]["cause"]["parity"]["disagreement"]["type"], "not-exact",
        "{rejected}"
    );
    // FR-450-AC-3: the disagreement retains the complete counterexample.
    assert_eq!(
        rejected["record"]["cause"]["parity"]["disagreement"]["counterexample"]["type"],
        "hybrid-counterexample",
        "{rejected}"
    );
    assert!(rejected.get("counterexample").is_none(), "{rejected}");
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
        json!({"mode": "Stopped", "from": {"dwell": 0, "transition": 0},
        "time": interval(jump_time.0, jump_time.1), "step": rational("10"), "box": {"v": interval("0", "0"), "a": interval("1", "2")},
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
fn a_trace_cut_by_end_replays_to_its_bad_prefix() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Braking using p {\n\
           mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
             flow { v' = -a; a' = 0; t' = 1; }\n\
             transition to Stopped when { v = 0 } reset { v = 0 };\n\
           mode Stopped invariant { v = 0 }\n\
             flow { v' = 0; a' = 0; t' = 1; }\n\
         }\n\
         temporal TooSlow using p over (h: Braking) clock \"model-time\" on origin {\n\
           always [0, 10] holds(h.v > 5)\n\
         }\n";
    let trace = |end_v: &str| {
        json!({"start": {"mode": "Moving", "time": interval("0", "0"),
            "enclosure": {"v": interval("20", "20"), "a": interval("2", "2"),
                "t": interval("0", "0")}},
            "jumps": [], "end": {"time": interval("8", "8"),
            "enclosure": {"v": interval(end_v, end_v), "a": interval("2", "2"),
                "t": interval("8", "8")}}})
    };
    let run = |name: &str, trace: Value| {
        let entry = refuted_entry(trace);
        let (fixture, _) = answering("hybrid-fixture", move |request| {
            let answer = entry_answer(entry.clone())(request);
            if let fsl_analyze::provider::HybridAnswer {
                verdict: HybridVerdict::Refuted(counterexample),
                ..
            } = &answer
            {
                assert!(
                    counterexample.trace.end.is_some(),
                    "end survives wire read-back"
                );
                let replay = fsl_replay::exact_replay(
                    request.graph,
                    request.claim,
                    &counterexample.trace,
                    &mut HybridReplayMeter::new(1_000_000),
                );
                let bad_prefix = counterexample
                    .trace
                    .end
                    .as_ref()
                    .is_some_and(|end| end.enclosure[0].lower == exact(4));
                match (bad_prefix, replay) {
                    (true, Ok(ExactReplay::Reproduced(_))) => {}
                    (false, Ok(ExactReplay::NotReproduced { at, .. })) => {
                        assert_eq!(at.to_string(), "/end");
                    }
                    (expected, actual) => {
                        panic!("expected bad-prefix={expected}, got {actual:?}")
                    }
                }
            }
            answer
        });
        first_item(
            name,
            unit,
            named("TooSlow", "hybrid-fixture"),
            json!({}),
            vec![fixture],
        )
        .1
    };
    let reproduced = run("hybrid-193-23a", trace("4"));
    assert_eq!(reproduced["record"]["result"], "refuted", "{reproduced}");
    let wrong = run("hybrid-193-23b", trace("5"));
    assert_eq!(wrong["record"]["result"], "inconclusive", "{wrong}");
}

/// FR-750-AC-9 (reader and irrational instant): the plugin reader reads a
/// written undefined-evaluation counterexample back to a `Refuted` answer
/// FR-450 reproduces, and an irrational instant is written as a rational
/// interval that contains it.
#[test]
#[trace("TC-3035", "FR-750-AC-9")]
fn an_undefined_evaluation_counterexample_reads_back() {
    use fsl_replay::{
        ExactInterval, ExactReplay, HybridReplayMeter, HybridTrace, TraceBefore, TraceState,
    };

    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Rising using p {\n\
           initial { mode Up when { v = 0 and a = 0 }; }\n\
           mode Up invariant { v >= 0 and v <= 2 and a = 0 } flow { v' = 1; a' = 0; }\n\
         }\n\
         temporal Undefined using p over (h: Rising) clock \"model-time\" on origin {\n\
           always [0, 2] holds(1 / (h.v * h.v - 2) != 0)\n\
         }\n";
    let point = |value| ExactInterval::point(exact(value));
    let (engine, _) = answering("hybrid-undefined-readback", move |request| {
        let claim_index = request
            .graph
            .hybrid_claims()
            .iter()
            .position(|candidate| std::ptr::eq(candidate, request.claim))
            .expect("the request claim belongs to its graph");
        let trace = HybridTrace {
            start: TraceState {
                mode: ModeIndex::new(0),
                time: point(0),
                enclosure: vec![point(0), point(0)],
            },
            jumps: Vec::new(),
            delta: None,
            loop_to: None,
            end: Some(TraceBefore {
                time: point(2),
                enclosure: vec![point(2), point(0)],
            }),
        };
        let ExactReplay::Reproduced(found) = fsl_replay::exact_replay(
            request.graph,
            request.claim,
            &trace,
            &mut HybridReplayMeter::new(1_000_000),
        )
        .expect("the point trace replays exactly") else {
            panic!("the trace reaches the irrational undefined instant");
        };
        let fsl_replay::HybridCounterexampleKind::UndefinedEvaluation { at, .. } = &found.kind
        else {
            panic!("the exact replay finds an undefined evaluation");
        };
        assert!(matches!(at.time, fsl_replay::ExactReal::Root(_)));
        let locus =
            fsl_replay::hybrid_locus_wire(request.graph, request.wire, claim_index, &at.locus)
                .expect("the division has an emitted term locus");
        let wire = found.to_wire(request.model, &|_| locus.clone());
        let entry = json!({"request_index": 0, "result": "refuted", "counterexample": 0,
            "counterexamples": [wire]});
        let counterexamples = entry["counterexamples"]
            .as_array()
            .expect("the entry has a counterexample array");
        HybridAnswer::read_entry(
            &entry,
            counterexamples,
            request.graph,
            request.claim,
            request.wire,
        )
        .unwrap_or_else(|refusal| panic!("the irrational counterexample reads back: {refusal}"))
    });
    let (run, item) = first_item(
        "hybrid-undefined-readback",
        unit,
        named("Undefined", "hybrid-undefined-readback"),
        json!({}),
        vec![engine],
    );
    assert_eq!(item["record"]["result"], "refuted", "{}", run.document());
    assert_eq!(item["replay"]["reproduced"], true, "{}", run.document());
    let counterexample = item["counterexample"].to_string();
    assert!(counterexample.contains("undefined-evaluation"), "{item}");
    assert!(counterexample.contains("division-by-zero"), "{item}");
}

fn grow_counterexample() -> fsl_replay::HybridCounterexample {
    use fsl_replay::{
        ExactInterval, HybridCounterexample, HybridCounterexampleKind, HybridTrace, TraceState,
    };
    let point = |value| ExactInterval::point(exact(value));
    HybridCounterexample {
        kind: HybridCounterexampleKind::Formula,
        trace: HybridTrace {
            start: TraceState {
                mode: ModeIndex::new(0),
                time: point(0),
                enclosure: vec![point(1)],
            },
            jumps: Vec::new(),
            delta: None,
            loop_to: None,
            end: None,
        },
    }
}

fn grow_replay(certificate: Option<Value>) -> fsl_replay::ReplayRequestWire {
    let mut wire = hybrid_replay_request_for(GROW, "Below", grow_counterexample());
    wire.certificate = certificate;
    wire
}

/// FR-750-AC-15: a `Below` replay request carrying FSpec FR-652-AC-3's
/// witness at `/replay/certificate` writes the `hybrid` row
/// `reproduced-with-validated-witness`; the cut witness or none writes
/// `inconclusive` `not-exact`; the witness on a `function` request refuses
/// `unknown-member` at `/replay/certificate`.
#[test]
#[trace("TC-3035", "FR-750-AC-15")]
#[trace("FSpec-TC-2049", "FSpec-FR-323-AC-48")]
fn a_witness_refuted_hybrid_item_replays_with_validated_witness() {
    let row = |certificate: Option<Value>| -> Value {
        let result = fsl_replay::replay(
            grow_replay(certificate),
            fsl_replay::ReplayLimits::default(),
        )
        .expect("the Grow counterexample settles as a row");
        serde_json::from_str(&result.to_function_json().expect("the row serializes"))
            .expect("the row is JSON")
    };
    let accepted = row(Some(grow_witness(9)));
    assert_eq!(
        accepted["settlement"], "reproduced-with-validated-witness",
        "{accepted}"
    );
    assert_eq!(accepted["category"], "success", "{accepted}");
    assert_eq!(
        accepted["replay"]["exact"]["outcome"], "not-exact",
        "{accepted}"
    );
    assert_eq!(accepted["counterexample"]["kind"], "formula", "{accepted}");
    for certificate in [Some(grow_witness(8)), None] {
        let inconclusive = row(certificate);
        assert_eq!(inconclusive["settlement"], "inconclusive", "{inconclusive}");
        assert_eq!(
            inconclusive["replay"]["exact"]["outcome"], "not-exact",
            "{inconclusive}"
        );
        assert_eq!(
            inconclusive["disagreement"]["disagreement"]["type"], "not-exact",
            "{inconclusive}"
        );
    }
    let mut foreign = grow_replay(Some(grow_witness(9)));
    foreign.source = fsl_replay::ReplaySource::Input(Vec::new());
    let error = fsl_replay::replay(foreign, fsl_replay::ReplayLimits::default())
        .expect_err("a certificate on a non-hybrid source is refused");
    assert_eq!(error.code(), fsl_foundation::Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("unknown-member"));
    assert_eq!(error.pointer().as_deref(), Some("/replay/certificate"));
    let wrong_form = fsl_replay::replay(
        grow_replay(Some(json!({"type": "flowpipe", "steps": []}))),
        fsl_replay::ReplayLimits::default(),
    )
    .expect_err("a certificate that is not a validated-witness is refused");
    assert_eq!(wrong_form.cause(), Some("invalid-value"));
    assert_eq!(
        wrong_form.pointer().as_deref(),
        Some("/replay/certificate/type")
    );
    let mut malformed = grow_witness(9);
    malformed["dwells"][0][0]["from"] = json!({"dwell": 0, "transition": 0});
    let error = fsl_replay::replay(
        grow_replay(Some(malformed)),
        fsl_replay::ReplayLimits::default(),
    )
    .expect_err("a structurally invalid witness is refused");
    assert_eq!(error.code(), fsl_foundation::Code::InvalidRuntimeInput);
    assert_eq!(
        error.pointer().as_deref(),
        Some("/replay/certificate/dwells/0/0/from")
    );
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

/// FR-1021-AC-3: the plan's `Fill`/`Drain` validation item reaches the
/// reference provider with its dwell and horizon as `Rational` objects, and a
/// trace bound written as `"8"` refuses `invalid_result`/`wrong-value-kind`.
#[test]
#[trace("TC-3035", "FR-1021-AC-3")]
#[ignore = "QSL-2874: the hybrid plan synthesis that builds the validation item is not implemented yet"]
fn a_plan_validation_item_carries_rational_objects() {
    todo!("QSL-1978: FR-1021-AC-3");
}

/// `Shuttle` of FR-750-AC-12, its `Moving -> Stopped` reset `reset`, with
/// the claims `Whole` (`fair Moving;`) and `Named` (`fair Moving::Stopped;`),
/// each over `eventually [0, *] holds(h.v > 1)`.
fn shuttle(reset: &str) -> String {
    shuttle_with(reset, true)
}

/// `Shuttle` with the claims of [`shuttle`] (`standard`), or with the one
/// claim `Zrange` alone: capture `tau + 3/2` of type `[0, 2]` is out of range
/// for `tau` in `(1/2, 1)`.
fn shuttle_with(reset: &str, standard: bool) -> String {
    let claim = |name: &str, fairness: &str| {
        format!(
            "temporal {name} using p over (h: Shuttle) clock \"model-time\" \
             on origin {{ fair {fairness}; eventually [0, *] holds(h.v > 1) }}\n"
        )
    };
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Shuttle using p {{\n\
           initial {{ mode Moving when {{ v = 0 }}; }}\n\
           mode Moving invariant {{ v >= 0 and v <= 1 }} flow {{ v' = 1; }}\n\
             transition to Moving when {{ v = 1 }} reset {{ v = 0 }};\n\
             transition to Stopped when {{ v >= 0 }} reset {{ {reset} }};\n\
           mode Stopped invariant {{ v >= 1 and v <= 2 }} flow {{ v' = 0; }}\n\
         }}\n{}{}",
        if standard {
            claim("Whole", "Moving")
        } else {
            String::new()
        },
        if standard {
            claim("Named", "Moving::Stopped")
        } else {
            String::new()
        },
    ) + if standard {
        SHUTTLE_CAPTURES
    } else {
        SHUTTLE_RANGE
    }
}

const SHUTTLE_RANGE: &str = "temporal Zrange using p over (h: Shuttle) clock \"model-time\" \
         on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 2; 1, 2] = q.v + 3 / 2; \
         eventually [0, 1] holds(h.v + 3 / 2 >= v0) }\n";

const SHUTTLE_CAPTURES: &str = "temporal Capture using p over (h: Shuttle) clock \"model-time\" \
         on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 2; 1, 2] = q.v; \
         eventually [0, 1] holds(h.v >= v0 + 1 / 2) }\n\
         temporal CaptureHeld using p over (h: Shuttle) clock \"model-time\" \
         on each (q: Shuttle) when (q.v > 0) { capture v0: Rational[0, 2; 1, 2] = q.v; \
         eventually [0, 1] holds(h.v >= v0) }\n";

/// The point lasso `L` of `Shuttle` as a provider's `refuted` entry: from
/// `(Moving, v = 0)` at time 0 by transition 0 (`before` `(time 1, v = 1)`)
/// to `(Moving, v = 0)` at time 1, `loop_to` 0.
fn shuttle_lasso_entry() -> Value {
    let state = |time: &str, v: &str| {
        json!({"mode": "Moving", "time": interval(time, time),
            "enclosure": {"v": interval(v, v)}})
    };
    let mut entry = refuted_entry(json!({"start": state("0", "0"),
        "jumps": [{"transition": 0,
            "before": {"time": interval("1", "1"), "enclosure": {"v": interval("1", "1")}},
            "state": state("1", "0")}],
        "loop": 0}));
    entry["counterexamples"][0]["trace"]["loop"] = json!(0);
    entry
}

/// FR-750-AC-12 (exact-replay half, fairness): over `Shuttle` and its point
/// lasso `L`, `fair Moving;` over `eventually [0, *] holds(h.v > 1)` reproduces
/// by exact replay with kind `formula` and the item settles `refuted`:
/// `Moving -> Stopped` is enabled at every instant, its reset `v >= 0`
/// admitting a post-jump `v` in `[1, 2]`, and the whole constraint is taken
/// every period by the self-loop. `fair Moving::Stopped;` is enabled at every
/// instant and never taken, so `L` is unfair, the replay is `NotReproduced`
/// at check 5 and the item settles `inconclusive`, `replay-parity` with the `hybrid-trace` reason; with the
/// reset `v <= 1/2`, which no state under `Stopped`'s invariant meets, the
/// enabled set is empty and the item settles `refuted`.
#[test]
#[trace("TC-3037", "FR-750-AC-12")]
fn exact_replay_decides_nondeterministic_reset_fairness_claims() {
    let run = |name: &str, reset: &str, claim: &str| {
        let entry = shuttle_lasso_entry();
        let (fixture, _) = answering("hybrid-fixture", entry_answer(entry));
        first_item(
            name,
            &shuttle(reset),
            named(claim, "hybrid-fixture"),
            json!({}),
            vec![fixture],
        )
        .1
    };
    let fair = run("hybrid-750-12-fair", "v >= 0", "Whole");
    assert_eq!(fair["record"]["result"], "refuted", "{fair}");
    assert_eq!(fair["replay"]["reproduced"], true, "{fair}");
    let unfair = run("hybrid-750-12-unfair", "v >= 0", "Named");
    assert_eq!(unfair["record"]["result"], "inconclusive", "{unfair}");
    assert_eq!(
        unfair["record"]["cause"]["kind"], "replay-parity",
        "{unfair}"
    );
    assert_eq!(
        unfair["record"]["cause"]["parity"]["reason"], "hybrid-trace",
        "{unfair}"
    );
    assert_eq!(
        unfair["record"]["cause"]["parity"]["disagreement"]["type"], "not-reproduced",
        "{unfair}"
    );
    let never_enabled = run("hybrid-750-12-never", "v <= 1/2", "Named");
    assert_eq!(
        never_enabled["record"]["result"], "refuted",
        "{never_enabled}"
    );
}

/// FR-750-AC-12 (exact-replay half, activation state): over `Shuttle` and
/// its point lasso `L`, the capture claim whose obligation at `tau` needs
/// `h.v >= tau + 1/2` within `[tau, tau + 1]` is false for `tau` in
/// `(1/2, 1)`: it reproduces with kind `formula`, the item settles
/// `refuted` with one counterexample and the replay row is
/// `reproduced-with-evaluated-witness`; with body `h.v >= v0`, true for
/// every `tau`, the replay is `NotReproduced` at check 5 and the item
/// settles `inconclusive`. (The checker requires a range on a capture's
/// `Rational` type, so the fixture writes `Rational[0, 2; 1, 2]`.)
#[test]
#[trace("TC-3037", "FR-750-AC-12")]
fn exact_replay_decides_activation_state_claims() {
    let run = |name: &str, claim: &str| {
        let (fixture, _) = answering("hybrid-fixture", entry_answer(shuttle_lasso_entry()));
        first_item(
            name,
            &shuttle("v >= 0"),
            named(claim, "hybrid-fixture"),
            json!({}),
            vec![fixture],
        )
        .1
    };
    let refuted = run("hybrid-750-12-capture", "Capture");
    assert_eq!(refuted["record"]["result"], "refuted", "{refuted}");
    assert_eq!(refuted["replay"]["reproduced"], true, "{refuted}");
    assert_eq!(refuted["counterexample"]["kind"], "formula", "{refuted}");
    assert!(refuted["counterexample"].is_object(), "{refuted}");
    let held = run("hybrid-750-12-capture-held", "CaptureHeld");
    assert_eq!(held["record"]["result"], "inconclusive", "{held}");
    assert_eq!(held["record"]["cause"]["kind"], "replay-parity", "{held}");
    assert_eq!(
        held["record"]["cause"]["parity"]["reason"], "hybrid-trace",
        "{held}"
    );
    // The replay request path writes `reproduced-with-evaluated-witness`.
    let counterexample = fsl_replay::HybridCounterexample {
        kind: fsl_replay::HybridCounterexampleKind::Formula,
        trace: shuttle_lasso_trace(),
    };
    let result = fsl_replay::replay(
        hybrid_replay_request_for(&shuttle("v >= 0"), "Capture", counterexample),
        fsl_replay::ReplayLimits::default(),
    )
    .expect("the hybrid counterexample replays");
    let wire: Value = serde_json::from_str(
        &result
            .to_function_json()
            .expect("the hybrid result serializes"),
    )
    .expect("the result row is JSON");
    assert_eq!(wire["settlement"], "reproduced-with-evaluated-witness");
}

/// `Shuttle`'s point lasso `L` as a trace.
fn shuttle_lasso_trace() -> fsl_replay::HybridTrace {
    use fsl_replay::{ExactInterval, HybridTrace, TraceBefore, TraceJump, TraceState};
    let point = |value| ExactInterval::point(exact(value));
    let state = |time, v| TraceState {
        mode: ModeIndex::new(0),
        time: point(time),
        enclosure: vec![point(v)],
    };
    HybridTrace {
        start: state(0, 0),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(TraceBefore {
                time: point(1),
                enclosure: vec![point(1)],
            }),
            state: state(1, 0),
        }],
        delta: None,
        loop_to: Some(0),
        end: None,
    }
}

/// FR-450-AC-12 (a), provider half: over `Shuttle` and `L`, a provider's
/// `refuted` entry of kind `undefined-evaluation` for the capture
/// `tau + 1/2` of type `[0, 1]` (out of range on `(1/2, 1)`) with
/// `where.time` `[1/2, 1/2]`, the reproduced infimum, settles `refuted`;
/// with `where.time` `[3/4, 3/4]`, which does not hold the instant, the
/// replay is `NotReproduced` at check 5 and the item settles `inconclusive`.
#[test]
#[trace("TC-1221", "FR-450-AC-12")]
fn a_provider_entry_is_settled_by_the_reproduced_infimum() {
    let run = |name: &str, time: &'static str| {
        let (fixture, _) = answering("hybrid-fixture", move |request| {
            let claim_index = request
                .graph
                .hybrid_claims()
                .iter()
                .position(|candidate| std::ptr::eq(candidate, request.claim))
                .expect("the request claim belongs to its graph");
            let ExactReplay::Reproduced(found) = fsl_replay::exact_replay(
                request.graph,
                request.claim,
                &shuttle_lasso_trace(),
                &mut HybridReplayMeter::new(1_000_000),
            )
            .expect("the point lasso replays exactly") else {
                panic!("the capture is out of range for some activation instant");
            };
            let fsl_replay::HybridCounterexampleKind::UndefinedEvaluation { at, .. } = &found.kind
            else {
                panic!("the exact replay finds an undefined evaluation");
            };
            let locus =
                fsl_replay::hybrid_locus_wire(request.graph, request.wire, claim_index, &at.locus)
                    .expect("the capture has an emitted locus");
            let mut wire = found.to_wire(request.model, &|_| locus.clone());
            wire["kind"]["where"]["time"] = interval(time, time);
            let entry = json!({"request_index": 0, "result": "refuted", "counterexample": 0,
                "counterexamples": [wire]});
            let counterexamples = entry["counterexamples"]
                .as_array()
                .expect("the entry has a counterexample array");
            HybridAnswer::read_entry(
                &entry,
                counterexamples,
                request.graph,
                request.claim,
                request.wire,
            )
            .unwrap_or_else(|refusal| panic!("the counterexample reads back: {refusal}"))
        });
        first_item(
            name,
            &shuttle_with("v >= 0", false),
            named("Zrange", "hybrid-fixture"),
            json!({}),
            vec![fixture],
        )
        .1
    };
    let at_infimum = run("hybrid-450-12-infimum", "1/2");
    assert_eq!(at_infimum["record"]["result"], "refuted", "{at_infimum}");
    let elsewhere = run("hybrid-450-12-elsewhere", "3/4");
    assert_eq!(elsewhere["record"]["result"], "inconclusive", "{elsewhere}");
    assert_eq!(
        elsewhere["record"]["cause"]["kind"], "replay-parity",
        "{elsewhere}"
    );
}

/// FR-450-AC-12 (d), provider half: over `Phase` and its point lasso
/// (`loop_to` 1), a provider's `refuted` trace for `on each (q: Phase)
/// { once [0, 1] once [0, 1] holds(h.w = 1) }` reproduces with kind
/// `formula` (past reach 2, three periods decided) and settles `refuted`.
#[test]
#[trace("TC-1221", "FR-450-AC-12")]
fn a_past_reach_of_two_periods_settles_refuted() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Phase using p {\n\
           initial { mode Warm when { x = 0 and w = 1 }; }\n\
           mode Warm invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; }\n\
             transition to Cold when { x = 1 } reset { x = 0 and w = 0 };\n\
           mode Cold invariant { x >= 0 and x <= 1 } flow { x' = 1; w' = 0; }\n\
             transition to Cold when { x = 1 } reset { x = 0 and w = 0 };\n\
         }\n\
         temporal Twice using p over (h: Phase) clock \"model-time\" \
           on each (q: Phase) { once [0, 1] once [0, 1] holds(h.w = 1) }\n";
    let state = |mode: &str, time: &str, w: &str| {
        json!({"mode": mode, "time": interval(time, time),
            "enclosure": {"x": interval("0", "0"), "w": interval(w, w)}})
    };
    let before = |time: &str, w: &str| {
        json!({"time": interval(time, time),
            "enclosure": {"x": interval("1", "1"), "w": interval(w, w)}})
    };
    let entry = refuted_entry(json!({"start": state("Warm", "0", "1"),
        "jumps": [
            {"transition": 0, "before": before("1", "1"), "state": state("Cold", "1", "0")},
            {"transition": 0, "before": before("2", "0"), "state": state("Cold", "2", "0")}],
        "loop": 1}));
    let (fixture, _) = answering("hybrid-fixture", entry_answer(entry));
    let item = first_item(
        "hybrid-450-12-phase",
        unit,
        named("Twice", "hybrid-fixture"),
        json!({}),
        vec![fixture],
    )
    .1;
    assert_eq!(item["record"]["result"], "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
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

/// A structurally valid `Braking` `validated-witness` for [`point_counterexample`]
/// whose first `box` misses the trace's start `v = 20`.
fn braking_witness_missing_start() -> Value {
    let moving = json!({"v": interval("0", "1"), "a": interval("1", "2")});
    let stopped = json!({"v": interval("0", "0"), "a": interval("1", "2")});
    json!({"type": "validated-witness", "dwells": [
        [{"mode": "Moving", "time": interval("0", "1"), "step": rational("1"),
            "box": moving.clone(), "apriori": moving}],
        [{"mode": "Stopped", "time": interval("10", "11"), "step": rational("1"),
            "box": stopped.clone(), "apriori": stopped}]]})
}

/// FR-750-AC-16: exact replay settles before any witness; a malformed
/// witness refuses at its `/replay/certificate/...` pointer; a witness check
/// the `work_units` bound stops writes the `incomplete` row; and the analyze
/// path records a certificate it does not rest on unchecked.
#[test]
#[trace("TC-3035", "FR-750-AC-16")]
fn exact_replay_settles_before_any_witness_and_a_bad_witness_refuses() {
    let row = |wire: fsl_replay::ReplayRequestWire| -> Value {
        let result = fsl_replay::replay(wire, fsl_replay::ReplayLimits::default())
            .expect("the request settles as a row");
        serde_json::from_str(&result.to_function_json().expect("the row serializes"))
            .expect("the row is JSON")
    };
    let mut reproduced = hybrid_replay_request(&braking_pair(), point_counterexample());
    reproduced.certificate = Some(braking_witness_missing_start());
    let wire = row(reproduced);
    assert_eq!(
        wire["settlement"], "reproduced-with-evaluated-witness",
        "{wire}"
    );
    let mut off = hybrid_replay_request(
        &braking_pair(),
        point_counterexample_with_acceleration(1, 20),
    );
    off.certificate = Some(braking_witness_missing_start());
    let wire = row(off);
    assert_eq!(wire["settlement"], "inconclusive", "{wire}");
    assert_eq!(
        wire["replay"]["exact"]["outcome"], "not-reproduced",
        "{wire}"
    );

    let refuse = |certificate: Value| {
        fsl_replay::replay(
            grow_replay(Some(certificate)),
            fsl_replay::ReplayLimits::default(),
        )
        .expect_err("the witness is refused before any row")
    };
    let mut step = grow_witness(9);
    step["dwells"][0][0]["step"] = json!("4");
    let error = refuse(step);
    assert_eq!(error.code(), fsl_foundation::Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("wrong-value-kind"));
    assert_eq!(
        error.pointer().as_deref(),
        Some("/replay/certificate/dwells/0/0/step")
    );
    let mut second = grow_witness(9);
    second["dwells"] = json!([second["dwells"][0].clone(), second["dwells"][0].clone()]);
    let error = refuse(second);
    assert_eq!(error.cause(), Some("invalid-value"));
    assert_eq!(
        error.pointer().as_deref(),
        Some("/replay/certificate/dwells")
    );

    let mut limited = grow_replay(Some(grow_witness(9)));
    limited.accounting_limits.work_units = 40;
    let wire = row(limited);
    assert_eq!(wire["category"], "incomplete", "{wire}");

    // A `bounded-solver` proof FR-193 settles `insufficient-horizon` records
    // its flowpipe unchecked: `form` and `identity`, no `accepted`.
    let mut entry = braking_flowpipe(("0", "10"));
    entry["basis"]["horizon"] = rational("1");
    let (_, item) = first_item(
        "hybrid-750-16",
        &braking_pair(),
        named("SlowEnough", "hybrid-fixture"),
        json!({}),
        vec![hybrid_fixture("hybrid-fixture", &entry)],
    );
    assert_ne!(item["record"]["certification"], "certified", "{item}");
    let certificate = item["certificate"]
        .as_object()
        .unwrap_or_else(|| panic!("the certificate is carried: {item}"));
    assert!(certificate.contains_key("form"), "{item}");
    assert!(certificate.contains_key("identity"), "{item}");
    assert!(!certificate.contains_key("accepted"), "{item}");
}
