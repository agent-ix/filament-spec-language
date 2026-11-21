// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-680: accepting and ill-typed `.native` pairs for indexing, tuple
//! position reads, source maps, `Some`, quantity scaling and the equality-sum
//! rule, and the type/operator matrix. Accepted units compile through S4;
//! refused units retain their exact S3 code and cause.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_replay::{compile_package, CompileUnit, ReplayLimits, StageLimits};
use fsl_semantics::check::{AssemblyRefusal, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

fn assemble(name: &str, bytes: &[u8]) -> Result<PackageDeclarations, AssemblyRefusal> {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("fsl-conformance", name),
        name,
        bytes,
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{name}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "conformance".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
}

/// The accepting case checks and the ill-typed case gives one refusal with
/// `code` and `cause`.
fn assert_pair(accepted: (&str, &[u8]), refused: (&str, &[u8]), code: Code, cause: &str) {
    compile_package(
        CompileUnit {
            source: SourceIdentity::new("fsl-conformance", accepted.0),
            package_name: "conformance".to_owned(),
            path: accepted.0,
            bytes: accepted.1,
        },
        std::iter::empty::<&[u8]>(),
        &fsl_replay::spine::DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("{} compiles through S4: {refusal:?}", accepted.0));
    // A type written in a signature is refused when the assembler resolves it.
    let assembled = match assemble(refused.0, refused.1) {
        Err(refusal) => {
            let [error] = refusal.errors.as_slice() else {
                panic!("{}: one typed assembly error: {refusal:?}", refused.0);
            };
            assert_eq!(error.cause.code(), code, "{error:?}");
            assert_eq!(error.cause.catalog_code().cause(), cause, "{error:?}");
            return;
        }
        Ok(assembled) => assembled,
    };
    let refusals = assembled
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .expect_err("the ill-typed case is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("{}: one typed refusal: {refusals:?}", refused.0);
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

macro_rules! case {
    ($file:expr) => {
        (
            $file,
            include_bytes!(concat!("../../cases/", $file)).as_slice(),
        )
    };
}

#[trace("TC-1820", "FR-680-AC-1", "FSpec-FR-144-AC-31", "FSpec-FR-144-AC-32")]
#[test]
fn index_pair() {
    assert_pair(
        case!("index-accepted.native"),
        case!("index-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1821", "FR-680-AC-2", "FSpec-FR-143-AC-22")]
#[test]
fn tuple_position_pair() {
    assert_pair(
        case!("tuple-position-accepted.native"),
        case!("tuple-position-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1822", "FR-680-AC-3", "FSpec-FR-144-AC-33", "FSpec-FR-149-AC-28")]
#[test]
fn map_pair() {
    assert_pair(
        case!("map-accepted.native"),
        case!("map-ill-typed.native"),
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_pair() {
    assert_pair(
        case!("some-accepted.native"),
        case!("some-ill-typed.native"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1824", "FR-680-AC-5", "FSpec-FR-142-AC-41", "FSpec-FR-142-AC-42")]
#[test]
fn scale_pair() {
    assert_pair(
        case!("scale-accepted.native"),
        case!("scale-ill-typed.native"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1825", "FR-680-AC-6", "FSpec-FR-149-AC-29")]
#[test]
fn eq_sum_pair() {
    assert_pair(
        case!("eq-sum-accepted.native"),
        case!("eq-sum-ill-typed.native"),
        Code::UndefinedExpression,
        "unproved-range",
    );
}

macro_rules! matrix_pairs {
    ($( $test:ident: $row:literal, $stem:literal => $code:expr, $cause:literal, $requirement:literal; )+) => {
        $(
            #[test]
            #[trace($requirement)]
            fn $test() {
                assert_pair(
                    case!(concat!("type-operators/", $stem, "-accepted.native")),
                    case!(concat!("type-operators/", $stem, "-ill-typed.native")),
                    $code,
                    $cause,
                );
            }
        )+

        #[test]
        fn matrix_case_files_have_executable_expectations() {
            use std::collections::BTreeSet;
            let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("cases/type-operators");
            let actual: BTreeSet<String> = std::fs::read_dir(directory)
                .expect("operator matrix case directory")
                .map(|entry| entry.expect("operator case entry").path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "native"))
                .map(|path| path.file_name().expect("case file name").to_string_lossy().into_owned())
                .collect();
            let expected: BTreeSet<String> = [$(
                concat!($stem, "-accepted.native"),
                concat!($stem, "-ill-typed.native"),
            )+].into_iter().map(str::to_owned).collect();
            assert_eq!(actual, expected, "every matrix source has an executable expectation");
            let rows: BTreeSet<&str> = [$($row,)+].into_iter().collect();
            assert_eq!(rows, BTreeSet::from([
                "boolean", "integer", "rational", "decimal", "float", "quantity",
                "text", "native", "enum", "option", "composite", "collection", "reference",
            ]));
        }
    };
}

matrix_pairs! {
    matrix_boolean_connectives: "boolean", "boolean-connectives" => Code::IllTyped, "type-mismatch", "FSpec-FR-146";
    matrix_boolean_equality: "boolean", "boolean-equality" => Code::IllTyped, "operator-ineligible", "FSpec-FR-149";
    matrix_integer_arithmetic: "integer", "integer-arithmetic" => Code::UndefinedExpression, "unproved-range", "FSpec-FR-146";
    matrix_integer_slash: "integer", "integer-slash" => Code::IllTyped, "type-mismatch", "FSpec-FR-146";
    matrix_integer_division: "integer", "integer-division" => Code::UndefinedExpression, "unproved-nonzero", "FSpec-FR-147";
    matrix_integer_modulo: "integer", "integer-modulo" => Code::UndefinedExpression, "unproved-nonzero", "FSpec-FR-147";
    matrix_integer_negation: "integer", "integer-negation" => Code::UndefinedExpression, "unproved-range", "FSpec-FR-146";
    matrix_integer_equality: "integer", "integer-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_integer_ordering: "integer", "integer-ordering" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_rational_arithmetic: "rational", "rational-arithmetic" => Code::UndefinedExpression, "unproved-range", "FSpec-FR-146";
    matrix_rational_division: "rational", "rational-division" => Code::IllTyped, "type-mismatch", "FSpec-FR-146";
    matrix_rational_negation: "rational", "rational-negation" => Code::UndefinedExpression, "unproved-range", "FSpec-FR-146";
    matrix_rational_equality: "rational", "rational-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_rational_ordering: "rational", "rational-ordering" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_decimal_arithmetic: "decimal", "decimal-arithmetic" => Code::IllTyped, "type-mismatch", "FSpec-FR-140";
    matrix_decimal_division: "decimal", "decimal-division" => Code::UndefinedExpression, "unproved-nonzero", "FSpec-FR-140";
    matrix_decimal_negation: "decimal", "decimal-negation" => Code::IllTyped, "type-mismatch", "FSpec-FR-140";
    matrix_decimal_equality: "decimal", "decimal-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_decimal_ordering: "decimal", "decimal-ordering" => Code::IllTyped, "type-mismatch", "FSpec-FR-140";
    matrix_float_arithmetic: "float", "float-arithmetic" => Code::IllTyped, "type-mismatch", "FSpec-FR-148";
    matrix_float_division: "float", "float-division" => Code::IllTyped, "type-mismatch", "FSpec-FR-148";
    matrix_float_negation: "float", "float-negation" => Code::IllTyped, "type-mismatch", "FSpec-FR-148";
    matrix_float_equality: "float", "float-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-148";
    matrix_float_ordering: "float", "float-ordering" => Code::IllTyped, "type-mismatch", "FSpec-FR-148";
    matrix_quantity_arithmetic: "quantity", "quantity-arithmetic" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_division: "quantity", "quantity-division" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_negation: "quantity", "quantity-negation" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_power: "quantity", "quantity-power" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_equality: "quantity", "quantity-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_ordering: "quantity", "quantity-ordering" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_text_equality: "text", "text-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_text_ordering: "text", "text-ordering" => Code::IllTyped, "operator-ineligible", "FSpec-FR-141";
    matrix_text_size: "text", "text-size" => Code::IllTyped, "type-mismatch", "FSpec-FR-141";
    matrix_native_equality: "native", "native-equality" => Code::IllTyped, "operator-ineligible", "FSpec-FR-149";
    matrix_native_kind_separation: "native", "native-kind-separation" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_enum_equality: "enum", "enum-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-141";
    matrix_enum_ordering: "enum", "enum-ordering" => Code::IllTyped, "operator-ineligible", "FSpec-FR-141";
    matrix_option_presence: "option", "option-presence" => Code::UndefinedExpression, "unproved-presence", "FSpec-FR-146";
    matrix_option_equality: "option", "option-equality" => Code::IllTyped, "operator-ineligible", "FSpec-FR-149";
    matrix_composite_equality: "composite", "composite-equality" => Code::IllTyped, "operator-ineligible", "FSpec-FR-149";
    matrix_collection_equality: "collection", "collection-equality" => Code::IllTyped, "type-mismatch", "FSpec-FR-149";
    matrix_collection_size: "collection", "collection-size" => Code::IllTyped, "type-mismatch", "FSpec-FR-145";
    matrix_collection_contains: "collection", "collection-contains" => Code::IllTyped, "type-mismatch", "FSpec-FR-145";
    matrix_reference_equality: "reference", "reference-equality" => Code::IllTyped, "operator-ineligible", "FSpec-FR-149";
    matrix_quantity_scaling: "quantity", "quantity-scaling" => Code::IllTyped, "type-mismatch", "FSpec-FR-142";
    matrix_quantity_unit_resolution: "quantity", "quantity-unit-resolution" => Code::MissingDeclaration, "missing-name", "FSpec-FR-142";
    matrix_collection_arithmetic_refused: "collection", "collection-arithmetic-refused" => Code::IllTyped, "type-mismatch", "FSpec-FR-145";
}
