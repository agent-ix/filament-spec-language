// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native reconstruction failure classification at the external command boundary.

use filament_semantic_value::catalog::Category;
use fsl_eval::simulation::model::ModelKeyError;
use fsl_eval::simulation::EncodingRefusal;
use fsl_foundation::Code;

use super::{
    ContextError, NativeContextError, ReferenceContextError, RestoreStop, TemporalReadbackError,
};
use crate::reference_codec::{ReferenceCodecError, ReferenceFailure};

fn context_code(error: &ContextError) -> Code {
    match error {
        ContextError::Allocation { .. } => Code::ResourceExhausted,
        ContextError::Invalid { .. } => Code::InvalidRuntimeInput,
        ContextError::Json(_) => Code::InvalidRequest,
    }
}

fn key_code(error: &EncodingRefusal) -> Code {
    match error {
        EncodingRefusal::NoEncoding(_) => Code::InvalidRuntimeInput,
        EncodingRefusal::Limit(_) => Code::StageLimitExceeded,
        EncodingRefusal::Allocation { .. } => Code::ResourceExhausted,
    }
}

fn model_key_code(error: &ModelKeyError) -> Code {
    match error {
        ModelKeyError::Denied | ModelKeyError::SizeOverflow => Code::ResourceExhausted,
        ModelKeyError::Cancelled(_) => Code::Cancelled,
        ModelKeyError::Encoding(error) => key_code(error),
        ModelKeyError::Unsupported(_) => Code::UnsupportedConstruct,
    }
}

fn reference_code(error: &ReferenceContextError) -> Code {
    match error {
        ReferenceContextError::SourceOwner => Code::InvalidRuntimeInput,
        ReferenceContextError::StateKey(error) => key_code(error),
        ReferenceContextError::Admission(error) => error.code(),
        ReferenceContextError::Catalog(error) => match error {
            fsl_package::ReferenceCatalogError::Normalization(_) => Code::ResourceExhausted,
            fsl_package::ReferenceCatalogError::Projection(_) => Code::InvalidPackage,
            fsl_package::ReferenceCatalogError::Denied(_) => Code::StageLimitExceeded,
            fsl_package::ReferenceCatalogError::UnknownDeclaration => Code::MissingDeclaration,
            fsl_package::ReferenceCatalogError::ConflictingMapping => Code::AmbiguousDeclaration,
            fsl_package::ReferenceCatalogError::ForeignModel => Code::InvalidModelBinding,
            fsl_package::ReferenceCatalogError::Allocation => Code::ResourceExhausted,
        },
        ReferenceContextError::Codec(error) => match error {
            ReferenceCodecError::Carrier { .. } => Code::OutputFailure,
            ReferenceCodecError::Decimal { cause, .. } => match cause {
                filament_exact::IntegerDecimalBudgetError::Denied => Code::ResourceExhausted,
                filament_exact::IntegerDecimalBudgetError::Decimal(cause) => match cause {
                    filament_exact::IntegerDecimalError::ByteLimit { .. } => {
                        Code::StageLimitExceeded
                    }
                    filament_exact::IntegerDecimalError::Allocation { .. }
                    | filament_exact::IntegerDecimalError::OutputSizeOverflow => {
                        Code::ResourceExhausted
                    }
                },
            },
            ReferenceCodecError::Normalization { .. } => Code::ResourceExhausted,
            ReferenceCodecError::IdentityCopy { cause, .. } => match cause {
                filament_exact::IdentityCopyError::Policy(cause) => model_key_code(cause),
                filament_exact::IdentityCopyError::Allocation { .. }
                | filament_exact::IdentityCopyError::Capacity { .. } => Code::ResourceExhausted,
            },
            ReferenceCodecError::Universe { .. } => Code::InvalidRuntimeInput,
            ReferenceCodecError::Native { cause, .. } => match cause {
                fsl_semantics::model::observation::AdmissionFailure::Refused(record)
                | fsl_semantics::model::observation::AdmissionFailure::Incomplete(record) => {
                    Code::from_code(record.catalog.code()).unwrap_or(Code::RuntimeInvariant)
                }
                fsl_semantics::model::observation::AdmissionFailure::Fault(_) => {
                    Code::RuntimeInvariant
                }
            },
            ReferenceCodecError::Transition { .. } => Code::InvalidRuntimeInput,
            ReferenceCodecError::Refused { reason, .. } => match reason {
                ReferenceFailure::ForeignCatalog => Code::ForeignReference,
                ReferenceFailure::UnknownDeclaration => Code::MissingDeclaration,
                ReferenceFailure::DepthLimit => Code::StageLimitExceeded,
                _ => Code::InvalidRuntimeInput,
            },
            ReferenceCodecError::Key { source, .. } => model_key_code(source),
        },
    }
}

fn readback_code(error: &TemporalReadbackError) -> Code {
    match error {
        TemporalReadbackError::TimedStop(stop) => match stop {
            fsl_eval::simulation::model::TimedStop::Expansion(stop) => expansion_code(stop),
            fsl_eval::simulation::model::TimedStop::MaxClocks { .. } => Code::StageLimitExceeded,
            fsl_eval::simulation::model::TimedStop::UnplacedElement { .. } => {
                Code::RuntimeInvariant
            }
            fsl_eval::simulation::model::TimedStop::Undecided(_) => Code::InvalidRuntimeInput,
        },
        TemporalReadbackError::Replay(error) => error.code(),
        TemporalReadbackError::Context(error) => context_code(error),
        TemporalReadbackError::Reference(error) => reference_code(error),
        TemporalReadbackError::Accounting { .. } => Code::ResourceExhausted,
        TemporalReadbackError::Expansion { stop } => expansion_code(stop),
        TemporalReadbackError::TemporalRefusal(fsl_eval::temporal::TemporalRefusal::Fault(_)) => {
            Code::RuntimeInvariant
        }
        TemporalReadbackError::TemporalOutcome(outcome)
            if matches!(
                outcome.as_ref(),
                fsl_eval::temporal::TemporalOutcome::Incomplete(_)
                    | fsl_eval::temporal::TemporalOutcome::WindowLimitReached(_)
            ) =>
        {
            Code::ResourceExhausted
        }
        TemporalReadbackError::Fairness(_)
        | TemporalReadbackError::TemporalRefusal(_)
        | TemporalReadbackError::TemporalOutcome(_)
        | TemporalReadbackError::NoMatch { .. }
        | TemporalReadbackError::Ambiguous { .. }
        | TemporalReadbackError::Delay { .. } => Code::InvalidRuntimeInput,
    }
}

fn expansion_code(stop: &fsl_eval::simulation::ExpansionStop) -> Code {
    Code::from_code(stop.cause.code()).unwrap_or(Code::RuntimeInvariant)
}

fn readback_expansion(
    error: &TemporalReadbackError,
) -> Option<&fsl_eval::simulation::ExpansionStop> {
    match error {
        TemporalReadbackError::Expansion { stop }
        | TemporalReadbackError::TimedStop(fsl_eval::simulation::model::TimedStop::Expansion(
            stop,
        )) => Some(stop),
        _ => None,
    }
}

impl NativeContextError {
    /// Native catalog classification, retaining each original typed refusal.
    pub fn code(&self) -> Code {
        match self {
            Self::Context(error) => context_code(error),
            Self::Request(error) => error.code(),
            Self::Replay(error) => error.code(),
            Self::Reference(error) => reference_code(error),
            Self::Resource(RestoreStop::Accounting(_)) => Code::ResourceExhausted,
            Self::TemporalReadback(error) => readback_code(error),
            Self::Output(super::ContextOutputError::Invariant { .. }) => Code::RuntimeInvariant,
            Self::Output(_) => Code::OutputFailure,
        }
    }

    /// An actual native replay refusal, when this failure came from replay.
    pub fn replay_refusal(&self) -> Option<&crate::ReplayRefusal> {
        match self {
            Self::Replay(error) => Some(error),
            Self::TemporalReadback(error) => match error.as_ref() {
                TemporalReadbackError::Replay(error) => Some(error),
                _ => None,
            },
            _ => None,
        }
    }

    /// Resource stops remain incomplete and output/fault failures remain faults.
    pub fn category(&self) -> Category {
        if let Some(refusal) = self.replay_refusal() {
            return crate::OutcomeDocument::from_replay_refusal(refusal).category();
        }
        if let Self::TemporalReadback(error) = self {
            if let Some(stop) = readback_expansion(error) {
                return fsl_foundation::diagnostic::category_of(&stop.cause)
                    .unwrap_or(Category::InternalFailure);
            }
        }
        match self {
            Self::TemporalReadback(error)
                if matches!(
                    error.as_ref(),
                    TemporalReadbackError::TimedStop(
                        fsl_eval::simulation::model::TimedStop::Undecided(_)
                    )
                ) =>
            {
                Category::Inconclusive
            }
            Self::TemporalReadback(error)
                if matches!(
                    error.as_ref(),
                    TemporalReadbackError::TimedStop(
                        fsl_eval::simulation::model::TimedStop::MaxClocks { .. }
                    )
                ) =>
            {
                Category::Incomplete
            }
            Self::Resource(RestoreStop::Accounting(_)) => Category::Incomplete,
            Self::Reference(ReferenceContextError::Catalog(
                fsl_package::ReferenceCatalogError::Normalization(_),
            )) => Category::Incomplete,
            Self::TemporalReadback(error)
                if matches!(
                    error.as_ref(),
                    TemporalReadbackError::Accounting { .. }
                        | TemporalReadbackError::Expansion { .. }
                        | TemporalReadbackError::TemporalOutcome(_)
                ) && self.code() == Code::ResourceExhausted =>
            {
                Category::Incomplete
            }
            Self::Output(_) => Category::InternalFailure,
            _ => self.code().category(),
        }
    }

    /// Existing native cause tags, without parsing an error message.
    pub fn cause(&self) -> Option<&str> {
        if let Some(refusal) = self.replay_refusal() {
            return refusal.cause();
        }
        match self {
            Self::Resource(RestoreStop::Accounting(_)) => Some("insufficient-next-charge"),
            Self::TemporalReadback(error) => match error.as_ref() {
                TemporalReadbackError::Accounting { .. } => Some("insufficient-next-charge"),
                error => readback_expansion(error).map(|stop| stop.cause.cause()),
            },
            Self::Request(error) => error.cause(),
            Self::Context(ContextError::Invalid { .. }) => Some("invalid-value"),
            Self::Context(ContextError::Allocation { .. }) => Some("allocation-failed"),
            _ => None,
        }
    }

    /// The offending context-relative member, when native admission retained one.
    pub fn pointer(&self) -> Option<&str> {
        match self {
            Self::Context(ContextError::Invalid { pointer, .. }) => Some(pointer),
            Self::TemporalReadback(error) => match error.as_ref() {
                TemporalReadbackError::Context(ContextError::Invalid { pointer, .. }) => {
                    Some(pointer)
                }
                _ => None,
            },
            _ => None,
        }
    }
}
