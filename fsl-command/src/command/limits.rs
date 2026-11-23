// SPDX-License-Identifier: AGPL-3.0-or-later
//! The caller limits of the `analyze`, `monitor` and `replay` entry points
//! (FR-255 "Lifecycle caller-limit coverage").
//!
//! [`CommandLimits`] is one operation's caller-limit object. It exposes
//! exactly the settings that operation consumes, through the shared
//! [`SettingLimits`] interface the driver applies `--limit` operands with:
//! [`SettingLimits::bounds`] lists the exposed settings and
//! [`SettingLimits::set_bound`] refuses, changing nothing, a setting the
//! operation does not expose. A setting the caller never sets leaves the
//! bound the request document or the published default gives it.

use std::collections::BTreeMap;

use fsl_analyze::analyze::AnalyzeLimits;
use fsl_analyze::monitor::{
    MonitorLimits, DEFAULT_MAX_WINDOW_VALUES, DEFAULT_RETAINED_IDENTITIES,
    DEFAULT_RETAINED_POSITIONS,
};
use fsl_foundation::diagnostic::LimitExceeded;
use fsl_foundation::{AccountingSetting, Setting, SettingLimits, SettingName};
use fsl_replay::spine::SpineLimits;
use fsl_replay::{AccountingLimits, CallerLimits};

/// The command entry point a [`CommandLimits`] configures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitedOperation {
    /// `analyze`: the request's program or package and its items.
    Analyze,
    /// `monitor`: the request's program, its trace and its clauses.
    Monitor,
    /// `replay`: the `native-replay/1` request.
    Replay,
}

impl LimitedOperation {
    /// The operation's verb, as a refusal names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Monitor => "monitor",
            Self::Replay => "replay",
        }
    }
}

/// A recognized setting the operation does not expose, refused before any
/// work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettingUnavailable {
    /// The operation that does not expose the setting.
    pub operation: LimitedOperation,
    /// The setting the caller named.
    pub setting: SettingName,
}

impl std::fmt::Display for SettingUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "setting `{}` is unavailable for the `{}` operation",
            self.setting.name(),
            self.operation.name()
        )
    }
}

impl std::error::Error for SettingUnavailable {}

/// Which monitor plan preparation allowance a refusal reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreparationBound {
    /// `monitor.preparation_work_units`, [`MonitorPreparationLimits::work`].
    Work,
    /// `monitor.preparation_storage`, [`MonitorPreparationLimits::storage`].
    Storage,
}

impl PreparationBound {
    /// The setting that raises this allowance.
    #[must_use]
    pub const fn setting(self) -> Setting {
        match self {
            Self::Work => Setting::MonitorPreparationWorkUnits,
            Self::Storage => Setting::MonitorPreparationStorage,
        }
    }
}

/// The caller's allowances for monitor plan preparation: cumulative work
/// and shared read storage, each a bound of its own (FR-255 engine table).
/// FSL owns the mapping from each setting to its bound; the caller supplies
/// the starting values, so a driver publishes its own defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorPreparationLimits {
    /// `monitor.preparation_work_units`: cumulative preparation work.
    pub work: u64,
    /// `monitor.preparation_storage`: bytes of read storage.
    pub storage: u64,
}

impl MonitorPreparationLimits {
    /// The refusal for a preparation that reached `bound`, whose counter
    /// would have reached `actual`. It names the setting and says how to
    /// raise it.
    #[must_use]
    pub fn refusal(&self, bound: PreparationBound, actual: u128) -> PreparationRefusal {
        let configured = match bound {
            PreparationBound::Work => self.work,
            PreparationBound::Storage => self.storage,
        };
        PreparationRefusal {
            exceeded: LimitExceeded::new(bound.setting(), configured, actual),
        }
    }
}

impl SettingLimits for MonitorPreparationLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (Setting::MonitorPreparationWorkUnits, self.work),
            (Setting::MonitorPreparationStorage, self.storage),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::MonitorPreparationWorkUnits => self.work = bound,
            Setting::MonitorPreparationStorage => self.storage = bound,
            _ => return false,
        }
        true
    }
}

/// A monitor plan preparation that reached one of its allowances.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparationRefusal {
    exceeded: LimitExceeded,
}

impl PreparationRefusal {
    /// FSL's typed limit outcome: the setting, its kind, the configured bound
    /// and the count the refused step would have reached.
    #[must_use]
    pub fn exceeded(&self) -> &LimitExceeded {
        &self.exceeded
    }
}

impl std::fmt::Display for PreparationRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let setting = self.exceeded.setting().name();
        write!(
            formatter,
            "monitor preparation reached `{setting}` (bound {}, needed {}); raise it with `--limit {setting}=<n>`",
            self.exceeded.configured_bound(),
            self.exceeded.actual()
        )
    }
}

impl std::error::Error for PreparationRefusal {}

/// The source, package, normalization, identity, I2-read, dependency and
/// observation settings `analyze`, `monitor` and `replay` all consume
/// (FR-255 "Lifecycle caller-limit coverage").
const COMMON: &[Setting] = &[
    Setting::S1InputBytes,
    Setting::S1Tokens,
    Setting::S1Nodes,
    Setting::S1WorkUnits,
    Setting::S3Nodes,
    Setting::S3InputBytes,
    Setting::S3WorkUnits,
    Setting::S3RationalBits,
    Setting::S3DecimalScale,
    Setting::S3DecimalLiteralScale,
    Setting::S3DecimalLiteralScaleTotal,
    Setting::EnvironmentAncestorSteps,
    Setting::EnvironmentWorkUnits,
    Setting::ModelDeclarationRecords,
    Setting::ModelSequenceMaximum,
    Setting::ModelDerivationFacts,
    Setting::ModelEffectiveDeclarations,
    Setting::ModelDispatchCandidates,
    Setting::ModelWorkUnits,
    Setting::ModelAncestorSteps,
    Setting::ModelFamilySteps,
    Setting::ObservationInputBytes,
    Setting::ObservationObjects,
    Setting::ObservationValues,
    Setting::IdentityInputBytes,
    Setting::I2InputBytes,
    Setting::I2Nodes,
    Setting::I2Edges,
    Setting::I2Occurrences,
    Setting::I2Diagnostics,
    Setting::I2WorkUnits,
    Setting::I2NativeJoinStorage,
    Setting::DependencyLibraries,
    Setting::DependencyImportEdges,
    Setting::DependencySourceBytes,
    Setting::IntakeInputBytes,
];

/// What `analyze` consumes beyond [`COMMON`]: the replay read bound and the
/// package emission bound.
const ANALYZE_ONLY: &[Setting] = &[Setting::ReplayInputBytes, Setting::EmitOutputBytes];

/// What `monitor` consumes beyond [`COMMON`].
const MONITOR_ONLY: &[Setting] = &[
    Setting::MonitorMaxWindowValues,
    Setting::MonitorRetainedPositions,
    Setting::MonitorRetainedIdentities,
    Setting::MonitorPreparationWorkUnits,
    Setting::MonitorPreparationStorage,
];

/// What `replay` executes beyond [`COMMON`]: population admission, library
/// resolution, the reader bound and the three model-check bounds graph
/// replay consumes.
const REPLAY_ONLY: &[Setting] = &[
    Setting::AdmissionPopulationMembers,
    Setting::AdmissionWorkUnits,
    Setting::AdmissionAncestorSteps,
    Setting::LibraryDefinitions,
    Setting::LibraryDependencyEdges,
    Setting::LibraryArtifactBytes,
    Setting::LibrarySingleArtifactBytes,
    Setting::ReplayInputBytes,
    Setting::ModelCheckMaxStates,
    Setting::ModelCheckMaxTransitions,
    Setting::ModelCheckMaxAutomatonStates,
];

/// One command entry point's caller limits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLimits {
    operation: LimitedOperation,
    stage: BTreeMap<Setting, u64>,
    accounting: BTreeMap<AccountingSetting, u64>,
    preparation: MonitorPreparationLimits,
}

impl CommandLimits {
    fn new(operation: LimitedOperation, preparation: MonitorPreparationLimits) -> Self {
        Self {
            operation,
            stage: BTreeMap::new(),
            accounting: BTreeMap::new(),
            preparation,
        }
    }

    /// The `analyze` limits with every setting at the request's or the
    /// published bound.
    #[must_use]
    pub fn analyze() -> Self {
        Self::new(LimitedOperation::Analyze, Self::unused_preparation())
    }

    /// The `replay` limits with every setting at the request's or the
    /// published bound.
    #[must_use]
    pub fn replay() -> Self {
        Self::new(LimitedOperation::Replay, Self::unused_preparation())
    }

    /// The `monitor` limits with every setting at the request's or the
    /// published bound, and plan preparation at `preparation`, the values the
    /// caller publishes as its defaults.
    #[must_use]
    pub fn monitor(preparation: MonitorPreparationLimits) -> Self {
        Self::new(LimitedOperation::Monitor, preparation)
    }

    /// The preparation bounds of an operation that prepares no monitor plan.
    const fn unused_preparation() -> MonitorPreparationLimits {
        MonitorPreparationLimits {
            work: u64::MAX,
            storage: u64::MAX,
        }
    }

    /// The operation these limits configure.
    #[must_use]
    pub const fn operation(&self) -> LimitedOperation {
        self.operation
    }

    /// Whether the operation exposes the stage `setting`.
    #[must_use]
    pub fn exposes(&self, setting: Setting) -> bool {
        COMMON.contains(&setting)
            || match self.operation {
                LimitedOperation::Analyze => ANALYZE_ONLY.contains(&setting),
                LimitedOperation::Monitor => MONITOR_ONLY.contains(&setting),
                LimitedOperation::Replay => REPLAY_ONLY.contains(&setting),
            }
    }

    /// Set the bound `name` names. Every accounting counter is exposed by
    /// every operation.
    ///
    /// # Errors
    ///
    /// [`SettingUnavailable`] when the operation does not expose a stage
    /// setting; nothing changes.
    pub fn set(&mut self, name: SettingName, bound: u64) -> Result<(), SettingUnavailable> {
        match name {
            SettingName::Stage(setting) if self.set_bound(setting, bound) => Ok(()),
            SettingName::Stage(_) => Err(SettingUnavailable {
                operation: self.operation,
                setting: name,
            }),
            SettingName::Accounting(setting) => {
                self.accounting.insert(setting, bound);
                Ok(())
            }
        }
    }

    /// The monitor plan preparation bounds: the caller's starting values with
    /// each `monitor.preparation_*` setting that was set.
    #[must_use]
    pub fn preparation(&self) -> MonitorPreparationLimits {
        let mut preparation = self.preparation;
        for (setting, bound) in &self.stage {
            preparation.set_bound(*setting, *bound);
        }
        preparation
    }

    /// The bound on each replay read: `replay.input_bytes` when set. It
    /// reaches the reader as an argument and is never a request member.
    #[must_use]
    pub fn replay_input_bytes(&self) -> u64 {
        self.resolved().replay.input_bytes
    }

    /// The compile limits: the published defaults with each set stage
    /// setting.
    #[must_use]
    pub fn spine(&self) -> SpineLimits {
        self.resolved().spine
    }

    /// `limits` with each set setting it carries replaced.
    pub(super) fn apply_to_analyze(&self, limits: &mut AnalyzeLimits) {
        let mut caller = CallerLimits {
            accounting: AccountingLimits(limits.accounting),
            observation: limits.observation,
            replay: limits.replay,
            ..CallerLimits::default()
        };
        caller.spine.model = limits.normalization;
        self.apply(&mut caller);
        limits.accounting = caller.accounting.0;
        limits.observation = caller.observation;
        limits.normalization = caller.spine.model;
        limits.replay = caller.replay;
        if let Some(bound) = self.stage.get(&Setting::EmitOutputBytes) {
            limits.emit = fsl_package::EmitLimits::new(*bound);
        }
    }

    /// The monitor run's limits: the published defaults with each set
    /// setting.
    pub(super) fn monitor_limits(&self) -> MonitorLimits {
        let caller = self.resolved();
        let bound = |setting, default| self.stage.get(&setting).copied().unwrap_or(default);
        MonitorLimits {
            observation: caller.observation,
            normalization: caller.spine.model,
            accounting: caller.accounting.0,
            max_window_values: bound(Setting::MonitorMaxWindowValues, DEFAULT_MAX_WINDOW_VALUES),
            retained_positions: bound(
                Setting::MonitorRetainedPositions,
                DEFAULT_RETAINED_POSITIONS,
            ),
            retained_identities: bound(
                Setting::MonitorRetainedIdentities,
                DEFAULT_RETAINED_IDENTITIES,
            ),
        }
    }

    /// The replay request's `stage_limits` and `accounting_limits` with each
    /// set setting replacing the request's entry for it.
    pub(super) fn apply_to_replay(
        &self,
        stage_limits: &mut BTreeMap<String, u64>,
        accounting: &mut filament_exact::ScalarLimits,
    ) {
        for (setting, bound) in &self.stage {
            // The reader bound is an argument, not a request member.
            if *setting != Setting::ReplayInputBytes {
                stage_limits.insert(setting.name().to_owned(), *bound);
            }
        }
        let mut limits = AccountingLimits(*accounting);
        for (setting, bound) in &self.accounting {
            limits.set_bound(*setting, *bound);
        }
        *accounting = limits.0;
    }

    fn apply(&self, caller: &mut CallerLimits) {
        for (setting, bound) in &self.stage {
            caller.set_bound(*setting, *bound);
        }
        for (setting, bound) in &self.accounting {
            caller.accounting.set_bound(*setting, *bound);
        }
    }

    fn resolved(&self) -> CallerLimits {
        let mut caller = CallerLimits::default();
        self.apply(&mut caller);
        caller
    }
}

impl SettingLimits for CommandLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let mut bounds: Vec<_> = self
            .resolved()
            .bounds()
            .into_iter()
            .filter(|(setting, _)| self.exposes(*setting))
            .collect();
        if self.operation == LimitedOperation::Monitor {
            let monitor = self.monitor_limits();
            bounds.extend([
                (Setting::MonitorMaxWindowValues, monitor.max_window_values),
                (
                    Setting::MonitorRetainedPositions,
                    monitor.retained_positions,
                ),
                (
                    Setting::MonitorRetainedIdentities,
                    monitor.retained_identities,
                ),
            ]);
            bounds.extend(self.preparation().bounds());
        }
        bounds
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        if !self.exposes(setting) {
            return false;
        }
        self.stage.insert(setting, bound);
        true
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const SOURCE_PACKAGE: &[&str] = &[
        "s1.input_bytes",
        "s1.tokens",
        "s1.nodes",
        "s1.work_units",
        "s3.nodes",
        "s3.input_bytes",
        "s3.work_units",
        "s3.rational_bits",
        "s3.decimal_scale",
        "s3.decimal_literal_scale",
        "s3.decimal_literal_scale_total",
        "environment.ancestor_steps",
        "environment.work_units",
        "model.declaration_records",
        "model.sequence_maximum",
        "model.derivation_facts",
        "model.effective_declarations",
        "model.dispatch_candidates",
        "model.work_units",
        "model.ancestor_steps",
        "model.family_steps",
        "observation.input_bytes",
        "observation.objects",
        "observation.values",
        "identity.input_bytes",
        "i2.input_bytes",
        "i2.nodes",
        "i2.edges",
        "i2.occurrences",
        "i2.diagnostics",
        "i2.work_units",
        "i2.native_join_storage",
        "dependency.libraries",
        "dependency.import_edges",
        "dependency.source_bytes",
        "intake.input_bytes",
    ];

    fn names(limits: &CommandLimits) -> Vec<&'static str> {
        let mut names: Vec<_> = limits
            .bounds()
            .into_iter()
            .map(|(setting, _)| setting.name())
            .collect();
        names.sort_unstable();
        names
    }

    fn expected(extra: &[&'static str]) -> Vec<&'static str> {
        let mut names: Vec<_> = SOURCE_PACKAGE.iter().chain(extra).copied().collect();
        names.sort_unstable();
        names
    }

    fn monitor() -> CommandLimits {
        CommandLimits::monitor(MonitorPreparationLimits {
            work: 16_777_216,
            storage: 268_435_456,
        })
    }

    /// TC-721 step 11, for the settings exposed today: each operation exposes exactly its settings.
    #[trace("TC-721")]
    #[test]
    fn each_operation_exposes_exactly_its_enumerated_settings() {
        assert_eq!(
            names(&CommandLimits::analyze()),
            expected(&["replay.input_bytes", "emit.output_bytes"])
        );
        assert_eq!(
            names(&monitor()),
            expected(&[
                "monitor.max_window_values",
                "monitor.retained_positions",
                "monitor.retained_identities",
                "monitor.preparation_work_units",
                "monitor.preparation_storage",
            ])
        );
        assert_eq!(
            names(&CommandLimits::replay()),
            expected(&[
                "admission.population_members",
                "admission.work_units",
                "admission.ancestor_steps",
                "library.definitions",
                "library.dependency_edges",
                "library.artifact_bytes",
                "library.single_artifact_bytes",
                "replay.input_bytes",
                "model_check.max_states",
                "model_check.max_transitions",
                "model_check.max_automaton_states",
            ])
        );
    }

    /// TC-721 step 11, for the settings exposed today: each accepted setting changes only its own bound.
    #[trace("TC-721")]
    #[test]
    fn each_accepted_setting_changes_only_its_bound() {
        for fresh in [CommandLimits::analyze(), monitor(), CommandLimits::replay()] {
            let before = fresh.bounds();
            for (setting, _) in &before {
                let mut limits = fresh.clone();
                assert!(limits.set_bound(*setting, 4242), "{setting}");
                for ((other, was), (_, now)) in before.iter().zip(limits.bounds()) {
                    let want = if other == setting { 4242 } else { *was };
                    assert_eq!(now, want, "{other} after setting {setting}");
                }
            }
        }
    }

    /// TC-721 step 11, for the settings exposed today: a recognized setting the operation does not execute is
    /// refused with its exact text and changes nothing.
    #[trace("TC-721")]
    #[test]
    fn an_unavailable_setting_is_refused_without_changing_a_bound() {
        let cases = [
            (
                CommandLimits::analyze(),
                Setting::RenderOutputBytes,
                "setting `render.output_bytes` is unavailable for the `analyze` operation",
            ),
            (
                CommandLimits::analyze(),
                Setting::MonitorPreparationStorage,
                "setting `monitor.preparation_storage` is unavailable for the `analyze` operation",
            ),
            (
                monitor(),
                Setting::ReplayInputBytes,
                "setting `replay.input_bytes` is unavailable for the `monitor` operation",
            ),
            (
                CommandLimits::replay(),
                Setting::ModelCheckWitnessSamples,
                "setting `model_check.witness_samples` is unavailable for the `replay` operation",
            ),
            (
                CommandLimits::replay(),
                Setting::MonitorRetainedPositions,
                "setting `monitor.retained_positions` is unavailable for the `replay` operation",
            ),
        ];
        for (mut limits, setting, text) in cases {
            let before = limits.bounds();
            let refused = limits.set(SettingName::Stage(setting), 7).unwrap_err();
            assert_eq!(refused.to_string(), text);
            assert!(!limits.set_bound(setting, 7));
            assert_eq!(limits.bounds(), before);
        }
    }

    /// TC-721 step 11, for the settings exposed today: every operation takes each accounting counter, and a
    /// counter changes only its own bound.
    #[trace("TC-721")]
    #[test]
    fn every_operation_takes_each_accounting_counter() {
        for fresh in [CommandLimits::analyze(), monitor(), CommandLimits::replay()] {
            for counter in AccountingSetting::ALL {
                let mut limits = fresh.clone();
                limits.set(SettingName::Accounting(counter), 99).unwrap();
                let mut accounting = AccountingLimits::default();
                let mut stage = BTreeMap::new();
                let mut scalar = accounting.0;
                limits.apply_to_replay(&mut stage, &mut scalar);
                accounting.set_bound(counter, 99);
                assert_eq!(scalar, accounting.0, "{counter}");
                assert!(stage.is_empty());
            }
        }
    }

    /// TC-721 step 11, for the settings exposed today: monitor sets its two preparation bounds independently,
    /// the caller supplies the starting values, and a reached bound names
    /// its setting.
    #[trace("TC-721")]
    #[test]
    fn monitor_preparation_work_and_storage_are_independent() {
        let mut limits = monitor();
        assert_eq!(
            limits.preparation(),
            MonitorPreparationLimits {
                work: 16_777_216,
                storage: 268_435_456
            }
        );
        let before = limits.monitor_limits();
        assert!(limits.set_bound(Setting::MonitorPreparationStorage, 125_611_543));
        assert_eq!(
            limits.preparation(),
            MonitorPreparationLimits {
                work: 16_777_216,
                storage: 125_611_543
            }
        );
        assert!(limits.set_bound(Setting::MonitorPreparationWorkUnits, 5));
        assert_eq!(
            limits.preparation(),
            MonitorPreparationLimits {
                work: 5,
                storage: 125_611_543
            }
        );
        let after = limits.monitor_limits();
        assert_eq!(
            (
                after.max_window_values,
                after.retained_positions,
                after.retained_identities
            ),
            (
                before.max_window_values,
                before.retained_positions,
                before.retained_identities
            )
        );
        let refusal = limits
            .preparation()
            .refusal(PreparationBound::Storage, 125_611_544);
        assert_eq!(
            refusal.to_string(),
            "monitor preparation reached `monitor.preparation_storage` (bound 125611543, needed 125611544); raise it with `--limit monitor.preparation_storage=<n>`"
        );
        assert_eq!(
            limits
                .preparation()
                .refusal(PreparationBound::Work, 6)
                .to_string(),
            "monitor preparation reached `monitor.preparation_work_units` (bound 5, needed 6); raise it with `--limit monitor.preparation_work_units=<n>`"
        );
    }

    /// TC-721 step 11, for the settings exposed today: `replay.input_bytes` reaches the reader argument and is
    /// never a `stage_limits` member; every other set setting replaces the
    /// request's entry.
    #[trace("TC-721")]
    #[test]
    fn replay_input_bytes_reaches_the_reader_and_not_stage_limits() {
        let mut limits = CommandLimits::replay();
        assert!(limits.set_bound(Setting::ReplayInputBytes, 1234));
        assert!(limits.set_bound(Setting::ModelCheckMaxStates, 9));
        assert_eq!(limits.replay_input_bytes(), 1234);
        let mut stage = BTreeMap::from([
            ("model_check.max_states".to_owned(), 1),
            ("s1.tokens".to_owned(), 2),
        ]);
        let mut accounting = AccountingLimits::default().0;
        limits.apply_to_replay(&mut stage, &mut accounting);
        assert_eq!(
            stage,
            BTreeMap::from([
                ("model_check.max_states".to_owned(), 9),
                ("s1.tokens".to_owned(), 2)
            ])
        );
        assert!(fsl_replay::StageLimits::default()
            .with_named("replay.input_bytes", 1)
            .is_err());
        assert!(fsl_replay::StageLimits::default()
            .with_named("monitor.retained_positions", 1)
            .is_err());
    }
}
