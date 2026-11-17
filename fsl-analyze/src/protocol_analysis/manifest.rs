// SPDX-License-Identifier: AGPL-3.0-or-later
//! The protocol analysis engine's FR-331 provider manifest (ADR-036 PA-4).
//!
//! The engine advertises `local-projection` and `realizability`, each in
//! the `unbounded` mode, and `finite-replay` and `global-conformance` in the
//! `bounded` mode. A supplied finite trace is replayed step by step. A `bounded-static` protocol has static roles,
//! literal capacities and no loop, so the engine decides it without
//! substituting a bound for any domain, and the horizon `max_depth` is a key
//! member whose cut settles `inconclusive`, never a domain bound. It names
//! the `population` domain, which bounds the subject a supplied trace replays
//! over.
//! FR-290's evidence kinds belong to `probabilistic-satisfaction` alone, so
//! it advertises none, and it adds no `NegotiationCause` (FR-057-AC-15).

use fsl_foundation::bound::DomainKind;
use fsl_route::{
    descriptor_from_manifest, BackendDescriptor, Candidate, ManifestMembers, Mode, ProviderOrigin,
    RegistrationRefusal,
};
use fsl_semantics::check::Capability;

/// The engine's backend identity, the identity its terminal records name.
pub const BACKEND_ID: &str = "protocol-analysis";

/// The boundable domains a supplied trace's replay is bounded by: the
/// populations of the subject's universes.
pub const DOMAINS: [&str; 1] = [DomainKind::Population.to_wire()];

/// The (kind, mode) pairs the engine advertises.
pub const ADVERTISED: [(Option<&str>, Option<&str>); 4] = [
    (
        Some(Capability::FiniteReplay.to_wire()),
        Some(Mode::Bounded.to_wire()),
    ),
    (
        Some(Capability::LocalProjection.to_wire()),
        Some(Mode::Unbounded.to_wire()),
    ),
    (
        Some(Capability::Realizability.to_wire()),
        Some(Mode::Unbounded.to_wire()),
    ),
    (
        Some(Capability::GlobalConformance.to_wire()),
        Some(Mode::Bounded.to_wire()),
    ),
];

/// The engine's routed descriptor, converted from [`ADVERTISED`] by FR-288's
/// conversion. `origin` is the host's fact (ADR-029 PV-1).
///
/// # Errors
///
/// Only if the registry's admission rules refuse this module's own data,
/// which the serving test of TC-155 rules out.
pub fn descriptor(origin: ProviderOrigin) -> Result<BackendDescriptor, RegistrationRefusal> {
    descriptor_from_manifest(
        Candidate::new(fsl_foundation::BackendId::new(BACKEND_ID)),
        origin,
        ADVERTISED,
        ManifestMembers {
            domains: Some(&DOMAINS),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
}
