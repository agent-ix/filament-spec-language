// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cases of the standard's source fixtures that the compiler does not yet
//! answer as the fixture says (QSL-2048). Each entry keys one case to an
//! open Linear ticket; keys only, no hashes. An unkeyed mismatch fails the
//! run, and so does a keyed case that starts to match.

/// One keyed case.
pub(crate) struct Pending {
    /// The runner: `finite-window` or `state-core`.
    pub runner: &'static str,
    /// The case id in the fixture (state-core ids are `file/id`).
    pub case: &'static str,
    /// The open ticket that owns the gap.
    pub ticket: &'static str,
}

/// The checked-in registry.
pub(crate) const PENDING: &[Pending] = &[];
