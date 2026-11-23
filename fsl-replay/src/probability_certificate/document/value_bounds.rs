// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1042's proof role: a `lower` or `upper` document that certifies one end
//! of a dyadic interval.
//!
//! An untimed base is wrapped in the closed `value-bounds/v1` profile, which
//! carries exactly the base's objective; a timed deadline keeps FR-920's
//! profile and states the side in its `kind`. Either shape is the base's own
//! exact document with the side stated, so it is read and written through the
//! base profile's strict canonical form: this module only converts between the
//! two spellings, and the base's checker compares every member with its own
//! derivation.

use serde_json::{json, Value};

use super::*;

/// The base profile a wrapper may carry: bounded reachability, which FR-197's
/// backward induction rounds into an interval.
fn is_wrappable_base(profile: &str) -> bool {
    profile == PROFILE
}

fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, ProbabilityDocumentError> {
    value.as_object().ok_or(ProbabilityDocumentError::Malformed)
}

fn side_of(kind: &str) -> Option<SubsystemBoundKind> {
    match kind {
        "lower" => Some(SubsystemBoundKind::Lower),
        "upper" => Some(SubsystemBoundKind::Upper),
        _ => None,
    }
}

/// Read value-bounds `bytes` as the exact document of their base and the side
/// they certify; `None` when `bytes` is not a proof-role bound document.
///
/// An `upper` document carries no `ranking` (the exact reader's required
/// member is written empty); a `lower` document's missing `ranking` is left
/// for the exact reader to name.
pub(super) fn unwrap(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Option<(SubsystemBoundKind, Vec<u8>)>, ProbabilityDocumentError> {
    poll(cancel)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            size,
        ));
    }
    #[derive(serde::Deserialize)]
    struct Dispatch<'a> {
        #[serde(borrow)]
        kind: &'a str,
    }
    // An exact document is read by its own profile; only a bound is parsed here.
    let Some(side) = serde_json::from_slice::<Dispatch<'_>>(bytes)
        .ok()
        .and_then(|dispatch| side_of(dispatch.kind))
    else {
        return Ok(None);
    };
    let mut document: Value =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    // The bytes offered must be the one canonical spelling of what was read.
    if canonical(&document, limits.bytes)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let profile = document
        .pointer("/objective/profile")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let root = document
        .as_object_mut()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    match profile.as_deref() {
        Some(measure::bounds::PROFILE) => {
            let objective = root
                .get("objective")
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let members = object(objective)?;
            let base = members
                .get("base")
                .and_then(Value::as_object)
                .ok_or(ProbabilityDocumentError::MalformedMember("objective"))?;
            if members.len() != 2
                || !base
                    .get("profile")
                    .and_then(Value::as_str)
                    .is_some_and(is_wrappable_base)
            {
                return Ok(None);
            }
            let base = objective["base"].clone();
            root.insert("objective".into(), base);
            match side {
                SubsystemBoundKind::Upper => {
                    if root.contains_key("ranking") {
                        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                    }
                    root.insert("ranking".into(), json!([]));
                }
                SubsystemBoundKind::Lower => {}
            }
        }
        Some(TIMED_PROFILE) => {
            let quantile = root
                .get("objective")
                .and_then(|objective| objective.pointer("/timed/form"))
                .and_then(Value::as_str)
                == Some("elapsed-quantile");
            if quantile && side == SubsystemBoundKind::Upper {
                return Ok(None);
            }
        }
        _ => return Ok(None),
    }
    root.insert("kind".into(), json!("exact"));
    let exact = canonical(&document, limits.bytes)?;
    Ok(Some((side, exact)))
}

/// The proof-role spelling of the base's exact `document` bytes for `side`.
pub(super) fn wrap(
    exact: &[u8],
    side: SubsystemBoundKind,
    limits: ProbabilityDocumentLimits,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let mut document: Value =
        serde_json::from_slice(exact).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let root = document
        .as_object_mut()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    root.insert(
        "kind".into(),
        json!(match side {
            SubsystemBoundKind::Lower => "lower",
            SubsystemBoundKind::Upper => "upper",
        }),
    );
    let timed = root
        .get("objective")
        .and_then(|objective| objective.get("profile"))
        .and_then(Value::as_str)
        == Some(TIMED_PROFILE);
    if !timed {
        let base = root
            .remove("objective")
            .ok_or(ProbabilityDocumentError::Malformed)?;
        root.insert(
            "objective".into(),
            json!({"profile": measure::bounds::PROFILE, "base": base}),
        );
        if side == SubsystemBoundKind::Upper {
            root.remove("ranking");
        }
    }
    canonical(&document, limits.bytes)
}
