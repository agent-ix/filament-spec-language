// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 Behavior 13, 15 and 16: the limits a run executes under, from its
//! request and the caller's overrides, and the one table of the overrides
//! each run route applies. A route applies a caller value or refuses it by
//! name.

use std::fmt;

use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply};
use fsl_foundation::{Setting, SettingLimits, SettingName};
use fsl_replay::spine::{RequestClocks, SpineLimits};
use fsl_replay::{AccountingLimits, CallerLimits};
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::implements;

use super::{CommandLimits, RequestOverrides, Result, RunCause};

/// A run route, in its source or package form. [`RunRoute::reads`] is the one
/// statement of the override members each one applies.
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
    /// A `call` run from source.
    Call,
    /// A `call` run over a selected package.
    PackageCall,
    /// A `clause` run from source.
    Clause,
    /// A `clause` run over a selected package.
    PackageClause,
    /// A `tl_lowering` run from source.
    TlLowering,
    /// A `tl_lowering` run over a selected package.
    PackageTlLowering,
    /// An `items` run, before its selections are read.
    Items,
    /// A `value_invariants` item selection.
    ValueInvariants,
    /// A `verification_plan` run.
    VerificationPlan,
    /// A compile.
    Compile,
    /// A compile of a model unit, which reads its own declarations only.
    ModelUnitCompile,
    /// An export from source.
    Export,
    /// An export over a selected package.
    PackageExport,
    /// An `export_evidence` run.
    ExportEvidence,
    /// A `version` run.
    Version,
    /// A `check` run.
    Check,
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
            Self::PackageCall => "package call",
            Self::Clause => "clause",
            Self::PackageClause => "package clause",
            Self::TlLowering => "tl_lowering",
            Self::PackageTlLowering => "package tl_lowering",
            Self::Items => "items",
            Self::ValueInvariants => "value_invariants",
            Self::VerificationPlan => "verification_plan",
            Self::Compile => "compile",
            Self::ModelUnitCompile => "model unit compile",
            Self::Export => "export",
            Self::PackageExport => "package export",
            Self::ExportEvidence => "export_evidence",
            Self::Version => "version",
            Self::Check => "check",
        })
    }
}

/// The part of a run that holds a stage or observation setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    /// The S1, S3, assembly and source dependency settings of a compile.
    Compile,
    /// `s3.nodes`, which a package run also reads for its clause.
    CheckedNodes,
    /// Model normalization.
    Model,
    /// The v2 package reader, `i2.*`.
    Imports,
    /// `emit.output_bytes`.
    Emit,
    /// The progress ledger.
    Progress,
    /// The `export.*` settings.
    Export,
    /// Observation admission.
    Observation,
    /// `expansion.max_candidates`.
    Expansion,
}

/// The family that holds `setting`, or `None` for a setting no run route
/// holds: every route refuses it.
fn family(setting: Setting) -> Option<Family> {
    match setting {
        Setting::S3Nodes => return Some(Family::CheckedNodes),
        Setting::EmitOutputBytes => return Some(Family::Emit),
        Setting::ExpansionMaxCandidates => return Some(Family::Expansion),
        _ => {}
    }
    if ObservationLimits::default().set_bound(setting, 0) {
        return Some(Family::Observation);
    }
    // Every member of the spine limits is named, so a new one is placed here.
    let SpineLimits {
        source: _,
        mut model,
        checking: _,
        dependencies: _,
        assembly: _,
        mut imports,
        mut progress,
        package: _,
        mut export,
    } = SpineLimits::default();
    if model.set_bound(setting, 0) {
        Some(Family::Model)
    } else if imports.set_bound(setting, 0) {
        Some(Family::Imports)
    } else if progress.set_bound(setting, 0) {
        Some(Family::Progress)
    } else if export.set_bound(setting, 0) {
        Some(Family::Export)
    } else {
        SpineLimits::default()
            .set_bound(setting, 0)
            .then_some(Family::Compile)
    }
}

/// The override members a route applies.
#[derive(Clone, Copy, Debug)]
struct Reads {
    accounting: bool,
    clocks: bool,
    output: bool,
    /// The setting families the route applies, or `None` where the
    /// operation's own setting table decides ([`command_limits`]).
    settings: Option<&'static [Family]>,
}

const NOTHING: Reads = Reads {
    accounting: false,
    clocks: false,
    output: false,
    settings: Some(&[]),
};

/// What a run that compiles one unit and evaluates it applies.
macro_rules! compiling {
    ($($more:ident),*) => {
        Some(&[
            Family::Compile,
            Family::CheckedNodes,
            Family::Model,
            Family::Imports,
            Family::Emit,
            $(Family::$more),*
        ])
    };
}

impl RunRoute {
    fn reads(self) -> Reads {
        let evaluates = |settings| Reads {
            accounting: true,
            settings,
            ..NOTHING
        };
        let clocked = |settings| Reads {
            clocks: true,
            ..evaluates(settings)
        };
        match self {
            Self::Synthesis | Self::PlanSynthesis => evaluates(compiling!(Observation)),
            Self::ValueSynthesis => evaluates(compiling!()),
            Self::PackageSynthesis => evaluates(Some(&[Family::Imports])),
            Self::ModelBasedTest => evaluates(compiling!(Observation, Expansion)),
            Self::RefinementTransfer
            | Self::Call
            | Self::TlLowering
            | Self::ValueInvariants => clocked(compiling!(Observation)),
            Self::Clause | Self::Items => clocked(compiling!(Observation, Progress)),
            Self::VerificationPlan => clocked(compiling!(Observation, Expansion)),
            Self::PackageCall => evaluates(Some(&[
                Family::Model,
                Family::Imports,
                Family::Observation,
            ])),
            Self::PackageClause => evaluates(Some(&[
                Family::CheckedNodes,
                Family::Model,
                Family::Imports,
                Family::Observation,
                Family::Progress,
            ])),
            Self::PackageTlLowering => evaluates(Some(&[
                Family::CheckedNodes,
                Family::Model,
                Family::Imports,
                Family::Observation,
            ])),
            Self::Monitor => clocked(None),
            Self::Replay => evaluates(None),
            Self::Compile => Reads {
                clocks: true,
                output: true,
                settings: compiling!(),
                ..NOTHING
            },
            Self::ModelUnitCompile => Reads {
                output: true,
                ..NOTHING
            },
            Self::Export => Reads {
                output: true,
                settings: compiling!(Export),
                ..NOTHING
            },
            Self::PackageExport => Reads {
                output: true,
                settings: Some(&[Family::Imports, Family::Export]),
                ..NOTHING
            },
            Self::ExportEvidence | Self::Version | Self::Check => NOTHING,
        }
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

/// The `emit.output_bytes` bound the caller named.
pub(super) fn stated_emit(overrides: &RequestOverrides<'_>) -> Option<u64> {
    overrides.limits?.get(&Setting::EmitOutputBytes).copied()
}

/// Refuse a `clocks` request member on a route that reads none, so that the
/// request's own selection is never ignored.
///
/// # Errors
///
/// `invalid-request` naming `clocks`.
#[implements("FR-255-AC-35")]
pub(super) fn refuse_request_clocks(route: RunRoute, clocks: &RequestClocks) -> Result<()> {
    if *clocks == RequestClocks::default() {
        return Ok(());
    }
    Err(RunCause::ClauseRequest {
        member: "clocks".to_owned(),
        reason: format!("a {route} run reads no clocks selection; remove it"),
    })
}

/// Refuse the first override member `route` does not apply.
///
/// # Errors
///
/// `invalid-request` naming `route` and the member or setting.
#[implements(
    "FR-255-AC-33",
    "FR-255-AC-35",
    "FR-255-AC-36",
    "FR-255-AC-37",
    "FR-255-AC-38"
)]
pub(super) fn admit(route: RunRoute, overrides: &RequestOverrides<'_>) -> Result<()> {
    // Every override member is named, so a new one is decided here.
    let RequestOverrides {
        accounting,
        clocks,
        output,
        limits,
    } = *overrides;
    let reads = route.reads();
    if output.is_some() && !reads.output {
        return Err(unapplied(route, "output"));
    }
    if clocks.is_some() && !reads.clocks {
        return Err(unapplied(route, "clocks"));
    }
    if !reads.accounting {
        if let Some(setting) = accounting.into_iter().flatten().map(|(name, _)| name).next() {
            return Err(unapplied(route, setting.name()));
        }
    }
    if let Some(held) = reads.settings {
        if let Some(setting) = limits
            .into_iter()
            .flatten()
            .map(|(setting, _)| *setting)
            .find(|setting| !family(*setting).is_some_and(|family| held.contains(&family)))
        {
            return Err(unapplied(route, setting.name()));
        }
    }
    Ok(())
}

/// The limits a call's requirement items settle under: the published
/// analysis limits with each value the caller names.
pub(super) fn item_limits(overrides: &RequestOverrides<'_>) -> AnalyzeLimits {
    analyze_limits(
        AnalyzeLimits::published(DeadlineSupply::Unbounded),
        overrides,
    )
}

/// `base`, the limits requirement items and plan steps settle under, with each
/// value the caller names.
#[implements("FR-255-AC-32")]
pub(super) fn analyze_limits(
    mut base: AnalyzeLimits,
    overrides: &RequestOverrides<'_>,
) -> AnalyzeLimits {
    base.accounting = overrides.accounting_over(base.accounting);
    for (setting, bound) in overrides.limits.into_iter().flatten() {
        base.observation.set_bound(*setting, *bound);
        base.normalization.set_bound(*setting, *bound);
        match setting {
            Setting::EmitOutputBytes => base.emit = fsl_package::EmitLimits::new(*bound),
            Setting::ExpansionMaxCandidates => base.expansion.max_candidates = *bound,
            _ => {}
        }
    }
    base
}

/// The limits `route` runs under: each accounting counter the caller names
/// replaces the request's, and every other counter keeps the request's.
///
/// # Errors
///
/// As [`admit`]; and `invalid-request` when the request and the
/// caller both state the observation limits.
#[implements("FR-255-AC-32")]
pub(super) fn effective_limits(
    route: RunRoute,
    request: RequestLimits,
    overrides: &RequestOverrides<'_>,
) -> Result<CallerLimits> {
    admit(route, overrides)?;
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
    admit(route, overrides)?;
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
