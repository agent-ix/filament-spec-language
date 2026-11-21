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
            .clause_name()
            .expect("the exact unique owning declaration");
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
    let members = projected
        .members_with_budget(&mut budget)
        .collect::<Result<Vec<_>, _>>()
        .expect("metered member attributions");
    assert_eq!(members.len(), 8);
    assert!(budget.consumed().work > declaration_charge.work);
    assert!(budget.consumed().storage > declaration_charge.storage);
    for member in members {
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
    }
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

    for (work, storage, kind, code, bound) in [
        (
            u64::try_from(expected_work - 1).expect("short work"),
            1_000_000,
            NativeJoinChargeKind::Work,
            NativeRowErrorCode::WorkLimit,
            expected_work - 1,
        ),
        (
            1_000_000,
            u64::try_from(expected_storage - 1).expect("short storage"),
            NativeJoinChargeKind::Storage,
            NativeRowErrorCode::StorageLimit,
            expected_storage - 1,
        ),
    ] {
        let mut short = NativeJoinBudget::new(work, storage);
        let Some(Err(CompositionViewRefusal::Native(refusal))) =
            view.members_with_budget(&mut short).next()
        else {
            panic!("the exact attribution refuses one unpaid {kind:?} unit");
        };
        assert_eq!(refusal.code(), code);
        assert_eq!(
            refusal.configured_bound(),
            Some(u64::try_from(bound).expect("bound"))
        );
        assert_eq!(refusal.denied(), short.first_unpaid());
        assert_eq!(refusal.denied().expect("unpaid charge").kind, kind);
        assert_eq!(refusal.locus(), short.first_unpaid_locus());
    }

    let mut no_reference_storage = NativeJoinBudget::new(1_000_000, 0);
    let Err(CompositionViewRefusal::Native(reference_denial)) =
        view.component_subject_with_budget("Producer", &mut no_reference_storage)
    else {
        panic!("a resolved held reference requires paid copied storage");
    };
    assert_eq!(reference_denial.code(), NativeRowErrorCode::StorageLimit);
    assert_eq!(
        reference_denial.denied(),
        no_reference_storage.first_unpaid()
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

/// A composition NodeRef must not choose the first naming occurrence when
/// two real dependency declarations share its semantic clause node.
#[trace("FR-530-AC-1", "FR-940-AC-1", "FSpec-FR-491-AC-6")]
#[test]
fn composition_view_refuses_an_ambiguous_owning_clause_occurrence() {
    let producer = dependency("test/producer", "temporal First using t over (p: M::Producer) clock \"model-steps\" on origin { always holds(p.sent_count >= 0) }\n temporal Second using t over (p: M::Producer) clock \"model-steps\" on origin { always holds(p.sent_count >= 0) }\n");
    let consumer = dependency("test/consumer", CONSUMER);
    let source = unit(&format!("compose Sys rule interface {{ {COMPONENTS} guarantee Producer::Second; guarantee Consumer::Done; }}"));
    let package = compile_with(&source, &[&producer, &consumer]).expect("the actual source checks");
    let emission = emit_checked(&package).expect("the actual source emits");
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models =
        fsl_semantics::model::intake::package_input([document.as_slice()]).expect("model input");
    let read = crate::checked_v2::read_evaluation_emission(
        &package,
        &emission,
        library(),
        &models,
        crate::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::SourceWitnessSupplier::new(&emission),
    )
    .expect("the genuine composition with its dependency packages is admitted");
    let found = read.claim_name("Sys").expect("admitted Sys name");
    let view = read
        .composition(&found.declaration)
        .expect("composition view");
    let member = view.members().map(|member| member.expect("actual attribution"))
        .find(|member| matches!(member.value(), CompositionMemberValue::Guarantee(clause) if clause.component.as_ref() == "Producer"))
        .expect("the Producer guarantee");
    let CompositionMemberValue::Guarantee(clause) = member.value() else {
        panic!("guarantee");
    };
    let mut budget = filament_contract_model::NativeJoinBudget::new(1_000_000, 1_000_000);
    for refusal in [
        view.clause(clause),
        view.clause_with_budget(clause, &mut budget),
    ] {
        let Err(CompositionViewRefusal::AmbiguousClause {
            component,
            clause: target,
        }) = refusal
        else {
            panic!("the target requires a claim occurrence instead of first-row selection");
        };
        assert_eq!(component, "Producer");
        let owner = read
            .admitted()
            .direct_dependencies()
            .find(|owner| owner.name() == "test/producer")
            .expect("authentic owner");
        let key = owner
            .structural_key(&target)
            .expect("authentic owner-held target");
        let mut names: Vec<_> = owner
            .temporal_clause_names()
            .expect("own naming rows")
            .iter()
            .filter(|row| owner.structural_key(&row.declaration) == Some(key))
            .map(|row| (row.occurrence, row.name.as_ref()))
            .collect();
        names.sort();
        assert_eq!(names, [(0, "First"), (1, "Second")]);
    }
}

/// Model-operation fairness survives real source emission/read-back with
/// exact effective identities, first occurrence order and finite ledgers.
#[trace("FR-940-AC-1", "FR-530-AC-1", "FSpec-FR-491-AC-6")]
#[test]
fn fair_clause_readback_preserves_constraints_and_key_fairness() {
    use crate::{BareFairnessRefusal, BareSupply, NativeRowErrorCode};
    use filament_contract_model::NativeJoinBudget;
    let dependency = dependency("test/producer", "temporal Fair using t over (p: M::Producer) clock \"model-steps\" on origin { fair strong whole M::Producer::send; fair weak each M::Producer::send; fair strong each M::Producer::send; fair weak whole M::Producer::send; fair strong whole M::Producer::send; always eventually holds(p.sent) }\ntemporal Calm using t over (p: M::Producer) clock \"model-steps\" on origin { always holds(p.sent) }\n");
    let emission = emit_checked(&dependency.package).expect("the actual source emits");
    let document = serde_json::to_vec(&fixture::document()).expect("model document");
    let models =
        fsl_semantics::model::intake::package_input([document.as_slice()]).expect("model input");
    let read = crate::checked_v2::read_evaluation_emission(
        &dependency.package,
        &emission,
        dependency.identity.clone(),
        &models,
        crate::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
        crate::SourceWitnessSupplier::new(&emission),
    )
    .expect("the actual fair clause is strictly admitted");
    let mut budget = NativeJoinBudget::new(1_000_000, 1_000_000);
    let rows = emission
        .identity_rows(read.admitted(), &mut budget)
        .expect("authentic identity rows");
    let bare = BarePackage::new(read.admitted())
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut budget,
        )
        .expect("the original caller budget joins the owning identity rows");
    let named = read
        .claim_name("Fair")
        .expect("the actual fair clause name");
    let source = dependency
        .package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.clause().name() == "Fair")
        .expect("source clause");
    let claim = fsl_foundation::source::provenance::OccurrenceKey::new(
        source.identity(),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), named.occurrence.ordinal),
    );
    // The production metadata seam indexes this authentic owner once and
    // expands only the requested clause through the existing formula walker.
    let body_projection = |budget: &mut NativeJoinBudget| {
        let bodies = crate::checked_views::ClauseBodies::new(read.admitted(), budget)?;
        bodies.body(&named.declaration, budget)
    };
    let before_bodies = budget.consumed();
    let body = body_projection(&mut budget).expect("charged owner body");
    let after_bodies = budget.consumed();
    let body_work = after_bodies.work - before_bodies.work;
    let body_storage = after_bodies.storage - before_bodies.storage;
    let old_view = crate::checked_views::temporal_subject(read.admitted(), &named.declaration)
        .expect("legacy subject");
    assert_eq!(body.formula, old_view.formula_graph());
    assert_eq!(body.over, old_view.over());
    assert_eq!(body.profile, old_view.profile());
    assert_eq!(&body.activation, old_view.activation());
    assert_eq!(body.captures, old_view.captures());
    assert_eq!(body.capture_names, old_view.capture_names());
    let mut exact_body_budget = NativeJoinBudget::new(body_work, body_storage);
    assert_eq!(
        body_projection(&mut exact_body_budget)
            .expect("exact body allowance")
            .formula,
        body.formula
    );
    for (work, storage, kind) in [
        (
            body_work - 1,
            1_000_000,
            filament_contract_model::NativeJoinChargeKind::Work,
        ),
        (
            1_000_000,
            body_storage - 1,
            filament_contract_model::NativeJoinChargeKind::Storage,
        ),
    ] {
        let mut short = NativeJoinBudget::new(work, storage);
        let Err(crate::checked_views::ClauseBodyRefusal::Native(error)) =
            body_projection(&mut short)
        else {
            panic!("the original first unpaid body charge remains typed");
        };
        assert_eq!(error.denied(), short.first_unpaid());
        assert_eq!(error.denied().expect("unpaid").kind, kind);
        assert_eq!(error.locus(), short.first_unpaid_locus());
        assert_eq!(
            error.configured_bound(),
            Some(match kind {
                filament_contract_model::NativeJoinChargeKind::Work => work,
                filament_contract_model::NativeJoinChargeKind::Storage => storage,
            })
        );
    }
    let calm = read.claim_name("Calm").expect("a second genuine clause");
    let mut batch_budget = NativeJoinBudget::new(1_000_000, 1_000_000);
    let bodies = crate::checked_views::ClauseBodies::new(read.admitted(), &mut batch_budget)
        .expect("one owner index");
    let setup = batch_budget.consumed();
    assert_eq!(
        bodies
            .node(&named.declaration)
            .expect("document-held lookup")
            .node_id,
        named.declaration
    );
    let first = bodies
        .body(&named.declaration, &mut batch_budget)
        .expect("first body");
    let first_paid = batch_budget.consumed();
    assert_eq!(first_paid.work - setup.work, body_work - setup.work);
    assert_eq!(
        first_paid.storage - setup.storage,
        body_storage - setup.storage
    );
    let second = bodies
        .body(&calm.declaration, &mut batch_budget)
        .expect("second body reuses the owner index");
    let batch_paid = batch_budget.consumed();
    let mut separate_budget = NativeJoinBudget::new(1_000_000, 1_000_000);
    let separate = crate::checked_views::ClauseBodies::new(read.admitted(), &mut separate_budget)
        .expect("separate owner index")
        .body(&calm.declaration, &mut separate_budget)
        .expect("same second body");
    assert_eq!(first.formula, body.formula);
    assert_eq!(second.formula, separate.formula);
    assert_eq!(
        batch_paid.work - first_paid.work,
        separate_budget.consumed().work - setup.work
    );
    assert_eq!(
        batch_paid.storage - first_paid.storage,
        separate_budget.consumed().storage - setup.storage
    );
    let before = budget.consumed();
    let fairness = bare
        .temporal_clause_fairness_with_budget(&named.declaration, &mut budget)
        .expect("the admitted operation fairness projects");
    let consumed = budget.consumed();
    let (work, storage) = (
        consumed.work - before.work,
        consumed.storage - before.storage,
    );
    assert!(work > 0 && storage > 0);
    assert_eq!(fairness, source.clause().fairness());
    assert_eq!(
        fairness
            .iter()
            .map(|constraint| (constraint.kind(), constraint.granularity()))
            .collect::<Vec<_>>(),
        [
            (FairnessKind::Strong, FairnessGranularity::Whole),
            (FairnessKind::Weak, FairnessGranularity::Each),
            (FairnessKind::Strong, FairnessGranularity::Each),
            (FairnessKind::Weak, FairnessGranularity::Whole),
        ],
        "first authored occurrence is retained; duplicate constraints are not reintroduced"
    );
    let canonical = |constraints: &[filament_semantic_value::fairness::FairnessConstraint]| {
        constraints
            .iter()
            .map(|constraint| {
                quire_canonical::to_vec(constraint, quire_canonical::Limits::new(1_000_000))
                    .expect("canonical fairness key member")
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(canonical(&fairness), canonical(source.clause().fairness()));
    let form = bare
        .temporal_clause_form(&claim)
        .expect("a fair clause is no longer a Pending fairness form");
    assert_eq!(form.fairness, fairness);
    let names = read
        .admitted()
        .temporal_clause_names()
        .expect("owning temporal names");
    let fair_row = names
        .iter()
        .find(|row| row.name.as_ref() == "Fair")
        .expect("authentic Fair row");
    let calm_row = names
        .iter()
        .find(|row| row.name.as_ref() == "Calm")
        .expect("authentic Calm row");
    let before_forms = budget.consumed();
    let forms = bare
        .temporal_clause_forms_with_budget(&[fair_row, calm_row], &mut budget)
        .expect("one charged owner batch creates actual package forms");
    let after_forms = budget.consumed();
    let form_work = after_forms.work - before_forms.work;
    let form_storage = after_forms.storage - before_forms.storage;
    assert_eq!(forms.len(), 2);
    let actual = &forms[0];
    assert_eq!(actual.name, form.name);
    assert_eq!(actual.span, form.span);
    assert_eq!(actual.profile, form.profile);
    assert_eq!(actual.package_name, form.package_name);
    assert_eq!(actual.over, form.over);
    assert_eq!(actual.clock, form.clock);
    assert_eq!(actual.activation, form.activation);
    assert_eq!(actual.trigger, form.trigger);
    assert_eq!(actual.captures, form.captures);
    assert_eq!(actual.fairness, form.fairness);
    assert_eq!(
        format!("{:?}", actual.formula),
        format!("{:?}", form.formula)
    );
    assert_eq!(forms[1].name, "Calm");
    assert!(forms[1].fairness.is_empty());
    let mut exact_forms = NativeJoinBudget::new(form_work, form_storage);
    assert_eq!(
        bare.temporal_clause_forms_with_budget(&[fair_row, calm_row], &mut exact_forms)
            .expect("exact batch form allowance")
            .len(),
        2
    );
    assert_eq!(exact_forms.consumed().work, form_work);
    assert_eq!(exact_forms.consumed().storage, form_storage);
    for (work, storage, kind) in [
        (
            form_work - 1,
            1_000_000,
            filament_contract_model::NativeJoinChargeKind::Work,
        ),
        (
            1_000_000,
            form_storage - 1,
            filament_contract_model::NativeJoinChargeKind::Storage,
        ),
    ] {
        let mut short = NativeJoinBudget::new(work, storage);
        let Err(crate::BareClauseFormRefusal::Native(error)) =
            bare.temporal_clause_forms_with_budget(&[fair_row, calm_row], &mut short)
        else {
            panic!("full batch projection keeps native denial typed");
        };
        assert_eq!(error.denied(), short.first_unpaid());
        assert_eq!(error.denied().expect("unpaid").kind, kind);
        assert_eq!(error.locus(), short.first_unpaid_locus());
        assert_eq!(
            error.configured_bound(),
            Some(match kind {
                filament_contract_model::NativeJoinChargeKind::Work => work,
                filament_contract_model::NativeJoinChargeKind::Storage => storage,
            })
        );
    }
    let copied_row = fair_row.clone();
    let mut ample = NativeJoinBudget::new(1_000_000, 1_000_000);
    assert!(matches!(
        bare.temporal_clause_forms_with_budget(&[&copied_row], &mut ample),
        Err(crate::BareClauseFormRefusal::View(
            crate::checked_views::ViewRefusal::UnresolvedClause
        ))
    ));
    let world = bare.world().expect("owning world");
    let mut bounded = form.clone();
    bounded.profile = fsl_semantics::check::TemporalProfile::EventPositionFalseExtension;
    assert!(
        matches!(
            fsl_semantics::check::CheckedTemporalClause::of_package_run_form(
                bounded,
                world.types(),
                1_000_000
            ),
            Err(fsl_semantics::check::PackageClauseRefusal::Profile(
                fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
            ))
        ),
        "read-back does not admit operation fairness under a bounded profile"
    );
    let rebuilt = fsl_semantics::check::CheckedTemporalClause::of_package_form(
        form,
        world.types(),
        1_000_000,
    )
    .expect("the checked package clause retains its actual fairness");
    assert_eq!(rebuilt.fairness(), source.clause().fairness());
    assert_eq!(rebuilt.property_form(), source.clause().property_form());
    assert_eq!(
        rebuilt
            .claim()
            .requirements_over(fsl_semantics::family::ClaimExtent::Bounded),
        source
            .clause()
            .claim()
            .requirements_over(fsl_semantics::family::ClaimExtent::Bounded)
    );
    let paid = budget.consumed();
    let mut exact = NativeJoinBudget::new(work, storage);
    assert_eq!(
        bare.temporal_clause_fairness_with_budget(&named.declaration, &mut exact)
            .expect("the exact paid allowance"),
        fairness
    );
    assert_eq!(exact.consumed().work, work);
    assert_eq!(exact.consumed().storage, storage);
    for (work, storage, code) in [
        (0, 1_000_000, NativeRowErrorCode::WorkLimit),
        (1_000_000, 0, NativeRowErrorCode::StorageLimit),
        (work - 1, 1_000_000, NativeRowErrorCode::WorkLimit),
        (1_000_000, storage - 1, NativeRowErrorCode::StorageLimit),
    ] {
        let mut denied = NativeJoinBudget::new(work, storage);
        let Err(BareFairnessRefusal::Native(refusal)) =
            bare.temporal_clause_fairness_with_budget(&named.declaration, &mut denied)
        else {
            panic!("the fairness read refuses unpaid work/storage before copying");
        };
        assert_eq!(refusal.code(), code);
        assert_eq!(refusal.denied(), denied.first_unpaid());
        let consumed = denied.consumed();
        assert!(
            matches!(bare.temporal_clause_fairness_with_budget(&named.declaration, &mut denied), Err(BareFairnessRefusal::Native(error)) if error == refusal)
        );
        assert_eq!(denied.consumed(), consumed);
    }
    assert_eq!(
        budget.consumed(),
        paid,
        "negative controls never replace the invocation's paid ledger"
    );
}
