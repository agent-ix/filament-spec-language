// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource policy for the single task-product metadata parser.

use super::*;

pub(super) trait Read {
    type Error: From<TaskProductShapeRefusal>;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error>;
    fn work(&mut self, units: usize) -> Result<(), Self::Error>;
    fn text(&mut self, value: &Value, at: &str) -> Result<String, Self::Error>;
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error>;
    fn rational(&mut self, value: &Value, at: &str) -> Result<Rational, Self::Error>;
    fn template(&mut self, value: &Value, at: &str) -> Result<TaskTransitionTemplate, Self::Error>;
}

pub(super) struct Native;
impl Read for Native {
    type Error = TaskProductShapeRefusal;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error> {
        Ok(Vec::with_capacity(len))
    }
    fn work(&mut self, _: usize) -> Result<(), Self::Error> {
        Ok(())
    }
    fn text(&mut self, value: &Value, at: &str) -> Result<String, Self::Error> {
        text(value, at)
    }
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error> {
        integer(value, at)
    }
    fn rational(&mut self, value: &Value, at: &str) -> Result<Rational, Self::Error> {
        rational(value, at)
    }
    fn template(&mut self, value: &Value, at: &str) -> Result<TaskTransitionTemplate, Self::Error> {
        TaskTransitionTemplate::read(value, at)
    }
}

impl Read for crate::ReferenceScope<'_, '_, '_, '_> {
    type Error = TaskProductReferenceError;
    fn rows<T>(&mut self, len: usize) -> Result<Vec<T>, Self::Error> {
        Ok(self.storage(len)?)
    }
    fn work(&mut self, units: usize) -> Result<(), Self::Error> {
        Ok(self.work(units)?)
    }
    fn text(&mut self, value: &Value, at: &str) -> Result<String, Self::Error> {
        if !value.is_string() {
            return refuse(at, TaskProductShapeReason::WrongKind).map_err(Into::into);
        }
        Ok(self.at_pointer(at, |s| s.metadata_text(value))?)
    }
    fn integer(&mut self, value: &Value, at: &str) -> Result<Integer, Self::Error> {
        Ok(self.at_pointer(at, |s| s.metadata_integer(value))?)
    }
    fn rational(&mut self, value: &Value, at: &str) -> Result<Rational, Self::Error> {
        Ok(self.at_pointer(at, |s| s.metadata_rational(value))?)
    }
    fn template(&mut self, value: &Value, at: &str) -> Result<TaskTransitionTemplate, Self::Error> {
        Ok(self.at_pointer(at, |s| s.restore_task_template(value))?)
    }
}

pub(super) fn rows<R: Read, T>(
    read: &mut R,
    value: &Value,
    at: &str,
    mut row: impl FnMut(&mut R, &Value, &str) -> Result<T, R::Error>,
) -> Result<Vec<T>, R::Error> {
    let input = array(value, at)?;
    let mut output = read.rows(input.len())?;
    for (index, value) in input.iter().enumerate() {
        read.work(1)?;
        output.push(row(read, value, &child(at, index))?);
    }
    Ok(output)
}
