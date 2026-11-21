// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original temporal-instance admission through the native closed universe.

use filament_exact::{Cancel, EffectiveId, IntegerDecimalBudget};

use super::{ModelKeyError, ModelSystem};
use crate::model_subject::ModelInstance;

fn admit(
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
    units: usize,
) -> Result<(), ModelKeyError> {
    cancel.poll();
    if let Some(cause) = cancel.tripped() {
        return Err(ModelKeyError::Cancelled(cause));
    }
    budget
        .work(u64::try_from(units).map_err(|_| ModelKeyError::SizeOverflow)?)
        .map_err(|_| ModelKeyError::Denied)
}

impl ModelSystem<'_> {
    /// Check an original temporal selection against this admitted model's
    /// exact population declaration, closed key universe and native FR125
    /// parameter overlap. No state or population is inferred from a reference.
    /// `initial_model_index` names the retained base model initial state,
    /// before protocol Cartesian or timed clock-count expansion.
    ///
    /// # Errors
    /// Preserves the caller's existing work denial and cancellation; `false`
    /// means the original selection is not an unambiguous admitted instance.
    pub fn admits_over_instance(
        &self,
        selected: &ModelInstance,
        parameter: &EffectiveId,
        initial_model_index: usize,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
    ) -> Result<bool, ModelKeyError> {
        admit(budget, cancel, 1)?;
        if self.initial.get(initial_model_index).is_none() {
            return Ok(false);
        }
        let selected_bytes = selected
            .population
            .package
            .len()
            .checked_add(selected.population.node.len())
            .and_then(|size| size.checked_add(1))
            .ok_or(ModelKeyError::SizeOverflow)?;
        // A tree lookup makes at most one comparison per retained population;
        // each prefix comparison is bounded by the selected declaration bytes.
        let lookup = selected_bytes
            .checked_mul(self.universes.len().max(1))
            .ok_or(ModelKeyError::SizeOverflow)?;
        admit(budget, cancel, lookup)?;
        let Some(keys) = self.universes.get(&selected.population) else {
            return Ok(false);
        };
        let mut keys = keys.iter();
        loop {
            admit(budget, cancel, 1)?;
            let Some(key) = keys.next() else {
                return Ok(false);
            };
            admit(
                budget,
                cancel,
                key.len()
                    .checked_add(selected.key.len())
                    .ok_or(ModelKeyError::SizeOverflow)?,
            )?;
            if key == &selected.key {
                break;
            }
        }
        let mut populations = self.populations.iter();
        let mut overlap = None;
        loop {
            admit(budget, cancel, 1)?;
            let Some(population) = populations.next() else {
                break;
            };
            let compared = selected_bytes
                .checked_add(population.key.package.len())
                .and_then(|size| size.checked_add(population.key.node.len()))
                .ok_or(ModelKeyError::SizeOverflow)?;
            admit(budget, cancel, compared)?;
            if population.key == selected.population {
                if overlap.is_some() {
                    return Ok(false);
                }
                admit(budget, cancel, population.member_types.len())?;
                overlap = Some(self.overlaps(population, parameter.clone()));
            }
        }
        Ok(overlap.unwrap_or(false))
    }
}
