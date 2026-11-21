// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native initial-state ordering with admission before expansion.

use filament_checked::key::DeclarationKey;
use filament_exact::{Cancel, Integer, IntegerDecimalBudget};

use super::{ClockValuation, ModelSimState, TimedState, TimedSystem};
use crate::simulation::model::ModelKeyError;
use crate::simulation::EncodingRefusal;

type Holder = (DeclarationKey, String, DeclarationKey, u64);

fn tick(budget: &mut dyn IntegerDecimalBudget, cancel: &Cancel) -> Result<(), ModelKeyError> {
    cancel.poll();
    if let Some(cause) = cancel.tripped() {
        return Err(ModelKeyError::Cancelled(cause));
    }
    budget.work(1).map_err(|_| ModelKeyError::Denied)
}

fn room<T>(
    values: &mut Vec<T>,
    budget: &mut dyn IntegerDecimalBudget,
) -> Result<(), ModelKeyError> {
    if values.len() < values.capacity() {
        return Ok(());
    }
    let capacity = values
        .capacity()
        .max(1)
        .checked_mul(2)
        .ok_or(ModelKeyError::SizeOverflow)?;
    let requested = capacity
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ModelKeyError::SizeOverflow)?;
    budget
        .reserve(requested)
        .map_err(|_| ModelKeyError::Denied)?;
    values
        .try_reserve_exact(capacity - values.len())
        .map_err(|_| EncodingRefusal::Allocation { requested }.into())
}

fn holders(
    timed: &TimedSystem<'_, '_>,
    state: &ModelSimState,
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
) -> Result<Vec<Holder>, ModelKeyError> {
    let mut result = Vec::new();
    for (type_key, field, min, max) in &timed.system.clock_counts {
        tick(budget, cancel)?;
        if max.is_some() {
            continue;
        }
        for (population, objects) in &state.state.populations {
            tick(budget, cancel)?;
            for (key, object) in objects {
                tick(budget, cancel)?;
                if object.type_identity == *type_key {
                    room(&mut result, budget)?;
                    result.push((population.clone(), key.clone(), field.key.clone(), *min));
                }
            }
        }
    }
    Ok(result)
}

// Advance a composition in the same lexicographic order as `distributions`,
// retaining only one admitted count vector rather than the whole level.
fn next(
    added: &mut [u64],
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
) -> Result<bool, ModelKeyError> {
    let Some(last) = added.len().checked_sub(1) else {
        return Ok(false);
    };
    let mut suffix = added[last];
    for index in (0..last).rev() {
        tick(budget, cancel)?;
        if suffix != 0 {
            added[index] = added[index]
                .checked_add(1)
                .ok_or(ModelKeyError::SizeOverflow)?;
            for value in &mut added[index + 1..last] {
                tick(budget, cancel)?;
                *value = 0;
            }
            added[last] = suffix - 1;
            return Ok(true);
        }
        suffix = suffix
            .checked_add(added[index])
            .ok_or(ModelKeyError::SizeOverflow)?;
    }
    Ok(false)
}

fn retain(
    states: &mut Vec<(u64, usize, TimedState)>,
    item: (u64, usize, TimedState),
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
) -> Result<(), ModelKeyError> {
    // Stable insertion gives the existing ascending-count order without an
    // unadmitted temporary allocation inside stable sorting.
    let mut at = states.len();
    while at != 0 {
        tick(budget, cancel)?;
        if states[at - 1].0 <= item.0 {
            break;
        }
        at -= 1;
    }
    room(states, budget)?;
    states.insert(at, item);
    Ok(())
}

pub(super) fn read(
    timed: &TimedSystem<'_, '_>,
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
) -> Result<Vec<(usize, TimedState)>, ModelKeyError> {
    let mut states = Vec::new();
    for (index, original) in timed.system.initial.iter().enumerate() {
        tick(budget, cancel)?;
        let holders = if timed.free_delay {
            Vec::new()
        } else {
            holders(timed, original, budget, cancel)?
        };
        let mut added = Vec::new();
        for _ in &holders {
            tick(budget, cancel)?;
            room(&mut added, budget)?;
            added.push(0_u64);
        }
        let mut total = 0_u64;
        loop {
            tick(budget, cancel)?;
            for value in &mut added {
                tick(budget, cancel)?;
                *value = 0;
            }
            if let Some(last) = added.last_mut() {
                *last = total;
            }
            let beyond = loop {
                tick(budget, cancel)?;
                let mut state = original.clone();
                for ((population, key, field, min), extra) in holders.iter().zip(&added) {
                    tick(budget, cancel)?;
                    if let Some(object) = state
                        .state
                        .populations
                        .get_mut(population)
                        .and_then(|objects| objects.get_mut(key))
                    {
                        object.fields.insert(
                            field.clone(),
                            filament_exact::Value::Integer(Integer::from(
                                min.saturating_add(*extra),
                            )),
                        );
                    }
                }
                let beyond =
                    !holders.is_empty() && timed.live_clocks(&state) > timed.max_clocks.get();
                let mut clocks = Vec::new();
                if !timed.free_delay {
                    for _ in &timed.places {
                        tick(budget, cancel)?;
                        room(&mut clocks, budget)?;
                        clocks.push(super::zero());
                    }
                }
                retain(
                    &mut states,
                    (
                        total,
                        index,
                        TimedState::new(state, ClockValuation::new(clocks)),
                    ),
                    budget,
                    cancel,
                )?;
                if beyond || !next(&mut added, budget, cancel)? {
                    break beyond;
                }
            };
            if holders.is_empty() || beyond {
                break;
            }
            total = total.checked_add(1).ok_or(ModelKeyError::SizeOverflow)?;
        }
    }
    let mut result = Vec::new();
    for (_, index, state) in states {
        tick(budget, cancel)?;
        room(&mut result, budget)?;
        result.push((index, state));
    }
    Ok(result)
}
