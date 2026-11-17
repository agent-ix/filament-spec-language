// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-core request-file intake and orchestration (FR-027, FR-100, FR-287).
//!
//! Filesystem reads live here, above the qualified core. The driver consumes
//! prepared typed inputs and the core's outcome documents without depending
//! on the FSL root package.

#![forbid(unsafe_code)]

pub mod checked_handoff;
pub mod command;
pub mod emit_checked_v2;
pub mod formal_source;
pub mod replay_request;
pub mod sources;
pub mod unique_json;

pub use sources::Sources;

/// Build the renderer's source provision from the admitted sources in an outcome.
pub fn render_sources(sources: &Sources) -> fsl_inspect::RenderSources {
    fsl_inspect::RenderSources::from_sources(sources.as_slice().iter().cloned())
}
