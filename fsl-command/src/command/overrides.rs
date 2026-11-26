// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 Behavior 13 and 15: the limits a run executes under, from its
//! request and the caller's overrides, and the overrides a run route cannot
//! apply. A route applies a caller value or refuses it by name.

use std::fmt;

use fsl_foundation::{Setting, SettingLimits, SettingName};
use fsl_replay::spine::{ExportLimits, ProgressLimits, RequestClocks, SpineLimits};
use fsl_replay::{AccountingLimits, CallerLimits};
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::implements;

use super::{CommandLimits, RequestOverrides, Result, RunCause};

/// A run route that applies or refuses each caller override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunRoute {
    /// A source `synthesis` run, before its selected declaration is read.
    Synthesis,
    /// A source synthesis of a value declaration.
    ValueSynthesis,
    /// A source synthesis of a plan declaration.
    PlanSynthesis,
    /// A synthesis over a package, which compiles nothing.
    PackageSynthesis,
    /// A direct `model_based_test` member.
    ModelBasedTest,
    /// A `refinement_transfer` member.
    RefinementTransfer,
    /// A batch `native-monitor/1` run.
    Monitor,
    /// A `native-replay/1` run.
    Replay,
    /// A `call` run, from source or from a package.
    Call,
    /// A `clause` run, from source or from a package.
    Clause,
    /// A compile.
    Compile,
    /// An export.
    Export,
    /// Any other `run` request.
    Direct,
}

impl fmt::Display for RunRoute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Synthesis => "synthesis",
            Self::ValueSynthesis => "value synthesis",
            Self::PlanSynthesis => "plan synthesis",
            Self::PackageSynthesis => "package synthesis",
            Self::ModelBasedTest => "model_based_test",
            Self::RefinementTransfer => "refinement_transfer",
            Self::Monitor => "monitor",
            Self::Replay => "replay",
            Self::Call => "call",
            Self::Clause => "clause",
            Self::Compile => "compile",
            Self::Export => "export",
            Self::Direct => "direct",
        })
    }
}

/// The limits a request states for itself.
#[derive(Clone, Copy, Debug)]
pub(super) struct RequestLimits {
    /// The request's accounting limits, with the defaults of the counters it
    /// does not name.
    pub(super) accounting: filament_exact::ScalarLimits,
    /// The request's `observation_limits` member, when it states one.
    pub(super) observation: Option<ObservationLimits>,
}

impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            accounting: AccountingLimits::default().0,
            observation: None,
        }
    }
}

fn unapplied(route: RunRoute, member: &'static str) -> RunCause {
    RunCause::OverrideUnapplied { route, member }
}

/// Whether `setting` bounds observation admission.
fn observes(setting: Setting) -> bool {
    ObservationLimits::default().set_bound(setting, 0)
}

/// The settings the caller names, observation admission's excluded.
fn named_stage<'a>(overrides: &RequestOverrides<'a>) -> impl Iterator<Item = (Setting, u64)> + 'a {
    overrides
        .limits
        .into_iter()
        .flatten()
        .map(|(setting, bound)| (*setting, *bound))
        .filter(|(setting, _)| !observes(*setting))
}

/// The `emit.output_bytes` bound the caller named.
pub(super) fn stated_emit(overrides: &RequestOverrides<'_>) -> Option<u64> {
    overrides.limits?.get(&Setting::EmitOutputBytes).copied()
}

/// Refuse a setting that neither stage limits nor observation limits hold:
/// no run but a monitor or a replay, which decide in [`command_limits`], has
/// a place for it.
///
/// # Errors
///
/// `invalid-request` naming the setting.
pub(super) fn refuse_unheld(route: RunRoute, overrides: &RequestOverrides<'_>) -> Result<()> {
    match named_stage(overrides).find(|(setting, _)| !SpineLimits::default().set_bound(*setting, 0))
    {
        Some((setting, _)) => Err(unapplied(route, setting.name())),
        None => Ok(()),
    }
}

/// Whether a run that compiles one unit reads `setting`: the progress ledger
/// and the export bounds belong to other routes.
fn compiles_with(setting: Setting) -> bool {
    SpineLimits::default().set_bound(setting, 0)
        && !ProgressLimits::default().set_bound(setting, 0)
        && !ExportLimits::default().set_bound(setting, 0)
}

/// Refuse a `clocks` request member on a route that reads none, so that the
/// request's own selection is never ignored.
///
/// # Errors
///
/// `invalid-request` naming `clocks`.
pub(super) fn refuse_request_clocks(route: RunRoute, clocks: &RequestClocks) -> Result<()> {
    if *clocks == RequestClocks::default() {
        return Ok(());
    }
    Err(RunCause::ClauseRequest {
        member: "clocks".to_owned(),
        reason: format!("a {route} run reads no clocks selection; remove it"),
    })
}

/// Refuse the output override of a `call` or a `clause` run, which writes no
/// file.
///
/// # Errors
///
/// `invalid-request` naming the override.
pub(super) fn refuse_output(route: RunRoute, overrides: &RequestOverrides<'_>) -> Result<()> {
    match overrides.output {
        Some(_) => Err(unapplied(route, "output")),
        None => Ok(()),
    }
}

/// Refuse the first override `route` cannot apply. A monitor and a replay run
/// decide their stage settings in [`command_limits`].
///
/// # Errors
///
/// `invalid-request` naming the override.
#[implements("FR-255-AC-33")]
pub(super) fn refuse_unapplied(route: RunRoute, overrides: &RequestOverrides<'_>) -> Result<()> {
    refuse_output(route, overrides)?;
    if overrides.clocks.is_some()
        && !matches!(route, RunRoute::RefinementTransfer | RunRoute::Monitor)
    {
        return Err(unapplied(route, "clocks"));
    }
    if overrides.observation_limits().is_some()
        && matches!(route, RunRoute::ValueSynthesis | RunRoute::PackageSynthesis)
    {
        return Err(unapplied(route, "observation_limits"));
    }
    let refused = |setting: Setting| match route {
        RunRoute::PackageSynthesis => true,
        RunRoute::Monitor | RunRoute::Replay => false,
        RunRoute::Synthesis
        | RunRoute::ValueSynthesis
        | RunRoute::PlanSynthesis
        | RunRoute::ModelBasedTest
        | RunRoute::RefinementTransfer
        | RunRoute::Call
        | RunRoute::Clause
        | RunRoute::Compile
        | RunRoute::Export
        | RunRoute::Direct => !compiles_with(setting),
    };
    match named_stage(overrides).find(|(setting, _)| refused(*setting)) {
        Some((setting, _)) => Err(unapplied(route, setting.name())),
        None => Ok(()),
    }
}

/// The limits `route` runs under: each accounting counter the caller names
/// replaces the request's, and every other counter keeps the request's.
///
/// # Errors
///
/// As [`refuse_unapplied`]; and `invalid-request` when the request and the
/// caller both state the observation limits.
#[implements("FR-255-AC-32")]
pub(super) fn effective_limits(
    route: RunRoute,
    request: RequestLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<CallerLimits> {
    refuse_unapplied(route, overrides)?;
    let observation = overrides.observation_limits();
    if request.observation.is_some() && observation.is_some() {
        return Err(RunCause::ClauseRequest {
            member: "observation_limits".to_owned(),
            reason: "request member and an observation limit override were both supplied"
                .to_owned(),
        });
    }
    Ok(CallerLimits {
        spine: overrides.spine_limits(),
        observation: observation.or(request.observation).unwrap_or_default(),
        accounting: AccountingLimits(overrides.accounting_over(request.accounting)),
        ..CallerLimits::default()
    })
}

/// `base` with each setting the caller names, for a monitor or a replay run.
///
/// # Errors
///
/// `invalid-request` naming an override or a setting the operation cannot
/// apply.
#[implements("FR-255-AC-32", "FR-255-AC-33")]
pub(super) fn command_limits(
    route: RunRoute,
    mut base: CommandLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<CommandLimits> {
    refuse_unapplied(route, overrides)?;
    for (setting, bound) in overrides.limits.into_iter().flatten() {
        base.set(SettingName::Stage(*setting), *bound)
            .map_err(|_| unapplied(route, setting.name()))?;
    }
    for (setting, bound) in overrides.accounting.into_iter().flatten() {
        base.set(SettingName::Accounting(*setting), *bound)
            .map_err(|_| unapplied(route, setting.name()))?;
    }
    Ok(base)
}
