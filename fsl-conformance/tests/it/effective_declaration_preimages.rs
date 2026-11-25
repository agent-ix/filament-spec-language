// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public effective-declaration identity and derivation mutations.

use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, OriginalDeclarationKey, Presence};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord, Multiplicity,
    ObjectTypeRecord, ValueTypeRef,
};
use fsl_semantics::model::key::{EffectiveDeclarationPreimage, Fact, RULE_QUALIFY};
use fsl_semantics::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use fsl_semantics::model::refusal::ModelRefusalCause;
use ix_trace_rs::trace;

const MULTIPLICITY_0_1: Multiplicity = Multiplicity {
    lower: 0,
    upper: Some(1),
    ordered: false,
    unique: true,
};

fn declaration_key(node: impl Into<String>) -> DeclarationKey {
    DeclarationKey {
        package: "test/orders".to_owned(),
        node: node.into(),
    }
}

fn object_type(identity: &str, supertypes: Vec<&str>) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: declaration_key(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: supertypes.into_iter().map(declaration_key).collect(),
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: declaration_key(identity),
        owner: declaration_key(owner),
        value_type: ValueTypeRef::Package(declaration_key(value_type)),
        multiplicity: MULTIPLICITY_0_1,
        presence: Presence::Required,
        subsets: Vec::new(),
        redefines: None,
        position: 0,
    })
}

fn diamond_fixture() -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("tc195-f2"),
        vec![
            object_type("ix://test/orders/A", vec![]),
            object_type("ix://test/orders/B", vec!["ix://test/orders/A"]),
            object_type("ix://test/orders/C", vec!["ix://test/orders/A"]),
            object_type(
                "ix://test/orders/D",
                vec!["ix://test/orders/B", "ix://test/orders/C"],
            ),
            field_member(
                "ix://test/orders/A/x",
                "ix://test/orders/A",
                "ix://test/orders/A",
            ),
        ],
    )
}

fn complete(package: &DomainPackage) -> EffectiveView {
    match normalize(package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("expected admitted effective view, got {other:?}"),
    }
}

fn declaration<'a>(
    view: &'a EffectiveView,
    original: &str,
    owner: Option<&EffectiveId>,
) -> &'a EffectiveDeclarationPreimage {
    &view
        .declarations()
        .iter()
        .find(|entry| {
            entry.preimage.original.node == original
                && entry.preimage.owner_effective_type.as_ref() == owner
        })
        .unwrap_or_else(|| panic!("missing effective declaration {original} with owner {owner:?}"))
        .preimage
}

#[trace("FR-400-AC-10", "FR-400-AC-17")]
#[test]
fn effective_declaration_identity_is_structural_and_derivation_refusals_are_typed() {
    let view = complete(&diamond_fixture());
    let a = declaration(&view, "ix://test/orders/A", None);
    let b = declaration(&view, "ix://test/orders/B", None);
    let c = declaration(&view, "ix://test/orders/C", None);
    let d = declaration(&view, "ix://test/orders/D", None);
    let type_entries = [
        (a, "ix://test/orders/A"),
        (b, "ix://test/orders/B"),
        (c, "ix://test/orders/C"),
        (d, "ix://test/orders/D"),
    ];
    for (entry, original) in type_entries {
        assert_eq!(
            fsl_semantics::model::key::EffectiveDeclarationPreimage::identity(entry),
            EffectiveId::new(None, OriginalDeclarationKey::new("test/orders", original)),
            "type identity {original} is the structural declaration key"
        );
        assert_eq!(
            fsl_semantics::model::key::EffectiveDeclarationPreimage::validate_derivation(entry),
            Ok(())
        );
    }
    let member_entries = [
        (a, "ix://test/orders/A"),
        (b, "ix://test/orders/B"),
        (c, "ix://test/orders/C"),
        (d, "ix://test/orders/D"),
    ]
    .map(|(owner, owner_key)| {
        (
            declaration(&view, "ix://test/orders/A/x", Some(&owner.identity())),
            owner,
            owner_key,
        )
    });
    for (member, owner, owner_key) in member_entries {
        assert_eq!(
            fsl_semantics::model::key::EffectiveDeclarationPreimage::identity(member),
            EffectiveId::new(
                Some(owner.identity()),
                OriginalDeclarationKey::new("test/orders", "ix://test/orders/A/x"),
            ),
            "member identity under {owner_key} includes that effective owner"
        );
        assert_eq!(
            fsl_semantics::model::key::EffectiveDeclarationPreimage::validate_derivation(member),
            Ok(())
        );
    }
    let d_x = declaration(&view, "ix://test/orders/A/x", Some(&d.identity()));

    let mut unsorted = d_x.clone();
    unsorted.derivation.swap(0, 1);
    let (cause, _) = unsorted
        .validate_derivation()
        .expect_err("swapped ordinals refuse");
    assert!(matches!(
        cause,
        ModelRefusalCause::UnsortedDerivation {
            position: 0,
            ordinal: 1,
            ..
        }
    ));

    let repeated = Fact {
        ordinal: 0,
        rule: RULE_QUALIFY,
        inputs: vec![unsorted.original.clone()],
        parent: None,
    };
    let duplicate = EffectiveDeclarationPreimage {
        owner_effective_type: None,
        original: unsorted.original.clone(),
        derivation: vec![
            repeated.clone(),
            Fact {
                ordinal: 1,
                ..repeated
            },
        ],
    };
    let (cause, _) = duplicate
        .validate_derivation()
        .expect_err("same rule and inputs refuse");
    assert!(matches!(
        cause,
        ModelRefusalCause::DuplicatePath {
            earlier: 0,
            later: 1,
            ..
        }
    ));
}

/// The duplicate-path example is constructed in this consumer crate, so the
/// validator call also exercises its public cross-crate API.
#[trace("FR-400-AC-16")]
#[test]
fn exact_qualify_duplicate_path_is_rejected_from_another_crate() {
    let original = declaration_key("ix://test/orders/A/x");
    let inputs = vec![original.clone()];
    let preimage = EffectiveDeclarationPreimage {
        owner_effective_type: None,
        original: original.clone(),
        derivation: vec![
            Fact {
                ordinal: 0,
                rule: RULE_QUALIFY,
                inputs: inputs.clone(),
                parent: None,
            },
            Fact {
                ordinal: 1,
                rule: RULE_QUALIFY,
                inputs,
                parent: None,
            },
        ],
    };

    let (cause, _) = preimage
        .validate_derivation()
        .expect_err("duplicate qualify path is refused");
    assert_eq!(
        cause,
        ModelRefusalCause::DuplicatePath {
            original,
            earlier: 0,
            later: 1,
        }
    );
}
