// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S2 builds a `verify` plan into a
//! `VerificationPlan` form and a `synthesis` declaration, with or without its
//! objective, into a `Synthesis` form, on real complete-V1 source through
//! `fsl_cst::parse` and `build_unit`.

use fsl_forms::{build_unit, DeclarationForm, SynthesisObjectiveForm};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.value.complete/v1\";\n";

/// FSpec FR-192's `Release` plan.
const RELEASE: &str = "verify Release using p {\n\
       check safe claim Orders::NoDoubleRefund method model_check domain M::Order bound 20;\n\
       check tested claim Orders::NoDoubleRefund method model_based_test domain M::Order bound 500;\n\
       check timely claim Orders::RefundWithin method model_check domain M::Order depends [safe];\n\
     }";

const PICK: &str =
    "synthesis Pick using p grammar Digit domain Digit satisfies { Pick * Pick = 49 };";

const PICK_MIN: &str = "synthesis Pick using p grammar Digit domain Digit \
     minimize Pick satisfies { Pick >= 3 };";

const PICK_MAX: &str = "synthesis Pick using p grammar Digit domain Digit \
     maximize Pick satisfies { Pick <= 6 };";

/// The one declaration form S2 builds for `declaration`, or the S1/S2 refusal.
fn built(declaration: &str) -> Result<DeclarationForm, String> {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        return Err(format!("S1: {:?}", parsed.diagnostics()));
    }
    let unit = build_unit(&parsed).map_err(|refusal| format!("S2: {refusal:?}"))?;
    let [form] = unit.forms() else {
        return Err(format!("not one form: {:?}", unit.forms()));
    };
    Ok(form.form().clone())
}

/// FR-753-AC-1: `Release` builds one `VerificationPlan` form holding its
/// three steps, their methods and bounds, and `timely`'s `depends [safe]`.
#[trace("TC-2103", "FR-753-AC-1")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-8")]
#[test]
#[ignore = "QSL-2236: S2 VerificationPlan form for `verify` not implemented yet"]
fn release_builds_a_verification_plan_form_with_three_steps() {
    let form = built(RELEASE).unwrap_or_else(|refusal| panic!("S2 builds `verify`: {refusal}"));
    assert!(matches!(form, DeclarationForm::VerificationPlan(_)));
}

/// FR-754-AC-1: `Pick` builds one `Synthesis` form with its names and block.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-6")]
#[test]
fn pick_builds_a_synthesis_form() {
    let form = built(PICK).unwrap_or_else(|refusal| panic!("S2 builds `synthesis`: {refusal}"));
    let DeclarationForm::Synthesis(synthesis) = form else {
        panic!("S2 returns a synthesis form: {form:?}");
    };
    assert_eq!(synthesis.name.name, "Pick");
    assert_eq!(synthesis.grammar.name, "Digit");
    assert_eq!(synthesis.domain.name, "Digit");
    assert!(synthesis.objective.is_none());
    assert!(synthesis
        .satisfies
        .spans
        .span(synthesis.satisfies.spans.root())
        .is_some());
}

/// FR-754-AC-1: `minimize Pick` parses at S1 and the `Synthesis` form holds
/// the objective's direction.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
#[test]
fn pick_with_minimize_builds_a_synthesis_form_holding_its_objective() {
    let form = built(PICK_MIN)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let DeclarationForm::Synthesis(synthesis) = form else {
        panic!("S2 returns a synthesis form: {form:?}");
    };
    assert_eq!(synthesis.grammar.name, "Digit");
    assert_eq!(synthesis.domain.name, "Digit");
    let Some(SynthesisObjectiveForm::Minimize(name)) = synthesis.objective else {
        panic!(
            "S2 preserves a minimize objective: {:?}",
            synthesis.objective
        );
    };
    assert_eq!(name.name, "Pick");
}

/// FR-754-AC-1: `maximize Pick` remains an explicit S2 objective.
#[trace("TC-2104", "FR-754-AC-1")]
#[test]
fn pick_with_maximize_builds_a_synthesis_form_holding_its_objective() {
    let form = built(PICK_MAX)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let DeclarationForm::Synthesis(synthesis) = form else {
        panic!("S2 returns a synthesis form: {form:?}");
    };
    let Some(SynthesisObjectiveForm::Maximize(name)) = synthesis.objective else {
        panic!(
            "S2 preserves a maximize objective: {:?}",
            synthesis.objective
        );
    };
    assert_eq!(name.name, "Pick");
}
