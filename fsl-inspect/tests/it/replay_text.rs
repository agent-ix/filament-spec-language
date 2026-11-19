// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-865-AC-6 (TC-2963): `render_outcome` writes a replay outcome's
//! `replayed` result (FR-286) as one `result <index>: <arm> <settlement>` line
//! per entry after the header, and a refused replay with no `result` line.

use filament_exact::{Cancel, ScalarLimits};
use filament_semantic_value::catalog::Category;
use fsl_inspect::{render_outcome, RenderForm, RenderSources};
use fsl_replay::{
    EvaluatedValue, InputArmResult, Operation, OutcomeDiagnostic, OutcomeDocument, RenderLimits,
    ReplayResult, Verdict, WitnessCheck,
};
use ix_trace_rs::trace;

fn text(document: &OutcomeDocument) -> String {
    let rendered = render_outcome(
        document,
        RenderForm::Text,
        &RenderSources::from_sources(Vec::new()),
        RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the document renders");
    rendered.value().as_str().to_owned()
}

/// FR-865-AC-6: the replayed result renders one line per entry after the
/// header. The document is the one `OutcomeDocument::from_replay` builds from
/// a reproduced `input` replay.
#[test]
#[trace("TC-2963", "FR-865-AC-6")]
fn a_replay_outcome_renders_one_result_line_per_entry() {
    let success = Verdict::from_category(Category::Success);
    let arm = InputArmResult::settle(
        success,
        success,
        Category::Success,
        Some(EvaluatedValue::Boolean(true)),
        &WitnessCheck::Agrees(None),
        Vec::new(),
        ScalarLimits {
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
        },
    );
    let document = OutcomeDocument::from_replay(&Ok(ReplayResult::Input(arm)));
    let text = text(&document);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines,
        [
            "replay: success (exit 0) at S8",
            "result 0: input reproduced-without-witness"
        ],
        "{text}"
    );
}

/// FR-286-AC-21 and FR-865-AC-6: a refused replay has a null result and text
/// carries its refusal diagnostic without a result line.
#[test]
#[trace("TC-2963", "FR-286-AC-21", "FR-865-AC-6")]
fn a_refused_replay_outcome_has_no_result_line() {
    let document = OutcomeDocument::new(Operation::Replay, None, Category::Refusal)
        .with_diagnostics(vec![OutcomeDiagnostic::new(
            None,
            "invalid-request",
            None,
            "the request carries a clocks member",
        )]);
    let json: serde_json::Value = serde_json::from_slice(&document.to_bytes().expect("JSON"))
        .expect("the outcome document is JSON");
    assert_eq!(json["result"], serde_json::Value::Null);

    let rendered = text(&document);
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(lines[0], "replay: refusal (exit 20)");
    assert!(lines[1].contains("invalid-request:"), "{rendered}");
    assert!(
        !lines.iter().any(|line| line.starts_with("result")),
        "{rendered}"
    );
}
