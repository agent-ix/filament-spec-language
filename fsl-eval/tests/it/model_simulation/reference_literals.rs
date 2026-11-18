// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000 hyper-evaluator coverage for subject-bound reference literals.

use filament_exact::Meter;
use fsl_eval::hyper::{HyperLetters, ParameterBinding};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::CheckedHyperLetters;
use fsl_eval::temporal::AtomReading;
use fsl_semantics::check::{FormulaNode, FormulaNodeId};
use ix_trace_rs::trace;

use super::*;

fn hyper_world(body: &str) -> World {
    let clause = format!(
        "hyper H using inf over behaviours (c: M::Counter) \
         forall trace a of M forall trace b of M {{ {body} }}"
    );
    build_world_document_with_profiles(
        counters_document(Vec::new(), false),
        "profile inf = \"filament.temporal.infinite-trace/v1\";\n",
        &clause,
        false,
        false,
    )
    .expect("the reference-literal hyper clause checks")
}

fn first_holds_atom(clause: &fsl_semantics::check::CheckedHyperClause) -> FormulaNodeId {
    clause
        .body()
        .nodes()
        .iter()
        .enumerate()
        .find(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .and_then(|(index, _)| clause.body().node_id(index))
        .expect("one holds atom")
}

/// FR-1000-AC-8 (TC-2957): a fixed ReferenceLiteral is admitted and evaluates
/// as the same reference in each hyper component.
#[trace("TC-2957", "FR-1000-AC-8")]
#[test]
fn a_fixed_reference_literal_is_constant_in_hyper_evaluation() {
    let mut world = hyper_world("always holds(M::Counter#\"a\" = M::Counter#\"a\")");
    let start = world.start("reference-literal-fixed", &[Obj("a", 0, 0, None)]);
    let system = system(&world, vec![start], &["a", "b"], UNLIMITED);
    let clause = world
        .package
        .graph()
        .hyper_clauses()
        .first()
        .expect("one checked hyper clause");
    let letters = CheckedHyperLetters::new(&world.package, clause, vec![&system, &system])
        .expect("fixed references are admitted in hyper atoms");
    let initial = system.initial();
    let states = [&initial[0], &initial[0]];
    let mut meter = Meter::new(UNLIMITED);
    let reading = letters
        .atom(first_holds_atom(clause), &[], &states, &mut meter)
        .expect("the fixed reference atom evaluates");
    assert_eq!(reading, AtomReading::True);
}

/// FR-1000-AC-9 (TC-2957): a CreatedObject ReferenceLiteral reads its key in
/// the innermost indexed component, including when the atom reads another
/// component too.
#[trace("TC-2957", "FR-1000-AC-9")]
#[test]
fn a_created_reference_literal_uses_its_indexed_hyper_component() {
    for body in [
        "always holds((deref(M::Counter#\"b\").value = 0) @ a)",
        "always holds((deref(M::Counter#\"b\").value) @ a = c.value @ b)",
    ] {
        let mut world = hyper_world(body);
        let start = world.start("reference-literal-created", &[Obj("a", 0, 0, None)]);
        let system = system(&world, vec![start], &["a", "b"], UNLIMITED);
        let clause = world
            .package
            .graph()
            .hyper_clauses()
            .first()
            .expect("one checked hyper clause");
        let letters = CheckedHyperLetters::new(&world.package, clause, vec![&system, &system])
            .expect("a created reference with an index is admitted");
        let initial = system.initial();
        let states = [&initial[0], &initial[0]];
        let bindings = [ParameterBinding {
            parameter: "c".to_owned(),
            instance: ModelInstance {
                population: counters(),
                key: "a".to_owned(),
            },
        }];
        let mut meter = Meter::new(UNLIMITED);
        let reading = letters
            .atom(first_holds_atom(clause), &bindings, &states, &mut meter)
            .expect("an absent created-object key is an undefined reading");
        assert!(matches!(reading, AtomReading::Undefined(_)), "{reading:?}");
    }
}
