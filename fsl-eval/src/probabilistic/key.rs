// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical eight-member bounded-measure product keys (FR-196).

use super::{Accumulator, Amount, MeasurePhase, MonitorState};
use crate::simulation::model::ModelKeyBudget;
use filament_exact::{
    Integer, IntegerDecimalBudgetError, IntegerDecimalError, NodeKey, ObjectReference, UnitId,
};
use quire_canonical::{Encode, Error, NodeRef, Sink, Writer};
use std::cell::RefCell;

/// Borrowed, checked reward descriptor used by accumulator identity.
#[derive(Clone, Copy)]
pub struct RewardKey<'a> {
    /// The checked declaring model's structural node.
    pub model: &'a NodeKey,
    /// The exact checked reward identifier.
    pub name: &'a filament_exact::Identifier,
    /// The actual selected unit, absent for an integer reward.
    pub unit: Option<&'a UnitId>,
}

/// Borrowed canonical key source. The model is the parsed complete simulation
/// state body; the caller admits its bytes through the model identity owner.
/// Null instance, activation and pending select an origin, resolved position.
pub struct MeasureProductKey<'a, 'budget> {
    /// The actual claim binding, absent when the claim has none.
    pub binding: Option<&'a ObjectReference>,
    /// Complete parsed model-state body.
    pub model: NodeRef<'a>,
    /// Shared bounded measure/fraction control.
    pub control: MonitorState,
    /// Shared exact amount or sums.
    pub accumulator: &'a Accumulator,
    /// Checked reward descriptor, present exactly for reward/weighted forms.
    pub reward: Option<RewardKey<'a>>,
    /// Cumulative caller work, representation and storage admission.
    pub budget: &'a RefCell<&'budget mut dyn ModelKeyBudget>,
    /// Actual raw identity byte ceiling for decimal rendering.
    pub max_bytes: u64,
}

impl MeasureProductKey<'_, '_> {
    fn integer<S: Sink + ?Sized>(
        &self,
        value: &Integer,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        self.budget
            .borrow_mut()
            .integer(value)
            .map_err(|_| Error::ReservationDenied { requested: 0 })?;
        let text = value
            .try_to_decimal_string_with_budget(
                usize::try_from(self.max_bytes).unwrap_or(usize::MAX),
                &mut **self.budget.borrow_mut(),
            )
            .map_err(|error| match error {
                IntegerDecimalBudgetError::Denied => Error::ReservationDenied { requested: 0 },
                IntegerDecimalBudgetError::Decimal(IntegerDecimalError::Allocation {
                    requested,
                }) => Error::Allocation { requested },
                IntegerDecimalBudgetError::Decimal(IntegerDecimalError::ByteLimit {
                    required,
                    limit,
                }) => Error::Limit(quire_canonical::LimitExceeded {
                    kind: quire_canonical::LimitKind::CanonicalBytes,
                    bound: u64::try_from(limit).unwrap_or(u64::MAX),
                    required: u64::try_from(required).unwrap_or(u64::MAX),
                }),
                IntegerDecimalBudgetError::Decimal(IntegerDecimalError::OutputSizeOverflow) => {
                    Error::Allocation {
                        requested: usize::MAX,
                    }
                }
            })?;
        writer.string(&text)
    }
    fn rational<S: Sink + ?Sized>(
        &self,
        value: &filament_exact::Rational,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_array()?;
        self.integer(value.numerator(), writer)?;
        self.integer(value.denominator(), writer)?;
        writer.end_array()
    }
    fn amount<S: Sink + ?Sized>(
        &self,
        amount: &Amount,
        steps: bool,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        match amount {
            Amount::Exact(value) if steps => self.integer(value.numerator(), writer),
            Amount::Exact(value) => self.rational(value, writer),
            Amount::AboveBound | Amount::Censored => {
                writer.begin_array()?;
                writer.string(if matches!(amount, Amount::AboveBound) {
                    "above-bound"
                } else {
                    "censored"
                })?;
                writer.end_array()
            }
        }
    }
    fn reward<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let reward = self
            .reward
            .ok_or(Error::ReservationDenied { requested: 0 })?;
        writer.begin_array()?;
        fsl_foundation::node_wire::encode_node_key(reward.model, writer)?;
        writer.string(reward.name.as_str())?;
        writer.end_array()?;
        match reward.unit {
            None => writer.null(),
            Some(unit) => {
                writer.begin_array()?;
                if let Some(key) = unit.declared_key() {
                    writer.string("declared")?;
                    fsl_foundation::node_wire::encode_node_key(key, writer)?;
                } else {
                    writer.string("compound")?;
                    writer.begin_array()?;
                    for (key, exponent) in unit.compound_terms().unwrap_or_default() {
                        writer.begin_object()?;
                        writer.name("unit")?;
                        fsl_foundation::node_wire::encode_node_key(key, writer)?;
                        writer.name("exponent")?;
                        self.integer(exponent, writer)?;
                        writer.end_object()?;
                    }
                    writer.end_array()?;
                }
                writer.end_array()
            }
        }
    }
}

impl Encode for MeasureProductKey<'_, '_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        writer.string("filament.probability-product-key/v1")?;
        match self.binding {
            Some(binding) => crate::simulation::model::encode_reference(binding, writer)?,
            None => writer.null()?,
        }
        writer.null()?;
        writer.null()?;
        writer.begin_array()?;
        writer.string("filament.simulation.state-key/v1")?;
        self.model.encode_into(writer)?;
        writer.end_array()?;
        writer.begin_array()?;
        match self.control {
            MonitorState::Measure { remaining, phase } => {
                writer.string("measure")?;
                self.integer(&Integer::from(remaining), writer)?;
                writer.string(match phase {
                    MeasurePhase::Inactive => "inactive",
                    MeasurePhase::Active => "active",
                    MeasurePhase::Complete => "complete",
                    MeasurePhase::Censored => "censored",
                    MeasurePhase::Unactivated => "unactivated",
                })?;
            }
            MonitorState::Fraction { remaining } => {
                writer.string("fraction")?;
                self.integer(&Integer::from(remaining), writer)?;
                writer.string(if remaining == 0 {
                    "complete"
                } else {
                    "collecting"
                })?;
            }
            MonitorState::Formula(_) | MonitorState::Deadline { .. } => {
                return Err(Error::ReservationDenied { requested: 0 })
            }
        }
        writer.end_array()?;
        writer.begin_array()?;
        match self.accumulator {
            Accumulator::Steps { activated, amount } => {
                writer.string("steps")?;
                writer.bool(*activated)?;
                self.amount(amount, true, writer)?;
            }
            Accumulator::Reward { activated, amount } => {
                writer.string("reward")?;
                writer.bool(*activated)?;
                self.reward(writer)?;
                self.amount(amount, false, writer)?;
            }
            Accumulator::CountFraction {
                true_count,
                position_count,
            } => {
                writer.string("count-fraction")?;
                self.integer(&Integer::from(*true_count), writer)?;
                self.integer(&Integer::from(*position_count), writer)?;
            }
            Accumulator::WeightedFraction {
                true_sum,
                total_sum,
            } => {
                writer.string("weighted-fraction")?;
                self.reward(writer)?;
                self.rational(true_sum, writer)?;
                self.rational(total_sum, writer)?;
            }
        }
        writer.end_array()?;
        writer.null()?;
        writer.end_array()
    }
}
