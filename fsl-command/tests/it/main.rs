// SPDX-License-Identifier: AGPL-3.0-or-later
//! External consumers of the non-core request-file owner.

#[path = "../../../examples/config-version/spine_corpus.rs"]
#[allow(
    missing_docs,
    reason = "the shared example catalog is included by path"
)]
pub mod config_version_spine;
mod integer_division_request;
mod native_source_projection;
mod prepared_command;
mod probabilistic_package_locus;
mod request_overrides;
