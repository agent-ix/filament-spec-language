// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 fix-hint catalog types and lookup key.

use fsl_foundation::diagnostic::Code;

/// One catalog entry keyed by diagnostic code and optional cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HintRow {
    /// The diagnostic code.
    pub code: Code,
    /// The typed cause, or `None` for a cause-less diagnostic.
    pub cause: Option<&'static str>,
    /// The imperative sentence shown to the user.
    pub text: &'static str,
}

/// The fix-hint catalog, populated with supported rows in the producer phase.
pub const HINTS: &[HintRow] = &[];

/// Find a hint by the exact `(code, cause)` key.
pub fn hint_for(_code: Code, _cause: Option<&str>) -> Option<&'static str> {
    None
}
