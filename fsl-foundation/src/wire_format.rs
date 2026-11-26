// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027: one catalog for native source, input, package and command formats.

use serde::{Serialize, Serializer};

macro_rules! formats {
    ($( $(#[$doc:meta])* $variant:ident => $name:literal ),+ $(,)?) => {
        /// Exact native format selector; profiles and cost models are separate contracts.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum WireFormat { $( $(#[$doc])* $variant, )+ }

        impl WireFormat {
            /// All native formats named by this catalog, independent of a command's admission.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The format whose label is exactly `label`, if any.
            pub fn from_label(label: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|format| format.as_str() == label)
            }

            /// Stable byte spelling used by producers and consumers.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }
        }
    };
}

formats! {
    /// File-driven native execution request.
    RunRequest => "native-run/1",
    /// Source-only native compilation request.
    CompileRequest => "native-compile/1",
    /// FR-620: an analyze request over every behaviour of a subject.
    AnalyzeRequest => "native-analyze/1",
    /// FR-644: a monitor request over a stream of observation records.
    MonitorRequest => "native-monitor/1",
    /// FR-862: a replay request in FSpec FR-336's replay request mapping.
    ReplayRequest => "native-replay/1",
    /// Native clause-run result or command-error envelope.
    RunResult => "native-run-result/2",
    /// Source-aware rule-model authoring profile.
    RuleModel => "native-rule-model/1",
    /// Rule-model authoring profile with exact IR rational scalar roles.
    RuleModelV2 => "native-rule-model/2",
    /// FR-100: the spine `run` outcome document for a `1-draft` program's
    /// named function call.
    SpineRunResult => "spine-run-result/1",
    /// FSpec FR-301: the `filament-outcome/1` outcome document.
    Outcome => "filament-outcome/1",
    /// Item stream of a native run.
    RunItems => "native-run-items/1",
    /// A model unit accepted by `fsl run`.
    ModelUnit => "native-model-unit/1",
    /// A monitor observation trace.
    MonitorTrace => "filament.monitor-trace/1",
    /// One monitor event record.
    MonitorEvent => "filament.monitor-event/1",
    /// A monitor summary record.
    MonitorSummary => "filament.monitor-summary/1",
    /// A monitor run record.
    MonitorRun => "filament.monitor-run/1",
    /// A saved monitor session state.
    MonitorState => "filament.monitor-state/1",
    /// A protocol recording.
    ProtocolRecording => "filament.protocol-recording/1",
    /// A trace population input of a finite hyper item.
    TracePopulation => "filament.trace-population/1",
    /// A progress ledger.
    ProgressLedger => "filament.progress-ledger/1",
    /// Checked-handoff selection document.
    HandoffSelection => "filament.checked-handoff.selection/1",
    /// Checked-handoff inventory document.
    HandoffInventory => "filament.checked-handoff.inventory/1",
    /// Checked-handoff mutations document.
    HandoffMutations => "filament.checked-handoff.mutations/1",
    /// Checked-handoff dependencies document.
    HandoffDependencies => "filament.checked-handoff.dependencies/1",
    /// Checked-handoff equality document.
    HandoffEquality => "filament.checked-handoff.equality/1",
    /// Checked-handoff identities document.
    HandoffIdentities => "filament.checked-handoff.identities/7",
}

impl core::fmt::Display for WireFormat {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for WireFormat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::WireFormat;
    use ix_trace_rs::trace;

    /// The label set is generated from the type: every variant of `ALL`
    /// has a distinct `name/version` label that parses back to itself.
    #[test]
    #[trace("FR-027")]
    fn every_format_label_is_unique_versioned_and_round_trips() {
        let mut seen = std::collections::BTreeSet::new();
        for format in WireFormat::ALL {
            let label = format.as_str();
            let (name, version) = label.rsplit_once('/').expect("label has a version");
            assert!(!name.is_empty() && version.bytes().all(|b| b.is_ascii_digit()));
            assert!(!version.is_empty(), "{label}");
            assert!(seen.insert(label), "duplicate label {label}");
            assert_eq!(WireFormat::from_label(label), Some(*format));
        }
        assert_eq!(WireFormat::from_label("native-run/9"), None);
    }
}
