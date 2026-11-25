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

use serde_json::{Value, json};

use super::*;

/// The base profiles a wrapper may carry: bounded reachability, which FR-197's
/// backward induction rounds into an interval, unbounded eventual
/// reachability over a cyclic chain or MDP, and the expected rewards.
fn is_wrappable_base(profile: &str) -> bool {
    matches!(
        profile,
        PROFILE
            | CYCLIC_PROFILE
            | CYCLIC_MDP_PROFILE
            | REWARD_PROFILE
            | CYCLIC_REWARD_PROFILE
            | CYCLIC_MDP_REWARD_PROFILE
            | measure::PROFILE
            | "filament.probability-objective/cyclic-mdp-fair-eventual/v1"
    )
}

/// An expected-reward base, whose `upper` carries its properness ranking
/// (FR-1042 "Conditions").
fn is_reward_base(profile: &str) -> bool {
    matches!(
        profile,
        REWARD_PROFILE | CYCLIC_REWARD_PROFILE | CYCLIC_MDP_REWARD_PROFILE
    )
}

/// One unchecked `policy` row of a bound document: a complete product key and
/// the FR-181 transition identity hinted there, as offered.
#[derive(Clone, Debug)]
pub(super) struct PolicyHint {
    pub(super) state: String,
    pub(super) choice: String,
}

/// A proof-role bound document read as its base's exact document, the side it
/// certifies and its `policy` hints, when it carries that member.
pub(super) struct BoundDocument {
    pub(super) side: SubsystemBoundKind,
    pub(super) exact: Vec<u8>,
    pub(super) policy: Option<Vec<PolicyHint>>,
}

/// The `policy` member of canonical bound-document `bytes`, row shapes only.
fn policy_hints(bytes: &[u8]) -> Result<Vec<PolicyHint>, ProbabilityDocumentError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Row<'a> {
        #[serde(borrow)]
        state: &'a str,
        #[serde(borrow)]
        choice: &'a serde_json::value::RawValue,
    }
    #[derive(Deserialize)]
    struct Hints<'a> {
        #[serde(borrow)]
        policy: Vec<Row<'a>>,
    }
    let hints: Hints<'_> = serde_json::from_slice(bytes)
        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
    Ok(hints
        .policy
        .into_iter()
        .map(|row| PolicyHint {
            state: row.state.to_owned(),
            choice: row.choice.get().to_owned(),
        })
        .collect())
}
fn is_fair_base(profile: &str) -> bool {
    profile == "filament.probability-objective/cyclic-mdp-fair-eventual/v1"
}

/// Fair-MDP intervals carry a progressing policy on the existential side:
/// upper for a minimum and lower for a maximum. The opposite endpoint is
/// certified universally and therefore has no ranking or policy member.
fn fair_existential_side(
    base: &Value,
    side: SubsystemBoundKind,
) -> Result<bool, ProbabilityDocumentError> {
    let optimization = base
        .get("optimization")
        .and_then(Value::as_str)
        .ok_or(ProbabilityDocumentError::MalformedMember("objective"))?;
    match (optimization, side) {
        ("minimum", SubsystemBoundKind::Upper) | ("maximum", SubsystemBoundKind::Lower) => Ok(true),
        ("minimum", SubsystemBoundKind::Lower) | ("maximum", SubsystemBoundKind::Upper) => {
            Ok(false)
        }
        _ => Err(ProbabilityDocumentError::Unsupported),
    }
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

/// Read value-bounds `bytes` as the exact document of their base, the side
/// they certify and their policy hints; `None` when `bytes` is not a
/// proof-role bound document.
///
/// An `upper` document carries no `ranking` (the exact reader's required
/// member is written empty); a `lower` document's missing `ranking` is left
/// for the exact reader to name. The `policy` member is removed from the exact
/// document and returned unchecked for the base's checker to resolve.
pub(super) fn unwrap(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Option<BoundDocument>, ProbabilityDocumentError> {
    poll(cancel)?;
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            size,
        ));
    }
    // One metered read admits the bytes (size, storage and cancel); the
    // `Value` below is built only from bytes that read admitted, and only for
    // a bound document.
    let parsed = quire_canonical::read_bounded(bytes, limits.bytes, limits.storage_bytes).map_err(
        |error| match error {
            quire_canonical::ReadError::Limit(limit) => exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limit.bound,
                limit.required,
            ),
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    // An exact document is read by its own profile; only a bound is rewritten.
    let Some(side) = parsed
        .root()
        .get("kind")
        .and_then(|node| match node.node() {
            quire_canonical::Node::String(kind) => side_of(kind),
            _ => None,
        })
    else {
        return Ok(None);
    };
    poll(cancel)?;
    // The bytes offered must be the one canonical spelling of what was read.
    let respelled = quire_canonical::to_vec(&parsed, quire_canonical::Limits::new(limits.bytes))
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if respelled != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let mut document: Value =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let mut policy = None;
    let profile = document
        .pointer("/objective/profile")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let reward = document
        .pointer("/objective/base/profile")
        .and_then(Value::as_str)
        .is_some_and(is_reward_base);
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
            let fair = base
                .get("profile")
                .and_then(Value::as_str)
                .is_some_and(is_fair_base);
            let existential = fair
                .then(|| fair_existential_side(&base, side))
                .transpose()?;
            root.insert("objective".into(), base);
            if root.remove("policy").is_some() {
                policy = Some(policy_hints(bytes)?);
            }
            if fair && !existential.expect("fair objective has a role") {
                if root.contains_key("ranking") || policy.is_some() {
                    return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                }
                root.insert("ranking".into(), json!([]));
            }
            match side {
                SubsystemBoundKind::Upper => {
                    if fair {
                        // The fair-MDP checker validates the rank against its
                        // independently reconstructed zero component for
                        // either interval endpoint.
                    } else if root.contains_key("ranking") && !reward {
                        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                    } else if !root.contains_key("ranking") {
                        root.insert("ranking".into(), json!([]));
                    }
                }
                SubsystemBoundKind::Lower if reward => {
                    root.entry("ranking").or_insert_with(|| json!([]));
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
    Ok(Some(BoundDocument {
        side,
        exact,
        policy,
    }))
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
    let reward = root
        .get("objective")
        .and_then(|objective| objective.get("profile"))
        .and_then(Value::as_str)
        .is_some_and(is_reward_base);
    if !timed {
        let base = root
            .remove("objective")
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let fair = base
            .get("profile")
            .and_then(Value::as_str)
            .is_some_and(is_fair_base);
        let existential = fair
            .then(|| fair_existential_side(&base, side))
            .transpose()?;
        root.insert(
            "objective".into(),
            json!({"profile": measure::bounds::PROFILE, "base": base}),
        );
        if fair {
            if !existential.expect("fair objective has a role") {
                root.remove("ranking");
                root.remove("policy");
            }
        } else if side == SubsystemBoundKind::Upper && !reward {
            root.remove("ranking");
        }
    }
    canonical(&document, limits.bytes)
}
