// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-895: read and assess a deployed protocol recording against a checked
//! protocol. Recording deviations are verdicts; only failures to correspond
//! to the declared protocol are refusals.

use std::collections::{BTreeMap, BTreeSet};

use filament_semantic_value::catalog::CatalogCode;
use fsl_forms::ChannelDelivery;
use fsl_semantics::check::CheckedProtocol;
use fsl_semantics::check::ProtocolNodeId;
use serde::{Deserialize, Serialize};

const FORMAT: &str = "filament.protocol-recording/1";

/// A verdict for one protocol obligation in a recorded execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingVerdict {
    /// The recording contains evidence that the obligation holds.
    Satisfied,
    /// The recording contains evidence that the obligation was violated.
    Violated,
    /// The recording does not decide the obligation.
    Pending,
}

/// A violated recording obligation and the observations that witness it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordingCause {
    /// The observed action is not enabled by the protocol history.
    Unexpected,
    /// A required observation is absent from a closed recording.
    Missing,
    /// A required causal predecessor follows its target.
    Misordered,
    /// A protocol action or effect is repeated beyond its allowed identity.
    Duplicated,
    /// A receive is correlated with a different send than the protocol permits.
    WronglyCorrelated,
}

/// The identity of one obligation assessed from a recording.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RecordingObligationId {
    /// The kind of protocol obligation.
    pub kind: String,
    /// The qualified protocol node when the obligation belongs to a node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// The protocol instance when the obligation is instance-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

/// The result for one obligation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordingObligation {
    /// The obligation being decided.
    pub obligation: RecordingObligationId,
    /// Its verdict.
    pub verdict: RecordingVerdict,
    /// Why the obligation was violated, when it was violated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<RecordingCause>,
    /// Observation indices that witness the verdict.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub witness: Vec<usize>,
}

/// The closure facts copied from the recording.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordingClosure {
    /// The recorder's stated closure state.
    pub state: String,
    /// The recorder's stated premise identities.
    pub premises: Vec<RecordingPremise>,
    /// The source of this closure record.
    pub producer: &'static str,
}

/// A premise asserted by the recording's closure record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingPremise {
    /// The premise kind, such as `execution` or `branch`.
    pub kind: String,
    /// The premise identity.
    pub identity: String,
}

/// The global result computed from the obligation verdicts and closure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordingGlobalResult {
    /// `proved`, `refuted`, or `inconclusive`.
    pub label: String,
    /// The basis of the result.
    pub basis: String,
    /// Whether every obligation and closure premise was decided successfully.
    pub complete: bool,
    /// The cause for an inconclusive result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}

/// An owned assessment of a protocol recording.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordingAssessment {
    /// One result per obligation represented by this recording assessment.
    pub obligations: Vec<RecordingObligation>,
    /// The closure record copied from the recording.
    pub closure: RecordingClosure,
    /// Attempt, recovery and duplicate-effect facts retained from the run.
    pub facts: Vec<RecordingFact>,
    /// The global result computed from the obligation results and closure.
    pub global: RecordingGlobalResult,
    /// The observation index of the first violating observation, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_position: Option<usize>,
}

/// A fact reported alongside the per-obligation assessment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RecordingFact {
    /// One observed operation or compensation attempt and its reported outcome.
    AttemptOutcome {
        /// The attempt's runtime identity.
        attempt: serde_json::Value,
        /// The registration when this is a compensation attempt.
        #[serde(skip_serializing_if = "Option::is_none")]
        registration: Option<serde_json::Value>,
        /// The predecessor attempt for a retry, when reported.
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_of: Option<serde_json::Value>,
        /// The reported attempt outcome.
        outcome: String,
        /// The source observation index.
        observation: usize,
    },
    /// One recovery outcome associated with a registration.
    Recovery {
        /// The registration's runtime identity.
        registration: serde_json::Value,
        /// The reported recovery outcome.
        outcome: String,
        /// The source observation index.
        observation: usize,
    },
    /// Two distinct business-effect keys bound to one effect occurrence.
    DuplicateBusinessEffect {
        /// The registration created by the effect, when one exists.
        #[serde(skip_serializing_if = "Option::is_none")]
        registration: Option<serde_json::Value>,
        /// The shared effect occurrence identity.
        occurrence: RecordingEffectOccurrence,
        /// The first and newly observed business-effect keys.
        business_effect_keys: Vec<serde_json::Value>,
        /// FR-236's aggregate fact value.
        value: String,
    },
    /// One complete observation key repeated with equal binding facts.
    ReplayedObservation {
        /// The kind of runtime subject that was replayed.
        subject_kind: String,
        /// The repeated business-effect identity.
        identity: serde_json::Value,
        /// The occurrence the subject is bound to.
        occurrence: RecordingEffectOccurrence,
        /// How many equal observations were retained as one subject.
        observations: usize,
    },
    /// Two distinct deliveries of one protocol message were received.
    DuplicateDelivery {
        /// The channel that carries the message.
        channel: String,
        /// The exact message identity.
        message: serde_json::Value,
        /// The distinct delivery identities observed for this message.
        delivery_keys: Vec<serde_json::Value>,
        /// FR-236's aggregate fact value.
        value: String,
    },
}

/// The FR-894 identity of an effect occurrence in a recording.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecordingEffectOccurrence {
    /// The protocol instance that established this effect.
    pub instance: String,
    /// The qualified effect node.
    pub node: String,
    /// The thread path at the effect occurrence.
    pub path: serde_json::Value,
    /// The occurrence ordinal of this effect node on the path.
    pub ordinal: u64,
}

/// Why a recording could not be read against the checked protocol.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_runtime_input/{cause}: {detail}")]
pub struct RecordingRefusal {
    cause: &'static str,
    detail: String,
    position: Option<usize>,
}

impl RecordingRefusal {
    fn new(cause: &'static str, detail: impl Into<String>, position: Option<usize>) -> Self {
        Self {
            cause,
            detail: detail.into(),
            position,
        }
    }

    /// The catalog code for a recording admission failure.
    pub fn code(&self) -> CatalogCode {
        CatalogCode::new("invalid_runtime_input", self.cause)
    }

    /// The catalog cause of the refusal.
    pub fn cause(&self) -> &str {
        self.cause
    }

    /// The observation index named by the refusal, when present.
    pub fn position(&self) -> Option<usize> {
        self.position
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ObservationKind {
    Send,
    Receive,
    Attempt,
    Effect,
    Event,
    Commit,
    CompensationAttempt,
    Recovery,
    Activate,
    Spawn,
    Retire,
    Finish,
    Case,
}

impl ObservationKind {
    fn label(self) -> &'static str {
        match self {
            Self::Send => "send",
            Self::Receive => "receive",
            Self::Attempt => "attempt",
            Self::Effect => "effect",
            Self::Event => "event",
            Self::Commit => "commit",
            Self::CompensationAttempt => "compensation-attempt",
            Self::Recovery => "recovery",
            Self::Activate => "activate",
            Self::Spawn => "spawn",
            Self::Retire => "retire",
            Self::Finish => "finish",
            Self::Case => "case",
        }
    }

    fn protocol_node_kind(self) -> Option<&'static str> {
        match self {
            Self::Send
            | Self::Receive
            | Self::Attempt
            | Self::Effect
            | Self::Event
            | Self::Commit
            | Self::Finish
            | Self::Case => Some(self.label()),
            Self::CompensationAttempt
            | Self::Recovery
            | Self::Activate
            | Self::Spawn
            | Self::Retire => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordingDocument {
    format: String,
    protocol: String,
    observations: Vec<Observation>,
    local_order: BTreeMap<String, Vec<usize>>,
    correlations: Vec<Correlation>,
    closure: ClosureDocument,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    kind: ObservationKind,
    node: String,
    role: String,
    #[serde(default)]
    instance: Option<String>,
    #[serde(default)]
    keys: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    record: Option<serde_json::Value>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    clocks: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    occurrence: Option<EffectOccurrenceDocument>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EffectOccurrenceDocument {
    path: serde_json::Value,
    ordinal: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Correlation {
    send: usize,
    receive: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosureDocument {
    state: String,
    #[serde(default)]
    premises: Vec<RecordingPremise>,
}

/// Assess a deployed protocol recording against `protocol`.
///
/// The decoder refuses malformed recordings and correspondence defects. A
/// protocol deviation remains a `violated` result. Returned data is owned so
/// an `analyze` adapter can retain it independently of the input bytes.
///
/// # Errors
///
/// [`RecordingRefusal`] when the document is malformed, names another
/// protocol, names an undeclared role or node, has invalid local-order or
/// correlation references, or contradicts a runtime identity.
pub fn assess_recording(
    protocol: &CheckedProtocol,
    bytes: &[u8],
) -> Result<RecordingAssessment, RecordingRefusal> {
    let document: RecordingDocument = serde_json::from_slice(bytes)
        .map_err(|error| RecordingRefusal::new("invalid-value", error.to_string(), None))?;
    if document.format != FORMAT {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "format must be filament.protocol-recording/1",
            None,
        ));
    }
    if document.protocol != protocol.name {
        return Err(RecordingRefusal::new(
            "invalid-value",
            format!(
                "recording names protocol `{}`, expected `{}`",
                document.protocol, protocol.name
            ),
            None,
        ));
    }
    if !matches!(
        document.closure.state.as_str(),
        "open" | "closed" | "contradicted"
    ) {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "closure state must be open, closed or contradicted",
            None,
        ));
    }

    let roles: BTreeSet<&str> = protocol
        .roles
        .iter()
        .map(|role| role.name.as_str())
        .collect();
    let nodes = protocol_node_kinds(protocol);
    for (position, observation) in document.observations.iter().enumerate() {
        if !roles.contains(observation.role.as_str()) {
            return Err(RecordingRefusal::new(
                "missing-name",
                format!("undeclared role `{}`", observation.role),
                Some(position),
            ));
        }
        let Some(expected_kind) = nodes.get(observation.node.as_str()) else {
            return Err(RecordingRefusal::new(
                "missing-name",
                format!("unknown protocol node `{}`", observation.node),
                Some(position),
            ));
        };
        if let Some(actual_kind) = observation.kind.protocol_node_kind() {
            if actual_kind != expected_kind {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!(
                        "observation kind `{}` names a `{expected_kind}` node",
                        observation.kind.label()
                    ),
                    Some(position),
                ));
            }
        }
    }
    validate_local_order(protocol, &document)?;
    validate_correlations(&document)?;
    validate_identities(&document)?;

    let obligations = assess_obligations(protocol, &document);
    let trace_position = obligations
        .iter()
        .filter(|entry| entry.verdict == RecordingVerdict::Violated)
        .flat_map(|entry| entry.witness.iter().copied())
        .min();
    let any_violated = obligations
        .iter()
        .any(|entry| entry.verdict == RecordingVerdict::Violated);
    let all_satisfied = obligations
        .iter()
        .all(|entry| entry.verdict == RecordingVerdict::Satisfied);
    let (label, basis, cause, complete) = if any_violated {
        ("refuted", "decisive-counterexample", None, false)
    } else if all_satisfied && document.closure.state == "closed" {
        ("proved", "closed-scope", None, true)
    } else {
        ("inconclusive", "unsettled", Some("open-history"), false)
    };
    Ok(RecordingAssessment {
        obligations,
        facts: recording_facts(protocol, &document),
        closure: RecordingClosure {
            state: document.closure.state,
            premises: document.closure.premises,
            producer: "recording",
        },
        global: RecordingGlobalResult {
            label: label.to_owned(),
            basis: basis.to_owned(),
            complete,
            cause: cause.map(str::to_owned),
        },
        trace_position,
    })
}

fn recording_facts(protocol: &CheckedProtocol, document: &RecordingDocument) -> Vec<RecordingFact> {
    let mut facts = Vec::new();
    let mut effects = BTreeMap::<
        String,
        (
            RecordingEffectOccurrence,
            Option<serde_json::Value>,
            BTreeMap<String, (serde_json::Value, usize)>,
        ),
    >::new();

    for (position, observation) in document.observations.iter().enumerate() {
        match observation.kind {
            ObservationKind::Attempt => {
                if let (Some(attempt), Some(outcome)) = (
                    observation.keys.get("attempt"),
                    observation.outcome.as_ref(),
                ) {
                    facts.push(RecordingFact::AttemptOutcome {
                        attempt: attempt.clone(),
                        registration: None,
                        retry_of: observation.keys.get("retry_of").cloned(),
                        outcome: outcome.clone(),
                        observation: position,
                    });
                }
            }
            ObservationKind::CompensationAttempt => {
                if let (Some(attempt), Some(outcome)) = (
                    observation.keys.get("compensation_attempt"),
                    observation.outcome.as_ref(),
                ) {
                    facts.push(RecordingFact::AttemptOutcome {
                        attempt: attempt.clone(),
                        registration: observation.keys.get("registration").cloned(),
                        retry_of: observation.keys.get("retry_of").cloned(),
                        outcome: outcome.clone(),
                        observation: position,
                    });
                }
            }
            ObservationKind::Recovery => {
                if let (Some(registration), Some(outcome)) = (
                    observation.keys.get("registration"),
                    observation.outcome.as_ref(),
                ) {
                    facts.push(RecordingFact::Recovery {
                        registration: registration.clone(),
                        outcome: outcome.clone(),
                        observation: position,
                    });
                }
            }
            ObservationKind::Effect => {
                let (Some(instance), Some(occurrence), Some(effect_key)) = (
                    observation.instance.as_ref(),
                    observation.occurrence.as_ref(),
                    observation.keys.get("business_effect"),
                ) else {
                    continue;
                };
                let effect_occurrence = RecordingEffectOccurrence {
                    instance: instance.clone(),
                    node: observation.node.clone(),
                    path: occurrence.path.clone(),
                    ordinal: occurrence.ordinal,
                };
                let registration = observation.keys.get("registration").cloned();
                let occurrence_key = serde_json::json!({
                    "instance": effect_occurrence.instance,
                    "node": effect_occurrence.node,
                    "path": effect_occurrence.path,
                    "ordinal": effect_occurrence.ordinal,
                    "registration": registration,
                })
                .to_string();
                let entry = effects
                    .entry(occurrence_key)
                    .or_insert_with(|| (effect_occurrence, registration, BTreeMap::new()));
                let (_, count) = entry
                    .2
                    .entry(effect_key.to_string())
                    .or_insert_with(|| (effect_key.clone(), 0));
                *count += 1;
            }
            _ => {}
        }
    }

    for (_, (occurrence, registration, keys)) in effects {
        let mut keys = keys.into_values();
        let Some((first, first_count)) = keys.next() else {
            continue;
        };
        let second = keys.next();
        if second.is_none() && first_count > 1 {
            facts.push(RecordingFact::ReplayedObservation {
                subject_kind: "business-effect".to_owned(),
                identity: first,
                occurrence,
                observations: first_count,
            });
            continue;
        }
        for (second, _) in second.into_iter().chain(keys) {
            facts.push(RecordingFact::DuplicateBusinessEffect {
                registration: registration.clone(),
                occurrence: occurrence.clone(),
                business_effect_keys: vec![first.clone(), second],
                value: "duplicate".to_owned(),
            });
        }
    }

    for duplicate in duplicate_deliveries(protocol, document) {
        facts.push(RecordingFact::DuplicateDelivery {
            channel: duplicate.channel,
            message: duplicate.message,
            delivery_keys: duplicate.delivery_keys,
            value: "duplicate".to_owned(),
        });
    }

    facts
}

struct DuplicateDeliveryEvidence {
    channel: String,
    message: serde_json::Value,
    delivery_keys: Vec<serde_json::Value>,
    receive_positions: Vec<usize>,
    policy: ChannelDelivery,
}

fn duplicate_deliveries(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
) -> Vec<DuplicateDeliveryEvidence> {
    let mut received = BTreeMap::<
        (String, String),
        (
            serde_json::Value,
            BTreeMap<String, (serde_json::Value, Vec<usize>)>,
        ),
    >::new();
    for pair in &document.correlations {
        let send = &document.observations[pair.send];
        let receive = &document.observations[pair.receive];
        let (Some(message), Some(received_message), Some(delivery)) = (
            send.keys.get("message"),
            receive.keys.get("message"),
            receive.keys.get("delivery"),
        ) else {
            continue;
        };
        if message != received_message {
            continue;
        }
        let Some(channel) = channel_for_send(protocol, send) else {
            continue;
        };
        let deliveries = received
            .entry((channel.to_owned(), message.to_string()))
            .or_insert_with(|| (message.clone(), BTreeMap::new()));
        deliveries
            .1
            .entry(delivery.to_string())
            .or_insert_with(|| (delivery.clone(), Vec::new()))
            .1
            .push(pair.receive);
    }

    received
        .into_iter()
        .filter_map(|((channel_name, _), (message, deliveries))| {
            if deliveries.len() < 2 {
                return None;
            }
            let channel = protocol
                .channels
                .iter()
                .find(|entry| entry.name == channel_name)?;
            Some(DuplicateDeliveryEvidence {
                channel: channel_name,
                message,
                delivery_keys: deliveries
                    .values()
                    .map(|(identity, _)| identity.clone())
                    .collect(),
                receive_positions: deliveries
                    .values()
                    .flat_map(|(_, positions)| positions.iter().copied())
                    .collect(),
                policy: channel.delivery,
            })
        })
        .collect()
}

fn channel_for_send<'a>(protocol: &'a CheckedProtocol, send: &Observation) -> Option<&'a str> {
    let via = protocol.vias.iter().find(|via| {
        send.node
            .ends_with(&protocol_node_name(protocol, via.node.0))
    })?;
    Some(via.channel.as_str())
}

fn protocol_node_kinds(protocol: &CheckedProtocol) -> BTreeMap<String, String> {
    let mut nodes = BTreeMap::new();
    for control in &protocol.controls.controls {
        nodes.insert(
            format!("{}::{}", protocol.name, control.path.join("::")),
            control.kind.label().to_owned(),
        );
    }
    if let Some((_, finish)) = &protocol.finish {
        nodes.insert(
            format!("{}::{}", protocol.name, finish),
            "finish".to_owned(),
        );
    }
    nodes
}

fn validate_local_order(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
) -> Result<(), RecordingRefusal> {
    let roles: BTreeSet<&str> = protocol
        .roles
        .iter()
        .map(|role| role.name.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    let mut ordered_positions = BTreeSet::new();
    for (role, positions) in &document.local_order {
        if !roles.contains(role.as_str()) {
            return Err(RecordingRefusal::new(
                "missing-name",
                format!("local_order names undeclared role `{role}`"),
                None,
            ));
        }
        for position in positions {
            if *position >= document.observations.len() {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!("/local_order/{role} names absent observation {position}"),
                    Some(*position),
                ));
            }
            if !seen.insert((role, position)) {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!("/local_order/{role} repeats observation {position}"),
                    Some(*position),
                ));
            }
            if !ordered_positions.insert(*position) {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!("observation {position} occurs in multiple local orders"),
                    Some(*position),
                ));
            }
            if document.observations[*position].role != *role {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!("/local_order/{role} contradicts observation role"),
                    Some(*position),
                ));
            }
        }
    }
    if ordered_positions.len() != document.observations.len() {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "local_order must place every observation exactly once",
            None,
        ));
    }
    Ok(())
}

fn validate_correlations(document: &RecordingDocument) -> Result<(), RecordingRefusal> {
    let mut receives = BTreeSet::new();
    for (index, pair) in document.correlations.iter().enumerate() {
        let send = document.observations.get(pair.send);
        let receive = document.observations.get(pair.receive);
        if !matches!(send.map(|entry| entry.kind), Some(ObservationKind::Send))
            || !matches!(
                receive.map(|entry| entry.kind),
                Some(ObservationKind::Receive)
            )
        {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!("/correlations/{index} must pair a send with a receive"),
                None,
            ));
        }
        if !receives.insert(pair.receive) {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!("/correlations/{index} repeats a receive"),
                Some(pair.receive),
            ));
        }
    }
    Ok(())
}

fn validate_identities(document: &RecordingDocument) -> Result<(), RecordingRefusal> {
    let mut facts = BTreeMap::<(String, String), serde_json::Value>::new();
    let mut messages = BTreeMap::<String, (Option<String>, Option<serde_json::Value>)>::new();
    let mut deliveries = BTreeMap::<String, (String, Option<String>)>::new();
    for (position, observation) in document.observations.iter().enumerate() {
        match observation.kind {
            ObservationKind::Send | ObservationKind::Receive => {
                let (Some(message_key), Some(delivery_key)) = (
                    observation.keys.get("message"),
                    observation.keys.get("delivery"),
                ) else {
                    return Err(RecordingRefusal::new(
                        "invalid-value",
                        "send and receive observations require message and delivery keys",
                        Some(position),
                    ));
                };
                let message = message_key.to_string();
                let message_facts = (observation.instance.clone(), observation.record.clone());
                if let Some(existing) = messages.get(&message) {
                    if existing != &message_facts {
                        return Err(RecordingRefusal::new(
                            "invalid-value",
                            "message key is reused with contradictory facts",
                            Some(position),
                        ));
                    }
                } else {
                    messages.insert(message.clone(), message_facts);
                }
                let delivery = delivery_key.to_string();
                let delivery_facts = (message, observation.instance.clone());
                if let Some(existing) = deliveries.get(&delivery) {
                    if existing != &delivery_facts {
                        return Err(RecordingRefusal::new(
                            "invalid-value",
                            "delivery key is reused with contradictory facts",
                            Some(position),
                        ));
                    }
                } else {
                    deliveries.insert(delivery, delivery_facts);
                }
            }
            ObservationKind::Attempt if !observation.keys.contains_key("attempt") => {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    "attempt observation requires an attempt key",
                    Some(position),
                ));
            }
            ObservationKind::Effect
                if !observation.keys.contains_key("business_effect")
                    || !observation.keys.contains_key("attempt")
                    || observation.instance.is_none()
                    || observation.occurrence.is_none() =>
            {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    "effect observation requires instance, occurrence, attempt and business-effect identities",
                    Some(position),
                ));
            }
            ObservationKind::CompensationAttempt
                if !observation.keys.contains_key("compensation_attempt")
                    || !observation.keys.contains_key("registration") =>
            {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    "compensation attempt requires attempt and registration keys",
                    Some(position),
                ));
            }
            _ => {}
        }
        let identity_kind = match observation.kind {
            ObservationKind::Attempt => Some("attempt"),
            ObservationKind::Effect => Some("business_effect"),
            ObservationKind::CompensationAttempt => Some("compensation_attempt"),
            _ => None,
        };
        if let Some(key_name) = identity_kind {
            if let Some(key) = observation.keys.get(key_name) {
                let identity = key.to_string();
                let signature = serde_json::json!({
                    "node": observation.node,
                    "role": observation.role,
                    "instance": observation.instance,
                    "record": observation.record,
                    "outcome": observation.outcome,
                    "clocks": observation.clocks,
                    "keys": observation.keys,
                    "occurrence": observation.occurrence,
                });
                let map_key = (key_name.to_owned(), identity);
                if let Some(previous) = facts.get(&map_key) {
                    if previous != &signature {
                        return Err(RecordingRefusal::new(
                            "invalid-value",
                            format!("runtime key `{key_name}` has conflicting facts"),
                            Some(position),
                        ));
                    }
                } else {
                    facts.insert(map_key, signature);
                }
            }
        }
    }
    Ok(())
}

fn assess_obligations(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
) -> Vec<RecordingObligation> {
    let mut obligations = Vec::new();
    let mut observed_nodes = BTreeMap::<String, Vec<usize>>::new();
    for (position, observation) in document.observations.iter().enumerate() {
        observed_nodes
            .entry(observation.node.clone())
            .or_default()
            .push(position);
        let enabled = observation_enabled(protocol, document, position);
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "enabledness".to_owned(),
                node: Some(observation.node.clone()),
                instance: observation.instance.clone(),
            },
            verdict: if enabled {
                RecordingVerdict::Satisfied
            } else {
                RecordingVerdict::Violated
            },
            cause: (!enabled).then_some(RecordingCause::Unexpected),
            witness: vec![position],
        });
    }

    for correlation in &document.correlations {
        let send = &document.observations[correlation.send];
        let receive = &document.observations[correlation.receive];
        let misordered = causally_after(document, correlation.send, correlation.receive);
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "causal-order".to_owned(),
                node: Some(format!("{} -> {}", send.node, receive.node)),
                instance: send.instance.clone().or_else(|| receive.instance.clone()),
            },
            verdict: if misordered {
                RecordingVerdict::Violated
            } else {
                RecordingVerdict::Satisfied
            },
            cause: misordered.then_some(RecordingCause::Misordered),
            witness: if misordered {
                vec![correlation.receive, correlation.send]
            } else {
                vec![correlation.send, correlation.receive]
            },
        });
    }

    for control in &protocol.controls.controls {
        let is_decision = matches!(
            control.kind.label(),
            "choice" | "repeat" | "parallel" | "check"
        );
        if !is_decision {
            continue;
        }
        let qualified = format!("{}::{}", protocol.name, control.path.join("::"));
        let decision = document
            .observations
            .iter()
            .enumerate()
            .find(|(_, observation)| {
                observation.kind == ObservationKind::Case
                    && observation.node.starts_with(&format!("{qualified}::"))
            });
        let closure_closed = document.closure.state == "closed";
        let (verdict, cause, witness) = match decision {
            Some((position, _)) => (RecordingVerdict::Satisfied, None, vec![position]),
            None if closure_closed => (
                RecordingVerdict::Violated,
                Some(RecordingCause::Missing),
                Vec::new(),
            ),
            None => (RecordingVerdict::Pending, None, Vec::new()),
        };
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "control".to_owned(),
                node: Some(qualified),
                instance: None,
            },
            verdict,
            cause,
            witness,
        });
    }

    for channel in &protocol.channels {
        let correlated = document.correlations.iter().any(|pair| {
            let send = &document.observations[pair.send];
            let receive = &document.observations[pair.receive];
            protocol.vias.iter().any(|via| {
                via.channel == channel.name
                    && send
                        .node
                        .ends_with(&protocol_node_name(protocol, via.node.0))
            }) || receive.node.contains(&channel.name)
        });
        let duplicate_delivery = duplicate_deliveries(protocol, document)
            .into_iter()
            .filter(|duplicate| duplicate.channel == channel.name)
            .collect::<Vec<_>>();
        let duplicate_positions = duplicate_delivery
            .iter()
            .flat_map(|duplicate| duplicate.receive_positions.iter().copied())
            .collect::<Vec<_>>();
        let duplicate_policy = duplicate_delivery.first().map(|duplicate| duplicate.policy);
        let (verdict, cause, witness) = match duplicate_policy {
            Some(ChannelDelivery::AtLeastOnce)
            | Some(ChannelDelivery::Between {
                upper: 2..=u64::MAX,
                ..
            }) => (RecordingVerdict::Satisfied, None, duplicate_positions),
            Some(ChannelDelivery::AtMostOnce | ChannelDelivery::ExactlyOncePremise)
            | Some(ChannelDelivery::Between { upper: 0..=1, .. }) => (
                RecordingVerdict::Violated,
                Some(RecordingCause::Duplicated),
                duplicate_positions,
            ),
            Some(ChannelDelivery::Unknown) => (RecordingVerdict::Pending, None, Vec::new()),
            None if correlated || document.correlations.is_empty() => {
                (RecordingVerdict::Satisfied, None, Vec::new())
            }
            None => (RecordingVerdict::Pending, None, Vec::new()),
        };
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "channel-premise".to_owned(),
                node: Some(channel.name.clone()),
                instance: None,
            },
            verdict,
            cause,
            witness,
        });
    }

    let instances: BTreeSet<String> = document
        .observations
        .iter()
        .map(|observation| {
            observation
                .instance
                .clone()
                .unwrap_or_else(|| "default".to_owned())
        })
        .collect();
    for instance in instances {
        let finish = protocol.finish.as_ref().and_then(|(_, name)| {
            let qualified = format!("{}::{}", protocol.name, name);
            observed_nodes.get(&qualified)
        });
        let finish_seen = finish.is_some_and(|positions| {
            positions.iter().any(|position| {
                document.observations[*position].kind == ObservationKind::Finish
                    && document.observations[*position].instance.as_deref()
                        == Some(instance.as_str())
            })
        });
        let (verdict, cause, witness) = if finish_seen && document.closure.state == "closed" {
            (
                RecordingVerdict::Satisfied,
                None,
                finish.cloned().unwrap_or_default(),
            )
        } else if document.closure.state == "closed" {
            (
                RecordingVerdict::Violated,
                Some(RecordingCause::Missing),
                Vec::new(),
            )
        } else {
            (RecordingVerdict::Pending, None, Vec::new())
        };
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "closure".to_owned(),
                node: None,
                instance: Some(instance),
            },
            verdict,
            cause,
            witness,
        });
    }
    obligations
}

/// Whether `later` is constrained to follow `earlier` by local orders and
/// message correlations. Observation-array position contributes no edge.
fn causally_after(document: &RecordingDocument, later: usize, earlier: usize) -> bool {
    let mut edges = vec![Vec::new(); document.observations.len()];
    for order in document.local_order.values() {
        for pair in order.windows(2) {
            if let [from, to] = pair {
                edges[*from].push(*to);
            }
        }
    }
    for pair in &document.correlations {
        edges[pair.send].push(pair.receive);
    }
    let mut pending = vec![earlier];
    let mut visited = BTreeSet::new();
    while let Some(at) = pending.pop() {
        if at == later {
            return true;
        }
        if visited.insert(at) {
            pending.extend(edges[at].iter().copied());
        }
    }
    false
}

/// Whether a recorded action has the branch decision required by its checked
/// control ancestry before it occurs.
fn observation_enabled(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    position: usize,
) -> bool {
    let observation = &document.observations[position];
    if observation.kind == ObservationKind::Case {
        return true;
    }
    if observation.kind == ObservationKind::Finish {
        return protocol
            .finish
            .as_ref()
            .is_some_and(|(_, name)| format!("{}::{}", protocol.name, name) == observation.node);
    }
    let Some(target) = protocol.controls.controls.iter().find(|control| {
        format!("{}::{}", protocol.name, control.path.join("::")) == observation.node
    }) else {
        return false;
    };
    let Some(root) = protocol.controls.root() else {
        return false;
    };
    let mut ancestry = Vec::new();
    if !control_path(protocol, root.node, target.node, &mut ancestry) {
        return false;
    }
    for (index, ancestor) in ancestry.iter().enumerate() {
        if ancestor.kind.label() != "choice" {
            continue;
        }
        let selected_case = ancestry[index + 1..]
            .iter()
            .find(|entry| entry.kind.label() == "case");
        let Some(selected_case) = selected_case else {
            continue;
        };
        let case_name = format!("{}::{}", protocol.name, selected_case.path.join("::"));
        let Some((case_position, _)) =
            document.observations.iter().enumerate().find(|(_, entry)| {
                entry.kind == ObservationKind::Case
                    && entry.node == case_name
                    && entry.instance == observation.instance
            })
        else {
            return false;
        };
        if !causally_after(document, position, case_position) {
            return false;
        }
    }
    true
}

fn control_path<'protocol>(
    protocol: &'protocol CheckedProtocol,
    current: ProtocolNodeId,
    target: ProtocolNodeId,
    path: &mut Vec<&'protocol fsl_semantics::check::CheckedControl>,
) -> bool {
    let Some(control) = protocol.controls.control(current) else {
        return false;
    };
    path.push(control);
    if current == target {
        return true;
    }
    for child in &control.children {
        if control_path(protocol, *child, target, path) {
            return true;
        }
    }
    path.pop();
    false
}

fn protocol_node_name(protocol: &CheckedProtocol, node: usize) -> String {
    protocol
        .controls
        .controls
        .iter()
        .find(|control| control.node.0 == node)
        .map(|control| control.path.join("::"))
        .unwrap_or_default()
}
