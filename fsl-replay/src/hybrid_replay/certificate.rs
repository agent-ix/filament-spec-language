// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact interval checks for FSpec FR-652 hybrid certificates.

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{
    CheckedGraph, CheckedHybridClaim, CheckedHybridModel, HybridActivation, HybridClock,
    HybridFormula, HybridTerm,
};
use serde_json::Value;

use super::expand::{Expander, ExpansionGap};
use super::interval::{IValue, Interval, IntervalEvaluator, Tri};
use super::{HybridReplayMeter, HybridReplayRefusal, SolverMethod};

/// The result of checking one provider-supplied hybrid certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridCertificateCheck {
    /// The certificate proves its claim or validated witness.
    Accepted,
    /// The first failed rule and the certificate member that failed.
    Rejected {
        /// FSpec FR-652 rule name.
        rule: String,
        /// RFC 6901 pointer into the certificate.
        at: String,
    },
}

/// A malformed certificate or an expression the checked model cannot read.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HybridCertificateRefusal {
    /// The certificate's form cannot establish the provider's result.
    #[error("hybrid certificate form does not match its result")]
    FormMismatch,
    /// A certificate member has the wrong shape or a noncanonical number.
    #[error("malformed hybrid certificate at {0}")]
    Malformed(String),
    /// The checked graph does not hold the claim's hybrid model.
    #[error("hybrid claim names no checked model")]
    UnknownModel,
    /// The checker could not evaluate a term in the admitted model.
    #[error("hybrid certificate term could not be evaluated")]
    Evaluation,
}

/// Check a provider certificate against the checked model and claim.
///
/// The caller supplies the result spelling so a certificate form cannot
/// certify an unrelated settlement. `meter` charges against the item's
/// shared work budget, including its subsequent replay.
pub fn check_hybrid_certificate(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    result: &str,
    method: Option<SolverMethod>,
    certificate: &Value,
    trace: Option<&super::HybridTrace>,
    meter: &mut HybridReplayMeter,
) -> Result<HybridCertificateCheck, HybridCertificateRefusal> {
    let model = graph
        .hybrid_models()
        .get(claim.model.get())
        .ok_or(HybridCertificateRefusal::UnknownModel)?;
    let kind = certificate
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| HybridCertificateRefusal::Malformed("/certificate/type".to_owned()))?;
    let checked = match (result, method, kind) {
        ("proved", Some(SolverMethod::InductiveEnclosure), "inductive-enclosure") => {
            check_inductive(graph, claim, model, certificate, meter)
        }
        ("proved", Some(SolverMethod::FlowpipeOutwardRounded), "flowpipe") => {
            check_flowpipe(graph, claim, model, certificate, meter)
        }
        ("refuted" | "inconclusive", _, "validated-witness") => {
            check_witness(graph, claim, model, certificate, trace, meter)
        }
        ("proved", _, "validated-witness")
        | ("refuted" | "inconclusive", _, "inductive-enclosure")
        | ("proved", _, "inductive-enclosure")
        | ("refuted" | "inconclusive", _, "flowpipe") => {
            Err(HybridCertificateRefusal::FormMismatch)
        }
        _ => Err(HybridCertificateRefusal::Malformed(
            "/certificate/type".to_owned(),
        )),
    };
    match checked {
        Err(HybridCertificateRefusal::Malformed(pointer)) if pointer == "work-limit" => {
            Ok(rejected("work-limit", "/certificate"))
        }
        other => other,
    }
}

fn check_inductive(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    model: &CheckedHybridModel,
    certificate: &Value,
    meter: &mut HybridReplayMeter,
) -> Result<HybridCertificateCheck, HybridCertificateRefusal> {
    let boxes = certificate
        .get("boxes")
        .and_then(Value::as_object)
        .ok_or_else(|| HybridCertificateRefusal::Malformed("/certificate/boxes".to_owned()))?;
    if boxes.len() != model.modes.len() {
        return Ok(rejected("box-shape", "/certificate/boxes"));
    }
    let mut mode_boxes = Vec::with_capacity(model.modes.len());
    for mode in &model.modes {
        let Some(raw) = boxes.get(mode.name.as_str()) else {
            return Ok(rejected(
                "box-shape",
                &format!("/certificate/boxes/{}", mode.name),
            ));
        };
        let Some(variables) = raw.as_object() else {
            return Ok(rejected(
                "box-shape",
                &format!("/certificate/boxes/{}", mode.name),
            ));
        };
        if variables.len() != model.variables.len() {
            return Ok(rejected(
                "box-shape",
                &format!("/certificate/boxes/{}", mode.name),
            ));
        }
        let mut intervals = Vec::with_capacity(model.variables.len());
        for variable in &model.variables {
            let pointer = format!("/certificate/boxes/{}/{}", mode.name, variable.name);
            let Some(value) = variables.get(variable.name.as_str()) else {
                return Ok(rejected("box-shape", &pointer));
            };
            let interval = read_interval(value)
                .ok_or_else(|| HybridCertificateRefusal::Malformed(pointer.clone()))?;
            if interval
                .lower
                .as_ref()
                .zip(interval.upper.as_ref())
                .is_some_and(|(lower, upper)| lower > upper)
            {
                return Ok(rejected("box-shape", &pointer));
            }
            intervals.push(interval);
        }
        mode_boxes.push(intervals);
    }

    for initial in &model.initial {
        let mode_index = initial.mode.get();
        let Some(boxed) = mode_boxes.get(mode_index) else {
            return Ok(rejected("initial", "/certificate/boxes"));
        };
        let invariant = &model.modes[mode_index].invariant;
        if truth(graph, &initial.condition, boxed, meter)? != Tri::True
            || truth(graph, invariant, boxed, meter)? != Tri::True
        {
            return Ok(rejected(
                "a",
                &format!("/certificate/boxes/{}", model.modes[mode_index].name),
            ));
        }
        // An initial condition or invariant point just outside any face must
        // not be admitted. This also detects a box that clips a closed initial
        // boundary, such as v = 20 in Braking.
        for variable in 0..model.variables.len() {
            for (end, direction) in [(false, -1_i64), (true, 1_i64)] {
                let Some(bound) = (if end {
                    boxed[variable].upper.as_ref()
                } else {
                    boxed[variable].lower.as_ref()
                }) else {
                    continue;
                };
                let outside = bound.add(&Rational::from_integer(Integer::from(direction)));
                let mut point = boxed.to_vec();
                point[variable] = Interval::point(outside);
                if truth(graph, &initial.condition, &point, meter)? == Tri::True
                    && truth(graph, invariant, &point, meter)? == Tri::True
                {
                    return Ok(rejected(
                        "a",
                        &format!("/certificate/boxes/{}", model.modes[mode_index].name),
                    ));
                }
            }
        }
    }

    for (mode_index, mode) in model.modes.iter().enumerate() {
        let boxed = &mode_boxes[mode_index];
        if truth(graph, &mode.invariant, boxed, meter)? != Tri::True {
            return Ok(rejected("b", &format!("/certificate/boxes/{}", mode.name)));
        }
        for (variable, rate) in mode.rates.iter().enumerate() {
            for upper_face in [false, true] {
                let Some(bound) = (if upper_face {
                    boxed[variable].upper.as_ref()
                } else {
                    boxed[variable].lower.as_ref()
                }) else {
                    continue;
                };
                let mut face = boxed.to_vec();
                face[variable] = Interval::point(bound.clone());
                let flow = value(graph, rate, &face, meter)?;
                let rate = match flow {
                    IValue::Number(rate) => rate,
                    IValue::Boolean(_) => return Err(HybridCertificateRefusal::Evaluation),
                };
                let points_inward = if upper_face {
                    rate.upper
                        .as_ref()
                        .is_some_and(|upper| upper <= &Rational::from_integer(Integer::from(0)))
                } else {
                    rate.lower
                        .as_ref()
                        .is_some_and(|lower| lower >= &Rational::from_integer(Integer::from(0)))
                };
                if !points_inward {
                    let epsilon = Rational::from_integer(Integer::from(1));
                    let outside_value = if upper_face {
                        bound.add(&epsilon)
                    } else {
                        bound.sub(&epsilon)
                    };
                    let mut outside = boxed.to_vec();
                    outside[variable] = Interval::point(outside_value);
                    if truth(graph, &mode.invariant, &outside, meter)? != Tri::False {
                        return Ok(rejected("b", &format!("/certificate/boxes/{}", mode.name)));
                    }
                }
            }
        }
    }

    for (mode_index, mode) in model.modes.iter().enumerate() {
        for transition in &mode.transitions {
            let target = transition.target.get();
            let Some(target_box) = mode_boxes.get(target) else {
                return Ok(rejected("successor", "/certificate/boxes"));
            };
            let source_box = &mode_boxes[mode_index];
            if truth(graph, &transition.guard, source_box, meter)? == Tri::False {
                continue;
            }
            let mut image = source_box.to_vec();
            for reset in &transition.resets {
                let evaluated = value(graph, &reset.value, source_box, meter)?;
                let IValue::Number(value) = evaluated else {
                    return Err(HybridCertificateRefusal::Evaluation);
                };
                image[reset.variable.get()] = value;
            }
            if !subset(&image, target_box) {
                return Ok(rejected(
                    "successor",
                    &format!("/certificate/boxes/{}", mode.name),
                ));
            }
        }
    }

    if !claim_holds(graph, claim, &mode_boxes, meter)? {
        return Ok(rejected("d", "/certificate/boxes"));
    }
    Ok(HybridCertificateCheck::Accepted)
}

fn check_flowpipe(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    model: &CheckedHybridModel,
    certificate: &Value,
    meter: &mut HybridReplayMeter,
) -> Result<HybridCertificateCheck, HybridCertificateRefusal> {
    let steps = certificate
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| HybridCertificateRefusal::Malformed("/certificate/steps".to_owned()))?;
    if steps.is_empty() {
        return Ok(rejected("coverage", "/certificate/steps"));
    }
    let mut parsed = Vec::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let at = format!("/certificate/steps/{index}");
        let mode_name = step
            .get("mode")
            .and_then(Value::as_str)
            .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/mode")))?;
        let Some(mode_index) = model
            .modes
            .iter()
            .position(|mode| mode.name.as_str() == mode_name)
        else {
            return Ok(rejected("box-shape", &format!("{at}/mode")));
        };
        let mode = &model.modes[mode_index];
        let time = step
            .get("time")
            .and_then(read_interval)
            .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/time")))?;
        let delta = step
            .get("step")
            .and_then(read_rational)
            .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/step")))?;
        if delta.numerator().is_negative() {
            return Ok(rejected("box-shape", &format!("{at}/step")));
        }
        let box_ = read_box(step.get("box"), model)
            .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/box")))?;
        let apriori = read_box(step.get("apriori"), model)
            .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/apriori")))?;
        parsed.push((mode_index, time, delta, box_, apriori));
    }

    if parsed[0].1.lower.as_ref() != Some(&zero()) {
        return Ok(rejected("coverage", "/certificate/steps/0/time"));
    }
    for (index, (mode_index, _, delta, box_, apriori)) in parsed.iter().enumerate() {
        let mode = &model.modes[*mode_index];
        if truth(graph, &mode.invariant, apriori, meter)? != Tri::True {
            return Ok(rejected("a", &format!("/certificate/steps/{index}")));
        }
        let mut rates = Vec::with_capacity(mode.rates.len());
        for rate in &mode.rates {
            let IValue::Number(rate) = value(graph, rate, apriori, meter)? else {
                return Err(HybridCertificateRefusal::Evaluation);
            };
            rates.push(rate);
        }
        let mut picard = Vec::with_capacity(box_.len());
        let mut endpoint = Vec::with_capacity(box_.len());
        for (variable, rate) in rates.iter().enumerate() {
            let displacement = scale_nonnegative(rate, delta);
            let tube = add_intervals(&box_[variable], &displacement)
                .ok_or(HybridCertificateRefusal::Evaluation)?;
            let point = add_intervals(&box_[variable], &scale_point(rate, delta))
                .ok_or(HybridCertificateRefusal::Evaluation)?;
            picard.push(contract_boundary(
                graph,
                &mode.invariant,
                variable,
                tube,
                apriori,
                meter,
            )?);
            endpoint.push(contract_boundary(
                graph,
                &mode.invariant,
                variable,
                point,
                apriori,
                meter,
            )?);
        }
        if !subset(&picard, apriori) {
            return Ok(rejected("picard", &format!("/certificate/steps/{index}")));
        }
        if let Some((next_mode_index, _, _, next_box, _)) = parsed.get(index + 1) {
            if next_mode_index == mode_index {
                if !subset(&endpoint, next_box) {
                    return Ok(rejected(
                        "successor",
                        &format!("/certificate/steps/{}", index + 1),
                    ));
                }
            } else {
                let mut image_found = false;
                for transition in &mode.transitions {
                    if transition.target.get() != *next_mode_index
                        || truth(graph, &transition.guard, &picard, meter)? == Tri::False
                    {
                        continue;
                    }
                    let mut image = picard.clone();
                    for reset in &transition.resets {
                        let IValue::Number(value) = value(graph, &reset.value, &picard, meter)?
                        else {
                            return Err(HybridCertificateRefusal::Evaluation);
                        };
                        image[reset.variable.get()] = value;
                    }
                    if subset(&image, next_box) {
                        image_found = true;
                        break;
                    }
                }
                if !image_found {
                    return Ok(rejected(
                        "jump",
                        &format!("/certificate/steps/{}", index + 1),
                    ));
                }
                if parsed[index + 1].1.lower.as_ref() != Some(&zero()) {
                    return Ok(rejected(
                        "jump",
                        &format!("/certificate/steps/{}", index + 1),
                    ));
                }
            }
        }
    }

    let (horizon, jumps) = match &claim.horizon {
        Some(fsl_semantics::check::HybridHorizon::Seconds(seconds)) => (Some(seconds), None),
        Some(fsl_semantics::check::HybridHorizon::Positions(positions)) => (None, Some(*positions)),
        None => (None, None),
    };
    let last = &parsed[parsed.len() - 1];
    if let Some(horizon) = horizon {
        if last.1.upper.as_ref().is_none_or(|upper| upper < horizon) {
            return Ok(rejected(
                "coverage",
                &format!("/certificate/steps/{}", parsed.len() - 1),
            ));
        }
    }
    if jumps.is_some_and(|jumps| {
        usize::try_from(jumps).map_or(true, |required| parsed.len().saturating_sub(1) < required)
    }) {
        return Ok(rejected("coverage", "/certificate/steps"));
    }
    if !claim_holds(
        graph,
        claim,
        &parsed.iter().map(|step| step.4.clone()).collect::<Vec<_>>(),
        meter,
    )? {
        return Ok(rejected("claim", "/certificate/steps"));
    }
    Ok(HybridCertificateCheck::Accepted)
}

fn check_witness(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    model: &CheckedHybridModel,
    certificate: &Value,
    trace: Option<&super::HybridTrace>,
    meter: &mut HybridReplayMeter,
) -> Result<HybridCertificateCheck, HybridCertificateRefusal> {
    let Some(trace) = trace else {
        return Ok(rejected("witness-start", "/certificate"));
    };
    let dwells = certificate
        .get("dwells")
        .and_then(Value::as_array)
        .ok_or_else(|| HybridCertificateRefusal::Malformed("/certificate/dwells".to_owned()))?;
    if dwells.len() != trace.state_count() {
        return Ok(rejected("witness-start", "/certificate/dwells"));
    }

    let mut decisive = false;
    let mut last_pointer = "/certificate/dwells".to_owned();
    let mut previous_end = trace.start.time.upper.clone();
    let mut previous_endpoint = trace
        .start
        .enclosure
        .iter()
        .map(|i| Interval {
            lower: Some(i.lower.clone()),
            upper: Some(i.upper.clone()),
        })
        .collect::<Vec<_>>();

    for (dwell_index, raw_steps) in dwells.iter().enumerate() {
        let at_dwell = format!("/certificate/dwells/{dwell_index}");
        let Some(steps) = raw_steps.as_array() else {
            return Err(HybridCertificateRefusal::Malformed(at_dwell));
        };
        let Some(state) = trace.state(dwell_index) else {
            return Ok(rejected("witness-start", &at_dwell));
        };
        if steps.is_empty() {
            return Ok(rejected("witness-start", &at_dwell));
        }
        let mode = model
            .modes
            .get(state.mode.get())
            .ok_or(HybridCertificateRefusal::UnknownModel)?;
        for (step_index, step) in steps.iter().enumerate() {
            let at = format!("{at_dwell}/{step_index}");
            last_pointer.clone_from(&at);
            let mode_name = step.get("mode").and_then(Value::as_str);
            if mode_name != Some(mode.name.as_str()) {
                return Ok(rejected(
                    if step_index == 0 && dwell_index == 0 {
                        "witness-start"
                    } else {
                        "witness-picard"
                    },
                    &format!("{at}/mode"),
                ));
            }
            let time = step
                .get("time")
                .and_then(read_interval)
                .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/time")))?;
            let delta = step
                .get("step")
                .and_then(read_rational)
                .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/step")))?;
            let box_ = read_box(step.get("box"), model)
                .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/box")))?;
            let apriori = read_box(step.get("apriori"), model)
                .ok_or_else(|| HybridCertificateRefusal::Malformed(format!("{at}/apriori")))?;
            let (Some(lower), Some(upper), Some(start)) = (
                time.lower.as_ref(),
                time.upper.as_ref(),
                previous_end.as_ref(),
            ) else {
                return Ok(rejected("witness-picard", &at));
            };
            if lower != start || delta <= zero() || upper.sub(lower) != delta {
                return Ok(rejected("witness-picard", &format!("{at}/time")));
            }
            if !subset(&previous_endpoint, &box_) {
                return Ok(rejected(
                    if step_index == 0 && dwell_index == 0 {
                        "witness-start"
                    } else {
                        "witness-picard"
                    },
                    &format!("{at}/box"),
                ));
            }
            if step_index == 0 && dwell_index == 0 {
                let (Some(start_lower), Some(start_upper)) = (
                    trace.start.time.lower.as_ref(),
                    trace.start.time.upper.as_ref(),
                ) else {
                    return Ok(rejected("witness-start", "/certificate/dwells/0/0/box"));
                };
                if start_lower != start_upper || lower != start_lower {
                    return Ok(rejected("witness-start", &format!("{at}/time")));
                }
            }
            if truth(graph, &mode.invariant, &apriori, meter)? != Tri::True {
                return Ok(rejected("witness-picard", &at));
            }
            let mut rates = Vec::with_capacity(mode.rates.len());
            for rate in &mode.rates {
                let IValue::Number(rate) = value(graph, rate, &apriori, meter)? else {
                    return Err(HybridCertificateRefusal::Evaluation);
                };
                rates.push(rate);
            }
            let mut picard = Vec::with_capacity(box_.len());
            let mut endpoint = Vec::with_capacity(box_.len());
            for (variable, rate) in rates.iter().enumerate() {
                let tube = add_intervals(&box_[variable], &scale_nonnegative(rate, &delta))
                    .ok_or(HybridCertificateRefusal::Evaluation)?;
                let point = add_intervals(&box_[variable], &scale_point(rate, &delta))
                    .ok_or(HybridCertificateRefusal::Evaluation)?;
                picard.push(contract_boundary(
                    graph,
                    &mode.invariant,
                    variable,
                    tube,
                    &apriori,
                    meter,
                )?);
                endpoint.push(contract_boundary(
                    graph,
                    &mode.invariant,
                    variable,
                    point,
                    &apriori,
                    meter,
                )?);
            }
            if !subset(&picard, &apriori) {
                return Ok(rejected("witness-picard", &at));
            }
            if let Some(next_step) = steps.get(step_index + 1) {
                let Some(next_box) = read_box(next_step.get("box"), model) else {
                    return Err(HybridCertificateRefusal::Malformed(format!(
                        "{at_dwell}/{}/box",
                        step_index + 1
                    )));
                };
                if !subset(&endpoint, &next_box) {
                    return Ok(rejected(
                        "witness-picard",
                        &format!("{at_dwell}/{}/box", step_index + 1),
                    ));
                }
            }
            if !decisive
                && violates_always_claim(
                    graph,
                    claim,
                    &trace.start.time,
                    &time,
                    &box_,
                    &apriori,
                    meter,
                )?
            {
                decisive = true;
            }
            previous_end = time.upper.clone();
            previous_endpoint = endpoint;
        }
        if let Some(jump) = trace.jumps.get(dwell_index) {
            let final_step = steps.last().expect("nonempty dwell checked");
            let final_box = read_box(final_step.get("apriori"), model).ok_or_else(|| {
                HybridCertificateRefusal::Malformed(format!("{last_pointer}/apriori"))
            })?;
            let Some(transition) = mode.transitions.get(jump.transition as usize) else {
                return Ok(rejected("witness-guard", &last_pointer));
            };
            if truth(graph, &transition.guard, &final_box, meter)? != Tri::True {
                return Ok(rejected("witness-guard", &last_pointer));
            }
            previous_endpoint = jump
                .state
                .enclosure
                .iter()
                .map(|i| Interval {
                    lower: Some(i.lower.clone()),
                    upper: Some(i.upper.clone()),
                })
                .collect();
            previous_end = jump.state.time.upper.clone();
        }
    }
    if decisive {
        Ok(HybridCertificateCheck::Accepted)
    } else {
        Ok(rejected("witness-violation", &last_pointer))
    }
}

fn violates_always_claim(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    activation_time: &super::ExactInterval,
    time: &Interval,
    box_: &[Interval],
    apriori: &[Interval],
    meter: &mut HybridReplayMeter,
) -> Result<bool, HybridCertificateRefusal> {
    if claim.clock != HybridClock::ModelTime || claim.activation != HybridActivation::Origin {
        return Ok(false);
    }
    let fsl_semantics::check::HybridFormula::Always(region, inner) = &claim.formula else {
        return Ok(false);
    };
    let fsl_semantics::check::HybridFormula::Holds(atom) = inner.as_ref() else {
        return Ok(false);
    };
    let Some(origin) = activation_time
        .lower
        .as_ref()
        .zip(activation_time.upper.as_ref())
        .filter(|(lower, upper)| lower == upper)
        .map(|(lower, _)| lower)
    else {
        return Ok(false);
    };
    let start = origin.add(
        &region
            .as_ref()
            .map_or_else(zero, |interval| interval.lower.clone()),
    );
    let end = region
        .as_ref()
        .and_then(|interval| interval.upper.as_ref())
        .map(|upper| origin.add(upper));
    let Some((time_lower, time_upper)) = time.lower.as_ref().zip(time.upper.as_ref()) else {
        return Ok(false);
    };
    if time_lower < &start
        || end.as_ref().is_some_and(|end| time_upper > end)
        || region
            .as_ref()
            .is_some_and(|interval| interval.lower_open && time_lower == &start)
        || region.as_ref().is_some_and(|interval| {
            interval.upper_open && end.as_ref().is_some_and(|end| time_upper == end)
        })
    {
        return Ok(false);
    }
    Ok(truth(graph, atom, box_, meter)? == Tri::False
        || truth(graph, atom, apriori, meter)? == Tri::False)
}

fn truth(
    graph: &CheckedGraph,
    term: &HybridTerm,
    variables: &[Interval],
    meter: &mut HybridReplayMeter,
) -> Result<Tri, HybridCertificateRefusal> {
    match value(graph, term, variables, meter)? {
        IValue::Boolean(value) => Ok(value),
        IValue::Number(_) => Ok(Tri::Unknown),
    }
}

fn value(
    graph: &CheckedGraph,
    term: &HybridTerm,
    variables: &[Interval],
    meter: &mut HybridReplayMeter,
) -> Result<IValue, HybridCertificateRefusal> {
    let mut expander = Expander::new(graph);
    let expression = expander.term(term, meter).map_err(map_evaluation_refusal)?;
    IntervalEvaluator::new(
        &mut expander,
        Rational::from_integer(Integer::from(0)),
        meter,
    )
    .evaluate(expression, variables)
    .map_err(map_evaluation_refusal)
}

fn claim_holds(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    boxes: &[Vec<Interval>],
    meter: &mut HybridReplayMeter,
) -> Result<bool, HybridCertificateRefusal> {
    fn holds(
        formula: &HybridFormula,
        graph: &CheckedGraph,
        boxes: &[Vec<Interval>],
        meter: &mut HybridReplayMeter,
    ) -> Result<bool, HybridCertificateRefusal> {
        match formula {
            HybridFormula::Holds(term) => {
                for boxed in boxes {
                    if truth(graph, term, boxed, meter)? != Tri::True {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            HybridFormula::Always(_, inner) => holds(inner, graph, boxes, meter),
            HybridFormula::Not(inner) => Ok(!holds(inner, graph, boxes, meter)?),
            HybridFormula::Logic(operator, left, right) => {
                let left = holds(left, graph, boxes, meter)?;
                let right = holds(right, graph, boxes, meter)?;
                Ok(match operator {
                    fsl_semantics::check::LogicOp::And => left && right,
                    fsl_semantics::check::LogicOp::Or => left || right,
                    fsl_semantics::check::LogicOp::Implies => !left || right,
                })
            }
            _ => Ok(false),
        }
    }
    holds(&claim.formula, graph, boxes, meter)
}

fn subset(inner: &[Interval], outer: &[Interval]) -> bool {
    inner.len() == outer.len()
        && inner.iter().zip(outer).all(|(inner, outer)| {
            inner
                .lower
                .as_ref()
                .zip(outer.lower.as_ref())
                .is_some_and(|(a, b)| a >= b)
                && inner
                    .upper
                    .as_ref()
                    .zip(outer.upper.as_ref())
                    .is_some_and(|(a, b)| a <= b)
        })
}

fn zero() -> Rational {
    Rational::from_integer(Integer::from(0))
}

fn add_intervals(left: &Interval, right: &Interval) -> Option<Interval> {
    Some(Interval {
        lower: Some(left.lower.as_ref()?.add(right.lower.as_ref()?)),
        upper: Some(left.upper.as_ref()?.add(right.upper.as_ref()?)),
    })
}

fn scale_nonnegative(value: &Interval, scale: &Rational) -> Interval {
    let zero = zero();
    let lower = value.lower.as_ref().map(|lower| {
        if lower >= &zero {
            zero.clone()
        } else {
            lower.mul(scale)
        }
    });
    let upper = value.upper.as_ref().map(|upper| {
        if upper <= &zero {
            zero.clone()
        } else {
            upper.mul(scale)
        }
    });
    Interval { lower, upper }
}

fn scale_point(value: &Interval, scale: &Rational) -> Interval {
    let lower = value
        .lower
        .as_ref()
        .zip(value.upper.as_ref())
        .map(|(a, b)| {
            let left = a.mul(scale);
            let right = b.mul(scale);
            if left <= right { left } else { right }
        });
    let upper = value
        .lower
        .as_ref()
        .zip(value.upper.as_ref())
        .map(|(a, b)| {
            let left = a.mul(scale);
            let right = b.mul(scale);
            if left >= right { left } else { right }
        });
    Interval { lower, upper }
}

fn contract_boundary(
    graph: &CheckedGraph,
    invariant: &HybridTerm,
    variable: usize,
    mut candidate: Interval,
    domain: &[Interval],
    meter: &mut HybridReplayMeter,
) -> Result<Interval, HybridCertificateRefusal> {
    let epsilon = Rational::from_integer(Integer::from(1));
    if let (Some(candidate_lower), Some(domain_lower)) =
        (candidate.lower.as_ref(), domain[variable].lower.as_ref())
    {
        if candidate_lower < domain_lower {
            let mut outside = domain.to_vec();
            outside[variable] = Interval::point(domain_lower.sub(&epsilon));
            if truth(graph, invariant, &outside, meter)? == Tri::False {
                candidate.lower = Some(domain_lower.clone());
            }
        }
    }
    if let (Some(candidate_upper), Some(domain_upper)) =
        (candidate.upper.as_ref(), domain[variable].upper.as_ref())
    {
        if candidate_upper > domain_upper {
            let mut outside = domain.to_vec();
            outside[variable] = Interval::point(domain_upper.add(&epsilon));
            if truth(graph, invariant, &outside, meter)? == Tri::False {
                candidate.upper = Some(domain_upper.clone());
            }
        }
    }
    Ok(candidate)
}

fn read_box(value: Option<&Value>, model: &CheckedHybridModel) -> Option<Vec<Interval>> {
    let variables = value?.as_object()?;
    if variables.len() != model.variables.len() {
        return None;
    }
    model
        .variables
        .iter()
        .map(|variable| {
            let interval = read_interval(variables.get(variable.name.as_str())?)?;
            if interval.lower.as_ref()? > interval.upper.as_ref()? {
                return None;
            }
            Some(interval)
        })
        .collect()
}

fn read_interval(value: &Value) -> Option<Interval> {
    let lower = read_rational(value.get("lower")?)?;
    let upper = read_rational(value.get("upper")?)?;
    Some(Interval {
        lower: Some(lower),
        upper: Some(upper),
    })
}

fn read_rational(value: &Value) -> Option<Rational> {
    let object = value.as_object()?;
    if object.len() != 2 {
        return None;
    }
    let integer = |member: &str| {
        let text = object.get(member)?.as_str()?;
        let parsed = text.parse::<Integer>().ok()?;
        (parsed.to_string() == text).then_some(parsed)
    };
    let numerator = integer("numerator")?;
    let denominator = integer("denominator")?;
    let reduced = Rational::new(numerator.clone(), denominator.clone()).ok()?;
    (reduced.numerator() == &numerator && reduced.denominator() == &denominator).then_some(reduced)
}

fn rejected(rule: &str, at: &str) -> HybridCertificateCheck {
    HybridCertificateCheck::Rejected {
        rule: rule.to_owned(),
        at: at.to_owned(),
    }
}

fn map_evaluation_refusal(refusal: HybridReplayRefusal) -> HybridCertificateRefusal {
    match refusal {
        HybridReplayRefusal::ResourceExhausted { .. } => {
            HybridCertificateRefusal::Malformed("work-limit".to_owned())
        }
        HybridReplayRefusal::Expansion(ExpansionGap::UnreachableFunction(_))
        | HybridReplayRefusal::Expansion(ExpansionGap::UnsupportedBodyForm { .. })
        | HybridReplayRefusal::Exact(_)
        | HybridReplayRefusal::Trace(_) => HybridCertificateRefusal::Evaluation,
        HybridReplayRefusal::UnknownModel(_) => HybridCertificateRefusal::UnknownModel,
    }
}
