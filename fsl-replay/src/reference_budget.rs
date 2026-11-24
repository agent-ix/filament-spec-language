// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine normalization-owner budget for reference conversion.

use filament_exact::{IntegerDecimalBudget, Reservation, ReservationDenied};
use fsl_eval::simulation::model::ModelKeyError;

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
    point: fsl_semantics::model::accounting::NormalizationChargePoint,
    selected: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    previous: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    pub(crate) stopped: Option<fsl_semantics::model::accounting::NormalizationIncomplete>,
    physical: PhysicalReferenceBudget,
}

impl<'a> NormalizationReferenceBudget<'a> {
    pub(crate) fn physical() -> Self {
        Self {
            owner: ResultOwner::Physical,
            point: fsl_semantics::model::accounting::NormalizationChargePoint::ReferenceResult,
            selected: None,
            previous: None,
            stopped: None,
            physical: PhysicalReferenceBudget::default(),
        }
    }
    /// Charge the original source-unit model-admission meter at `point`.
    pub fn source(
        owner: &'a fsl_semantics::model::intake::SelectedModels,
        point: fsl_semantics::model::accounting::NormalizationChargePoint,
    ) -> Self {
        Self {
            owner: ResultOwner::Source(owner),
            point,
            selected: None,
            previous: None,
            stopped: None,
            physical: PhysicalReferenceBudget::default(),
        }
    }
    /// Select each snapshot reference's real declaration owner before writing.
    pub fn snapshot(owner: &'a fsl_package::ReferenceCatalog<'a>) -> Self {
        Self {
            owner: ResultOwner::Snapshot(owner),
            point: fsl_semantics::model::accounting::NormalizationChargePoint::ReferenceResult,
            selected: None,
            previous: None,
            stopped: None,
            physical: PhysicalReferenceBudget::default(),
        }
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
pub(crate) struct PhysicalReferenceBudget {
    work: u64,
    storage: usize,
}

impl Reservation for PhysicalReferenceBudget {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.storage = self.storage.checked_add(bytes).ok_or(ReservationDenied)?;
        Ok(())
    }
}

impl IntegerDecimalBudget for PhysicalReferenceBudget {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.work = self.work.checked_add(units).ok_or(ReservationDenied)?;
        Ok(())
    }
}
