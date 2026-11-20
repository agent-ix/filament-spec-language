// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concrete refinement outcomes and population traversal charges.

use super::*;

#[trace("FR-406", "FR-139")]
#[test]
fn refinement_population_rows_use_current_values_and_charge_each_visit() {
    let world = World::new();
    for (values, expected) in [([0, 0], false), ([0, 1], true)] {
        let state = world.state(0, 2, values);
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = world.evaluate(&state, BUSY, &mut meter);
        let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(actual))) =
            evaluation.outcome
        else {
            panic!(
                "the population predicate must complete: {:?}",
                evaluation.outcome
            );
        };
        assert_eq!(actual, expected, "slot values {values:?}");
        assert!(!meter.charge_log_truncated());
        // Both cases must examine s0 and s1: only the final slot can
        // match. No optional early-exit policy is assumed.
        let expected_visits = 2;
        let visits = meter
            .admitted_charges()
            .iter()
            .filter(|charge| charge.point == ChargePoint::CollectionVisit)
            .count();
        assert_eq!(visits, expected_visits, "slot values {values:?}");
    }
}

#[trace("FR-406", "FR-138")]
#[test]
fn history_updates_select_their_own_pre_and_post_observations() {
    let source = ring_is_queue(&all_rows()).replace(
        "population Q::cells from R::slots;",
        "population Q::cells from R::slots;\n\
         history R::Slot.before : Int[0, 1] = 0 { on R::Slot::put : pre(self.value); }\n\
         history R::Slot.after : Int[0, 1] = 0 { on R::Slot::put : self.value; }",
    );
    let world = World::of(checked(&source));
    let histories = world.refinement().history();
    assert_eq!(histories.len(), 2);
    for (before, after) in [(0, 1), (1, 0)] {
        let pre = world.observe(&world.state(0, 2, [before, 0]));
        let post = world.observe(&world.state(0, 2, [after, 0]));
        for (history, expected) in histories.iter().zip([before, after]) {
            let [update] = history.updates.as_slice() else {
                panic!("one real update per history field");
            };
            let mut meter = Meter::new(UNLIMITED);
            let evaluation = fsl_eval::value::evaluate_history_update(
                &world.package,
                &update.expression,
                (&pre, &post),
                vec![
                    Value::Reference(world.reference("Slot", "s0")),
                    integer(after),
                ],
                &mut meter,
            )
            .expect("the real history-update body runs");
            let FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(actual))) =
                evaluation.outcome
            else {
                panic!(
                    "the selected field value must complete: {:?}",
                    evaluation.outcome
                );
            };
            assert_eq!(actual, Integer::from(expected), "history {}", history.field);
            assert!(!meter.charge_log_truncated());
            assert!(meter
                .admitted_charges()
                .iter()
                .any(|c| c.point == ChargePoint::ModelNavigate));
        }
    }
}

#[trace("FR-406", "FR-138")]
#[test]
fn history_reads_use_the_observed_history_value_and_refuse_a_missing_field() {
    let source = ring_queue::ring_is_queue_over(&all_rows(), "value = self.last;").replace(
        "population Q::cells from R::slots;",
        "population Q::cells from R::slots;\n\
         history R::Slot.last : Int[0, 1] = 0 { on R::Slot::put : x; }",
    );
    let world = World::of(checked(&source));
    let [_, cell] = world.refinement().objects() else {
        panic!("the real Queue and Cell mappings");
    };
    let [field] = cell.fields.as_slice() else {
        panic!("one mapped Cell field");
    };
    let state = world.state(0, 2, [0, 1]);
    let receiver = world.reference("Slot", "s0");
    for value in [0, 1] {
        let environment = world.observe(&state).with_history(BTreeMap::from([(
            (receiver.clone(), "last".to_owned()),
            integer(value),
        )]));
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = evaluate_refinement_row(
            &world.package,
            &field.expression,
            &environment,
            vec![Value::Reference(receiver.clone())],
            &mut meter,
        )
        .expect("the real history read runs");
        let FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(actual))) =
            evaluation.outcome
        else {
            panic!(
                "the observed history value must complete: {:?}",
                evaluation.outcome
            );
        };
        assert_eq!(actual, Integer::from(value));
        assert!(
            meter.admitted_charges().is_empty(),
            "reading the stored scalar adds no kernel operation"
        );
    }
    let mut meter = Meter::new(UNLIMITED);
    let missing = evaluate_refinement_row(
        &world.package,
        &field.expression,
        &world.observe(&state),
        vec![Value::Reference(receiver)],
        &mut meter,
    )
    .expect("a missing history has a family result");
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = missing.outcome else {
        panic!(
            "a missing history cannot invent an initial value: {:?}",
            missing.outcome
        );
    };
    let code = cause.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("missing_declaration", "missing-name")
    );
    assert_eq!(
        cause.catalog_fields().expect("the typed history field")["history"],
        "last"
    );
    assert!(
        meter.admitted_charges().is_empty(),
        "the unresolved scalar read performs no kernel operation"
    );
}

#[trace("FR-406", "FR-139")]
#[test]
fn population_objects_are_reference_ordered_with_stable_charge_records() {
    let world = World::new();
    let state = world.state_keyed(0, 2, [0, 1], ["z", "a"]);
    let expected = ["a", "z"].map(|key| render(&Value::Reference(world.reference("Slot", key))));
    let mut reference_charges = None;
    // FR-406 AC6 explicitly requires ten equal result/charge runs. This is
    // one runtime stability oracle, not an independent evaluator claim.
    for run in 0..10 {
        let mut meter = Meter::new(UNLIMITED);
        let evaluation = world.evaluate(&state, CELLS, &mut meter);
        let FamilyOutcome::Evaluated(Outcome::Completed(value)) = evaluation.outcome else {
            panic!(
                "the actual PopulationObjects read completes: {:?}",
                evaluation.outcome
            );
        };
        assert_eq!(
            elements(&value),
            expected,
            "run {run}: ascending actual reference identities"
        );
        assert!(!meter.charge_log_truncated());
        assert!(
            !meter.admitted_charges().is_empty(),
            "materializing the two-member collection must be charged"
        );
        if let Some(reference) = &reference_charges {
            assert_eq!(
                meter.admitted_charges(),
                reference,
                "run {run}: complete admitted charge records"
            );
        } else {
            reference_charges = Some(meter.admitted_charges().to_vec());
        }
    }
}
