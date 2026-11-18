// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015: strict reader for an export selection embedded in a native run.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Caller ceilings for checked-package export.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportLimits {
    /// Maximum target text bytes.
    pub output_bytes: Option<u64>,
    /// Maximum checked nodes inspected.
    pub nodes: Option<u64>,
    /// Maximum mapper work units.
    pub work_units: Option<u64>,
}

/// A closed export request, before package-dependent validation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportRequest {
    /// The selected `filament.output.*` artifact profile.
    pub profile: String,
    /// Optional target family selection.
    pub target_family: Option<String>,
    /// Ordered obligations to export.
    pub obligations: Vec<String>,
    /// Minimum admitted fidelity, defaulting to `conditional`.
    #[serde(default = "default_fidelity")]
    pub minimum_fidelity: String,
    /// Optional observer binding.
    pub observer: Option<String>,
    /// Caller-raised output limits.
    #[serde(default)]
    pub limits: ExportLimits,
}

fn default_fidelity() -> String {
    "conditional".to_owned()
}

impl ExportRequest {
    /// Decode and validate request-local export fields.
    pub fn read(value: serde_json::Value) -> Result<Self, ExportRequestError> {
        let request: Self = serde_json::from_value(value).map_err(ExportRequestError::Malformed)?;
        if request.profile.is_empty() {
            return Err(ExportRequestError::InvalidValue("profile"));
        }
        let family = match request.profile.as_str() {
            "filament.output.tla2/v1" => "tla",
            "filament.output.lean4/v1" => "lean",
            _ => return Err(ExportRequestError::UnknownProfile),
        };
        if request
            .target_family
            .as_deref()
            .is_some_and(|target| target != family)
        {
            return Err(ExportRequestError::TargetFamilyMismatch);
        }
        if request.obligations.is_empty() || request.obligations.iter().any(String::is_empty) {
            return Err(ExportRequestError::InvalidValue("obligations"));
        }
        let mut names = BTreeSet::new();
        if request
            .obligations
            .iter()
            .any(|obligation| !names.insert(obligation))
        {
            return Err(ExportRequestError::InvalidValue("obligations"));
        }
        if !matches!(
            request.minimum_fidelity.as_str(),
            "any" | "conditional" | "preserved"
        ) {
            return Err(ExportRequestError::InvalidValue("minimum_fidelity"));
        }
        if request
            .observer
            .as_deref()
            .is_some_and(|observer| !matches!(observer, "sany" | "tlc" | "apalache" | "lean"))
        {
            return Err(ExportRequestError::InvalidValue("observer"));
        }
        if [
            request.limits.output_bytes,
            request.limits.nodes,
            request.limits.work_units,
        ]
        .into_iter()
        .flatten()
        .any(|limit| limit == 0)
        {
            return Err(ExportRequestError::InvalidValue("limits"));
        }
        Ok(request)
    }
}

/// One obligation's checked-package representation record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportRecord {
    /// Selected obligation, in request order.
    pub obligation: String,
    /// What the target text represents.
    pub disposition: ExportDisposition,
    /// Conditions on a conditional representation.
    pub conditions: Vec<String>,
    /// Located causes for unrepresented or refused constructs.
    pub causes: Vec<serde_json::Value>,
    /// Source regions in the generated text.
    pub regions: Vec<serde_json::Value>,
}

/// Fidelity classification of one exported obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportDisposition {
    /// Meaning is preserved by the target encoding.
    Preserved,
    /// Meaning is preserved under stated assumptions.
    Conditional,
    /// At least one source construct has no target encoding.
    Unrepresented,
    /// The mapper refused to interpret this obligation.
    Refused,
}

/// The origin and mapper version of the assembled target package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportPackage {
    /// Checked source package identity.
    pub source_package: String,
    /// Mapper name and version.
    pub generator: String,
}

/// The `filament.export-outcome/v1` wire shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExportOutcome {
    /// Fixed FR-817 identity.
    pub format: &'static str,
    /// Selected FSL mapping profile.
    pub mapping_profile_id: String,
    /// Target family (`tla` or `lean`).
    pub target_family: String,
    /// Generated target module or namespace.
    pub module: Option<String>,
    /// Outcome category.
    pub status: ExportStatus,
    /// One mapping record per requested obligation.
    pub records: Vec<ExportRecord>,
    /// Package identity and generator version, present when assembled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<ExportPackage>,
    /// Target source bytes decoded as UTF-8, present when assembled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_text: Option<String>,
    /// Deterministic mapping from target names to checked nodes and regions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_map: Option<Vec<serde_json::Value>>,
    /// Result of an optional target observer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observer: Option<serde_json::Value>,
    /// Limit or refusal cause, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<serde_json::Value>,
}

impl ExportOutcome {
    /// Build a status-bearing export outcome with FR-817's wire identity.
    pub fn new(
        profile: String,
        target_family: String,
        status: ExportStatus,
        records: Vec<ExportRecord>,
    ) -> Self {
        Self {
            format: "filament.export-outcome/v1",
            mapping_profile_id: profile,
            target_family,
            module: None,
            status,
            records,
            package: None,
            target_text: None,
            name_map: None,
            observer: None,
            cause: None,
        }
    }

    /// Serialize in the native run-value wire form.
    pub fn into_value(self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// Top-level export outcome status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStatus {
    /// The package and target text were produced.
    Assembled,
    /// The request was refused or did not meet its fidelity floor.
    Refused,
    /// An export limit stopped the mapper before package assembly.
    Incomplete,
}

impl ExportStatus {
    /// The FSpec FR-817 process exit assigned to this export result.
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Assembled => 0,
            Self::Refused => 21,
            Self::Incomplete => 22,
        }
    }
}

/// Why an export request member could not be admitted.
#[derive(Debug, thiserror::Error)]
pub enum ExportRequestError {
    /// The request did not match the closed JSON shape.
    #[error("invalid_runtime_input: malformed or unknown export request member: {0}")]
    Malformed(serde_json::Error),
    /// A request-local value is invalid.
    #[error("invalid_runtime_input: invalid value for {0}")]
    InvalidValue(&'static str),
    /// The request's profile is not one of the two output mapping profiles.
    #[error("invalid_runtime_input: unknown output mapping profile")]
    UnknownProfile,
    /// The requested family does not match the selected profile.
    #[error("invalid_runtime_input: target family does not match the profile")]
    TargetFamilyMismatch,
}

impl ExportRequestError {
    /// The request member whose value failed admission.
    pub const fn member(&self) -> &'static str {
        match self {
            Self::Malformed(_) => "export",
            Self::InvalidValue(member) => member,
            Self::UnknownProfile => "profile",
            Self::TargetFamilyMismatch => "target_family",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportDisposition, ExportOutcome, ExportRecord, ExportRequest, ExportStatus};
    use serde_json::json;

    #[test]
    fn export_status_maps_to_fspec_exit_codes() {
        assert_eq!(ExportStatus::Assembled.exit_code(), 0);
        assert_eq!(ExportStatus::Refused.exit_code(), 21);
        assert_eq!(ExportStatus::Incomplete.exit_code(), 22);
    }

    #[test]
    fn reads_export_fields_and_default_fidelity() {
        let request = ExportRequest::read(json!({
            "profile": "filament.output.tla2/v1",
            "obligations": ["Low"],
            "limits": {"output_bytes": 64}
        }))
        .unwrap();
        assert_eq!(request.minimum_fidelity, "conditional");
        assert_eq!(request.limits.output_bytes, Some(64));
    }

    #[test]
    fn rejects_unknown_duplicate_and_zero_limit_members() {
        assert!(
            ExportRequest::read(json!({
                "profile": "filament.output.tla2/v1", "obligations": ["Low"], "extra": true
            }))
            .is_err()
        );
        assert!(
            ExportRequest::read(json!({
                "profile": "filament.output.tla2/v1", "obligations": ["Low", "Low"]
            }))
            .is_err()
        );
        assert!(
            ExportRequest::read(json!({
                "profile": "filament.output.tla2/v1", "obligations": ["Low"],
                "limits": {"nodes": 0}
            }))
            .is_err()
        );
        assert!(
            ExportRequest::read(json!({
                "profile": "filament.output.lean4/v1", "target_family": "tla",
                "obligations": ["Low"]
            }))
            .is_err()
        );
    }

    #[test]
    fn names_profile_family_and_limit_admission_failures() {
        assert_eq!(
            ExportRequest::read(json!({
                "profile": "unknown",
                "obligations": ["Low"]
            }))
            .unwrap_err()
            .member(),
            "profile"
        );
        assert_eq!(
            ExportRequest::read(json!({
                "profile": "filament.output.lean4/v1",
                "target_family": "tla",
                "obligations": ["Low"]
            }))
            .unwrap_err()
            .member(),
            "target_family"
        );
        assert_eq!(
            ExportRequest::read(json!({
                "profile": "filament.output.tla2/v1",
                "obligations": ["Low"],
                "limits": {"nodes": 0}
            }))
            .unwrap_err()
            .member(),
            "limits"
        );
    }

    #[test]
    fn export_outcome_omits_target_bytes_until_assembled() {
        let outcome = ExportOutcome::new(
            "filament.output.tla2/v1".to_owned(),
            "tla".to_owned(),
            ExportStatus::Refused,
            vec![ExportRecord {
                obligation: "Low".to_owned(),
                disposition: ExportDisposition::Unrepresented,
                conditions: Vec::new(),
                causes: vec![serde_json::json!({"code": "target-unrepresentable"})],
                regions: Vec::new(),
            }],
        );
        let value = outcome.into_value().unwrap();
        assert_eq!(value["format"], "filament.export-outcome/v1");
        assert!(value.get("target_text").is_none());
        assert_eq!(value["records"][0]["disposition"], "unrepresented");
    }
}
