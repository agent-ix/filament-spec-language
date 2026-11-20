// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact comparisons preserve decimal thresholds beyond binary precision.

use super::*;

#[trace("FR-195-AC-1", "FR-195-AC-3")]
#[test]
fn thresholds_one_decimal_unit_around_the_exact_probability_remain_distinct() {
    let expected =
        Rational::new(Integer::from(24_i64), Integer::from(25_i64)).expect("nonzero denominator");
    for (threshold, holds) in [
        ("0.95999999999999999999", true),
        ("0.96", true),
        ("0.96000000000000000001", false),
    ] {
        let claims = CLAIMS.replacen("0.95", threshold, 1);
        let world = World::new(&prob_world::link(&claims));
        let records = served(
            &world,
            &[item(&world, 17, ExactProbLimits::default())],
            &Cancel::new(),
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].request_index(), RequestIndex::new(17));
        assert_eq!(records[0].backend().map(BackendId::as_str), Some("en-5"));
        if holds {
            let TerminalValue::Proved {
                basis: ProofBasis::ExactValue { value, .. },
                certification,
            } = records[0].value()
            else {
                panic!(
                    "threshold {threshold} must hold exactly: {:?}",
                    records[0].value()
                );
            };
            assert_eq!(value.value(), &expected);
            assert_eq!(*certification, Certification::Certified);
        } else {
            assert_eq!(
                records[0].value(),
                &TerminalValue::Refuted,
                "threshold {threshold}"
            );
        }
    }
}
