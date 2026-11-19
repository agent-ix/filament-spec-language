// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-622 and FR-267-AC-22 (TC-1582), FSpec FR-521: the `axes` object a
//! clause-run result carries is four independent members. Each axis takes
//! every value of its own vocabulary while the other three stay as they
//! were, and the written object reads back to an equal value; the reader
//! refuses an unknown member and an absent one.

use fsl_replay::{ActivationAxis, ExecutionAxis, OutcomeAxes, SettlementBasis, TruthAxis};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const BASE: OutcomeAxes = OutcomeAxes {
    activation: ActivationAxis::Active,
    execution: Some(ExecutionAxis::Completed),
    truth: Some(TruthAxis::True),
    settlement: Some(SettlementBasis::ClosedScope),
};

fn written(axes: OutcomeAxes) -> Value {
    serde_json::to_value(axes).expect("the axes write")
}

#[trace("TC-1582", "FR-622-AC-1")]
#[trace("TC-1081", "FSpec-FR-521-AC-1")]
#[test]
fn each_axis_is_independently_settable_and_echoed() {
    assert_eq!(
        written(BASE),
        json!({"activation": "active", "execution": "completed", "truth": "true",
            "settlement": "closed-scope"})
    );
    let mut variants: Vec<(&str, OutcomeAxes)> = Vec::new();
    for activation in [
        ActivationAxis::Inactive,
        ActivationAxis::Unknown,
        ActivationAxis::Unavailable,
    ] {
        variants.push(("activation", OutcomeAxes { activation, ..BASE }));
    }
    for execution in [Some(ExecutionAxis::Failed), None] {
        variants.push(("execution", OutcomeAxes { execution, ..BASE }));
    }
    for truth in [Some(TruthAxis::False), Some(TruthAxis::Pending), None] {
        variants.push(("truth", OutcomeAxes { truth, ..BASE }));
    }
    for settlement in [
        Some(SettlementBasis::DecisiveWitness),
        Some(SettlementBasis::DecisiveCounterexample),
        Some(SettlementBasis::Unsettled),
        Some(SettlementBasis::Unavailable),
        None,
    ] {
        variants.push(("settlement", OutcomeAxes { settlement, ..BASE }));
    }
    let base = written(BASE);
    for (axis, axes) in variants {
        let value = written(axes);
        for member in ["activation", "execution", "truth", "settlement"] {
            if member == axis {
                assert_ne!(value[member], base[member], "{axis}: {value}");
            } else {
                assert_eq!(value[member], base[member], "{axis} moved {member}");
            }
        }
        let read: OutcomeAxes = serde_json::from_value(value.clone()).expect("the axes read");
        assert_eq!(read, axes, "{value}");
    }
}

#[trace("TC-1582", "FR-267-AC-22")]
#[test]
fn the_axes_reader_is_closed() {
    let mut extra = written(BASE);
    extra["colour"] = json!("blue");
    assert!(serde_json::from_value::<OutcomeAxes>(extra).is_err());
    let mut absent = written(BASE);
    absent.as_object_mut().expect("an object").remove("truth");
    assert!(serde_json::from_value::<OutcomeAxes>(absent).is_err());
    let mut unknown = written(BASE);
    unknown["activation"] = json!(null);
    assert!(serde_json::from_value::<OutcomeAxes>(unknown).is_err());
}
