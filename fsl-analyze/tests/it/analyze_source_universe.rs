// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569 source declarations named by request snapshots.

use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
use crate::prob_world;
use ix_trace_rs::trace;
use serde_json::json;

fn run_with_source_population_universe() -> Run {
    let model = prob_world::coin(
        "object Lamp { lit: Boolean; population lamps; }\n\
         invariant LampOff using v on Lamp at current { not self.lit }\n\
         probabilistic Seen using ev on M over (c: M::Coin) under every scheduler on origin\n\
         { probability >= 0.5 [ always[0,1] holds(c.v = 0) ] }\n",
    );
    let (unit, model_document) = model.unit_and_document();
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current",
        "model": {"identity": model.package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [
            {"population": "ix://test/coin/coins", "complete": true,
             "objects": [{"key": "c", "type": "ix://test/coin/Coin",
                "fields": {"v": {"integer": "0"}}}]},
            {"population": "lamps", "complete": true,
             "objects": [{"key": "l", "type": "Lamp",
                "fields": {"lit": {"boolean": true}}}]}
        ]
    });
    let ws = Workspace::new("request-source-population-universe");
    analyze(&request_over(
        &ws,
        &unit,
        &model_document,
        &[snapshot.to_string().into_bytes()],
        json!({"items": [{"claim": "Seen"}, {"claim": "LampOff"}]}),
    ))
}

/// FR-569-AC-9: a scheme-less source population is admitted and read beside a
/// model population in the same request snapshot.
#[trace("TC-1408", "FR-569-AC-9")]
#[test]
fn request_snapshot_admits_a_scheme_less_source_population_universe() {
    let run = run_with_source_population_universe();
    assert_ne!(run.document()["category"], "refusal", "{}", run.document());
    assert!(run.document()["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(run.document()["items"].as_array().unwrap().len(), 2);
    let lamp = run.item(1);
    assert_eq!(lamp["claim_name"], "LampOff", "{}", run.document());
    assert_eq!(label(&lamp), "refuted", "{}", run.document());
    assert_eq!(lamp["replay"]["reproduced"], true, "{}", run.document());
}
