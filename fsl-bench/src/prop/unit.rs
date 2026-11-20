// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete grammar inputs and typed package-round-trip inputs.
use super::value::{self, TypedExpression};
use proptest::prelude::{BoxedStrategy, Strategy};

/// One grammar-derived complete unit, selected from the real rule table.
pub fn grammar_units() -> BoxedStrategy<fsl_cst::grammar::generate::Derivation> {
    proptest::sample::select(fsl_cst::grammar::generate::covering_derivations()).boxed()
}

/// Typed units use the same independent expression description as value suites.
/// These units select no external model, so package admission needs no library data.
pub fn package_units() -> BoxedStrategy<TypedExpression> {
    value::expressions()
}
