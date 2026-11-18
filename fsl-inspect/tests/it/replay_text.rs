// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-865-AC-6 (TC-2963): `render_outcome` writes a replay outcome's
//! `replayed` result (FR-286) as one `result <index>: <arm> <settlement>` line
//! per entry after the header, and a refused replay with no `result` line.

use filament_exact::Cancel;
use filament_semantic_value::catalog::Category;
use fsl_inspect::{render_outcome, RenderForm, RenderSources};
use fsl_replay::{Operation, OutcomeDocument, OutcomeStage, RenderLimits};
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
/// header. The document is the one `OutcomeDocument::from_replay` builds for
/// FR-862-AC-2's replay of `entry`; until FR-286's `replayed` result exists
/// the stub builds the header-only document, so the result line is missing.
#[test]
#[trace("TC-2963", "FR-865-AC-6")]
#[ignore = "QSL-2292: the replay outcome document has no replayed result and render_outcome writes no result line for it yet"]
fn a_replay_outcome_renders_one_result_line_per_entry() {
    let document =
        OutcomeDocument::new(Operation::Replay, Some(OutcomeStage::S8), Category::Success);
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
