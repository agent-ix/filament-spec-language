// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source enum forms over genuine checked and strictly admitted artifacts.

use super::*;
use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    NativeJoinBudget,
};
use fsl_package::{BarePackage, BareSupply, ReadIdentityRows};

fn fixture(test: impl FnOnce(&fsl_package::CheckedPackage, &fsl_package::BareWorld)) {
    let packages =
        fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
    let text = b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nenum Status { READY, DONE }\nordered enum Level { LOW, MID, HIGH }\ntuple Pair(Status, Option<Level>);\nfunction status using v(s: Status): Status pure { s }\nfunction level using v(l: Level): Level pure { l }\nfunction pair using v(p: Pair): Pair pure { p }\n";
    let compiled = crate::spine::compose(
        fsl_foundation::SourceIdentity::new("test", "admitted-enum-forms"),
        "admitted-enum-forms.native",
        text,
        &packages,
        &crate::spine::DependencyInput::default(),
        crate::spine::SpineLimits::default(),
    )
    .expect("the actual source checks");
    let emission = fsl_package::emit_checked(&compiled.package).unwrap();
    let wire: Json = serde_json::from_slice(emission.package.bytes()).unwrap();
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let admitted = match CheckedPackageV2::read(
        emission.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("strict admission of genuine bytes: {other:?}"),
    };
    let mut joins = NativeJoinBudget::new(10_000_000, 10_000_000);
    let original = emission.identity_rows(&admitted, &mut joins).unwrap();
    let bytes = original.to_bytes(&admitted).unwrap();
    let read = ReadIdentityRows::read(&bytes, &admitted, &mut joins).unwrap();
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &read,
                equality: None,
            },
            &mut joins,
        )
        .unwrap();
    let world = bare
        .world()
        .expect("the admitted world retains its source enums");
    test(&compiled.package, &world);
}

/// Trace: FR-531-AC-50
#[test]
fn admitted_forms_match_source_names_variants_ranks_and_canonical_arguments() {
    fixture(|checked, world| {
        let scope = checked.graph().scope();
        let source = Forms::new(scope.types(), scope.enums());
        let mut joins = NativeJoinBudget::new(100_000, 100_000);
        let budget = std::cell::RefCell::new(Some(&mut joins));
        let admitted = Forms::from_admitted(world.types(), world.source_enums(), &budget);
        for (name, cases) in [
            ("Status", &["DONE", "READY"][..]),
            ("Level", &["LOW", "MID", "HIGH"][..]),
        ] {
            let binding = scope
                .enums()
                .iter()
                .find(|binding| binding.name == name)
                .unwrap();
            let declared = ValueType::Enum(binding.shape());
            assert!(admitted.has_form_scoped(&declared).unwrap());
            for (rank, case) in cases.iter().enumerate() {
                let form = json!({"variant":{"type":name,"case":case}});
                let expected = source.read(&form, &declared).unwrap();
                let actual = admitted.read(&form, &declared).unwrap();
                assert_eq!(actual, expected);
                let Value::Enum(member) = &actual else {
                    panic!("enum value")
                };
                assert_eq!(member.variant().case(), *case);
                assert_eq!(member.rank(), u32::try_from(rank).unwrap());
                assert_eq!(Some(member.variant()), binding.shape().variants().nth(rank));
                assert_eq!(admitted.write(&actual).unwrap(), form);
                let run = std::sync::Mutex::new(
                    filament_semantic_value::supplied_stop::SuppliedRun::new(
                        filament_semantic_value::supplied_stop::SuppliedLimits::default(),
                        filament_exact::Cancel::new(),
                    )
                    .unwrap(),
                );
                let canonical_name =
                    fsl_eval::simulation::model::run_declaration_name(member.variant().owner())
                        .unwrap();
                let canonical = json!({"type":"enum","name":canonical_name,"member":case});
                assert_eq!(
                    admitted
                        .read_canonical(&canonical, &declared, 0, &run)
                        .unwrap(),
                    expected
                );
            }
            let bad = json!({"variant":{"type":name,"case":"MISSING"}});
            let ReadStop::Defect(source_error) = source.read(&bad, &declared).unwrap_err() else {
                panic!("source argument defect")
            };
            let ReadStop::Defect(admitted_error) = admitted.read(&bad, &declared).unwrap_err()
            else {
                panic!("admitted argument defect")
            };
            assert_eq!(admitted_error, source_error);
            assert_eq!(admitted_error.path, "/variant/case");
        }
    });
}

/// Trace: FR-531-AC-44
#[test]
fn enum_lookup_preserves_exact_and_unpaid_native_metadata() {
    fixture(|checked, world| {
        let shape = checked
            .graph()
            .scope()
            .enums()
            .iter()
            .find(|binding| binding.name == "Status")
            .unwrap()
            .shape();
        let probe = |work, storage| {
            let mut joins = NativeJoinBudget::new(work, storage);
            let result = {
                let budget = std::cell::RefCell::new(Some(&mut joins));
                let forms = Forms::from_admitted(world.types(), world.source_enums(), &budget);
                forms.has_form_scoped(&ValueType::Enum(shape.clone()))
            };
            (result, joins)
        };
        let (result, paid) = probe(100_000, 100_000);
        assert!(result.unwrap());
        let spent = paid.consumed();
        assert!(spent.work > 0);
        let (exact, exact_budget) = probe(spent.work, spent.storage);
        assert!(exact.unwrap());
        assert_eq!(exact_budget.consumed(), spent);
        let mut limits = vec![(0, spent.storage), (spent.work - 1, spent.storage)];
        if spent.storage > 0 {
            limits.extend([(spent.work, 0), (spent.work, spent.storage - 1)]);
        }
        for (work, storage) in limits {
            let (result, budget) = probe(work, storage);
            let error = result.unwrap_err();
            let super::super::RunRefusal::SourceEnum(error) = error.as_ref() else {
                panic!("original enum refusal: {error:?}")
            };
            let SourceEnumCatalogError::Native(native) = error.as_ref() else {
                panic!("original native refusal: {error:?}")
            };
            assert_eq!(native.denied(), budget.first_unpaid());
            assert_eq!(native.locus(), budget.first_unpaid_locus());
            let denied = native.denied().unwrap();
            assert_eq!(
                native.configured_bound(),
                Some(match denied.kind {
                    filament_contract_model::NativeJoinChargeKind::Work => work,
                    filament_contract_model::NativeJoinChargeKind::Storage => storage,
                })
            );
        }
    });
}

/// Trace: FR-531-AC-43
#[test]
fn permission_is_required_only_for_requested_enum_forms() {
    fixture(|checked, world| {
        let budget = std::cell::RefCell::new(None);
        let forms = Forms::from_admitted(world.types(), world.source_enums(), &budget);
        assert!(forms.has_form_scoped(&ValueType::Boolean).unwrap());
        let shape = checked
            .graph()
            .scope()
            .enums()
            .iter()
            .find(|binding| binding.name == "Level")
            .unwrap()
            .shape();
        let nested = ValueType::Option(std::sync::Arc::new(ValueType::Enum(shape)));
        let error = forms.has_form_scoped(&nested).unwrap_err();
        assert!(
            matches!(error.as_ref(), super::super::RunRefusal::SourceEnum(error)
            if matches!(error.as_ref(), SourceEnumCatalogError::MissingNativePermission))
        );
        assert_eq!(error.code(), Code::UnknownRequiredFeature);
    });
}
