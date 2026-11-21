// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-290 `solver-inconclusive{trace}`: the `trace` member of a plugin's
//! cause read in FSpec FR-193's "Hybrid trace" form, its names resolved to
//! indices against the claim's `CheckedHybridModel`, and the correspondence
//! failures FR-250 lists refused `Malformed` at their member.

use serde_json::{Map, Value};

use super::{
    at_index, canonical_integer, child, count_of, first_undefined, invalid, malformed, member,
    read_plugin_inconclusive_cause, unknown_member, wrong_kind,
};
use crate::hybrid_replay::{
    admit, ExactInterval, ExactReal, ExpressionSite, HybridCounterexample,
    HybridCounterexampleKind, HybridTrace, TraceBefore, TraceJump, TraceState, UndefinedCause,
    UndefinedInstant,
};
use crate::proof_result::plugin::PluginResultRefusal;
use crate::proof_result::InconclusiveCause;
use filament_exact::Rational;
use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{
    CheckedFunctionRef, CheckedGraph, CheckedHybridClaim, CheckedHybridModel, HybridTerm, ModeIndex,
};

use crate::hybrid::{hybrid_counterexample_locus, hybrid_term_at_locus};

/// Read a provider's FR-331 `hybrid-counterexample` for a hybrid claim.
pub(super) fn read_hybrid_counterexample(
    entry: &Value,
    model: Option<&CheckedHybridModel>,
    context: Option<(&CheckedGraph, &CheckedHybridClaim, &WireGraph)>,
) -> Result<HybridCounterexample, PluginResultRefusal> {
    const BASE: &str = "/counterexample";
    let Value::Object(members) = entry else {
        return Err(wrong_kind(String::new()));
    };
    if let Some(extra) = first_undefined(members, &["backend", "counterexample", "request_index"]) {
        return Err(unknown_member(child("", extra)));
    }
    let Some(counterexample) = members.get("counterexample") else {
        return Err(PluginResultRefusal::MissingCounterexample {
            pointer: BASE.to_owned(),
            cause: crate::proof_result::plugin::ValueFault::MissingMember,
        });
    };
    let Some(model) = model else {
        return Err(PluginResultRefusal::MissingCounterexample {
            pointer: child(BASE, "type"),
            cause: crate::proof_result::plugin::ValueFault::InvalidValue,
        });
    };
    let pointer = BASE.to_owned();
    let fields = object(counterexample, &["kind", "trace", "type"], &pointer)?;
    let type_value = member(fields, "type", &pointer)?;
    match type_value.as_str() {
        Some("hybrid-counterexample") => {}
        Some(_) => {
            return Err(PluginResultRefusal::MissingCounterexample {
                pointer: child(BASE, "type"),
                cause: crate::proof_result::plugin::ValueFault::InvalidValue,
            });
        }
        None => {
            return Err(PluginResultRefusal::MissingCounterexample {
                pointer: child(BASE, "type"),
                cause: crate::proof_result::plugin::ValueFault::WrongValueKind,
            });
        }
    }
    let kind_pointer = child(BASE, "kind");
    let kind_value = member(fields, "kind", &pointer)?;
    let kind = match kind_value.as_str() {
        Some("formula") => HybridCounterexampleKind::Formula,
        Some(_) => return Err(invalid(kind_pointer)),
        None if kind_value.get("type").and_then(Value::as_str) == Some("undefined-evaluation") => {
            let (graph, claim, wire) = context.ok_or_else(|| invalid(kind_pointer.clone()))?;
            let checked_claim = graph
                .hybrid_claims()
                .iter()
                .position(|candidate| candidate == claim)
                .ok_or_else(|| invalid(kind_pointer.clone()))?;
            read_undefined_kind(kind_value, graph, checked_claim, wire)?
        }
        None if kind_value.is_object() => return Err(invalid(kind_pointer)),
        None => return Err(wrong_kind(kind_pointer)),
    };
    let trace_pointer = child(BASE, "trace");
    let trace = read_trace(model, member(fields, "trace", &pointer)?, &trace_pointer)?;
    Ok(HybridCounterexample { kind, trace })
}

/// A plugin's `inconclusive` `cause` over an item whose claim is a hybrid
/// claim over `model`, or over any other item when `model` is `None`
/// (FR-290, "Basis and cause"). A `solver-inconclusive` `trace` reads as
/// `Some(HybridTrace)` on a hybrid claim and is `Malformed` at
/// `/cause/trace` on any other item; `insufficient-horizon` is `Malformed`
/// at `/cause/type` on any other item. The trace is kept, neither replayed
/// nor settled from.
///
/// # Errors
///
/// [`PluginResultRefusal`], as for [`read_plugin_inconclusive_cause`], with
/// the trace's own shape and correspondence refusals.
#[ix_trace_rs::string_edge]
pub fn read_plugin_inconclusive_cause_over(
    cause: &Value,
    model: Option<&CheckedHybridModel>,
) -> Result<InconclusiveCause, PluginResultRefusal> {
    const BASE: &str = "/cause";
    let kind = cause
        .as_object()
        .and_then(|members| members.get("type"))
        .and_then(Value::as_str);
    match (kind, model) {
        (Some("insufficient-horizon"), None) => Err(invalid(child(BASE, "type"))),
        (Some("solver-inconclusive"), _) => {
            let Some(members) = cause.as_object() else {
                return read_plugin_inconclusive_cause(cause);
            };
            let Some(trace) = members.get("trace") else {
                return read_plugin_inconclusive_cause(cause);
            };
            let mut rest = members.clone();
            rest.remove("trace");
            read_plugin_inconclusive_cause(&Value::Object(rest))?;
            let pointer = child(BASE, "trace");
            let Some(model) = model else {
                return Err(invalid(pointer));
            };
            Ok(InconclusiveCause::SolverInconclusive {
                trace: Some(Box::new(read_trace(model, trace, &pointer)?)),
                replay: None,
            })
        }
        _ => read_plugin_inconclusive_cause(cause),
    }
}

fn object<'a>(
    value: &'a Value,
    allowed: &[&str],
    pointer: &str,
) -> Result<&'a Map<String, Value>, PluginResultRefusal> {
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer.to_owned()));
    };
    if let Some(extra) = first_undefined(members, allowed) {
        return Err(unknown_member(child(pointer, extra)));
    }
    Ok(members)
}

fn read_undefined_kind(
    value: &Value,
    graph: &CheckedGraph,
    claim_index: usize,
    wire: &WireGraph,
) -> Result<HybridCounterexampleKind, PluginResultRefusal> {
    let base = "/kind";
    let fields = object(value, &["cause", "type", "where"], base)?;
    if member(fields, "type", base)?.as_str() != Some("undefined-evaluation") {
        return Err(invalid(child(base, "type")));
    }
    let cause_value = member(fields, "cause", base)?;
    let cause_name = cause_value
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| malformed(cause_value.is_string(), child(base, "cause")))?;
    let location_value = member(fields, "where", base)?;
    let location = object(
        location_value,
        &["locus", "position", "time"],
        &child(base, "where"),
    )?;
    let position_pointer = child(&child(base, "where"), "position");
    let position = count_of(
        member(location, "position", &child(base, "where"))?,
        false,
        position_pointer.clone(),
    )?;
    let state = usize::try_from(position)
        .ok()
        .ok_or_else(|| invalid(position_pointer))?;
    let time_pointer = child(&child(base, "where"), "time");
    let time = interval(
        member(location, "time", &child(base, "where"))?,
        &time_pointer,
    )?;
    if time.lower > time.upper {
        return Err(invalid(time_pointer));
    }
    let locus_pointer = child(&child(base, "where"), "locus");
    let wire_locus = member(location, "locus", &child(base, "where"))?;
    let locus = hybrid_counterexample_locus(graph, wire, claim_index, wire_locus)
        .ok_or_else(|| invalid(locus_pointer.clone()))?;
    let call = || -> Option<(CheckedFunctionRef, Option<usize>)> {
        let path = &locus.term.term;
        let term = hybrid_term_at_locus(graph, claim_index, &locus)?;
        if let HybridTerm::Call(function, arguments) = term {
            // A call-node locus names the function application, not a child
            // position in the surrounding expression. Its argument index is
            // recoverable without further context only for a unary call.
            return Some((*function, (arguments.len() == 1).then_some(0)));
        }

        // When the locus names a term inside a call's argument, use the path
        // segment immediately below the nearest call node. Earlier segments
        // belong to the surrounding expression and must not become a
        // parameter index.
        for prefix_len in (0..path.len()).rev() {
            let mut parent = locus.clone();
            parent.term.term.truncate(prefix_len);
            if let Some(HybridTerm::Call(function, arguments)) =
                hybrid_term_at_locus(graph, claim_index, &parent)
            {
                let parameter = *path.get(prefix_len)?;
                return (parameter < arguments.len()).then_some((*function, Some(parameter)));
            }
        }
        None
    };
    let cause = match cause_name {
        "division-by-zero" => UndefinedCause::DivisionByZero,
        "outside-capture-range" => match locus.site {
            ExpressionSite::Capture { capture } => UndefinedCause::OutsideCaptureRange { capture },
            _ => return Err(invalid(child(base, "cause"))),
        },
        "outside-parameter-range" => {
            let (function, parameter) = call()
                .and_then(|(function, parameter)| parameter.map(|parameter| (function, parameter)))
                .ok_or_else(|| invalid(locus_pointer.clone()))?;
            UndefinedCause::OutsideParameterRange {
                function,
                parameter,
            }
        }
        "outside-result-range" => {
            let (function, _) = call().ok_or_else(|| invalid(locus_pointer.clone()))?;
            UndefinedCause::OutsideResultRange { function }
        }
        _ => return Err(invalid(child(base, "cause"))),
    };
    Ok(HybridCounterexampleKind::UndefinedEvaluation {
        at: Box::new(UndefinedInstant {
            state,
            time: ExactReal::Rational(time.lower.clone()),
            time_interval: Some(time),
            locus,
        }),
        cause,
    })
}

/// An FR-331 exact `Rational`, `{numerator, denominator}` in lowest terms.
fn rational(value: &Value, pointer: &str) -> Result<Rational, PluginResultRefusal> {
    let members = object(value, &["denominator", "numerator"], pointer)?;
    let part = |name: &str| {
        members
            .get(name)
            .and_then(Value::as_str)
            .and_then(canonical_integer)
    };
    let (Some(numerator), Some(denominator)) = (part("numerator"), part("denominator")) else {
        return Err(invalid(pointer.to_owned()));
    };
    let reduced = Rational::new(numerator.clone(), denominator.clone())
        .map_err(|_| invalid(pointer.to_owned()))?;
    if reduced.numerator() != &numerator || reduced.denominator() != &denominator {
        return Err(invalid(pointer.to_owned()));
    }
    Ok(reduced)
}

/// An FR-331 `Interval`, `{lower, upper}`.
fn interval(value: &Value, pointer: &str) -> Result<ExactInterval, PluginResultRefusal> {
    let members = object(value, &["lower", "upper"], pointer)?;
    let end = |name: &str| rational(member(members, name, pointer)?, &child(pointer, name));
    Ok(ExactInterval::new(end("lower")?, end("upper")?))
}

/// The enclosure of `model`'s variables: one interval for each, keyed by
/// name.
fn enclosure(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<Vec<ExactInterval>, PluginResultRefusal> {
    let Value::Object(members) = value else {
        return Err(wrong_kind(pointer.to_owned()));
    };
    let names: Vec<&str> = model
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    if let Some(extra) = first_undefined(members, &names) {
        return Err(invalid(child(pointer, extra)));
    }
    names
        .iter()
        .map(|name| match members.get(*name) {
            None => Err(invalid(pointer.to_owned())),
            Some(interval_value) => interval(interval_value, &child(pointer, name)),
        })
        .collect()
}

fn mode(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<ModeIndex, PluginResultRefusal> {
    let Some(name) = value.as_str() else {
        return Err(malformed(false, pointer.to_owned()));
    };
    model
        .modes
        .iter()
        .position(|mode| mode.name.as_str() == name)
        .map(ModeIndex::new)
        .ok_or_else(|| invalid(pointer.to_owned()))
}

fn state(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceState, PluginResultRefusal> {
    let members = object(value, &["enclosure", "mode", "time"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceState {
        mode: mode(model, member(members, "mode", pointer)?, &at("mode"))?,
        time: interval(member(members, "time", pointer)?, &at("time"))?,
        enclosure: enclosure(
            model,
            member(members, "enclosure", pointer)?,
            &at("enclosure"),
        )?,
    })
}

fn before(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceBefore, PluginResultRefusal> {
    let members = object(value, &["enclosure", "time"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceBefore {
        time: interval(member(members, "time", pointer)?, &at("time"))?,
        enclosure: enclosure(
            model,
            member(members, "enclosure", pointer)?,
            &at("enclosure"),
        )?,
    })
}

fn jump(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<TraceJump, PluginResultRefusal> {
    let members = object(value, &["before", "state", "transition"], pointer)?;
    let at = |name: &str| child(pointer, name);
    Ok(TraceJump {
        transition: count_of(
            member(members, "transition", pointer)?,
            false,
            at("transition"),
        )?,
        before: members
            .get("before")
            .map(|value| before(model, value, &at("before")))
            .transpose()?,
        state: state(model, member(members, "state", pointer)?, &at("state"))?,
    })
}

/// A `HybridTrace` in FSpec FR-193's wire form over `model`, after the
/// correspondence refusals of "Hybrid trace".
fn read_trace(
    model: &CheckedHybridModel,
    value: &Value,
    pointer: &str,
) -> Result<HybridTrace, PluginResultRefusal> {
    let members = object(value, &["delta", "end", "jumps", "loop", "start"], pointer)?;
    let at = |name: &str| child(pointer, name);
    let Some(jumps) = member(members, "jumps", pointer)?.as_array() else {
        return Err(wrong_kind(at("jumps")));
    };
    let trace = HybridTrace {
        start: state(model, member(members, "start", pointer)?, &at("start"))?,
        jumps: jumps
            .iter()
            .enumerate()
            .map(|(index, value)| jump(model, value, &at_index(&at("jumps"), index)))
            .collect::<Result<_, _>>()?,
        delta: members
            .get("delta")
            .map(|value| rational(value, &at("delta")))
            .transpose()?,
        loop_to: members
            .get("loop")
            .map(|value| count_of(value, false, at("loop")))
            .transpose()?,
        end: members
            .get("end")
            .map(|value| before(model, value, &at("end")))
            .transpose()?,
    };
    admit(model, &trace).map_err(|refusal| PluginResultRefusal::Malformed {
        pointer: format!("{pointer}{}", refusal.pointer),
        cause: crate::proof_result::plugin::ValueFault::InvalidValue,
    })?;
    Ok(trace)
}
