// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-494: producer-linked dispatch decisions survive checked-package
//! emission, and IR's typed dispatch reader refuses substituted witnesses.

use filament_contract_model::{
    CheckedEffectiveId, CheckedOccurrence, CheckedPackageEvidence, CheckedPackageReadLimits,
    CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_exact::{Cancel, EffectiveId, NodeKey};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::{
    self, CheckedUnit, DependencyInput, PackageLimits, ParseRequest, SpineLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const PACKAGE: &str = "example/config-version";
const EXAMPLE: &str = include_str!("../../examples/config-version/model.semantic-ir.json");

fn operation(owner: &str, name: &str, redefines: Option<&str>) -> Value {
    let identity = format!("ix://{PACKAGE}/{owner}/{name}");
    let mut result = json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://filament/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false
        },
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.fspec", "startLine": 1, "startColumn": 1
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []}
    });
    if let Some(parent) = redefines {
        result["redefines"] = json!(format!("ix://{PACKAGE}/{parent}/{name}"));
    }
    result
}

fn optional_note_operation() -> Value {
    let mut note = operation("ConfigVersion", "note", None);
    let identity = format!("ix://{PACKAGE}/ConfigVersion/note/secondApprover");
    note["params"] = json!([{
        "identity": identity,
        "name": "secondApprover",
        "typeRef": format!("ix://{PACKAGE}/ConfigVersion"),
        "presence": "optional",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }},
    }]);
    note
}

fn model() -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("example model parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let root = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    root["operations"] = json!([
        operation("ConfigVersion", "size", None),
        operation("ConfigVersion", "probe", None)
    ]);
    for (name, parent, ops) in [
        (
            "Sub",
            "ConfigVersion",
            vec![
                operation("Sub", "size", Some("ConfigVersion")),
                operation("Sub", "probe", Some("ConfigVersion")),
            ],
        ),
        ("Leaf", "Sub", vec![operation("Leaf", "size", Some("Sub"))]),
        (
            "Other",
            "",
            vec![
                operation("Other", "size", None),
                operation("Other", "probe", None),
            ],
        ),
    ] {
        let identity = format!("ix://{PACKAGE}/{name}");
        types.push(json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"},
            "roles": [], "origin": {"generated": {
                "generatorIdentity": identity, "generatorVersion": "1.0.0", "inputIdentities": [identity]
            }},
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "supertypes": if parent.is_empty() { json!([]) } else { json!([format!("ix://{PACKAGE}/{parent}")]) },
            "fields": [], "operations": ops
        }));
    }
    serde_json::to_vec(&envelope).expect("model JSON")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedCandidate {
    selection_key: EffectiveId,
    operation_body: NodeKey,
    effective_precondition: Option<NodeKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedDecision {
    application: NodeKey,
    declaring: NodeKey,
    operation: String,
    context: NodeKey,
    candidates: Vec<CapturedCandidate>,
}

fn compile(declarations: &str) -> CheckedUnit {
    compile_with_model(model(), declarations)
}

fn compile_with_model(document: Vec<u8>, declarations: &str) -> CheckedUnit {
    let packages =
        fsl_semantics::model::intake::package_input([document.as_slice()]).expect("model intake");
    let source_text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = {PACKAGE:?};\n\
         {declarations}\n"
    );
    let source = SourceIdentity::new("test", "dispatch");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let failure = |failure| spine::refusal_or_fault(failure).expect("no internal front-end fault");
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "dispatch.native",
            bytes: source_text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("source parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("model selected")
        .into_value();
    spine::check(
        &parsed,
        PACKAGE.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("source checks")
    .into_value()
}

fn optional_parameter_model() -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("example model parses");
    let config_version = envelope["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    config_version["operations"] = json!([optional_note_operation()]);
    serde_json::to_vec(&envelope).expect("optional parameter model JSON")
}

fn optional_note_source(argument: &str) -> String {
    format!(
        "body noteA using v on Config::ConfigVersion::note(secondApprover) {{ 1 }}\n\
         invariant Calls using v on Config::ConfigVersion at current {{ self.note({argument}) = 1 }}\n"
    )
}

fn capture_dispatch_decisions(checked: &CheckedUnit) -> Vec<CapturedDecision> {
    let graph = checked.package().graph();
    graph
        .dispatch_producer_decisions()
        .iter()
        .map(|((application, _), decision)| CapturedDecision {
            application: application.clone(),
            declaring: decision.declaring_node.clone(),
            operation: decision.operation.selected_operation.clone(),
            context: decision.context_node.clone(),
            candidates: decision
                .table
                .entries()
                .iter()
                .map(|(selection_key, candidate)| CapturedCandidate {
                    selection_key: selection_key.clone(),
                    operation_body: graph
                        .function_identity_at(candidate.body)
                        .expect("linked candidate body has a checked function identity"),
                    effective_precondition: candidate.precondition.map(|index| {
                        graph
                            .function_identity_at(index)
                            .expect("effective precondition has a checked function identity")
                    }),
                })
                .collect(),
        })
        .collect()
}

fn emit_checked(checked: &CheckedUnit) -> Vec<u8> {
    let failure = |failure| spine::refusal_or_fault(failure).expect("no internal package fault");
    spine::package(checked, PackageLimits::default(), &Cancel::new())
        .map_err(failure)
        .expect("package emits")
        .into_value()
        .package()
        .bytes()
        .to_vec()
}

fn emit_with_capture(declarations: &str) -> (Vec<u8>, Vec<CapturedDecision>) {
    let checked = compile(declarations);
    let captured = capture_dispatch_decisions(&checked);
    (emit_checked(&checked), captured)
}

fn checked_effective_id(identity: &EffectiveId) -> CheckedEffectiveId {
    let mut current = None;
    for original in identity.owner_chain() {
        current = Some(CheckedEffectiveId {
            owner_effective_type: current.map(Box::new),
            original: filament_contract_model::CheckedOriginalDeclarationId {
                package: original.package().into(),
                node: original.node().into(),
            },
        });
    }
    current.expect("effective declaration identity has an original declaration")
}

fn assert_captured_decisions(
    bytes: &[u8],
    package: &CheckedPackageV2,
    captured: &[CapturedDecision],
) {
    let wire: Value = serde_json::from_slice(bytes).unwrap();
    let wire_graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(bytes)
        .expect("wire graph resolves structural identities");
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("dispatch carrier");
    assert_eq!(rows.len(), captured.len());
    for decision in captured {
        let application = wire_graph
            .wire_id(&decision.application)
            .expect("captured application is emitted");
        let row = rows
            .iter()
            .find(|row| row["application"] == serde_json::to_value(&application).unwrap())
            .expect("row uses captured application identity");
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        assert!(
            package.source_map().iter().any(|entry| {
                entry.node_id == application
                    && entry.role == occurrence.role
                    && entry.ordinal == occurrence.ordinal
            }),
            "row application and occurrence pair with the source map"
        );
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("typed reader returns exact producer row");
        assert_eq!(view.table.application, application);
        assert_eq!(
            view.table.operation.declaring,
            wire_graph
                .wire_id(&decision.declaring)
                .expect("declaring node is emitted")
        );
        assert_eq!(view.table.operation.operation.as_ref(), decision.operation);
        assert_eq!(
            view.table.context,
            wire_graph
                .wire_id(&decision.context)
                .expect("receiver context is emitted")
        );
        assert_eq!(view.candidates().len(), decision.candidates.len());
        for (candidate, expected) in view.candidates().iter().zip(&decision.candidates) {
            assert_eq!(
                candidate.selection_key,
                checked_effective_id(&expected.selection_key)
            );
            assert_eq!(
                candidate.operation_body,
                wire_graph
                    .wire_id(&expected.operation_body)
                    .expect("linked operation body identity is emitted")
            );
            assert_eq!(
                candidate.effective_precondition,
                expected
                    .effective_precondition
                    .as_ref()
                    .map(|key| wire_graph
                        .wire_id(key)
                        .expect("precondition identity is emitted"))
            );
            assert_eq!(candidate.selected_operation, view.table.operation);
        }
    }
}

fn evidence(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("features")
    {
        evidence.support_feature(feature.as_str().expect("feature name"));
    }
    let document = model();
    for (identity, document) in
        fsl_semantics::model::intake::package_input([document.as_slice()]).expect("model intake")
    {
        evidence
            .insert_domain_package_document(identity.as_str(), document)
            .expect("evidence");
    }
    evidence
}

fn admitted(bytes: &[u8]) -> CheckedPackageV2 {
    let wire: Value = serde_json::from_slice(bytes).expect("emitted JSON");
    match CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence(&wire)) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("typed IR reader rejected emitted dispatch data: {other:?}"),
    }
}

#[trace("TC-1282", "FR-494-AC-1", "FR-494-AC-3", "FR-494-AC-4")]
#[test]
fn producer_dispatch_table_reads_with_ordered_candidate_witnesses() {
    let (bytes, captured) = emit_with_capture(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let package = admitted(&bytes);
    assert_captured_decisions(&bytes, &package, &captured);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("dispatch carrier");
    let family_rows: Vec<_> = rows
        .iter()
        .filter(|row| row["operation"]["operation"] == "size")
        .collect();
    assert_eq!(
        family_rows.len(),
        1,
        "one exact row for the dispatched call"
    );
    for row in family_rows {
        let application: filament_contract_model::CheckedNodeId =
            serde_json::from_value(row["application"].clone()).unwrap();
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("typed reader returns exact producer row");
        assert_eq!(view.table.application, application);
        let candidates = view.candidates();
        assert_eq!(candidates.len(), 3);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.selected_operation == view.table.operation));
        assert!(candidates
            .iter()
            .all(|candidate| candidate.effective_precondition.is_none()));
        assert!(candidates
            .iter()
            .all(|candidate| wire["semantic_graph"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|node| node["node_tag"] == "function"
                    && node["node_id"]
                        == serde_json::to_value(&candidate.operation_body).unwrap())));
        assert_eq!(
            candidates[0].selection_key.original.node.as_ref(),
            "ix://example/config-version/ConfigVersion"
        );
        assert_eq!(
            candidates[1].selection_key.original.node.as_ref(),
            "ix://example/config-version/Leaf"
        );
        assert_eq!(
            candidates[2].selection_key.original.node.as_ref(),
            "ix://example/config-version/Sub"
        );
    }
    let mut missing: Value = serde_json::from_slice(&bytes).unwrap();
    missing["checked_term_data"]["dispatch_tables"][0]["candidates"][0]
        .as_object_mut()
        .unwrap()
        .remove("selected_operation");
    let refusal = match CheckedPackageV2::read(
        &serde_json::to_vec(&missing).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence(&missing),
    ) {
        CheckedPackageV2ReadResult::Refused(refusal) => refusal,
        other => panic!("missing candidate witness must refuse: {other:?}"),
    };
    assert_eq!(
        refusal.loci[0]
            .path
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("/checked_term_data/dispatch_tables/0/candidates/0")
    );
    let graph_ids: Vec<_> = package
        .resolved_graph()
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect();
    assert_eq!(
        graph_ids.len(),
        package.resolved_graph().nodes.len(),
        "graph node identities remain unique"
    );
    let mut without_carrier = wire.clone();
    without_carrier
        .as_object_mut()
        .unwrap()
        .remove("checked_term_data");
    let without_carrier_bytes = serde_json::to_vec(&without_carrier).unwrap();
    // No local function body holds a term, so no `body_locations` row is
    // owed and the package reads without the carrier (FSpec FR-322).
    let without_carrier = match CheckedPackageV2::read(
        &without_carrier_bytes,
        CheckedPackageReadLimits::bounded(),
        &evidence(&without_carrier),
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("graph-only package remains admitted: {other:?}"),
    };
    assert_eq!(package.graph(), without_carrier.graph());

    let (with_precondition, precondition_decisions) = emit_with_capture(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre inherited using v on Config::ConfigVersion::size { true }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n",
    );
    let precondition_package = admitted(&with_precondition);
    assert_captured_decisions(
        &with_precondition,
        &precondition_package,
        &precondition_decisions,
    );
    assert!(precondition_decisions
        .iter()
        .flat_map(|decision| &decision.candidates)
        .any(|candidate| candidate.effective_precondition.is_some()));
    let pre_wire: Value = serde_json::from_slice(&with_precondition).unwrap();
    let pre_row = &pre_wire["checked_term_data"]["dispatch_tables"][0];
    let pre_application: filament_contract_model::CheckedNodeId =
        serde_json::from_value(pre_row["application"].clone()).unwrap();
    let pre_occurrence: CheckedOccurrence =
        serde_json::from_value(pre_row["occurrence"].clone()).unwrap();
    let pre_view = precondition_package
        .dispatch_candidates(&pre_application, &pre_occurrence, 100)
        .expect("typed precondition-bearing dispatch row is admitted");
    assert_eq!(
        pre_view.candidates().len(),
        precondition_decisions[0].candidates.len()
    );
    assert!(pre_view
        .candidates()
        .iter()
        .zip(&precondition_decisions[0].candidates)
        .any(|(candidate, expected)| {
            candidate.effective_precondition.is_some() && expected.effective_precondition.is_some()
        }));
}

/// FR-103-AC-12: omitting an optional operation argument lowers exactly as
/// passing `absent`; the call carries the normalized `none` argument.
#[trace("FR-103-AC-12")]
#[test]
fn an_omitted_optional_parameter_matches_an_explicit_absent_call() {
    let document = optional_parameter_model();
    let omitted = emit_checked(&compile_with_model(
        document.clone(),
        &optional_note_source(""),
    ));
    let explicit_absent = emit_checked(&compile_with_model(
        document,
        &optional_note_source("absent"),
    ));
    let omitted_wire: Value = serde_json::from_slice(&omitted).expect("omitted package JSON");
    let explicit_wire: Value =
        serde_json::from_slice(&explicit_absent).expect("explicit absent package JSON");

    assert_eq!(
        omitted_wire["semantic_graph"]["nodes"], explicit_wire["semantic_graph"]["nodes"],
        "both calls lower to the same semantic graph"
    );
    assert_eq!(
        omitted_wire["checked_term_data"], explicit_wire["checked_term_data"],
        "both calls produce the same checked term data"
    );
    let dispatch_calls: Vec<_> = omitted_wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "call")
        .collect();
    assert_eq!(dispatch_calls.len(), 1, "one dispatch call is emitted");
    let arguments = dispatch_calls[0]["body"]["arguments"]
        .as_array()
        .expect("dispatch call arguments");
    assert_eq!(arguments.len(), 2, "receiver and optional argument");
    let normalized_absent_target = &arguments[1]["target"];
    let normalized_absent_path = normalized_absent_target["path"]
        .as_array()
        .expect("normalized absent target path");
    assert!(
        arguments[1]["term"] == "reference"
            && normalized_absent_path.iter().any(|part| part == "value")
            && normalized_absent_path.iter().any(|part| part == "literal")
            && normalized_absent_path.iter().any(|part| part == "none"),
        "second dispatch argument is the normalized none literal: {:#}",
        arguments[1]
    );
}

#[trace("TC-1283", "FR-494-AC-2", "FR-494-AC-4")]
#[test]
fn dispatch_candidate_witness_substitution_refuses_at_candidate_locus() {
    // Every redefinition in the called family carries its own body: the typed
    // reader joins a candidate to the row of its receiver's effective
    // operation owner, and a body-less redefinition's row is `no_body`.
    let declarations = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre otherCall using v on Config::Other::probe { self.size() > 0 }\n";
    let (bytes, captured) = emit_with_capture(declarations);
    let package = admitted(&bytes);
    assert_captured_decisions(&bytes, &package, &captured);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let original_rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .unwrap();
    assert_eq!(original_rows.len(), 2);
    assert_ne!(
        original_rows[0]["application"],
        original_rows[1]["application"]
    );
    // Occurrence ordinals are graph-local per node, so the two calls can both
    // be ordinal zero. Their exact application/occurrence pairs must resolve
    // to distinct producer rows below.
    assert_ne!(original_rows[0]["context"], original_rows[1]["context"]);
    assert_ne!(original_rows[0]["operation"], original_rows[1]["operation"]);
    for row in original_rows {
        let application: filament_contract_model::CheckedNodeId =
            serde_json::from_value(row["application"].clone()).unwrap();
        let occurrence: CheckedOccurrence =
            serde_json::from_value(row["occurrence"].clone()).unwrap();
        let view = package
            .dispatch_candidates(&application, &occurrence, 100)
            .expect("each exact call row reads independently");
        assert!(view
            .candidates()
            .iter()
            .all(|candidate| candidate.selected_operation == view.table.operation));
    }

    let permuted_declarations =
        "pre otherCall using v on Config::Other::probe { self.size() > 0 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body sizeA using v on Config::ConfigVersion::size() { 1 }\n";
    let (permuted_bytes, permuted_capture) = emit_with_capture(permuted_declarations);
    let permuted_package = admitted(&permuted_bytes);
    assert_captured_decisions(&permuted_bytes, &permuted_package, &permuted_capture);
    assert_eq!(
        captured, permuted_capture,
        "checked producer identities are stable"
    );
    let original_graph = fsl_semantics::check::wire_id::WireGraph::of_package_bytes(&bytes)
        .expect("original wire graph");
    let permuted_graph =
        fsl_semantics::check::wire_id::WireGraph::of_package_bytes(&permuted_bytes)
            .expect("permuted wire graph");
    let original_rows = serde_json::from_slice::<Value>(&bytes).unwrap()["checked_term_data"]
        ["dispatch_tables"]
        .clone();
    let permuted_rows = serde_json::from_slice::<Value>(&permuted_bytes).unwrap()
        ["checked_term_data"]["dispatch_tables"]
        .clone();
    assert_eq!(
        original_graph.structural_value(&original_rows).unwrap(),
        permuted_graph.structural_value(&permuted_rows).unwrap(),
        "canonical row order and all checked identities survive declaration permutation"
    );
    let mut row_keys: Vec<_> = serde_json::from_value::<Vec<Value>>(original_rows.clone())
        .unwrap()
        .iter()
        .map(|row| {
            let id: filament_contract_model::CheckedNodeId =
                serde_json::from_value(row["application"].clone()).unwrap();
            original_graph.key(&id).unwrap()
        })
        .collect();
    let canonical_keys = row_keys.clone();
    row_keys.sort();
    assert_eq!(
        row_keys, canonical_keys,
        "rows use canonical application order"
    );

    let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = wire["checked_term_data"]["dispatch_tables"]
        .as_array_mut()
        .unwrap();
    let root_index = rows
        .iter()
        .position(|row| row["context"].to_string().contains("ConfigVersion"))
        .expect("root family row");
    let other_operation = rows
        .iter()
        .find(|row| row["context"].to_string().contains("Other"))
        .expect("other family row")["operation"]
        .clone();
    let candidate = &mut rows[root_index];
    let candidates = candidate["candidates"].as_array_mut().unwrap();
    candidates[0]["selected_operation"] = other_operation;
    let refusal = match CheckedPackageV2::read(
        &serde_json::to_vec(&wire).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence(&wire),
    ) {
        CheckedPackageV2ReadResult::Refused(refusal) => refusal,
        other => panic!("substituted candidate witness must refuse: {other:?}"),
    };
    assert_eq!(
        refusal.loci[0]
            .path
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("/checked_term_data/dispatch_tables/0/candidates/0/selected_operation")
    );
}

/// FR-576-AC-4, AC-7: a model with several bodies reads back through the typed
/// reader, which joins each dispatch candidate to the `operation_bodies` row
/// of its receiver's effective operation owner and requires them to agree.
#[trace("FR-576-AC-7", "TC-1011")]
#[test]
fn operation_bodies_of_a_multi_body_model_read_back_through_the_typed_reader() {
    let (bytes, _) = emit_with_capture(
        "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
         body sizeB using v on Config::Sub::size() { 2 }\n\
         body otherSize using v on Config::Other::size() { 9 }\n\
         body sizeC using v on Config::Leaf::size() { 3 }\n\
         pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
         pre otherCall using v on Config::Other::probe { self.size() > 0 }\n",
    );
    let package = admitted(&bytes);
    let rows = package
        .operation_body_rows()
        .expect("the package carries operation_bodies");
    let with_body = rows
        .iter()
        .filter(|row| {
            matches!(
                row.body,
                filament_contract_model::OperationBody::Function { .. }
            )
        })
        .count();
    assert_eq!(with_body, 4, "one body row per authored body: {rows:?}");
    assert_eq!(rows.len(), 7, "one row per operation: {rows:?}");
}

const SQRT_PRECONDITION_UNIT: &str = "body sizeA using v on Config::ConfigVersion::size() { 1 }\n\
     body sizeB using v on Config::Sub::size() { 2 }\n\
     body sizeC using v on Config::Leaf::size() { 3 }\n\
     pre rootCall using v on Config::ConfigVersion::probe { self.size() > 0 }\n\
     pre sqrtPre using v on Config::ConfigVersion::size { \
     filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) <= \
     filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) }\n";

/// FR-531-AC-39: the clause function of a precondition whose body holds two
/// `sqrt` terms carries one `body_locations` row per term of its body walk,
/// the two `sqrt` rows at the paths `[0]` and `[1]`, and the strict reader
/// refuses the package once the second row is removed.
#[trace("TC-3133", "FR-531-AC-39")]
#[test]
fn a_clause_function_body_carries_its_location_rows() {
    let (bytes, _) = emit_with_capture(SQRT_PRECONDITION_UNIT);
    admitted(&bytes);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = wire["checked_term_data"]["body_locations"]
        .as_array()
        .expect("body_locations carrier");
    let is_sqrt = |row: &Value| {
        row["application"]["path"]
            .to_string()
            .contains("float64.sqrt")
    };
    let sqrt_paths: Vec<Value> = rows
        .iter()
        .filter(|row| is_sqrt(row))
        .map(|row| row["location"]["path"].clone())
        .collect();
    assert_eq!(
        sqrt_paths,
        vec![json!([0]), json!([1])],
        "the two sqrt terms carry the checker's paths: {rows:?}"
    );
    let function = rows
        .iter()
        .find(|row| is_sqrt(row))
        .map(|row| row["location"]["origin"].clone())
        .unwrap();
    assert!(
        rows.iter()
            .filter(|row| is_sqrt(row))
            .all(|row| row["location"]["origin"] == function),
        "both rows belong to the clause function"
    );
    let function_node = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("graph nodes")
        .iter()
        .find(|node| node["node_id"] == function["function"])
        .expect("the rows' function node is emitted");
    let clause = function_node["body"]["members"]
        .as_array()
        .and_then(|members| members.iter().find(|member| member["name"] == "clause"))
        .map(|member| member["value"]["value"].clone());
    assert_eq!(
        clause,
        Some(json!("precondition")),
        "the rows locate a clause function: {function_node}"
    );

    let refused_at = |wire: &Value| match CheckedPackageV2::read(
        &serde_json::to_vec(wire).unwrap(),
        CheckedPackageReadLimits::bounded(),
        &evidence(wire),
    ) {
        CheckedPackageV2ReadResult::Refused(refusal) => refusal.loci[0]
            .path
            .as_ref()
            .map(ToString::to_string)
            .expect("the refusal has a path"),
        other => panic!("the package must be refused: {other:?}"),
    };
    let mut without_second = wire.clone();
    let second = without_second["checked_term_data"]["body_locations"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| is_sqrt(row) && row["location"]["path"] == json!([1]))
        .unwrap();
    without_second["checked_term_data"]["body_locations"]
        .as_array_mut()
        .unwrap()
        .remove(second);
    assert_eq!(
        refused_at(&without_second),
        "/checked_term_data/body_locations"
    );

    let mut without_carrier = wire.clone();
    without_carrier["checked_term_data"]["body_locations"] = json!([]);
    assert_eq!(
        refused_at(&without_carrier),
        "/checked_term_data/body_locations"
    );
}

/// FR-531-AC-39, FSpec FR-322: one unit holding every shape of function body
/// the producer locates (arithmetic, a conversion, `sqrt`, an attribute read
/// through `self`, an implicit coercion, `let`, `if`, `case`, a node
/// reached twice and a clause function) is emitted and read back through the
/// strict reader, which owes a row for every term of every body.
#[trace("TC-3133", "FR-531-AC-39")]
#[test]
fn a_unit_of_every_body_shape_reads_back_through_the_strict_reader() {
    let float = "Float64[nearest-even]";
    let sqrt = "filament::value::ieee::sqrt";
    let (bytes, _) = emit_with_capture(&format!(
        "body sizeA using v on Config::ConfigVersion::size() {{ 1 }}\n\
         body sizeB using v on Config::Sub::size() {{ 2 }}\n\
         body sizeC using v on Config::Leaf::size() {{ 3 }}\n\
         union Shape {{ Empty, Circle(Integer), Rect(Integer, Integer) }}\n\
         function arith using v(x: Integer, y: Integer): Integer pure {{ x + y * 2 - 1 }}\n\
         function conv using v(a: Rational[-10, 10; 1, 10]): {float} pure {{ convert<{float}>(a) }}\n\
         function root using v(p: {float}, q: {float}): Boolean pure {{ {sqrt}(p) <= {sqrt}(q) }}\n\
         function coerce using v(a: Int[0, 9], b: Integer): Integer pure {{ a + b }}\n\
         function twice using v(a: Integer): Integer pure {{ let t = a + 1 in t * t }}\n\
         function pick using v(a: Boolean, x: Integer): Integer pure {{ if a then x + 1 else 0 }}\n\
         function area using v(s: Shape): Integer pure \
         {{ case s {{ Empty: 0; Circle(r): r; Rect(w, h): w * h; }} }}\n\
         pre rootCall using v on Config::ConfigVersion::probe {{ self.size() > 0 }}\n\
         pre sqrtPre using v on Config::ConfigVersion::size {{ {sqrt}(float64(bits: 0x4000000000000000)) <= {sqrt}(float64(bits: 0x4000000000000000)) and self.versionNumber = self.versionNumber }}\n"
    ));
    admitted(&bytes);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let rows = wire["checked_term_data"]["body_locations"]
        .as_array()
        .expect("body_locations carrier");
    let mut functions: Vec<String> = rows
        .iter()
        .map(|row| row["location"]["origin"]["name"].to_string())
        .collect();
    functions.sort();
    functions.dedup();
    assert_eq!(
        functions,
        [
            "area",
            "arith",
            "coerce",
            "conv",
            "ix://example/config-version/ConfigVersion/size.precondition",
            "pick",
            "root",
            "twice",
        ]
        .map(|name| json!(name).to_string()),
        "a row group per body with a term"
    );
}
