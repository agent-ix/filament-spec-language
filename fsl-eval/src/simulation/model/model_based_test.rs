// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 model-based run results and native state-invariant execution.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ops::Deref;

use filament_eval::{CallFailure, FamilyOutcome, FamilyResult};
use filament_exact::{Integer, IntegerInterval, Meter, NodeKey, Outcome, Value, ValueType};
use filament_semantic_value::catalog::Category;
use fsl_foundation::selection::DefinitionRef;
use fsl_semantics::check::{CallableFunction, TemporalClauseNode};
use fsl_semantics::model::object_environment::ObjectEnvironment;

use crate::simulation::order::Expanded;
use crate::simulation::{
    EncodingRefusal, Limits, TransitionSystem,
    expansion::{ExpansionLimit, ExpansionStop},
    trace::{Step, Trace},
};

use super::domain::{Domain, admitted_text_value};
use super::{ContractClause, ContractClauseKind};
use super::{
    ModelBoundaryPartition, ModelFinding, ModelOperation, ModelOperationCase,
    ModelOperationCoverageEntry, ModelOperationCoverageLimit, ModelOperationCoverageLimits,
    ModelOperationCoverageReason, ModelOperationCoverageReasonKind,
    ModelOperationCoverageSummaryStatus, ModelPartitionStatus, ModelPartitionUncoveredReason,
    ModelSampleError, ModelShrinkError, ModelSimState, ModelSystem, ModelTrace,
    model_operation_coverage, sample_model_with_omitted_clause,
    shrink_model_trace_with_omitted_clause,
};
use crate::simulation::choice::ChoicePoint;
use crate::simulation::trace::SampleProvenance;
use crate::value::{CheckedPackageEvaluation, QualifiedName};

/// The number of model-based cases evaluated by the run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelBasedTestCounts {
    /// Cases drawn, including cases later discarded.
    pub generated: u64,
    /// Cases that passed native evaluation.
    pub passing: u64,
    /// Cases that failed native evaluation.
    pub failing: u64,
    /// Cases discarded by a domain or precondition.
    pub discarded: u64,
}

/// Why a generated case did not reach a decided claim check.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ModelBasedTestDiscardReason {
    /// A partition value was refused by native domain admission.
    DomainRefusal,
    /// A generated operation's precondition was false for a postcondition
    /// check.
    PreconditionFalse,
    /// An invariant premise was false at the check point.
    InvariantFalse,
    /// No applicable check point was reached by the generated case.
    NoCheckPoint,
}

/// Count and first occurrence of one model-based discard reason.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelBasedTestDiscardCount {
    /// Number of discarded cases with this reason.
    pub count: u64,
    /// First zero-based generated case with this reason.
    pub first_case: Option<u64>,
}

/// Check points evaluated or skipped under the clause's premises.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedTestChecks {
    /// Check points evaluated, including failing points.
    pub evaluated: u64,
    /// Skipped points, grouped by the false premise.
    pub skipped: BTreeMap<ModelBasedTestDiscardReason, u64>,
}

impl Default for ModelBasedTestChecks {
    fn default() -> Self {
        Self {
            evaluated: 0,
            skipped: BTreeMap::from([
                (ModelBasedTestDiscardReason::PreconditionFalse, 0),
                (ModelBasedTestDiscardReason::InvariantFalse, 0),
            ]),
        }
    }
}

/// Summary of the root partitions exercised by the suite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedPartitionCoverage {
    /// Complete when every partition was covered or refused.
    pub status: ModelOperationCoverageSummaryStatus,
    /// Number of uncovered partitions grouped by their reason.
    pub uncovered: BTreeMap<String, u64>,
}

/// Per-case draw provenance, including the value-size bound used to admit the
/// generated roots.
#[derive(Clone, Debug)]
pub struct ModelBasedCaseProvenance {
    /// The seeded trace draw.
    pub sample: Option<SampleProvenance>,
    /// The typed parameter assignment for a value or pure-Boolean case.
    pub assignment: Option<Vec<Value>>,
    /// The requested size bound for otherwise-unbounded integer roots.
    pub max_value_bits: Option<u64>,
}

impl PartialEq for ModelBasedCaseProvenance {
    fn eq(&self, other: &Self) -> bool {
        self.sample == other.sample
            && match (&self.assignment, &other.assignment) {
                (Some(left), Some(right)) => super::probabilistic::same_values(left, right),
                (None, None) => true,
                _ => false,
            }
            && self.max_value_bits == other.max_value_bits
    }
}

impl Eq for ModelBasedCaseProvenance {}

/// One root admitted from the size-bounded domain for an otherwise-unbounded
/// integer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedSizeBoundedRoot {
    /// The structural root declaration identity.
    pub node: NodeKey,
    /// FR-331 child path to the root.
    pub path: Vec<u32>,
    /// The number of values in the size-bounded domain.
    pub values: String,
}

/// A value-bound cut observed while constructing operation successors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedValueBoundCut {
    /// The operation declaration identity.
    pub operation: String,
    /// The state key at which an enabled invocation had no admitted result.
    pub state: String,
}

/// Value-bound cut provenance and its total count.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelBasedValueBoundCuts {
    /// Number of value-bound cuts observed.
    pub count: u64,
    /// The operation/state pairs that caused those cuts.
    pub entries: Vec<ModelBasedValueBoundCut>,
}

/// The provenance of one completed model-based run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedProvenance {
    /// The requested size bound, if the run has an unbounded integer root.
    pub max_value_bits: Option<u64>,
    /// One entry per size-bounded integer root.
    pub size_bounded: Vec<ModelBasedSizeBoundedRoot>,
    /// Value-bound cuts observed during model expansion.
    pub value_bound_cuts: ModelBasedValueBoundCuts,
    /// Provenance for each generated case.
    pub cases: Vec<ModelBasedCaseProvenance>,
}

impl Deref for ModelBasedProvenance {
    type Target = [ModelBasedCaseProvenance];

    fn deref(&self) -> &Self::Target {
        &self.cases
    }
}

/// The retained native counterexample, either a replayable model trace or a
/// value assignment.
#[derive(Clone, Debug)]
pub enum ModelBasedCounterexample {
    /// A shrunk trace replayable against the model system.
    Trace(Box<ModelTrace>),
    /// A typed assignment replayable against the checked value function.
    ValueAssignment(Vec<Value>),
}

impl PartialEq for ModelBasedCounterexample {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Trace(left), Self::Trace(right)) => left == right,
            (Self::ValueAssignment(left), Self::ValueAssignment(right)) => {
                super::probabilistic::same_values(left, right)
            }
            _ => false,
        }
    }
}

impl Eq for ModelBasedCounterexample {}

/// The evidence produced by a model-based test run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelBasedTestEvidence {
    /// No generated case failed; this settles no verdict.
    Tested,
    /// A failing case was shrunk and replayed.
    Refuted,
    /// Every generated case was discarded, so no case was evaluated.
    Inconclusive,
    /// A resource limit stopped generation before completion.
    Incomplete,
}

impl ModelBasedTestEvidence {
    /// FR-301 exit category for the clause evidence.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Tested => Category::Success.exit_code(),
            Self::Refuted => Category::Violation.exit_code(),
            Self::Inconclusive => Category::Inconclusive.exit_code(),
            Self::Incomplete => Category::Incomplete.exit_code(),
        }
    }
}

/// FR-190's completed-run cause when every generated case is discarded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelBasedTestCause {
    /// No generated case reached an evaluated check point.
    NoEvaluatedCase,
    /// The scalar accounting budget could not cover generation work.
    LimitReached {
        /// Counter whose budget was exhausted.
        limit: filament_exact::LimitKind,
        /// Configured limit, rather than the denied charge.
        value: u64,
        /// Setting that raises the reached limit.
        setting: String,
    },
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

/// The FR-190 result record for a run over a checked clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBasedTestResult {
    /// The checked clause selected by the run.
    pub clause: String,
    /// The selected generation profile.
    pub profile: DefinitionRef,
    /// The generator seed.
    pub seed: u64,
    /// The requested maximum number of cases.
    pub max_cases: u64,
    /// The requested maximum trace length.
    pub max_trace_length: usize,
    /// Maximum integer value size used to bound unbounded roots.
    pub max_value_bits: Option<u64>,
    /// Passing, failing and discarded case counts.
    pub counts: ModelBasedTestCounts,
    /// Discarded cases grouped by reason, with their first case index.
    pub discarded_by: BTreeMap<ModelBasedTestDiscardReason, ModelBasedTestDiscardCount>,
    /// Native clause check-point accounting, excluding shrink replays.
    pub checks: ModelBasedTestChecks,
    /// The model's boundary partitions.
    pub partitions: Vec<ModelBoundaryPartition>,
    /// Coverage of the declared root partitions.
    pub partition_coverage: ModelBasedPartitionCoverage,
    /// Sampling provenance for every generated case, including discards.
    pub provenance: ModelBasedProvenance,
    /// One entry per declared operation.
    pub operations: Option<Vec<ModelOperationCoverageEntry>>,
    /// Operation coverage summary.
    pub coverage: Option<ModelBasedCoverage>,
    /// The FR-331 settlement of the selected clause.
    pub result: ModelBasedTestEvidence,
    /// The typed settlement cause, when one applies.
    pub cause: Option<ModelBasedTestCause>,
    /// The optional failing trace, shrunk and replayable against its model.
    pub counterexample: Option<ModelBasedCounterexample>,
    /// The evidence kind.
    pub evidence: ModelBasedTestEvidence,
}

impl ModelBasedTestResult {
    /// FR-301 process exit for the method result.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.evidence.exit_code()
    }
}

/// A model-based run could not complete.
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
    /// Native evaluation of a checked value function failed before it
    /// produced an outcome.
    #[error(transparent)]
    ValueCall(#[from] CallFailure),
    /// A checked parameter has no finite generation domain under this run's
    /// size bound.
    #[error("the value clause parameter has no finite generation domain: {parameter}")]
    ValueDomain {
        /// The parameter name.
        parameter: String,
    },
    /// A model boundary partition's declared refusal value was admitted by
    /// the checked domain.
    #[error("native admission accepted a generated refusal partition for {parameter}")]
    OracleDisagreement {
        /// The parameter name.
        parameter: String,
    },
    /// The selected clause is absent or is not a state invariant.
    #[error("the selected clause is not a state invariant: {clause}")]
    InvalidClause {
        /// The requested clause name.
        clause: String,
    },
    /// A required unsigned request limit is outside its domain.
    #[error("the model-based request has an invalid limit: {member}")]
    InvalidLimit {
        /// The invalid request member.
        member: &'static str,
    },
    /// The request omitted the size bound required by an unbounded integer
    /// root.
    #[error("the model-based request is missing max_value_bits")]
    MissingValueBits,
    /// The request supplied a size bound but has no unbounded integer root.
    #[error("max_value_bits does not bound a generation root")]
    UnknownValueBits,
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
    run_model_based_state_invariant_with_value_bits(
        system,
        clause,
        sampler,
        seed,
        max_cases,
        max_trace_length,
        None,
        position_limit,
        reachability_limits,
    )
}

/// Run an invariant model test with FR-190's optional size bound for
/// unbounded integer roots.
#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
pub fn run_model_based_state_invariant_with_value_bits(
    system: &ModelSystem<'_>,
    clause: &str,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    max_value_bits: Option<u64>,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    validate_limits(max_cases, max_trace_length)?;
    let _value_bits = install_value_bits(system, max_value_bits, &[], true)?;
    let valid_invariant = system
        .source_package()
        .and_then(|package| package.graph().state_clause(clause))
        .is_some_and(|checked| checked.is_invariant());
    if !valid_invariant {
        return Err(ModelBasedTestError::InvalidClause {
            clause: clause.to_owned(),
        });
    }

    system.reset_value_bound_cuts();
    run_model_based_with_oracle(
        system,
        clause,
        sampler,
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        position_limit,
        reachability_limits,
        None,
        None,
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
    run_model_based_test_with_value_bits(
        system,
        identity,
        sampler,
        seed,
        max_cases,
        max_trace_length,
        None,
        position_limit,
        reachability_limits,
    )
}

/// Run a checked model clause with FR-190's optional unbounded-integer size
/// bound.
#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
pub fn run_model_based_test_with_value_bits(
    system: &ModelSystem<'_>,
    identity: &NodeKey,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    max_value_bits: Option<u64>,
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
        validate_limits(max_cases, max_trace_length)?;
        if checked.is_invariant() {
            return run_model_based_state_invariant_with_value_bits(
                system,
                checked.name(),
                sampler,
                seed,
                max_cases,
                max_trace_length,
                max_value_bits,
                position_limit,
                reachability_limits,
            );
        }
        let contract = system
            .contract_clauses()
            .into_iter()
            .find(|contract| contract.identity == *identity)
            .ok_or_else(|| ModelBasedTestError::InvalidClause {
                clause: checked.name().to_owned(),
            })?;
        let _value_bits = install_value_bits(system, max_value_bits, &[], true)?;
        return run_model_based_contract_clause(
            system,
            &contract,
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
            position_limit,
            reachability_limits,
        );
    }
    if let Some(clause) = graph
        .temporal_clauses()
        .iter()
        .find(|clause| clause.identity() == *identity)
    {
        validate_limits(max_cases, max_trace_length)?;
        let _value_bits = install_value_bits(system, max_value_bits, &[], true)?;
        return run_model_based_temporal_clause(
            system,
            clause,
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
            position_limit,
            reachability_limits,
        );
    }
    if let Some(function) = graph.callable_by_identity(identity.clone()) {
        validate_case_limit(max_cases)?;
        let parameter_types: Vec<ValueType> = function
            .parameters
            .iter()
            .map(|(_, value_type)| value_type.clone())
            .collect();
        let _value_bits = install_value_bits(system, max_value_bits, &parameter_types, false)?;
        return run_model_based_value_function(
            system,
            package,
            identity,
            &function,
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
        );
    }
    Err(ModelBasedTestError::InvalidClause {
        clause: format!("{identity:?}"),
    })
}

fn validate_limits(max_cases: u64, max_trace_length: usize) -> Result<(), ModelBasedTestError> {
    validate_case_limit(max_cases)?;
    if max_trace_length == 0 {
        return Err(ModelBasedTestError::InvalidLimit {
            member: "max_trace_length",
        });
    }
    Ok(())
}

fn validate_case_limit(max_cases: u64) -> Result<(), ModelBasedTestError> {
    if max_cases == 0 {
        return Err(ModelBasedTestError::InvalidLimit {
            member: "max_cases",
        });
    }
    Ok(())
}

struct ValueBitsGuard<'a> {
    system: &'a ModelSystem<'a>,
    previous: u8,
}

impl Drop for ValueBitsGuard<'_> {
    fn drop(&mut self) {
        self.system
            .max_value_bits
            .store(self.previous, std::sync::atomic::Ordering::Relaxed);
    }
}

fn install_value_bits<'a>(
    system: &'a ModelSystem<'a>,
    requested: Option<u64>,
    additional_roots: &[ValueType],
    include_model_roots: bool,
) -> Result<ValueBitsGuard<'a>, ModelBasedTestError> {
    if requested.is_some_and(|bits| !(1..=64).contains(&bits)) {
        return Err(ModelBasedTestError::InvalidLimit {
            member: "max_value_bits",
        });
    }
    let prior = system
        .max_value_bits
        .swap(0, std::sync::atomic::Ordering::Relaxed);
    let needs_bound = additional_roots
        .iter()
        .any(|value_type| matches!(value_type, ValueType::Integer))
        || (include_model_roots
            && system
                .rooted_domains()
                .iter()
                .any(|(_, _, value_type)| matches!(value_type, ValueType::Integer)));
    system
        .max_value_bits
        .store(prior, std::sync::atomic::Ordering::Relaxed);
    match (needs_bound, requested) {
        (true, None) => return Err(ModelBasedTestError::MissingValueBits),
        (false, Some(_)) => return Err(ModelBasedTestError::UnknownValueBits),
        _ => {}
    }
    let value = match requested {
        Some(bits) => u8::try_from(bits).map_err(|_| ModelBasedTestError::InvalidLimit {
            member: "max_value_bits",
        })?,
        None => 0,
    };
    let previous = system
        .max_value_bits
        .swap(value, std::sync::atomic::Ordering::Relaxed);
    Ok(ValueBitsGuard { system, previous })
}

fn size_domain_count(bits: u64) -> Integer {
    Integer::one()
        .shifted_left(bits.saturating_add(1))
        .sub(&Integer::one())
}

fn discard_counts() -> BTreeMap<ModelBasedTestDiscardReason, ModelBasedTestDiscardCount> {
    [
        ModelBasedTestDiscardReason::DomainRefusal,
        ModelBasedTestDiscardReason::PreconditionFalse,
        ModelBasedTestDiscardReason::InvariantFalse,
        ModelBasedTestDiscardReason::NoCheckPoint,
    ]
    .into_iter()
    .map(|reason| (reason, ModelBasedTestDiscardCount::default()))
    .collect()
}

fn partition_coverage(partitions: &[ModelBoundaryPartition]) -> ModelBasedPartitionCoverage {
    let mut uncovered = BTreeMap::new();
    for partition in partitions {
        let reason = match &partition.status {
            Some(ModelPartitionStatus::Covered { .. } | ModelPartitionStatus::Refused { .. }) => {
                continue;
            }
            Some(ModelPartitionStatus::Uncovered { reason, .. }) => match reason {
                ModelPartitionUncoveredReason::PreconditionFalse => "precondition-false",
                ModelPartitionUncoveredReason::Unreachable => "unreachable",
                ModelPartitionUncoveredReason::UnreachableWithinBound => "unreachable-within-bound",
                ModelPartitionUncoveredReason::NotGenerated => "not-generated",
                ModelPartitionUncoveredReason::NoCheckPoint => "no-check-point",
                ModelPartitionUncoveredReason::Undetermined { .. } => "undetermined",
            },
            None => "not-generated",
        };
        *uncovered.entry(reason.to_owned()).or_insert(0) += 1;
    }
    ModelBasedPartitionCoverage {
        status: if uncovered.is_empty() {
            ModelOperationCoverageSummaryStatus::Complete
        } else {
            ModelOperationCoverageSummaryStatus::Partial
        },
        uncovered,
    }
}

fn prepare_text_roots(
    roots: impl IntoIterator<Item = ValueType>,
    meter: &mut Meter,
) -> Result<(), filament_exact::Incomplete> {
    use filament_exact::{Charge, ChargePoint, TextProfile};
    for root in roots {
        if let ValueType::Text(text) = root {
            let states = match text.profile() {
                TextProfile::UnicodeScalars | TextProfile::BinaryUtf8 => 1_u64,
                TextProfile::Nfd | TextProfile::Nfkd => 256,
                TextProfile::Nfc | TextProfile::Nfkc => 512,
            };
            let cells = Integer::from(text.max())
                .add(&Integer::one())
                .mul(&Integer::from(states));
            meter.charge(Charge::new(ChargePoint::TextNormalizeInput).work(cells))?;
            // Boundary strings are built before seeded draws. Fund the longest
            // one before allocating any of them, including the refusal value.
            meter.charge(
                Charge::new(ChargePoint::TextNormalizeOutput)
                    .work(Integer::zero())
                    .exact_size(
                        filament_exact::LimitKind::NormalizedScalars,
                        Integer::from(text.max().checked_add(1).unwrap_or(text.max())),
                    ),
            )?;
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the existing internal run inputs"
)]
fn text_limit_record(
    model: Option<&ModelSystem<'_>>,
    clause: String,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    max_value_bits: Option<u64>,
    stop: filament_exact::Incomplete,
) -> ModelBasedTestResult {
    let operations = model.map(|system| {
        system
            .operations()
            .into_iter()
            .map(|(_, name, identity)| ModelOperationCoverageEntry {
                operation: ModelOperation {
                    identity: identity.to_owned(),
                    name: name.to_owned(),
                },
                status: super::ModelOperationCoverageStatus::NotCovered {
                    reason: ModelOperationCoverageReason::Undetermined {
                        limit: ModelOperationCoverageLimit::Accounting(stop.limit_kind),
                        value: u128::from(stop.limit),
                    },
                },
            })
            .collect::<Vec<_>>()
    });
    let coverage = operations.as_ref().map(|entries| ModelBasedCoverage {
        status: if entries.is_empty() {
            ModelOperationCoverageSummaryStatus::Complete
        } else {
            ModelOperationCoverageSummaryStatus::Partial
        },
        uncovered: if entries.is_empty() {
            BTreeMap::new()
        } else {
            BTreeMap::from([(
                ModelOperationCoverageReasonKind::Undetermined,
                entries.len(),
            )])
        },
        limits: ModelOperationCoverageLimits {
            undetermined: entries.len(),
        },
    });
    ModelBasedTestResult {
        clause,
        profile: sampler.clone(),
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        counts: ModelBasedTestCounts::default(),
        discarded_by: discard_counts(),
        checks: ModelBasedTestChecks::default(),
        partition_coverage: partition_coverage(&[]),
        partitions: Vec::new(),
        provenance: ModelBasedProvenance {
            max_value_bits,
            size_bounded: model
                .map(|system| model_size_bounded_roots(system, max_value_bits))
                .unwrap_or_default(),
            value_bound_cuts: ModelBasedValueBoundCuts::default(),
            cases: Vec::new(),
        },
        operations,
        coverage,
        result: ModelBasedTestEvidence::Incomplete,
        evidence: ModelBasedTestEvidence::Incomplete,
        cause: Some(ModelBasedTestCause::LimitReached {
            limit: stop.limit_kind,
            value: stop.limit,
            setting: format!("accounting.{}", stop.limit_kind.as_str()),
        }),
        counterexample: None,
    }
}

fn model_size_bounded_roots(
    system: &ModelSystem<'_>,
    max_value_bits: Option<u64>,
) -> Vec<ModelBasedSizeBoundedRoot> {
    let Some(bits) = max_value_bits else {
        return Vec::new();
    };
    let previous = system
        .max_value_bits
        .swap(0, std::sync::atomic::Ordering::Relaxed);
    let roots: Vec<_> = system
        .rooted_domains()
        .into_iter()
        .filter(|(_, _, value_type)| matches!(value_type, ValueType::Integer))
        .map(|(node, path, _)| ModelBasedSizeBoundedRoot {
            node,
            path,
            values: size_domain_count(bits).to_string(),
        })
        .collect();
    system
        .max_value_bits
        .store(previous, std::sync::atomic::Ordering::Relaxed);
    roots
}

#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
fn run_model_based_value_function(
    system: &ModelSystem<'_>,
    package: &fsl_package::CheckedPackage,
    identity: &NodeKey,
    function: &CallableFunction<'_>,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    max_value_bits: Option<u64>,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    let name = package
        .graph()
        .function_by_identity(identity.clone())
        .map(|function| function.name.to_owned())
        .ok_or_else(|| ModelBasedTestError::InvalidClause {
            clause: format!("{identity:?}"),
        })?;
    let qualified_name = QualifiedName::unqualified(name.clone()).map_err(|_| {
        ModelBasedTestError::InvalidClause {
            clause: name.clone(),
        }
    })?;
    let mut domains = Vec::with_capacity(function.parameters.len());
    let mut generation_meter = Meter::new(system.meter_limits());
    if let Err(stop) = prepare_text_roots(
        function.parameters.iter().map(|(_, ty)| ty.clone()),
        &mut generation_meter,
    ) {
        return Ok(text_limit_record(
            None,
            name,
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
            stop,
        ));
    }
    let mut partitions = Vec::new();
    for (index, (parameter, value_type)) in function.parameters.iter().enumerate() {
        let domain = system.value_domain(value_type, &[]).ok_or_else(|| {
            ModelBasedTestError::ValueDomain {
                parameter: parameter.clone(),
            }
        })?;
        if domain.len() == 0 {
            return Err(ModelBasedTestError::ValueDomain {
                parameter: parameter.clone(),
            });
        }
        if let ValueType::Int(interval) = value_type {
            let path =
                vec![
                    u32::try_from(index).map_err(|_| ModelBasedTestError::ValueDomain {
                        parameter: parameter.clone(),
                    })?,
                ];
            let node = identity.clone();
            let one = Integer::one();
            for (kind, value) in [
                (
                    super::ModelBoundaryKind::LowerRefusal,
                    interval.lower().sub(&one),
                ),
                (
                    super::ModelBoundaryKind::LowerEndpoint,
                    interval.lower().clone(),
                ),
                (
                    super::ModelBoundaryKind::UpperEndpoint,
                    interval.upper().clone(),
                ),
                (
                    super::ModelBoundaryKind::UpperRefusal,
                    interval.upper().add(&one),
                ),
            ] {
                partitions.push((
                    index,
                    parameter.clone(),
                    value_type.clone(),
                    ModelBoundaryPartition {
                        node: node.clone(),
                        path: path.clone(),
                        domain: interval.clone(),
                        kind,
                        value,
                        text_value: None,
                        text_profile: None,
                        status: None,
                    },
                ));
            }
        } else if matches!(value_type, ValueType::Integer) {
            let bits = max_value_bits.ok_or(ModelBasedTestError::MissingValueBits)?;
            let magnitude = Integer::one().shifted_left(bits).sub(&Integer::one());
            let lower = magnitude.neg();
            let interval =
                IntegerInterval::new(lower.clone(), magnitude.clone()).map_err(|_| {
                    ModelBasedTestError::ValueDomain {
                        parameter: parameter.clone(),
                    }
                })?;
            let path =
                vec![
                    u32::try_from(index).map_err(|_| ModelBasedTestError::ValueDomain {
                        parameter: parameter.clone(),
                    })?,
                ];
            for (kind, value) in [
                (super::ModelBoundaryKind::SizeLeast, Integer::zero()),
                (super::ModelBoundaryKind::SizeUnit, Integer::from(-1_i64)),
                (super::ModelBoundaryKind::SizeUnit, Integer::one()),
                (super::ModelBoundaryKind::SizeExtremeLow, lower),
                (super::ModelBoundaryKind::SizeExtremeHigh, magnitude),
            ] {
                partitions.push((
                    index,
                    parameter.clone(),
                    value_type.clone(),
                    ModelBoundaryPartition {
                        node: identity.clone(),
                        path: path.clone(),
                        domain: interval.clone(),
                        kind,
                        value,
                        text_value: None,
                        text_profile: None,
                        status: None,
                    },
                ));
            }
        } else if let ValueType::Text(text_type) = value_type {
            let domain = IntegerInterval::new(
                Integer::from(text_type.min()),
                Integer::from(text_type.max()),
            )
            .map_err(|_| ModelBasedTestError::ValueDomain {
                parameter: parameter.clone(),
            })?;
            for (kind, length, text) in super::text_boundary_specs(text_type) {
                partitions.push((
                    index,
                    parameter.clone(),
                    value_type.clone(),
                    ModelBoundaryPartition {
                        node: identity.clone(),
                        path: vec![u32::try_from(index).map_err(|_| {
                            ModelBasedTestError::ValueDomain {
                                parameter: parameter.clone(),
                            }
                        })?],
                        domain: domain.clone(),
                        kind,
                        value: Integer::from(length),
                        text_value: Some(text),
                        text_profile: Some(text_type.profile()),
                        status: None,
                    },
                ));
            }
        }
        domains.push(domain);
    }

    let mut counts = ModelBasedTestCounts::default();
    let mut discarded_by = discard_counts();
    let mut checks = ModelBasedTestChecks::default();
    let mut provenance = Vec::new();
    let mut generated_partitions: Vec<_> = partitions
        .iter()
        .map(|(_, _, _, partition)| {
            let mut partition = partition.clone();
            partition.status = Some(ModelPartitionStatus::Uncovered {
                reason: ModelPartitionUncoveredReason::NotGenerated,
                bounds: Vec::new(),
            });
            partition
        })
        .collect();
    let mut counterexample = None;
    let mut case_index = 0_u64;

    for (generated_partition, (parameter_index, parameter, value_type, partition)) in
        generated_partitions.iter_mut().zip(&partitions)
    {
        if case_index >= max_cases {
            break;
        }
        let refusal = matches!(
            partition.kind,
            super::ModelBoundaryKind::LowerRefusal | super::ModelBoundaryKind::UpperRefusal
        );
        let mut assignment = domains
            .iter()
            .map(|domain| {
                domain
                    .nth(0)
                    .ok_or_else(|| ModelBasedTestError::ValueDomain {
                        parameter: parameter.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let partition_value = if refusal {
            Some(partition_refusal_value(system, partition).ok_or_else(|| {
                ModelBasedTestError::ValueDomain {
                    parameter: parameter.clone(),
                }
            })?)
        } else {
            Some(partition_native_value(partition, system).ok_or_else(|| {
                ModelBasedTestError::ValueDomain {
                    parameter: parameter.clone(),
                }
            })?)
        };
        if let Some(value) = &partition_value {
            assignment[*parameter_index] = value.clone();
        }
        counts.generated = counts.generated.saturating_add(1);
        provenance.push(ModelBasedCaseProvenance {
            sample: None,
            assignment: partition_value.as_ref().map(|_| assignment.clone()),
            max_value_bits,
        });

        let admitted = partition_value
            .as_ref()
            .is_some_and(|value| value_type.admits(value));
        match partition.kind {
            super::ModelBoundaryKind::LowerRefusal | super::ModelBoundaryKind::UpperRefusal => {
                if admitted {
                    return Err(ModelBasedTestError::OracleDisagreement {
                        parameter: parameter.clone(),
                    });
                }
                record_discard(
                    &mut counts,
                    &mut discarded_by,
                    ModelBasedTestDiscardReason::DomainRefusal,
                    case_index,
                );
                generated_partition.status = Some(ModelPartitionStatus::Refused {
                    first_case: case_index,
                });
            }
            super::ModelBoundaryKind::LowerEndpoint
            | super::ModelBoundaryKind::UpperEndpoint
            | super::ModelBoundaryKind::SizeLeast
            | super::ModelBoundaryKind::SizeUnit
            | super::ModelBoundaryKind::SizeExtremeLow
            | super::ModelBoundaryKind::SizeExtremeHigh
            | super::ModelBoundaryKind::TextEmpty
            | super::ModelBoundaryKind::TextLowerEndpoint
            | super::ModelBoundaryKind::TextSingleton
            | super::ModelBoundaryKind::TextUpperEndpoint
            | super::ModelBoundaryKind::TextNormalization => {
                if !admitted {
                    return Err(ModelBasedTestError::OracleDisagreement {
                        parameter: parameter.clone(),
                    });
                }
                let outcome = value_function_result(package, &qualified_name, &assignment, system)?;
                checks.evaluated += 1;
                generated_partition.status = Some(ModelPartitionStatus::Covered {
                    first_case: case_index,
                });
                match outcome {
                    CaseResult::Pass => counts.passing = counts.passing.saturating_add(1),
                    CaseResult::Discard(reason) => {
                        record_discard(&mut counts, &mut discarded_by, reason, case_index)
                    }
                    CaseResult::Fail(_) => {
                        counts.failing = counts.failing.saturating_add(1);
                        counterexample = Some(ModelBasedCounterexample::ValueAssignment(
                            shrink_value_assignment(
                                package,
                                &qualified_name,
                                system,
                                &domains,
                                &assignment,
                                case_index,
                                seed,
                            )?,
                        ));
                        case_index = case_index.saturating_add(1);
                        break;
                    }
                }
            }
        }
        case_index = case_index.saturating_add(1);
    }

    while case_index < max_cases && counterexample.is_none() {
        let mut assignment = Vec::with_capacity(domains.len());
        for (parameter_index, ((parameter, _), domain)) in
            function.parameters.iter().zip(&domains).enumerate()
        {
            let count = domain
                .exact_len()
                .ok_or_else(|| ModelBasedTestError::ValueDomain {
                    parameter: parameter.clone(),
                })?;
            let (index, _) = crate::simulation::choice::choose_integer(
                ChoicePoint {
                    seed,
                    trace: case_index,
                    step: 0,
                    choice: u32::try_from(parameter_index).unwrap_or(u32::MAX),
                },
                &count,
            )
            .ok_or_else(|| ModelBasedTestError::ValueDomain {
                parameter: parameter.clone(),
            })?;
            assignment.push(domain.nth_exact(&index).ok_or_else(|| {
                ModelBasedTestError::ValueDomain {
                    parameter: parameter.clone(),
                }
            })?);
        }
        counts.generated = counts.generated.saturating_add(1);
        provenance.push(ModelBasedCaseProvenance {
            sample: None,
            assignment: Some(assignment.clone()),
            max_value_bits,
        });
        match value_function_result(package, &qualified_name, &assignment, system)? {
            CaseResult::Pass => counts.passing = counts.passing.saturating_add(1),
            CaseResult::Discard(reason) => {
                record_discard(&mut counts, &mut discarded_by, reason, case_index)
            }
            CaseResult::Fail(_) => {
                counts.failing = counts.failing.saturating_add(1);
                counterexample = Some(ModelBasedCounterexample::ValueAssignment(
                    shrink_value_assignment(
                        package,
                        &qualified_name,
                        system,
                        &domains,
                        &assignment,
                        case_index,
                        seed,
                    )?,
                ));
            }
        }
        checks.evaluated += 1;
        case_index = case_index.saturating_add(1);
    }

    let evidence = if counterexample.is_some() {
        ModelBasedTestEvidence::Refuted
    } else if counts.passing == 0 {
        ModelBasedTestEvidence::Inconclusive
    } else {
        ModelBasedTestEvidence::Tested
    };
    let cause = (evidence == ModelBasedTestEvidence::Inconclusive)
        .then_some(ModelBasedTestCause::NoEvaluatedCase);
    let size_bounded = function
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(index, (_, value_type))| {
            if !matches!(value_type, ValueType::Integer) {
                return None;
            }
            Some(ModelBasedSizeBoundedRoot {
                node: identity.clone(),
                path: vec![u32::try_from(index).ok()?],
                values: size_domain_count(max_value_bits?).to_string(),
            })
        })
        .collect();
    Ok(ModelBasedTestResult {
        clause: name,
        profile: sampler.clone(),
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        counts,
        discarded_by,
        checks,
        partition_coverage: partition_coverage(&generated_partitions),
        partitions: generated_partitions,
        provenance: ModelBasedProvenance {
            max_value_bits,
            size_bounded,
            value_bound_cuts: ModelBasedValueBoundCuts::default(),
            cases: provenance,
        },
        operations: None,
        coverage: None,
        result: evidence,
        cause,
        counterexample,
        evidence,
    })
}

fn record_discard(
    counts: &mut ModelBasedTestCounts,
    discarded_by: &mut BTreeMap<ModelBasedTestDiscardReason, ModelBasedTestDiscardCount>,
    reason: ModelBasedTestDiscardReason,
    case_index: u64,
) {
    counts.discarded = counts.discarded.saturating_add(1);
    let entry = discarded_by.entry(reason).or_default();
    entry.count = entry.count.saturating_add(1);
    entry.first_case.get_or_insert(case_index);
}

fn value_function_result(
    package: &fsl_package::CheckedPackage,
    function: &QualifiedName,
    assignment: &[Value],
    system: &ModelSystem<'_>,
) -> Result<CaseResult, ModelBasedTestError> {
    let evaluation = package.call(
        function,
        assignment.to_vec(),
        &ObjectEnvironment::default(),
        &mut Meter::new(system.meter_limits()),
    )?;
    Ok(match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))) => CaseResult::Pass,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {
            CaseResult::Fail("value-false".to_owned())
        }
        FamilyOutcome::Evaluated(Outcome::Undefined(_))
        | FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(_)) => {
            CaseResult::Fail("undefined-evaluation".to_owned())
        }
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
            return Err(ModelBasedTestError::ClauseUndetermined {
                clause: function.to_string(),
                outcome: format!("{incomplete:?}"),
            });
        }
        outcome => {
            return Err(ModelBasedTestError::ClauseUndetermined {
                clause: function.to_string(),
                outcome: format!("{outcome:?}"),
            });
        }
    })
}

fn shrink_value_assignment(
    package: &fsl_package::CheckedPackage,
    function: &QualifiedName,
    system: &ModelSystem<'_>,
    domains: &[Domain],
    assignment: &[Value],
    _case_index: u64,
    _seed: u64,
) -> Result<Vec<Value>, ModelBasedTestError> {
    let CaseResult::Fail(disposition) =
        value_function_result(package, function, assignment, system)?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    let mut minimized = assignment.to_vec();
    for parameter in 0..domains.len() {
        while let Some(current) = domains[parameter].index_of(&minimized[parameter]) {
            if current == 0 {
                break;
            }
            let candidate = domains[parameter].nth(current - 1).ok_or_else(|| {
                ModelBasedTestError::ValueDomain {
                    parameter: parameter.to_string(),
                }
            })?;
            let mut reduced = minimized.clone();
            reduced[parameter] = candidate;
            match value_function_result(package, function, &reduced, system)? {
                CaseResult::Fail(candidate_disposition) if candidate_disposition == disposition => {
                    minimized = reduced
                }
                CaseResult::Fail(_) => break,
                CaseResult::Pass | CaseResult::Discard(_) => break,
            }
        }
    }
    Ok(minimized)
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
    max_value_bits: Option<u64>,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    system.reset_value_bound_cuts();
    run_model_based_with_oracle(
        system,
        clause.clause().name(),
        sampler,
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        position_limit,
        reachability_limits,
        None,
        None,
        |trace| temporal_result(system, clause, trace),
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "these are the model-based request inputs"
)]
fn run_model_based_contract_clause(
    system: &ModelSystem<'_>,
    clause: &ContractClause,
    sampler: &DefinitionRef,
    seed: u64,
    max_cases: u64,
    max_trace_length: usize,
    max_value_bits: Option<u64>,
    position_limit: u64,
    reachability_limits: Limits,
) -> Result<ModelBasedTestResult, ModelBasedTestError> {
    let identity = clause.identity.clone();
    system.reset_value_bound_cuts();
    if let Err(stop) = prepare_text_roots(
        system.rooted_domains().into_iter().map(|(_, _, ty)| ty),
        &mut Meter::new(system.meter_limits()),
    ) {
        return Ok(text_limit_record(
            Some(system),
            clause.name.clone(),
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
            stop,
        ));
    }
    let partitions = contract_partition_cases(
        system,
        clause,
        &identity,
        max_trace_length,
        reachability_limits,
    )?;
    run_model_based_with_oracle(
        system,
        &clause.name,
        sampler,
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        position_limit,
        reachability_limits,
        Some(identity.clone()),
        Some(partitions),
        |trace| contract_result(system, clause, &identity, trace),
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
    max_value_bits: Option<u64>,
    position_limit: u64,
    reachability_limits: Limits,
    omitted_clause: Option<NodeKey>,
    partition_cases: Option<Vec<PartitionRunCase>>,
    mut oracle: F,
) -> Result<ModelBasedTestResult, ModelBasedTestError>
where
    F: FnMut(&ModelTrace) -> Result<CaseResult, ModelBasedTestError>,
{
    if let Err(stop) = prepare_text_roots(
        system.rooted_domains().into_iter().map(|(_, _, ty)| ty),
        &mut Meter::new(system.meter_limits()),
    ) {
        return Ok(text_limit_record(
            Some(system),
            clause.to_owned(),
            sampler,
            seed,
            max_cases,
            max_trace_length,
            max_value_bits,
            stop,
        ));
    }
    let mut generated_cases = Vec::new();
    let mut partitions = Vec::new();
    let mut provenance = Vec::new();
    let mut counts = ModelBasedTestCounts::default();
    let mut checks = ModelBasedTestChecks::default();
    let mut discarded_by = discard_counts();
    let mut counterexample = None;

    let mut case_index = 0_u64;
    for mut partition_case in partition_cases.unwrap_or_default() {
        if partition_case.trace.is_none() && partition_case.discard.is_none() {
            partitions.push(partition_case.partition);
            continue;
        }
        if case_index >= max_cases {
            partition_case.partition.status = Some(ModelPartitionStatus::Uncovered {
                reason: ModelPartitionUncoveredReason::NotGenerated,
                bounds: Vec::new(),
            });
            partitions.push(partition_case.partition);
            continue;
        }
        let current_case = case_index;
        case_index = case_index.saturating_add(1);
        counts.generated = counts.generated.saturating_add(1);
        provenance.push(ModelBasedCaseProvenance {
            sample: None,
            assignment: None,
            max_value_bits,
        });
        if let Some(reason) = partition_case.discard {
            if reason == ModelBasedTestDiscardReason::DomainRefusal {
                partition_case.partition.status = Some(ModelPartitionStatus::Refused {
                    first_case: current_case,
                });
            }
            record_discard(&mut counts, &mut discarded_by, reason, current_case);
            partitions.push(partition_case.partition);
            continue;
        }
        if let Some(trace) = partition_case.trace {
            add_trace_checks(system, clause, &trace, omitted_clause.as_ref(), &mut checks)?;
            let result = oracle(&trace)?;
            partition_case.partition.status = match result {
                CaseResult::Pass | CaseResult::Fail(_) => Some(ModelPartitionStatus::Covered {
                    first_case: current_case,
                }),
                CaseResult::Discard(ModelBasedTestDiscardReason::PreconditionFalse) => {
                    Some(ModelPartitionStatus::Uncovered {
                        reason: ModelPartitionUncoveredReason::PreconditionFalse,
                        bounds: Vec::new(),
                    })
                }
                CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint) => {
                    Some(ModelPartitionStatus::Uncovered {
                        reason: ModelPartitionUncoveredReason::NoCheckPoint,
                        bounds: Vec::new(),
                    })
                }
                CaseResult::Discard(_) => partition_case.partition.status.clone(),
            };
            match result {
                CaseResult::Pass => {
                    generated_cases.push(case_for(&trace, current_case));
                    counts.passing = counts.passing.saturating_add(1);
                }
                CaseResult::Discard(reason) => {
                    record_discard(&mut counts, &mut discarded_by, reason, current_case);
                }
                CaseResult::Fail(disposition) => {
                    generated_cases.push(case_for(&trace, current_case));
                    counts.failing = counts.failing.saturating_add(1);
                    let mut error = None;
                    let minimized = shrink_model_trace_with_omitted_clause(
                        system,
                        &trace,
                        omitted_clause.clone(),
                        |candidate| match oracle(candidate) {
                            Ok(CaseResult::Fail(candidate_disposition)) => {
                                candidate_disposition == disposition
                            }
                            Ok(CaseResult::Pass | CaseResult::Discard(_)) => false,
                            Err(run_error) => {
                                error = Some(run_error);
                                false
                            }
                        },
                    )?;
                    if let Some(run_error) = error {
                        return Err(run_error);
                    }
                    counterexample = Some(ModelBasedCounterexample::Trace(Box::new(minimized)));
                }
            }
            partitions.push(partition_case.partition);
        } else {
            partitions.push(partition_case.partition);
        }
    }

    while case_index < max_cases && counterexample.is_none() {
        let current_case = case_index;
        case_index = case_index.saturating_add(1);
        counts.generated = counts.generated.saturating_add(1);
        let trace = sample_model_with_omitted_clause(
            system,
            position_limit,
            sampler,
            seed,
            current_case,
            max_trace_length,
            omitted_clause.clone(),
        )?;
        if let Some(sample) = &trace.trace.provenance {
            provenance.push(ModelBasedCaseProvenance {
                sample: Some(sample.clone()),
                assignment: None,
                max_value_bits,
            });
        }
        for partition in &trace.boundary_partitions {
            if !partitions.iter().any(|existing: &ModelBoundaryPartition| {
                existing.node == partition.node
                    && existing.path == partition.path
                    && existing.kind == partition.kind
                    && existing.value == partition.value
                    && existing.text_value == partition.text_value
            }) {
                partitions.push(partition.clone());
            }
        }
        add_trace_checks(system, clause, &trace, omitted_clause.as_ref(), &mut checks)?;
        match oracle(&trace)? {
            CaseResult::Pass => {
                generated_cases.push(case_for(&trace, current_case));
                counts.passing = counts.passing.saturating_add(1);
            }
            CaseResult::Discard(reason) => {
                counts.discarded = counts.discarded.saturating_add(1);
                let entry = discarded_by.entry(reason).or_default();
                entry.count = entry.count.saturating_add(1);
                entry.first_case.get_or_insert(current_case);
            }
            CaseResult::Fail(disposition) => {
                generated_cases.push(case_for(&trace, current_case));
                counts.failing = counts.failing.saturating_add(1);
                let mut error = None;
                let minimized = shrink_model_trace_with_omitted_clause(
                    system,
                    &trace,
                    omitted_clause.clone(),
                    |candidate| match oracle(candidate) {
                        Ok(CaseResult::Fail(candidate_disposition)) => {
                            candidate_disposition == disposition
                        }
                        Ok(CaseResult::Pass | CaseResult::Discard(_)) => false,
                        Err(run_error) => {
                            error = Some(run_error);
                            false
                        }
                    },
                )?;
                if let Some(run_error) = error {
                    return Err(run_error);
                }
                counterexample = Some(ModelBasedCounterexample::Trace(Box::new(minimized)));
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
    let evidence = if counterexample.is_some() {
        ModelBasedTestEvidence::Refuted
    } else if counts.passing == 0 {
        ModelBasedTestEvidence::Inconclusive
    } else {
        ModelBasedTestEvidence::Tested
    };
    let cause = (evidence == ModelBasedTestEvidence::Inconclusive)
        .then_some(ModelBasedTestCause::NoEvaluatedCase);
    let value_bound_cuts = system.value_bound_cuts();

    Ok(ModelBasedTestResult {
        clause: clause.to_owned(),
        profile: sampler.clone(),
        seed,
        max_cases,
        max_trace_length,
        max_value_bits,
        counts,
        discarded_by,
        checks,
        partition_coverage: partition_coverage(&partitions),
        partitions,
        provenance: ModelBasedProvenance {
            max_value_bits,
            size_bounded: model_size_bounded_roots(system, max_value_bits),
            value_bound_cuts: ModelBasedValueBoundCuts {
                count: u64::try_from(value_bound_cuts.len()).unwrap_or(u64::MAX),
                entries: value_bound_cuts,
            },
            cases: provenance,
        },
        operations: Some(operation_coverage.operations.clone()),
        coverage: Some(ModelBasedCoverage {
            status: operation_coverage.status,
            uncovered: operation_coverage.uncovered,
            limits: operation_coverage.limits,
        }),
        evidence,
        result: evidence,
        cause,
        counterexample,
    })
}

fn add_trace_checks(
    system: &ModelSystem<'_>,
    clause: &str,
    trace: &ModelTrace,
    identity: Option<&NodeKey>,
    checks: &mut ModelBasedTestChecks,
) -> Result<(), ModelBasedTestError> {
    let Some(identity) = identity else {
        let invariant = system
            .source_package()
            .and_then(|package| package.graph().state_clause(clause))
            .is_some_and(|clause| clause.is_invariant());
        checks.evaluated += if invariant {
            u64::try_from(trace.trace.steps.len())
                .unwrap_or(u64::MAX)
                .saturating_add(1)
        } else {
            1
        };
        return Ok(());
    };
    let contract = system
        .contract_clauses()
        .into_iter()
        .find(|contract| contract.identity == *identity)
        .ok_or_else(|| ModelBasedTestError::InvalidClause {
            clause: clause.to_owned(),
        })?;
    let Some((_, states)) = super::rebuild_model_trace(
        system,
        &trace.trace.initial,
        &trace.trace.steps,
        trace.trace.provenance.as_ref(),
        Some(identity),
    )?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    let Some((declaring, name)) = &contract.operation else {
        return Err(ModelBasedTestError::InvalidClause {
            clause: clause.to_owned(),
        });
    };
    let operation = system
        .operation_text(declaring.clone(), name)
        .ok_or_else(|| ModelBasedTestError::InvalidClause {
            clause: clause.to_owned(),
        })?;
    for (index, state) in states.iter().enumerate() {
        let points = match contract.kind {
            ContractClauseKind::Precondition => system
                .precondition_cases(state, identity.clone())
                .map_err(ModelBasedTestError::Expansion)?
                .len(),
            ContractClauseKind::Postcondition => usize::from(
                trace
                    .trace
                    .steps
                    .get(index)
                    .is_some_and(|step| step.transition.operation() == operation),
            ),
            ContractClauseKind::Invariant => 1,
        };
        let points = u64::try_from(points).unwrap_or(u64::MAX);
        if points == 0 {
            continue;
        }
        if has_false_invariant(system, state)? {
            *checks
                .skipped
                .entry(ModelBasedTestDiscardReason::InvariantFalse)
                .or_default() += points;
        } else {
            checks.evaluated += points;
        }
    }
    Ok(())
}

fn contract_result(
    system: &ModelSystem<'_>,
    clause: &ContractClause,
    identity: &NodeKey,
    trace: &ModelTrace,
) -> Result<CaseResult, ModelBasedTestError> {
    let Some((_, states)) = super::rebuild_model_trace(
        system,
        &trace.trace.initial,
        &trace.trace.steps,
        trace.trace.provenance.as_ref(),
        Some(identity),
    )?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    let (declaring, operation) =
        clause
            .operation
            .as_ref()
            .ok_or_else(|| ModelBasedTestError::InvalidClause {
                clause: clause.name.clone(),
            })?;
    let operation_text = system
        .operation_text(declaring.clone(), operation)
        .ok_or_else(|| ModelBasedTestError::InvalidClause {
            clause: clause.name.clone(),
        })?;
    if clause.kind == ContractClauseKind::Precondition {
        let mut evaluated = false;
        let mut skipped = false;
        let mut undefined = false;
        for state in &states {
            let cases = system
                .precondition_cases(state, identity.clone())
                .map_err(ModelBasedTestError::Expansion)?;
            if has_false_invariant(system, state)? {
                skipped |= !cases.is_empty();
                continue;
            }
            for case in cases {
                evaluated = true;
                if case.undecided.is_some() {
                    undefined = true;
                }
            }
        }
        return Ok(if undefined {
            CaseResult::Fail("undefined-precondition".to_owned())
        } else if evaluated {
            CaseResult::Pass
        } else {
            CaseResult::Discard(if skipped {
                ModelBasedTestDiscardReason::InvariantFalse
            } else {
                ModelBasedTestDiscardReason::NoCheckPoint
            })
        });
    }
    if let Some(attempted) = &trace.attempted {
        if attempted.operation() != operation_text {
            return Ok(match clause.kind {
                ContractClauseKind::Postcondition
                    if attempted_precondition_satisfied(system, &states, attempted)?
                        == Some(false) =>
                {
                    CaseResult::Discard(ModelBasedTestDiscardReason::PreconditionFalse)
                }
                _ => CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint),
            });
        }
        return match clause.kind {
            ContractClauseKind::Precondition => {
                let Some(state) = states.first() else {
                    return Ok(CaseResult::Discard(
                        ModelBasedTestDiscardReason::NoCheckPoint,
                    ));
                };
                let cases = system
                    .precondition_cases(state, identity.clone())
                    .map_err(ModelBasedTestError::Expansion)?;
                let Some(case) = cases.iter().find(|case| case.transition == *attempted) else {
                    return Ok(CaseResult::Discard(
                        ModelBasedTestDiscardReason::NoCheckPoint,
                    ));
                };
                Ok(if case.undecided.is_some() {
                    CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint)
                } else {
                    CaseResult::Pass
                })
            }
            ContractClauseKind::Postcondition => Ok(
                match attempted_precondition_satisfied(system, &states, attempted)? {
                    Some(false) => {
                        CaseResult::Discard(ModelBasedTestDiscardReason::PreconditionFalse)
                    }
                    _ => CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint),
                },
            ),
            ContractClauseKind::Invariant => Err(ModelBasedTestError::InvalidClause {
                clause: clause.name.clone(),
            }),
        };
    }
    let mut checked = false;
    let mut skipped = false;
    let mut failure = None;
    for (index, step) in trace.trace.steps.iter().enumerate() {
        if step.transition.operation() != operation_text {
            continue;
        }
        if has_false_invariant(system, &states[index])? {
            skipped = true;
            continue;
        }
        checked = true;
        match clause.kind {
            ContractClauseKind::Precondition => {
                let cases = system
                    .precondition_cases(&states[index], identity.clone())
                    .map_err(ModelBasedTestError::Expansion)?;
                let Some(case) = cases.iter().find(|case| case.transition == step.transition)
                else {
                    return Ok(CaseResult::Discard(
                        ModelBasedTestDiscardReason::NoCheckPoint,
                    ));
                };
                if case.undecided.is_some() {
                    return Ok(CaseResult::Discard(
                        ModelBasedTestDiscardReason::NoCheckPoint,
                    ));
                }
            }
            ContractClauseKind::Postcondition => {
                let operation_identity = (declaring.clone(), operation.clone());
                let selected: Vec<NodeKey> = system
                    .contract_clauses()
                    .into_iter()
                    .filter(|candidate| candidate.operation.as_ref() == Some(&operation_identity))
                    .map(|candidate| candidate.identity)
                    .collect();
                let cases = system
                    .operation_cases(
                        &states[index],
                        declaring.clone(),
                        operation,
                        clause.context.clone(),
                        &|candidate| selected.contains(&candidate),
                    )
                    .map_err(ModelBasedTestError::Expansion)?;
                if cases.undecided.iter().any(|undecided| {
                    undecided.transition == step.transition && undecided.clause == clause.name
                }) {
                    failure.get_or_insert_with(|| "undefined-postcondition".to_owned());
                    continue;
                }
                let post_key = system.state_key(&states[index + 1])?.1;
                let mut clause_holds = false;
                for candidate in &cases.steps {
                    if candidate.transition != step.transition {
                        continue;
                    }
                    let candidate_key = system.state_key(&candidate.post)?.1;
                    clause_holds |= candidate_key == post_key;
                }
                if !clause_holds {
                    failure.get_or_insert_with(|| "postcondition-false".to_owned());
                }
            }
            ContractClauseKind::Invariant => {
                return Err(ModelBasedTestError::InvalidClause {
                    clause: clause.name.clone(),
                });
            }
        }
    }
    Ok(if let Some(disposition) = failure {
        CaseResult::Fail(disposition)
    } else if checked {
        CaseResult::Pass
    } else if skipped {
        CaseResult::Discard(ModelBasedTestDiscardReason::InvariantFalse)
    } else {
        CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint)
    })
}

fn attempted_precondition_satisfied(
    system: &ModelSystem<'_>,
    states: &[ModelSimState],
    attempted: &filament_semantic_value::model_transition::ModelTransition,
) -> Result<Option<bool>, ModelBasedTestError> {
    let Some(state) = states.first() else {
        return Ok(None);
    };
    let Some((declaring, name, _)) = system
        .operations()
        .into_iter()
        .find(|(_, _, text)| *text == attempted.operation())
    else {
        return Ok(None);
    };
    let operation = (declaring, name.to_owned());
    for clause in system.contract_clauses().into_iter().filter(|clause| {
        clause.kind == ContractClauseKind::Precondition
            && clause.operation.as_ref() == Some(&operation)
    }) {
        let cases = system
            .precondition_cases(state, clause.identity)
            .map_err(ModelBasedTestError::Expansion)?;
        if let Some(case) = cases.iter().find(|case| case.transition == *attempted) {
            return Ok(case.undecided.is_none().then_some(case.satisfied));
        }
    }
    Ok(None)
}

fn has_false_invariant(
    system: &ModelSystem<'_>,
    state: &super::ModelSimState,
) -> Result<bool, ModelBasedTestError> {
    Ok(system
        .invariant_findings_at(state)
        .map_err(ModelBasedTestError::Expansion)?
        .iter()
        .any(|finding| matches!(finding, ModelFinding::InvariantViolated { .. })))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CaseResult {
    Pass,
    Fail(String),
    Discard(ModelBasedTestDiscardReason),
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

struct PartitionRunCase {
    partition: ModelBoundaryPartition,
    trace: Option<ModelTrace>,
    discard: Option<ModelBasedTestDiscardReason>,
}

struct PartitionPath {
    state: ModelSimState,
    initial: crate::simulation::StateKey,
    steps: Vec<Step<filament_semantic_value::model_transition::ModelTransition>>,
    findings: Vec<crate::simulation::expansion::StateFindings<ModelFinding>>,
    depth: usize,
}

fn contract_partition_cases(
    system: &ModelSystem<'_>,
    clause: &ContractClause,
    identity: &NodeKey,
    max_trace_length: usize,
    limits: Limits,
) -> Result<Vec<PartitionRunCase>, ModelBasedTestError> {
    let mut cases = Vec::new();
    for mut partition in system.boundary_partitions() {
        let refusal = matches!(
            partition.kind,
            super::ModelBoundaryKind::LowerRefusal | super::ModelBoundaryKind::UpperRefusal
        );
        if refusal {
            let parameter = partition_parameter_name(system, &partition)
                .unwrap_or_else(|| format!("{:?}", partition.node));
            let admitted = partition_refusal_value(system, &partition)
                .zip(partition_value_type(system, &partition))
                .is_some_and(|(value, value_type)| value_type.admits(&value));
            require_refusal(admitted, parameter)?;
            cases.push(PartitionRunCase {
                partition,
                trace: None,
                discard: Some(ModelBasedTestDiscardReason::DomainRefusal),
            });
            continue;
        }
        match partition.path.first().copied() {
            Some(0) => {
                let Some(field) = partition_field_key(system, &partition) else {
                    partition.status = Some(ModelPartitionStatus::Uncovered {
                        reason: ModelPartitionUncoveredReason::NotGenerated,
                        bounds: Vec::new(),
                    });
                    cases.push(PartitionRunCase {
                        partition,
                        trace: None,
                        discard: None,
                    });
                    continue;
                };
                match shortest_partition_path(
                    system,
                    identity,
                    &partition,
                    max_trace_length,
                    limits,
                    |state| state_has_field_value(system, state, &field, &partition),
                    None,
                )? {
                    PathSearch::Found(trace) => cases.push(PartitionRunCase {
                        partition,
                        trace: Some(trace),
                        discard: None,
                    }),
                    PathSearch::Attempted(_) => unreachable!("field search does not attempt"),
                    PathSearch::Unreachable { bounds } => {
                        partition.status = Some(ModelPartitionStatus::Uncovered {
                            reason: if bounds.is_empty() {
                                ModelPartitionUncoveredReason::Unreachable
                            } else {
                                ModelPartitionUncoveredReason::UnreachableWithinBound
                            },
                            bounds,
                        });
                        cases.push(PartitionRunCase {
                            partition,
                            trace: None,
                            discard: None,
                        });
                    }
                    PathSearch::Undetermined { limit, value } => {
                        partition.status = Some(ModelPartitionStatus::Uncovered {
                            reason: ModelPartitionUncoveredReason::Undetermined { limit, value },
                            bounds: Vec::new(),
                        });
                        cases.push(PartitionRunCase {
                            partition,
                            trace: None,
                            discard: None,
                        });
                    }
                }
            }
            Some(1) => {
                let Some((operation, parameter)) = partition_operation(system, &partition) else {
                    partition.status = Some(ModelPartitionStatus::Uncovered {
                        reason: ModelPartitionUncoveredReason::NotGenerated,
                        bounds: Vec::new(),
                    });
                    cases.push(PartitionRunCase {
                        partition,
                        trace: None,
                        discard: None,
                    });
                    continue;
                };
                let attempt = |state: &ModelSimState| {
                    partition_attempt(system, state, identity, &operation, parameter, &partition)
                };
                let result = if clause.kind == ContractClauseKind::Precondition {
                    first_partition_attempt(system, identity, &partition, attempt)?
                } else {
                    shortest_partition_path(
                        system,
                        identity,
                        &partition,
                        max_trace_length,
                        limits,
                        |_| false,
                        Some(&attempt),
                    )?
                };
                match result {
                    PathSearch::Found(trace) => cases.push(PartitionRunCase {
                        partition,
                        trace: Some(trace),
                        discard: None,
                    }),
                    PathSearch::Attempted(trace) => cases.push(PartitionRunCase {
                        partition,
                        trace: Some(trace),
                        discard: None,
                    }),
                    PathSearch::Unreachable { .. } => {
                        partition.status = Some(ModelPartitionStatus::Uncovered {
                            reason: ModelPartitionUncoveredReason::PreconditionFalse,
                            bounds: Vec::new(),
                        });
                        cases.push(PartitionRunCase {
                            partition,
                            trace: None,
                            discard: Some(ModelBasedTestDiscardReason::PreconditionFalse),
                        });
                    }
                    PathSearch::Undetermined { limit, value } => {
                        partition.status = Some(ModelPartitionStatus::Uncovered {
                            reason: ModelPartitionUncoveredReason::Undetermined { limit, value },
                            bounds: Vec::new(),
                        });
                        cases.push(PartitionRunCase {
                            partition,
                            trace: None,
                            discard: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Ok(cases)
}

fn partition_value_type(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
) -> Option<ValueType> {
    system
        .rooted_domains()
        .into_iter()
        .find(|(node, path, _)| node == &partition.node && path == &partition.path)
        .map(|(_, _, value_type)| value_type)
}

fn partition_parameter_name(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
) -> Option<String> {
    system
        .rooted_domains()
        .into_iter()
        .find(|(node, path, _)| node == &partition.node && path == &partition.path)
        .and_then(|(_, path, _)| {
            (path.first() == Some(&1))
                .then(|| path.get(2).copied())
                .flatten()
                .map(|parameter| format!("parameter[{parameter}]"))
        })
}

fn partition_refusal_value(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
) -> Option<Value> {
    if let Some(text) = &partition.text_value {
        let profile = partition.text_profile?;
        let length = profile.length(text);
        let widened = filament_exact::TextType::new(0, length.max(1), profile).ok()?;
        return admitted_text_value(&widened, text);
    }
    let _ = system;
    Some(Value::Integer(partition.value.clone()))
}

fn require_refusal(admitted: bool, parameter: String) -> Result<(), ModelBasedTestError> {
    if admitted {
        Err(ModelBasedTestError::OracleDisagreement { parameter })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod refusal_tests {
    use super::{ModelBasedTestError, ModelBasedTestResult, require_refusal};

    /// FR-752-AC-8: if admission accepts a refusal partition, the run exits
    /// through the API error and produces no model-based result record.
    #[test]
    fn an_admitted_refusal_returns_no_test_result() {
        let result: Result<ModelBasedTestResult, ModelBasedTestError> =
            require_refusal(true, "by".to_owned()).map(|()| unreachable!("oracle disagreement"));
        assert!(matches!(
            result,
            Err(ModelBasedTestError::OracleDisagreement { parameter }) if parameter == "by"
        ));
    }
}

enum PathSearch {
    Found(ModelTrace),
    Attempted(ModelTrace),
    Unreachable { bounds: Vec<String> },
    Undetermined { limit: String, value: u128 },
}

fn first_partition_attempt(
    system: &ModelSystem<'_>,
    clause: &NodeKey,
    partition: &ModelBoundaryPartition,
    attempt: impl Fn(
        &ModelSimState,
    ) -> Option<filament_semantic_value::model_transition::ModelTransition>,
) -> Result<PathSearch, ModelBasedTestError> {
    for initial in ordered_initial_states(system)? {
        if let Some(attempted) = attempt(&initial.state) {
            return build_partition_trace(
                system,
                clause,
                initial.key,
                Vec::new(),
                Vec::new(),
                initial.state,
                Some(attempted),
                partition,
            )
            .map(PathSearch::Attempted);
        }
    }
    Ok(PathSearch::Unreachable { bounds: Vec::new() })
}

fn shortest_partition_path(
    system: &ModelSystem<'_>,
    clause: &NodeKey,
    partition: &ModelBoundaryPartition,
    max_depth: usize,
    limits: Limits,
    target: impl Fn(&ModelSimState) -> bool,
    attempt: Option<
        &dyn Fn(
            &ModelSimState,
        ) -> Option<filament_semantic_value::model_transition::ModelTransition>,
    >,
) -> Result<PathSearch, ModelBasedTestError> {
    let view = super::ModelRunView::new(system, Some(clause.clone()));
    let initial = ordered_initial_states(system)?;
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    for item in initial {
        if visited.insert(item.key.clone()) {
            queue.push_back(PartitionPath {
                state: item.state,
                initial: item.key,
                steps: Vec::new(),
                findings: Vec::new(),
                depth: 0,
            });
        }
    }
    let first_initial = queue
        .front()
        .map(|path| (path.state.clone(), path.initial.clone()));
    let mut first_attempt = None;
    let mut transitions = 0_usize;
    let mut bounded = false;
    while let Some(mut current) = queue.pop_front() {
        let expanded =
            match crate::simulation::order::expand(&view, &current.state, super::identity_limits())
            {
                Ok(Expanded::Successors {
                    successors,
                    findings,
                }) => {
                    if !findings.is_empty() {
                        current
                            .findings
                            .push(crate::simulation::expansion::StateFindings {
                                state: system.state_key(&current.state)?.1,
                                depth: current.depth,
                                findings,
                            });
                    }
                    successors
                }
                Ok(Expanded::Stopped(stop)) => {
                    return Ok(PathSearch::Undetermined {
                        limit: format!("{:?}", stop.limit),
                        value: 0,
                    });
                }
                Err(error) => return Err(ModelBasedTestError::Key(error)),
            };
        if target(&current.state) {
            return build_partition_trace(
                system,
                clause,
                current.initial,
                current.steps,
                current.findings,
                current.state,
                None,
                partition,
            )
            .map(PathSearch::Found);
        }
        if let Some(attempt) = attempt {
            if let Some(attempted) = attempt(&current.state) {
                if expanded
                    .iter()
                    .any(|successor| successor.transition == attempted)
                {
                    let Some(successor) = expanded
                        .iter()
                        .find(|successor| successor.transition == attempted)
                    else {
                        continue;
                    };
                    let mut steps = current.steps.clone();
                    let ordinal = expanded
                        .iter()
                        .take_while(|candidate| candidate.transition != attempted)
                        .filter(|candidate| candidate.transition == attempted)
                        .count();
                    steps.push(Step {
                        transition: attempted,
                        ordinal,
                        key: successor.key.clone(),
                    });
                    return build_partition_trace(
                        system,
                        clause,
                        current.initial,
                        steps,
                        current.findings,
                        successor.state.clone(),
                        None,
                        partition,
                    )
                    .map(PathSearch::Found);
                }
                if first_attempt.is_none() {
                    first_attempt = Some((
                        current.state.clone(),
                        current.initial.clone(),
                        current.steps.clone(),
                        current.findings.clone(),
                        attempted,
                    ));
                }
            }
        }
        if current.depth >= max_depth {
            bounded |= !expanded.is_empty();
            continue;
        }
        for (successor_index, successor) in expanded.iter().enumerate() {
            transitions = transitions.saturating_add(1);
            if transitions > limits.max_transitions {
                return Ok(PathSearch::Undetermined {
                    limit: "transitions".to_owned(),
                    value: transitions as u128,
                });
            }
            if visited.insert(successor.key.clone()) {
                if visited.len() > limits.max_states {
                    return Ok(PathSearch::Undetermined {
                        limit: "states".to_owned(),
                        value: visited.len() as u128,
                    });
                }
                let mut steps = current.steps.clone();
                let ordinal = expanded[..successor_index]
                    .iter()
                    .filter(|previous| previous.transition == successor.transition)
                    .count();
                steps.push(Step {
                    transition: successor.transition.clone(),
                    ordinal,
                    key: successor.key.clone(),
                });
                queue.push_back(PartitionPath {
                    state: successor.state.clone(),
                    initial: current.initial.clone(),
                    steps,
                    findings: current.findings.clone(),
                    depth: current.depth.saturating_add(1),
                });
            }
        }
    }
    if let Some((state, initial, steps, findings, attempted)) = first_attempt {
        return build_partition_trace(
            system,
            clause,
            initial,
            steps,
            findings,
            state,
            Some(attempted),
            partition,
        )
        .map(PathSearch::Attempted);
    }
    if let (Some(attempt), Some((state, initial))) = (attempt, first_initial) {
        if let Some(attempted) = attempt(&state) {
            return build_partition_trace(
                system,
                clause,
                initial,
                Vec::new(),
                Vec::new(),
                state,
                Some(attempted),
                partition,
            )
            .map(PathSearch::Attempted);
        }
    }
    let mut bounds = Vec::new();
    if bounded {
        bounds.push("max_trace_length".to_owned());
    }
    Ok(PathSearch::Unreachable { bounds })
}

fn ordered_initial_states(
    system: &ModelSystem<'_>,
) -> Result<Vec<crate::simulation::order::InitialState<ModelSimState>>, ModelBasedTestError> {
    crate::simulation::order::sorted_initial(system, super::identity_limits())
        .map_err(ModelBasedTestError::Key)
}

fn build_partition_trace(
    system: &ModelSystem<'_>,
    clause: &NodeKey,
    initial: crate::simulation::StateKey,
    steps: Vec<Step<filament_semantic_value::model_transition::ModelTransition>>,
    mut findings: Vec<crate::simulation::expansion::StateFindings<ModelFinding>>,
    final_state: ModelSimState,
    attempted: Option<filament_semantic_value::model_transition::ModelTransition>,
    partition: &ModelBoundaryPartition,
) -> Result<ModelTrace, ModelBasedTestError> {
    let view = super::ModelRunView::new(system, Some(clause.clone()));
    let Expanded::Successors {
        findings: final_findings,
        ..
    } = crate::simulation::order::expand(&view, &final_state, super::identity_limits())
        .map_err(ModelBasedTestError::Key)?
    else {
        return Err(ModelBasedTestError::ClauseUndetermined {
            clause: clause.to_string(),
            outcome: "a partition trace stopped before its final state".to_owned(),
        });
    };
    if !final_findings.is_empty() {
        findings.push(crate::simulation::expansion::StateFindings {
            state: system.state_key(&final_state)?.1,
            depth: steps.len(),
            findings: final_findings,
        });
    }
    let trace = Trace {
        initial,
        steps,
        findings,
        provenance: None,
    };
    let effects = super::effects_of_with_omitted_clause(system, &trace, Some(clause))
        .map_err(ModelBasedTestError::Sample)?;
    Ok(ModelTrace {
        trace,
        effects,
        attempted,
        boundary_partitions: vec![partition.clone()],
    })
}

fn partition_field_key(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
) -> Option<filament_checked::key::DeclarationKey> {
    let [0, wanted_ordinal] = partition.path.as_slice() else {
        return None;
    };
    for declaration in system.types.object_types() {
        let Some(type_key) = system.model.type_key(declaration.key()) else {
            continue;
        };
        let (Some(fields), Some(attributes)) = (
            system.fields.get(type_key),
            system.types.attributes(declaration.key()),
        ) else {
            continue;
        };
        for field in fields {
            let attribute = attributes
                .iter()
                .find(|attribute| attribute.field().name() == field.name)?;
            let owner = attribute.owner();
            let original = owner.original();
            let node = fsl_semantics::model::key::model_node_key(
                original.package(),
                "model",
                "object_type",
                original.node(),
            )
            .ok()?;
            let ordinal = system
                .evaluator()
                .own_field_ordinal(&owner, attribute.field().name())
                .and_then(|ordinal| u32::try_from(ordinal).ok())?;
            if node == partition.node && ordinal == *wanted_ordinal {
                return Some(field.key.clone());
            }
        }
    }
    None
}

fn state_has_field_value(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    field: &filament_checked::key::DeclarationKey,
    partition: &ModelBoundaryPartition,
) -> bool {
    state
        .state()
        .populations
        .values()
        .flat_map(|objects| objects.values())
        .filter_map(|object| object.fields.get(field))
        .any(|candidate| partition_has_value(system, partition, candidate))
}

fn partition_operation(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
) -> Option<(String, usize)> {
    let [1, wanted_operation, parameter] = partition.path.as_slice() else {
        return None;
    };
    let mut by_type: BTreeMap<NodeKey, Vec<(String, String)>> = BTreeMap::new();
    for (declaring, name, text) in system.operations() {
        let original = declaring.original();
        let node = fsl_semantics::model::key::model_node_key(
            original.package(),
            "model",
            "object_type",
            original.node(),
        )
        .ok()?;
        by_type
            .entry(node)
            .or_default()
            .push((name.to_owned(), text.to_owned()));
    }
    let operations = by_type.get_mut(&partition.node)?;
    operations.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    let (_, text) = operations.get(*wanted_operation as usize)?;
    Some((text.clone(), *parameter as usize))
}

fn partition_attempt(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    clause: &NodeKey,
    operation: &str,
    parameter: usize,
    partition: &ModelBoundaryPartition,
) -> Option<filament_semantic_value::model_transition::ModelTransition> {
    let expected = super::value_json(&partition_native_value(partition, system)?)?;
    let clauses = system.contract_clauses();
    let selected = clauses
        .iter()
        .find(|candidate| candidate.identity == *clause)?;
    let (declaring, _, _) = system
        .operations()
        .into_iter()
        .find(|(_, _, text)| *text == operation)?;
    let operation_identity = (declaring, operation.to_owned());
    let preconditions: Vec<_> = clauses
        .iter()
        .filter(|candidate| {
            candidate.kind == ContractClauseKind::Precondition
                && candidate.operation.as_ref() == Some(&operation_identity)
        })
        .collect();
    let candidates = if selected.kind == ContractClauseKind::Precondition
        && selected.operation.as_ref() == Some(&operation_identity)
    {
        system.precondition_cases(state, clause.clone()).ok()?
    } else if preconditions.is_empty() {
        return system
            .transition_identities(state)
            .ok()?
            .into_iter()
            .find(|transition| {
                transition.operation() == operation
                    && transition
                        .arguments()
                        .get(parameter)
                        .and_then(|argument| {
                            serde_json::from_str::<serde_json::Value>(argument).ok()
                        })
                        .is_some_and(|argument| argument == expected)
            });
    } else {
        let mut candidates = Vec::new();
        for precondition in preconditions {
            candidates.extend(
                system
                    .precondition_cases(state, precondition.identity.clone())
                    .ok()?,
            );
        }
        candidates
    };
    candidates.into_iter().find_map(|case| {
        (case.transition.operation() == operation
            && case
                .arguments
                .get(parameter)
                .is_some_and(|value| partition_has_value(system, partition, value)))
        .then_some(case.transition)
    })
}

fn partition_has_value(
    system: &ModelSystem<'_>,
    partition: &ModelBoundaryPartition,
    value: &Value,
) -> bool {
    partition_native_value(partition, system)
        .is_some_and(|expected| super::probabilistic::same_value(&expected, value))
}

fn partition_native_value(
    partition: &ModelBoundaryPartition,
    system: &ModelSystem<'_>,
) -> Option<Value> {
    if let Some(text) = &partition.text_value {
        let text_type = filament_exact::TextType::new(
            partition.domain.lower().to_u64()?,
            partition.domain.upper().to_u64()?,
            partition.text_profile?,
        )
        .ok()?;
        let payload = filament_exact::TextPayload::from_utf8(text.as_bytes()).ok()?;
        return match filament_exact::admit_text(
            &payload,
            &text_type,
            &mut Meter::new(system.meter_limits()),
        ) {
            Outcome::Completed(value) => Some(Value::Text(value)),
            Outcome::Undefined(_) | Outcome::Refused(_) | Outcome::Incomplete(_) => None,
        };
    }
    Some(Value::Integer(partition.value.clone()))
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
        None,
    )?
    else {
        return Err(ModelBasedTestError::Shrink(
            ModelShrinkError::NotCounterexample,
        ));
    };
    let mut failed = false;
    for state in &states {
        let findings = system
            .invariant_finding_alone(state, clause)
            .map_err(ModelBasedTestError::Expansion)?;
        for finding in findings {
            match finding {
                ModelFinding::InvariantViolated { clause: found, .. } if found == clause => {
                    failed = true;
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
    Ok(if failed {
        CaseResult::Fail("invariant-violated".to_owned())
    } else {
        CaseResult::Pass
    })
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
        None,
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
            capture_values: BTreeMap::new(),
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
                CaseResult::Discard(ModelBasedTestDiscardReason::NoCheckPoint)
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
    /// Capture values fixed at each origin or activating position.
    capture_values: BTreeMap<u64, Vec<Value>>,
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
        let capture_position = match self.node.clause().capture_site() {
            fsl_semantics::check::CaptureSite::Origin => 0,
            fsl_semantics::check::CaptureSite::Never
            | fsl_semantics::check::CaptureSite::Activations => position.index(),
        };
        let captures = self
            .capture_values
            .get(&capture_position)
            .map_or(&[][..], Vec::as_slice);
        self.letters
            .atom_with_captures(atom, &self.instance, self.state(position)?, captures, meter)
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
        let (reading, values) = self
            .letters
            .captures_with_values(&self.instance, self.state(position)?, meter)
            .map_err(crate::temporal::LetterStop::from)?;
        self.capture_values.insert(position.index(), values);
        Ok(reading)
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
            stopped = Some((
                ModelOperationCoverageLimit::States,
                u128::try_from(limits.max_states).unwrap_or(u128::MAX),
            ));
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
                    stopped = Some((
                        ModelOperationCoverageLimit::Candidates,
                        u128::from(system.limits.max_candidates),
                    ));
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
                stopped = Some((
                    ModelOperationCoverageLimit::Transitions,
                    u128::try_from(limits.max_transitions).unwrap_or(u128::MAX),
                ));
                break;
            }
            transitions = transitions.saturating_add(1);
            enabled
                .entry(transition.operation().to_owned())
                .or_insert_with(|| current.path.clone());
            if enabled.len() == operations.len() {
                queue.clear();
                break;
            }
            let (key, _) = system.state_key(&state)?;
            let depth = current.depth.saturating_add(1);
            if visited.contains_key(&key) {
                continue;
            }
            if visited.len() >= limits.max_states {
                stopped = Some((
                    ModelOperationCoverageLimit::States,
                    u128::try_from(limits.max_states).unwrap_or(u128::MAX),
                ));
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
    let size_bounded = system
        .max_value_bits
        .load(std::sync::atomic::Ordering::Relaxed)
        > 0;
    Ok(operations
        .into_iter()
        .map(|(_, _, identity)| {
            let reason = if let Some(path) = enabled.get(identity) {
                ModelOperationCoverageReason::NotGenerated {
                    enabled_at: path.clone(),
                }
            } else if let Some((limit, value)) = stopped {
                ModelOperationCoverageReason::Undetermined { limit, value }
            } else if bounded || size_bounded {
                let mut bounds = Vec::new();
                if bounded {
                    bounds.push("max_trace_length".to_owned());
                }
                if size_bounded {
                    bounds.push("max_value_bits".to_owned());
                }
                ModelOperationCoverageReason::UnreachableWithinBound { bounds }
            } else {
                ModelOperationCoverageReason::Unreachable
            };
            (identity.to_owned(), reason)
        })
        .collect())
}
