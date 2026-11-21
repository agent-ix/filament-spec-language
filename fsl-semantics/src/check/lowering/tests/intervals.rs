// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 "Formula intervals": the canonical interval text a bounded
//! temporal operator holds in its `temporal`/`formula` node path.

use super::*;
use filament_exact::{Integer, Rational};
use fsl_foundation::bound::{TemporalInterval, TemporalUpper};

use crate::value::member::TimedMember;

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn timed(
    lower: Rational,
    lower_closed: bool,
    upper: Option<Rational>,
    upper_closed: bool,
) -> TimedMember {
    TimedMember {
        lower,
        lower_closed,
        upper,
        upper_closed,
    }
}

/// FR-322 "Formula intervals": the integer form is `steps:<lo>..<hi>`, with
/// `inf` for an open upper end.
#[trace("FR-322")]
#[test]
fn integer_intervals_spell_steps_with_inf_for_an_open_end() {
    let finite = TemporalInterval::new(0, TemporalUpper::Finite(2)).unwrap();
    let open = TemporalInterval::new(0, TemporalUpper::Open).unwrap();
    assert_eq!(interval_spelling(finite), "steps:0..2");
    assert_eq!(interval_spelling(open), "steps:0..inf");
}

/// FR-322 "Formula intervals": the timed form writes each bound as its exact
/// decimal when it terminates and as `numerator/denominator` otherwise, the
/// bracket kinds from the ends, and `inf` for no upper bound; `[0, 21/2]` is
/// `time:[0..10.5]` and `(1/3, 1]` is `time:(1/3..1]`.
#[trace("FR-322")]
#[test]
fn timed_intervals_spell_exact_decimals_brackets_and_inf() {
    let spell = |member: TimedMember| timed_interval_spelling(&member);
    assert_eq!(
        spell(timed(ratio(0, 1), true, Some(ratio(21, 2)), true)),
        "time:[0..10.5]"
    );
    assert_eq!(
        spell(timed(ratio(1, 3), false, Some(ratio(1, 1)), true)),
        "time:(1/3..1]"
    );
    assert_eq!(
        spell(timed(ratio(1, 4), true, None, false)),
        "time:[0.25..inf)"
    );
    assert_eq!(
        spell(timed(ratio(1, 8), true, Some(ratio(3, 100)), false)),
        "time:[0.125..0.03)"
    );
}
