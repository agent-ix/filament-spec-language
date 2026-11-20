// SPDX-License-Identifier: AGPL-3.0-or-later
//! Instance admission for actual engine-produced hyper counterexamples.

use super::*;

#[trace("FR-183", "TC-608")]
#[test]
fn genuine_refuting_traces_do_not_authorize_a_foreign_or_missing_instance() {
    let world = leaky();
    let original = refuted(&world);
    assert!(matches!(
        world.replay(&original).expect("genuine replay"),
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
    assert_eq!(original.instance.len(), 1, "one declared object parameter");
    let mut foreign_key = original.clone();
    foreign_key.instance[0].1 = "outside-vault-universe".to_owned();
    let mut foreign_parameter = original.clone();
    foreign_parameter.instance[0].0 = "undeclared".to_owned();
    let mut missing = original.clone();
    missing.instance.clear();
    let mut duplicate = original.clone();
    duplicate.instance.push(duplicate.instance[0].clone());
    for changed in [foreign_key, foreign_parameter, missing, duplicate] {
        assert_eq!(
            changed.traces, original.traces,
            "only instance admission is challenged"
        );
        assert!(
            matches!(
                refusal(world.replay(&changed)),
                ModelTraceRefusal::HyperShape { .. }
            ),
            "a trace cannot authenticate another binding"
        );
    }
}
