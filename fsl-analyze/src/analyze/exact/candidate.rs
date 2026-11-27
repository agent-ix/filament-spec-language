// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cumulative candidate identity admission shared by quantitative proof profiles.

use super::*;
use fsl_replay::ProbabilityKeyAdmission;
use ix_trace_rs::implements;

pub(super) fn rational_bytes(value: &Rational) -> usize {
    usize::try_from(
        value
            .numerator()
            .magnitude_bits()
            .saturating_add(value.denominator().magnitude_bits())
            .div_ceil(8),
    )
    .unwrap_or(usize::MAX)
}

pub(super) struct CandidateBudget<'a> {
    pub(super) meter: filament_exact::Meter,
    retained: u64,
    storage_bound: u64,
    pub(super) identity: fsl_foundation::IdentityLimits,
    pub(super) cancel: &'a Cancel,
    work: &'a WorkScope,
}
impl<'a> CandidateBudget<'a> {
    #[implements("FR-203-AC-29")]
    pub(super) fn new(
        limits: ProbabilityDocumentLimits,
        work: &'a WorkScope,
        cancel: &'a Cancel,
    ) -> Self {
        Self {
            meter: work.meter().with_cancel(cancel.clone()),
            retained: 0,
            storage_bound: limits.storage_bytes,
            identity: fsl_foundation::IdentityLimits::default(),
            cancel,
            work,
        }
    }
    pub(super) fn work_scope(&self) -> &WorkScope {
        self.work
    }
    pub(super) fn admission(&mut self) -> ProbabilityKeyAdmission<'_> {
        ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.retained,
            self.storage_bound,
            self.cancel,
        )
    }
    pub(super) fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = self.admission();
        filament_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied))
    }
    pub(super) fn key(
        &mut self,
        binding: Option<&ObjectReference>,
        raw: &[u8],
    ) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
        let identity = self.identity;
        let cancel = self.cancel;
        let mut admission = self.admission();
        ProbabilityProductKey::for_model_state_with_budget(
            binding,
            raw,
            identity,
            &mut admission,
            cancel,
        )
        .map_err(|error| admission.refusal(error))
    }
    pub(super) fn retain_row(
        &mut self,
        row: &ExactProbabilityValue,
    ) -> Result<(), ProbabilityDocumentError> {
        self.reserve(
            std::mem::size_of::<ExactProbabilityValue>()
                .saturating_add(row.state.as_bytes().len())
                .saturating_add(rational_bytes(&row.value)),
        )?;
        let mut admission = self.admission();
        filament_exact::IntegerDecimalBudget::work(
            &mut admission,
            u64::try_from(
                row.state
                    .as_bytes()
                    .len()
                    .saturating_add(rational_bytes(&row.value)),
            )
            .unwrap_or(u64::MAX),
        )
        .map_err(|_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied))
    }
}

#[implements("FR-203-AC-29")]
pub(super) fn values(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
    budget: &mut CandidateBudget<'_>,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if product.states.len() != values.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    budget.reserve(
        values
            .len()
            .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
    )?;
    let mut rows = Vec::with_capacity(values.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        budget.reserve(rational_bytes(value))?;
        rows.push(ExactProbabilityValue {
            state: budget.key(binding, &state.model)?,
            value: value.clone(),
        });
    }
    sort(
        &mut rows,
        |row| &row.state,
        &mut budget.meter,
        budget.cancel,
    )?;
    if rows.windows(2).any(|pair| pair[0].state == pair[1].state) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(ExactProbabilityCertificate { values: rows })
}

// Heapsort keeps comparison work logarithmic per item without temporary rows.
#[implements("FR-203-AC-29")]
pub(super) fn sort<T>(
    rows: &mut [T],
    key: impl Fn(&T) -> &ProbabilityProductKey,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    let len = rows.len();
    for root in (0..len / 2).rev() {
        sift_down(rows, root, len, &key, meter, cancel)?;
    }
    for end in (1..len).rev() {
        rows.swap(0, end);
        sift_down(rows, 0, end, &key, meter, cancel)?;
    }
    Ok(())
}

#[implements("FR-203-AC-29")]
fn sift_down<T>(
    rows: &mut [T],
    mut root: usize,
    end: usize,
    key: &impl Fn(&T) -> &ProbabilityProductKey,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    loop {
        let left = root.saturating_mul(2).saturating_add(1);
        if left >= end {
            return Ok(());
        }
        let right = left.saturating_add(1);
        let child = if right < end
            && compare_rows(rows, left, right, key, meter, cancel)? == std::cmp::Ordering::Less
        {
            right
        } else {
            left
        };
        if compare_rows(rows, root, child, key, meter, cancel)? != std::cmp::Ordering::Less {
            return Ok(());
        }
        rows.swap(root, child);
        root = child;
    }
}

#[implements("FR-203-AC-29")]
fn compare_rows<T>(
    rows: &[T],
    left: usize,
    right: usize,
    key: &impl Fn(&T) -> &ProbabilityProductKey,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<std::cmp::Ordering, ProbabilityDocumentError> {
    if let Some(cause) = cancel.tripped() {
        return Err(ProbabilityDocumentError::Cancelled(cause));
    }
    let left_key = key(&rows[left]);
    let right_key = key(&rows[right]);
    let bytes = left_key
        .as_bytes()
        .len()
        .min(right_key.as_bytes().len())
        .saturating_add(1);
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge).work(
                filament_exact::Integer::from(u64::try_from(bytes).unwrap_or(u64::MAX)),
            ),
        )
        .map_err(|error| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(error)),
        })?;
    Ok(left_key.cmp(right_key))
}
