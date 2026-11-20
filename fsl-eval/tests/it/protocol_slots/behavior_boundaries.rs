// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol expression calls retain Boolean meaning, frozen inputs and admission order.

use super::*;
use filament_exact::{LimitKind, Outcome};

fn boolean_package() -> CheckedPackage {
    package_from_source(&unit().replace(
        "on each (opened: Boolean) when ((1 + 2 = 3) and opened)",
        "on each (a: Boolean, b: Boolean, c: Boolean) when \
         ((gate(a, b) and gate(b, c)) or (a and c))",
    ).replace(
        "protocol P using p",
        "function gate using v(left: Boolean, right: Boolean): Boolean pure { left implies right }\n\
         protocol P using p",
    ))
}

fn activation(package: &CheckedPackage) -> ProtocolSlot {
    ProtocolSlot {
        node: package
            .graph()
            .protocol_node_keys("P")
            .expect("P exists")
            .clause
            .clone()
            .expect("P has a clause"),
        member: "activation.guard".to_owned(),
        index: None,
    }
}

fn booleans(package: &CheckedPackage, values: [bool; 3]) -> ProtocolSupply<'static> {
    let keys = package.graph().protocol_node_keys("P").expect("P exists");
    ProtocolSupply {
        binders: ["a", "b", "c"]
            .into_iter()
            .zip(values)
            .map(|(name, value)| {
                let key = keys
                    .binders
                    .iter()
                    .find(|binder| binder.name == name)
                    .expect("written activation source")
                    .node
                    .clone();
                (
                    key,
                    protocol_value(SuppliedValue::Value(Value::Boolean(value))),
                )
            })
            .collect(),
        observation: None,
    }
}

fn completed(output: ProtocolSlotEvaluation) -> Value {
    let ProtocolSlotEvaluation::Evaluated(
        filament_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Completed(value)),
            ..
        },
        _,
    ) = output
    else {
        panic!("expected a completed protocol expression: {output:?}");
    };
    value
}

#[trace("FR-390")]
#[test]
fn activation_helper_calls_match_the_complete_boolean_truth_table() {
    let package = boolean_package();
    let slot = activation(&package);
    let mut true_results = 0;
    let mut false_results = 0;
    for bits in 0_u8..8 {
        let [a, b, c] = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        let expected = ((!a || b) && (!b || c)) || (a && c);
        let mut meter = Meter::new(UNLIMITED);
        let output = package
            .evaluate_protocol_slot(&slot, &booleans(&package, [a, b, c]), &mut meter)
            .expect("all three required Boolean sources are supplied");
        assert!(
            matches!(completed(output), Value::Boolean(actual) if actual == expected),
            "assignment {bits}"
        );
        if expected {
            true_results += 1;
        } else {
            false_results += 1;
        }
        assert!(meter.consumed(LimitKind::WorkUnits) > 0);
    }
    assert_eq!((true_results, false_results), (5, 3));
}

#[trace("FR-390-AC-6", "FR-390-AC-11")]
#[test]
fn unread_boolean_branch_still_admits_every_supplied_values_shape_without_charging() {
    let package = boolean_package();
    let slot = activation(&package);
    let mut meter = Meter::new(UNLIMITED);
    // Both implications hold through false antecedents, so c is never evaluated.
    // Admission still checks c before starting the body.
    let mut supply = booleans(&package, [false, false, false]);
    let (bad_binder, bad_value) = supply.binders.last_mut().expect("c is supplied");
    let bad_binder = bad_binder.clone();
    *bad_value = protocol_value(SuppliedValue::Value(Value::Integer(7_i64.into())));
    let before = meter.clone();
    let result = package.evaluate_protocol_slot(&slot, &supply, &mut meter);
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("wrong-kind supply must refuse before running: {result:?}");
    };
    assert!(
        matches!(failure.as_ref(), CallAdmissionFailure::Refused(record)
        if record.record.catalog == CatalogCode::new("invalid_runtime_input", "wrong-value-kind")
            && record.locus == CallAdmissionLocus::Binder(bad_binder))
    );
    assert_eq!(meter, before);
}

#[trace("FR-390-AC-2")]
#[test]
fn recovery_uses_each_supplied_frozen_capture_and_activation_flag() {
    let package = compensation_package();
    let keys = package.graph().protocol_node_keys("P").expect("P exists");
    let node = keys.compensations[&0].clone();
    let binder = |name: &str| {
        keys.binders
            .iter()
            .find(|binder| binder.name == name)
            .expect("the compensation binds its input")
            .node
            .clone()
    };
    let slot = ProtocolSlot {
        node,
        member: "recovery.predicate".to_owned(),
        index: None,
    };
    for saved in [0, 5, 9] {
        for recovered in [0, 4, 5, 9] {
            for activated in [false, true] {
                let supply = ProtocolSupply {
                    binders: vec![
                        (
                            binder("recovered"),
                            protocol_value(SuppliedValue::Value(money(&package, recovered))),
                        ),
                        (
                            binder("saved"),
                            protocol_value(SuppliedValue::Value(money(&package, saved))),
                        ),
                        (
                            binder("activated"),
                            protocol_value(SuppliedValue::Value(Value::Boolean(activated))),
                        ),
                    ],
                    observation: None,
                };
                let mut meter = Meter::new(UNLIMITED);
                let result = package
                    .evaluate_protocol_slot(&slot, &supply, &mut meter)
                    .expect("only frozen inputs needed by recovery are supplied");
                let expected = saved == recovered && !activated;
                assert!(
                    matches!(completed(result), Value::Boolean(actual) if actual == expected),
                    "saved {saved}, recovered {recovered}, activated {activated}"
                );
            }
        }
    }
}

#[trace("FR-390")]
#[test]
fn a_helper_call_resumes_only_with_a_sufficient_fresh_work_budget() {
    let package = boolean_package();
    let slot = activation(&package);
    let supply = booleans(&package, [true, true, false]);
    let mut measured = Meter::new(UNLIMITED);
    let output = package
        .evaluate_protocol_slot(&slot, &supply, &mut measured)
        .expect("the helper call admits");
    assert!(matches!(completed(output), Value::Boolean(false)));
    let work = measured.consumed(LimitKind::WorkUnits);
    assert!(work > 1);
    let mut stopped = Meter::new(ScalarLimits {
        work_units: work - 1,
        ..UNLIMITED
    });
    let output = package
        .evaluate_protocol_slot(&slot, &supply, &mut stopped)
        .expect("resource stop is an outcome");
    assert!(
        matches!(output, ProtocolSlotEvaluation::Evaluated(filament_eval::Evaluation {
        outcome: FamilyOutcome::Evaluated(Outcome::Incomplete(ref record)), ..
    }, _) if record.limit_kind == LimitKind::WorkUnits)
    );
    assert_eq!(stopped.consumed(LimitKind::WorkUnits), work - 1);
    let mut exact = Meter::new(ScalarLimits {
        work_units: work,
        ..UNLIMITED
    });
    let output = package
        .evaluate_protocol_slot(&slot, &supply, &mut exact)
        .expect("exact work budget admits");
    assert!(matches!(completed(output), Value::Boolean(false)));
    assert_eq!(exact.admitted_charges(), measured.admitted_charges());
}
