// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-core request-file intake and orchestration (FR-027, FR-100, FR-287).
//!
//! Filesystem reads live here, above the qualified core. The driver consumes
//! prepared typed inputs and the core's outcome documents without depending
//! on the FSL root package.

#![forbid(unsafe_code)]

pub mod command;
pub mod formal_source;
pub mod replay_request;
pub mod unique_json;
