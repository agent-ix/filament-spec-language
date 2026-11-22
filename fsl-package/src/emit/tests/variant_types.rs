// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360-AC-8, FR-360-AC-4 (TC-1009, TC-1007 step 3): a unit whose
//! function, clause or field read names a model variant type emits it as one
//! model-owned `variant_type` node, each case as a `value`/`enum_value` node
//! of it (any non-empty case name, kept exactly), passes a variant value by
//! `ref` with no conversion node, and reads back `Verified` through IR's
//! checked-package/v2 reader. The admitted-equality accessor returns the exact
//! missing enum-member witness until FSL's per-fact rows are consumed.

use super::owners::node_where;
use super::*;
use filament_semantic_value::checking::CheckMode;

#[path = "../../../../fsl-semantics/tests/support/variant_fields.rs"]
pub(super) mod fixture;

/// The cases of the variant type `Odd`, in ascending UTF-8 byte order: none
/// is a source identifier.
const ODD_CASES: [&str; 3] = ["2x", "Café", "Gold Plus"];

/// The units that name only `Tier`, whose cases are identifiers.
const TIER_UNIT: &str = "\
function pick using v(t: M::Tier): M::Tier pure { t }\n\
function isGold using v(t: M::Tier): Boolean pure { t = M::Tier::Gold }\n\
function notGold using v(t: M::Tier): Boolean pure { t != M::Tier::Gold }\n\
function tierOf using v(s: M::Settings): M::Tier pure { s.tier }\n\
post PromoteKeeps using v on M::Account::promote { result = t }\n";

/// [`TIER_UNIT`] and a function over `Odd`.
pub(super) const UNIT: &str = "\
function pick using v(t: M::Tier): M::Tier pure { t }\n\
function isGold using v(t: M::Tier): Boolean pure { t = M::Tier::Gold }\n\
function notGold using v(t: M::Tier): Boolean pure { t != M::Tier::Gold }\n\
function tierOf using v(s: M::Settings): M::Tier pure { s.tier }\n\
function oddSame using v(a: M::Odd, b: M::Odd): Boolean pure { a = b }\n\
post PromoteKeeps using v on M::Account::promote { result = t }\n";

struct Emitted {
    emission: Emission,
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// The emission of `unit_body` over the `Tier` and `Odd` model enums.
pub(super) fn emitted_enums(unit_body: &str) -> Emission {
    emit_unit(unit_body).emission
}

pub(super) fn emitted_complete_enums(unit_body: &str) -> Emission {
    let (graph, document) = checked_unit_with_profile(unit_body, "filament.model.complete/v1");
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the complete package emits");
    emitted_from(emission, document).emission
}

fn emit_unit(unit_body: &str) -> Emitted {
    let (graph, document) = checked_unit(unit_body);
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    emitted_from(emission, document)
}

/// The checked graph of `unit_body` over the `Tier` and `Odd` model enums,
/// with the domain document it selects.
fn checked_unit(unit_body: &str) -> (fsl_semantics::check::CheckedGraph, Vec<u8>) {
    checked_unit_with_profile(unit_body, "filament.value.complete/v1")
}

fn checked_unit_with_profile(
    unit_body: &str,
    profile: &str,
) -> (fsl_semantics::check::CheckedGraph, Vec<u8>) {
    let document = serde_json::to_vec(&fixture::with_types(vec![fixture::variant_type(
        "Odd",
        &["Gold Plus", "Café", "2x"],
    )]))
    .expect("the fixture serializes");
    checked_document(unit_body, profile, document)
}

fn checked_document(
    unit_body: &str,
    profile: &str,
    document: Vec<u8>,
) -> (fsl_semantics::check::CheckedGraph, Vec<u8>) {
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = {profile:?};\n\
         model M = {package:?};\n\
         {unit_body}\n",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (graph, document)
}

fn emitted_from(emission: Emission, document: Vec<u8>) -> Emitted {
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    Emitted {
        emission,
        wire,
        evidence,
    }
}

fn bare_enums() -> Emitted {
    let document = serde_json::to_vec(&fixture::package(
        vec![
            fixture::variant_type("Tier", &["Gold", "Standard"]),
            fixture::variant_type("Odd", &["Gold Plus", "Café", "2x"]),
        ],
        false,
    ))
    .unwrap();
    let (graph, document) = checked_document(
        "function keep using v(): Boolean pure { true }",
        "filament.value.complete/v1",
        document,
    );
    emitted_from(
        emit_checked(&CheckedPackage::link(graph)).expect("real selected enums emit"),
        document,
    )
}

impl Emitted {
    fn read(&self, bytes: &[u8]) -> Read {
        read_v2(bytes, library(), V2ReadLimits::fixture(), &self.evidence)
    }

    /// The node of the variant type `name`.
    fn variant_node(&self, name: &str) -> &Value {
        let found: Vec<&Value> = node_where(&self.wire, "model", "variant_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id(name))
            .collect();
        assert_eq!(found.len(), 1, "{name} is one variant_type node");
        found[0]
    }

    /// The case names of the `enum_value` nodes typed by `variant`, in the
    /// order the wire holds them.
    fn cases_of(&self, variant: &Value) -> Vec<String> {
        node_where(&self.wire, "value", "enum_value")
            .into_iter()
            .filter(|node| node["semantic_type"] == variant["node_id"])
            .map(|node| node["body"]["value"].as_str().unwrap().to_owned())
            .collect()
    }
}

/// Checks emitted enum producer rows and the exact IR accessor refusal for
/// each decision, using this fixture's selected domain-package evidence.
fn assert_enum_equality_reader_outcomes(emitted: &Emitted) {
    let decisions = emitted.wire["checked_term_data"]["equality_decisions"]
        .as_array()
        .expect("the package emits equality decisions");
    assert!(!decisions.is_empty());
    assert_eq!(
        emitted.emission.equality_producer_identity_rows().len(),
        decisions.len(),
        "each serialized decision retains its producer application row"
    );
    assert!(decisions
        .iter()
        .filter(|decision| decision["schedule"] == "enum")
        .all(|decision| !decision["enum_members"].as_array().unwrap().is_empty()));
    super::assert_equality_reader_outcomes(
        &emitted.emission,
        emitted.read(emitted.emission.package.bytes()),
    );
}

/// FR-360-AC-8 item 12: each variant type the unit names is one
/// `model`/`variant_type` node with a `ModelOwner` of the declaring package
/// and the type's node, none is a unit-owned `scalar_type`/`enum`, and the
/// cases of the type are its `enum_value` nodes, any non-empty name kept
/// exactly.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_variant_type_is_one_model_owned_node_with_one_enum_value_per_case() {
    let emitted = emit_unit(
        "function pick using v(t: M::Tier): M::Tier pure { t }\n\
         function oddPick using v(t: M::Odd): M::Odd pure { t }",
    );
    for name in ["Tier", "Odd"] {
        assert_eq!(
            emitted.variant_node(name)["owner"],
            json!({"kind": "model", "identity": fixture::PACKAGE, "node": fixture::id(name)})
        );
    }
    assert_eq!(
        node_where(&emitted.wire, "scalar_type", "enum").len(),
        0,
        "no variant type is a unit-owned enum"
    );
    let mut cases = emitted.cases_of(emitted.variant_node("Tier"));
    cases.sort();
    assert_eq!(cases, ["Gold", "Standard"]);
    let mut odd = emitted.cases_of(emitted.variant_node("Odd"));
    odd.sort();
    assert_eq!(odd, ODD_CASES);
}

/// QSL-1398 acceptance evidence. No current FSL spec criterion covers selected
/// model enum witness row joins.
#[test]
fn selected_model_enum_rows_join_and_refuse_missing_mismatched_duplicate_and_foreign_rows() {
    use filament_contract_model::{
        SelectedModelEnumDeclarationRow, SelectedModelEnumErrorCode, SelectedModelEnumMemberRow,
        SelectedModelEnums,
    };

    let emitted = emit_unit(
        "function pick using v(t: M::Tier): M::Tier pure { t }\n\
         function oddPick using v(t: M::Odd): M::Odd pure { t }",
    );
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("the emitted model enums are admitted")
    };
    let declarations = emitted.emission.selected_model_enum_declaration_rows();
    let members = emitted.emission.selected_model_enum_member_rows();
    assert_eq!(declarations.len(), 4);
    assert_eq!(members.len(), 9);
    for row in declarations {
        assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
        assert!([
            fixture::id("Tier"),
            fixture::id("Odd"),
            fixture::id("Level"),
            fixture::id("Grade")
        ]
        .iter()
        .any(|identity| identity.as_str() == row.original_identity.as_ref()));
        let selected = emitted
            .emission
            .selected_model_enums(&admitted, 128)
            .expect("the selected enum declaration rows join");
        assert_eq!(
            selected.read_declaration(&admitted, &row.structural_node),
            Ok(row)
        );
    }
    for row in members {
        assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
        let (owner, member) = match row.case.as_ref() {
            "Gold" if row.enum_original_identity.as_ref() == fixture::id("Tier").as_str() => {
                ("Tier", "variant/Tier-Gold")
            }
            "Standard" if row.enum_original_identity.as_ref() == fixture::id("Tier").as_str() => {
                ("Tier", "variant/Tier-Standard")
            }
            "Gold" if row.enum_original_identity.as_ref() == fixture::id("Level").as_str() => {
                ("Level", "variant/Level-Gold")
            }
            "Gold" if row.enum_original_identity.as_ref() == fixture::id("Grade").as_str() => {
                ("Grade", "variant/Grade-Gold")
            }
            "Standard" if row.enum_original_identity.as_ref() == fixture::id("Grade").as_str() => {
                ("Grade", "variant/Grade-Standard")
            }
            "Bronze" => ("Grade", "variant/Grade-Bronze"),
            "2x" => ("Odd", "variant/Odd-2x"),
            "Café" => ("Odd", "variant/Odd-Caf"),
            "Gold Plus" => ("Odd", "variant/Odd-Gold-Plus"),
            other => panic!("unexpected selected model case {other:?}"),
        };
        assert_eq!(
            row.enum_original_identity.as_ref(),
            fixture::id(owner).as_str()
        );
        assert_eq!(
            row.member_original_identity.as_ref(),
            fixture::id(member).as_str()
        );
        let selected = emitted
            .emission
            .selected_model_enums(&admitted, 128)
            .expect("the selected enum member rows join");
        let row_id = serde_json::to_value(&row.structural_node)
            .expect("the selected enum row id serializes");
        assert!(
            node_where(&emitted.wire, "value", "enum_value")
                .iter()
                .any(|node| node["node_id"] == row_id),
            "the selected enum member row names an emitted enum_value node"
        );
        assert_eq!(
            selected.read_member(&admitted, &row.structural_node),
            Ok(row)
        );
    }

    let declaration = declarations[0].clone();
    let missing = SelectedModelEnums::from_stage_rows(
        &admitted,
        Vec::<SelectedModelEnumDeclarationRow>::new(),
        Vec::<SelectedModelEnumMemberRow>::new(),
        128,
    )
    .expect("an empty row index is valid until read");
    assert_eq!(
        missing
            .read_declaration(&admitted, &declaration.structural_node)
            .unwrap_err()
            .code,
        SelectedModelEnumErrorCode::UnavailableWitness
    );

    let mut mismatched = declaration.clone();
    mismatched.selected_document = "foreign/document".into();
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [mismatched],
            Vec::<SelectedModelEnumMemberRow>::new(),
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::MismatchedWitness
    );

    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone(), declaration.clone()],
            Vec::<SelectedModelEnumMemberRow>::new(),
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::ConflictingWitness
    );

    let member = members[0].clone();
    let declaration = declarations
        .iter()
        .find(|row| row.structural_node == member.declaration_node)
        .expect("the selected member has its declaration row")
        .clone();
    let mut mismatched_member = member.clone();
    mismatched_member.case = "unknown-case".into();
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone()],
            [mismatched_member],
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::MismatchedWitness
    );
    assert_eq!(
        SelectedModelEnums::from_stage_rows(
            &admitted,
            [declaration.clone()],
            [member.clone(), member],
            128,
        )
        .unwrap_err()
        .code,
        SelectedModelEnumErrorCode::ConflictingWitness
    );

    let selected = emitted
        .emission
        .selected_model_enums(&admitted, 128)
        .expect("all rows join");
    let Read::Verified {
        admitted: foreign, ..
    } = emitted.read(emitted.emission.package.bytes())
    else {
        panic!("a second reader admits an independent package")
    };
    assert_eq!(
        selected
            .read_declaration(&foreign, &declaration.structural_node)
            .unwrap_err()
            .code,
        SelectedModelEnumErrorCode::ForeignPackage
    );
}

/// FR-360-AC-8: a call passing a variant value takes it by `ref`, no node
/// converts it, an equality over variant values is the enumeration equality
/// application, and `!=` negates it.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_variant_value_is_passed_by_ref_and_compared_by_the_enumeration_equality() {
    let emitted = emit_unit(UNIT);
    assert!(
        !emitted.wire.to_string().contains("convert"),
        "no conversion node anywhere"
    );
    let applied = |identity: &str| {
        nodes(&emitted.wire)
            .iter()
            .filter(|node| node["body"]["operation"]["identity"] == identity)
            .count()
    };
    assert_eq!(
        applied("filament.op.enum.eq"),
        3,
        "isGold, oddSame and the clause"
    );
    assert_eq!(applied("filament.op.enum.ne"), 1, "notGold");
}

/// QSL-1390: model enum members keep their exact declaration and member
/// identities in the equality sidecar, including case names that are not
/// source identifiers.
/// Provenance: QSL-1390, QSL-1445
#[ix_trace_rs::trace("FSpec-FR-322-AC-288")]
#[test]
fn model_enum_equality_emits_exact_member_identities() {
    let emitted = emit_unit(UNIT);
    let decisions = emitted.wire["checked_term_data"]["equality_decisions"]
        .as_array()
        .expect("the checked-term sidecar carries equality decisions");
    let decision = decisions
        .iter()
        .find(|decision| decision["schedule"] == "enum")
        .expect("oddSame uses the enum schedule");
    let variant = emitted.variant_node("Odd");
    let members = decision["enum_members"]
        .as_array()
        .expect("the enum decision carries exact members");
    assert_eq!(members.len(), ODD_CASES.len());
    for (member, case) in members.iter().zip(ODD_CASES) {
        assert_eq!(member["enum_type"], variant["node_id"]);
        assert_eq!(member["case"], case);
        let member_node = nodes(&emitted.wire)
            .iter()
            .find(|node| node["node_id"] == member["member"])
            .expect("the sidecar member identity names an emitted node");
        assert_eq!(member_node["semantic_form"], "enum_value");
        assert_eq!(member_node["semantic_type"], variant["node_id"]);
        assert_eq!(member_node["body"]["value"], case);
    }

    let identity = emitted
        .emission
        .equality_producer_identity_rows()
        .iter()
        .find(|row| {
            row.producer_identity()
                .enum_member_facts()
                .iter()
                .any(|(_, _, _, case, _, _)| ODD_CASES.contains(&case.as_ref()))
        })
        .expect("selected imported Odd enum equality has producer rows");
    let odd_facts = identity
        .producer_identity()
        .enum_member_facts()
        .iter()
        .filter(|(_, _, _, case, _, _)| ODD_CASES.contains(&case.as_ref()))
        .collect::<Vec<_>>();
    assert_eq!(odd_facts.len(), ODD_CASES.len());
    let emitted_ids = nodes(&emitted.wire)
        .iter()
        .filter_map(|node| node.get("node_id").cloned())
        .collect::<Vec<_>>();
    for (source_declaration, declaration, member, case, ordinal, ordered) in odd_facts {
        let rows = emitted
            .emission
            .equality_fact_witness_rows()
            .iter()
            .filter(|row| {
                row.application() == identity.application()
                    && row.occurrence() == identity.occurrence()
                    && row.structural_key() == member
                    && matches!(row.fact(), EqualityFactWitness::EnumMember { .. })
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1, "each exact selected enum fact has one row");
        let row = rows[0];
        let EqualityFactWitness::EnumMember {
            source_declaration: actual_source,
            source_declaration_node: actual_source_node,
            declaration: actual_declaration,
            declaration_node: actual_declaration_node,
            member: actual_member,
            member_node: actual_member_node,
            case: actual_case,
            ordinal: actual_ordinal,
            ordered: actual_ordered,
        } = row.fact()
        else {
            unreachable!()
        };
        assert_eq!(actual_source, source_declaration);
        assert_eq!(actual_declaration, declaration);
        assert_eq!(actual_member, member);
        assert_eq!(actual_case.as_ref(), case.as_ref());
        assert_eq!(actual_ordinal, ordinal);
        assert_eq!(actual_ordered, ordered);
        assert_eq!(row.structural_node(), actual_member_node);
        for node in [
            actual_source_node,
            actual_declaration_node,
            actual_member_node,
        ] {
            assert!(
                emitted_ids.contains(&serde_json::to_value(node).unwrap()),
                "each exact selected key maps to an emitted checked node"
            );
        }
    }
}

/// Original model declaration keys are document-owned, but their final nodes
/// are local equality facts and must reach the native decision reader.
#[trace("FR-531-AC-42", "FSpec-FR-322-AC-288")]
#[test]
fn retained_model_enum_equality_facts_form_native_decisions() {
    let emitted = emit_unit(UNIT);
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("actual emitted enums admit")
    };
    let mut budget = filament_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000);
    assert_eq!(emitted.emission.equality_producer_identity_rows().len(), 4);
    for row in emitted.emission.equality_producer_identity_rows() {
        let decision = emitted
            .emission
            .admitted_equality(&admitted, row.checked_node(), row.occurrence(), &mut budget)
            .expect("complete local model enum equality witness");
        assert_eq!(
            decision.schedule(),
            filament_semantic_value::declaration::EqualitySchedule::Enum
        );
    }
}

/// FR-360-AC-8: a unit naming only a variant type with identifier cases emits
/// its model enum; IR admits the carrier and the accessor reports the exact
/// enum-member witness it still needs. The document's `Account` operations
/// carry their `operation_effects` rows, which root only the nodes they name
/// (FR-094), never the other declared variant types.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn an_identifier_cased_variant_type_emits_and_reports_enum_member_witness() {
    let emitted = emit_unit(TIER_UNIT);
    assert_eq!(
        node_where(&emitted.wire, "model", "variant_type").len(),
        4,
        "every selected payload-free enum is retained, including unused Level, Grade and Odd"
    );
    assert!(emitted.wire["operation_effects"].is_array());
    assert_enum_equality_reader_outcomes(&emitted);
}

/// FR-360-AC-8, FR-360-AC-4: the package of a unit with a variant-typed
/// parameter, result, field read and clause retains its equality facts; IR
/// admits the carrier and the accessor reports the exact enum-member witness.
#[trace("FR-360-AC-8", "FR-360-AC-4", "TC-1009", "TC-1007")]
#[test]
fn a_package_naming_variant_types_reports_enum_member_witness() {
    let emitted = emit_unit(UNIT);
    assert_enum_equality_reader_outcomes(&emitted);
}

/// The package with `edit` applied to the wire of the first `Odd` case node
/// named `case`.
fn tampered_read(case: &str, edit: impl FnOnce(&mut Value, &Value)) -> Read {
    let emitted = emit_unit(
        "function pick using v(t: M::Tier): M::Tier pure { t }\n\
         function oddPick using v(t: M::Odd): M::Odd pure { t }",
    );
    let tier = emitted.variant_node("Tier")["node_id"].clone();
    let mut wire = emitted.wire.clone();
    let node = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["semantic_form"] == "enum_value" && node["body"]["value"] == case)
        .expect("the case node is emitted");
    edit(node, &tier);
    emitted.read(&jcs(&wire))
}

/// A case of the owner's own node that the declaring variant type does not
/// hold is refused by IR's reader, as is a case moved to
/// another variant type's node.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn a_case_outside_its_variant_type_is_refused_by_the_reader() {
    let renamed = tampered_read("Gold Plus", |node, _| {
        node["body"]["value"] = json!("Silver");
    });
    assert!(matches!(renamed, Read::Refused(_)), "{renamed:?}");
    let moved = tampered_read("Gold Plus", |node, tier| {
        node["semantic_type"] = tier.clone();
        node["dependencies"] = json!([tier]);
        node["body"]["type"] = tier.clone();
    });
    assert!(matches!(moved, Read::Refused(_)), "{moved:?}");
}

/// An empty case name is refused by IR's reader.
#[trace("FR-360-AC-8", "TC-1009")]
#[test]
fn an_empty_case_name_is_refused_by_the_reader() {
    let emptied = tampered_read("2x", |node, _| {
        node["body"]["value"] = json!("");
    });
    assert!(matches!(emptied, Read::Refused(_)), "{emptied:?}");
}

/// FR-092-AC-26 (TC-1021): each emitted model enum has one shape row whose
/// declaration and case ids are the model enum rows' final ids, whose cases
/// are in the shape's canonical (case-name byte) order and whose variants are
/// the shape's at their rank; an altered shape is not the row's shape.
#[trace("FR-092-AC-26", "TC-1021")]
#[test]
fn a_model_enum_shape_row_pairs_the_checked_shape_with_the_final_node_ids() {
    use filament_exact::{EnumShape, VariantId};

    let emitted = emit_unit(
        "function pick using v(t: M::Tier): M::Tier pure { t }\n\
         function oddPick using v(t: M::Odd): M::Odd pure { t }",
    );
    let Read::Verified { .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("the emitted model enums are admitted")
    };
    let rows = emitted.emission.selected_model_enum_shape_rows();
    assert_eq!(rows.len(), 4, "one shape row per selected model enum");
    let declarations = emitted.emission.selected_model_enum_declaration_rows();
    let members = emitted.emission.selected_model_enum_member_rows();
    for (name, expected) in [
        ("Tier", vec!["Gold", "Standard"]),
        ("Odd", ODD_CASES.to_vec()),
    ] {
        let row = rows
            .iter()
            .find(|row| row.original_identity.as_ref() == fixture::id(name).as_str())
            .unwrap_or_else(|| panic!("{name} has a shape row"));
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.node_key == row.node_key)
            .expect("the declaration row of the shape row");
        assert_eq!(row.structural_node, declaration.structural_node);
        assert_eq!(row.selected_document, declaration.selected_document);
        let cases: Vec<&str> = row.cases.iter().map(|case| case.case.as_ref()).collect();
        assert_eq!(cases, expected, "{name}: cases in canonical order");
        let variants: Vec<VariantId> = row.shape.variants().cloned().collect();
        assert_eq!(variants.len(), expected.len());
        assert!(!row.shape.is_ordered(), "a variant type is unordered");
        for (rank, case) in row.cases.iter().enumerate() {
            assert_eq!(case.variant, variants[rank], "{name}: variant at rank");
            let member = members
                .iter()
                .find(|member| member.member_key == case.member_key)
                .expect("the member row of the case");
            assert_eq!(case.structural_node, member.structural_node);
            assert_eq!(member.declaration_node, row.structural_node);
            assert_eq!(
                case.member_original_identity,
                member.member_original_identity
            );
        }
        // Adverse: a reordered, a missing and an extra variant each differ.
        let mut reordered = variants.clone();
        reordered.reverse();
        assert_ne!(row.shape, EnumShape::new(false, reordered));
        assert_ne!(
            row.shape,
            EnumShape::new(false, variants[1..].iter().cloned())
        );
        assert_ne!(
            row.shape,
            EnumShape::new(false, variants.iter().cloned().chain([variants[0].clone()]))
        );
        assert_eq!(row.shape, EnumShape::new(false, variants));
    }
}

/// FR-092-AC-26 (TC-1021): a model enum shape row is a writer fault when a
/// case's member node or selected declaration node is not emitted.
#[trace("FR-092-AC-26", "TC-1021")]
#[test]
fn a_model_enum_shape_row_refuses_an_unemitted_case_or_declaration() {
    let unit = "function oddPick using v(t: M::Odd): M::Odd pure { t }";
    let (graph, _) = checked_unit(unit);
    let emission = emit_unit(unit).emission;
    let mut ids: HashMap<NodeKey, CheckedNodeId> = HashMap::new();
    for row in emission.selected_model_enum_declaration_rows() {
        ids.insert(row.node_key.clone(), row.structural_node.clone());
    }
    for row in emission.selected_model_enum_member_rows() {
        ids.insert(row.member_key.clone(), row.structural_node.clone());
    }
    let rows = super::super::model_enum_shape_rows(&graph, &ids).expect("complete ids give rows");
    assert_eq!(rows.len(), 4);
    let odd = rows
        .iter()
        .find(|row| row.original_identity.as_ref() == fixture::id("Odd"))
        .expect("Odd retained");
    assert_eq!(odd.cases.len(), ODD_CASES.len());

    let dropped = odd.cases[1].member_key.clone();
    let mut missing = ids.clone();
    missing.remove(&dropped);
    assert!(
        super::super::model_enum_shape_rows(&graph, &missing).is_err(),
        "a case with no emitted node is a writer fault"
    );

    let mut undeclared = ids;
    undeclared.remove(&rows[0].node_key);
    assert!(
        super::super::model_enum_shape_rows(&graph, &undeclared).is_err(),
        "a selected declaration cannot disappear"
    );
}

/// Complete original model enums cross the strict identities carrier even
/// when no function reads their types. Authored case positions do not replace
/// canonical ranks authenticated by IR against the model document.
#[trace("FR-531-AC-8", "FR-531-AC-42", "FR-092-AC-26")]
#[test]
fn model_enums_round_trip_unused_declarations_and_canonical_runtime_members() {
    use crate::{BarePackage, BareSupply, ReadIdentityRows};
    use filament_contract_model::NativeJoinBudget;
    let emitted = bare_enums();
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("the selected model admits")
    };
    let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
    let original = emitted
        .emission
        .identity_rows(&admitted, &mut budget)
        .expect("producer rows");
    let bytes = original.to_bytes(&admitted).expect("carrier");
    let value: Value = serde_json::from_slice(&bytes).expect("JSON");
    assert_eq!(value["format"], "filament.checked-handoff.identities/5");
    assert_eq!(
        value["model_enum_declarations"].as_array().unwrap().len(),
        2
    );
    assert_eq!(value["model_enum_members"].as_array().unwrap().len(), 5);
    assert_eq!(value["model_enum_shapes"].as_array().unwrap().len(), 2);
    let rows =
        ReadIdentityRows::read(&bytes, &admitted, &mut budget).expect("native authenticated read");
    assert_eq!(rows, original);
    let identities = rows
        .selected(&admitted, &mut budget)
        .expect("same local join");
    let types = admitted
        .project_selected_type_declarations(&identities, &[], &[], &mut budget)
        .expect("complete full projection");
    assert_eq!(types.enums().len(), 2);
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut budget,
        )
        .expect("bounded world admission");
    let world = bare.world().expect("world");
    for row in types.enums() {
        for (rank, case) in row.cases.iter().enumerate() {
            let member = world
                .enum_members()
                .resolve(case.variant.clone())
                .expect("every selected member is retained");
            assert_eq!(member.position(), rank);
            assert_eq!(member.member(), case.member_key);
            assert_eq!(member.case(), case.case.as_ref());
            assert!(!member.is_ordered());
        }
    }
}

/// Required enum channels are closed and versioned; native witness defects
/// cannot become a partial successful selection.
#[trace("FR-531-AC-12", "FR-531-AC-43", "FR-531-AC-8")]
#[test]
fn model_enum_carrier_refuses_missing_repeated_foreign_and_conflicting_evidence() {
    use crate::{IdentityFileRefusal, ReadIdentityRows};
    use filament_contract_model::NativeJoinBudget;
    let emitted = bare_enums();
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("selected model admits")
    };
    let budget = || NativeJoinBudget::new(10_000_000, 10_000_000);
    let rows = emitted
        .emission
        .identity_rows(&admitted, &mut budget())
        .expect("rows");
    let bytes = rows.to_bytes(&admitted).expect("carrier");
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    for channel in [
        "source_tuples",
        "model_enum_declarations",
        "model_enum_members",
        "model_enum_shapes",
        "model_unions",
    ] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(channel);
        assert!(
            matches!(
                ReadIdentityRows::read(
                    &serde_json::to_vec(&missing).unwrap(),
                    &admitted,
                    &mut budget()
                ),
                Err(IdentityFileRefusal::Malformed { .. })
            ),
            "{channel} is required"
        );
        let text = String::from_utf8(bytes.clone()).unwrap();
        let repeated = format!("{{{channel:?}:[],{}", &text[1..]);
        assert!(
            matches!(
                ReadIdentityRows::read(repeated.as_bytes(), &admitted, &mut budget()),
                Err(IdentityFileRefusal::Malformed { .. })
            ),
            "{channel} is unique"
        );
    }
    for edit in [
        "foreign",
        "missing",
        "duplicate",
        "rank",
        "case_order",
        "unknown",
        "old",
    ] {
        let mut changed = value.clone();
        match edit {
            "foreign" => {
                changed["model_enum_declarations"][0]["selected_document"] =
                    json!("foreign/document")
            }
            "missing" => {
                changed["model_enum_members"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                let row = changed["model_enum_members"][0].clone();
                changed["model_enum_members"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            "rank" => changed["model_enum_shapes"][0]["shape"]["variants"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            "case_order" => changed["model_enum_shapes"][0]["cases"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            "unknown" => {
                changed
                    .as_object_mut()
                    .unwrap()
                    .insert("model_enum_unknown".into(), json!([]));
            }
            "old" => changed["format"] = json!("filament.checked-handoff.identities/4"),
            _ => unreachable!(),
        }
        let result = ReadIdentityRows::read(
            &serde_json::to_vec(&changed).unwrap(),
            &admitted,
            &mut budget(),
        );
        use filament_contract_model::ExactTypeProjectionErrorCode as Native;
        match (edit, result) {
            ("unknown", Err(IdentityFileRefusal::Malformed { .. })) => {}
            ("old", Err(IdentityFileRefusal::Format(format))) => {
                assert_eq!(format, "filament.checked-handoff.identities/4")
            }
            ("missing", Err(IdentityFileRefusal::Native(error))) => {
                assert_eq!(error.code, Native::UnavailableIdentityWitness)
            }
            ("duplicate", Err(IdentityFileRefusal::Native(error))) => {
                assert_eq!(error.code, Native::ConflictingIdentityWitness)
            }
            ("foreign" | "rank" | "case_order", Err(IdentityFileRefusal::Native(error))) => {
                assert_eq!(error.code, Native::MismatchedIdentityWitness)
            }
            (_, other) => panic!("{edit} must preserve its typed refusal: {other:?}"),
        }
    }
}

/// Producer and native enum copies consume one cumulative allowance. Exact
/// boundaries succeed; one-short and zero ceilings preserve the first unpaid
/// resource rather than returning incomplete evidence.
#[trace("FR-531-AC-31", "FR-531-AC-44", "FR-092-AC-26")]
#[test]
fn model_enum_identity_and_full_projection_budgets_are_cumulative() {
    use filament_contract_model::{NativeJoinBudget, NativeJoinChargeKind};
    let emitted = bare_enums();
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("selected model admits")
    };
    let mut budget = NativeJoinBudget::new(10_000_000, 10_000_000);
    let rows = emitted
        .emission
        .identity_rows(&admitted, &mut budget)
        .expect("rows");
    let identities = rows
        .selected(&admitted, &mut budget)
        .expect("native enum join");
    let before_projection = budget.consumed();
    let types = admitted
        .project_selected_type_declarations(&identities, &[], &[], &mut budget)
        .expect("full projection");
    assert_eq!(types.enums().len(), 2);
    let need = budget.consumed();
    assert!(need.work > before_projection.work && need.storage > before_projection.storage);
    for (work, storage, denied) in [
        (need.work, need.storage, None),
        (
            need.work - 1,
            need.storage,
            Some(NativeJoinChargeKind::Work),
        ),
        (
            need.work,
            need.storage - 1,
            Some(NativeJoinChargeKind::Storage),
        ),
        (0, need.storage, Some(NativeJoinChargeKind::Work)),
        (need.work, 0, Some(NativeJoinChargeKind::Storage)),
    ] {
        let mut exact = NativeJoinBudget::new(work, storage);
        let result = emitted
            .emission
            .identity_rows(&admitted, &mut exact)
            .ok()
            .and_then(|rows| rows.selected(&admitted, &mut exact).ok())
            .and_then(|identities| {
                admitted
                    .project_selected_type_declarations(&identities, &[], &[], &mut exact)
                    .ok()
            });
        if let Some(kind) = denied {
            assert!(result.is_none());
            assert_eq!(exact.first_unpaid().expect("original denial").kind, kind);
        } else {
            assert_eq!(result.expect("exact allowance").enums().len(), 2);
            assert_eq!(exact.consumed(), need);
        }
    }
}

/// A document whose variant types hold payload cases: `Shape` over an
/// `Integer` and a record, `Phase` over a Clock.
fn bare_unions() -> Emitted {
    let mut shape = fixture::variant_type("Shape", &["Circle", "Empty", "Square"]);
    shape["variants"][0]["payloadType"] = json!("ix://filament/native/Integer");
    shape["variants"][2]["payloadType"] = json!(fixture::id("Settings"));
    let mut phase = fixture::variant_type("Phase", &["Armed", "Idle"]);
    phase["variants"][0]["payloadType"] = json!("ix://filament/native/Clock");
    let document = serde_json::to_vec(&fixture::with_types(vec![shape, phase])).unwrap();
    let (graph, document) = checked_document(
        "function circle using v(): M::Shape pure { M::Shape::Circle(3) }\n\
         function idle using v(): M::Phase pure { M::Phase::Idle }",
        "filament.value.complete/v1",
        document,
    );
    emitted_from(
        emit_checked(&CheckedPackage::link(graph)).expect("selected payload unions emit"),
        document,
    )
}

/// Payload-bearing variant types emit one canonically ordered union row each,
/// round trip through the identities carrier, and refuse a missing, repeated,
/// foreign or reordered witness with a typed refusal.
#[trace("FR-531-AC-12", "FR-531-AC-43", "FR-531-AC-8")]
#[test]
fn model_union_rows_round_trip_and_refuse_missing_foreign_and_reordered_evidence() {
    use crate::{IdentityFileRefusal, ReadIdentityRows};
    use filament_contract_model::{NativeJoinBudget, SelectedModelUnionPayload as Payload};
    let emitted = bare_unions();
    let rows = emitted.emission.selected_model_union_rows();
    let names = |row: &filament_contract_model::SelectedModelUnionRow| {
        row.cases
            .iter()
            .map(|case| case.name.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(rows.len(), 2);
    let phase = &rows[0];
    let shape = &rows[1];
    assert_eq!(names(phase), ["Armed", "Idle"]);
    assert!(matches!(phase.cases[0].payload, Payload::Clock { .. }));
    assert!(matches!(phase.cases[1].payload, Payload::None));
    assert_eq!(names(shape), ["Circle", "Empty", "Square"]);
    assert!(matches!(shape.cases[0].payload, Payload::Data { .. }));
    assert!(matches!(shape.cases[1].payload, Payload::None));
    assert!(matches!(shape.cases[2].payload, Payload::Data { .. }));
    for row in rows {
        for (rank, case) in row.cases.iter().enumerate() {
            assert_eq!(case.rank, rank as u64);
        }
    }
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("selected model admits")
    };
    let budget = || NativeJoinBudget::new(10_000_000, 10_000_000);
    let carried = emitted
        .emission
        .identity_rows(&admitted, &mut budget())
        .expect("rows");
    let bytes = carried.to_bytes(&admitted).expect("carrier");
    let read = ReadIdentityRows::read(&bytes, &admitted, &mut budget()).expect("round trip");
    assert_eq!(read.model_unions().len(), 2);
    assert_eq!(read.to_bytes(&admitted).expect("carrier"), bytes);
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("model_unions");
    assert!(matches!(
        ReadIdentityRows::read(
            &serde_json::to_vec(&missing).unwrap(),
            &admitted,
            &mut budget()
        ),
        Err(IdentityFileRefusal::Malformed { .. })
    ));
    for edit in ["foreign", "missing", "duplicate", "position", "rank"] {
        let mut changed = value.clone();
        match edit {
            "foreign" => changed["model_unions"][0]["selected_document"] = json!("foreign/doc"),
            "missing" => {
                changed["model_unions"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                let row = changed["model_unions"][0].clone();
                changed["model_unions"].as_array_mut().unwrap().push(row);
            }
            "position" => changed["model_unions"][1]["cases"][0]["position"] = json!(2),
            "rank" => changed["model_unions"][1]["cases"]
                .as_array_mut()
                .unwrap()
                .reverse(),
            _ => unreachable!(),
        }
        let result = ReadIdentityRows::read(
            &serde_json::to_vec(&changed).unwrap(),
            &admitted,
            &mut budget(),
        );
        assert!(
            matches!(result, Err(IdentityFileRefusal::Native(_))),
            "{edit} must keep its typed native refusal: {result:?}"
        );
    }
}

/// Union producer copies and the native union join consume one cumulative
/// allowance: the exact total succeeds and one unit short refuses with the
/// first unpaid resource.
#[trace("FR-531-AC-31", "FR-531-AC-44", "FR-092-AC-26")]
#[test]
fn model_union_identity_and_join_budgets_are_cumulative() {
    use filament_contract_model::{NativeJoinBudget, NativeJoinChargeKind};
    let emitted = bare_unions();
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("selected model admits")
    };
    let run = |budget: &mut NativeJoinBudget| {
        emitted
            .emission
            .identity_rows(&admitted, budget)
            .ok()
            .and_then(|rows| rows.selected(&admitted, budget).ok())
            .map(|identities| identities.model_union_rows().count())
    };
    let mut full = NativeJoinBudget::new(10_000_000, 10_000_000);
    assert_eq!(run(&mut full), Some(2));
    let need = full.consumed();
    assert!(need.work > 0 && need.storage > 0);
    for (work, storage, denied) in [
        (need.work, need.storage, None),
        (
            need.work - 1,
            need.storage,
            Some(NativeJoinChargeKind::Work),
        ),
        (
            need.work,
            need.storage - 1,
            Some(NativeJoinChargeKind::Storage),
        ),
    ] {
        let mut exact = NativeJoinBudget::new(work, storage);
        let result = run(&mut exact);
        match denied {
            Some(kind) => {
                assert_eq!(result, None);
                assert_eq!(exact.first_unpaid().expect("original denial").kind, kind);
            }
            None => {
                assert_eq!(result, Some(2));
                assert_eq!(exact.consumed(), need);
            }
        }
    }
}

/// A union witness whose Clock payload names another type, or whose member
/// key is not an original structural key, is refused rather than joined.
#[trace("FR-531-AC-12", "FR-531-AC-8")]
#[test]
fn a_malformed_model_union_witness_is_refused() {
    use crate::{IdentityFileRefusal, ReadIdentityRows};
    use filament_contract_model::NativeJoinBudget;
    let emitted = bare_unions();
    let Read::Verified { admitted, .. } = emitted.read(emitted.emission.package.bytes()) else {
        panic!("selected model admits")
    };
    let budget = || NativeJoinBudget::new(10_000_000, 10_000_000);
    let bytes = emitted
        .emission
        .identity_rows(&admitted, &mut budget())
        .expect("rows")
        .to_bytes(&admitted)
        .expect("carrier");
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    for edit in ["clock_type", "unknown_member", "payload_kind"] {
        let mut changed = value.clone();
        match edit {
            "clock_type" => {
                changed["model_unions"][0]["cases"][0]["payload"]["payload_type"] =
                    json!("ix://filament/native/Integer")
            }
            "unknown_member" => changed["model_unions"][0]["cases"][0]["extra"] = json!(1),
            "payload_kind" => {
                changed["model_unions"][0]["cases"][0]["payload"] = json!({"kind": "weight"})
            }
            _ => unreachable!(),
        }
        let result = ReadIdentityRows::read(
            &serde_json::to_vec(&changed).unwrap(),
            &admitted,
            &mut budget(),
        );
        match (edit, result) {
            ("clock_type", Err(IdentityFileRefusal::Native(_))) => {}
            ("unknown_member" | "payload_kind", Err(IdentityFileRefusal::Malformed { .. })) => {}
            (_, other) => panic!("{edit} must keep its typed refusal: {other:?}"),
        }
    }
}
