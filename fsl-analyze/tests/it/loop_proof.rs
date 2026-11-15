// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-730 (TC-2022): a repeat's loop proof is discharged over the explored
//! protocol subject as two derived items of the repeat, `loop-invariant` and
//! `loop-termination`, each settling `proved`, `refuted` or `incomplete`.

use fsl_route::request::ItemKind;
use ix_trace_rs::trace;

/// FR-730-AC-4: the request writer has the two derived item kinds a loop
/// proof contributes, beside deadlock freedom, so a repeat's invariant and
/// termination obligations are routed and settled like every other derived
/// item instead of being recorded and left undischarged.
#[trace("TC-2022", "FR-730-AC-4", "FSpec-TC-1520", "FSpec-FR-171-AC-6")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn a_loop_proof_contributes_invariant_and_termination_item_kinds() {
    let kinds: Vec<&str> = ItemKind::ALL.iter().map(|kind| kind.to_wire()).collect();
    for expected in ["loop-invariant", "loop-termination"] {
        assert!(
            kinds.contains(&expected),
            "{expected} is a derived item kind: {kinds:?}"
        );
    }
}

/// A unit whose protocol `P` holds `repeat Retry` with no maximum, the loop
/// proof `invariant { invariant } variant { variant }` and guard `guard`,
/// its body one `touch` attempt.
fn loop_unit(invariant: &str, variant: &str, guard: &str) -> String {
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} sequence Body {{ {} }} }}",
        crate::fill::attempt("T", "touch", "TouchPre", "t", "true")
    );
    crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d")
}

/// The derived items `loop-invariant` and `loop-termination` of `P` in
/// `body`, through the request writer that derives every protocol item.
fn loop_items(body: &str) -> Vec<&'static str> {
    use crate::derived_request::protocol_request;
    use crate::model_settle::protocol_anchor;
    use fsl_route::deadlock::ProtocolTemporalItem;

    let checked = crate::fill::package_with(crate::fill::PROTOCOL_PROFILE, body);
    let protocol = checked.graph().protocol("P").expect("P");
    let occurrence = protocol_anchor(&checked, "P").1;
    let items = [ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"P",
        protocol,
        models: &[],
        non_atomic_access: false,
    }];
    let written = protocol_request(&items);
    written
        .items()
        .iter()
        .filter_map(|item| item.item_kind())
        .map(ItemKind::to_wire)
        .filter(|kind| kind.starts_with("loop-"))
        .collect()
}

/// `analyze`'s outcome items for `P`'s `loop-invariant` and
/// `loop-termination` items over `body`, from cell `c` at `v = 0`, with the
/// further `temporal` members `extra` on each item, through the in-process
/// command adapter.
fn analyze_loop(name: &str, body: &str, extra: serde_json::Value) -> [serde_json::Value; 2] {
    use crate::analyze_cli_support::{analyze, request_over, Workspace};

    let mut documents = crate::fill::Documents::new();
    documents.start("start", &[("c", 0)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.into_values().collect();
    let item = |kind: &str| {
        let mut temporal = serde_json::json!({"item_kind": kind});
        for (member, value) in extra.as_object().expect("temporal members") {
            temporal[member] = value.clone();
        }
        serde_json::json!({"claim": "P", "temporal": temporal})
    };
    let ws = Workspace::new(name);
    let run = analyze(&request_over(
        &ws,
        &crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, body),
        &crate::fill::document(),
        &snapshots,
        serde_json::json!({"items": [item("loop-invariant"), item("loop-termination")]}),
    ));
    [run.item(0), run.item(1)]
}

/// FR-730-AC-1: a repeat whose variant cannot fail to decrease settles both
/// loop items `proved` over a complete exploration.
#[trace("TC-2022", "FR-730-AC-1")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn a_decreasing_variant_proves_both_loop_items() {
    use crate::analyze_cli_support::label;

    let body = loop_unit("true", "0", "false");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    for item in analyze_loop("loop-proved", &body, serde_json::json!({})) {
        assert_eq!(label(&item), "proved", "{item}");
        assert_eq!(item["record"]["basis"]["type"], "exhaustive", "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}

/// FR-730-AC-2: a variant that never decreases refutes `loop-termination`,
/// and a false invariant refutes `loop-invariant`, each with a counterexample
/// that replays.
#[trace("TC-2022", "FR-730-AC-2")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn an_unchanged_variant_and_a_false_invariant_refute_with_replayed_counterexamples() {
    use crate::analyze_cli_support::label;

    for (invariant, variant, refuted, other) in
        [("true", "1", 1, Some("proved")), ("false", "0", 0, None)]
    {
        let body = loop_unit(invariant, variant, "true");
        assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
        let items = analyze_loop(
            &format!("loop-refuted-{refuted}"),
            &body,
            serde_json::json!({}),
        );
        let item = &items[refuted];
        assert_eq!(label(item), "refuted", "{invariant}/{variant}: {item}");
        assert!(item["counterexample"]["witness"].is_object(), "{item}");
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        if let Some(expected) = other {
            assert_eq!(label(&items[1 - refuted]), expected, "{items:?}");
        }
    }
}

/// FR-730-AC-3: exploration stopped by a state limit below the reachable
/// state count settles both loop items `incomplete`, naming the limit and
/// claiming neither result.
#[trace("TC-2022", "FR-730-AC-3")]
#[ignore = "QSL-2222: loop-invariant and loop-termination items are not implemented yet"]
#[test]
fn a_state_limit_leaves_both_loop_items_inconclusive_naming_it() {
    use crate::analyze_cli_support::label;

    let body = loop_unit("true", "1", "true");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    let items = analyze_loop(
        "loop-limited",
        &body,
        serde_json::json!({"limits": {"max_states": 1}}),
    );
    for item in items {
        assert_eq!(label(&item), "incomplete", "{item}");
        assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}
