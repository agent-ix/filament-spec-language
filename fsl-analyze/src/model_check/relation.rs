// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-179: explicit-state checking of a relation over reachable model
//! transitions. Each execution ranges over the reachable edges of its named
//! operation; the tuple product is enumerated in execution and edge order.

use filament_exact::{Cancel, Meter, ScalarLimits};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    check_model_bounds, ModelEvaluator, ModelFinding, ModelKey, ModelRelationDescriptor,
    ModelRelationTransition as RelationTransition, ModelRelationTuple, ModelSimState, ModelSystem,
};
use fsl_eval::simulation::{Expansion, ExpansionStop, Outcome, StateKey, TransitionSystem};
use fsl_package::CheckedPackage;
use fsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace,
    HyperUndefined, ModelCheckLimit, ModelStep, ReachedLimit,
};
use fsl_semantics::check::CheckedModelRelation;
use std::collections::BTreeMap;

use super::check::{not_simulated, ModelCheckRefusal, BOUND_POSITIONS};
use super::graph::{retain, RetainedEdge, RetainedGraph};

type RelationGraph = RetainedGraph<ModelSimState, RelationTransition, ModelFinding>;

/// Model expansion with result-aware transition identities for relation checks.
struct RelationSystem<'s, 'a> {
    system: &'s ModelSystem<'a>,
    cancel: &'s Cancel,
}

impl TransitionSystem for RelationSystem<'_, '_> {
    type State = ModelSimState;
    type TransitionId = RelationTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    type InitialError = std::convert::Infallible;

    fn initial_iter(&self) -> impl Iterator<Item = Result<Self::State, Self::InitialError>> + '_ {
        self.system.initial_states().to_vec().into_iter().map(Ok)
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        self.system.key(state)
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        self.system.relation_successors(state, self.cancel)
    }
}

/// Inputs to EN-1's explicit-state model-relation checker (FR-179).
pub struct ModelRelationCheckRequest<'a> {
    /// The checked relation.
    pub relation: &'a CheckedModelRelation,
    /// The checked package whose body the relation evaluates.
    pub package: &'a CheckedPackage,
    /// One subject per distinct used relation alias, in model declaration order.
    pub subjects: Vec<ModelSubject<'a>>,
    /// FR-126's graph budgets and FR-184's relation tuple budget.
    pub limits: fsl_replay::ModelCheckLimits,
    /// Optional per-subject breadth-first search horizon.
    pub max_depth: Option<usize>,
    /// The S6a budget for each relation-body evaluation.
    pub meter: ScalarLimits,
}

/// Source-free inputs to the same EN-1 relation engine.
pub struct ModelRelationInput<'a> {
    /// The admitted claim's execution facts.
    pub relation: &'a ModelRelationDescriptor,
    /// The evaluator that owns that claim occurrence.
    pub evaluator: &'a dyn ModelEvaluator,
    /// One subject for each alias in declaration order.
    pub subjects: Vec<ModelSubject<'a>>,
    /// Graph and tuple budgets.
    pub limits: fsl_replay::ModelCheckLimits,
    /// Optional per-subject breadth-first search horizon.
    pub max_depth: Option<usize>,
    /// A fresh S6a budget for each body evaluation.
    pub meter: ScalarLimits,
}

/// Counts for one relation check.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelRelationStats {
    /// The sum of states retained for the executions' subjects.
    pub states: u64,
    /// The sum of edges retained for the executions' subjects.
    pub transitions: u64,
    /// The number of transition tuples evaluated.
    pub relation_tuples: u64,
}

/// Result of FR-179's reachable-transition relation check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRelationOutcome {
    /// Every tuple in the product of reachable transition sets satisfied the
    /// relation body.
    Holds {
        /// Independently checkable reachable subject closures.
        certificate: fsl_replay::ProductClosureCertificate,
        /// Search counts.
        stats: ModelRelationStats,
    },
    /// The first canonical tuple on which the relation body was false.
    Refuted {
        /// FR-183's replayable `step-tuple` evidence.
        counterexample: Box<HyperCounterexample>,
        /// Search counts.
        stats: ModelRelationStats,
    },
    /// A subject reached its requested depth horizon before closure.
    BoundReached {
        /// The requested horizon.
        depth: u64,
        /// Search counts.
        stats: ModelRelationStats,
    },
    /// A relation body evaluation did not yield a Boolean.
    Undecided {
        /// Search counts.
        stats: ModelRelationStats,
    },
    /// A graph or tuple budget stopped the check.
    Stopped {
        /// The reached request limit, when applicable.
        limit: Option<ReachedLimit>,
        /// Number of tuples at the tuple budget, when applicable.
        count: Option<u64>,
        /// Search counts.
        stats: ModelRelationStats,
    },
}

/// A relation request that cannot name a checked model for an execution.
#[derive(Debug, thiserror::Error)]
pub enum ModelRelationRefusal {
    /// EN-1 requires one subject per checked execution.
    #[error("the relation has {expected} executions and the request supplies {found} subjects")]
    SubjectCount {
        /// Number of checked executions.
        expected: usize,
        /// Number of supplied subjects.
        found: usize,
    },
    /// The subject's selected model differs from the checked execution alias.
    #[error(
        "execution `{execution}` requires model `{expected}`, but its subject selects `{found}`"
    )]
    SubjectAlias {
        /// Execution variable.
        execution: String,
        /// Checked alias.
        expected: String,
        /// Selected alias.
        found: String,
    },
    /// Subject admission or finite-domain classification failed.
    #[error(transparent)]
    Model(#[from] ModelCheckRefusal),
    /// The evaluator or trace builder encountered an impossible checked-data
    /// mismatch.
    #[error("model relation evaluation failed: {0:?}")]
    Fault(InternalFault),
}

/// Decide a checked model relation by enumerating reachable transition tuples
/// explicitly. No SMT encoding is used by this path.
///
/// # Errors
///
/// [`ModelRelationRefusal`] when the request does not supply each checked
/// execution's subject, or a subject cannot be admitted.
pub fn check_model_relation(
    request: ModelRelationCheckRequest<'_>,
    cancel: &Cancel,
) -> Result<ModelRelationOutcome, ModelRelationRefusal> {
    let relation = ModelRelationDescriptor::of_source(request.package, request.relation)
        .map_err(ModelRelationRefusal::Fault)?;
    check_model_relation_input(
        ModelRelationInput {
            relation: &relation,
            evaluator: request.package,
            subjects: request.subjects,
            limits: request.limits,
            max_depth: request.max_depth,
            meter: request.meter,
        },
        cancel,
    )
}

/// Decide an admitted relation through its route's evaluator.
///
/// # Errors
///
/// As [`check_model_relation`], without requiring a source package.
pub fn check_model_relation_input(
    request: ModelRelationInput<'_>,
    cancel: &Cancel,
) -> Result<ModelRelationOutcome, ModelRelationRefusal> {
    let executions = request.relation.executions();
    let aliases = request.relation.aliases();
    if request.subjects.len() != aliases.len() {
        return Err(ModelRelationRefusal::SubjectCount {
            expected: aliases.len(),
            found: request.subjects.len(),
        });
    }

    let mut systems = Vec::with_capacity(request.subjects.len());
    let mut graphs = Vec::with_capacity(request.subjects.len());
    let mut stats = ModelRelationStats::default();
    let mut bounded_depth = None;
    for (subject, expected) in request.subjects.iter().zip(aliases) {
        let expected_package = request
            .relation
            .model_package_identity(expected)
            .ok_or_else(|| {
                ModelRelationRefusal::Fault(InternalFault::new("EN-1", "relation-alias-selected"))
            })?;
        let system = ModelSystem::for_model_identity(subject.simulation.clone(), expected_package)
            .map_err(|failure| {
                ModelRelationRefusal::Model(ModelCheckRefusal::Admission(Box::new(failure)))
            })?;
        let found = system.model().unwrap_or_default();
        if found != expected_package {
            return Err(ModelRelationRefusal::SubjectAlias {
                execution: executions
                    .iter()
                    .zip(request.relation.execution_aliases())
                    .find(|(_, alias)| alias.as_str() == expected.as_str())
                    .map(|((name, _), _)| name.clone())
                    .unwrap_or_default(),
                expected: expected.clone(),
                found: found.to_owned(),
            });
        }
        check_model_bounds(&system, BOUND_POSITIONS)
            .map_err(|refusal| ModelRelationRefusal::Model(not_simulated(refusal)))?;
        let relation_system = RelationSystem {
            system: &system,
            cancel,
        };
        let graph = retain(
            &relation_system,
            request.limits.limits,
            fsl_foundation::IdentityLimits::default(),
            request.max_depth,
            || cancel.poll() && cancel.cause().is_some(),
        )
        .map_err(|error| {
            ModelRelationRefusal::Model(
                error.model_refusal(fsl_foundation::IdentityLimits::default()),
            )
        })?;
        stats.states = stats
            .states
            .saturating_add(u64::try_from(graph.graph.states().len()).unwrap_or(u64::MAX));
        stats.transitions = stats
            .transitions
            .saturating_add(u64::try_from(graph.graph.edges().len()).unwrap_or(u64::MAX));
        match graph.outcome {
            Outcome::Exhaustive(_) => {}
            Outcome::BoundReached { depth, .. } => {
                bounded_depth = Some(u64::try_from(depth).unwrap_or(u64::MAX));
            }
            Outcome::Bounded { limit, .. } => {
                let (limit, value) = match limit {
                    fsl_eval::simulation::Limit::States(value) => {
                        (ModelCheckLimit::MaxStates, value)
                    }
                    fsl_eval::simulation::Limit::Transitions(value) => {
                        (ModelCheckLimit::MaxTransitions, value)
                    }
                };
                return Ok(ModelRelationOutcome::Stopped {
                    limit: Some(ReachedLimit {
                        limit,
                        value: u64::try_from(value).unwrap_or(u64::MAX),
                    }),
                    count: None,
                    stats,
                });
            }
            Outcome::Cancelled { .. } => {
                return Ok(ModelRelationOutcome::Stopped {
                    limit: None,
                    count: None,
                    stats,
                });
            }
            Outcome::Stopped { .. } => {
                if cancel.cause().is_some() {
                    return Ok(ModelRelationOutcome::Stopped {
                        limit: None,
                        count: None,
                        stats,
                    });
                }
                return Ok(ModelRelationOutcome::Undecided { stats });
            }
        }
        systems.push(system);
        graphs.push(graph.graph);
    }

    let alias_indices: BTreeMap<_, _> = aliases
        .iter()
        .enumerate()
        .map(|(index, alias)| (alias.as_str(), index))
        .collect();
    let execution_subjects: Vec<usize> = request
        .relation
        .execution_aliases()
        .iter()
        .map(|alias| {
            alias_indices
                .get(alias.as_str())
                .copied()
                .ok_or_else(|| InternalFault::new("EN-1", "relation-alias-has-a-subject"))
        })
        .collect::<Result<_, _>>()
        .map_err(ModelRelationRefusal::Fault)?;
    let operation_names: Vec<String> = executions
        .iter()
        .map(|(_, operation)| fsl_semantics::check::operation_text(operation))
        .collect();
    let edges: Vec<Vec<usize>> = operation_names
        .iter()
        .enumerate()
        .map(|(execution_index, operation)| {
            let graph = &graphs[execution_subjects[execution_index]];
            graph
                .edges()
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    (edge.transition.transition.operation() == operation.as_str()).then_some(index)
                })
                .collect()
        })
        .collect();

    if edges.iter().any(Vec::is_empty) {
        return Ok(if let Some(depth) = bounded_depth {
            ModelRelationOutcome::BoundReached { depth, stats }
        } else {
            ModelRelationOutcome::Holds {
                certificate: certificate(&request, &graphs)?,
                stats,
            }
        });
    }

    let mut positions = vec![0usize; edges.len()];
    loop {
        if stats.relation_tuples >= request.limits.max_relation_tuples {
            return Ok(ModelRelationOutcome::Stopped {
                limit: Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxRelationTuples,
                    value: request.limits.max_relation_tuples,
                }),
                count: Some(stats.relation_tuples.checked_add(1).ok_or_else(|| {
                    ModelRelationRefusal::Fault(InternalFault::new("EN-1", "relation-count-fits"))
                })?),
                stats,
            });
        }
        let chosen: Vec<&RetainedEdge<RelationTransition>> = edges
            .iter()
            .zip(&positions)
            .enumerate()
            .map(|(index, (choices, position))| {
                &graphs[execution_subjects[index]].edges()[choices[*position]]
            })
            .collect();
        let pre_observations: Vec<fsl_eval::simulation::model::StateObservation> = chosen
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                let subject_index = execution_subjects[index];
                graphs[subject_index]
                    .state(edge.source)
                    .ok_or_else(|| InternalFault::new("S6a", "relation-pre-state-exists"))
                    .and_then(|state| {
                        systems[subject_index]
                            .observe(state.state.state())
                            .map_err(|_| InternalFault::new("S6a", "relation-pre-state-observes"))
                    })
            })
            .collect::<Result<_, _>>()
            .map_err(ModelRelationRefusal::Fault)?;
        let post_observations: Vec<fsl_eval::simulation::model::StateObservation> = chosen
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                let subject_index = execution_subjects[index];
                graphs[subject_index]
                    .state(edge.target)
                    .ok_or_else(|| InternalFault::new("S6a", "relation-post-state-exists"))
                    .and_then(|state| {
                        systems[subject_index]
                            .observe(state.state.state())
                            .map_err(|_| InternalFault::new("S6a", "relation-post-state-observes"))
                    })
            })
            .collect::<Result<_, _>>()
            .map_err(ModelRelationRefusal::Fault)?;
        let pre: Vec<_> = pre_observations
            .iter()
            .map(|observation| observation.environment())
            .collect();
        let post: Vec<_> = post_observations
            .iter()
            .map(|observation| observation.environment())
            .collect();
        let transitions: Vec<&ModelTransition> = chosen
            .iter()
            .map(|edge| &edge.transition.transition)
            .collect();
        let results: Vec<Option<&str>> = chosen
            .iter()
            .map(|edge| edge.transition.result.as_deref())
            .collect();
        stats.relation_tuples = stats.relation_tuples.checked_add(1).ok_or_else(|| {
            ModelRelationRefusal::Fault(InternalFault::new("EN-1", "relation-count-fits"))
        })?;
        let evaluation = request
            .evaluator
            .evaluate_model_relation(
                request.relation,
                ModelRelationTuple {
                    transitions: &transitions,
                    results: &results,
                    pre: &pre,
                    post: &post,
                },
                &mut Meter::new(request.meter),
            )
            .map_err(ModelRelationRefusal::Fault)?;
        match evaluation {
            fsl_eval::value::ModelRelationEvaluation::Holds => {}
            fsl_eval::value::ModelRelationEvaluation::Refuted => {
                let traces = relation_traces(
                    &graphs,
                    &execution_subjects,
                    &chosen,
                    executions,
                    request.relation.execution_aliases(),
                )
                .map_err(ModelRelationRefusal::Fault)?;
                return Ok(ModelRelationOutcome::Refuted {
                    counterexample: Box::new(HyperCounterexample {
                        kind: HyperCounterexampleKind::StepTuple,
                        instance: Vec::new(),
                        traces,
                        trace_position: 0,
                    }),
                    stats,
                });
            }
            fsl_eval::value::ModelRelationEvaluation::Undefined(cause) => {
                let traces = relation_traces(
                    &graphs,
                    &execution_subjects,
                    &chosen,
                    executions,
                    request.relation.execution_aliases(),
                )
                .map_err(ModelRelationRefusal::Fault)?;
                let state_keys = chosen
                    .iter()
                    .enumerate()
                    .map(|(index, edge)| {
                        graphs[execution_subjects[index]]
                            .state(edge.target)
                            .map(|state| StateKey::from_canonical_bytes(state.key.clone()))
                            .ok_or_else(|| {
                                InternalFault::new("S6a", "relation-undefined-state-exists")
                            })
                    })
                    .collect::<Result<_, _>>()
                    .map_err(ModelRelationRefusal::Fault)?;
                let variables = executions
                    .iter()
                    .zip(request.relation.execution_aliases())
                    .map(|((variable, _), alias)| (variable.clone(), alias.clone()))
                    .collect();
                return Ok(ModelRelationOutcome::Refuted {
                    counterexample: Box::new(HyperCounterexample {
                        kind: HyperCounterexampleKind::Undefined(HyperUndefined {
                            position: 0,
                            state_keys,
                            variables,
                            locus: HyperLocus::RelationBody,
                            transition: None,
                            cause,
                        }),
                        instance: Vec::new(),
                        traces,
                        trace_position: 0,
                    }),
                    stats,
                });
            }
            fsl_eval::value::ModelRelationEvaluation::Undetermined(_) => {
                return Ok(ModelRelationOutcome::Undecided { stats });
            }
        }
        if !advance_tuple(&mut positions, &edges) {
            break;
        }
    }
    Ok(if let Some(depth) = bounded_depth {
        ModelRelationOutcome::BoundReached { depth, stats }
    } else {
        ModelRelationOutcome::Holds {
            certificate: certificate(&request, &graphs)?,
            stats,
        }
    })
}

fn advance_tuple(positions: &mut [usize], choices: &[Vec<usize>]) -> bool {
    for index in (0..positions.len()).rev() {
        positions[index] += 1;
        if positions[index] < choices[index].len() {
            return true;
        }
        positions[index] = 0;
    }
    false
}

fn relation_traces(
    graphs: &[RelationGraph],
    execution_subjects: &[usize],
    chosen: &[&RetainedEdge<RelationTransition>],
    executions: &[(String, fsl_semantics::check::OperationKey)],
    aliases: &[String],
) -> Result<Vec<HyperTrace>, InternalFault> {
    chosen
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            trace_to_edge(
                &graphs[execution_subjects[index]],
                edge,
                &executions[index].0,
                &aliases[index],
            )
        })
        .collect()
}

fn trace_to_edge(
    graph: &RelationGraph,
    last: &RetainedEdge<RelationTransition>,
    variable: &str,
    alias: &str,
) -> Result<HyperTrace, InternalFault> {
    let mut current = last.source;
    let mut path = Vec::new();
    loop {
        if let Some((initial_index, _)) = graph
            .initial()
            .iter()
            .enumerate()
            .find(|(_, initial)| **initial == current)
        {
            let initial = u32::try_from(initial_index)
                .map_err(|_| InternalFault::new("S6a", "relation-initial-index-fits"))?;
            path.reverse();
            path.push(last.clone());
            let mut results = Vec::with_capacity(path.len());
            let steps = path
                .into_iter()
                .map(|edge| {
                    results.push(edge.transition.result.clone());
                    graph
                        .state(edge.target)
                        .map(|target| {
                            HyperStep::Model(ModelStep {
                                transition: edge.transition.transition,
                                post_state: StateKey::from_canonical_bytes(target.key.clone()),
                                delay: None,
                            })
                        })
                        .ok_or_else(|| InternalFault::new("S6a", "relation-trace-target-exists"))
                })
                .collect::<Result<_, _>>()?;
            return Ok(HyperTrace {
                variable: variable.to_owned(),
                alias: alias.to_owned(),
                initial,
                steps,
                results,
                marks: Vec::new(),
            });
        }
        let state = graph
            .state(current)
            .ok_or_else(|| InternalFault::new("S6a", "relation-trace-state-exists"))?;
        if state.depth == 0 {
            return Err(InternalFault::new("S6a", "relation-trace-reaches-initial"));
        }
        let incoming = graph.edges().iter().find(|edge| {
            edge.target == current
                && graph
                    .state(edge.source)
                    .is_some_and(|source| source.depth.saturating_add(1) == state.depth)
        });
        let Some(incoming) = incoming else {
            return Err(InternalFault::new(
                "S6a",
                "relation-trace-predecessor-exists",
            ));
        };
        path.push(incoming.clone());
        current = incoming.source;
    }
}

/// Write only the retained evidence; the independent checker recomputes it.
fn certificate(
    request: &ModelRelationInput<'_>,
    graphs: &[RelationGraph],
) -> Result<fsl_replay::ProductClosureCertificate, ModelRelationRefusal> {
    let mut subjects = Vec::new();
    for (alias, graph) in request.relation.aliases().iter().zip(graphs) {
        let mut states: Vec<_> = graph
            .states()
            .iter()
            .map(|state| StateKey::from_canonical_bytes(state.key.clone()))
            .collect();
        states.sort();
        let mut executions = Vec::new();
        for ((name, operation), owner) in request
            .relation
            .executions()
            .iter()
            .zip(request.relation.execution_aliases())
        {
            if owner != alias {
                continue;
            }
            let operation = fsl_semantics::check::operation_text(operation);
            let transitions = graph
                .edges()
                .iter()
                .filter(|edge| edge.transition.transition.operation() == operation)
                .map(|edge| {
                    let state_key = |id| {
                        graph
                            .state(id)
                            .map(|state| StateKey::from_canonical_bytes(state.key.clone()))
                            .ok_or_else(|| {
                                ModelRelationRefusal::Fault(InternalFault::new(
                                    "EN-1",
                                    "relation-edge-state",
                                ))
                            })
                    };
                    Ok(fsl_replay::HyperTransitionEdge {
                        pre_state: state_key(edge.source)?,
                        transition: edge.transition.transition.clone(),
                        post_state: state_key(edge.target)?,
                        result: edge.transition.result.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ModelRelationRefusal>>()?;
            executions.push(fsl_replay::HyperExecutionEdges {
                execution: name.clone(),
                transitions,
            });
        }
        subjects.push(fsl_replay::HyperSubjectClosure {
            alias: alias.clone(),
            states,
            executions,
        });
    }
    Ok(fsl_replay::ProductClosureCertificate {
        form: fsl_semantics::check::HyperForm::StepRelation,
        instance: Vec::new(),
        reduction: None,
        closure: fsl_replay::ProductClosure::Relation(subjects),
        components: None,
    })
}
