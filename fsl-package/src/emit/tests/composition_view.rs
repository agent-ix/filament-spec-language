// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed composition facts through both sealed package views.

use super::*;
use crate::{AnalysisView, BarePackage, CompositionMemberValue, CompositionViewRefusal};

/// Source read-back and independently admitted bytes preserve the declaration,
/// canonical member attribution, authored ordering and foreign target owners.
#[trace("FR-940-AC-1", "FR-530-AC-1", "FSpec-FR-491-AC-6", "FSpec-FR-491-AC-7")]
#[test]
fn composition_views_keep_member_regions_and_owner_bound_targets() {
    let world = World::new();
    let members = format!(
        "{CASE_ONE_MEMBERS}environment Sched by \"scheduler\" fair weak each M::Producer::send;"
    );
    let declaration = world.declare("interface", &members);
    let source = unit(&declaration);
    let package = world
        .compile(&declaration)
        .expect("the actual source checks");
    let emission = emit_checked(&package).expect("the actual package emits");
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the actual model input");
    let read = crate::checked_v2::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &models,
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::checked_v2::SourceWitnessSupplier::new(&emission),
    )
    .expect("strict source read-back with the genuine dependencies");
    let mut evidence = emission.evidence.clone();
    for (identity, document) in &models {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .expect("authentic model evidence");
    }
    for dependency in read.admitted().direct_dependencies() {
        evidence
            .insert_dependency_package(dependency.name(), dependency.clone())
            .expect("the actual admitted dependency");
    }
    let mut wire = raw_wire(&emission);
    wire["source_map"]
        .as_array_mut()
        .expect("source map")
        .reverse();
    let bytes = serde_json::to_vec(&wire).expect("byte artifact");
    let Read::Verified { admitted, .. } = crate::checked_v2::read_v2(
        &bytes,
        library(),
        crate::checked_v2::V2ReadLimits::fixture(),
        &evidence,
    ) else {
        panic!("the independently admitted byte artifact");
    };
    let bare = BarePackage::new(&admitted);
    let selected = read.claim_name("Sys").expect("source read-back names Sys");
    let byte_selected = bare.claim_name("Sys").expect("bytes name Sys");
    let source_view = read
        .composition(&selected.declaration)
        .expect("source view");
    let byte_view = bare
        .composition(&byte_selected.declaration)
        .expect("byte view");
    assert_eq!(source_view.name(), byte_view.name());
    assert_eq!(source_view.name().qualified_name, [Box::<str>::from("Sys")]);
    assert_eq!(source_view.rule(), "interface");
    assert_eq!(source_view.rule(), byte_view.rule());
    assert_eq!(source_view.source(), byte_view.source());
    assert_eq!(
        source_view.source().region.start(),
        u64::try_from(source.find("compose Sys").expect("declaration")).expect("start")
    );
    let source_members = source_view
        .members()
        .collect::<Result<Vec<_>, _>>()
        .expect("source members");
    let byte_members = byte_view
        .members()
        .collect::<Result<Vec<_>, _>>()
        .expect("byte members");
    assert_eq!(source_members.len(), 9);
    let mut authored_guarantees = Vec::new();
    let mut canonical_guarantees = Vec::new();
    for (index, (member, byte_member)) in source_members.iter().zip(&byte_members).enumerate() {
        assert_eq!(member.occurrence().role, CheckedOccurrenceRole::Generated);
        assert_eq!(
            member.occurrence().ordinal,
            u64::try_from(index).expect("ordinal")
        );
        assert_eq!(
            member.source(),
            byte_member.source(),
            "source-map row order is immaterial"
        );
        let region = &member.source().region;
        assert!(region.start() > source_view.source().region.start());
        assert!(region.end() < source_view.source().region.end());
        let (component, byte_component, guarantee) = match (member.value(), byte_member.value()) {
            (CompositionMemberValue::Guarantee(left), CompositionMemberValue::Guarantee(right)) => {
                (&left.component, &right.component, Some((left, right)))
            }
            (CompositionMemberValue::Component(left), CompositionMemberValue::Component(right)) => {
                let target = source_view
                    .component_subject(&left.name)
                    .expect("component subject");
                let byte_target = byte_view
                    .component_subject(&right.name)
                    .expect("byte component subject");
                assert_eq!(target.owner().name(), left.package.as_ref());
                assert_eq!(
                    target.owner().structural_key(target.node()),
                    byte_target.owner().structural_key(byte_target.node())
                );
                assert!(target.owner().term(target.node()).is_some());
                continue;
            }
            (CompositionMemberValue::Rely(left), CompositionMemberValue::Rely(right))
            | (
                CompositionMemberValue::LivenessDependency(left),
                CompositionMemberValue::LivenessDependency(right),
            ) => {
                for (left, right) in [(&left.guarantee, &right.guarantee), (&left.on, &right.on)] {
                    let target = source_view
                        .clause(left)
                        .expect("owner-bound premise clause");
                    let byte_target = byte_view.clause(right).expect("byte premise clause");
                    assert_eq!(
                        target.owner().structural_key(target.node()),
                        byte_target.owner().structural_key(byte_target.node())
                    );
                    assert!(target.owner().term(target.node()).is_some());
                }
                continue;
            }
            (CompositionMemberValue::Supply(left), CompositionMemberValue::Supply(right)) => {
                assert_eq!(left, right);
                continue;
            }
            (
                CompositionMemberValue::Environment(left),
                CompositionMemberValue::Environment(right),
            ) => {
                assert_eq!(left, right);
                continue;
            }
            _ => panic!("the canonical member kinds agree"),
        };
        assert_eq!(component, byte_component);
        let (clause, byte_clause) = guarantee.expect("the guarantee arm");
        let target = source_view
            .clause(clause)
            .expect("authenticated clause target");
        let byte_target = byte_view
            .clause(byte_clause)
            .expect("byte authenticated clause target");
        let expected_owner = match component.as_ref() {
            "Producer" => "test/producer",
            "Consumer" => "test/consumer",
            other => panic!("unexpected component {other}"),
        };
        assert_eq!(target.owner().name(), expected_owner);
        assert_ne!(target.owner().name(), source_view.package().name());
        assert_eq!(
            target.owner().structural_key(target.node()),
            byte_target.owner().structural_key(byte_target.node())
        );
        assert!(target.owner().term(target.node()).is_some());
        let named = target
            .owner()
            .temporal_clause_names()
            .expect("owner names")
            .iter()
            .find(|row| {
                target.owner().structural_key(&row.declaration)
                    == target.owner().structural_key(target.node())
            })
            .expect("the exact owning declaration");
        canonical_guarantees.push(format!("{}::{}", component, named.name));
        authored_guarantees.push((region.start(), format!("{}::{}", component, named.name)));
        let forged = clause.clone();
        assert!(
            matches!(
                source_view.clause(&forged),
                Err(CompositionViewRefusal::UnknownTarget)
            ),
            "a copied foreign operand is not authenticated by the composition body"
        );
    }
    authored_guarantees.sort();
    assert_eq!(
        authored_guarantees
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>(),
        ["Producer::Sends", "Consumer::Done", "Producer::Counted"]
    );
    assert_ne!(
        canonical_guarantees[0], "Producer::Sends",
        "canonical component order differs from authored order"
    );
    assert!(matches!(
        source_view.component_subject("Typo"),
        Err(CompositionViewRefusal::UnknownComponent)
    ));
    let marker = read
        .declared_names()
        .into_iter()
        .find(|(name, _)| name == "marker")
        .expect("local marker")
        .1;
    assert!(matches!(
        read.composition(marker),
        Err(CompositionViewRefusal::NotComposition)
    ));
}

/// Attribution and owner-reference projection continue one finite native
/// ledger, with actual source-map visits and first-unpaid limit refusals.
#[trace("FR-940-AC-1", "FR-531-AC-39", "FSpec-FR-491-AC-7")]
#[test]
fn composition_view_projection_preserves_the_cumulative_native_budget() {
    use crate::NativeRowErrorCode;
    use filament_contract_model::{NativeJoinBudget, NativeJoinChargeKind};

    let world = World::new();
    let package = world.checked("interface", CASE_ONE_MEMBERS);
    let emission = emit_checked(&package).expect("the actual source emits");
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the actual model input");
    let read = crate::checked_v2::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &models,
        crate::checked_v2::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::checked_v2::SourceWitnessSupplier::new(&emission),
    )
    .expect("the actual dependency packages and source read-back are admitted");
    let claim = read.claim_name("Sys").expect("composition name");
    let view = read.composition(&claim.declaration).expect("legacy view");
    let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
    let projected = read
        .composition_with_budget(&claim.declaration, &mut budget)
        .expect("metered declaration");
    let declaration_charge = budget.consumed();
    assert!(declaration_charge.work > 0 && declaration_charge.storage > 0);
    assert_eq!(view.name(), projected.name());
    assert_eq!(view.source(), projected.source());
    let mut visited = 0;
    // Consume one member at a time so reference reads continue the same budget.
    for index in 0..view.members().count() {
        let before = budget.consumed();
        let member = projected
            .members_with_budget(&mut budget)
            .nth(index)
            .expect("the canonical member")
            .expect("metered attribution");
        let after_attribution = budget.consumed();
        assert!(after_attribution.work > before.work);
        assert!(after_attribution.storage > before.storage);
        match member.value() {
            CompositionMemberValue::Component(component) => {
                let target = projected
                    .component_subject_with_budget(&component.name, &mut budget)
                    .expect("metered owner-bound subject");
                assert!(target.owner().term(target.node()).is_some());
            }
            CompositionMemberValue::Guarantee(clause) => {
                let target = projected
                    .clause_with_budget(clause, &mut budget)
                    .expect("metered owner-bound clause");
                assert!(target.owner().term(target.node()).is_some());
            }
            CompositionMemberValue::Rely(premise)
            | CompositionMemberValue::LivenessDependency(premise) => {
                for clause in [&premise.guarantee, &premise.on] {
                    let target = projected
                        .clause_with_budget(clause, &mut budget)
                        .expect("metered owner-bound premise");
                    assert!(target.owner().term(target.node()).is_some());
                }
            }
            CompositionMemberValue::Supply(_) | CompositionMemberValue::Environment(_) => {}
        }
        visited += 1;
    }
    assert_eq!(visited, 8);
    assert!(budget.consumed().work > declaration_charge.work);
    assert!(budget.consumed().storage > declaration_charge.storage);

    let first = view
        .members()
        .next()
        .expect("first member")
        .expect("attribution");
    let row_visits = read
        .admitted()
        .source_map()
        .iter()
        .position(|entry| {
            entry.node_id == *view.node()
                && entry.role == first.occurrence().role
                && entry.ordinal == first.occurrence().ordinal
        })
        .expect("the exact admitted member row")
        + 1;
    let region = &first.source().region;
    let expected_work = 1
        + ("generated".len() + 1)
        + row_visits
        + 1
        + 2 * (region.source().authority().len() + 1)
        + 2 * (region.source().identity().len() + 1);
    let expected_storage = ("generated".len() + 1)
        + 2
        + 2 * (region.source().authority().len() + 1)
        + 2 * (region.source().identity().len() + 1);
    let mut exact = NativeJoinBudget::new(
        u64::try_from(expected_work).expect("work"),
        u64::try_from(expected_storage).expect("storage"),
    );
    let actual = view
        .members_with_budget(&mut exact)
        .next()
        .expect("first member")
        .expect("exact work and storage bounds admit the attribution");
    assert_eq!(actual.source(), first.source());
    assert_eq!(
        exact.consumed().work,
        u64::try_from(expected_work).expect("work")
    );
    assert_eq!(
        exact.consumed().storage,
        u64::try_from(expected_storage).expect("storage")
    );

    for (work, storage, kind, code) in [
        (
            0,
            1_000_000,
            NativeJoinChargeKind::Work,
            NativeRowErrorCode::WorkLimit,
        ),
        (
            1_000_000,
            0,
            NativeJoinChargeKind::Storage,
            NativeRowErrorCode::StorageLimit,
        ),
    ] {
        let mut denied = NativeJoinBudget::new(work, storage);
        let Err(CompositionViewRefusal::Native(refusal)) =
            read.composition_with_budget(&claim.declaration, &mut denied)
        else {
            panic!("projection preserves the native {kind:?} denial");
        };
        assert_eq!(refusal.code(), code);
        assert_eq!(refusal.configured_bound(), Some(0));
        assert_eq!(refusal.denied(), denied.first_unpaid());
        assert_eq!(refusal.denied().expect("first unpaid").kind, kind);
        assert_eq!(refusal.locus(), denied.first_unpaid_locus());
        let consumed = denied.consumed();
        let first_unpaid = denied.first_unpaid();
        let Some(Err(CompositionViewRefusal::Native(repeated))) =
            view.members_with_budget(&mut denied).next()
        else {
            panic!("a later attribution preserves the original denial");
        };
        assert_eq!(repeated, refusal);
        assert!(
            matches!(view.component_subject_with_budget("Producer", &mut denied), Err(CompositionViewRefusal::Native(error)) if error == refusal)
        );
        assert_eq!(denied.first_unpaid(), first_unpaid);
        assert_eq!(
            denied.consumed(),
            consumed,
            "a denied phase performs no later charge or copy"
        );
    }
}
