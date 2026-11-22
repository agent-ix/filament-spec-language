// SPDX-License-Identifier: AGPL-3.0-or-later
//! Literal loop invariants execute from admitted bytes at real repeat decisions.

use fsl_foundation::source::SourceLabel;
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Run, Workspace, analyze, label, package_selection_named, request_over,
};

fn request(
    ws: &Workspace,
    invariant: &str,
    variant: &str,
    guard: &str,
    kind: &str,
) -> std::path::PathBuf {
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () max 1 invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} sequence Body {{ {} }} \
         exhausted sequence Exhausted {{ }} }}",
        crate::fill::attempt("Touch", "touch", "TouchPre", "t", "true")
    );
    let body = crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d");
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots = documents.snapshots.into_values().collect::<Vec<_>>();
    request_over(
        ws,
        &crate::fill::unit_text(&body),
        &crate::fill::document(),
        &snapshots,
        json!({"subject": {"protocol": {"protocol": "P",
        "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
        "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}}},
        "items": [{"claim": "P::Main::Retry", "temporal": {"item_kind": kind}}]}),
    )
}

fn package_after_source_deletion(ws: &Workspace, path: &std::path::Path) -> Run {
    let input =
        fsl_command::command::prepare_analyze(path, fsl_command::command::RunLimits::default())
            .expect("the source request admits")
            .into_input();
    let package_name = input
        .source
        .source()
        .identity()
        .label(SourceLabel::Identity);
    let request: Value = serde_json::from_slice(&std::fs::read(path).expect("source request"))
        .expect("request JSON");
    let (members, bytes) = package_selection_named(ws, &request["request"], package_name);
    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("actual producer handoff compiles");
    fsl_package::admit_checked_package(
        &bytes,
        compiled.packages(),
        fsl_package::V2ReadLimits::bounded(1 << 24),
    )
    .expect("actual producer bytes admit");
    drop(input);
    drop(compiled);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained source copies");
    analyze(
        &ws.write(
            "package-analyze.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    )
}

/// A true invariant closes through EN-1; false is independently replayed at
/// entry even when the pretest guard is false. A true guard executes the body.
#[trace("TC-2022", "FR-730-AC-1", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn literal_loop_invariants_preserve_source_results_after_source_deletion() {
    for invariant in ["true", "false"] {
        for guard in ["true", "false"] {
            let ws = Workspace::new(&format!("package-loop-{invariant}-{guard}"));
            let path = request(&ws, invariant, "1", guard, "loop-invariant");
            let source = analyze(&path);
            let item = source.item(0);
            assert_eq!(
                label(&item),
                if invariant == "true" {
                    "proved"
                } else {
                    "refuted"
                },
                "{}",
                source.document()
            );
            assert_eq!(item["record"]["backend"], "en-1", "{item}");
            if invariant == "false" {
                assert_eq!(item["replay"]["reproduced"], true, "{item}");
            }
            let package = package_after_source_deletion(&ws, &path);
            assert_eq!(package.exit, source.exit, "{}", package.document());
            assert_eq!(
                package.stdout, source.stdout,
                "whole serialized result, including accounting"
            );
        }
    }
}

/// The admitted generic variant term remains attached and unexecuted when
/// only its invariant is selected, exactly as on the source route.
#[trace("TC-2022", "FR-730-AC-1", "FR-940-AC-6")]
#[test]
fn a_literal_invariant_does_not_evaluate_its_application_variant() {
    let ws = Workspace::new("package-loop-unused-variant");
    let path = request(&ws, "true", "1 + 0", "true", "loop-invariant");
    let source = analyze(&path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let package = package_after_source_deletion(&ws, &path);
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(package.stdout, source.stdout);
}

/// Application invariants and termination keep their named typed refusal.
#[trace("TC-2022", "FR-730", "FR-850", "FR-940")]
#[test]
fn unsupported_loop_proofs_refuse_by_name_before_model_execution() {
    for (invariant, kind) in [("1 = 1", "loop-invariant"), ("true", "loop-termination")] {
        let ws = Workspace::new(&format!("package-loop-unsupported-{kind}"));
        let path = request(&ws, invariant, "1", "false", kind);
        let package = package_after_source_deletion(&ws, &path);
        assert_eq!(package.exit, 21, "{}", package.document());
        let document = package.document();
        assert_eq!(document["items"], json!([]), "{document}");
        let serialized = document.to_string();
        assert!(serialized.contains("unsupported_construct"), "{document}");
        assert!(
            serialized.contains(&format!("names the {kind} family")),
            "{document}"
        );
    }
}

/// The actual source evaluator admits no scalar charge for either Boolean
/// leaf. Package decision reads retain that same zero-charge behavior.
#[trace("TC-2022", "FR-730-AC-1", "FR-940-AC-6")]
#[test]
fn literal_loop_invariant_source_evaluator_has_no_charges() {
    use filament_exact::{Meter, ScalarLimits};
    use fsl_eval::simulation::model::ModelEvaluator;
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::observation::{ObservationLimits, admit_trace_snapshot};
    for invariant in ["true", "false"] {
        let ws = Workspace::new(&format!("literal-loop-charges-{invariant}"));
        let path = request(&ws, invariant, "1", "true", "loop-invariant");
        // This produces the real handoff before inspecting the checked source.
        let input = fsl_command::command::prepare_analyze(
            &path,
            fsl_command::command::RunLimits::default(),
        )
        .expect("source admits");
        drop(input);
        let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
            .expect("actual producer handoff compiles");
        let checked = compiled.checked();
        let graph = checked.graph();
        let mut documents = crate::fill::Documents::new();
        let selected = documents.start("s0", &[("c", 0)]);
        let (objects, _) = admit_trace_snapshot(
            graph.model_selections(),
            graph.scope().types(),
            compiled.packages(),
            ModelNormalizationLimits::default(),
            &documents.snapshots,
            &selected,
            ObservationLimits::default(),
            &[],
        )
        .expect("snapshot admits");
        let proof = graph.protocol("P").expect("P").slots.repeats[0]
            .loop_proof
            .as_ref()
            .expect("proof");
        let mut meter = Meter::new(ScalarLimits {
            integer_bits: 0,
            decimal_digits: 0,
            scale_expansion: 0,
            text_input_bytes: 0,
            text_scalars: 0,
            normalized_scalars: 0,
            unit_edges: 0,
            value_occurrences: 0,
            work_units: 0,
            result_units: 0,
        });
        let evaluated = checked
            .evaluate_protocol_slot(&proof.invariant, vec![], &objects, &mut meter)
            .expect("literal evaluates");
        assert!(matches!(evaluated.outcome,
            filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(filament_exact::Value::Boolean(value)))
                if value == (invariant == "true")));
        assert_eq!(
            meter.admission_count(),
            0,
            "literal invariant admitted scalar charges"
        );
    }
}
