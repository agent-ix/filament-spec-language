// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-730 (TC-2022): a repeat's loop proof is discharged over the explored
//! protocol subject as two derived items of the repeat, `loop-invariant` and
//! `loop-termination`, each settling `proved`, `refuted` or `incomplete`.

use fsl_route::deadlock::ProtocolTemporalItem;
use fsl_route::request::ItemKind;
use ix_trace_rs::trace;

/// FR-730-AC-4: the request writer has the two derived item kinds a loop
/// proof contributes, beside deadlock freedom, so a repeat's invariant and
/// termination obligations are routed and settled like every other derived
/// item instead of being recorded and left undischarged.
#[trace("TC-2022", "FR-730-AC-4", "FSpec-TC-1520", "FSpec-FR-171-AC-6")]
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
/// proof `invariant { invariant } variant { variant }` and guard `guard`.
/// Its body steps the cell `k` with `operation` and then observes the cell's
/// counter as `got.left`, so the decision slots read a body binder, as
/// FR-730's use case does.
fn loop_unit(invariant: &str, variant: &str, guard: &str, operation: &str) -> String {
    let contracts = match operation {
        "dec" => "DecPre, DecPost",
        _ => "TouchPre",
    };
    let run = format!(
        "sequence Main {{ repeat Retry by w visible (got.left) invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} \
         sequence Body {{ {} event Seen by w as (got: Left) {{ got.left = k.v }}; }} }}",
        crate::fill::attempt("T", operation, contracts, "t", "true")
    );
    format!("record Left {{ left: Int[0, 3]; }}\n{}", loop_pieces(&run))
}

/// The contracts and the protocol `P` over `run`.
fn loop_pieces(run: &str) -> String {
    crate::fill::contracts("self.v = 0")
        + "pre DecPre using v on M::Cell::dec { self.v >= 1 }\n\
           post DecPost using v on M::Cell::dec { pre(self.v) - self.v = 1 }\n"
        + &crate::fill::protocol("P", "", run, "d")
}

/// The derived items `loop-invariant` and `loop-termination` of `P` in
/// `body`, through the request writer that derives every protocol item.
fn loop_items(body: &str) -> Vec<&'static str> {
    loop_items_for_request_count(body, 1)
}

/// The loop-proof item kinds written for `request_count` caller requests of
/// the same protocol subject.
fn loop_items_for_request_count(body: &str, request_count: usize) -> Vec<&'static str> {
    use crate::derived_request::protocol_request;
    use crate::model_settle::protocol_anchor;

    let checked = crate::fill::package_with(crate::fill::PROTOCOL_PROFILE, body);
    let protocol = checked.graph().protocol("P").expect("P");
    let occurrence = protocol_anchor(&checked, "P").1;
    let item = ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"P",
        protocol,
        models: &[],
        non_atomic_access: false,
    };
    let items = vec![item; request_count];
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
/// `loop-termination` items over `body`, from cell `c` at `v = initial`, with the
/// further `temporal` members `extra` on each item, through the in-process
/// command adapter.
fn analyze_loop(
    name: &str,
    body: &str,
    initial: i64,
    extra: serde_json::Value,
) -> [serde_json::Value; 2] {
    use crate::analyze_cli_support::{analyze, request_over, Workspace};

    let mut documents = crate::fill::Documents::new();
    documents.start("start", &[("c", initial)]);
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

/// FR-730-AC-1: a repeat with no maximum, the loop proof
/// `invariant { got.left >= 0 } variant { got.left }` and a body that
/// decreases `got.left` by one from `2` settles both loop items `proved` over a complete
/// exploration.
#[trace("TC-2022", "FR-730-AC-1")]
#[test]
fn a_decreasing_variant_proves_both_loop_items() {
    use crate::analyze_cli_support::label;

    let body = loop_unit("got.left >= 0", "got.left", "got.left > 0", "dec");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    for item in analyze_loop("loop-proved", &body, 2, serde_json::json!({})) {
        assert_eq!(label(&item), "proved", "{item}");
        assert_eq!(item["record"]["basis"]["type"], "exhaustive", "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}

/// FR-730-AC-2: a body that leaves `got.left` unchanged refutes
/// `loop-termination` with a counterexample that replays and still proves
/// `loop-invariant`; the invariant `got.left > 0` over the decreasing body
/// refutes `loop-invariant` at the decision where `got.left` is `0`, after two
/// body steps.
#[trace("TC-2022", "FR-730-AC-2")]
#[test]
fn an_unchanged_variant_and_a_false_invariant_refute_with_replayed_counterexamples() {
    use crate::analyze_cli_support::label;

    for (invariant, operation, refuted, other) in [
        ("got.left >= 0", "touch", 1, Some("proved")),
        ("got.left > 0", "dec", 0, None),
    ] {
        let body = loop_unit(invariant, "got.left", "got.left > 0", operation);
        assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
        let items = analyze_loop(
            &format!("loop-refuted-{refuted}"),
            &body,
            2,
            serde_json::json!({}),
        );
        let item = &items[refuted];
        assert_eq!(label(item), "refuted", "{invariant}/{operation}: {item}");
        assert!(item["counterexample"]["witness"].is_object(), "{item}");
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        if let Some(expected) = other {
            assert_eq!(label(&items[1 - refuted]), expected, "{items:?}");
        }
    }
}

/// FR-730-AC-3: the AC-1 protocol checked with a state limit below its
/// reachable state count settles both loop items `incomplete`, naming the
/// state limit and claiming neither result.
#[trace("TC-2022", "FR-730-AC-3")]
#[test]
fn a_state_limit_leaves_both_loop_items_incomplete_naming_it() {
    use crate::analyze_cli_support::label;

    let body = loop_unit("got.left >= 0", "got.left", "got.left > 0", "dec");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    let items = analyze_loop(
        "loop-limited",
        &body,
        2,
        serde_json::json!({"limits": {"max_states": 1}}),
    );
    for item in items {
        assert_eq!(label(&item), "incomplete", "{item}");
        assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
        assert!(item.get("counterexample").is_none(), "{item}");
    }
}

/// A protocol with two proof-carrying repeats and one repeat with only a
/// maximum.
fn two_proofs_and_a_bounded_repeat() -> String {
    let attempt = |node: &str, binder: &str| {
        crate::fill::attempt(node, "dec", "DecPre, DecPost", binder, "true")
    };
    let run = format!(
        "sequence Main {{ \
         repeat First by w visible () invariant {{ true }} variant {{ 1 }} while {{ false }} \
           sequence FirstBody {{ {} }} \
         repeat Second by w visible () invariant {{ true }} variant {{ 1 }} while {{ false }} \
           sequence SecondBody {{ {} }} \
         repeat Bounded by w visible () max 1 while {{ false }} \
           sequence BoundedBody {{ {} }} \
           exhausted check Out using v {{ true }}; }}",
        attempt("FirstStep", "first"),
        attempt("SecondStep", "second"),
        attempt("BoundedStep", "bounded"),
    );
    loop_pieces(&run)
}

/// FR-730-AC-4: two proof-carrying repeats contribute two items each, the
/// repeat with only a maximum contributes none, and two requests over the one
/// subject derive the four items once.
#[trace("TC-2022", "FR-730-AC-4")]
#[test]
fn two_loop_proofs_derive_four_items_once_and_a_bounded_repeat_derives_none() {
    let body = two_proofs_and_a_bounded_repeat();
    let four = [
        "loop-invariant",
        "loop-termination",
        "loop-invariant",
        "loop-termination",
    ];
    assert_eq!(loop_items(&body), four);
    assert_eq!(loop_items_for_request_count(&body, 2), four);
}

/// FR-730 "Obligations": a repeat with no loop proof derives neither
/// obligation.
#[trace("TC-2022", "FR-730-AC-4")]
#[test]
fn a_repeat_without_a_loop_proof_derives_no_loop_items() {
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () while {{ false }} sequence Body {{ {} }} }}",
        crate::fill::attempt("T", "touch", "TouchPre", "t", "true")
    );
    let body = crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d");
    assert!(loop_items(&body).is_empty());
}
