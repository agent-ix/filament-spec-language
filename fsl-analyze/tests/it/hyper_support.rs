// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "shared hyperproperty fixture helpers span split integration test binaries"
)]
//! Shared hyperproperty outcome helpers.

use filament_exact::{Charge, ChargePoint, Meter};
use fsl_eval::hyper::{MatchReading, ParameterBinding, StepLabel, StepMatch};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_replay::{HyperCounterexample, HyperOutcome, HyperStep};

use crate::counter::ZERO;
use crate::hyper_world::{high, low, World};

/// A Kernel-checked remainder function callable from a Linked match body.
pub(crate) const KERNEL_MOD_FUNCTION: &str =
    "function q using v(x: Integer): Integer pure { 1 mod x }\n";

/// The counterexample of a violated run.
pub(crate) fn refuted(outcome: HyperOutcome) -> HyperCounterexample {
    match outcome {
        HyperOutcome::Violated { counterexample, .. } => *counterexample,
        other => panic!("expected a violation, got {other:?}"),
    }
}

/// The input `i` of the model step `step`, which must be a `step(i)`.
pub(crate) fn input(step: &HyperStep) -> i64 {
    match step {
        HyperStep::Model(step) => {
            let [argument] = step.transition.arguments() else {
                panic!("step takes one input");
            };
            argument
                .split("\"value\":\"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .expect("an integer argument")
                .parse()
                .expect("a small input")
        }
        HyperStep::TerminalStutter => panic!("the vault has no terminal state"),
    }
}

/// The inputs `step(i)` a trace's steps take, by `i`; the steps must all be
/// model steps of `step`.
pub(crate) fn inputs(counterexample: &HyperCounterexample, trace: usize) -> Vec<i64> {
    counterexample.traces[trace]
        .steps
        .iter()
        .map(input)
        .collect()
}

/// The `(h, l)` of each position of trace `trace`, read by re-executing its
/// steps through the vault from its initial state (the oracle).
pub(crate) fn positions(
    world: &World,
    counterexample: &HyperCounterexample,
    trace: usize,
) -> Vec<(i64, i64)> {
    let system = ModelSystem::new(world.subject().simulation).expect("the subject admits");
    let trace = &counterexample.traces[trace];
    let mut state = system.initial()[trace.initial as usize].clone();
    let mut seen = vec![(high(&state), low(&state))];
    for step in &trace.steps {
        let HyperStep::Model(step) = step else {
            panic!("the vault has no terminal state");
        };
        state = system
            .successors(&state)
            .expect("the vault expands")
            .successors
            .into_iter()
            .find(|(transition, next)| {
                *transition == step.transition
                    && system.state_key(next, fsl_foundation::IdentityLimits::default()).expect("a key").0.as_slice()
                        == step.post_state.as_bytes()
            })
            .unwrap_or_else(|| panic!("{step:?} is not a step of the vault"))
            .1;
        seen.push((high(&state), low(&state)));
    }
    seen
}

/// A `match` that refuses: the reading of every joint step is `Refused`.
pub(crate) struct Refusing;

impl StepMatch for Refusing {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        Ok(MatchReading::Refused {
            conjunct: 0,
            outcome: fsl_eval::simulation::model::ClauseOutcome::NotBoolean,
        })
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        panic!("an HP-2 clause has no existential conjunct")
    }
}

/// A `match` whose evaluation exhausts its meter: it charges a meter that
/// admits no work.
pub(crate) struct Exhausted;

impl StepMatch for Exhausted {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        Meter::new(ZERO)
            .charge(Charge::new(ChargePoint::GraphExpand))
            .map_err(|incomplete| fsl_eval::hyper::MatchIncomplete {
                conjunct: 0,
                incomplete,
            })?;
        Ok(MatchReading::Holds)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        _steps: &[StepLabel<'_>],
        _meter: &mut Meter,
    ) -> Result<MatchReading, fsl_eval::hyper::MatchIncomplete> {
        panic!("an HP-2 clause has no existential conjunct")
    }
}
