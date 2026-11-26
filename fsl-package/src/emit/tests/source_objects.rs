// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-owned object declarations and their FR-995/996 identity keys.

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
  operation measure(n: Integer): Integer;
  operation tally(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation measure(m: Integer): Integer redefines Order::measure;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::measure(m) { m }
body C using P on Order::measure(n) { n }
pre Qs using P on Order::measure { self.total >= 0 }
invariant Sized using P on Order at current { self.measure(0) >= 0 }
";

/// S, S-R and S-P emit the source-owned object declarations.
#[trace("FR-569-AC-6", "TC-1408")]
#[test]
fn fixtures_s_s_r_and_s_p_emit_source_objects() {
    let variants = [
        ("S", String::new()),
        (
            "S-R",
            "pre Qs2 using P on Sub::measure { self.total >= 1 }\n".to_owned(),
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
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
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

use super::tc235_k2 as correspondence_fixture;

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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("the source-owned package emits");
    let wire = wire(&emission);
    source_object_node(&wire, "Order");
    source_object_node(&wire, "Sub");
    // One correspondence node, for the selected model: the source model is no
    // model selection and contributes none.
    let correspondence: Vec<&Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "model_correspondence")
        .collect();
    let [selected] = correspondence.as_slice() else {
        panic!("one model correspondence node: {correspondence:?}");
    };
    let declarations: Vec<&str> = selected["body"]["members"]
        .as_array()
        .expect("the correspondence body is an aggregate")
        .iter()
        .filter_map(|member| member["value"].as_str())
        .filter(|value| value.starts_with("ix://"))
        .collect();
    assert!(declarations.contains(&"ix://test/orders/Order"));
    assert!(
        declarations
            .iter()
            .all(|declaration| declaration.starts_with("ix://test/orders/")),
        "only the selected model's declarations: {declarations:?}"
    );
}

fn correspondence_fixture(diamond: bool) -> (Emission, Vec<u8>) {
    correspondence_unit(
        diamond,
        "model M = \"test/orders\";\nfunction keep(): Boolean pure { true }\n",
    )
}

/// The unit of `declarations` emitted against the TC-195 domain package, and
/// that package's document.
fn correspondence_unit(diamond: bool, declarations: &str) -> (Emission, Vec<u8>) {
    use correspondence_fixture::{construct, declaration, domain_document, field, id};

    let mut types = vec![
        declaration(
            "A",
            "thing",
            json!({
                "identityFields": [id("A/id")],
                "supertypes": [],
                "fields": [
                    field("A", "id", "ix://filament/native/UUID"),
                    field("A", "x", "ix://filament/native/Integer"),
                ],
                "operations": [],
            }),
        ),
        declaration(
            "B",
            "thing",
            json!({
                "identityFields": [id("B/bid")],
                "supertypes": [id("A")],
                "fields": [field("B", "bid", "ix://filament/native/UUID")],
                "operations": [],
            }),
        ),
    ];
    if diamond {
        types.extend([
            declaration(
                "C",
                "thing",
                json!({
                    "identityFields": [id("C/cid")],
                    "supertypes": [id("A")],
                    "fields": [field("C", "cid", "ix://filament/native/UUID")],
                    "operations": [],
                }),
            ),
            declaration(
                "D",
                "thing",
                json!({
                    "identityFields": [id("D/did")],
                    "supertypes": [id("B"), id("C")],
                    "fields": [field("D", "did", "ix://filament/native/UUID")],
                    "operations": [],
                }),
            ),
        ]);
    }
    let document = crate::emit::tests::wire_bytes(&domain_document(
        types,
        vec![construct(
            "thing",
            fsl_semantics::model::intake::meaning::OBJECT_TYPE,
        )],
    ))
    .expect("the TC-195 domain package encodes");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the TC-195 model package admits");
    let text = format!("{HEADER}{declarations}");
    let mut declarations = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles against the TC-195 package");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("the package emits");
    (emission, document)
}

pub(super) fn read_correspondence(
    emission: &Emission,
    document: &[u8],
) -> Vec<fsl_semantics::model::key::EffectiveDeclarationPreimage> {
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("the selected TC-195 model document is reader evidence");
    let read = crate::checked_v2::read_checked_package_v2(
        emission.package().bytes(),
        library(),
        crate::checked_v2::V2ReadLimits::fixture(),
        &evidence,
    )
    .expect("IR and the FSL typed model-correspondence projection admit");
    read.into_value().package.model_correspondence().to_vec()
}

fn correspondence_test_evidence(emission: &Emission, document: &[u8]) -> CheckedPackageEvidence {
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("the selected TC-195 model document is reader evidence");
    evidence
}

fn read_mutated_correspondence(wire: &Value, document: &[u8], emission: &Emission) -> Read {
    let mut slotted = wire.clone();
    fsl_semantics::check::wire_id::write_operand_rows(&mut slotted);
    let bytes = quire_canonical::to_vec(&slotted, ceiling().canonical()).expect("canonical");
    read_v2(
        &bytes,
        library(),
        V2ReadLimits::fixture(),
        &correspondence_test_evidence(emission, document),
    )
}

fn correspondence_members_mut(wire: &mut Value) -> &mut Vec<Value> {
    let node = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| {
            node["node_tag"] == "correspondence" && node["semantic_form"] == "model_correspondence"
        })
        .expect("the package emits a correspondence node");
    node["body"]["members"].as_array_mut().unwrap()
}

fn literal_text(members: &[Value], at: usize) -> &str {
    members[at]["value"].as_str().expect("text literal")
}

fn literal_usize(members: &[Value], at: usize) -> usize {
    members[at]["value"]
        .as_str()
        .expect("integer literal uses its decimal spelling")
        .parse()
        .expect("integer literal is a non-negative count")
}

fn skip_effective_id(members: &[Value], at: &mut usize) {
    match literal_text(members, *at) {
        "none" => *at += 1,
        "some" => {
            let mut depth = 0;
            while literal_text(members, *at) == "some" {
                depth += 1;
                *at += 1;
            }
            assert_eq!(literal_text(members, *at), "none");
            *at += 1 + depth * 2;
        }
        marker => panic!("closed effective-id marker: {marker}"),
    }
}

fn skip_declaration(at: &mut usize) {
    *at += 2;
}

fn find_first_derivation_ordinal(members: &[Value]) -> usize {
    let mut at = 1;
    skip_effective_id(members, &mut at);
    skip_declaration(&mut at);
    assert!(literal_usize(members, at) > 0);
    at + 1
}

fn first_inherit_parent_key(members: &[Value]) -> (usize, usize) {
    let row_count = literal_usize(members, 0);
    let mut at = 1;
    for _ in 0..row_count {
        skip_effective_id(members, &mut at);
        skip_declaration(&mut at);
        let fact_count = literal_usize(members, at);
        at += 1;
        for _ in 0..fact_count {
            at += 1;
            let rule = literal_text(members, at);
            at += 1;
            let input_count = literal_usize(members, at);
            at += 1 + input_count * 2;
            let parent_start = at;
            let has_parent = literal_text(members, at) == "some";
            skip_effective_id(members, &mut at);
            if rule == "filament.model.normalize.inherit/v2" && has_parent {
                let depth = members[parent_start..at]
                    .iter()
                    .filter(|term| term["value"] == "some")
                    .count();
                let package = parent_start + depth * 3 - 1;
                return (package, package + 1);
            }
        }
    }
    panic!("N01 has an inherited parent link")
}

fn effective_id_json(identity: &filament_exact::EffectiveId) -> Value {
    let owner = identity
        .owner_effective_type()
        .map(|owner| effective_id_json(&owner))
        .unwrap_or(Value::Null);
    json!({
        "owner_effective_type": owner,
        "original": {
            "package": identity.original().package(),
            "node": identity.original().node(),
        },
    })
}

fn preimages_for_original<'a>(
    rows: &'a [fsl_semantics::model::key::EffectiveDeclarationPreimage],
    original: &str,
) -> Vec<&'a fsl_semantics::model::key::EffectiveDeclarationPreimage> {
    rows.iter()
        .filter(|row| row.original.package == "test/orders" && row.original.node == original)
        .collect()
}

#[trace("FR-400-AC-14", "FSpec-TC-195")]
#[test]
fn tc195_n01_checked_package_reader_exposes_structural_preimages() {
    let (emission, document) = correspondence_fixture(false);
    let rows = read_correspondence(&emission, &document);
    let a_rows = preimages_for_original(&rows, "ix://test/orders/A");
    let [a] = a_rows.as_slice() else {
        panic!("N01 exposes type A once")
    };
    assert_eq!(
        effective_id_json(&a.identity()),
        json!({
            "owner_effective_type": null,
            "original": {"package": "test/orders", "node": "ix://test/orders/A"},
        })
    );
    let b_rows = preimages_for_original(&rows, "ix://test/orders/B");
    let [b] = b_rows.as_slice() else {
        panic!("N01 exposes type B once")
    };
    assert_eq!(
        effective_id_json(&b.identity()),
        json!({
            "owner_effective_type": null,
            "original": {"package": "test/orders", "node": "ix://test/orders/B"},
        })
    );
    let b_ax_rows = preimages_for_original(&rows, "ix://test/orders/A/x")
        .into_iter()
        .filter(|row| {
            row.owner_effective_type
                .as_ref()
                .is_some_and(|owner| owner.original().node() == "ix://test/orders/B")
        })
        .collect::<Vec<_>>();
    assert_eq!(b_ax_rows.len(), 1, "N01 exposes member (B, A/x) once");
    let [b_ax] = b_ax_rows.as_slice() else {
        panic!("N01 exposes member (B, A/x) once")
    };
    assert_eq!(
        effective_id_json(&b_ax.identity()),
        json!({
            "owner_effective_type": {
                "owner_effective_type": null,
                "original": {"package": "test/orders", "node": "ix://test/orders/B"},
            },
            "original": {"package": "test/orders", "node": "ix://test/orders/A/x"},
        })
    );
    assert_eq!(
        effective_id_json(&b_ax.identity())
            .as_object()
            .expect("identity is an object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["original".to_owned(), "owner_effective_type".to_owned()]),
        "FR-150 identity is the structural pair and contains no digest"
    );
}

#[trace("FR-400-AC-14", "FSpec-TC-195")]
#[test]
fn tc195_n02_checked_package_reader_retains_one_diamond_member_with_direct_links() {
    let (emission, document) = correspondence_fixture(true);
    let rows = read_correspondence(&emission, &document);
    let d_rows = preimages_for_original(&rows, "ix://test/orders/D");
    let [d] = d_rows.as_slice() else {
        panic!("N02 exposes type D once")
    };
    assert_eq!(
        effective_id_json(&d.identity()),
        json!({
            "owner_effective_type": null,
            "original": {"package": "test/orders", "node": "ix://test/orders/D"},
        })
    );
    let d_ax_rows = preimages_for_original(&rows, "ix://test/orders/A/x")
        .into_iter()
        .filter(|row| {
            row.owner_effective_type
                .as_ref()
                .is_some_and(|owner| owner.original().node() == "ix://test/orders/D")
        })
        .collect::<Vec<_>>();
    let [d_ax] = d_ax_rows.as_slice() else {
        panic!("N02 exposes member (D, A/x) exactly once")
    };
    assert_eq!(
        effective_id_json(&d_ax.identity()),
        json!({
            "owner_effective_type": {
                "owner_effective_type": null,
                "original": {"package": "test/orders", "node": "ix://test/orders/D"},
            },
            "original": {"package": "test/orders", "node": "ix://test/orders/A/x"},
        })
    );
    assert_eq!(d_ax.derivation.len(), 2);
    assert_eq!(
        d_ax.derivation[0].rule.identity,
        "filament.model.normalize.inherit/v2"
    );
    assert_eq!(
        d_ax.derivation[1].rule.identity,
        "filament.model.normalize.inherit/v2"
    );
    assert_eq!(
        d_ax.derivation
            .iter()
            .map(|fact| {
                effective_id_json(fact.parent.as_ref().expect("inherit fact has parent"))
            })
            .collect::<Vec<_>>(),
        [
            json!({
                "owner_effective_type": {
                    "owner_effective_type": null,
                    "original": {"package": "test/orders", "node": "ix://test/orders/B"},
                },
                "original": {"package": "test/orders", "node": "ix://test/orders/A/x"},
            }),
            json!({
                "owner_effective_type": {
                    "owner_effective_type": null,
                    "original": {"package": "test/orders", "node": "ix://test/orders/C"},
                },
                "original": {"package": "test/orders", "node": "ix://test/orders/A/x"},
            }),
        ]
    );
    assert_eq!(
        d_ax.derivation
            .iter()
            .map(|fact| fact
                .inputs
                .iter()
                .map(|key| key.node.as_str())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        [
            vec!["ix://test/orders/B", "ix://test/orders/A/x"],
            vec!["ix://test/orders/C", "ix://test/orders/A/x"],
        ]
    );
}

/// Each row's `(owner original node, original node)`, sorted, repeats kept.
/// Every original is the TC-195 package's.
fn identity_nodes(
    rows: &[fsl_semantics::model::key::EffectiveDeclarationPreimage],
) -> Vec<(Option<String>, String)> {
    let mut nodes: Vec<_> = rows
        .iter()
        .map(|row| {
            assert_eq!(row.original.package, "test/orders");
            (
                row.owner_effective_type
                    .as_ref()
                    .map(|owner| owner.original().node().to_owned()),
                row.original.node.clone(),
            )
        })
        .collect();
    nodes.sort();
    nodes
}

/// The effective declarations of the TC-195 N01 chain: types `A` and `B`,
/// `A`'s two fields, and `B`'s two inherited fields and own field.
fn n01_identity_nodes() -> Vec<(Option<String>, String)> {
    [
        (None, "A"),
        (None, "B"),
        (Some("A"), "A/id"),
        (Some("A"), "A/x"),
        (Some("B"), "A/id"),
        (Some("B"), "A/x"),
        (Some("B"), "B/bid"),
    ]
    .into_iter()
    .map(|(owner, node)| {
        (
            owner.map(|owner| format!("ix://test/orders/{owner}")),
            format!("ix://test/orders/{node}"),
        )
    })
    .collect()
}

#[trace("FR-400-AC-14", "FSpec-TC-195")]
#[test]
fn a_source_object_package_reads_back_only_the_selected_models_rows() {
    const WIDGET: &str = "object Widget { ready: Boolean; population widgets; }\n";
    let (mixed, document) =
        correspondence_unit(false, &format!("model M = \"test/orders\";\n{WIDGET}"));
    source_object_node(&wire(&mixed), "Widget");
    assert_eq!(
        identity_nodes(&read_correspondence(&mixed, &document)),
        n01_identity_nodes()
    );

    let (source_only, _) = correspondence_unit(false, WIDGET);
    source_object_node(&wire(&source_only), "Widget");
    let read = crate::checked_v2::read_checked_package_v2(
        source_only.package().bytes(),
        library(),
        crate::checked_v2::V2ReadLimits::fixture(),
        source_only.evidence(),
    )
    .expect("the source-only package reads");
    assert_eq!(read.into_value().package.model_correspondence(), []);
}

#[trace("FR-400-AC-14", "FSpec-TC-195")]
#[test]
fn two_aliases_of_one_package_read_back_each_row_once() {
    let (emission, document) = correspondence_unit(
        false,
        "model M = \"test/orders\";\nmodel N = \"test/orders\";\n\
         function keep(): Boolean pure { true }\n",
    );
    assert_eq!(
        identity_nodes(&read_correspondence(&emission, &document)),
        n01_identity_nodes()
    );
}

/// A view under an alias with no `model_root` is not the source model's, so
/// emission refuses it rather than leaving its declarations out.
#[test]
fn a_selected_view_without_a_model_root_refuses() {
    let emit = |rootless: bool| {
        let packages =
            fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
        let text = format!("{HEADER}function keep(): Boolean pure {{ true }}\n");
        let mut declarations =
            assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
        let pending = declarations
            .link_dispatch(Default::default())
            .unwrap_or_else(|refusals| panic!("{refusals:?}"));
        if rootless {
            let fsl_semantics::model::normalize::NormalizeOutcome::Completed(view) =
                fsl_semantics::model::normalize::normalize(
                    &orders_model::model_package(false),
                    fsl_semantics::model::accounting::ModelNormalizationLimits::UNLIMITED,
                )
            else {
                panic!("fixture D normalizes");
            };
            declarations.selected_views.insert("Ghost".to_owned(), view);
        }
        let graph = declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
        emit_checked(&CheckedPackage::link(graph)).map(|_| ())
    };
    assert!(matches!(emit(false), Ok(())));
    assert!(matches!(emit(true), Err(EmitRefusal::SourceMapInvariant)));
}

#[trace("FR-400-AC-14", "FSpec-TC-195")]
#[test]
fn tc195_reader_refuses_a_malformed_derivation_ordinal() {
    let (emission, document) = correspondence_fixture(false);
    let mut edited = wire(&emission);
    let members = correspondence_members_mut(&mut edited);
    let ordinal = find_first_derivation_ordinal(members);
    members[ordinal]["value"] = json!("7");

    assert_eq!(
        read_mutated_correspondence(&edited, &document, &emission),
        Read::Refused(crate::checked_v2::V2ReadRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "I2",
                "admitted-model-correspondence-has-closed-preimage-shape",
            ),
        )),
        "a derivation ordinal that disagrees with its position is refused",
    );
}

#[trace("FR-400-AC-14", "FR-081-AC-9", "FSpec-TC-195")]
#[test]
fn tc195_reader_refuses_an_inherit_parent_with_the_wrong_original() {
    let (emission, document) = correspondence_fixture(false);
    let mut edited = wire(&emission);
    let members = correspondence_members_mut(&mut edited);
    let (package, node) = first_inherit_parent_key(members);
    members[package]["value"] = json!("test/orders");
    members[node]["value"] = json!("ix://test/orders/not-the-parent");

    assert_eq!(
        read_mutated_correspondence(&edited, &document, &emission),
        Read::Refused(crate::checked_v2::V2ReadRefusal::Fault(
            filament_semantic_value::catalog::InternalFault::new(
                "I2",
                "admitted-model-correspondence-has-closed-preimage-shape",
            ),
        )),
        "an inherit parent that is not the selected model's declared parent is refused",
    );
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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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

    let measure_body = wire["operation_bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["document"] == "test:spine-model" && row["operation"] == "measure")
        .expect("source measure operation-body row");
    assert_eq!(measure_body["body"]["kind"], "function");
    let source_body = nodes(&wire)
        .iter()
        .find(|node| node["node_id"] == measure_body["body"]["node"])
        .expect("measure body function node");
    assert_eq!(source_body["owner"]["kind"], "source");
    assert_eq!(
        source_body["declaration"]["qualified_name"],
        json!(["Order", "measure", "body"])
    );
}

#[trace("FR-569-AC-6", "FR-569-AC-8", "FR-569-AC-16", "TC-1408")]
#[test]
fn source_object_identity_rows_roundtrip_through_qc30_intake() {
    let source =
        format!("{HEADER}record Point {{ x: Integer; }}\nobject Order {{ point: Point; }}\n");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations = assemble_with_models(source.as_bytes(), &packages).unwrap();
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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
        emission.source_object_operations.clone(),
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
    assert_eq!(identity_json["format"], crate::IDENTITIES_FORMAT);
    let objects_json = identity_json["source_objects"].as_array().expect("rows");
    assert_eq!(objects_json.len(), 1);
    let by_identity = |name: &str| {
        objects_json
            .iter()
            .find(|row| row["source_identity"] == name)
            .unwrap_or_else(|| panic!("row for {name}"))
    };
    assert_eq!(by_identity("Order")["operations"], json!([]));
    let object_json = by_identity("Order");
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
            "collections",
            "member_type",
            "original_member_identity",
            "presence",
            "type_node",
            "value_type",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(member["admitted_name"], "point");
    assert!(
        member["original_member_identity"]
            .as_str()
            .is_some_and(|identity| !identity.is_empty())
    );
    assert_eq!(member["presence"], "required");
    assert!(member["type_node"].is_object());
    assert!(member["value_type"].is_object());
    // FR-531: a data entry holds no clock rendering and no collection layer.
    assert_eq!(member["member_type"], Value::Null);
    assert_eq!(member["collections"], json!([]));

    let read = crate::ReadIdentityRows::read(
        &bytes,
        &package,
        &mut crate::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("QC-30 reads source identity rows");
    assert_eq!(read, rows);
    assert_eq!(read.to_bytes(&package).unwrap(), bytes);
    assert_eq!(read.source_objects().len(), 1);
    assert!(
        read.source_objects()
            .iter()
            .any(|row| row.source_identity.as_ref() == "Order")
    );
    assert_eq!(
        read.source_object_operations(),
        emission.source_object_operations()
    );

    // FR-569-AC-16: the reader requires `operations` exactly once, holding
    // `{identity, query}` entries and nothing else.
    let text = String::from_utf8(bytes.clone()).expect("identity file is UTF-8");
    let mut absent: Value = serde_json::from_str(&text).unwrap();
    absent["source_objects"][0]
        .as_object_mut()
        .unwrap()
        .remove("operations");
    let mut extra: Value = serde_json::from_str(&text).unwrap();
    extra["source_objects"][0]["operations"] =
        json!([{"identity": "Order::size", "query": true, "extra": 1}]);
    let mut misspelt: Value = serde_json::from_str(&text).unwrap();
    misspelt["source_objects"][0]["operations"] = json!([{"identity": "Order::size"}]);
    let repeated = text.replacen(
        "\"operations\": []",
        "\"operations\": [], \"operations\": []",
        1,
    );
    assert_ne!(repeated, text, "the fixture row writes `operations`");
    for forged in [
        serde_json::to_vec(&absent).unwrap(),
        serde_json::to_vec(&extra).unwrap(),
        serde_json::to_vec(&misspelt).unwrap(),
        repeated.into_bytes(),
    ] {
        let refusal = crate::ReadIdentityRows::read(
            &forged,
            &package,
            &mut crate::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect_err("a malformed `operations` member is refused");
        assert!(
            matches!(
                refusal,
                crate::IdentityFileRefusal::Malformed {
                    member: crate::IdentityMember::SourceObjects
                }
            ),
            "{refusal:?}"
        );
    }
}

/// FR-569-AC-16 through the QC-30 intake: the reader admits source
/// `operation_bodies` rows, and each own operation joins its identity row as
/// `{identity, query, clock_parameters, parameters}`.
#[trace("FR-569-AC-16", "TC-1408")]
#[test]
fn source_object_operations_roundtrip_through_qc30_intake() {
    let source = format!(
        "{HEADER}record Point {{ x: Integer; }}\nobject Order {{ point: Point; operation size(): Integer; operation bump(): Boolean modifies self.point; }}\nobject Sub extends Order {{ operation size(): Integer redefines Order::size; }}\n"
    );
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations = assemble_with_models(source.as_bytes(), &packages).unwrap();
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
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
    let identity_json: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    let objects = identity_json["source_objects"].as_array().expect("rows");
    let row = |name: &str| {
        objects
            .iter()
            .find(|row| row["source_identity"] == name)
            .unwrap_or_else(|| panic!("row for {name}"))
    };
    assert_eq!(
        row("Order")["operations"],
        json!([
            {"identity": "Order::size", "query": true, "clock_parameters": [], "parameters": []},
            {"identity": "Order::bump", "query": false, "clock_parameters": [], "parameters": []},
        ])
    );
    assert_eq!(
        row("Sub")["operations"],
        json!([{"identity": "Sub::size", "query": true, "clock_parameters": [], "parameters": []}])
    );
    let read = crate::ReadIdentityRows::read(
        &bytes,
        &package,
        &mut crate::V2ReadLimits::fixture().native_join_budget(),
    )
    .expect("QC-30 reads source identity rows");
    assert_eq!(read, rows);
    assert_eq!(
        read.source_object_operations(),
        emission.source_object_operations()
    );
}

/// FR-569-AC-16 over fixture S's objects: a row carries one `{identity, query}` entry
/// per operation its type declares itself, none for an inherited one, and
/// each entry has exactly one `operation_effects` row naming it, declared on
/// the row's own node.
#[trace("FR-569-AC-16", "TC-1408")]
#[test]
fn source_object_rows_carry_their_own_operations_and_query_flag() {
    // The object block of TC-1408 fixture S, whose operation names the AC states.
    let source = format!(
        "{HEADER}object Order identity (total) {{\n  total: Int[0, 9];\n  note: Text[0, 8; nfc]?;\n  population orders;\n  operation size(): Integer;\n  operation count(): Integer;\n  operation scaled(n: Integer): Integer;\n  operation bump(): Boolean modifies self.total;\n}}\nobject Sub extends Order {{\n  operation size(): Integer redefines Order::size;\n  operation scaled(m: Integer): Integer redefines Order::scaled;\n}}\nabstract object Shape {{ population shapes; }}\n"
    );
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations = assemble_with_models(source.as_bytes(), &packages).unwrap();
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("source object checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let by_name = emission
        .selected_source_object_rows()
        .iter()
        .zip(emission.source_object_operations())
        .map(|(row, operations)| {
            let entries = operations
                .iter()
                .map(|operation| (operation.identity.to_string(), operation.query))
                .collect::<Vec<_>>();
            (row.source_identity.to_string(), (row, entries))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let entries = |name: &str| -> Vec<(&str, bool)> {
        by_name[name]
            .1
            .iter()
            .map(|(identity, query)| (identity.as_str(), *query))
            .collect()
    };
    assert_eq!(
        entries("Order"),
        [
            ("Order::size", true),
            ("Order::count", true),
            ("Order::scaled", true),
            ("Order::bump", false),
        ]
    );
    assert_eq!(
        entries("Sub"),
        [("Sub::size", true), ("Sub::scaled", true)],
        "only the redefinitions the subtype declares itself"
    );
    assert!(entries("Shape").is_empty());
    let effects = wire["operation_effects"].as_array().expect("effect rows");
    for (row, own) in by_name.values() {
        let node = serde_json::to_value(&row.structural_node).expect("node id serializes");
        for (identity, _) in own {
            let matching = effects
                .iter()
                .filter(|effect| {
                    effect["provenance"]["member"] == identity.as_str()
                        && effect["declaring"] == node
                })
                .count();
            assert_eq!(matching, 1, "{identity}");
        }
    }
    assert_eq!(
        effects.len(),
        by_name.values().map(|(_, own)| own.len()).sum::<usize>(),
        "no operation_effects row lacks an entry"
    );
}

/// FR-569-AC-16: fixture D, which selects the model-owned `Order` and declares
/// no source object, emits no source-object row and so no operation entry.
#[trace("FR-569-AC-16", "TC-1408")]
#[test]
fn fixture_d_emission_has_no_source_object_operations() {
    let text = format!(
        "{HEADER}model M = \"test/orders\";\nfunction f using P (x: Boolean): Boolean pure {{ x }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:d"),
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "pkg".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("D assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("D checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("D emits");
    assert!(emission.selected_source_object_rows().is_empty());
    assert!(emission.source_object_operations().is_empty());
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
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
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
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("source checks: {refusals:?}"));
        let emission = emit_checked(&CheckedPackage::link(graph)).expect("source emits");
        let wire = wire(&emission);
        nodes(&wire)[source_object_node(&wire, name)]["node_id"].clone()
    };

    let base = "object Order { total: Integer; operation measure(): Integer; }\n";
    let with_field =
        "object Order { total: Integer; note: Boolean; operation measure(): Integer; }\n";
    let renamed_operation = "object Order { total: Integer; operation tally(): Integer; }\n";
    let moved = "object Other {}\nobject Order { total: Integer; operation measure(): Integer; }\n";
    let renamed_object = "object Purchase { total: Integer; operation measure(): Integer; }\n";
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
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
        wire(&emit_checked(&CheckedPackage::link(graph)).expect("S-M emits"))
    };

    let source_wire = emit_with_model(format!("{HEADER}model M = \"test/orders\";\n{S}"));
    let source_node = &nodes(&source_wire)[source_object_node(&source_wire, "Order")];
    let selected_wire = emit_with_model(format!(
        "{HEADER}model M = \"test/orders\";\nfunction f(x: M::Order): Boolean pure {{ true }}\n"
    ));
    let selected_model_node = nodes(&selected_wire)
        .iter()
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
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    CheckedPackage::link(graph)
}

#[trace("FSpec-FR-610-AC-7", "FSpec-TC-1443", "FR-569-AC-14", "TC-1941")]
#[test]
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
            node["declaration"]["qualified_name"]
                == json!(["Order", "measure", "precondition", "Qs"])
        })
        .expect("Qs lowers to a source clause function");
    assert_eq!(precondition["owner"]["kind"], "source");
}

/// The code, cause and first locus of FSL's strict I2 refusal of `wire`.
fn refused(wire: &Value) -> (Value, Option<Value>, Option<String>) {
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope { refusal, .. }) =
        read_edited(wire)
    else {
        panic!("the I2 reader refuses the edited package");
    };
    (
        serde_json::to_value(refusal.code).expect("a code"),
        refusal
            .cause
            .map(|cause| serde_json::to_value(cause).expect("a cause")),
        refusal
            .loci
            .first()
            .and_then(|locus| locus.path.as_ref())
            .map(|path| path.to_string()),
    )
}

/// The `SourceOwner` `identity` of the source-declared node `name`.
fn owner_identity(wire: &Value, name: &str) -> String {
    let owner = &nodes(wire)[source_object_node(wire, name)]["owner"];
    assert_eq!(owner["kind"], "source");
    owner["identity"]
        .as_str()
        .expect("a source owner identity")
        .to_owned()
}

#[trace("FR-995-AC-1", "TC-2816")]
#[test]
fn source_owner_key_uses_its_identity_qualified_ix_spelling() {
    let package = fixture_s_package();
    let graph = package.graph();
    let source_keys = super::super::SourceDeclarationKeys::from_graph(graph)
        .expect("actual checked source-owner declarations");
    let node = graph
        .semantic_graph()
        .nodes()
        .find(|node| {
            node.declaration()
                .is_some_and(|name| name.len() == 1 && name[0].as_str() == "Order")
        })
        .expect("the checked original Order declaration");
    let original = graph
        .resolve_declaration(node.key())
        .expect("Order retains its original declaration key");
    let identity = filament_exact::EffectiveId::new(
        None,
        filament_exact::OriginalDeclarationKey::new(&original.package, &original.node),
    );

    let emitted = super::super::checked_effective_id(&identity, &source_keys)
        .expect("a source-owned declaration key is emitted");

    assert_eq!(
        serde_json::to_value(emitted).expect("the typed identity serializes"),
        json!({
            "owner_effective_type": null,
            "original": {
                "package": "test:spine-model",
                "node": "ix://test:spine-model/Order"
            }
        })
    );
}

#[trace("FR-996-AC-1", "TC-2818")]
#[test]
fn source_clock_place_keys_use_the_source_owner_identity() {
    let source = "language \"ix:native\" edition \"1-draft\";\n\
        profile P = \"filament.temporal.complete/v1\";\n\
        dimension Time; unit ms : Time = rational(1, 1); time Order dense unit ms;\n\
        record W { c: Clock; }\n\
        object Order { w: W; }\n";
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("source-only package input");
    let graph = assemble_with_models(source.as_bytes(), &packages)
        .expect("actual source declarations")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("source clock place checks: {refusals:?}"));
    let source_keys = super::super::SourceDeclarationKeys::from_graph(&graph)
        .expect("checked source declaration context");
    let row = graph
        .timed_models()
        .first()
        .expect("timed Order")
        .clocks()
        .first()
        .expect("W.c clock")
        .resolved_path()
        .last()
        .expect("W.c row");
    assert_eq!(
        source_keys
            .member(&row.declaring_type, &row.member_identity)
            .expect("actual W.c member in its declaring row"),
        "ix://test:spine-model/W/c",
    );
    assert!(
        source_keys
            .member("ix://test:spine-model/Order", &row.member_identity)
            .is_err(),
        "W/c must not resolve through an unrelated declaring type"
    );
}

/// The candidates of the one dispatch table whose context is `Order`'s node.
fn order_candidates(wire: &Value) -> Vec<Value> {
    let order = nodes(wire)[source_object_node(wire, "Order")]["node_id"].clone();
    let tables = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("the package carries dispatch tables");
    let [table] = &tables
        .iter()
        .filter(|table| table["context"] == order)
        .collect::<Vec<_>>()[..]
    else {
        panic!("one dispatch table over `Order`: {tables:?}");
    };
    table["candidates"].as_array().expect("candidates").clone()
}

#[trace(
    "FR-995-AC-1",
    "FR-995-AC-2",
    "TC-2816",
    "FSpec-FR-788-AC-1",
    "FSpec-TC-1915"
)]
#[test]
fn source_selection_keys_are_spelled_from_the_source_owner() {
    let emission = emit_checked(&fixture_s_package()).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let identity = owner_identity(&wire, "Order");
    let order = &nodes(&wire)[source_object_node(&wire, "Order")];
    assert_eq!(order["node_tag"], "model");
    assert_eq!(order["semantic_form"], "object_type");
    assert_eq!(order["declaration"]["qualified_name"], json!(["Order"]));
    assert_eq!(
        order["owner"],
        json!({
            "kind": "source",
            "authority": "agent-ix",
            "identity": identity,
        })
    );
    assert_eq!(order["semantic_type"], order["node_id"]);
    let keys: Vec<Value> = order_candidates(&wire)
        .iter()
        .map(|candidate| candidate["selection_key"].clone())
        .collect();
    assert_eq!(
        keys,
        ["Order", "Sub"]
            .map(|name| json!({"owner_effective_type": null, "original": {
                "package": identity, "node": format!("ix://{identity}/{name}")}}))
            .to_vec()
    );
    let document = wire["operation_effects"]
        .as_array()
        .expect("operation effects")
        .iter()
        .find(|row| row["provenance"]["member"] == "Order::bump")
        .expect("bump's effects row")["document"]
        .clone();
    assert_eq!(document, json!(identity));
    let text = wire.to_string();
    assert!(
        !text.contains("ix://source/"),
        "no key is spelled from a check-time alias"
    );
}

#[trace("FR-995-AC-3", "TC-2817", "FSpec-FR-788-AC-2", "FSpec-TC-1916")]
#[test]
fn a_source_selection_key_reads_back_and_a_foreign_spelling_refuses() {
    let emission = emit_checked(&fixture_s_package()).expect("S4 emits fixture S");
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the I2 reader admits fixture S's dispatch table");
    };
    let wire = wire(&emission);
    let tables = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("dispatch tables");
    let order = nodes(&wire)[source_object_node(&wire, "Order")]["node_id"].clone();
    let table = tables
        .iter()
        .position(|table| table["context"] == order)
        .expect("the `Order` table");
    for (member, value) in [("node", json!("Order")), ("package", json!("other/unit"))] {
        let mut edited = wire.clone();
        edited["checked_term_data"]["dispatch_tables"][table]["candidates"][0]["selection_key"]["original"]
            [member] = value;
        assert_eq!(
            refused(&edited),
            (
                json!("invalid_package"),
                Some(json!("invalid-value")),
                Some(format!(
                    "/checked_term_data/dispatch_tables/{table}/candidates/0/selection_key"
                ))
            ),
            "{member}"
        );
    }
}

/// FR-230-AC-65's timed source model over a source object `O`.
const TIMED: &str = "\
profile T = \"filament.temporal.complete/v1\";\n\
dimension Time;
unit ms : Time = rational(1, 1);
record W { c: Clock; }
object O { w: W; d: Clock; operation step() modifies self.d; }
time O dense unit ms;
";

fn timed_source_package() -> CheckedPackage {
    let text = format!("{HEADER}{TIMED}");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    CheckedPackage::link(graph)
}

/// The `(declaring_type, member_identity, member_type)` text of each
/// `resolved_path` row of the clock place at graph index `place`.
fn place_rows(wire: &Value, place: usize) -> Vec<[String; 3]> {
    let rows = super::clock_places::member(&nodes(wire)[place]["body"], "resolved_path");
    rows["members"]
        .as_array()
        .unwrap_or_else(|| panic!("resolved_path rows: {rows}"))
        .iter()
        .map(|row| {
            let text = |name: &str| {
                row["members"]
                    .as_array()
                    .expect("row bindings")
                    .iter()
                    .find(|binding| binding["name"] == name)
                    .and_then(|binding| binding["value"]["value"].as_str())
                    .unwrap_or_default()
                    .to_owned()
            };
            [
                text("declaring_type"),
                text("member_identity"),
                text("member_type"),
            ]
        })
        .collect()
}

/// The graph indexes of the `clock_place` nodes.
fn places(wire: &Value) -> Vec<usize> {
    nodes(wire)
        .iter()
        .enumerate()
        .filter(|(_, node)| node["semantic_form"] == "clock_place")
        .map(|(index, _)| index)
        .collect()
}

#[trace("FR-996-AC-1", "TC-2818", "FSpec-FR-789-AC-1", "FSpec-TC-1917")]
#[test]
fn a_timed_source_model_emits_its_clock_places_with_source_rows() {
    let package = timed_source_package();
    let emission = emit_checked(&package).expect("S4 emits the timed source model");
    let wire = wire(&emission);
    let o = nodes(&wire)[source_object_node(&wire, "O")]["node_id"].clone();
    let root = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "timed_model")
        .expect("a timed root");
    assert_eq!(
        super::clock_places::member(&root["body"], "model")["target"],
        o,
        "{root}"
    );
    let i = owner_identity(&wire, "O");
    let clock = "ix://filament/native/Clock".to_owned();
    let mut rows: Vec<Vec<[String; 3]>> = places(&wire)
        .into_iter()
        .map(|place| place_rows(&wire, place))
        .collect();
    rows.sort();
    let mut expected = vec![
        vec![[
            format!("ix://{i}/O"),
            format!("ix://{i}/O/d"),
            clock.clone(),
        ]],
        vec![
            [
                format!("ix://{i}/O"),
                format!("ix://{i}/O/w"),
                format!("ix://{i}/W"),
            ],
            [format!("ix://{i}/W"), format!("ix://{i}/W/c"), clock],
        ],
    ];
    expected.sort();
    assert_eq!(rows, expected);
}

#[trace("FR-996-AC-2", "TC-2819", "FSpec-FR-789-AC-2", "FSpec-TC-1918")]
#[test]
fn a_forged_source_clock_place_row_is_a_malformed_clock_place() {
    let emission = emit_checked(&timed_source_package()).expect("S4 emits the timed source model");
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the I2 reader admits the timed source model");
    };
    let wire = wire(&emission);
    let i = owner_identity(&wire, "O");
    for place in places(&wire) {
        let rows = place_rows(&wire, place);
        let last = rows.len() - 1;
        let forged = [
            (0, "member_identity", format!("ix://{i}/O/e")),
            (0, "declaring_type", "ix://other/unit/O".to_owned()),
            (0, "member_type", "Integer".to_owned()),
            (last, "declaring_type", "ix://acme/orders/Order".to_owned()),
        ];
        for (row, member, value) in forged {
            let mut edited = wire.clone();
            let bindings = edited["semantic_graph"]["nodes"][place]["body"]["members"][2]["value"]
                ["members"][row]["members"]
                .as_array_mut()
                .expect("row bindings");
            let binding = bindings
                .iter_mut()
                .find(|binding| binding["name"] == member)
                .expect("the row binding");
            binding["value"]["value"] = json!(value);
            assert_eq!(
                refused(&edited),
                (
                    json!("invalid_package"),
                    Some(json!("malformed-clock-place")),
                    Some(format!(
                        "/semantic_graph/nodes/{place}/body/members/2/value/members/{row}"
                    ))
                ),
                "{member} of row {row} of place {place}"
            );
        }
    }
}

/// `(declaring source type name, operation, body kind, body declaration)` of
/// each source `operation_bodies` row.
fn source_body_rows(wire: &Value) -> Vec<(String, String, String, Value)> {
    let declarations: std::collections::BTreeMap<String, Value> = nodes(wire)
        .iter()
        .map(|node| {
            (
                node["node_id"].to_string(),
                node["declaration"]["qualified_name"].clone(),
            )
        })
        .collect();
    wire["operation_bodies"]
        .as_array()
        .expect("the package carries operation bodies")
        .iter()
        .map(|row| {
            let declaring = declarations[&row["declaring"].to_string()][0]
                .as_str()
                .unwrap_or_default()
                .to_owned();
            let body = row["body"]["node"].to_string();
            (
                declaring,
                row["operation"].as_str().expect("an operation").to_owned(),
                row["body"]["kind"]
                    .as_str()
                    .expect("a body kind")
                    .to_owned(),
                declarations.get(&body).cloned().unwrap_or(Value::Null),
            )
        })
        .collect()
}

#[trace("FR-997-AC-1", "TC-2820", "FSpec-FR-790-AC-1", "FSpec-TC-1919")]
#[test]
fn source_operations_carry_operation_body_rows() {
    let emission = emit_checked(&fixture_s_package()).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let mut rows = source_body_rows(&wire);
    rows.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let row = |ty: &str, operation: &str, kind: &str, body: Value| {
        (ty.to_owned(), operation.to_owned(), kind.to_owned(), body)
    };
    assert_eq!(
        rows,
        vec![
            row("Order", "bump", "no_body", Value::Null),
            row(
                "Order",
                "measure",
                "function",
                json!(["Order", "measure", "body"])
            ),
            row("Order", "scaled", "no_body", Value::Null),
            row("Order", "tally", "no_body", Value::Null),
            row(
                "Sub",
                "measure",
                "function",
                json!(["Sub", "measure", "body"])
            ),
            row("Sub", "scaled", "no_body", Value::Null),
        ]
    );
    let identity = owner_identity(&wire, "Order");
    assert!(
        wire["operation_bodies"]
            .as_array()
            .expect("rows")
            .iter()
            .all(|row| row["document"] == json!(identity))
    );
}

#[trace("FR-997-AC-2", "TC-2821", "FSpec-FR-790-AC-2", "FSpec-TC-1920")]
#[test]
fn a_forged_source_operation_body_row_refuses() {
    let emission = emit_checked(&fixture_s_package()).expect("S4 emits fixture S");
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("the I2 reader admits fixture S's operation bodies");
    };
    let wire = wire(&emission);
    let rows = source_body_rows(&wire);
    let at = |ty: &str, operation: &str| {
        rows.iter()
            .position(|(declaring, name, _, _)| declaring == ty && name == operation)
            .expect("the row")
    };
    let size = at("Order", "measure");
    let bump = at("Order", "bump");
    let sub_size = at("Sub", "measure");
    let precondition = nodes(&wire)
        .iter()
        .find(|node| {
            node["declaration"]["qualified_name"]
                == json!(["Order", "measure", "precondition", "Qs"])
        })
        .expect("Qs's clause function")["node_id"]
        .clone();
    let size_body = wire["operation_bodies"][size]["body"].clone();
    let invalid = ("invalid_package", "invalid-value");
    type Mutation = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(Mutation, String, (&str, &str))> = vec![
        (
            Box::new(move |w: &mut Value| {
                w["operation_bodies"][size]["body"] = json!({"kind": "no_body"});
            }),
            format!("/operation_bodies/{size}/body"),
            invalid,
        ),
        (
            Box::new(move |w: &mut Value| {
                w["operation_bodies"][bump]["body"] = size_body.clone();
            }),
            format!("/operation_bodies/{bump}/body"),
            invalid,
        ),
        (
            Box::new(move |w: &mut Value| {
                w["operation_bodies"][size]["body"]["node"] = precondition.clone();
            }),
            format!("/operation_bodies/{size}/body/node"),
            invalid,
        ),
        (
            Box::new(move |w: &mut Value| {
                w["operation_bodies"]
                    .as_array_mut()
                    .expect("rows")
                    .remove(sub_size);
            }),
            "/operation_bodies".to_owned(),
            invalid,
        ),
        (
            Box::new(move |w: &mut Value| {
                w["operation_bodies"][size]["document"] = json!("other/unit");
            }),
            format!("/operation_bodies/{size}/document"),
            ("missing_declaration", "missing-selection"),
        ),
    ];
    for (edit, locus, (code, cause)) in cases {
        let mut edited = wire.clone();
        edit(&mut edited);
        assert_eq!(
            refused(&edited),
            (json!(code), Some(json!(cause)), Some(locus.clone())),
            "{locus}"
        );
    }
}

/// The emitted wire, the strictly read package and the `identities.json` bytes
/// S4 writes for `unit`.
fn admitted_identity_file(unit: &str) -> (Value, CheckedPackageV2, Vec<u8>) {
    let text = format!("{HEADER}{unit}");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("S4 emits the unit");
    let wire = wire(&emission);
    let package = strict_read(emission.package().bytes(), &wire);
    let rows = emission
        .identity_rows(
            &package,
            &mut filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("source identity rows join");
    let bytes = rows.to_bytes(&package).expect("identity rows encode");
    (wire, package, bytes)
}

/// The strict read of `bytes`, which must admit them with no identity row.
fn strict_read(bytes: &[u8], wire: &Value) -> CheckedPackageV2 {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence);
    let CheckedPackageV2ReadResult::Admitted(package) = read else {
        panic!("the strict read admits the package with no identity row: {read:?}");
    };
    *package
}

/// The identity-file read of `file` over `package`.
fn read_identity_file(
    file: &Value,
    package: &CheckedPackageV2,
) -> Result<crate::ReadIdentityRows, crate::IdentityFileRefusal> {
    let mut text = serde_json::to_vec_pretty(file).expect("identity JSON writes");
    text.push(b'\n');
    crate::ReadIdentityRows::read(
        &text,
        package,
        &mut crate::V2ReadLimits::fixture().native_join_budget(),
    )
}

/// The identity row named `name` in the `member` list of `file`.
fn identity_row<'a>(file: &'a Value, member: &str, name: &str) -> &'a Value {
    file[member]
        .as_array()
        .unwrap_or_else(|| panic!("{member} rows"))
        .iter()
        .find(|row| row["source_identity"] == name || row["qualified_declaration"] == json!([name]))
        .unwrap_or_else(|| panic!("{member} row {name}"))
}

/// FR-230-AC-65's timed unit with a clock-parameter operation added.
const TIMED_ARM: &str = "\
profile T = \"filament.temporal.complete/v1\";\n\
dimension Time;
unit ms : Time = rational(1, 1);
record W { c: Clock; }
object O { w: W; d: Clock; operation arm(x: Clock, k: Integer) modifies self.d; }
time O dense unit ms;
";

#[trace("FR-569-AC-17", "TC-1408")]
#[test]
fn source_identity_rows_list_clock_members_and_clock_parameters() {
    let (_, package, bytes) = admitted_identity_file(TIMED_ARM);
    let file: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    let o = identity_row(&file, "source_objects", "O");
    let members = o["members"].as_array().expect("O's members");
    assert_eq!(members.len(), 2, "{o}");
    assert_eq!(members[0]["admitted_name"], "w");
    assert!(members[0]["type_node"].is_object());
    assert_eq!(members[0]["member_type"], Value::Null);
    assert_eq!(members[1]["admitted_name"], "d");
    assert_eq!(members[1]["type_node"], Value::Null);
    assert_eq!(members[1]["value_type"], Value::Null);
    assert_eq!(members[1]["member_type"], "ix://filament/native/Clock");
    assert_eq!(members[1]["collections"], json!([]));
    // FR-569-AC-17: the operation lists the clock entry `x`, then the data
    // entry `k` of `Integer`'s type node.
    let k_type = o["operations"][0]["parameters"][1]["type_node"].clone();
    assert!(k_type.is_object(), "{o}");
    assert_eq!(
        o["operations"],
        json!([{
            "identity": "O::arm",
            "query": false,
            "clock_parameters": ["x"],
            "parameters": [
                {
                    "name": "x",
                    "optional": false,
                    "type_node": null,
                    "member_type": "ix://filament/native/Clock",
                    "collections": []
                },
                {
                    "name": "k",
                    "optional": false,
                    "type_node": k_type,
                    "member_type": null,
                    "collections": []
                }
            ]
        }])
    );
    let w = identity_row(&file, "source_records", "W");
    assert_eq!(w["members"][0]["admitted_name"], "c");
    assert_eq!(w["members"][0]["member_type"], "ix://filament/native/Clock");
    assert!(
        read_identity_file(&file, &package).is_ok(),
        "the file reads back"
    );
}

/// FR-531-AC-34's unit.
const CLOCK_ENTRIES: &str = "\
profile T = \"filament.temporal.complete/v1\";\n\
dimension Time;
unit ms : Time = rational(1, 1);
record W { c: Clock; n: Integer; }
tuple P (Clock, Integer);
union U { armed(Clock), idle }
object O { w: W; d: Clock; q: Sequence<Clock>[0, 3]; p: P; u: U; operation arm(x: Clock, k: Integer) modifies self.d; }
time O dense unit ms;
";

#[trace("FR-531-AC-34", "TC-1331")]
#[test]
fn the_identity_file_carries_data_and_clock_member_entries() {
    let (_, package, bytes) = admitted_identity_file(CLOCK_ENTRIES);
    let file: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    assert_eq!(file["format"], crate::IDENTITIES_FORMAT);
    let o = identity_row(&file, "source_objects", "O");
    let kinds: Vec<(&str, bool)> = o["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|entry| {
            (
                entry["admitted_name"].as_str().unwrap_or_default(),
                entry["member_type"].is_null(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("w", true),
            ("d", false),
            ("q", false),
            ("p", true),
            ("u", true)
        ]
    );
    let q = &o["members"][2];
    assert_eq!(q["member_type"], "Sequence<ix://filament/native/Clock>");
    assert_eq!(
        q["collections"],
        json!([{"kind": "sequence", "minimum": "0", "maximum": "3"}])
    );
    let p = identity_row(&file, "source_tuples", "P");
    assert_eq!(
        p["positions"][0]["member_type"],
        "ix://filament/native/Clock"
    );
    assert!(p["positions"][1]["type_node"].is_object());
    let u = identity_row(&file, "source_unions", "U");
    let armed = &u["members"][0];
    assert_eq!(armed["payload_nodes"], json!([]));
    assert_eq!(
        armed["clock_payload"][0]["member_type"],
        "ix://filament/native/Clock"
    );
    assert!(
        read_identity_file(&file, &package).is_ok(),
        "the file reads back"
    );
    let mut old = file.clone();
    old["format"] = json!("filament.checked-handoff.identities/4");
    let mut both = file.clone();
    both["source_objects"][0]["members"][1]["type_node"] = o["members"][0]["type_node"].clone();
    let mut no_tuples = file.clone();
    no_tuples.as_object_mut().unwrap().remove("source_tuples");
    let mut no_parameters = file.clone();
    for row in no_parameters["source_objects"].as_array_mut().unwrap() {
        for entry in row["operations"].as_array_mut().unwrap() {
            entry.as_object_mut().unwrap().remove("clock_parameters");
        }
    }
    for (name, edited) in [
        ("/4", old),
        ("both", both),
        ("no tuples", no_tuples),
        ("no clock_parameters", no_parameters),
    ] {
        assert!(
            read_identity_file(&edited, &package).is_err(),
            "{name} refuses"
        );
    }
}

#[trace("FR-996-AC-3", "TC-2999", "FSpec-FR-789-AC-3", "FSpec-TC-2023")]
#[test]
fn a_source_clock_member_row_refuses_at_the_identity_row_join() {
    let (wire, package, bytes) = admitted_identity_file(TIMED);
    let mut file: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    assert!(
        read_identity_file(&file, &package).is_ok(),
        "both places join"
    );
    let index = file["source_objects"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["source_identity"] == "O")
        .expect("O's row");
    let members = file["source_objects"][index]["members"]
        .as_array_mut()
        .unwrap();
    let d = members
        .iter()
        .position(|entry| entry["admitted_name"] == "d")
        .expect("the clock entry d");
    members.remove(d);
    let place = places(&wire)
        .into_iter()
        .find(|&place| place_rows(&wire, place).len() == 1)
        .expect("d's place");
    let refusal = read_identity_file(&file, &package).expect_err("the join refuses d's place");
    let text = format!("{refusal:?}");
    assert!(text.contains("MalformedClockPlace"), "{text}");
    assert!(
        text.contains(&format!(
            "/semantic_graph/nodes/{place}/body/members/2/value/members/0"
        )),
        "{text}"
    );
}

/// FR-230 "Operation parameters" (QSL-2726): a timed source model whose
/// operation guards a `Clock` parameter and resets it in the postcondition
/// emits the guard's parameter constraint and the parameter reset.
#[trace("TC-1408")]
#[test]
fn a_source_clock_parameter_emits_its_constraint_and_reset() {
    let text = format!(
        "{HEADER}{TIMED_ARM}\
         pre G using T on O::arm {{ x <= 3 ms }}\n\
         post R using T on O::arm {{ x = 0 ms }}\n"
    );
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("S4 emits the timed source model");
    let wire = wire(&emission);
    let of_form = |form: &str| {
        nodes(&wire)
            .iter()
            .filter(|node| node["semantic_form"] == form)
            .count()
    };
    assert_eq!(of_form("clock_constraint"), 1, "{wire}");
    assert_eq!(of_form("clock_reset"), 1, "{wire}");
}

/// FSpec FR-789 rule 4 (QSL-2726): a source tuple position and a source union
/// member are spelled `.../P/p0` and `.../U/armed` in a clock place's
/// `resolved_path` rows.
#[trace("TC-2818", "FSpec-FR-789-AC-4")]
#[test]
fn a_source_tuple_and_union_clock_place_keep_the_source_spelling() {
    let text = format!("{HEADER}{CLOCK_ENTRIES}");
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let package = CheckedPackage::link(graph);
    let emission = emit_checked(&package).expect("S4 emits the timed source model");
    let wire = wire(&emission);
    let i = owner_identity(&wire, "O");
    let identities: Vec<String> = places(&wire)
        .into_iter()
        .flat_map(|place| place_rows(&wire, place))
        .map(|[_, member, _]| member)
        .collect();
    for expected in [format!("ix://{i}/P/p0"), format!("ix://{i}/U/armed")] {
        assert!(
            identities.contains(&expected),
            "{expected} in {identities:?}"
        );
    }
}

/// The id of the one node declared `name`.
fn declared_node(wire: &Value, name: &[&str]) -> Value {
    let [node] = &nodes(wire)
        .iter()
        .filter(|node| node["declaration"]["qualified_name"] == json!(name))
        .collect::<Vec<_>>()[..]
    else {
        panic!("one node declared {name:?}: {wire}");
    };
    node["node_id"].clone()
}

/// FSpec FR-322 "Clause functions" item 5: a source candidate whose operation
/// has one authored `pre` names its own effective-precondition function, a node
/// distinct from that clause's authored function.
#[trace("TC-1408", "FR-569-AC-6", "FSpec-FR-322-AC-544")]
#[test]
fn a_source_operation_declares_its_effective_precondition_function() {
    let emission = emit_checked(&fixture_s_package()).expect("S4 emits fixture S");
    let wire = wire(&emission);
    let authored = declared_node(&wire, &["Order", "measure", "precondition", "Qs"]);
    let effective = declared_node(&wire, &["Order", "measure", "effective_precondition"]);
    assert_ne!(effective, authored);
    let candidates = order_candidates(&wire);
    let candidate_with = |body: &[&str]| {
        let body = declared_node(&wire, body);
        candidates
            .iter()
            .find(|candidate| candidate["operation_body"] == body)
            .unwrap_or_else(|| panic!("a candidate with that body: {candidates:?}"))
    };
    let order = candidate_with(&["Order", "measure", "body"]);
    assert_eq!(order["effective_precondition"], effective);
    assert_ne!(order["effective_precondition"], authored);
    // `Sub.measure` declares no `pre`.
    let sub = candidate_with(&["Sub", "measure", "body"]);
    assert_eq!(sub["effective_precondition"], Value::Null);
}

/// FR-569: each source clause function has one `declaration` occurrence, at the
/// declaration it is declared by: an authored precondition at its `pre`, a body
/// at its `body`, an effective precondition at the candidate's `operation`
/// member. `T` is the body of an operation no call dispatches.
#[trace("TC-1408", "FR-569-AC-6")]
#[test]
fn a_source_clause_function_is_located_at_the_declaration_it_is_declared_by() {
    let text = format!(
        "{HEADER}{S}pre Qs2 using P on Sub::measure {{ self.total >= 1 }}\n\
         body T using P on Order::tally() {{ 3 }}\n"
    );
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let mut declarations =
        assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("S4 emits S-R");
    assert!(
        matches!(read_back(&emission), Read::Verified { .. }),
        "the strict reader admits the package"
    );
    let wire = wire(&emission);
    let occurrence = |name: &[&str]| -> &str {
        let node = declared_node(&wire, name);
        let [row] = &wire["source_map"]
            .as_array()
            .expect("a source map")
            .iter()
            .filter(|row| row["node_id"] == node)
            .collect::<Vec<_>>()[..]
        else {
            panic!("one occurrence of {name:?}");
        };
        assert_eq!(row["role"], "declaration", "{name:?}");
        let [region] = &row["regions"].as_array().expect("regions")[..] else {
            panic!("one region of {name:?}: {row}");
        };
        let at = |end: &str| usize::try_from(region[end].as_u64().unwrap()).unwrap();
        &text[at("start")..at("end")]
    };
    assert_eq!(
        occurrence(&["Order", "measure", "precondition", "Qs"]),
        "pre Qs using P on Order::measure { self.total >= 0 }"
    );
    assert_eq!(
        occurrence(&["Sub", "measure", "precondition", "Qs2"]),
        "pre Qs2 using P on Sub::measure { self.total >= 1 }"
    );
    assert_eq!(
        occurrence(&["Order", "measure", "body"]),
        "body C using P on Order::measure(n) { n }"
    );
    assert_eq!(
        occurrence(&["Sub", "measure", "body"]),
        "body B using P on Sub::measure(m) { m }"
    );
    assert_eq!(
        occurrence(&["Order", "tally", "body"]),
        "body T using P on Order::tally() { 3 }"
    );
    assert_eq!(
        occurrence(&["Order", "measure", "effective_precondition"]),
        "operation measure(n: Integer): Integer;"
    );
    assert_eq!(
        occurrence(&["Sub", "measure", "effective_precondition"]),
        "operation measure(m: Integer): Integer redefines Order::measure;"
    );
}

const CLOCK_OPTIONS: &str = "\
profile T = \"filament.temporal.complete/v1\";\n\
dimension Time;
unit ms : Time = rational(1, 1);
record W { c: Clock; }
tuple P (Clock, Integer);
union U { armed(Clock), idle }
object O { d: Clock?; q: Sequence<Clock>[0, 3]; w: W; p: P; u: U; operation arm(x: Clock, k: Integer) modifies self.d; }
time O dense unit ms;
";

/// FR-367 "Member entries" and FR-789 (TC-1331): one renderer spells a clock
/// member's type for the place rows and the identity entries. An optional
/// `Clock` field spells `Option<..>` exactly once, a collection, record field,
/// tuple position and union payload spell the clock bare, and the strict
/// reader joins every entry to its place. (The checker admits `Option` around
/// a clock only on an object field, so no other member form can carry one.)
#[trace("FR-531-AC-34", "TC-1331")]
#[test]
fn an_optional_clock_is_rendered_once_in_every_member_entry() {
    let clock = "ix://filament/native/Clock";
    let (_, package, bytes) = admitted_identity_file(CLOCK_OPTIONS);
    let file: Value = serde_json::from_slice(&bytes).expect("identity JSON parses");
    let o = identity_row(&file, "source_objects", "O");
    let types: Vec<(&str, String)> = o["members"]
        .as_array()
        .expect("members")
        .iter()
        .filter(|entry| !entry["member_type"].is_null())
        .map(|entry| {
            (
                entry["admitted_name"].as_str().expect("a name"),
                entry["member_type"].as_str().expect("a type").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        types,
        [
            ("d", format!("Option<{clock}>")),
            ("q", format!("Sequence<{clock}>")),
        ]
    );
    let w = identity_row(&file, "source_records", "W");
    assert_eq!(w["members"][0]["member_type"], clock);
    let p = identity_row(&file, "source_tuples", "P");
    assert_eq!(p["positions"][0]["member_type"], clock);
    let u = identity_row(&file, "source_unions", "U");
    assert_eq!(u["members"][0]["clock_payload"][0]["member_type"], clock);
    assert!(
        read_identity_file(&file, &package).is_ok(),
        "the file reads back"
    );
}

/// FSpec FR-322 "Expression occurrence ordinals", FR-825-AC-11: fixture S
/// shares applications between function owners. Each record names the
/// occurrence its owner's walk visit takes, the source-map region of that
/// occurrence is the use the record stands for (the record's own region), and
/// each body-location row of an application lies inside the declaration of the
/// function it names, so the rows moved with their uses.
/// The functions the body-location rows of `application` at `occurrence` name.
fn data_rows(
    admitted: &filament_contract_model::CheckedPackageV2,
    application: &CheckedNodeId,
    occurrence: u64,
) -> Vec<CheckedNodeId> {
    use filament_contract_model::CheckedBodyLocationOrigin;
    admitted
        .checked_term_data()
        .into_iter()
        .flat_map(|data| &data.body_locations)
        .filter(|row| row.application == *application && row.occurrence.ordinal == occurrence)
        .map(|row| {
            let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &row.location.origin;
            function.clone()
        })
        .collect()
}

#[trace("FR-825-AC-11", "FSpec-FR-322")]
#[test]
fn fixture_s_regions_and_rows_follow_the_owner_walk_numbering() {
    use filament_contract_model::{
        CheckedBodyLocationOrigin, CheckedOccurrenceRole as Role, CheckedPackageReadLimits,
        CheckedPackageV2, CheckedPackageV2ReadResult,
    };
    let package = fixture_s_package();
    let emission = emit_checked(&package).expect("S4 emits fixture S");
    let admitted = match CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        CheckedPackageV2ReadResult::Admitted(admitted) => admitted,
        failure => panic!("the strict reader admits fixture S: {failure:?}"),
    };
    let region = |node: &CheckedNodeId, role: Role, ordinal: u64| {
        let entries: Vec<_> = admitted
            .source_map()
            .iter()
            .filter(|entry| {
                entry.node_id == *node && entry.role == role && entry.ordinal == ordinal
            })
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "one source-map entry for {node:?} {ordinal}"
        );
        let region = &entries[0].regions[0];
        (region.start, region.end)
    };
    let records: Vec<_> = admitted
        .requirement_items()
        .iter()
        .filter_map(crate::emit::value_validity_requirements::value_record)
        .collect();
    assert!(!records.is_empty(), "fixture S has scalar records");
    let mut shared = 0;
    for record in &records {
        let application = admitted
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.node_id == record.application)
            .expect("the application is written");
        let uses = application
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.role == Role::Expression)
            .count();
        shared += usize::from(uses > 1);
        let own = admitted
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| node.node_id == record.node)
            .expect("the record is written")
            .occurrences[0]
            .ordinal;
        assert_eq!(
            region(&record.application, Role::Expression, record.occurrence),
            region(&record.node, Role::Generated, own),
            "the occurrence a record names is the use the record stands for"
        );
    }
    assert!(shared > 0, "fixture S shares an application between owners");
    let data = admitted
        .checked_term_data()
        .expect("fixture S has term rows");
    // A body-location row names the function whose body holds the use, so a row
    // at a record's occurrence names the record's owner.
    let mut checked = 0;
    for record in &records {
        for row in data_rows(&admitted, &record.application, record.occurrence) {
            assert_eq!(
                row, record.owner,
                "a row follows its use to the record's owner"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "some record's use has a body-location row");
    // An effective precondition is declared at its `operation` member
    // (FR-569) and its body is read from the `pre` clauses (FR-096), so its
    // uses lie outside its declaration region.
    let is_effective = |function: &CheckedNodeId| {
        admitted.resolved_graph().nodes.iter().any(|node| {
            node.node_id == *function
                && node.declaration.as_ref().is_some_and(|declared| {
                    declared.qualified_name.last().map(AsRef::as_ref)
                        == Some("effective_precondition")
                })
        })
    };
    let (mut inside, mut outside) = (0, 0);
    for row in &data.body_locations {
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &row.location.origin;
        let Some(declaration) = admitted
            .source_map()
            .iter()
            .find(|entry| entry.node_id == *function && entry.role == Role::Declaration)
        else {
            continue;
        };
        let (start, end) = region(&row.application, Role::Expression, row.occurrence.ordinal);
        let declared = &declaration.regions[0];
        if is_effective(function) {
            assert!(
                end <= declared.start || declared.end <= start,
                "an effective precondition's use is at its `pre`: {row:?}"
            );
            outside += 1;
            continue;
        }
        assert!(
            declared.start <= start && end <= declared.end,
            "a body-location row's occurrence is a use in the function it names: {row:?}"
        );
        inside += 1;
    }
    assert!(
        inside > 0,
        "fixture S has body-location rows of declared functions"
    );
    assert!(
        outside > 0,
        "fixture S has body-location rows of an effective precondition"
    );
}
