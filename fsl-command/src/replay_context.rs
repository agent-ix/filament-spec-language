// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621's bounded external replay-context input, retaining raw package JSON.

use std::collections::{BTreeMap, BTreeSet};

use fsl_replay::replay_context::{
    ConsumerLimits, ContextError, DocumentKind, EffectiveLimits, ReplayContext,
    ReplayReconstructionWire,
};
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::replay_request::RequestDefect;
mod output;
pub use output::RawContextOutcome;

/// The command input format of an independent reconstruction replay.
pub const FORMAT: &str = "native-replay-context/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    context: Box<RawValue>,
    effective_limits: EffectiveLimits,
}

/// Why a serialized context could not reach native reconstruction.
#[derive(Debug, thiserror::Error)]
pub enum ContextRequestError {
    /// Malformed or inconsistent context data.
    #[error(transparent)]
    Context(#[from] ContextError),
    /// A common request member failed its existing owner mapping.
    #[error(transparent)]
    Descriptor(#[from] RequestDefect),
    /// The consumer's explicit replay input bound was exceeded.
    #[error(transparent)]
    Limit(#[from] fsl_replay::BoundExceeded),
}

/// Read original data and consumer limits without running a backend, making
/// up a source, or accepting a filesystem location as a byte authority.
pub struct PreparedContext {
    /// Immutable original composition.
    pub context: ReplayContext,
    /// The common native reconstruction input, with no fake family source.
    pub common: ReplayReconstructionWire,
    /// Explicit admitted consumer execution/read limits.
    pub limits: ConsumerLimits,
}

/// A transport admission failure or the original native reconstruction result.
#[derive(Debug, thiserror::Error)]
pub enum ContextReplayError {
    /// The serialized input failed admission before native reconstruction.
    #[error(transparent)]
    Request(#[from] ContextRequestError),
    /// Native source, family, reference or replay admission refused.
    #[error(transparent)]
    Native(Box<fsl_replay::replay_context::NativeContextError>),
}

impl PreparedContext {
    /// Reconstruct genuine source and dispatch the actual native family replay.
    pub fn replay(
        self,
    ) -> Result<
        fsl_replay::replay_context::NativeReplayOutput,
        fsl_replay::replay_context::NativeContextError,
    > {
        let family = self.context.family;
        let native = fsl_replay::replay_context::ReconstructedContext::prepare(
            self.context,
            self.common,
            self.limits,
        )?;
        match family {
            fsl_replay::replay_context::ReplayFamily::ModelTemporal => native.replay_model_output(),
            fsl_replay::replay_context::ReplayFamily::ProtocolTemporal => {
                native.replay_protocol_output()
            }
            fsl_replay::replay_context::ReplayFamily::Hybrid => native.replay_hybrid_output(),
        }
    }
}

/// Read and independently replay the serialized source-backed context. The
/// result is fresh native `filament-outcome/1`, never the producer's history.
pub fn replay(
    bytes: &[u8],
) -> Result<fsl_replay::replay_context::NativeReplayOutput, ContextReplayError> {
    read(bytes)?
        .replay()
        .map_err(|error| ContextReplayError::Native(Box::new(error)))
}

impl ContextReplayError {
    /// Native catalog code of the actual read/reconstruction failure.
    pub fn code(&self) -> fsl_foundation::Code {
        use fsl_foundation::Code;
        match self {
            Self::Native(error) => error.code(),
            Self::Request(ContextRequestError::Limit(_)) => Code::StageLimitExceeded,
            Self::Request(ContextRequestError::Context(ContextError::Allocation { .. })) => {
                Code::ResourceExhausted
            }
            Self::Request(_) => Code::InvalidRequest,
        }
    }

    /// Native category, including incomplete resource stops and genuine faults.
    pub fn category(&self) -> filament_semantic_value::catalog::Category {
        match self {
            Self::Native(error) => error.category(),
            _ => self.code().category(),
        }
    }

    /// The original native cause tag, when the owner supplies one.
    pub fn cause(&self) -> Option<&str> {
        match self {
            Self::Native(error) => error.cause(),
            Self::Request(ContextRequestError::Limit(_)) => Some("input-bytes-exceeded"),
            Self::Request(ContextRequestError::Context(ContextError::Allocation { .. })) => {
                Some("allocation-failed")
            }
            _ => None,
        }
    }

    /// A native replay refusal whose existing outcome must remain unchanged.
    pub fn replay_refusal(&self) -> Option<&fsl_replay::ReplayRefusal> {
        match self {
            Self::Native(error) => error.replay_refusal(),
            _ => None,
        }
    }

    /// Preserve the native refusal document, or locate this composition's own
    /// transport/admission failure in the ordinary replay outcome envelope.
    pub fn outcome(&self, request_path: Option<&str>) -> fsl_replay::OutcomeDocument {
        if let Some(refusal) = self.replay_refusal() {
            return fsl_replay::OutcomeDocument::from_replay_refusal_at(refusal, request_path);
        }
        let pointer = match self {
            Self::Native(error) => error.pointer().map(|pointer| format!("/context{pointer}")),
            Self::Request(ContextRequestError::Descriptor(error)) => {
                Some(format!("/context/request{}", error.pointer()))
            }
            Self::Request(ContextRequestError::Context(ContextError::Invalid {
                pointer, ..
            })) => Some(pointer.clone()),
            Self::Request(ContextRequestError::Limit(_)) => {
                Some("/effective_limits/replay_input_bytes".to_owned())
            }
            _ => None,
        };
        let locus = pointer.map(|pointer| fsl_replay::OutcomeLocus::Input {
            operand: request_path.unwrap_or("request").to_owned(),
            member: None,
            pointer: Some(pointer),
        });
        fsl_replay::OutcomeDocument::new(fsl_replay::Operation::Replay, None, self.category())
            .with_diagnostics(vec![fsl_replay::OutcomeDiagnostic::new(
                self.cause(),
                self.code().as_str(),
                locus,
                self.to_string(),
            )])
    }
}

/// Read one closed command input. The command caller bounds the outer file;
/// this reader additionally enforces the explicit consumer replay byte bound.
pub fn read(bytes: &[u8]) -> Result<PreparedContext, ContextRequestError> {
    // Validation never becomes the retained package carrier: deserialize the
    // original byte slice separately so RawValue preserves its lexical bytes.
    let _unique = crate::unique_json::Deep::new(
        crate::unique_json::read(bytes).map_err(|reason| ContextError::invalid("", reason))?,
    );
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(ContextError::from)?;
    if envelope.format != FORMAT {
        return Err(ContextError::invalid("/format", "expected native-replay-context/1").into());
    }
    let limits = envelope.effective_limits.admit()?;
    let actual = envelope.context.get().len() as u128;
    if actual > u128::from(limits.replay().input_bytes) {
        return Err(fsl_replay::BoundExceeded {
            bound: limits.replay().input_bytes,
            actual,
        }
        .into());
    }
    let context: ReplayContext =
        serde_json::from_str(envelope.context.get()).map_err(ContextError::from)?;
    context.authenticate_packet()?;

    let mut sources = BTreeMap::new();
    for (index, source) in context.source_bytes.iter().enumerate() {
        if sources
            .insert(source.bytes_path.as_str(), source.bytes.as_slice())
            .is_some()
        {
            return Err(ContextError::invalid(
                format!("/source_bytes/{index}/bytes_path"),
                "duplicate source token",
            )
            .into());
        }
    }
    let mut requested = BTreeSet::new();
    for (index, provision) in context.request.byte_provision.iter().enumerate() {
        if !requested.insert(provision.bytes_path.as_str())
            || !sources.contains_key(provision.bytes_path.as_str())
        {
            return Err(ContextError::invalid(
                format!("/request/byte_provision/{index}/bytes_path"),
                "missing or multiply selected source token",
            )
            .into());
        }
    }
    if requested.len() != sources.len() {
        return Err(ContextError::invalid("/source_bytes", "unselected source token").into());
    }
    let mut dependencies = BTreeSet::new();
    let mut documents = Vec::new();
    for (index, document) in context.documents.iter().enumerate() {
        match (document.kind, &document.selection) {
            (DocumentKind::DependencyPackage, Some(selection))
                if dependencies.insert(selection) => {}
            (DocumentKind::DependencyPackage, _) => {
                return Err(ContextError::invalid(
                    format!("/documents/{index}/selection"),
                    "missing or duplicate dependency selection",
                )
                .into())
            }
            (_, Some(_)) => {
                return Err(ContextError::invalid(
                    format!("/documents/{index}/selection"),
                    "only dependency packages have a selection",
                )
                .into())
            }
            (_, None) => documents.push(document.bytes.clone()),
        }
    }
    let common = crate::replay_request::read_common_context_descriptor::<ContextRequestError>(
        &context.request,
        limits.replay().input_bytes,
        |token, pointer| {
            sources
                .get(token)
                .map(|bytes| bytes.to_vec())
                .ok_or_else(|| {
                    ContextError::invalid(pointer, "source token is not provisioned").into()
                })
        },
        documents,
    )?;
    Ok(PreparedContext {
        context,
        common,
        limits,
    })
}
