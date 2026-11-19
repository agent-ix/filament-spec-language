// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-500 and FR-501 item 4 through S1, S2 and S3: a unit with no
//! `profile` and a function with no `using` select no layer, and `Some(e)` is
//! the present-option constructor outside a refinement row.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

/// Parse, build, assemble and check `body` under `1-draft` after `profiles`;
/// `Ok` when it checks, else the refusal's debug text.
fn outcome(profiles: &str, body: &str) -> Result<(), String> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{profiles}{body}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "profile-defaults"),
        "profile-defaults.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).map_err(|refusal| format!("{refusal:?}"))?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?
    .check(CheckMode::Linked, CheckingLimits::default())
    .map(|_| ())
    .map_err(|refusals| format!("{refusals:?}"))
}

const V: &str = "profile v = \"filament.value.complete/v1\";\n";

/// FSpec FR-500-AC-2: a profile-less unit of one function checks.
#[trace("FSpec-TC-1001", "FSpec-FR-500-AC-2")]
#[test]
fn a_unit_with_no_profile_checks() {
    outcome("", "function f(): Boolean pure { true }").expect("checks");
}

/// FSpec FR-500-AC-3: a form of another layer refuses `unsupported_construct`
/// in a profile-less unit exactly as in a unit selecting only the value layer.
#[trace("FSpec-TC-1001", "FSpec-FR-500-AC-3")]
#[test]
#[ignore = "QSL-2161: the profile-less unit's layer refusal is not yet asserted equal to the value layer's; not implemented yet"]
fn a_profile_less_unit_refuses_another_layer_s_form_as_the_value_layer_does() {
    let body = "function f(c: Clock): Boolean pure { true }";
    let bare = outcome("", body).expect_err("refused");
    let value =
        outcome(V, &body.replace("function f(", "function f using v(")).expect_err("refused");
    for refusal in [&bare, &value] {
        assert!(refusal.contains("UnsupportedConstruct"), "{refusal}");
    }
}

/// FSpec FR-500-AC-4: a function and a predicate without `using` check in a
/// unit that selects the value layer.
#[trace("FSpec-TC-1001", "FSpec-FR-500-AC-4")]
#[test]
fn a_function_and_predicate_without_using_check() {
    outcome(
        V,
        "function g(x: Integer): Integer pure { x + 1 }\npredicate p(x: Integer): Boolean { x > 0 }",
    )
    .expect("checks");
}

/// FSpec FR-375-AC-17 and FR-501-AC-4: outside a refinement row `Some(e)` is
/// the present-option constructor, whether or not a function named `Some` is
/// declared.
#[trace(
    "FSpec-TC-310",
    "FSpec-FR-375-AC-17",
    "FSpec-TC-1005",
    "FSpec-FR-501-AC-4"
)]
#[test]
fn some_is_the_constructor_outside_a_refinement_row() {
    outcome(
        V,
        "function n using v(b: Integer): Boolean pure { present(Some(b)) }",
    )
    .expect("checks");
    outcome(
        V,
        "function Some using v(x: Integer): Integer pure { x }\n\
         function h using v(): Option<Integer> pure { Some(1) }",
    )
    .expect("Some(1) is the constructor, not the declared function");
    let wrong = outcome(
        V,
        "function w using v(b: Integer): Option<Integer> pure { Some(b, b) }",
    )
    .expect_err("refused");
    assert!(wrong.contains("IllTyped"), "{wrong}");
}
