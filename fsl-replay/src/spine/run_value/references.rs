// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical conversion borrows the invocation's actual accounting owners.

use filament_exact::{IntegerDecimalBudget, Reservation, ReservationDenied};
use fsl_eval::simulation::model::ModelKeyError;
use fsl_foundation::diagnostic::Code;

use crate::reference_codec::{ReferenceCodecError, ReferenceFailure};

pub(crate) fn catalog_refusal(
    cause: fsl_package::ReferenceCatalogError,
) -> Box<super::super::RunRefusal> {
    Box::new(match cause {
        fsl_package::ReferenceCatalogError::Normalization(cause) => {
            super::super::RunRefusal::ObjectEnvironment(Box::new(
                fsl_semantics::model::observation::normalization_limit_record(&cause),
            ))
        }
        cause => super::super::RunRefusal::ReferenceCatalog(Box::new(cause)),
    })
}

pub(crate) fn catalog_error_code(cause: &fsl_package::ReferenceCatalogError) -> Code {
    use fsl_package::ReferenceCatalogError;
    match cause {
        ReferenceCatalogError::Normalization(_) | ReferenceCatalogError::Allocation => {
            Code::ResourceExhausted
        }
        ReferenceCatalogError::Denied(_) => Code::StageLimitExceeded,
        ReferenceCatalogError::Projection(error) => match error.code {
            filament_contract_model::ExactTypeProjectionErrorCode::ExhaustedWork
            | filament_contract_model::ExactTypeProjectionErrorCode::ExhaustedStorage => {
                Code::StageLimitExceeded
            }
            filament_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType => {
                Code::UnknownRequiredFeature
            }
            _ => Code::InvalidPackage,
        },
        ReferenceCatalogError::UnknownDeclaration
        | ReferenceCatalogError::ConflictingMapping
        | ReferenceCatalogError::ForeignModel => Code::ForeignReference,
    }
}

pub(crate) fn catalog_error_cause(
    cause: &fsl_package::ReferenceCatalogError,
) -> Option<&'static str> {
    match cause {
        fsl_package::ReferenceCatalogError::Normalization(cause) => Some(
            fsl_semantics::model::observation::normalization_limit_record(cause)
                .catalog
                .cause(),
        ),
        fsl_package::ReferenceCatalogError::Denied(denied) => Some(match denied.kind {
            filament_contract_model::NativeJoinChargeKind::Work => "exhausted_work",
            filament_contract_model::NativeJoinChargeKind::Storage => "exhausted_storage",
        }),
        fsl_package::ReferenceCatalogError::Projection(error) => Some(error.code.as_str()),
        fsl_package::ReferenceCatalogError::Allocation => None,
        fsl_package::ReferenceCatalogError::UnknownDeclaration
        | fsl_package::ReferenceCatalogError::ConflictingMapping
        | fsl_package::ReferenceCatalogError::ForeignModel => Some("foreign-model-selection"),
    }
}

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
    if let Some(refusal) = take_normalization_refusal(budget) {
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
    if let Some(refusal) = take_normalization_refusal(budget) {
        return Err(super::WriteStop::Refusal(refusal));
    }
    result.map_err(|cause| super::WriteStop::Refusal(reference_refusal(cause)))
}

pub(crate) struct SuppliedReferenceAdmission<'a> {
    run: &'a mut filament_semantic_value::supplied_stop::SuppliedRun,
    argument: usize,
    node: u64,
    pub(crate) stopped: Option<Box<filament_semantic_value::supplied_stop::SuppliedStop>>,
    physical: PhysicalReferenceBudget,
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
            physical: PhysicalReferenceBudget::default(),
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

use crate::reference_budget::{NormalizationReferenceBudget, PhysicalReferenceBudget};

pub(crate) fn take_normalization_refusal(
    budget: &mut NormalizationReferenceBudget<'_>,
) -> Option<Box<super::super::RunRefusal>> {
    budget.take_normalization_failure().map(|cause| {
        Box::new(super::super::RunRefusal::ObjectEnvironment(Box::new(
            fsl_semantics::model::observation::normalization_limit_record(&cause),
        )))
    })
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
