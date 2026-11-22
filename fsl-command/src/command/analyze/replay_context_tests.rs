// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine source acquisition with lost optional library-retention handles.

use super::*;
use serde_json::json;

#[path = "../../../tests/it/negotiation.rs"]
mod negotiation;

/// FR-621-AC-18: optional context failure cannot erase a mandatory scoped
/// empty-prefix Protocol counterexample or change its reproduced refutation.
#[test]
#[ix_trace_rs::trace("FR-621-AC-18", "FR-621-AC-20")]
fn missing_library_retention_preserves_protocol_refutation() {
    let directory = tempfile::tempdir().unwrap();
    let source = r#"language "ix:native" edition "1-draft";
profile values = "filament.value.complete/v1";
profile paths = "filament.temporal.infinite-trace/v1";
profile protocols = "filament.protocol.finite-global/v1";
import "test/retained" as L;
model M = "test/counter";
function Ready using values(): Boolean pure { L::Ready() }
temporal EventuallyChanged using paths over (q: M::Counter) clock "model-steps" on origin {
  eventually holds(q.value > 0)
}
protocol Flow using protocols over (view: M::Counter) on each (opened: Boolean) when (false) {
  role Worker on M::Counter;
  run sequence Main { }
  finish Done as (done: Boolean) { true };
}
"#;
    std::fs::write(directory.path().join("program.fsl"), source).unwrap();
    std::fs::write(directory.path().join("library.fsl"),
        "language \"ix:native\" edition \"1-draft\";\nprofile values = \"filament.value.complete/v1\";\nfunction Ready using values(): Boolean pure { true }\n").unwrap();
    std::fs::write(
        directory.path().join("counter.json"),
        include_bytes!("../../../tests/fixtures/cli-contract/counter.semantic-ir.json"),
    )
    .unwrap();
    let start = json!({"format":"filament.state.snapshot/v1", "identity":{"authority":"test","identity":"start"},
        "observation":"current","model":{"identity":"test/counter"},"anchor":{"kind":"initialization","name":"start"},
        "populations":[{"population":"ix://test/counter/counters","complete":true,"objects":[{"key":"q","type":"ix://test/counter/Counter","fields":{"value":{"integer":"0"}}}]}]});
    std::fs::write(
        directory.path().join("start.json"),
        serde_json::to_vec(&start).unwrap(),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&json!({"format":"native-analyze/1","request":{
        "program":{"source":{"file":"program.fsl","authority":"test","identity":"program","document":"Program","formal_revision":1}},
        "libraries":[{"identity":"test/retained","source":{"file":"library.fsl","authority":"test","identity":"library","document":"Library","formal_revision":1}}],
        "models":[{"format":"semantic-ir/2.0.0","source":{"file":"counter.json","authority":"test","identity":"counter","document":"Counter","formal_revision":1}}],
        "documents":[{"file":"start.json"}],"items":[{"claim":"EventuallyChanged","temporal":{"subject":{"protocol":{"protocol":"Flow","over":{"reference":{"population":"ix://test/counter/counters","key":"q"}},"roles":{"Worker":{"type":"model-binding","selection":"test/counter"}}}}}}]}})).unwrap();
    let Readout::Source(input, mut intake) =
        read(directory.path(), &bytes, RunLimits::default()).unwrap()
    else {
        panic!("actual source request");
    };
    let AnalyzeInput {
        source,
        compile,
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
        integer_division,
    } = input;
    let routed = RoutedItems {
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
    };
    let engines = engines_with(Vec::new(), &ProviderOptions::default()).unwrap();
    let cancel = Cancel::new();
    let settled = check_sources(
        &compile,
        &mut intake,
        std::slice::from_ref(&source),
        false,
        fsl_replay::spine::SpineLimits::default(),
        integer_division,
        None,
        |_intake, checked, selections, _, packages, libraries, _, _, _, _| {
            assert!(
                !checked.shared_library_packages().is_empty(),
                "genuine checked library was acquired"
            );
            settle_items(
                checked.package(),
                checked.model_admission_owner().cloned(),
                std::sync::Arc::default(),
                &packages,
                std::iter::once(&source).chain(libraries),
                selections,
                &routed,
                &Settle {
                    negotiate: &negotiation::Fr290,
                    engines: &engines,
                    cancel: &cancel,
                },
            )
        },
    )
    .unwrap();
    assert_eq!(settled.exit_code, 10, "{}", settled.document);
    assert_eq!(settled.document["items"][0]["record"]["result"], "refuted");
    assert!(settled.document["items"][0]["counterexample"]["prefix"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        settled.document["items"][0]["counterexample"]["loop"][0]["transition"]["type"],
        "terminal-stutter"
    );
    let capture = settled
        .replay_contexts
        .values()
        .next()
        .expect("explicit optional retention failure");
    assert!(capture.raw().is_none());
    assert!(capture
        .failure()
        .unwrap()
        .contains("genuine dependency package handle was not retained"));
}
