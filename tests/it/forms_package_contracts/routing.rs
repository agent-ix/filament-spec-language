// SPDX-License-Identifier: AGPL-3.0-or-later
//! Registration conflicts and exact-evidence candidate selection.

use filament_semantic_value::catalog::CatalogCode;
use fsl_foundation::BackendId;
use fsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, CandidateOutcome, ItemEvidence,
    ManifestMembers, ProviderOrigin, RegistrationCause, Registry,
};
use fsl_semantics::check::Capability;
use ix_trace_rs::trace;

fn descriptor(
    id: &str,
    kind: &str,
    evidence: Option<&str>,
    origin: ProviderOrigin,
) -> BackendDescriptor {
    let evidence_labels = evidence.map(|label| [label]);
    descriptor_from_manifest(
        Candidate::new(BackendId::new(id)),
        origin,
        [(Some(kind), Some("unbounded"))],
        ManifestMembers {
            evidence_kinds: evidence_labels.as_ref().map(|labels| labels.as_slice()),
            ..ManifestMembers::default()
        },
    )
    .expect("all fixture labels are exact supported manifest labels")
}

fn ids(outcome: CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("an unnamed request always computes a candidate set")
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}

#[trace(
    "FR-075-AC-1",
    "FR-075-AC-2",
    "FR-075-AC-3",
    "FR-075-AC-4",
    "TC-193",
    "TC-194",
    "TC-195",
    "TC-196"
)]
#[test]
fn reordered_registrations_keep_exact_evidence_and_permanently_withdraw_conflicts() {
    let live = descriptor("live", "value-validity", None, ProviderOrigin::Linked);
    let exact = descriptor(
        "exact",
        "probabilistic-satisfaction",
        Some("exact"),
        ProviderOrigin::Linked,
    );
    let statistical = descriptor(
        "statistical",
        "probabilistic-satisfaction",
        Some("statistical"),
        ProviderOrigin::Process,
    );
    let conflict = descriptor("poisoned", "value-validity", None, ProviderOrigin::Linked);
    let different = descriptor("poisoned", "value-validity", None, ProviderOrigin::Process);
    let descriptors = [
        live.clone(),
        exact,
        statistical,
        conflict.clone(),
        different,
        live,
        descriptor("alt", "value-validity", None, ProviderOrigin::Process),
    ];
    let mut registries = Vec::new();
    for order in [
        [0, 1, 2, 3, 4, 5, 6],
        [6, 5, 4, 3, 2, 1, 0],
        [3, 0, 4, 6, 5, 1, 2],
    ] {
        let mut registry = Registry::new();
        for index in order {
            if let Err(refusal) = registry.register(descriptors[index].clone()) {
                assert_eq!(refusal.backend().id(), &BackendId::new("poisoned"));
                assert_eq!(refusal.cause(), &RegistrationCause::DuplicateBackend);
            }
        }
        let snapshot = registry.clone();
        let refusal = registry
            .register(conflict.clone())
            .expect_err("a withdrawn identity cannot be revived");
        assert_eq!(refusal.cause(), &RegistrationCause::DuplicateBackend);
        assert_eq!(
            registry, snapshot,
            "a rejected revival changes no live descriptor"
        );
        let refusals = registry.refusals().collect::<Vec<_>>();
        assert_eq!(refusals.len(), 1, "one refusal per conflicted identity");
        assert_eq!(refusals[0].backend().id(), &BackendId::new("poisoned"));
        assert_eq!(refusals[0].cause(), &RegistrationCause::DuplicateBackend);
        assert_eq!(
            ids(registry.candidates(Capability::ValueValidity, None)),
            ["alt", "live"]
        );
        assert_eq!(
            ids(registry.candidates_for_evidence(
                Capability::ProbabilisticSatisfaction,
                &ItemEvidence::from_label(Some("exact")),
                None
            )),
            ["exact"]
        );
        assert_eq!(
            ids(registry.candidates_for_evidence(
                Capability::ProbabilisticSatisfaction,
                &ItemEvidence::from_label(Some("statistical")),
                None
            )),
            ["statistical"]
        );
        assert_eq!(
            ids(registry.candidates(Capability::Refinement, None)),
            Vec::<String>::new()
        );
        for name in ["poisoned", "never-registered"] {
            assert_eq!(
                registry.candidates(Capability::ValueValidity, Some(&BackendId::new(name))),
                CandidateOutcome::UnknownBackend(BackendId::new(name))
            );
        }
        assert_eq!(
            ids(registry.candidates(Capability::ValueValidity, Some(&BackendId::new("exact")))),
            Vec::<String>::new(),
            "a registered wrong-kind provider is empty rather than unknown"
        );
        registries.push(registry);
    }
    assert!(
        registries.windows(2).all(|pair| pair[0] == pair[1]),
        "registration order changes neither held descriptors nor persistent conflicts"
    );
}

#[trace("FR-057-AC-8", "TC-155")]
#[test]
fn manifest_admission_refuses_exact_bad_kind_and_mode_without_replacement() {
    let backend = Candidate::new(BackendId::new("source-provider"));
    let pairs = [
        (None, Some("unbounded"), RegistrationCause::AbsentKind),
        (
            Some(""),
            Some("unbounded"),
            RegistrationCause::UnknownKind("".to_owned()),
        ),
        (
            Some("ValueValidity"),
            Some("unbounded"),
            RegistrationCause::UnknownKind("ValueValidity".to_owned()),
        ),
        (
            Some("value_validity"),
            Some("unbounded"),
            RegistrationCause::UnknownKind("value_validity".to_owned()),
        ),
        (
            Some(" value-validity"),
            Some("unbounded"),
            RegistrationCause::UnknownKind(" value-validity".to_owned()),
        ),
        (
            Some("FamilyCheck"),
            Some("unbounded"),
            RegistrationCause::UnknownKind("FamilyCheck".to_owned()),
        ),
        (
            Some("value-validity"),
            None,
            RegistrationCause::UnknownMode(None),
        ),
        (
            Some("value-validity"),
            Some("Unbounded"),
            RegistrationCause::UnknownMode(Some("Unbounded".to_owned())),
        ),
        (
            Some("value-validity"),
            Some("unbounded "),
            RegistrationCause::UnknownMode(Some("unbounded ".to_owned())),
        ),
    ];
    for (kind, mode, expected) in pairs {
        let refusal = descriptor_from_manifest(
            backend.clone(),
            ProviderOrigin::Process,
            [(kind, mode)],
            ManifestMembers::default(),
        )
        .expect_err("invalid manifest pair cannot yield a typed descriptor");
        assert_eq!(refusal.backend(), &backend);
        assert_eq!(refusal.cause(), &expected);
        assert_eq!(
            refusal.catalog_code(),
            CatalogCode::new("invalid_capability", expected.as_str())
        );
    }
    let accepted = descriptor_from_manifest(
        backend.clone(),
        ProviderOrigin::Process,
        [(Some("value-validity"), Some("unbounded"))],
        ManifestMembers::default(),
    )
    .expect("exact counterpart admits");
    let mut registry = Registry::new();
    registry
        .register(accepted)
        .expect("admitted descriptor registers");
    assert_eq!(
        ids(registry.candidates(Capability::ValueValidity, None)),
        ["source-provider"]
    );
}
