// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621 "Items over several claim instances": which instance's record an
//! item with one record per instance reports.

use filament_exact::Rational;
use fsl_replay::{
    Certification, Execution, InconclusiveCause, ProofBasis, ResultLabel, RunStats,
    SettlementBasis, StateGraphMethod, StateGraphRecord, TerminalValue, Truth, EXPLICIT_STATE,
};
use ix_trace_rs::implements;

use super::{EngineReport, GraphReport};

/// The index of the instance whose record the item reports: the first refuted
/// one, else the first not proved, else the proved one with the weakest
/// support. `records` is in FR-125 canonical instance order.
#[implements("FR-621-AC-9", "FR-621-AC-23", "FR-621-AC-24")]
pub(super) fn decisive(records: &[StateGraphRecord]) -> usize {
    if let Some(refuted) = records
        .iter()
        .position(|record| matches!(record.value, TerminalValue::Refuted))
    {
        return refuted;
    }
    let mut proofs = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        match &record.value {
            TerminalValue::Proved {
                basis,
                certification,
            } => proofs.push(InstanceProof {
                basis,
                certification: *certification,
                // No engine that records each instance runs under a state
                // constraint.
                constrained: false,
            }),
            _ => return index,
        }
    }
    weakest(&proofs)
}

/// The report of an item with no instance to decide, which settles
/// inconclusive with `cause` and is never a fault (FR-621).
#[implements("FR-181-AC-11")]
pub(super) fn undecided(
    cause: InconclusiveCause,
    method: &StateGraphMethod,
    witness_samples: u64,
) -> EngineReport {
    let record = StateGraphRecord {
        value: TerminalValue::Inconclusive(cause),
        label: ResultLabel::Inconclusive,
        basis: SettlementBasis::Unsettled,
        execution: Execution::Completed,
        truth: Truth::Pending,
        method: EXPLICIT_STATE,
        max_depth: method.max_depth,
        limits: method.limits.used(),
        limit_reached: None,
        seed: method.seed,
        witness_samples,
        walks_stopped: 0,
        end: None,
        undefined: None,
    };
    EngineReport::StateGraph(Box::new(GraphReport {
        record,
        exploration: RunStats::default(),
        counterexample: None,
        evidence: None,
        certificate_document: None,
        checker: None,
    }))
}

/// What one proved instance rests on.
struct InstanceProof<'a> {
    basis: &'a ProofBasis,
    certification: Certification,
    /// Whether the instance ran under a state constraint.
    constrained: bool,
}

/// A bound a proof holds within, in its unit.
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

/// How far down FR-243 a certification is: a trusted proof is the weakest.
fn weakness(certification: Certification) -> u8 {
    match certification {
        Certification::Certified => 0,
        Certification::Uncertified => 1,
        Certification::Trusted => 2,
    }
}

/// Keep the members of `kept` that satisfy `wanted`, when any does.
fn narrow(kept: &mut Vec<usize>, wanted: impl Fn(usize) -> bool) {
    if kept.iter().any(|index| wanted(*index)) {
        kept.retain(|index| wanted(*index));
    }
}

/// The index of the proof a proved item over `proofs` takes its basis,
/// payload and certificate from (FR-621 "Proved record over several
/// instances"): among those with the item's certification, the depth-bounded
/// ones if any, the smallest bound when all are in one unit, then the reduced
/// or constrained ones if any, then the first.
#[implements("FR-621-AC-25")]
fn weakest(proofs: &[InstanceProof<'_>]) -> usize {
    let Some(item) = proofs
        .iter()
        .map(|proof| weakness(proof.certification))
        .max()
    else {
        return 0;
    };
    let at = |index: usize| proofs.get(index);
    let mut kept: Vec<usize> = (0..proofs.len())
        .filter(|index| at(*index).is_some_and(|proof| weakness(proof.certification) == item))
        .collect();
    narrow(&mut kept, |index| {
        at(index).is_some_and(|proof| bound(proof.basis).is_some())
    });
    let bounds: Vec<Bound<'_>> = kept
        .iter()
        .filter_map(|index| at(*index).and_then(|proof| bound(proof.basis)))
        .collect();
    let steps = bounds
        .iter()
        .filter_map(|bound| match bound {
            Bound::Steps(depth) => Some(*depth),
            Bound::Seconds(_) => None,
        })
        .collect::<Vec<_>>();
    let seconds = bounds
        .iter()
        .filter_map(|bound| match bound {
            Bound::Seconds(seconds) => Some(*seconds),
            Bound::Steps(_) => None,
        })
        .collect::<Vec<_>>();
    if seconds.is_empty() {
        if let Some(least) = steps.iter().min() {
            kept.retain(|index| {
                matches!(
                    at(*index).and_then(|proof| bound(proof.basis)),
                    Some(Bound::Steps(depth)) if depth == *least
                )
            });
        }
    } else if steps.is_empty() {
        if let Some(least) = seconds.iter().min() {
            kept.retain(|index| {
                matches!(
                    at(*index).and_then(|proof| bound(proof.basis)),
                    Some(Bound::Seconds(held)) if held == *least
                )
            });
        }
    }
    narrow(&mut kept, |index| {
        at(index).is_some_and(|proof| {
            proof.constrained || matches!(proof.basis, ProofBasis::Reduced { .. })
        })
    });
    kept.first().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsl_replay::{SolverHorizon, SolverMethod, WitnessSource};
    use ix_trace_rs::trace;
    use std::collections::BTreeSet;
    use Certification::{Certified, Trusted, Uncertified};

    fn bounded(depth: u64) -> ProofBasis {
        ProofBasis::BoundedComplete { depth }
    }

    fn reduced() -> ProofBasis {
        ProofBasis::Reduced {
            reductions: Vec::new(),
        }
    }

    /// The instance, 0 for `p` and 1 for `q`, a proved item over the two
    /// takes its record from.
    fn pick(p: (ProofBasis, Certification), q: (ProofBasis, Certification)) -> usize {
        pick_constrained(p, q, false)
    }

    fn pick_constrained(
        p: (ProofBasis, Certification),
        q: (ProofBasis, Certification),
        q_constrained: bool,
    ) -> usize {
        weakest(&[
            InstanceProof {
                basis: &p.0,
                certification: p.1,
                constrained: false,
            },
            InstanceProof {
                basis: &q.0,
                certification: q.1,
                constrained: q_constrained,
            },
        ])
    }

    /// FR-621: the first refuted instance decides, whatever came before it;
    /// else the first instance that is not proved.
    #[trace("TC-3219", "FR-621-AC-9")]
    #[test]
    fn a_refuted_instance_decides_before_an_earlier_unproved_one() {
        let method = StateGraphMethod {
            limits: Default::default(),
            max_depth: None,
            seed: 0,
        };
        let EngineReport::StateGraph(report) = undecided(InconclusiveCause::NoInstance, &method, 0)
        else {
            panic!("a state-graph report");
        };
        let record = |value: TerminalValue| StateGraphRecord {
            value,
            ..report.record.clone()
        };
        let open = || record(TerminalValue::Inconclusive(InconclusiveCause::NoInstance));
        let refuted = || record(TerminalValue::Refuted);
        let proved = || {
            record(TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certified,
            })
        };
        assert_eq!(decisive(&[open(), refuted(), refuted()]), 1);
        assert_eq!(decisive(&[proved(), open(), open()]), 1);
        assert_eq!(decisive(&[proved(), proved()]), 0);
    }

    #[trace("TC-3219", "FR-621-AC-25")]
    #[test]
    fn a_proved_item_rests_on_its_weakest_instance() {
        let exhaustive = || ProofBasis::Exhaustive;
        let inductive = || ProofBasis::Inductive { depth: 1 };
        let witness = || ProofBasis::Witness {
            sources: vec![WitnessSource::Explored],
        };
        let solver = ProofBasis::BoundedSolver {
            method: SolverMethod::FlowpipeOutwardRounded,
            horizon: SolverHorizon::new(
                Rational::new(3_i64.into(), 1_i64.into()).expect("nonzero"),
            )
            .expect("a horizon"),
            jumps: 1,
            exhaustive: BTreeSet::new(),
        };
        // (a) to (l) of FR-621-AC-25, `p` first.
        assert_eq!(
            pick((exhaustive(), Certified), (exhaustive(), Uncertified)),
            1
        );
        assert_eq!(
            pick((bounded(3), Certified), (exhaustive(), Uncertified)),
            1
        );
        assert_eq!(
            pick((exhaustive(), Uncertified), (exhaustive(), Trusted)),
            1
        );
        assert_eq!(pick((reduced(), Uncertified), (bounded(5), Uncertified)), 1);
        assert_eq!(pick((bounded(5), Certified), (bounded(3), Certified)), 1);
        assert_eq!(
            pick((exhaustive(), Uncertified), (reduced(), Uncertified)),
            1
        );
        assert_eq!(
            pick((exhaustive(), Certified), (exhaustive(), Certified)),
            0
        );
        assert_eq!(pick((exhaustive(), Certified), (inductive(), Certified)), 0);
        assert_eq!(pick((inductive(), Certified), (bounded(5), Certified)), 1);
        assert_eq!(pick((witness(), Certified), (bounded(5), Certified)), 1);
        assert_eq!(
            pick_constrained(
                (exhaustive(), Uncertified),
                (exhaustive(), Uncertified),
                true
            ),
            1
        );
        assert_eq!(pick((bounded(5), Uncertified), (solver, Uncertified)), 0);
    }
}
