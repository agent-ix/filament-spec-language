// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal values exposed at the replay facade boundary.
//!
//! The spine exposes replay-owned valuations alongside its temporal value
//! names, keeping evaluator paths outside the spine surface (FR-100-AC-8).

/// A single atom's temporal reading.
pub type SpineEvalAtomReading = fsl_eval::temporal::AtomReading;
/// A valuation observed over a trace.
#[derive(Debug)]
pub struct SpineEvalObservedValuation {
    /// The clause's atoms in arena order.
    pub atoms: Vec<fsl_semantics::check::FormulaNodeId>,
    /// The atom readings at each represented position.
    pub readings: Vec<Vec<SpineEvalAtomReading>>,
    /// Each activation and its native outcome.
    pub activations: Vec<(
        fsl_eval::temporal::TemporalPosition,
        SpineEvalTemporalOutcome,
    )>,
    /// The finite or lasso trace represented by the readings.
    pub trace: fsl_eval::temporal::TemporalTrace,
}

impl From<fsl_eval::temporal::ObservedValuation> for SpineEvalObservedValuation {
    fn from(value: fsl_eval::temporal::ObservedValuation) -> Self {
        Self {
            atoms: value.atoms,
            readings: value.readings,
            activations: value.activations,
            trace: value.trace,
        }
    }
}
/// The result of evaluating a temporal clause.
pub type SpineEvalTemporalOutcome = fsl_eval::temporal::TemporalOutcome;
/// A refusal produced while evaluating a temporal clause.
pub type SpineEvalTemporalRefusal = fsl_eval::temporal::TemporalRefusal;
/// A decisive atom retained by temporal evaluation.
pub type SpineEvalDecisiveAtom = fsl_eval::temporal::DecisiveAtom;
/// A clause's retained temporal letter tape.
pub type SpineEvalLetterTape = fsl_eval::temporal::LetterTape;
/// A stream opened for a temporal clause.
pub type SpineEvalClauseStream = fsl_eval::temporal::ClauseStream;
/// The first undefined reading retained by a temporal evaluation.
pub type SpineEvalFirstUndefined = fsl_eval::temporal::FirstUndefined;
/// One retained entry in a temporal letter tape.
pub type SpineEvalTapeEntry = fsl_eval::temporal::TapeEntry;
/// The carried finite-window state of a temporal evaluation.
pub type SpineEvalCarriedWindow = fsl_eval::temporal::CarriedWindow;
/// The evaluator's covered-position set.
pub type SpineEvalCoveredList = fsl_eval::temporal::CoveredList;
/// An interval stamp used by late temporal inputs.
pub type SpineEvalStampInterval = fsl_eval::temporal::StampInterval;
/// A cutoff used by late temporal inputs.
pub type SpineEvalLateCutoff = fsl_eval::temporal::LateCutoff;
/// Results for evaluating value invariants.
pub type SpineEvalValueInvariantEvaluation = fsl_eval::value::ValueInvariantEvaluation;
