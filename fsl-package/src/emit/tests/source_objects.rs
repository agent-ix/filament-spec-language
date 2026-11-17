// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569-AC-11: a unit that declares a source object type
//! is refused at emission, once, at its first `object`, whichever other
//! declarations it holds.

use super::*;
use filament_contract_model::{
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use filament_semantic_value::checking::CheckMode;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"filament.model.complete/v1\";\n";

/// TC-1408 fixture S.
const S: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
invariant Sized using P on Order at current { self.size() >= 0 }
";

/// S, S-R and S-P emit the source-owned object declarations.
#[trace("FR-569-AC-6", "TC-1408")]
#[test]
fn fixtures_s_s_r_and_s_p_emit_source_objects() {
    let variants = [
        ("S", String::new()),
        (
            "S-R",
            "pre Qs2 using P on Sub::size { self.total >= 1 }\n".to_owned(),
        ),
        (
            "S-P",
            "invariant V4 using P on Order at current { forall(o in orders : deref(o).total >= 0) }\n"
                .to_owned(),
        ),
    ];
    for (name, extra) in variants {
        let text = format!("{HEADER}{S}{extra}");
        let packages =
            fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
        let mut declarations =
            assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
        let pending = declarations
            .link_dispatch(Default::default())
            .unwrap_or_else(|refusals| panic!("{name}: {refusals:?}"));
        let graph = declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{name}: {:?}", pending.report(refusals)));
        let package = CheckedPackage::link(graph);
        let emission = emit_checked(&package).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let wire = wire(&emission);
        source_object_node(&wire, "Order");
        source_object_node(&wire, "Sub");
        source_object_node(&wire, "Shape");
    }
}

mod orders_model {
    include!("../../../../fsl-semantics/tests/fixtures/orders_model.rs");
}

/// TC-1408 variant S-M: S with `model M = "test/orders";` selecting fixture
/// D's package and the protocol `Sell`, whose roles and `over` name the model
/// and the source `Order`. The package emits both ownership routes.
#[trace("FR-569-AC-6", "TC-1408")]
#[test]
fn fixture_s_m_emits_the_selected_and_source_object_routes() {
    let text = format!(
        "{HEADER}profile Pp = \"filament.protocol.finite-global/v1\";\n\
         model M = \"test/orders\";\n{S}\
         protocol Sell using Pp over (view: M::Order) on origin {{\n\
           role w on M;\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:s-m"),
        "program.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = fsl_semantics::model::intake::SelectedModels::fixture(
        "M",
        fsl_foundation::Span { start: 0, end: 1 },
        orders_model::model_package(false),
        fsl_semantics::model::accounting::ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "pkg".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("S-M assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("the source-owned package emits");
    let wire = wire(&emission);
    source_object_node(&wire, "Order");
    source_object_node(&wire, "Sub");
}

use super::protocol_controls::tc1070 as tc1070_source;

/// TC-1407 fixture E-S and its domain package's input.
fn fixture_e_s() -> (String, fsl_semantics::model::intake::PackageInput) {
    let (text, document) = tc1070_source::source_node();
    let bytes = crate::emit::tests::wire_bytes(&document).expect("the document encodes");
    let packages = fsl_semantics::model::intake::package_input([bytes.as_slice()])
        .expect("E-S's domain package admits");
    (text, packages)
}

/// TC-1407: E-S checks and emits the source-declared `Node` object.
#[trace("FR-569-AC-6", "FR-383-AC-67", "TC-1407")]
#[test]
fn fixture_e_s_checks_and_emits_object_node() {
    let (text, packages) = fixture_e_s();
    let mut declarations = assemble_with_models(text.as_bytes(), &packages).expect("E-S assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("E-S checks: {:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("the source-owned package emits");
    source_object_node(&wire(&emission), "Node");
}

// FR-569-AC-14 (TC-1941): the target emission of a source object type, read
// back through FSL's I2 reader, and the owner forms that read refuses.

/// Fixture S checked and linked, ready for S4.
fn fixture_s_package() -> CheckedPackage {
    let text = format!("{HEADER}{S}");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let mut declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    CheckedPackage::link(graph)
}

/// The index of the source-declared `model`/`object_type` node named `name`.
fn source_object_node(wire: &Value, name: &str) -> usize {
    nodes(wire)
        .iter()
        .position(|node| {
            node["node_tag"] == "model"
                && node["semantic_form"] == "object_type"
                && node["declaration"]["qualified_name"] == json!([name])
        })
        .unwrap_or_else(|| panic!("`{name}` is a source-declared object type node"))
}

/// FSL's I2 read of an edited `wire`, with its lock's features as evidence.
fn read_edited(wire: &Value) -> Read {
    let mut slotted = wire.clone();
    fsl_semantics::check::wire_id::write_operand_rows(&mut slotted);
    let bytes = quire_canonical::to_vec(&slotted, ceiling().canonical()).expect("canonical");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    read_v2(&bytes, library(), V2ReadLimits::fixture(), &evidence)
}

#[trace(
    "FR-569-AC-14",
    "FR-569-AC-6",
    "FSpec-FR-610-AC-3",
    "TC-1941",
    "FSpec-TC-1441"
)]
#[test]
#[ignore = "QSL-2347: source dispatch, operation-body, and timed-place reader joins are pending IR"]
fn fixture_s_emits_and_the_i2_reader_reads_it() {
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let order = &nodes(&wire)[source_object_node(&wire, "Order")];
    assert_eq!(order["owner"]["kind"], "source");
    assert_eq!(order["semantic_type"], order["node_id"]);
    source_object_node(&wire, "Sub");
    source_object_node(&wire, "Shape");
    let read = read_back(&emission);
    let Read::Verified { .. } = read else {
        panic!("the I2 reader admits fixture S's package: {read:?}");
    };
}

#[trace("FR-569-AC-6", "FR-390-AC-70", "TC-1408", "TC-1407")]
#[test]
fn fixture_s_emits_source_object_and_operation_rows_and_bodies() {
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let wire = wire(&emission);

    let object = |identity: &str| {
        emission
            .selected_source_object_rows()
            .iter()
            .find(|row| row.source_identity.as_ref() == identity)
            .unwrap_or_else(|| panic!("source object row for {identity}"))
    };
    let order = object("Order");
    assert_eq!(
        order
            .identity_fields
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>(),
        ["total"]
    );
    assert_eq!(
        order
            .members
            .iter()
            .map(|member| member.name.as_ref())
            .collect::<Vec<_>>(),
        ["total", "note"]
    );
    let sub = object("Sub");
    assert_eq!(sub.identity_fields, order.identity_fields);
    assert_eq!(sub.members, order.members);
    assert!(object("Shape").is_abstract);
    assert!(object("Shape").members.is_empty());

    let operation = emission
        .selected_source_operation_rows()
        .iter()
        .find(|row| row.source_identity.as_ref() == "Order::bump")
        .expect("bump source-operation row");
    assert_eq!(operation.anchor_id, operation.structural_node);
    let bump_effect = wire["operation_effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["provenance"]["member"] == "Order::bump")
        .expect("bump source effect row");
    assert_eq!(bump_effect["document"], "test:spine-model");
    assert_eq!(
        bump_effect["provenance"]["origin"],
        "source_declaration_operation_frame"
    );

    let size_body = wire["operation_bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["document"] == "test:spine-model" && row["operation"] == "size")
        .expect("source size operation-body row");
    assert_eq!(size_body["body"]["kind"], "function");
    let source_body = nodes(&wire)
        .iter()
        .find(|node| node["node_id"] == size_body["body"]["node"])
        .expect("size body function node");
    assert_eq!(source_body["owner"]["kind"], "source");
    assert_eq!(
        source_body["declaration"]["qualified_name"],
        json!(["Order", "size", "body"])
    );
}

#[trace("FR-569-AC-6", "FR-569-AC-8", "TC-1408")]
#[test]
fn source_object_identity_rows_roundtrip_through_qc30_intake() {
    let source =
        format!("{HEADER}record Point {{ x: Integer; }}\nobject Order {{ point: Point; }}\n");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations = assemble_with_models(source.as_bytes(), &packages).unwrap();
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("source object checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the source emits");
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(package) = read else {
        panic!("QC-30 intake admits source object package: {read:?}");
    };
    let rows = emission
        .identity_rows(
            &package,
            &mut filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("source identity rows join");
    let bytes = rows.to_bytes(&package).expect("identity rows encode");
    let constructed = crate::ReadIdentityRows::of_rows_with_source_rows(
        &package,
        rows.declarations().to_vec(),
        rows.fields().to_vec(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
        rows.source_objects().to_vec(),
        rows.source_operations().to_vec(),
        emission.source_object_member_facts.clone(),
    );
    assert_eq!(
        constructed
            .to_bytes(&package)
            .expect("public constructor encodes"),
        bytes
    );
    // Assert the normative wire shape directly: round-tripping through the
    // local row decoder would allow a reduced encoder and decoder to agree
    // on the same non-normative schema.
    let identity_json: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    assert_eq!(
        identity_json["format"],
        "filament.checked-handoff.identities/4"
    );
    let object_json = &identity_json["source_objects"][0];
    assert!(object_json.get("abstract").is_some());
    assert!(object_json.get("is_abstract").is_none());
    let members = object_json["members"].as_array().expect("member array");
    assert_eq!(members.len(), 1);
    let member = &members[0];
    let member_keys = member
        .as_object()
        .expect("member object")
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        member_keys,
        [
            "admitted_name",
            "original_member_identity",
            "presence",
            "type_node",
            "value_type",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(member["admitted_name"], "point");
    assert!(member["original_member_identity"]
        .as_str()
        .is_some_and(|identity| !identity.is_empty()));
    assert_eq!(member["presence"], "required");
    assert!(member["type_node"].is_object());
    assert!(member["value_type"].is_object());

    let read = crate::ReadIdentityRows::read(
        &bytes,
        &package,
        &mut crate::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("QC-30 reads source identity rows");
    assert_eq!(read, rows);
    assert_eq!(read.to_bytes(&package).unwrap(), bytes);
    assert_eq!(read.source_objects().len(), 1);
    assert_eq!(read.source_objects()[0].source_identity.as_ref(), "Order");
}

#[trace("FR-088-AC-13", "FR-569-AC-6", "TC-2360")]
#[test]
fn source_spelling_registry_changes_do_not_change_emitted_package_bytes() {
    let emitted = |spelling: &str| {
        let text = format!(
            "{HEADER}model {spelling} = \"example/config-version\";\nfunction f(x: {spelling}::ConfigVersion): Boolean pure {{ true }}\n"
        );
        let model = super::admission_corpus::config_version_unit::DOMAIN_PACKAGE;
        let packages = fsl_semantics::model::intake::package_input([model.as_bytes()]).unwrap();
        let declarations = assemble_with_models(text.as_bytes(), &packages)
            .expect("the spelling variant assembles");
        let graph = declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the spelling variant checks: {refusals:?}"));
        emit_checked(&CheckedPackage::link(graph))
            .expect("the spelling variant emits")
            .package()
            .bytes()
            .to_vec()
    };

    let model_alias_bytes = emitted("M");
    let alternate_alias_bytes = emitted("N");
    assert_eq!(model_alias_bytes, alternate_alias_bytes);
}

#[trace("FR-569-AC-7", "TC-1408")]
#[test]
fn source_object_node_identity_tracks_only_the_object_declaration_identity() {
    let node_id = |body: &str, name: &str| {
        let text = format!("{HEADER}{body}");
        let packages =
            fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
        let declarations = assemble_with_models(text.as_bytes(), &packages)
            .unwrap_or_else(|error| panic!("source assembles: {error:?}"));
        let graph = declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("source checks: {refusals:?}"));
        let emission = emit_checked(&CheckedPackage::link(graph)).expect("source emits");
        let wire = wire(&emission);
        nodes(&wire)[source_object_node(&wire, name)]["node_id"].clone()
    };

    let base = "object Order { total: Integer; operation size(): Integer; }\n";
    let with_field = "object Order { total: Integer; note: Boolean; operation size(): Integer; }\n";
    let renamed_operation = "object Order { total: Integer; operation count(): Integer; }\n";
    let moved = "object Other {}\nobject Order { total: Integer; operation size(): Integer; }\n";
    let renamed_object = "object Purchase { total: Integer; operation size(): Integer; }\n";
    let base_id = node_id(base, "Order");

    assert_eq!(node_id(with_field, "Order"), base_id);
    assert_eq!(node_id(renamed_operation, "Order"), base_id);
    assert_eq!(node_id(moved, "Order"), base_id);
    assert_ne!(node_id(renamed_object, "Purchase"), base_id);
}

#[trace("FR-569-AC-7", "TC-1408")]
#[test]
fn source_object_node_identity_differs_from_selected_model_node_identity() {
    let emit_with_model = |text: String| {
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "test:s-m-identity"),
            "program.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        assert_eq!(parsed.diagnostics(), []);
        let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let selected = fsl_semantics::model::intake::SelectedModels::fixture(
            "M",
            fsl_foundation::Span { start: 0, end: 1 },
            orders_model::model_package(false),
            fsl_semantics::model::accounting::ModelNormalizationLimits::UNLIMITED,
        )
        .expect("selected model admits");
        let mut declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "pkg".to_owned(),
            unit,
            selected,
            Vec::new(),
        )
        .expect("S-M assembles");
        let pending = declarations
            .link_dispatch(Default::default())
            .unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let graph = declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
        wire(&emit_checked(&CheckedPackage::link(graph)).expect("S-M emits"))
    };

    let source_wire = emit_with_model(format!("{HEADER}model M = \"test/orders\";\n{S}"));
    let source_node = &nodes(&source_wire)[source_object_node(&source_wire, "Order")];
    let selected_wire = emit_with_model(format!(
        "{HEADER}model M = \"test/orders\";\nfunction f(x: M::Order): Boolean pure {{ true }}\n"
    ));
    let selected_model_node = nodes(&selected_wire)
        .into_iter()
        .find(|node| {
            node["node_tag"] == "model"
                && node["semantic_form"] == "object_type"
                && node["owner"]["kind"] == "model"
                && node["owner"]["node"] == "ix://test/orders/Order"
        })
        .expect("selected model's Order node");
    assert_ne!(source_node["node_id"], selected_model_node["node_id"]);
}

#[trace("FR-569-AC-14", "FSpec-FR-610-AC-4", "TC-1941", "FSpec-TC-1441")]
#[test]
#[ignore = "QSL-2347: source-owner reader validation is pending the IR reader update"]
fn a_source_object_node_with_a_model_owner_is_malformed_wire() {
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let mut edited = wire(&emission);
    let at = source_object_node(&edited, "Order");
    edited["semantic_graph"]["nodes"][at]["owner"] = json!({
        "kind": "model",
        "identity": "test/orders",
        "node": "ix://test/orders/Order",
    });
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) =
        read_edited(&edited)
    else {
        panic!("the I2 reader refuses a source object with a model owner");
    };
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::MalformedWire
    );
    assert_eq!(
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|p| p.to_string()),
        Some(format!("/semantic_graph/nodes/{at}/owner"))
    );
}

#[trace("FR-569-AC-14", "FSpec-FR-610-AC-4", "TC-1941", "FSpec-TC-1441")]
#[test]
#[ignore = "QSL-2347: source-owner reader validation is pending the IR reader update"]
fn an_object_type_node_with_a_source_owner_and_no_declaration_is_malformed_wire() {
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let mut edited = wire(&emission);
    let at = source_object_node(&edited, "Order");
    edited["semantic_graph"]["nodes"][at]
        .as_object_mut()
        .expect("a node is an object")
        .remove("declaration");
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) =
        read_edited(&edited)
    else {
        panic!("the I2 reader refuses the node");
    };
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::MalformedWire
    );
    assert_eq!(
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|p| p.to_string()),
        Some(format!("/semantic_graph/nodes/{at}/owner"))
    );
}

/// Fixture S with a protocol whose `over`, role and attempt name the source
/// `Order`.
fn fixture_s_protocol_package() -> CheckedPackage {
    let text = format!(
        "{HEADER}profile Pp = \"filament.protocol.finite-global/v1\";\n{S}\
         protocol Sell using Pp over (view: Order) on origin {{\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    );
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let mut declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    CheckedPackage::link(graph)
}

#[trace("FSpec-FR-610-AC-7", "FSpec-TC-1443", "FR-569-AC-14", "TC-1941")]
#[test]
#[ignore = "QSL-2347: source-owner protocol reader joins are pending the IR reader update"]
fn a_protocol_over_a_source_object_names_its_node_and_joins_its_source_owner() {
    let package = fixture_s_protocol_package();
    let emission = emit_checked(&package).expect("S4 emits the protocol over Order");
    let wire = wire(&emission);
    let order = source_object_node(&wire, "Order");
    let clause = nodes(&wire)
        .iter()
        .find(|node| node.get("owner_model").is_some())
        .expect("a protocol clause");
    assert_eq!(clause["owner_model"], nodes(&wire)[order]["node_id"]);
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the I2 reader admits the protocol over a source object");
    };

    let mut edited = wire.clone();
    edited["semantic_graph"]["nodes"][order]["owner"]["authority"] = json!("elsewhere");
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) =
        read_edited(&edited)
    else {
        panic!("an owner joining no lock source is refused");
    };
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::MissingDeclaration
    );
}

#[trace("FSpec-FR-610-AC-8", "FSpec-TC-1443", "FR-569-AC-6", "TC-1408")]
#[test]
fn source_operation_effects_and_clause_functions_carry_the_source_forms() {
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let order = source_object_node(&wire, "Order");
    let row = wire["operation_effects"]
        .as_array()
        .expect("the package carries operation effects")
        .iter()
        .find(|row| row["provenance"]["member"] == "Order::bump")
        .expect("bump has an effects row");
    assert_eq!(
        row["provenance"]["origin"],
        "source_declaration_operation_frame"
    );
    assert_eq!(row["declaring"], nodes(&wire)[order]["node_id"]);
    assert_eq!(row["document"], "test:spine-model");
    let precondition = nodes(&wire)
        .iter()
        .find(|node| {
            node["declaration"]["qualified_name"] == json!(["Order", "size", "precondition", "Qs"])
        })
        .expect("Qs lowers to a source clause function");
    assert_eq!(precondition["owner"]["kind"], "source");
}
