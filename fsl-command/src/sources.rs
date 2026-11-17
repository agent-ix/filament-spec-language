// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admitted source values retained by command outcomes (FR-865).

use fsl_foundation::Source;

/// Source text and paths admitted while producing a command outcome.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    sources: Vec<Source>,
}

impl Sources {
    /// Retain admitted sources in intake order.
    pub fn from_sources(sources: impl IntoIterator<Item = Source>) -> Self {
        Self {
            sources: sources.into_iter().collect(),
        }
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
