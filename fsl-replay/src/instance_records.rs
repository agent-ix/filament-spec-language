// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621 selection of the instance record that settles a multi-instance item.

use std::num::NonZeroUsize;

use filament_exact::Rational;
use ix_trace_rs::implements;

use crate::{Certification, ProofBasis, StateGraphRecord, TerminalRecord, TerminalValue};

/// A record whose terminal value participates in FR-621 instance selection.
pub trait InstanceRecord {
    /// The terminal value settled for this instance.
    fn terminal_value(&self) -> &TerminalValue;
}

impl InstanceRecord for StateGraphRecord {
    fn terminal_value(&self) -> &TerminalValue {
        &self.value
    }
}

impl InstanceRecord for TerminalRecord {
    fn terminal_value(&self) -> &TerminalValue {
        self.value()
    }
}

impl<T: InstanceRecord + ?Sized> InstanceRecord for &T {
    fn terminal_value(&self) -> &TerminalValue {
        (*self).terminal_value()
    }
}

/// One instance's verdict, pushed to an [`InstanceFold`] in canonical
/// instance order. `O` is the route's own outcome for the instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstanceVerdict<O> {
    /// The instance is refuted: the item is refuted by it.
    Refuted(O),
    /// The instance is proved.
    Proved(O),
    /// The instance ended undecided, at a bound or on a per-instance limit.
    NotProved(O),
    /// A run-level end stopped the instance (FR-126 "TP-4 items"): no later
    /// instance is decided. `cancel` is set for a cancellation, which gives
    /// the item's outcome whatever an earlier instance gave.
    RunStop {
        /// The stopped instance's outcome.
        outcome: O,
        /// Whether the end is a cancellation.
        cancel: bool,
    },
}

/// Whether the caller decides the next instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    /// Decide the next instance.
    Next,
    /// Decide no further instance and call [`InstanceFold::finish`].
    Stop,
}

/// The item's settlement over its instances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Combined<O> {
    /// The first refuted instance, at `index`.
    Refuted {
        /// The instance's position in canonical order.
        index: usize,
        /// Its outcome.
        outcome: O,
    },
    /// A run-level end settles the item: a cancellation, or a stop with no
    /// earlier not-proved instance.
    Stopped {
        /// The stopped instance's position.
        index: usize,
        /// Its outcome.
        outcome: O,
    },
    /// No instance is refuted and the first not-proved one, at `index`,
    /// settles the item.
    NotProved {
        /// The instance's position in canonical order.
        index: usize,
        /// Its outcome.
        outcome: O,
    },
    /// Every instance is proved: their outcomes in canonical order.
    Proved {
        /// One outcome per instance.
        proofs: Vec<O>,
    },
}

/// The fold read fewer instances than it was given and was not stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the instance fold finished before every instance was pushed")]
pub struct UnfinishedFold;

/// FSpec FR-361 "Instancing" and FR-126 "TP-4 items": the one streaming
/// combination of per-instance verdicts. A refutation wins and stops the
/// loop; a run-level end stops it and reports the first not-proved instance
/// (the stopped one when none), a cancellation reporting itself; otherwise
/// the first not-proved instance settles the item, and when every instance is
/// proved the item holds the proofs in canonical order.
#[derive(Debug)]
pub struct InstanceFold<O> {
    count: usize,
    read: usize,
    proofs: Vec<O>,
    first_not_proved: Option<(usize, O)>,
    settled: Option<Combined<O>>,
}

impl<O> InstanceFold<O> {
    /// A fold over `count` instances in canonical order.
    pub fn new(count: NonZeroUsize) -> Self {
        Self {
            count: count.get(),
            read: 0,
            proofs: Vec::new(),
            first_not_proved: None,
            settled: None,
        }
    }

    /// Take the next instance's verdict.
    pub fn push(&mut self, verdict: InstanceVerdict<O>) -> Step {
        if self.settled.is_some() {
            return Step::Stop;
        }
        let index = self.read;
        self.read += 1;
        match verdict {
            InstanceVerdict::Refuted(outcome) => {
                self.settled = Some(Combined::Refuted { index, outcome });
                Step::Stop
            }
            InstanceVerdict::RunStop { outcome, cancel } => {
                self.settled = Some(match self.first_not_proved.take() {
                    Some((first, kept)) if !cancel => Combined::NotProved {
                        index: first,
                        outcome: kept,
                    },
                    _ => Combined::Stopped { index, outcome },
                });
                Step::Stop
            }
            InstanceVerdict::NotProved(outcome) => {
                if self.first_not_proved.is_none() {
                    self.first_not_proved = Some((index, outcome));
                }
                Step::Next
            }
            InstanceVerdict::Proved(outcome) => {
                self.proofs.push(outcome);
                Step::Next
            }
        }
    }

    /// The item's settlement.
    pub fn finish(self) -> Result<Combined<O>, UnfinishedFold> {
        if let Some(settled) = self.settled {
            return Ok(settled);
        }
        if self.read < self.count {
            return Err(UnfinishedFold);
        }
        Ok(match self.first_not_proved {
            Some((index, outcome)) => Combined::NotProved { index, outcome },
            None => Combined::Proved {
                proofs: self.proofs,
            },
        })
    }
}

/// Select the item record from per-instance records in canonical instance order.
/// A replay-confirmed refutation wins; otherwise the first non-proof wins; if
/// every record proves the item, report the weakest proof basis. A thin user
/// of [`InstanceFold`]: a record is settled per instance, so none is a run
/// stop.
#[implements("FR-621-AC-9", "FR-621-AC-23", "FR-621-AC-24", "FR-621-AC-25")]
pub fn combine_instance_records<T: InstanceRecord>(records: &[T]) -> usize {
    let Some(count) = NonZeroUsize::new(records.len()) else {
        return 0;
    };
    let mut fold = InstanceFold::new(count);
    for (index, record) in records.iter().enumerate() {
        let verdict = match record.terminal_value() {
            TerminalValue::Refuted => InstanceVerdict::Refuted(index),
            TerminalValue::Proved { .. } => InstanceVerdict::Proved(index),
            _ => InstanceVerdict::NotProved(index),
        };
        if fold.push(verdict) == Step::Stop {
            break;
        }
    }
    match fold.finish() {
        Ok(Combined::Refuted { outcome, .. } | Combined::NotProved { outcome, .. }) => outcome,
        Ok(Combined::Stopped { index, .. }) => index,
        Ok(Combined::Proved { proofs }) => {
            let proofs: Vec<_> = proofs
                .into_iter()
                .filter_map(|index| match records.get(index)?.terminal_value() {
                    TerminalValue::Proved {
                        basis,
                        certification,
                    } => Some((basis, *certification)),
                    _ => None,
                })
                .collect();
            weakest(&proofs)
        }
        Err(UnfinishedFold) => 0,
    }
}

enum Bound<'a> {
    Steps(u64),
    Seconds(&'a Rational),
}

fn bound(basis: &ProofBasis) -> Option<Bound<'_>> {
    match basis {
        ProofBasis::BoundedComplete { depth } => Some(Bound::Steps(*depth)),
        ProofBasis::BoundedSolver { horizon, .. } => Some(Bound::Seconds(horizon.seconds())),
        ProofBasis::Checks { .. }
        | ProofBasis::Exhaustive
        | ProofBasis::ZoneCertified
        | ProofBasis::ExactValue { .. }
        | ProofBasis::ValueBounds { .. }
        | ProofBasis::Inductive { .. }
        | ProofBasis::ClosedForm { .. }
        | ProofBasis::InductiveSolver { .. }
        | ProofBasis::Witness { .. }
        | ProofBasis::Reduced { .. } => None,
    }
}

fn weakness(certification: Certification) -> u8 {
    match certification {
        Certification::Certified => 0,
        Certification::Uncertified => 1,
        Certification::Trusted => 2,
    }
}

fn narrow(kept: &mut Vec<usize>, wanted: impl Fn(usize) -> bool) {
    if kept.iter().any(|index| wanted(*index)) {
        kept.retain(|index| wanted(*index));
    }
}

fn weakest(proofs: &[(&ProofBasis, Certification)]) -> usize {
    let Some(worst_certification) = proofs.iter().map(|(_, c)| weakness(*c)).max() else {
        return 0;
    };
    let basis = |i: usize| proofs.get(i).map(|(basis, _)| *basis);
    let certification = |i: usize| proofs.get(i).map(|(_, certification)| *certification);
    let mut kept: Vec<usize> = (0..proofs.len())
        .filter(|&i| certification(i).is_some_and(|c| weakness(c) == worst_certification))
        .collect();
    narrow(&mut kept, |i| basis(i).and_then(bound).is_some());
    let bounds: Vec<_> = kept
        .iter()
        .filter_map(|&i| basis(i).and_then(bound))
        .collect();
    let steps: Vec<_> = bounds
        .iter()
        .filter_map(|b| match b {
            Bound::Steps(n) => Some(*n),
            _ => None,
        })
        .collect();
    let seconds: Vec<_> = bounds
        .iter()
        .filter_map(|b| match b {
            Bound::Seconds(n) => Some(*n),
            _ => None,
        })
        .collect();
    if seconds.is_empty() {
        if let Some(minimum) = steps.iter().min() {
            kept.retain(
                |&i| matches!(basis(i).and_then(bound), Some(Bound::Steps(n)) if n == *minimum),
            );
        }
    } else if steps.is_empty() {
        if let Some(minimum) = seconds.iter().min() {
            kept.retain(
                |&i| matches!(basis(i).and_then(bound), Some(Bound::Seconds(n)) if n == *minimum),
            );
        }
    }
    narrow(&mut kept, |i| {
        basis(i).is_some_and(|basis| matches!(basis, ProofBasis::Reduced { .. }))
    });
    kept.first().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InconclusiveCause, StateGraphRecord};
    use ix_trace_rs::trace;

    fn record(value: TerminalValue) -> StateGraphRecord {
        StateGraphRecord {
            value,
            label: crate::ResultLabel::Inconclusive,
            basis: crate::SettlementBasis::Unsettled,
            execution: crate::Execution::Completed,
            truth: crate::Truth::Pending,
            method: crate::EXPLICIT_STATE,
            max_depth: None,
            limits: [
                (crate::ModelCheckLimit::MaxStates, 0),
                (crate::ModelCheckLimit::MaxTransitions, 0),
                (crate::ModelCheckLimit::MaxAutomatonStates, 0),
            ],
            limit_reached: None,
            seed: 0,
            witness_samples: 0,
            walks_stopped: 0,
            end: None,
            undefined: None,
        }
    }

    #[trace("FR-621-AC-9")]
    #[test]
    fn refutation_beats_an_earlier_incomplete_record() {
        let records = [
            record(TerminalValue::Inconclusive(InconclusiveCause::NoInstance)),
            record(TerminalValue::Refuted),
        ];
        assert_eq!(combine_instance_records(&records), 1);
    }

    #[trace("FR-621-AC-25")]
    #[test]
    fn reduced_proof_is_selected_when_all_instances_prove() {
        let records = [
            record(TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certification::Certified,
            }),
            record(TerminalValue::Proved {
                basis: ProofBasis::Reduced {
                    reductions: Vec::new(),
                },
                certification: Certification::Certified,
            }),
        ];
        assert_eq!(combine_instance_records(&records), 1);
    }

    #[trace("FR-621-AC-25")]
    #[test]
    fn weaker_certification_is_selected_when_all_instances_prove() {
        let records = [
            record(TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certification::Certified,
            }),
            record(TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certification::Uncertified,
            }),
        ];
        assert_eq!(combine_instance_records(&records), 1);
    }

    fn push_all(
        verdicts: Vec<InstanceVerdict<&'static str>>,
    ) -> (Vec<Step>, Combined<&'static str>) {
        let count = NonZeroUsize::new(verdicts.len()).expect("a non-empty instance list");
        let mut fold = InstanceFold::new(count);
        let mut steps = Vec::new();
        for verdict in verdicts {
            let step = fold.push(verdict);
            steps.push(step);
            if step == Step::Stop {
                break;
            }
        }
        (steps, fold.finish().expect("the fold is settled"))
    }

    /// FSpec FR-361-AC-10: the item is refuted exactly when some instance is
    /// refuted, in both orders, and the first refuted instance is reported.
    #[trace("FR-621-AC-9", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
    #[test]
    fn the_fold_reports_the_first_refuted_instance_in_either_order() {
        use InstanceVerdict::{NotProved, Proved, Refuted};
        let (steps, combined) = push_all(vec![NotProved("x0"), Proved("p1"), Refuted("r2")]);
        assert_eq!(steps, [Step::Next, Step::Next, Step::Stop]);
        assert_eq!(
            combined,
            Combined::Refuted {
                index: 2,
                outcome: "r2"
            }
        );
        let (steps, combined) = push_all(vec![Refuted("r0"), NotProved("x1")]);
        assert_eq!(steps, [Step::Stop]);
        assert_eq!(
            combined,
            Combined::Refuted {
                index: 0,
                outcome: "r0"
            }
        );
        let (_, combined) = push_all(vec![NotProved("x0"), Refuted("r1")]);
        assert_eq!(
            combined,
            Combined::Refuted {
                index: 1,
                outcome: "r1"
            }
        );
    }

    /// FR-126 "TP-4 items": with no refutation the first not-proved instance
    /// settles the item, and a proof needs every instance proved.
    #[trace("FR-621-AC-9", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
    #[test]
    fn the_fold_settles_the_first_not_proved_instance_or_every_proof() {
        use InstanceVerdict::{NotProved, Proved};
        let (_, combined) = push_all(vec![Proved("p0"), NotProved("x1"), NotProved("x2")]);
        assert_eq!(
            combined,
            Combined::NotProved {
                index: 1,
                outcome: "x1"
            }
        );
        let (_, combined) = push_all(vec![Proved("p0"), Proved("p1")]);
        assert_eq!(
            combined,
            Combined::Proved {
                proofs: vec!["p0", "p1"]
            }
        );
    }

    /// Arch Q1: a run-level end after an earlier not-proved instance reports
    /// that instance; with none earlier it reports the stopped instance; a
    /// cancellation reports itself whatever came before.
    #[trace("FR-621-AC-9", "FSpec-TC-290", "FSpec-FR-361-AC-10")]
    #[test]
    fn a_run_stop_reports_the_first_not_proved_instance_unless_cancelled() {
        use InstanceVerdict::{NotProved, Proved, RunStop};
        let stop = |outcome, cancel| RunStop { outcome, cancel };
        let (steps, combined) = push_all(vec![NotProved("x0"), stop("s1", false), Proved("p2")]);
        assert_eq!(steps, [Step::Next, Step::Stop]);
        assert_eq!(
            combined,
            Combined::NotProved {
                index: 0,
                outcome: "x0"
            }
        );
        let (_, combined) = push_all(vec![Proved("p0"), stop("s1", false)]);
        assert_eq!(
            combined,
            Combined::Stopped {
                index: 1,
                outcome: "s1"
            }
        );
        let (_, combined) = push_all(vec![NotProved("x0"), stop("s1", true)]);
        assert_eq!(
            combined,
            Combined::Stopped {
                index: 1,
                outcome: "s1"
            }
        );
    }

    #[test]
    fn a_fold_finished_before_every_instance_is_a_fault() {
        let mut fold = InstanceFold::new(NonZeroUsize::new(2).expect("two"));
        fold.push(InstanceVerdict::Proved("p0"));
        assert_eq!(fold.finish(), Err(UnfinishedFold));
    }
}
