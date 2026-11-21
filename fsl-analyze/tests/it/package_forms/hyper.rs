// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine joint-state evaluation of a package atom through the shared machine.

use super::*;
use fsl_eval::hyper::HyperSlot;
use fsl_semantics::check::CheckedHyperClause;

const ASYMMETRIC: &str = "hyper Equal using t0 over behaviours (c: M::Counter) \
    forall trace a of M forall trace b of M \
    { always holds(c.value @ a <= 3 and c.value @ b <= 2) }";

fn atom(clause: &CheckedHyperClause) -> FormulaNodeId {
    FormulaNodeId::from_arena_index(
        clause
            .body()
            .nodes()
            .iter()
            .position(|node| matches!(node, FormulaNode::Holds { .. }))
            .expect("one holds atom"),
    )
}

fn slots(clause: &CheckedHyperClause, components: &[&ObjectEnvironment]) -> Vec<HyperSlot> {
    clause
        .atom_slots()
        .iter()
        .map(|slot| HyperSlot::Bound(Value::Reference(counter(components[slot.trace]))))
        .collect()
}

/// A different value in each component exposes a swapped or default component
/// assignment. Compare every admitted charge, including its ordering.
/// Covers only AC-17's atom value/charge portion, not analysis or certificates.
#[trace("TC-2958", "FR-940-AC-17", "FR-487-AC-3")]
#[test]
fn package_hyper_atom_has_source_values_and_complete_charge_logs() {
    let (ws, handoff, mut budget) = handoff_of("package-hyper-atom", ASYMMETRIC);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff supply joins");
    let source_package = handoff.compiled.checked();
    let source = &source_package.graph().hyper_clauses()[0];
    let form = bare
        .hyper_clause_form(source.assembled_occurrence().expect("claim occurrence"))
        .expect("the package form");
    let rebuilt =
        CheckedHyperClause::of_package_form(form, bare.world().expect("world").types(), 1_000_000)
            .expect("the package clause");
    assert!(rebuilt.condition(atom(&rebuilt)).is_none());
    std::fs::write(ws.path("program.fsl"), b"not a native unit").expect("overwrite source");
    std::fs::remove_dir_all(ws.path(HANDOFF).join("sources")).expect("remove retained sources");
    for (left, right, expected) in [(0, 3, false), (3, 0, true), (0, 2, true), (3, 3, false)] {
        let environments = [objects(&handoff, left), objects(&handoff, right)];
        let components = [&environments[0], &environments[1]];
        let mut source_meter = Meter::new(UNLIMITED);
        let mut bare_meter = Meter::new(UNLIMITED);
        let source_result = source_package
            .evaluate_hyper_holds(
                source,
                atom(source),
                slots(source, &components),
                &components,
                &mut source_meter,
            )
            .expect("source atom");
        let bare_result = bare
            .evaluate_hyper_holds(
                &rebuilt,
                atom(&rebuilt),
                slots(&rebuilt, &components),
                &components,
                &mut bare_meter,
            )
            .expect("package atom");
        assert_eq!(
            boolean(&source_result),
            expected,
            "source ({left}, {right})"
        );
        assert_eq!(boolean(&bare_result), expected, "package ({left}, {right})");
        assert!(!source_meter.admitted_charges().is_empty());
        assert!(!source_meter.charge_log_truncated());
        assert!(!bare_meter.charge_log_truncated());
        assert_eq!(
            bare_meter.admitted_charges(),
            source_meter.admitted_charges(),
            "complete ordered charge log at ({left}, {right})"
        );
        for bound in [
            0,
            source_meter
                .consumed(LimitKind::WorkUnits)
                .saturating_sub(1),
        ] {
            let limits = ScalarLimits {
                work_units: bound,
                ..UNLIMITED
            };
            let mut source_meter = Meter::new(limits);
            let mut bare_meter = Meter::new(limits);
            let source_result = source_package
                .evaluate_hyper_holds(
                    source,
                    atom(source),
                    slots(source, &components),
                    &components,
                    &mut source_meter,
                )
                .expect("bounded source atom");
            let bare_result = bare
                .evaluate_hyper_holds(
                    &rebuilt,
                    atom(&rebuilt),
                    slots(&rebuilt, &components),
                    &components,
                    &mut bare_meter,
                )
                .expect("bounded package atom");
            assert!(matches!(
                source_result.outcome,
                filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Incomplete(_))
            ));
            assert_eq!(outcome(&bare_result), outcome(&source_result));
            assert_eq!(
                bare_meter.admitted_charges(),
                source_meter.admitted_charges()
            );
            assert!(!bare_meter.charge_log_truncated());
            assert!(!source_meter.charge_log_truncated());
        }
    }
}

/// An absent slot retains the model evaluator's absent-object outcome and
/// exact charge sequence independently in either component.
/// Covers only AC-17's atom outcome/charge portion, not request parity.
#[trace("TC-2958", "FR-940-AC-17", "FR-125")]
#[test]
fn package_hyper_atom_preserves_absent_object_outcomes_and_charges() {
    let (_ws, handoff, mut budget) = handoff_of("package-hyper-absent", ASYMMETRIC);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff supply joins");
    let source_package = handoff.compiled.checked();
    let source = &source_package.graph().hyper_clauses()[0];
    let form = bare
        .hyper_clause_form(source.assembled_occurrence().expect("claim occurrence"))
        .expect("package form");
    let rebuilt =
        CheckedHyperClause::of_package_form(form, bare.world().expect("world").types(), 1_000_000)
            .expect("the package clause");
    let present = objects(&handoff, 0);
    let absent = ObjectEnvironment::default();
    let reference = counter(&present);
    for missing in 0..2 {
        let components = if missing == 0 {
            [&absent, &present]
        } else {
            [&present, &absent]
        };
        let bindings = |clause: &CheckedHyperClause| {
            clause
                .atom_slots()
                .iter()
                .map(|slot| {
                    if slot.trace == missing {
                        HyperSlot::Absent {
                            key: reference.clone(),
                            population: crate::counter::key("counters").node,
                        }
                    } else {
                        HyperSlot::Bound(Value::Reference(reference.clone()))
                    }
                })
                .collect()
        };
        let mut source_meter = Meter::new(UNLIMITED);
        let mut bare_meter = Meter::new(UNLIMITED);
        let source_result = source_package
            .evaluate_hyper_holds(
                source,
                atom(source),
                bindings(source),
                &components,
                &mut source_meter,
            )
            .expect("source absent atom");
        let bare_result = bare
            .evaluate_hyper_holds(
                &rebuilt,
                atom(&rebuilt),
                bindings(&rebuilt),
                &components,
                &mut bare_meter,
            )
            .expect("package absent atom");
        assert!(matches!(
            source_result.outcome,
            filament_eval::FamilyOutcome::FamilyEvaluated(filament_eval::FamilyResult::Undefined(
                _
            ))
        ));
        assert_eq!(outcome(&bare_result), outcome(&source_result));
        assert_eq!(source_meter.admitted_charges().is_empty(), missing == 0);
        assert_eq!(
            bare_meter.admitted_charges(),
            source_meter.admitted_charges()
        );
        assert!(!bare_meter.charge_log_truncated());
        assert!(!source_meter.charge_log_truncated());
    }
}

/// Invalid slot and component vector lengths fail before evaluating any node.
#[trace("TC-2958", "FR-487-AC-3")]
#[test]
fn package_hyper_atom_refuses_wrong_binding_shapes_without_charges() {
    let (_ws, handoff, mut budget) = handoff_of("package-hyper-shape", ASYMMETRIC);
    let bare = BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: handoff.equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff supply joins");
    let source = &handoff.compiled.checked().graph().hyper_clauses()[0];
    let form = bare
        .hyper_clause_form(source.assembled_occurrence().expect("claim occurrence"))
        .expect("package form");
    let rebuilt =
        CheckedHyperClause::of_package_form(form, bare.world().expect("world").types(), 1_000_000)
            .expect("the package clause");
    let environment = objects(&handoff, 0);
    for (bindings, components) in [
        (vec![], vec![&environment, &environment]),
        (
            slots(&rebuilt, &[&environment, &environment]),
            vec![&environment],
        ),
    ] {
        let mut meter = Meter::new(UNLIMITED);
        let result =
            bare.evaluate_hyper_holds(&rebuilt, atom(&rebuilt), bindings, &components, &mut meter);
        assert!(
            matches!(result, Err(filament_eval::CallFailure::Fault(_))),
            "{result:?}"
        );
        assert!(meter.admitted_charges().is_empty());
    }
    let claim = rebuilt.assembled_occurrence().expect("claim occurrence");
    assert!(matches!(bare.project_hyper_holds(claim, usize::MAX),
        Err(fsl_package::BareClauseRefusal::Selection(fsl_package::BareSelectionFailure::Slot(
            fsl_package::SlotSelectionFailure::MalformedNode(node)))) if node == *claim.node()));
}

/// The emitted Linked alias reaches its present reference in the selected
/// component. Nested aliases and shadowed initializer scopes retain ownership.
/// This covers the atom portion of AC-17, not its analysis/certificate path.
#[trace("FR-940-AC-17", "FR-488-AC-16", "FR-487-AC-9")]
#[test]
fn package_hyper_local_aliases_preserve_component_values_and_complete_charges() {
    let document = include_bytes!("../../../../examples/config-version/model.semantic-ir.json");
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the ConfigVersion model admits");
    for alias in [
        "let r = v.parent in present(r) and deref(value(r)).versionNumber = 1",
        "let r = v.parent in let s = r in present(s) and deref(value(s)).versionNumber = 1",
        "let r = (let r = v.parent in r) in present(r) and deref(value(r)).versionNumber = 1",
    ] {
        for (trace, selected) in [("a", 0), ("b", 1)] {
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"filament.value.complete/v1\";\n\
                profile inf = \"filament.temporal.infinite-trace/v1\";\n\
                model M = \"example/config-version\";\n\
                hyper Linked using inf over behaviours (v: M::ConfigVersion) \
                forall trace a of M forall trace b of M {{ always holds(({alias}) @ {trace}) }}"
            );
            let package = crate::counter::package_from_text(document, &source);
            let emission = fsl_package::emit_checked(&package, fsl_package::EmitLimits::fixture())
                .expect("the Linked clause emits");
            assert!(emission.omitted.is_empty());
            let limits = fsl_package::V2ReadLimits::fixture();
            let admitted =
                fsl_package::admit_checked_package(emission.package().bytes(), &packages, limits)
                    .expect("the strict reader admits the Linked bytes");
            let mut budget = limits.native_join_budget();
            let identities = emission
                .identity_rows(&admitted, &mut budget)
                .expect("identity rows");
            let equality = emission
                .equality_witness(&admitted, &mut budget)
                .expect("equality rows");
            let bare = BarePackage::new(&admitted)
                .with_supply(
                    BareSupply {
                        identities: &identities,
                        equality: Some(&equality),
                    },
                    &mut budget,
                )
                .expect("the supply joins");
            let clause = &package.graph().hyper_clauses()[0];
            let form = bare
                .hyper_clause_form(clause.assembled_occurrence().expect("claim occurrence"))
                .expect("Linked package form");
            let rebuilt = CheckedHyperClause::of_package_form(
                form,
                bare.world().expect("world").types(),
                1_000_000,
            )
            .expect("the package clause");
            assert!(rebuilt.condition(atom(&rebuilt)).is_none());
            let observed = |parent: bool, version: i64| {
                let selected = DocumentRef {
                    authority: "test".to_owned(),
                    identity: "linked-state".to_owned(),
                };
                let population = "ix://example/config-version/config_history";
                let parent = if parent {
                    serde_json::json!({"present": {"reference": {"population": population, "key": "tail"}}})
                } else {
                    serde_json::json!({"absent": {}})
                };
                let snapshot = serde_json::json!({
                    "format": "filament.state.snapshot/v1", "identity": {"authority": "test", "identity": "linked-state"},
                    "observation": "current", "model": {"identity": "example/config-version"},
                    "anchor": {"kind": "initialization", "name": "start"},
                    "populations": [{"population": population, "complete": true, "objects": [
                        {"key": "head", "type": "ix://example/config-version/ConfigVersion",
                            "fields": {"versionNumber": {"integer": "3"}, "parent": parent}},
                        {"key": "tail", "type": "ix://example/config-version/ConfigVersion",
                            "fields": {"versionNumber": {"integer": version.to_string()}, "parent": {"absent": {}}}},
                    ]}],
                });
                let provisions = BTreeMap::from([(
                    selected.clone(),
                    serde_json::to_vec(&snapshot).expect("snapshot bytes"),
                )]);
                admit_trace_snapshot(
                    package.graph().model_selections(),
                    package.graph().scope().types(),
                    &packages,
                    ModelNormalizationLimits::default(),
                    &provisions,
                    &selected,
                    ObservationLimits::default(),
                    &[],
                )
                .expect("the closed Linked snapshot admits")
                .0
            };
            let one = observed(true, 1);
            let two = observed(true, 2);
            let absent = observed(false, 1);
            for (components, expected) in [
                ([&one, &two], selected == 0),
                ([&two, &one], selected == 1),
                ([&absent, &one], selected == 1),
                ([&one, &absent], selected == 0),
            ] {
                let bindings = |clause: &CheckedHyperClause| {
                    clause
                        .atom_slots()
                        .iter()
                        .map(|slot| {
                            let head = components[slot.trace]
                                .references()
                                .iter()
                                .find(|reference| reference.object().as_str() == "head")
                                .expect("head reference");
                            HyperSlot::Bound(Value::Reference(head.clone()))
                        })
                        .collect()
                };
                let mut source_meter = Meter::new(UNLIMITED);
                let mut bare_meter = Meter::new(UNLIMITED);
                let source_result = package
                    .evaluate_hyper_holds(
                        clause,
                        atom(clause),
                        bindings(clause),
                        &components,
                        &mut source_meter,
                    )
                    .expect("source Linked atom");
                let bare_result = bare
                    .evaluate_hyper_holds(
                        &rebuilt,
                        atom(&rebuilt),
                        bindings(&rebuilt),
                        &components,
                        &mut bare_meter,
                    )
                    .expect("package Linked atom");
                assert_eq!(
                    boolean(&source_result),
                    expected,
                    "source @{trace}: {alias}"
                );
                assert_eq!(boolean(&bare_result), expected, "package @{trace}: {alias}");
                assert!(!source_meter.admitted_charges().is_empty());
                assert!(!source_meter.charge_log_truncated());
                assert!(!bare_meter.charge_log_truncated());
                assert_eq!(
                    source_meter.admitted_charges(),
                    bare_meter.admitted_charges(),
                    "@{trace}: {alias}"
                );
            }
        }
    }
}
