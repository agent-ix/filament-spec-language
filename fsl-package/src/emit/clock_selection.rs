// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640, FSpec FR-540: the refusal of a clock selection keyed by clock
//! name, and the clock kind each temporal profile reads.

use filament_contract_model::ClockConfiguration;
use fsl_semantics::check::TemporalProfile;

/// Why a request's clock selections, keyed by clock name, do not fit the
/// program's clock-bearing declarations (FR-640 "Behavior").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClockSelectionCause {
    /// A declaration needs a configuration and the selection has no member
    /// for its clock name.
    MissingClockSelection,
    /// A member names a clock no declaration taking its configuration from
    /// the selection writes.
    UnusedClockSelection,
    /// A member's `kind` is not the one a declaration's profile reads.
    ClockKindMismatch,
    /// A member is named `model-steps` or `model-time`, which bind a model
    /// subject's steps or time and are never a configurable clock role.
    ReservedClockName,
    /// A `timed` member's `unit` names no unit the program unit declares
    /// (FR-933).
    UnknownClockUnit,
    /// A `timed` member's `unit` is a unit of another dimension than `Time`
    /// (FR-933).
    ClockUnitDimension,
}

impl ClockSelectionCause {
    /// The cause tag the refusal's `details.cause` carries.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingClockSelection => "missing-clock-selection",
            Self::UnusedClockSelection => "unused-clock-selection",
            Self::ClockKindMismatch => "clock-kind-mismatch",
            Self::ReservedClockName => "reserved-clock-name",
            Self::UnknownClockUnit => "unknown-clock-unit",
            Self::ClockUnitDimension => "clock-unit-dimension",
        }
    }
}

/// The declaration a [`ClockSelectionRefusal`] is at fault for, when one
/// declaration is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockSelectionDeclaration {
    /// The declaration's authored name.
    pub name: String,
    /// The declaration's temporal profile.
    pub profile: TemporalProfile,
}

/// FR-640: a clock selection keyed by clock name that does not fit the
/// program. The message names the clock and the `clocks` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockSelectionRefusal {
    /// Why the selection does not fit.
    pub cause: ClockSelectionCause,
    /// The clock name at fault.
    pub clock: String,
    /// The one declaration at fault, for a missing or mismatched selection.
    pub declaration: Option<ClockSelectionDeclaration>,
    /// The kind of the supplied configuration, for a mismatch.
    pub supplied: Option<&'static str>,
    /// The configured unit, for an unknown or non-time unit.
    pub unit: Option<String>,
}

impl std::fmt::Display for ClockSelectionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let clock = &self.clock;
        let declaration = self.declaration.as_ref();
        let name = declaration.map_or("a temporal declaration", |at| at.name.as_str());
        let profile = declaration.map_or("its profile", |at| at.profile.identity());
        match self.cause {
            ClockSelectionCause::MissingClockSelection => write!(
                f,
                "clock \"{clock}\" used by {name} under {profile} has no configuration; \
                 add \"{clock}\" to the request's clocks member"
            ),
            ClockSelectionCause::UnusedClockSelection => write!(
                f,
                "the clocks member \"{clock}\" names a clock no temporal declaration uses"
            ),
            ClockSelectionCause::ClockKindMismatch => {
                let expected = declaration
                    .and_then(|at| profile_clock_kind(at.profile))
                    .unwrap_or("no");
                let supplied = self.supplied.unwrap_or("different");
                write!(
                    f,
                    "clock \"{clock}\" is a {supplied} configuration but {name} uses \
                     {profile}, which needs {expected}"
                )
            }
            ClockSelectionCause::UnknownClockUnit => write!(
                f,
                "clock \"{clock}\" used by {name} under {profile} names the unit \"{}\", \
                 which the program unit does not declare",
                self.unit.as_deref().unwrap_or("")
            ),
            ClockSelectionCause::ClockUnitDimension => write!(
                f,
                "clock \"{clock}\" used by {name} under {profile} names the unit \"{}\", \
                 which is not a unit of the dimension Time",
                self.unit.as_deref().unwrap_or("")
            ),
            ClockSelectionCause::ReservedClockName => write!(
                f,
                "clock \"{clock}\" is a model binding name; a trace-subject clock role \
                 cannot be named model-steps or model-time, and a model subject's binding \
                 comes from the model"
            ),
        }
    }
}

/// The `kind` tag of `clock`'s `TemporalBinding` wire form.
pub fn clock_kind(clock: &ClockConfiguration) -> &'static str {
    match clock {
        ClockConfiguration::EventPosition { .. } => "event_position",
        ClockConfiguration::FixedSample { .. } => "fixed_sample",
        ClockConfiguration::TimestampedEvent { .. } => "timestamped_event",
        ClockConfiguration::Timed { .. } => "timed",
    }
}

/// The `kind` of the clock configuration `profile` reads over a trace
/// subject (FR-640 "Inputs"); `None` for infinite-trace, which reads the
/// trace's own positions.
pub const fn profile_clock_kind(profile: TemporalProfile) -> Option<&'static str> {
    match profile {
        TemporalProfile::EventPositionFalseExtension => Some("event_position"),
        TemporalProfile::FixedSampleFalseExtension => Some("fixed_sample"),
        TemporalProfile::TimestampedEventFiniteWindow => Some("timestamped_event"),
        TemporalProfile::Timed => Some("timed"),
        TemporalProfile::InfiniteTrace => None,
    }
}

/// Whether `clock` is a model binding name (FR-233), which no clock
/// selection configures.
pub fn is_model_binding_name(clock: &str) -> bool {
    matches!(clock, "model-steps" | "model-time")
}
