// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference wire admission shares the run reader's unbounded typed-admission
//! phase. The existing evaluator applies the call's execution/helper limits.

use filament_exact::{IntegerDecimalBudget, Reservation, ReservationDenied};
use fsl_eval::simulation::model::ModelKeyError;
use fsl_foundation::diagnostic::Code;

use crate::reference_codec::{ReferenceCodecError, ReferenceFailure};

#[derive(Default)]
pub(crate) struct ReferenceAdmission {
    work: u64,
    storage: usize,
}

impl Reservation for ReferenceAdmission {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.storage = self.storage.checked_add(bytes).ok_or(ReservationDenied)?;
        Ok(())
    }
}

impl IntegerDecimalBudget for ReferenceAdmission {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.work = self.work.checked_add(units).ok_or(ReservationDenied)?;
        Ok(())
    }
}

pub(crate) fn reference_error_code(error: &ReferenceCodecError) -> Code {
    use fsl_semantics::model::observation::AdmissionFailure;
    match error {
        ReferenceCodecError::IdentityCopy { cause, .. } => match cause {
            filament_exact::IdentityCopyError::Policy(source) => key_code(source),
            filament_exact::IdentityCopyError::Allocation { .. }
            | filament_exact::IdentityCopyError::Capacity { .. } => Code::ResourceExhausted,
        },
        ReferenceCodecError::Universe { .. } => Code::RuntimeInvariant,
        ReferenceCodecError::Native { cause, .. } => match cause {
            AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record) => {
                Code::from_code(record.catalog.code()).unwrap_or(Code::InvalidRuntimeInput)
            }
            AdmissionFailure::Fault(_) => Code::RuntimeInvariant,
        },
        ReferenceCodecError::Refused { reason, .. } => match reason {
            ReferenceFailure::MalformedWire
            | ReferenceFailure::UnknownDeclaration
            | ReferenceFailure::InvalidIdentity
            | ReferenceFailure::MissingPopulation
            | ReferenceFailure::WrongPopulation => Code::InvalidRuntimeInput,
            ReferenceFailure::ForeignCatalog | ReferenceFailure::IncompatibleUniverse => {
                Code::ForeignReference
            }
            ReferenceFailure::DepthLimit => Code::ResourceExhausted,
        },
        ReferenceCodecError::Key { source, .. } => key_code(source),
        ReferenceCodecError::Transition { .. } => Code::InvalidRuntimeInput,
    }
}

fn key_code(error: &ModelKeyError) -> Code {
    match error {
        ModelKeyError::Cancelled(_) => Code::Cancelled,
        ModelKeyError::Denied | ModelKeyError::SizeOverflow => Code::ResourceExhausted,
        ModelKeyError::Encoding(error) => match error {
            fsl_eval::simulation::EncodingRefusal::Allocation { .. } => Code::ResourceExhausted,
            fsl_eval::simulation::EncodingRefusal::Limit(_) => Code::StageLimitExceeded,
            fsl_eval::simulation::EncodingRefusal::NoEncoding(_) => Code::UnknownRequiredFeature,
        },
        ModelKeyError::Unsupported(_) => Code::UnknownRequiredFeature,
    }
}

pub(crate) fn reference_error_pointer(error: &ReferenceCodecError) -> &str {
    match error {
        ReferenceCodecError::IdentityCopy { pointer, .. }
        | ReferenceCodecError::Universe { pointer, .. }
        | ReferenceCodecError::Native { pointer, .. }
        | ReferenceCodecError::Transition { pointer, .. }
        | ReferenceCodecError::Refused { pointer, .. }
        | ReferenceCodecError::Key { pointer, .. } => pointer,
    }
}

pub(crate) fn reference_cancel_cause(
    error: &ReferenceCodecError,
) -> Option<filament_exact::CancelCause> {
    match error {
        ReferenceCodecError::Key {
            source: ModelKeyError::Cancelled(cause),
            ..
        }
        | ReferenceCodecError::IdentityCopy {
            cause: filament_exact::IdentityCopyError::Policy(ModelKeyError::Cancelled(cause)),
            ..
        } => Some(*cause),
        ReferenceCodecError::IdentityCopy { .. }
        | ReferenceCodecError::Universe { .. }
        | ReferenceCodecError::Key { .. }
        | ReferenceCodecError::Native { .. }
        | ReferenceCodecError::Refused { .. }
        | ReferenceCodecError::Transition { .. } => None,
    }
}
