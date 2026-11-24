// SPDX-License-Identifier: AGPL-3.0-or-later
//! Step-relation settlement preserves the complete source result from package bytes.

use fsl_foundation::source::SourceLabel;
use fsl_package::{AnalysisView, BarePackage};
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Workspace, analyze, label, package_selection_named, request_over,
};
use crate::hyper_world::{STARTS, Vault, request_files_over_starts};
use crate::package_model_relation::assert_tuple_evaluator_parity;
use crate::relation_claims::DETERMINISTIC;

/// The request explicitly selects the relation's TemporalSatisfaction capability.
/// Both routes retain the same model documents, snapshots, limits and options.
#[trace("FR-940-AC-23", "FR-940-AC-24", "FR-940-AC-25", "FR-179-AC-1")]
#[test]
fn step_relation_package_preserves_whole_result_exit_and_tuple_charges() {
    for (case, vault, reset, relation, temporal, expected) in [
        (
            "holds",
            Vault::Secure,
            false,
            DETERMINISTIC,
            json!({}),
            "proved",
        ),
        (
            "refuted",
            Vault::Custom("post Step using v on V::Vault::step { self.l = i or self.l = 1 - i }\n"),
            false,
            DETERMINISTIC,
            json!({}),
            "refuted",
        ),
        (
            "tuple-stop",
            Vault::Secure,
            false,
            DETERMINISTIC,
            json!({"limits": {"max_relation_tuples": 63}}),
            "incomplete",
        ),
        (
            "tuple-paid",
            Vault::Secure,
            false,
            DETERMINISTIC,
            json!({"limits": {"max_relation_tuples": 64}}),
            "proved",
        ),
        (
            "depth",
            Vault::Secure,
            false,
            DETERMINISTIC,
            json!({"max_depth": 1}),
            "inconclusive",
        ),
        (
            "reset-holds",
            Vault::Custom(""),
            true,
            "relation Det using v over (x: V::Vault::reset, y: V::Vault::reset) { pre(x.self.h) = pre(y.self.h) implies x.self.h = y.self.h }",
            json!({}),
            "proved",
        ),
        (
            "reset-refuted",
            Vault::Custom(""),
            true,
            "relation Det using v over (x: V::Vault::reset, y: V::Vault::reset) { x.self.l = y.self.l }",
            json!({}),
            "refuted",
        ),
    ] {
        eprintln!("step-relation whole-result/exit/charge parity case: {case}");
        let ws = Workspace::new(&format!("step-relation-whole-parity-{case}"));
        let (unit, model, snapshots) = request_files_over_starts(vault, reset, &STARTS, relation);
        let path = request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "Det", "kind": "temporal-satisfaction", "temporal": temporal}]}),
        );
        let source = analyze(&path);
        assert_eq!(
            label(&source.item(0)),
            expected,
            "{case}: {}",
            source.document()
        );
        assert_eq!(source.item(0)["record"]["backend"], "en-1");

        let input = fsl_command::command::prepare_analyze(
            &path,
            fsl_command::command::RunLimits::default(),
        )
        .expect("the source request admits")
        .into_input();
        let package_name = input
            .source
            .source()
            .identity()
            .label(SourceLabel::Identity);
        let request: Value = serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
        let (members, bytes) = package_selection_named(&ws, &request["request"], package_name);
        let (packages, _, _) = crate::relation_claims::initial_world(&model);
        let admitted = fsl_package::admit_checked_package(
            &bytes,
            &packages,
            fsl_package::V2ReadLimits::bounded(1 << 24),
        )
        .expect("actual producer bytes admit through the strict reader");
        let bare = BarePackage::new(&admitted);
        let form = bare
            .model_relation_form(&bare.claim_name("Det").expect("relation occurrence"))
            .expect("the admitted step-relation form");
        assert_eq!(
            form.requirement().kind,
            filament_contract_model::CapabilityKind::TemporalSatisfaction
        );
        assert_tuple_evaluator_parity(&ws, &bytes, false, false);
        drop(input);

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
        assert_eq!(package.exit, source.exit, "{case}: {}", package.document());
        assert_eq!(
            package.stdout, source.stdout,
            "{case}: whole serialized result"
        );
    }
}
