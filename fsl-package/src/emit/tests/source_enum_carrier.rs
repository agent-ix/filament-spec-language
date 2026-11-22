// SPDX-License-Identifier: AGPL-3.0-or-later
//! Identities/6 preserves source enum producer identities through byte admission.

use filament_contract_model::{ExactTypeProjectionErrorCode, NativeJoinBudget};

use super::source_union_rows::admit_bytes;
use super::*;

const ENUMS: &str = "enum Status { READY, DONE }\n\
    ordered enum Level { LOW, MID, HIGH }\n\
    function ready using v(): Status pure { Status::READY }\n\
    function low using v(): Level pure { Level::LOW }\n";

fn fixture() -> (
    CheckedPackage,
    Emission,
    filament_contract_model::CheckedPackageV2,
) {
    let (_, source) = package_from_text(ENUMS);
    let emission = emit_checked(&source).expect("genuine enum emission");
    let admitted = admit_bytes(&emission, &wire(&emission));
    (source, emission, admitted)
}
fn budget() -> NativeJoinBudget {
    NativeJoinBudget::new(10_000_000, 10_000_000)
}

#[trace("FR-531-AC-50", "FR-531-AC-8")]
#[test]
fn source_enum_carrier_preserves_original_rows_shapes_order_and_unused_case() {
    let (source, emission, admitted) = fixture();
    let mut ledger = budget();
    let original = emission
        .selected_source_enum_rows(&mut ledger)
        .expect("original stage claims");
    let rows = emission
        .identity_rows(&admitted, &mut ledger)
        .expect("paid producer rows");
    assert_eq!(rows.source_enums(), original);
    let bytes = rows.to_bytes(&admitted).expect("identities/6");
    let json: Value = serde_json::from_slice(&bytes).expect("identity object");
    assert_eq!(json["format"], "filament.checked-handoff.identities/6");
    assert_eq!(json["model_enum_declarations"], json!([]));
    assert_eq!(json["model_enum_members"], json!([]));
    assert_eq!(json["model_enum_shapes"], json!([]));
    let reread =
        crate::ReadIdentityRows::read(&bytes, &admitted, &mut ledger).expect("strict byte reader");
    assert_eq!(reread, rows);
    let selected = reread
        .selected(&admitted, &mut ledger)
        .expect("bounded complete source join");
    for (name, ordered, cases) in [
        ("Status", false, ["DONE", "READY"].as_slice()),
        ("Level", true, ["LOW", "MID", "HIGH"].as_slice()),
    ] {
        let binding = source
            .graph()
            .scope()
            .enums()
            .iter()
            .find(|binding| binding.name == name)
            .expect("original checked binding");
        let row = reread
            .source_enums()
            .iter()
            .find(|row| row.declaration.node_key == binding.declaration.key())
            .expect("original declaration identity");
        assert_eq!(row.declaration.shape, binding.shape());
        assert_eq!(row.declaration.shape.is_ordered(), ordered);
        assert_eq!(
            row.members
                .iter()
                .map(|member| member.case.as_ref())
                .collect::<Vec<_>>(),
            cases
        );
        let projected = admitted
            .project_exact_type(&row.declaration.structural_node, &selected, &mut ledger)
            .expect("original enum projects");
        assert_eq!(projected.value_type(), &ValueType::Enum(binding.shape()));
        assert_eq!(projected.source_enum_view(), Some(row));
        for (case, member) in row.members.iter().zip(&binding.members) {
            assert_eq!(case.member_key, member.member());
            assert_eq!(case.variant, member.variant());
        }
    }
    assert_eq!(
        bytes,
        emission
            .identity_rows(&admitted, &mut budget())
            .expect("second producer materialization")
            .to_bytes(&admitted)
            .expect("deterministic encoding")
    );
    let text = std::str::from_utf8(&bytes).expect("UTF8");
    assert!(
        text.find("\n  \"source_unions\":").expect("unions")
            < text.find("\n  \"source_enums\":").expect("enums")
    );
    assert!(
        text.find("\n  \"source_enums\":").expect("enums")
            < text.find("\n  \"source_objects\":").expect("objects")
    );
}

#[trace("FR-531-AC-43")]
#[test]
fn source_enum_carrier_refuses_closed_schema_corruption_and_missing_population() {
    let (_, emission, admitted) = fixture();
    let rows = emission
        .identity_rows(&admitted, &mut budget())
        .expect("stage rows");
    let bytes = rows.to_bytes(&admitted).expect("identity bytes");
    let original: Value = serde_json::from_slice(&bytes).expect("JSON");
    let mut missing = original.clone();
    missing
        .as_object_mut()
        .expect("object")
        .remove("source_enums");
    assert!(matches!(
        crate::ReadIdentityRows::read(
            &serde_json::to_vec(&missing).expect("negative bytes"),
            &admitted,
            &mut budget()
        ),
        Err(crate::IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::SourceEnums
        })
    ));
    let mut old = original.clone();
    old["format"] = json!("filament.checked-handoff.identities/5");
    assert!(matches!(
        crate::ReadIdentityRows::read(
            &serde_json::to_vec(&old).expect("old format"),
            &admitted,
            &mut budget()
        ),
        Err(crate::IdentityFileRefusal::Format(_))
    ));
    for mutate in [0, 1, 2, 3, 4] {
        let mut invalid = original.clone();
        let entry = &mut invalid["source_enums"][0];
        match mutate {
            0 => {
                entry["members"].as_array_mut().expect("members").reverse();
            }
            1 => {
                entry["members"].as_array_mut().expect("members").pop();
            }
            2 => {
                entry["declaration"]["source_authority"] = json!("foreign");
            }
            3 => {
                let member = entry["members"][0].clone();
                entry["members"]
                    .as_array_mut()
                    .expect("members")
                    .push(member);
            }
            4 => {
                entry["declaration"]["shape"]["ordered"] = json!(!entry["declaration"]["shape"]
                    ["ordered"]
                    .as_bool()
                    .expect("ordered flag"));
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                crate::ReadIdentityRows::read(
                    &serde_json::to_vec(&invalid).expect("corrupted witness"),
                    &admitted,
                    &mut budget()
                ),
                Err(crate::IdentityFileRefusal::Native(_))
            ),
            "corruption {mutate}"
        );
    }
    let mut absent = original.clone();
    absent["source_enums"] = json!([]);
    let read = crate::ReadIdentityRows::read(
        &serde_json::to_vec(&absent).expect("empty channel"),
        &admitted,
        &mut budget(),
    )
    .expect("empty join itself supplies no enum witness");
    assert!(matches!(
        read.selected(&admitted, &mut budget()),
        Err(crate::IdentityJoinFailure::Witness(
            crate::IdentityWitnessRefusal::Unknown
        ))
    ));
    let error = admitted
        .project_exact_type(
            &rows.source_enums()[0].declaration.structural_node,
            &filament_contract_model::SelectedExactIdentities::empty(),
            &mut budget(),
        )
        .expect_err("missing demanded enum identity");
    assert_eq!(
        error.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}

#[trace("FR-531-AC-44", "FR-531-AC-30")]
#[test]
fn source_enum_carrier_selection_keeps_original_cumulative_native_bounds() {
    let (_, emission, admitted) = fixture();
    let rows = emission
        .identity_rows(&admitted, &mut budget())
        .expect("original rows");
    let mut measured = budget();
    rows.selected(&admitted, &mut measured)
        .expect("complete bounded join");
    let paid = measured.consumed();
    let mut exact = NativeJoinBudget::new(paid.work, paid.storage);
    rows.selected(&admitted, &mut exact)
        .expect("exact work and storage");
    assert_eq!(exact.consumed(), paid);
    for (work, storage) in [
        (paid.work - 1, paid.storage),
        (paid.work, paid.storage - 1),
        (0, paid.storage),
        (paid.work, 0),
    ] {
        let mut short = NativeJoinBudget::new(work, storage);
        assert!(rows.selected(&admitted, &mut short).is_err());
        let denied = short.first_unpaid().expect("original first denial");
        let locus = short.first_unpaid_locus();
        let spent = short.consumed();
        assert!(rows.selected(&admitted, &mut short).is_err());
        assert_eq!(short.first_unpaid(), Some(denied));
        assert_eq!(short.first_unpaid_locus(), locus);
        assert_eq!(short.consumed(), spent);
    }
}
