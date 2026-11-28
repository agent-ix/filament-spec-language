// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small shared helpers for timed test targets.

use filament_exact::{Integer, Rational};

pub(crate) fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
}
