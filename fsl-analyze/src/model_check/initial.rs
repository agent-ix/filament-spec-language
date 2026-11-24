// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-126-AC-26 (ADR-018 EN-1): the initial states of an explicit-state run,
//! keyed and coalesced one at a time under the `max_states` budget.

use std::collections::BTreeMap;

#[cfg(feature = "test-support")]
thread_local! {
    static KEYED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many initial states this thread has handed to the keying step of
/// [`admit_initial`] so far: the count FR-126-AC-26's tests read.
#[cfg(feature = "test-support")]
pub fn keyed_initial_states() -> u64 {
    KEYED.with(std::cell::Cell::get)
}

/// Why an initial set was not admitted.
pub(super) enum Refused<E, I> {
    /// The distinct keys exceed the budget; the states after the one that
    /// exceeded it were never keyed.
    OverBudget,
    /// Construction failed at a state pulled before the budget refused.
    Construction(I),
    /// Keying a state failed.
    Keying(E),
}

/// Key each initial state through `key` and coalesce equal keys into the
/// first state that has the key, returning `(key, index, payload)` ascending
/// by key. Stops as soon as the distinct count exceeds `max_states`, before
/// constructing or keying the states that follow. A construction failure
/// reached before that stop is passed through unchanged.
///
/// # Errors
///
/// [`Refused::OverBudget`] past the budget, [`Refused::Construction`] with a
/// source error, or [`Refused::Keying`] with a key error.
pub(super) fn admit_initial<T, P, E, I>(
    states: impl IntoIterator<Item = Result<T, I>>,
    max_states: usize,
    mut key: impl FnMut(usize, T) -> Result<(Vec<u8>, P), E>,
) -> Result<Vec<(Vec<u8>, usize, P)>, Refused<E, I>> {
    let mut distinct: BTreeMap<Vec<u8>, (usize, P)> = BTreeMap::new();
    for (index, state) in states.into_iter().enumerate() {
        let state = state.map_err(Refused::Construction)?;
        #[cfg(feature = "test-support")]
        KEYED.with(|keyed| keyed.set(keyed.get().saturating_add(1)));
        let (bytes, payload) = key(index, state).map_err(Refused::Keying)?;
        distinct.entry(bytes).or_insert((index, payload));
        if distinct.len() > max_states {
            return Err(Refused::OverBudget);
        }
    }
    Ok(distinct
        .into_iter()
        .map(|(bytes, (index, payload))| (bytes, index, payload))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::convert::Infallible;

    use ix_trace_rs::trace;

    use super::*;

    /// FR-126-AC-26: with `max_states` 1 and a thousand distinct initial
    /// states, exactly two are pulled from the source and keyed before the
    /// refusal.
    #[test]
    #[trace("TC-521", "FR-126-AC-26")]
    fn over_budget_initial_set_stops_after_the_state_that_exceeds_it() {
        let pulled = Cell::new(0_usize);
        let source = (0_u32..1000)
            .inspect(|_| pulled.set(pulled.get() + 1))
            .map(Ok::<_, Infallible>);
        let mut keyed = 0_usize;
        let refused = admit_initial(source, 1, |_, state| -> Result<_, ()> {
            keyed += 1;
            Ok((state.to_be_bytes().to_vec(), ()))
        });
        assert!(matches!(refused, Err(Refused::OverBudget)));
        assert_eq!(keyed, 2);
        assert_eq!(
            pulled.get(),
            2,
            "the source yields no state past the second"
        );
    }

    /// FR-126-AC-26: equal keys coalesce into the first state and consume one
    /// slot; the result is ascending by key.
    #[test]
    #[trace("TC-521", "FR-126-AC-26")]
    fn equal_keys_coalesce_into_the_first_state() {
        let source = [3_u8, 1, 3, 1].map(Ok::<_, Infallible>);
        let admitted = admit_initial(source, 2, |index, state| -> Result<_, ()> {
            Ok((vec![state], index))
        });
        let Ok(admitted) = admitted else {
            panic!("two distinct keys fit a budget of two");
        };
        assert_eq!(admitted, vec![(vec![1], 1, 1), (vec![3], 0, 0)]);
    }

    /// FR-126-AC-26: a keying failure before the budget is exceeded is
    /// reported.
    #[test]
    #[trace("TC-521", "FR-126-AC-26")]
    fn a_keying_failure_is_reported() {
        let refused = admit_initial([Ok::<_, Infallible>(1_u8)], 4, |_, _| {
            Err::<(Vec<u8>, ()), _>("unkeyable")
        });
        assert!(matches!(refused, Err(Refused::Keying("unkeyable"))));
    }

    /// FR-126-AC-26: a fault during construction reaches the caller with
    /// the same variant and invariant. A state past the refusal is unobserved.
    #[test]
    #[trace("TC-521", "FR-126-AC-26")]
    fn construction_fault_precedence_follows_initial_admission_order() {
        use filament_semantic_value::catalog::InternalFault;
        use fsl_eval::simulation::protocol::ProtocolRefusal;

        let fault = ProtocolRefusal::Fault(InternalFault::new(
            "protocol-simulation",
            "settling-reaches-a-rest-point",
        ));
        let constructed = Cell::new(0_u8);
        let source = || {
            constructed.set(0);
            (0_u8..4).map(|state| {
                constructed.set(constructed.get() + 1);
                if state == 2 {
                    Err(fault.clone())
                } else {
                    Ok(state)
                }
            })
        };
        let key = |_: usize, state| Ok::<_, Infallible>((vec![state], ()));
        assert!(matches!(
            admit_initial(source(), 2, key),
            Err(Refused::Construction(refusal)) if refusal == fault
        ));
        assert_eq!(constructed.get(), 3);
        assert!(matches!(
            admit_initial(source(), 1, key),
            Err(Refused::OverBudget)
        ));
        assert_eq!(constructed.get(), 2);
    }
}
