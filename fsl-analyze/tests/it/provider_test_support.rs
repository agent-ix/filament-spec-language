// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test doubles for provider admission and handoff tests.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use filament_exact::Cancel;
use fsl_analyze::provider::{ExternalProvider, ProviderRequest, ProviderVerdict};

/// Counts invocations of an external provider that answers `Unknown`.
#[derive(Debug, Clone, Default)]
pub(crate) struct CountingUnknownProvider {
    calls: Arc<AtomicUsize>,
}

impl CountingUnknownProvider {
    /// A provider and its shared invocation counter.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The number of requests the provider has received.
    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl ExternalProvider for CountingUnknownProvider {
    fn settle(&self, request: &ProviderRequest<'_>, _cancel: &Cancel) -> Vec<ProviderVerdict> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        vec![ProviderVerdict::Unknown; request.items().len()]
    }
}
