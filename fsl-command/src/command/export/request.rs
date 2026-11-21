// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-817 "Request": the strict reader of a run request's `export`
//! member, and the admission refusals of FR-1015.

use super::super::{Result, RunCause};
use fsl_analyze::analyze::document::{DocumentSelection, ProtocolSelection};
use fsl_foundation::Setting;
use serde_json::{Map, Value};

/// The members of an `export` request (FSpec FR-817).
const MEMBERS: [&str; 10] = [
    "profile",
    "target_family",
    "obligations",
    "minimum_fidelity",
    "observer",
    "initial",
    "limits",
    "output",
    "protocols",
    "universes",
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
    /// The protocol subjects named for clauses whose activation requires them.
    pub(super) protocols: Vec<ProtocolSelection>,
    /// The replicated-role universes supplied by the caller.
    pub(super) universes: Vec<DocumentSelection>,
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
    let protocols = protocol_inputs(object)?;
    let universes = match object.get("universes") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(entries)) => entries
            .iter()
            .enumerate()
            .map(|(index, value)| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    refuse(
                        &format!("/universes/{index}"),
                        "invalid-value",
                        format!("the universe is not an analyze document reference: {error}"),
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => {
            return Err(refuse(
                "/universes",
                "wrong-value-kind",
                "expected an array of universe bindings",
            ));
        }
    };
    Ok(Request {
        target,
        obligations,
        floor,
        observer,
        initial: text(object, "initial")?.map(str::to_owned),
        limits,
        output,
        protocols,
        universes,
    })
}

fn protocol_inputs(object: &Map<String, Value>) -> Result<Vec<ProtocolSelection>> {
    let entries = match object.get("protocols") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(entries)) => entries,
        Some(_) => {
            return Err(refuse(
                "/protocols",
                "wrong-value-kind",
                "expected an array of protocol bindings",
            ));
        }
    };
    let mut seen = std::collections::BTreeSet::new();
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let path = format!("/protocols/{index}");
            let Some(entry_object) = entry.as_object() else {
                return Err(refuse(
                    &path,
                    "wrong-value-kind",
                    "expected a protocol binding object",
                ));
            };
            if let Some(unknown) = entry_object
                .keys()
                .find(|key| !["protocol", "over", "roles", "capacities"].contains(&key.as_str()))
            {
                return Err(RunCause::ExportProtocolRequest {
                    path: format!("/request/export{path}"),
                    reason: format!("unknown protocol binding member `{unknown}`"),
                });
            }
            let selection: ProtocolSelection =
                serde_json::from_value(entry.clone()).map_err(|error| {
                    refuse(
                        &path,
                        "invalid-value",
                        format!("the protocol binding is not an analyze protocol subject: {error}"),
                    )
                })?;
            if selection.protocol.is_empty() {
                return Err(refuse(
                    &format!("{path}/protocol"),
                    "wrong-value-kind",
                    "expected a non-empty protocol name",
                ));
            }
            if !seen.insert(selection.protocol.clone()) {
                return Err(refuse(
                    &format!("{path}/protocol"),
                    "invalid-value",
                    "a protocol binding is named twice",
                ));
            }
            Ok(selection)
        })
        .collect()
}

pub(super) fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn member() -> Value {
        json!({
            "profile": "filament.output.tla2/v1",
            "obligations": ["T"],
            "protocols": [{
                "protocol": "Hand",
                "over": {"reference": {"population": "counters", "key": "<<99>>"}},
                "roles": {"a": {"type": "model-binding", "selection": "model/Counter"}},
                "capacities": {"Hand::ch": "2"}
            }],
            "universes": [{"authority": "agent-ix", "identity": "snapshots/counters"}]
        })
    }

    #[test]
    #[trace("FR-1015-AC-17")]
    fn protocol_and_universe_bindings_use_the_analyze_reader_shapes() {
        let request = read(&member(), &Default::default()).expect("the analyze subject shape");
        assert_eq!(request.protocols.len(), 1);
        assert_eq!(request.protocols[0].protocol, "Hand");
        assert_eq!(
            request.protocols[0].over.as_ref().unwrap().reference.key,
            "<<99>>"
        );
        assert_eq!(request.protocols[0].roles["a"].selection, "model/Counter");
        assert_eq!(request.protocols[0].capacities["Hand::ch"], "2");
        assert_eq!(request.universes.len(), 1);
        assert_eq!(request.universes[0].identity, "snapshots/counters");
    }

    #[test]
    #[trace("FR-1015-AC-17")]
    fn protocol_bindings_reject_unknown_members_like_analyze_reader() {
        let mut member = member();
        member["protocols"][0]["unexpected"] = json!(true);
        let error = read(&member, &Default::default()).expect_err("closed protocol binding");
        assert_eq!(error.code(), fsl_foundation::Code::InvalidRequest);
        let output = super::super::super::output::error(
            &super::super::super::RunError {
                cause: error,
                sources: crate::Sources::default(),
            },
            false,
        )
        .expect("serialize request failure");
        assert_eq!(output["code"], "invalid-request", "{output}");
        assert!(output.get("cause").is_none(), "{output}");
        assert_eq!(
            output["details"]["path"], "/request/export/protocols/0",
            "{output}"
        );
    }
}
