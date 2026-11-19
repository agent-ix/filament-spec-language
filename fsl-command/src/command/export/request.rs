// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-817 "Request": the strict reader of a run request's `export`
//! member, and the admission refusals of FR-1015.

use super::super::{Result, RunCause};
use fsl_foundation::Setting;
use serde_json::{Map, Value};

/// The members of an `export` request (FSpec FR-817).
const MEMBERS: [&str; 8] = [
    "profile",
    "target_family",
    "obligations",
    "minimum_fidelity",
    "observer",
    "initial",
    "limits",
    "output",
];

/// The observers of FSpec FR-817.
const OBSERVERS: [&str; 4] = ["sany", "tlc", "apalache", "lean"];

/// The selected output-mapping profile (FSpec FR-297).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Target {
    /// `filament.output.tla2/v1`.
    Tla,
    /// `filament.output.lean4/v1`.
    Lean,
}

impl Target {
    /// The profile identity.
    pub(super) const fn profile(self) -> &'static str {
        match self {
            Self::Tla => "filament.output.tla2/v1",
            Self::Lean => "filament.output.lean4/v1",
        }
    }

    /// The target family a request's `target_family` names.
    pub(super) const fn family(self) -> &'static str {
        match self {
            Self::Tla => "tla",
            Self::Lean => "lean",
        }
    }

    fn of_profile(profile: &str) -> Option<Self> {
        [Self::Tla, Self::Lean]
            .into_iter()
            .find(|target| target.profile() == profile)
    }
}

/// FSpec FR-811's `minimum_fidelity`, weakest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Floor {
    /// Nothing fails the floor.
    Any,
    /// `unrepresented` and `refused` records fail it.
    Conditional,
    /// Every record that is not `preserved` fails it.
    Preserved,
}

/// The three export limits, each as the request or the caller gave it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Limits {
    pub(super) output_bytes: Option<u64>,
    pub(super) nodes: Option<u64>,
    pub(super) work_units: Option<u64>,
}

/// The default of `export.output_bytes`: `emit.output_bytes`'s (FR-255).
pub(super) const DEFAULT_OUTPUT_BYTES: u64 = fsl_replay::spine::EMIT_OUTPUT_BYTES_DEFAULT;
/// The default of `export.nodes`: `s3.nodes`'s (FR-255).
pub(super) const DEFAULT_NODES: u64 = 100_000;
/// The default of `export.work_units`: `s3.work_units`'s (FR-255).
pub(super) const DEFAULT_WORK_UNITS: u64 = 16_777_216;

/// The effective value of each export limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EffectiveLimits {
    pub(super) output_bytes: u64,
    pub(super) nodes: u64,
    pub(super) work_units: u64,
}

impl Limits {
    pub(super) fn effective(self) -> EffectiveLimits {
        EffectiveLimits {
            output_bytes: self.output_bytes.unwrap_or(DEFAULT_OUTPUT_BYTES),
            nodes: self.nodes.unwrap_or(DEFAULT_NODES),
            work_units: self.work_units.unwrap_or(DEFAULT_WORK_UNITS),
        }
    }
}

/// One export request, read strictly.
#[derive(Debug)]
pub(super) struct Request {
    pub(super) target: Target,
    pub(super) obligations: Vec<String>,
    pub(super) floor: Floor,
    /// The requested observer (FSpec FR-817); this build runs none.
    pub(super) observer: Option<String>,
    pub(super) initial: Option<String>,
    pub(super) limits: Limits,
    pub(super) output: Option<String>,
}

/// A member the reader refuses, with its FR-805 `cause`.
fn refuse(pointer: &str, cause: &'static str, reason: impl Into<String>) -> RunCause {
    RunCause::ExportRequest {
        path: format!("/request/export{pointer}"),
        cause,
        reason: reason.into(),
    }
}

/// A `limits` entry the reader refuses: `invalid-request` naming the entry.
fn refuse_limit(pointer: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: format!("/request/export{pointer}"),
        reason: reason.into(),
    }
}

fn text<'a>(object: &'a Map<String, Value>, name: &str) -> Result<Option<&'a str>> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text)),
        Some(_) => Err(refuse(
            &format!("/{name}"),
            "wrong-value-kind",
            "expected a string",
        )),
    }
}

/// Read `member`, the request's `export` value, merging the caller's
/// `overrides` (the driver's `--limit`) into its limits.
///
/// # Errors
///
/// `invalid-request` at the member's pointer for a malformed member, an
/// unknown profile or limit name, a family mismatch, a duplicate obligation,
/// a limit given both in the member and by the caller.
pub(super) fn read(member: &Value, given: &fsl_replay::spine::ExportLimits) -> Result<Request> {
    let Some(object) = member.as_object() else {
        return Err(refuse("", "wrong-value-kind", "expected an object"));
    };
    if let Some(unknown) = object.keys().find(|key| !MEMBERS.contains(&key.as_str())) {
        return Err(refuse(
            &format!("/{unknown}"),
            "unknown-member",
            "unknown export member",
        ));
    }
    let profile = text(object, "profile")?
        .ok_or_else(|| refuse("/profile", "missing-member", "the profile is required"))?;
    let target = Target::of_profile(profile).ok_or_else(|| {
        refuse(
            "/profile",
            "invalid-value",
            format!("unknown output-mapping profile `{profile}`"),
        )
    })?;
    if let Some(family) = text(object, "target_family")? {
        if family != target.family() {
            return Err(refuse(
                "/target_family",
                "invalid-value",
                format!(
                    "target family `{family}` does not match profile `{profile}`, whose family is `{}`",
                    target.family()
                ),
            ));
        }
    }
    let obligations = obligations(object)?;
    let floor = match text(object, "minimum_fidelity")? {
        None | Some("conditional") => Floor::Conditional,
        Some("any") => Floor::Any,
        Some("preserved") => Floor::Preserved,
        Some(other) => {
            return Err(refuse(
                "/minimum_fidelity",
                "invalid-value",
                format!("`{other}` is none of preserved, conditional and any"),
            ))
        }
    };
    let observer = match text(object, "observer")? {
        None => None,
        Some(name) if OBSERVERS.contains(&name) => Some(name.to_owned()),
        Some(name) => {
            return Err(refuse(
                "/observer",
                "invalid-value",
                format!("`{name}` is none of sany, tlc, apalache and lean"),
            ))
        }
    };
    let limits = limits(object, given)?;
    let output = match object.get("output") {
        None | Some(Value::Null) => None,
        Some(Value::Object(record)) => Some(output_file(record)?),
        Some(_) => return Err(refuse("/output", "wrong-value-kind", "expected an object")),
    };
    Ok(Request {
        target,
        obligations,
        floor,
        observer,
        initial: text(object, "initial")?.map(str::to_owned),
        limits,
        output,
    })
}

fn obligations(object: &Map<String, Value>) -> Result<Vec<String>> {
    let names = match object.get("obligations") {
        None | Some(Value::Null) => {
            return Err(refuse(
                "/obligations",
                "missing-member",
                "the obligations member is required",
            ))
        }
        Some(Value::Array(names)) if names.is_empty() => {
            return Err(refuse(
                "/obligations",
                "invalid-value",
                "name at least one obligation",
            ))
        }
        Some(Value::Array(names)) => names,
        Some(_) => {
            return Err(refuse(
                "/obligations",
                "wrong-value-kind",
                "expected an array of obligation names",
            ))
        }
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::with_capacity(names.len());
    for (index, name) in names.iter().enumerate() {
        let Some(name) = name.as_str() else {
            return Err(refuse(
                &format!("/obligations/{index}"),
                "wrong-value-kind",
                "expected a string",
            ));
        };
        if !seen.insert(name) {
            return Err(refuse(
                &format!("/obligations/{index}"),
                "invalid-value",
                format!("obligation `{name}` is named twice"),
            ));
        }
        result.push(name.to_owned());
    }
    Ok(result)
}

fn output_file(record: &Map<String, Value>) -> Result<String> {
    if let Some(unknown) = record.keys().find(|key| key.as_str() != "file") {
        return Err(refuse(
            &format!("/output/{unknown}"),
            "unknown-member",
            "unknown output member",
        ));
    }
    match record.get("file") {
        Some(Value::String(file)) if !file.is_empty() => Ok(file.clone()),
        _ => Err(refuse(
            "/output/file",
            "invalid-value",
            "expected a non-empty path",
        )),
    }
}

/// The `limits` member by FR-255 setting name, merged with the caller's.
/// A setting present in both refuses naming `limits` and the setting.
fn limits(object: &Map<String, Value>, given: &fsl_replay::spine::ExportLimits) -> Result<Limits> {
    let mut limits = Limits::default();
    let entries = match object.get("limits") {
        None | Some(Value::Null) => None,
        Some(Value::Object(entries)) => Some(entries),
        Some(_) => return Err(refuse("/limits", "wrong-value-kind", "expected an object")),
    };
    for (name, value) in entries.into_iter().flatten() {
        let slot = match Setting::from_name(name) {
            Some(Setting::ExportOutputBytes) => &mut limits.output_bytes,
            Some(Setting::ExportNodes) => &mut limits.nodes,
            Some(Setting::ExportWorkUnits) => &mut limits.work_units,
            _ => {
                return Err(refuse_limit(
                    &format!("/limits/{name}"),
                    format!(
                        "`{name}` is no export limit; the settings are export.output_bytes, \
                         export.nodes and export.work_units"
                    ),
                ))
            }
        };
        let Some(bound) = value.as_u64() else {
            return Err(refuse_limit(
                &format!("/limits/{name}"),
                "expected an unsigned integer",
            ));
        };
        *slot = Some(bound);
    }
    for (setting, own, supplied) in [
        (
            Setting::ExportOutputBytes,
            &mut limits.output_bytes,
            given.output_bytes,
        ),
        (Setting::ExportNodes, &mut limits.nodes, given.nodes),
        (
            Setting::ExportWorkUnits,
            &mut limits.work_units,
            given.work_units,
        ),
    ] {
        match (own.is_some(), supplied) {
            (true, Some(_)) => {
                return Err(RunCause::ClauseRequest {
                    member: "limits".to_owned(),
                    reason: format!(
                        "{} is given both in export.limits and as a limit override \
                         (--limit); give it once",
                        setting.name()
                    ),
                })
            }
            (false, Some(bound)) => *own = Some(bound),
            (_, None) => {}
        }
    }
    Ok(limits)
}
