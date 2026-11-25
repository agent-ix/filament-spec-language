// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1163 (FR-195-AC-6, FR-075-AC-18): S3 records each probabilistic
//! claim's evidence kind on its requirement record, and FR-075's request
//! builder writes items that name it, the one kind a request selects, or
//! each kind once when it selects both.

use crate::probabilistic_request_support::{candidate_ids, registry};
use std::collections::BTreeMap;

use filament_exact::{Origin, Role};
use fsl_foundation::bound::{DomainKey, DomainKind};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_route::request::{items_with_evidence, EvidenceSelection, RequirementItem};
use fsl_route::EvidenceKind;
use fsl_semantics::check::{Capability, RequirementMap, RequirementRecord};
use fsl_semantics::family::{ClaimEvidence, ClaimExtent, Requirements};
use ix_trace_rs::trace;

use crate::prob_world::{self, World};

const CLAIMS: &str = "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
     with alpha 0.01, beta 0.01, indifference 0.01 \
     { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
     until holds(s.phase = 2) within 10) <= 2 }\n\
     probabilistic Every using ev on M over (s: M::Server) under every scheduler \
     on origin with alpha 0.01, beta 0.01, indifference 0.01 \
     { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic FairTerminates using ev on M over (s: M::Server) under every scheduler \
     on origin { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic Reach using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic Bare using ev on M over (s: M::Server) under Steady on origin \
     { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
     probabilistic Expected using inf on M over (s: M::Server) under Steady on origin \
     { expected accumulate duration until holds(s.phase = 2) <= 5 }";

fn node(fill: u8) -> filament_exact::NodeKey {
    crate::fixture_ids::node_key([fill])
}

fn key(fill: u8) -> OccurrenceKey {
    OccurrenceKey::new(node(fill), Origin::new(Role::new("claim"), 0))
}

/// The record S3 writes for the claim `name` of `world`, keyed by `fill`.
fn record(world: &World, name: &str, fill: u8) -> RequirementRecord {
    RequirementRecord::model_claim(world.claim(name).requirements(node(fill)), node(99))
}

fn evidence_of(record: &RequirementRecord) -> Option<ClaimEvidence> {
    record.requirements().evidence()
}

/// Checks TC-1163 step 1: each probabilistic claim has a
/// `probabilistic-satisfaction` record, unbounded over a domain naming the
/// claim, with a `Boolean` result bound, no conversion context and an empty
/// path condition; `P95` records `statistical`, and a claim under every
/// scheduler, one with no confidence parameters and an expected-reward form
/// record `exact`.
#[trace("TC-1163")]
#[test]
fn each_probabilistic_claim_records_its_evidence_kind() {
    let world = World::new(&prob_world::service(CLAIMS));
    for (name, evidence) in [
        ("P95", ClaimEvidence::Statistical),
        ("Every", ClaimEvidence::Exact),
        ("FairTerminates", ClaimEvidence::Exact),
        ("Reach", ClaimEvidence::Exact),
        ("Bare", ClaimEvidence::Exact),
        ("Expected", ClaimEvidence::Exact),
    ] {
        let record = record(&world, name, 1);
        let requirements = record.requirements();
        assert_eq!(
            requirements.kind(),
            Capability::ProbabilisticSatisfaction,
            "{name}"
        );
        assert_eq!(evidence_of(&record), Some(evidence), "{name}");
        assert_eq!(
            requirements.extent(),
            &ClaimExtent::from_domains(BTreeMap::from([(
                DomainKey::Node {
                    node: node(1),
                    path: Vec::new()
                },
                DomainKind::InfiniteTrace
            )])),
            "{name}"
        );
        assert_eq!(record.result_bound().node(), node(99), "{name}");
        assert!(record.conversion_context().is_none(), "{name}");
        assert!(record.path_condition().is_empty(), "{name}");
    }
}

/// (index, evidence, candidates, occurrence) of each item.
fn summary(
    items: &[RequirementItem],
) -> Vec<(usize, Option<EvidenceKind>, Vec<String>, OccurrenceKey)> {
    items
        .iter()
        .map(|item| {
            (
                item.index().get(),
                item.evidence(),
                candidate_ids(item),
                item.occurrence().clone(),
            )
        })
        .collect()
}

/// Checks TC-1163 step 3 and FR-075-AC-18: P95, FairTerminates, Reach and a
/// value-validity record, in key order, with EN-4 `{statistical}` and EN-5
/// `{exact}` registered. Each evidence selection preserves those occurrences;
/// `both` appends the other evidence item for each probabilistic record.
#[trace("TC-1163", "FR-075-AC-18", "FR-195-AC-6")]
#[test]
fn the_request_builder_names_the_evidence_kind_of_each_probabilistic_item() {
    let world = World::new(&prob_world::service(CLAIMS));
    let records: RequirementMap = [
        (key(1), record(&world, "P95", 1)),
        (key(2), record(&world, "FairTerminates", 2)),
        (key(3), record(&world, "Reach", 3)),
        (
            key(4),
            RequirementRecord::model_claim(
                Requirements::new(Capability::ValueValidity, ClaimExtent::Bounded),
                node(99),
            ),
        ),
    ]
    .into_iter()
    .collect();
    let registry = registry();
    let en4 = vec!["en-4".to_owned()];
    let en5 = vec!["en-5".to_owned()];
    let stat = Some(EvidenceKind::Statistical);
    let exact = Some(EvidenceKind::Exact);
    let record_items = items_with_evidence(&records, &registry, None, EvidenceSelection::Record);
    let exact_items = items_with_evidence(
        &records,
        &registry,
        None,
        EvidenceSelection::One(EvidenceKind::Exact),
    );
    let both_items = items_with_evidence(&records, &registry, None, EvidenceSelection::Both);
    let build = |selection| summary(&items_with_evidence(&records, &registry, None, selection));

    let record_kinds = build(EvidenceSelection::Record);
    assert_eq!(record_kinds.len(), 4);
    assert_eq!(
        record_kinds
            .iter()
            .map(|i| (i.0, i.1, i.2.clone()))
            .collect::<Vec<_>>(),
        [
            (0, stat, en4.clone()),
            (1, exact, en5.clone()),
            (2, exact, en5.clone()),
            (3, None, vec![]),
        ]
    );
    assert_eq!(
        record_kinds[0].3,
        key(1),
        "P95 keeps its occurrence identity"
    );
    assert_eq!(record_kinds[1].3, key(2));
    assert_eq!(record_kinds[2].3, key(3));
    assert_eq!(record_kinds[3].3, key(4));

    let only_exact = build(EvidenceSelection::One(EvidenceKind::Exact));
    assert_eq!(only_exact.len(), 4);
    assert_eq!(
        only_exact
            .iter()
            .map(|i| (i.0, i.1, i.2.clone()))
            .collect::<Vec<_>>(),
        [
            (0, exact, en5.clone()),
            (1, exact, en5.clone()),
            (2, exact, en5.clone()),
            (3, None, vec![]),
        ]
    );
    assert_eq!(only_exact[0].3, key(1), "P95 keeps its occurrence identity");
    assert_eq!(only_exact[1].3, key(2));
    assert_eq!(only_exact[2].3, key(3));
    assert_eq!(only_exact[3].3, key(4));

    let both = build(EvidenceSelection::Both);
    assert_eq!(both.len(), 7);
    assert_eq!(both[..4], record_kinds[..]);
    assert_eq!(
        both[4..]
            .iter()
            .map(|i| (i.0, i.1, i.2.clone()))
            .collect::<Vec<_>>(),
        [
            (4, exact, en5.clone()),
            (5, stat, en4.clone()),
            (6, stat, en4.clone()),
        ]
    );
    assert_eq!(both[4].3, key(1));
    assert_eq!(both[5].3, key(2));
    assert_eq!(both[6].3, key(3));
    assert_eq!(record_kinds[0].3, only_exact[0].3);
    assert_eq!(record_kinds[0].3, both[0].3);
    assert_eq!(record_kinds[0].3, both[4].3);

    for item in [
        &record_items[0],
        &exact_items[0],
        &both_items[0],
        &both_items[4],
    ] {
        assert_eq!(item.kind(), Capability::ProbabilisticSatisfaction);
        assert_eq!(
            item.extent(),
            fsl_route::request::ExtentClassification::Unbounded {
                finite_bound_available: false
            }
        );
        assert_eq!(item.unbounded().len(), 1);
        assert_eq!(item.result_bound(), &node(99));
        assert!(item.conversion_context().is_none());
        assert!(item.path_condition().is_empty());
        assert_eq!(item.occurrence(), &key(1));
    }
}
