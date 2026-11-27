// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255: the one stable setting name of every configurable resource limit.
//!
//! [`Setting`] is the closed set of names FSL accepts at every entry point:
//! a limits type's builder, a replay request's `stage_limits` entry and the
//! settings operation behind the driver's `--limit <name>=<value>`. Each
//! setting fixes its dotted name, the stage that owns it and the
//! [`LimitKind`] its outcome reports, so a producer of a limit outcome names
//! the setting and gets its kind from the same row.
//!
//! A limits type reports its fields through [`SettingLimits`]: each field has
//! exactly one setting, and [`SettingLimits::set_bound`] changes that field
//! and nothing else. Every bound is used as given, with no ceiling.

use crate::diagnostic::LimitKind;
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use ix_trace_rs::implements;

/// Declares [`Setting`] and its table from one list, so a name, its stage, its
/// limit kind and its published default are written once.
macro_rules! settings {
    ($($variant:ident => $name:literal, $stage:literal, $kind:ident, $default:expr;)+) => {
        /// A configurable resource limit's stable setting name (FR-255).
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum Setting {
            $(
                #[doc = concat!("`", $name, "`.")]
                $variant,
            )+
        }

        impl Setting {
            /// Every setting, in FR-255's table order.
            pub const ALL: &'static [Setting] = &[$(Self::$variant),+];

            /// The setting's dotted name, such as `s3.nodes`.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }

            /// The stage that owns the limit, as FR-255's table spells it.
            #[must_use]
            pub const fn stage(self) -> &'static str {
                match self {
                    $(Self::$variant => $stage,)+
                }
            }

            /// The limit kind an outcome for this setting reports.
            #[must_use]
            pub const fn kind(self) -> LimitKind {
                match self {
                    $(Self::$variant => LimitKind::$kind,)+
                }
            }

            /// The published default FR-255's tables give the setting, or
            /// `None` for a row whose value the caller must supply.
            #[must_use]
            #[implements("FR-255-AC-6", "FR-255-AC-27")]
            pub const fn default_bound(self) -> Option<u64> {
                match self {
                    $(Self::$variant => $default,)+
                }
            }
        }
    };
}

settings! {
    S1InputBytes => "s1.input_bytes", "S1", InputBytes, Some(1_048_576);
    S1Tokens => "s1.tokens", "S1", TokenCount, Some(100_000);
    S1Nodes => "s1.nodes", "S1", NodeCount, Some(50_000);
    S1WorkUnits => "s1.work_units", "S1", WorkBudget, Some(288);
    S1Diagnostics => "s1.diagnostics", "S1", DiagnosticCount, Some(100);
    S3Nodes => "s3.nodes", "S3", NodeCount, Some(100_000);
    S3InputBytes => "s3.input_bytes", "S3", InputBytes, Some(16_777_216);
    S3WorkUnits => "s3.work_units", "S3", WorkBudget, Some(16_777_216);
    S3RationalBits => "s3.rational_bits", "S3", RationalBits, Some(65_536);
    S3DecimalScale => "s3.decimal_scale", "S3 assembly", WorkBudget, Some(4096);
    S3DecimalLiteralScale => "s3.decimal_literal_scale", "S3", WorkBudget, Some(65_535);
    S3DecimalLiteralScaleTotal => "s3.decimal_literal_scale_total", "S3", WorkBudget, Some(1_048_576);
    EnvironmentAncestorSteps => "environment.ancestor_steps", "S3 type-environment admission", EdgeCount, Some(16_777_216);
    EnvironmentWorkUnits => "environment.work_units", "S3 type-environment admission", WorkBudget, Some(16_777_216);
    ModelDeclarationRecords => "model.declaration_records", "model normalization", NodeCount, Some(100_000);
    ModelSequenceMaximum => "model.sequence_maximum", "model intake", NodeCount, Some(10_000);
    ModelDerivationFacts => "model.derivation_facts", "model normalization", NodeCount, Some(1_600_000);
    ModelEffectiveDeclarations => "model.effective_declarations", "model normalization", NodeCount, Some(1_600_000);
    ModelDispatchCandidates => "model.dispatch_candidates", "model normalization", NodeCount, Some(1_600_000);
    ModelWorkUnits => "model.work_units", "model normalization", WorkBudget, Some(16_777_216);
    ModelAncestorSteps => "model.ancestor_steps", "model normalization", EdgeCount, Some(16_777_216);
    ModelFamilySteps => "model.family_steps", "model normalization", EdgeCount, Some(16_777_216);
    TemporalEvidenceFormulaNodes => "temporal_evidence.formula_nodes", "temporal evidence call", NodeCount, Some(100_000);
    TemporalEvidenceInstances => "temporal_evidence.instances", "temporal evidence call", NodeCount, Some(10_000);
    TemporalEvidencePositions => "temporal_evidence.positions", "temporal evidence call", NodeCount, Some(1_000_000);
    TemporalEvidenceValuations => "temporal_evidence.valuations", "temporal evidence call", NodeCount, Some(1_000_000);
    TemporalEvidenceCaptures => "temporal_evidence.captures", "temporal evidence call", NodeCount, Some(100_000);
    TemporalEvidenceSupport => "temporal_evidence.support", "temporal evidence call", NodeCount, Some(1_000_000);
    TemporalEvidenceHistorySpan => "temporal_evidence.history_span", "temporal evidence call", TickCount, Some(u64::MAX);
    TemporalEvidenceEvaluationSteps => "temporal_evidence.evaluation_steps", "temporal evidence call", WorkBudget, Some(1_000_000);
    TemporalEvidenceLineage => "temporal_evidence.lineage", "temporal evidence call", NodeCount, Some(1024);
    TemporalEvidenceVisited => "temporal_evidence.visited", "temporal evidence call", WorkBudget, Some(2_000_000);
    TemporalEvidenceRequestBytes => "temporal_evidence.request_bytes", "temporal evidence call", InputBytes, Some(8_388_608);
    TemporalEvidenceResultBytes => "temporal_evidence.result_bytes", "temporal evidence call", InputBytes, Some(8_388_608);
    TemporalEvidenceMemberBytes => "temporal_evidence.member_bytes", "temporal evidence call", InputBytes, Some(1_048_576);
    AdmissionPopulationMembers => "admission.population_members", "population admission", NodeCount, Some(100_000);
    AdmissionWorkUnits => "admission.work_units", "population admission", WorkBudget, Some(16_777_216);
    AdmissionAncestorSteps => "admission.ancestor_steps", "population admission", EdgeCount, Some(16_777_216);
    ObservationInputBytes => "observation.input_bytes", "observation admission", InputBytes, Some(1_048_576);
    ObservationObjects => "observation.objects", "observation admission", NodeCount, Some(10_000);
    ObservationValues => "observation.values", "observation admission", NodeCount, Some(100_000);
    DependencyLibraries => "dependency.libraries", "S4 source resolution", NodeCount, Some(4096);
    DependencyImportEdges => "dependency.import_edges", "S4 source resolution", EdgeCount, Some(16_384);
    DependencySourceBytes => "dependency.source_bytes", "S4 source resolution", InputBytes, Some(16_777_216);
    LibraryDefinitions => "library.definitions", "library resolution", NodeCount, Some(4096);
    LibraryDependencyEdges => "library.dependency_edges", "library resolution", EdgeCount, Some(16_384);
    LibraryArtifactBytes => "library.artifact_bytes", "library resolution", InputBytes, Some(16_777_216);
    LibrarySingleArtifactBytes => "library.single_artifact_bytes", "library resolution", InputBytes, Some(1_048_576);
    IntakeInputBytes => "intake.input_bytes", "I1 semantic-IR intake", InputBytes, Some(67_108_864);
    IdentityInputBytes => "identity.input_bytes", "identity encoding", InputBytes, Some(16_777_216);
    ReplayInputBytes => "replay.input_bytes", "replay envelope readers", InputBytes, Some(1_048_576);
    FormatOutputBytes => "format.output_bytes", "format", InputBytes, Some(1_048_576);
    RenderOutputBytes => "render.output_bytes", "render", InputBytes, Some(16_777_216);
    PrepareCompileWorkUnits => "prepare.compile_work_units", "execution-backend prepare", WorkBudget, Some(16_777_216);
    PrepareCodeBytes => "prepare.code_bytes", "execution-backend prepare", InputBytes, Some(67_108_864);
    I2InputBytes => "i2.input_bytes", "I2 v2 reader", InputBytes, Some(16_777_216);
    I2Nodes => "i2.nodes", "I2 v2 reader", NodeCount, Some(10_000);
    I2Edges => "i2.edges", "I2 v2 reader", EdgeCount, Some(100_000);
    I2Occurrences => "i2.occurrences", "I2 v2 reader", OccurrenceCount, Some(100_000);
    I2Diagnostics => "i2.diagnostics", "I2 v2 reader", DiagnosticCount, Some(10_000);
    I2WorkUnits => "i2.work_units", "I2 v2 reader", WorkBudget, Some(1_000_000);
    I2NativeJoinStorage => "i2.native_join_storage", "I2 v2 reader native identity join", LogicalStorage, Some(16_777_216);
    EmitOutputBytes => "emit.output_bytes", "E4 emit", InputBytes, Some(16_777_216);
    ExportOutputBytes => "export.output_bytes", "export", InputBytes, Some(16_777_216);
    ExportNodes => "export.nodes", "export", NodeCount, Some(100_000);
    ExportWorkUnits => "export.work_units", "export", WorkBudget, Some(16_777_216);
    ExploreStates => "explore.states", "exploration", NodeCount, Some(10_000_000);
    ExploreTransitions => "explore.transitions", "exploration", EdgeCount, Some(100_000_000);
    ModelCheckMaxStates => "model_check.max_states", "EN-1 model checker", NodeCount, Some(10_000_000);
    ModelCheckMaxTransitions => "model_check.max_transitions", "EN-1 model checker", EdgeCount, Some(100_000_000);
    ModelCheckMaxAutomatonStates => "model_check.max_automaton_states", "EN-1 model checker", NodeCount, Some(1_048_576);
    ModelCheckWitnessSamples => "model_check.witness_samples", "EN-1 phase 0", NodeCount, Some(64);
    ModelCheckMaxWalkSteps => "model_check.max_walk_steps", "EN-1 phase 0", EdgeCount, Some(4096);
    ModelCheckMaxWitnessSet => "model_check.max_witness_set", "EN-1 hyper runs", NodeCount, Some(65_536);
    ModelCheckMaxRelationTuples => "model_check.max_relation_tuples", "EN-1 hyper runs", NodeCount, Some(16_777_216);
    ModelCheckMaxLiveInstances => "model_check.max_live_instances", "EN-1 protocol subjects", NodeCount, Some(3);
    ModelCheckMaxStoreBuffer => "model_check.max_store_buffer", "EN-1 weak memory", NodeCount, Some(4);
    ModelCheckMaxMessages => "model_check.max_messages", "EN-1 weak memory", NodeCount, Some(4);
    ExpansionMaxCandidates => "expansion.max_candidates", "model-subject expansion", NodeCount, Some(200_000);
    MonitorMaxWindowValues => "monitor.max_window_values", "monitor window aggregates", NodeCount, Some(1_048_576);
    MonitorRetainedPositions => "monitor.retained_positions", "monitor evidence retention", NodeCount, Some(1_048_576);
    MonitorRetainedIdentities => "monitor.retained_identities", "monitor evidence retention", NodeCount, Some(1_048_576);
    MonitorMaxActivations => "monitor.max_activations", "timed monitor plan", NodeCount, None;
    MonitorPreparationWorkUnits => "monitor.preparation_work_units", "monitor plan preparation", WorkBudget, None;
    MonitorPreparationStorage => "monitor.preparation_storage", "monitor plan read storage", LogicalStorage, None;
    InspectMaxViewEntries => "inspect.max_view_entries", "typed view construction", NodeCount, Some(10_000_000);
    ProgressCoveredPositions => "progress.covered_positions", "progress ledger", NodeCount, Some(1_000_000);
    SimulationMaxSteps => "simulation.max_steps", "simulation and model-based test traces", NodeCount, Some(16_777_216);
    ExactProbMaxStates => "exact_prob.max_states", "EN-5 product", NodeCount, Some(10_000_000);
    ExactProbMaxTransitions => "exact_prob.max_transitions", "EN-5 product", EdgeCount, Some(100_000_000);
    ExactProbMaxAutomatonStates => "exact_prob.max_automaton_states", "EN-5 product", NodeCount, Some(1_048_576);
    ExactProbMaxIterations => "exact_prob.max_iterations", "EN-5", WorkBudget, Some(1_000_000);
    ExactProbPrecisionBits => "exact_prob.precision_bits", "EN-5", RationalBits, Some(64);
    ExactProbMaxPrecisionBits => "exact_prob.max_precision_bits", "EN-5", RationalBits, Some(4096);
    ExactProbMaxRationalBits => "exact_prob.max_rational_bits", "EN-5", RationalBits, Some(65_536);
    ExactProbMaxPolicyIterations => "exact_prob.max_policy_iterations", "EN-5", WorkBudget, Some(10_000);
    ExactProbMaxWitnessPaths => "exact_prob.max_witness_paths", "EN-5", NodeCount, Some(1024);
    StatisticalMaxSamples => "statistical.max_samples", "EN-4", NodeCount, Some(16_777_216);
    StatisticalMaxDraws => "statistical.max_draws", "EN-4", NodeCount, Some(67_108_864);
    StatisticalMaxCycleSteps => "statistical.max_cycle_steps", "EN-4", EdgeCount, Some(16_777_216);
    StatisticalMaxSampleSteps => "statistical.max_sample_steps", "EN-4 timed sampling", EdgeCount, Some(16_777_216);
    ZoneCheckMaxSymbolicStates => "zone_check.max_symbolic_states", "EN-6 zone search", NodeCount, Some(1_048_576);
    ZoneCheckMaxSymbolicTransitions => "zone_check.max_symbolic_transitions", "EN-6 zone search", EdgeCount, Some(16_777_216);
    ZoneCheckMaxAutomatonStates => "zone_check.max_automaton_states", "EN-6 zone search", NodeCount, Some(1_048_576);
    ZoneCheckMaxZoneBytes => "zone_check.max_zone_bytes", "EN-6 zone search", InputBytes, Some(1_073_741_824);
    ZoneCheckMaxCheckWork => "zone_check.max_check_work", "EN-6 task-product search pass and certificate checker", WorkBudget, Some(100_000_000);
    AnalysisRetainedBytes => "analysis.retained_bytes", "analysis item retained exploration state", LogicalStorage, Some(4_294_967_296);
    CertificateCheckMaxCertificateEdges => "certificate_check.max_certificate_edges", "certificate checkers", EdgeCount, Some(16_777_216);
    CertificateCheckMaxCertificateBytes => "certificate_check.max_certificate_bytes", "certificate checkers", InputBytes, Some(1_073_741_824);
    CertificateCheckMaxWorkingBytes => "certificate_check.max_working_bytes", "probability document writer/reader", InputBytes, Some(1_073_741_824);
    CertificateCheckMaxRank => "certificate_check.max_rank", "exact reachability document writer", NodeCount, Some(9_007_199_254_740_991);
    ScheduleMaxDemandPoints => "schedule.max_demand_points", "schedulability analysis", NodeCount, Some(1_048_576);
    HybridMaxSolverSteps => "hybrid.max_solver_steps", "hybrid provider", WorkBudget, Some(1_000_000);
    HybridMaxSolverTime => "hybrid.max_solver_time", "hybrid provider", WorkBudget, Some(60);
    SmtEncodingMaxConstants => "smt_encoding.max_constants", "SMT proof-query encoding", NodeCount, Some(1_048_576);
    SmtEncodingMaxApplications => "smt_encoding.max_applications", "SMT proof-query encoding", NodeCount, Some(65_536);
    SmtEncodingMaxMonitorAtoms => "smt_encoding.max_monitor_atoms", "SMT proof-query encoding", NodeCount, Some(8);
    SmtEncodingMaxMonitorStates => "smt_encoding.max_monitor_states", "SMT proof-query encoding", NodeCount, Some(64);
    SmtEncodingMaxCandidates => "smt_encoding.max_candidates", "SMT proof-query encoding", NodeCount, Some(65_536);
    SmtEncodingMaxScriptBytes => "smt_encoding.max_script_bytes", "SMT proof-query encoding", InputBytes, Some(268_435_456);
    SmtMaxSolverSteps => "smt.max_solver_steps", "SMT backend", WorkBudget, Some(100_000_000);
    SmtMaxSolverTime => "smt.max_solver_time", "SMT backend", WorkBudget, Some(60);
    PlanSearchMaxStates => "plan_search.max_states", "plan search", NodeCount, Some(10_000_000);
    PlanSearchMaxWork => "plan_search.max_work", "plan search", WorkBudget, Some(100_000_000);
    PlanSearchMaxCertificateEntries => "plan_search.max_certificate_entries", "plan search", NodeCount, Some(16_777_216);
    PlanSearchMaxCheckWork => "plan_search.max_check_work", "plan certificate checker", WorkBudget, Some(100_000_000);
    PlanSearchMaxStateKeyBytes => "plan_search.max_state_key_bytes", "plan search", InputBytes, Some(1_048_576);
}

impl Setting {
    /// How many settings there are.
    pub const COUNT: usize = Self::ALL.len();

    /// The setting's position in [`Self::ALL`], below [`Self::COUNT`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The largest bound this setting takes: the width of the quantity it
    /// bounds (FR-255 "Setting names"). Every limit over a `u64` count takes
    /// `u64::MAX`; `model.sequence_maximum`, over a declared collection
    /// maximum that is a `u32`, takes `u32::MAX` (FR-506).
    #[must_use]
    pub const fn greatest_bound(self) -> u64 {
        match self {
            Self::ModelSequenceMaximum => u32::MAX as u64,
            _ => u64::MAX,
        }
    }

    /// Whether the setting is an engine-table row whose value is a positive
    /// count (FR-255 "Engine limit setting names"): every engine row except
    /// `model_check.witness_samples` and `exact_prob.max_witness_paths`, whose
    /// owners give 0 a meaning. The settings operation refuses 0 for it.
    #[must_use]
    pub const fn is_positive_count(self) -> bool {
        match self {
            Self::ModelCheckWitnessSamples | Self::ExactProbMaxWitnessPaths => false,
            Self::ModelCheckMaxStates
            | Self::ModelCheckMaxTransitions
            | Self::ModelCheckMaxAutomatonStates
            | Self::ModelCheckMaxWalkSteps
            | Self::ModelCheckMaxWitnessSet
            | Self::ModelCheckMaxRelationTuples
            | Self::ModelCheckMaxLiveInstances
            | Self::ModelCheckMaxStoreBuffer
            | Self::ModelCheckMaxMessages
            | Self::ExpansionMaxCandidates
            | Self::SimulationMaxSteps => true,
            _ => self.is_item_engine_setting(),
        }
    }

    /// Whether the setting bounds an analysis item engine that FSL's replay
    /// never executes: the engine rows beyond `model_check.*` and
    /// `expansion.max_candidates`. A replay request refuses it (FR-255 "Lifecycle
    /// caller-limit coverage").
    #[must_use]
    #[implements("FR-255-AC-24")]
    pub const fn is_item_engine_setting(self) -> bool {
        matches!(
            self,
            Self::ExactProbMaxStates
                | Self::ExactProbMaxTransitions
                | Self::ExactProbMaxAutomatonStates
                | Self::ExactProbMaxIterations
                | Self::ExactProbPrecisionBits
                | Self::ExactProbMaxPrecisionBits
                | Self::ExactProbMaxRationalBits
                | Self::ExactProbMaxPolicyIterations
                | Self::ExactProbMaxWitnessPaths
                | Self::StatisticalMaxSamples
                | Self::StatisticalMaxDraws
                | Self::StatisticalMaxCycleSteps
                | Self::StatisticalMaxSampleSteps
                | Self::ZoneCheckMaxSymbolicStates
                | Self::ZoneCheckMaxSymbolicTransitions
                | Self::ZoneCheckMaxAutomatonStates
                | Self::ZoneCheckMaxZoneBytes
                | Self::ZoneCheckMaxCheckWork
                | Self::AnalysisRetainedBytes
                | Self::CertificateCheckMaxCertificateEdges
                | Self::CertificateCheckMaxCertificateBytes
                | Self::CertificateCheckMaxWorkingBytes
                | Self::CertificateCheckMaxRank
                | Self::ScheduleMaxDemandPoints
                | Self::HybridMaxSolverSteps
                | Self::HybridMaxSolverTime
                | Self::SmtEncodingMaxConstants
                | Self::SmtEncodingMaxApplications
                | Self::SmtEncodingMaxMonitorAtoms
                | Self::SmtEncodingMaxMonitorStates
                | Self::SmtEncodingMaxCandidates
                | Self::SmtEncodingMaxScriptBytes
                | Self::SmtMaxSolverSteps
                | Self::SmtMaxSolverTime
                | Self::PlanSearchMaxStates
                | Self::PlanSearchMaxWork
                | Self::PlanSearchMaxCertificateEntries
                | Self::PlanSearchMaxCheckWork
                | Self::PlanSearchMaxStateKeyBytes
        )
    }

    /// The setting named `name`, or `None` when no setting has that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|setting| setting.name() == name)
    }
}

impl core::fmt::Display for Setting {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The ten accounting budgets of a call (`filament.value.accounting/v1`
/// counters), each named `accounting.<counter>` (FR-255's accounting
/// table, Behavior 9). They are settings but not stage limits: a reached budget settles
/// `Incomplete`, so they have no [`LimitKind`] and are not [`Setting`]s,
/// which keeps every `stage_limit_exceeded` producer total over its kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AccountingSetting {
    /// `integer_bits`.
    IntegerBits,
    /// `decimal_digits`.
    DecimalDigits,
    /// `scale_expansion`.
    ScaleExpansion,
    /// `text_input_bytes`.
    TextInputBytes,
    /// `text_scalars`.
    TextScalars,
    /// `normalized_scalars`.
    NormalizedScalars,
    /// `unit_edges`.
    UnitEdges,
    /// `value_occurrences`.
    ValueOccurrences,
    /// `work_units`.
    WorkUnits,
    /// `result_units`.
    ResultUnits,
}

impl AccountingSetting {
    /// Every accounting setting, in FR-255's accounting-table order.
    pub const ALL: [Self; 10] = [
        Self::IntegerBits,
        Self::DecimalDigits,
        Self::ScaleExpansion,
        Self::TextInputBytes,
        Self::TextScalars,
        Self::NormalizedScalars,
        Self::UnitEdges,
        Self::ValueOccurrences,
        Self::WorkUnits,
        Self::ResultUnits,
    ];

    /// The setting name, `accounting.<counter>`, such as `accounting.work_units`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IntegerBits => "accounting.integer_bits",
            Self::DecimalDigits => "accounting.decimal_digits",
            Self::ScaleExpansion => "accounting.scale_expansion",
            Self::TextInputBytes => "accounting.text_input_bytes",
            Self::TextScalars => "accounting.text_scalars",
            Self::NormalizedScalars => "accounting.normalized_scalars",
            Self::UnitEdges => "accounting.unit_edges",
            Self::ValueOccurrences => "accounting.value_occurrences",
            Self::WorkUnits => "accounting.work_units",
            Self::ResultUnits => "accounting.result_units",
        }
    }

    /// The counter's bare `filament.value.accounting/v1` member name, such as
    /// `work_units`: the key a call's `accounting` object uses, which is no
    /// setting name.
    #[must_use]
    pub const fn counter(self) -> &'static str {
        match self {
            Self::IntegerBits => "integer_bits",
            Self::DecimalDigits => "decimal_digits",
            Self::ScaleExpansion => "scale_expansion",
            Self::TextInputBytes => "text_input_bytes",
            Self::TextScalars => "text_scalars",
            Self::NormalizedScalars => "normalized_scalars",
            Self::UnitEdges => "unit_edges",
            Self::ValueOccurrences => "value_occurrences",
            Self::WorkUnits => "work_units",
            Self::ResultUnits => "result_units",
        }
    }

    /// The setting of the counter whose member name is `counter`, or `None`.
    #[must_use]
    pub fn from_counter(counter: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|setting| setting.counter() == counter)
    }
}

impl core::fmt::Display for AccountingSetting {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Any name the settings operation and a request's `stage_limits` accept:
/// a stage [`Setting`] or an [`AccountingSetting`]. A stage name holds a
/// `.` and an accounting name holds none, so no name is both.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingName {
    /// A stage limit.
    Stage(Setting),
    /// An accounting budget.
    Accounting(AccountingSetting),
}

impl SettingName {
    /// The setting named `name` in either table, or `None`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Setting::from_name(name).map(Self::Stage).or_else(|| {
            AccountingSetting::ALL
                .iter()
                .copied()
                .find(|setting| setting.name() == name)
                .map(Self::Accounting)
        })
    }

    /// The setting's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stage(setting) => setting.name(),
            Self::Accounting(setting) => setting.name(),
        }
    }
}

/// A limits type: the settings its fields carry, and the one place that maps
/// each field to its setting (FR-255 Behavior 2).
pub trait SettingLimits {
    /// Every field of this type with its setting and its current bound. Each
    /// setting appears at most once, and no two types report the same one.
    fn bounds(&self) -> Vec<(Setting, u64)>;

    /// Set the bound of the field `setting` names and change nothing else.
    /// Returns `false`, changing nothing, when this type has no such field.
    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool;
}

/// Why a `<name>=<value>` operand of the settings operation was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsageCause {
    /// The operand has no `=`.
    NotAnAssignment,
    /// The name is not a setting of FR-255's table.
    UnknownSetting,
    /// The setting is in FR-255's table but the limits type asked to hold it
    /// has no field for it.
    UnavailableSetting,
    /// The value is not a non-negative decimal integer that fits a bound.
    NotAnInteger,
    /// A worker count is zero; worker settings require a positive integer.
    InvalidWorkerValue,
    /// The setting was given more than once.
    Repeated,
}

/// The settings operation's usage refusal: it names the offending operand and
/// no stage runs (FR-255 Behavior 6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageRefusal {
    /// The operand as the caller wrote it.
    pub operand: String,
    /// Why it was refused.
    pub cause: UsageCause,
}

impl core::fmt::Display for UsageRefusal {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "usage: {:?} in `--limit {}`",
            self.cause, self.operand
        )?;
        if self.cause == UsageCause::UnknownSetting {
            formatter.write_str("; accepted settings:")?;
            for setting in Setting::ALL {
                write!(formatter, " {}", setting.name())?;
            }
            for setting in AccountingSetting::ALL {
                write!(formatter, " {}", setting.name())?;
            }
        }
        Ok(())
    }
}

impl core::error::Error for UsageRefusal {}

/// Parse the `<name>=<value>` operands of the settings operation (a name of
/// either FR-255 table) into settings and bounds, in operand order.
///
/// # Errors
///
/// [`UsageRefusal`] naming the first operand that is not `<name>=<value>`,
/// names no setting, holds a value that is not a non-negative decimal integer
/// of at most `u64::MAX`, or names a setting an earlier operand named.
#[implements("FR-255-AC-27")]
pub fn parse_operands<'a>(
    operands: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<(SettingName, u64)>, UsageRefusal> {
    let mut parsed: Vec<(SettingName, u64)> = Vec::new();
    for operand in operands {
        let refuse = |cause| UsageRefusal {
            operand: operand.to_owned(),
            cause,
        };
        let (name, value) = operand
            .split_once('=')
            .ok_or_else(|| refuse(UsageCause::NotAnAssignment))?;
        let setting =
            SettingName::from_name(name).ok_or_else(|| refuse(UsageCause::UnknownSetting))?;
        let bound = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
            value
                .parse::<u64>()
                .ok()
                .filter(|bound| match setting {
                    SettingName::Stage(stage) => {
                        *bound <= stage.greatest_bound() && (*bound != 0 || !stage.is_positive_count())
                    }
                    SettingName::Accounting(_) => true,
                })
                .ok_or_else(|| refuse(UsageCause::NotAnInteger))?
        } else {
            return Err(refuse(UsageCause::NotAnInteger));
        };
        if parsed.iter().any(|(earlier, _)| *earlier == setting) {
            return Err(refuse(UsageCause::Repeated));
        }
        parsed.push((setting, bound));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{parse_operands, AccountingSetting, Setting, SettingName, UsageCause};
    use alloc::vec;

    #[test]
    fn names_are_distinct_and_round_trip() {
        for (index, setting) in Setting::ALL.iter().enumerate() {
            assert_eq!(Setting::from_name(setting.name()), Some(*setting));
            assert!(
                Setting::ALL[index + 1..]
                    .iter()
                    .all(|other| other.name() != setting.name()),
                "{setting}"
            );
        }
    }

    #[ix_trace_rs::trace("FR-255-AC-3")]
    #[test]
    fn accounting_names_are_prefixed_counters_in_neither_stage_table_nor_bare() {
        for counter in AccountingSetting::ALL {
            let bare = counter
                .name()
                .strip_prefix("accounting.")
                .expect("an accounting setting is `accounting.<counter>`");
            assert_eq!(Setting::from_name(counter.name()), None, "{counter}");
            assert_eq!(
                SettingName::from_name(counter.name()),
                Some(SettingName::Accounting(counter))
            );
            assert_eq!(SettingName::from_name(bare), None, "bare counter {bare}");
            assert_eq!(counter.counter(), bare);
            assert_eq!(AccountingSetting::from_counter(bare), Some(counter));
        }
        assert!(Setting::ALL
            .iter()
            .all(|setting| !setting.name().starts_with("accounting.")));
    }

    #[ix_trace_rs::trace("FR-255-AC-6")]
    #[test]
    fn every_row_defaults_to_the_value_fr_255_tabulates() {
        const TABLE: &[(&str, Option<u64>)] = &[
            ("s1.input_bytes", Some(1048576)),
            ("s1.tokens", Some(100000)),
            ("s1.nodes", Some(50000)),
            ("s1.work_units", Some(288)),
            ("s1.diagnostics", Some(100)),
            ("s3.nodes", Some(100000)),
            ("s3.input_bytes", Some(16777216)),
            ("s3.work_units", Some(16777216)),
            ("s3.rational_bits", Some(65536)),
            ("s3.decimal_scale", Some(4096)),
            ("s3.decimal_literal_scale", Some(65535)),
            ("s3.decimal_literal_scale_total", Some(1048576)),
            ("environment.ancestor_steps", Some(16777216)),
            ("environment.work_units", Some(16777216)),
            ("model.declaration_records", Some(100000)),
            ("model.sequence_maximum", Some(10000)),
            ("model.derivation_facts", Some(1600000)),
            ("model.effective_declarations", Some(1600000)),
            ("model.dispatch_candidates", Some(1600000)),
            ("model.work_units", Some(16777216)),
            ("model.ancestor_steps", Some(16777216)),
            ("model.family_steps", Some(16777216)),
            ("temporal_evidence.formula_nodes", Some(100000)),
            ("temporal_evidence.instances", Some(10000)),
            ("temporal_evidence.positions", Some(1000000)),
            ("temporal_evidence.valuations", Some(1000000)),
            ("temporal_evidence.captures", Some(100000)),
            ("temporal_evidence.support", Some(1000000)),
            ("temporal_evidence.history_span", Some(u64::MAX)),
            ("temporal_evidence.evaluation_steps", Some(1000000)),
            ("temporal_evidence.lineage", Some(1024)),
            ("temporal_evidence.visited", Some(2000000)),
            ("temporal_evidence.request_bytes", Some(8388608)),
            ("temporal_evidence.result_bytes", Some(8388608)),
            ("temporal_evidence.member_bytes", Some(1048576)),
            ("admission.population_members", Some(100000)),
            ("admission.work_units", Some(16777216)),
            ("admission.ancestor_steps", Some(16777216)),
            ("observation.input_bytes", Some(1048576)),
            ("observation.objects", Some(10000)),
            ("observation.values", Some(100000)),
            ("dependency.libraries", Some(4096)),
            ("dependency.import_edges", Some(16384)),
            ("dependency.source_bytes", Some(16777216)),
            ("library.definitions", Some(4096)),
            ("library.dependency_edges", Some(16384)),
            ("library.artifact_bytes", Some(16777216)),
            ("library.single_artifact_bytes", Some(1048576)),
            ("intake.input_bytes", Some(67108864)),
            ("identity.input_bytes", Some(16777216)),
            ("replay.input_bytes", Some(1048576)),
            ("format.output_bytes", Some(1048576)),
            ("render.output_bytes", Some(16777216)),
            ("prepare.compile_work_units", Some(16777216)),
            ("prepare.code_bytes", Some(67108864)),
            ("i2.input_bytes", Some(16777216)),
            ("i2.nodes", Some(10000)),
            ("i2.edges", Some(100000)),
            ("i2.occurrences", Some(100000)),
            ("i2.diagnostics", Some(10000)),
            ("i2.work_units", Some(1000000)),
            ("i2.native_join_storage", Some(16777216)),
            ("emit.output_bytes", Some(16777216)),
            ("export.output_bytes", Some(16777216)),
            ("export.nodes", Some(100000)),
            ("export.work_units", Some(16777216)),
            ("explore.states", Some(10000000)),
            ("explore.transitions", Some(100000000)),
            ("model_check.max_states", Some(10000000)),
            ("model_check.max_transitions", Some(100000000)),
            ("model_check.max_automaton_states", Some(1048576)),
            ("model_check.witness_samples", Some(64)),
            ("model_check.max_walk_steps", Some(4096)),
            ("model_check.max_witness_set", Some(65536)),
            ("model_check.max_relation_tuples", Some(16777216)),
            ("model_check.max_live_instances", Some(3)),
            ("model_check.max_store_buffer", Some(4)),
            ("model_check.max_messages", Some(4)),
            ("expansion.max_candidates", Some(200000)),
            ("monitor.max_window_values", Some(1048576)),
            ("monitor.retained_positions", Some(1048576)),
            ("monitor.retained_identities", Some(1048576)),
            ("monitor.max_activations", None),
            ("monitor.preparation_work_units", None),
            ("monitor.preparation_storage", None),
            ("inspect.max_view_entries", Some(10000000)),
            ("progress.covered_positions", Some(1000000)),
            ("simulation.max_steps", Some(16777216)),
            ("exact_prob.max_states", Some(10000000)),
            ("exact_prob.max_transitions", Some(100000000)),
            ("exact_prob.max_automaton_states", Some(1048576)),
            ("exact_prob.max_iterations", Some(1000000)),
            ("exact_prob.precision_bits", Some(64)),
            ("exact_prob.max_precision_bits", Some(4096)),
            ("exact_prob.max_rational_bits", Some(65536)),
            ("exact_prob.max_policy_iterations", Some(10000)),
            ("exact_prob.max_witness_paths", Some(1024)),
            ("statistical.max_samples", Some(16777216)),
            ("statistical.max_draws", Some(67108864)),
            ("statistical.max_cycle_steps", Some(16777216)),
            ("statistical.max_sample_steps", Some(16777216)),
            ("zone_check.max_symbolic_states", Some(1048576)),
            ("zone_check.max_symbolic_transitions", Some(16777216)),
            ("zone_check.max_automaton_states", Some(1048576)),
            ("zone_check.max_zone_bytes", Some(1073741824)),
            ("zone_check.max_check_work", Some(100000000)),
            ("analysis.retained_bytes", Some(4294967296)),
            ("certificate_check.max_certificate_edges", Some(16777216)),
            ("certificate_check.max_certificate_bytes", Some(1073741824)),
            ("certificate_check.max_working_bytes", Some(1073741824)),
            ("certificate_check.max_rank", Some(9007199254740991)),
            ("schedule.max_demand_points", Some(1048576)),
            ("hybrid.max_solver_steps", Some(1000000)),
            ("hybrid.max_solver_time", Some(60)),
            ("smt_encoding.max_constants", Some(1048576)),
            ("smt_encoding.max_applications", Some(65536)),
            ("smt_encoding.max_monitor_atoms", Some(8)),
            ("smt_encoding.max_monitor_states", Some(64)),
            ("smt_encoding.max_candidates", Some(65536)),
            ("smt_encoding.max_script_bytes", Some(268435456)),
            ("smt.max_solver_steps", Some(100000000)),
            ("smt.max_solver_time", Some(60)),
            ("plan_search.max_states", Some(10000000)),
            ("plan_search.max_work", Some(100000000)),
            ("plan_search.max_certificate_entries", Some(16777216)),
            ("plan_search.max_check_work", Some(100000000)),
            ("plan_search.max_state_key_bytes", Some(1048576)),
        ];
        assert_eq!(TABLE.len(), Setting::COUNT);
        for (name, default) in TABLE {
            let setting = Setting::from_name(name).expect("a table row names a setting");
            assert_eq!(setting.default_bound(), *default, "{name}");
            assert_eq!(Setting::ALL[setting.index()], setting, "{name}");
        }
    }

    #[test]
    fn operands_parse_in_order_and_refuse_the_first_bad_one() {
        assert_eq!(
            parse_operands(["s3.nodes=5", "s1.tokens=0"]),
            Ok(vec![
                (SettingName::Stage(Setting::S3Nodes), 5),
                (SettingName::Stage(Setting::S1Tokens), 0)
            ])
        );
        for (operand, cause) in [
            ("s9.nodes=1", UsageCause::UnknownSetting),
            ("s3.nodes=ten", UsageCause::NotAnInteger),
            ("s3.nodes=-1", UsageCause::NotAnInteger),
            ("s3.nodes=+1", UsageCause::NotAnInteger),
            ("s3.nodes=", UsageCause::NotAnInteger),
            ("s3.nodes=18446744073709551616", UsageCause::NotAnInteger),
            ("s3.nodes", UsageCause::NotAnAssignment),
        ] {
            let refusal = parse_operands([operand]).unwrap_err();
            assert_eq!((refusal.operand.as_str(), refusal.cause), (operand, cause));
        }
        let repeated = parse_operands(["s3.nodes=5", "s3.nodes=6"]).unwrap_err();
        assert_eq!(repeated.operand, "s3.nodes=6");
        assert_eq!(repeated.cause, UsageCause::Repeated);
    }
}
