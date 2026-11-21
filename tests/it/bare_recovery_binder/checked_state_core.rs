// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compound checked expressions evaluated after the source-side handoff has gone away.

use super::*;

fn checked_recipe(condition: &str, declarations: &str) -> Recipe {
    let holder = tempfile::tempdir().unwrap();
    let mut request = request("event_position", Corpus::None);
    let text = std::fs::read_to_string(&request.emit.source).unwrap();
    let attempt = "as (attempted: Boolean) { attempted };";
    assert_eq!(text.matches(attempt).count(), 1);
    let text = text.replace(FULL_RECOVER, condition).replace(
        "model M = \"agent-ix/checked-handoff\";",
        &format!("model M = \"agent-ix/checked-handoff\";\n{declarations}"),
    );
    let unit = holder.path().join("unit.native");
    std::fs::write(&unit, text).unwrap();
    request.emit.source = unit;
    read_recipe(holder, &request)
}

#[test]
#[trace("FR-531", "FR-520-AC-1", "FR-495-AC-24")]
fn bytes_only_recovery_combines_modulo_multiplicity_sum_and_branch_selection() {
    let recipe = checked_recipe(
        "let n = size(recoveredFull.receipts) in if activatedFull then n mod 3 = 0 else sum<M::Total>(r in recoveredFull.receipts: r) = targetFull and n mod 3 = 1",
        "",
    );
    for (receipts, target, activated, expected) in [
        (&[3, 3, 3, 3][..], 12, false, true),
        (&[4, 4, 4][..], 12, false, false),
        (&[12][..], 12, false, true),
        (&[12][..], 11, false, false),
        (&[4, 4, 4][..], 0, true, true),
        (&[3, 3, 3, 3][..], 0, true, false),
    ] {
        assert_eq!(
            holds(recipe.recover("Full", receipts, target, activated)),
            expected,
            "receipts={receipts:?}, target={target}, activated={activated}"
        );
    }
}

#[test]
#[trace("FR-531", "FR-520-AC-1")]
fn bytes_only_recovery_calls_the_emitted_function_with_the_query_result() {
    let recipe = checked_recipe(
        "residue(size(recoveredFull.receipts)) = 2 and sum<M::Total>(r in recoveredFull.receipts: r) = targetFull and not activatedFull",
        "function residue using v(n: Integer): Integer pure { (n + 4) mod 5 }",
    );
    for (receipts, target, activated, expected) in [
        (&[2, 2, 5][..], 9, false, true),
        (&[4, 5][..], 9, false, false),
        (&[2, 2, 5][..], 8, false, false),
        (&[2, 2, 5][..], 9, true, false),
    ] {
        assert_eq!(
            holds(recipe.recover("Full", receipts, target, activated)),
            expected,
            "receipts={receipts:?}, target={target}, activated={activated}"
        );
    }
}

#[test]
#[trace("FR-531", "FR-520-AC-1")]
fn bytes_only_recovery_reports_the_unavailable_target_only_on_the_branch_that_reads_it() {
    use filament_semantic_value::catalog::CatalogCode;
    use fsl_eval::value::ProtocolSlotEvaluation;

    let recipe = checked_recipe(
        "if activatedFull then size(recoveredFull.receipts) mod 3 < 1 else sum<M::Total>(r in recoveredFull.receipts: r) < targetFull",
        "",
    );
    let bare = recipe.bare();
    let node = recipe.compensation("Full");
    let slot = ProtocolSlot {
        node: node.clone(),
        member: "recovery.predicate".into(),
        index: None,
    };
    let selected = bare
        .select_protocol_slot(&ProtocolSlotDescriptor::new(
            node,
            "recovery.predicate",
            None,
        ))
        .unwrap();
    let binder = |name| {
        selected
            .sources()
            .iter()
            .find(|source| source.name() == name)
            .unwrap()
            .binder()
            .clone()
    };
    let target = binder("targetFull");
    let cause = CatalogCode::new("unavailable_observation", "missing-required-artifact");
    for activated in [true, false] {
        let given = ProtocolSupply {
            binders: vec![
                (
                    binder("recoveredFull"),
                    SuppliedValue::Value(recipe.recovery(&bare, &[2, 2, 5])).into(),
                ),
                (
                    target.clone(),
                    SuppliedValue::Unavailable(cause.clone()).into(),
                ),
                (
                    binder("activatedFull"),
                    SuppliedValue::Value(Value::Boolean(activated)).into(),
                ),
            ],
            observation: None,
        };
        let result = bare
            .evaluate_protocol_slot(&slot, &given, &mut Meter::new(UNLIMITED))
            .expect("unavailability is an evaluation outcome");
        if activated {
            let ProtocolSlotEvaluation::Evaluated(evaluation, _) = result else {
                panic!("the chosen branch never reads the unavailable target: {result:?}");
            };
            assert_eq!(completed_boolean(&evaluation.outcome), Some(true));
        } else {
            let ProtocolSlotEvaluation::Unavailable {
                binder,
                cause: actual,
                location,
                ..
            } = result
            else {
                panic!("the sum equality reads the unavailable target: {result:?}");
            };
            assert_eq!(binder, target);
            assert_eq!(actual, cause);
            assert_eq!(
                location, None,
                "the bytes carry no source expression location"
            );
        }
    }
}
