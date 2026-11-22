// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered, suspended joint choices: only the visited successor is admitted
//! and charged as a move, and a completed real tuple is judged immediately.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use filament_exact::Meter;
use fsl_eval::hyper::{judge_closed_tuple, ClosedExecution, ClosedInputs, ClosedStep};
use fsl_eval::simulation::TransitionSystem;
use fsl_replay::{HyperCounterexample, HyperCounterexampleKind, HyperOutcome, ModelCheckLimit};

use super::super::{fault, widen, Run, Totals};
use super::reduction::{charge, Admission, Member, Order, Scope, Stop};
use super::{ConstraintReading, Reading};

struct Open {
    paths: Vec<ClosedExecution>,
    ended: Vec<bool>,
    position: usize,
}

/// Component alternatives are stored once, never their Cartesian product.
/// Splitting ranges suspends all but the least pending joint candidate.
#[derive(Clone)]
struct Choices {
    paths: Arc<Vec<Vec<ClosedExecution>>>,
    ranges: Vec<Range<usize>>,
    previous: Option<Arc<Vec<ClosedExecution>>>,
    ended: Vec<bool>,
    position: usize,
}

enum Work {
    Open(Open),
    Choices(Choices),
}

type Frontier = BTreeMap<(Order, u8, usize), Work>;

struct Prefix<'p> {
    paths: &'p [ClosedExecution],
    previous: &'p [ClosedExecution],
    position: usize,
}

impl Run<'_, '_> {
    pub(super) fn enumerate_closed_joint(
        &self,
        inputs: &ClosedInputs<'_>,
        horizon: u64,
        totals: &mut Totals,
        meter: &mut Meter,
        matched: &mut bool,
    ) -> Result<Scope, Stop> {
        let selected = self.symmetric || self.request.reductions.constraint.is_some();
        let members = self.closed_order_members(inputs.instance, selected, meter)?;
        let mut initial = Vec::with_capacity(self.slots.len());
        for subject in &self.slots {
            let system = self
                .systems
                .get(*subject)
                .ok_or_else(|| fault("a-finite-variable-names-a-subject"))?;
            let mut paths = Vec::new();
            for (index, state) in system.initial().into_iter().enumerate() {
                charge(meter, self.cancel)?;
                paths.push(ClosedExecution {
                    initial: index,
                    states: vec![state],
                    steps: Vec::new(),
                });
            }
            if paths.is_empty() {
                return Ok(Scope::default());
            }
            initial.push(paths);
        }
        let mut frontier = Frontier::new();
        let mut serial = 0;
        let ranges = initial.iter().map(|paths| 0..paths.len()).collect();
        self.queue_closed_choices(
            Choices {
                paths: Arc::new(initial),
                ranges,
                previous: None,
                ended: vec![false; self.slots.len()],
                position: 0,
            },
            &members,
            selected,
            &mut frontier,
            &mut serial,
            meter,
        )?;
        let mut scope = Scope::default();
        while let Some(((order, _, _), work)) = frontier.pop_first() {
            charge(meter, self.cancel)?;
            match work {
                Work::Choices(choices) => {
                    if let Some(component) = choices.ranges.iter().position(|range| range.len() > 1)
                    {
                        let range = &choices.ranges[component];
                        let middle = range.start + range.len() / 2;
                        let mut left = choices.clone();
                        left.ranges[component].end = middle;
                        let mut right = choices;
                        right.ranges[component].start = middle;
                        self.queue_closed_choices(
                            left,
                            &members,
                            selected,
                            &mut frontier,
                            &mut serial,
                            meter,
                        )?;
                        self.queue_closed_choices(
                            right,
                            &members,
                            selected,
                            &mut frontier,
                            &mut serial,
                            meter,
                        )?;
                        continue;
                    }
                    // Only this singleton choice becomes an actual joint tuple.
                    let paths = choices
                        .paths
                        .iter()
                        .zip(&choices.ranges)
                        .map(|(paths, range)| paths[range.start].clone())
                        .collect::<Vec<_>>();
                    let previous = choices
                        .previous
                        .as_deref()
                        .map_or(paths.as_slice(), Vec::as_slice);
                    if !self.admit_closed_prefix(
                        inputs,
                        Prefix {
                            paths: &paths,
                            previous,
                            position: choices.position,
                        },
                        &mut scope,
                        totals,
                        meter,
                    )? {
                        continue;
                    }
                    if choices.previous.is_some() {
                        let bound = widen(self.request.limits.limits.max_transitions);
                        if totals.edges >= bound {
                            return Err(Stop::Limit(ModelCheckLimit::MaxTransitions, bound));
                        }
                        totals.edges += 1;
                    }
                    // An admitted tuple is visited before another choice with
                    // the same bound; no later sibling is admitted in advance.
                    frontier.insert(
                        (order, 0, serial),
                        Work::Open(Open {
                            paths,
                            ended: choices.ended,
                            position: choices.position,
                        }),
                    );
                    serial += 1;
                }
                Work::Open(mut node) => {
                    let mut alternatives = Vec::with_capacity(node.paths.len());
                    for (component, path) in node.paths.iter().enumerate() {
                        if node.ended[component] || widen(path.steps.len()) >= horizon {
                            node.ended[component] = true;
                            alternatives.push(vec![path.clone()]);
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
                        let mut options = Vec::with_capacity(expanded.successors.len());
                        for (transition, state) in expanded.successors {
                            charge(meter, self.cancel)?;
                            let (_, post_state) = system
                                .state_key(&state)
                                .map_err(crate::model_check::ModelCheckRefusal::Encoding)?;
                            let mut following = path.clone();
                            following.states.push(state);
                            following.steps.push(ClosedStep {
                                transition,
                                post_state,
                                skipped: false,
                            });
                            options.push(following);
                        }
                        if options.is_empty() {
                            node.ended[component] = true;
                            options.push(path.clone());
                        }
                        alternatives.push(options);
                    }
                    if node.ended.iter().all(|ended| *ended) {
                        self.visit_closed_tuple(
                            inputs,
                            &node.paths,
                            &mut scope,
                            totals,
                            meter,
                            matched,
                        )?;
                        continue;
                    }
                    let bound = widen(self.request.limits.limits.max_transitions);
                    let position = node
                        .position
                        .checked_add(1)
                        .ok_or(Stop::Limit(ModelCheckLimit::MaxTransitions, bound))?;
                    let ranges = alternatives.iter().map(|paths| 0..paths.len()).collect();
                    self.queue_closed_choices(
                        Choices {
                            paths: Arc::new(alternatives),
                            ranges,
                            previous: Some(Arc::new(node.paths)),
                            ended: node.ended,
                            position,
                        },
                        &members,
                        selected,
                        &mut frontier,
                        &mut serial,
                        meter,
                    )?;
                }
            }
        }
        Ok(scope)
    }

    fn queue_closed_choices(
        &self,
        choices: Choices,
        members: &[Member],
        selected: bool,
        frontier: &mut Frontier,
        serial: &mut usize,
        meter: &mut Meter,
    ) -> Result<(), Stop> {
        charge(meter, self.cancel)?;
        let order =
            self.closed_choice_order(&choices.paths, &choices.ranges, members, selected, meter)?;
        frontier.insert((order, 1, *serial), Work::Choices(choices));
        *serial += 1;
        Ok(())
    }

    fn visit_closed_tuple(
        &self,
        inputs: &ClosedInputs<'_>,
        paths: &[ClosedExecution],
        scope: &mut Scope,
        totals: &mut Totals,
        meter: &mut Meter,
        matched: &mut bool,
    ) -> Result<(), Stop> {
        let canonical = self.retain_closed(
            paths,
            inputs.instance,
            Admission::Complete,
            scope,
            totals,
            meter,
        )?;
        let representative = scope
            .complete
            .get(&canonical)
            .ok_or_else(|| fault("a-retained-closed-tuple-is-listed"))?;
        if representative.actual != canonical || !scope.judged.insert(canonical) {
            return Ok(());
        }
        let tuple = representative.paths.iter().collect::<Vec<_>>();
        match self.tuple_reading(judge_closed_tuple(inputs, &tuple), matched, totals)? {
            Reading::Continue => Ok(()),
            Reading::Ended(outcome) => Err(Stop::Outcome(Box::new(outcome))),
            Reading::Refuted(position) => Err(Stop::Outcome(Box::new(HyperOutcome::Violated {
                counterexample: Box::new(HyperCounterexample {
                    kind: HyperCounterexampleKind::Closed,
                    instance: Self::instance_of(inputs.instance),
                    traces: self.closed_traces(&tuple)?,
                    trace_position: position,
                }),
                stats: totals.stats(),
            }))),
        }
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
