// SPDX-License-Identifier: AGPL-3.0-or-later
//! One allowance of the FR-255 limits, shared by every clone.
//!
//! A [`Budget`] holds a bound and a consumed total for every [`Setting`], and
//! the cumulative `accounting.work_units` allowance of a
//! [`filament_exact::WorkScope`]. A caller charges before the step it pays
//! for; a charge the bound cannot cover returns a [`BudgetStop`] and changes
//! nothing, so the step is never taken.

use crate::diagnostic::LimitExceeded;
use crate::setting::{Setting, SettingLimits};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use filament_exact::{Cancel, CancelCause, Incomplete, Meter, ScalarLimits, WorkScope};
use ix_trace_rs::implements;

/// Why a [`Budget`] refused a charge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BudgetStop {
    /// A setting's bound cannot cover the charge.
    Limit(LimitExceeded),
    /// A `filament.value.accounting/v1` counter cannot cover a kernel charge.
    Accounting(Incomplete),
    /// The caller's handle was cancelled, or its deadline passed.
    Cancelled(CancelCause),
}

impl From<LimitExceeded> for BudgetStop {
    fn from(limit: LimitExceeded) -> Self {
        Self::Limit(limit)
    }
}

impl From<Incomplete> for BudgetStop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Accounting(incomplete)
    }
}

/// The allowance of one unit of work, such as one analysis item.
///
/// Clones share every total, so a producer, its checker and parallel workers
/// draw on one allowance. A row with no published default and no caller value
/// has bound 0 and admits nothing.
#[derive(Clone, Debug)]
pub struct Budget {
    work: WorkScope,
    cancel: Cancel,
    counters: Arc<Counters>,
}

#[derive(Debug)]
struct Counters {
    bounds: [u64; Setting::COUNT],
    consumed: [AtomicU64; Setting::COUNT],
}

impl Budget {
    /// A budget with nothing consumed. Each setting takes the value `limits`
    /// reports for it, else its published default; `accounting` bounds the
    /// kernel meters and their cumulative work; `cancel` is polled at every
    /// charge.
    #[must_use]
    pub fn from_settings(
        limits: &dyn SettingLimits,
        accounting: ScalarLimits,
        cancel: &Cancel,
    ) -> Self {
        let mut bounds = [0; Setting::COUNT];
        for setting in Setting::ALL {
            bounds[setting.index()] = setting.default_bound().unwrap_or(0);
        }
        for (setting, bound) in limits.bounds() {
            bounds[setting.index()] = bound;
        }
        Self {
            work: WorkScope::new(accounting),
            cancel: cancel.clone(),
            counters: Arc::new(Counters {
                bounds,
                consumed: core::array::from_fn(|_| AtomicU64::new(0)),
            }),
        }
    }

    /// The bound in force for `setting`.
    #[must_use]
    pub fn bound(&self, setting: Setting) -> u64 {
        self.counters.bounds[setting.index()]
    }

    /// What has been charged to `setting`: the total of a cumulative row, or
    /// the largest size admitted under a size row.
    #[must_use]
    pub fn consumed(&self, setting: Setting) -> u64 {
        self.counters.consumed[setting.index()].load(Ordering::Relaxed)
    }

    /// Add `amount` (a count, work units or bytes) to `setting`'s total,
    /// before the step it pays for.
    ///
    /// # Errors
    ///
    /// [`BudgetStop::Cancelled`] for a cancelled handle, and
    /// [`BudgetStop::Limit`] naming the setting, its bound and the total the
    /// charge would have reached when that total exceeds the bound. A refused
    /// charge adds nothing.
    #[implements("FR-255-AC-28")]
    pub fn charge(&self, setting: Setting, amount: u64) -> Result<(), BudgetStop> {
        self.poll()?;
        let bound = self.bound(setting);
        self.counters.consumed[setting.index()]
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(amount).filter(|total| *total <= bound)
            })
            .map(drop)
            .map_err(|used| {
                let actual = u128::from(used) + u128::from(amount);
                LimitExceeded::new(setting, bound, actual).into()
            })
    }

    /// Admit one item of `size` under a setting that bounds each item rather
    /// than their total, such as a key length.
    ///
    /// # Errors
    ///
    /// As [`Self::charge`], with the refused size as the count reached.
    pub fn admit_size(&self, setting: Setting, size: u64) -> Result<(), BudgetStop> {
        self.poll()?;
        let bound = self.bound(setting);
        if size > bound {
            return Err(LimitExceeded::new(setting, bound, u128::from(size)).into());
        }
        self.counters.consumed[setting.index()].fetch_max(size, Ordering::Relaxed);
        Ok(())
    }

    /// The shared `accounting.work_units` allowance, for a caller that takes a
    /// [`WorkScope`].
    #[must_use]
    pub const fn work_scope(&self) -> &WorkScope {
        &self.work
    }

    /// A fresh kernel meter: its work draws on this budget's shared
    /// allowance and it polls this budget's cancel handle.
    #[must_use]
    pub fn meter(&self) -> Meter {
        self.work.meter().with_cancel(self.cancel.clone())
    }

    fn poll(&self) -> Result<(), BudgetStop> {
        match self.cancel.poll().then(|| self.cancel.cause()).flatten() {
            Some(cause) => Err(BudgetStop::Cancelled(cause)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Budget, BudgetStop};
    use crate::diagnostic::LimitExceeded;
    use crate::setting::{Setting, SettingLimits};
    use alloc::{vec, vec::Vec};
    use filament_exact::{
        Cancel, CancelCause, Charge, ChargePoint, Integer, LimitKind, ScalarLimits,
    };
    use ix_trace_rs::trace;

    struct Caller(Vec<(Setting, u64)>);

    impl SettingLimits for Caller {
        fn bounds(&self) -> Vec<(Setting, u64)> {
            self.0.clone()
        }

        fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
            self.0.push((setting, bound));
            true
        }
    }

    fn accounting(work_units: u64) -> ScalarLimits {
        ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units,
            result_units: 64,
        }
    }

    fn budget(bounds: Vec<(Setting, u64)>) -> Budget {
        Budget::from_settings(&Caller(bounds), accounting(5), &Cancel::new())
    }

    fn reached(setting: Setting, bound: u64, actual: u128) -> BudgetStop {
        BudgetStop::Limit(LimitExceeded::new(setting, bound, actual))
    }

    #[trace("FR-255-AC-28")]
    #[test]
    fn two_parts_sharing_an_allowance_admit_exactly_the_bound() {
        let candidates = Setting::ExpansionMaxCandidates;
        let producer = budget(vec![(candidates, 3)]);
        let checker = producer.clone();
        let mut steps = 0;
        let mut step = |part: &Budget| part.charge(candidates, 1).map(|()| steps += 1);
        for part in [&producer, &checker, &producer] {
            step(part).expect("within the shared bound");
        }
        assert_eq!(step(&checker), Err(reached(candidates, 3, 4)));
        assert_eq!(steps, 3);
        assert_eq!(producer.consumed(candidates), 3);
    }

    #[trace("FR-255-AC-6")]
    #[test]
    fn an_unset_row_takes_its_default_and_a_required_row_admits_nothing() {
        let budget = budget(vec![(Setting::S3Nodes, 4)]);
        assert_eq!(budget.bound(Setting::S3Nodes), 4);
        assert_eq!(budget.bound(Setting::ExpansionMaxCandidates), 200_000);
        let required = Setting::MonitorMaxActivations;
        assert_eq!(budget.charge(required, 1), Err(reached(required, 0, 1)));
    }

    #[trace("FR-255-AC-1")]
    #[test]
    fn a_size_bound_is_per_item_and_records_the_largest() {
        let bytes = Setting::IdentityInputBytes;
        let budget = budget(vec![(bytes, 10)]);
        for size in [7, 10, 3] {
            budget.admit_size(bytes, size).expect("each item fits");
        }
        assert_eq!(budget.consumed(bytes), 10);
        assert_eq!(budget.admit_size(bytes, 11), Err(reached(bytes, 10, 11)));
    }

    #[trace("FR-255-AC-23")]
    #[test]
    fn meters_share_the_cumulative_work_allowance() {
        let budget = budget(Vec::new());
        let work = |units: u32| {
            Charge::new(ChargePoint::CollectionVisit).work(Integer::from(u64::from(units)))
        };
        budget.meter().charge(work(3)).expect("3 of 5");
        let refused = budget.meter().charge(work(3)).expect_err("3 more exceed 5");
        assert_eq!(refused.limit_kind, LimitKind::WorkUnits);
        assert_eq!((refused.limit, refused.consumed), (5, 3));
        assert_eq!(budget.work_scope().consumed_work(), 3);
    }

    #[trace("FR-255")]
    #[test]
    fn a_cancelled_handle_stops_the_next_charge() {
        let cancel = Cancel::new();
        let budget = Budget::from_settings(&Caller(Vec::new()), accounting(5), &cancel);
        cancel.cancel(CancelCause::Deadline);
        assert_eq!(
            budget.charge(Setting::S3Nodes, 1),
            Err(BudgetStop::Cancelled(CancelCause::Deadline))
        );
        assert_eq!(budget.consumed(Setting::S3Nodes), 0);
    }
}
