// SPDX-License-Identifier: AGPL-3.0-or-later
//! Named timed clauses preserve source results and evaluator accounting from bytes.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection_named, request_over, Workspace, HANDOFF,
};

const DENSE: &str = "dimension Time;\nunit ms : Time = rational(1, 1);\ntime M dense unit ms;\n";

fn unit(formula: &str) -> String {
    crate::timed_world::unit_text(
        crate::timed_world::PACKAGE,
        "profile tt = \"filament.temporal.timed/v1\";\n",
        &format!(
            "{DENSE}temporal Claim using tt over (c: M::Call) clock \"model-time\" on origin {{ {formula} }}\n"
        ),
    )
}

/// Both timed forms run over one required direct Clock and a quiescent model.
#[trace("FR-234-AC-1", "FR-632-AC-1", "FR-940-AC-6", "FR-850-AC-2")]
#[test]
fn dense_timed_clauses_preserve_whole_result_and_exit_without_source() {
    let mut mismatches = Vec::new();
    for (case, formula, expected) in [
        ("invariant-true", "always holds(c.phase = 0)", "proved"),
        ("invariant-false", "always holds(c.phase = 1)", "refuted"),
        (
            "bounded-true",
            "eventually[0 ms, 1 ms] holds(c.phase = 0)",
            "proved",
        ),
        (
            "bounded-false",
            "eventually[0 ms, 1 ms] holds(c.phase = 1)",
            "refuted",
        ),
    ] {
        eprintln!("dense timed clause parity: {case}");
        let ws = Workspace::new(&format!("dense-timed-clause-{case}"));
        let world =
            crate::timed_world::world(&crate::timed_world::document(&["x"], &[]), "", DENSE);
        let path = request_over(
            &ws,
            &unit(formula),
            &world.document,
            &world.snapshots.values().cloned().collect::<Vec<_>>(),
            json!({"items": [{"claim": "Claim", "kind": "temporal-satisfaction"}]}),
        );
        let source = analyze(&path);
        let item = source.item(0);
        assert_eq!(label(&item), expected, "{case}: {}", source.document());
        assert_eq!(item["record"]["backend"], "en-6", "{case}: {item}");
        if expected == "proved" {
            assert_eq!(item["record"]["basis"]["type"], "zone-certified", "{item}");
            assert_eq!(item["record"]["certification"], "certified", "{item}");
        } else {
            assert_eq!(item["replay"]["reproduced"], true, "{item}");
        }
        assert_eq!(source.exit, if expected == "proved" { 0 } else { 10 });
        let request: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
        let (members, _) = package_selection_named(&ws, &request["request"], "analyze-unit");
        std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
        std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
            .expect("delete retained source copies");
        assert!(!ws.path("program.fsl").exists());
        assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
        let package = analyze(
            &ws.write(
                "package-analyze.json",
                json!({"format": "native-analyze/1", "request": members})
                    .to_string()
                    .as_bytes(),
            ),
        );
        if package.exit != source.exit || package.stdout != source.stdout {
            mismatches.push(format!(
                "{case}: source exit {}; package exit {}; source {}; package {}",
                source.exit,
                package.exit,
                source.document(),
                package.document()
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "whole result/exit parity: {mismatches:#?}"
    );
}

/// Every admitted atom charge stays in its original order on the package route.
#[trace("FR-940-AC-6", "FR-850-AC-2")]
#[test]
fn dense_timed_clause_atoms_preserve_complete_ordered_charges() {
    use filament_exact::{Meter, ValueType};
    use fsl_eval::model_subject::ModelLetters;
    use fsl_eval::simulation::model::{ModelSystem, PackageModelInput};
    use fsl_eval::simulation::TransitionSystem;
    use fsl_package::{AnalysisView, BarePackage, BareSupply, EmitLimits, V2ReadLimits};
    use fsl_semantics::check::{FormulaNode, FormulaNodeId};
    use fsl_semantics::model::accounting::ModelNormalizationLimits;

    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x"], &[]),
        "profile tt = \"filament.temporal.timed/v1\";\n",
        &format!(
            "{DENSE}temporal Claim using tt over (c: M::Call) clock \"model-time\" on origin \
             {{ always holds(c.phase = 0) }}\n"
        ),
    );
    let request = world.request();
    let emission = fsl_package::emit_checked(&world.package, EmitLimits::fixture()).expect("emit");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        request.packages,
        V2ReadLimits::fixture(),
    )
    .expect("admit real producer bytes");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("identities");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("equality");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("authentic supply");
    let input = PackageModelInput::of(&bare, request.packages, ModelNormalizationLimits::default())
        .expect("direct dense input");
    let node = &world.package.graph().temporal_clauses()[0];
    let selected = bare.claim_name("Claim").expect("named clause");
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        bare.node_key(&selected.declaration)
            .expect("structural clause key"),
        filament_exact::Origin::new(
            filament_exact::Role::new("claim"),
            selected.occurrence.ordinal,
        ),
    );
    let clause = input
        .temporal_run_clause(&occurrence, 1000)
        .expect("package timed clause");
    assert_eq!(
        clause.over(),
        node.clause().over(),
        "the genuine clause parameter identity"
    );
    let source = world.system();
    let mut package_request = request.clone();
    package_request.input = input.input();
    let package = ModelSystem::new(package_request).expect("package system");
    let source_letters = source.clause_letters(node).expect("source letters");
    let package_letters = package
        .package_clause_letters(&clause, occurrence.node())
        .expect("package letters");
    let ValueType::Reference(object) = &node.clause().over().1 else {
        panic!("reference parameter");
    };
    let instances = source
        .instances(&ValueType::Reference(object.clone()))
        .expect("instances");
    let package_instances = package
        .instances(&clause.over().1)
        .expect("package instances");
    assert_eq!(
        package_instances, instances,
        "the genuine model instance identities"
    );
    assert_eq!(instances.len(), 1);
    let source_states = source.initial();
    let package_states = package.initial();
    assert_eq!(source_states.len(), 1);
    assert_eq!(package_states.len(), 1);
    assert_eq!(
        package
            .state_key(&package_states[0])
            .expect("package state key"),
        source
            .state_key(&source_states[0])
            .expect("source state key"),
        "the actual initial model states"
    );
    let bare_world = bare.world().expect("bare expression world");
    eprintln!(
        "dense atom data declarations: retained input {:?}; expression world {:?}",
        input.input().types.attributes(object.clone()),
        bare_world.types().attributes(object.clone()),
    );
    let mut evaluated = 0;
    for (index, formula) in node.clause().formula().nodes().iter().enumerate() {
        if !matches!(formula, FormulaNode::Holds { .. }) {
            continue;
        }
        let atom = FormulaNodeId::from_arena_index(index);
        let mut source_meter = Meter::new(request.meter);
        let mut package_meter = Meter::new(request.meter);
        let expected = source_letters
            .atom(atom, &instances[0], &source_states[0], &mut source_meter)
            .expect("source atom");
        let actual = package_letters
            .atom(
                atom,
                &package_instances[0],
                &package_states[0],
                &mut package_meter,
            )
            .unwrap_or_else(|failure| {
                panic!(
                    "package atom: {failure:?}; original native call: {:?}",
                    package_letters.take_call_failure()
                )
            });
        assert_eq!(actual, expected);
        assert!(!source_meter.charge_log_truncated());
        assert!(!package_meter.charge_log_truncated());
        assert!(!source_meter.admitted_charges().is_empty());
        assert_eq!(
            package_meter.admitted_charges(),
            source_meter.admitted_charges()
        );
        evaluated += 1;
    }
    assert_eq!(evaluated, 1, "the actual holds atom was evaluated");
}
