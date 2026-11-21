// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine joint-state evaluation of a package atom through the shared machine.

use super::*;
use fsl_eval::hyper::HyperSlot;
use fsl_semantics::check::CheckedHyperClause;

const ASYMMETRIC: &str = "hyper Equal using t0 over behaviours (c: M::Counter) \
    forall trace a of M forall trace b of M \
    { always holds(c.value @ a <= 3 and c.value @ b <= 2) }";

fn atom(clause: &CheckedHyperClause) -> FormulaNodeId {
    FormulaNodeId::from_arena_index(clause.body().nodes().iter()
        .position(|node| matches!(node, FormulaNode::Holds { .. })).expect("one holds atom"))
}

fn slots(clause: &CheckedHyperClause, components: &[&ObjectEnvironment]) -> Vec<HyperSlot> {
    clause.atom_slots().iter().map(|slot| {
        HyperSlot::Bound(Value::Reference(counter(components[slot.trace])))
    }).collect()
}

/// A different value in each component exposes a swapped or default component
/// assignment. Compare every admitted charge, including its ordering.
#[trace("TC-2958", "FR-940-AC-6", "FR-487-AC-3")]
#[test]
fn package_hyper_atom_has_source_values_and_complete_charge_logs() {
    let (_ws, handoff, mut budget) = handoff_of("package-hyper-atom", ASYMMETRIC);
    let bare = BarePackage::new(&handoff.admitted).with_supply(BareSupply {
        identities: &handoff.identities, equality: handoff.equality.as_ref(),
    }, &mut budget).expect("the handoff supply joins");
    let source_package = handoff.compiled.checked();
    let source = &source_package.graph().hyper_clauses()[0];
    let form = bare.hyper_clause_form(source.assembled_occurrence().expect("claim occurrence"))
        .expect("the package form");
    let rebuilt = CheckedHyperClause::of_package_form(form, bare.world().expect("world").types(), 1_000_000)
        .expect("the package clause");
    assert!(rebuilt.condition(atom(&rebuilt)).is_none());
    for (left, right, expected) in [(0, 3, false), (3, 0, true), (0, 2, true), (3, 3, false)] {
        let environments = [objects(&handoff, left), objects(&handoff, right)];
        let components = [&environments[0], &environments[1]];
        let mut source_meter = Meter::new(UNLIMITED);
        let mut bare_meter = Meter::new(UNLIMITED);
        source_meter.enable_charge_capture();
        bare_meter.enable_charge_capture();
        let source_result = source_package.evaluate_hyper_holds(source, atom(source),
            slots(source, &components), &components, &mut source_meter).expect("source atom");
        let bare_result = bare.evaluate_hyper_holds(&rebuilt, atom(&rebuilt),
            slots(&rebuilt, &components), &components, &mut bare_meter).expect("package atom");
        assert_eq!(boolean(&source_result), expected, "source ({left}, {right})");
        assert_eq!(boolean(&bare_result), expected, "package ({left}, {right})");
        assert!(!source_meter.admitted_charges().is_empty());
        assert_eq!(bare_meter.admitted_charges(), source_meter.admitted_charges(),
            "complete ordered charge log at ({left}, {right})");
    }
}

/// Invalid slot and component vector lengths fail before evaluating any node.
#[trace("TC-2958", "FR-487-AC-3")]
#[test]
fn package_hyper_atom_refuses_wrong_binding_shapes_without_charges() {
    let (_ws, handoff, mut budget) = handoff_of("package-hyper-shape", ASYMMETRIC);
    let bare = BarePackage::new(&handoff.admitted).with_supply(BareSupply {
        identities: &handoff.identities, equality: handoff.equality.as_ref(),
    }, &mut budget).expect("the handoff supply joins");
    let source = &handoff.compiled.checked().graph().hyper_clauses()[0];
    let form = bare.hyper_clause_form(source.assembled_occurrence().expect("claim occurrence"))
        .expect("package form");
    let rebuilt = CheckedHyperClause::of_package_form(form, bare.world().expect("world").types(), 1_000_000)
        .expect("the package clause");
    let environment = objects(&handoff, 0);
    for (bindings, components) in [
        (vec![], vec![&environment, &environment]),
        (slots(&rebuilt, &[&environment, &environment]), vec![&environment]),
    ] {
        let mut meter = Meter::new(UNLIMITED);
        meter.enable_charge_capture();
        let result = bare.evaluate_hyper_holds(&rebuilt, atom(&rebuilt), bindings, &components, &mut meter);
        assert!(matches!(result, Err(filament_eval::CallFailure::Fault(_))), "{result:?}");
        assert!(meter.admitted_charges().is_empty());
    }
    let claim = rebuilt.assembled_occurrence().expect("claim occurrence");
    assert!(matches!(bare.project_hyper_holds(claim, usize::MAX),
        Err(fsl_package::BareClauseRefusal::Selection(fsl_package::BareSelectionFailure::Slot(
            fsl_package::SlotSelectionFailure::MalformedNode(node)))) if node == *claim.node()));
}
