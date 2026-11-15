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
