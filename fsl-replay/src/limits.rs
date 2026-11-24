// SPDX-License-Identifier: AGPL-3.0-or-later
//! The caller limits and FR-255's settings operation.
//!
//! [`CallerLimits`] holds every limits type of the stages FSL runs, each at
//! its published default. [`CallerLimits::from_operands`] is the settings
//! operation the driver CLI calls with its `--limit <name>=<value>`
//! operands, and a replay request's `stage_limits` entries are applied by the
//! same [`SettingLimits::set_bound`], so one name raises one limit at every
//! entry point.

use filament_exact::{LimitKind as Counter, ScalarLimits};
use fsl_eval::execution::PrepareLimits;
use fsl_eval::simulation::model::ExpansionLimits;
use fsl_eval::simulation::Limits as ExploreLimits;
use fsl_eval::temporal::evidence::TemporalEvidenceLimits;
use fsl_foundation::{AccountingSetting, Setting, SettingLimits, SettingName, UsageRefusal};
use fsl_semantics::library::bundle::PackageLimits as LibraryLimits;
use fsl_semantics::model::observation::ObservationLimits;
use fsl_semantics::model::population::PopulationAdmissionLimits;

use crate::bounds::ReplayLimits;
use crate::certificate::zone_certificate::CertificateCheckLimits;
use crate::model_check::limits::{ExactProbLimits, ModelCheckLimits};
use crate::spine::{default_accounting, SpineLimits, DEFAULT_WORK_UNITS};
use crate::statistical::{StatisticalLimits, TimedSampling};
use crate::SmtEncodingLimits;

/// The call's accounting limits (`filament.value.accounting/v1`): each of the
/// ten counters is an [`AccountingSetting`] named `accounting.<counter>` (FR-255's
/// accounting table). A newtype over the foreign [`ScalarLimits`]. The
/// default is FR-100's: `work_units` 1,000,000 and every other counter
/// `u64::MAX`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountingLimits(pub ScalarLimits);

impl Default for AccountingLimits {
    fn default() -> Self {
        Self(default_accounting(DEFAULT_WORK_UNITS))
    }
}

impl AccountingLimits {
    /// The accounting limits a replay's call runs under (FR-255
    /// Behavior 10): `base`, the request's own accounting limits, with each
    /// counter an entry names at the entry's bound.
    pub(crate) fn with_entries(
        base: ScalarLimits,
        entries: impl IntoIterator<Item = (AccountingSetting, u64)>,
    ) -> ScalarLimits {
        let mut limits = Self(base);
        for (setting, bound) in entries {
            limits.set_bound(setting, bound);
        }
        limits.0
    }

    /// The setting that raises `counter`'s bound (FR-255 Behavior 9).
    #[must_use]
    pub const fn setting_of(counter: Counter) -> AccountingSetting {
        match counter {
            Counter::IntegerBits => AccountingSetting::IntegerBits,
            Counter::DecimalDigits => AccountingSetting::DecimalDigits,
            Counter::ScaleExpansion => AccountingSetting::ScaleExpansion,
            Counter::TextInputBytes => AccountingSetting::TextInputBytes,
            Counter::TextScalars => AccountingSetting::TextScalars,
            Counter::NormalizedScalars => AccountingSetting::NormalizedScalars,
            Counter::UnitEdges => AccountingSetting::UnitEdges,
            Counter::ValueOccurrences => AccountingSetting::ValueOccurrences,
            Counter::WorkUnits => AccountingSetting::WorkUnits,
            Counter::ResultUnits => AccountingSetting::ResultUnits,
            Counter::LiveOccurrences => {
                panic!("LiveOccurrences is configured outside AccountingSetting")
            }
        }
    }

    /// The public setting name for an incomplete evaluation record, including
    /// the evaluation-session limit that is outside `ScalarLimits`.
    #[must_use]
    pub const fn setting_name(counter: Counter) -> &'static str {
        match counter {
            Counter::LiveOccurrences => "evaluation.live_occurrences",
            _ => Self::setting_of(counter).name(),
        }
    }

    /// Every counter with its setting and current bound, in the accounting
    /// table's order.
    #[must_use]
    pub fn bounds(&self) -> Vec<(AccountingSetting, u64)> {
        let limits = &self.0;
        vec![
            (AccountingSetting::IntegerBits, limits.integer_bits),
            (AccountingSetting::DecimalDigits, limits.decimal_digits),
            (AccountingSetting::ScaleExpansion, limits.scale_expansion),
            (AccountingSetting::TextInputBytes, limits.text_input_bytes),
            (AccountingSetting::TextScalars, limits.text_scalars),
            (
                AccountingSetting::NormalizedScalars,
                limits.normalized_scalars,
            ),
            (AccountingSetting::UnitEdges, limits.unit_edges),
            (
                AccountingSetting::ValueOccurrences,
                limits.value_occurrences,
            ),
            (AccountingSetting::WorkUnits, limits.work_units),
            (AccountingSetting::ResultUnits, limits.result_units),
        ]
    }

    /// Set the bound of the counter `setting` names and change nothing else.
    pub fn set_bound(&mut self, setting: AccountingSetting, bound: u64) {
        let limits = &mut self.0;
        let field = match setting {
            AccountingSetting::IntegerBits => &mut limits.integer_bits,
            AccountingSetting::DecimalDigits => &mut limits.decimal_digits,
            AccountingSetting::ScaleExpansion => &mut limits.scale_expansion,
            AccountingSetting::TextInputBytes => &mut limits.text_input_bytes,
            AccountingSetting::TextScalars => &mut limits.text_scalars,
            AccountingSetting::NormalizedScalars => &mut limits.normalized_scalars,
            AccountingSetting::UnitEdges => &mut limits.unit_edges,
            AccountingSetting::ValueOccurrences => &mut limits.value_occurrences,
            AccountingSetting::WorkUnits => &mut limits.work_units,
            AccountingSetting::ResultUnits => &mut limits.result_units,
        };
        *field = bound;
    }
}

/// FR-298: the output-byte budget of `render`, the FR-255 setting
/// `render.output_bytes`. The default is 16777216 bytes (FSpec FR-461
/// rule 2); the bound is used as given, including zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderLimits {
    /// Inclusive count of every byte `render` returns, newlines included.
    pub output_bytes: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            output_bytes: 16_777_216,
        }
    }
}

impl SettingLimits for RenderLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(
            Setting::RenderOutputBytes,
            u64::try_from(self.output_bytes).unwrap_or(u64::MAX),
        )]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        if setting != Setting::RenderOutputBytes {
            return false;
        }
        self.output_bytes = usize::try_from(bound).unwrap_or(usize::MAX);
        true
    }
}

/// FR-255 "Engine limit setting names" defaults of the `plan_search.*` rows
/// (FR-1011).
const PLAN_SEARCH_DEFAULTS: [u64; 5] =
    [10_000_000, 100_000_000, 16_777_216, 100_000_000, 1_048_576];

/// The default of `certificate_check.max_rank` (FR-255): the closed
/// profile's Count representability ceiling.
pub const DEFAULT_CERTIFICATE_MAX_RANK: u64 = 9_007_199_254_740_991;

/// The FR-255 engine-table limits accepted by the settings operation.
/// `expansion` is absent until a caller supplies its required
/// `expansion.max_candidates` bound; every other engine limit holds its
/// published default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineLimits {
    /// EN-1's search and product budgets.
    pub model_check: ModelCheckLimits,
    /// FR-120's caller-supplied candidate bound, absent by default.
    pub expansion: Option<ExpansionLimits>,
    /// EN-1's protocol-subject live-instance bound.
    pub max_live_instances: u64,
    /// EN-1's weak-memory TSO store-buffer bound.
    pub max_store_buffer: u64,
    /// EN-1's weak-memory RA live-message bound.
    pub max_messages: u64,
    /// `simulation.max_steps`: the trace positions of a simulation item.
    pub simulation_max_steps: u64,
    /// EN-5's `exact_prob.*` limits, including its document ceilings.
    pub exact_prob: ExactProbLimits,
    /// EN-4's `statistical.*` limits.
    pub statistical: StatisticalLimits,
    /// `statistical.max_sample_steps` (FR-254).
    pub statistical_max_sample_steps: u64,
    /// `zone_check.max_symbolic_states`, `.max_symbolic_transitions`,
    /// `.max_automaton_states` and `.max_zone_bytes` (FR-239), in that order.
    pub zone_check: [u64; 4],
    /// The certificate checkers' limits (FR-245): edges and bytes.
    pub certificate: CertificateCheckLimits,
    /// `certificate_check.max_rank` (FR-201).
    pub certificate_max_rank: u64,
    /// `schedule.max_demand_points` (FR-247).
    pub schedule_max_demand_points: u64,
    /// `hybrid.max_solver_steps` and `hybrid.max_solver_time` (FR-250).
    pub hybrid: [u64; 2],
    /// `smt_encoding.*` (FR-315).
    pub smt_encoding: SmtEncodingLimits,
    /// `smt.max_solver_steps` and `smt.max_solver_time` (FR-315).
    pub smt: [u64; 2],
    /// `plan_search.max_states`, `.max_work`, `.max_certificate_entries`,
    /// `.max_check_work` and `.max_state_key_bytes` (FR-1011), in that order.
    pub plan_search: [u64; 5],
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            model_check: ModelCheckLimits::default(),
            expansion: None,
            max_live_instances: 3,
            max_store_buffer: 4,
            max_messages: 4,
            simulation_max_steps: 16_777_216,
            exact_prob: ExactProbLimits::default(),
            statistical: StatisticalLimits::default(),
            statistical_max_sample_steps: TimedSampling::default().max_sample_steps,
            zone_check: [1 << 20, 1 << 24, 1 << 20, 1 << 30],
            certificate: CertificateCheckLimits::default(),
            certificate_max_rank: DEFAULT_CERTIFICATE_MAX_RANK,
            schedule_max_demand_points: 1_048_576,
            hybrid: [1_000_000, 60],
            smt_encoding: SmtEncodingLimits::default(),
            smt: [100_000_000, 60],
            plan_search: PLAN_SEARCH_DEFAULTS,
        }
    }
}

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn narrow(bound: u64) -> usize {
    usize::try_from(bound).unwrap_or(usize::MAX)
}

fn narrow32(bound: u64) -> u32 {
    u32::try_from(bound).unwrap_or(u32::MAX)
}

impl SettingLimits for EngineLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let model = &self.model_check;
        let exact = &self.exact_prob;
        let smt = &self.smt_encoding;
        let mut bounds = vec![
            (Setting::ModelCheckMaxStates, widen(model.limits.max_states)),
            (
                Setting::ModelCheckMaxTransitions,
                widen(model.limits.max_transitions),
            ),
            (
                Setting::ModelCheckMaxAutomatonStates,
                model.max_automaton_states,
            ),
            (Setting::ModelCheckWitnessSamples, model.witness_samples),
            (Setting::ModelCheckMaxWalkSteps, model.max_walk_steps),
            (Setting::ModelCheckMaxWitnessSet, model.max_witness_set),
            (
                Setting::ModelCheckMaxRelationTuples,
                model.max_relation_tuples,
            ),
            (Setting::ModelCheckMaxLiveInstances, self.max_live_instances),
            (Setting::ModelCheckMaxStoreBuffer, self.max_store_buffer),
            (Setting::ModelCheckMaxMessages, self.max_messages),
            (Setting::SimulationMaxSteps, self.simulation_max_steps),
            (
                Setting::ExactProbMaxStates,
                widen(exact.model.limits.max_states),
            ),
            (
                Setting::ExactProbMaxTransitions,
                widen(exact.model.limits.max_transitions),
            ),
            (
                Setting::ExactProbMaxAutomatonStates,
                exact.model.max_automaton_states,
            ),
            (Setting::ExactProbMaxIterations, exact.max_iterations),
            (
                Setting::ExactProbPrecisionBits,
                u64::from(exact.precision_bits),
            ),
            (
                Setting::ExactProbMaxPrecisionBits,
                u64::from(exact.max_precision_bits),
            ),
            (Setting::ExactProbMaxRationalBits, exact.max_rational_bits),
            (
                Setting::ExactProbMaxPolicyIterations,
                exact.max_policy_iterations,
            ),
            (Setting::ExactProbMaxWitnessPaths, exact.max_witness_paths),
            (Setting::StatisticalMaxSamples, self.statistical.max_samples),
            (Setting::StatisticalMaxDraws, self.statistical.max_draws),
            (
                Setting::StatisticalMaxCycleSteps,
                self.statistical.max_cycle_steps,
            ),
            (
                Setting::StatisticalMaxSampleSteps,
                self.statistical_max_sample_steps,
            ),
            (Setting::ZoneCheckMaxSymbolicStates, self.zone_check[0]),
            (Setting::ZoneCheckMaxSymbolicTransitions, self.zone_check[1]),
            (Setting::ZoneCheckMaxAutomatonStates, self.zone_check[2]),
            (Setting::ZoneCheckMaxZoneBytes, self.zone_check[3]),
            (
                Setting::CertificateCheckMaxCertificateEdges,
                self.certificate.max_certificate_edges,
            ),
            (
                Setting::CertificateCheckMaxCertificateBytes,
                self.certificate.max_certificate_bytes,
            ),
            (
                Setting::CertificateCheckMaxWorkingBytes,
                exact.document.storage_bytes,
            ),
            (Setting::CertificateCheckMaxRank, self.certificate_max_rank),
            (
                Setting::ScheduleMaxDemandPoints,
                self.schedule_max_demand_points,
            ),
            (Setting::HybridMaxSolverSteps, self.hybrid[0]),
            (Setting::HybridMaxSolverTime, self.hybrid[1]),
            (Setting::SmtEncodingMaxConstants, smt.max_constants),
            (Setting::SmtEncodingMaxApplications, smt.max_applications),
            (Setting::SmtEncodingMaxMonitorAtoms, smt.max_monitor_atoms),
            (Setting::SmtEncodingMaxMonitorStates, smt.max_monitor_states),
            (Setting::SmtEncodingMaxCandidates, smt.max_candidates),
            (Setting::SmtEncodingMaxScriptBytes, smt.max_script_bytes),
            (Setting::SmtMaxSolverSteps, self.smt[0]),
            (Setting::SmtMaxSolverTime, self.smt[1]),
            (Setting::PlanSearchMaxStates, self.plan_search[0]),
            (Setting::PlanSearchMaxWork, self.plan_search[1]),
            (
                Setting::PlanSearchMaxCertificateEntries,
                self.plan_search[2],
            ),
            (Setting::PlanSearchMaxCheckWork, self.plan_search[3]),
            (Setting::PlanSearchMaxStateKeyBytes, self.plan_search[4]),
        ];
        if let Some(expansion) = self.expansion {
            bounds.push((Setting::ExpansionMaxCandidates, expansion.max_candidates));
        }
        bounds
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let model = &mut self.model_check;
        let exact = &mut self.exact_prob;
        let smt = &mut self.smt_encoding;
        match setting {
            Setting::ModelCheckMaxStates => model.limits.max_states = narrow(bound),
            Setting::ModelCheckMaxTransitions => model.limits.max_transitions = narrow(bound),
            Setting::ModelCheckMaxAutomatonStates => model.max_automaton_states = bound,
            Setting::ModelCheckWitnessSamples => model.witness_samples = bound,
            Setting::ModelCheckMaxWalkSteps => model.max_walk_steps = bound,
            Setting::ModelCheckMaxWitnessSet => model.max_witness_set = bound,
            Setting::ModelCheckMaxRelationTuples => model.max_relation_tuples = bound,
            Setting::ModelCheckMaxLiveInstances => self.max_live_instances = bound,
            Setting::ModelCheckMaxStoreBuffer => self.max_store_buffer = bound,
            Setting::ModelCheckMaxMessages => self.max_messages = bound,
            Setting::ExpansionMaxCandidates => {
                self.expansion = Some(ExpansionLimits {
                    max_candidates: bound,
                });
            }
            Setting::SimulationMaxSteps => self.simulation_max_steps = bound,
            Setting::ExactProbMaxStates => exact.model.limits.max_states = narrow(bound),
            Setting::ExactProbMaxTransitions => exact.model.limits.max_transitions = narrow(bound),
            Setting::ExactProbMaxAutomatonStates => exact.model.max_automaton_states = bound,
            Setting::ExactProbMaxIterations => exact.max_iterations = bound,
            Setting::ExactProbPrecisionBits => exact.precision_bits = narrow32(bound),
            Setting::ExactProbMaxPrecisionBits => exact.max_precision_bits = narrow32(bound),
            Setting::ExactProbMaxRationalBits => exact.max_rational_bits = bound,
            Setting::ExactProbMaxPolicyIterations => exact.max_policy_iterations = bound,
            Setting::ExactProbMaxWitnessPaths => exact.max_witness_paths = bound,
            Setting::StatisticalMaxSamples => self.statistical.max_samples = bound,
            Setting::StatisticalMaxDraws => self.statistical.max_draws = bound,
            Setting::StatisticalMaxCycleSteps => self.statistical.max_cycle_steps = bound,
            Setting::StatisticalMaxSampleSteps => self.statistical_max_sample_steps = bound,
            Setting::ZoneCheckMaxSymbolicStates => self.zone_check[0] = bound,
            Setting::ZoneCheckMaxSymbolicTransitions => self.zone_check[1] = bound,
            Setting::ZoneCheckMaxAutomatonStates => self.zone_check[2] = bound,
            Setting::ZoneCheckMaxZoneBytes => self.zone_check[3] = bound,
            Setting::CertificateCheckMaxCertificateEdges => {
                self.certificate.max_certificate_edges = bound;
            }
            Setting::CertificateCheckMaxCertificateBytes => {
                self.certificate.max_certificate_bytes = bound;
                exact.document.bytes = bound;
            }
            Setting::CertificateCheckMaxWorkingBytes => exact.document.storage_bytes = bound,
            Setting::CertificateCheckMaxRank => self.certificate_max_rank = bound,
            Setting::ScheduleMaxDemandPoints => self.schedule_max_demand_points = bound,
            Setting::HybridMaxSolverSteps => self.hybrid[0] = bound,
            Setting::HybridMaxSolverTime => self.hybrid[1] = bound,
            Setting::SmtEncodingMaxConstants => smt.max_constants = bound,
            Setting::SmtEncodingMaxApplications => smt.max_applications = bound,
            Setting::SmtEncodingMaxMonitorAtoms => smt.max_monitor_atoms = bound,
            Setting::SmtEncodingMaxMonitorStates => smt.max_monitor_states = bound,
            Setting::SmtEncodingMaxCandidates => smt.max_candidates = bound,
            Setting::SmtEncodingMaxScriptBytes => smt.max_script_bytes = bound,
            Setting::SmtMaxSolverSteps => self.smt[0] = bound,
            Setting::SmtMaxSolverTime => self.smt[1] = bound,
            Setting::PlanSearchMaxStates => self.plan_search[0] = bound,
            Setting::PlanSearchMaxWork => self.plan_search[1] = bound,
            Setting::PlanSearchMaxCertificateEntries => self.plan_search[2] = bound,
            Setting::PlanSearchMaxCheckWork => self.plan_search[3] = bound,
            Setting::PlanSearchMaxStateKeyBytes => self.plan_search[4] = bound,
            _ => return false,
        }
        true
    }
}

/// Every limit type a caller can configure, one field per stage or engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallerLimits {
    /// S1, I1, S3, type-environment admission and the I2 read.
    pub spine: SpineLimits,
    /// Library resolution.
    pub library: LibraryLimits,
    /// Population admission.
    pub admission: PopulationAdmissionLimits,
    /// Observation admission.
    pub observation: ObservationLimits,
    /// Exploration.
    pub explore: ExploreLimits,
    /// The replay request and envelope readers.
    pub replay: ReplayLimits,
    /// Formatter output bytes, configured independently of replay stages.
    pub format: fsl_cst::format::FormatLimits,
    /// Rendered output bytes (FR-298).
    pub render: RenderLimits,
    /// Execution-backend prepare (FR-294).
    pub prepare: PrepareLimits,
    /// FR-255's analysis-engine limits.
    pub engines: EngineLimits,
    /// The thirteen counters of one temporal evidence call (FR-523).
    pub temporal_evidence: TemporalEvidenceLimits,
    /// The typed-view entry limit consumed by `fsl-inspect`.
    pub inspect_max_view_entries: u64,
    /// The call's accounting budgets (`filament.value.accounting/v1`).
    pub accounting: AccountingLimits,
}

impl Default for CallerLimits {
    fn default() -> Self {
        Self {
            spine: SpineLimits::default(),
            library: LibraryLimits::default(),
            admission: PopulationAdmissionLimits::default(),
            observation: ObservationLimits::default(),
            explore: ExploreLimits::default(),
            replay: ReplayLimits::default(),
            format: fsl_cst::format::FormatLimits::default(),
            render: RenderLimits::default(),
            prepare: PrepareLimits::default(),
            engines: EngineLimits::default(),
            temporal_evidence: TemporalEvidenceLimits::default(),
            inspect_max_view_entries: 10_000_000,
            accounting: AccountingLimits::default(),
        }
    }
}

impl CallerLimits {
    /// The settings operation (FR-255 Behavior 6): the limits with each
    /// `<name>=<value>` operand's setting at its value, and every other
    /// limit at its default.
    ///
    /// # Errors
    ///
    /// [`UsageRefusal`] naming the first operand that is not
    /// `<name>=<value>`, names no setting, holds a value that is not a
    /// non-negative decimal integer, or repeats a name. Nothing runs.
    pub fn from_operands<'a>(
        operands: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, UsageRefusal> {
        let mut limits = Self::default();
        for (setting, bound) in fsl_foundation::setting::parse_operands(operands)? {
            match setting {
                SettingName::Stage(setting) => {
                    if !limits.set_bound(setting, bound) {
                        return Err(UsageRefusal {
                            operand: format!("{}={bound}", setting.name()),
                            cause: fsl_foundation::UsageCause::UnavailableSetting,
                        });
                    }
                }
                SettingName::Accounting(setting) => limits.accounting.set_bound(setting, bound),
            }
        }
        Ok(limits)
    }

    /// The limits as the replay of a request with `stage_limits` runs them:
    /// `replay.input_bytes` from the library entry's own limit (a
    /// `stage_limits` entry for it is ignored, whichever way the map was
    /// built), and every other setting at the request's entry or its
    /// default. `accounting` is the request's own accounting limits; the
    /// result's [`Self::accounting`] is those with each counter a
    /// `stage_limits` entry names replaced (FR-255 Behavior 10).
    pub(crate) fn for_request(
        stage_limits: &crate::request::StageLimits,
        accounting: ScalarLimits,
        replay: ReplayLimits,
    ) -> Self {
        let mut limits = Self {
            replay,
            accounting: AccountingLimits(AccountingLimits::with_entries(
                accounting,
                stage_limits.accounting_entries(),
            )),
            ..Self::default()
        };
        for (setting, bound) in stage_limits.iter() {
            // Requests carry neither their own envelope limit nor the
            // formatter's limit: replay performs neither operation.
            if !matches!(
                setting,
                Setting::ReplayInputBytes | Setting::FormatOutputBytes
            ) {
                limits.set_bound(setting, bound);
            }
        }
        limits
    }

    fn parts(&self) -> [&dyn SettingLimits; 11] {
        [
            &self.spine,
            &self.library,
            &self.admission,
            &self.observation,
            &self.explore,
            &self.replay,
            &self.format,
            &self.render,
            &self.prepare,
            &self.engines,
            &self.temporal_evidence,
        ]
    }
}

impl SettingLimits for CallerLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let mut bounds: Vec<_> = self.parts().iter().flat_map(|part| part.bounds()).collect();
        bounds.push((
            Setting::InspectMaxViewEntries,
            self.inspect_max_view_entries,
        ));
        bounds
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let parts: [&mut dyn SettingLimits; 11] = [
            &mut self.spine,
            &mut self.library,
            &mut self.admission,
            &mut self.observation,
            &mut self.explore,
            &mut self.replay,
            &mut self.format,
            &mut self.render,
            &mut self.prepare,
            &mut self.engines,
            &mut self.temporal_evidence,
        ];
        if setting == Setting::InspectMaxViewEntries {
            self.inspect_max_view_entries = bound;
            return true;
        }
        parts.into_iter().any(|part| part.set_bound(setting, bound))
    }
}

#[cfg(test)]
mod tests {
    use fsl_foundation::UsageCause;
    use ix_trace_rs::trace;

    use super::*;
    use crate::request::StageLimits;

    const MONITOR_SETTINGS: [Setting; 5] = [
        Setting::MonitorMaxWindowValues,
        Setting::MonitorRetainedPositions,
        Setting::MonitorRetainedIdentities,
        Setting::MonitorPreparationWorkUnits,
        Setting::MonitorPreparationStorage,
    ];

    fn bound_of(limits: &CallerLimits, setting: Setting) -> u64 {
        limits
            .bounds()
            .into_iter()
            .find(|(reported, _)| *reported == setting)
            .map(|(_, bound)| bound)
            .unwrap_or_else(|| panic!("no limits type reports {setting}"))
    }

    /// The builders and the settings mapping are one: each builder sets the
    /// field its setting names.
    #[trace("TC-720", "FR-255-AC-3")]
    #[test]
    fn builders_set_the_field_their_setting_names() {
        let by_builder = |limits: CallerLimits| limits.bounds();
        let mut mapped = CallerLimits::default();
        mapped.set_bound(Setting::ObservationInputBytes, 11);
        mapped.set_bound(Setting::ObservationObjects, 12);
        mapped.set_bound(Setting::ObservationValues, 13);
        mapped.set_bound(Setting::LibraryDefinitions, 14);
        mapped.set_bound(Setting::LibraryDependencyEdges, 15);
        mapped.set_bound(Setting::LibraryArtifactBytes, 16);
        mapped.set_bound(Setting::LibrarySingleArtifactBytes, 17);
        mapped.set_bound(Setting::ReplayInputBytes, 18);
        mapped.set_bound(Setting::I2InputBytes, 19);
        mapped.set_bound(Setting::I2Nodes, 20);
        mapped.set_bound(Setting::I2Edges, 21);
        mapped.set_bound(Setting::I2Occurrences, 22);
        mapped.set_bound(Setting::I2Diagnostics, 23);
        mapped.set_bound(Setting::I2WorkUnits, 24);
        mapped.set_bound(Setting::I2NativeJoinStorage, 43);
        mapped.set_bound(Setting::ExploreStates, 25);
        mapped.set_bound(Setting::ExploreTransitions, 26);
        mapped.set_bound(Setting::IdentityInputBytes, 27);
        mapped.set_bound(Setting::S1InputBytes, 28);
        mapped.set_bound(Setting::S1Tokens, 29);
        mapped.set_bound(Setting::S1Nodes, 30);
        mapped.set_bound(Setting::S1WorkUnits, 31);
        mapped.set_bound(Setting::S3Nodes, 32);
        mapped.set_bound(Setting::S3InputBytes, 33);
        mapped.set_bound(Setting::S3WorkUnits, 34);
        mapped.set_bound(Setting::S3RationalBits, 37);
        mapped.set_bound(Setting::IntakeInputBytes, 35);
        mapped.set_bound(Setting::FormatOutputBytes, 36);
        mapped.set_bound(Setting::RenderOutputBytes, 38);
        mapped.set_bound(Setting::InspectMaxViewEntries, 41);
        mapped.set_bound(Setting::ProgressCoveredPositions, 42);
        mapped.set_bound(Setting::PrepareCompileWorkUnits, 39);
        mapped.set_bound(Setting::PrepareCodeBytes, 40);
        mapped.set_bound(Setting::EmitOutputBytes, 44);
        let built = CallerLimits {
            spine: SpineLimits {
                source: fsl_cst::Limits::default()
                    .with_source_bytes(28)
                    .with_tokens(29)
                    .with_nodes(30)
                    .with_work_units(31),
                checking: filament_semantic_value::checking::CheckingLimits::default()
                    .with_nodes(32)
                    .with_input_bytes(33)
                    .with_work_budget(34)
                    .with_rational_bits(37),
                assembly: fsl_semantics::check::AssemblyLimits {
                    identity: fsl_foundation::IdentityLimits::default().with_input_bytes(27),
                    ..fsl_semantics::check::AssemblyLimits::default()
                },
                imports: fsl_package::V2ReadLimits::fixture()
                    .with_artifact_bytes(19)
                    .with_nodes(20)
                    .with_edges(21)
                    .with_occurrences(22)
                    .with_diagnostics(23)
                    .with_work(24)
                    .with_storage(43),
                model: fsl_semantics::model::accounting::ModelNormalizationLimits {
                    intake: fsl_foundation::IntakeLimits::default().with_input_bytes(35),
                    ..SpineLimits::default().model
                },
                progress: crate::spine::ProgressLimits {
                    covered_positions: 42,
                },
                package: crate::spine::PackageLimits::default().with_output_bytes(44),
                ..SpineLimits::default()
            },
            library: LibraryLimits::default()
                .with_definitions(14)
                .with_dependency_edges(15)
                .with_artifact_bytes(16)
                .with_single_artifact_bytes(17),
            observation: ObservationLimits::default()
                .with_document_bytes(11)
                .with_objects_per_document(12)
                .with_values_per_document(13),
            explore: ExploreLimits::default()
                .with_max_states(25)
                .with_max_transitions(26),
            replay: ReplayLimits::default().with_input_bytes(18),
            format: fsl_cst::format::FormatLimits::default().with_output_bytes(36),
            render: RenderLimits { output_bytes: 38 },
            inspect_max_view_entries: 41,
            prepare: PrepareLimits {
                compile_work_units: 39,
                code_bytes: 40,
            },
            ..CallerLimits::default()
        };
        assert_eq!(by_builder(built), mapped.bounds());
    }

    /// FR-255-AC-4's entry points: the settings operation takes every
    /// setting of the table, and a replay request's `stage_limits` entry
    /// takes each setting but `replay.input_bytes` and `format.output_bytes`,
    /// which a request does not carry and which `for_request` leaves at their
    /// defaults however the entries were built. The stage-driven half, an input that reached
    /// a limit and passes once it is raised, is
    /// `a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request`.
    #[trace("TC-721", "FR-255-AC-4", "FR-255-AC-12", "FR-255-AC-13")]
    #[test]
    fn the_settings_operation_and_a_request_take_every_setting() {
        for setting in Setting::ALL {
            // The monitor settings belong to the monitor command's own limits
            // (`fsl_command::command::CommandLimits`), not to a replayed stage.
            if MONITOR_SETTINGS.contains(setting) {
                let operand = format!("{}=123456789", setting.name());
                let refusal = CallerLimits::from_operands([operand.as_str()]).unwrap_err();
                assert_eq!(refusal.cause, UsageCause::UnavailableSetting);
                assert_eq!(
                    refusal.to_string(),
                    format!("usage: UnavailableSetting in `--limit {operand}`")
                );
                continue;
            }
            let operand = format!("{}=123456789", setting.name());
            let limits = CallerLimits::from_operands([operand.as_str()]).unwrap();
            assert_eq!(bound_of(&limits, *setting), 123_456_789, "{setting}");
            let entries: StageLimits = [(*setting, 987_654_321)].into_iter().collect();
            let replay = ReplayLimits::default();
            let requested =
                CallerLimits::for_request(&entries, AccountingLimits::default().0, replay);
            let expected = if matches!(
                setting,
                Setting::ReplayInputBytes | Setting::FormatOutputBytes
            ) {
                bound_of(&CallerLimits::default(), *setting)
            } else {
                987_654_321
            };
            assert_eq!(bound_of(&requested, *setting), expected, "{setting}");
        }
    }

    /// FR-255-AC-20: `emit.output_bytes` changes only the output-byte bound of
    /// the compile lifecycle's E4 limits, through the library builder, the
    /// settings operation and a request's `stage_limits` entry; the outer
    /// configuration supplies 16777216; zero and `u64::MAX` are accepted.
    #[trace("TC-721", "FR-255-AC-20")]
    #[test]
    fn the_emit_output_setting_changes_only_the_output_byte_bound() {
        let defaults = CallerLimits::default();
        assert_eq!(bound_of(&defaults, Setting::EmitOutputBytes), 16_777_216);
        assert_eq!(defaults.spine.package.output_bytes, 16_777_216);
        for bound in [0, 1_234_567, u64::MAX] {
            let operand = format!("emit.output_bytes={bound}");
            let by_operand = CallerLimits::from_operands([operand.as_str()]).unwrap();
            let entries: StageLimits = [(Setting::EmitOutputBytes, bound)].into_iter().collect();
            let by_request = CallerLimits::for_request(
                &entries,
                AccountingLimits::default().0,
                ReplayLimits::default(),
            );
            let by_builder = CallerLimits {
                spine: SpineLimits {
                    package: crate::spine::PackageLimits::default().with_output_bytes(bound),
                    ..SpineLimits::default()
                },
                ..CallerLimits::default()
            };
            for limits in [by_operand, by_request, by_builder] {
                assert_eq!(limits.spine.package.emit().output_bytes, bound);
                assert_eq!(bound_of(&limits, Setting::EmitOutputBytes), bound);
            }
        }
    }

    /// FR-506-AC-7 and AC-8 (TC-1301 rows 14 and 15): `model.sequence_maximum`
    /// reaches model intake through the library builder, a request's
    /// `stage_limits` entry and the settings operand, each refusing a
    /// declared maximum 4 at the limit 3 and admitting 3; the limit takes
    /// `u32::MAX`, and the operand 4294967296 is a usage refusal that
    /// changes no limit.
    #[trace("FR-506-AC-7", "FR-506-AC-8", "TC-1301")]
    #[test]
    fn the_sequence_maximum_reaches_intake_through_every_entry_point() {
        use fsl_semantics::model::intake::PackageDocument;
        let package = |maximum: u64| {
            serde_json::json!({
                "types": [{"fields": [{"multiplicity": {
                    "lower": 0, "upper": maximum, "ordered": false, "unique": true,
                }}]}],
            })
            .to_string()
            .into_bytes()
        };
        let by_builder = fsl_foundation::IntakeLimits::default().with_sequence_maximum(3);
        let by_operand = CallerLimits::from_operands(["model.sequence_maximum=3"])
            .unwrap()
            .spine
            .model
            .intake;
        let entries: StageLimits = [(Setting::ModelSequenceMaximum, 3)].into_iter().collect();
        let by_request = CallerLimits::for_request(
            &entries,
            AccountingLimits::default().0,
            ReplayLimits::default(),
        )
        .spine
        .model
        .intake;
        for intake in [by_builder, by_operand, by_request] {
            assert_eq!(intake.sequence_maximum, 3);
            let refusal = PackageDocument::parse(&package(4), intake).unwrap_err();
            let fields = refusal.cause.catalog_fields().unwrap();
            assert_eq!(fields["bound"], "3");
            assert_eq!(fields["actual"], "4");
            assert_eq!(fields["setting"], "model.sequence_maximum");
            assert!(PackageDocument::parse(&package(3), intake).is_ok());
        }
        let widest = CallerLimits::from_operands(["model.sequence_maximum=4294967295"]).unwrap();
        assert_eq!(widest.spine.model.intake.sequence_maximum, u32::MAX);
        assert!(
            PackageDocument::parse(&package(u64::from(u32::MAX)), widest.spine.model.intake)
                .is_ok()
        );
        let refusal =
            CallerLimits::from_operands(["model.sequence_maximum=4294967296"]).unwrap_err();
        assert_eq!(refusal.operand, "model.sequence_maximum=4294967296");
        assert_eq!(refusal.cause, UsageCause::NotAnInteger);
    }

    /// FR-255-AC-5: a malformed operand is a usage refusal naming it.
    #[trace("TC-721", "FR-255-AC-5")]
    #[test]
    fn a_malformed_operand_is_a_usage_refusal_naming_it() {
        for (operands, operand, cause) in [
            (vec!["s9.nodes=1"], "s9.nodes=1", UsageCause::UnknownSetting),
            (
                vec!["s3.nodes=ten"],
                "s3.nodes=ten",
                UsageCause::NotAnInteger,
            ),
            (vec!["s3.nodes=-1"], "s3.nodes=-1", UsageCause::NotAnInteger),
            (
                vec!["s3.nodes=5", "s3.nodes=6"],
                "s3.nodes=6",
                UsageCause::Repeated,
            ),
        ] {
            let refusal = CallerLimits::from_operands(operands).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause);
        }
    }

    /// FR-523-AC-1: for each of the thirteen counters the setting
    /// `temporal_evidence.<counter>` is accepted by the settings operation,
    /// sets that counter's ceiling and no other, and the defaults are
    /// FR-255's.
    #[trace("TC-1318", "FR-523-AC-1")]
    #[test]
    fn each_temporal_evidence_setting_sets_its_counter_alone() {
        use fsl_eval::temporal::evidence::EvidenceCounter;
        let defaults = TemporalEvidenceLimits::default();
        assert_eq!(
            CallerLimits::from_operands([]).unwrap().temporal_evidence,
            defaults
        );
        assert_eq!(EvidenceCounter::ALL.len(), 13);
        for counter in EvidenceCounter::ALL {
            let operand = format!("temporal_evidence.{}=123456789", counter.name());
            let set = CallerLimits::from_operands([operand.as_str()])
                .unwrap()
                .temporal_evidence;
            for other in EvidenceCounter::ALL {
                let expected = if other == counter {
                    123_456_789
                } else {
                    defaults.ceiling(other)
                };
                assert_eq!(set.ceiling(other), expected, "{operand}: {}", other.name());
            }
            assert_eq!(
                counter.setting().name(),
                format!("temporal_evidence.{}", counter.name())
            );
        }
    }

    /// FR-523-AC-2: `temporal_evidence.formula_depth`,
    /// `temporal_evidence.depth`, a bare counter name and a value that is not
    /// a non-negative integer are each refused as unknown or non-numeric
    /// settings.
    #[trace("TC-1318", "FR-523-AC-2")]
    #[test]
    fn retired_and_malformed_temporal_evidence_settings_are_refused() {
        for (operand, cause) in [
            (
                "temporal_evidence.formula_depth=5",
                UsageCause::UnknownSetting,
            ),
            ("temporal_evidence.depth=5", UsageCause::UnknownSetting),
            ("temporal_evidence.json_depth=5", UsageCause::UnknownSetting),
            ("formula_nodes=5", UsageCause::UnknownSetting),
            ("positions=5", UsageCause::UnknownSetting),
            ("temporal_evidence.positions=ten", UsageCause::NotAnInteger),
            ("temporal_evidence.positions=-1", UsageCause::NotAnInteger),
            ("temporal_evidence.positions=1.5", UsageCause::NotAnInteger),
            ("temporal_evidence.positions=", UsageCause::NotAnInteger),
        ] {
            let refusal = CallerLimits::from_operands([operand]).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause, "{operand}");
        }
    }

    /// FR-255-AC-8: each accounting setting sets its counter alone, no
    /// operand gives the defaults, and `u64::MAX` is a bound. Partial evidence
    /// for NFR-009-AC-1: counters are set independently and zero stays
    /// effective; it does not assert that the retired evaluation-work
    /// identity is unexposed.
    #[trace("TC-921", "FR-255-AC-8", "NFR-009-AC-1")]
    #[test]
    fn each_accounting_setting_sets_its_counter_alone() {
        let defaults = CallerLimits::from_operands([]).unwrap().accounting;
        assert_eq!(defaults, AccountingLimits::default());
        for (setting, _) in defaults.bounds() {
            for bound in [0, 123_456_789] {
                let operand = format!("{}={bound}", setting.name());
                let set = CallerLimits::from_operands([operand.as_str()])
                    .unwrap()
                    .accounting;
                for ((other, expected), (_, actual)) in
                    defaults.bounds().into_iter().zip(set.bounds())
                {
                    let expected = if other == setting { bound } else { expected };
                    assert_eq!(actual, expected, "{operand}: {other:?}");
                }
            }
        }
        let max =
            CallerLimits::from_operands(["accounting.work_units=18446744073709551615"]).unwrap();
        assert_eq!(max.accounting.0.work_units, u64::MAX);
    }

    /// FR-255-AC-9: the grammar, duplicate and overflow rules of the stage
    /// settings hold for the accounting names.
    #[trace("TC-921", "FR-255-AC-9")]
    #[test]
    fn a_malformed_accounting_operand_is_a_usage_refusal_naming_it() {
        for (operands, operand, cause) in [
            (vec!["depth=1"], "depth=1", UsageCause::UnknownSetting),
            (
                vec!["work_units=1"],
                "work_units=1",
                UsageCause::UnknownSetting,
            ),
            (
                vec!["accounting=1"],
                "accounting=1",
                UsageCause::UnknownSetting,
            ),
            (
                vec!["accounting.work_units=ten"],
                "accounting.work_units=ten",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=-1"],
                "accounting.work_units=-1",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=18446744073709551616"],
                "accounting.work_units=18446744073709551616",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=5", "accounting.work_units=6"],
                "accounting.work_units=6",
                UsageCause::Repeated,
            ),
        ] {
            let refusal = CallerLimits::from_operands(operands).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause);
        }
    }

    /// FR-255-AC-3, FR-255-AC-4: the supplied-value ceilings of FR-277 are no
    /// setting-table rows, so the settings operation refuses them as it
    /// refuses any name that is no setting; they travel in the typed
    /// `limits.supplied` member.
    #[trace("TC-721", "FR-255-AC-3", "FR-255-AC-4")]
    #[test]
    fn the_supplied_ceilings_are_not_settings() {
        for name in [
            "supplied.admission_work_units",
            "supplied.conversion_work_units",
        ] {
            let operand = format!("{name}=5");
            let refusal = CallerLimits::from_operands([operand.as_str()]).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, UsageCause::UnknownSetting);
        }
    }

    /// FR-255-AC-10: a `stage_limits` counter entry replaces that counter of
    /// the request's accounting limits and no other.
    #[trace("TC-921", "FR-255-AC-10")]
    #[test]
    fn a_stage_limits_counter_replaces_only_that_accounting_counter() {
        let base = crate::spine::default_accounting(77);
        let effective = AccountingLimits::with_entries(base, [(AccountingSetting::WorkUnits, 5)]);
        assert_eq!(
            effective,
            ScalarLimits {
                work_units: 5,
                ..base
            }
        );
    }
}
