// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-190, FSL FR-752: the one declared type of the
//! `native-model-based-test-result/1` document (ADR-037).
//!
//! The record is the shape the direct run writes today, member for member;
//! the FSpec FR-190 differences are QSL-3637's. The writer builds these types
//! and the strict reader decodes the same types. A member that is always
//! written is required, a nullable member is required and may be `null`, and
//! no member is omitted.

use std::collections::BTreeMap;

use fsl_eval::simulation::model::{
    ModelBasedCoverage, ModelBasedPartitionCoverage, ModelBasedTestDiscardCount,
    ModelBasedTestDiscardReason, ModelBasedTestEvidence, ModelBoundaryKind,
    ModelOperationCoverageEntry,
};
use fsl_foundation::serde_object::required_nullable as nullable;
use fsl_foundation::wire_format::WireFormat;
use fsl_replay::spine::{
    read_route_result, FormatMarker, Label, Labelled, NativeRunLimits, NativeRunReadError,
    RouteResult,
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};

/// The marker of [`WireFormat::ModelBasedTestResult`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelBasedTestMarker;

impl FormatMarker for ModelBasedTestMarker {
    const FORMAT: WireFormat = WireFormat::ModelBasedTestResult;
}

/// The `native-model-based-test-result/1` document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBasedTestDocument {
    /// The document's catalog label.
    pub format: Label<ModelBasedTestMarker>,
    /// The result record of the selected clause.
    pub model_based_test: ModelBasedTestResultRecord,
}

impl Labelled for ModelBasedTestDocument {
    const FORMAT: WireFormat = WireFormat::ModelBasedTestResult;
}

/// The only `verdict` a record writes, for a refuted clause.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// The clause is refuted.
    Refuted,
}

/// A cause named by a bare string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CauseName {
    /// No generated case reached an evaluated check point.
    NoEvaluatedCase,
    /// A frame clause has no implementation binding.
    NoContractLevelReading,
}

/// The only `kind` of a limit cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LimitReachedKind {
    /// A limit stopped the run.
    #[serde(rename = "limit-reached")]
    LimitReached,
}

/// The only `kind` of a tool-failure cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolFailureKind {
    /// The generator or shrinker disagreed with native admission or replay.
    #[serde(rename = "tool-failure")]
    ToolFailure,
}

/// A native-oracle consistency failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Defect {
    /// Native admission accepted a value the generator classified as refused.
    OracleDisagreement,
    /// A retained shrink left its domain or changed the failing clause.
    InvalidShrink,
}

/// A cause that names the limit reached.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitCause {
    /// Always `limit-reached`.
    pub kind: LimitReachedKind,
    /// The limit's name.
    pub limit: String,
    /// The configured limit.
    pub value: u64,
    /// The setting that raises the limit.
    pub setting: String,
}

/// A cause that names a tool defect.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolFailureCause {
    /// Always `tool-failure`.
    pub kind: ToolFailureKind,
    /// The failed consistency check.
    pub defect: Defect,
}

/// The settlement cause of a record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cause {
    /// A bare name.
    Name(CauseName),
    /// A reached limit.
    Limit(LimitCause),
    /// A tool failure.
    ToolFailure(ToolFailureCause),
}

/// The case counts, with the discards by reason.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cases {
    /// Cases drawn.
    pub generated: u64,
    /// Cases that passed.
    pub passed: u64,
    /// Cases that failed.
    pub failed: u64,
    /// Cases discarded.
    pub discarded: u64,
    /// Discarded cases by reason.
    pub discarded_by: BTreeMap<ModelBasedTestDiscardReason, ModelBasedTestDiscardCount>,
}

/// The check points evaluated and skipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    /// Check points evaluated.
    pub evaluated: u64,
    /// Check points skipped.
    pub skipped: usize,
}

/// The run's limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunLimits {
    /// The requested number of cases.
    pub max_cases: u64,
    /// The requested trace length.
    pub max_trace_length: usize,
    /// The value-size bound, `null` when none.
    #[serde(deserialize_with = "nullable")]
    pub max_value_bits: Option<u64>,
}

/// One root admitted from a size-bounded domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeBounded {
    /// The root declaration node.
    pub node: String,
    /// The child path to the root.
    pub path: Vec<u32>,
    /// The number of values in the domain.
    pub values: String,
}

/// The value-bound cuts observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueBoundCuts {
    /// The number of cuts.
    pub count: u64,
}

/// Sampling provenance of the run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// The generator seed.
    pub seed: u64,
    /// The value-size bound, `null` when none.
    #[serde(deserialize_with = "nullable")]
    pub max_value_bits: Option<u64>,
    /// The size-bounded roots.
    pub size_bounded: Vec<SizeBounded>,
    /// The value-bound cuts.
    pub value_bound_cuts: ValueBoundCuts,
}

/// The only `type` of an integer value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegerTag {
    /// An integer.
    Integer,
}

/// A typed integer value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegerValue {
    /// Always `integer`.
    #[serde(rename = "type")]
    pub kind: IntegerTag,
    /// The decimal spelling.
    pub value: String,
}

/// How one partition was accounted for.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase", deny_unknown_fields)]
pub enum PartitionStatus {
    /// A case held the value at the root.
    Covered {
        /// The first covering case.
        first_case: u64,
    },
    /// Native admission refused the value.
    Refused {
        /// The refusing case.
        first_case: u64,
    },
    /// No case established coverage.
    Uncovered {
        /// The limits that bounded the claim.
        bounds: Vec<String>,
    },
}

/// One generated boundary partition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    /// The declaration node that owns the domain.
    pub node: String,
    /// The path from the node to the domain.
    pub path: Vec<u32>,
    /// The boundary the value represents.
    pub kind: ModelBoundaryKind,
    /// The generated value.
    pub value: IntegerValue,
    /// The text value of a Text partition, `null` otherwise.
    #[serde(deserialize_with = "nullable")]
    pub text_value: Option<String>,
    /// The accounting of the partition, `null` before a run completes.
    #[serde(deserialize_with = "nullable")]
    pub status: Option<PartitionStatus>,
}

/// The only `minimality` of a counterexample.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Minimality {
    /// Removing any step removes the failure.
    #[serde(rename = "1-minimal")]
    OneMinimal,
}

/// The only `replay` of a counterexample.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Replay {
    /// The failure reproduced.
    Reproduced,
}

/// How a trace counterexample fails.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceDisposition {
    /// The clause's invariant is violated.
    InvariantViolated,
    /// The model check failed.
    ModelCheckFailed,
}

/// The only disposition of an assignment counterexample.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssignmentDisposition {
    /// The predicate is false for the assignment.
    PredicateFalse,
}

/// One invocation of a trace counterexample.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    /// The operation identity.
    pub operation: String,
    /// The receiver, `null` when none.
    #[serde(deserialize_with = "nullable")]
    pub receiver: Option<String>,
    /// The argument texts.
    pub arguments: Vec<String>,
}

/// A shrunk, replayed trace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceCounterexample {
    /// Always `1-minimal`.
    pub minimality: Minimality,
    /// Always `reproduced`.
    pub replay: Replay,
    /// How the trace fails.
    pub disposition: TraceDisposition,
    /// The initial state key, in hex.
    pub initial: String,
    /// The invocations in step order.
    pub invocations: Vec<Invocation>,
}

/// One value of an assignment counterexample.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssignedValue {
    /// An integer value.
    Integer(IntegerValue),
    /// A value of another type, as its Rust `Debug` text.
    Debug {
        /// The `Debug` text.
        debug: String,
    },
}

/// A shrunk, replayed value assignment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignmentCounterexample {
    /// Always `1-minimal`.
    pub minimality: Minimality,
    /// Always `reproduced`.
    pub replay: Replay,
    /// Always `predicate-false`.
    pub disposition: AssignmentDisposition,
    /// The assigned values.
    pub assignment: Vec<AssignedValue>,
}

/// The retained counterexample.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Counterexample {
    /// A trace over a model.
    Trace(TraceCounterexample),
    /// A value assignment.
    Assignment(AssignmentCounterexample),
}

/// The result record of a model-based test run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBasedTestResultRecord {
    /// The selected clause.
    pub clause: String,
    /// The package the clause belongs to.
    pub domain: String,
    /// The settlement of the clause.
    pub status: ModelBasedTestEvidence,
    /// The evidence kind, the same value as `status`.
    pub evidence: ModelBasedTestEvidence,
    /// `refuted` for a refuted clause, `null` otherwise.
    #[serde(deserialize_with = "nullable")]
    pub verdict: Option<Verdict>,
    /// The settlement cause, `null` when none applies.
    #[serde(deserialize_with = "nullable")]
    pub cause: Option<Cause>,
    /// The case counts.
    pub cases: Cases,
    /// The check points.
    pub checks: Checks,
    /// The run's limits.
    pub limits: RunLimits,
    /// The sampling provenance.
    pub provenance: Provenance,
    /// The boundary partitions.
    pub partitions: Vec<Partition>,
    /// The coverage of the partitions.
    pub partition_coverage: ModelBasedPartitionCoverage,
    /// One entry per declared operation, `null` for a value clause.
    #[serde(deserialize_with = "nullable")]
    pub operations: Option<Vec<ModelOperationCoverageEntry>>,
    /// The operation coverage summary, `null` for a value clause.
    #[serde(deserialize_with = "nullable")]
    pub coverage: Option<ModelBasedCoverage>,
    /// The retained counterexample, `null` when none.
    #[serde(deserialize_with = "nullable")]
    pub counterexample: Option<Counterexample>,
}

/// Read the result of a direct model-based test run: its own document, or the
/// `native-run-result/2` command error of a refused request.
///
/// # Errors
///
/// [`NativeRunReadError`] for another label, a refused bound, or an unknown,
/// missing or mistyped member, naming that member.
#[implements("FR-752-AC-21")]
pub fn read_model_based_test_result(
    bytes: &[u8],
    limits: NativeRunLimits,
) -> Result<RouteResult<ModelBasedTestDocument>, NativeRunReadError> {
    read_route_result(bytes, limits)
}
