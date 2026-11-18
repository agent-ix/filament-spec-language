// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-895: read and assess a deployed protocol recording against a checked
//! protocol. Recording deviations are verdicts; only failures to correspond
//! to the declared protocol are refusals.

use std::collections::{BTreeMap, BTreeSet};

use crate::analyze::document::{ActivatedRoleMember, ActivatedRolePopulation};
use filament_eval::FamilyOutcome;
use filament_exact::{FieldValue, Integer, Meter, Outcome, ScalarLimits, Value, ValueType};
use filament_semantic_value::catalog::CatalogCode;
use fsl_forms::ChannelDelivery;
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedProtocol;
use fsl_semantics::check::ProtocolNodeId;
use fsl_semantics::model::object_environment::ObjectEnvironment;
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
    /// The replicated role member when this is a fan-out control obligation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_instance: Option<RecordingRoleInstance>,
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
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingPremise {
    /// The premise kind, such as `execution` or `branch`.
    pub kind: RecordingPremiseKind,
    /// The protocol instance key, absent when the protocol has no `on each`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// The checked protocol node that closes this premise.
    pub node: String,
    /// The replicated role member, present on a fan-out branch premise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_instance: Option<RecordingRoleInstance>,
}

/// The closed kinds of FR-990 closure premise.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingPremiseKind {
    /// One protocol instance's finish terminal.
    Execution,
    /// One case, parallel branch or fan-out member.
    Branch,
    /// One registered compensation workflow.
    Workflow,
}

/// A replicated-role member reference used by an FR-990 branch premise.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordingRoleInstance {
    /// The reference authority.
    pub authority: String,
    /// The reference identity.
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
    role_instance: Option<RecordingRoleInstance>,
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

/// Assess a deployed protocol recording against the protocol in `package`.
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
    package: &CheckedPackage,
    bytes: &[u8],
) -> Result<RecordingAssessment, RecordingRefusal> {
    assess_recording_with_population(package, bytes, None)
}

/// Assess a recording with its optional activated role population.
pub fn assess_recording_with_population(
    package: &CheckedPackage,
    bytes: &[u8],
    population: Option<&[ActivatedRolePopulation]>,
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
    let protocol = package
        .graph()
        .protocol(&document.protocol)
        .ok_or_else(|| {
            RecordingRefusal::new(
                "invalid-value",
                format!(
                    "recording names protocol `{}` not held by the checked package",
                    document.protocol
                ),
                None,
            )
        })?;
    if !matches!(
        document.closure.state.as_str(),
        "open" | "closed" | "incomplete" | "contradicted"
    ) {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "closure state must be open, closed, incomplete or contradicted",
            None,
        ));
    }
    validate_population(protocol, population)?;
    let closure_scope_complete = validate_closure_premises(protocol, &document, population)?;

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
        if let Some(role_instance) = &observation.role_instance {
            if role_instance.authority.is_empty() || role_instance.identity.is_empty() {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    "role_instance requires nonempty authority and identity",
                    Some(position),
                ));
            }
            if !protocol
                .roles
                .iter()
                .find(|role| role.name == observation.role)
                .is_some_and(|role| role.replicated)
            {
                return Err(RecordingRefusal::new(
                    "invalid-value",
                    format!(
                        "static role `{}` cannot name a role_instance",
                        observation.role
                    ),
                    Some(position),
                ));
            }
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

    let mut obligations = assess_obligations(package, protocol, &document, closure_scope_complete);
    add_fanout_control_obligations(protocol, &document, population, &mut obligations);
    let trace_position = obligations
        .iter()
        .find(|entry| {
            entry.obligation.kind == "causal-edge"
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause == Some(RecordingCause::Misordered)
        })
        .and_then(|entry| entry.witness.first().copied())
        .or_else(|| {
            obligations
                .iter()
                .find(|entry| {
                    entry.obligation.kind == "causal-order"
                        && entry.verdict == RecordingVerdict::Violated
                })
                .and_then(|entry| entry.witness.first().copied())
        })
        .or_else(|| {
            obligations
                .iter()
                .find(|entry| entry.verdict == RecordingVerdict::Violated)
                .and_then(|entry| entry.witness.first().copied())
        });
    let any_violated = obligations
        .iter()
        .any(|entry| entry.verdict == RecordingVerdict::Violated);
    let all_satisfied = obligations
        .iter()
        .all(|entry| entry.verdict == RecordingVerdict::Satisfied);
    let (label, basis, cause, complete) = if any_violated {
        ("refuted", "decisive-counterexample", None, false)
    } else if all_satisfied && document.closure.state == "closed" && closure_scope_complete {
        ("proved", "closed-scope", None, true)
    } else {
        (
            "inconclusive",
            "unsettled",
            Some(
                if document.closure.state == "closed" && !closure_scope_complete {
                    "incomplete-closure-scope"
                } else {
                    "open-history"
                },
            ),
            false,
        )
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

fn recording_member(member: &ActivatedRoleMember) -> RecordingRoleInstance {
    RecordingRoleInstance {
        authority: member.authority.clone(),
        identity: member.identity.clone(),
    }
}

fn validate_population(
    protocol: &CheckedProtocol,
    population: Option<&[ActivatedRolePopulation]>,
) -> Result<(), RecordingRefusal> {
    let Some(population) = population else {
        return Ok(());
    };
    let on_each = matches!(
        protocol.obligations.activation,
        fsl_semantics::check::CheckedActivation::OnEach { .. }
    );
    let mut groups = BTreeSet::new();
    for group in population {
        let Some(role) = protocol.roles.iter().find(|role| role.name == group.role) else {
            return Err(RecordingRefusal::new(
                "missing-name",
                format!(
                    "activated population names undeclared role `{}`",
                    group.role
                ),
                None,
            ));
        };
        if !role.replicated {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!(
                    "static role `{}` cannot have an activated population",
                    group.role
                ),
                None,
            ));
        }
        if group.role.is_empty()
            || group.instance.as_deref().is_some_and(str::is_empty)
            || group
                .members
                .iter()
                .any(|member| member.authority.is_empty() || member.identity.is_empty())
        {
            return Err(RecordingRefusal::new(
                "invalid-value",
                "activated population role, instance, authority and identity must be nonempty",
                None,
            ));
        }
        if group.instance.is_some() != on_each {
            return Err(RecordingRefusal::new(
                "invalid-value",
                "activated population instance must match the protocol activation scope",
                None,
            ));
        }
        if !groups.insert((group.role.as_str(), group.instance.as_deref())) {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!("activated population repeats role `{}`", group.role),
                None,
            ));
        }
        let members: BTreeSet<_> = group.members.iter().collect();
        if members.len() != group.members.len() {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!(
                    "activated population repeats a member of role `{}`",
                    group.role
                ),
                None,
            ));
        }
    }
    Ok(())
}

fn validate_closure_premises(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    population: Option<&[ActivatedRolePopulation]>,
) -> Result<bool, RecordingRefusal> {
    if document.closure.state != "closed" {
        return Ok(false);
    }
    let on_each = matches!(
        protocol.obligations.activation,
        fsl_semantics::check::CheckedActivation::OnEach { .. }
    );
    let instances = if on_each {
        document
            .observations
            .iter()
            .filter_map(|observation| observation.instance.clone())
            .map(Some)
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::from([None])
    };
    if on_each
        && document
            .observations
            .iter()
            .any(|observation| observation.instance.is_none())
    {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "an on-each recording observation has no instance key",
            None,
        ));
    }

    let mut expected = BTreeSet::new();
    let mut has_unresolved_fanout_scope = false;
    let Some((_, finish)) = &protocol.finish else {
        return Err(RecordingRefusal::new(
            "established-invariant-broken",
            "the checked protocol has no finish terminal",
            None,
        ));
    };
    let finish_node = format!("{}::{finish}", protocol.name);
    for instance in &instances {
        expected.insert(RecordingPremise {
            kind: RecordingPremiseKind::Execution,
            instance: instance.clone(),
            node: finish_node.clone(),
            role_instance: None,
        });
        for control in &protocol.controls.controls {
            if !matches!(
                control.kind,
                fsl_forms::ProtocolNodeKind::Case | fsl_forms::ProtocolNodeKind::Branch
            ) {
                continue;
            }
            let node = format!("{}::{}", protocol.name, control.path.join("::"));
            let fanout_roles = protocol
                .owners
                .iter()
                .filter_map(|owner| {
                    let role = protocol.roles.get(owner.role)?;
                    if !role.replicated
                        || !protocol_node_name(protocol, owner.node.0)
                            .starts_with(&format!("{node}::"))
                    {
                        return None;
                    }
                    Some(role.name.as_str())
                })
                .collect::<BTreeSet<_>>();
            let members = document
                .observations
                .iter()
                .filter(|observation| {
                    observation.instance == *instance
                        && fanout_roles.contains(observation.role.as_str())
                        && observation.node.starts_with(&format!("{node}::"))
                })
                .collect::<Vec<_>>();
            if fanout_roles.is_empty() {
                expected.insert(RecordingPremise {
                    kind: RecordingPremiseKind::Branch,
                    instance: instance.clone(),
                    node,
                    role_instance: None,
                });
            } else {
                let mut role_instances = BTreeSet::new();
                if let Some(population) = population {
                    for role in &fanout_roles {
                        for entry in population.iter().filter(|entry| {
                            entry.role == *role && entry.instance.as_deref() == instance.as_deref()
                        }) {
                            role_instances.extend(entry.members.iter().map(recording_member));
                        }
                    }
                } else {
                    has_unresolved_fanout_scope = true;
                    for observation in members {
                        let Some(role_instance) = &observation.role_instance else {
                            return Err(RecordingRefusal::new(
                                "invalid-value",
                                format!(
                                    "fan-out observation at `{}` has no role_instance",
                                    observation.node
                                ),
                                None,
                            ));
                        };
                        role_instances.insert(role_instance.clone());
                    }
                }
                for role_instance in role_instances {
                    expected.insert(RecordingPremise {
                        kind: RecordingPremiseKind::Branch,
                        instance: instance.clone(),
                        node: node.clone(),
                        role_instance: Some(role_instance),
                    });
                }
            }
        }
    }

    let mut workflow_identities = BTreeMap::<RecordingPremise, usize>::new();
    for observation in &document.observations {
        if observation.kind != ObservationKind::Effect {
            continue;
        }
        let Some(template) =
            protocol.obligations.compensations.iter().find(|template| {
                protocol_node_name(protocol, template.effect.0) == observation.node
            })
        else {
            continue;
        };
        if !observation.keys.contains_key("registration") {
            return Err(RecordingRefusal::new(
                "invalid-value",
                format!(
                    "compensation registration at `{}` has no registration key",
                    observation.node
                ),
                None,
            ));
        }
        let identity = RecordingPremise {
            kind: RecordingPremiseKind::Workflow,
            instance: if on_each {
                observation.instance.clone()
            } else {
                None
            },
            node: observation.node.clone(),
            role_instance: None,
        };
        *workflow_identities.entry(identity.clone()).or_default() += 1;
        expected.insert(identity);
        let _ = template;
    }

    let actual = &document.closure.premises;
    let actual_set: BTreeSet<_> = actual.iter().cloned().collect();
    if actual.len() != actual_set.len() {
        return Err(RecordingRefusal::new(
            "invalid-value",
            "closure premise set contains a duplicate premise",
            None,
        ));
    }
    if actual_set != expected {
        let missing: Vec<_> = expected.difference(&actual_set).cloned().collect();
        let extra: Vec<_> = actual_set.difference(&expected).cloned().collect();
        return Err(RecordingRefusal::new(
            "invalid-value",
            format!(
                "closure premise set does not match protocol scope (missing: {missing:?}; extra: {extra:?})"
            ),
            None,
        ));
    }

    // FR-990's identity has no registration-key component. Multiple workflow
    // registrations at one checked node and instance therefore remain
    // unsettled instead of being collapsed into one apparently complete fact.
    let ambiguous_workflows = workflow_identities.values().any(|count| *count > 1);
    let unresolved_fanout =
        population.is_none() && actual.iter().any(|premise| premise.role_instance.is_some());
    Ok(!ambiguous_workflows
        && !unresolved_fanout
        && !has_unresolved_fanout_scope
        && !(on_each && instances.is_empty()))
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

fn protocol_node_ids(protocol: &CheckedProtocol) -> BTreeMap<String, ProtocolNodeId> {
    let mut nodes = BTreeMap::new();
    for control in &protocol.controls.controls {
        nodes.insert(
            format!("{}::{}", protocol.name, control.path.join("::")),
            control.node,
        );
    }
    if let Some((node, finish)) = &protocol.finish {
        nodes.insert(format!("{}::{finish}", protocol.name), *node);
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
    package: &CheckedPackage,
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    closure_scope_complete: bool,
) -> Vec<RecordingObligation> {
    let mut obligations = Vec::new();
    let closure_closed = document.closure.state == "closed";
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
                role_instance: None,
            },
            verdict: match enabled {
                Some(true) => RecordingVerdict::Satisfied,
                Some(false) => RecordingVerdict::Violated,
                None => RecordingVerdict::Pending,
            },
            cause: (enabled == Some(false)).then_some(RecordingCause::Unexpected),
            witness: vec![position],
        });
    }

    for correlation in &document.correlations {
        let send = &document.observations[correlation.send];
        let receive = &document.observations[correlation.receive];
        let correctly_correlated = correlation_matches(protocol, send, receive);
        let misordered = correctly_correlated
            && causally_after(protocol, document, correlation.send, correlation.receive);
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "causal-order".to_owned(),
                node: Some(format!("{} -> {}", send.node, receive.node)),
                instance: send.instance.clone().or_else(|| receive.instance.clone()),
                role_instance: None,
            },
            verdict: if !correctly_correlated || misordered {
                RecordingVerdict::Violated
            } else {
                RecordingVerdict::Satisfied
            },
            cause: if !correctly_correlated {
                Some(RecordingCause::WronglyCorrelated)
            } else if misordered {
                Some(RecordingCause::Misordered)
            } else {
                None
            },
            witness: if !correctly_correlated || misordered {
                vec![correlation.receive, correlation.send]
            } else {
                vec![correlation.send, correlation.receive]
            },
        });
    }

    // One obligation for every compiled FR-052 edge, independently of the
    // runtime message-correlation obligations above.
    for edge in &protocol.edges {
        if !matches!(
            edge.kind,
            fsl_semantics::check::EdgeKind::Message | fsl_semantics::check::EdgeKind::Effect
        ) {
            continue;
        }
        let source_name = protocol_node_name(protocol, edge.from.node.0);
        let target_name = protocol_node_name(protocol, edge.to.node.0);
        let instances = recording_instances(document);
        for instance in instances {
            let instance_filter = instance.as_deref();
            let sources = document
                .observations
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.node == source_name && entry.instance.as_deref() == instance_filter
                })
                .collect::<Vec<_>>();
            let targets = document
                .observations
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.node == target_name && entry.instance.as_deref() == instance_filter
                })
                .collect::<Vec<_>>();
            let pairs = match edge.kind {
                fsl_semantics::check::EdgeKind::Message => document
                    .correlations
                    .iter()
                    .filter_map(|pair| {
                        let send = document.observations.get(pair.send)?;
                        let receive = document.observations.get(pair.receive)?;
                        (send.node == source_name
                            && receive.node == target_name
                            && send.instance.as_deref() == instance_filter
                            && receive.instance.as_deref() == instance_filter)
                            .then_some((pair.send, pair.receive))
                    })
                    .collect::<Vec<_>>(),
                fsl_semantics::check::EdgeKind::Effect => targets
                    .iter()
                    .filter_map(|(target_pos, target)| {
                        sources
                            .iter()
                            .find(|(_, source)| {
                                source.keys.get("attempt") == target.keys.get("attempt")
                            })
                            .map(|(source_pos, _)| (*source_pos, *target_pos))
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            let pairs = if pairs.is_empty() && !sources.is_empty() && !targets.is_empty() {
                vec![(sources[0].0, targets[0].0)]
            } else {
                pairs
            };
            if pairs.is_empty() {
                let source_enabled =
                    control_path_enabled(protocol, document, &source_name, instance.as_deref());
                let target_enabled =
                    control_path_enabled(protocol, document, &target_name, instance.as_deref());
                if source_enabled == Some(false) || target_enabled == Some(false) {
                    continue;
                }
                let (verdict, cause) = if document.closure.state == "closed"
                    && source_enabled == Some(true)
                    && target_enabled == Some(true)
                {
                    (RecordingVerdict::Violated, Some(RecordingCause::Missing))
                } else {
                    (RecordingVerdict::Pending, None)
                };
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "causal-edge".to_owned(),
                        node: Some(format!("{source_name} -> {target_name}")),
                        instance: instance.clone(),
                        role_instance: None,
                    },
                    verdict,
                    cause,
                    witness: Vec::new(),
                });
                continue;
            }
            for (source_pos, target_pos) in pairs {
                let send_receive_mismatch = edge.kind == fsl_semantics::check::EdgeKind::Message
                    && !correlation_matches(
                        protocol,
                        &document.observations[source_pos],
                        &document.observations[target_pos],
                    );
                let misordered = causally_after(protocol, document, source_pos, target_pos);
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "causal-edge".to_owned(),
                        node: Some(format!("{source_name} -> {target_name}")),
                        instance: instance.clone(),
                        role_instance: None,
                    },
                    verdict: if send_receive_mismatch || misordered {
                        RecordingVerdict::Violated
                    } else {
                        RecordingVerdict::Satisfied
                    },
                    cause: if send_receive_mismatch {
                        Some(RecordingCause::WronglyCorrelated)
                    } else if misordered {
                        Some(RecordingCause::Misordered)
                    } else {
                        None
                    },
                    witness: if misordered || send_receive_mismatch {
                        vec![target_pos, source_pos]
                    } else {
                        vec![source_pos, target_pos]
                    },
                });
            }
        }
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
        let closure_closed = document.closure.state == "closed";
        for instance in recording_instances(document) {
            let path_enabled =
                control_path_enabled(protocol, document, &qualified, instance.as_deref());
            if path_enabled == Some(false) {
                continue;
            }
            let decisions = document
                .observations
                .iter()
                .enumerate()
                .filter(|(_, observation)| {
                    observation.kind == ObservationKind::Case
                        && observation.node.starts_with(&format!("{qualified}::"))
                        && observation.instance == instance
                })
                .collect::<Vec<_>>();
            let choice_verdict = if control.kind.label() != "choice" || decisions.is_empty() {
                None
            } else {
                let positions = decisions.iter().map(|(position, _)| *position).collect();
                let mut grouped =
                    BTreeMap::<Option<RecordingRoleInstance>, Vec<(usize, &Observation)>>::new();
                for (position, observation) in &decisions {
                    grouped
                        .entry(observation.role_instance.clone())
                        .or_default()
                        .push((*position, observation));
                }
                let choice = protocol
                    .slots
                    .choices
                    .iter()
                    .find(|choice| choice.node == control.node);
                let mut guard_failed = false;
                let mut guard_pending = false;
                for group in grouped.values() {
                    let repeated_choice_is_ambiguous = control_repeat_limit(protocol, control) > 1
                        && group
                            .iter()
                            .map(|(_, observation)| observation.node.as_str())
                            .collect::<BTreeSet<_>>()
                            .len()
                            > 1;
                    if repeated_choice_is_ambiguous {
                        guard_pending = true;
                        continue;
                    }
                    for (position, observation) in group {
                        let Some(choice) = choice else {
                            guard_failed = true;
                            continue;
                        };
                        let Some(case_name) = observation.node.rsplit("::").next() else {
                            guard_failed = true;
                            continue;
                        };
                        let Some(case) = choice.cases.iter().find(|case| case.name == case_name)
                        else {
                            guard_failed = true;
                            continue;
                        };
                        match evaluate_body(
                            package,
                            protocol,
                            document,
                            &case.guard,
                            observation.instance.as_deref(),
                            &observation.role,
                            observation.role_instance.as_ref(),
                            *position,
                        ) {
                            Some(true) => {}
                            Some(false) => guard_failed = true,
                            None => guard_pending = true,
                        }
                    }
                }
                let result = if guard_failed {
                    Some(false)
                } else if guard_pending {
                    None
                } else {
                    Some(true)
                };
                Some((positions, result))
            };
            let check_verdict = if control.kind.label() == "check" {
                let body = protocol
                    .slots
                    .bodies
                    .iter()
                    .find(|body| body.node == control.node);
                let owner_role = protocol
                    .owners
                    .iter()
                    .find(|owner| owner.node == control.node)
                    .and_then(|owner| protocol.roles.get(owner.role))
                    .map(|role| role.name.as_str());
                body.map(|body| {
                    let control_index = protocol
                        .controls
                        .controls
                        .iter()
                        .position(|candidate| candidate.node == control.node)
                        .unwrap_or_default();
                    let prior_nodes = protocol
                        .controls
                        .controls
                        .iter()
                        .take(control_index)
                        .map(|candidate| {
                            format!("{}::{}", protocol.name, candidate.path.join("::"))
                        })
                        .collect::<BTreeSet<_>>();
                    let order = causal_topological_order(protocol, document);
                    let ranks = order
                        .iter()
                        .enumerate()
                        .map(|(rank, position)| (*position, rank))
                        .collect::<BTreeMap<_, _>>();
                    let latest = document
                        .observations
                        .iter()
                        .enumerate()
                        .filter(|(_, observation)| {
                            owner_role.is_none_or(|role| observation.role == role)
                                && prior_nodes.contains(&observation.node)
                                && observation.instance == instance
                        })
                        .max_by_key(|(position, _)| ranks.get(position).copied());
                    match latest {
                        Some((position, _)) => {
                            let observation = &document.observations[position];
                            (
                                Some(position),
                                evaluate_body(
                                    package,
                                    protocol,
                                    document,
                                    &body.body,
                                    instance.as_deref(),
                                    &observation.role,
                                    observation.role_instance.as_ref(),
                                    position,
                                ),
                            )
                        }
                        None => (None, None),
                    }
                })
            } else {
                None
            };
            let repeat_verdict = if control.kind.label() == "repeat" {
                let repeat = protocol
                    .slots
                    .repeats
                    .iter()
                    .find(|repeat| repeat.node == control.node);
                repeat.map(|repeat| {
                    let descendants = protocol
                        .controls
                        .controls
                        .iter()
                        .filter(|child| {
                            child.path.len() > control.path.len()
                                && child.path.starts_with(&control.path)
                        })
                        .map(|child| format!("{}::{}", protocol.name, child.path.join("::")))
                        .collect::<BTreeSet<_>>();
                    let mut counts = BTreeMap::<String, Vec<usize>>::new();
                    for (position, observation) in document.observations.iter().enumerate() {
                        if descendants.contains(&observation.node)
                            && observation.instance == instance
                        {
                            counts
                                .entry(observation.node.clone())
                                .or_default()
                                .push(position);
                        }
                    }
                    if counts.is_empty() {
                        // A closed recording does not show whether the repeat
                        // guard was false or an iteration was omitted. Without
                        // an iteration/guard witness, absence is unsettled.
                        return (RecordingVerdict::Pending, None, Vec::new());
                    }
                    let iterations = counts.values().map(Vec::len).max().unwrap_or_default();
                    let witness = counts.into_values().flatten().collect::<Vec<_>>();
                    if iterations == 0 {
                        (RecordingVerdict::Pending, None, witness)
                    } else if repeat.max.is_some_and(|maximum| {
                        u64::try_from(iterations).is_ok_and(|count| count > maximum)
                    }) {
                        (
                            RecordingVerdict::Violated,
                            Some(RecordingCause::Duplicated),
                            witness,
                        )
                    } else {
                        (RecordingVerdict::Satisfied, None, witness)
                    }
                })
            } else {
                None
            };
            let parallel_verdict = if control.kind.label() == "parallel" {
                let parallel = protocol
                    .parallels
                    .iter()
                    .find(|parallel| parallel.node == control.node);
                parallel.map(|parallel| {
                    let branch_names = control
                        .children
                        .iter()
                        .filter_map(|child| protocol.controls.control(*child))
                        .map(|branch| format!("{}::{}", protocol.name, branch.path.join("::")))
                        .collect::<Vec<_>>();
                    let branch_states = branch_names
                        .iter()
                        .map(|branch| {
                            branch_completion(
                                protocol,
                                document,
                                &qualified,
                                branch,
                                instance.as_deref(),
                            )
                        })
                        .collect::<Vec<_>>();
                    let completed_branches = branch_states
                        .iter()
                        .filter(|completed| **completed == Some(true))
                        .count();
                    let undecided_branches = branch_states
                        .iter()
                        .filter(|completed| completed.is_none())
                        .count();
                    let policy_holds = match &parallel.policy {
                        fsl_semantics::check::CheckedJoinPolicy::All => {
                            if completed_branches == branch_names.len() {
                                Some(true)
                            } else if completed_branches + undecided_branches < branch_names.len() {
                                Some(false)
                            } else {
                                None
                            }
                        }
                        fsl_semantics::check::CheckedJoinPolicy::Any => {
                            if completed_branches > 0 {
                                Some(true)
                            } else if undecided_branches == 0 {
                                Some(false)
                            } else {
                                None
                            }
                        }
                        fsl_semantics::check::CheckedJoinPolicy::Quorum(count) => {
                            usize::try_from(*count).ok().and_then(|required| {
                                if completed_branches >= required {
                                    Some(true)
                                } else if completed_branches + undecided_branches < required {
                                    Some(false)
                                } else {
                                    None
                                }
                            })
                        }
                        fsl_semantics::check::CheckedJoinPolicy::Predicate { function } => {
                            let mut labels = parallel.joined.clone();
                            labels.sort();
                            let arguments = labels
                                .iter()
                                .map(|label| {
                                    let branch = control
                                        .children
                                        .iter()
                                        .filter_map(|child| protocol.controls.control(*child))
                                        .find(|branch| branch.name == *label)?;
                                    let branch_qualified =
                                        format!("{}::{}", protocol.name, branch.path.join("::"));
                                    branch_completion(
                                        protocol,
                                        document,
                                        &qualified,
                                        &branch_qualified,
                                        instance.as_deref(),
                                    )
                                    .map(Value::Boolean)
                                })
                                .collect::<Option<Vec<_>>>();
                            arguments.and_then(|arguments| {
                                fsl_eval::value::evaluate_join_predicate(
                                    package,
                                    function,
                                    arguments,
                                    &mut recording_meter(),
                                )
                                .ok()
                                .and_then(|evaluation| {
                                    match evaluation.outcome {
                                        FamilyOutcome::Evaluated(Outcome::Completed(
                                            Value::Boolean(value),
                                        )) => Some(value),
                                        _ => None,
                                    }
                                })
                            })
                        }
                    };
                    if policy_holds == Some(true) {
                        (RecordingVerdict::Satisfied, None, Vec::new())
                    } else if policy_holds == Some(false) && closure_closed {
                        (
                            RecordingVerdict::Violated,
                            Some(RecordingCause::Missing),
                            Vec::new(),
                        )
                    } else {
                        (RecordingVerdict::Pending, None, Vec::new())
                    }
                })
            } else {
                None
            };
            let (verdict, cause, witness) = match choice_verdict {
                Some((positions, Some(true))) => (RecordingVerdict::Satisfied, None, positions),
                Some((positions, Some(false))) => (
                    RecordingVerdict::Violated,
                    Some(RecordingCause::Unexpected),
                    positions,
                ),
                Some((positions, None)) => (RecordingVerdict::Pending, None, positions),
                None if check_verdict.is_some() => match check_verdict.unwrap_or((None, None)) {
                    (Some(position), Some(true)) => {
                        (RecordingVerdict::Satisfied, None, vec![position])
                    }
                    (Some(position), Some(false)) => (
                        RecordingVerdict::Violated,
                        Some(RecordingCause::Unexpected),
                        vec![position],
                    ),
                    (Some(position), None) => (RecordingVerdict::Pending, None, vec![position]),
                    (None, _) if closure_closed && path_enabled == Some(true) => (
                        RecordingVerdict::Violated,
                        Some(RecordingCause::Missing),
                        Vec::new(),
                    ),
                    (None, _) => (RecordingVerdict::Pending, None, Vec::new()),
                },
                None if repeat_verdict.is_some() => {
                    repeat_verdict.unwrap_or((RecordingVerdict::Pending, None, Vec::new()))
                }
                None if parallel_verdict.is_some() => {
                    parallel_verdict.unwrap_or((RecordingVerdict::Pending, None, Vec::new()))
                }
                None if closure_closed && path_enabled == Some(true) => (
                    RecordingVerdict::Violated,
                    Some(RecordingCause::Missing),
                    Vec::new(),
                ),
                None => (RecordingVerdict::Pending, None, Vec::new()),
            };
            obligations.push(RecordingObligation {
                obligation: RecordingObligationId {
                    kind: if control.kind.label() == "parallel" {
                        "join"
                    } else {
                        "control"
                    }
                    .to_owned(),
                    node: Some(qualified.clone()),
                    instance: instance.clone(),
                    role_instance: None,
                },
                verdict,
                cause,
                witness,
            });
        }
    }

    for template in &protocol.obligations.compensations {
        let effect_name = protocol_node_name(protocol, template.effect.0);
        let attempts: Vec<_> = document
            .observations
            .iter()
            .enumerate()
            .filter(|(_, observation)| {
                observation.kind == ObservationKind::CompensationAttempt
                    && observation.node == effect_name
            })
            .collect();
        for (position, observation) in attempts {
            let reading = observation
                .clocks
                .get(&template.clock)
                .and_then(json_integer);
            let (verdict, cause) = match reading {
                Some(value) if value >= template.within.lower && value <= template.within.upper => {
                    (RecordingVerdict::Satisfied, None)
                }
                Some(_) => (RecordingVerdict::Violated, Some(RecordingCause::Unexpected)),
                None if closure_closed => {
                    (RecordingVerdict::Violated, Some(RecordingCause::Missing))
                }
                None => (RecordingVerdict::Pending, None),
            };
            obligations.push(RecordingObligation {
                obligation: RecordingObligationId {
                    kind: "window".to_owned(),
                    node: Some(effect_name.clone()),
                    instance: observation.instance.clone(),
                    role_instance: None,
                },
                verdict,
                cause,
                witness: vec![position],
            });
        }
    }

    for await_clause in &protocol.awaits {
        let node = protocol_node_name(protocol, await_clause.node.0);
        for instance in recording_instances(document) {
            let path_enabled = control_path_enabled(protocol, document, &node, instance.as_deref());
            if path_enabled == Some(false) {
                continue;
            }
            let observed: Vec<_> = document
                .observations
                .iter()
                .enumerate()
                .filter(|(_, observation)| {
                    (observation.node == node || observation.node.starts_with(&format!("{node}::")))
                        && observation.instance == instance
                })
                .collect();
            if observed.is_empty() {
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "window".to_owned(),
                        node: Some(node.clone()),
                        instance: instance.clone(),
                        role_instance: None,
                    },
                    verdict: if closure_closed && path_enabled == Some(true) {
                        RecordingVerdict::Violated
                    } else {
                        RecordingVerdict::Pending
                    },
                    cause: (closure_closed && path_enabled == Some(true))
                        .then_some(RecordingCause::Missing),
                    witness: Vec::new(),
                });
            }
            for (position, observation) in observed {
                let reading = observation
                    .clocks
                    .get(&await_clause.clock)
                    .and_then(json_integer);
                let (verdict, cause) = match reading {
                    Some(value)
                        if value >= await_clause.within.lower
                            && value <= await_clause.within.upper =>
                    {
                        (RecordingVerdict::Satisfied, None)
                    }
                    Some(_) => (RecordingVerdict::Violated, Some(RecordingCause::Unexpected)),
                    None if closure_closed => {
                        (RecordingVerdict::Violated, Some(RecordingCause::Missing))
                    }
                    None => (RecordingVerdict::Pending, None),
                };
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "window".to_owned(),
                        node: Some(node.clone()),
                        instance: observation.instance.clone(),
                        role_instance: None,
                    },
                    verdict,
                    cause,
                    witness: vec![position],
                });
            }
        }
    }

    for template in &protocol.obligations.compensations {
        let effect_name = protocol_node_name(protocol, template.effect.0);
        let mut registrations = BTreeMap::<
            (Option<String>, String),
            (serde_json::Value, Option<String>, Vec<usize>),
        >::new();
        for observation in &document.observations {
            if observation.kind != ObservationKind::Effect || observation.node != effect_name {
                continue;
            }
            let Some(registration) = observation.keys.get("registration") else {
                continue;
            };
            registrations
                .entry((observation.instance.clone(), registration.to_string()))
                .or_insert_with(|| {
                    (
                        registration.clone(),
                        observation.instance.clone(),
                        Vec::new(),
                    )
                });
        }
        for (position, observation) in document.observations.iter().enumerate() {
            if observation.kind != ObservationKind::CompensationAttempt
                || observation.node != effect_name
            {
                continue;
            }
            let Some(registration) = observation.keys.get("registration") else {
                continue;
            };
            registrations
                .entry((observation.instance.clone(), registration.to_string()))
                .or_insert_with(|| {
                    (
                        registration.clone(),
                        observation.instance.clone(),
                        Vec::new(),
                    )
                })
                .2
                .push(position);
        }
        for (_, (registration, instance, attempts)) in registrations {
            let effect_position = document
                .observations
                .iter()
                .enumerate()
                .find(|(_, observation)| {
                    observation.kind == ObservationKind::Effect
                        && observation.instance == instance
                        && observation.keys.get("registration") == Some(&registration)
                        && observation.node == effect_name
                })
                .map(|(position, _)| position);
            let outcomes_valid = attempts.iter().all(|position| {
                matches!(
                    document.observations[*position].outcome.as_deref(),
                    Some("successful" | "failed" | "timed-out")
                )
            });
            let after_effect = effect_position.is_some_and(|effect| {
                attempts
                    .iter()
                    .all(|attempt| causally_after(protocol, document, *attempt, effect))
            });
            let within_limit =
                u64::try_from(attempts.len()).is_ok_and(|count| count <= template.maximum_attempts);
            let commit_position = match template.commit {
                fsl_semantics::check::CheckedCommit::Never => None,
                fsl_semantics::check::CheckedCommit::Node(node) => {
                    let commit_name = protocol_node_name(protocol, node.0);
                    document
                        .observations
                        .iter()
                        .enumerate()
                        .find(|(_, observation)| {
                            observation.kind == ObservationKind::Commit
                                && observation.node == commit_name
                                && observation.instance == instance
                        })
                        .map(|(position, _)| position)
                }
            };
            let before_commit = commit_position.is_none_or(|commit| {
                attempts
                    .iter()
                    .all(|attempt| causally_after(protocol, document, commit, *attempt))
            });
            let recoveries = document
                .observations
                .iter()
                .enumerate()
                .filter(|(_, observation)| {
                    observation.kind == ObservationKind::Recovery
                        && observation.instance == instance
                        && observation.keys.get("registration") == Some(&registration)
                })
                .collect::<Vec<_>>();
            let recovery_valid = recoveries.iter().all(|(_, recovery)| {
                matches!(
                    recovery.outcome.as_deref(),
                    Some("successful" | "failed" | "timed-out")
                )
            });
            let settled = !recoveries.is_empty();
            let valid =
                outcomes_valid && after_effect && within_limit && before_commit && recovery_valid;
            let witness = attempts
                .iter()
                .copied()
                .chain(recoveries.iter().map(|(position, _)| *position))
                .collect();
            obligations.push(RecordingObligation {
                obligation: RecordingObligationId {
                    kind: "compensation".to_owned(),
                    node: Some(effect_name.clone()),
                    instance,
                    role_instance: None,
                },
                verdict: if valid && settled {
                    RecordingVerdict::Satisfied
                } else if valid && document.closure.state != "closed" {
                    RecordingVerdict::Pending
                } else {
                    RecordingVerdict::Violated
                },
                cause: if valid && !settled && document.closure.state == "closed" {
                    Some(RecordingCause::Missing)
                } else if !valid {
                    Some(if !outcomes_valid || !recovery_valid || !before_commit {
                        RecordingCause::Unexpected
                    } else if within_limit {
                        RecordingCause::Misordered
                    } else {
                        RecordingCause::Duplicated
                    })
                } else {
                    None
                },
                witness,
            });
        }
    }

    for requirement in &protocol.requirements {
        let temporal = package.graph().temporal_clauses().get(requirement.temporal);
        let result = temporal.and_then(|clause| {
            let subject = clause.clause().over().1.clone();
            let types = package.graph().scope().types();
            let samples = causal_topological_order(protocol, document)
                .into_iter()
                .filter_map(|position| {
                    document.observations[position]
                        .record
                        .as_ref()
                        .and_then(|record| recording_value(record, &subject, types))
                })
                .collect::<Vec<_>>();
            if samples.is_empty() {
                return None;
            }
            let packages = fsl_semantics::model::intake::PackageInput::new();
            let snapshots = fsl_semantics::model::observation::DocumentProvision::new();
            let sources = [];
            let request = crate::monitor::MonitorRequest {
                clauses: vec![crate::monitor::MonitorClause {
                    name: clause.clause().name().to_owned(),
                    over: None,
                }],
                trace: crate::monitor::MonitorTrace::Samples {
                    subject,
                    samples: &samples,
                },
                snapshots: &snapshots,
                packages: &packages,
                sources: &sources,
                temporal_selections: &[],
            };
            let limits = crate::monitor::MonitorLimits {
                observation: fsl_semantics::model::observation::ObservationLimits::default(),
                normalization: fsl_semantics::model::accounting::ModelNormalizationLimits::default(
                ),
                accounting: ScalarLimits {
                    integer_bits: u64::MAX,
                    decimal_digits: u64::MAX,
                    scale_expansion: u64::MAX,
                    text_input_bytes: u64::MAX,
                    text_scalars: u64::MAX,
                    normalized_scalars: u64::MAX,
                    unit_edges: u64::MAX,
                    value_occurrences: u64::MAX,
                    work_units: u64::MAX,
                    result_units: u64::MAX,
                },
                max_window_values: crate::monitor::DEFAULT_MAX_WINDOW_VALUES,
                retained_positions: crate::monitor::DEFAULT_RETAINED_POSITIONS,
                retained_identities: crate::monitor::DEFAULT_RETAINED_IDENTITIES,
            };
            let outcome =
                crate::monitor::monitor(package, &request, &limits, &filament_exact::Cancel::new())
                    .ok()?
                    .into_value();
            outcome.records.first().map(|record| record.verdict)
        });
        let (verdict, cause) = match result {
            Some(crate::monitor::MonitorVerdict::Tested) => (RecordingVerdict::Satisfied, None),
            Some(
                crate::monitor::MonitorVerdict::Violated
                | crate::monitor::MonitorVerdict::Undefined,
            ) => (RecordingVerdict::Violated, Some(RecordingCause::Unexpected)),
            _ if document.closure.state == "closed" => {
                (RecordingVerdict::Violated, Some(RecordingCause::Missing))
            }
            _ => (RecordingVerdict::Pending, None),
        };
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "requirement".to_owned(),
                node: Some(format!("temporal:{}", requirement.temporal)),
                instance: None,
                role_instance: None,
            },
            verdict,
            cause,
            witness: Vec::new(),
        });
    }

    for channel in &protocol.channels {
        let mut sends = document
            .observations
            .iter()
            .enumerate()
            .filter(|(_, observation)| {
                observation.kind == ObservationKind::Send
                    && channel_for_send(protocol, observation) == Some(channel.name.as_str())
            })
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        let mut receives = document
            .observations
            .iter()
            .enumerate()
            .filter(|(_, observation)| {
                observation.kind == ObservationKind::Receive
                    && channel_for_send(protocol, observation) == Some(channel.name.as_str())
            })
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        sends.sort_unstable();
        receives.sort_unstable();
        let mut delivered = BTreeMap::<usize, Vec<usize>>::new();
        for pair in &document.correlations {
            if sends.contains(&pair.send) && receives.contains(&pair.receive) {
                delivered.entry(pair.send).or_default().push(pair.receive);
            }
        }
        let mut violated = None;
        let mut witness = Vec::new();
        let mut undecided = false;
        let (minimum, maximum) = match channel.delivery {
            ChannelDelivery::Unknown => (None, None),
            ChannelDelivery::AtMostOnce => (Some(0), Some(1)),
            ChannelDelivery::AtLeastOnce => (Some(1), None),
            ChannelDelivery::ExactlyOncePremise => (Some(1), Some(1)),
            ChannelDelivery::Between { lower, upper } => (Some(lower), Some(upper)),
        };
        if violated.is_none() {
            if let fsl_semantics::check::CheckedOrdering::Fifo { key, .. } = &channel.ordering {
                let mut keyed_pairs = BTreeMap::<String, Vec<(usize, usize)>>::new();
                for (send, delivered_receives) in &delivered {
                    let value = evaluate_body_value(
                        package,
                        protocol,
                        document,
                        key,
                        document.observations[*send].instance.as_deref(),
                        &document.observations[*send].role,
                        document.observations[*send].role_instance.as_ref(),
                        *send,
                    )
                    .map(|value| format!("key:{value:?}"))
                    .or_else(|| {
                        // The key expression is pure. Identical recorded
                        // payloads therefore prove equal keys even when a
                        // channel binder is unavailable in this assessor's
                        // protocol-slot frame; differing payloads stay
                        // undecided rather than being grouped speculatively.
                        undecided = true;
                        document.observations[*send]
                            .record
                            .as_ref()
                            .map(|record| format!("record:{record}"))
                    });
                    let Some(value) = value else { continue };
                    for receive in delivered_receives {
                        keyed_pairs
                            .entry(value.clone())
                            .or_default()
                            .push((*send, *receive));
                    }
                }
                'fifo: for pairs in keyed_pairs.values() {
                    for (first_index, (first_send, first_receive)) in pairs.iter().enumerate() {
                        for (second_send, second_receive) in pairs.iter().skip(first_index + 1) {
                            if causally_after(protocol, document, *second_send, *first_send)
                                && causally_after(
                                    protocol,
                                    document,
                                    *first_receive,
                                    *second_receive,
                                )
                            {
                                violated = Some(RecordingCause::Misordered);
                                witness.extend([*second_receive, *first_receive]);
                                break 'fifo;
                            }
                            if causally_after(protocol, document, *first_send, *second_send)
                                && causally_after(
                                    protocol,
                                    document,
                                    *second_receive,
                                    *first_receive,
                                )
                            {
                                violated = Some(RecordingCause::Misordered);
                                witness.extend([*first_receive, *second_receive]);
                                break 'fifo;
                            }
                        }
                    }
                }
            }
        }
        let mut dropped = BTreeSet::<usize>::new();
        if violated.is_none() {
            let order = causal_topological_order(protocol, document);
            let mut queue = Vec::<usize>::new();
            let mut max_occupancy = 0usize;
            for position in order {
                if sends.contains(&position) {
                    let capacity = match channel.capacity {
                        fsl_semantics::check::ChannelCapacity::Literal(value) => {
                            usize::try_from(value).ok()
                        }
                        fsl_semantics::check::ChannelCapacity::Symbolic(_) => None,
                    };
                    if let Some(capacity) = capacity {
                        if queue.len() >= capacity {
                            match channel.overflow {
                                fsl_semantics::check::ChannelOverflow::Reject
                                | fsl_semantics::check::ChannelOverflow::Block => {
                                    violated = Some(RecordingCause::Unexpected);
                                    witness.push(position);
                                    break;
                                }
                                fsl_semantics::check::ChannelOverflow::Loss(
                                    fsl_semantics::check::LossSelector::Head,
                                ) => {
                                    if let Some(oldest) = queue.first().copied() {
                                        dropped.insert(oldest);
                                        queue.remove(0);
                                    } else {
                                        dropped.insert(position);
                                        continue;
                                    }
                                }
                                fsl_semantics::check::ChannelOverflow::Loss(
                                    fsl_semantics::check::LossSelector::Tail,
                                ) => {
                                    if let Some(newest) = queue.pop() {
                                        dropped.insert(newest);
                                    }
                                }
                                fsl_semantics::check::ChannelOverflow::Loss(
                                    fsl_semantics::check::LossSelector::New,
                                ) => {
                                    dropped.insert(position);
                                    continue;
                                }
                            }
                        }
                    }
                    queue.push(position);
                    max_occupancy = max_occupancy.max(queue.len());
                } else if receives.contains(&position) {
                    let Some(send) = document
                        .correlations
                        .iter()
                        .find(|pair| pair.receive == position && sends.contains(&pair.send))
                        .map(|pair| pair.send)
                    else {
                        continue;
                    };
                    if dropped.contains(&send) {
                        violated = Some(RecordingCause::WronglyCorrelated);
                        witness.extend([send, position]);
                        break;
                    }
                    if let Some(index) = queue.iter().position(|queued| *queued == send) {
                        queue.remove(index);
                    }
                }
            }
            if violated.is_none()
                && matches!(
                    channel.capacity,
                    fsl_semantics::check::ChannelCapacity::Symbolic(_)
                )
                && max_occupancy > 0
            {
                violated = None;
            }
        }
        if violated.is_none() {
            if let Some(minimum) = minimum {
                for send in &sends {
                    let count = delivered.get(send).map_or(0, |items| {
                        items
                            .iter()
                            .filter_map(|position| {
                                document.observations[*position].keys.get("delivery")
                            })
                            .map(serde_json::Value::to_string)
                            .collect::<BTreeSet<_>>()
                            .len()
                    });
                    let count = u64::try_from(count).unwrap_or(u64::MAX);
                    if count < minimum && !dropped.contains(send) {
                        if document.closure.state == "closed" {
                            violated = Some(RecordingCause::Missing);
                            witness.push(*send);
                            break;
                        }
                        undecided = true;
                    }
                    if maximum.is_some_and(|maximum| count > maximum) {
                        violated = Some(RecordingCause::Duplicated);
                        witness.extend(delivered.get(send).into_iter().flatten().copied());
                        break;
                    }
                }
            }
        }
        let duplicate_delivery = duplicate_deliveries(protocol, document)
            .into_iter()
            .filter(|duplicate| duplicate.channel == channel.name)
            .collect::<Vec<_>>();
        if violated.is_none()
            && duplicate_delivery.iter().any(|duplicate| {
                matches!(
                    duplicate.policy,
                    ChannelDelivery::AtMostOnce
                        | ChannelDelivery::ExactlyOncePremise
                        | ChannelDelivery::Between { upper: 0..=1, .. }
                )
            })
        {
            violated = Some(RecordingCause::Duplicated);
            witness.extend(
                duplicate_delivery
                    .iter()
                    .flat_map(|duplicate| duplicate.receive_positions.iter().copied()),
            );
        }
        let (verdict, cause) = if let Some(cause) = violated {
            (RecordingVerdict::Violated, Some(cause))
        } else if undecided
            || channel.delivery == ChannelDelivery::Unknown
            || matches!(
                channel.capacity,
                fsl_semantics::check::ChannelCapacity::Symbolic(_)
            )
        {
            (RecordingVerdict::Pending, None)
        } else {
            (RecordingVerdict::Satisfied, None)
        };
        obligations.push(RecordingObligation {
            obligation: RecordingObligationId {
                kind: "channel-premise".to_owned(),
                node: Some(channel.name.clone()),
                instance: None,
                role_instance: None,
            },
            verdict,
            cause,
            witness,
        });
    }

    let instances = recording_instances(document);
    for body in &protocol.slots.bodies {
        let node = protocol_node_name(protocol, body.node.0);
        for instance in &instances {
            let path_enabled = control_path_enabled(protocol, document, &node, instance.as_deref());
            if path_enabled == Some(false) {
                continue;
            }
            let positions = observed_nodes
                .get(&node)
                .into_iter()
                .flatten()
                .copied()
                .filter(|position| document.observations[*position].instance == *instance)
                .collect::<Vec<_>>();
            if positions.is_empty() {
                let (verdict, cause) =
                    if document.closure.state == "closed" && path_enabled == Some(true) {
                        (RecordingVerdict::Violated, Some(RecordingCause::Missing))
                    } else {
                        (RecordingVerdict::Pending, None)
                    };
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "constraint".to_owned(),
                        node: Some(node.clone()),
                        instance: instance.clone(),
                        role_instance: None,
                    },
                    verdict,
                    cause,
                    witness: Vec::new(),
                });
                continue;
            }
            for position in positions {
                let observation = &document.observations[position];
                let (verdict, cause) = match evaluate_body(
                    package,
                    protocol,
                    document,
                    &body.body,
                    observation.instance.as_deref(),
                    &observation.role,
                    observation.role_instance.as_ref(),
                    position,
                ) {
                    Some(true) => (RecordingVerdict::Satisfied, None),
                    Some(false) => (RecordingVerdict::Violated, Some(RecordingCause::Unexpected)),
                    None => (RecordingVerdict::Pending, None),
                };
                obligations.push(RecordingObligation {
                    obligation: RecordingObligationId {
                        kind: "constraint".to_owned(),
                        node: Some(node.clone()),
                        instance: instance.clone(),
                        role_instance: None,
                    },
                    verdict,
                    cause,
                    witness: vec![position],
                });
            }
        }
    }

    let instances = recording_instances(document);
    for instance in instances {
        let finish = protocol.finish.as_ref().and_then(|(_, name)| {
            let qualified = format!("{}::{}", protocol.name, name);
            observed_nodes.get(&qualified)
        });
        let finish_enabled = protocol.finish.as_ref().map(|_| true).unwrap_or(false);
        let finish_seen = finish.is_some_and(|positions| {
            positions.iter().any(|position| {
                document.observations[*position].kind == ObservationKind::Finish
                    && document.observations[*position].instance == instance
            })
        });
        let (verdict, cause, witness) = if finish_seen
            && document.closure.state == "closed"
            && finish_enabled
            && closure_scope_complete
        {
            (
                RecordingVerdict::Satisfied,
                None,
                finish.cloned().unwrap_or_default(),
            )
        } else if document.closure.state == "closed" && finish_enabled && closure_scope_complete {
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
                instance,
                role_instance: None,
            },
            verdict,
            cause,
            witness,
        });
    }
    obligations
}

fn add_fanout_control_obligations(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    population: Option<&[ActivatedRolePopulation]>,
    obligations: &mut Vec<RecordingObligation>,
) {
    let on_each = matches!(
        protocol.obligations.activation,
        fsl_semantics::check::CheckedActivation::OnEach { .. }
    );
    let instances = recording_instances(document);
    for control in &protocol.controls.controls {
        if control.kind != fsl_forms::ProtocolNodeKind::Branch {
            continue;
        }
        let node = format!("{}::{}", protocol.name, control.path.join("::"));
        let fanout_roles = protocol
            .owners
            .iter()
            .filter_map(|owner| {
                let role = protocol.roles.get(owner.role)?;
                (role.replicated
                    && protocol_node_name(protocol, owner.node.0).starts_with(&format!("{node}::")))
                .then_some(role.name.as_str())
            })
            .collect::<BTreeSet<_>>();
        for instance in &instances {
            for role in &fanout_roles {
                let mut members = BTreeSet::new();
                if let Some(population) = population {
                    for group in population.iter().filter(|group| {
                        group.role == *role && group.instance.as_deref() == instance.as_deref()
                    }) {
                        members.extend(group.members.iter().map(recording_member));
                    }
                } else {
                    members.extend(
                        document
                            .observations
                            .iter()
                            .filter(|observation| {
                                observation.instance == *instance
                                    && observation.role == *role
                                    && observation.node.starts_with(&format!("{node}::"))
                            })
                            .filter_map(|observation| observation.role_instance.clone()),
                    );
                }
                for member in members {
                    let observed = document
                        .observations
                        .iter()
                        .enumerate()
                        .filter(|(_, observation)| {
                            observation.instance == *instance
                                && observation.role == *role
                                && observation.role_instance.as_ref() == Some(&member)
                                && observation.node.starts_with(&format!("{node}::"))
                        })
                        .map(|(position, _)| position)
                        .collect::<Vec<_>>();
                    let (verdict, cause) = if population.is_none() {
                        (RecordingVerdict::Pending, None)
                    } else if !observed.is_empty() {
                        (RecordingVerdict::Satisfied, None)
                    } else if document.closure.state == "closed" && population.is_some() {
                        (RecordingVerdict::Violated, Some(RecordingCause::Missing))
                    } else {
                        (RecordingVerdict::Pending, None)
                    };
                    obligations.push(RecordingObligation {
                        obligation: RecordingObligationId {
                            kind: "control".to_owned(),
                            node: Some(node.clone()),
                            instance: on_each.then(|| instance.clone()).flatten(),
                            role_instance: Some(member),
                        },
                        verdict,
                        cause,
                        witness: observed,
                    });
                }
            }
        }
    }
}

fn recording_instances(document: &RecordingDocument) -> BTreeSet<Option<String>> {
    let mut instances = document
        .observations
        .iter()
        .map(|observation| observation.instance.clone())
        .collect::<BTreeSet<_>>();
    if instances.is_empty() {
        instances.insert(None);
    }
    instances
}

/// Whether `later` is constrained to follow `earlier` by local orders and
/// message correlations. Observation-array position contributes no edge.
fn causally_after(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    later: usize,
    earlier: usize,
) -> bool {
    let mut edges = vec![Vec::new(); document.observations.len()];
    for order in document.local_order.values() {
        for pair in order.windows(2) {
            if let [from, to] = pair {
                edges[*from].push(*to);
            }
        }
    }
    for pair in &document.correlations {
        let send = &document.observations[pair.send];
        let receive = &document.observations[pair.receive];
        if correlation_matches(protocol, send, receive) {
            edges[pair.send].push(pair.receive);
        }
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

fn causal_topological_order(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
) -> Vec<usize> {
    let mut edges = vec![BTreeSet::new(); document.observations.len()];
    let mut indegree = vec![0usize; document.observations.len()];
    for order in document.local_order.values() {
        for pair in order.windows(2) {
            if let [from, to] = pair {
                if edges[*from].insert(*to) {
                    indegree[*to] += 1;
                }
            }
        }
    }
    for pair in &document.correlations {
        let send = &document.observations[pair.send];
        let receive = &document.observations[pair.receive];
        if correlation_matches(protocol, send, receive) && edges[pair.send].insert(pair.receive) {
            indegree[pair.receive] += 1;
        }
    }
    let sort_key = |position: usize| {
        let observation = &document.observations[position];
        (
            observation
                .instance
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
            observation.role.clone(),
            observation.node.clone(),
            format!("{:?}", observation.keys),
        )
    };
    let mut ready = BTreeSet::new();
    for (position, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.insert((sort_key(position), position));
        }
    }
    let mut ordered = Vec::with_capacity(document.observations.len());
    while let Some((key, position)) = ready.pop_first() {
        let _ = key;
        ordered.push(position);
        for target in edges[position].iter().copied() {
            indegree[target] -= 1;
            if indegree[target] == 0 {
                ready.insert((sort_key(target), target));
            }
        }
    }
    ordered
}

fn correlation_matches(
    protocol: &CheckedProtocol,
    send: &Observation,
    receive: &Observation,
) -> bool {
    let same_message = send.keys.get("message").is_some()
        && send.keys.get("message") == receive.keys.get("message");
    let same_instance = send.instance.is_some() && send.instance == receive.instance;
    let send_channel = channel_for_send(protocol, send);
    let receive_channel = channel_for_send(protocol, receive);
    same_message && same_instance && send_channel.is_some() && send_channel == receive_channel
}

fn json_integer(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
}

fn recording_value(
    value: &serde_json::Value,
    value_type: &ValueType,
    types: &filament_semantic_value::declaration::TypeEnvironment,
) -> Option<Value> {
    match (value, value_type) {
        (serde_json::Value::Bool(value), ValueType::Boolean) => Some(Value::Boolean(*value)),
        (serde_json::Value::Number(value), ValueType::Integer | ValueType::Int(_)) => value
            .as_i64()
            .map(|value| Value::Integer(Integer::from(value)))
            .or_else(|| {
                value
                    .as_u64()
                    .map(|value| Value::Integer(Integer::from(value)))
            }),
        (serde_json::Value::String(value), ValueType::String) => Some(Value::String(value.clone())),
        (serde_json::Value::Null, ValueType::Option(payload)) => {
            Some(filament_exact::OptionValue::none((**payload).clone()))
        }
        (value, ValueType::Option(payload)) => {
            let value = recording_value(value, payload, types)?;
            filament_exact::OptionValue::present((**payload).clone(), value).ok()
        }
        (serde_json::Value::Object(values), ValueType::Composite(key)) => {
            let declaration = types.composite(key.clone())?;
            let filament_semantic_value::declaration::CompositeShape::Record(fields) =
                declaration.shape()
            else {
                return None;
            };
            if values
                .keys()
                .any(|name| !fields.iter().any(|field| field.name() == name))
            {
                return None;
            }
            let mut supplied = Vec::new();
            for field in fields {
                let Some(member) = values.get(field.name()) else {
                    continue;
                };
                let value = recording_value(member, field.value_type(), types)?;
                supplied.push((field.name(), FieldValue::Present(value)));
            }
            types.record(key.clone(), supplied).ok()
        }
        _ => None,
    }
}

fn recording_meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    })
}

fn evaluate_body_value(
    package: &CheckedPackage,
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    body: &fsl_semantics::check::CheckedBody,
    instance: Option<&str>,
    role: &str,
    role_instance: Option<&RecordingRoleInstance>,
    before: usize,
) -> Option<Value> {
    let types = package.graph().scope().types();
    let order = causal_topological_order(protocol, document);
    let ranks = order
        .iter()
        .enumerate()
        .map(|(rank, position)| (*position, rank))
        .collect::<BTreeMap<_, _>>();
    let before_rank = *ranks.get(&before)?;
    let mut frame = vec![None; protocol.slots.binders.len()];
    for (slot, binder) in protocol.slots.binders.iter().enumerate() {
        let Some(node) = binder.node else {
            continue;
        };
        let binder_role = protocol
            .owners
            .iter()
            .find(|owner| owner.node == node)
            .and_then(|owner| protocol.roles.get(owner.role));
        let binder_is_replicated = binder_role.is_some_and(|owner| owner.replicated);
        if binder_is_replicated && binder_role.is_some_and(|owner| owner.name != role) {
            return None;
        }
        let binder_role_instance = if binder_is_replicated {
            Some(role_instance?)
        } else {
            None
        };
        let node_name = protocol_node_name(protocol, node.0);
        let source_position = order
            .iter()
            .copied()
            .filter(|position| ranks.get(position).is_some_and(|rank| *rank <= before_rank))
            .filter(|position| {
                let observation = &document.observations[*position];
                observation.node == node_name
                    && observation.instance.as_deref() == instance
                    && binder_role.is_none_or(|owner| observation.role == owner.name)
                    && (!binder_is_replicated
                        || observation.role_instance.as_ref() == binder_role_instance)
            })
            .max_by_key(|position| ranks.get(position).copied());
        let Some(source) = source_position.map(|position| &document.observations[position]) else {
            continue;
        };
        let Some(record) = source.record.as_ref() else {
            continue;
        };
        frame[slot] = recording_value(record, &binder.value_type, types);
    }
    let mut meter = Meter::new(ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    });
    let evaluation = fsl_eval::value::evaluate_protocol_slot(
        package,
        body,
        frame,
        &ObjectEnvironment::default(),
        &mut meter,
    )
    .ok()?;
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => Some(value),
        _ => None,
    }
}

fn evaluate_body(
    package: &CheckedPackage,
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    body: &fsl_semantics::check::CheckedBody,
    instance: Option<&str>,
    role: &str,
    role_instance: Option<&RecordingRoleInstance>,
    before: usize,
) -> Option<bool> {
    match evaluate_body_value(
        package,
        protocol,
        document,
        body,
        instance,
        role,
        role_instance,
        before,
    )? {
        Value::Boolean(value) => Some(value),
        _ => None,
    }
}

fn branch_completion(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    parallel_node: &str,
    branch_node: &str,
    instance: Option<&str>,
) -> Option<bool> {
    let premise_instance = matches!(
        protocol.obligations.activation,
        fsl_semantics::check::CheckedActivation::OnEach { .. }
    )
    .then_some(instance)
    .flatten();
    let fanout_roles = protocol
        .owners
        .iter()
        .filter_map(|owner| {
            let role = protocol.roles.get(owner.role)?;
            if !role.replicated
                || !protocol_node_name(protocol, owner.node.0)
                    .starts_with(&format!("{branch_node}::"))
            {
                return None;
            }
            Some(role.name.as_str())
        })
        .collect::<BTreeSet<_>>();

    if fanout_roles.is_empty() {
        return Some(document.closure.premises.iter().any(|premise| {
            premise.kind == RecordingPremiseKind::Branch
                && premise.instance.as_deref() == premise_instance
                && premise.node == branch_node
                && premise.role_instance.is_none()
        }));
    }

    let mut members = BTreeSet::new();
    for observation in document.observations.iter().filter(|observation| {
        observation.instance.as_deref() == instance
            && fanout_roles.contains(observation.role.as_str())
            && observation.node.starts_with(&format!("{parallel_node}::"))
    }) {
        let Some(role_instance) = &observation.role_instance else {
            return None;
        };
        members.insert(role_instance.clone());
    }
    if members.is_empty() {
        return None;
    }

    Some(members.iter().all(|role_instance| {
        document.closure.premises.iter().any(|premise| {
            premise.kind == RecordingPremiseKind::Branch
                && premise.instance.as_deref() == premise_instance
                && premise.node == branch_node
                && premise.role_instance.as_ref() == Some(role_instance)
        })
    }))
}

/// Whether a recorded action has the branch decision required by its checked
/// control ancestry before it occurs.
fn observation_enabled(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    position: usize,
) -> Option<bool> {
    let observation = &document.observations[position];
    if !observation_owned_by(protocol, observation) {
        return Some(false);
    }
    if observation.kind == ObservationKind::Finish {
        let finish_is_declared = protocol
            .finish
            .as_ref()
            .is_some_and(|(_, name)| format!("{}::{}", protocol.name, name) == observation.node);
        let occurrences = document
            .observations
            .iter()
            .filter(|entry| {
                entry.node == observation.node && entry.instance == observation.instance
            })
            .count();
        return Some(finish_is_declared && occurrences == 1);
    }
    let Some(target) = protocol.controls.controls.iter().find(|control| {
        format!("{}::{}", protocol.name, control.path.join("::")) == observation.node
    }) else {
        return Some(false);
    };
    let occurrence_limit = control_repeat_limit(protocol, target);
    let occurrences = document
        .observations
        .iter()
        .filter(|entry| entry.node == observation.node && entry.instance == observation.instance)
        .filter(|entry| entry.role_instance == observation.role_instance)
        .count();
    if !u64::try_from(occurrences).is_ok_and(|count| count <= occurrence_limit) {
        return Some(false);
    }
    if target.kind.label() == "case" {
        let choice_path = &target.path[..target.path.len().saturating_sub(1)];
        let choice_name = format!("{}::{}", protocol.name, choice_path.join("::"));
        let selected_cases = document
            .observations
            .iter()
            .filter(|entry| {
                entry.kind == ObservationKind::Case
                    && entry.instance == observation.instance
                    && entry.role_instance == observation.role_instance
                    && entry.node.starts_with(&format!("{choice_name}::"))
            })
            .count();
        if !u64::try_from(selected_cases).is_ok_and(|count| count <= occurrence_limit) {
            return Some(false);
        }
    }
    let Some(root) = protocol.controls.root() else {
        return Some(false);
    };
    let mut ancestry = Vec::new();
    if !control_path(protocol, root.node, target.node, &mut ancestry) {
        return Some(false);
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
        if observation.kind == ObservationKind::Case && selected_case.node == target.node {
            continue;
        }
        let case_name = format!("{}::{}", protocol.name, selected_case.path.join("::"));
        let choice_name = format!("{}::{}", protocol.name, ancestor.path.join("::"));
        let selected = document
            .observations
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.kind == ObservationKind::Case
                    && entry.node.starts_with(&format!("{choice_name}::"))
                    && entry.instance == observation.instance
                    && entry.role_instance == observation.role_instance
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Some(false);
        }
        let repeat_limit = control_repeat_limit(protocol, ancestor);
        if !u64::try_from(selected.len()).is_ok_and(|count| count <= repeat_limit) {
            return Some(false);
        }
        if selected.len() > 1 && repeat_limit == 1 {
            return Some(false);
        }
        if selected.iter().any(|(_, entry)| entry.node != case_name) {
            // The recording has no iteration association for descendants.
            // Mixed repeated choices cannot safely prove or refute this path.
            return None;
        }
        let Some((case_position, _)) = selected.iter().find(|(_, entry)| entry.node == case_name)
        else {
            return Some(false);
        };
        if !causally_after(protocol, document, position, *case_position) {
            return Some(false);
        }
    }
    Some(true)
}

fn control_repeat_limit(
    protocol: &CheckedProtocol,
    target: &fsl_semantics::check::CheckedControl,
) -> u64 {
    protocol
        .controls
        .controls
        .iter()
        .filter(|control| {
            control.kind.label() == "repeat"
                && control.path.len() < target.path.len()
                && target.path.starts_with(&control.path)
        })
        .filter_map(|control| {
            protocol
                .slots
                .repeats
                .iter()
                .find(|repeat| repeat.node == control.node)
                .map(|repeat| repeat.max.unwrap_or(u64::MAX))
        })
        .fold(1, u64::saturating_mul)
}

fn control_path_enabled(
    protocol: &CheckedProtocol,
    document: &RecordingDocument,
    node_name: &str,
    instance: Option<&str>,
) -> Option<bool> {
    let target =
        protocol.controls.controls.iter().find(|control| {
            format!("{}::{}", protocol.name, control.path.join("::")) == node_name
        })?;
    let root = protocol.controls.root()?;
    let mut ancestry = Vec::new();
    if !control_path(protocol, root.node, target.node, &mut ancestry) {
        return None;
    }
    for (index, ancestor) in ancestry.iter().enumerate() {
        if ancestor.kind.label() != "choice" {
            continue;
        }
        let Some(expected_case) = ancestry[index + 1..]
            .iter()
            .find(|entry| entry.kind.label() == "case")
        else {
            continue;
        };
        let choice_name = format!("{}::{}", protocol.name, ancestor.path.join("::"));
        let selected = document
            .observations
            .iter()
            .filter(|observation| {
                observation.kind == ObservationKind::Case
                    && observation.instance.as_deref() == instance
                    && observation.node.starts_with(&format!("{choice_name}::"))
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return None;
        }
        let repeat_limit = control_repeat_limit(protocol, ancestor);
        if !u64::try_from(selected.len()).is_ok_and(|count| count <= repeat_limit) {
            return Some(false);
        }
        let expected_name = format!("{}::{}", protocol.name, expected_case.path.join("::"));
        if selected.len() == 1 && selected[0].node != expected_name {
            return Some(false);
        }
        if selected.iter().any(|case| case.node != expected_name) {
            // A repeated choice may take different cases on different
            // iterations. The recording format does not pair each action
            // with an iteration, so keep this path unresolved.
            return None;
        }
    }
    Some(true)
}

fn observation_owned_by(protocol: &CheckedProtocol, observation: &Observation) -> bool {
    let node_ids = protocol_node_ids(protocol);
    let Some(node) = node_ids.get(&observation.node).copied() else {
        return false;
    };
    let mut owner = protocol
        .owners
        .iter()
        .find(|entry| entry.node == node)
        .map(|entry| entry.role);

    if owner.is_none() && observation.kind == ObservationKind::Case {
        if let Some(case) = protocol.controls.control(node) {
            owner = case.path[..case.path.len().saturating_sub(1)]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, _)| {
                    let ancestor = protocol.controls.controls.iter().find(|control| {
                        control.path == case.path[..=index] && control.kind.label() == "choice"
                    })?;
                    protocol
                        .owners
                        .iter()
                        .find(|entry| entry.node == ancestor.node)
                        .map(|entry| entry.role)
                });
        }
    }

    if owner.is_none() && observation.kind == ObservationKind::Effect {
        let attempt = protocol.edges.iter().find(|edge| {
            edge.kind == fsl_semantics::check::EdgeKind::Effect && edge.to.node == node
        });
        if let Some(attempt) = attempt {
            owner = protocol
                .owners
                .iter()
                .find(|entry| entry.node == attempt.from.node)
                .map(|entry| entry.role);
        }
    }

    owner.is_none_or(|role| {
        protocol
            .roles
            .get(role)
            .is_some_and(|declared| declared.name == observation.role)
    })
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
    let name = protocol
        .controls
        .controls
        .iter()
        .find(|control| control.node.0 == node)
        .map(|control| control.path.join("::"))
        .or_else(|| {
            protocol
                .finish
                .as_ref()
                .filter(|(finish, _)| finish.0 == node)
                .map(|(_, name)| name.clone())
        })
        .unwrap_or_default();
    format!("{}::{name}", protocol.name)
}
