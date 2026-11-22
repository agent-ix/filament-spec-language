// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487 "The clause seams": the production [`StepMatch`] of a checked hyper
//! clause, which evaluates each conjunct of `match` through the one clause
//! evaluator (FR-107).

use filament_eval::CallFailure;
use filament_exact::Meter;
use filament_semantic_value::catalog::InternalFault;
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedHyperClause;

use crate::hyper::{
    MatchIncomplete, MatchPart, MatchReading, ParameterBinding, StepLabel, StepMatch,
};
use crate::simulation::model::ModelEvaluator;
use crate::simulation::model::{verdict, ClauseOutcome, Verdict};

/// The `match` conjuncts of one checked hyper clause, evaluated over the
/// labels of a joint step.
///
/// The conjuncts of a part are evaluated in index order, each once, sharing
/// the one `meter`, and the first that is not `Completed(true)` ends the
/// reading: a conjunct after it is neither evaluated nor charged, as FR-107
/// skips the right operand of a decided `and`. A step label is a transition
/// identity, so a conjunct reads no state and the instance is not read
/// (ADR-023 HS-5).
pub struct CheckedStepMatch<'s> {
    package: &'s dyn ModelEvaluator,
    clause: &'s CheckedHyperClause,
}

impl<'s> CheckedStepMatch<'s> {
    /// The `match` of `clause`, evaluated through `package`.
    pub fn new(package: &'s CheckedPackage, clause: &'s CheckedHyperClause) -> Self {
        Self::for_evaluator(package, clause)
    }

    /// Evaluate matches through the authenticated source or package evaluator.
    pub fn for_evaluator(package: &'s dyn ModelEvaluator, clause: &'s CheckedHyperClause) -> Self {
        Self { package, clause }
    }

    fn read(
        &self,
        part: MatchPart,
        labels: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        let conjuncts = match part {
            MatchPart::Universal => self.clause.mu_universal(),
            MatchPart::Existential => self.clause.mu_existential(),
        };
        // A conjunct that reads a component whose closed trace has ended is not
        // evaluated at this joint step (FR-173 "Event-position form").
        let ended: Vec<&str> = self
            .clause
            .quantifiers()
            .iter()
            .zip(labels)
            .filter(|(_, label)| matches!(label, StepLabel::Ended))
            .map(|(quantifier, _)| quantifier.variable.as_str())
            .collect();
        for (conjunct, draft) in conjuncts.iter().enumerate() {
            if draft
                .variables()
                .iter()
                .any(|variable| ended.contains(&variable.as_str()))
            {
                continue;
            }
            let evaluation =
                match self
                    .package
                    .evaluate_hyper_match(self.clause, part, conjunct, labels, meter)
                {
                    Ok(evaluation) => evaluation,
                    Err(CallFailure::Fault(fault)) => return Ok(MatchReading::Fault(fault)),
                    Err(_) => {
                        return Ok(MatchReading::Fault(InternalFault::new(
                            "hyper-match",
                            "a-conjunct-evaluates",
                        )))
                    }
                };
            match verdict(&evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => return Ok(MatchReading::Fails),
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        return Ok(MatchReading::Undefined { conjunct, cause });
                    }
                    return match outcome {
                        ClauseOutcome::Incomplete(incomplete) => Err(MatchIncomplete {
                            conjunct,
                            incomplete,
                        }),
                        outcome => Ok(MatchReading::Refused { conjunct, outcome }),
                    };
                }
            }
        }
        Ok(MatchReading::Holds)
    }
}

impl StepMatch for CheckedStepMatch<'_> {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        self.read(MatchPart::Universal, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        self.read(MatchPart::Existential, steps, meter)
    }
}
