// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical eight-member bounded-measure product keys (FR-196).

use super::{Accumulator, Amount, MeasurePhase, MonitorState};
use crate::simulation::model::ModelKeyBudget;
use filament_exact::{Integer, NodeKey, ObjectReference, UnitId};
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
    /// Actual checked draw awaiting a post-state resolution, absent otherwise.
    pub pending: Option<NodeRef<'a>>,
    /// Checked reward descriptor, present exactly for reward/weighted forms.
    pub reward: Option<RewardKey<'a>>,
    /// Cumulative caller work, representation and storage admission.
    pub budget: &'a RefCell<&'budget mut dyn ModelKeyBudget>,
    /// Actual raw identity byte ceiling for decimal rendering.
    pub max_bytes: u64,
}

impl MeasureProductKey<'_, '_> {
    /// Validate structural control and reward members before budgeted encoding.
    ///
    /// # Errors
    /// A non-measure control, mismatched accumulator or missing/extraneous reward.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(
            (self.control, self.accumulator),
            (
                MonitorState::Measure { .. },
                Accumulator::Steps { .. } | Accumulator::Reward { .. }
            ) | (
                MonitorState::Fraction { .. },
                Accumulator::CountFraction { .. } | Accumulator::WeightedFraction { .. }
            )
        ) {
            return Err("measure control and accumulator differ");
        }
        if matches!(
            self.accumulator,
            Accumulator::Reward { .. } | Accumulator::WeightedFraction { .. }
        ) != self.reward.is_some()
        {
            return Err("measure reward descriptor differs");
        }
        Ok(())
    }

    fn integer<S: Sink + ?Sized>(
        &self,
        value: &Integer,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        let mut budget = self.budget.borrow_mut();
        budget
            .integer(value)
            .map_err(|_| Error::ReservationDenied { requested: 0 })?;
        fsl_foundation::scalar_wire::integer(value, self.max_bytes, Some(&mut **budget), writer)
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
            .ok_or_else(|| Error::Serialize("reward descriptor required".into()))?;
        writer.begin_array()?;
        fsl_foundation::node_wire::encode_node_key(reward.model, writer)?;
        writer.string(reward.name.as_str())?;
        writer.end_array()?;
        fsl_foundation::scalar_wire::unit(reward.unit, writer, |value, writer| {
            self.integer(value, writer)
        })
    }
}

impl Encode for MeasureProductKey<'_, '_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.validate()
            .map_err(|cause| Error::Serialize(cause.into()))?;
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
                return Err(Error::Serialize("control is not a bounded measure".into()))
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
        match self.pending {
            Some(pending) => pending.encode_into(writer)?,
            None => writer.null()?,
        }
        writer.end_array()
    }
}

impl super::MeasurePlan<'_> {
    /// Restore this claim's fraction control and accumulator. The model and
    /// reward descriptor are checked by the caller's full owner-key round trip.
    ///
    /// # Errors
    /// An impossible phase/count/sum or a caller's actual admission stop.
    pub fn restore_fraction_control(
        &self,
        control: NodeRef<'_>,
        accumulator: NodeRef<'_>,
        budget: &mut dyn crate::simulation::model::ModelRestoreBudget,
    ) -> Result<(MonitorState, Accumulator), crate::simulation::model::ModelRestoreError> {
        use crate::simulation::model::ModelRestoreError as Stop;
        use filament_exact::{
            OrderedOperands, OrderingOperator, Outcome, Rational, RationalArithmetic,
        };
        use quire_canonical::Node;
        let malformed = || Stop::Malformed("control");
        fn parts<const N: usize>(node: NodeRef<'_>) -> Result<[NodeRef<'_>; N], Stop> {
            let Node::Array(items) = node.node() else {
                return Err(Stop::Malformed("control"));
            };
            if items.len() != N {
                return Err(Stop::Malformed("control"));
            }
            items
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| Stop::Malformed("control"))
        }
        fn text(node: NodeRef<'_>) -> Result<&str, Stop> {
            match node.node() {
                Node::String(text) => Ok(text),
                _ => Err(Stop::Malformed("control")),
            }
        }
        fn completed<T>(outcome: Outcome<T>) -> Result<T, Stop> {
            match outcome {
                Outcome::Completed(value) => Ok(value),
                Outcome::Incomplete(error) => Err(Stop::Accounting(Box::new(error))),
                Outcome::Undefined(_) | Outcome::Refused(_) => Err(Stop::Malformed("accumulator")),
            }
        }
        fn rational(
            node: NodeRef<'_>,
            budget: &mut dyn crate::simulation::model::ModelRestoreBudget,
        ) -> Result<Rational, Stop> {
            let [n, d] = parts::<2>(node)?;
            let mut read = |node| -> Result<Integer, Stop> {
                let text = text(node)?;
                budget
                    .work(
                        u64::try_from(text.len().saturating_mul(text.len())).map_err(|_| {
                            Stop::Key(crate::simulation::model::ModelKeyError::SizeOverflow)
                        })?,
                    )
                    .map_err(|_| Stop::Key(crate::simulation::model::ModelKeyError::Denied))?;
                budget
                    .reserve(text.len().saturating_mul(4))
                    .map_err(|_| Stop::Key(crate::simulation::model::ModelKeyError::Denied))?;
                let value = text.parse().map_err(|_| Stop::Malformed("accumulator"))?;
                budget
                    .integer(&value)
                    .map_err(|_| Stop::Key(crate::simulation::model::ModelKeyError::Denied))?;
                Ok(value)
            };
            let n = Rational::from_integer(read(n)?);
            let d = Rational::from_integer(read(d)?);
            completed(filament_exact::evaluate_rational_arithmetic(
                RationalArithmetic::Divide(&n, &d),
                None,
                budget.meter(),
            ))
        }
        let super::MeasurePlan::Fraction { window, weight } = self else {
            return Err(malformed());
        };
        budget
            .work(1)
            .map_err(|_| Stop::Key(crate::simulation::model::ModelKeyError::Denied))?;
        budget
            .reserve(16 * std::mem::size_of::<NodeRef<'_>>())
            .map_err(|_| Stop::Key(crate::simulation::model::ModelKeyError::Denied))?;
        let [tag, remaining, phase] = parts::<3>(control)?;
        if text(tag)? != "fraction" {
            return Err(malformed());
        }
        let remaining: u64 = text(remaining)?.parse().map_err(|_| malformed())?;
        let positions = window.checked_add(1).ok_or_else(malformed)?;
        if remaining > positions
            || text(phase)?
                != if remaining == 0 {
                    "complete"
                } else {
                    "collecting"
                }
        {
            return Err(malformed());
        }
        let sums = match weight {
            None => {
                let [tag, truths, count] = parts::<3>(accumulator)?;
                if text(tag)? != "count-fraction" {
                    return Err(malformed());
                }
                let true_count: u64 = text(truths)?.parse().map_err(|_| malformed())?;
                let position_count: u64 = text(count)?.parse().map_err(|_| malformed())?;
                if true_count > position_count
                    || position_count.checked_add(remaining) != Some(positions)
                {
                    return Err(malformed());
                }
                Accumulator::CountFraction {
                    true_count,
                    position_count,
                }
            }
            Some(_) => {
                let [tag, _, _, truths, total] = parts::<5>(accumulator)?;
                if text(tag)? != "weighted-fraction" {
                    return Err(malformed());
                }
                let true_sum = rational(truths, budget)?;
                let total_sum = rational(total, budget)?;
                let zero = Rational::from_integer(Integer::zero());
                for (left, right) in [(&zero, &true_sum), (&true_sum, &total_sum)] {
                    if !completed(filament_exact::order_numbers(
                        OrderingOperator::LessOrEqual,
                        OrderedOperands::Rationals(left, right),
                        budget.meter(),
                    ))? {
                        return Err(malformed());
                    }
                }
                if remaining == positions && (!true_sum.is_zero() || !total_sum.is_zero())
                    || remaining == 0 && total_sum.is_zero()
                {
                    return Err(malformed());
                }
                Accumulator::WeightedFraction {
                    true_sum,
                    total_sum,
                }
            }
        };
        Ok((MonitorState::Fraction { remaining }, sums))
    }
}
