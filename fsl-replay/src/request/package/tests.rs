// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-640-AC-8: the replay recompile's clocks come from the proving
//! package's `temporal_bindings` rows.

use super::ReplayPackage;
use filament_contract_model::ClockConfiguration;
use ix_trace_rs::trace;

fn package(rows: &str) -> ReplayPackage {
    let bytes = format!(r#"{{"name":"p","temporal_bindings":{rows}}}"#);
    ReplayPackage::read(bytes.as_bytes(), u64::MAX).expect("JSON")
}

/// Each row's `clock_name` names a member whose value is the row's
/// `clock`; a row two declarations share gives one member, and a row bound
/// by a model binding name is derived from the model and gives none.
#[trace("TC-2182", "FR-640-AC-8")]
#[test]
fn the_rows_give_one_member_per_trace_subject_clock() {
    let clocks = package(
        r#"[
        {"declaration":"a","occurrence":0,"clock_name":"plant-ms",
         "clock":{"kind":"timestamped_event","timestamp_unit":"ms"}},
        {"declaration":"b","occurrence":0,"clock_name":"plant-ms",
         "clock":{"kind":"timestamped_event","timestamp_unit":"ms"}},
        {"declaration":"c","occurrence":0,"clock_name":"model-steps",
         "clock":{"kind":"event_position","sequence_authority":"m"}},
        {"declaration":"d","occurrence":0,"clock_name":"sample",
         "clock":{"kind":"fixed_sample","epoch":{"kind":"integer","decimal":"0"},
                  "period":{"kind":"rational","numerator":"1","denominator":"2"},"unit":"s"}}
        ]"#,
    )
    .clocks()
    .expect("well-formed rows");
    assert_eq!(
        clocks.get("plant-ms"),
        Some(&ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into()
        })
    );
    assert!(matches!(
        clocks.get("sample"),
        Some(ClockConfiguration::FixedSample { unit, .. }) if &**unit == "s"
    ));
    assert_eq!(clocks.get("model-steps"), None);
    assert_eq!(
        package("[]").clocks().expect("no rows"),
        crate::spine::RequestClocks::default()
    );
}

/// A row whose `clock` is not a configuration refuses at the row's pointer.
#[trace("FR-640-AC-8")]
#[test]
fn a_malformed_row_refuses_at_its_pointer() {
    let refusal = package(
        r#"[{"declaration":"a","occurrence":0,"clock_name":"plant-ms",
            "clock":{"kind":"sundial"}}]"#,
    )
    .clocks()
    .expect_err("an unknown kind");
    assert_eq!(
        refusal.to_string(),
        super::field_refusal(
            "/package/temporal_bindings/0/clock",
            super::PackageMemberCause::WrongKind
        )
        .to_string()
    );
}
