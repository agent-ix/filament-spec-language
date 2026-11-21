// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1000 hyper-evaluator coverage for subject-bound reference literals.

use filament_exact::Meter;
use fsl_eval::hyper::{HyperLetters, ParameterBinding};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::CheckedHyperLetters;
use fsl_eval::temporal::AtomReading;
use fsl_semantics::check::{
    check_hyper_clauses, CheckedHyper, FormulaNode, FormulaNodeId, PackageDeclarations,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input};
use ix_trace_rs::trace;

use super::*;

fn hyper_fixture(
    body: &str,
    operations: Vec<serde_json::Value>,
) -> (World, fsl_semantics::check::CheckedHyperClause) {
    let clause = format!(
        "hyper H using inf over behaviours (c: M::Counter) \
         forall trace a of M forall trace b of M {{ {body} }}"
    );
    let document = counters_document(operations.clone(), false);
    let packages = package_input([document.as_slice()]).expect("the Counter package admits");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         model M = \"{PACKAGE}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{clause}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "fr-120"),
        "fr-120.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 parses the hyper clause");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the hyper clause");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    let mut checked = check_hyper_clauses(&declarations);
    let clause = match checked.pop() {
        Some(Ok(CheckedHyper::Behaviours(clause))) if checked.is_empty() => *clause,
        other => panic!("one checked hyper clause: {other:?}"),
    };
    (super::world_with_level_bound(operations, "", 5), clause)
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
    // Both component predicates are false (4 and 5 exceed the threshold),
    // so True requires the fixed literal branch.
    let body = "always holds(c.value @ a <= 3 or c.value @ b <= 3 or c = M::Counter#\"a\")";
    let (mut world, clause) = hyper_fixture(body, Vec::new());
    let start_a = world.start("reference-literal-fixed-a", &[Obj("a", 4, 0, None)]);
    let start_b = world.start("reference-literal-fixed-b", &[Obj("a", 5, 0, None)]);
    let system_a = system(&world, vec![start_a], &["a", "b"], UNLIMITED);
    let system_b = system(&world, vec![start_b], &["a", "b"], UNLIMITED);
    let letters = CheckedHyperLetters::new(&world.package, &clause, vec![&system_a, &system_b])
        .expect("fixed references are admitted in hyper atoms");
    let initial_a = system_a.initial();
    let initial_b = system_b.initial();
    assert_eq!(super::int_field(initial_a[0].state(), "a", "value"), "4");
    assert_eq!(super::int_field(initial_b[0].state(), "a", "value"), "5");
    let states = [&initial_a[0], &initial_b[0]];
    let bindings = [ParameterBinding {
        parameter: "c".to_owned(),
        instance: ModelInstance {
            population: counters(),
            key: "a".to_owned(),
        },
    }];
    let mut meter = Meter::new(UNLIMITED);
    let reading = letters
        .atom(first_holds_atom(&clause), &bindings, &states, &mut meter)
        .expect("the fixed reference atom evaluates");
    assert_eq!(reading, AtomReading::True);
}

/// FR-1000-AC-9 (TC-2957): a CreatedObject ReferenceLiteral reads its key in
/// the innermost indexed component, including when the atom reads another
/// component too.
#[trace("TC-2957", "FR-1000-AC-9")]
#[test]
fn a_created_reference_literal_uses_its_indexed_hyper_component() {
    let spawn = super::operation("spawn", &[], None, super::frame(&[], true, false));
    for body in [
        "always holds((deref(M::Counter#\"b\").value = 0) @ a)",
        "always holds((deref(M::Counter#\"b\").value) @ a = c.value @ b)",
    ] {
        let (mut world, clause) = hyper_fixture(body, vec![spawn.clone()]);
        let start = world.start("reference-literal-created", &[Obj("a", 0, 0, None)]);
        let system = system(&world, vec![start], &["a", "b"], UNLIMITED);
        let letters = CheckedHyperLetters::new(&world.package, &clause, vec![&system, &system])
            .expect("a created reference with an index is admitted");
        let initial = system.initial();
        let spawned = system
            .successors(&initial[0])
            .expect("the Counter state expands")
            .successors
            .into_iter()
            .map(|(_, state)| state)
            .find(|state| state.state().populations[&counters()].contains_key("b"))
            .expect("spawn creates b in a distinct component state");
        let states = [&initial[0], &spawned];
        let bindings = [ParameterBinding {
            parameter: "c".to_owned(),
            instance: ModelInstance {
                population: counters(),
                key: "a".to_owned(),
            },
        }];
        let mut meter = Meter::new(UNLIMITED);
        let reading = letters
            .atom(first_holds_atom(&clause), &bindings, &states, &mut meter)
            .expect("an absent created-object key is an undefined reading");
        assert!(
            matches!(reading, AtomReading::Undefined(_))
                && format!("{reading:?}").contains("AbsentKey"),
            "the literal resolves in component a where b is absent: {reading:?}"
        );
    }
}
