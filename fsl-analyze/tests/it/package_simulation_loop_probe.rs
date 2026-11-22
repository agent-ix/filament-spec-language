// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-valid probes for simulation and protocol loop-proof package execution.

use fsl_foundation::source::SourceLabel;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, label, package_selection_named, request_over, Run, Workspace, HANDOFF,
};

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
        .expect("the actual handoff compiles");
    let _admitted = fsl_package::admit_checked_package(
        &bytes,
        compiled.packages(),
        fsl_package::V2ReadLimits::bounded(1 << 24),
    )
    .expect("actual producer bytes admit through the strict reader");
    drop(input);
    drop(compiled);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained source copies");
    assert!(!ws.path("program.fsl").exists());
    assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
    analyze(
        &ws.write(
            "package-analyze.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    )
}

/// The claimless FR-634 analysis item enumerates through the native simulator.
/// Whole result equality includes published accounting; this probe does not
/// expose the engine's internal charge log.
#[trace("TC-1594", "FR-634-AC-1", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn simulation_package_preserves_whole_source_result_and_exit() {
    let ws = Workspace::new("package-simulation-source-parity");
    let path = counter_request(
        &ws,
        "",
        json!([{"kind": "simulation", "simulation": {"mode": "enumerate"}}]),
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    let item = source.item(0);
    assert_eq!(item["category"], "success", "{item}");
    assert_eq!(item["coverage"]["states"], "4", "{item}");
    assert_eq!(item["coverage"]["transitions"], "3", "{item}");
    assert_eq!(item["traces"].as_array().map(Vec::len), Some(1), "{item}");
    assert!(item.get("record").is_none(), "{item}");
    let package = package_after_source_deletion(&ws, &path);
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(package.stdout, source.stdout, "whole serialized result");
}

/// FR-730's real item kinds name the scoped proof-carrying repeat and reach
/// EN-1 from source; they are independent of refinement transfer aliases.
#[trace("TC-2022", "FR-730-AC-1", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn loop_proof_package_preserves_whole_source_result_and_exit() {
    let ws = Workspace::new("package-loop-proof-source-parity");
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () invariant {{ true }} \
         variant {{ 1 }} while {{ false }} sequence Body {{ {} }} }}",
        crate::fill::attempt("Touch", "touch", "TouchPre", "t", "true")
    );
    let body = crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d");
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 0)]);
    let snapshots = documents.snapshots.into_values().collect::<Vec<_>>();
    let path = request_over(
        &ws,
        &crate::fill::unit_text(&body),
        &crate::fill::document(),
        &snapshots,
        json!({"subject": {"protocol": {"protocol": "P",
        "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
        "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}}},
        "items": [
            {"claim": "P::Main::Retry", "temporal": {"item_kind": "loop-invariant"}},
            {"claim": "P::Main::Retry", "temporal": {"item_kind": "loop-termination"}}
        ]}),
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    for index in 0..2 {
        assert_eq!(
            label(&source.item(index)),
            "proved",
            "{}",
            source.document()
        );
        assert_eq!(source.item(index)["record"]["backend"], "en-1");
    }
    let package = package_after_source_deletion(&ws, &path);
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(package.stdout, source.stdout, "whole serialized result");
}
