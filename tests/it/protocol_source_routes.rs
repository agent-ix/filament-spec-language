// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original protocol source programs and authoritative parts-saga journeys.

use filament_exact::Cancel;
use fsl_command::command;
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn saga() -> PathBuf {
    let root = std::env::var_os("FSL_EXAMPLES_DIR")
        .map(PathBuf::from)
        .expect("FSL_EXAMPLES_DIR must point to the authoritative filament-examples checkout");
    root.join("examples/parts-saga")
}

fn corpus(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/corpus/protocol")
        .join(file)
}

fn source(file: &Path) -> Value {
    json!({"file":file,"authority":"test","identity":"test:protocol-corpus",
        "document":"ProtocolCorpus","formal_revision":1})
}

fn compile_request(file: &Path) -> Value {
    json!({"format":"native-compile/1","request":{
        "program":{"source":source(file)},
        "models":[{"format":"semantic-ir/2.0.0",
            "source":source(&saga().join("model/parts-saga.semantic-ir.json"))}]}})
}

fn write(directory: &Path, request: &Value) -> PathBuf {
    let path = directory.join("request.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    path
}

fn compile(directory: &Path, file: &Path) {
    let output = command::compile_outcome(&write(directory, &compile_request(file)))
        .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
    assert!(!output.package_bytes().is_empty(), "{}", file.display());
}

fn refusal(directory: &Path, file: &Path, code: &str, cause: &str) {
    let error = match command::compile_outcome(&write(directory, &compile_request(file))) {
        Ok(_) => panic!("{} must refuse {code}/{cause}", file.display()),
        Err(error) => error,
    };
    let artifact = error.artifact_value().unwrap();
    assert_eq!(artifact["code"], code, "{}: {artifact}", file.display());
    assert_eq!(artifact["cause"], cause, "{}: {artifact}", file.display());
}

#[test]
#[trace("FR-218", "FR-219", "FR-220", "FR-224", "FR-884")]
fn original_protocol_declarations_communications_and_refusals() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read_to_string(corpus("original/exchange.native")).unwrap();
    compile(directory.path(), &corpus("original/exchange.native"));
    // Each mutation retains the same accepted model, subject and surrounding source.
    for (name, from, to, code, cause) in [
        ("duplicate-role", "role Receiver on Factory::MachiningStation;",
         "role Receiver on Factory::MachiningStation; role Receiver on Factory::MachiningStation;",
         "ambiguous_declaration", "ambiguous-name"),
        ("record-role", "role Receiver on Factory::MachiningStation;", "role Receiver on Note;",
         "invalid_model_binding", "malformed-declaration"),
        ("object-message", "carries Note", "carries Factory::Job",
         "invalid_model_binding", "malformed-declaration"),
        ("wrong-channel", "receive Seen via Queue", "receive Seen via Absent",
         "ill_typed", "type-mismatch"),
        ("effect-of-send", "receive Seen via Queue of Sent as (got: Note) { true };",
         "effect Seen of Sent as (got: Note) { true };", "ill_typed", "type-mismatch"),
    ] {
        assert!(original.contains(from));
        let file = directory.path().join(format!("{name}.native"));
        std::fs::write(&file, original.replacen(from, to, 1)).unwrap();
        refusal(directory.path(), &file, code, cause);
    }
}

#[test]
#[trace(
    "FR-218", "FR-219", "FR-220", "FR-223", "FR-224", "FR-225", "FR-226", "FR-227", "FR-228",
    "FR-229", "FR-230", "FR-231", "FR-232", "FR-233"
)]
fn authoritative_parts_saga_protocol_constructs_compile_from_their_own_repository() {
    let directory = tempfile::tempdir().unwrap();
    for file in [
        "part-flow-untimed.fsl",
        "part-flow.fsl",
        "inspection.fsl",
        "protocols/declaration/declaration-minimal.fsl",
        "protocols/declaration/declaration-on-each.fsl",
        "protocols/declaration/declaration-terminal-scheduling.fsl",
        "protocols/roles-and-channels/roles-replicated.fsl",
        "protocols/roles-and-channels/channels-loss-head.fsl",
        "protocols/roles-and-channels/channels-fifo-by-part.fsl",
        "protocols/steps/steps-all-kinds.fsl",
        "protocols/control-flow/control-parallel-all.fsl",
        "protocols/control-flow/control-parallel-any-cancel.fsl",
        "protocols/control-flow/control-repeat-progress.fsl",
        "protocols/retries-and-timeouts/timeouts-await.fsl",
    ] {
        compile(directory.path(), &saga().join("rules").join(file));
    }
}

#[test]
#[trace("FR-218", "FR-220", "FR-224", "FR-225", "FR-226", "FR-231", "FR-233")]
fn authoritative_parts_saga_rejects_invalid_protocols_at_the_right_boundary() {
    let directory = tempfile::tempdir().unwrap();
    for (file, code, cause) in [
        (
            "protocols/declaration/declaration-no-finish.fsl",
            "invalid_syntax",
            "unexpected-token",
        ),
        (
            "protocols/roles-and-channels/channels-object-payload.fsl",
            "invalid_model_binding",
            "malformed-declaration",
        ),
        (
            "protocols/steps/steps-effect-of-send.fsl",
            "ill_typed",
            "type-mismatch",
        ),
        (
            "protocols/steps/steps-receive-other-channel.fsl",
            "ill_typed",
            "type-mismatch",
        ),
        (
            "protocols/control-flow/control-repeat-no-progress.fsl",
            "unsupported_construct",
            "declaration-form",
        ),
        (
            "protocols/control-flow/control-join-omits-branch.fsl",
            "invalid_package",
            "invalid-value",
        ),
        (
            "protocols/compensation/compensation-zero-attempts.fsl",
            "ill_typed",
            "type-mismatch",
        ),
    ] {
        refusal(
            directory.path(),
            &saga().join("rules").join(file),
            code,
            cause,
        );
    }
}

/// Resolve file references without reproducing an example's source or model.
fn authoritative_request(path: &Path) -> Value {
    let mut request: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let base = path.parent().unwrap();
    let program = &mut request["request"]["program"]["source"]["file"];
    *program = json!(base.join(program.as_str().unwrap()));
    for model in request["request"]["models"].as_array_mut().unwrap() {
        let file = &mut model["source"]["file"];
        *file = json!(base.join(file.as_str().unwrap()));
    }
    for file in request["request"]["clause"]["observations"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        *file = json!(base.join(file.as_str().unwrap()));
    }
    request
}

#[test]
#[trace("FR-330", "FR-52")]
fn parts_saga_observations_distinguish_tracked_untracked_and_third_rework() {
    let directory = tempfile::tempdir().unwrap();
    for (name, exit, result) in [
        ("jobs-rework-tracked", 0, "tested"),
        ("jobs-rework-untracked", 10, "refuted"),
        ("jobs-third-rework", 10, "refuted"),
    ] {
        let request = authoritative_request(&saga().join(format!("requests/{name}.json")));
        let output = command::run(
            &write(directory.path(), &request),
            &crate::negotiation::Fr290,
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        let native = output.value.as_value();
        fsl_replay::spine::read_native_run_result(&serde_json::to_vec(native).unwrap())
            .unwrap_or_else(|error| panic!("{name}: strict native result: {error}"));
        assert_eq!(output.exit_code, exit, "{name}: {native}");
        let lifecycle: Value =
            serde_json::from_slice(&output.outcome.unwrap().to_bytes().unwrap()).unwrap();
        assert_eq!(
            lifecycle["items"][0]["result"], result,
            "{name}: {lifecycle}"
        );
    }
}

fn assessed_root() -> fsl_replay::protocol_analysis::AssessedRootDraft {
    use filament_exact::{
        EffectiveId, NodeKey, ObjectId, ObjectReference, OriginalDeclarationKey, PathSegment,
        UniverseId,
    };
    fsl_replay::protocol_analysis::AssessedRootDraft {
        observation: Some(fsl_replay::ObservationIdentity {
            authority: "example".into(),
            identity: "ix://example/parts/data/saga-start".into(),
        }),
        object: Some(ObjectReference::new(
            UniverseId::new(
                NodeKey::new(
                    "example/parts",
                    vec![
                        PathSegment::Text("model".into()),
                        PathSegment::Text("object_type".into()),
                        PathSegment::Text("ix://example/parts/Job".into()),
                    ],
                    0,
                )
                .unwrap(),
            ),
            EffectiveId::new(
                None,
                OriginalDeclarationKey::new("example/parts", "ix://example/parts/Job"),
            ),
            ObjectId::new("j1").unwrap(),
        )),
    }
}

fn handoff(directory: &Path, file: &Path) -> fsl_command::checked_handoff::CompiledHandoff {
    use fsl_command::checked_handoff::{self, CheckedHandoffRequest, Corpus};
    let output = directory.join("handoff");
    checked_handoff::write(
        &CheckedHandoffRequest {
            emit: fsl_command::emit_checked_v2::Request {
                source: file.to_path_buf(),
                package_name: "protocol-corpus".into(),
                models: vec![saga().join("model/parts-saga.semantic-ir.json")],
                libraries: Vec::new(),
                execution_points: None,
                temporal_selections: Vec::new(),
                out: None,
            },
            corpus: Corpus::None,
            native_join: fsl_package::NativeJoinLimits::new(1 << 28, 1 << 24),
        },
        &output,
    )
    .expect("source protocol handoff");
    checked_handoff::compile(&output).expect("read and compile the real handoff")
}

fn project(
    handoff: &fsl_command::checked_handoff::CompiledHandoff,
    role: &str,
) -> fsl_analyze::protocol_analysis::ProjectionOutcome {
    use fsl_replay::protocol_analysis::{ProjectionBoundsDraft, RunBudget};
    let package = handoff.admitted();
    let protocol = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| &*node.semantic_form == "protocol_clause")
        .expect("source protocol");
    let role = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            node.declaration.as_ref().is_some_and(|declaration| {
                declaration
                    .qualified_name
                    .last()
                    .is_some_and(|name| &**name == role)
            })
        })
        .expect("source role");
    fsl_analyze::protocol_analysis::project(
        package,
        fsl_analyze::protocol_analysis::ProjectionRequest {
            protocol: protocol.node_id.clone(),
            definition_key: Some(
                protocol
                    .declaration
                    .as_ref()
                    .expect("named protocol")
                    .qualified_name
                    .iter()
                    .map(|part| part.as_ref())
                    .collect::<Vec<&str>>()
                    .join("::"),
            ),
            role: role.node_id.clone(),
            profile: Some("bounded-static".into()),
            bounds: Some(ProjectionBoundsDraft {
                max_depth: Some(64),
            }),
            assessed_root: Some(assessed_root()),
            budget: RunBudget::default(),
        },
    )
    .expect("well-formed source projection request")
}

#[test]
#[trace("FR-570", "FR-574", "FR-575")]
fn source_projection_preserves_send_receive_direction_and_local_order() {
    use fsl_replay::protocol_analysis::{AnalysisOutcome, LocalEventKind};
    let directory = tempfile::tempdir().unwrap();
    let package = handoff(directory.path(), &corpus("original/exchange.native"));
    for (role, sends) in [("Sender", true), ("Receiver", false)] {
        let outcome = project(&package, role);
        let AnalysisOutcome::Proved(witness) = outcome.outcome else {
            panic!("{role}: expected a proved projection: {outcome:?}");
        };
        assert_eq!(witness.local_graph.events.len(), 1, "{role}: {witness:?}");
        assert!(witness.local_graph.edges.is_empty(), "{role}: {witness:?}");
        let kind = &witness.local_graph.events[0].kind;
        if sends {
            assert!(matches!(kind, LocalEventKind::Send { .. }), "{kind:?}");
        } else {
            assert!(matches!(kind, LocalEventKind::Receive { .. }), "{kind:?}");
        }
        assert_eq!(witness.correspondence.0.len(), 1, "{role}: {witness:?}");
    }
}

fn analysis_request(file: &Path) -> Value {
    let mut request = compile_request(file);
    request["format"] = json!("native-analyze/1");
    request["request"]["documents"] = json!([{"file":saga().join("data/saga-start.json")}]);
    let binding = json!({"type":"model-binding","selection":"example/parts"});
    request["request"]["subject"] = json!({"protocol":{"protocol":"Exchange",
        "over":{"reference":{"population":"ix://example/parts/jobs","key":"j1"}},
        "roles":{"Sender":binding,"Receiver":binding}}});
    request["request"]["items"] = json!([{"claim":"Exchange","kind":"global-conformance",
        "temporal":{"item_kind":"deadlock-freedom"}}]);
    request
}

#[test]
#[trace("FR-636", "FR-330")]
fn public_source_analysis_distinguishes_consumed_and_stranded_messages() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read_to_string(corpus("original/exchange.native")).unwrap();
    let stranded = directory.path().join("stranded.native");
    std::fs::write(
        &stranded,
        original.replace(
            "receive Seen via Queue of Sent as (got: Note) { true };",
            "send Second via Queue as (second: Note) { true };",
        ),
    )
    .unwrap();
    for (file, expected) in [
        (corpus("original/exchange.native"), "proved"),
        (stranded, "refuted"),
    ] {
        let output = command::analyze(
            &write(directory.path(), &analysis_request(&file)),
            &crate::negotiation::Fr290,
            &Cancel::new(),
        )
        .expect("a protocol analysis settles");
        let item = &output.value.as_value()["items"][0];
        assert_eq!(item["record"]["result"], expected, "{item}");
        assert_eq!(
            output.exit_code,
            if expected == "proved" { 0 } else { 10 },
            "{item}"
        );
        if expected == "refuted" {
            assert!(item.get("counterexample").is_some(), "{item}");
        }
    }
}

#[test]
#[trace("FR-225", "FR-226", "FR-227", "FR-230")]
fn original_choice_parallel_and_repeat_have_positive_negative_and_zero_bound_controls() {
    let directory = tempfile::tempdir().unwrap();
    let original = std::fs::read_to_string(corpus("original/exchange.native")).unwrap();
    let marker = "receive Seen via Queue of Sent as (got: Note) { true };";
    for (name, control) in [
        ("exclusive-choice", "choice Decision by Receiver visible (got.ok) { case yes when { got.ok } sequence Yes {} case no when { not got.ok } sequence No {} }"),
        ("zero-repeat", "repeat Again by Receiver visible (got.ok) max 0 while { got.ok } sequence Cycle { commit Tick by Receiver as (tick: Boolean) { true }; } exhausted sequence Exhausted {}"),
        ("bounded-repeat", "repeat Again by Receiver visible (got.ok) max 2 while { got.ok } sequence Cycle { commit Tick by Receiver as (tick: Boolean) { true }; } exhausted sequence Exhausted {}"),
    ] {
        let file = directory.path().join(format!("{name}.native"));
        std::fs::write(&file,original.replace(marker,&format!("{marker}\n{control}"))).unwrap();
        compile(directory.path(),&file);
    }
    for (name, control, code, cause) in [
        ("overlapping-choice", "choice Decision by Receiver visible (got.ok) { case a when { got.ok } sequence A {} case b when { got.ok } sequence B {} }", "undefined_expression", "unproved-exhaustiveness"),
        ("nonexhaustive-choice", "choice Decision by Receiver visible (got.ok) { case a when { got.ok } sequence A {} case b when { false } sequence B {} }", "undefined_expression", "unproved-exhaustiveness"),
        ("no-progress-repeat", "repeat Again by Receiver visible (got.ok) max 2 while { got.ok } sequence Cycle {} exhausted sequence Exhausted {}", "unsupported_construct", "declaration-form"),
    ] {
        let file = directory.path().join(format!("{name}.native"));
        std::fs::write(&file,original.replace(marker,&format!("{marker}\n{control}"))).unwrap();
        refusal(directory.path(),&file,code,cause);
    }
    let start = original.find("  run sequence Main {").unwrap();
    let end = original.find("  finish Done").unwrap();
    for (name, join) in [
        ("all", "all [A, B]"),
        ("any-cancel", "any [A, B] outstanding cancel"),
        ("any-continue", "any [A, B] outstanding continue"),
    ] {
        let file = directory.path().join(format!("parallel-{name}.native"));
        let program = format!("{} run parallel Both {{ branch A sequence First {{ commit One by Sender as (one: Boolean) {{ true }}; }} branch B sequence Second {{ commit Two by Receiver as (two: Boolean) {{ true }}; }} }} join {join};\n{}",&original[..start],&original[end..]);
        std::fs::write(&file, program).unwrap();
        compile(directory.path(), &file);
    }
}

#[test]
#[trace("FR-575", "FR-570")]
fn source_projection_names_each_construct_outside_bounded_static() {
    use fsl_replay::protocol_analysis::AnalysisOutcome;
    for (file, role, slug) in [
        (
            "protocols/declaration/declaration-on-each.fsl",
            "Planning",
            "activation-on-each",
        ),
        (
            "protocols/roles-and-channels/roles-replicated.fsl",
            "Worker",
            "dynamic-role-population",
        ),
        (
            "protocols/control-flow/control-parallel-any-cancel.fsl",
            "Machining",
            "unsupported-join",
        ),
        (
            "protocols/control-flow/control-repeat-progress.fsl",
            "Inspection",
            "repeat",
        ),
        (
            "protocols/retries-and-timeouts/timeouts-await.fsl",
            "Machining",
            "timed-await",
        ),
        (
            "protocols/steps/steps-all-kinds.fsl",
            "Machining",
            "unsupported-control",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let package = handoff(directory.path(), &saga().join("rules").join(file));
        let result = project(&package, role);
        let AnalysisOutcome::Unsupported(cause) = result.outcome else {
            panic!("{file}: expected unsupported {slug}: {result:?}");
        };
        assert_eq!(cause.slug.as_str(), slug, "{file}: {cause:?}");
        assert!(
            package
                .admitted()
                .resolved_graph()
                .nodes
                .iter()
                .any(|node| node.node_id == cause.node),
            "{file}: the refusal names the actual source construct"
        );
    }
}

#[test]
#[trace("FR-570", "FR-574")]
fn source_projection_refutes_a_choice_hidden_from_the_target() {
    use fsl_replay::protocol_analysis::AnalysisOutcome;
    let directory = tempfile::tempdir().unwrap();
    let package = handoff(directory.path(), &corpus("original/hidden-choice.native"));
    let target = project(&package, "Target");
    assert!(
        matches!(target.outcome, AnalysisOutcome::Refuted(_)),
        "{target:?}"
    );
    let owner = project(&package, "Picker");
    let AnalysisOutcome::Proved(witness) = owner.outcome else {
        panic!("the deciding role knows its own choice: {owner:?}");
    };
    assert_eq!(witness.merge_evidence.len(), 1, "{witness:?}");
}

fn root_wire() -> Value {
    use filament_exact::{NodeKey, PathSegment};
    let universe = NodeKey::new(
        "example/parts",
        vec![
            PathSegment::Text("model".into()),
            PathSegment::Text("object_type".into()),
            PathSegment::Text("ix://example/parts/Job".into()),
        ],
        0,
    )
    .unwrap();
    json!({"observation":{"authority":"example","identity":"ix://example/parts/data/saga-start"},
        "subject":{"tag":"object","universe":fsl_foundation::node_wire::node_key_to_text(&universe).unwrap(),
            "type":serde_json::to_string(&json!([{"package":"example/parts","node":"ix://example/parts/Job"}])).unwrap(),
            "object_identity":"j1"}})
}

#[test]
#[trace("FR-570", "FR-571", "FR-636", "FR-574")]
fn public_source_projection_realizability_and_deadlock_agree_on_bounded_static_programs() {
    let directory = tempfile::tempdir().unwrap();
    for (file, protocol, roles) in [
        (
            corpus("original/exchange.native"),
            "Exchange",
            vec!["Sender", "Receiver"],
        ),
        (
            saga().join("rules/protocols/declaration/declaration-minimal.fsl"),
            "PartFlow",
            vec!["Planning"],
        ),
    ] {
        let mut request = analysis_request(&file);
        let bindings = roles
            .iter()
            .map(|role| {
                (
                    (*role).to_owned(),
                    json!({"type":"model-binding","selection":"example/parts"}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        request["request"]["subject"]["protocol"]["protocol"] = json!(protocol);
        request["request"]["subject"]["protocol"]["roles"] = json!(bindings);
        let mut items = roles
            .iter()
            .map(|role| {
                json!({"claim":protocol,"kind":"local-projection",
            "options":{"role":role},"assessed_root":root_wire(),
            "temporal":{"profile":"bounded-static","max_depth":64}})
            })
            .collect::<Vec<_>>();
        items.push(
            json!({"claim":protocol,"kind":"realizability","assessed_root":root_wire(),
            "premises":{"role_knowledge":[],"environment":[]},
            "temporal":{"profile":"bounded-static","max_depth":64,"max_history":16}}),
        );
        items.push(json!({"claim":protocol,"kind":"global-conformance","temporal":{"item_kind":"deadlock-freedom"}}));
        request["request"]["items"] = json!(items);
        let output = command::analyze(
            &write(directory.path(), &request),
            &crate::negotiation::Fr290,
            &Cancel::new(),
        )
        .expect("public protocol analysis");
        let records = output.value.as_value()["items"]
            .as_array()
            .expect("one result per requested analysis");
        assert_eq!(records.len(), roles.len() + 2, "{}", output.value);
        for item in records {
            assert_eq!(item["record"]["result"], "proved", "{protocol}: {item}");
        }
        assert_eq!(output.exit_code, 0, "{}", output.value);
    }
}
