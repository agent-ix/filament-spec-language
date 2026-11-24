// SPDX-License-Identifier: AGPL-3.0-or-later
//! Published qualification vectors executed through authored FSL source or
//! their specified value-admission boundary. Rows without an applicable
//! adapter remain explicit; they are not successful qualifications.

use filament_eval::FamilyOutcome;
use filament_exact::{
    ChargePoint, Decimal, DecimalRepresentation, IeeeComparison, IeeeValue, Incomplete, Integer,
    LimitKind, Meter, Outcome, Quantity, Rational, Refusal, ScalarLimits, TextPayload, TextProfile,
    TextType, UnitDomain, Value, admit_text, compare_ieee,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::loss::ValueLoss;
use filament_specification::{QUALIFICATION_VECTORS, QualificationVector, TestCase};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{DefinitionRole, admit_ieee_profile};
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

#[derive(Debug, PartialEq)]
enum Expected {
    Boolean(bool),
    Integer(Integer),
    Decimal {
        value: DecimalRepresentation,
        losses: Vec<DecimalLoss>,
    },
    Rational {
        numerator: Integer,
        denominator: Integer,
    },
    IeeeRefused {
        code: String,
        cause: Option<String>,
        flags: Vec<String>,
    },
    Float {
        bits: u64,
        flags: Vec<String>,
    },
    Quantity {
        numerator: Integer,
        denominator: Integer,
        unit_domain: UnitDomain,
    },
    Refused {
        code: String,
        cause: Option<String>,
    },
    Incomplete {
        limit_kind: LimitKind,
        limit: u64,
        consumed: u64,
        next_charge: Integer,
        charge_point: ChargePoint,
    },
    Undefined,
    Admitted,
}

#[derive(Debug, PartialEq)]
struct DecimalLoss {
    numerator: Integer,
    denominator: Integer,
    coefficient: Integer,
    scale: u32,
    mode: String,
}

#[derive(Debug)]
struct SourceCase {
    declarations: String,
    expected: Expected,
    arguments: Vec<Value>,
    /// Magnitudes bound to a checked quantity parameter's unit identity.
    quantity_parameter_arguments: Vec<(usize, Rational)>,
    /// Remaining budget at the source expression under test. The source
    /// function call's `FunctionCall` work unit is added by `observed`.
    limits: ScalarLimits,
    /// Charge points for the expression node, excluding the wrapper call.
    expected_charges: Option<Vec<ChargePoint>>,
    check_mode: CheckMode,
}

fn function(result: &str, body: &str, expected: Expected) -> SourceCase {
    SourceCase {
        declarations: format!("function f using v(): {result} pure {{ {body} }}"),
        expected,
        arguments: Vec::new(),
        quantity_parameter_arguments: Vec::new(),
        limits: UNLIMITED,
        expected_charges: None,
        check_mode: CheckMode::Linked,
    }
}

fn limited_function(
    result: &str,
    body: &str,
    limits: ScalarLimits,
    expected: Expected,
    expected_charges: &[ChargePoint],
) -> SourceCase {
    limited_function_with(
        result,
        "",
        body,
        Vec::new(),
        limits,
        expected,
        expected_charges,
    )
}

fn limited_function_with(
    result: &str,
    params: &str,
    body: &str,
    arguments: Vec<Value>,
    limits: ScalarLimits,
    expected: Expected,
    expected_charges: &[ChargePoint],
) -> SourceCase {
    SourceCase {
        declarations: format!("function f using v({params}): {result} pure {{ {body} }}"),
        expected,
        arguments,
        quantity_parameter_arguments: Vec::new(),
        limits,
        expected_charges: Some(expected_charges.to_vec()),
        check_mode: CheckMode::Linked,
    }
}

fn quoted(text: &str) -> Vec<&str> {
    text.split('`').skip(1).step_by(2).collect()
}

fn pairs(text: &str) -> Vec<(Integer, u32)> {
    text.match_indices('(')
        .filter_map(|(start, _)| {
            let tail = text.get(start + 1..)?;
            let (pair, _) = tail.split_once(')')?;
            let (coefficient, scale) = pair.split_once(',')?;
            Some((coefficient.trim().parse().ok()?, scale.trim().parse().ok()?))
        })
        .collect()
}

fn decimal(pair: &(Integer, u32)) -> String {
    format!("decimal({}, {})", pair.0, pair.1)
}

fn decimal_expected(text: &str) -> Expected {
    let (coefficient, scale) = pairs(text)
        .into_iter()
        .next()
        .expect("authored decimal result");
    let losses = if text.contains("exact_numerator:") {
        let field = |name| {
            text.split_once(name)
                .expect("authored loss field")
                .1
                .split([',', '}'])
                .next()
                .expect("field value")
                .trim()
        };
        vec![DecimalLoss {
            numerator: field("exact_numerator:")
                .parse()
                .expect("authored numerator"),
            denominator: field("exact_denominator:")
                .parse()
                .expect("authored denominator"),
            coefficient: field("rounded_coefficient:")
                .parse()
                .expect("authored rounded coefficient"),
            scale: field("rounded_scale:")
                .parse()
                .expect("authored rounded scale"),
            mode: field("mode:").to_owned(),
        }]
    } else {
        Vec::new()
    };
    Expected::Decimal {
        value: DecimalRepresentation::new(coefficient, scale).normalized(),
        losses,
    }
}

fn refusal_expected(text: &str) -> Expected {
    let field = |name: &str| {
        text.split_once(name).map(|(_, tail)| {
            tail.split([',', '}', '`'])
                .next()
                .expect("field")
                .trim()
                .to_owned()
        })
    };
    Expected::Refused {
        code: field("code:").expect("authored refusal code"),
        cause: field("cause:"),
    }
}

fn hexes(text: &str) -> Vec<u64> {
    text.match_indices("0x")
        .map(|(start, _)| {
            let digits: String = text[start + 2..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .collect();
            u64::from_str_radix(&digits, 16).expect("authored IEEE bits")
        })
        .collect()
}

fn float_expected(text: &str) -> Expected {
    let mut flags: Vec<String> = [
        "invalid",
        "divide_by_zero",
        "overflow",
        "underflow",
        "inexact",
    ]
    .into_iter()
    .filter(|flag| {
        text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .any(|word| word == *flag)
    })
    .map(str::to_owned)
    .collect();
    flags.sort();
    Expected::Float {
        bits: *hexes(text).first().expect("authored result bits"),
        flags,
    }
}

fn decimal_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let operands = pairs(vector.inputs);
    let result = match vector.id {
        "D01" => vec![SourceCase {
            declarations: format!(
                "function left using v(): Decimal[-1000,1000;0,2;exact] pure {{ {} }}\n\
                 function right using v(): Decimal[-1000,1000;0,2;exact] pure {{ {} }}\n\
                 function f using v(): Boolean pure {{ left() = right() }}",
                decimal(&operands[0]),
                decimal(&operands[1])
            ),
            expected: Expected::Boolean(vector.expected.starts_with("equal ")),
            arguments: Vec::new(),
            quantity_parameter_arguments: Vec::new(),
            limits: UNLIMITED,
            expected_charges: None,
            check_mode: CheckMode::Linked,
        }],
        "D02" | "D04" => {
            let (operator, result_type) = if vector.id == "D02" {
                ("+", "Decimal[-1000,1000;0,2;exact]")
            } else {
                ("/", "Decimal[-1000,1000;0,3;exact]")
            };
            vec![function(
                result_type,
                &format!(
                    "{} {operator} {}",
                    decimal(&operands[0]),
                    decimal(&operands[1])
                ),
                decimal_expected(vector.expected),
            )]
        }
        "D05" => {
            let (exact, rounded) = vector
                .expected
                .split_once("; `nearest-even`:")
                .expect("both modes");
            ["exact", "nearest-even"]
                .into_iter()
                .map(|mode| {
                    function(
                        &format!("Decimal[-1000,1000;0,2;{mode}]"),
                        &format!("{} / {}", decimal(&operands[0]), decimal(&operands[1])),
                        if mode == "exact" {
                            refusal_expected(exact)
                        } else {
                            decimal_expected(rounded)
                        },
                    )
                })
                .collect()
        }
        "D13" | "D18" => {
            let tokens = quoted(vector.inputs);
            let result_types = tokens
                .into_iter()
                .filter(|token| token.starts_with("Decimal["))
                .collect::<Vec<_>>();
            assert!(!result_types.is_empty(), "{} has result types", vector.id);

            let (operation, operator) = if vector.inputs.contains("multiply") {
                ("multiply", "*")
            } else if vector.inputs.contains("divide") {
                ("divide", "/")
            } else {
                panic!("{} has no recognized decimal operation", vector.id);
            };
            let operands_text = vector
                .inputs
                .split_once(operation)
                .expect("recognized operation phrase")
                .1
                .split_once(" into ")
                .expect("operation operands end before result type")
                .0;
            let operands = pairs(operands_text);
            assert_eq!(operands.len(), 2, "{} has exactly two operands", vector.id);
            let expression = format!(
                "{} {operator} {}",
                decimal(&operands[0]),
                decimal(&operands[1])
            );
            let refusal = || Expected::Refused {
                code: "ill_typed".to_owned(),
                cause: Some("type-mismatch".to_owned()),
            };
            result_types
                .into_iter()
                .map(|result_type| function(result_type, &expression, refusal()))
                .collect()
        }
        "D10" => quoted(vector.inputs)
            .into_iter()
            .map(|ty| SourceCase {
                declarations: format!(
                    "type P = {ty}; function f using v(): Boolean pure {{ true }}"
                ),
                expected: if ty == "Decimal[2,2;1,1;exact]"
                    || ty == "Decimal[0,9;0,4294967295;exact]"
                {
                    Expected::Admitted
                } else {
                    Expected::Refused {
                        code: "ill_typed".to_owned(),
                        cause: Some("type-mismatch".to_owned()),
                    }
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: UNLIMITED,
                expected_charges: None,
                check_mode: CheckMode::Linked,
            })
            .collect(),
        "D26" | "D27" => {
            // These rows explicitly supply source-admitted counterparts to
            // evaluator-boundary operands in D13 and D18.
            let expression = quoted(vector.inputs)[0];
            let first_result = if vector.id == "D26" {
                vector
                    .expected
                    .split_once(";")
                    .expect("result before accounting variants")
                    .0
            } else {
                vector
                    .expected
                    .split_once("result ")
                    .expect("rounded result")
                    .1
            };
            let types = if vector.id == "D26" {
                vec!["Decimal[-9,9;0,0;toward-zero]", "Decimal[-9,9;0,0;exact]"]
            } else {
                vec!["Decimal[-1000,1000;0,1;nearest-even]"]
            };
            types
                .into_iter()
                .map(|ty| function(ty, expression, decimal_expected(first_result)))
                .collect()
        }
        "D28" => {
            let tokens = quoted(vector.inputs);
            assert_eq!(tokens.len(), 5);
            let refusal = || Expected::Refused {
                code: "ill_typed".to_owned(),
                cause: Some("type-mismatch".to_owned()),
            };
            vec![
                function(tokens[0], tokens[1], refusal()),
                function(tokens[2], tokens[3], refusal()),
                function(
                    tokens[0],
                    tokens[4],
                    decimal_expected(
                        self::vector("D26")
                            .expected
                            .split_once(';')
                            .expect("D26 result before accounting variants")
                            .0,
                    ),
                ),
            ]
        }
        _ => return None,
    };
    Some(result)
}

fn decimal_accounting_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let exact = |n: i64, scale| Value::Decimal(Decimal::new(Integer::from(n), scale));
    let tuple =
        |integer_bits, decimal_digits, scale_expansion, work_units, result_units| ScalarLimits {
            integer_bits,
            decimal_digits,
            scale_expansion,
            text_input_bytes: 0,
            text_scalars: 0,
            normalized_scalars: 0,
            unit_edges: 0,
            value_occurrences: 2,
            work_units,
            result_units,
        };
    let div_source = |mode: &str| {
        limited_function(
            &format!("Decimal[-1000, 1000; 0, 2; {mode}]"),
            "decimal(1, 0) / decimal(3, 0)",
            tuple(8, 3, 2, 5, 1),
            Expected::Decimal {
                value: DecimalRepresentation::new(Integer::from(33_i64), 2).normalized(),
                losses: vec![DecimalLoss {
                    numerator: Integer::from(1_i64),
                    denominator: Integer::from(3_i64),
                    coefficient: Integer::from(33_i64),
                    scale: 2,
                    mode: "nearest-even".to_owned(),
                }],
            },
            &[
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
                ChargePoint::DecimalRounding,
                ChargePoint::DecimalResultRetain,
            ],
        )
    };
    Some(match vector.id {
        "D09" => {
            let mut complete = div_source("nearest-even");
            let short = limited_function_with(
                "Decimal[-1000, 1000; 0, 2; nearest-even]",
                "",
                "decimal(1, 0) / decimal(3, 0)",
                vec![],
                tuple(8, 3, 2, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 4,
                    consumed: 4,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::DecimalResultRetain,
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                    ChargePoint::DecimalArithmetic,
                    ChargePoint::DecimalRounding,
                ],
            );
            complete.expected_charges = Some(vec![
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
                ChargePoint::DecimalRounding,
                ChargePoint::DecimalResultRetain,
            ]);
            vec![complete, short]
        }
        "D14" => vec![
            limited_function_with(
                "Decimal[-1000, 1000; 0, 2; exact]",
                "",
                "decimal(1, 0) / decimal(3, 0)",
                vec![],
                tuple(8, 3, 2, 3, 1),
                Expected::Refused {
                    code: "inexact_decimal".to_owned(),
                    cause: Some("nonzero-discarded-digit".to_owned()),
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                    ChargePoint::DecimalArithmetic,
                ],
            ),
            limited_function_with(
                "Decimal[-1000, 1000; 0, 2; exact]",
                "",
                "decimal(1, 0) / decimal(3, 0)",
                vec![],
                tuple(8, 3, 2, 2, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 2,
                    consumed: 2,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::DecimalArithmetic,
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                ],
            ),
        ],
        "D15" => vec![
            {
                let mut case = limited_function_with(
                    "Decimal[-1000, 1000; 0, 2; nearest-even]",
                    "a: Decimal[-1000, 1000; 0, 2; nearest-even], b: Decimal[-1000, 1000; 0, 2; nearest-even]",
                    "a / b",
                    vec![exact(1, 0), exact(0, 3)],
                    tuple(8, 3, 2, 1, 1),
                    Expected::Undefined,
                    &[ChargePoint::DecimalOperands],
                );
                case.check_mode = CheckMode::Kernel;
                case
            },
            {
                let mut case = limited_function_with(
                    "Decimal[-1000, 1000; 0, 2; nearest-even]",
                    "a: Decimal[-1000, 1000; 0, 2; nearest-even], b: Decimal[-1000, 1000; 0, 2; nearest-even]",
                    "a / b",
                    vec![exact(1, 0), exact(0, 3)],
                    tuple(8, 3, 2, 0, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 0,
                        consumed: 0,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::DecimalOperands,
                    },
                    &[],
                );
                case.check_mode = CheckMode::Kernel;
                case
            },
        ],
        "D16" => vec![
            limited_function_with(
                "Decimal[-2, 2; 0, 0; nearest-away]",
                "x: Decimal[-100, 100; 0, 1; exact]",
                "convert<Decimal[-2, 2; 0, 0; nearest-away]>(x)",
                vec![exact(25, 1)],
                ScalarLimits {
                    integer_bits: 5,
                    decimal_digits: 2,
                    scale_expansion: 0,
                    text_input_bytes: 0,
                    text_scalars: 0,
                    normalized_scalars: 0,
                    unit_edges: 0,
                    value_occurrences: 1,
                    work_units: 4,
                    result_units: 0,
                },
                Expected::Refused {
                    code: "decimal_out_of_domain".to_owned(),
                    cause: Some("outside-domain".to_owned()),
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                    ChargePoint::DecimalArithmetic,
                    ChargePoint::DecimalRounding,
                ],
            ),
            limited_function_with(
                "Decimal[-2, 2; 0, 0; nearest-away]",
                "x: Decimal[-100, 100; 0, 1; exact]",
                "convert<Decimal[-2, 2; 0, 0; nearest-away]>(x)",
                vec![exact(25, 1)],
                ScalarLimits {
                    integer_bits: 5,
                    decimal_digits: 2,
                    scale_expansion: 0,
                    text_input_bytes: 0,
                    text_scalars: 0,
                    normalized_scalars: 0,
                    unit_edges: 0,
                    value_occurrences: 1,
                    work_units: 3,
                    result_units: 0,
                },
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 3,
                    consumed: 3,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::DecimalRounding,
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                    ChargePoint::DecimalArithmetic,
                ],
            ),
        ],
        "D17" => vec![limited_function_with(
            "Decimal[-1000, 1000; 0, 2; exact]",
            "",
            "decimal(100, 2) / decimal(1, 0)",
            vec![],
            tuple(7, 3, 1, 4, 1),
            Expected::Decimal {
                value: DecimalRepresentation::new(Integer::from(1_i64), 0).normalized(),
                losses: Vec::new(),
            },
            &[
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
                ChargePoint::DecimalResultRetain,
            ],
        )],
        "D20" | "D21" => {
            let (left, right, full_bits, low_bits, full_scale, expected_low): (
                Value,
                Value,
                u64,
                u64,
                u64,
                i64,
            ) = if vector.id == "D20" {
                (exact(15, 1), exact(2, 0), 6, 5, 1, 6)
            } else {
                (exact(100, 2), exact(2, 0), 9, 8, 2, 9)
            };
            let source_scale = if vector.id == "D20" { 1 } else { 2 };
            let mut cases = vec![limited_function_with(
                "Boolean",
                &format!(
                    "a: Decimal[0, 100; 0, {source_scale}; exact], b: Decimal[0, 100; 0, {source_scale}; exact]"
                ),
                "a < b",
                vec![left.clone(), right.clone()],
                tuple(
                    full_bits,
                    if vector.id == "D20" { 2 } else { 3 },
                    full_scale,
                    3,
                    1,
                ),
                Expected::Boolean(true),
                &[
                    ChargePoint::OrderingOperands,
                    ChargePoint::OrderingArithmetic,
                    ChargePoint::OrderingResultRetain,
                ],
            )];
            cases.push(limited_function_with(
                "Boolean",
                &format!(
                    "a: Decimal[0, 100; 0, {source_scale}; exact], b: Decimal[0, 100; 0, {source_scale}; exact]"
                ),
                "a < b",
                vec![left, right],
                tuple(low_bits, if vector.id == "D20" { 2 } else { 3 }, full_scale, 3, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: low_bits,
                    consumed: if vector.id == "D20" { 4 } else { 7 },
                    next_charge: Integer::from(expected_low),
                    charge_point: ChargePoint::OrderingArithmetic,
                },
                &[ChargePoint::OrderingOperands],
            ));
            cases
        }
        "D22" => {
            let result = "Decimal[0, 1; 0, 1000; exact]";
            let params = "a: Decimal[0, 1; 0, 1000; exact], b: Decimal[0, 1; 0, 1000; exact]";
            let operands = vec![exact(1, 0), exact(1, 0)];
            let points = [
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
            ];
            let mut cases = vec![limited_function_with(
                result,
                params,
                "a * b",
                operands.clone(),
                tuple(3323, 1001, 1000, 4, 1),
                Expected::Decimal {
                    value: DecimalRepresentation::new(Integer::from(1_i64), 0).normalized(),
                    losses: Vec::new(),
                },
                &[
                    ChargePoint::DecimalOperands,
                    ChargePoint::DecimalScaleExpansion,
                    ChargePoint::DecimalArithmetic,
                    ChargePoint::DecimalResultRetain,
                ],
            )];
            cases.push(limited_function_with(
                result,
                params,
                "a * b",
                operands.clone(),
                tuple(3322, 1001, 1000, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: 3322,
                    consumed: 2,
                    next_charge: Integer::from(3323_u64),
                    charge_point: ChargePoint::DecimalResultRetain,
                },
                &points,
            ));
            cases.push(limited_function_with(
                result,
                params,
                "a * b",
                operands,
                tuple(3323, 1001, 999, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::ScaleExpansion,
                    limit: 999,
                    consumed: 0,
                    next_charge: Integer::from(1000_u64),
                    charge_point: ChargePoint::DecimalResultRetain,
                },
                &points,
            ));
            cases
        }
        "D23" => vec![limited_function_with(
            "Decimal[0, 1; 0, 4294967295; exact]",
            "a: Decimal[0, 1; 0, 4294967295; exact], b: Decimal[0, 1; 0, 4294967295; exact]",
            "a * b",
            vec![exact(1, 0), exact(1, 0)],
            ScalarLimits {
                integer_bits: u64::MAX,
                decimal_digits: 64,
                scale_expansion: 4_294_967_295,
                text_input_bytes: 0,
                text_scalars: 0,
                normalized_scalars: 0,
                unit_edges: 0,
                value_occurrences: 2,
                work_units: 4,
                result_units: 1,
            },
            Expected::Incomplete {
                limit_kind: LimitKind::DecimalDigits,
                limit: 64,
                consumed: 2,
                next_charge: Integer::from(4_294_967_296_u64),
                charge_point: ChargePoint::DecimalResultRetain,
            },
            &[
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
            ],
        )],
        _ => return None,
    })
}

fn unicode_scalars(text: &str) -> Vec<char> {
    text.match_indices("U+")
        .map(|(start, _)| {
            let digits: String = text[start + 2..]
                .chars()
                .take_while(char::is_ascii_hexdigit)
                .collect();
            char::from_u32(u32::from_str_radix(&digits, 16).expect("authored code point"))
                .expect("Unicode scalar")
        })
        .collect()
}

fn text_comparison(
    profile: &str,
    left: &str,
    right: &str,
    operator: &str,
    expected: bool,
) -> SourceCase {
    SourceCase {
        declarations: format!(
            "function left using v(): Text[0,8;{profile}] pure {{ Text({left}) }}\n\
             function right using v(): Text[0,8;{profile}] pure {{ Text({right}) }}\n\
             function f using v(): Boolean pure {{ left() {operator} right() }}"
        ),
        expected: Expected::Boolean(expected),
        arguments: Vec::new(),
        quantity_parameter_arguments: Vec::new(),
        limits: UNLIMITED,
        expected_charges: None,
        check_mode: CheckMode::Linked,
    }
}

fn text_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let quote = |text: &str| serde_json::to_string(text).expect("source string spelling");
    let cases = match vector.id {
        "T01" | "T02" => {
            let scalars = unicode_scalars(self::vector("T01").inputs);
            let left = quote(&scalars[0].to_string());
            let right = quote(&scalars[1..].iter().collect::<String>());
            if vector.id == "T01" {
                assert!(matches!(vector.expected, "equal" | "unequal"));
                vec![text_comparison(
                    "unicode-scalars",
                    &left,
                    &right,
                    "=",
                    vector.expected == "equal",
                )]
            } else {
                assert!(vector.expected.starts_with("equal under each profile;"));
                quoted(vector.inputs)
                    .into_iter()
                    .map(|profile| text_comparison(profile, &left, &right, "=", true))
                    .collect()
            }
        }
        "T03" => {
            let inputs = quoted(vector.inputs);
            let left = quote(&unicode_scalars(vector.inputs)[0].to_string());
            let right = quote(inputs[0]);
            assert!(
                vector
                    .expected
                    .starts_with("equal under compatibility profiles; unequal under `nfc`")
            );
            inputs[1..]
                .iter()
                .map(|profile| text_comparison(profile, &left, &right, "=", *profile != "nfc"))
                .collect()
        }
        "T05b" => vec![SourceCase {
            declarations: "type P = Text;".to_owned(),
            expected: refusal_expected(vector.expected),
            arguments: Vec::new(),
            quantity_parameter_arguments: Vec::new(),
            limits: UNLIMITED,
            expected_charges: None,
            check_mode: CheckMode::Linked,
        }],
        "T06b" => {
            let inputs = quoted(vector.inputs);
            assert!(vector.expected.contains("compare equal"));
            vec![text_comparison(
                "binary-utf8",
                inputs[0],
                inputs[1],
                "=",
                true,
            )]
        }
        "T12" => {
            let inputs = quoted(vector.inputs);
            let bytes = |text: &str| {
                let bytes: Vec<u8> = text
                    .split_whitespace()
                    .map(|byte| u8::from_str_radix(byte, 16).expect("authored byte"))
                    .collect();
                quote(std::str::from_utf8(&bytes).expect("authored UTF-8"))
            };
            assert!(vector.expected.starts_with("`a < b`;"));
            assert!(vector.expected.contains("less than"));
            vec![
                text_comparison(
                    "unicode-scalars",
                    &quote(inputs[0]),
                    &quote(inputs[1]),
                    "<",
                    true,
                ),
                text_comparison(
                    "binary-utf8",
                    &bytes(inputs[2]),
                    &bytes(inputs[3]),
                    "<",
                    true,
                ),
            ]
        }
        _ => return None,
    };
    Some(cases)
}

fn text_accounting_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let limits = |bytes, scalars, normalized, occurrences, work, results| ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: bytes,
        text_scalars: scalars,
        normalized_scalars: normalized,
        unit_edges: 0,
        value_occurrences: occurrences,
        work_units: work,
        result_units: results,
    };
    let decomposed = "é";
    Some(match vector.id {
        "T11" => {
            let declarations = format!(
                "function left using v(): Text[0, 8; nfc] pure {{ Text(\"é\") }}\nfunction right using v(): Text[0, 8; nfc] pure {{ Text(\"{decomposed}\") }}\nfunction f using v(): Boolean pure {{ left() = right() }}"
            );
            let full_points = [
                ChargePoint::TextInputBytes,
                ChargePoint::TextDecodeScalars,
                ChargePoint::TextNormalizeInput,
                ChargePoint::TextNormalizeOutput,
                ChargePoint::TextNormalizeOutput,
                ChargePoint::TextResultRetain,
            ];
            let mut complete = limited_function(
                "Boolean",
                "true",
                limits(5, 3, 2, 2, 6, 1),
                Expected::Boolean(true),
                &full_points,
            );
            complete.declarations = declarations.clone();
            let mut normalized_limit = limited_function(
                "Boolean",
                "true",
                limits(5, 3, 1, 2, 6, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::NormalizedScalars,
                    limit: 1,
                    consumed: 1,
                    next_charge: Integer::from(2_i64),
                    charge_point: ChargePoint::TextNormalizeOutput,
                },
                &full_points[..4],
            );
            normalized_limit.declarations = declarations.clone();
            let mut result_limit = limited_function(
                "Boolean",
                "true",
                limits(5, 3, 2, 2, 6, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::ResultUnits,
                    limit: 0,
                    consumed: 0,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::TextResultRetain,
                },
                &full_points[..5],
            );
            result_limit.declarations = declarations;
            vec![complete, normalized_limit, result_limit]
        }
        "T14" => {
            let mut cases = Vec::new();
            for profile in ["unicode-scalars", "binary-utf8"] {
                let body = format!(
                    "function left using v(): Text[0, 8; {profile}] pure {{ Text(\"é\") }}\nfunction right using v(): Text[0, 8; {profile}] pure {{ Text(\"{decomposed}\") }}\nfunction f using v(): Boolean pure {{ left() = right() }}"
                );
                let mut complete = function("Boolean", "true", Expected::Boolean(false));
                complete.declarations = body.clone();
                complete.limits = limits(5, 3, 0, 2, 3, 1);
                complete.expected_charges = Some(vec![
                    ChargePoint::TextInputBytes,
                    ChargePoint::TextDecodeScalars,
                    ChargePoint::TextResultRetain,
                ]);
                let mut denied = function(
                    "Boolean",
                    "true",
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 2,
                        consumed: 2,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::TextResultRetain,
                    },
                );
                denied.declarations = body.clone();
                denied.limits = limits(5, 3, 0, 2, 2, 1);
                denied.expected_charges = Some(vec![
                    ChargePoint::TextInputBytes,
                    ChargePoint::TextDecodeScalars,
                ]);
                cases.push(complete);
                cases.push(denied);
            }
            cases
        }
        "T15" => vec![
            SourceCase {
                declarations: "enum Status { READY, DONE }\nfunction f using v(): Boolean pure { Status::READY < Status::DONE }".to_owned(),
                expected: Expected::Refused {
                    code: "ill_typed".to_owned(),
                    cause: Some("operator-ineligible".to_owned()),
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: limits(0, 0, 0, 0, 0, 0),
                expected_charges: Some(Vec::new()),
                check_mode: CheckMode::Linked,
            },
            SourceCase {
                declarations: "enum A { READY }\nenum B { READY }\nfunction f using v(): Boolean pure { A::READY = B::READY }".to_owned(),
                expected: Expected::Refused {
                    code: "ill_typed".to_owned(),
                    cause: Some("type-mismatch".to_owned()),
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: limits(0, 0, 0, 0, 0, 0),
                expected_charges: Some(Vec::new()),
                check_mode: CheckMode::Linked,
            },
        ],
        _ => return None,
    })
}

fn float_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let operands = hexes(vector.inputs);
    let (width, expression, expected) = match vector.id {
        "F03" | "F04" => (
            32,
            format!(
                "float32(bits: {:#010x}) + float32(bits: {:#010x})",
                operands[0], operands[1]
            ),
            float_expected(vector.expected),
        ),
        "F07" | "F08" | "F10" => {
            let width = if vector.id == "F10" { 64 } else { 32 };
            (
                width,
                format!(
                    "float{width}(bits: {:#x}) + float{width}(bits: {:#x})",
                    operands[0], operands[1]
                ),
                float_expected(vector.expected),
            )
        }
        "F11" => (
            32,
            format!(
                "filament::value::ieee::fma(float32(bits: {:#x}), float32(bits: {:#x}), float32(bits: {:#x}))",
                operands[0], operands[1], operands[2]
            ),
            float_expected(vector.expected),
        ),
        "F15" | "F16" => (
            32,
            format!(
                "float32(bits: {:#010x}) * float32(bits: {:#010x})",
                operands[0], operands[1]
            ),
            float_expected(vector.expected),
        ),
        _ => return None,
    };
    Some(vec![function(
        &format!("Float{width}[nearest-even]"),
        &expression,
        expected,
    )])
}

fn float_accounting_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let limits = |integer_bits, value_occurrences, work_units, result_units| ScalarLimits {
        integer_bits,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences,
        work_units,
        result_units,
    };
    let ieee = [
        ChargePoint::IeeeOperands,
        ChargePoint::IeeeExactIntermediate,
        ChargePoint::IeeeRound,
        ChargePoint::IeeeResultRetain,
    ];
    Some(match vector.id {
        "F17" => {
            let mut cases = Vec::new();
            for (width, ty, qn_bits, one_bits) in [
                (
                    32_u64,
                    "Float32[nearest-even]",
                    0x7fc0_0001_u64,
                    0x3f80_0000_u64,
                ),
                (
                    64,
                    "Float64[nearest-even]",
                    0x7ff8_0000_0000_0001,
                    0x3ff0_0000_0000_0000,
                ),
            ] {
                let (qn, one) = if width == 32 {
                    (
                        Value::Float(IeeeValue::binary32(
                            u32::try_from(qn_bits).expect("binary32 NaN bits fit u32"),
                        )),
                        Value::Float(IeeeValue::binary32(
                            u32::try_from(one_bits).expect("binary32 one bits fit u32"),
                        )),
                    )
                } else {
                    (
                        Value::Float(IeeeValue::binary64(qn_bits)),
                        Value::Float(IeeeValue::binary64(one_bits)),
                    )
                };
                let params = format!("qn: {ty}, one: {ty}");
                for (body, expected, includes_boolean_retain) in [
                    ("qn = one", false, false),
                    ("filament::value::ieee::numericEqual(qn, one)", false, false),
                    ("qn != one", true, true),
                    ("one = one", true, false),
                ] {
                    let mut charges =
                        vec![ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain];
                    if includes_boolean_retain {
                        charges.push(ChargePoint::BooleanResultRetain);
                    }
                    cases.push(limited_function_with(
                        "Boolean",
                        &params,
                        body,
                        vec![qn.clone(), one.clone()],
                        limits(
                            width,
                            2,
                            2 + u64::from(includes_boolean_retain),
                            1 + u64::from(includes_boolean_retain),
                        ),
                        Expected::Boolean(expected),
                        &charges,
                    ));
                    cases.push(limited_function_with(
                        "Boolean",
                        &params,
                        body,
                        vec![qn.clone(), one.clone()],
                        limits(width, 2, 1, 1),
                        Expected::Incomplete {
                            limit_kind: LimitKind::WorkUnits,
                            limit: 1,
                            consumed: 1,
                            next_charge: Integer::from(1_u64),
                            charge_point: ChargePoint::IeeeResultRetain,
                        },
                        &[ChargePoint::IeeeOperands],
                    ));
                    cases.push(limited_function_with(
                        "Boolean",
                        &params,
                        body,
                        vec![qn.clone(), one.clone()],
                        limits(width, 0, 2, 1),
                        Expected::Incomplete {
                            limit_kind: LimitKind::ValueOccurrences,
                            limit: 0,
                            consumed: 0,
                            next_charge: Integer::from(2_u64),
                            charge_point: ChargePoint::IeeeOperands,
                        },
                        &[],
                    ));
                }
            }
            cases
        }
        "F18" => vec![limited_function(
            "Float32[nearest-even]",
            "float32(bits: 0x00000000) * float32(bits: 0x7f800000)",
            limits(32, 2, 2, 1),
            Expected::Float {
                bits: 0x7fc00000,
                flags: vec!["invalid".to_owned()],
            },
            &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain],
        )],
        "F19" => vec![limited_function(
            "Float32[nearest-even]",
            "filament::value::ieee::sqrt(float32(bits: 0x40000000))",
            limits(32, 1, 4, 1),
            Expected::Float {
                bits: 0x3fb504f3,
                flags: vec!["inexact".to_owned()],
            },
            &ieee,
        )],
        "F21" => [
            (
                "float32(bits: 0x00000000) + float32(bits: 0x00000000)",
                0x00000000,
            ),
            (
                "float32(bits: 0x80000000) * float32(bits: 0x3f800000)",
                0x80000000,
            ),
        ]
        .into_iter()
        .flat_map(|(body, bits)| {
            [
                limited_function(
                    "Float32[exact]",
                    body,
                    limits(32, 2, 4, 1),
                    Expected::Float {
                        bits,
                        flags: Vec::new(),
                    },
                    &ieee,
                ),
                limited_function(
                    "Float32[exact]",
                    body,
                    limits(32, 2, 3, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 3,
                        consumed: 3,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::IeeeResultRetain,
                    },
                    &ieee[..3],
                ),
            ]
        })
        .collect(),
        "F22" => {
            let body = "float32(bits: 0x3f800000) + float32(bits: 0x33800000)";
            let exact = limited_function(
                "Float32[exact]",
                body,
                limits(32, 2, 3, 1),
                Expected::Refused {
                    code: "ieee_not_exact".to_owned(),
                    cause: Some("rounding-required".to_owned()),
                },
                &ieee[..3],
            );
            let nearest = limited_function(
                "Float32[nearest-even]",
                body,
                limits(32, 2, 3, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 3,
                    consumed: 3,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::IeeeResultRetain,
                },
                &ieee[..3],
            );
            let short = limited_function(
                "Float32[exact]",
                body,
                limits(32, 2, 2, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 2,
                    consumed: 2,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::IeeeRound,
                },
                &ieee[..2],
            );
            vec![exact, nearest, short]
        }
        "F30" => {
            let body = "filament::value::ieee::sqrt(float32(bits: 0x80000000))";
            vec![
                limited_function(
                    "Float32[nearest-even]",
                    body,
                    limits(32, 1, 4, 1),
                    Expected::Float {
                        bits: 0x80000000,
                        flags: Vec::new(),
                    },
                    &ieee,
                ),
                limited_function(
                    "Float32[nearest-even]",
                    body,
                    limits(32, 1, 3, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 3,
                        consumed: 3,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::IeeeResultRetain,
                    },
                    &ieee[..3],
                ),
            ]
        }
        "F33" => {
            let body = "-float32(bits: 0x3f800000)";
            vec![
                limited_function(
                    "Float32[exact]",
                    body,
                    limits(32, 1, 2, 1),
                    Expected::Float {
                        bits: 0xbf800000,
                        flags: Vec::new(),
                    },
                    &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain],
                ),
                limited_function(
                    "Float32[exact]",
                    body,
                    limits(32, 1, 1, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 1,
                        consumed: 1,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::IeeeResultRetain,
                    },
                    &[ChargePoint::IeeeOperands],
                ),
                limited_function(
                    "Float32[exact]",
                    body,
                    limits(32, 0, 2, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::ValueOccurrences,
                        limit: 0,
                        consumed: 0,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::IeeeOperands,
                    },
                    &[],
                ),
            ]
        }
        _ => return None,
    })
}

fn float_extended_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let limits = |bits, occurrences, work, results| ScalarLimits {
        integer_bits: bits,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: occurrences,
        work_units: work,
        result_units: results,
    };
    let float = |bits, flags: &[&str]| Expected::Float {
        bits,
        flags: flags.iter().map(|flag| (*flag).to_owned()).collect(),
    };
    let ieee = [
        ChargePoint::IeeeOperands,
        ChargePoint::IeeeExactIntermediate,
        ChargePoint::IeeeRound,
        ChargePoint::IeeeResultRetain,
    ];
    Some(match vector.id {
        "F20" => [
            "filament::value::ieee::numericEqual(float32(bits: 0x3f800000), float64(bits: 0x3ff0000000000000))",
            "filament::value::ieee::totalOrder(float32(bits: 0x3f800000), float64(bits: 0x3ff0000000000000))",
            "filament::value::ieee::bitIdentical(float32(bits: 0x3f800000), float64(bits: 0x3ff0000000000000))",
            "float32(bits: 0x3f800000) + float64(bits: 0x3ff0000000000000)",
            "float32(bits: 0x3f800000) + 1",
            "convert<Decimal[0, 10; 0, 0; exact]>(float32(bits: 0x3f800000))",
            "convert<Int[0, 1]>(float32(bits: 0x3f800000))",
        ]
        .into_iter()
        .map(|body| SourceCase {
            declarations: format!("function f using v(): Boolean pure {{ {body} }}"),
            expected: Expected::Refused {
                code: "ill_typed".to_owned(),
                cause: Some("type-mismatch".to_owned()),
            },
            arguments: Vec::new(),
            quantity_parameter_arguments: Vec::new(),
            limits: limits(0, 0, 0, 0),
            expected_charges: Some(Vec::new()),
            check_mode: CheckMode::Linked,
        })
        .collect(),
        "F23" => [
            ("nearest-even", 0x7f800000, &["inexact", "overflow"][..]),
            ("toward-zero", 0x7f7fffff, &["inexact"][..]),
        ]
        .into_iter()
        .map(|(mode, bits, flags)| {
            function(
                &format!("Float32[{mode}]"),
                "float32(bits: 0x7f7fffff) + float32(bits: 0x73000000)",
                float(bits, flags),
            )
        })
        .chain(std::iter::once(function(
            "Float32[exact]",
            "float32(bits: 0x7f7fffff) + float32(bits: 0x73000000)",
                Expected::Refused {
                    code: "ieee_not_exact".to_owned(),
                    cause: Some("rounding-required".to_owned()),
                },
        )))
        .collect(),
        "F24" => vec![
            limited_function(
                "Float64[nearest-even]",
                "convert<Float64[nearest-even]>(float32(bits: 0x3f800000))",
                limits(64, 1, 4, 1),
                float(0x3ff0000000000000, &[]),
                &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeRound, ChargePoint::IeeeResultRetain],
            ),
            limited_function(
                "Float64[nearest-even]",
                "convert<Float64[nearest-even]>(float32(bits: 0x3f800000))",
                limits(32, 1, 4, 1),
                Expected::Incomplete { limit_kind: LimitKind::IntegerBits, limit: 32, consumed: 32, next_charge: Integer::from(64_i64), charge_point: ChargePoint::IeeeExactIntermediate },
                &[ChargePoint::IeeeOperands],
            ),
            limited_function(
                "Float32[nearest-even]",
                "convert<Float32[nearest-even]>(float64(bits: 0x3ff0000000000000))",
                limits(32, 1, 4, 1),
                Expected::Incomplete { limit_kind: LimitKind::IntegerBits, limit: 32, consumed: 0, next_charge: Integer::from(64_i64), charge_point: ChargePoint::IeeeOperands },
                &[],
            ),
            limited_function_with(
                "Float32[nearest-even]",
                "q: Rational[0, 1; 1, 3]",
                "convert<Float32[nearest-even]>(q)",
                vec![Value::Rational(Rational::new(Integer::from(1_i64), Integer::from(3_i64)).expect("literal rational"))],
                limits(32, 1, 4, 1),
                float(0x3eaaaaab, &["inexact"]),
                &ieee,
            ),
            limited_function_with(
                "Float32[nearest-even]",
                "q: Rational[0, 1; 1, 3]",
                "convert<Float32[nearest-even]>(q)",
                vec![Value::Rational(Rational::new(Integer::from(1_i64), Integer::from(3_i64)).expect("literal rational"))],
                limits(2, 1, 4, 1),
                Expected::Incomplete { limit_kind: LimitKind::IntegerBits, limit: 2, consumed: 2, next_charge: Integer::from(32_i64), charge_point: ChargePoint::IeeeExactIntermediate },
                &[ChargePoint::IeeeOperands],
            ),
            limited_function(
                "Rational[0, 1; 1, 2]",
                "convert<Rational[0, 1; 1, 2]>(float32(bits: 0x3f000000))",
                limits(32, 1, 3, 1),
                Expected::Rational { numerator: Integer::from(1_i64), denominator: Integer::from(2_i64) },
                &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeResultRetain],
            ),
            limited_function(
                "Rational[0, 1; 1, 2]",
                "convert<Rational[0, 1; 1, 2]>(float32(bits: 0x7fc00000))",
                limits(32, 1, 2, 1),
                Expected::Undefined,
                &[ChargePoint::IeeeOperands],
            ),
        ],
        "F25" => vec![
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x7ff0000000000001))", limits(64, 1, 2, 1), float(0x7fc00001, &["invalid"]), &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0xfff8000000000003))", limits(64, 1, 2, 1), float(0xffc00003, &[]), &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]),
            limited_function("Float64[nearest-even]", "convert<Float64[nearest-even]>(float32(bits: 0x7fc00001))", limits(32, 1, 2, 1), float(0x7ff8000000000001, &[]), &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x7ff8000000400000))", limits(64, 1, 1, 0), Expected::Refused { code: "ieee_nan_payload_not_representable".to_owned(), cause: Some("payload-exceeds-target".to_owned()) }, &[ChargePoint::IeeeOperands]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x7ff8000000400000))", limits(64, 1, 0, 0), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 0, consumed: 0, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeOperands }, &[]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x7ff0000000400000))", limits(64, 1, 1, 0), Expected::Refused { code: "ieee_nan_payload_not_representable".to_owned(), cause: Some("payload-exceeds-target".to_owned()) }, &[ChargePoint::IeeeOperands]),
        ],
        "F26" => {
            let mut cases = vec![
                limited_function("Float64[nearest-even]", "convert<Float64[nearest-even]>(float32(bits: 0x80000000))", limits(64, 1, 4, 1), float(0x8000000000000000, &[]), &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeRound, ChargePoint::IeeeResultRetain]),
                limited_function("Float64[nearest-even]", "convert<Float64[nearest-even]>(float32(bits: 0x80000000))", limits(64, 1, 3, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 3, consumed: 3, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeResultRetain }, &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeRound]),
            ];
            for mode in ["nearest-even", "toward-zero", "toward-positive", "toward-negative", "nearest-away", "exact"] {
                cases.push(function(&format!("Float32[{mode}]"), "float32(bits: 0x80000000) + float32(bits: 0x80000000)", float(0x80000000, &[])));
                let cancel = if mode == "toward-negative" { 0x80000000 } else { 0x00000000 };
                cases.push(function(&format!("Float32[{mode}]"), "float32(bits: 0x3f800000) + float32(bits: 0xbf800000)", float(cancel, &[])));
                cases.push(function(&format!("Float32[{mode}]"), "float32(bits: 0x80000000) - float32(bits: 0x00000000)", float(0x80000000, &[])));
                cases.push(function(&format!("Float32[{mode}]"), "float32(bits: 0x00000000) - float32(bits: 0x00000000)", float(cancel, &[])));
                cases.push(function(&format!("Float32[{mode}]"), "float32(bits: 0x3f800000) - float32(bits: 0x3f800000)", float(cancel, &[])));
            }
            cases
        }
        "F27" => {
            let denominator = Integer::from(2_i64).pow(&Integer::from(1074_u64));
            let target = format!("Rational[0, 1; 1, {denominator}]");
            vec![
                limited_function(&target, &format!("convert<{target}>(float64(bits: 0x0000000000000001))"), limits(1075, 1, 3, 1), Expected::Rational { numerator: Integer::from(1_i64), denominator: denominator.clone() }, &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeResultRetain]),
                limited_function(&target, &format!("convert<{target}>(float64(bits: 0x0000000000000001))"), limits(1074, 1, 3, 1), Expected::Incomplete { limit_kind: LimitKind::IntegerBits, limit: 1074, consumed: 64, next_charge: Integer::from(1075_i64), charge_point: ChargePoint::IeeeExactIntermediate }, &[ChargePoint::IeeeOperands]),
                limited_function("Rational[0, 0; 1, 1]", "convert<Rational[0, 0; 1, 1]>(float32(bits: 0x80000000))", limits(32, 1, 3, 1), Expected::Rational { numerator: Integer::from(0_i64), denominator: Integer::from(1_i64) }, &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate, ChargePoint::IeeeResultRetain]),
                limited_function("Rational[0, 0; 1, 1]", "convert<Rational[0, 0; 1, 1]>(float32(bits: 0x80000000))", limits(32, 1, 2, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 2, consumed: 2, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeResultRetain }, &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate]),
                limited_function("Rational[0, 1; 1, 2]", "convert<Rational[0, 1; 1, 2]>(float32(bits: 0x7f800000))", limits(32, 1, 1, 0), Expected::Undefined, &[ChargePoint::IeeeOperands]),
                limited_function("Rational[0, 1; 1, 2]", "convert<Rational[0, 1; 1, 2]>(float32(bits: 0x7f800000))", limits(32, 1, 0, 0), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 0, consumed: 0, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeOperands }, &[]),
                limited_function("Rational[0, 1; 1, 1]", "convert<Rational[0, 1; 1, 1]>(float32(bits: 0x3f000000))", limits(32, 1, 2, 0), Expected::Refused { code: "ieee_rational_out_of_domain".to_owned(), cause: Some("outside-domain".to_owned()) }, &[ChargePoint::IeeeOperands, ChargePoint::IeeeExactIntermediate]),
                limited_function("Rational[0, 1; 1, 1]", "convert<Rational[0, 1; 1, 1]>(float32(bits: 0x3f000000))", limits(32, 1, 1, 0), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 1, consumed: 1, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeExactIntermediate }, &[ChargePoint::IeeeOperands]),
            ]
        }
        "F28" => vec![
            limited_function_with("Float32[nearest-even]", "d: Decimal[0, 100; 0, 2; exact]", "convert<Float32[nearest-even]>(d)", vec![Value::Decimal(Decimal::new(Integer::from(100_i64), 2))], limits(32, 1, 4, 1), float(0x3f800000, &[]), &ieee),
            limited_function_with("Float32[nearest-even]", "d: Decimal[0, 100; 0, 2; exact]", "convert<Float32[nearest-even]>(d)", vec![Value::Decimal(Decimal::new(Integer::from(100_i64), 2))], limits(6, 1, 4, 1), Expected::Incomplete { limit_kind: LimitKind::IntegerBits, limit: 6, consumed: 0, next_charge: Integer::from(7_i64), charge_point: ChargePoint::IeeeOperands }, &[]),
        ],
        "F29" => vec![
            function("Float32[nearest-even]", "float32(bits: 0x7f7fffff) + float32(bits: 0x72800000)", float(0x7f7fffff, &["inexact"])),
            function("Float32[exact]", "float32(bits: 0x7f7fffff) + float32(bits: 0x72800000)", Expected::Refused { code: "ieee_not_exact".to_owned(), cause: Some("rounding-required".to_owned()) }),
            function("Float32[nearest-even]", "float32(bits: 0x007fffff) * float32(bits: 0x3f800001)", float(0x00800000, &["inexact"])),
            function("Float32[exact]", "float32(bits: 0x007fffff) * float32(bits: 0x3f800001)", Expected::Refused { code: "ieee_not_exact".to_owned(), cause: Some("rounding-required".to_owned()) }),
        ],
        "F31" => vec![
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0xfff0000000000000))", limits(64, 1, 2, 1), float(0xff800000, &[]), &[ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0xfff0000000000000))", limits(64, 1, 1, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 1, consumed: 1, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeResultRetain }, &[ChargePoint::IeeeOperands]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x3fb999999999999a))", limits(64, 1, 4, 1), float(0x3dcccccd, &["inexact"]), &ieee),
            limited_function("Float32[exact]", "convert<Float32[exact]>(float64(bits: 0x3fb999999999999a))", limits(64, 1, 3, 1), Expected::Refused { code: "ieee_not_exact".to_owned(), cause: Some("rounding-required".to_owned()) }, &ieee[..3]),
            limited_function("Float32[exact]", "convert<Float32[exact]>(float64(bits: 0x3fb999999999999a))", limits(64, 1, 2, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 2, consumed: 2, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeRound }, &ieee[..2]),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x47efffffe0000000))", limits(64, 1, 4, 1), float(0x7f7fffff, &[]), &ieee),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x47effffff0000000))", limits(64, 1, 4, 1), float(0x7f800000, &["inexact", "overflow"]), &ieee),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x47effffff0000000))", limits(64, 1, 3, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 3, consumed: 3, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeResultRetain }, &ieee[..3]),
            limited_function("Float32[toward-zero]", "convert<Float32[toward-zero]>(float64(bits: 0x47effffff0000000))", limits(64, 1, 4, 1), float(0x7f7fffff, &["inexact"]), &ieee),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x0000000000000001))", limits(64, 1, 4, 1), float(0x00000000, &["inexact", "underflow"]), &ieee),
            limited_function("Float32[nearest-even]", "convert<Float32[nearest-even]>(float64(bits: 0x0000000000000001))", limits(64, 1, 3, 1), Expected::Incomplete { limit_kind: LimitKind::WorkUnits, limit: 3, consumed: 3, next_charge: Integer::from(1_i64), charge_point: ChargePoint::IeeeResultRetain }, &ieee[..3]),
        ],
        _ => return None,
    })
}

fn division_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    match vector.id {
        "DIV-02" => Some(
            ["truncating", "floor", "euclidean"]
                .into_iter()
                .map(|law| {
                    let expression = quoted(vector.inputs)[0].replace("mod(-7,3)", "-7 mod 3");
                    let expected = quoted(vector.expected)[0]
                        .parse()
                        .expect("authored remainder");
                    let mut case = function("Integer", &expression, Expected::Integer(expected));
                    case.declarations.insert_str(
                        0,
                        &format!("profile d = \"filament.value.integer-division.{law}/v1\";\n"),
                    );
                    case
                })
                .collect(),
        ),
        "DIV-04" => {
            let operands = quoted(vector.inputs)[0];
            let expected = quoted(vector.expected);
            Some(
                [("div", expected[0]), ("rem", expected[1])]
                    .into_iter()
                    .map(|(operator, expected)| {
                        let (left, _) = operands.split_once(" div ").expect("authored operands");
                        let mut case = function(
                            "Integer",
                            &format!("{left} {operator} -1"),
                            Expected::Integer(expected.parse().expect("authored integer")),
                        );
                        case.declarations.insert_str(
                            0,
                            "profile d = \"filament.value.integer-division.truncating/v1\";\n",
                        );
                        case
                    })
                    .collect(),
            )
        }
        _ => None,
    }
}

fn division_accounting_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let limits = |bits, work, results| ScalarLimits {
        integer_bits: bits,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: work,
        result_units: results,
    };
    let division_points = [
        ChargePoint::IntegerDivisionOperands,
        ChargePoint::IntegerDivisionArithmetic,
        ChargePoint::IntegerDivisionDomain,
        ChargePoint::IntegerDivisionResultRetain,
    ];
    let modulus_points = [
        ChargePoint::IntegerModulusOperands,
        ChargePoint::IntegerModulusArithmetic,
        ChargePoint::IntegerModulusDomain,
        ChargePoint::IntegerModulusResultRetain,
    ];
    Some(match vector.id {
        "DIV-08" => [("div", 2_i64), ("rem", 1_i64)]
            .into_iter()
            .flat_map(|(operator, result)| {
                let mut complete = limited_function(
                    "Integer",
                    &format!("7 {operator} 3"),
                    limits(3, 4, 1),
                    Expected::Integer(Integer::from(result)),
                    &division_points,
                );
                complete.declarations.insert_str(
                    0,
                    "profile d = \"filament.value.integer-division.truncating/v1\";\n",
                );
                let mut denied = limited_function(
                    "Integer",
                    &format!("7 {operator} 3"),
                    limits(3, 3, 1),
                    Expected::Incomplete {
                        limit_kind: LimitKind::WorkUnits,
                        limit: 3,
                        consumed: 3,
                        next_charge: Integer::from(1_i64),
                        charge_point: ChargePoint::IntegerDivisionResultRetain,
                    },
                    &division_points[..3],
                );
                denied.declarations.insert_str(
                    0,
                    "profile d = \"filament.value.integer-division.truncating/v1\";\n",
                );
                [complete, denied]
            })
            .collect(),
        "DIV-10" => (0..=4)
            .map(|work| {
                let mut case = limited_function_with(
                    "Integer",
                    "a: Integer, b: Integer",
                    "a mod b",
                    vec![
                        Value::Integer(Integer::from(-7_i64)),
                        Value::Integer(Integer::from(3_i64)),
                    ],
                    limits(3, work, 1),
                    if work == 4 {
                        Expected::Integer(Integer::from(2_i64))
                    } else {
                        Expected::Incomplete {
                            limit_kind: LimitKind::WorkUnits,
                            limit: work,
                            consumed: work,
                            next_charge: Integer::from(1_i64),
                            charge_point: modulus_points[work as usize],
                        }
                    },
                    &modulus_points[..work as usize],
                );
                case.declarations.insert_str(
                    0,
                    "profile d = \"filament.value.integer-division.truncating/v1\";\n",
                );
                case.check_mode = CheckMode::Kernel;
                case
            })
            .collect(),
        "DIV-11" => {
            let mut cases = Vec::new();
            for (body, points) in [
                ("a div b", &division_points[..]),
                ("a mod b", &modulus_points[..]),
            ] {
                for work in [1, 0] {
                    let mut case = limited_function_with(
                        "Integer",
                        "a: Integer, b: Integer",
                        body,
                        vec![
                            Value::Integer(Integer::from(7_i64)),
                            Value::Integer(Integer::from(0_i64)),
                        ],
                        limits(3, work, 1),
                        if work == 1 {
                            Expected::Undefined
                        } else {
                            Expected::Incomplete {
                                limit_kind: LimitKind::WorkUnits,
                                limit: 0,
                                consumed: 0,
                                next_charge: Integer::from(1_i64),
                                charge_point: points[0],
                            }
                        },
                        if work == 1 { &points[..1] } else { &[] },
                    );
                    case.check_mode = CheckMode::Kernel;
                    cases.push(case);
                }
            }
            for case in &mut cases {
                case.declarations.insert_str(
                    0,
                    "profile d = \"filament.value.integer-division.truncating/v1\";\n",
                );
            }
            cases
        }
        "DIV-12" => vec![
            limited_function_with(
                "Int[0, 1]",
                "a: Integer, b: Integer",
                "convert<Int[0, 1]>(a mod b)",
                vec![
                    Value::Integer(Integer::from(-7_i64)),
                    Value::Integer(Integer::from(3_i64)),
                ],
                limits(3, 4, 0),
                Expected::Refused {
                    code: "modulo_out_of_domain".to_owned(),
                    cause: Some("outside-domain".to_owned()),
                },
                &modulus_points[..3],
            ),
            limited_function_with(
                "Int[0, 1]",
                "a: Integer, b: Integer",
                "convert<Int[0, 1]>(a mod b)",
                vec![
                    Value::Integer(Integer::from(-7_i64)),
                    Value::Integer(Integer::from(3_i64)),
                ],
                limits(3, 2, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 2,
                    consumed: 2,
                    next_charge: Integer::from(1_i64),
                    charge_point: ChargePoint::IntegerModulusDomain,
                },
                &modulus_points[..2],
            ),
        ]
        .into_iter()
        .map(|mut case| {
            case.check_mode = CheckMode::Kernel;
            case
        })
        .collect(),
        "DIV-13" => {
            let mut case = limited_function(
                "Integer",
                "7 div 3",
                limits(2, 0, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: 2,
                    consumed: 0,
                    next_charge: Integer::from(3_i64),
                    charge_point: ChargePoint::IntegerDivisionOperands,
                },
                &[],
            );
            case.declarations.insert_str(
                0,
                "profile d = \"filament.value.integer-division.truncating/v1\";\n",
            );
            vec![case]
        }
        _ => return None,
    })
}

fn unit_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let snippets = quoted(vector.inputs);
    let source = match vector.id {
        "U11a" => format!("dimension L; {} {}", snippets[1], snippets[0]),
        "U11b" => format!("dimension L; {} {}", snippets[0], snippets[1]),
        "U11c" | "U11f" | "U11h" => format!("dimension L; unit m : L = 1; {}", snippets[0]),
        "U11g1" | "U11g2" | "U11g3" | "U11g4" | "U11n" | "U11o" | "U11w" => snippets.join("\n"),
        _ => return None,
    };
    let expected = if vector.expected.starts_with("admitted:") {
        Expected::Admitted
    } else {
        let fields = quoted(vector.expected);
        Expected::Refused {
            code: fields[0].to_owned(),
            cause: Some(fields[1].to_owned()),
        }
    };
    // The standard's mathematical unit scale is rendered with the native
    // grammar's exact-rational literal; dimension terms are left intact.
    let source: String = source
        .split(';')
        .filter(|declaration| !declaration.trim().is_empty())
        .map(|declaration| {
            let declaration = declaration.trim();
            if declaration.starts_with("unit ") {
                let (prefix, expression) = declaration.split_once('=').expect("unit definition");
                let (amount, target) = expression
                    .trim()
                    .split_once(" * ")
                    .map_or((expression.trim(), None), |(amount, target)| {
                        (amount, Some(target))
                    });
                let (numerator, denominator) = amount.split_once('/').unwrap_or((amount, "1"));
                let target = target.map_or(String::new(), |target| format!(" * {target}"));
                format!("{prefix}= rational({numerator}, {denominator}){target};\n")
            } else {
                format!("{declaration};\n")
            }
        })
        .collect();
    Some(vec![SourceCase {
        declarations: format!("{source}function f using v(): Boolean pure {{ true }}"),
        expected,
        arguments: Vec::new(),
        quantity_parameter_arguments: Vec::new(),
        limits: UNLIMITED,
        expected_charges: None,
        check_mode: CheckMode::Linked,
    }])
}

const ACCOUNTING_UNITS: &str = "\
    dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
    dimension Length; unit m : Length = rational(1, 1); unit cm : Length = rational(1, 100) * m;\n\
    unit inch : Length = rational(127, 5000) * m; unit rev : Length = rational(-1, 1) * m;\n\
    dimension Temperature; unit K : Temperature = rational(1, 1);\n\
    unit degC : Temperature = rational(1, 1) * K + rational(5463, 20);\n\
    unit degF : Temperature = rational(5, 9) * K + rational(45967, 180);\n\
    unit u1 : Temperature = rational(1, 1) * K + rational(10, 1);\n\
    unit u2 : Temperature = rational(1, 1) * u1 + rational(-10, 1);\n\
    unit u3 : Temperature = rational(1, 1) * degC;\n\
    dimension Mass; unit kg : Mass = rational(1, 1);\n\
    dimension Dimensionless; unit one : Dimensionless = rational(1, 1);\n\
    dimension Torque = Mass * Length ^ 2 / Time ^ 2; unit N_m : Torque = rational(1, 1);\n\
    dimension Energy = Mass * Length ^ 2 / Time ^ 2; unit J : Energy = rational(1, 1);\n\
    dimension Area = Length ^ 2; unit m2 : Area = rational(1, 1);\n\
    unit cm2 : Area = rational(1, 10000) * m2;\n";

fn unit_limits(
    bits: u64,
    digits: u64,
    scale: u64,
    edges: u64,
    occurrences: u64,
    work: u64,
    result: u64,
) -> ScalarLimits {
    ScalarLimits {
        integer_bits: bits,
        decimal_digits: digits,
        scale_expansion: scale,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: edges,
        value_occurrences: occurrences,
        work_units: work,
        result_units: result,
    }
}

fn quantity_expected(numerator: i64, denominator: i64, unit_domain: UnitDomain) -> Expected {
    Expected::Quantity {
        numerator: Integer::from(numerator),
        denominator: Integer::from(denominator),
        unit_domain,
    }
}

fn accounting_unit_case(
    result: &str,
    expression: &str,
    limits: ScalarLimits,
    expected: Expected,
    charges: &[ChargePoint],
) -> SourceCase {
    SourceCase {
        declarations: format!(
            "{ACCOUNTING_UNITS}function f using v(): {result} pure {{ {expression} }}"
        ),
        expected,
        arguments: Vec::new(),
        quantity_parameter_arguments: Vec::new(),
        limits,
        expected_charges: Some(charges.to_vec()),
        check_mode: CheckMode::Linked,
    }
}

fn accounting_unit_case_with_quantity_parameter(
    result: &str,
    parameter: &str,
    expression: &str,
    magnitude: i64,
    limits: ScalarLimits,
    expected: Expected,
    charges: &[ChargePoint],
) -> SourceCase {
    let mut case = limited_function_with(
        result,
        parameter,
        expression,
        Vec::new(),
        limits,
        expected,
        charges,
    );
    case.declarations = format!("{ACCOUNTING_UNITS}{}", case.declarations);
    case.quantity_parameter_arguments
        .push((0, Rational::from_integer(Integer::from(magnitude))));
    case
}

fn unit_accounting_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    use ChargePoint::{
        UnitEdge, UnitIdentityRead, UnitRationalArithmetic, UnitResultRetain, UnitTargetDomain,
    };
    let quantity = UnitDomain::Declared;
    let compound = UnitDomain::Compound;
    Some(match vector.id {
        "U10" => vec![
            accounting_unit_case(
                "Quantity<m>",
                "convert<Quantity<m>>(100 cm)",
                unit_limits(8, 0, 0, 1, 1, 6, 1),
                quantity_expected(1, 1, quantity),
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<m>",
                "convert<Quantity<m>>(100 cm)",
                unit_limits(8, 0, 0, 1, 1, 6, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::ResultUnits,
                    limit: 0,
                    consumed: 0,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitResultRetain,
                },
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                ],
            ),
        ],
        "U13" => vec![accounting_unit_case(
            "Quantity<m ^ 18446744073709551616>",
            "2 m ^ 18446744073709551616",
            unit_limits(64, 0, 0, 0, 1, 5, 1),
            Expected::Incomplete {
                limit_kind: LimitKind::IntegerBits,
                limit: 64,
                consumed: 2,
                next_charge: "36893488147419103232".parse().expect("literal amount"),
                charge_point: UnitRationalArithmetic,
            },
            &[UnitIdentityRead],
        )],
        "U14" => {
            let bad = [
                "1 m + 1 s",
                "1 degC + 1 s",
                "1 degC + 1 degC",
                "1 degC / 0 degC",
                "1 cm + 1 m",
            ];
            bad.into_iter()
                .map(|expression| SourceCase {
                    declarations: format!(
                    "{ACCOUNTING_UNITS}function f using v(): Boolean pure {{ {expression} = {expression} }}"
                    ),
                    expected: Expected::Refused {
                        code: "ill_typed".to_owned(),
                        cause: Some("type-mismatch".to_owned()),
                    },
                    arguments: Vec::new(),
                    quantity_parameter_arguments: Vec::new(),
                    limits: unit_limits(0, 0, 0, 0, 0, 0, 0),
                    expected_charges: Some(Vec::new()),
                    check_mode: CheckMode::Linked,
                })
                .collect()
        }
        "U15" => vec![
            accounting_unit_case(
                "Quantity<cm>",
                "convert<Quantity<cm>>(1 inch)",
                unit_limits(15, 0, 0, 2, 1, 9, 1),
                quantity_expected(127, 50, quantity),
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<cm>",
                "convert<Quantity<cm>>(1 inch)",
                unit_limits(7, 0, 0, 1, 1, 9, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::UnitEdges,
                    limit: 1,
                    consumed: 1,
                    next_charge: Integer::from(2_i64),
                    charge_point: UnitEdge,
                },
                &[UnitIdentityRead, UnitEdge],
            ),
        ],
        "U16" => vec![
            accounting_unit_case(
                "Quantity<m, Decimal[-1000, 1000; 0, 2; exact]>",
                "convert<Quantity<m, Decimal[-1000, 1000; 0, 2; exact]>>(1 inch)",
                unit_limits(15, 5, 0, 1, 1, 6, 1),
                Expected::Refused {
                    code: "inexact_conversion".to_owned(),
                    cause: Some("inexact-conversion".to_owned()),
                },
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
            accounting_unit_case(
                "Quantity<m, Decimal[-1000, 1000; 0, 2; nearest-even]>",
                "convert<Quantity<m, Decimal[-1000, 1000; 0, 2; nearest-even]>>(1 inch)",
                unit_limits(15, 5, 0, 1, 1, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 4,
                    consumed: 4,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitTargetDomain,
                },
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
        ],
        "U17" => vec![
            accounting_unit_case(
                "Quantity<m, Decimal[-2, 2; 0, 0; exact]>",
                "convert<Quantity<m, Decimal[-2, 2; 0, 0; exact]>>(3 m)",
                unit_limits(2, 1, 0, 1, 1, 2, 0),
                Expected::Refused {
                    code: "decimal_out_of_domain".to_owned(),
                    cause: Some("outside-domain".to_owned()),
                },
                &[UnitIdentityRead, UnitTargetDomain],
            ),
            accounting_unit_case(
                "Quantity<m, Decimal[-2, 2; 0, 0; exact]>",
                "convert<Quantity<m, Decimal[-2, 2; 0, 0; exact]>>(3 m)",
                unit_limits(2, 1, 0, 1, 1, 1, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 1,
                    consumed: 1,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitTargetDomain,
                },
                &[UnitIdentityRead],
            ),
        ],
        "U18" => vec![accounting_unit_case(
            "Quantity<m, Decimal[0, 1; 0, 4294967295; exact]>",
            "convert<Quantity<m, Decimal[0, 1; 0, 4294967295; exact]>>(1 m)",
            unit_limits(u64::MAX, 64, 0, 1, 1, 2, 1),
            Expected::Incomplete {
                limit_kind: LimitKind::DecimalDigits,
                limit: 64,
                consumed: 0,
                next_charge: Integer::from(4_294_967_296_u64),
                charge_point: UnitTargetDomain,
            },
            &[UnitIdentityRead],
        )],
        "U19" => vec![
            accounting_unit_case(
                "Quantity<m>",
                "2 m + 3 m",
                unit_limits(4, 0, 0, 2, 2, 5, 1),
                quantity_expected(5, 1, quantity),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<m>",
                "2 m + 3 m",
                unit_limits(4, 0, 0, 2, 2, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 4,
                    consumed: 4,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitResultRetain,
                },
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                ],
            ),
        ],
        "U20" => {
            let result = accounting_unit_case(
                "Quantity<u2>",
                "1 u2 + 2 u2",
                unit_limits(4, 0, 0, 2, 2, 5, 1),
                quantity_expected(3, 1, quantity),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            );
            let affine = ["1 u1 + 2 u1", "1 u3 * 1 m"]
                .into_iter()
                .map(|expression| SourceCase {
                    declarations: format!(
                        "{ACCOUNTING_UNITS}function f using v(): Boolean pure {{ {expression} = {expression} }}"
                    ),
                    expected: Expected::Refused {
                        code: "ill_typed".to_owned(),
                        cause: Some("type-mismatch".to_owned()),
                    },
                    arguments: Vec::new(),
                    quantity_parameter_arguments: Vec::new(),
                    limits: unit_limits(0, 0, 0, 0, 0, 0, 0),
                    expected_charges: Some(Vec::new()),
                    check_mode: CheckMode::Linked,
                });
            std::iter::once(result).chain(affine).collect()
        }
        "U21" => vec![
            accounting_unit_case(
                "Quantity<m ^ 0>",
                "0 m ^ 0",
                unit_limits(1, 0, 0, 0, 1, 4, 1),
                quantity_expected(1, 1, compound),
                &[
                    UnitIdentityRead,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<m ^ -1>",
                "0 m ^ -1",
                unit_limits(1, 0, 0, 0, 1, 1, 0),
                Expected::Undefined,
                &[UnitIdentityRead],
            ),
            accounting_unit_case(
                "Quantity<m ^ -1>",
                "0 m ^ -1",
                unit_limits(1, 0, 0, 0, 1, 0, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 0,
                    consumed: 0,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitIdentityRead,
                },
                &[],
            ),
        ]
        .into_iter()
        .map(|mut case| {
            case.check_mode = CheckMode::Kernel;
            case
        })
        .collect(),
        "U22" => vec![
            SourceCase {
                declarations: format!(
                    "{ACCOUNTING_UNITS}function f using v(): Quantity<J> pure {{ convert<Quantity<J>>(1 N_m) }}"
                ),
                expected: Expected::Refused {
                    code: "ill_typed".to_owned(),
                    cause: Some("type-mismatch".to_owned()),
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: unit_limits(0, 0, 0, 0, 0, 0, 0),
                expected_charges: Some(Vec::new()),
                check_mode: CheckMode::Linked,
            },
            accounting_unit_case_with_quantity_parameter(
                "Quantity<m2>",
                "q: Quantity<m ^ 2>",
                "convert<Quantity<m2>>(q)",
                4,
                unit_limits(3, 0, 0, 0, 1, 3, 1),
                quantity_expected(4, 1, quantity),
                &[UnitIdentityRead, UnitTargetDomain, UnitResultRetain],
            ),
            accounting_unit_case_with_quantity_parameter(
                "Quantity<cm2>",
                "q: Quantity<m ^ 2>",
                "convert<Quantity<cm2>>(q)",
                4,
                unit_limits(17, 0, 0, 1, 1, 6, 1),
                quantity_expected(40000, 1, quantity),
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case_with_quantity_parameter(
                "Quantity<cm2>",
                "q: Quantity<m ^ 2>",
                "convert<Quantity<cm2>>(q)",
                4,
                unit_limits(16, 0, 0, 1, 1, 6, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: 16,
                    consumed: 5,
                    next_charge: Integer::from(17_i64),
                    charge_point: UnitRationalArithmetic,
                },
                &[UnitIdentityRead, UnitEdge, UnitRationalArithmetic],
            ),
        ],
        "U23" => vec![
            accounting_unit_case(
                "Boolean",
                "1 degC < 2 degC",
                unit_limits(15, 0, 0, 2, 2, 9, 1),
                Expected::Boolean(true),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Boolean",
                "1 degC < 2 degC",
                unit_limits(15, 0, 0, 2, 2, 8, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 8,
                    consumed: 8,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitResultRetain,
                },
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
            accounting_unit_case(
                "Boolean",
                "1 rev < 2 rev",
                UNLIMITED,
                Expected::Boolean(false),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Boolean",
                "1 degC = 1 degC",
                UNLIMITED,
                Expected::Boolean(true),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitResultRetain,
                ],
            ),
            SourceCase {
                declarations: format!(
                    "{ACCOUNTING_UNITS}function f using v(): Boolean pure {{ 0 degC < 32 degF }}"
                ),
                expected: Expected::Refused {
                    code: "ill_typed".to_owned(),
                    cause: Some("type-mismatch".to_owned()),
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: unit_limits(0, 0, 0, 0, 0, 0, 0),
                expected_charges: Some(Vec::new()),
                check_mode: CheckMode::Linked,
            },
            SourceCase {
                declarations: format!(
                    "{ACCOUNTING_UNITS}function f using v(): Boolean pure {{ 0 degC = 32 degF }}"
                ),
                expected: Expected::Refused {
                    code: "ill_typed".to_owned(),
                    cause: Some("type-mismatch".to_owned()),
                },
                arguments: Vec::new(),
                quantity_parameter_arguments: Vec::new(),
                limits: unit_limits(0, 0, 0, 0, 0, 0, 0),
                expected_charges: Some(Vec::new()),
                check_mode: CheckMode::Linked,
            },
        ],
        "U24" => vec![accounting_unit_case(
            "Quantity<cm>",
            "1 cm + 2 cm",
            unit_limits(4, 0, 0, 2, 2, 5, 1),
            quantity_expected(3, 1, quantity),
            &[
                UnitIdentityRead,
                UnitIdentityRead,
                UnitRationalArithmetic,
                UnitTargetDomain,
                UnitResultRetain,
            ],
        )],
        "U25" => vec![
            accounting_unit_case(
                "Quantity<kg*m^2*s^-2>",
                "convert<Quantity<kg*m^2*s^-2>>(1 N_m)",
                unit_limits(1, 0, 0, 1, 1, 3, 1),
                quantity_expected(1, 1, compound),
                &[UnitIdentityRead, UnitTargetDomain, UnitResultRetain],
            ),
            accounting_unit_case_with_quantity_parameter(
                "Quantity<J>",
                "q: Quantity<kg*m^2*s^-2>",
                "convert<Quantity<J>>(q)",
                1,
                unit_limits(1, 0, 0, 1, 1, 3, 1),
                quantity_expected(1, 1, quantity),
                &[UnitIdentityRead, UnitTargetDomain, UnitResultRetain],
            ),
        ],
        "U26" => vec![
            accounting_unit_case(
                "Quantity<u1>",
                "convert<Quantity<u1>>(1 u2)",
                unit_limits(6, 0, 0, 3, 1, 12, 1),
                quantity_expected(-9, 1, quantity),
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<u1>",
                "convert<Quantity<u1>>(1 u2)",
                unit_limits(6, 0, 0, 2, 1, 12, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::UnitEdges,
                    limit: 2,
                    consumed: 2,
                    next_charge: Integer::from(3_i64),
                    charge_point: UnitEdge,
                },
                &[UnitIdentityRead, UnitEdge, UnitEdge],
            ),
        ],
        "U27" => vec![
            accounting_unit_case(
                "Quantity<m ^ 0>",
                "1 cm / 0 cm",
                unit_limits(1, 0, 0, 2, 2, 2, 0),
                Expected::Undefined,
                &[UnitIdentityRead, UnitIdentityRead],
            ),
            accounting_unit_case(
                "Quantity<m ^ 0>",
                "1 cm / 0 cm",
                unit_limits(1, 0, 0, 2, 2, 1, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 1,
                    consumed: 1,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitIdentityRead,
                },
                &[UnitIdentityRead],
            ),
        ]
        .into_iter()
        .map(|mut case| {
            case.check_mode = CheckMode::Kernel;
            case
        })
        .collect(),
        "U28" => vec![
            accounting_unit_case(
                "Quantity<m ^ 2>",
                "1 cm * 1 inch",
                unit_limits(20, 0, 2, 2, 2, 11, 1),
                quantity_expected(127, 500000, compound),
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitTargetDomain,
                    UnitResultRetain,
                ],
            ),
            accounting_unit_case(
                "Quantity<m ^ 2>",
                "1 cm * 1 inch",
                unit_limits(13, 0, 2, 2, 2, 11, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: 13,
                    consumed: 9,
                    next_charge: Integer::from(14_i64),
                    charge_point: UnitRationalArithmetic,
                },
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
            accounting_unit_case(
                "Quantity<m ^ 2>",
                "1 cm * 1 inch",
                unit_limits(19, 0, 2, 2, 2, 11, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::IntegerBits,
                    limit: 19,
                    consumed: 15,
                    next_charge: Integer::from(20_i64),
                    charge_point: UnitRationalArithmetic,
                },
                &[
                    UnitIdentityRead,
                    UnitIdentityRead,
                    UnitEdge,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
        ],
        "U29" => vec![
            accounting_unit_case(
                "Quantity<m, Int[-2, 2; exact]>",
                "convert<Quantity<m, Int[-2, 2; exact]>>(1 inch)",
                unit_limits(15, 0, 0, 1, 1, 4, 1),
                Expected::Refused {
                    code: "inexact_conversion".to_owned(),
                    cause: Some("inexact-conversion".to_owned()),
                },
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
            accounting_unit_case(
                "Quantity<m, Int[-2, 2; nearest-even]>",
                "convert<Quantity<m, Int[-2, 2; nearest-even]>>(1 inch)",
                unit_limits(15, 0, 0, 1, 1, 4, 1),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 4,
                    consumed: 4,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitTargetDomain,
                },
                &[
                    UnitIdentityRead,
                    UnitEdge,
                    UnitRationalArithmetic,
                    UnitRationalArithmetic,
                ],
            ),
            accounting_unit_case(
                "Quantity<m, Int[-2, 2]>",
                "convert<Quantity<m, Int[-2, 2]>>(3 m)",
                unit_limits(2, 0, 0, 1, 1, 2, 0),
                Expected::Refused {
                    code: "integer_out_of_domain".to_owned(),
                    cause: Some("outside-domain".to_owned()),
                },
                &[UnitIdentityRead, UnitTargetDomain],
            ),
            accounting_unit_case(
                "Quantity<m, Int[-2, 2]>",
                "convert<Quantity<m, Int[-2, 2]>>(3 m)",
                unit_limits(2, 0, 0, 1, 1, 1, 0),
                Expected::Incomplete {
                    limit_kind: LimitKind::WorkUnits,
                    limit: 1,
                    consumed: 1,
                    next_charge: Integer::from(1_i64),
                    charge_point: UnitTargetDomain,
                },
                &[UnitIdentityRead],
            ),
        ],
        _ => return None,
    })
}

fn source_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    match vector.test_case {
        TestCase::Tc185 => decimal_accounting_cases(vector).or_else(|| decimal_cases(vector)),
        TestCase::Tc186 => text_accounting_cases(vector).or_else(|| text_cases(vector)),
        TestCase::Tc187 => unit_accounting_cases(vector).or_else(|| unit_cases(vector)),
        TestCase::Tc192 => division_accounting_cases(vector).or_else(|| division_cases(vector)),
        TestCase::Tc193 => float_accounting_cases(vector)
            .or_else(|| float_extended_cases(vector))
            .or_else(|| float_cases(vector)),
    }
}

struct Observation {
    result: Expected,
    node_charges: Vec<ChargePoint>,
    function_call_charges: usize,
    structural_function_call_charges: usize,
    charge_log: Vec<ChargeProjection>,
}

#[derive(Debug, PartialEq, Eq)]
struct ChargeProjection {
    point: ChargePoint,
    sizes: Vec<(LimitKind, Integer)>,
}

fn charge_projection(point: ChargePoint, sizes: &[(LimitKind, u64)]) -> ChargeProjection {
    ChargeProjection {
        point,
        sizes: sizes
            .iter()
            .map(|(kind, size)| (*kind, Integer::from(*size)))
            .collect(),
    }
}

fn structural_function_call_charges(graph: &CheckedGraph, entry_index: usize) -> usize {
    fn called_function_charges(
        graph: &CheckedGraph,
        function_index: usize,
        active: &mut Vec<usize>,
    ) -> usize {
        assert!(
            !active.contains(&function_index),
            "qualification source call graph is acyclic"
        );
        active.push(function_index);
        let function = graph
            .function_state(function_index)
            .expect("checked call target has a source body");
        let mut local_calls = Vec::new();
        let mut imported_calls = 0_usize;
        let mut pending = vec![function.body];
        while let Some(node) = pending.pop() {
            match node.kind() {
                filament_checked::NodeKind::Call { function, .. } => local_calls.push(function),
                filament_checked::NodeKind::ImportedCall { .. } => imported_calls += 1,
                _ => {}
            }
            pending.extend(node.children());
        }
        let count = 1
            + imported_calls
            + local_calls
                .into_iter()
                .map(|callee| called_function_charges(graph, *callee, active))
                .sum::<usize>();
        active.pop();
        count
    }

    called_function_charges(graph, entry_index, &mut Vec::new())
}

/// The cross-package source adapter for DIV-17/18 lives in
/// `tests/it/checked_package_state_core.rs`, where the front-end exposes the
/// operator's source region for state-clause refusals.
fn has_state_clause_adapter(vector: &QualificationVector) -> bool {
    vector.test_case == TestCase::Tc192 && matches!(vector.id, "DIV-17" | "DIV-18")
}

/// T04, T05 and T13 take runtime payloads through value admission. They are
/// not source-literal checks, which FR-056-AC-52 specifies separately.
fn compare_text_admission(vector: &QualificationVector) -> Result<usize, String> {
    let rows: Vec<(&str, u64, u64, TextProfile, Result<(), (&str, &str)>)> = match vector.id {
        "T04" => {
            assert!(vector.expected.contains("scalar-profile value succeeds"));
            assert!(vector.expected.contains("text_length_out_of_domain"));
            vec![
                ("é", 1, 1, TextProfile::Nfc, Ok(())),
                (
                    "é",
                    1,
                    1,
                    TextProfile::BinaryUtf8,
                    Err(("text_length_out_of_domain", "outside-domain")),
                ),
            ]
        }
        "T05" => {
            assert!(vector
                .expected
                .contains("empty and exact-bound inputs succeed"));
            assert!(vector.expected.contains("text_length_out_of_domain"));
            vec![
                ("", 0, 0, TextProfile::Nfc, Ok(())),
                ("a", 1, 1, TextProfile::Nfc, Ok(())),
                (
                    "a",
                    0,
                    0,
                    TextProfile::Nfc,
                    Err(("text_length_out_of_domain", "outside-domain")),
                ),
            ]
        }
        "T13" => {
            assert!(vector
                .expected
                .contains("nfc` admits at normalized length one"));
            assert!(vector
                .expected
                .contains("nfd` and `unicode-scalars` refuse"));
            vec![
                ("e\u{301}", 1, 1, TextProfile::Nfc, Ok(())),
                (
                    "e\u{301}",
                    1,
                    1,
                    TextProfile::Nfd,
                    Err(("text_length_out_of_domain", "outside-domain")),
                ),
                (
                    "e\u{301}",
                    1,
                    1,
                    TextProfile::UnicodeScalars,
                    Err(("text_length_out_of_domain", "outside-domain")),
                ),
                ("é", 2, 2, TextProfile::Nfd, Ok(())),
            ]
        }
        _ => return Err(format!("{} is not a text-admission vector", vector.id)),
    };

    for (index, (text, min, max, profile, expected)) in rows.iter().enumerate() {
        let payload = TextPayload::from_utf8(text.as_bytes())
            .map_err(|error| format!("{}[{index}] invalid fixture UTF-8: {error:?}", vector.id))?;
        let text_type = TextType::new(*min, *max, *profile)
            .map_err(|error| format!("{}[{index}] invalid text bounds: {error:?}", vector.id))?;
        let actual = admit_text(&payload, &text_type, &mut Meter::new(UNLIMITED));
        match (expected, actual) {
            (Ok(()), Outcome::Completed(_)) => {}
            (Err((code, cause)), Outcome::Refused(refusal))
                if refusal.code() == Some(*code) && refusal.cause() == Some(*cause) => {}
            (expected, actual) => {
                return Err(format!(
                    "TC-186:{}[{index}] expected value-admission {expected:?}, got {actual:?}",
                    vector.id
                ));
            }
        }
    }
    Ok(rows.len())
}

fn observed(case: &SourceCase) -> Observation {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{}",
        case.declarations
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qualification", "source"),
        "vector.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source intake");
    if !parsed.is_admissible() {
        let diagnostic = parsed
            .diagnostics()
            .first()
            .expect("refused source diagnostic");
        return Observation {
            result: Expected::Refused {
                code: diagnostic.code.as_str().to_owned(),
                cause: None,
            },
            node_charges: Vec::new(),
            function_call_charges: 0,
            structural_function_call_charges: 0,
            charge_log: Vec::new(),
        };
    }
    let unit = fsl_forms::build_unit(&parsed).expect("forms of admitted source");
    let mut declarations = match PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qualification-source".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    ) {
        Ok(declarations) => declarations,
        Err(errors) => {
            let error = errors.errors.first().expect("assembly refusal");
            return Observation {
                result: Expected::Refused {
                    code: error.cause.code().as_str().to_owned(),
                    cause: Some(error.cause.catalog_code().cause().to_owned()),
                },
                node_charges: Vec::new(),
                function_call_charges: 0,
                structural_function_call_charges: 0,
                charge_log: Vec::new(),
            };
        }
    };
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("public IEEE definition selection"),
    );
    let checked = match declarations.check(
        case.check_mode,
        CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    ) {
        Ok(checked) => checked,
        Err(errors) => {
            let error = errors.first().expect("checking refusal");
            return Observation {
                result: Expected::Refused {
                    code: error.cause.code().as_str().to_owned(),
                    cause: error.cause.cause().map(str::to_owned),
                },
                node_charges: Vec::new(),
                function_call_charges: 0,
                structural_function_call_charges: 0,
                charge_log: Vec::new(),
            };
        }
    };
    if case.expected == Expected::Admitted {
        return Observation {
            result: Expected::Admitted,
            node_charges: Vec::new(),
            function_call_charges: 0,
            structural_function_call_charges: 0,
            charge_log: Vec::new(),
        };
    }
    let package = CheckedPackage::link(checked);
    let entry_index = (0..)
        .find(|index| {
            package
                .graph()
                .function_state(*index)
                .is_some_and(|function| function.name == "f")
        })
        .expect("checked source function f exists");
    let structural_function_call_charges =
        structural_function_call_charges(package.graph(), entry_index);
    let mut limits = case.limits;
    if case.expected_charges.is_some() && limits.work_units != u64::MAX {
        let source_function_calls = structural_function_call_charges as u64;
        limits.work_units = limits
            .work_units
            .checked_add(source_function_calls)
            .expect("source function call work charges fit");
    }
    let mut meter = Meter::new(limits);
    let callable = package.graph().callable("f").expect("function f exists");
    let mut arguments = case.arguments.clone();
    for (index, magnitude) in &case.quantity_parameter_arguments {
        assert_eq!(*index, arguments.len(), "quantity arguments are in order");
        let (_, parameter_type) = callable
            .parameters
            .get(*index)
            .expect("quantity argument has a function parameter");
        let filament_exact::ValueType::Quantity(unit) = parameter_type else {
            panic!("parameter {index} is a quantity");
        };
        arguments.push(Value::Quantity(Quantity::new(
            magnitude.clone(),
            unit.clone(),
        )));
    }
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("function name"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("checked function invocation");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("value outcome: {:?}", evaluation.outcome);
    };
    let result = match outcome {
        Outcome::Completed(Value::Boolean(value)) => Expected::Boolean(value),
        Outcome::Completed(Value::Integer(value)) => Expected::Integer(value),
        Outcome::Completed(Value::Decimal(value)) => {
            let losses = evaluation
                .losses
                .iter()
                .map(|loss| {
                    let ValueLoss::Decimal(loss) = &loss.loss else {
                        panic!("decimal loss, got {:?}", loss.loss);
                    };
                    DecimalLoss {
                        numerator: loss.exact_numerator().clone(),
                        denominator: loss.exact_denominator(),
                        coefficient: loss.rounded_coefficient().clone(),
                        scale: loss.rounded_scale(),
                        mode: loss.mode().as_str().to_owned(),
                    }
                })
                .collect();
            Expected::Decimal {
                value: value.normalized().clone(),
                losses,
            }
        }
        Outcome::Completed(Value::Rational(value)) => Expected::Rational {
            numerator: value.numerator().clone(),
            denominator: value.denominator().clone(),
        },
        Outcome::Completed(Value::Float(value)) => {
            let mut flags: Vec<String> = evaluation
                .losses
                .iter()
                .flat_map(|loss| match loss.loss {
                    ValueLoss::IeeeFlags(flags) => {
                        flags.iter().map(|flag| flag.as_str().to_owned()).collect()
                    }
                    _ => Vec::new(),
                })
                .collect();
            flags.sort();
            Expected::Float {
                bits: value.bits(),
                flags,
            }
        }
        Outcome::Completed(Value::Quantity(value)) => Expected::Quantity {
            numerator: value.magnitude().numerator().clone(),
            denominator: value.magnitude().denominator().clone(),
            unit_domain: value.unit().domain(),
        },
        Outcome::Refused(refusal) => {
            let mut flags: Vec<String> = evaluation
                .losses
                .iter()
                .flat_map(|loss| match &loss.loss {
                    ValueLoss::IeeeFlags(flags) => {
                        flags.iter().map(|flag| flag.as_str().to_owned()).collect()
                    }
                    _ => Vec::new(),
                })
                .collect();
            flags.sort();
            assert!(
                flags.is_empty()
                    || evaluation
                        .losses
                        .iter()
                        .all(|loss| matches!(&loss.loss, ValueLoss::IeeeFlags(_))),
                "IEEE refusal exposes only its would-be flags: {:?}",
                evaluation.losses
            );
            if flags.is_empty() {
                assert!(
                    evaluation.losses.is_empty(),
                    "refusal exposes no other losses"
                );
                Expected::Refused {
                    code: refusal.code().expect("refusal code").to_owned(),
                    cause: refusal.cause().map(str::to_owned),
                }
            } else {
                Expected::IeeeRefused {
                    code: refusal.code().expect("refusal code").to_owned(),
                    cause: refusal.cause().map(str::to_owned),
                    flags,
                }
            }
        }
        Outcome::Incomplete(mut incomplete) => {
            if case.expected_charges.is_some() && incomplete.limit_kind == LimitKind::WorkUnits {
                let source_calls = meter
                    .admitted_charges()
                    .iter()
                    .filter(|charge| charge.point == ChargePoint::FunctionCall)
                    .count() as u64;
                incomplete.limit -= source_calls;
                incomplete.consumed -= source_calls;
            }
            Expected::Incomplete {
                limit_kind: incomplete.limit_kind,
                limit: incomplete.limit,
                consumed: incomplete.consumed,
                next_charge: incomplete.next_charge,
                charge_point: incomplete.charge_point,
            }
        }
        Outcome::Undefined(_) => Expected::Undefined,
        other => panic!("qualification result has an unexpected shape: {other:?}"),
    };
    let node_charges = meter
        .admitted_charges()
        .iter()
        .filter(|charge| charge.point != ChargePoint::FunctionCall)
        .map(|charge| charge.point)
        .collect();
    let function_call_charges = meter
        .admitted_charges()
        .iter()
        .filter(|charge| charge.point == ChargePoint::FunctionCall)
        .count();
    let charge_log = meter
        .admitted_charges()
        .iter()
        .map(|charge| ChargeProjection {
            point: charge.point,
            sizes: [
                (LimitKind::IntegerBits, charge.integer_bits),
                (LimitKind::DecimalDigits, charge.decimal_digits),
                (LimitKind::ScaleExpansion, charge.scale_expansion),
                (LimitKind::TextInputBytes, charge.text_input_bytes),
                (LimitKind::TextScalars, charge.text_scalars),
                (LimitKind::NormalizedScalars, charge.normalized_scalars),
                (LimitKind::UnitEdges, charge.unit_edges),
                (LimitKind::ValueOccurrences, charge.value_occurrences),
            ]
            .into_iter()
            .filter_map(|(kind, size)| size.map(|size| (kind, Integer::from(size))))
            .collect(),
        })
        .collect();
    Observation {
        result,
        node_charges,
        function_call_charges,
        structural_function_call_charges,
        charge_log,
    }
}

fn compare(vector: &QualificationVector) -> Result<usize, String> {
    if vector.test_case == TestCase::Tc186 && matches!(vector.id, "T04" | "T05" | "T13") {
        return compare_text_admission(vector);
    }
    let cases = source_cases(vector).expect("this vector has a source adapter");
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let actual = observed(case);
        let matches = case.expected == actual.result;
        let charges_match = case
            .expected_charges
            .as_ref()
            .is_none_or(|expected| *expected == actual.node_charges);
        let function_calls_match =
            actual.function_call_charges == actual.structural_function_call_charges;
        if !matches || !charges_match || !function_calls_match {
            failures.push(format!(
                "{}:{}[{index}] expected {:?} with node charges {:?} and {} structural FunctionCall charges, actual {:?} with node charges {:?} and {} FunctionCall charges; source {}",
                vector.test_case.id(),
                vector.id,
                case.expected,
                case.expected_charges,
                actual.structural_function_call_charges,
                actual.result,
                actual.node_charges,
                actual.function_call_charges,
                case.declarations,
            ));
        }
    }
    if failures.is_empty() {
        Ok(cases.len())
    } else {
        Err(failures.join("\n"))
    }
}

fn vector(id: &str) -> &'static QualificationVector {
    QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.id == id)
        .expect("published vector identity")
}

macro_rules! source_vectors {
    ($requirement:literal; $( $test:ident => $id:literal ),+ $(,)?) => {$ (
        #[trace($requirement)]
        #[test]
        fn $test() { compare(vector($id)).unwrap_or_else(|failure| panic!("{failure}")); }
    )+};
}

source_vectors!("FSpec-TC-185"; d01=>"D01", d02=>"D02", d04=>"D04", d05=>"D05", d10=>"D10", d26=>"D26", d27=>"D27");
source_vectors!("FSpec-TC-186"; t01=>"T01", t02=>"T02", t03=>"T03", t05b=>"T05b", t06b=>"T06b", t12=>"T12");
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-4")]
#[test]
fn t04_text_length_uses_value_admission() {
    compare(vector("T04")).unwrap_or_else(|failure| panic!("{failure}"));
}
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-4")]
#[test]
fn t05_text_length_uses_value_admission() {
    compare(vector("T05")).unwrap_or_else(|failure| panic!("{failure}"));
}
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-4")]
#[test]
fn t13_normalized_text_length_uses_value_admission() {
    compare(vector("T13")).unwrap_or_else(|failure| panic!("{failure}"));
}
source_vectors!("FSpec-TC-187"; u11a=>"U11a", u11b=>"U11b", u11c=>"U11c", u11f=>"U11f", u11h=>"U11h", u11g1=>"U11g1", u11g2=>"U11g2", u11g3=>"U11g3", u11g4=>"U11g4", u11n=>"U11n", u11o=>"U11o", u11w=>"U11w");
source_vectors!("FSpec-TC-192"; div02=>"DIV-02", div04=>"DIV-04");
source_vectors!("FSpec-TC-193"; f03=>"F03", f04=>"F04", f07=>"F07", f08=>"F08", f10=>"F10", f11=>"F11", f15=>"F15", f16=>"F16");

macro_rules! accounting_source_vectors {
    ($owner:literal, $test_case:literal; $( $test:ident => $id:literal ),+ $(,)?) => {$ (
        #[trace($owner, $test_case)]
        #[test]
        fn $test() { compare_accounting(vector($id)).unwrap_or_else(|failure| panic!("{failure}")); }
    )+};
}

#[trace("FR-397-AC-27", "FSpec-TC-193")]
#[test]
fn f32_negation_source_vectors_cover_each_profile_and_meter_boundary() {
    let limits = |integer_bits, value_occurrences, work_units, result_units| ScalarLimits {
        integer_bits,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences,
        work_units,
        result_units,
    };
    let patterns = [
        (0x0000_0000_u32, 0x8000_0000_u32),
        (0x8000_0000, 0x0000_0000),
        (0x0000_0001, 0x8000_0001),
        (0x3f80_0000, 0xbf80_0000),
        (0x7f80_0000, 0xff80_0000),
        (0x7fc0_0012, 0xffc0_0012),
        (0x7f80_0001, 0xff80_0001),
    ];
    let mut count = 0;
    for mode in ["nearest-even", "toward-zero", "exact"] {
        let ty = format!("Float32[{mode}]");
        for (input, negated) in patterns {
            let value = Value::Float(IeeeValue::binary32(input));
            for (body, expected_bits, operations) in [
                ("-a".to_owned(), u64::from(negated), 1_u64),
                ("-(-a)".to_owned(), u64::from(input), 2),
            ] {
                let mut points = Vec::new();
                for _ in 0..operations {
                    points.extend([ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]);
                }
                let cases = [
                    limited_function_with(
                        &ty,
                        &format!("a: {ty}"),
                        &body,
                        vec![value.clone()],
                        limits(32 * operations, operations, operations * 2, operations),
                        Expected::Float {
                            bits: expected_bits,
                            flags: Vec::new(),
                        },
                        &points,
                    ),
                    limited_function_with(
                        &ty,
                        &format!("a: {ty}"),
                        &body,
                        vec![value.clone()],
                        limits(32 * operations, operations, 1, 1),
                        Expected::Incomplete {
                            limit_kind: LimitKind::WorkUnits,
                            limit: 1,
                            consumed: 1,
                            next_charge: Integer::from(1_u64),
                            charge_point: ChargePoint::IeeeResultRetain,
                        },
                        &[ChargePoint::IeeeOperands],
                    ),
                    limited_function_with(
                        &ty,
                        &format!("a: {ty}"),
                        &body,
                        vec![value.clone()],
                        limits(32 * operations, 0, operations * 2, operations),
                        Expected::Incomplete {
                            limit_kind: LimitKind::ValueOccurrences,
                            limit: 0,
                            consumed: 0,
                            next_charge: Integer::from(1_u64),
                            charge_point: ChargePoint::IeeeOperands,
                        },
                        &[],
                    ),
                ];
                for case in cases {
                    let actual = observed(&case);
                    assert_eq!(
                        actual.result, case.expected,
                        "{mode}, {input:#010x}, {body}"
                    );
                    assert_eq!(
                        actual.node_charges,
                        *case.expected_charges.as_ref().unwrap(),
                        "{mode}, {input:#010x}, {body}"
                    );
                    assert_eq!(
                        actual.function_call_charges, 1,
                        "one authored source call per case"
                    );
                    assert_eq!(
                        actual.structural_function_call_charges, 1,
                        "one checked function invocation per case"
                    );
                    let expected_log: Vec<_> = case
                        .expected_charges
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|point| match point {
                            ChargePoint::IeeeOperands => charge_projection(
                                *point,
                                &[
                                    (LimitKind::IntegerBits, 32),
                                    (LimitKind::ValueOccurrences, 1),
                                ],
                            ),
                            ChargePoint::IeeeResultRetain => charge_projection(*point, &[]),
                            other => panic!("unexpected F32 node charge {other:?}"),
                        })
                        .collect();
                    let actual_node_log: Vec<_> = actual
                        .charge_log
                        .into_iter()
                        .filter(|charge| charge.point != ChargePoint::FunctionCall)
                        .collect();
                    assert_eq!(
                        actual_node_log, expected_log,
                        "{mode}, {input:#010x}, {body}"
                    );
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 126);
}

fn compare_accounting(vector: &QualificationVector) -> Result<usize, String> {
    let cases = source_cases(vector).expect("accounting vector has a declared adapter");
    assert!(!cases.is_empty(), "{} must have cases", vector.id);
    let result_only = matches!(vector.id, "F23" | "F29");
    let budgeted_cases = cases
        .iter()
        .filter(|case| case.expected_charges.is_some())
        .count();
    for (index, case) in cases.iter().enumerate() {
        if result_only {
            assert!(
                case.expected_charges.is_none(),
                "{}[{index}] has no row budget",
                vector.id
            );
        } else if case.expected_charges.is_some() {
            assert!(
                !case.expected_charges.as_ref().unwrap().is_empty()
                    || matches!(
                        case.expected,
                        Expected::Refused { .. } | Expected::Incomplete { .. }
                    ),
                "{}[{index}] has an explicit charge expectation",
                vector.id
            );
        }
    }
    if !result_only {
        assert!(
            budgeted_cases > 0,
            "{} must include a tuple-budget case",
            vector.id
        );
    }
    compare(vector)
}

#[trace("FR-397-AC-28", "FSpec-TC-193")]
#[test]
fn f17_source_charge_log_matches_literal_node_events_and_comparison_oracle() {
    let cases = source_cases(vector("F17")).expect("F17 source adapter");
    assert_eq!(cases.len(), 24);
    for (index, case) in cases.iter().enumerate() {
        let actual = observed(case);
        assert_eq!(
            actual.function_call_charges,
            actual.structural_function_call_charges
        );
        let width = case.limits.integer_bits;
        let mut expected = vec![charge_projection(ChargePoint::FunctionCall, &[])];
        for point in case.expected_charges.as_ref().expect("F17 row budget") {
            expected.push(match point {
                ChargePoint::IeeeOperands => charge_projection(
                    *point,
                    &[
                        (LimitKind::IntegerBits, width),
                        (LimitKind::ValueOccurrences, 2),
                    ],
                ),
                ChargePoint::IeeeResultRetain | ChargePoint::BooleanResultRetain => {
                    charge_projection(*point, &[])
                }
                other => panic!("unexpected F17 node charge {other:?}"),
            });
        }
        assert_eq!(actual.charge_log, expected, "F17 case {index}");
        if matches!(&case.expected, Expected::Boolean(_)) {
            let (Value::Float(qn), Value::Float(one)) = (&case.arguments[0], &case.arguments[1])
            else {
                panic!("F17 arguments are IEEE values")
            };
            let (left, right) = if case.declarations.contains("one = one") {
                (one, one)
            } else {
                (qn, one)
            };
            let expected_equal = compare_ieee(
                IeeeComparison::NumericEqual,
                *left,
                *right,
                &mut Meter::new(case.limits),
            )
            .expect("same-width F17 values");
            let expected_value = match expected_equal {
                Outcome::Completed(value) => value,
                other => panic!("complete F17 tuple gives a comparison: {other:?}"),
            };
            let source_value = match &case.expected {
                Expected::Boolean(value) => *value,
                _ => unreachable!(),
            };
            if case.declarations.contains("qn != one") {
                assert_eq!(source_value, !expected_value);
            } else {
                assert_eq!(source_value, expected_value);
            }
        }
    }
}

struct T16ProfileRow {
    profile: TextProfile,
    input: &'static str,
    retained: &'static str,
    exact_length: u64,
    refused_charges: &'static [ChargePoint],
    success_charges: &'static [ChargePoint],
    incomplete_limits: ScalarLimits,
    incomplete: Incomplete,
}

#[trace("FSpec-FR-141-AC-4", "FSpec-FR-141-AC-6", "FSpec-TC-186")]
#[test]
fn t16_direct_text_type_admission_covers_each_profile_boundary() {
    use ChargePoint::{
        TextDecodeScalars, TextInputBytes, TextNormalizeInput, TextNormalizeOutput,
        TextResultRetain,
    };
    use LimitKind::{NormalizedScalars, TextInputBytes as InputBytes, TextScalars};
    use TextProfile::{BinaryUtf8, Nfc, Nfd, Nfkc, Nfkd, UnicodeScalars};

    let rows = [
        T16ProfileRow {
            profile: UnicodeScalars,
            input: "é",
            retained: "é",
            exact_length: 2,
            refused_charges: &[TextInputBytes, TextDecodeScalars],
            success_charges: &[TextInputBytes, TextDecodeScalars, TextResultRetain],
            incomplete_limits: ScalarLimits {
                text_scalars: 1,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: TextScalars,
                limit: 1,
                consumed: 0,
                next_charge: Integer::from(2_u64),
                charge_point: TextDecodeScalars,
            },
        },
        T16ProfileRow {
            profile: Nfc,
            input: "ab",
            retained: "ab",
            exact_length: 2,
            refused_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
            ],
            success_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextResultRetain,
            ],
            incomplete_limits: ScalarLimits {
                normalized_scalars: 1,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: NormalizedScalars,
                limit: 1,
                consumed: 1,
                next_charge: Integer::from(2_u64),
                charge_point: TextNormalizeOutput,
            },
        },
        T16ProfileRow {
            profile: Nfd,
            input: "é",
            retained: "é",
            exact_length: 2,
            refused_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
            ],
            success_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextResultRetain,
            ],
            incomplete_limits: ScalarLimits {
                normalized_scalars: 1,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: NormalizedScalars,
                limit: 1,
                consumed: 1,
                next_charge: Integer::from(2_u64),
                charge_point: TextNormalizeOutput,
            },
        },
        T16ProfileRow {
            profile: Nfkc,
            input: "ﬃ",
            retained: "ffi",
            exact_length: 3,
            refused_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextNormalizeOutput,
            ],
            success_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextResultRetain,
            ],
            incomplete_limits: ScalarLimits {
                normalized_scalars: 2,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: NormalizedScalars,
                limit: 2,
                consumed: 2,
                next_charge: Integer::from(3_u64),
                charge_point: TextNormalizeOutput,
            },
        },
        T16ProfileRow {
            profile: Nfkd,
            input: "é",
            retained: "é",
            exact_length: 2,
            refused_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
            ],
            success_charges: &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
                TextResultRetain,
            ],
            incomplete_limits: ScalarLimits {
                normalized_scalars: 1,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: NormalizedScalars,
                limit: 1,
                consumed: 1,
                next_charge: Integer::from(2_u64),
                charge_point: TextNormalizeOutput,
            },
        },
        T16ProfileRow {
            profile: BinaryUtf8,
            input: "é",
            retained: "é",
            exact_length: 2,
            refused_charges: &[TextInputBytes],
            success_charges: &[TextInputBytes, TextDecodeScalars, TextResultRetain],
            incomplete_limits: ScalarLimits {
                text_input_bytes: 1,
                ..UNLIMITED
            },
            incomplete: Incomplete {
                limit_kind: InputBytes,
                limit: 1,
                consumed: 0,
                next_charge: Integer::from(2_u64),
                charge_point: TextInputBytes,
            },
        },
    ];

    for row in rows {
        let payload = TextPayload::from_utf8(row.input.as_bytes()).expect("valid UTF-8 vector");
        let refused_type = TextType::new(1, 1, row.profile).expect("valid bounded Text type");
        let mut refused_meter = Meter::new(UNLIMITED);
        assert!(
            matches!(
                admit_text(&payload, &refused_type, &mut refused_meter),
                Outcome::Refused(Refusal::TextLengthOutOfDomain { .. })
            ),
            "{:?}",
            row.profile
        );
        assert_eq!(
            refused_meter
                .admitted_charges()
                .iter()
                .map(|charge| charge.point)
                .collect::<Vec<_>>(),
            row.refused_charges,
            "{:?}",
            row.profile
        );

        let exact_type = TextType::new(row.exact_length, row.exact_length, row.profile)
            .expect("exact-bound Text type");
        let mut success_meter = Meter::new(UNLIMITED);
        let Outcome::Completed(text) = admit_text(&payload, &exact_type, &mut success_meter) else {
            panic!("exact-bound {:?} text is admitted", row.profile);
        };
        assert_eq!(text.length(), row.exact_length, "{:?}", row.profile);
        assert_eq!(text.retained(), row.retained, "{:?}", row.profile);
        assert_eq!(
            success_meter
                .admitted_charges()
                .iter()
                .map(|charge| charge.point)
                .collect::<Vec<_>>(),
            row.success_charges,
            "{:?}",
            row.profile
        );

        let mut incomplete_meter = Meter::new(row.incomplete_limits);
        assert!(
            matches!(
                admit_text(&payload, &exact_type, &mut incomplete_meter),
                Outcome::Incomplete(actual) if actual == row.incomplete
            ),
            "{:?}",
            row.profile
        );
    }
}

accounting_source_vectors!("FSpec-FR-140-AC-6", "FSpec-TC-185"; d09_source_accounting=>"D09", d14_source_accounting=>"D14", d15_source_accounting=>"D15", d16_source_accounting=>"D16", d17_source_accounting=>"D17", d20_source_accounting=>"D20", d21_source_accounting=>"D21", d22_source_accounting=>"D22", d23_source_accounting=>"D23");
#[derive(Debug, PartialEq, Eq)]
enum MemberQualificationStatus {
    SourceExpressible,
    DirectTextAdmission,
    NonQualification {
        ac: &'static str,
        reason: &'static str,
    },
}

fn member_qualification_status(id: &str) -> MemberQualificationStatus {
    if id == "T16" {
        MemberQualificationStatus::DirectTextAdmission
    } else if id == "D19" {
        MemberQualificationStatus::NonQualification {
            ac: "FSpec-FR-140-AC-5",
            reason: "The pinned vector admits a Decimal boundary value whose nonzero coefficient at maximum scale cannot be formed as an FSL source value: the contextual literal is rejected as ill_typed/type-mismatch before evaluation, and passing the evaluator value through CheckedPackage::call is rejected as WrongValueKind.",
        }
    } else {
        MemberQualificationStatus::SourceExpressible
    }
}
accounting_source_vectors!("FSpec-FR-141-AC-6", "FSpec-TC-186"; t11_source_accounting=>"T11", t14_source_accounting=>"T14");
accounting_source_vectors!("FSpec-FR-141-AC-5", "FSpec-TC-186"; t15_source_accounting=>"T15");
// DIV-12's `modulo_out_of_domain` mismatch is owned by QSL-3058 / FR-147-AC-9,
// outside QSL-3060's owner map; do not qualify it under this ticket.
accounting_source_vectors!("FSpec-FR-147-AC-6", "FSpec-TC-192"; div08_source_accounting=>"DIV-08", div10_source_accounting=>"DIV-10", div11_source_accounting=>"DIV-11", div13_source_accounting=>"DIV-13");
accounting_source_vectors!("FR-397-AC-28", "FSpec-TC-193"; f17_source_accounting=>"F17");
accounting_source_vectors!("FSpec-FR-148-AC-6", "FSpec-TC-193"; f18_source_accounting=>"F18", f19_source_accounting=>"F19", f22_source_accounting=>"F22", f30_source_accounting=>"F30");
accounting_source_vectors!("FSpec-FR-148-AC-8", "FSpec-TC-193"; f21_source_accounting=>"F21");
accounting_source_vectors!("FR-397-AC-27", "FSpec-TC-193"; f33_source_accounting=>"F33");
accounting_source_vectors!("FSpec-FR-148-AC-8", "FSpec-TC-193"; f20_source_accounting=>"F20", f26_source_accounting=>"F26");
accounting_source_vectors!("FSpec-FR-148-AC-6", "FSpec-TC-193"; f23_source_accounting=>"F23", f29_source_accounting=>"F29");
accounting_source_vectors!("FR-397-AC-27", "FSpec-TC-193"; f24_source_accounting=>"F24", f25_source_accounting=>"F25", f27_source_accounting=>"F27", f28_source_accounting=>"F28", f31_source_accounting=>"F31");
accounting_source_vectors!("FSpec-FR-142-AC-7", "FSpec-TC-187"; u10_source_accounting=>"U10", u15_source_accounting=>"U15", u16_source_accounting=>"U16", u17_source_accounting=>"U17", u18_source_accounting=>"U18", u19_source_accounting=>"U19", u20_source_accounting=>"U20", u22_source_accounting=>"U22", u23_source_accounting=>"U23", u24_source_accounting=>"U24", u25_source_accounting=>"U25", u26_source_accounting=>"U26", u27_source_accounting=>"U27", u28_source_accounting=>"U28", u29_source_accounting=>"U29");
accounting_source_vectors!("FR-093-AC-58", "FSpec-TC-187"; u13_source_accounting=>"U13", u21_source_accounting=>"U21");
accounting_source_vectors!("FSpec-FR-142-AC-2", "FSpec-TC-187"; u14_source_accounting=>"U14");

/// Every QSL-3060 member has an explicit source status. Each scalar-limits
/// member's cases assert a literal result and node charge sequence; F23/F29
/// have no ScalarLimitsV1 tuple in their pinned rows, so their source result
/// is qualified without a synthetic budget tuple.
#[trace(
    "FSpec-TC-185",
    "FSpec-TC-186",
    "FSpec-TC-187",
    "FSpec-TC-192",
    "FSpec-TC-193"
)]
#[test]
fn qsl3060_members_have_explicit_source_qualification_status() {
    let members = [
        "D09", "D14", "D15", "D16", "D17", "D19", "D20", "D21", "D22", "D23", "T11", "T14", "T15",
        "T16", "U10", "U13", "U14", "U15", "U16", "U17", "U18", "U19", "U20", "U21", "U22", "U23",
        "U24", "U25", "U26", "U27", "U28", "U29", "DIV-08", "DIV-10", "DIV-11", "DIV-13", "F17",
        "F18", "F19", "F20", "F21", "F22", "F23", "F24", "F25", "F26", "F27", "F28", "F29", "F30",
        "F31", "F33",
    ];
    let tuple_members = [
        "D09", "D14", "D15", "D16", "D17", "D20", "D21", "D22", "D23", "T11", "T14", "T15", "U10",
        "U13", "U14", "U15", "U16", "U17", "U18", "U19", "U20", "U21", "U22", "U23", "U24", "U25",
        "U26", "U27", "U28", "U29", "DIV-08", "DIV-10", "DIV-11", "DIV-13", "F17", "F18", "F19",
        "F20", "F21", "F22", "F24", "F25", "F26", "F27", "F28", "F30", "F31", "F33",
    ];
    for id in members {
        match member_qualification_status(id) {
            MemberQualificationStatus::NonQualification { ac, reason } => {
                assert_eq!(id, "D19");
                assert_eq!(ac, "FSpec-FR-140-AC-5");
                assert!(reason.contains("WrongValueKind"));
                assert!(source_cases(vector(id)).is_none());
            }
            MemberQualificationStatus::DirectTextAdmission => {
                assert_eq!(id, "T16");
                assert!(source_cases(vector(id)).is_none());
            }
            MemberQualificationStatus::SourceExpressible => {
                let cases = source_cases(vector(id)).unwrap_or_else(|| {
                    panic!("{id} lacks a source adapter or non-qualification status")
                });
                if tuple_members.contains(&id) {
                    let budgeted = cases
                        .iter()
                        .filter(|case| case.expected_charges.is_some())
                        .collect::<Vec<_>>();
                    assert!(!budgeted.is_empty(), "{id} has tuple cases");
                    assert!(
                        budgeted.iter().all(|case| case.expected_charges.is_some()),
                        "{id} tuple cases have explicit node charge expectations"
                    );
                }
            }
        }
    }
}

#[trace("FSpec-TC-185", "FR-093-AC-56")]
#[test]
fn source_runner_applies_node_budget_after_function_call() {
    let rational = |n, d| {
        Value::Rational(
            Rational::new(Integer::from(n), Integer::from(d)).expect("nonzero denominator"),
        )
    };
    let case = limited_function_with(
        "Decimal[0, 100; 2, 2; nearest-even]",
        "q: Rational[0, 2; 1, 3]",
        "convert<Decimal[0, 100; 2, 2; nearest-even]>(q)",
        vec![rational(2_i64, 3_i64)],
        ScalarLimits {
            integer_bits: 9,
            decimal_digits: 3,
            scale_expansion: 2,
            text_input_bytes: 0,
            text_scalars: 0,
            normalized_scalars: 0,
            unit_edges: 0,
            value_occurrences: 2,
            work_units: 5,
            result_units: 1,
        },
        Expected::Decimal {
            value: DecimalRepresentation::new(Integer::from(67_i64), 2).normalized(),
            losses: vec![DecimalLoss {
                numerator: Integer::from(2_i64),
                denominator: Integer::from(3_i64),
                coefficient: Integer::from(67_i64),
                scale: 2,
                mode: "nearest-even".to_owned(),
            }],
        },
        &[
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
            ChargePoint::DecimalRounding,
            ChargePoint::DecimalResultRetain,
        ],
    );
    let actual = observed(&case);
    assert_eq!(actual.result, case.expected);
    assert_eq!(actual.node_charges, case.expected_charges.unwrap());
    assert_eq!(actual.function_call_charges, 1);
}

#[trace("FSpec-FR-140-AC-8", "FSpec-TC-185")]
#[test]
fn d13_source_form_refuses_at_type_check() {
    compare(vector("D13")).unwrap_or_else(|failure| panic!("D13: {failure}"));
}

#[trace("FSpec-FR-140-AC-8", "FSpec-TC-185")]
#[test]
fn d18_source_form_refuses_at_type_check() {
    compare(vector("D18")).unwrap_or_else(|failure| panic!("D18: {failure}"));
}

#[trace("FSpec-FR-140-AC-8", "FSpec-TC-185")]
#[test]
fn d28_checks_both_source_refusals_and_the_admitted_counterpart() {
    let cases = source_cases(vector("D28")).expect("D28 has a source adapter");
    assert_eq!(cases.len(), 3);
    for refusal in &cases[..2] {
        assert!(matches!(
            &refusal.expected,
            Expected::Refused { code, cause }
                if code == "ill_typed" && cause.as_deref() == Some("type-mismatch")
        ));
    }
    assert!(matches!(&cases[2].expected, Expected::Decimal { .. }));
    compare(vector("D28")).unwrap_or_else(|failure| panic!("D28: {failure}"));
}

#[trace("FSpec-TC-185")]
#[test]
fn an_altered_standard_expectation_reports_the_vector_identity() {
    let mut altered = *vector("D02");
    altered.expected = "exact `(999,0)` with no loss";
    let mismatch =
        compare(&altered).expect_err("real evaluator disagrees with altered expectation");
    assert!(mismatch.starts_with("TC-185:D02[0]"), "{mismatch}");
    assert!(mismatch.contains("999"), "{mismatch}");
}

#[test]
fn every_published_vector_has_an_explicit_source_qualification_status() {
    let mut sampled = Vec::new();
    let mut unqualified = Vec::new();
    let mut partial_samples = Vec::new();
    for vector in QUALIFICATION_VECTORS {
        let identity = format!("{}:{}", vector.test_case.id(), vector.id);
        if source_cases(vector).is_some()
            || has_state_clause_adapter(vector)
            || (vector.test_case == TestCase::Tc186 && matches!(vector.id, "T04" | "T05" | "T13"))
        {
            if vector.test_case == TestCase::Tc187
                || matches!(
                    vector.id,
                    "D01" | "D26" | "D27" | "T02" | "T06b" | "F07" | "F08" | "F10" | "F11"
                )
            {
                partial_samples.push(identity.clone());
            }
            sampled.push(identity);
        } else {
            let reason = match (vector.test_case, vector.id) {
                (TestCase::Tc186, "T06") => {
                    "invalid UTF-8 reader-boundary input, not a source value"
                }
                (TestCase::Tc192, "DIV-03" | "DIV-09") => {
                    "mutation of an already checked wire definition closure"
                }
                (TestCase::Tc192, "DIV-07" | "DIV-14") => {
                    "backend negotiation rather than source evaluation"
                }
                _ => "remaining source/accounting adapter not implemented",
            };
            unqualified.push(format!("{identity}: {reason}"));
        }
    }
    assert!(!sampled.is_empty(), "zero source samples cannot pass");
    assert_eq!(
        sampled.len() + unqualified.len(),
        QUALIFICATION_VECTORS.len()
    );
    eprintln!(
        "source samples: {} rows, {} without an adapter; partially sampled rows: {}; unqualified ids/reasons: {}",
        sampled.len(),
        unqualified.len(),
        partial_samples.join(", "),
        unqualified.join(", ")
    );
    if std::env::var_os("FSL_FULL_SOURCE_QUALIFICATION").is_some() {
        assert!(
            unqualified.is_empty() && partial_samples.is_empty(),
            "full source qualification is incomplete; partial samples: {}; no adapter: {}",
            partial_samples.join(", "),
            unqualified.join(", ")
        );
    }
}
