// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621's producer-owned reconstruction input for an independent replay.
//!
//! The raw package document is retained through serialization. The transport
//! composes existing family inputs; it does not extend the FR-336 source union.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::Value;

mod encode;
pub use encode::{accounting_wire, declared_domain_wire, envelope_wire, request_wire};
mod consumer;
pub use consumer::{accounting_from_wire, unsigned, ConsumerLimits, EffectiveLimits};
mod packet;
pub use packet::SourceDescriptor;
mod subject;
pub use crate::request::{ReplayReconstructionInput, ReplayReconstructionWire};
pub use subject::{ModelSubject, ProtocolSubject};
mod error;
mod native;
pub use native::{NativeContextError, ReconstructedContext};
mod output;
mod references;
mod restore_budget;
mod temporal_input;
pub use crate::model_check::replay::protocol_readback::TemporalReadbackError;
pub use output::{ContextOutputError, NativeReplayOutput};
pub use references::{with_source_reference_codec, ReferenceContextError};
pub use restore_budget::{RestoreBudget, RestoreStop};
pub(crate) use temporal_input::{StateKeyInput, TemporalInput, TemporalMetadata};

/// The native replay entry selected by a reconstruction context.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReplayFamily {
    /// A temporal counterexample over a model subject.
    #[serde(rename = "model-temporal")]
    ModelTemporal,
    /// A temporal counterexample over a protocol subject.
    #[serde(rename = "protocol-temporal")]
    ProtocolTemporal,
    /// A hybrid trace and its original checked claim.
    #[serde(rename = "hybrid")]
    Hybrid,
}

/// A genuine source byte provision, addressed by an in-document token.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceBytes {
    /// The token a request provision entry names; never opened as a path.
    pub bytes_path: String,
    /// Every original byte under that token.
    pub bytes: Vec<u8>,
}

/// The native admission role of an original document.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentKind {
    /// A selected domain package.
    DomainPackage,
    /// A snapshot, including an initial state.
    Snapshot,
    /// An original population-universe document.
    Universe,
    /// A family observation document.
    Observation,
    /// A recursively selected emitted library package.
    DependencyPackage,
}

/// A complete original document and the role under which it is admitted.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextDocument {
    /// The native admission role.
    pub kind: DocumentKind,
    /// Selected library identity for a dependency package; otherwise absent.
    #[serde(deserialize_with = "explicit_option")]
    pub selection: Option<String>,
    /// Complete original bytes, never a claim of admission by themselves.
    pub bytes: Vec<u8>,
}

/// The thirteen common request members; source, documents and certificate
/// are supplied by the surrounding FR-621 composition.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestDescriptor {
    /// Original package JSON, retained without parsing and re-emitting it.
    pub package: Box<RawValue>,
    /// Actual selected semantic profiles.
    pub profile_selections: Value,
    /// Exact selected dependencies and source locks.
    pub dependencies: Value,
    /// Original resolved target and instance binding.
    pub replay_target: Value,
    /// Original checked obligation identity.
    pub obligation_identity: Value,
    /// Backend that produced the original terminal record.
    pub backend: String,
    /// Original scalar environment.
    pub state_environment: Value,
    /// Original proving accounting; never consumer execution authority.
    pub accounting_limits: BTreeMap<String, String>,
    /// Original stage settings, including original accounting entries.
    pub stage_limits: BTreeMap<String, String>,
    /// Original supplied-value helper ceilings.
    pub supplied_limits: Value,
    /// Original selected object-environment reference.
    pub object_environment: Value,
    /// Actual substituted proving domains.
    pub declared_domains: Vec<Value>,
    /// Source labels and tokens resolving only within `source_bytes`.
    pub byte_provision: Vec<ProvisionEntry>,
}

/// One source label and its recoverable byte provision token.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionEntry {
    /// Source authority label.
    pub authority: String,
    /// Source identity label.
    pub identity: String,
    /// Token owned by the context, not a filesystem path.
    pub bytes_path: String,
}

/// The original witness packet, including its complete family payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeDescriptor {
    /// Original obligation.
    pub obligation_identity: Value,
    /// Original checked occurrence.
    pub occurrence_key: Value,
    /// Original clause node.
    pub clause_node: Value,
    /// Original replay target members.
    pub replay_target: Value,
    /// Original semantic profiles.
    pub profile_selections: Value,
    /// Original proving accounting, immutable after authentication.
    pub run_limits: BTreeMap<String, String>,
    /// Original declared domains.
    pub declared_domains: Vec<Value>,
    /// Original producer identity.
    pub backend: String,
    /// Explicit null when the family has no position.
    pub trace_position: Value,
    /// Actual witness source, not a replacement for the family payload.
    pub source: Value,
    /// Complete original family-owned evidence.
    pub family_payload: Value,
}

/// The complete serialized FR-621 reconstruction context.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayContext {
    /// Immutable original request descriptor.
    pub request: RequestDescriptor,
    /// Complete original source bytes.
    pub source_bytes: Vec<SourceBytes>,
    /// Complete original domain, state, universe and dependency bytes.
    pub documents: Vec<ContextDocument>,
    /// Native family entry to reconstruct and check.
    pub family: ReplayFamily,
    /// Original counterexample in the owning family's wire form.
    pub payload: Value,
    /// Original witness envelope, absent for hybrid.
    #[serde(deserialize_with = "explicit_option")]
    pub envelope: Option<EnvelopeDescriptor>,
    /// Actual selected family subject inputs.
    pub subject: Value,
    /// Complete original required family certificate, when any.
    #[serde(deserialize_with = "explicit_option")]
    pub certificate: Option<Value>,
}

fn explicit_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Why native reconstruction cannot retain or admit a context.
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    /// Bounded reconstruction/output storage could not be reserved.
    #[error("replay context allocation refused for {requested} entries")]
    Allocation {
        /// Requested entry count, before attempting the allocation.
        requested: usize,
    },
    /// Invalid input at a context-relative JSON pointer.
    #[error("invalid replay context at {pointer}: {reason}")]
    Invalid {
        /// Context-relative pointer.
        pointer: String,
        /// Native validation detail; never interpreted as a discriminator.
        reason: String,
    },
    /// JSON encoding or read failure.
    #[error("replay context JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl ContextError {
    /// Report malformed or inconsistent original input.
    pub fn invalid(pointer: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid {
            pointer: pointer.into(),
            reason: reason.into(),
        }
    }
}

impl ReplayContext {
    /// Serialize the composition while preserving its original package bytes.
    pub fn to_raw(&self) -> Result<Box<RawValue>, ContextError> {
        Ok(serde_json::value::to_raw_value(self)?)
    }

    /// Check the original packet/request relationship before any consumer
    /// accounting overlay or native execution is constructed.
    pub fn authenticate_packet(&self) -> Result<(), ContextError> {
        match (&self.family, &self.envelope) {
            (ReplayFamily::Hybrid, None) => Ok(()),
            (ReplayFamily::Hybrid, Some(_)) => Err(ContextError::invalid(
                "/envelope",
                "hybrid has no witness packet",
            )),
            (_, None) => Err(ContextError::invalid(
                "/envelope",
                "missing original witness packet",
            )),
            (_, Some(packet)) => {
                let request = &self.request;
                for (field, matches) in [
                    (
                        "obligation_identity",
                        packet.obligation_identity == request.obligation_identity,
                    ),
                    (
                        "replay_target",
                        request.replay_target.get("members") == Some(&packet.replay_target),
                    ),
                    (
                        "profile_selections",
                        packet.profile_selections == request.profile_selections,
                    ),
                    (
                        "declared_domains",
                        packet.declared_domains == request.declared_domains,
                    ),
                    ("backend", packet.backend == request.backend),
                    ("family_payload", packet.family_payload == self.payload),
                ] {
                    if !matches {
                        return Err(ContextError::invalid(
                            format!("/envelope/{field}"),
                            "original packet/request mismatch",
                        ));
                    }
                }
                let mut effective_original = request.accounting_limits.clone();
                for (name, bound) in &request.stage_limits {
                    if let Some(fsl_foundation::SettingName::Accounting(setting)) =
                        fsl_foundation::SettingName::from_name(name)
                    {
                        effective_original.insert(setting.counter().to_owned(), bound.clone());
                    }
                }
                if packet.run_limits != effective_original {
                    return Err(ContextError::invalid(
                        "/envelope/run_limits",
                        "original proving accounting mismatch",
                    ));
                }
                Ok(())
            }
        }
    }
}
