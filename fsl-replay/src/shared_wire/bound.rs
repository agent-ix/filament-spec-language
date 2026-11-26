// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-413's claim bound, which a probabilistic counterexample and a
//! statistical verdict (FR-410) both carry.

use filament_exact::Rational;
use fsl_semantics::check::{Bound, ProbForm};
use serde::de::{Deserialize, Deserializer, Error as _};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

use super::{closed_object, member, unique_object, WireDefect, WireDefectKind};
use crate::certificate::task_product::{rational, rational_wire};
use crate::statistical::{BoundComparison, ClaimForm, VerdictBound};

/// The `type` of a claim bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundForm {
    /// A form EN-4 decides.
    Claim(ClaimForm),
    /// An expected accumulated reward, which only exact evidence decides.
    ExpectedTotal,
}

impl BoundForm {
    const ALL: [Self; 5] = [
        Self::Claim(ClaimForm::Probability),
        Self::Claim(ClaimForm::Quantile),
        Self::Claim(ClaimForm::MeanOfFraction),
        Self::Claim(ClaimForm::LongRunFraction),
        Self::ExpectedTotal,
    ];

    /// The written `type`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claim(form) => form.bound_type(),
            Self::ExpectedTotal => "expected-total",
        }
    }
}

/// `{type, comparison, threshold}` and, exactly for a quantile, `level`: the
/// claim's own comparison and threshold, never a reduction of them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilisticBound {
    form: BoundForm,
    comparison: BoundComparison,
    threshold: Rational,
    level: Option<Rational>,
}

const COMPARISONS: [BoundComparison; 2] = [
    BoundComparison::GreaterOrEqual,
    BoundComparison::LessOrEqual,
];

impl ProbabilisticBound {
    /// The bound of the checked claim form `form`, `None` for a form with no
    /// claim bound.
    pub fn of_claim(form: &ProbForm) -> Option<Self> {
        let (form, bound, level) = match form {
            ProbForm::Reach { bound, .. } | ProbForm::Probability { bound, .. } => {
                (BoundForm::Claim(ClaimForm::Probability), bound, None)
            }
            ProbForm::MeanOfFraction { bound, .. } => {
                (BoundForm::Claim(ClaimForm::MeanOfFraction), bound, None)
            }
            ProbForm::LongRunFraction { bound, .. } => {
                (BoundForm::Claim(ClaimForm::LongRunFraction), bound, None)
            }
            ProbForm::Quantile { q, threshold, .. } => (
                BoundForm::Claim(ClaimForm::Quantile),
                threshold,
                Some(q.clone()),
            ),
            ProbForm::ExpectedReward { bound, .. } => (BoundForm::ExpectedTotal, bound, None),
            ProbForm::MeanTime { .. } => return None,
        };
        let (comparison, threshold) = match bound {
            Bound::AtLeast(threshold) => (BoundComparison::GreaterOrEqual, threshold),
            Bound::AtMost(threshold) => (BoundComparison::LessOrEqual, threshold),
        };
        Some(Self {
            form,
            comparison,
            threshold: threshold.clone(),
            level,
        })
    }

    /// The bound `value` holds.
    ///
    /// # Errors
    /// [`WireDefect`] naming the member at fault.
    pub fn read(value: &Value) -> Result<Self, WireDefect> {
        let map = closed_object(value, &["type", "comparison", "threshold"], &["level"])?;
        let spelling = |name: &str| {
            member(map, name)?
                .as_str()
                .ok_or_else(|| WireDefect::new(format!("/{name}"), WireDefectKind::WrongKind))
        };
        let kind = spelling("type")?;
        let form = BoundForm::ALL
            .into_iter()
            .find(|form| form.as_str() == kind)
            .ok_or_else(|| WireDefect::new("/type", WireDefectKind::UnknownVariant))?;
        let written = spelling("comparison")?;
        let comparison = COMPARISONS
            .into_iter()
            .find(|comparison| comparison.as_str() == written)
            .ok_or_else(|| WireDefect::new("/comparison", WireDefectKind::UnknownVariant))?;
        let threshold = exact(member(map, "threshold")?, "/threshold")?;
        let level = map
            .get("level")
            .map(|level| exact(level, "/level"))
            .transpose()?;
        match (form, &level) {
            (BoundForm::Claim(ClaimForm::Quantile), None) => {
                Err(WireDefect::new("/level", WireDefectKind::Missing))
            }
            (BoundForm::Claim(ClaimForm::Quantile), Some(_)) | (_, None) => Ok(Self {
                form,
                comparison,
                threshold,
                level,
            }),
            (_, Some(_)) => Err(WireDefect::new("/level", WireDefectKind::UnknownMember)),
        }
    }
}

fn exact(value: &Value, pointer: &str) -> Result<Rational, WireDefect> {
    rational(value, pointer).map_err(|_| WireDefect::new(pointer, WireDefectKind::Invalid))
}

impl From<&VerdictBound> for ProbabilisticBound {
    fn from(bound: &VerdictBound) -> Self {
        Self {
            form: BoundForm::Claim(bound.form),
            comparison: bound.comparison,
            threshold: bound.threshold.clone(),
            level: bound.level.clone(),
        }
    }
}

impl Serialize for ProbabilisticBound {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let members = if self.level.is_some() { 4 } else { 3 };
        let mut object = serializer.serialize_struct("ProbabilisticBound", members)?;
        object.serialize_field("type", self.form.as_str())?;
        object.serialize_field("comparison", self.comparison.as_str())?;
        object.serialize_field("threshold", &rational_wire(&self.threshold))?;
        if let Some(level) = &self.level {
            object.serialize_field("level", &rational_wire(level))?;
        }
        object.end()
    }
}

impl<'de> Deserialize<'de> for ProbabilisticBound {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::read(&unique_object(decoder)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::ProbabilisticBound;
    use crate::shared_wire::{WireDefect, WireDefectKind};

    // FSpec FR-413's claim-bound table, by hand.
    #[trace("FSpec-FR-413", "FR-192-AC-9")]
    #[test]
    fn each_claim_bound_form_reads_back_as_it_was_written() {
        let half = json!({"numerator": "1", "denominator": "2"});
        for value in [
            json!({"type": "probability", "comparison": "less-or-equal", "threshold": half}),
            json!({"type": "mean-of-fraction", "comparison": "greater-or-equal",
                "threshold": half}),
            json!({"type": "long-run-fraction", "comparison": "greater-or-equal",
                "threshold": half}),
            json!({"type": "expected-total", "comparison": "less-or-equal", "threshold": half}),
            json!({"type": "quantile", "comparison": "greater-or-equal",
                "threshold": {"numerator": "3", "denominator": "1"}, "level": half}),
        ] {
            let read = ProbabilisticBound::read(&value).expect("reads");
            assert_eq!(serde_json::to_value(&read).expect("writes"), value);
        }
    }

    #[trace("FSpec-FR-413", "FR-192-AC-9")]
    #[test]
    fn a_claim_bound_refuses_every_other_shape() {
        let half = json!({"numerator": "1", "denominator": "2"});
        for (value, pointer, kind) in [
            (
                json!({"type": "quantile", "comparison": "less-or-equal", "threshold": half}),
                "/level",
                WireDefectKind::Missing,
            ),
            (
                json!({"type": "probability", "comparison": "less-or-equal",
                    "threshold": half, "level": half}),
                "/level",
                WireDefectKind::UnknownMember,
            ),
            (
                json!({"type": "probability", "comparison": "less", "threshold": half}),
                "/comparison",
                WireDefectKind::UnknownVariant,
            ),
            (
                json!({"type": "probability", "comparison": "less-or-equal",
                    "threshold": {"numerator": "2", "denominator": "4"}}),
                "/threshold",
                WireDefectKind::Invalid,
            ),
        ] {
            assert_eq!(
                ProbabilisticBound::read(&value),
                Err(WireDefect {
                    pointer: pointer.to_owned(),
                    kind,
                }),
                "{value}"
            );
        }
    }
}
