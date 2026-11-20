// SPDX-License-Identifier: AGPL-3.0-or-later
//! Temporal holds executes the checked body with its actual population.

use super::*;
use filament_exact::ChargePoint;

#[trace("FR-406")]
#[test]
fn holds_atoms_read_actual_population_members_and_meter_each_visit() {
    let mut unit = Unit::new(
        false,
        "\
        bound B on M::counters [3]\n\
        invariant Seen using v on M::Counter at current { true }\n\
        temporal C using t over (w: M::Watch) clock \"model-steps\" on origin {\
            eventually holds(forall(x in M::counters : x.v < 2)) }\n",
    );
    for (value, expected) in [(0, true), (2, false)] {
        let snapshot = unit.snapshot(
            &format!("value-{value}"),
            "current",
            &[("a", 0), ("b", value)],
            true,
        );
        let observations = unit
            .current("Seen", &snapshot, "a")
            .expect("the actual snapshot admits");
        let current = &observations
            .current
            .as_ref()
            .expect("current observation")
            .environment;
        let mut meter = Meter::new(UNLIMITED);
        let result = holds_atom_metered(&unit, current, &mut meter);
        let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(actual))) = result else {
            panic!("the admitted holds atom completes: {result:?}");
        };
        assert_eq!(actual, expected, "the final population member is {value}");
        assert!(!meter.charge_log_truncated());
        assert_eq!(
            meter
                .admitted_charges()
                .iter()
                .filter(|c| c.point == ChargePoint::CollectionVisit)
                .count(),
            2,
            "both the true and false cases must visit the final member"
        );
    }
}

#[trace("FR-268", "FR-406")]
#[test]
fn separation_replays_the_actual_deciding_element_and_rejects_a_changed_index() {
    use fsl_eval::value::{Separation, SeparationStep, WitnessClaim};

    let mut unit = Unit::new(
        false,
        "\
        bound B on M::counters [3]\n\
        invariant Below using v on M::Counter at current { forall(x in M::counters : x.v < 2) }\n",
    );
    let snapshot = unit.snapshot("two-counters", "current", &[("a", 2), ("b", 0)], true);
    let observations = unit
        .current("Below", &snapshot, "a")
        .expect("the snapshot admits");
    let clause = QualifiedName::unqualified("Below").expect("a declared name");
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = unit
        .package
        .evaluate_clause(&clause, &observations, &mut meter)
        .expect("the actual quantified clause runs");
    assert!(matches!(
        evaluation.evaluation.outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    let declaration = unit
        .graph()
        .state_clause("Below")
        .expect("the clause is checked");
    let stop = evaluation
        .stop(declaration.body().location())
        .expect("the first of two elements decides the forall");
    assert_eq!(stop.index, 0);
    let actual_claim = WitnessClaim {
        quantifier: &stop.quantifier,
        element: &stop.element,
        index: stop.index,
        value_path: &stop.value_path,
    };
    let mut replay_meter = Meter::new(UNLIMITED);
    assert!(matches!(
        unit.package
            .check_separation(&clause, &observations, actual_claim, &mut replay_meter)
            .expect("the actual witness is replayed"),
        Separation::Holds
    ));
    assert!(!replay_meter.charge_log_truncated());
    assert!(
        replay_meter
            .admitted_charges()
            .iter()
            .any(|c| c.point == ChargePoint::ModelNavigate),
        "separation must really reevaluate the deciding field read"
    );
    let changed_index = WitnessClaim {
        index: 1,
        ..actual_claim
    };
    assert!(matches!(
        unit.package
            .check_separation(
                &clause,
                &observations,
                changed_index,
                &mut Meter::new(UNLIMITED)
            )
            .expect("the changed claim gets a typed separation verdict"),
        Separation::Unmet(SeparationStep::Element)
    ));
}

#[trace("FR-391", "FR-406")]
#[test]
fn holds_distinguishes_complete_empty_from_missing_or_incomplete_populations() {
    let mut unit = Unit::new(
        false,
        "\
        bound B on M::counters [3]\n\
        invariant Seen using v on M::Widget at current { true }\n\
        temporal C using t over (w: M::Watch) clock \"model-steps\" on origin {\
            eventually holds(forall(x in M::counters : x.v < 2)) }\n",
    );
    for (index, completeness) in [Some(true), Some(false), None].into_iter().enumerate() {
        let snapshot = unit.listed(
            &format!("empty-{index}"),
            "current",
            &Listing {
                counters: &[],
                complete: completeness,
                others_complete: Some(true),
                link: false,
            },
        );
        let observations = unit
            .admit(
                "Seen",
                ClauseSelectionInput::Current {
                    snapshot,
                    anchor: SelectedAnchor {
                        kind: fsl_semantics::model::observation::AnchorKind::Handler,
                        name: "validate".to_owned(),
                    },
                    self_object: Some(SelectedObject {
                        population: ident("widgets"),
                        key: "w1".to_owned(),
                    }),
                },
            )
            .expect("the actual Widget observation needs no counter population");
        let current = &observations.current.as_ref().expect("current").environment;
        let mut meter = Meter::new(UNLIMITED);
        let outcome = holds_atom_metered(&unit, current, &mut meter);
        if completeness == Some(true) {
            assert!(
                matches!(
                    outcome,
                    FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
                ),
                "forall of the admitted empty domain is true: {outcome:?}"
            );
        } else if completeness == Some(false) {
            assert_refused_incomplete(outcome);
        } else {
            // FR-391 Evaluation distinguishes an absent population row
            // from a row explicitly marked incomplete.
            let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = outcome else {
                panic!("a missing population cannot become an empty domain: {outcome:?}");
            };
            let code = cause.catalog_code();
            assert_eq!(
                (code.code(), code.cause()),
                ("incomplete_population", "missing-required-artifact")
            );
            assert_eq!(
                cause.catalog_fields().expect("typed population fields")["population"],
                ident("counters")
            );
        }
        assert!(!meter.charge_log_truncated());
        assert!(
            !meter
                .admitted_charges()
                .iter()
                .any(|c| c.point == ChargePoint::CollectionVisit),
            "neither an empty domain nor a refused population visits an element"
        );
    }
}
