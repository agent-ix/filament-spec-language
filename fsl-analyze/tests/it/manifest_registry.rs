// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-288: manifest fields are converted into registry descriptors.

use std::num::NonZeroU64;

use fsl_foundation::bound::DomainKind;
use fsl_foundation::BackendId;
use fsl_route::{
    Candidate, ManifestMembers, MaxCasesMember, Mode, ProviderOrigin, RegistrationCause, Registry,
};
use fsl_semantics::check::Capability;
use ix_trace_rs::trace;

const BACKEND: &str = "vendor.Backend-A_1";

fn descriptor(
    id: &str,
    origin: ProviderOrigin,
    kind: Option<&str>,
    domain: &str,
) -> Result<fsl_route::BackendDescriptor, fsl_route::RegistrationRefusal> {
    fsl_route::descriptor_from_manifest_with_max_cases(
        Candidate::new(BackendId::new(id)),
        origin,
        [(kind, Some(Mode::Bounded.to_wire()))],
        ManifestMembers {
            domains: Some(&[domain]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
        MaxCasesMember::Count(12),
    )
}

#[trace("TC-772", "FR-288-AC-1")]
#[test]
fn one_manifest_converts_to_descriptors_equal_except_for_host_origin() {
    let linked = descriptor(
        BACKEND,
        ProviderOrigin::Linked,
        Some(Capability::ValueValidity.to_wire()),
        DomainKind::Integer.to_wire(),
    )
    .expect("linked manifest fields admit");
    let process = descriptor(
        BACKEND,
        ProviderOrigin::Process,
        Some(Capability::ValueValidity.to_wire()),
        DomainKind::Integer.to_wire(),
    )
    .expect("the same manifest fields admit for a process provider");

    assert_eq!(linked.origin(), ProviderOrigin::Linked);
    assert_eq!(process.origin(), ProviderOrigin::Process);
    assert_eq!(linked.id(), process.id());
    assert_eq!(
        linked.advertises().collect::<Vec<_>>(),
        process.advertises().collect::<Vec<_>>()
    );
    assert_eq!(linked.domains(), process.domains());
    assert_eq!(linked.max_cases(), process.max_cases());
    assert_eq!(linked.fairness_kinds(), process.fairness_kinds());
    assert_eq!(linked.evidence_kinds(), process.evidence_kinds());
    assert_eq!(linked.memory_models(), process.memory_models());
    assert_eq!(linked.reductions(), process.reductions());
    assert_eq!(linked.state_graph_forms(), process.state_graph_forms());
    assert_eq!(linked.options(), process.options());
    assert_eq!(linked.option_defaults(), process.option_defaults());
    assert_eq!(linked.dynamics(), process.dynamics());
    assert_eq!(linked.solver_bounds(), process.solver_bounds());
    assert_eq!(linked.decides_timed(), process.decides_timed());
    assert_eq!(
        linked.decides_task_product(),
        process.decides_task_product()
    );
}

#[trace("TC-772", "FR-288-AC-2")]
#[test]
fn manifest_identity_is_verbatim_and_other_fields_change_descriptor_capabilities() {
    let base = descriptor(
        BACKEND,
        ProviderOrigin::Linked,
        Some(Capability::ValueValidity.to_wire()),
        DomainKind::Integer.to_wire(),
    )
    .expect("base manifest fields admit");
    assert_eq!(base.id().as_str(), BACKEND);

    let changed_kind = descriptor(
        BACKEND,
        ProviderOrigin::Linked,
        Some(Capability::OperationContract.to_wire()),
        DomainKind::Integer.to_wire(),
    )
    .expect("changed advertised kind admits");
    assert_ne!(
        base.advertises().collect::<Vec<_>>(),
        changed_kind.advertises().collect::<Vec<_>>()
    );
    assert_eq!(changed_kind.id().as_str(), BACKEND);

    let changed_domain = descriptor(
        BACKEND,
        ProviderOrigin::Linked,
        Some(Capability::ValueValidity.to_wire()),
        DomainKind::Collection.to_wire(),
    )
    .expect("changed domain admits");
    assert_ne!(base.domains(), changed_domain.domains());
    assert_eq!(changed_domain.id().as_str(), BACKEND);

    let changed_option = base
        .clone()
        .with_options([("mode".to_owned(), "strict".to_owned())]);
    assert_ne!(base.option_defaults(), changed_option.option_defaults());
    assert_eq!(changed_option.id().as_str(), BACKEND);

    let changed_bound = base
        .clone()
        .with_max_cases(NonZeroU64::new(13).expect("positive bound"));
    assert_ne!(base.max_cases(), changed_bound.max_cases());
    assert_eq!(changed_bound.id().as_str(), BACKEND);
}

#[trace("TC-772", "FR-288-AC-3")]
#[test]
fn manifest_without_advertised_kind_is_refused_and_only_valid_descriptor_registers() {
    let refusal = descriptor(
        BACKEND,
        ProviderOrigin::Process,
        None,
        DomainKind::Integer.to_wire(),
    )
    .expect_err("missing advertised kind is refused by the manifest conversion");
    assert_eq!(refusal.cause(), &RegistrationCause::AbsentKind);

    let valid = descriptor(
        "vendor.Valid_2",
        ProviderOrigin::Linked,
        Some(Capability::ValueValidity.to_wire()),
        DomainKind::Integer.to_wire(),
    )
    .expect("valid manifest converts");
    let mut registry = Registry::new();
    registry
        .register(valid)
        .expect("valid descriptor registers");
    assert_eq!(
        registry
            .backends()
            .map(BackendId::as_str)
            .collect::<Vec<_>>(),
        ["vendor.Valid_2"]
    );
}
