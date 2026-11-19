// SPDX-License-Identifier: AGPL-3.0-or-later
//! The root surfaces filament-protocol uses:
//! complete-V1 editor tooling over `fsl-cst`, and the
//! editor tooling. The language stages live in the `fsl-*` layer crates
//! (ADR-011 §6.1); this crate re-exports none of them.
/// The version of this package as built, which `cargo xtask qualify` reports
/// as the FSL version (FR-567-AC-5).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod complete;
#[cfg(feature = "population-fixtures")]
pub mod population_fixtures;

// The shared integration-test support below names this crate by its package
// name; the alias lets the same file compile as a unit-test module. The module
// is public under `cfg(test)` only because most of its helpers serve the
// integration tests that include the same file, not this crate's unit tests.
#[cfg(test)]
extern crate self as filament_spec_language;
