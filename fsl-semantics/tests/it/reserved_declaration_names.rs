// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035-AC-7 through S1, S2 and S3: under `1-draft`, `helper`, `rec`,
//! `cast` and `Tuple` are ordinary identifiers, so a unit that names a
//! function, a `using` profile alias, a parameter, a `let` binder, a record,
//! its field, an enum member or a type alias by one of them is built and
//! checks (FSpec FR-339-AC-5, QSL-1649).

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

const SPELLINGS: [&str; 4] = ["helper", "rec", "cast", "Tuple"];

/// Parse, build and check `body` under `1-draft`, after the declared
/// `profile` lines.
fn checks(profile: &str, body: &str) {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{profile}{body}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "ordinary-identifier-spellings"),
        "ordinary.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|refusal| panic!("{text}: {refusal:?}"));
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: assembly: {refusal:?}"))
    .check(CheckMode::Linked, CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
}

const V: &str = "profile v = \"filament.value.complete/v1\";\n";

#[trace("TC-113", "FR-035-AC-7", "FSpec-TC-180", "FSpec-FR-339-AC-5")]
#[test]
fn a_function_named_by_the_spelling_is_declared_and_called() {
    for w in SPELLINGS {
        checks(
            V,
            &format!(
                "function {w} using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
                 function answer using v(x: Int[0, 9]): Int[0, 10] pure {{ {w}(x) }}"
            ),
        );
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn the_spelling_is_a_using_alias_parameter_and_let_binder() {
    for w in SPELLINGS {
        checks(
            &format!("profile {w} = \"filament.value.complete/v1\";\n"),
            &format!("function f using {w}(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}"),
        );
        checks(
            V,
            &format!(
                "function f using v({w}: Int[0, 9]): Int[0, 10] pure {{ let {w}2 = {w} in {w}2 + 1 }}\n\
                 function g using v(x: Int[0, 9]): Int[0, 10] pure {{ let {w} = x in {w} + 1 }}"
            ),
        );
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn the_spelling_names_a_record_field_enum_member_and_type_alias() {
    for w in SPELLINGS {
        checks(
            V,
            &format!(
                "record {w} {{ {w}: Int[0, 9]; }}\n\
                 enum E {{ a, {w} }}\n\
                 type {w}Alias = Int[0, 9];\n\
                 type A{w} = Int[0, 9];\n\
                 function f using v(x: Int[0, 9]): Int[0, 9] pure {{ x }}"
            ),
        );
    }
}
