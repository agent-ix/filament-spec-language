// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621 selection of the instance record that settles a multi-instance item.

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

/// How one instance's result stands in the combination of an item over
/// several instances (FSpec FR-361 "Instancing").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceVerdict {
    /// The instance is refuted.
    Refuted,
    /// The instance is not proved: it ended undecided, at a bound, stopped,
    /// or was not decided at all.
    NotProved,
    /// The instance is proved.
    Proved,
}

/// What the instances read so far give the item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceSelection {
    /// The item is refuted, by the first refuted instance, at this index.
    Refuted(usize),
    /// No instance is refuted and the first that is not proved is at this
    /// index: the item is not proved and takes that instance's outcome.
    NotProved(usize),
    /// Every instance read is proved.
    Proved,
}

/// The one fold of FSpec FR-361: an item over several instances is refuted
/// exactly when some instance is refuted, by the first refuted instance in
/// the canonical order, whatever an earlier instance gave; otherwise it is
/// proved only when every instance is, and else it takes the outcome of the
/// first instance that is not proved. Every route that reads the instances of
/// an item, over records or over engine outcomes, combines them with this.
#[derive(Clone, Debug, Default)]
pub struct InstanceFold {
    read: usize,
    refuted: Option<usize>,
    first_not_proved: Option<usize>,
}

impl InstanceFold {
    /// A fold that has read no instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the next instance in canonical order. Returns whether the item is
    /// now refuted, so that no later instance need be read.
    pub fn push(&mut self, verdict: InstanceVerdict) -> bool {
        let index = self.read;
        self.read += 1;
        match verdict {
            InstanceVerdict::Refuted => {
                self.refuted.get_or_insert(index);
            }
            InstanceVerdict::NotProved => {
                self.first_not_proved.get_or_insert(index);
            }
            InstanceVerdict::Proved => {}
        }
        self.refuted.is_some()
    }

    /// What the instances read so far give the item.
    #[must_use]
    pub fn selection(&self) -> InstanceSelection {
        match (self.refuted, self.first_not_proved) {
            (Some(index), _) => InstanceSelection::Refuted(index),
            (None, Some(index)) => InstanceSelection::NotProved(index),
            (None, None) => InstanceSelection::Proved,
        }
    }
}

/// [`InstanceFold`] over `items` in canonical order, each read by `verdict`.
pub fn fold_instances<T>(
    items: &[T],
    verdict: impl Fn(&T) -> InstanceVerdict,
) -> InstanceSelection {
    let mut fold = InstanceFold::new();
    for item in items {
        if fold.push(verdict(item)) {
            break;
        }
    }
    fold.selection()
}

/// Select the item record from per-instance records in canonical instance
/// order, by [`InstanceFold`]: a refutation wins, otherwise the first
/// non-proof, and if every record proves the item, the weakest proof basis.
#[implements("FR-621-AC-9", "FR-621-AC-23", "FR-621-AC-24", "FR-621-AC-25")]
pub fn combine_instance_records<T: InstanceRecord>(records: &[T]) -> usize {
    match fold_instances(records, |record| match record.terminal_value() {
        TerminalValue::Refuted => InstanceVerdict::Refuted,
        TerminalValue::Proved { .. } => InstanceVerdict::Proved,
        _ => InstanceVerdict::NotProved,
    }) {
        InstanceSelection::Refuted(index) | InstanceSelection::NotProved(index) => index,
        InstanceSelection::Proved => {
            let proofs: Vec<_> = records
                .iter()
                .filter_map(|record| match record.terminal_value() {
                    TerminalValue::Proved {
                        basis,
                        certification,
                    } => Some((basis, *certification)),
                    _ => None,
                })
                .collect();
            weakest(&proofs)
        }
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

    #[trace("FR-621-AC-9")]
    #[test]
    fn the_fold_reads_a_refutation_past_an_earlier_not_proved_instance() {
        let mut fold = InstanceFold::new();
        assert!(!fold.push(InstanceVerdict::NotProved));
        assert_eq!(fold.selection(), InstanceSelection::NotProved(0));
        assert!(!fold.push(InstanceVerdict::Proved));
        assert!(fold.push(InstanceVerdict::Refuted));
        assert_eq!(fold.selection(), InstanceSelection::Refuted(2));
    }

    #[trace("FR-621-AC-9")]
    #[test]
    fn the_fold_proves_only_when_every_instance_is_proved() {
        let verdict = |v: &InstanceVerdict| *v;
        assert_eq!(
            fold_instances(&[InstanceVerdict::Proved, InstanceVerdict::Proved], verdict),
            InstanceSelection::Proved
        );
        assert_eq!(
            fold_instances(
                &[
                    InstanceVerdict::Proved,
                    InstanceVerdict::NotProved,
                    InstanceVerdict::NotProved
                ],
                verdict
            ),
            InstanceSelection::NotProved(1)
        );
    }
}
