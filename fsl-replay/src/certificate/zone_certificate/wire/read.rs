// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admission policy for the single native zone-certificate metadata reader.

use super::*;

pub(super) trait Read {
    type Error: From<ZoneCertificateWireError> + From<TaskProductShapeRefusal>;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error>;
    fn work(&mut self, units: usize) -> Result<(), Self::Error>;
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error>;
    fn rational(
        &mut self,
        value: &Value,
        at: &str,
    ) -> Result<filament_exact::Rational, Self::Error>;
    fn state(&mut self, value: &Value, at: &str) -> Result<StateKey, Self::Error>;
    fn edge(
        &mut self,
        value: &Value,
        at: &str,
        source: bool,
    ) -> Result<EdgeTransitions, Self::Error>;
}

pub(super) struct Native;
impl Read for Native {
    type Error = ZoneCertificateWireError;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error> {
        let bytes = len
            .checked_mul(size_of::<T>())
            .ok_or(fsl_eval::simulation::model::ModelKeyError::SizeOverflow)?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(len).map_err(|_| {
            fsl_eval::simulation::model::ModelKeyError::Encoding(
                fsl_eval::simulation::EncodingRefusal::Allocation { requested: bytes },
            )
        })?;
        Ok(rows)
    }
    fn work(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error> {
        Ok(integer(value, at)?)
    }
    fn rational(
        &mut self,
        value: &Value,
        at: &str,
    ) -> Result<filament_exact::Rational, Self::Error> {
        Ok(rational(value, at)?)
    }
    fn state(&mut self, value: &Value, at: &str) -> Result<StateKey, Self::Error> {
        Ok(read_state(value, at)?)
    }
    fn edge(
        &mut self,
        value: &Value,
        at: &str,
        source: bool,
    ) -> Result<EdgeTransitions, Self::Error> {
        native_edge_read(value, at, source)
    }
}

impl Read for crate::ReferenceScope<'_, '_, '_, '_> {
    type Error = ZoneCertificateReferenceError;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error> {
        Ok(self.storage(len)?)
    }
    fn work(&mut self, units: usize) -> Result<(), Self::Error> {
        Ok(self.work(units)?)
    }
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error> {
        Ok(self.at_pointer(at, |s| s.metadata_integer(value))?)
    }
    fn rational(
        &mut self,
        value: &Value,
        at: &str,
    ) -> Result<filament_exact::Rational, Self::Error> {
        Ok(self.at_pointer(at, |s| s.metadata_rational(value))?)
    }
    fn state(&mut self, value: &Value, at: &str) -> Result<StateKey, Self::Error> {
        Ok(self.at_pointer(at, |s| s.metadata_state(value))?)
    }
    fn edge(
        &mut self,
        value: &Value,
        at: &str,
        source: bool,
    ) -> Result<EdgeTransitions, Self::Error> {
        if value.get("type").and_then(Value::as_str) == Some("task-transition-template") {
            Ok((
                None,
                Some(self.at_pointer(at, |s| s.restore_task_template(value))?),
            ))
        } else if value.is_null() && source {
            Ok((None, None))
        } else {
            Ok((
                Some(self.at_pointer(at, |s| s.restore_task_transition(value))?),
                None,
            ))
        }
    }
}
