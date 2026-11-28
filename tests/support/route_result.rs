// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-037: what every strict reader of a route's own result document
//! (`native-model-based-test-result/1`, `native-tl-lowering-result/1`,
//! `native-refinement-transfer-result/1`) does the same way, checked against
//! bytes the production writer produced.

use fsl_replay::spine::{
    read_native_run_result, Labelled, NativeRunLimits, NativeRunReadError, RouteResult,
};
use serde::Serialize;
use serde_json::{json, Value};

/// A reader of one route's result document.
pub type Reader<T> = fn(&[u8], NativeRunLimits) -> Result<RouteResult<T>, NativeRunReadError>;

/// A `native-run-result/2` clause-run document.
const CLAUSE_RUN: &str = include_str!("../fixtures/native-run-result-decisive.json");

/// The canonical bytes of `value`.
pub fn canonical(value: &Value) -> Vec<u8> {
    fsl_foundation::canonical_json::encode(value).expect("a JSON value is canonical")
}

/// The record `read` returns for `value`'s canonical bytes, which must write
/// back to the same bytes.
pub fn read_back<T: Labelled + Serialize>(value: &Value, read: Reader<T>) -> T {
    let bytes = canonical(value);
    let Ok(RouteResult::Record(document)) = read(&bytes, NativeRunLimits::UNBOUNDED) else {
        panic!("expected the record form of {value}");
    };
    let rewritten = fsl_foundation::canonical_json::encode(&document).expect("the record writes");
    assert_eq!(rewritten, bytes, "the record writes back to its own bytes");
    document
}

/// The refusals every route reader gives for the record document `value`
/// whose members are written under `member` (a root member name).
pub fn assert_route_refusals<T: Labelled>(
    value: &Value,
    label: &str,
    member: &str,
    read: Reader<T>,
) {
    let bytes = canonical(value);
    let unbounded = NativeRunLimits::UNBOUNDED;
    // The `native-run-result/2` reader refuses the route's label.
    assert!(
        matches!(
            read_native_run_result(&bytes),
            Err(NativeRunReadError::UnsupportedFormat(found)) if found == label
        ),
        "the /2 reader refuses {label}"
    );
    // The route's reader refuses the old label and any other label.
    for other in ["native-run/9", "native-run-result/1"] {
        let mut changed = value.clone();
        changed["format"] = json!(other);
        assert!(
            matches!(
                read(&canonical(&changed), unbounded),
                Err(NativeRunReadError::UnsupportedFormat(found)) if found == other
            ),
            "{other}"
        );
    }
    let mut old_label = value.clone();
    old_label["format"] = json!("native-run-result/2");
    assert!(
        matches!(
            read(&canonical(&old_label), unbounded),
            Err(NativeRunReadError::MalformedMember { .. } | NativeRunReadError::Invalid { .. })
        ),
        "a /2 label over the record's members is no /2 document"
    );
    // A clause-run document of `/2` is not a result of this route.
    let clause_run: Value = serde_json::from_str(CLAUSE_RUN).expect("the fixture is JSON");
    assert!(matches!(
        read(&canonical(&clause_run), unbounded),
        Err(NativeRunReadError::Invalid {
            member: "format",
            ..
        })
    ));
    // A `/2` command error is the other arm.
    let command_error = json!({
        "format": "native-run-result/2", "stage": "profile",
        "code": "unknown_edition", "message": "unknown edition in program.native",
        "details": {"phase": "profile", "code": "unknown_edition", "path": "program.native"},
        "basis": "unavailable"
    });
    let Ok(RouteResult::CommandError(error)) = read(&canonical(&command_error), unbounded) else {
        panic!("expected the command-error form");
    };
    assert_eq!(error.code(), "unknown_edition");
    // An unknown member is refused at the record, naming the member.
    let mut unknown = value.clone();
    unknown[member]["x"] = json!(1);
    let Err(NativeRunReadError::MalformedMember { member: at, source }) =
        read(&canonical(&unknown), unbounded)
    else {
        panic!("an unknown member is refused");
    };
    assert_eq!(at, member);
    assert!(source.to_string().contains("unknown field `x`"), "{source}");
    // Each bound is enforced before the record is read.
    let length = u64::try_from(bytes.len()).expect("a length");
    assert!(matches!(
        read(
            &bytes,
            NativeRunLimits {
                max_input_bytes: length - 1,
                max_working_bytes: u64::MAX,
            }
        ),
        Err(NativeRunReadError::Read(_))
    ));
    assert!(matches!(
        read(
            &bytes,
            NativeRunLimits {
                max_input_bytes: u64::MAX,
                max_working_bytes: 1,
            }
        ),
        Err(NativeRunReadError::WorkingBytes { .. })
    ));
}

/// The error of reading `value` with `member`'s `name` removed.
pub fn refusal_without<T: Labelled>(
    value: &Value,
    path: &[&str],
    name: &str,
    read: Reader<T>,
) -> NativeRunReadError {
    let mut changed = value.clone();
    let mut at = &mut changed;
    for step in path {
        at = &mut at[*step];
    }
    at.as_object_mut()
        .expect("an object")
        .remove(name)
        .unwrap_or_else(|| panic!("{name} is written"));
    match read(&canonical(&changed), NativeRunLimits::UNBOUNDED) {
        Err(error) => error,
        Ok(_) => panic!("{name} removed is refused"),
    }
}
