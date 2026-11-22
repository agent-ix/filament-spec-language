// SPDX-License-Identifier: AGPL-3.0-or-later
//! The original scalar ledger for policy representation and instance storage.
use crate::automaton::PolicyStateError;
use filament_exact::{Charge, ChargePoint, Integer, Meter};
use quire_canonical::{Error, Reservation, ReservationDenied, Sink};
use std::cell::RefCell;
use std::mem::size_of;
pub(crate) struct Budget<'a> {
    pub(crate) meter: RefCell<&'a mut Meter>,
    pub(crate) stopped: RefCell<Option<PolicyStateError>>,
}
impl Budget<'_> {
    pub(crate) fn pay(&self, charge: Charge) -> Result<(), PolicyStateError> {
        let mut meter = self.meter.borrow_mut();
        if let Some(cause) = meter.cancel().and_then(filament_exact::Cancel::cause) {
            return Err(PolicyStateError::Cancelled(cause));
        }
        match meter.charge(charge) {
            Ok(()) => Ok(()),
            Err(incomplete) => Err(
                match meter.cancel().and_then(filament_exact::Cancel::cause) {
                    Some(cause) => PolicyStateError::Cancelled(cause),
                    None => PolicyStateError::Accounting(incomplete),
                },
            ),
        }
    }
    pub(crate) fn work(&self, units: usize) -> Result<(), PolicyStateError> {
        self.pay(Charge::new(ChargePoint::ModelCandidate).work(Integer::from(units)))
    }
    pub(crate) fn reserve(&self, bytes: usize) -> Result<(), PolicyStateError> {
        self.pay(
            Charge::new(ChargePoint::ModelCandidate)
                .work(Integer::zero())
                .results(u64::try_from(bytes).map_err(|_| PolicyStateError::Invalid)?),
        )
    }
    pub(crate) fn growth<T>(
        &self,
        values: &mut Vec<T>,
        extra: usize,
    ) -> Result<(), PolicyStateError> {
        let needed = values
            .len()
            .checked_add(extra)
            .ok_or(PolicyStateError::Invalid)?;
        if needed > values.capacity() {
            self.reserve(
                needed
                    .checked_mul(size_of::<T>())
                    .ok_or(PolicyStateError::Invalid)?,
            )?;
            values
                .try_reserve_exact(needed - values.len())
                .map_err(|_| Error::Allocation {
                    requested: needed.saturating_mul(size_of::<T>()),
                })?;
        }
        Ok(())
    }
    pub(crate) fn copy(&self, output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), PolicyStateError> {
        self.work(bytes.len())?;
        self.growth(output, bytes.len())?;
        output.extend_from_slice(bytes);
        Ok(())
    }
    pub(crate) fn compare_bytes(
        &self,
        left: &[u8],
        right: &[u8],
    ) -> Result<std::cmp::Ordering, PolicyStateError> {
        for (left, right) in left.iter().zip(right) {
            self.work(1)?;
            let order = left.cmp(right);
            if order != std::cmp::Ordering::Equal {
                return Ok(order);
            }
        }
        self.work(1)?;
        Ok(left.len().cmp(&right.len()))
    }
    pub(crate) fn compare_declarations(
        &self,
        left: &filament_checked::key::DeclarationKey,
        right: &filament_checked::key::DeclarationKey,
    ) -> Result<std::cmp::Ordering, PolicyStateError> {
        self.work(1)?;
        let order = self.compare_bytes(left.package.as_bytes(), right.package.as_bytes())?;
        if order != std::cmp::Ordering::Equal {
            return Ok(order);
        }
        self.compare_bytes(left.node.as_bytes(), right.node.as_bytes())
    }
    pub(crate) fn text(&self, text: &str) -> Result<String, PolicyStateError> {
        let mut bytes = Vec::new();
        self.copy(&mut bytes, text.as_bytes())?;
        self.work(bytes.len())?;
        String::from_utf8(bytes).map_err(|_| PolicyStateError::Invalid)
    }
    pub(crate) fn compare_effectives(
        &self,
        left: &filament_exact::EffectiveId,
        right: &filament_exact::EffectiveId,
    ) -> Result<std::cmp::Ordering, PolicyStateError> {
        self.work(1)?;
        let order = left.owner_chain().len().cmp(&right.owner_chain().len());
        if order != std::cmp::Ordering::Equal {
            return Ok(order);
        }
        for (left, right) in left.owner_chain().iter().zip(right.owner_chain()) {
            self.work(1)?;
            let order =
                self.compare_bytes(left.package().as_bytes(), right.package().as_bytes())?;
            if order != std::cmp::Ordering::Equal {
                return Ok(order);
            }
            let order = self.compare_bytes(left.node().as_bytes(), right.node().as_bytes())?;
            if order != std::cmp::Ordering::Equal {
                return Ok(order);
            }
        }
        Ok(std::cmp::Ordering::Equal)
    }
    pub(crate) fn equal_effectives(
        &self,
        left: &filament_exact::EffectiveId,
        right: &filament_exact::EffectiveId,
    ) -> Result<bool, PolicyStateError> {
        self.work(1)?;
        if left.owner_chain().len() != right.owner_chain().len() {
            return Ok(false);
        }
        for (left, right) in left.owner_chain().iter().zip(right.owner_chain()) {
            self.work(1)?;
            if self.compare_bytes(left.package().as_bytes(), right.package().as_bytes())?
                != std::cmp::Ordering::Equal
                || self.compare_bytes(left.node().as_bytes(), right.node().as_bytes())?
                    != std::cmp::Ordering::Equal
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(crate) fn effective(
        &self,
        effective: &filament_exact::EffectiveId,
    ) -> Result<filament_exact::EffectiveId, PolicyStateError> {
        for original in effective.owner_chain() {
            self.work(1)?;
            self.work(original.package().len())?;
            self.work(original.node().len())?;
        }
        effective
            .try_clone_with(&mut |bytes| self.reserve(bytes))
            .map_err(|error| match error {
                filament_exact::IdentityCopyError::Policy(error) => error,
                filament_exact::IdentityCopyError::Allocation { requested }
                | filament_exact::IdentityCopyError::Capacity { requested } => {
                    PolicyStateError::Encoding(Error::Allocation { requested })
                }
            })
    }
    pub(crate) fn bridge(&self, error: PolicyStateError) -> Error {
        if self.stopped.borrow().is_none() {
            *self.stopped.borrow_mut() = Some(error);
        }
        Error::ReservationDenied { requested: 0 }
    }
    pub(crate) fn event(&self, units: usize) -> Result<(), Error> {
        self.work(units).map_err(|error| self.bridge(error))
    }
}
pub(crate) struct Provision<'a, 'm>(pub(crate) &'a Budget<'m>);
impl Reservation for Provision<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.0.reserve(bytes).map_err(|error| {
            self.0.bridge(error);
            ReservationDenied
        })
    }
}
pub(crate) struct Output<'a, 'm> {
    pub(crate) budget: &'a Budget<'m>,
    pub(crate) bytes: Vec<u8>,
}
impl Sink for Output<'_, '_> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.budget
            .copy(&mut self.bytes, bytes)
            .map_err(|error| self.budget.bridge(error))
    }
}

/// Borrow the existing FR-181 declaration projection for ordering instances.
pub(crate) fn declaration_bytes(
    key: &filament_checked::key::DeclarationKey,
    budget: &Budget<'_>,
) -> Result<Vec<u8>, PolicyStateError> {
    let mut output = Output {
        budget,
        bytes: Vec::new(),
    };
    let mut reservation = Provision(budget);
    let mut writer = quire_canonical::Writer::with_reservation(
        &mut output,
        quire_canonical::Limits::new(u64::MAX),
        &mut reservation,
    );
    let result = (|| {
        budget.event(1)?;
        writer.begin_object()?;
        budget.event(4 + key.node.len())?;
        writer.name("node")?;
        writer.string(&key.node)?;
        budget.event(7 + key.package.len())?;
        writer.name("package")?;
        writer.string(&key.package)?;
        writer.end_object()?;
        writer.finish().map(|_| ())
    })();
    if let Some(error) = budget.stopped.borrow_mut().take() {
        return Err(error);
    }
    result?;
    Ok(output.bytes)
}
