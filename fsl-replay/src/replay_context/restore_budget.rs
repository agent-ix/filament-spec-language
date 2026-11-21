// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference restoration borrows the consumer meter; allocators own allocation failure.

use filament_exact::{
    Charge, ChargePoint, Incomplete, Integer, IntegerDecimalBudget, LimitKind, Meter, Reservation,
    ReservationDenied,
};
use fsl_eval::simulation::model::{ModelKeyBudget, ModelRestoreBudget};

/// The original resource cause behind a model-key admission denial.
#[derive(Debug, thiserror::Error)]
pub enum RestoreStop {
    /// The original scalar meter denial.
    #[error("reference restoration accounting stopped: {0:?}")]
    Accounting(Incomplete),
}

/// A borrow of one execution scope, never a fresh per-value budget.
pub struct RestoreBudget<'a> {
    meter: &'a mut Meter,
    stop: Option<RestoreStop>,
}

impl<'a> RestoreBudget<'a> {
    /// Borrow the actual operation meter without resetting its consumed work.
    pub fn new(meter: &'a mut Meter) -> Self {
        Self { meter, stop: None }
    }

    /// The first resource refusal, retained even when a native key returns Denied.
    pub fn stop(&self) -> Option<&RestoreStop> {
        self.stop.as_ref()
    }

    /// Finish this scope and retain its first original admission failure.
    pub fn into_stop(self) -> Option<RestoreStop> {
        self.stop
    }

    fn charge(&mut self, charge: Charge) -> Result<(), ReservationDenied> {
        if self.stop.is_some() {
            return Err(ReservationDenied);
        }
        self.meter.charge(charge).map_err(|cause| {
            self.stop = Some(RestoreStop::Accounting(cause));
            ReservationDenied
        })
    }
}

impl Reservation for RestoreBudget<'_> {
    fn reserve(&mut self, _bytes: usize) -> Result<(), ReservationDenied> {
        if self.stop.is_some() {
            return Err(ReservationDenied);
        }
        // Replay input bytes bound the offered input. Physical allocation is
        // attempted by the native owner's fallible allocator at each use.
        Ok(())
    }
}

impl IntegerDecimalBudget for RestoreBudget<'_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.charge(Charge::new(ChargePoint::GraphEdge).work(Integer::from(units)))
    }
}

impl ModelKeyBudget for RestoreBudget<'_> {
    fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
        self.charge(
            Charge::new(ChargePoint::GraphResultRetain)
                .size(LimitKind::IntegerBits, value.magnitude_bits()),
        )
    }
}

impl ModelRestoreBudget for RestoreBudget<'_> {
    fn meter(&mut self) -> &mut Meter {
        self.meter
    }
}
