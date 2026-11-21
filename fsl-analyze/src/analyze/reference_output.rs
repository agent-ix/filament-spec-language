// SPDX-License-Identifier: AGPL-3.0-or-later
//! Carrier conversion borrowing the actual source normalization owner.

use filament_exact::Cancel;
use fsl_eval::simulation::model::{ModelKeyError, ModelSystem};
use fsl_package::ReferenceCatalog;
use fsl_replay::{
    NormalizationReferenceBudget, ReferenceCodec, ReferenceCodecError, ReferenceScope,
};
use fsl_semantics::model::accounting::ChargePoint;
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::observation::{normalization_limit_record, AdmissionFailure};

use super::{items, Refusal};

fn normalization(cause: &fsl_semantics::model::accounting::NormalizationIncomplete) -> Refusal {
    items::admission(&AdmissionFailure::Incomplete(Box::new(
        normalization_limit_record(cause),
    )))
}

/// An adapter view never creates or resets an allowance: both phases debit the
/// same source admission through its original shared normalization meter.
pub(super) fn with_source<T>(
    system: &ModelSystem<'_>,
    owner: Option<&SelectedModels>,
    cancel: &Cancel,
    write: impl FnOnce(&mut ReferenceScope<'_, '_, '_, '_>) -> Result<T, ReferenceCodecError>,
) -> Result<T, Refusal> {
    let (Some(package), Some(owner)) = (system.source_package(), owner) else {
        // Package-only routes must retain their genuine per-model snapshot
        // owner; an absent source admission cannot stand in for that owner.
        return Err(Refusal::Unsupported(items::NOT_YET));
    };
    let mut catalog_budget =
        NormalizationReferenceBudget::source(owner, ChargePoint::ReferenceCatalog);
    let catalog = match ReferenceCatalog::from_checked_model(
        package,
        system.reference_model(),
        &mut catalog_budget,
    ) {
        Ok(catalog) => catalog,
        Err(_) => {
            return Err(match catalog_budget.take_normalization_failure() {
                Some(cause) => normalization(&cause),
                None => Refusal::Failed(items::BROKEN),
            })
        }
    };
    let codec =
        ReferenceCodec::new(&catalog, system).map_err(|_| Refusal::Failed(items::BROKEN))?;
    let mut budget = NormalizationReferenceBudget::source(owner, ChargePoint::ReferenceResult);
    // Preserve the former canonical writer's unbounded byte ceiling. Work is
    // governed by the real owner; the codec retains its existing depth policy.
    let result = write(&mut codec.scope(
        &mut budget,
        cancel,
        usize::MAX,
        quire_canonical::Limits::new(u64::MAX),
    ));
    if let Some(cause) = budget.take_normalization_failure() {
        return Err(normalization(&cause));
    }
    if let Some(source) = cancel.tripped() {
        return Err(Refusal::Incomplete(
            fsl_replay::IncompleteCause::Cancelled { source },
        ));
    }
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(cause) = budget.take_normalization_failure() {
                return Err(normalization(&cause));
            }
            match error {
                ReferenceCodecError::Normalization { cause, .. } => Err(normalization(&cause)),
                ReferenceCodecError::Key {
                    source: ModelKeyError::Cancelled(source),
                    ..
                } => Err(Refusal::Incomplete(
                    fsl_replay::IncompleteCause::Cancelled { source },
                )),
                ReferenceCodecError::Native { cause, .. } => Err(items::admission(&cause)),
                _ => Err(Refusal::Failed(items::BROKEN)),
            }
        }
    }
}
