// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1011 (FR-576-AC-1 to AC-9): the emission carries, for every operation
//! FR-496 keys, the checked body function node of an operation with an
//! `operation-body` or the explicit absence `no_body`, whether or not a
//! dispatched call reaches the body. The model is the
//! `example/config-version` package with `ConfigVersion` (`A`) and the
//! subtypes the cases need.

use filament_contract_model::CheckedNodeId;

use super::*;
use filament_contract_model::OperationBody;
use filament_semantic_value::checking::CheckMode;

const PACKAGE: &str = "example/config-version";
const EXAMPLE: &str = include_str!("../../../../examples/config-version/model.semantic-ir.json");

fn query(owner: &str, name: &str, redefines: Option<&str>) -> Value {
    let identity = format!("ix://{PACKAGE}/{owner}/{name}");
    let mut operation = json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://filament/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []},
    });
    if let Some(parent) = redefines {
        operation["redefines"] = json!(format!("ix://{PACKAGE}/{parent}/{name}"));
    }
    operation
}

/// `ConfigVersion` declaring `operations`, and each of `subtypes` as
/// `(name, supertype, operations)`; an empty supertype declares none.
fn package(operations: Vec<Value>, subtypes: &[(&str, &str, Vec<Value>)]) -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("the example parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let config_version = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    config_version["operations"] = json!(operations);
    for (name, supertype, operations) in subtypes {
        let identity = format!("ix://{PACKAGE}/{name}");
        types.push(json!({
            "identity": identity,
            "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"},
            "roles": [],
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": if supertype.is_empty() {
                json!([])
            } else {
                json!([format!("ix://{PACKAGE}/{supertype}")])
            },
            "fields": [],
            "operations": operations,
        }));
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn unit(declarations: &str, aliases: &[&str]) -> String {
    let models: String = aliases
        .iter()
        .map(|alias| format!("model {alias} = {PACKAGE:?};\n"))
        .collect();
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         {models}{declarations}\n"
    )
}

/// Compile through the dispatch link and the check, then emit.
fn compile(
    document: &[u8],
    declarations: &str,
    aliases: &[&str],
) -> Result<Emission, Vec<CheckRefusal>> {
    compile_in_mode(CheckMode::Linked, document, declarations, aliases)
}

/// [`compile`] under the requested check `mode`.
fn compile_in_mode(
    mode: CheckMode,
    document: &[u8],
    declarations: &str,
    aliases: &[&str],
) -> Result<Emission, Vec<CheckRefusal>> {
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let mut assembled = assemble_with_models(unit(declarations, aliases).as_bytes(), &packages)
        .expect("the unit assembles");
    let pending = assembled.link_dispatch(Default::default())?;
    let graph = assembled
        .check(
            mode,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map_err(|refusals| pending.report(refusals))?;
    Ok(emit_checked(&CheckedPackage::link(graph)).expect("the package emits"))
}

fn emitted(document: &[u8], declarations: &str) -> Emission {
    compile(document, declarations, &["Config"])
        .unwrap_or_else(|refusals| panic!("the unit compiles: {refusals:?}"))
}

type Key = (String, String);

/// Each row as `(declaring owner node, operation)` with its body.
fn rows(emission: &Emission) -> Vec<(Key, OperationBody)> {
    let wire = wire(emission);
    body_rows(emission)
        .expect("the package carries operation_bodies")
        .iter()
        .map(|row| {
            assert_eq!(&*row.document, PACKAGE);
            let declaring = nodes(&wire)
                .iter()
                .find(|node| node["node_id"] == json!(row.declaring))
                .expect("the declaring node is emitted");
            (
                (
                    declaring["owner"]["node"].as_str().unwrap().to_owned(),
                    row.operation.to_string(),
                ),
                row.body.clone(),
            )
        })
        .collect()
}

fn owner(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn body_of(rows: &[(Key, OperationBody)], declaring: &str, operation: &str) -> OperationBody {
    rows.iter()
        .find(|((owner_node, name), _)| owner_node == &owner(declaring) && name == operation)
        .unwrap_or_else(|| panic!("a row for {declaring}.{operation}: {rows:?}"))
        .1
        .clone()
}

fn function(body: OperationBody) -> CheckedNodeId {
    match body {
        OperationBody::Function { node } => node,
        OperationBody::NoBody => panic!("a body function node"),
    }
}

/// The function node `id` of the emitted package.
fn node_of<'w>(wire: &'w Value, id: &CheckedNodeId) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == json!(id))
        .expect("the body node is emitted")
}

const CALLED: &str = "pre c using v on Config::ConfigVersion::probe { self.size() > 0 }\n";

fn base_operations() -> Vec<Value> {
    vec![
        query("ConfigVersion", "size", None),
        query("ConfigVersion", "empty", None),
        query("ConfigVersion", "probe", None),
    ]
}

/// FR-576-AC-1, AC-9: `A.size` has a body that a dispatched call reaches. Its
/// row references the body function node, whose owner is the operation's own
/// and whose clause is `body`; the node yields the authored expression with
/// its source provenance.
#[trace("FR-576-AC-1", "FR-576-AC-9", "TC-1011")]
#[test]
fn a_called_body_is_referenced_by_its_row() {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        &format!("body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n{CALLED}"),
    );
    let rows = rows(&emission);
    let id = function(body_of(&rows, "ConfigVersion", "size"));
    let wire = wire(&emission);
    let node = node_of(&wire, &id);
    assert_eq!(node["owner"]["node"], owner("ConfigVersion/size"));
    assert!(
        serde_json::to_string(&node["body"]).unwrap().contains("41"),
        "the node yields the authored expression 41: {node}"
    );
    assert!(
        wire["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["node_id"] == json!(id)),
        "the body node has source provenance"
    );
}

/// FR-576-AC-2: `A.size` has a body no call reaches. The package still holds
/// its body function node and a row referencing it, and the node's id is the
/// one it has in the package where a dispatched call reaches it.
#[trace("FR-576-AC-2", "TC-1011")]
#[test]
fn an_uncalled_body_has_the_node_id_it_has_when_called() {
    let document = package(base_operations(), &[]);
    let body = "body sizeA using v on Config::ConfigVersion::size() { 41 }\n";
    let called = rows(&emitted(&document, &format!("{body}{CALLED}")));
    let uncalled_emission = emitted(&document, body);
    let uncalled = rows(&uncalled_emission);
    let id = function(body_of(&uncalled, "ConfigVersion", "size"));
    assert_eq!(
        id,
        function(body_of(&called, "ConfigVersion", "size")),
        "the node id does not depend on a call"
    );
    node_of(&wire(&uncalled_emission), &id);
}

/// FR-576-AC-3: an operation without an `operation-body` has exactly one row
/// whose `body` is `no_body`, and the package holds no function node owned by
/// the operation.
#[trace("FR-576-AC-3", "TC-1011")]
#[test]
fn an_operation_without_a_body_has_a_no_body_row_and_no_node() {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
    );
    let rows = rows(&emission);
    let empty: Vec<_> = rows
        .iter()
        .filter(|((_, name), _)| name == "empty")
        .collect();
    assert_eq!(empty.len(), 1);
    assert_eq!(empty[0].1, OperationBody::NoBody);
    let owned = nodes(&wire(&emission))
        .iter()
        .filter(|node| node["owner"]["node"] == owner("ConfigVersion/empty"))
        .count();
    assert_eq!(owned, 0, "no node, default value or empty body");
}

/// FR-576-AC-4: `A` and `B` each declare `size`, `A` with a body and `B`
/// without. The rows are separate, and permuting declaration order emits an
/// equal member.
#[trace("FR-576-AC-4", "TC-1011")]
#[test]
fn same_named_operations_of_two_owners_have_separate_rows() {
    let document = |reverse: bool| {
        let mut subtypes = vec![("B", "", vec![query("B", "size", None)])];
        let mut operations = base_operations();
        if reverse {
            subtypes.reverse();
            operations.reverse();
        }
        package(operations, &subtypes)
    };
    let declarations = |reverse: bool| {
        let mut lines = [
            "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
            CALLED,
        ];
        if reverse {
            lines.reverse();
        }
        lines.concat()
    };
    let forward = emitted(&document(false), &declarations(false));
    let rows_forward = rows(&forward);
    assert!(matches!(
        body_of(&rows_forward, "ConfigVersion", "size"),
        OperationBody::Function { .. }
    ));
    assert_eq!(body_of(&rows_forward, "B", "size"), OperationBody::NoBody);
    let permuted = emitted(&document(true), &declarations(true));
    assert_eq!(body_rows(&forward), body_rows(&permuted));
}

/// FR-576-AC-5: a subtype that only inherits `size` has no row; a subtype
/// that redefines it with its own body has its own row naming its own node,
/// distinct from the declaring type's.
#[trace("FR-576-AC-5", "TC-1011")]
#[test]
fn an_inherited_operation_has_no_row_and_a_redefinition_has_its_own() {
    let document = package(
        base_operations(),
        &[
            ("S", "ConfigVersion", vec![]),
            (
                "R",
                "ConfigVersion",
                vec![query("R", "size", Some("ConfigVersion"))],
            ),
        ],
    );
    let emission = emitted(
        &document,
        &format!(
            "body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n\
             body sizeR using v on Config::R::size() {{ 42 }}\n{CALLED}"
        ),
    );
    let rows = rows(&emission);
    assert!(
        rows.iter()
            .all(|((declaring, _), _)| declaring != &owner("S")),
        "S declares nothing: {rows:?}"
    );
    let a = function(body_of(&rows, "ConfigVersion", "size"));
    let r = function(body_of(&rows, "R", "size"));
    assert_ne!(a, r);
    assert_eq!(
        node_of(&wire(&emission), &r)["owner"]["node"],
        owner("R/size")
    );
}

/// FR-576-AC-6: a document selected through two model aliases emits one row
/// per operation, not one per alias.
#[trace("FR-576-AC-6", "TC-1011")]
#[test]
fn two_aliases_of_one_document_emit_one_row_per_operation() {
    let document = package(base_operations(), &[]);
    let single = emitted(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
    );
    let aliased = compile(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
        &["Config", "Other"],
    )
    .unwrap_or_else(|refusals| panic!("the unit compiles: {refusals:?}"));
    assert_eq!(body_rows(&aliased).map(|rows| rows.len()), Some(3));
    assert_eq!(body_rows(&single), body_rows(&aliased));
}

/// FR-576-AC-7: the row of an operation that is a candidate of a dispatch
/// table carries the candidate's operation-body identity, and the member's
/// keys equal the `operation_effects` keys of the same package.
#[trace("FR-576-AC-7", "TC-1011")]
#[test]
fn a_candidates_row_carries_the_tables_operation_body_identity() {
    let document = package(
        base_operations(),
        &[(
            "R",
            "ConfigVersion",
            vec![query("R", "size", Some("ConfigVersion"))],
        )],
    );
    let emission = emitted(
        &document,
        &format!(
            "body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n\
             body sizeR using v on Config::R::size() {{ 42 }}\n{CALLED}"
        ),
    );
    let wire = wire(&emission);
    let rows = rows(&emission);
    let candidates: Vec<&Value> = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("a dispatch table")
        .iter()
        .flat_map(|table| table["candidates"].as_array().unwrap())
        .collect();
    assert!(!candidates.is_empty());
    for body in ["ConfigVersion", "R"] {
        let id = json!(function(body_of(&rows, body, "size")));
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate["operation_body"] == id),
            "{body}.size is a candidate with its row's body identity"
        );
    }
    let keys: Vec<Value> = body_rows(&emission)
        .unwrap()
        .iter()
        .map(|row| json!([row.document, row.declaring, row.operation]))
        .collect();
    let effect_keys: Vec<Value> = wire["operation_effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| json!([row["document"], row["declaring"], row["operation"]]))
        .collect();
    assert_eq!(keys, effect_keys, "the same keys in the same order");
}

/// FR-576: the timed-model writer re-encodes the admitted base package and
/// carries its `operation_bodies` member unchanged, not drops it.
#[trace("FR-576-AC-1", "TC-1011")]
#[test]
fn the_timed_model_writer_carries_the_operation_bodies() {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        &format!("body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n{CALLED}"),
    );
    let before = body_rows(&emission).expect("the base carries operation_bodies");
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document), &*document)
        .expect("the selected document is retained as evidence");
    let admitted = match filament_contract_model::read_checked_package(
        emission.package.bytes(),
        filament_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("IR admits the emitted package: {other:?}"),
    };
    let timed = crate::emit::emit_timed_models(&admitted, &[], crate::EmitLimits::fixture())
        .expect("a package without timed nodes re-emits");
    assert_eq!(body_rows(&timed), Some(before));
}

/// `query` without a declared result.
fn resultless(owner: &str, name: &str) -> Value {
    let mut operation = query(owner, name, None);
    operation.as_object_mut().unwrap().remove("returns");
    operation
}

/// FR-576-AC-10: an operation with a body and no declared result refuses
/// `ill_typed`/`type-mismatch` located at the body; without the body it emits
/// a `no_body` row; with a declared result and a matching body it emits a row
/// referencing the body function.
#[trace("FR-576-AC-10", "TC-1011")]
#[test]
fn a_body_on_an_operation_without_a_result_refuses_type_mismatch() {
    let mut operations = base_operations();
    operations.push(resultless("ConfigVersion", "touch"));
    let document = package(operations, &[]);
    let refusals = compile(
        &document,
        "body noResult using v on Config::ConfigVersion::touch() { true }\n",
        &["Config"],
    )
    .expect_err("a body with no declared result to check against refuses");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
    assert!(matches!(
        &refusal.location.origin,
        filament_semantic_value::location::ExpressionOwner::Body { function, .. }
            if function.ends_with("touch")
    ));

    let without_body = rows(&emitted(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
    ));
    assert_eq!(
        body_of(&without_body, "ConfigVersion", "touch"),
        OperationBody::NoBody
    );

    let mut operations = base_operations();
    operations.push(query("ConfigVersion", "touch", None));
    let document = package(operations, &[]);
    let with_body = rows(&emitted(
        &document,
        "body touchOk using v on Config::ConfigVersion::touch() { 1 }\n",
    ));
    function(body_of(&with_body, "ConfigVersion", "touch"));
}

/// FR-952-AC-1: under `Kernel` an operation's `body` clause function records
/// `kernel`, the mode its body was checked under, on the wire.
#[trace("TC-2662", "FR-952-AC-1")]
#[test]
fn under_kernel_an_operation_body_clause_records_kernel() {
    let document = package(base_operations(), &[]);
    let declarations = "body sizeA using v on Config::ConfigVersion::size() { 41 }\n";
    let emission = compile_in_mode(CheckMode::Kernel, &document, declarations, &["Config"])
        .unwrap_or_else(|refusals| panic!("the unit checks under Kernel: {refusals:?}"));
    let id = function(body_of(&rows(&emission), "ConfigVersion", "size"));
    assert_eq!(node_of(&wire(&emission), &id)["check_mode"], "kernel");
}

/// FR-952-AC-1, FR-094: under `Kernel` the declared `precondition` clause
/// function of an operation is checked and recorded `linked`, while the
/// operation's `body` clause function is `kernel`.
#[trace("TC-2662", "FR-952-AC-1")]
#[test]
fn under_kernel_a_precondition_clause_function_is_linked_and_a_body_is_kernel() {
    let document = package(base_operations(), &[]);
    let declarations = format!(
        "body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n\
         pre ps using v on Config::ConfigVersion::size {{ true }}\n{CALLED}"
    );
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let mut assembled =
        assemble_with_models(unit(&declarations, &["Config"]).as_bytes(), &packages)
            .expect("the unit assembles");
    let pending = assembled.link_dispatch(Default::default()).expect("links");
    let graph = assembled
        .check(
            CheckMode::Kernel,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let modes: Vec<_> = (0..)
        .map_while(|index| graph.function_check_mode(index))
        .collect();
    assert!(modes.contains(&CheckMode::Kernel), "{modes:?}");
    assert!(modes.contains(&CheckMode::Linked), "{modes:?}");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let modes: Vec<_> = nodes(&wire(&emission))
        .iter()
        .filter(|node| node["node_tag"] == "function")
        .map(|node| node["check_mode"].clone())
        .collect();
    assert!(modes.contains(&json!("kernel")), "{modes:?}");
    assert!(modes.contains(&json!("linked")), "{modes:?}");
}

/// Rewards over the config-version package: `cost` and `load` on `size`
/// (which has a body) and `stable` on `empty` (which has none).
const REWARDS: [&str; 3] = [
    "reward cost on ConfigVersion::size = 1;",
    "reward load on ConfigVersion::size = 2;",
    "reward stable on ConfigVersion::empty = 3;",
];
const SIZE_BODY: &str = "body sizeA using v on Config::ConfigVersion::size() { 41 }\n";

fn rewarded(declarations: &[&str]) -> (Vec<u8>, Emission) {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        &format!("{SIZE_BODY}{}\n", declarations.join("\n")),
    );
    (document, emission)
}

/// FSL's strict read of `wire` re-encoded, with `document` as the domain
/// package evidence.
pub(super) fn read_wire_over(wire: &Value, document: Option<&[u8]>) -> Read {
    let mut edited = wire.clone();
    fsl_semantics::check::wire_id::write_operand_rows(&mut edited);
    let bytes = quire_canonical::to_vec(&edited, ceiling().canonical()).expect("canonical");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in edited["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    if let Some(document) = document {
        evidence
            .insert_domain_package_document(package_identity(document), document)
            .expect("a fresh evidence key");
    }
    read_v2(&bytes, library(), V2ReadLimits::fixture(), &evidence)
}

/// The position in `wire`'s graph of the node with `id`.
fn position_of(wire: &Value, id: &Value) -> usize {
    nodes(wire)
        .iter()
        .position(|node| &node["node_id"] == id)
        .expect("the node is in the graph")
}

/// The function node of the reward `name` in `wire`, its position and the
/// position of its `reward` node.
pub(super) fn reward_function_of(wire: &Value, name: &str) -> (usize, usize) {
    let reward = nodes(wire)
        .iter()
        .position(|node| {
            node["semantic_form"] == "reward"
                && node["body"]["members"].as_array().is_some_and(|members| {
                    members
                        .iter()
                        .any(|member| member["name"] == "name" && member["value"]["value"] == name)
                })
        })
        .unwrap_or_else(|| panic!("the reward `{name}`"));
    let expression = nodes(wire)[reward]["body"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["name"] == "expression")
        .expect("the expression binding")["value"]["target"]
        .clone();
    (position_of(wire, &expression), reward)
}

pub(super) fn set_clause(wire: &mut Value, function: usize, clause: &str) {
    let members = wire["semantic_graph"]["nodes"][function]["body"]["members"]
        .as_array_mut()
        .expect("an aggregate body");
    members
        .iter_mut()
        .find(|member| member["name"] == "clause")
        .expect("the clause binding")["value"]["value"] = json!(clause);
}

pub(super) fn refusal_of(read: Read) -> filament_contract_model::CheckedPackageRefusal {
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) = read else {
        panic!("the edited package is refused with an envelope");
    };
    *refusal
}

/// FSpec FR-322-AC-471, FR-640-AC-7: a reward's function carries the clause
/// kind `reward` under the `ModelOwner` of its operation member and the owner
/// join admits it; the operation without a body keeps its `no_body` row and no
/// function owned by it has another clause kind than `reward`.
#[trace("FSpec-TC-1563", "FSpec-FR-322-AC-471")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-7")]
#[test]
fn a_reward_on_a_bodyless_operation_leaves_its_no_body_row_and_reads_as_no_precondition() {
    let (document, emission) = rewarded(&REWARDS);
    let wire = wire(&emission);
    let mut evidence = read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    assert!(matches!(
        read_with(&emission, &evidence),
        Read::Verified { .. }
    ));
    let rows = rows(&emission);
    assert_eq!(
        body_of(&rows, "ConfigVersion", "empty"),
        OperationBody::NoBody
    );
    let (function, _) = reward_function_of(&wire, "stable");
    let node = &nodes(&wire)[function];
    assert_eq!(node["owner"]["node"], owner("ConfigVersion/empty"));
    for node in nodes(&wire).iter().filter(|node| {
        node["node_tag"] == "function" && node["owner"]["node"] == owner("ConfigVersion/empty")
    }) {
        let clause = node["body"]["members"]
            .as_array()
            .and_then(|members| members.iter().find(|member| member["name"] == "clause"))
            .map(|member| member["value"]["value"].clone());
        assert_eq!(clause, Some(json!("reward")), "{node}");
    }
}

/// `refusal` as `(code, cause, pointer)`.
pub(super) fn summary(
    refusal: &filament_contract_model::CheckedPackageRefusal,
) -> (
    filament_contract_model::CheckedPackageRefusalCode,
    Option<filament_contract_model::CheckedPackageRefusalCause>,
    Option<String>,
) {
    (
        refusal.code,
        refusal.cause,
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|path| path.as_str().to_owned()),
    )
}

fn set_owner(wire: &mut Value, function: usize, node: &str) {
    wire["semantic_graph"]["nodes"][function]["owner"]["node"] = json!(node);
}

/// FSpec FR-322-AC-471, FR-640-AC-7: the mutations of a reward's function the
/// strict reader refuses, each at its stated locus.
#[trace("FSpec-TC-1563", "FSpec-FR-322-AC-471")]
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-7")]
#[test]
fn a_reward_function_with_another_clause_kind_or_owner_is_refused() {
    use filament_contract_model::{
        CheckedPackageRefusalCause as Cause, CheckedPackageRefusalCode as Code,
    };
    let (document, emission) = rewarded(&REWARDS);
    let wire = wire(&emission);
    let (stable, stable_reward) = reward_function_of(&wire, "stable");
    let (cost, cost_reward) = reward_function_of(&wire, "cost");
    let expression =
        |reward: usize| format!("/semantic_graph/nodes/{reward}/body/members/4/value/target");
    let refused = |edited: &Value| summary(&refusal_of(read_wire_over(edited, Some(&document))));

    // `precondition`: the owner join admits it; reader rule 4 refuses it.
    let mut edited = wire.clone();
    set_clause(&mut edited, cost, "precondition");
    assert_eq!(
        refused(&edited),
        (
            Code::InvalidPackage,
            Some(Cause::InvalidValue),
            Some(expression(cost_reward))
        )
    );

    // `body` on the bodyless operation: its `no_body` row refuses.
    let row = wire["operation_bodies"]
        .as_array()
        .expect("the rows")
        .iter()
        .position(|row| row["operation"] == "empty")
        .expect("the row of `empty`");
    let mut edited = wire.clone();
    set_clause(&mut edited, stable, "body");
    assert_eq!(
        refused(&edited),
        (
            Code::InvalidPackage,
            Some(Cause::InvalidValue),
            Some(format!("/operation_bodies/{row}/body"))
        )
    );

    // Owned by another operation member: rule 4 refuses it.
    let mut edited = wire.clone();
    set_owner(&mut edited, stable, &owner("ConfigVersion/size"));
    assert_eq!(
        refused(&edited),
        (
            Code::InvalidPackage,
            Some(Cause::InvalidValue),
            Some(expression(stable_reward))
        )
    );

    // Owned by the object type, and by a field member of it: the owner join
    // finds no operation member.
    for (name, owner_node) in [
        ("type", owner("ConfigVersion")),
        ("field", owner("ConfigVersion/versionNumber")),
    ] {
        let mut edited = wire.clone();
        set_owner(&mut edited, stable, &owner_node);
        assert_eq!(
            refused(&edited),
            (
                Code::MissingDeclaration,
                Some(Cause::MissingSelection),
                Some(format!("/semantic_graph/nodes/{stable}/node_id")),
            ),
            "owned by the {name}"
        );
    }
}

/// The `(reward name, ordinal)` of every reward function of `wire`, and the
/// ordinal of the function of the body of `size`.
fn function_ordinals(wire: &Value) -> (Vec<(String, u64)>, u64) {
    let ordinal = |position: usize| {
        nodes(wire)[position]["node_id"]["ordinal"]
            .as_u64()
            .unwrap()
    };
    let rewards = ["cost", "load", "stable"]
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                ordinal(reward_function_of(wire, name).0),
            )
        })
        .collect();
    let body = nodes(wire)
        .iter()
        .position(|node| {
            node["node_tag"] == "function"
                && node["owner"]["node"] == owner("ConfigVersion/size")
                && node["body"]["members"].as_array().is_some_and(|members| {
                    members.iter().any(|member| {
                        member["name"] == "clause" && member["value"]["value"] == "body"
                    })
                })
        })
        .expect("the body function of `size`");
    (rewards, ordinal(body))
}

/// FSpec FR-640-AC-3 "Reward identity": the reward functions of a model take
/// ordinals after every other clause function of their owner, ranked by
/// (operation key, reward name), and the package is byte-identical whatever
/// order the rewards are written in.
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn reward_functions_follow_every_other_clause_function_in_operation_then_name_order() {
    let [cost, load, stable] = REWARDS;
    let orders = [
        [cost, load, stable],
        [stable, load, cost],
        [load, stable, cost],
    ];
    // The graph, with each id resolved past positional indexes and no
    // source occurrence (those follow the written text).
    let graphs: Vec<Vec<Value>> = orders
        .iter()
        .map(|order| {
            let wire = wire(&rewarded(order).1);
            nodes(&wire)
                .iter()
                .map(|node| {
                    let mut node = node.clone();
                    node["occurrences"] = Value::Null;
                    node["resolved_id"] =
                        super::probabilistic::resolved_id(&wire, &node["node_id"], 8);
                    node
                })
                .collect()
        })
        .collect();
    assert_eq!(graphs[0], graphs[1], "written order changes no node");
    assert_eq!(graphs[0], graphs[2], "written order changes no node");

    let (_, emission) = rewarded(&REWARDS);
    let (rewards, body) = function_ordinals(&wire(&emission));
    let ordinal = |name: &str| rewards.iter().find(|(n, _)| n == name).unwrap().1;
    assert!(
        rewards.iter().all(|(_, ordinal)| *ordinal > body),
        "after the body function ({body}): {rewards:?}"
    );
    assert!(
        ordinal("stable") < ordinal("cost") && ordinal("cost") < ordinal("load"),
        "`empty` before `size`, then by reward name: {rewards:?}"
    );
}

/// FSpec FR-640-AC-3: rewriting a reward's expression changes its `reward`
/// node and leaves its function's id alone (the function key holds no
/// content).
#[trace("FSpec-TC-1563", "FSpec-FR-640-AC-3")]
#[test]
fn a_changed_reward_expression_changes_the_reward_and_keeps_the_function_id() {
    let ids = |expression: &str| {
        let declarations = format!("reward cost on ConfigVersion::size = {expression};");
        let (_, emission) = rewarded(&[declarations.as_str()]);
        let wire = wire(&emission);
        let (function, reward) = reward_function_of(&wire, "cost");
        (
            super::probabilistic::resolved_id(&wire, &nodes(&wire)[function]["node_id"], 8),
            super::probabilistic::resolved_id(&wire, &nodes(&wire)[reward]["node_id"], 8),
        )
    };
    let (function_one, reward_one) = ids("1");
    let (function_two, reward_two) = ids("2");
    assert_eq!(function_one, function_two);
    assert_ne!(reward_one, reward_two);
}
