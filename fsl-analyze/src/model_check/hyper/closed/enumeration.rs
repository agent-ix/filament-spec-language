// SPDX-License-Identifier: AGPL-3.0-or-later
//! Joint finite prefixes: entering matches and state constraints decide
//! admission before any descendant expansion or completed-orbit bookkeeping.

use filament_exact::Meter;
use fsl_eval::hyper::{ClosedExecution, ClosedInputs, ClosedStep, Tuples};
use fsl_eval::simulation::model::ModelSimState;
use fsl_eval::simulation::TransitionSystem;

use super::super::{fault, widen, Run, Totals};
use super::reduction::{charge, Admission, Scope, Stop};
use super::{ConstraintReading, Reading};
use fsl_replay::ModelCheckLimit;

struct Open {
    paths: Vec<ClosedExecution>,
    ended: Vec<bool>,
    position: usize,
}

struct Step {
    label: ClosedStep,
    state: ModelSimState,
}

struct Prefix<'p> {
    paths: &'p [ClosedExecution],
    previous: &'p [ClosedExecution],
    position: usize,
}

impl Run<'_, '_> {
    /// The finite graph pass owns one work meter across all instances; each
    /// atom and match keeps its separately scoped evaluator meter. Product
    /// limits count admitted closed orbit members and real joint steps only.
    pub(super) fn enumerate_closed_joint(
        &self,
        inputs: &ClosedInputs<'_>,
        horizon: u64,
        totals: &mut Totals,
        meter: &mut Meter,
    ) -> Result<Scope, Stop> {
        let mut initial = Vec::with_capacity(self.slots.len());
        for subject in &self.slots {
            let system = self
                .systems
                .get(*subject)
                .ok_or_else(|| fault("a-finite-variable-names-a-subject"))?;
            initial.push(system.initial());
        }
        let mut scope = Scope::default();
        let choices = Tuples::over(initial.iter().map(Vec::len).collect());
        for picked in choices {
            charge(meter, self.cancel)?;
            let mut paths = Vec::with_capacity(picked.len());
            for (component, index) in picked.into_iter().enumerate() {
                let state = initial
                    .get(component)
                    .and_then(|states| states.get(index))
                    .ok_or_else(|| fault("a-finite-initial-tuple-is-listed"))?;
                paths.push(ClosedExecution {
                    initial: index,
                    states: vec![state.clone()],
                    steps: Vec::new(),
                });
            }
            if !self.admit_closed_prefix(
                inputs,
                Prefix {
                    paths: &paths,
                    previous: &paths,
                    position: 0,
                },
                &mut scope,
                totals,
                meter,
            )? {
                continue;
            }
            let mut open = vec![Open {
                ended: vec![false; paths.len()],
                paths,
                position: 0,
            }];
            while let Some(mut node) = open.pop() {
                charge(meter, self.cancel)?;
                let mut options = Vec::with_capacity(node.paths.len());
                for (component, path) in node.paths.iter().enumerate() {
                    if node.ended[component] || widen(path.steps.len()) >= horizon {
                        node.ended[component] = true;
                        options.push(Vec::new());
                        continue;
                    }
                    let system = self
                        .slots
                        .get(component)
                        .and_then(|slot| self.systems.get(*slot))
                        .ok_or_else(|| fault("a-finite-component-has-a-system"))?;
                    let state = path
                        .states
                        .last()
                        .ok_or_else(|| fault("a-finite-prefix-has-a-current-state"))?;
                    let expanded = system
                        .successors(state)
                        .map_err(|stop| Stop::Expansion(component, stop))?;
                    let mut keyed = Vec::with_capacity(expanded.successors.len());
                    for (transition, state) in expanded.successors {
                        charge(meter, self.cancel)?;
                        let (key, post_state) = system
                            .state_key(&state)
                            .map_err(crate::model_check::ModelCheckRefusal::Encoding)?;
                        let order = system
                            .transition_key(&transition)
                            .map_err(crate::model_check::ModelCheckRefusal::Encoding)?;
                        keyed.push((
                            (order, key),
                            Step {
                                label: ClosedStep {
                                    transition,
                                    post_state,
                                    skipped: false,
                                },
                                state,
                            },
                        ));
                    }
                    keyed.sort_by(|left, right| left.0.cmp(&right.0));
                    if keyed.is_empty() {
                        node.ended[component] = true;
                    }
                    options.push(keyed.into_iter().map(|(_, step)| step).collect::<Vec<_>>());
                }
                if node.ended.iter().all(|ended| *ended) {
                    self.retain_closed(
                        &node.paths,
                        inputs.instance,
                        Admission::Complete,
                        &mut scope,
                        totals,
                        meter,
                    )?;
                    continue;
                }
                let choices =
                    Tuples::over(options.iter().map(|steps| steps.len().max(1)).collect());
                let mut following = Vec::new();
                for picked in choices {
                    charge(meter, self.cancel)?;
                    let mut paths = node.paths.clone();
                    for (component, index) in picked.into_iter().enumerate() {
                        if let Some(step) =
                            options.get(component).and_then(|steps| steps.get(index))
                        {
                            let path = paths
                                .get_mut(component)
                                .ok_or_else(|| fault("a-joint-step-has-every-finite-component"))?;
                            path.states.push(step.state.clone());
                            path.steps.push(step.label.clone());
                        }
                    }
                    let bound = widen(self.request.limits.limits.max_transitions);
                    let position = node
                        .position
                        .checked_add(1)
                        .ok_or(Stop::Limit(ModelCheckLimit::MaxTransitions, bound))?;
                    if !self.admit_closed_prefix(
                        inputs,
                        Prefix {
                            paths: &paths,
                            previous: &node.paths,
                            position,
                        },
                        &mut scope,
                        totals,
                        meter,
                    )? {
                        continue;
                    }
                    // A matched, uncut successor is one actual joint move.
                    // Removed candidates and group applications are graph work,
                    // not extra moves in the product.
                    if totals.edges >= bound {
                        return Err(Stop::Limit(ModelCheckLimit::MaxTransitions, bound));
                    }
                    totals.edges += 1;
                    following.push(Open {
                        paths,
                        ended: node.ended.clone(),
                        position,
                    });
                }
                // The first transition tuple is the next one popped.
                open.extend(following.into_iter().rev());
            }
        }
        Ok(scope)
    }

    fn admit_closed_prefix(
        &self,
        inputs: &ClosedInputs<'_>,
        prefix: Prefix<'_>,
        scope: &mut Scope,
        totals: &mut Totals,
        meter: &mut Meter,
    ) -> Result<bool, Stop> {
        let paths = prefix.paths.iter().collect::<Vec<_>>();
        let reading = self.closed_admission(inputs, &paths, prefix.position, totals)?;
        if self.cancel.cause().is_some() {
            return Err(Stop::Cancelled);
        }
        match reading {
            ConstraintReading::Allowed => Ok(true),
            ConstraintReading::Unmatched => Ok(false),
            ConstraintReading::Cut => {
                self.retain_closed(
                    prefix.previous,
                    inputs.instance,
                    Admission::Boundary,
                    scope,
                    totals,
                    meter,
                )?;
                Ok(false)
            }
            ConstraintReading::Ended(outcome) => Err(Stop::Outcome(Box::new(outcome))),
            ConstraintReading::Stopped(verdict) => match self.stopped_reading(verdict, totals)? {
                Reading::Ended(outcome) => Err(Stop::Outcome(Box::new(outcome))),
                Reading::Continue | Reading::Refuted(_) => {
                    Err(fault("a-stopped-prefix-admission-ends-the-item").into())
                }
            },
        }
    }
}
