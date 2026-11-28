// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared routing fixture support spans split integration test binaries"
)]
//! Shared EN-1 and EN-6 routing fixture helpers.

use fsl_analyze::model_check::manifest as en1;
use fsl_analyze::zone_check::manifest as en6;
use fsl_route::{CandidateOutcome, ProviderOrigin, Registry};

/// EN-1 and EN-6, each registered from its own manifest.
pub(crate) fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register(en1::descriptor(ProviderOrigin::Linked).expect("EN-1 admits"))
        .expect("registered");
    registry
        .register(en6::descriptor(ProviderOrigin::Linked).expect("EN-6 admits"))
        .expect("registered");
    registry
}

/// The backend identities of a computed candidate set.
pub(crate) fn ids(outcome: &CandidateOutcome) -> Vec<String> {
    let CandidateOutcome::Candidates(set) = outcome else {
        panic!("a computed candidate set: {outcome:?}");
    };
    set.candidates()
        .iter()
        .map(|candidate| candidate.id().as_str().to_owned())
        .collect()
}
