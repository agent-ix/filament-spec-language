// SPDX-License-Identifier: AGPL-3.0-or-later
//! Strict native run-result reader behavior owned by replay.

use fsl_replay::spine::{read_native_run_result, NativeRunReadError, NativeRunResult};
use serde_json::{json, Value};

fn valid_clause_result() -> Value {
    json!({
        "format": "native-run-result/2",
        "basis": "unavailable",
        "usage": {
            "admission": {"document_bytes": 0, "objects": 0, "values": 0},
            "evaluation_admissions": 0,
            "evaluation_consumed": []
        },
        "axes": {
            "activation": "unavailable",
            "execution": null,
            "truth": null,
            "settlement": "unavailable"
        },
        "disposition": {
            "stage": "admit",
            "category": "refusal",
            "refusal": {
                "code": "invalid_runtime_input",
                "cause": "missing-member",
                "fields": {"member": "version"}
            }
        }
    })
}

fn read(value: &Value) -> Result<NativeRunResult, NativeRunReadError> {
    read_native_run_result(&serde_json::to_vec(value).expect("valid JSON fixture"))
}

#[test]
fn reader_accepts_declared_v2_and_refuses_unsupported_and_foreign_members() {
    let valid = valid_clause_result();
    let decoded = read(&valid);
    match decoded {
        Ok(NativeRunResult::Clause(_)) => {}
        Ok(_) => panic!("the valid clause fixture selects another result variant"),
        Err(error) => panic!("the valid clause fixture was refused: {error:?}"),
    }

    for format in ["native-run-result/1", "native-run-result/99"] {
        let mut unsupported = valid.clone();
        unsupported["format"] = json!(format);
        assert!(matches!(
            read(&unsupported),
            Err(NativeRunReadError::UnsupportedFormat(actual)) if actual == format
        ));
    }

    let mut invalid_v2 = valid.clone();
    invalid_v2["basis"] = json!("not-a-basis");
    let invalid_error = match read(&invalid_v2) {
        Err(error) => error,
        Ok(_) => panic!("a `/2` result with an invalid basis was accepted"),
    };
    assert!(
        matches!(invalid_error, NativeRunReadError::MalformedMember { ref member, .. } if member == "basis"),
        "{invalid_error:?}"
    );

    for member in ["generated_source", "source_map", "coverage_producer"] {
        let mut injected = valid.clone();
        injected[member] = json!({"injected": true});
        assert!(matches!(
            read(&injected),
            Err(NativeRunReadError::MalformedMember { member: actual, .. }) if actual == member
        ));
    }
}
