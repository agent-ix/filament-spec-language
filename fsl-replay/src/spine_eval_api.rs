// SPDX-License-Identifier: AGPL-3.0-or-later
//! Names for evaluator-owned types at the replay facade boundary.
//!
//! The spine exposes these through its own module so its public syntax does
//! not depend on `fsl_eval` paths (FR-100-AC-8).

/// A single atom's temporal reading.
pub type SpineEvalAtomReading = fsl_eval::temporal::AtomReading;
/// A valuation observed over a trace.
pub type SpineEvalObservedValuation = fsl_eval::temporal::ObservedValuation;
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
