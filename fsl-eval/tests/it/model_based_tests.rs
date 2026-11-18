// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 (TC-2102): model-based testing is requested by a `verify` plan step
//! whose method is `model_based_test`; the step reaches the generator with
//! its clause, domain and case bound. The generation and shrinking
//! properties (FR-752-AC-1, AC-2) are asserted through the generator's entry
//! point, which does not exist yet; this stub asserts the request surface
//! that carries them to it.

use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.value.complete/v1\";\n\
     verify Tests using p {\n\
       check t claim Bounded method model_based_test domain M::Counter bound 50;\n\
     }\n";

/// FR-752-AC-3: the `model_based_test` step builds with its clause, domain
/// and its case bound 50, so the run it requests is bounded by 50 cases.
#[trace("TC-2102", "FR-752-AC-3")]
#[trace("FSpec-TC-1600", "FSpec-FR-190-AC-7")]
#[test]
#[ignore = "QSL-2237: model-based test generation requested by a `verify` step not implemented yet"]
fn a_model_based_test_step_carries_its_clause_domain_and_case_bound() {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|refusal| panic!("S2: {refusal:?}"));
    let [form] = unit.forms() else {
        panic!("one form: {:?}", unit.forms());
    };
    let form = format!("{:?}", form.form());
    assert!(form.starts_with("VerificationPlan("), "{form}");
    for word in ["model_based_test", "Bounded", "Counter", "50"] {
        assert!(form.contains(word), "{word} in {form}");
    }
}

/// FSpec FR-190 "Operation coverage" over the three-operation counter: `inc`
/// raises `value` in `Int[0, 5]` while `value < 5`, `reset` is enabled at
/// `value = 5` and `jam` at `value > 5`, which no state of the domain holds.
const COVERAGE_MODEL: [(&str, &str); 3] = [
    ("inc", "self.value < 5"),
    ("reset", "self.value = 5"),
    ("jam", "self.value > 5"),
];

/// The `operations` entries and `coverage` summary of a model-based test run
/// over [`COVERAGE_MODEL`] with `max_trace_length`, seed 7 and `max_cases`
/// 100. `fsl-eval` has no model-based test entry point yet, so no record
/// carries operation coverage and this returns `None`; the generator's
/// result record replaces it.
fn generated_coverage(max_trace_length: u64) -> Option<serde_json::Value> {
    let _ = (COVERAGE_MODEL, max_trace_length);
    None
}

/// FR-752-AC-4 (FSpec FR-190-AC-11): every operation has one entry in
/// declaration order; the unreachable `jam` is reported uncovered with
/// reason `unreachable`, never covered.
#[trace("TC-2952", "FR-752-AC-4")]
#[trace("FSpec-TC-2002", "FSpec-FR-190-AC-11")]
#[test]
#[ignore = "QSL-2237: operation coverage in the model-based test result not implemented yet"]
fn an_unreachable_operation_is_reported_uncovered_with_its_reason() {
    let record = generated_coverage(8).expect("the run's result record carries `operations`");
    let entries = record["operations"].as_array().expect("one entry per operation");
    let names: Vec<&str> = entries
        .iter()
        .map(|entry| entry["operation"]["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(names, ["inc", "reset", "jam"]);
    for entry in &entries[..2] {
        assert_eq!(entry["status"], "covered", "{entry}");
        assert!(entry["first_case"].is_u64(), "{entry}");
    }
    assert_eq!(entries[2]["status"], "uncovered");
    assert_eq!(entries[2]["reason"], "unreachable");
    assert_eq!(record["coverage"]["status"], "partial");
}

/// FR-752-AC-4 (FSpec FR-190-AC-12): below the bound that reaches
/// `value = 5`, `reset` is uncovered within the bound, never unreachable.
#[trace("TC-2952", "FR-752-AC-4")]
#[trace("FSpec-TC-2002", "FSpec-FR-190-AC-12")]
#[test]
#[ignore = "QSL-2237: bounded operation coverage reasons not implemented yet"]
fn an_operation_beyond_the_trace_bound_is_uncovered_within_the_bound() {
    let record = generated_coverage(3).expect("the run's result record carries `operations`");
    let reset = record["operations"]
        .as_array()
        .and_then(|entries| entries.iter().find(|entry| entry["operation"]["name"] == "reset"))
        .expect("an entry for `reset`");
    assert_eq!(reset["status"], "uncovered", "{reset}");
    assert_eq!(reset["reason"], "unreachable-within-bound", "{reset}");
}

/// FR-752-AC-4 (FSpec FR-190-AC-13): coverage changes no verdict and is
/// deterministic.
#[trace("TC-2952", "FR-752-AC-4")]
#[trace("FSpec-TC-2002", "FSpec-FR-190-AC-13")]
#[test]
#[ignore = "QSL-2237: operation coverage determinism and verdict independence not implemented yet"]
fn operation_coverage_is_deterministic_and_settles_no_verdict() {
    let first = generated_coverage(8).expect("the run's result record carries `operations`");
    let second = generated_coverage(8).expect("the run's result record carries `operations`");
    assert_eq!(first["operations"], second["operations"]);
    assert_eq!(first["evidence"], "tested", "{first}");
    assert!(first.get("verdict").is_none_or(serde_json::Value::is_null), "{first}");
}
