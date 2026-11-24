// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared statistical request helpers for analyzer integration targets.

use fsl_analyze::statistical::manifest;
use fsl_foundation::bound::DomainKind;
use fsl_route::request::RequirementItem;
use fsl_route::{
    Candidate, CandidateOutcome, EvidenceKind, ManifestMembers, Mode, ProviderOrigin, Registry,
    descriptor_from_manifest,
};
use fsl_semantics::check::Capability;

fn exact_backend() -> fsl_route::BackendDescriptor {
    descriptor_from_manifest(
        Candidate::new(fsl_foundation::BackendId::new("en-5")),
        ProviderOrigin::Linked,
        [(
            Some(Capability::ProbabilisticSatisfaction.to_wire()),
            Some(Mode::Bounded.to_wire()),
        )],
        ManifestMembers {
            domains: Some(&[DomainKind::Population.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: Some(&[EvidenceKind::Exact.to_wire()]),
            memory_models: None,
            reductions: None,
        },
    )
    .expect("a well-formed exact backend")
}

pub(crate) fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(manifest::descriptor(ProviderOrigin::Linked).expect("EN-4 is admitted"))
        .expect("EN-4 registers");
    registry.register(exact_backend()).expect("EN-5 registers");
    registry
}

pub(crate) fn candidate_ids(item: &RequirementItem) -> Vec<String> {
    match item.candidates() {
        CandidateOutcome::Candidates(set) => set
            .candidates()
            .iter()
            .map(|candidate| candidate.id().as_str().to_owned())
            .collect(),
        CandidateOutcome::UnknownBackend(id) => panic!("unknown backend {id}"),
    }
}
