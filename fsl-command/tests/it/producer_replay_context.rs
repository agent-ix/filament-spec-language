// SPDX-License-Identifier: AGPL-3.0-or-later
//! The public source producer and native context reader use the actual hybrid trace.

use filament_exact::Cancel;
use fsl_analyze::provider::ProviderOptions;
use fsl_command::command::{self, RunLimits};
use fsl_replay::replay_context::{accounting_wire, EffectiveLimits, ReconstructedContext};
use fsl_replay::{HybridReplaySettlement, ReplayResult};
use ix_trace_rs::trace;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct Produced {
    items: Vec<ProducedItem>,
}

#[derive(Deserialize)]
struct ProducedItem {
    replay_context: Box<RawValue>,
}

#[derive(Serialize)]
struct ReplayInput<'a> {
    format: &'static str,
    context: &'a RawValue,
    effective_limits: EffectiveLimits,
}

/// FR-621-AC-18, FR-621-AC-19: serialization retains the actual proving inputs,
/// and a separate consumer meter reproduces the original hybrid violation.
#[test]
#[trace("FR-621-AC-18", "FR-621-AC-19")]
fn source_backed_hybrid_context_replays_original_counterexample() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/hybrid-braking/too-slow.analyze.json");
    let produced = command::analyze_with_providers_and_limits(
        &path,
        RunLimits::default(),
        &crate::negotiation::Fr290,
        vec![fsl_analyze::analyze::Engine::hybrid_reference().expect("native reference provider")],
        &ProviderOptions::default(),
        &Cancel::new(),
    )
    .expect("the actual hybrid source request runs");
    assert_eq!(produced.exit_code, 10, "{}", produced.value);
    assert_eq!(
        produced.value.as_value()["items"][0]["record"]["result"],
        "refuted"
    );
    let stdout = produced
        .value
        .to_bytes()
        .expect("raw native producer output");
    let rows: Produced =
        serde_json::from_slice(&stdout).expect("producer includes its retained context");
    let context = &rows.items[0].replay_context;
    let original: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(context.get()).unwrap();
    let forwarded = serde_json::to_vec(&ReplayInput {
        format: fsl_command::replay_context::FORMAT,
        context,
        effective_limits: EffectiveLimits {
            replay_input_bytes: "16777216".to_owned(),
            accounting_limits: accounting_wire(fsl_replay::AccountingLimits::default().0),
            stage_limits: Default::default(),
        },
    })
    .unwrap();
    let prepared =
        fsl_command::replay_context::read(&forwarded).expect("public native context reader");
    assert_eq!(
        prepared.context.request.package.get(),
        original.request.package.get(),
        "authenticated package lexical bytes survive forwarding"
    );
    let result = ReconstructedContext::prepare(prepared.context, prepared.common, prepared.limits)
        .expect("genuine source reconstruction")
        .replay_hybrid()
        .expect("native hybrid admission and replay");
    let ReplayResult::Hybrid(result) = result else {
        panic!("native hybrid result arm")
    };
    assert!(
        matches!(result.settlement, HybridReplaySettlement::Reproduced { .. }),
        "{:?}",
        result.settlement
    );
}

fn hybrid_context() -> Box<RawValue> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../examples/hybrid-braking/too-slow.analyze.json");
    let produced = command::analyze_with_providers_and_limits(
        &path,
        RunLimits::default(),
        &crate::negotiation::Fr290,
        vec![fsl_analyze::analyze::Engine::hybrid_reference().unwrap()],
        &ProviderOptions::default(),
        &Cancel::new(),
    )
    .expect("actual hybrid source producer");
    assert_eq!(produced.exit_code, 10);
    let document: Produced = serde_json::from_slice(&produced.value.to_bytes().unwrap()).unwrap();
    document.items.into_iter().next().unwrap().replay_context
}

fn forward(context: &RawValue, bound: &str) -> Vec<u8> {
    serde_json::to_vec(&ReplayInput {
        format: fsl_command::replay_context::FORMAT,
        context,
        effective_limits: EffectiveLimits {
            replay_input_bytes: bound.to_owned(),
            accounting_limits: accounting_wire(fsl_replay::AccountingLimits::default().0),
            stage_limits: Default::default(),
        },
    })
    .unwrap()
}

const COUNTER_SOURCE: &str = r#"language "ix:native" edition "1-draft";
profile values = "filament.value.complete/v1";
profile paths = "filament.temporal.infinite-trace/v1";
model Queue = "test/counter";
pre SpaceAvailable using values on Queue::Counter::inc { self.value < 2 }
post AppendOne using values on Queue::Counter::inc { self.value - pre(self.value) = 1 }
temporal NeverFull using paths over (q: Queue::Counter) clock "model-steps" on origin {
  always holds(q.value < 2)
}
"#;

const PROTOCOL_SOURCE: &str = r#"protocol Flow using protocols over (view: Queue::Counter) on origin {
  role Worker on Queue::Counter;
  run sequence Main {
    attempt Apply by Worker on Queue::Counter::inc contracts [SpaceAvailable, AppendOne] as (applied: Boolean) { true };
    attempt Again by Worker on Queue::Counter::inc contracts [SpaceAvailable, AppendOne] as (applied_again: Boolean) { true };
  }
  finish Done as (done: Boolean) { true };
}
"#;

const STUTTER_PROTOCOL_SOURCE: &str = r#"protocol Flow using protocols over (view: Queue::Counter) on each (opened: Boolean) when (false) {
  role Worker on Queue::Counter;
  run sequence Main { }
  finish Done as (done: Boolean) { true };
}
"#;

const TIMED_SOURCE: &str = r#"language "ix:native" edition "1-draft";
profile values = "filament.model.complete/v1";
profile paths = "filament.temporal.infinite-trace/v1";
profile timing = "filament.temporal.complete/v1";
model Queue = "test/counter";
dimension Time;
unit s : Time = rational(1, 1);
unit ms : Time = rational(1, 1000) * s;
time Queue dense unit ms;
pre SpaceAvailable using values on Queue::Counter::inc { self.value < 2 and self.x >= 1 ms }
post AppendOne using values on Queue::Counter::inc { self.value - pre(self.value) = 1 }
temporal NeverFull using paths over (q: Queue::Counter) clock "model-steps" on origin {
  always holds(q.value < 2)
}
"#;

#[derive(Clone, Copy)]
enum CounterFixture {
    Model,
    Protocol,
    ProtocolStutter,
    Timed,
}

fn counter_context(fixture: CounterFixture) -> Box<RawValue> {
    use serde_json::json;
    let directory = tempfile::tempdir().unwrap();
    let source = match fixture {
        CounterFixture::Model => COUNTER_SOURCE.to_owned(),
        CounterFixture::Protocol | CounterFixture::ProtocolStutter => {
            let source = COUNTER_SOURCE.replacen(
                "model Queue",
                "profile protocols = \"filament.protocol.finite-global/v1\";\nmodel Queue",
                1,
            );
            if matches!(fixture, CounterFixture::ProtocolStutter) {
                format!(
                    "{}{STUTTER_PROTOCOL_SOURCE}",
                    source.replace("always holds(q.value < 2)", "eventually holds(q.value > 0)")
                )
            } else {
                format!("{source}{PROTOCOL_SOURCE}")
            }
        }
        CounterFixture::Timed => TIMED_SOURCE.to_owned(),
    };
    std::fs::write(directory.path().join("workflow.fsl"), source).unwrap();
    let domain = include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json");
    if matches!(fixture, CounterFixture::Timed) {
        let mut domain: serde_json::Value = serde_json::from_slice(domain).unwrap();
        let counter = domain["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["identity"] == "ix://test/counter/Counter")
            .unwrap();
        let clock = "ix://test/counter/Counter/x";
        counter["fields"].as_array_mut().unwrap().push(json!({
            "identity":clock,"name":"x","typeRef":"ix://filament/native/Clock",
            "presence":"required","nullable":false,"defaultKind":"none",
            "multiplicity":{"lower":1,"upper":1,"ordered":false,"unique":false},
            "origin":{"generated":{"generatorIdentity":clock,"generatorVersion":"1.0.0","inputIdentities":[clock]}}
        }));
        std::fs::write(
            directory.path().join("counter.json"),
            serde_json::to_vec(&domain).unwrap(),
        )
        .unwrap();
    } else {
        std::fs::write(directory.path().join("counter.json"), domain).unwrap();
    }
    let start = json!({"format":"filament.state.snapshot/v1", "identity":{"authority":"contracts","identity":"queue-start"},
        "observation":"current","model":{"identity":"test/counter"},"anchor":{"kind":"initialization","name":"queue-start"},
        "populations":[{"population":"ix://test/counter/counters","complete":true,"objects":[{"key":"queue-east","type":"ix://test/counter/Counter","fields":{"value":{"integer":"0"}}}]}]});
    std::fs::write(
        directory.path().join("start.json"),
        serde_json::to_vec(&start).unwrap(),
    )
    .unwrap();
    let item = if matches!(
        fixture,
        CounterFixture::Protocol | CounterFixture::ProtocolStutter
    ) {
        json!({"claim":"NeverFull","temporal":{"subject":{"protocol":{"protocol":"Flow","over":{"reference":{"population":"ix://test/counter/counters","key":"queue-east"}},"roles":{"Worker":{"type":"model-binding","selection":"test/counter"}}}}}})
    } else {
        json!({"claim":"NeverFull"})
    };
    let request = json!({"format":"native-analyze/1","request":{
        "program":{"source":{"file":"workflow.fsl","authority":"contracts","identity":"queue-workflow","document":"QueueWorkflow","formal_revision":1}},
        "models":[{"format":"semantic-ir/2.0.0","source":{"file":"counter.json","authority":"contracts","identity":"counter-reference","document":"CounterInput","formal_revision":1}}],
        "documents":[{"file":"start.json"}],"items":[item]}});
    let path = directory.path().join("analyze.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let produced = command::analyze_with_providers_and_limits(
        &path,
        RunLimits::default(),
        &crate::negotiation::Fr290,
        Vec::new(),
        &ProviderOptions::default(),
        &Cancel::new(),
    )
    .expect("the actual model/protocol producer request runs");
    assert_eq!(produced.exit_code, 10, "{}", produced.value);
    let stdout = produced.value.to_bytes().unwrap();
    let document: Produced =
        serde_json::from_slice(&stdout).expect("the source producer retains its original context");
    // The original files disappear here; independent native replay uses only stdout.
    document.items.into_iter().next().unwrap().replay_context
}

fn assert_fresh_temporal(context: &RawValue, arm: &str) {
    let output = fsl_command::replay_context::replay(&forward(context, "16777216"))
        .expect("native source reconstruction and actual family replay");
    let document: serde_json::Value = serde_json::from_str(output.as_raw().get()).unwrap();
    assert_eq!(document["format"], "filament-outcome/1");
    assert_eq!(document["operation"], "replay");
    assert_eq!(document["result"]["kind"], "replayed");
    let row = &document["result"]["results"][0];
    assert_eq!(row["arm"], arm);
    assert_eq!(row["settlement"], "reproduced-with-evaluated-witness");
    assert_eq!(row["value"], false);
}

/// FR-621-AC-18, FR-621-AC-20: a nonempty native model transition prefix
/// reconstructs from authentic source and the original selected instance.
#[test]
#[trace("FR-621-AC-18", "FR-621-AC-20")]
fn source_backed_model_context_replays_actual_transitions() {
    let context = counter_context(CounterFixture::Model);
    assert_fresh_temporal(&context, "model-trace");
}

/// FR-621-AC-20: the protocol alphabet restores through actual admitted
/// protocol successors and is re-executed by the native protocol entry.
#[test]
#[trace("FR-621-AC-20")]
fn source_backed_protocol_context_replays_actual_transitions() {
    let context = counter_context(CounterFixture::Protocol);
    assert_fresh_temporal(&context, "protocol-trace");
}

/// FR-621-AC-20: an initial protocol terminal stutter retains its genuine
/// native key through the existing replay gate, without reconstructing a key.
#[test]
#[trace("FR-621-AC-20")]
fn protocol_context_retains_genuine_empty_prefix_terminal_key() {
    let context = counter_context(CounterFixture::ProtocolStutter);
    let original: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(context.get()).unwrap();
    assert!(original.payload["prefix"].as_array().unwrap().is_empty());
    assert_eq!(
        original.payload["loop"][0]["transition"]["type"],
        "terminal-stutter"
    );
    assert_fresh_temporal(&context, "protocol-trace");
}

/// FR-621-AC-19, FR-621-AC-20: protocol replay uses the consumer's own
/// accounting and rejects a foreign proving context before applying it.
#[test]
#[trace("FR-621-AC-19", "FR-621-AC-20")]
fn protocol_context_preserves_original_authentication_and_consumer_denial() {
    let context = counter_context(CounterFixture::Protocol);
    let limited = |context: &RawValue| {
        serde_json::to_vec(&ReplayInput {
            format: fsl_command::replay_context::FORMAT,
            context,
            effective_limits: EffectiveLimits {
                replay_input_bytes: "16777216".to_owned(),
                accounting_limits: accounting_wire(fsl_replay::AccountingLimits::default().0),
                stage_limits: [("accounting.work_units".to_owned(), "0".to_owned())].into(),
            },
        })
        .unwrap()
    };
    match fsl_command::replay_context::replay(&limited(&context)) {
        Err(error) => assert_eq!(
            error.category(),
            filament_semantic_value::catalog::Category::Incomplete
        ),
        Ok(output) => {
            assert_eq!(
                output.category(),
                filament_semantic_value::catalog::Category::Incomplete
            );
            let document: serde_json::Value = serde_json::from_str(output.as_raw().get()).unwrap();
            assert_eq!(document["result"]["results"][0]["settlement"], "incomplete");
        }
    }
    let mut forged: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(context.get()).unwrap();
    forged
        .request
        .accounting_limits
        .insert("work_units".to_owned(), "0".to_owned());
    forged
        .request
        .stage_limits
        .insert("accounting.work_units".to_owned(), "0".to_owned());
    let forged = serde_json::value::to_raw_value(&forged).unwrap();
    assert!(matches!(
        fsl_command::replay_context::read(&limited(&forged)),
        Err(fsl_command::replay_context::ContextRequestError::Context(
            fsl_replay::replay_context::ContextError::Invalid { .. }
        ))
    ), "protocol proof limits must authenticate against the original envelope before consumer overlays");
}

/// FR-621-AC-20: actual timed guards require a genuine retained delay, and
/// the native reader reconstructs the original clock-state keys.
#[test]
#[trace("FR-621-AC-20")]
fn source_backed_timed_context_replays_actual_delays() {
    let context = counter_context(CounterFixture::Timed);
    let original: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(context.get()).unwrap();
    assert!(
        original.subject["timed"].is_object(),
        "the source producer must select the timed model path"
    );
    assert!(
        original.payload["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| {
                step["delay"]["numerator"]
                    .as_str()
                    .is_some_and(|value| value != "0")
            }),
        "the actual clock guard requires a positive native delay"
    );
    assert_fresh_temporal(&context, "model-trace");
}

/// FR-621-AC-18: genuine source bytes cannot be changed beneath an original
/// emitted package, and the consumer's own byte denial remains an explicit limit.
#[test]
#[trace("FR-621-AC-18")]
fn hybrid_context_refuses_changed_source_and_consumer_byte_denial() {
    let original = hybrid_context();
    assert!(matches!(
        fsl_command::replay_context::read(&forward(&original, "1")),
        Err(fsl_command::replay_context::ContextRequestError::Limit(_))
    ));
    let mut context: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(original.get()).unwrap();
    let source = context
        .source_bytes
        .iter_mut()
        .find(|source| {
            std::str::from_utf8(&source.bytes).is_ok_and(|text| text.contains("h.v > 5"))
        })
        .expect("the actual hybrid claim source was retained");
    source.bytes = std::str::from_utf8(&source.bytes)
        .unwrap()
        .replace("h.v > 5", "h.v > 6")
        .into_bytes();
    let changed = context.to_raw().unwrap();
    let prepared = fsl_command::replay_context::read(&forward(&changed, "16777216")).unwrap();
    assert!(
        ReconstructedContext::prepare(prepared.context, prepared.common, prepared.limits).is_err(),
        "changed actual source must fail authentication against the original emitted package"
    );
}

/// FR-621-AC-19, FR-621-AC-20: the native entry returns a fresh replay
/// settlement and applies consumer stage accounting without rewriting proof limits.
#[test]
#[trace("FR-621-AC-19", "FR-621-AC-20")]
fn native_hybrid_result_and_consumer_work_denial_preserve_original_context() {
    let context = hybrid_context();
    let original = context.get().to_owned();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("replay.json");
    std::fs::write(&path, forward(&context, "16777216")).unwrap();
    let output = command::run(&path, &crate::negotiation::Fr290)
        .expect("actual native command hybrid dispatcher");
    let fresh: serde_json::Value =
        serde_json::from_slice(&output.value.to_bytes().unwrap()).unwrap();
    assert_eq!(fresh["format"], "filament-outcome/1");
    assert_eq!(fresh["operation"], "replay");
    assert_eq!(fresh["result"]["kind"], "replayed");
    assert_eq!(fresh["result"]["results"][0]["arm"], "hybrid");
    assert_eq!(
        fresh["result"]["results"][0]["settlement"],
        "reproduced-with-evaluated-witness"
    );

    let denied = serde_json::to_vec(&ReplayInput {
        format: fsl_command::replay_context::FORMAT,
        context: &context,
        effective_limits: EffectiveLimits {
            replay_input_bytes: "16777216".to_owned(),
            accounting_limits: accounting_wire(fsl_replay::AccountingLimits::default().0),
            stage_limits: [("accounting.work_units".to_owned(), "0".to_owned())].into(),
        },
    })
    .unwrap();
    let denial = fsl_command::replay_context::replay(&denied)
        .expect("native hybrid replay retains its genuine incomplete settlement");
    assert_eq!(
        denial.category(),
        filament_semantic_value::catalog::Category::Incomplete
    );
    let row: serde_json::Value = serde_json::from_str(denial.as_raw().get()).unwrap();
    assert_eq!(row["result"]["results"][0]["settlement"], "incomplete");
    assert_eq!(
        row["result"]["results"][0]["limit"]["setting"],
        "accounting.work_units"
    );
    assert_eq!(row["result"]["results"][0]["limit"]["value"], "0");
    assert_eq!(
        context.get(),
        original,
        "neither native execution rewrites the original proof context"
    );
}

/// FR-621-AC-19: original model witness limits are authenticated before
/// independent consumer accounting can be applied.
#[test]
#[trace("FR-621-AC-19")]
fn model_context_authenticates_original_limits_before_consumer_overlay() {
    let context = counter_context(CounterFixture::Model);
    let mut forged: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(context.get()).unwrap();
    forged
        .request
        .accounting_limits
        .insert("work_units".to_owned(), "0".to_owned());
    forged
        .request
        .stage_limits
        .insert("accounting.work_units".to_owned(), "0".to_owned());
    let forged = serde_json::value::to_raw_value(&forged).unwrap();
    let forged = serde_json::to_vec(&ReplayInput {
        format: fsl_command::replay_context::FORMAT,
        context: &forged,
        effective_limits: EffectiveLimits {
            replay_input_bytes: "16777216".to_owned(),
            accounting_limits: accounting_wire(fsl_replay::AccountingLimits::default().0),
            stage_limits: [("accounting.work_units".to_owned(), "0".to_owned())].into(),
        },
    })
    .unwrap();
    assert!(matches!(
        fsl_command::replay_context::replay(&forged),
        Err(fsl_command::replay_context::ContextReplayError::Request(
            fsl_command::replay_context::ContextRequestError::Context(
                fsl_replay::replay_context::ContextError::Invalid { .. }
            )
        ))
    ), "foreign proving limits must fail original-envelope authentication before the consumer overlay");
}

/// FR-621-AC-18: an identity held by the genuine package still cannot replace
/// the original claim binding in an otherwise authentic context.
#[test]
#[trace("FR-621-AC-18")]
fn native_hybrid_context_refuses_foreign_admitted_node() {
    let original = hybrid_context();
    let mut context: fsl_replay::replay_context::ReplayContext =
        serde_json::from_str(original.get()).unwrap();
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(
        context.request.package.get().as_bytes(),
    )
    .unwrap();
    let claim = context.subject["claim"]["node"].clone();
    let foreign = graph
        .keys()
        .iter()
        .map(|key| graph.wire_value(key).unwrap())
        .find(|node| node != &claim)
        .expect("the original admitted graph holds another node");
    context.subject["claim"]["node"] = foreign;
    let changed = context.to_raw().unwrap();
    let error = fsl_command::replay_context::replay(&forward(&changed, "16777216"))
        .expect_err("a genuine but foreign node must not acquire the original claim's authority");
    assert_eq!(error.code(), fsl_foundation::Code::InvalidRuntimeInput);
}
