// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public model assembly and conformance boundaries.

use filament_checked::key::DeclarationKey;
use filament_exact::Presence;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::model::accounting::{LimitKind, Meter, ModelNormalizationLimits};
use fsl_semantics::model::conformance::{self, ConformanceCheckOutcome, ConformanceOutcome};
use fsl_semantics::model::domain_package::{
    DeclaredSlot, DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord,
    Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, RelationshipDirection, RelationshipEnd,
    RelationshipRecord, ValueTypeRef,
};
use fsl_semantics::model::index::ModelIndex;
use fsl_semantics::model::normalize::{self, EffectiveView, ModelRefusalCause, NormalizeOutcome};
use ix_trace_rs::trace;

fn key(name: &str) -> DeclarationKey {
    DeclarationKey::fixture(&format!("ix://test/binding/{name}"))
}

fn package(records: Vec<DomainPackageRecord>) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef {
            identity: "test/binding".to_owned(),
        },
        records,
    )
}

fn object(name: &str, parents: &[&str]) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type: false,
        supertypes: parents.iter().map(|name| key(name)).collect(),
    })
}

fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

fn field(owner: &str, name: &str, value_type: ValueTypeRef) -> FieldMemberRecord {
    FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type,
        multiplicity: one(),
        presence: Presence::Required,
        subsets: Vec::new(),
        redefines: None,
        position: 0,
    }
}

fn completed(input: &DomainPackage) -> EffectiveView {
    match normalize::normalize(input, ModelNormalizationLimits::default()) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("valid model must normalize: {other:?}"),
    }
}

fn operation(owner: &str) -> OperationMemberRecord {
    OperationMemberRecord {
        key: key(&format!("{owner}/update")),
        owner: key(owner),
        parameters: vec![OperationParameterRecord {
            key: key(&format!("{owner}/update/value")),
            value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            multiplicity: one(),
            presence: Presence::Required,
        }],
        result: Some(OperationResult {
            value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            multiplicity: one(),
        }),
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines: (owner == "Child").then(|| key("Base/update")),
    }
}

#[trace("FR-082-AC-1", "TC-218")]
#[test]
fn operation_conformance_reports_each_independent_axis_and_all_three_together() {
    for mask in 0..8 {
        let base = operation("Base");
        let mut child = operation("Child");
        if mask & 1 != 0 {
            child.parameters[0].value_type = ValueTypeRef::Native(NativeValueType::Integer);
        }
        if mask & 2 != 0 {
            child
                .result
                .as_mut()
                .expect("fixture result")
                .multiplicity
                .lower = 0;
        }
        if mask & 4 != 0 {
            child.effect.modifies.push(key("Child/extra"));
        }
        let input = package(vec![
            object("Base", &[]),
            object("Child", &["Base"]),
            DomainPackageRecord::FieldMember(field(
                "Child",
                "extra",
                ValueTypeRef::Native(NativeValueType::Boolean),
            )),
            DomainPackageRecord::OperationMember(base),
            DomainPackageRecord::OperationMember(child),
        ]);
        let index = ModelIndex::build(input);
        let mut meter = Meter::new(ModelNormalizationLimits::default());
        let outcome = conformance::check_operation_redefinition(
            &index,
            &key("Child/update"),
            &key("Base/update"),
            &mut meter,
        );
        if mask == 0 {
            assert_eq!(
                outcome,
                ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
            );
            continue;
        }
        let ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(failures)) = outcome
        else {
            panic!("mask {mask} must refuse its failing axes: {outcome:?}")
        };
        let mut expected = Vec::new();
        if mask & 1 != 0 {
            expected.push("parameter-type");
        }
        if mask & 2 != 0 {
            expected.push("result-multiplicity");
        }
        if mask & 4 != 0 {
            expected.push("effect");
        }
        assert_eq!(
            failures
                .iter()
                .map(|failure| failure.axis)
                .collect::<Vec<_>>(),
            expected,
            "mask {mask}"
        );
        for failure in failures {
            assert_eq!(failure.code, Code::IllTyped);
            match failure.axis {
                "parameter-type" => assert!(matches!(
                    failure.cause,
                    ModelRefusalCause::VarianceParameter { index: 1, .. }
                )),
                "result-multiplicity" => assert!(
                    matches!(failure.cause, ModelRefusalCause::MultiplicityNarrowing { from, to } if from.lower == 0 && to == one())
                ),
                "effect" => assert!(
                    matches!(failure.cause, ModelRefusalCause::EffectEscape { field } if field == key("Child/extra"))
                ),
                unexpected => panic!("unexpected conformance axis {unexpected}"),
            }
        }
    }
}

#[trace("FR-082-AC-2", "TC-219")]
#[test]
fn redefinition_requires_a_proper_ancestor_and_cycles_yield_no_view() {
    let inherited = operation("Base");
    let redefining = operation("Child");
    for inherited_parent in [true, false] {
        let input = package(vec![
            object("Base", &[]),
            object("Child", if inherited_parent { &["Base"] } else { &[] }),
            DomainPackageRecord::OperationMember(inherited.clone()),
            DomainPackageRecord::OperationMember(redefining.clone()),
        ]);
        match normalize::normalize(&input, ModelNormalizationLimits::default()) {
            NormalizeOutcome::Completed(view) => {
                assert!(inherited_parent, "unrelated owner must not redefine Base");
                assert!(
                    view.declarations()
                        .iter()
                        .any(|entry| entry.preimage.original == key("Child/update"))
                );
            }
            NormalizeOutcome::Refused(refusals) => {
                assert!(!inherited_parent, "actual inherited target must resolve");
                assert!(refusals.iter().any(|refusal| matches!(&refusal.cause,
                    ModelRefusalCause::RedefinitionTarget { redefiners, target }
                    if redefiners == &[key("Child/update")] && target == &key("Base/update")
                )));
            }
            other => panic!("small model must not exhaust limits: {other:?}"),
        }
    }
    let cycle = package(vec![object("Base", &["Child"]), object("Child", &["Base"])]);
    let NormalizeOutcome::Refused(refusals) =
        normalize::normalize(&cycle, ModelNormalizationLimits::default())
    else {
        panic!("cyclic specialization must produce no completed effective view")
    };
    let declarations = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            ModelRefusalCause::SpecializationCycle { declaration, cycle } => {
                assert_eq!(cycle, &[key("Base"), key("Child")]);
                Some(declaration.clone())
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        declarations,
        [key("Base"), key("Child")].into_iter().collect()
    );
}

fn chain_index(edges: usize) -> ModelIndex {
    let mut records = vec![object("T0", &[])];
    for index in 1..=edges {
        records.push(object(&format!("T{index}"), &[&format!("T{}", index - 1)]));
    }
    records.push(DomainPackageRecord::FieldMember(field(
        "T0",
        "general",
        ValueTypeRef::Package(key("T0")),
    )));
    records.push(DomainPackageRecord::FieldMember(field(
        "T0",
        "specific",
        ValueTypeRef::Package(key(&format!("T{edges}"))),
    )));
    ModelIndex::build(package(records))
}

#[trace("FR-082-AC-3", "TC-220")]
#[test]
fn conformance_ancestor_limit_denies_the_next_edge_then_accepts_the_exact_bound() {
    let index = chain_index(3);
    let mut short = Meter::new(ModelNormalizationLimits::default().with_ancestor_steps(2));
    let ConformanceCheckOutcome::Incomplete(stop) = conformance::check_field_redefinition(
        &index,
        &key("T0/specific"),
        &key("T0/general"),
        &mut short,
    ) else {
        panic!("three-edge conformance must not return a verdict with only two edges")
    };
    assert_eq!(stop.limit_kind, LimitKind::AncestorSteps);
    assert_eq!((stop.limit, stop.consumed, stop.next_charge), (2, 2, 1));
    let mut exact = Meter::new(ModelNormalizationLimits::default().with_ancestor_steps(3));
    assert_eq!(
        conformance::check_field_redefinition(
            &index,
            &key("T0/specific"),
            &key("T0/general"),
            &mut exact
        ),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
    );
}

#[trace("FR-082-AC-3", "TC-220")]
#[test]
fn ten_thousand_conformance_edges_complete_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let index = chain_index(10_000);
            let mut meter = Meter::new(ModelNormalizationLimits::default());
            assert_eq!(
                conformance::check_field_redefinition(
                    &index,
                    &key("T0/specific"),
                    &key("T0/general"),
                    &mut meter
                ),
                ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible)
            );
        })
        .expect("start small-stack conformance worker")
        .join()
        .expect("conformance worker finishes without stack overflow");
}

fn relationship() -> RelationshipRecord {
    RelationshipRecord {
        key: key("Source/links"),
        declared_at: Some(DeclaredSlot {
            owner: key("Source"),
            index: 0,
        }),
        source: RelationshipEnd {
            type_identity: key("Source"),
            role: Some("owners".to_owned()),
            multiplicity: Some(Multiplicity {
                lower: 2,
                upper: Some(4),
                ordered: true,
                unique: false,
            }),
        },
        target: RelationshipEnd {
            type_identity: key("Target"),
            role: None,
            multiplicity: None,
        },
        direction: RelationshipDirection::SourceToTarget,
    }
}

#[trace("FR-085-AC-1", "FR-085-AC-2", "TC-230", "TC-231")]
#[test]
fn relationship_binding_preserves_declared_ends_and_refuses_each_missing_type() {
    let valid = relationship();
    let input = package(vec![
        object("Source", &[]),
        object("Target", &[]),
        DomainPackageRecord::Relationship(valid.clone()),
    ]);
    let view = completed(&input);
    let retained = view
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Relationship(record) => Some(record),
            _ => None,
        })
        .expect("resolved relationship remains in the effective model");
    assert_eq!(retained, &valid);
    assert_eq!(
        retained
            .source
            .multiplicity
            .expect("declared source bound")
            .lower,
        2
    );
    assert_eq!(
        retained.target.multiplicity, None,
        "an absent target bound is never defaulted"
    );
    assert!(view.type_identities().contains_key(&key("Source")));
    assert!(view.type_identities().contains_key(&key("Target")));
    for (side, member) in [
        ("source", "Relationship.source"),
        ("target", "Relationship.target"),
    ] {
        let mut altered = valid.clone();
        if side == "source" {
            altered.source.type_identity = key("Missing");
        } else {
            altered.target.type_identity = key("Missing");
        }
        let input = package(vec![
            object("Source", &[]),
            object("Target", &[]),
            DomainPackageRecord::Relationship(altered),
        ]);
        let NormalizeOutcome::Refused(refusals) =
            normalize::normalize(&input, ModelNormalizationLimits::default())
        else {
            panic!("missing {side} type must refuse the complete binding")
        };
        assert_eq!(refusals.len(), 1, "one missing end, one localized refusal");
        assert_eq!(refusals[0].code, Code::MissingDeclaration);
        assert!(
            matches!(&refusals[0].cause, ModelRefusalCause::UnknownRelationshipEnd { relationship, missing, member: actual }
            if relationship == &key("Source/links") && missing == &key("Missing") && actual.to_string() == member)
        );
    }
}
