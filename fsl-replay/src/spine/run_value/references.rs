// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical conversion borrows the invocation's actual accounting owners.

use filament_exact::{IntegerDecimalBudget, Reservation, ReservationDenied};
use fsl_eval::simulation::model::ModelKeyError;
use fsl_foundation::diagnostic::Code;

use crate::reference_codec::{ReferenceCodecError, ReferenceFailure};

pub(crate) fn reference_refusal(cause: ReferenceCodecError) -> Box<super::super::RunRefusal> {
    Box::new(match cause {
        ReferenceCodecError::Normalization { cause, .. } => {
            super::super::RunRefusal::ObjectEnvironment(Box::new(
                fsl_semantics::model::observation::normalization_limit_record(&cause),
            ))
        }
        cause => super::super::RunRefusal::Reference(Box::new(cause)),
    })
}

pub(crate) fn write_run_value(
    value: &filament_exact::Value,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    enums: &[fsl_semantics::check::EnumBinding],
    variants: &[fsl_semantics::check::VariantTypeDeclaration],
    codec: Option<&crate::reference_codec::ReferenceCodec<'_, '_, '_>>,
    budget: &mut NormalizationReferenceBudget<'_>,
    cancel: &filament_exact::Cancel,
) -> Result<serde_json::Value, super::WriteStop> {
    let written = fsl_eval::simulation::model::run_value_bytes(
        value,
        types,
        enums,
        variants,
        fsl_foundation::IdentityLimits {
            input_bytes: u64::MAX,
        },
        budget,
        cancel,
    );
    if let Some(refusal) = budget.take_normalization_refusal() {
        return Err(super::WriteStop::Refusal(refusal));
    }
    let written = written.map_err(|source| {
        super::WriteStop::Refusal(Box::new(super::super::RunRefusal::Reference(Box::new(
            ReferenceCodecError::Key {
                pointer: String::new(),
                source,
            },
        ))))
    })?;
    let result = match codec {
        Some(codec) => codec.encode_value_bytes(&written, budget, cancel, usize::MAX),
        None => {
            crate::reference_codec::admit_native_value_bytes(&written, budget, cancel, usize::MAX)
        }
    };
    if let Some(refusal) = budget.take_normalization_refusal() {
        return Err(super::WriteStop::Refusal(refusal));
    }
    result.map_err(|cause| super::WriteStop::Refusal(reference_refusal(cause)))
}

pub(crate) struct SuppliedReferenceAdmission<'a> {
    run: &'a mut filament_semantic_value::supplied_stop::SuppliedRun,
    argument: usize,
    node: u64,
    pub(crate) stopped: Option<Box<filament_semantic_value::supplied_stop::SuppliedStop>>,
    physical: ReferenceAdmission,
}

impl<'a> SuppliedReferenceAdmission<'a> {
    pub(crate) fn new(
        run: &'a mut filament_semantic_value::supplied_stop::SuppliedRun,
        argument: usize,
        node: u64,
    ) -> Self {
        Self {
            run,
            argument,
            node,
            stopped: None,
            physical: ReferenceAdmission::default(),
        }
    }
}
impl Reservation for SuppliedReferenceAdmission<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.physical.reserve(bytes)
    }
}
impl IntegerDecimalBudget for SuppliedReferenceAdmission<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        use filament_semantic_value::supplied_stop::{SuppliedEvent, SuppliedLocus};
        for _ in 0..units {
            if let Err(stop) = self.run.charge_conversion(
                self.argument,
                SuppliedEvent::DescriptorQuery,
                SuppliedLocus::Witness {
                    argument: self.argument,
                    position: self.node,
                },
            ) {
                self.stopped = Some(stop);
                return Err(ReservationDenied);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ResultOwner<'a> {
    Physical,
    Source(&'a fsl_semantics::model::intake::SelectedModels),
    Snapshot(&'a fsl_package::ReferenceCatalog<'a>),
}

/// Borrow the genuine normalization owner for catalog and result conversion.
/// Physical reservations are tracked separately and introduce no policy cap.
pub struct NormalizationReferenceBudget<'a> {
    owner: ResultOwner<'a>,
    point: fsl_semantics::model::accounting::ChargePoint,
    selected: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    previous: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    pub(crate) stopped: Option<fsl_semantics::model::accounting::NormalizationIncomplete>,
    physical: ReferenceAdmission,
}

impl<'a> NormalizationReferenceBudget<'a> {
    pub(crate) fn physical() -> Self {
        Self {
            owner: ResultOwner::Physical,
            point: fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
            selected: None,
            previous: None,
            stopped: None,
            physical: ReferenceAdmission::default(),
        }
    }
    /// Charge the original source-unit model-admission meter at `point`.
    pub fn source(
        owner: &'a fsl_semantics::model::intake::SelectedModels,
        point: fsl_semantics::model::accounting::ChargePoint,
    ) -> Self {
        Self {
            owner: ResultOwner::Source(owner),
            point,
            selected: None,
            previous: None,
            stopped: None,
            physical: ReferenceAdmission::default(),
        }
    }
    /// Select each snapshot reference's real declaration owner before writing.
    pub fn snapshot(owner: &'a fsl_package::ReferenceCatalog<'a>) -> Self {
        Self {
            owner: ResultOwner::Snapshot(owner),
            point: fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
            selected: None,
            previous: None,
            stopped: None,
            physical: ReferenceAdmission::default(),
        }
    }
    pub(crate) fn take_normalization_refusal(&mut self) -> Option<Box<super::super::RunRefusal>> {
        self.stopped.take().map(|cause| {
            Box::new(super::super::RunRefusal::ObjectEnvironment(Box::new(
                fsl_semantics::model::observation::normalization_limit_record(&cause),
            )))
        })
    }
    /// Take the original typed denial after an erased core-budget refusal.
    /// The owner retains all previously charged counters.
    pub fn take_normalization_failure(
        &mut self,
    ) -> Option<fsl_semantics::model::accounting::NormalizationIncomplete> {
        self.stopped.take()
    }
}
impl Reservation for NormalizationReferenceBudget<'_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.physical.reserve(bytes)
    }
}
impl IntegerDecimalBudget for NormalizationReferenceBudget<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        let result = match self.owner {
            ResultOwner::Physical => return self.physical.work(units),
            ResultOwner::Source(owner) => owner.charge_reference_work(self.point, units),
            ResultOwner::Snapshot(_) => match self.selected {
                Some(owner) => owner.charge_reference_work(self.point, units),
                None => return self.physical.work(units),
            },
        };
        result.map_err(|cause| {
            self.stopped = Some(cause);
            ReservationDenied
        })
    }
}
impl fsl_eval::simulation::model::ModelKeyBudget for NormalizationReferenceBudget<'_> {
    fn integer(&mut self, _: &filament_exact::Integer) -> Result<(), ReservationDenied> {
        self.work(1)
    }
}
impl fsl_eval::simulation::model::RunValueBudget for NormalizationReferenceBudget<'_> {
    fn begin_reference(
        &mut self,
        reference: &filament_exact::ObjectReference,
    ) -> Result<(), ModelKeyError> {
        self.previous = self.selected;
        if matches!(self.owner, ResultOwner::Physical) {
            return Err(ModelKeyError::Unsupported(
                "reference has no admitted normalization owner",
            ));
        }
        if let ResultOwner::Snapshot(catalog) = self.owner {
            self.selected = catalog
                .normalization_owner_for(reference)
                .map_err(|cause| {
                    self.stopped = Some(cause);
                    ModelKeyError::Denied
                })?;
            if self.selected.is_none() {
                return Err(ModelKeyError::Unsupported("unowned reference declaration"));
            }
        }
        Ok(())
    }
    fn end_reference(&mut self) {
        self.selected = self.previous.take();
    }
}

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
        ReferenceCodecError::Normalization { cause, .. } => Code::from_code(
            fsl_semantics::model::observation::normalization_limit_record(cause)
                .catalog
                .code(),
        )
        .unwrap_or(Code::ResourceExhausted),
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
        ReferenceCodecError::Normalization { pointer, .. }
        | ReferenceCodecError::IdentityCopy { pointer, .. }
        | ReferenceCodecError::Universe { pointer, .. }
        | ReferenceCodecError::Native { pointer, .. }
        | ReferenceCodecError::Transition { pointer, .. }
        | ReferenceCodecError::Refused { pointer, .. }
        | ReferenceCodecError::Key { pointer, .. } => pointer,
    }
}

pub(crate) fn reference_error_cause(error: &ReferenceCodecError) -> Option<&'static str> {
    use fsl_semantics::model::observation::AdmissionFailure;
    match error {
        ReferenceCodecError::Normalization { .. } => Some("normalization-limit"),
        ReferenceCodecError::Native { cause, .. } => match cause {
            AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record) => {
                Some(record.catalog.cause())
            }
            AdmissionFailure::Fault(_) => None,
        },
        ReferenceCodecError::Refused { reason, .. } => Some(match reason {
            ReferenceFailure::MalformedWire | ReferenceFailure::UnknownDeclaration => {
                "wrong-value-kind"
            }
            ReferenceFailure::ForeignCatalog => "foreign-model-selection",
            ReferenceFailure::IncompatibleUniverse => "foreign-universe",
            ReferenceFailure::InvalidIdentity => "invalid-value",
            ReferenceFailure::MissingPopulation => "missing-member",
            ReferenceFailure::WrongPopulation => "wrong-role-mapping",
            ReferenceFailure::DepthLimit => "nesting-depth",
        }),
        ReferenceCodecError::IdentityCopy { .. }
        | ReferenceCodecError::Universe { .. }
        | ReferenceCodecError::Transition { .. }
        | ReferenceCodecError::Key { .. } => None,
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
        ReferenceCodecError::Normalization { .. }
        | ReferenceCodecError::IdentityCopy { .. }
        | ReferenceCodecError::Universe { .. }
        | ReferenceCodecError::Key { .. }
        | ReferenceCodecError::Native { .. }
        | ReferenceCodecError::Refused { .. }
        | ReferenceCodecError::Transition { .. } => None,
    }
}
