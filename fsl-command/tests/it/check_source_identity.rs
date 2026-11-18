// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-866 "The checked source's identity" (TC-2977): `check_outcome` names
//! the checked source by authority `filament` and the path as given, on the
//! parse-refusal path and past it, so its refusal equals the library's.

use filament_exact::Cancel;
use fsl_command::command::{self, CheckInputs};
use fsl_replay::spine::{parse, ParseRequest, SpineLimits};
use fsl_replay::{OutcomeDocument, SourceIdentity};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const MALFORMED: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function f using v(: Boolean pure { true }\n";

const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function f using v(): Boolean pure { 7 }\n";

fn document(outcome: &OutcomeDocument) -> Value {
    serde_json::from_slice(&outcome.to_bytes().unwrap()).unwrap()
}

fn assert_named_by_path(outcome: &command::CheckOutcome, path: &str) {
    let expected = json!({"authority": "filament", "identity": path});
    let value = document(outcome.document());
    assert_eq!(value["category"], "refusal", "{value}");
    let diagnostics = value["diagnostics"].as_array().expect("diagnostics");
    assert!(!diagnostics.is_empty(), "{value}");
    for diagnostic in diagnostics {
        assert_eq!(diagnostic["locus"]["source"], expected, "{diagnostic}");
    }
    let text = value.to_string();
    assert!(!text.contains("fsl-command"), "{text}");
    assert!(!text.contains("check-source"), "{text}");
    assert!(outcome
        .sources()
        .as_slice()
        .iter()
        .all(|source| source.identity() == &SourceIdentity::new("filament", path)));
}

/// FR-866-AC-6: the malformed source's refusal names the supplied file and
/// is byte-equal to the library parse refusal under that identity; an S3
/// refusal names the same identity.
#[test]
#[trace("TC-2977", "FR-866-AC-6")]
#[ignore = "QSL-2423: check_outcome names the checked source fsl-command/check-source instead of filament and the path as given"]
fn a_check_refusal_names_the_supplied_file() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("malformed.fsl");
    std::fs::write(&file, MALFORMED).unwrap();
    let path = file.to_str().expect("a UTF-8 temporary path").to_owned();

    let outcome = command::check_outcome(&file, &CheckInputs::default());
    assert_named_by_path(&outcome, &path);

    let source = SourceIdentity::new("filament", &path);
    let failure = parse(
        &ParseRequest {
            source: &source,
            path: &path,
            bytes: MALFORMED.as_bytes(),
        },
        SpineLimits::default().source,
        &Cancel::new(),
    )
    .expect_err("the malformed source does not parse");
    let library = OutcomeDocument::from_check(&Err(failure));
    assert_eq!(
        outcome.document().to_bytes().unwrap(),
        library.to_bytes().unwrap()
    );

    let file = directory.path().join("ill-typed.fsl");
    std::fs::write(&file, ILL_TYPED).unwrap();
    let path = file.to_str().expect("a UTF-8 temporary path").to_owned();
    assert_named_by_path(
        &command::check_outcome(&file, &CheckInputs::default()),
        &path,
    );
}
