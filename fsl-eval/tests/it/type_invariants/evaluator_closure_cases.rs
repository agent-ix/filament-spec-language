// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real invariant entry over every reachable checked Money value.

use super::*;

#[trace("FR-406", "FR-366-AC-4")]
#[test]
fn value_invariants_change_with_the_actual_money_value_and_charge_each_comparison() {
    for (amount, expected) in [(-1, false), (1, true)] {
        let mut documents = documents_with(&fixture_f(|_| {}), &order_unit("requote"));
        let object = order_json("o1", money_json(amount), &[1, 1], Some(1), vec![]);
        let observations = invoked(
            &mut documents,
            &[object],
            "i1",
            "requote",
            json!({"amount": money_json(1)}),
        );
        let mut meter = unlimited();
        let result = documents
            .package
            .evaluate_value_invariants(&observations, &mut meter)
            .expect("the actual invariant evaluator runs on admitted observations");
        assert_eq!(
            result.outcome,
            if expected {
                ValueInvariantOutcome::True
            } else {
                ValueInvariantOutcome::False
            }
        );
        let results = rows(&result);
        assert_eq!(
            results.len(),
            6,
            "discount, two history entries, quoted, total and parameter"
        );
        let quoted = object_path(&observations, "s1", "o1", vec![member("quoted")]);
        assert_eq!(
            results
                .iter()
                .filter(|(_, path, _)| *path == quoted)
                .map(|(_, _, holds)| *holds)
                .collect::<Vec<_>>(),
            [expected]
        );
        assert!(!meter.charge_log_truncated());
        for point in [
            ChargePoint::OrderingOperands,
            ChargePoint::OrderingArithmetic,
            ChargePoint::OrderingResultRetain,
        ] {
            assert_eq!(
                meter
                    .admitted_charges()
                    .iter()
                    .filter(|c| c.point == point)
                    .count(),
                6,
                "one comparison per actual invariant value, {point:?}"
            );
        }
    }
}
