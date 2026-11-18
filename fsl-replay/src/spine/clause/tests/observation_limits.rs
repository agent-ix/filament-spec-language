// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 observation input-byte limits use one name and one item shape.

use fsl_foundation::{RequestIndex, Setting, SettingLimits};
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use super::{no_cycle_request, run_clause};
use crate::outcome::OutcomeItem;

const LIMIT: u64 = 10;

fn limited_item(limits: ObservationLimits) -> Value {
    let (mut request, _) = no_cycle_request(&[("a", Some("b")), ("b", None)]);
    request.observation_limits = limits;
    let report = run_clause(request).expect("a selected snapshot produces a clause report");
    serde_json::to_value(
        OutcomeItem::from_clause_run(RequestIndex::new(0), "NoCycle", &report)
            .expect("an incomplete admission is a result item"),
    )
    .expect("the result item serializes")
}

/// Trace: FR-255-AC-1..4, request and caller settings produce the same item.
#[trace("TC-106", "FR-255-AC-1", "FR-255-AC-2", "FR-255-AC-3", "FR-255-AC-4")]
#[test]
fn input_byte_limit_has_one_name_and_item_result_shape() {
    let wire: crate::spine::ObservationLimitsWire = serde_json::from_value(json!({
        "document_bytes": LIMIT
    }))
    .expect("the request member uses ObservationLimits' document_bytes field");
    let request_limits = crate::spine::ObservationLimitsWire::resolve(Some(&wire))
        .expect("the request limit resolves");

    let mut caller_limits = ObservationLimits::default();
    assert!(caller_limits.set_bound(Setting::ObservationInputBytes, LIMIT));

    let request_item = limited_item(request_limits);
    let caller_item = limited_item(caller_limits);
    assert_eq!(request_item, caller_item);
    assert!(request_item.get("diagnostic").is_none(), "{request_item}");

    let cause = &request_item["cause"];
    assert_eq!(cause["code"], "stage_limit_exceeded");
    assert_eq!(cause["kind"], "input-bytes-exceeded");
    assert_eq!(cause["limit"], "input-bytes-exceeded", "{request_item}");
    assert_eq!(cause["value"], LIMIT);
    assert_eq!(cause["bound"], LIMIT.to_string());
    assert_eq!(cause["setting"], "observation.input_bytes");
    let actual = cause["counter"]
        .as_str()
        .expect("the observed byte count is retained")
        .parse::<u64>()
        .expect("the byte count is a decimal integer");
    assert!(actual > LIMIT, "observed size: {actual}");
}

/// Trace: FR-255-AC-3, the setting name remains input_bytes while the request uses document_bytes.
#[trace("TC-106", "FR-255-AC-3")]
#[test]
fn the_request_field_maps_to_the_input_bytes_setting_name() {
    let refusal = serde_json::from_value::<crate::spine::ObservationLimitsWire>(json!({
        "input_bytes": LIMIT
    }))
    .err()
    .expect("the request shape does not use the setting suffix");
    assert!(refusal.to_string().contains("unknown field `input_bytes`"));

    assert!(Setting::from_name("observation.document_bytes").is_none());
    assert!(Setting::from_name("observation.input_bytes").is_some());
    let unknown = fsl_foundation::setting::parse_operands(["observation.document_bytes=10"])
        .expect_err("the retired setting name is unknown");
    assert!(
        unknown.to_string().contains("observation.input_bytes"),
        "{unknown}"
    );
}
