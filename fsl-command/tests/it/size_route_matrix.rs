// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compile-route probes for the QSL-3457 route-by-size matrix.

use std::path::{Path, PathBuf};

use filament_exact::Cancel;
use fsl_analyze::provider::ProviderOptions;
use fsl_command::command::{self, RequestOverrides, RunLimits};
use fsl_foundation::Setting;
use ix_trace_rs::trace;
use serde_json::{Value, json};

const NESTED: usize = 1_000;

fn compile_request(directory: &Path, source: &str) -> PathBuf {
    std::fs::write(directory.join("program.native"), source).expect("source writes");
    let request = json!({"format":"native-compile/1", "request": {
        "models": [], "program": {"source": {"file":"program.native",
            "authority":"agent-ix", "identity":"test:qsl-3570", "document":"SizeMatrix",
            "formal_revision":1}}
    }});
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn compile_request_with_model(directory: &Path, source: &str, model: &str) -> PathBuf {
    let path = compile_request(directory, source);
    std::fs::write(directory.join("model.fsl"), model).expect("model writes");
    let request = json!({"format":"native-compile/1", "request": {
        "models": [{"format":"native-model-unit/1", "source": {
            "file":"model.fsl", "authority":"agent-ix", "identity":"test:qsl-3570-model",
            "document":"SizeMatrixModel", "formal_revision":1
        }}],
        "program": {"source": {"file":"program.native", "authority":"agent-ix",
            "identity":"test:qsl-3570", "document":"SizeMatrix", "formal_revision":1}}
    }});
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

fn compile_request_with_semantic_model(directory: &Path, source: &str, model: &Value) -> PathBuf {
    std::fs::write(directory.join("program.native"), source).expect("source writes");
    let model_path = directory.join("model.json");
    std::fs::write(
        &model_path,
        serde_json::to_vec(model).expect("semantic model serializes"),
    )
    .expect("semantic model writes");
    let request = json!({"format":"native-compile/1", "request": {
        "models": [{"format":"semantic-ir/2.0.0", "source": {
            "file":"model.json", "authority":"agent-ix", "identity":"test:qsl-3570-model",
            "document":"SizeMatrixModel", "formal_revision":1
        }}],
        "program": {"source": {"file":"program.native", "authority":"agent-ix",
            "identity":"test:qsl-3570", "document":"SizeMatrix", "formal_revision":1}}
    }});
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request serializes"),
    )
    .expect("request writes");
    path
}

struct FslNegotiate;

impl fsl_analyze::analyze::document::Negotiate for FslNegotiate {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::NegotiationOutcome;
        use fsl_route::routing::Disposition;

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

fn analyze_source(extra: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile values = \"filament.value.complete/v1\";\n\
         profile paths = \"filament.temporal.infinite-trace/v1\";\n\
         model Queue = \"test/counter\";\n\
         {extra}\
         temporal Bounded using paths over (q: Queue::Counter) clock \"model-steps\" on origin {{\n\
           always holds(q.value <= 2)\n\
         }}\n"
    )
}

fn nested_analyze_source() -> String {
    analyze_source(&format!(
        "function nested using values(): Boolean pure {{ {}true{} }}\n",
        "not (".repeat(NESTED),
        ")".repeat(NESTED)
    ))
}

fn flat_analyze_source() -> String {
    let terms = std::iter::repeat_n("1", 10_000)
        .collect::<Vec<_>>()
        .join(" + ");
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile values = \"filament.value.complete/v1\";\n\
         profile paths = \"filament.temporal.infinite-trace/v1\";\n\
         model Queue = \"test/counter\";\n\
         function total using values(): Integer pure {{ {terms} }}\n\
         temporal Bounded using paths over (q: Queue::Counter) clock \"model-steps\" on origin {{\n\
           always holds(q.value <= 2)\n\
         }}\n"
    )
}

fn analyze_request(directory: &Path, source_text: &str) -> PathBuf {
    analyze_request_with_model(
        directory,
        source_text,
        include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json"),
    )
}

fn analyze_request_with_model(directory: &Path, source_text: &str, model: &[u8]) -> PathBuf {
    std::fs::write(directory.join("workflow.fsl"), source_text).expect("program writes");
    std::fs::write(directory.join("counter.json"), model).expect("counter model writes");
    let start = json!({"format":"filament.state.snapshot/v1",
        "identity":{"authority":"contracts","identity":"queue-start"},
        "observation":"current","model":{"identity":"test/counter"},
        "anchor":{"kind":"initialization","name":"queue-start"},
        "populations":[{"population":"ix://test/counter/counters","complete":true,
            "objects":[{"key":"queue-east","type":"ix://test/counter/Counter",
                "fields":{"value":{"integer":"0"}}}]}]});
    std::fs::write(
        directory.join("start.json"),
        serde_json::to_vec(&start).expect("snapshot serializes"),
    )
    .expect("snapshot writes");
    let request = json!({"format":"native-analyze/1","request":{
        "program":{"source":{"file":"workflow.fsl","authority":"contracts",
            "identity":"qsl-3570","document":"SizeMatrix","formal_revision":1}},
        "models":[{"format":"semantic-ir/2.0.0","source":{"file":"counter.json",
            "authority":"contracts","identity":"counter-reference",
            "document":"CounterInput","formal_revision":1}}],
        "documents":[{"file":"start.json"}],"items":[{"claim":"Bounded"}]}});
    let path = directory.join("analyze.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("analyze request serializes"),
    )
    .expect("analyze request writes");
    path
}

fn counter_model_with_one_thousand_enums() -> Vec<u8> {
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/cli-contract/counter.semantic-ir.json"
    ))
    .expect("counter model fixture is JSON");
    let constructs = model["constructs"].as_array_mut().expect("constructs");
    constructs.push(json!({
        "kind":{"module":"test/counter","name":"variant_type"},
        "moduleVersion":"1.0.0",
        "construct":{"identity":"none","shape":"enumeration",
            "members":{"variants":"required"},
            "meaning":"filament.meaning.model.variant-type/v1"}
    }));
    let types = model["types"].as_array_mut().expect("types");
    for index in 0..1_000 {
        let name = format!("SizeEnum{index:04}");
        let identity = format!("ix://test/counter/{name}");
        let generated = |suffix: &str| {
            json!({"generated":{"generatorIdentity":format!("{identity}/{suffix}"),
                "generatorVersion":"1.0.0",
                "inputIdentities":[format!("{identity}/{suffix}")]}})
        };
        types.push(json!({
            "identity":identity,"displayName":name,
            "kind":{"module":"test/counter","name":"variant_type"},
            "roles":[],"origin":generated("type"),"constraints":[],"extensions":[],
            "unknownPolicy":"reject",
            "variants":[
                {"identity":format!("{identity}/A"),"name":"A","origin":generated("A")},
                {"identity":format!("{identity}/B"),"name":"B","origin":generated("B")}
            ]
        }));
    }
    serde_json::to_vec(&model).expect("extended counter model serializes")
}

fn counter_model_with_one_thousand_links(redefines: bool) -> Vec<u8> {
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/cli-contract/counter.semantic-ir.json"
    ))
    .expect("counter model fixture is JSON");
    let types = model["types"].as_array_mut().expect("types");
    for index in 0_u64..=1_000 {
        let name = format!("Link{index:04}");
        let identity = format!("ix://test/counter/{name}");
        let generated = |suffix: &str| {
            json!({"generated":{"generatorIdentity":format!("{identity}/{suffix}"),
                "generatorVersion":"1.0.0",
                "inputIdentities":[format!("{identity}/{suffix}")]}})
        };
        let parent = index
            .checked_sub(1)
            .map(|parent| format!("ix://test/counter/Link{parent:04}"));
        let operations = if redefines {
            let mut operation = json!({
                "identity":format!("{identity}/query"),"name":"query",
                "params":[],"pre":[],"post":[],"origin":generated("query"),
                "frame":{"modifies":[],"creates":[],"deletes":[]}
            });
            if let Some(parent) = &parent {
                operation["redefines"] = json!(format!("{parent}/query"));
            }
            vec![operation]
        } else {
            Vec::new()
        };
        let supertypes = parent.into_iter().collect::<Vec<_>>();
        types.push(json!({
            "identity":identity,"displayName":name,
            "kind":{"module":"test/counter","name":"object_type"},
            "roles":[],"origin":generated("type"),"constraints":[],"extensions":[],
            "unknownPolicy":"reject","supertypes":supertypes,"fields":[],
            "operations":operations
        }));
    }
    serde_json::to_vec(&model).expect("extended counter model serializes")
}

fn counter_model_with_unbounded_sequence() -> Vec<u8> {
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/cli-contract/counter.semantic-ir.json"
    ))
    .expect("counter model fixture is JSON");
    let counter = model["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://test/counter/Counter")
        .expect("Counter type");
    counter["fields"]
        .as_array_mut()
        .expect("fields")
        .push(json!({
            "identity":"ix://test/counter/Counter/values","name":"values",
            "typeRef":"ix://filament/native/Integer","presence":"required","nullable":false,
            "defaultKind":"none",
            "multiplicity":{"lower":0,"upper":null,"ordered":true,"unique":false},
            "origin":{"generated":{"generatorIdentity":"ix://test/counter/Counter/values",
                "generatorVersion":"1.0.0",
                "inputIdentities":["ix://test/counter/Counter/values"]}}
        }));
    serde_json::to_vec(&model).expect("sequence-capable counter model serializes")
}

fn analyze_with_request_bound(
    path: &Path,
    request_bytes: usize,
) -> Result<command::RunResult, Box<command::RunError>> {
    command::analyze_with_providers_and_limits(
        path,
        RunLimits {
            request_bytes,
            ..RunLimits::default()
        },
        &command::CommandLimits::analyze(),
        &FslNegotiate,
        Vec::new(),
        &ProviderOptions::default(),
        &Cancel::new(),
    )
}

fn assert_analyze_request_exact_and_one_byte_over(path: &Path) {
    let request_bytes = usize::try_from(std::fs::metadata(path).expect("request metadata").len())
        .expect("request size fits usize");
    let result = analyze_with_request_bound(path, request_bytes)
        .expect("the request at its exact byte count is admitted");
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        result.value.as_value()["items"][0]["record"]["result"],
        "proved"
    );

    let mut bytes = std::fs::read(path).expect("request reads");
    bytes.push(b' ');
    std::fs::write(path, bytes).expect("one byte appends");
    let error = analyze_with_request_bound(path, request_bytes)
        .err()
        .expect("one byte above the request limit refuses before the item");
    let document = error.outcome_document(fsl_replay::Operation::Analyze);
    let value: Value =
        serde_json::from_slice(&document.to_bytes().expect("incomplete outcome serializes"))
            .expect("incomplete outcome is JSON");
    assert_eq!(error.category().exit_code(), 22);
    assert_eq!(value["category"], "resource_exhausted");
    assert_eq!(value["last_stage"], Value::Null);
    assert_eq!(value["items"], json!([]));
}

/// Analyze admits the nested-source request exactly and rejects one appended
/// request byte before any item runs.
#[trace("TC-3242", "FR-620-AC-12")]
#[test]
fn analyze_admits_the_thousand_nested_expression_at_its_request_byte_bound() {
    let directory = tempfile::tempdir().expect("analyze workspace");
    let path = analyze_request(directory.path(), &nested_analyze_source());
    assert_analyze_request_exact_and_one_byte_over(&path);
}

/// Analyze retains the 10,000-term package request boundary independently of
/// the analysis engine's item outcome.
#[trace("TC-3242", "FR-620-AC-13")]
#[test]
fn analyze_admits_the_ten_thousand_term_package_at_its_request_byte_bound() {
    let directory = tempfile::tempdir().expect("analyze workspace");
    let path = analyze_request(directory.path(), &flat_analyze_source());
    assert_analyze_request_exact_and_one_byte_over(&path);
}

/// Analyze admits the 1,000-enum model and keeps its request-byte ceiling
/// independent of model declaration intake.
#[trace("TC-3242", "FR-620-AC-14")]
#[test]
fn analyze_admits_one_thousand_enums_at_its_request_byte_bound() {
    let directory = tempfile::tempdir().expect("analyze workspace");
    let model = counter_model_with_one_thousand_enums();
    let path = analyze_request_with_model(directory.path(), &analyze_source(""), &model);
    assert_analyze_request_exact_and_one_byte_over(&path);
}

/// Analyze admits both 1,000-link model fixtures at exact request bytes and
/// settles the bounded claim; one extra request byte stays document-level.
#[trace("TC-3242", "FR-620-AC-15")]
#[test]
fn analyze_admits_both_thousand_link_models_at_the_request_byte_bound() {
    for redefines in [false, true] {
        let directory = tempfile::tempdir().expect("analyze workspace");
        let model = counter_model_with_one_thousand_links(redefines);
        let path = analyze_request_with_model(directory.path(), &analyze_source(""), &model);
        assert_analyze_request_exact_and_one_byte_over(&path);
    }
}

/// Analyze admits an unbounded sequence field carrying exactly 100,000
/// ordered values, and its observation-value ceiling remains independently
/// typed at one value under.
#[trace("TC-3242", "FR-620-AC-16")]
#[test]
fn analyze_admits_one_hundred_thousand_observation_values_at_their_bound() {
    let directory = tempfile::tempdir().expect("analyze workspace");
    std::fs::write(
        directory.path().join("workflow.fsl"),
        "language \"ix:native\" edition \"1-draft\";\n\
         profile paths = \"filament.temporal.infinite-trace/v1\";\n\
         model Queue = \"test/counter\";\n\
         temporal Bounded using paths over (q: Queue::Counter) clock \"model-steps\" on origin {\n\
           always holds(size(q.values) = 100000)\n\
         }\n",
    )
    .expect("program writes");
    let values = (0..100_000)
        .map(|value| json!({"integer":value.to_string()}))
        .collect::<Vec<_>>();
    let start = json!({"format":"filament.state.snapshot/v1",
        "identity":{"authority":"contracts","identity":"queue-sequence-start"},
        "observation":"current","model":{"identity":"test/counter"},
        "anchor":{"kind":"initialization","name":"queue-sequence-start"},
        "populations":[{"population":"ix://test/counter/counters","complete":true,
            "objects":[{"key":"queue-east","type":"ix://test/counter/Counter",
                "fields":{"value":{"integer":"0"},"values":{"sequence":values}}}]}]});
    std::fs::write(
        directory.path().join("start.json"),
        serde_json::to_vec(&start).expect("sequence snapshot serializes"),
    )
    .expect("sequence snapshot writes");
    let request = json!({"format":"native-analyze/1","request":{
        "program":{"source":{"file":"workflow.fsl","authority":"contracts",
            "identity":"qsl-3570-sequence","document":"SizeMatrixSequence","formal_revision":1}},
        "models":[{"format":"semantic-ir/2.0.0","source":{"file":"counter.json",
            "authority":"contracts","identity":"counter-sequence-reference",
            "document":"CounterSequenceInput","formal_revision":1}}],
        "documents":[{"file":"start.json"}],"items":[{"claim":"Bounded"}],
        "limits":{"observation":{"values":100001}}}});
    std::fs::write(
        directory.path().join("counter.json"),
        counter_model_with_unbounded_sequence(),
    )
    .expect("sequence-capable counter model writes");
    let path = directory.path().join("analyze.json");
    let request = serde_json::to_vec(&request).expect("sequence analyze request serializes");
    std::fs::write(&path, &request).expect("sequence analyze request writes");
    let request_bytes = request.len();
    let result = analyze_with_request_bound(&path, request_bytes)
        .expect("the 100,000-value observation admits at the exact observation bound");
    assert_eq!(result.exit_code, 0);
    assert_eq!(
        result.value.as_value()["items"][0]["record"]["result"],
        "proved"
    );

    let mut over = request.clone();
    over.push(b' ');
    std::fs::write(&path, over).expect("one request byte appends");
    let error = analyze_with_request_bound(&path, request_bytes)
        .err()
        .expect("one request byte over remains a whole-document refusal");
    let document = error.outcome_document(fsl_replay::Operation::Analyze);
    let value: Value = serde_json::from_slice(
        &document
            .to_bytes()
            .expect("request-limit outcome serializes"),
    )
    .expect("request-limit outcome is JSON");
    assert_eq!(error.category().exit_code(), 22);
    assert_eq!(value["category"], "resource_exhausted");
    assert_eq!(value["last_stage"], Value::Null);
    assert_eq!(value["items"], json!([]));

    let mut below: Value = serde_json::from_slice(&request).expect("request parses");
    below["request"]["limits"]["observation"]["values"] = json!(100_000);
    let below = serde_json::to_vec(&below).expect("one-under request serializes");
    assert_eq!(
        below.len(),
        request.len(),
        "request byte ceiling is independent"
    );
    std::fs::write(&path, below).expect("one-under observation limit writes");
    let result = analyze_with_request_bound(&path, request_bytes)
        .expect("the observation limit is an item-level typed result");
    assert_eq!(result.exit_code, 22);
    let result = result.value.as_value();
    let rendered = result.to_string();
    assert!(rendered.contains("stage_limit_exceeded"), "{rendered}");
    assert!(rendered.contains("observation.values"), "{rendered}");
    assert!(rendered.contains("100000"), "{rendered}");
    assert!(rendered.contains("100001"), "{rendered}");
}

fn chain_model(prefix: &str, redefines: bool) -> String {
    let mut model =
        String::from("language \"ix:native\" edition \"1-draft\";\ndomain \"test/qsl-3570\";\n");
    for index in 0_u64..=1_000 {
        let name = format!("{prefix}{index:04}");
        let object_kind = if index == 1_000 {
            "object"
        } else {
            "abstract object"
        };
        let parent = index
            .checked_sub(1)
            .map(|parent| format!("{prefix}{parent:04}"));
        let extends = parent
            .as_ref()
            .map_or_else(String::new, |parent| format!(" extends {parent}"));
        let operation = if redefines {
            let target = parent
                .as_ref()
                .map_or_else(String::new, |parent| format!(" redefines {parent}::query"));
            format!(" operation query(): Integer{target};")
        } else {
            String::new()
        };
        model.push_str(&format!("{object_kind} {name}{extends} {{{operation} }}\n"));
    }
    model
}

fn compile_with_model_limit(
    directory: &Path,
    model: &str,
    setting: Setting,
    bound: u64,
) -> Result<command::CompileOutcome, Box<command::RunError>> {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        model M = \"test/qsl-3570\";\n\
        function ready using v(): Boolean pure { true }\n";
    let path = compile_request_with_model(directory, source, model);
    let limits = std::collections::BTreeMap::from([(setting, bound)]);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
}

fn nested_source() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function nested using v(): Boolean pure {{ {}true{} }}\n",
        "not (".repeat(NESTED),
        ")".repeat(NESTED)
    )
}

fn compile_at(path: &Path, nodes: u64) -> Result<command::CompileOutcome, Box<command::RunError>> {
    compile_at_setting(path, Setting::S1Nodes, nodes)
}

fn compile_at_setting(
    path: &Path,
    setting: Setting,
    bound: u64,
) -> Result<command::CompileOutcome, Box<command::RunError>> {
    let mut limits = std::collections::BTreeMap::from([
        (Setting::S1Tokens, 1_000_000),
        (Setting::S1Nodes, 1_000_000),
        (Setting::S1WorkUnits, 1_000_000),
        (Setting::EmitOutputBytes, 1_073_741_824),
    ]);
    limits.insert(setting, bound);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    command::compile_outcome_with_limits_and_overrides(path, RunLimits::default(), &overrides)
}

fn minimum_s1_nodes(path: &Path) -> u64 {
    minimum_setting(path, Setting::S1Nodes)
}

fn minimum_setting(path: &Path, setting: Setting) -> u64 {
    let attempt = |bound| compile_at_setting(path, setting, bound);
    let mut high = 1;
    while high < 1_000_000 {
        if attempt(high).is_ok() {
            break;
        }
        high *= 2;
    }
    if let Err(error) = attempt(high) {
        let failure = refusal(&error);
        panic!(
            "fixture does not fit within bounded {setting:?} search: stage={:?}, code={:?}",
            failure["stage"], failure["code"]
        );
    }
    let mut low = 0;
    for _ in 0..32 {
        if low + 1 >= high {
            break;
        }
        let middle = low + (high - low) / 2;
        if attempt(middle).is_ok() {
            high = middle;
        } else {
            low = middle;
        }
    }
    high
}

fn refusal(error: &command::RunError) -> Value {
    error.value().expect("compile stage refusal")
}

/// The successful bound is the smallest admitted S1 node count; the adjacent
/// lower value must retain its typed stage-limit refusal.
#[trace("TC-3242", "FR-869-AC-5")]
#[test]
fn compile_admits_the_thousand_nested_expression_at_its_counted_s1_bound() {
    let directory = tempfile::tempdir().expect("temporary workspace");
    let path = compile_request(directory.path(), &nested_source());
    let counted = minimum_s1_nodes(&path);

    let outcome = compile_at(&path, counted).expect("counted S1 boundary admits the source");
    assert!(!outcome.package_bytes().is_empty());

    let error = compile_at(&path, counted - 1)
        .err()
        .expect("one below the count refuses");
    let refusal = refusal(&error);
    assert_eq!(refusal["code"], "stage_limit_exceeded", "{refusal}");
    assert!(
        refusal["message"].as_str().unwrap().contains("s1.nodes"),
        "{refusal}"
    );
}

/// The 10,000-term source is admitted at its smallest S3 node bound and
/// refused at the adjacent lower value.
#[trace("TC-3242", "FR-869-AC-6")]
#[test]
fn compile_admits_the_ten_thousand_term_expression_at_its_counted_s3_bound() {
    let directory = tempfile::tempdir().expect("temporary workspace");
    let terms = std::iter::repeat_n("1", 10_000)
        .collect::<Vec<_>>()
        .join(" + ");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function total using v(): Integer pure {{ {terms} }}\n"
    );
    let path = compile_request(directory.path(), &source);
    let counted = minimum_setting(&path, Setting::S3Nodes);
    let outcome = compile_at_setting(&path, Setting::S3Nodes, counted)
        .expect("counted S3 boundary admits the source");
    assert!(!outcome.package_bytes().is_empty());

    let error = compile_at_setting(&path, Setting::S3Nodes, counted - 1)
        .err()
        .expect("one below the counted S3 nodes refuses");
    let failure = refusal(&error);
    assert_eq!(failure["code"], "stage_limit_exceeded", "{failure}");
    assert!(
        failure["message"].as_str().unwrap().contains("s3.nodes"),
        "{failure}"
    );
}

/// Model intake of 1,000 real declarations retains its exact counter outcome.
#[trace("TC-3242", "FR-869-AC-7")]
#[test]
fn compile_admits_one_thousand_enum_declarations_at_the_model_record_bound() {
    let directory = tempfile::tempdir().expect("temporary workspace");
    let mut model =
        String::from("language \"ix:native\" edition \"1-draft\";\ndomain \"test/qsl-3570\";\n");
    for index in 0..1_000 {
        model.push_str(&format!("enum E{index:04} {{ A, B }}\n"));
    }
    let source = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        model M = \"test/qsl-3570\";\n\
        function ready using v(): Boolean pure { true }\n";
    let path = compile_request_with_model(directory.path(), source, &model);
    let mut limits = std::collections::BTreeMap::from([(Setting::ModelDeclarationRecords, 3_000)]);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .expect("the 3,000-record model intake admits all 1,000 enum types and variants");
    assert!(!outcome.package_bytes().is_empty());

    limits.insert(Setting::ModelDeclarationRecords, 2_999);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let error =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .err()
            .expect("the 2,999-record model bound refuses the next record");
    assert_eq!(
        error.category(),
        filament_semantic_value::catalog::Category::Incomplete
    );
    let command::RunCause::Spine(spine) = &error.cause else {
        panic!(
            "expected a typed spine intake refusal, got {:?}",
            error.cause
        );
    };
    let fsl_replay::spine::CompileRefusal::Intake { refusal, .. } = &spine.refusal else {
        panic!(
            "expected the selected model intake limit, got {:?}",
            spine.refusal
        );
    };
    let fsl_semantics::model::intake::UnitIntakeCause::Limit(limit) = &refusal.cause else {
        panic!("expected the normalization limit, got {:?}", refusal.cause);
    };
    assert_eq!(limit.limit_kind.setting(), Setting::ModelDeclarationRecords);
    assert_eq!(limit.limit, 2_999);
    assert_eq!(limit.consumed, 2_999);
    assert_eq!(limit.next_charge, 3_000);
    assert_eq!(refusal.cause.cause(), Some("insufficient-next-charge"));
}

/// The two 1,000-edge model link variants retain their distinct exact
/// normalization counters through compile intake.
#[trace("TC-3242", "FR-869-AC-8")]
#[test]
fn compile_admits_the_one_thousand_link_inheritance_and_redefinition_variants() {
    let inheritance = chain_model("Inherit", false);
    let inheritance_directory = tempfile::tempdir().expect("inheritance workspace");
    let admitted = compile_with_model_limit(
        inheritance_directory.path(),
        &inheritance,
        Setting::ModelAncestorSteps,
        1_000,
    )
    .expect("1,000 inheritance edges admit at their counted bound");
    assert!(!admitted.package_bytes().is_empty());
    let below_directory = tempfile::tempdir().expect("inheritance boundary workspace");
    let error = compile_with_model_limit(
        below_directory.path(),
        &inheritance,
        Setting::ModelAncestorSteps,
        999,
    )
    .err()
    .expect("999 ancestor steps refuses the 1,000th edge");
    let command::RunCause::Spine(spine) = &error.cause else {
        panic!("expected a spine intake refusal, got {:?}", error.cause);
    };
    let fsl_replay::spine::CompileRefusal::Intake { refusal, .. } = &spine.refusal else {
        panic!("expected a model intake refusal, got {:?}", spine.refusal);
    };
    let fsl_semantics::model::intake::UnitIntakeCause::Limit(limit) = &refusal.cause else {
        panic!("expected an ancestor-step limit, got {:?}", refusal.cause);
    };
    assert_eq!(limit.limit_kind.setting(), Setting::ModelAncestorSteps);
    assert_eq!(limit.limit, 999);
    assert_eq!(limit.consumed, 999);
    assert_eq!(limit.next_charge, 1);

    let redefinition = chain_model("Redefine", true);
    let redefinition_directory = tempfile::tempdir().expect("redefinition workspace");
    let error = compile_with_model_limit(
        redefinition_directory.path(),
        &redefinition,
        Setting::ModelDerivationFacts,
        1_504_502,
    )
    .err()
    .expect("default emit bytes refuse the completed redefinition package");
    let command::RunCause::Spine(spine) = &error.cause else {
        panic!("expected a spine emit refusal, got {:?}", error.cause);
    };
    let fsl_replay::spine::CompileRefusal::Limit(limit) = &spine.refusal else {
        panic!("expected the emit output limit, got {:?}", spine.refusal);
    };
    assert_eq!(limit.setting(), Setting::EmitOutputBytes);
    assert_eq!(limit.configured_bound(), 16_777_216);
    assert_eq!(limit.actual(), 16_777_217);

    let below_directory = tempfile::tempdir().expect("redefinition boundary workspace");
    let error = compile_with_model_limit(
        below_directory.path(),
        &redefinition,
        Setting::ModelDerivationFacts,
        1_504_501,
    )
    .err()
    .expect("one below the derivation-facts count refuses");
    let command::RunCause::Spine(spine) = &error.cause else {
        panic!("expected a spine intake refusal, got {:?}", error.cause);
    };
    let fsl_replay::spine::CompileRefusal::Intake { refusal, .. } = &spine.refusal else {
        panic!("expected a model intake refusal, got {:?}", spine.refusal);
    };
    let fsl_semantics::model::intake::UnitIntakeCause::Limit(limit) = &refusal.cause else {
        panic!("expected a derivation-facts limit, got {:?}", refusal.cause);
    };
    assert_eq!(limit.limit_kind.setting(), Setting::ModelDerivationFacts);
    assert_eq!(limit.limit, 1_504_501);
    assert_eq!(limit.next_charge, 1_504_502);
}

/// The 100,000-value sequence keeps model intake and source/S3/emission
/// ceilings independent on the compile route.
#[trace("TC-3242", "FR-869-AC-9")]
#[test]
fn compile_admits_one_hundred_thousand_sequence_values_when_model_limit_is_raised() {
    let directory = tempfile::tempdir().expect("collection workspace");
    let values = (0..100_000)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model M = \"example/config-version\";\n\
         function allValues using v(): Sequence<Integer>[0, 100000] pure {{ sequence[{values}] }}\n"
    );
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../../../examples/config-version/model.semantic-ir.json"
    ))
    .expect("the semantic model fixture is JSON");
    let config_version = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/ConfigVersion")
        .expect("ConfigVersion type");
    config_version["fields"][0]["multiplicity"] = json!({
        "lower": 0, "upper": 100_000, "ordered": true, "unique": false
    });
    let path = compile_request_with_semantic_model(directory.path(), &source, &model);

    let source_limits = std::collections::BTreeMap::from([
        (Setting::S1Tokens, 2_000_000),
        (Setting::S1Nodes, 2_000_000),
        (Setting::S1WorkUnits, 2_000_000),
    ]);
    let source_overrides = RequestOverrides {
        limits: Some(&source_limits),
        ..RequestOverrides::default()
    };
    let error = command::compile_outcome_with_limits_and_overrides(
        &path,
        RunLimits::default(),
        &source_overrides,
    )
    .err()
    .expect("the default model sequence ceiling refuses the declaration");
    let command::RunCause::Spine(spine) = &error.cause else {
        panic!(
            "expected a spine model intake refusal, got {:?}",
            error.cause
        );
    };
    let fsl_replay::spine::CompileRefusal::Intake { refusal, .. } = &spine.refusal else {
        panic!("expected a model intake refusal, got {:?}", spine.refusal);
    };
    let fsl_semantics::model::intake::UnitIntakeCause::Refused(refusals) = &refusal.cause else {
        panic!(
            "expected a sequence maximum refusal, got {:?}",
            refusal.cause
        );
    };
    let cause = &refusals[0].cause;
    let catalog = cause.catalog_code();
    assert_eq!(
        (catalog.code(), catalog.cause()),
        ("resource_exhausted", "intake-limit-exceeded")
    );
    let fields = cause.catalog_fields().expect("sequence limit fields");
    assert_eq!(fields["kind"], "node-count-exceeded");
    assert_eq!(fields["setting"], "model.sequence_maximum");
    assert_eq!(fields["bound"], "10000");
    assert_eq!(fields["actual"], "100000");

    let limits = std::collections::BTreeMap::from([
        (Setting::ModelSequenceMaximum, 100_000),
        (Setting::S1Tokens, 2_000_000),
        (Setting::S1Nodes, 2_000_000),
        (Setting::S1WorkUnits, 2_000_000),
        (Setting::S3Nodes, 2_000_000),
        (Setting::S3WorkUnits, 2_000_000),
        (Setting::EmitOutputBytes, 1_073_741_824),
    ]);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };
    let outcome =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .expect("raised independent ceilings admit and emit the 100,000-value package");
    assert!(!outcome.package_bytes().is_empty());
}

/// The 10,000-value accepted compile must rewrite node IDs with a bounded
/// number of candidate checks (QSL-3570, FR-869-AC-9 / TC-3242).
#[trace("TC-3242", "FR-869-AC-9")]
#[test]
fn compile_has_bounded_operand_rewrite_candidates_at_ten_thousand() {
    const N: usize = 10_000;
    let directory = tempfile::tempdir().expect("collection workspace");
    let values = (0..N)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         model M = \"example/config-version\";\n\
         function allValues using v(): Sequence<Integer>[0, {N}] pure {{ sequence[{values}] }}\n"
    );
    let mut model: Value = serde_json::from_slice(include_bytes!(
        "../../../examples/config-version/model.semantic-ir.json"
    ))
    .expect("the semantic model fixture is JSON");
    let config_version = model["types"]
        .as_array_mut()
        .expect("model types")
        .iter_mut()
        .find(|ty| ty["identity"] == "ix://example/config-version/ConfigVersion")
        .expect("ConfigVersion type");
    config_version["fields"][0]["multiplicity"] = json!({
        "lower": 0, "upper": N, "ordered": true, "unique": false
    });
    let path = compile_request_with_semantic_model(directory.path(), &source, &model);
    let limits = std::collections::BTreeMap::from([
        (Setting::ModelSequenceMaximum, (N + 1) as u64),
        (Setting::S1Tokens, 2_000_000),
        (Setting::S1Nodes, 2_000_000),
        (Setting::S1WorkUnits, 2_000_000),
        (Setting::S3Nodes, 2_000_000),
        (Setting::S3WorkUnits, 2_000_000),
        (Setting::EmitOutputBytes, 1_073_741_824),
    ]);
    let overrides = RequestOverrides {
        limits: Some(&limits),
        ..RequestOverrides::default()
    };

    fsl_package::test_support::reset_node_id_rewrite_candidate_checks();
    let outcome =
        command::compile_outcome_with_limits_and_overrides(&path, RunLimits::default(), &overrides)
            .expect("the 10,000-value sequence compiles with the raised independent limits");
    assert!(!outcome.package_bytes().is_empty());

    let checks = fsl_package::test_support::node_id_rewrite_candidate_checks();
    assert!(
        checks <= 60_000,
        "10,000-value operand rewrite used {checks} candidate checks; expected at most 60,000"
    );
}
