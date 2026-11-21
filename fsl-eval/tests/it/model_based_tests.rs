// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-752 (TC-2102): a `verify` plan retains the model-based method, clause,
//! domain and case bound on its named step.

use fsl_forms::{build_unit, DeclarationForm};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.value.complete/v1\";\n\
     verify Tests using p {\n\
       check t claim Bounded method model_based_test domain M::Counter bound 50;\n\
     }\n";

/// FR-752-AC-3: the named `model_based_test` step retains its claim, domain
/// and case bound in the checked verification plan.
#[trace("TC-2102", "FR-752-AC-3")]
#[test]
fn a_model_based_test_plan_preserves_step_clause_domain_and_case_bound() {
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
    let DeclarationForm::VerificationPlan(plan) = form.form() else {
        panic!("verification plan form: {:?}", form.form());
    };
    assert!(
        plan.source
            .contains("check t claim Bounded method model_based_test domain M::Counter bound 50;"),
        "the named model-based check and its case bound must be retained: {}",
        plan.source
    );
}
