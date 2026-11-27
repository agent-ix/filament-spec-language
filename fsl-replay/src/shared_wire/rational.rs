// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checked-package v2 `Rational` object `{numerator, denominator}`, which
//! every certificate form writes its exact numbers in (FSpec FR-331, FR-418,
//! FR-652).

use filament_exact::{Integer, Rational};
use serde_json::{json, Value};

use super::{closed_object, member, WireDefect, WireDefectKind};

/// The `Rational` object: two canonical integer strings, the denominator
/// positive.
#[derive(Debug)]
pub struct RationalObject;

impl RationalObject {
    /// The object's members.
    pub const MEMBERS: [&'static str; 2] = ["denominator", "numerator"];

    /// The written object.
    pub fn value(rational: &Rational) -> Value {
        json!({
            "denominator": rational.denominator().to_string(),
            "numerator": rational.numerator().to_string(),
        })
    }

    /// The rational `value` spells, which must be in lowest terms.
    ///
    /// # Errors
    /// [`WireDefect`] as [`Self::read_unreduced`] gives it, and `Invalid` at
    /// the object for a rational not in lowest terms.
    pub fn read(value: &Value) -> Result<Rational, WireDefect> {
        let (numerator, denominator, reduced) = parts(value)?;
        if reduced.numerator() == &numerator && reduced.denominator() == &denominator {
            Ok(reduced)
        } else {
            Err(WireDefect::new("", WireDefectKind::Invalid))
        }
    }

    /// The rational `value` spells, normalised: FSpec FR-652's certificate
    /// numbers need not be written reduced.
    ///
    /// # Errors
    /// [`WireDefect`] for anything but the two members, each a canonical
    /// integer string, with a positive denominator.
    pub fn read_unreduced(value: &Value) -> Result<Rational, WireDefect> {
        parts(value).map(|(_, _, reduced)| reduced)
    }
}

/// The written numerator and denominator, and their reduced rational.
fn parts(value: &Value) -> Result<(Integer, Integer, Rational), WireDefect> {
    let map = closed_object(value, &RationalObject::MEMBERS, &[])?;
    let integer = |name: &str| {
        member(map, name)?
            .as_str()
            .ok_or_else(|| WireDefect::new(format!("/{name}"), WireDefectKind::WrongKind))?
            .parse::<Integer>()
            .map_err(|_| WireDefect::new(format!("/{name}"), WireDefectKind::Invalid))
    };
    let numerator = integer("numerator")?;
    let denominator = integer("denominator")?;
    let invalid = || WireDefect::new("", WireDefectKind::Invalid);
    if denominator.is_zero() || denominator.is_negative() {
        return Err(invalid());
    }
    let reduced = Rational::new(numerator.clone(), denominator.clone()).map_err(|_| invalid())?;
    Ok((numerator, denominator, reduced))
}

#[cfg(test)]
mod tests {
    use filament_exact::{Integer, Rational};
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::RationalObject;
    use crate::shared_wire::{WireDefect, WireDefectKind};

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
    }

    // The checked-package v2 `Rational`: `numerator` an `IntegerString`,
    // `denominator` a `PositiveIntegerString`.
    // FR-290-AC-9 is withdrawn and no FSL criterion governs this object; the
    // owning requirement is FSpec FR-331 (and FR-418, FR-652), so only it is
    // tagged.
    #[trace("FSpec-FR-331")]
    #[test]
    fn a_rational_object_is_two_canonical_integer_strings() {
        let written = json!({"numerator": "-3", "denominator": "2"});
        assert_eq!(RationalObject::value(&rational(-3, 2)), written);
        assert_eq!(RationalObject::read(&written), Ok(rational(-3, 2)));
        let unreduced = json!({"numerator": "2", "denominator": "4"});
        assert_eq!(
            RationalObject::read_unreduced(&unreduced),
            Ok(rational(1, 2))
        );
        let defect = |pointer: &str, kind| {
            Err(WireDefect {
                pointer: pointer.to_owned(),
                kind,
            })
        };
        assert_eq!(
            RationalObject::read(&unreduced),
            defect("", WireDefectKind::Invalid)
        );
        for (value, expected) in [
            (json!("1/2"), defect("", WireDefectKind::WrongKind)),
            (
                json!({"numerator": "1"}),
                defect("/denominator", WireDefectKind::Missing),
            ),
            (
                json!({"numerator": "1", "denominator": "2", "unit": "s"}),
                defect("/unit", WireDefectKind::UnknownMember),
            ),
            (
                json!({"numerator": 1, "denominator": "2"}),
                defect("/numerator", WireDefectKind::WrongKind),
            ),
            (
                json!({"numerator": "01", "denominator": "2"}),
                defect("/numerator", WireDefectKind::Invalid),
            ),
            (
                json!({"numerator": "1", "denominator": "+2"}),
                defect("/denominator", WireDefectKind::Invalid),
            ),
            (
                json!({"numerator": "1", "denominator": "0"}),
                defect("", WireDefectKind::Invalid),
            ),
            (
                json!({"numerator": "1", "denominator": "-2"}),
                defect("", WireDefectKind::Invalid),
            ),
        ] {
            assert_eq!(RationalObject::read_unreduced(&value), expected, "{value}");
            assert_eq!(RationalObject::read(&value), expected, "{value}");
        }
    }
}
