// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 model-based run results and native state-invariant execution.

use std::collections::{BTreeMap, HashMap, VecDeque};

use filament_exact::{Meter, NodeKey};
use filament_semantic_value::catalog::Category;
use fsl_foundation::selection::DefinitionRef;
use fsl_semantics::check::TemporalClauseNode;

use crate::simulation::{
    expansion::{ExpansionLimit, ExpansionStop},
    EncodingRefusal, Limits, TransitionSystem,
};

use super::{
    model_operation_coverage, sample_model, shrink_model_trace, ModelBoundaryPartition,
    ModelFinding, ModelOperation, ModelOperationCase, ModelOperationCoverageEntry,
    ModelOperationCoverageLimit, ModelOperationCoverageLimits, ModelOperationCoverageReason,
    ModelOperationCoverageReasonKind, ModelOperationCoverageSummaryStatus, ModelSampleError,
    ModelShrinkError, ModelSystem, ModelTrace,
};
use crate::simulation::trace::SampleProvenance;

/// The number of model-based cases evaluated by the run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelBasedTestCounts {
    /// Cases that passed native evaluation.
    pub passing: u64,
    /// Cases that failed native evaluation.
    pub failing: u64,
    /// Cases discarded by a domain or precondition.
    pub discarded: u64,
}

/// The evidence produced by a model-based test run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelBasedTestEvidence {
    /// No generated case failed; this settles no verdict.
    Tested,
    /// A failing case was shrunk and replayed.
    Refuted,
}

/// The operation-coverage summary in an FR-190 run record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedCoverage {
    /// Complete when every operation was covered, partial otherwise.
    pub status: ModelOperationCoverageSummaryStatus,
    /// Counts of uncovered operations by reason.
    pub uncovered: BTreeMap<ModelOperationCoverageReasonKind, usize>,
    /// Counts of operations whose reachability remained undetermined.
    pub limits: ModelOperationCoverageLimits,
}

/// The FR-190 result record for a run over a checked state invariant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedTestResult {
    /// The checked invariant selected by the run.
    pub clause: String,
    /// The selected generation profile.
    pub profile: DefinitionRef,
    /// The generator seed.
    pub seed: u64,
    /// The requested maximum number of cases.
    pub max_cases: u64,
    /// The requested maximum trace length.
    pub max_trace_length: usize,
    /// Passing, failing and discarded case counts.
    pub counts: ModelBasedTestCounts,
    /// The model's boundary partitions.
    pub partitions: Vec<ModelBoundaryPartition>,
    /// Sampling provenance for each evaluated generated case.
    pub provenance: Vec<SampleProvenance>,
    /// One entry per declared operation.
    pub operations: Vec<ModelOperationCoverageEntry>,
    /// Operation coverage summary.
    pub coverage: ModelBasedCoverage,
    /// The optional failing trace, shrunk and replayable against its model.
    pub counterexample: Option<ModelTrace>,
    /// The evidence kind.
    pub evidence: ModelBasedTestEvidence,
}

/// A model-based state-invariant run could not complete.
#[derive(Debug, thiserror::Error)]
pub enum ModelBasedTestError {
    /// Trace generation or replay failed.
    #[error(transparent)]
    Sample(#[from] ModelSampleError),
    /// Counterexample shrinking or replay failed.
    #[error(transparent)]
    Shrink(#[from] ModelShrinkError),
    /// Reachability could not encode a model state.
    #[error(transparent)]
    Key(#[from] EncodingRefusal),
    /// Native evaluation of the target invariant was undetermined.
    #[error("the target invariant could not be decided")]
    InvariantUndetermined {
        /// The checked invariant name.
        clause: String,
    },
    /// The model could not evaluate the target invariant at a sampled state.
    #[error("native model expansion stopped while evaluating the target invariant")]
    Expansion(ExpansionStop),
    /// Coverage input did not match the declared model operations.
    #[error("operation coverage input was inconsistent")]
    Coverage,
    /// The selected clause is absent or is not a state invariant.
    #[error("the selected clause is not a state invariant: {clause}")]
    InvalidClause {
        /// The requested clause name.
        clause: String,
    },
    /// The checked clause is valid, but its native model-based semantics are
    /// awaiting the FR-752 contract/value ruling.
    #[error(
        "the checked clause kind is not supported by the model-based runner: {clause} ({kind})"
    )]
    UnsupportedClause {
        /// The checked clause name.
        clause: String,
        /// The checked clause kind.
        kind: String,
    },
    /// A temporal clause could not be read through the model's native letters.
    #[error("the temporal clause could not be evaluated: {0}")]
    Temporal(#[from] crate::temporal::TemporalRefusal),
    /// The model could not construct native letters for the temporal clause.
    #[error("the temporal clause's native model letters could not be constructed: {0}")]
    ClauseLetters(#[from] super::LettersRefusal),
    /// A temporal clause could not be evaluated for the generated model trace.
    #[error("the temporal clause's native model letters stopped: {0:?}")]
    TemporalLetters(crate::temporal::LetterStop),
    /// A checked clause returned an inconclusive outcome for a generated case.
    #[error("the selected clause was not decided for a generated case: {outcome}")]
    ClauseUndetermined {
        /// The selected clause name.
        clause: String,
        /// The typed outcome that did not decide the clause.
        outcome: String,
    },
    /// The model system could not derive the clause's model instances.
    #[error(transparent)]
    NotSimulated(#[from] crate::simulation::NotSimulated),
}

/// Run seeded model-based cases for the invariant named by clause.
///
/// The model expansion evaluates invariant bodies through FR-181. A failing
/// trace is minimized by the native model shrinker and every accepted
/// candidate is replayed before it is retained. The same model graph is
/// explored for operation reachability, independently of the sampled cases.
///
/// # Errors
///
/// Returns an error when sampling, native evaluation, replay, shrinking or
/// operation reachability cannot complete.
#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
pub fn run_model_based_state_invariant(
    system: &ModelSystem<'_>,
    clause: &str,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    let valid_invariant = system
        .source_package()
        .and_then(|package| package.graph().state_clause(clause))
        .is_some_and(|checked| checked.is_invariant());
    if !valid_invariant {
        return Err(ModelBasedTestError::InvalidClause {
            clause: clause.to_owned(),
        });
    }

    run_model_based_with_oracle(
        system,
        clause,
        sampler,
        seed,
        max_cases,
        max_trace_length,
        position_limit,
        reachability_limits,
        |trace| invariant_result(system, trace, clause),
    )
}

/// Run seeded model-based cases for one checked clause identity, selecting
/// the native state or temporal evaluator from the checked graph.
#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
pub fn run_model_based_test(
    system: &ModelSystem<'_>,
    identity: &NodeKey,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    let package = system
        .source_package()
        .ok_or_else(|| ModelBasedTestError::InvalidClause {
            clause: format!("{identity:?}"),
        })?;
    let graph = package.graph();
    if let Some(checked) = graph.state_clause_by_identity(identity.clone()) {
        if checked.is_invariant() {
            return run_model_based_state_invariant(
                system,
                checked.name(),
                sampler,
                seed,
                max_cases,
                max_trace_length,
                position_limit,
                reachability_limits,
            );
        }
        return Err(ModelBasedTestError::UnsupportedClause {
            clause: checked.name().to_owned(),
            kind: format!("{:?}", checked.kind()),
        });
    }
    if let Some(clause) = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *identity)
    {
        return run_model_based_temporal_clause(
            system,
            clause,
            sampler,
            seed,
            max_cases,
            max_trace_length,
            position_limit,
            reachability_limits,
        );
    }
    Err(ModelBasedTestError::InvalidClause {
        clause: format!("{identity:?}"),
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
fn run_model_based_temporal_clause(
    system: &ModelSystem<'_>,
    clause: &TemporalClauseNode,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    run_model_based_with_oracle(
        system,
        clause.clause().name(),
        sampler,
        seed,
        max_cases,
        max_trace_length,
        position_limit,
        reachability_limits,
        |trace| temporal_result(system, clause, trace),
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
fn run_model_based_with_oracle<F>(
    system: &ModelSystem<'_>,
    clause: &str,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    position_limit: u64,
    reachability_limits: Limits,
    mut oracle: F,
) -> Result<ModelBasedTestResult, ModelBasedTestError>
where
    F: FnMut(&ModelTrace) -> Result<CaseResult, ModelBasedTestError>,
{
    let mut generated_cases = Vec::new();
    let mut partitions = Vec::new();
    let mut provenance = Vec::new();
    let mut counts = ModelBasedTestCounts::default();
    let mut counterexample = None;

    for case_index in 0..max_cases {
        let trace = sample_model(
            system,
            position_limit,
            sampler,
            seed,
            case_index,
            max_trace_length,
        )?;
        if let Some(sample) = &trace.trace.provenance {
            provenance.push(sample.clone());
        }
        for partition in &trace.boundary_partitions {
            if !partitions.contains(partition) {
                partitions.push(partition.clone());
            }
        }
        match oracle(&trace)? {
            CaseResult::Pass => {
                generated_cases.push(case_for(&trace, case_index));
                counts.passing = counts.passing.saturating_add(1);
            }
            CaseResult::Discard => counts.discarded = counts.discarded.saturating_add(1),
            CaseResult::Fail(disposition) => {
                generated_cases.push(case_for(&trace, case_index));
                counts.failing = counts.failing.saturating_add(1);
                let mut error = None;
                let minimized =
                    shrink_model_trace(system, &trace, |candidate| match oracle(candidate) {
                        Ok(CaseResult::Fail(candidate_disposition)) => {
                            candidate_disposition == disposition
                        }
                        Ok(CaseResult::Pass | CaseResult::Discard) => false,
                        Err(run_error) => {
                            error = Some(run_error);
                            false
                        }
                    })?;
                if let Some(run_error) = error {
                    return Err(run_error);
                }
                counterexample = Some(minimized);
                break;
            }
        }
    }

    let declared: Vec<ModelOperation> = system
        .operations()
        .into_iter()
        .map(|(_, name, identity)| ModelOperation {
            identity: identity.to_owned(),
            name: name.to_owned(),
        })
        .collect();
    let reachability = operation_reachability(system, max_trace_length, reachability_limits)?;
    let operation_coverage = model_operation_coverage(&declared, &generated_cases, &reachability)
        .ok_or(ModelBasedTestError::Coverage)?;

    Ok(ModelBasedTestResult {
        clause: clause.to_owned(),
        profile: sampler.clone(),
        seed,
        max_cases,
        max_trace_length,
        counts,
        partitions,
        provenance,
        operations: operation_coverage.operations.clone(),
        coverage: ModelBasedCoverage {
            status: operation_coverage.status,
            uncovered: operation_coverage.uncovered,
            limits: operation_coverage.limits,
        },
        evidence: if counterexample.is_some() {
            ModelBasedTestEvidence::Refuted
        } else {
            ModelBasedTestEvidence::Tested
        },
        counterexample,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CaseResult {
    Pass,
    Fail(String),
    Discard,
}

fn case_for(trace: &ModelTrace, case_index: u64) -> ModelOperationCase {
    ModelOperationCase {
        case_index,
        operations: trace
            .trace
            .steps
            .iter()
            .map(|step| step.transition.operation().to_owned())
            .collect(),
    }
}

fn invariant_result(
    system: &ModelSystem<'_>,
    trace: &ModelTrace,
    clause: &str,
) -> Result<CaseResult, ModelBasedTestError> {
    let Some((_, states)) = super::rebuild_model_trace(
        system,
        &trace.trace.initial,
        &trace.trace.steps,
        trace.trace.provenance.as_ref(),
    )?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    for state in &states {
        let findings = system
            .invariant_finding_alone(state, clause)
            .map_err(ModelBasedTestError::Expansion)?;
        for finding in findings {
            match finding {
                ModelFinding::InvariantViolated { clause: found, .. } if found == clause => {
                    return Ok(CaseResult::Fail("invariant-violated".to_owned()));
                }
                ModelFinding::InvariantUndetermined { clause: found, .. } if found == clause => {
                    return Err(ModelBasedTestError::InvariantUndetermined {
                        clause: clause.to_owned(),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(CaseResult::Pass)
}

fn temporal_result(
    system: &ModelSystem<'_>,
    node: &TemporalClauseNode,
    trace: &ModelTrace,
) -> Result<CaseResult, ModelBasedTestError> {
    let Some((_, states)) = super::rebuild_model_trace(
        system,
        &trace.trace.initial,
        &trace.trace.steps,
        trace.trace.provenance.as_ref(),
    )?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    let instances = system.instances(&node.clause().over().1)?;
    if instances.is_empty() {
        return Err(ModelBasedTestError::ClauseUndetermined {
            clause: node.clause().name().to_owned(),
            outcome: "temporal clause has no model instances".to_owned(),
        });
    }
    let trace_shape = crate::temporal::TemporalTrace::Finite {
        len: u64::try_from(states.len()).map_err(|_| ModelBasedTestError::ClauseUndetermined {
            clause: node.clause().name().to_owned(),
            outcome: "trace length exceeds u64".to_owned(),
        })?,
    };
    let mut failing = None;
    let mut discarded = false;
    for instance in instances {
        let mut letters = system.clause_letters(node)?;
        let mut temporal_letters = ModelTemporalLetters {
            letters: &mut letters,
            instance,
            states: &states,
            node,
        };
        let outcome = crate::temporal::evaluate(
            node.clause(),
            trace_shape,
            &mut temporal_letters,
            &mut Meter::new(system.meter_limits()),
        )?;
        match outcome.category() {
            Category::Success => {}
            Category::Violation => {
                let disposition = match outcome {
                    crate::temporal::TemporalOutcome::Violated { .. } => "temporal-violated",
                    crate::temporal::TemporalOutcome::Undefined(_) => "temporal-undefined",
                    _ => "temporal-violation",
                };
                failing = Some(disposition.to_owned());
                break;
            }
            Category::Undefined
            | Category::Pending
            | Category::Indeterminate
            | Category::Inconclusive => discarded = true,
            Category::Unsupported | Category::Refusal | Category::InternalFailure => {
                return Err(ModelBasedTestError::ClauseUndetermined {
                    clause: node.clause().name().to_owned(),
                    outcome: format!("{outcome:?}"),
                });
            }
            Category::Incomplete => {
                return Err(ModelBasedTestError::ClauseUndetermined {
                    clause: node.clause().name().to_owned(),
                    outcome: format!("{outcome:?}"),
                });
            }
        }
    }
    Ok(failing.map_or_else(
        || {
            if discarded {
                CaseResult::Discard
            } else {
                CaseResult::Pass
            }
        },
        CaseResult::Fail,
    ))
}

struct ModelTemporalLetters<'l, 's, 'a> {
    letters: &'l mut super::ClauseLetters<'s, 'a>,
    instance: crate::model_subject::ModelInstance,
    states: &'l [super::ModelSimState],
    node: &'l TemporalClauseNode,
}

impl ModelTemporalLetters<'_, '_, '_> {
    fn state(
        &self,
        position: crate::temporal::TemporalPosition,
    ) -> Result<&super::ModelSimState, crate::temporal::LetterStop> {
        let index = usize::try_from(position.index()).map_err(|_| {
            crate::temporal::LetterStop::Fault(
                filament_semantic_value::catalog::InternalFault::new(
                    "model-based-test",
                    "temporal-position-fits-trace",
                ),
            )
        })?;
        self.states.get(index).ok_or_else(|| {
            crate::temporal::LetterStop::Fault(
                filament_semantic_value::catalog::InternalFault::new(
                    "model-based-test",
                    "temporal-position-is-in-trace",
                ),
            )
        })
    }
}

impl crate::temporal::TemporalLetters for ModelTemporalLetters<'_, '_, '_> {
    fn atom(
        &mut self,
        atom: fsl_semantics::check::FormulaNodeId,
        position: crate::temporal::TemporalPosition,
        meter: &mut Meter,
    ) -> Result<crate::temporal::AtomReading, crate::temporal::LetterStop> {
        crate::model_subject::ModelLetters::atom(
            self.letters,
            atom,
            &self.instance,
            self.state(position)?,
            meter,
        )
        .map_err(Into::into)
    }

    fn activation(
        &mut self,
        position: crate::temporal::TemporalPosition,
        meter: &mut Meter,
    ) -> Result<crate::temporal::AtomReading, crate::temporal::LetterStop> {
        crate::model_subject::ModelLetters::activation(
            self.letters,
            &self.instance,
            self.state(position)?,
            meter,
        )
        .map_err(Into::into)
    }

    fn captures(
        &mut self,
        position: crate::temporal::TemporalPosition,
        meter: &mut Meter,
    ) -> Result<crate::temporal::CaptureReading, crate::temporal::LetterStop> {
        crate::model_subject::ModelLetters::captures(
            self.letters,
            &self.instance,
            self.state(position)?,
            meter,
        )
        .map_err(Into::into)
    }

    fn reads_captures(&self, atom: fsl_semantics::check::FormulaNodeId) -> bool {
        self.node.atom_reads_captures(atom)
    }
}

fn operation_reachability(
    system: &ModelSystem<'_>,
    max_depth: usize,
    limits: Limits,
) -> Result<Vec<(String, ModelOperationCoverageReason)>, ModelBasedTestError> {
    struct Pending {
        state: super::ModelSimState,
        path: Vec<String>,
        depth: usize,
    }

    let operations = system.operations();
    let mut enabled = BTreeMap::<String, Vec<String>>::new();
    let mut visited = HashMap::<Vec<u8>, usize>::new();
    let mut queue = VecDeque::new();
    let mut stopped = None;
    for initial in system.initial() {
        let (key, _) = system.state_key(&initial)?;
        if visited.contains_key(&key) {
            continue;
        }
        if visited.len() >= limits.max_states {
            stopped = Some(ModelOperationCoverageLimit::States);
            break;
        }
        visited.insert(key, 0);
        queue.push_back(Pending {
            state: initial,
            path: Vec::new(),
            depth: 0,
        });
    }

    let mut transitions = 0_usize;
    while let Some(current) = queue.pop_front() {
        if current.depth >= max_depth {
            queue.push_front(current);
            break;
        }
        let expansion = match system.successors(&current.state) {
            Ok(expansion) => expansion,
            Err(stop) => match stop.limit {
                Some(ExpansionLimit::MaxCandidates) => {
                    stopped = Some(ModelOperationCoverageLimit::Candidates);
                    break;
                }
                Some(ExpansionLimit::EvaluationMeter(_)) | None => {
                    return Err(ModelBasedTestError::Expansion(stop));
                }
            },
        };
        let mut successors = expansion.successors;
        successors.sort_by(|left, right| left.0.cmp(&right.0));
        for (transition, state) in successors {
            if transitions >= limits.max_transitions {
                stopped = Some(ModelOperationCoverageLimit::Transitions);
                break;
            }
            transitions = transitions.saturating_add(1);
            enabled
                .entry(transition.operation().to_owned())
                .or_insert_with(|| current.path.clone());
            let (key, _) = system.state_key(&state)?;
            let depth = current.depth.saturating_add(1);
            if visited.contains_key(&key) {
                continue;
            }
            if visited.len() >= limits.max_states {
                stopped = Some(ModelOperationCoverageLimit::States);
                break;
            }
            visited.insert(key, depth);
            let mut path = current.path.clone();
            path.push(transition.operation().to_owned());
            queue.push_back(Pending { state, path, depth });
        }
        if stopped.is_some() {
            break;
        }
    }

    let bounded = !queue.is_empty();
    Ok(operations
        .into_iter()
        .map(|(_, _, identity)| {
            let reason = if let Some(path) = enabled.get(identity) {
                ModelOperationCoverageReason::NotGenerated {
                    enabled_at: path.clone(),
                }
            } else if let Some(limit) = stopped {
                ModelOperationCoverageReason::Undetermined { limit }
            } else if bounded {
                ModelOperationCoverageReason::UnreachableWithinBound
            } else {
                ModelOperationCoverageReason::Unreachable
            };
            (identity.to_owned(), reason)
        })
        .collect())
}
