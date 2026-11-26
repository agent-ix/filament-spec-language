// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-038 RL-7: the route admission table, declared once.
//!
//! The table is data: its rows and the names of the conditions each row
//! tests, as FSpec FR-421 "Closed and strict conditions" and ADR-038's table
//! give them. EN-5 and EN-5's certificate checker both read it, and EN-1
//! reads its own rows. Nothing here reads a model, a claim or a state: each
//! side establishes every condition with its own code and then looks the
//! row up, so a wrong condition on one side is caught by the other and a
//! wrong row is a specification defect.

use ix_trace_rs::implements;

/// How a subject's delays are given.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DelayClass {
    /// Every delay is given by a `discrete` distribution.
    Given,
    /// No delay is given.
    Free,
    /// Some delays are given and others are not.
    Mixed,
}

/// The claim forms the table names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimForm {
    /// Reachability or until over state predicates, or its `always` dual.
    Reachability,
    /// `expected accumulate ... until`.
    ExpectedAccumulate,
    /// A step-bounded or long-run measure that reads only the sequence of
    /// discrete steps.
    DiscreteStepMeasure,
    /// `expected elapsed until holds(B)`.
    ExpectedElapsed,
    /// `eventually[0, D]`, `until[0, D]` over state predicates, or the
    /// `always[0, D]` complement.
    DeadlineEvent,
    /// A `steps` or `accumulate` measure within a time horizon, or a
    /// `fraction` over `[0, h]` `weighted by R`, `R` reading no clock.
    TimeHorizonMeasure,
    /// `M <= c` of an elapsed measure.
    ElapsedAtMost,
    /// A `weighted by delay` fraction.
    DelayWeightedFraction,
    /// Any other bounded-window or temporal formula.
    OtherFormula,
    /// An unweighted fraction over a time window.
    UnweightedTimeFraction,
    /// Any other `elapsed` form.
    OtherElapsed,
}

/// Where the claim is read from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadFrom {
    /// `on origin`; for an elapsed measure, its first position is position 0
    /// on every run.
    Origin,
    /// `on each`; for an elapsed measure, its first position can follow
    /// position 0.
    Later,
}

/// The conditions one claim over one subject is looked up with. A side
/// fills every member from its own reading of the checked subject and claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteConditions {
    /// Every clock constraint and every timed interval of the claim, `M < c`
    /// included, is non-strict. No constraint compares two clocks.
    pub closed: bool,
    /// The subject's delay class.
    pub delays: DelayClass,
    /// Whether the claim states a fairness set.
    pub fairness: bool,
    /// The claim form.
    pub form: ClaimForm,
    /// The position the claim is read from.
    pub read_from: ReadFrom,
    /// Whether the value the item takes is the maximum of the probability of
    /// the reach, until or deadline event, which is the minimum of the
    /// matching `always`: a `<=` bound on `eventually` or `until`, or a `>=`
    /// bound on `always`.
    pub takes_event_maximum: bool,
}

/// What a row admits for a `free` subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FreeSubject {
    /// No `free` subject.
    Never,
    /// A `free` subject with no reachable time-lock.
    NoTimeLock,
    /// A `free` subject with no reachable time-lock, and with one for the
    /// maximum of the event alone.
    MaximumUnderTimeLock,
}

/// One row of EN-5's digital admission: the forms it covers at the positions
/// it covers them from. Every row admits a `given` subject and a `free` one
/// as `free` states; every row needs a closed subject and no fairness set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionRow {
    /// The claim forms.
    pub forms: &'static [ClaimForm],
    /// The positions.
    pub read_from: &'static [ReadFrom],
    /// What the row admits for a `free` subject.
    pub free: FreeSubject,
}

const ANY_POSITION: &[ReadFrom] = &[ReadFrom::Origin, ReadFrom::Later];

/// FSpec FR-421 "Closed and strict conditions", EN-5's digital rows.
pub const EN5_DIGITAL: &[AdmissionRow] = &[
    AdmissionRow {
        forms: &[ClaimForm::Reachability],
        read_from: ANY_POSITION,
        free: FreeSubject::MaximumUnderTimeLock,
    },
    AdmissionRow {
        forms: &[
            ClaimForm::ExpectedAccumulate,
            ClaimForm::DiscreteStepMeasure,
            ClaimForm::ExpectedElapsed,
        ],
        read_from: ANY_POSITION,
        free: FreeSubject::NoTimeLock,
    },
    AdmissionRow {
        forms: &[ClaimForm::DeadlineEvent],
        read_from: &[ReadFrom::Origin],
        free: FreeSubject::MaximumUnderTimeLock,
    },
    AdmissionRow {
        forms: &[ClaimForm::TimeHorizonMeasure, ClaimForm::ElapsedAtMost],
        read_from: &[ReadFrom::Origin],
        free: FreeSubject::NoTimeLock,
    },
    AdmissionRow {
        forms: &[
            ClaimForm::DeadlineEvent,
            ClaimForm::TimeHorizonMeasure,
            ClaimForm::ElapsedAtMost,
        ],
        read_from: &[ReadFrom::Later],
        free: FreeSubject::Never,
    },
    AdmissionRow {
        forms: &[ClaimForm::DelayWeightedFraction],
        read_from: ANY_POSITION,
        free: FreeSubject::Never,
    },
];

/// Why a claim leaves digital clocks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RegionCause {
    /// A strict constraint or interval.
    Strict,
    /// Some delays are given and others are not.
    MixedDelays,
    /// The claim states a fairness set.
    Fairness,
    /// A time-lock is reachable in a `free` subject, or its absence is not
    /// established, and the form needs it absent.
    TimeLock,
    /// A `free` subject read from a later position.
    LaterPosition,
    /// A bounded-window or temporal formula other than a deadline event.
    OtherFormula,
    /// An unweighted fraction over a time window.
    UnweightedTimeFraction,
    /// A `weighted by delay` fraction over a `free` subject.
    DelayWeightedFraction,
    /// An `elapsed` form other than `M <= c`.
    OtherElapsed,
}

impl RegionCause {
    /// The name a result states the cause by.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict-constraint",
            Self::MixedDelays => "mixed-delays",
            Self::Fairness => "fairness",
            Self::TimeLock => "time-lock",
            Self::LaterPosition => "later-position",
            Self::OtherFormula => "other-formula",
            Self::UnweightedTimeFraction => "unweighted-time-fraction",
            Self::DelayWeightedFraction => "delay-weighted-fraction",
            Self::OtherElapsed => "other-elapsed",
        }
    }
}

/// What the table says of one set of conditions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    /// Digital clocks decide the claim.
    Digital,
    /// Digital clocks decide it only where no time-lock is reachable over the
    /// subject's dense runs; the caller establishes that and otherwise reads
    /// [`RegionCause::TimeLock`].
    DigitalIfNoTimeLock,
    /// Clock regions decide it.
    Region(RegionCause),
}

/// The row of [`EN5_DIGITAL`] that admits `conditions`, or why none does.
#[implements("FSpec-FR-421-AC-8", "FR-921-AC-5")]
pub fn en5_admission(conditions: RouteConditions) -> Admission {
    let RouteConditions {
        closed,
        delays,
        fairness,
        form,
        read_from,
        takes_event_maximum,
    } = conditions;
    if !closed {
        return Admission::Region(RegionCause::Strict);
    }
    if delays == DelayClass::Mixed {
        return Admission::Region(RegionCause::MixedDelays);
    }
    if fairness {
        return Admission::Region(RegionCause::Fairness);
    }
    let row = EN5_DIGITAL
        .iter()
        .find(|row| row.forms.contains(&form) && row.read_from.contains(&read_from));
    let Some(row) = row else {
        return Admission::Region(match form {
            ClaimForm::UnweightedTimeFraction => RegionCause::UnweightedTimeFraction,
            ClaimForm::OtherElapsed => RegionCause::OtherElapsed,
            ClaimForm::OtherFormula
            | ClaimForm::Reachability
            | ClaimForm::ExpectedAccumulate
            | ClaimForm::DiscreteStepMeasure
            | ClaimForm::ExpectedElapsed
            | ClaimForm::DeadlineEvent
            | ClaimForm::TimeHorizonMeasure
            | ClaimForm::ElapsedAtMost
            | ClaimForm::DelayWeightedFraction => RegionCause::OtherFormula,
        });
    };
    if delays == DelayClass::Given {
        return Admission::Digital;
    }
    match row.free {
        FreeSubject::Never => Admission::Region(if form == ClaimForm::DelayWeightedFraction {
            RegionCause::DelayWeightedFraction
        } else {
            RegionCause::LaterPosition
        }),
        FreeSubject::MaximumUnderTimeLock if takes_event_maximum => Admission::Digital,
        FreeSubject::NoTimeLock | FreeSubject::MaximumUnderTimeLock => {
            Admission::DigitalIfNoTimeLock
        }
    }
}

/// The items of EN-1's `digital-clocks` method the table names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchItem {
    /// A temporal claim that is digitisable by FSL FR-243 (a) to (f).
    DigitisableTemporalClaim,
    /// `possible`: location reachability.
    Possible,
    /// Deadlock freedom.
    DeadlockFreedom,
    /// `always possible`.
    AlwaysPossible,
    /// `unique path`.
    UniquePath,
}

/// ADR-038's EN-1 rows: what `digital-clocks` admits over a closed subject.
pub const EN1_DIGITAL: &[SearchItem] = &[
    SearchItem::DigitisableTemporalClaim,
    SearchItem::Possible,
    SearchItem::DeadlockFreedom,
];

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    use Admission::{Digital, DigitalIfNoTimeLock, Region};
    use ClaimForm as F;
    use ReadFrom::{Later, Origin};

    fn closed(delays: DelayClass, form: ClaimForm, read_from: ReadFrom) -> RouteConditions {
        RouteConditions {
            closed: true,
            delays,
            fairness: false,
            form,
            read_from,
            takes_event_maximum: false,
        }
    }

    /// Each row of FSpec FR-421's table, written out by form and position:
    /// what a given subject gets, what a free one gets for the value that is
    /// not the event's maximum, and what it gets for the maximum.
    #[trace("FSpec-FR-421-AC-8")]
    #[test]
    fn every_row_gives_its_hand_route() {
        let later = Region(RegionCause::LaterPosition);
        let other = Region(RegionCause::OtherFormula);
        let fraction = Region(RegionCause::UnweightedTimeFraction);
        let elapsed = Region(RegionCause::OtherElapsed);
        let weighted = Region(RegionCause::DelayWeightedFraction);
        let rows = [
            (
                F::Reachability,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                Digital,
            ),
            (
                F::Reachability,
                Later,
                Digital,
                DigitalIfNoTimeLock,
                Digital,
            ),
            (
                F::ExpectedAccumulate,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                DigitalIfNoTimeLock,
            ),
            (
                F::DiscreteStepMeasure,
                Later,
                Digital,
                DigitalIfNoTimeLock,
                DigitalIfNoTimeLock,
            ),
            (
                F::ExpectedElapsed,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                DigitalIfNoTimeLock,
            ),
            (
                F::DeadlineEvent,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                Digital,
            ),
            (F::DeadlineEvent, Later, Digital, later, later),
            (
                F::TimeHorizonMeasure,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                DigitalIfNoTimeLock,
            ),
            (F::TimeHorizonMeasure, Later, Digital, later, later),
            (
                F::ElapsedAtMost,
                Origin,
                Digital,
                DigitalIfNoTimeLock,
                DigitalIfNoTimeLock,
            ),
            (F::ElapsedAtMost, Later, Digital, later, later),
            (
                F::DelayWeightedFraction,
                Origin,
                Digital,
                weighted,
                weighted,
            ),
            (F::DelayWeightedFraction, Later, Digital, weighted, weighted),
            (F::OtherFormula, Origin, other, other, other),
            (
                F::UnweightedTimeFraction,
                Origin,
                fraction,
                fraction,
                fraction,
            ),
            (F::OtherElapsed, Origin, elapsed, elapsed, elapsed),
        ];
        for (form, from, given, free, maximum) in rows {
            let given_case = closed(DelayClass::Given, form, from);
            assert_eq!(en5_admission(given_case), given, "given {form:?} {from:?}");
            let free_case = closed(DelayClass::Free, form, from);
            assert_eq!(en5_admission(free_case), free, "free {form:?} {from:?}");
            let top = RouteConditions {
                takes_event_maximum: true,
                ..free_case
            };
            assert_eq!(en5_admission(top), maximum, "maximum {form:?} {from:?}");
        }
    }

    /// Closedness, the delay class and fairness leave digital clocks for
    /// every form.
    #[trace("FSpec-FR-421-AC-10", "FSpec-FR-421-AC-13")]
    #[test]
    fn strict_mixed_and_fair_claims_are_not_digital() {
        let deadline = closed(DelayClass::Given, F::DeadlineEvent, Origin);
        assert_eq!(
            en5_admission(RouteConditions {
                closed: false,
                ..deadline
            }),
            Region(RegionCause::Strict)
        );
        assert_eq!(
            en5_admission(closed(DelayClass::Mixed, F::Reachability, Origin)),
            Region(RegionCause::MixedDelays)
        );
        for delays in [DelayClass::Given, DelayClass::Free] {
            assert_eq!(
                en5_admission(RouteConditions {
                    fairness: true,
                    ..closed(delays, F::Reachability, Origin)
                }),
                Region(RegionCause::Fairness)
            );
        }
    }

    #[trace("FR-243-AC-5")]
    #[test]
    fn en1_admits_possible_and_deadlock_freedom_only_of_the_state_forms() {
        assert!(EN1_DIGITAL.contains(&SearchItem::Possible));
        assert!(EN1_DIGITAL.contains(&SearchItem::DeadlockFreedom));
        assert!(!EN1_DIGITAL.contains(&SearchItem::AlwaysPossible));
        assert!(!EN1_DIGITAL.contains(&SearchItem::UniquePath));
    }
}
