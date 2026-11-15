// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 (TC-2103) and FR-754 (TC-2104): S2 builds a `verify` plan into a
//! `VerificationPlan` form and a `synthesis` declaration, with or without its
//! objective, into a `Synthesis` form, on real complete-V1 source through
//! `fsl_cst::parse` and `build_unit`.

use fsl_forms::build_unit;
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
#[ignore = "QSL-2235: S2 Synthesis form for `synthesis` not implemented yet"]
fn pick_builds_a_synthesis_form() {
    let form = built(PICK).unwrap_or_else(|refusal| panic!("S2 builds `synthesis`: {refusal}"));
    assert!(form.starts_with("Synthesis("), "{form}");
    for word in ["Pick", "Digit"] {
        assert!(form.contains(word), "{word} in {form}");
    }
}

/// FR-754-AC-1: `minimize Pick` parses at S1 and the `Synthesis` form holds
/// the objective's direction.
#[trace("TC-2104", "FR-754-AC-1")]
#[trace("FSpec-TC-1602", "FSpec-FR-194-AC-7")]
#[test]
#[ignore = "QSL-2235: synthesis objective (`minimize`/`maximize`) not implemented yet"]
fn pick_with_minimize_builds_a_synthesis_form_holding_its_objective() {
    let form = built(PICK_MIN)
        .unwrap_or_else(|refusal| panic!("S1 and S2 build the objective: {refusal}"));
    assert!(form.starts_with("Synthesis("), "{form}");
    assert!(form.contains("Minimize"), "{form}");
}
