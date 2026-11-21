// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-194 "Evaluation": the [`WindowSource`] that reads a window aggregate's
//! filter, value and predicate from record samples, one per represented
//! position, with the clause's subject (and trigger) binder bound to the
//! sample and each fixed capture after it. The finite-window route reads a
//! duration window against the trace's time stamps through it.

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Meter, Outcome, Rational, Value};
use filament_semantic_value::catalog::InternalFault;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedAggregateConditionsRef, CheckedNode, StatePredicate};

use super::window::{WindowRead, WindowSource};
use super::{LetterStop, TemporalPosition, UndefinedCause};

fn fault(code: &'static str) -> LetterStop {
    LetterStop::Fault(InternalFault::new("temporal.observed-aggregate", code))
}

/// One checked expression as a state predicate of the clause evaluator.
pub(super) struct Expression<'n> {
    pub(super) body: CheckedNode<'n>,
    pub(super) slots: usize,
}

impl StatePredicate for Expression<'_> {
    fn body(&self) -> CheckedNode<'_> {
        self.body
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

/// What reading one expression gave.
pub(super) enum Reading {
    Defined(Value),
    Undefined(UndefinedCause),
}

/// Evaluate `expression` with `bindings` bound after the clause's `over`
/// parameter. A refused or failed evaluation is returned whole in
/// `refusal`; the run then stops.
pub(super) fn read(
    package: &CheckedPackage,
    expression: (CheckedNode<'_>, usize),
    bindings: Vec<Value>,
    refusal: &mut Option<filament_eval::Evaluation>,
    meter: &mut Meter,
) -> Result<Reading, LetterStop> {
    let (body, slots) = expression;
    let evaluation = evaluate_expression(package, &Expression { body, slots }, bindings, meter)?;
    if matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
    ) {
        refusal.get_or_insert(evaluation);
        return Err(LetterStop::Refused);
    }
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(Reading::Defined(value)),
        FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
            Ok(Reading::Undefined(UndefinedCause::Kernel(undefined)))
        }
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            Err(LetterStop::Incomplete(incomplete))
        }
        FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(Reading::Undefined(
            UndefinedCause::Family(cause.undefined_record()),
        )),
        FamilyOutcome::Evaluated(Outcome::Refused(_))
        | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(LetterStop::Refused),
        #[cfg(seam_probe_eval_downstream)]
        FamilyOutcome::__SeamProbe => unreachable!("never constructed outside the probe build"),
    }
}

fn evaluate_expression(
    package: &CheckedPackage,
    predicate: &Expression<'_>,
    bindings: Vec<Value>,
    meter: &mut Meter,
) -> Result<filament_eval::Evaluation, LetterStop> {
    let environment = Default::default();
    crate::value::evaluate_state_predicate_with(
        package,
        predicate,
        &environment,
        None,
        bindings,
        meter,
    )
    .map_err(|_| fault("aggregate-expression-call"))
}

/// The magnitude of a numeric value: an integer, a rational or a quantity
/// (its magnitude, in the unit the checker converted the threshold into).
pub(super) fn magnitude(value: Value) -> Result<Rational, LetterStop> {
    match value {
        Value::Integer(value) => Ok(Rational::from_integer(value)),
        Value::Rational(value) => Ok(value),
        Value::Quantity(quantity) => Ok(quantity.magnitude().clone()),
        _ => Err(fault("aggregate-value-is-number")),
    }
}

/// The record samples a window reads: listed, or held by value on a carried
/// clause state's tape (FR-820).
#[derive(Clone, Copy)]
pub(super) enum Held<'a> {
    Listed(&'a [Value]),
    Tape(&'a super::tape::LetterTape),
}

impl<'a> Held<'a> {
    /// The sample at `slot`; a position the tape read into a letter holds none.
    pub(super) fn get(self, slot: usize) -> Option<&'a Value> {
        match self {
            Self::Listed(samples) => samples.get(slot),
            Self::Tape(tape) => match tape.entries().get(slot)? {
                super::tape::TapeEntry::Held(sample)
                | super::tape::TapeEntry::Anchored(sample, _) => Some(sample),
                super::tape::TapeEntry::Read(_) | super::tape::TapeEntry::Object(_) => None,
            },
        }
    }
}

/// A [`WindowSource`] over the record samples of a trace.
pub(super) struct SampleWindowSource<'a> {
    pub(super) package: &'a CheckedPackage,
    pub(super) expressions: CheckedAggregateConditionsRef<'a>,
    pub(super) samples: Held<'a>,
    /// Whether the clause has a trigger binder, bound to the sample too.
    pub(super) trigger: bool,
    /// The captures the activation fixed.
    pub(super) captures: &'a [Value],
    /// The first refused evaluation, kept whole.
    pub(super) refused: Option<filament_eval::Evaluation>,
}

impl SampleWindowSource<'_> {
    fn read(
        &mut self,
        expression: Option<(CheckedNode<'_>, usize)>,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Value>, LetterStop> {
        let expression = expression.ok_or_else(|| fault("aggregate-expression-present"))?;
        let Some(sample) = usize::try_from(position.index())
            .ok()
            .and_then(|slot| self.samples.get(slot))
        else {
            return Ok(WindowRead::Missing);
        };
        let mut bindings = vec![sample.clone()];
        if self.trigger {
            bindings.push(sample.clone());
        }
        bindings.extend(self.captures.iter().cloned());
        Ok(
            match read(self.package, expression, bindings, &mut self.refused, meter)? {
                Reading::Defined(value) => WindowRead::Defined(value),
                Reading::Undefined(cause) => WindowRead::Undefined(cause),
            },
        )
    }

    fn boolean(
        &mut self,
        expression: Option<(CheckedNode<'_>, usize)>,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        match self.read(expression, position, meter)? {
            WindowRead::Defined(Value::Boolean(held)) => Ok(WindowRead::Defined(held)),
            WindowRead::Defined(_) => Err(fault("aggregate-condition-is-boolean")),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
            WindowRead::Missing => Ok(WindowRead::Missing),
        }
    }
}

impl WindowSource for SampleWindowSource<'_> {
    fn admits(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        match self.expressions.filter {
            None => Ok(WindowRead::Defined(true)),
            filter => self.boolean(filter, position, meter),
        }
    }

    fn value(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<Rational>, LetterStop> {
        let expression = self.expressions.value;
        match self.read(expression, position, meter)? {
            WindowRead::Defined(value) => Ok(WindowRead::Defined(magnitude(value)?)),
            WindowRead::Undefined(cause) => Ok(WindowRead::Undefined(cause)),
            WindowRead::Missing => Ok(WindowRead::Missing),
        }
    }

    fn predicate(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<WindowRead<bool>, LetterStop> {
        let expression = self.expressions.predicate;
        self.boolean(expression, position, meter)
    }
}
