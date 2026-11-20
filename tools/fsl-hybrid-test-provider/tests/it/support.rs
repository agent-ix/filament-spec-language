// SPDX-License-Identifier: AGPL-3.0-or-later
//! The harness of the process-provider tests: a request document over a
//! hybrid unit, the FSpec FR-290 negotiation double, and the engine of a
//! scripted process provider behind the [`crate::host::StdioHost`].

use std::path::{Path, PathBuf};

use fsl_analyze::analyze::{BackendTool, Engine};
use fsl_analyze::provider::{ProviderLimits, ProviderOptions};
use serde_json::{json, Value};

use crate::host::StdioHost;

/// The provider executable this package builds.
pub const PROVIDER: &str = env!("CARGO_BIN_EXE_fsl-hybrid-test-provider");

/// The backend identity of the process fixture.
pub const BACKEND: &str = "hybrid-process";

/// A fresh directory holding one request's files.
pub struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    pub fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "fsl-hybrid-process-{name}-{}-{serial}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self { dir }
    }

    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes).expect("the file is written");
        path
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

/// The FSpec FR-290 settlement the driver's `negotiate_*` gives an item.
struct Fr290;

impl fsl_analyze::analyze::document::Negotiate for Fr290 {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::routing::Disposition;
        use fsl_semantics::check::NegotiationOutcome;
        match candidates {
            fsl_route::CandidateOutcome::UnknownBackend(backend) => (
                Disposition::InvalidRequest,
                Some(Diagnostic {
                    code: NegotiationOutcome::UnknownBackend.code(),
                    payload: Payload::Backend(backend.clone()),
                }),
            ),
            fsl_route::CandidateOutcome::Candidates(set) => match set.candidates() {
                [candidate] => (Disposition::Supported(candidate.clone()), None),
                [] => (
                    Disposition::Unsupported,
                    Some(Diagnostic {
                        code: NegotiationOutcome::UnsupportedRequestedCapability.code(),
                        payload: Payload::Kind(item.kind),
                    }),
                ),
                _ => (
                    Disposition::InvalidRequest,
                    Some(Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code())),
                ),
            },
        }
    }
}

/// FR-447's `Braking`; `claims` follow the declaration.
pub fn braking(claims: &str) -> String {
    braking_with("v' = -a;", claims)
}

/// `Braking` whose `Moving` flow of `v` is `rate`.
pub fn braking_with(rate: &str, claims: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Braking using p {{\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ {rate} a' = 0; }}\n\
             transition to Stopped when {{ v = 0 }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ v' = 0; a' = 0; }}\n\
         }}\n{claims}"
    )
}

/// A `model-time` temporal claim `name` over `Braking` with `formula`.
pub fn claim(name: &str, formula: &str) -> String {
    format!(
        "temporal {name} using p over (h: Braking) clock \"model-time\" \
         on origin {{ {formula} }}\n"
    )
}

pub fn rational(text: &str) -> Value {
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    json!({"numerator": numerator, "denominator": denominator})
}

pub fn interval(lower: &str, upper: &str) -> Value {
    json!({"lower": rational(lower), "upper": rational(upper)})
}

/// FR-450-AC-2's point trace from `(Moving, v = 20)` to `(Stopped, v = 0)`
/// at time 10 whose `before` box holds `v = before_v`.
pub fn point_trace(before_v: &str) -> Value {
    let state = |mode: &str, time: &str, v: &str| {
        json!({"mode": mode, "time": interval(time, time),
            "enclosure": {"v": interval(v, v), "a": interval("2", "2")}})
    };
    json!({"start": state("Moving", "0", "20"), "jumps": [{"transition": 0,
        "before": {"time": interval("10", "10"),
            "enclosure": {"v": interval(before_v, before_v), "a": interval("2", "2")}},
        "state": state("Stopped", "10", "0")}]})
}

/// The FSpec FR-331 outcome stage whose one result entry answers item 0 with
/// `entry` (its `request_index`, `backend` and `kind` filled in) and whose
/// `counterexamples` hold `counterexamples`.
pub fn outcome(mut entry: Value, counterexamples: &[Value]) -> Value {
    entry["request_index"] = json!(0);
    entry["backend"] = json!(BACKEND);
    entry["kind"] = json!("temporal-satisfaction");
    let found: Vec<Value> = counterexamples
        .iter()
        .map(|found| json!({"request_index": 0, "backend": BACKEND, "counterexample": found}))
        .collect();
    stage(vec![entry], found)
}

/// An FSpec FR-331 outcome stage around `results` and `counterexamples`.
pub fn stage(results: Vec<Value>, counterexamples: Vec<Value>) -> Value {
    json!({"contract_version": "filament.backend-provider/v1",
        "capability_vocabulary": "filament.capability-kind/v1",
        "dispositions": [], "artifacts": [], "results": results,
        "counterexamples": counterexamples,
        "accounting": [{"request_index": 0, "settled_by": "result"}]})
}

/// A `refuted` outcome carrying the formula counterexample over `trace`.
pub fn refuted(trace: Value) -> Value {
    outcome(
        json!({"result": "refuted", "counterexample": 0}),
        &[json!({"type": "hybrid-counterexample", "kind": "formula", "trace": trace})],
    )
}

/// The descriptor of the process fixture `id`.
pub fn descriptor(id: &str) -> fsl_route::BackendDescriptor {
    fsl_route::descriptor_from_manifest_with_dynamics(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(id)),
        fsl_route::ProviderOrigin::Process,
        [
            (Some("temporal-satisfaction"), Some("bounded")),
            (Some("temporal-satisfaction"), Some("unbounded")),
        ],
        fsl_route::ManifestMembers {
            domains: Some(&["integer"]),
            fairness_kinds: Some(fsl_route::FairnessKindsMember::List(&[])),
            ..fsl_route::ManifestMembers::default()
        },
        Some(fsl_route::DynamicsMember::List(&["hybrid"])),
        Some(fsl_route::SolverBounds {
            max_solver_steps: Some(1000),
            max_solver_time: Some(1),
        }),
    )
    .expect("the process descriptor registers")
    .with_options([("step".to_owned(), String::new())])
}

/// The process engine `id` that runs `script` on [`PROVIDER`] under `limits`.
pub fn engine_at(id: &str, program: &Path, script: &Path, limits: ProviderLimits) -> Engine {
    Engine::hybrid_process(
        descriptor(id),
        BackendTool::new("fsl-hybrid-test-provider", "1").expect("a tool"),
        Box::new(StdioHost::new(
            program.to_owned(),
            script.to_owned(),
            id,
            limits,
        )),
    )
    .expect("the process engine is made")
}

/// The run's default provider limits.
pub fn limits() -> ProviderLimits {
    ProviderOptions::default().provider_limits
}

/// Write `script` and return the process engine running it.
pub fn process_engine(ws: &Workspace, script: &Value, limits: ProviderLimits) -> Engine {
    let path = ws.write("script.json", script.to_string().as_bytes());
    engine_at(BACKEND, Path::new(PROVIDER), &path, limits)
}

/// [`run`] under `cancel`, which the caller trips while the provider runs.
pub fn run_with_cancel(
    ws: &Workspace,
    unit: &str,
    item: Value,
    engines: Vec<Engine>,
    limits: ProviderLimits,
    cancel: &filament_exact::Cancel,
) -> Value {
    settle(ws, unit, item, engines, limits, cancel)
}

/// The first outcome item of the request over `unit` naming `claim` and
/// `backend`, with `options` on the item, settled with `engines` under
/// `limits`.
pub fn run(
    ws: &Workspace,
    unit: &str,
    item: Value,
    engines: Vec<Engine>,
    limits: ProviderLimits,
) -> Value {
    settle(
        ws,
        unit,
        item,
        engines,
        limits,
        &filament_exact::Cancel::new(),
    )
}

fn settle(
    ws: &Workspace,
    unit: &str,
    item: Value,
    engines: Vec<Engine>,
    limits: ProviderLimits,
    cancel: &filament_exact::Cancel,
) -> Value {
    ws.write("unit.fsl", unit.as_bytes());
    let document = json!({"format": "native-analyze/1", "request": {
        "program": {"source": {"file": "unit.fsl", "authority": "test",
            "identity": "hybrid-unit", "document": "HybridUnit", "formal_revision": 1}},
        "items": [item]}});
    let request = ws.write("analyze.json", document.to_string().as_bytes());
    let bytes = match fsl_command::command::analyze_with_providers(
        &request,
        &Fr290,
        engines,
        &ProviderOptions {
            provider_limits: limits,
        },
        cancel,
    ) {
        Ok(result) => result.value.to_string().into_bytes(),
        Err(error) => error
            .outcome_document(fsl_replay::Operation::Analyze)
            .to_bytes()
            .expect("the outcome document serializes"),
    };
    let document: Value = serde_json::from_slice(&bytes).expect("one JSON document");
    document["items"][0].clone()
}
