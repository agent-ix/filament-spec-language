// SPDX-License-Identifier: AGPL-3.0-or-later

use filament_exact::{NodeKey, PathSegment};
use serde_json::json;

use super::{settle_transfer, Budget, TransferPremiseValue, TransferSettlement};
use crate::outcome::ItemLabel;
use crate::proof_result::Certification;

fn node(name: &str) -> NodeKey {
    NodeKey::new(
        "fixture".to_owned(),
        vec![PathSegment::Text(name.to_owned())],
        0,
    )
    .expect("a valid test node")
}

fn request(step_for_step: bool, fragment_operator: Option<&str>) -> TransferSettlement {
    TransferSettlement {
        refinement: node("R"),
        clause: node("P"),
        subject: json!({"identity": "example/concrete", "initial_states": []}),
        step_for_step: if step_for_step {
            json!({"holds": true})
        } else {
            json!({"holds": false, "row": {"operation": "M::Counter::retry", "arm": "stutter"}})
        },
        fragment_operator: fragment_operator.map(str::to_owned),
        fairness_implied: true,
        unimplied_constraint: None,
        liveness: false,
        stutter_closure: false,
        premises: [
            (
                json!({"key": "refinement"}),
                TransferPremiseValue::Proved(Certification::Certified),
            ),
            (
                json!({"key": "abstract"}),
                TransferPremiseValue::Proved(Certification::Certified),
            ),
        ],
    }
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-1")]
fn a_step_for_step_transfer_records_both_premises_and_is_proved() {
    let result = settle_transfer(&request(true, Some("once[1,1]")), Budget::default())
        .expect("a transfer record is writable");
    assert_eq!(result["value"], "proved");
    assert_eq!(result["basis"], "decisive-witness");
    assert_eq!(result["certification"], "certified");
    assert_eq!(result["transfer"]["fairness"]["implied"], true);
    assert_eq!(result["transfer"]["premise_form"], "safety");
    assert_eq!(result["transfer"]["fragment"]["class"], "position-counting");
    assert_eq!(result["transfer"]["fragment"]["operator"], "once[1,1]");
    assert_eq!(
        result["transfer"]["premises"].as_array().map(Vec::len),
        Some(2)
    );
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-1")]
fn a_transfer_keeps_the_weakest_certification_of_its_premises() {
    for (second, expected) in [
        (Certification::Uncertified, "uncertified"),
        (Certification::Trusted, "trusted"),
    ] {
        let mut request = request(true, None);
        request.premises[1].1 = TransferPremiseValue::Proved(second);
        let result = settle_transfer(&request, Budget::default()).expect("a record is writable");
        assert_eq!(result["value"], "proved");
        assert_eq!(result["certification"], expected);
    }
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
fn a_dense_time_refinement_is_not_step_for_step_before_any_row() {
    let mut request = request(false, Some("once[1,1]"));
    request.step_for_step = json!({"holds": false, "time": "dense"});
    let result = settle_transfer(&request, Budget::default()).expect("a record is writable");
    assert_eq!(result["cause"]["reason"], "fragment");
    assert_eq!(result["cause"]["operator"], "once[1,1]");
    assert_eq!(result["transfer"]["step_for_step"]["time"], "dense");
    assert!(result["transfer"]["step_for_step"].get("row").is_none());
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-2")]
fn a_non_step_for_step_transfer_names_the_first_position_counting_operator() {
    let result = settle_transfer(&request(false, Some("eventually[0,2]")), Budget::default())
        .expect("a transfer record is writable");
    assert_eq!(result["value"], "inconclusive");
    assert_eq!(result["basis"], "unsettled");
    assert_eq!(result["cause"]["type"], "transfer-not-preserving");
    assert_eq!(result["cause"]["reason"], "fragment");
    assert_eq!(result["cause"]["operator"], "eventually[0,2]");
    assert_eq!(result["transfer"]["step_for_step"]["row"]["arm"], "stutter");
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-4")]
fn transfer_settlement_checks_premise_form_then_premise_values() {
    let mut request = request(true, None);
    request.liveness = true;
    let result = settle_transfer(&request, Budget::default()).expect("a record is writable");
    assert_eq!(result["cause"]["reason"], "stutter-closure");

    assert_eq!(result["transfer"]["premise_form"], "maximal");

    request.stutter_closure = true;
    request.premises[0].1 = TransferPremiseValue::Other(ItemLabel::Unsupported);
    let result = settle_transfer(&request, Budget::default()).expect("a record is writable");
    assert_eq!(result["cause"]["type"], "premise-unproved");
    assert_eq!(result["cause"]["premise"], 0);
    assert_eq!(result["cause"]["result"], "unsupported");
    assert_eq!(result["transfer"]["premise_form"], "stutter-closure");

    request.premises[0].1 = TransferPremiseValue::Proved(Certification::Certified);
    for (label, spelling) in [
        (ItemLabel::Unsupported, "unsupported"),
        (ItemLabel::Incomplete, "incomplete"),
        (ItemLabel::Refuted, "refuted"),
    ] {
        request.premises[1].1 = TransferPremiseValue::Other(label);
        let result = settle_transfer(&request, Budget::default()).expect("a record is writable");
        assert_eq!(result["value"], "inconclusive");
        assert_eq!(result["cause"]["premise"], 1);
        assert_eq!(result["cause"]["result"], spelling);
    }
}

#[test]
#[ix_trace_rs::trace("FSpec-TC-2000", "FSpec-FR-820-AC-5")]
fn transfer_work_limit_keeps_the_complete_transfer_record() {
    let result = settle_transfer(&request(true, None), Budget { max_work: 0 })
        .expect("a record is writable");
    assert_eq!(result["value"], "incomplete");
    assert_eq!(result["cause"]["type"], "limit-reached");
    assert_eq!(result["cause"]["limit"], "max_work");
    assert_eq!(result["transfer"]["step_for_step"]["holds"], true);
    assert_eq!(result["transfer"]["fragment"]["class"], "stutter-invariant");
    assert_eq!(result["transfer"]["fairness"]["implied"], true);
}
