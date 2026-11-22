// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed package-round-trip inputs.
use super::value::{self, TypedExpression};
use proptest::prelude::BoxedStrategy;

/// Typed units use the same independent expression description as value suites.
/// These units select no external model, so package admission needs no library data.
pub fn package_units() -> BoxedStrategy<TypedExpression> {
    value::expressions()
}
