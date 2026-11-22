// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admitted source values retained by command outcomes (FR-865).

use fsl_foundation::Source;
use std::sync::Arc;

/// Source text and paths admitted while producing a command outcome.
#[derive(Clone, Default)]
pub struct Sources {
    sources: Vec<Source>,
    source_map: Option<Arc<fsl_foundation::source::provenance::PackageSourceMap>>,
}

impl std::fmt::Debug for Sources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field("source_count", &self.sources.len())
            .field("has_source_map", &self.source_map.is_some())
            .finish()
    }
}

impl Sources {
    /// Retain admitted sources in intake order.
    pub fn from_sources(sources: impl IntoIterator<Item = Source>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
            source_map: None,
        }
    }

    pub(crate) fn with_source_map(
        mut self,
        source_map: Option<Arc<fsl_foundation::source::provenance::PackageSourceMap>>,
    ) -> Self {
        self.source_map = source_map;
        self
    }

    pub(crate) fn source_map_shared(
        &self,
    ) -> Option<Arc<fsl_foundation::source::provenance::PackageSourceMap>> {
        self.source_map.as_ref().map(Arc::clone)
    }

    /// Admitted source values, including their paths and source text.
    pub fn as_slice(&self) -> &[Source] {
        &self.sources
    }

    /// Number of admitted sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether the outcome admitted no source values.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}
