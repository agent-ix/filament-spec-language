// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2241: text-to-number conversion type checking and checked lowering.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{AssemblyRefusal, CheckRefusal, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

const UNITS: &str = "dimension T;\nunit s : T = rational(1, 1);\nunit ms : T = rational(1, 1000) * s;\n";

fn unit_text(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}\n"
    )
}

fn assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = unit_text(declarations);
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "text-to-number"),
        "text-to-number.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    assemble(declarations)
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
        .check(CheckMode::Linked, CheckingLimits::default())
}

fn assert_refused(declarations: &str, code: Code, cause: &str) {
    let refusals = check(declarations).expect_err("the expression is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

#[trace("TC-1196", "FR-093-AC-35")]
#[test]
fn text_and_string_conversions_build_their_typed_operations() {
    let graph = check(&format!(
        "{UNITS}\
         function ti using v(t: Text[0, 8; nfc]): Int[0, 99] pure {{ convert<Int[0, 99]>(t) }}\n\
         function td using v(t: Text[0, 8; nfc]): Decimal[0, 1000; 0, 2; nearest-even] pure {{ convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }}\n\
         function tr using v(t: Text[0, 8; nfc]): Rational[-9, 9; 1, 9] pure {{ convert<Rational[-9, 9; 1, 9]>(t) }}\n\
         function tf32 using v(t: Text[0, 8; nfc]): Float32[nearest-even] pure {{ convert<Float32[nearest-even]>(t) }}\n\
         function tf64 using v(t: Text[0, 8; nfc]): Float64[exact] pure {{ convert<Float64[exact]>(t) }}\n\
         function tq using v(t: Text[0, 8; nfc]): Quantity<ms> pure {{ convert<Quantity<ms>>(t) }}\n\
         function si using v(s: String): Int[0, 99] pure {{ convert<Int[0, 99]>(s) }}\n\
         function sd using v(s: String): Decimal[0, 1000; 0, 2; nearest-even] pure {{ convert<Decimal[0, 1000; 0, 2; nearest-even]>(s) }}\n\
         function sr using v(s: String): Rational[-9, 9; 1, 9] pure {{ convert<Rational[-9, 9; 1, 9]>(s) }}\n\
         function sf32 using v(s: String): Float32[nearest-even] pure {{ convert<Float32[nearest-even]>(s) }}\n\
         function sf64 using v(s: String): Float64[exact] pure {{ convert<Float64[exact]>(s) }}\n\
         function sq using v(s: String): Quantity<ms> pure {{ convert<Quantity<ms>>(s) }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("text number conversions check: {refusals:?}"));
    let rendered = format!("{graph:?}");
    for identity in [
        "filament.op.text.to_integer",
        "filament.op.text.to_decimal",
        "filament.op.text.to_rational",
        "filament.op.text.to_float32",
        "filament.op.text.to_float64",
        "filament.op.text.to_quantity",
        "filament.op.string.to_integer",
        "filament.op.string.to_decimal",
        "filament.op.string.to_rational",
        "filament.op.string.to_float32",
        "filament.op.string.to_float64",
        "filament.op.string.to_quantity",
    ] {
        assert!(rendered.contains(identity), "missing {identity}: {rendered}");
    }
}

#[trace("TC-1196", "FR-093-AC-47")]
#[test]
fn text_to_non_numeric_targets_is_refused_as_operator_ineligible() {
    assert_refused(
        "function f using v(t: Text[0, 8; nfc]): Boolean pure { convert<Boolean>(t) }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { convert<Text[0, 8; nfc]>(t) }",
        Code::IllTyped,
        "operator-ineligible",
    );
}
