// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S2 builds a `verify` plan into a
//! `VerificationPlan` form and a `synthesis` declaration, with or without its
//! objective, into a `Synthesis` form, on real complete-V1 source through
//! `fsl_cst::parse` and `build_unit`.

use fsl_forms::{DeclarationForm, SynthesisDirectionForm, SynthesisForm, build_unit};
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
     maximize Pick satisfies { Pick <= 7 };";

/// The one declaration form S2 builds for `declaration`, rendered by its
/// `Debug` form, or the S1/S2 refusal rendered the same way.
fn built(declaration: &str) -> Result<String, String> {
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
    Ok(format!("{:?}", form.form()))
}

fn synthesis_form(declaration: &str) -> Result<SynthesisForm, String> {
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
    match form.form() {
        DeclarationForm::Synthesis(form) => Ok(form.as_ref().clone()),
        other => Err(format!("not a synthesis form: {other:?}")),
    }
}

/// FR-753-AC-1: `Release` builds one `VerificationPlan` form holding its
/// three steps, their methods and bounds, and `timely`'s `depends [safe]`.
#[trace("TC-2103", "FR-753-AC-1")]
#[trace("FSpec-TC-1601", "FSpec-FR-192-AC-8")]
#[test]
#[ignore = "QSL-2236: S2 VerificationPlan form for `verify` not implemented yet"]
fn release_builds_a_verification_plan_form_with_three_steps() {
    let form = built(RELEASE).unwrap_or_else(|refusal| panic!("S2 builds `verify`: {refusal}"));
    assert!(form.starts_with("VerificationPlan("), "{form}");
    for word in [
        "Release",
        "safe",
        "tested",
        "timely",
        "model_check",
        "model_based_test",
        "NoDoubleRefund",
        "RefundWithin",
        "500",
    ] {
        assert!(form.contains(word), "{word} in {form}");
    }
}

/// FR-754-AC-1: `Pick` builds one `Synthesis` form with its names and block.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-6")]
#[test]
fn pick_builds_a_synthesis_form() {
    let form =
        synthesis_form(PICK).unwrap_or_else(|refusal| panic!("S2 builds `synthesis`: {refusal}"));
    assert_eq!(form.name.name, "Pick");
    assert!(form.name.span.start < form.name.span.end);
    assert_eq!(form.grammar.name, "Digit");
    assert!(form.grammar.span.start < form.grammar.span.end);
    assert_eq!(form.domain.name, "Digit");
    assert!(form.domain.span.start < form.domain.span.end);
    assert!(form.objective.is_none());
    assert!(form.satisfies_span.start < form.satisfies_span.end);
    assert!(form.satisfies.spans.span(form.satisfies.spans.root()).is_some());
    assert!(format!("{:?}", form.satisfies.expression).contains("Multiply"));
}

/// FR-754-AC-1: `minimize Pick` parses at S1 and the `Synthesis` form holds
/// the objective's direction.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
#[test]
fn pick_with_minimize_builds_a_synthesis_form_holding_its_objective() {
    let form = synthesis_form(PICK_MIN)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let objective = form.objective.expect("minimize is retained");
    assert_eq!(objective.direction, SynthesisDirectionForm::Minimize);
    assert!(format!("{:?}", objective.expression.expression).contains("Pick"));
    assert!(objective.span.start < objective.span.end);
    assert!(objective.expression.spans.span(objective.expression.spans.root()).is_some());
}

/// FR-754-AC-1: `maximize Pick` parses and its objective direction is retained.
#[trace("TC-2104", "FR-754-AC-1")]
#[test]
fn pick_with_maximize_builds_a_synthesis_form_holding_its_objective() {
    let form = synthesis_form(PICK_MAX)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    let objective = form.objective.expect("maximize is retained");
    assert_eq!(objective.direction, SynthesisDirectionForm::Maximize);
}
