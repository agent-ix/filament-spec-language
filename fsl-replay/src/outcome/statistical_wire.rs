// SPDX-License-Identifier: AGPL-3.0-or-later
//! One FR-410 projection, with the existing native or genuine scoped admission.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};
use filament_exact::{ObjectReference, Rational};
use serde_json::{Map, Value};

trait Write {
    type Error;
    fn work(&mut self, units: usize) -> Result<(), Self::Error>;
    fn text(&mut self, value: &str) -> Result<Value, Self::Error>;
    fn display(&mut self, value: &impl std::fmt::Display) -> Result<Value, Self::Error>;
    fn rational(&mut self, value: &Rational) -> Result<Value, Self::Error>;
    fn reference(&mut self, value: &ObjectReference) -> Result<Value, Self::Error>;
    fn state(&mut self, value: &fsl_eval::simulation::StateKey) -> Result<Value, Self::Error>;
    fn array(&mut self, len: usize) -> Result<Vec<Value>, Self::Error>;
    fn record<const N: usize>(&mut self, fields: [(&str, Value); N]) -> Result<Value, Self::Error>;
    fn put(&mut self, record: &mut Value, key: &str, value: Value) -> Result<(), Self::Error>;
    fn at<T>(
        &mut self,
        pointer: &str,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
    fn index<T>(
        &mut self,
        pointer: &str,
        index: usize,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
    fn missing(&mut self, member: &'static str) -> Self::Error;
}

pub(super) struct Native;
impl Write for Native {
    type Error = CounterexampleWireError;
    fn work(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<Value, Self::Error> {
        Ok(Value::String(value.to_owned()))
    }
    fn display(&mut self, value: &impl std::fmt::Display) -> Result<Value, Self::Error> {
        Ok(Value::String(value.to_string()))
    }
    fn rational(&mut self, value: &Rational) -> Result<Value, Self::Error> {
        Ok(super::rational_wire(value))
    }
    fn reference(&mut self, value: &ObjectReference) -> Result<Value, Self::Error> {
        counterexample::binding_wire(Some(value))
    }
    fn state(&mut self, value: &fsl_eval::simulation::StateKey) -> Result<Value, Self::Error> {
        counterexample::state_key_wire(value)
    }
    fn array(&mut self, len: usize) -> Result<Vec<Value>, Self::Error> {
        Ok(Vec::with_capacity(len))
    }
    fn record<const N: usize>(&mut self, fields: [(&str, Value); N]) -> Result<Value, Self::Error> {
        Ok(Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect::<Map<_, _>>(),
        ))
    }
    fn put(&mut self, record: &mut Value, key: &str, value: Value) -> Result<(), Self::Error> {
        let map = record
            .as_object_mut()
            .ok_or(CounterexampleWireError::Unspelled("statistical record"))?;
        map.insert(key.to_owned(), value);
        Ok(())
    }
    fn at<T>(
        &mut self,
        _: &str,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error> {
        write(self)
    }
    fn index<T>(
        &mut self,
        _: &str,
        _: usize,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error> {
        write(self)
    }
    fn missing(&mut self, member: &'static str) -> Self::Error {
        CounterexampleWireError::Unspelled(member)
    }
}

impl Write for ReferenceScope<'_, '_, '_, '_> {
    type Error = ReferenceCodecError;
    fn work(&mut self, units: usize) -> Result<(), Self::Error> {
        self.work(units)
    }
    fn text(&mut self, value: &str) -> Result<Value, Self::Error> {
        self.text(value)
    }
    fn display(&mut self, value: &impl std::fmt::Display) -> Result<Value, Self::Error> {
        self.display(value)
    }
    fn rational(&mut self, value: &Rational) -> Result<Value, Self::Error> {
        self.rational(value)
    }
    fn reference(&mut self, value: &ObjectReference) -> Result<Value, Self::Error> {
        self.encode_reference(value)
    }
    fn state(&mut self, value: &fsl_eval::simulation::StateKey) -> Result<Value, Self::Error> {
        self.state(value)
    }
    fn array(&mut self, len: usize) -> Result<Vec<Value>, Self::Error> {
        self.array(len)
    }
    fn record<const N: usize>(&mut self, fields: [(&str, Value); N]) -> Result<Value, Self::Error> {
        self.record(fields)
    }
    fn put(&mut self, record: &mut Value, key: &str, value: Value) -> Result<(), Self::Error> {
        self.put(record, key, value)
    }
    fn at<T>(
        &mut self,
        pointer: &str,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error> {
        self.at_pointer(pointer, write)
    }
    fn index<T>(
        &mut self,
        pointer: &str,
        index: usize,
        write: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error> {
        self.at_index(pointer, index, write)
    }
    fn missing(&mut self, member: &'static str) -> Self::Error {
        self.carrier_refusal(member)
    }
}

fn decision<W: Write>(
    w: &mut W,
    decision: Decision,
    undefined: Option<&statistical::SampledUndefined>,
) -> Result<Value, W::Error> {
    let tag = w.text(match decision {
        Decision::Accepted => "accepted",
        Decision::Rejected => "rejected",
        Decision::Undecided(_) => "undecided",
    })?;
    let mut out = w.record([("type", tag)])?;
    if let Decision::Undecided(cause) = decision {
        let tag = w.text(match cause {
            statistical::UndecidedCause::IndifferenceRegion => "indifference-region",
            statistical::UndecidedCause::UndecidedSuccessor => "undecided-successor",
        })?;
        let cause = w.record([("type", tag)])?;
        w.put(&mut out, "cause", cause)?;
    } else if decision == Decision::Rejected {
        if let Some(found) = undefined {
            let cause = w.at("/cause", |w| {
                let state = w.at("/where/state_key", |w| {
                    w.state(&found.evaluation.site.state_key)
                })?;
                let trace = w.display(&found.trace)?;
                let at = w.record([
                    ("position", Value::from(found.evaluation.site.position)),
                    ("state_key", state),
                    ("test", Value::from(found.test)),
                    ("trace_index", trace),
                ])?;
                let tag = w.text("undefined-evaluation")?;
                let reason = w.text(super::undefined_reason(&found.evaluation.cause))?;
                w.record([("type", tag), ("where", at), ("cause", reason)])
            })?;
            w.put(&mut out, "cause", cause)?;
        }
    }
    Ok(out)
}

fn estimate<W: Write>(
    w: &mut W,
    value: Option<&Rational>,
    quantile: Option<&statistical::MeasureValue>,
) -> Result<Option<Value>, W::Error> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = w.at("/value", |w| w.rational(value))?;
    let mut out = w.record([("value", value)])?;
    if let Some(statistical::MeasureValue::Value(quantile)) = quantile {
        let value = w.at("/quantile/value", |w| w.rational(quantile))?;
        let quantile = w.record([("value", value)])?;
        w.put(&mut out, "quantile", quantile)?;
    }
    Ok(Some(out))
}

fn interval<W: Write>(
    w: &mut W,
    value: Option<&(Rational, Rational)>,
) -> Result<Option<Value>, W::Error> {
    let Some((lower, upper)) = value else {
        return Ok(None);
    };
    let lower = w.at("/lower", |w| w.rational(lower))?;
    let upper = w.at("/upper", |w| w.rational(upper))?;
    Ok(Some(w.record([("lower", lower), ("upper", upper)])?))
}

fn write<W: Write>(verdict: &StatisticalVerdict, w: &mut W) -> Result<Value, W::Error> {
    w.work(verdict.tests.len())?;
    let rejection = verdict
        .tests
        .iter()
        .find_map(|test| test.undefined.as_ref());
    let accepted = w.at("/decision", |w| decision(w, verdict.decision, rejection))?;
    let mut tests = w.array(verdict.tests.len())?;
    for (index, test) in verdict.tests.iter().enumerate() {
        tests.push(w.index("/tests", index, |w| {
            let decision = w.at("/decision", |w| {
                decision(w, test.decision, test.undefined.as_ref())
            })?;
            let mut out = w.record([
                ("index", Value::from(test.index)),
                ("decision", decision),
                ("samples", Value::from(test.samples)),
                ("draws", Value::from(test.draws)),
            ])?;
            match w.at("/estimate", |w| {
                estimate(w, test.estimate.as_ref(), test.quantile.as_ref())
            })? {
                Some(estimate) => w.put(&mut out, "estimate", estimate)?,
                None if test.undefined.is_some()
                    || test.decision
                        == Decision::Undecided(statistical::UndecidedCause::UndecidedSuccessor) => {
                }
                None => return Err(w.missing("test estimate")),
            }
            if let Some(interval) = w.at("/interval", |w| interval(w, test.interval.as_ref()))? {
                w.put(&mut out, "interval", interval)?;
            }
            Ok(out)
        })?);
    }
    let mut provenance_tests = w.array(verdict.provenance.tests.len())?;
    for (index, test) in verdict.provenance.tests.iter().enumerate() {
        provenance_tests.push(w.index("/provenance/tests", index, |w| {
            w.work(1)?;
            let last = test.traces.first.saturating_add(
                test.traces
                    .count
                    .saturating_sub(1)
                    .saturating_mul(test.traces.step),
            );
            let binding = match &test.reference {
                Some(reference) => w.at("/binding", |w| w.reference(reference))?,
                None => Value::Null,
            };
            let first = w.display(&test.traces.first)?;
            let last = w.display(&last)?;
            w.record([
                ("index", Value::from(test.index)),
                ("initial_state", Value::from(test.initial)),
                ("binding", binding),
                ("instance", Value::Null),
                ("first_trace_index", first),
                ("last_trace_index", last),
            ])
        })?);
    }
    let basis = w.at("/basis", |w| {
        let tag = w.text(match verdict.basis.method {
            statistical::StatisticalMethod::Okamoto => "okamoto",
            statistical::StatisticalMethod::Sprt => "sprt",
            statistical::StatisticalMethod::ClopperPearson => "clopper-pearson",
            statistical::StatisticalMethod::Regenerative { .. } => "regenerative",
        })?;
        let mut method = w.record([("type", tag)])?;
        if let statistical::StatisticalMethod::Regenerative { min_cycles } = verdict.basis.method {
            w.put(&mut method, "min_cycles", Value::from(min_cycles))?;
        }
        let confidence = w.at("/confidence", |w| {
            let alpha = w.at("/alpha", |w| w.rational(verdict.basis.confidence.alpha()))?;
            let beta = w.at("/beta", |w| w.rational(verdict.basis.confidence.beta()))?;
            let iota = w.at("/iota", |w| {
                w.rational(verdict.basis.confidence.indifference())
            })?;
            w.record([("alpha", alpha), ("beta", beta), ("iota", iota)])
        })?;
        let per_test = w.at("/tests", |w| {
            let alpha = w.at("/alpha", |w| w.rational(&verdict.basis.alpha_per_test))?;
            let beta = w.at("/beta", |w| w.rational(&verdict.basis.beta_per_test))?;
            w.record([
                ("m", Value::from(verdict.basis.tests)),
                ("alpha", alpha),
                ("beta", beta),
            ])
        })?;
        let coverage = w.text(match verdict.basis.coverage {
            statistical::Coverage::FiniteSample => "finite-sample",
            statistical::Coverage::Asymptotic => "asymptotic",
        })?;
        let mut basis = w.record([
            ("method", method),
            ("confidence", confidence),
            ("samples", Value::from(verdict.basis.samples)),
            ("draws", Value::from(verdict.basis.draws)),
            ("tests", per_test),
            ("coverage", coverage),
        ])?;
        if let Some(grid) = verdict.basis.delay_grid_bits {
            w.put(&mut basis, "grid", Value::from(grid))?;
        }
        Ok(basis)
    })?;
    let seed = w.display(&verdict.provenance.seed)?;
    let sampler = w.text("filament.simulation.sampler/v1")?;
    let provenance = w.record([
        ("seed", seed),
        ("sampler", sampler),
        ("tests", Value::Array(provenance_tests)),
    ])?;
    let witnesses = w.array(0)?;
    let mut out = w.record([
        ("decision", accepted),
        ("basis", basis),
        ("provenance", provenance),
        ("tests", Value::Array(tests)),
        ("witnesses", Value::Array(witnesses)),
    ])?;
    match w.at("/estimate", |w| {
        estimate(w, verdict.estimate.as_ref(), verdict.quantile.as_ref())
    })? {
        Some(estimate) => w.put(&mut out, "estimate", estimate)?,
        None if rejection.is_some()
            || verdict.decision
                == Decision::Undecided(statistical::UndecidedCause::UndecidedSuccessor) => {}
        None => return Err(w.missing("verdict estimate")),
    }
    if let Some(interval) = w.at("/interval", |w| interval(w, verdict.interval.as_ref()))? {
        w.put(&mut out, "interval", interval)?;
    }
    Ok(out)
}

pub(super) fn native(verdict: &StatisticalVerdict) -> Result<Value, CounterexampleWireError> {
    write(verdict, &mut Native)
}

impl ReferenceScope<'_, '_, '_, '_> {
    /// Write a full FR-410 verdict while borrowing its genuine admitted runtime.
    /// Every provenance binding uses the authoritative shared reference codec.
    ///
    /// # Errors
    /// Preserves typed reference, metadata, original owner and cancellation failures.
    pub fn statistical_verdict_wire(
        &mut self,
        verdict: &StatisticalVerdict,
    ) -> Result<Value, ReferenceCodecError> {
        write(verdict, self)
    }
}
