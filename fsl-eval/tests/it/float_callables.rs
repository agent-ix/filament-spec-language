// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397 "Intrinsics" and "Comparison": the `fma`, `numericEqual`,
//! `totalOrder` and `bitIdentical` calls and the float ordering operators,
//! evaluated from source through the checked package.

use filament_eval::FamilyOutcome;
use filament_exact::{
    FloatType, IeeeFlag, IeeeFlags, IeeeValue, IeeeWidth, LimitKind, Meter, OptionValue, Outcome,
    RoundingMode, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use filament_semantic_value::loss::ValueLoss;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::{emit_checked, CheckedPackage, EmitLimits};
use fsl_semantics::check::{CheckCause, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use ix_trace_rs::trace;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const IEEE: &str = "filament::value::ieee::";

/// Compiles `function f using v(): <result> pure { <body> }` and evaluates it.
fn run(result: &str, body: &str) -> (Outcome<Value>, Vec<ValueLoss>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function f using v(): {result} pure {{ {body} }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the floating type");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    (
        outcome,
        evaluation
            .losses
            .into_iter()
            .map(|each| each.loss)
            .collect(),
    )
}

fn boolean(body: &str) -> bool {
    match run("Boolean", body).0 {
        Outcome::Completed(Value::Boolean(value)) => value,
        other => panic!("{body}: a completed Boolean, not {other:?}"),
    }
}

/// Source for a float constant of `width` (32 or 64) with `bits`.
fn constant(width: u32, bits: u64) -> String {
    if width == 32 {
        format!("float32(bits: {bits:#010x})")
    } else {
        format!("float64(bits: {bits:#018x})")
    }
}

fn call(name: &str, width: u32, left: u64, right: u64) -> bool {
    boolean(&format!(
        "{IEEE}{name}({}, {})",
        constant(width, left),
        constant(width, right)
    ))
}

fn operator(width: u32, operator: &str, left: u64, right: u64) -> bool {
    boolean(&format!(
        "{} {operator} {}",
        constant(width, left),
        constant(width, right)
    ))
}

fn float(width: u32, body: &str) -> (u64, Vec<ValueLoss>) {
    let result = format!("Float{width}[nearest-even]");
    match run(&result, body) {
        (Outcome::Completed(Value::Float(value)), losses) => (value.bits(), losses),
        other => panic!("{body}: a completed float, not {other:?}"),
    }
}

fn flags(losses: &[ValueLoss]) -> Vec<IeeeFlag> {
    losses
        .iter()
        .filter_map(|loss| match loss {
            ValueLoss::IeeeFlags(flags) => Some(*flags),
            _ => None,
        })
        .flat_map(IeeeFlags::iter)
        .collect()
}

fn checked_source(result: &str, body: &str) -> CheckedPackage {
    checked_function("", result, body)
}

fn checked_function(parameters: &str, result: &str, body: &str) -> CheckedPackage {
    checked_module(&format!(
        "function f using v({parameters}): {result} pure {{ {body} }}\n"
    ))
}

fn checked_module(items: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {items}"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "fr397-bit-literal"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fr397-bit-literal".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler accepts the unit");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the IEEE profile admits"),
    );
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{items}: {refusal:?}")),
    )
}

fn assert_ieee_operation_has_selected_law(wire: &serde_json::Value, identity: &str) {
    let selections: Vec<&serde_json::Value> = wire["lock"]["definition_selections"]
        .as_array()
        .expect("definition selections are emitted")
        .iter()
        .filter(|selection| selection["identity"] == "filament.value.ieee754-2019-default/v1")
        .collect();
    let [selected] = selections.as_slice() else {
        panic!("one IEEE profile definition selection is emitted: {selections:?}");
    };
    let node = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes are emitted")
        .iter()
        .find(|node| node["body"]["operation"]["identity"] == identity)
        .unwrap_or_else(|| panic!("the operation {identity} is emitted: {wire}"));
    assert_eq!(
        node["body"]["operation"]["laws"],
        serde_json::json!([{"role": "ieee_profile", "definition": selected}]),
        "{identity} carries the selected IEEE profile"
    );
}

fn emitted_wire(package: &CheckedPackage) -> serde_json::Value {
    let emission =
        emit_checked(package, EmitLimits::new(16 * 1024 * 1024)).expect("the package emits");
    serde_json::from_slice(emission.package().bytes()).expect("the package JSON parses")
}

fn literal_nodes(package: &CheckedPackage) -> Vec<serde_json::Value> {
    let wire = emitted_wire(package);
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the semantic graph has nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "literal")
        .cloned()
        .collect()
}

fn assert_literal_float_type(
    wire: &serde_json::Value,
    literal: &serde_json::Value,
    width: &str,
    rounding: &str,
) {
    let type_id = &literal["semantic_type"];
    let ty = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes are emitted")
        .iter()
        .find(|node| node["node_id"] == *type_id)
        .expect("literal type node is emitted");
    assert_eq!(ty["semantic_form"], "float_rounding");
    assert_eq!(ty["semantic_type"]["path"][1], width);
    assert_eq!(ty["body"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(ty["body"]["members"][0]["name"], "rounding");
    assert_eq!(ty["body"]["members"][0]["value"]["value"], rounding);
}

fn evaluate_nullary(package: &CheckedPackage) -> (Outcome<Value>, Vec<ValueLoss>) {
    evaluate_named(package, "f")
}

fn evaluate_named(package: &CheckedPackage, name: &str) -> (Outcome<Value>, Vec<ValueLoss>) {
    evaluate_named_with_args(package, name, Vec::new())
}

fn evaluate_named_with_args(
    package: &CheckedPackage,
    name: &str,
    arguments: Vec<Value>,
) -> (Outcome<Value>, Vec<ValueLoss>) {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("function name is valid"),
            arguments,
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the checked function runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!(
            "the evaluator returns a kernel outcome: {:?}",
            evaluation.outcome
        );
    };
    (
        outcome,
        evaluation
            .losses
            .into_iter()
            .map(|loss| loss.loss)
            .collect(),
    )
}

/// FR-397-AC-10 (TC-1091): bit constructors inherit expected types, emit
/// exact literal payloads, and evaluate special payloads without flags.
#[trace("TC-1091", "FR-397-AC-10")]
#[test]
fn bit_constructors_keep_expected_types_and_exact_payloads() {
    let one64 = checked_source("Float64[nearest-even]", "float64(bits: 0x3ff0000000000000)");
    let literals64 = literal_nodes(&one64);
    let [literal] = literals64.as_slice() else {
        panic!("the constructor lowers to exactly one literal");
    };
    assert_literal_float_type(&emitted_wire(&one64), literal, "float64", "nearest-even");
    assert_eq!(literal["body"]["value_kind"], "float64_bits");
    assert_eq!(literal["body"]["value"], "3ff0000000000000");
    assert!(matches!(
        evaluate_nullary(&one64).0,
        Outcome::Completed(Value::Float(value)) if value.bits() == 0x3ff0_0000_0000_0000
    ));

    let one32 = checked_source("Float32[toward-zero]", "float32(bits: 0x3f800000)");
    let literals32 = literal_nodes(&one32);
    let [literal] = literals32.as_slice() else {
        panic!("the single-precision constructor lowers to exactly one literal");
    };
    assert_literal_float_type(&emitted_wire(&one32), literal, "float32", "toward-zero");
    assert_eq!(literal["body"]["value_kind"], "float32_bits");
    assert_eq!(literal["body"]["value"], "3f800000");
    assert!(matches!(
        evaluate_nullary(&one32).0,
        Outcome::Completed(Value::Float(value)) if value.bits() == 0x3f80_0000
    ));

    let uncontextualized = checked_source(
        "Boolean",
        "float64(bits: 0x0000000000000000) = float64(bits: 0x8000000000000000)",
    );
    let literals = literal_nodes(&uncontextualized);
    assert_eq!(literals.len(), 2);
    let wire = emitted_wire(&uncontextualized);
    for literal in &literals {
        let type_id = &literal["semantic_type"];
        let ty = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["node_id"] == *type_id)
            .expect("the constructor's type is emitted");
        assert_eq!(ty["semantic_form"], "float_rounding");
        assert_eq!(ty["semantic_type"]["path"][1], "float64");
        assert_eq!(ty["body"]["members"][0]["name"], "rounding");
        assert_eq!(ty["body"]["members"][0]["value"]["value"], "exact");
    }

    for (bits, text) in [
        (0x7ff0_0000_0000_0001, "0x7ff0000000000001"),
        (0xfff8_0000_0000_0005, "0xfff8000000000005"),
        (0x8000_0000_0000_0000, "0x8000000000000000"),
        (0x7ff0_0000_0000_0000, "0x7ff0000000000000"),
    ] {
        let package = checked_source("Float64[nearest-even]", &format!("float64(bits: {text})"));
        let (outcome, losses) = evaluate_nullary(&package);
        assert!(matches!(
            outcome,
            Outcome::Completed(Value::Float(value)) if value.bits() == bits
        ));
        assert!(flags(&losses).is_empty(), "{text}: {losses:?}");
    }
}

/// FR-397-AC-11 (TC-1091): malformed bit syntax refuses as a located invalid
/// token; a valid constructor with an incompatible expected type is ill-typed.
#[trace("TC-1091", "FR-397-AC-11")]
#[test]
fn malformed_and_mistyped_bit_constructors_refuse_at_the_constructor() {
    use fsl_cst::{CompleteCause, CompleteCode};

    for (result, constructor) in [
        ("Float64[nearest-even]", "float64(bits: 0x3ff0)"),
        ("Float64[nearest-even]", "float64(bits: 0x3FF0000000000000)"),
        ("Float32[nearest-even]", "float32(bits: 3f800000)"),
        ("Float32[nearest-even]", "float32(bits: 0x3f80000g)"),
        ("Float64[nearest-even]", "float64(0x3ff0000000000000)"),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             function f using v(): {result} pure {{ {constructor} }}\n"
        );
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "fr397-bit-literal-refusal"),
            "unit.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("S1 runs");
        let [diagnostic] = parsed.diagnostics() else {
            panic!(
                "one syntax diagnostic for {constructor}: {:?}",
                parsed.diagnostics()
            );
        };
        assert_eq!(
            diagnostic.code,
            CompleteCode::InvalidSyntax,
            "{constructor}"
        );
        assert_eq!(
            diagnostic.cause,
            CompleteCause::InvalidToken,
            "{constructor}"
        );
        let span = diagnostic.byte_span().expect("the refusal is located");
        let constructor_start = text.find(constructor).expect("constructor occurs");
        assert!(
            span.start <= constructor_start && constructor_start < span.end,
            "{constructor}: diagnostic span {span:?}"
        );
    }

    for (result, constructor) in [
        ("Float64[nearest-even]", "float32(bits: 0x3f800000)"),
        ("Int[0, 10]", "float64(bits: 0x3ff0000000000000)"),
    ] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"filament.value.complete/v1\";\n\
             function f using v(): {result} pure {{ {constructor} }}\n"
        );
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "fr397-bit-literal-type-refusal"),
            "unit.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("S1 runs");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let declarations = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fr397-bit-literal".to_owned(),
            unit,
            Vec::new(),
            Vec::new(),
        )
        .expect("the assembler accepts the unit");
        let refusals = declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect_err("the expected type rejects the bit constructor");
        let [refusal] = refusals.as_slice() else {
            panic!("one typed refusal for {constructor}: {refusals:?}");
        };
        assert_eq!(
            &*refusal.cause,
            &CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
        );
        assert!(refusal.location.path().is_empty(), "{refusal:?}");
    }
}

/// FR-397-AC-19 (TC-1091): expression-only float use selects the IEEE law even
/// when the unit declares no float type.
#[test]
#[trace("TC-1091", "FR-397-AC-19")]
fn expression_only_float_forms_carry_the_ieee_profile_law() {
    for (parameters, result, body, operation) in [
        (
            "",
            "Boolean",
            "float64(bits: 0x3ff0000000000000) = float64(bits: 0x3ff0000000000000)",
            "filament.op.ieee.numeric_equal",
        ),
        (
            "",
            "Boolean",
            "filament::value::ieee::bitIdentical(float64(bits: 0x3ff0000000000000), float64(bits: 0x3ff0000000000000))",
            "filament.op.ieee.bit_identical",
        ),
        (
            "i: Int[0, 10]",
            "Float64[nearest-even]",
            "convert<Float64[nearest-even]>(i)",
            "filament.op.ieee.from_exact",
        ),
    ] {
        let package = checked_function(parameters, result, body);
        let wire = emitted_wire(&package);
        assert_ieee_operation_has_selected_law(&wire, operation);
    }
}

/// FR-397-AC-22 (TC-1091): structural equality retains IEEE float leaves in
/// every specified composite shape and evaluates the literal vectors.
#[trace("TC-1091", "FR-397-AC-22")]
#[test]
fn structural_composites_compare_float_leaves_numerically() {
    let structural = checked_module(
        r#"
record R { x: Float64[nearest-even]; y: Integer; }
record RT { x: Float64[nearest-even]; label: Text[0, 8; binary-utf8]; }
union U { V(Float64[nearest-even]), N }
tuple T(Float32[exact], Integer);
function eq_record using v(a: R, b: R): Boolean pure { a = b }
function ne_record using v(a: R, b: R): Boolean pure { a != b }
function eq_option using v(a: Option<Float64[nearest-even]>, b: Option<Float64[nearest-even]>): Boolean pure { a = b }
function ne_option using v(a: Option<Float64[nearest-even]>, b: Option<Float64[nearest-even]>): Boolean pure { a != b }
function eq_union using v(a: U, b: U): Boolean pure { a = b }
function ne_union using v(a: U, b: U): Boolean pure { a != b }
function eq_sequence using v(a: Sequence<Float64[nearest-even]>[0, 2], b: Sequence<Float64[nearest-even]>[0, 2]): Boolean pure { a = b }
function ne_sequence using v(a: Sequence<Float64[nearest-even]>[0, 2], b: Sequence<Float64[nearest-even]>[0, 2]): Boolean pure { a != b }
function eq_tuple using v(a: T, b: T): Boolean pure { a = b }
function ne_tuple using v(a: T, b: T): Boolean pure { a != b }
function eq_record_text using v(a: RT, b: RT): Boolean pure { a = b }
"#,
    );
    let wire = emitted_wire(&structural);
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes are emitted");
    let eq_nodes: Vec<_> = nodes
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "filament.op.structural.eq")
        .collect();
    let ne_nodes: Vec<_> = nodes
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "filament.op.structural.ne")
        .collect();
    assert_eq!(eq_nodes.len(), 6);
    assert_eq!(ne_nodes.len(), 5);
    let mut float_paths = Vec::new();
    for node in eq_nodes.iter().chain(ne_nodes.iter()) {
        let leaves = node["body"]["operation"]["leaves"]
            .as_array()
            .expect("structural operation leaves are emitted");
        for leaf in leaves {
            let path: Vec<_> = leaf["path"]
                .as_array()
                .expect("leaf path is emitted")
                .iter()
                .map(|segment| segment.as_str().expect("path member is text"))
                .collect();
            let path = path.join("/");
            if leaf["laws"]
                .as_array()
                .is_some_and(|laws| laws.iter().any(|law| law["role"] == "ieee_profile"))
            {
                assert!(leaf["mode"].is_null(), "{path}: {leaf}");
                float_paths.push(path);
            }
        }
    }
    float_paths.sort();
    assert_eq!(
        float_paths,
        [
            "field:x",
            "field:x",
            "field:x",
            "inner",
            "inner",
            "inner",
            "inner",
            "member:V/position:0",
            "member:V/position:0",
            "position:0",
            "position:0",
        ]
    );
    let record_text = eq_nodes
        .iter()
        .find(|node| {
            node["body"]["operation"]["leaves"]
                .as_array()
                .is_some_and(|leaves| leaves.iter().any(|leaf| leaf["path"][0] == "field:label"))
        })
        .expect("the record with a float and text leaf is emitted");
    let record_paths: Vec<Vec<String>> = record_text["body"]["operation"]["leaves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|leaf| {
            leaf["path"]
                .as_array()
                .unwrap()
                .iter()
                .map(|segment| segment.as_str().unwrap().to_owned())
                .collect()
        })
        .collect();
    assert_eq!(
        record_paths,
        [vec!["field:x".to_owned()], vec!["field:label".to_owned()]]
    );

    let vectors = checked_module(
        r#"
record R { x: Float64[nearest-even]; y: Integer; }
union U { V(Float64[nearest-even]), N }
tuple T(Float32[exact], Integer);
function eq_option using v(a: Option<Float64[nearest-even]>, b: Option<Float64[nearest-even]>): Boolean pure { a = b }
function eq_sequence using v(a: Sequence<Float64[nearest-even]>[0, 2], b: Sequence<Float64[nearest-even]>[0, 2]): Boolean pure { a = b }
function record_zero using v(): Boolean pure { R { x: float64(bits: 0x0000000000000000), y: 1 } = R { x: float64(bits: 0x8000000000000000), y: 1 } }
function record_nan using v(): Boolean pure { R { x: float64(bits: 0x7ff8000000000000), y: 1 } = R { x: float64(bits: 0x7ff8000000000000), y: 1 } }
function record_nan_ne using v(): Boolean pure { R { x: float64(bits: 0x7ff8000000000000), y: 1 } != R { x: float64(bits: 0x7ff8000000000000), y: 1 } }
function record_other_field using v(): Boolean pure { R { x: float64(bits: 0x0000000000000000), y: 1 } = R { x: float64(bits: 0x0000000000000000), y: 2 } }
function union_zero using v(): Boolean pure { U::V(float64(bits: 0x0000000000000000)) = U::V(float64(bits: 0x8000000000000000)) }
function union_empty using v(): Boolean pure { U::V(float64(bits: 0x0000000000000000)) = U::N }
function sequence_zero using v(): Boolean pure { eq_sequence(sequence[float64(bits: 0x0000000000000000), float64(bits: 0x3ff0000000000000)], sequence[float64(bits: 0x8000000000000000), float64(bits: 0x3ff0000000000000)]) }
function sequence_nan using v(): Boolean pure { eq_sequence(sequence[float64(bits: 0x7ff8000000000000)], sequence[float64(bits: 0x7ff8000000000000)]) }
function tuple_zero using v(): Boolean pure { T(float32(bits: 0x80000000), 1) = T(float32(bits: 0x00000000), 1) }
"#,
    );
    for (name, expected) in [
        ("record_zero", true),
        ("record_nan", false),
        ("record_nan_ne", true),
        ("record_other_field", false),
        ("union_zero", true),
        ("union_empty", false),
        ("sequence_zero", true),
        ("sequence_nan", false),
        ("tuple_zero", true),
    ] {
        assert!(
            matches!(evaluate_named(&vectors, name).0, Outcome::Completed(Value::Boolean(value)) if value == expected),
            "{name} must be {expected}"
        );
    }
    let float_type = ValueType::Float(FloatType::new(
        IeeeWidth::Binary64,
        RoundingMode::NearestEven,
    ));
    let nan = Value::Float(IeeeValue::binary64(0x7ff8_0000_0000_0000));
    let option = OptionValue::present(float_type, nan).expect("the NaN is a present float value");
    assert!(matches!(
        evaluate_named_with_args(&vectors, "eq_option", vec![option.clone(), option]).0,
        Outcome::Completed(Value::Boolean(false))
    ));
}

/// FR-397-AC-23 (TC-1091): sets match float members by bits while their
/// structural leaf uses numeric equality; bags and sequences retain their
/// distinct value semantics.
#[test]
#[trace("TC-1091", "FR-397-AC-23")]
fn set_bag_and_sequence_float_equality_use_their_defined_semantics() {
    let package = checked_module(
        r#"
function eq_set using v(a: Set<Float64[nearest-even]>[0, 2], b: Set<Float64[nearest-even]>[0, 2]): Boolean pure { a = b }
function ne_set using v(a: Set<Float64[nearest-even]>[0, 2], b: Set<Float64[nearest-even]>[0, 2]): Boolean pure { a != b }
function eq_bag using v(a: Bag<Float64[nearest-even]>[0, 2], b: Bag<Float64[nearest-even]>[0, 2]): Boolean pure { a = b }
function eq_sequence using v(a: Sequence<Float64[nearest-even]>[0, 2], b: Sequence<Float64[nearest-even]>[0, 2]): Boolean pure { a = b }
function set_nan using v(): Boolean pure { eq_set(set[float64(bits: 0x7ff8000000000000)], set[float64(bits: 0x7ff8000000000000)]) }
function set_zero using v(): Boolean pure { eq_set(set[float64(bits: 0x0000000000000000)], set[float64(bits: 0x8000000000000000)]) }
function set_zero_ne using v(): Boolean pure { ne_set(set[float64(bits: 0x0000000000000000)], set[float64(bits: 0x8000000000000000)]) }
function set_order using v(): Boolean pure { eq_set(set[float64(bits: 0x0000000000000000), float64(bits: 0x8000000000000000)], set[float64(bits: 0x8000000000000000), float64(bits: 0x0000000000000000)]) }
function set_distinct_nan using v(): Boolean pure { eq_set(set[float64(bits: 0x7ff8000000000000)], set[float64(bits: 0x7ff8000000000001)]) }
function bag_nan using v(): Boolean pure { eq_bag(bag[float64(bits: 0x7ff8000000000000)], bag[float64(bits: 0x7ff8000000000000)]) }
function bag_zero using v(): Boolean pure { eq_bag(bag[float64(bits: 0x0000000000000000)], bag[float64(bits: 0x8000000000000000)]) }
function sequence_nan using v(): Boolean pure { eq_sequence(sequence[float64(bits: 0x7ff8000000000000)], sequence[float64(bits: 0x7ff8000000000000)]) }
"#,
    );
    let wire = emitted_wire(&package);
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes are emitted");
    for (identity, expected_count) in [
        ("filament.op.structural.eq", 3),
        ("filament.op.structural.ne", 1),
    ] {
        let operations: Vec<_> = nodes
            .iter()
            .filter(|node| node["body"]["operation"]["identity"] == identity)
            .collect();
        assert_eq!(operations.len(), expected_count, "{identity}");
        for operation in operations {
            let leaves = operation["body"]["operation"]["leaves"]
                .as_array()
                .expect("structural leaves are emitted");
            assert_eq!(leaves.len(), 1, "{identity}: {leaves:?}");
            let leaf = &leaves[0];
            assert_eq!(leaf["path"], serde_json::json!(["inner"]));
            assert_eq!(leaf["mode"], serde_json::Value::Null);
            assert_eq!(
                leaf["laws"][0]["role"], "ieee_profile",
                "{identity}: {leaf}"
            );
        }
    }
    for (name, expected) in [
        ("set_nan", true),
        ("set_zero", false),
        ("set_zero_ne", true),
        ("set_order", true),
        ("set_distinct_nan", false),
        ("bag_nan", true),
        ("bag_zero", false),
        ("sequence_nan", false),
    ] {
        assert!(
            matches!(evaluate_named(&package, name).0, Outcome::Completed(Value::Boolean(value)) if value == expected),
            "{name} must be {expected}"
        );
    }
}

/// The FR-397-AC-13 vector table at one width: zero, negative zero, one,
/// the infinities, a quiet NaN and a signaling NaN.
struct Vectors {
    width: u32,
    p0: u64,
    n0: u64,
    one: u64,
    pinf: u64,
    ninf: u64,
    qnan: u64,
    snan: u64,
}

const VECTORS: [Vectors; 2] = [
    Vectors {
        width: 64,
        p0: 0x0000_0000_0000_0000,
        n0: 0x8000_0000_0000_0000,
        one: 0x3ff0_0000_0000_0000,
        pinf: 0x7ff0_0000_0000_0000,
        ninf: 0xfff0_0000_0000_0000,
        qnan: 0x7ff8_0000_0000_0000,
        snan: 0x7ff0_0000_0000_0001,
    },
    Vectors {
        width: 32,
        p0: 0x0000_0000,
        n0: 0x8000_0000,
        one: 0x3f80_0000,
        pinf: 0x7f80_0000,
        ninf: 0xff80_0000,
        qnan: 0x7fc0_0000,
        snan: 0x7f80_0001,
    },
];

/// Trace: FR-397-AC-13, FR-397-AC-24 (F34); FSpec FR-148-AC-12's vectors,
/// with no flag, at both widths.
#[trace(
    "TC-1091",
    "FR-397-AC-13",
    "FR-397-AC-24",
    "FSpec-TC-193",
    "FSpec-FR-148-AC-12"
)]
#[trace("FSpec-TC-194", "FSpec-FR-149-AC-13")]
#[test]
fn ordering_and_equality_vectors_hold_at_both_widths() {
    for v in &VECTORS {
        let w = v.width;
        let expected = [
            (v.n0, "=", v.p0, true),
            (v.n0, "<", v.p0, false),
            (v.n0, "<=", v.p0, true),
            (v.p0, ">=", v.n0, true),
            (v.p0, ">", v.n0, false),
            (v.one, "<", v.pinf, true),
            (v.ninf, "<", v.one, true),
            (v.qnan, "=", v.qnan, false),
            (v.qnan, "!=", v.qnan, true),
            (v.qnan, "!=", v.one, true),
            (v.qnan, "<", v.one, false),
            (v.qnan, "<=", v.one, false),
            (v.one, ">", v.qnan, false),
            (v.one, ">=", v.qnan, false),
            (v.snan, "<", v.one, false),
            (v.snan, "=", v.snan, false),
            (v.pinf, "=", v.pinf, true),
            (v.pinf, "<=", v.pinf, true),
        ];
        for (left, op, right, value) in expected {
            let body = format!("{} {op} {}", constant(w, left), constant(w, right));
            let (outcome, losses) = run("Boolean", &body);
            assert!(
                matches!(outcome, Outcome::Completed(Value::Boolean(holds)) if holds == value),
                "{w}: {left:#x} {op} {right:#x}: {outcome:?}"
            );
            // FSpec FR-148-AC-12: a quiet comparison raises no flag, a
            // signaling NaN operand included.
            assert!(flags(&losses).is_empty(), "{w}: {body}: {losses:?}");
        }
    }
}

/// Trace: FR-397-AC-16
#[trace("TC-1091", "FR-397-AC-16")]
#[trace("FSpec-TC-194", "FSpec-FR-149-AC-13")]
#[test]
fn total_order_and_bit_identity_calls_separate_zero_signs_and_nan_payloads() {
    let f64 = |name, left, right| call(name, 64, left, right);
    assert!(f64("totalOrder", 0x8000_0000_0000_0000, 0));
    assert!(!f64("totalOrder", 0, 0x8000_0000_0000_0000));
    assert!(f64(
        "totalOrder",
        0x7ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
    assert!(!f64(
        "totalOrder",
        0x7ff8_0000_0000_0000,
        0x7ff0_0000_0000_0000
    ));
    assert!(f64(
        "totalOrder",
        0x7ff0_0000_0000_0001,
        0x7ff8_0000_0000_0000
    ));
    assert!(f64(
        "totalOrder",
        0xfff8_0000_0000_0000,
        0xfff0_0000_0000_0000
    ));
    assert!(!f64("bitIdentical", 0x8000_0000_0000_0000, 0));
    assert!(f64("numericEqual", 0x8000_0000_0000_0000, 0));
    assert!(f64(
        "bitIdentical",
        0x7ff8_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
    assert!(!f64(
        "numericEqual",
        0x7ff8_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
}

/// Trace: FR-397-AC-24 (F01, F02, F02b)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f01_f02_f02b_zeros_and_nans_separate_equality_order_and_identity() {
    let f32 = |name, left, right| call(name, 32, left, right);
    // F01
    assert!(operator(32, "=", 0, 0x8000_0000));
    assert!(!f32("bitIdentical", 0x8000_0000, 0));
    assert!(f32("totalOrder", 0x8000_0000, 0));
    assert!(!f32("totalOrder", 0, 0x8000_0000));
    // F02
    let (a, b) = (0x7fc0_0001, 0x7fc0_0002);
    assert!(!operator(32, "=", a, a));
    assert!(f32("bitIdentical", a, a));
    assert!(f32("totalOrder", a, b));
    assert!(!f32("totalOrder", b, a));
    // F02b
    for (low, high) in [(0xffc0_0002, 0xffc0_0001), (0x7f80_0001, 0x7fc0_0001)] {
        assert!(f32("totalOrder", low, high), "{low:#x} {high:#x}");
        assert!(!f32("totalOrder", high, low), "{high:#x} {low:#x}");
    }
}

/// Trace: FR-397-AC-24 (F05)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f05_infinities_and_finite_extrema_follow_value_order() {
    let f32 = |name, left, right| call(name, 32, left, right);
    for (low, high) in [
        (0xff80_0000, 0xff7f_ffff),
        (0xff7f_ffff, 0x7f7f_ffff),
        (0x7f7f_ffff, 0x7f80_0000),
    ] {
        assert!(operator(32, "<", low, high));
        assert!(operator(32, "<=", low, high));
        assert!(f32("totalOrder", low, high));
        assert!(!f32("numericEqual", low, high));
        assert!(!operator(32, "<", high, low));
        assert!(!operator(32, "<=", high, low));
    }
    assert!(f32("bitIdentical", 0x7f80_0000, 0x7f80_0000));
    assert!(!f32("bitIdentical", 0x7f80_0000, 0xff80_0000));
}

/// Trace: FR-397-AC-15
#[trace("TC-1091", "FR-397-AC-15")]
#[test]
fn fma_rounds_once_where_multiply_then_add_rounds_twice() {
    let (bits, _) = float(
        64,
        &format!(
            "{IEEE}fma({}, {}, {})",
            constant(64, 0x3ff0_0000_0000_0001),
            constant(64, 0x3fef_ffff_ffff_ffff),
            constant(64, 0xbff0_0000_0000_0000)
        ),
    );
    assert_eq!(bits, 0x3c9f_ffff_ffff_fffe);
    let (separate, _) = float(
        64,
        &format!(
            "{} * {} + {}",
            constant(64, 0x3ff0_0000_0000_0001),
            constant(64, 0x3fef_ffff_ffff_ffff),
            constant(64, 0xbff0_0000_0000_0000)
        ),
    );
    assert_eq!(separate, 0);
}

/// Trace: FR-397-AC-24 (F11)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f11_fused_multiply_add_rounds_once_unlike_multiply_then_add() {
    let (bits, _) = float(
        32,
        &format!(
            "{IEEE}fma({}, {}, {})",
            constant(32, 0x3f80_0001),
            constant(32, 0x3f7f_fffe),
            constant(32, 0xbf80_0000)
        ),
    );
    assert_eq!(bits, 0xa880_0000);
    let (separate, _) = float(
        32,
        &format!(
            "{} * {} + {}",
            constant(32, 0x3f80_0001),
            constant(32, 0x3f7f_fffe),
            constant(32, 0xbf80_0000)
        ),
    );
    assert_eq!(separate, 0);
}

/// Trace: FR-397-AC-24 (F04b)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f04b_leftmost_nan_payload_is_kept_and_a_signaling_nan_raises_invalid() {
    let sum = format!(
        "{} + {}",
        constant(32, 0x7fc0_0012),
        constant(32, 0xff80_0021)
    );
    let fused = format!(
        "{IEEE}fma({}, {}, {})",
        constant(32, 0x3f80_0000),
        constant(32, 0x7fc0_0012),
        constant(32, 0xff80_0021)
    );
    for body in [sum, fused] {
        let (bits, losses) = float(32, &body);
        assert_eq!(bits, 0x7fc0_0012, "{body}");
        assert_eq!(flags(&losses), [IeeeFlag::Invalid], "{body}");
    }
}

/// Trace: FR-397-AC-12, FR-397-AC-13
///
/// A signaling NaN sum records one `invalid` flag per evaluation, so the
/// number of flag records counts how often an operand is evaluated.
#[trace("TC-1091", "FR-397-AC-12", "FR-397-AC-13")]
#[test]
fn each_ordering_operand_is_evaluated_exactly_once() {
    let signaling_sum = format!(
        "{} + {}",
        constant(32, 0x7f80_0001),
        constant(32, 0x3f80_0000)
    );
    let one = constant(32, 0x3f80_0000);
    for operator in ["<", "<=", ">", ">="] {
        for body in [
            format!("({signaling_sum}) {operator} {one}"),
            format!("{one} {operator} ({signaling_sum})"),
        ] {
            let (outcome, losses) = run("Boolean", &body);
            assert!(
                matches!(outcome, Outcome::Completed(Value::Boolean(false))),
                "{body}: {outcome:?}"
            );
            assert_eq!(flags(&losses), [IeeeFlag::Invalid], "{body}");
        }
    }
}

/// Compiles `function f using v(p: Float64[nearest-even], q: ..): <result>
/// pure { <body> }` and evaluates it at `p` and `q` under `limits`, returning
/// the outcome and the work and value-occurrence units it spent.
fn spend(result: &str, body: &str, p: u64, q: u64, limits: ScalarLimits) -> (Outcome<Value>, u64) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         function f using v(p: Float64[nearest-even], q: Float64[nearest-even]): {result} pure {{ {body} }}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the floating type");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            vec![
                Value::Float(IeeeValue::binary64(p)),
                Value::Float(IeeeValue::binary64(q)),
            ],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    let spent = meter.consumed(LimitKind::WorkUnits);
    (outcome, spent)
}

/// Trace: FR-397-AC-26
#[trace("TC-1091", "FR-397-AC-26")]
#[test]
fn an_ordering_spends_each_operand_once_and_stops_at_its_limit() {
    const FOUR: u64 = 0x4010_0000_0000_0000;
    const NINE: u64 = 0x4022_0000_0000_0000;
    const QNAN: u64 = 0x7ff8_0000_0000_0000;
    let root = |operand: &str| format!("{IEEE}sqrt({operand})");
    let at = |result, body: &str, p, q| spend(result, body, p, q, UNLIMITED);
    let float = "Float64[nearest-even]";
    let (_, p_alone) = at(float, "p", FOUR, NINE);
    let (_, q_alone) = at(float, "q", FOUR, NINE);
    let (_, sqrt_p) = at(float, &root("p"), FOUR, NINE);
    let (_, sqrt_q) = at(float, &root("q"), FOUR, NINE);
    for (operator, expected) in [("<", true), ("<=", true), (">", false), (">=", false)] {
        let parameters = format!("p {operator} q");
        let roots = format!("{} {operator} {}", root("p"), root("q"));
        let (plain, plain_spend) = at("Boolean", &parameters, FOUR, NINE);
        let (sqrt, sqrt_spend) = at("Boolean", &roots, FOUR, NINE);
        assert!(matches!(plain, Outcome::Completed(Value::Boolean(value)) if value == expected));
        assert!(matches!(sqrt, Outcome::Completed(Value::Boolean(value)) if value == expected));
        assert_eq!(
            sqrt_spend,
            plain_spend + (sqrt_p - p_alone) + (sqrt_q - q_alone),
            "{roots}: each operand is evaluated once"
        );
        let limited = |units: u64| {
            let mut limits = UNLIMITED;
            limits.work_units = units;
            spend("Boolean", &roots, FOUR, NINE, limits).0
        };
        assert!(
            matches!(limited(sqrt_spend), Outcome::Completed(Value::Boolean(value)) if value == expected),
            "{roots}: completes at its spend"
        );
        assert!(
            matches!(limited(sqrt_spend - 1), Outcome::Incomplete(..)),
            "{roots}: incomplete one unit below its spend"
        );
        // A NaN operand: false, each operand once, no more spend than the
        // ordered operands.
        let (nan, nan_spend) = at("Boolean", &roots, QNAN, NINE);
        assert!(
            matches!(nan, Outcome::Completed(Value::Boolean(false))),
            "{roots}: {nan:?}"
        );
        assert!(
            nan_spend <= sqrt_spend,
            "{roots}: {nan_spend} > {sqrt_spend}"
        );
    }
}

/// FR-397-AC-26 and FR-148: a standalone `=`, `!=` and `numericEqual` charge
/// the operands and the result as one owned comparison core (the same spend
/// as `totalOrder`), while the numeric-equality parts of an ordering's
/// composition add no charge, so an ordering costs far less than the three
/// comparisons its parts would charge on top of its one `totalOrder`.
#[trace("TC-1091", "FR-397-AC-26")]
#[test]
fn a_standalone_numeric_equality_charges_and_an_orderings_parts_do_not() {
    const FOUR: u64 = 0x4010_0000_0000_0000;
    const NINE: u64 = 0x4022_0000_0000_0000;
    let at = |body: &str| spend("Boolean", body, FOUR, NINE, UNLIMITED).1;
    let nothing = at("true");
    let compare = at(&format!("{IEEE}totalOrder(p, q)")) - nothing;
    assert!(compare > 0, "a comparison charges its operands and result");
    assert_eq!(at("p = q") - nothing, compare, "standalone `=`");
    assert_eq!(
        at(&format!("{IEEE}numericEqual(p, q)")) - nothing,
        compare,
        "standalone numericEqual"
    );
    assert!(at("p != q") - nothing >= compare, "standalone `!=`");
    for operator in ["<", "<=", ">", ">="] {
        let ordering = at(&format!("p {operator} q")) - nothing;
        // One totalOrder plus the connectives costs 3 comparisons; charging
        // the three numeric-equality parts would make it 6.
        assert!(
            ordering < 5 * compare,
            "p {operator} q spends {ordering}; a comparison is {compare}"
        );
    }
}
