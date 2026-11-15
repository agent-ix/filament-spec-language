// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620, FR-621 and FR-622 (TC-1580 to TC-1582): the `native-analyze/1`
//! reader's refusals of a request with no program and of an unknown item
//! kind, the exit code of every outcome category the document writes, and
//! the independence of the four outcome axes.

use filament_semantic_value::catalog::Category;
use fsl_replay::{ActivationAxis, ExecutionAxis, OutcomeAxes, SettlementBasis, TruthAxis};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, counter_request, counter_request_with, request_members,
    Workspace,
};

/// FR-620-AC-3: a request with neither `program` nor `package`, and an item
/// whose `temporal.item_kind` is no FSpec FR-368 spelling, each refuse the
/// whole request at the request stage with `invalid-request`, exit 20, and
/// run no item.
#[trace("TC-1580", "FR-620-AC-3")]
#[test]
fn analyze_request_rejects_missing_program_and_unknown_item_kind() {
    let ws = Workspace::new("wire-no-program");
    let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
    request
        .as_object_mut()
        .expect("the request object")
        .remove("program");
    let run = analyze(&counter_request_with(&ws, &bounded(), request));
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["items"], json!([]));
    assert_eq!(document["diagnostics"][0]["code"], "invalid-request");

    let ws = Workspace::new("wire-unknown-item-kind");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded", "temporal": {"item_kind": "eventually-free"}}]),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
}

/// FR-621 "Categories and exit": every category has exactly one exit code,
/// and each written document exits with its category's code: `proved` 0,
/// `refuted` 10, `incomplete` 22, and an unknown claim beside a proof 20.
#[trace("TC-1581", "FR-621-AC-4")]
#[test]
fn analyze_outcome_exit_codes_are_total() {
    let codes: Vec<(&str, u8)> = Category::ALL
        .iter()
        .map(|category| (category.as_str(), category.exit_code()))
        .collect();
    for (label, code) in &codes {
        assert_eq!(
            codes.iter().filter(|(other, _)| other == label).count(),
            1,
            "{label} has one row"
        );
        assert!([0, 10, 11, 20, 21, 22, 30].contains(code), "{label} {code}");
    }
    let body = format!("{}{}", bounded(), below_three());
    for (items, exit) in [
        (json!([{"claim": "Bounded"}]), 0),
        (json!([{"claim": "BelowThree"}]), 10),
        (
            json!([{"claim": "Bounded", "temporal": {"limits": {"max_states": 1}}}]),
            22,
        ),
        (json!([{"claim": "Bounded"}, {"claim": "Missing"}]), 20),
    ] {
        let ws = Workspace::new(&format!("exit-{exit}"));
        let run = analyze(&counter_request(&ws, &body, items));
        let document = run.document();
        let category = Category::ALL
            .iter()
            .find(|category| Value::from(category.as_str()) == document["category"])
            .expect("a registered category");
        assert_eq!(run.exit, i32::from(category.exit_code()), "{document}");
        assert_eq!(run.exit, exit, "{document}");
    }
}

/// FR-622 "Independence": an analyze item's axes with one axis altered is
/// not equal to the original, and each axis is written under its own member
/// in FSpec FR-521's spelling.
#[trace("TC-1582", "FR-622-AC-3")]
#[test]
fn changing_one_axis_changes_only_that_field() {
    let ws = Workspace::new("axes-one");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"}]),
    ));
    let written = run.item(0)["axes"].clone();
    let original: OutcomeAxes = serde_json::from_value(written.clone()).expect("the axes read");
    assert_eq!(
        serde_json::to_value(original).expect("the axes write"),
        written
    );
    let altered = [
        OutcomeAxes {
            activation: ActivationAxis::Inactive,
            ..original
        },
        OutcomeAxes {
            execution: Some(ExecutionAxis::Failed),
            ..original
        },
        OutcomeAxes {
            truth: Some(TruthAxis::False),
            ..original
        },
        OutcomeAxes {
            settlement: Some(SettlementBasis::Unsettled),
            ..original
        },
    ];
    for (axis, changed) in ["activation", "execution", "truth", "settlement"]
        .into_iter()
        .zip(altered)
    {
        assert_ne!(changed, original, "{axis}");
        let value = serde_json::to_value(changed).expect("the axes write");
        for member in ["activation", "execution", "truth", "settlement"] {
            if member == axis {
                assert_ne!(value[member], written[member], "{axis}");
            } else {
                assert_eq!(value[member], written[member], "{axis} moved {member}");
            }
        }
    }
}
