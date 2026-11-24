// SPDX-License-Identifier: AGPL-3.0-or-later
//! Published qualification vectors executed through authored FSL source.
//! Wire-only obligations and rows without a source adapter remain explicit;
//! they are not successful source qualifications.

use filament_eval::FamilyOutcome;
use filament_exact::{DecimalRepresentation, Integer, Meter, Outcome, ScalarLimits, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::loss::ValueLoss;
use filament_specification::{QualificationVector, TestCase, QUALIFICATION_VECTORS};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
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

/// Marks a declaration list to be checked under `CheckMode::Kernel`, the mode
/// in which the evaluator, not the checker, meets a zero divisor.
const KERNEL: &str = "#kernel\n";

/// Marks a declaration list to be written under the state-only facet.
const STATE_ONLY: &str = "#state-only\n";

#[derive(Debug, PartialEq)]
enum Expected {
    Boolean(bool),
    Integer(Integer),
    Decimal {
        value: DecimalRepresentation,
        losses: Vec<DecimalLoss>,
    },
    Float {
        bits: u64,
        flags: Vec<String>,
    },
    Refused {
        code: String,
        cause: Option<String>,
    },
    Admitted,
    Undefined,
    /// A refusal whose authored cell names no code.
    RefusedAny,
    Quantity {
        magnitude: String,
        losses: Vec<DecimalLoss>,
    },
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
}

fn function(result: &str, body: &str, expected: Expected) -> SourceCase {
    SourceCase {
        declarations: format!("function f using v(): {result} pure {{ {body} }}"),
        expected,
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

/// The backtick-quoted `Decimal[..]` types of an authored cell, in order.
fn decimal_types(text: &str) -> Vec<&str> {
    quoted(text)
        .into_iter()
        .filter(|token| token.starts_with("Decimal["))
        .collect()
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
                    refusal_expected(vector.expected)
                },
            })
            .collect(),
        "D03" => {
            let operands = pairs(vector.inputs);
            let (numerators, denominator) = {
                let tail = vector
                    .expected
                    .split_once("exact_numerator:")
                    .expect("authored loss numerators")
                    .1;
                let numerators: Vec<Integer> = tail
                    .split(',')
                    .next()
                    .expect("numerators")
                    .trim()
                    .split('|')
                    .map(|n| n.parse().expect("authored numerator"))
                    .collect();
                let denominator: Integer = tail
                    .split_once("exact_denominator:")
                    .expect("authored denominator")
                    .1
                    .split(',')
                    .next()
                    .expect("denominator")
                    .trim()
                    .parse()
                    .expect("authored denominator");
                (numerators, denominator)
            };
            let mut cases = Vec::new();
            for row in vector.expected.split(';') {
                let tokens = quoted(row);
                let Some(mode) = tokens
                    .first()
                    .filter(|name| **name == "exact" || DIRECTIONS.contains(name))
                else {
                    continue;
                };
                for (index, operand) in operands.iter().enumerate() {
                    let expected = if *mode == "exact" {
                        refusal_expected(row)
                    } else {
                        let coefficient: Integer = tokens[1]
                            .split(',')
                            .nth(index)
                            .expect("tabled coefficient")
                            .trim()
                            .parse()
                            .expect("authored coefficient");
                        Expected::Decimal {
                            value: DecimalRepresentation::new(coefficient.clone(), 0).normalized(),
                            losses: vec![DecimalLoss {
                                numerator: numerators[index].clone(),
                                denominator: denominator.clone(),
                                coefficient,
                                scale: 0,
                                mode: (*mode).to_owned(),
                            }],
                        }
                    };
                    cases.push(SourceCase {
                        declarations: format!(
                            "function p using v(): Decimal[-100,100;1,1;exact] pure {{ {} }}\n\
                             function f using v(): Decimal[-100,100;0,0;{mode}] pure {{ convert<Decimal[-100,100;0,0;{mode}]>(p()) }}",
                            decimal(operand)
                        ),
                        expected,
                    });
                }
            }
            cases
        }
        "D06" => {
            // D15 names the operands and the result type of the zero divisor.
            let operands = pairs(self::vector("D15").inputs);
            let result_type = decimal_types(self::vector("D15").inputs)[0];
            assert!(vector.expected.starts_with("undefined"));
            vec![SourceCase {
                declarations: format!(
                    "{KERNEL}function f using v(): {result_type} pure {{ {} / {} }}",
                    decimal(&operands[0]),
                    decimal(&operands[1])
                ),
                expected: Expected::Undefined,
            }]
        }
        "D07" => {
            let tokens = quoted(vector.inputs);
            let (mode, domain) = (tokens[1], tokens[2].trim_matches(['[', ']']));
            vec![SourceCase {
                declarations: format!(
                    "function p using v(): Decimal[-100,100;1,1;exact] pure {{ {} }}\n\
                     function f using v(): Decimal[{domain};0,0;{mode}] pure {{ convert<Decimal[{domain};0,0;{mode}]>(p()) }}",
                    decimal(&pairs(vector.inputs)[0])
                ),
                expected: refusal_expected(vector.expected),
            }]
        }
        "D08" => {
            let tokens = quoted(vector.inputs);
            let domain = tokens[4].trim_matches(['[', ']']);
            let refusal = refusal_expected(vector.expected);
            tokens[..4]
                .iter()
                .enumerate()
                .map(|(index, coefficient)| SourceCase {
                    declarations: format!(
                        "{KERNEL}function p using v(): Decimal[-9,9;0,0;exact] pure {{ decimal({coefficient}, 0) }}\n\
                         function f using v(): Decimal[{domain};0,0;exact] pure {{ convert<Decimal[{domain};0,0;exact]>(p()) }}"
                    ),
                    expected: if index < 2 {
                        Expected::Decimal {
                            value: DecimalRepresentation::new(
                                coefficient.parse().expect("authored coefficient"),
                                0,
                            )
                            .normalized(),
                            losses: Vec::new(),
                        }
                    } else {
                        match &refusal {
                            Expected::Refused { code, cause } => Expected::Refused {
                                code: code.clone(),
                                cause: cause.clone(),
                            },
                            _ => unreachable!("refusal_expected returns a refusal"),
                        }
                    },
                })
                .collect()
        }
        "D12" => {
            let operands = pairs(vector.inputs);
            let types = decimal_types(vector.inputs);
            let (first, _) = vector.expected.split_once(';').expect("two outcomes");
            let (coefficient, scale) = pairs(first).into_iter().next().expect("first result");
            let loss = quoted(first)[1];
            let (numerator, denominator) = loss.split_once('/').expect("authored loss ratio");
            let source = format!("{} / {}", decimal(&operands[0]), decimal(&operands[1]));
            vec![
                function(
                    types[0],
                    &source,
                    Expected::Decimal {
                        value: DecimalRepresentation::new(coefficient.clone(), scale).normalized(),
                        losses: vec![DecimalLoss {
                            numerator: numerator.parse().expect("authored numerator"),
                            denominator: denominator.parse().expect("authored denominator"),
                            coefficient,
                            scale,
                            mode: "nearest-even".to_owned(),
                        }],
                    },
                ),
                // The second outcome is a refusal on the coefficient, the
                // code D07 gives for a coefficient outside its domain.
                function(
                    types[1],
                    &source,
                    refusal_expected(self::vector("D07").expected),
                ),
            ]
        }
        "D28" => {
            let tokens = quoted(vector.inputs);
            let refusal =
                |ty: &str, body: &str| function(ty, body, refusal_expected(vector.expected));
            vec![
                refusal(tokens[0], tokens[1]),
                refusal(tokens[2], tokens[3]),
                function(
                    tokens[0],
                    tokens[4],
                    decimal_expected(
                        self::vector("D26")
                            .expected
                            .split_once(';')
                            .expect("result")
                            .0,
                    ),
                ),
            ]
        }
        "D29" => {
            let syntax = refusal_expected(self::vector("T05b").expected);
            let refused = |declarations: &str| SourceCase {
                declarations: declarations.to_owned(),
                expected: match &syntax {
                    Expected::Refused { code, .. } => Expected::Refused {
                        code: code.clone(),
                        cause: None,
                    },
                    _ => unreachable!("refusal_expected returns a refusal"),
                },
            };
            let admitted = |declarations: &str| SourceCase {
                declarations: declarations.to_owned(),
                expected: Expected::Admitted,
            };
            let quantity = "dimension L;\nunit m : L = rational(1, 1);\n";
            vec![
                refused("type P = Decimal[0,10;0,0];\nfunction f using v(): Boolean pure { true }"),
                refused("function f using v(q: Rational[0,2;1,3]): Boolean pure { convert<Decimal[0,100;2,2]>(q) = convert<Decimal[0,100;2,2]>(q) }"),
                refused(&format!("{quantity}type Q = Quantity<m, Decimal[-100, 100; 2, 2]>;\nfunction f using v(): Boolean pure {{ true }}")),
                admitted("type P = Decimal[0,10;0,0;exact];\nfunction f using v(): Boolean pure { true }"),
                admitted("function f using v(q: Rational[0,2;1,3]): Decimal[0,100;2,2;exact] pure { convert<Decimal[0,100;2,2;exact]>(q) }"),
                admitted(&format!("{quantity}type Q = Quantity<m, Decimal[-100, 100; 2, 2; exact]>;\nfunction f using v(): Boolean pure {{ true }}")),
            ]
        }
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
        _ => return None,
    };
    Some(result)
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
            assert!(vector
                .expected
                .starts_with("equal under compatibility profiles; unequal under `nfc`"));
            inputs[1..]
                .iter()
                .map(|profile| text_comparison(profile, &left, &right, "=", *profile != "nfc"))
                .collect()
        }
        "T05b" => vec![SourceCase {
            declarations: "type P = Text;".to_owned(),
            expected: refusal_expected(vector.expected),
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

const IEEE: &str = "filament::value::ieee::";

fn float_constant(width: u32, bits: u64) -> String {
    if width == 32 {
        format!("float32(bits: {bits:#010x})")
    } else {
        format!("float64(bits: {bits:#018x})")
    }
}

/// The width a vector's authored text names (`binary32` or `binary64`),
/// falling back to the digit count of the first authored bit pattern.
fn float_width(vector: &QualificationVector) -> u32 {
    if vector.inputs.contains("binary64") {
        64
    } else if vector.inputs.contains("binary32") {
        32
    } else {
        let digits = vector.inputs.split("0x").nth(1).map_or(0, |tail| {
            tail.chars().take_while(char::is_ascii_hexdigit).count()
        });
        if digits > 8 {
            64
        } else {
            32
        }
    }
}

const DIRECTIONS: [&str; 5] = [
    "nearest-even",
    "toward-zero",
    "toward-positive",
    "toward-negative",
    "nearest-away",
];

fn float_function(width: u32, mode: &str, expression: &str, expected: Expected) -> SourceCase {
    function(&format!("Float{width}[{mode}]"), expression, expected)
}

fn float_flags(words: &str) -> Vec<String> {
    match float_expected(words) {
        Expected::Float { flags, .. } => flags,
        _ => unreachable!("float_expected returns a float"),
    }
}

fn float_value(bits: u64, flags: &[String]) -> Expected {
    Expected::Float {
        bits,
        flags: flags.to_vec(),
    }
}

/// `direction/direction: bits; ...` rows of a rounding-direction table,
/// as (direction, bits) pairs.
fn direction_rows(expected: &str) -> Vec<(String, u64)> {
    expected
        .split(';')
        .filter_map(|row| {
            let (names, tail) = row.split_once(':')?;
            let bits = *hexes(tail).first()?;
            Some(
                names
                    .split('/')
                    .map(|name| (name.trim().to_owned(), bits))
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .filter(|(name, _)| DIRECTIONS.contains(&name.as_str()))
        .collect()
}

fn boolean_case(width: u32, body: &str, expected: bool) -> SourceCase {
    let _ = width;
    function("Boolean", body, Expected::Boolean(expected))
}

fn float_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let operands = hexes(vector.inputs);
    let width = float_width(vector);
    let constant = |bits: u64| float_constant(width, bits);
    let add = |left: u64, right: u64| format!("{} + {}", constant(left), constant(right));
    let cases = match vector.id {
        "F01" => {
            let (positive, negative) = (operands[0], operands[1]);
            vec![
                boolean_case(
                    width,
                    &format!("{} = {}", constant(negative), constant(positive)),
                    true,
                ),
                boolean_case(
                    width,
                    &format!(
                        "{IEEE}bitIdentical({}, {})",
                        constant(negative),
                        constant(positive)
                    ),
                    false,
                ),
                boolean_case(
                    width,
                    &format!(
                        "{IEEE}totalOrder({}, {})",
                        constant(negative),
                        constant(positive)
                    ),
                    true,
                ),
            ]
        }
        "F02" => {
            let (first, second) = (operands[0], operands[1]);
            let mut cases = Vec::new();
            for bits in [first, second] {
                cases.push(boolean_case(
                    width,
                    &format!("{} = {}", constant(bits), constant(bits)),
                    false,
                ));
                cases.push(boolean_case(
                    width,
                    &format!("{IEEE}bitIdentical({}, {})", constant(bits), constant(bits)),
                    true,
                ));
            }
            cases.push(boolean_case(
                width,
                &format!(
                    "{IEEE}totalOrder({}, {})",
                    constant(first),
                    constant(second)
                ),
                true,
            ));
            cases
        }
        "F02b" => operands
            .chunks(2)
            .map(|pair| {
                boolean_case(
                    width,
                    &format!(
                        "{IEEE}totalOrder({}, {})",
                        constant(pair[0]),
                        constant(pair[1])
                    ),
                    true,
                )
            })
            .collect(),
        "F03" | "F04" => vec![float_function(
            32,
            "nearest-even",
            &add(operands[0], operands[1]),
            float_expected(vector.expected),
        )],
        "F04b" => {
            let flags = float_flags(vector.expected);
            let bits = *hexes(vector.expected)
                .first()
                .expect("authored result bits");
            vec![
                float_function(
                    32,
                    "nearest-even",
                    &add(operands[0], operands[1]),
                    float_value(bits, &flags),
                ),
                float_function(
                    32,
                    "nearest-even",
                    &format!(
                        "{IEEE}fma({}, {}, {})",
                        constant(operands[2]),
                        constant(operands[3]),
                        constant(operands[4])
                    ),
                    float_value(bits, &flags),
                ),
            ]
        }
        "F06" => {
            // The authored word `ill-typed` is the catalog code `ill_typed`.
            let code = vector
                .expected
                .split_whitespace()
                .find(|word| *word == "ill-typed")
                .expect("authored refusal word")
                .replace('-', "_");
            vec![boolean_case(
                width,
                &format!(
                    "{} = {}",
                    float_constant(32, operands[0]),
                    float_constant(64, operands[1])
                ),
                true,
            )]
            .into_iter()
            .map(|case| SourceCase {
                expected: Expected::Refused {
                    code: code.clone(),
                    cause: None,
                },
                ..case
            })
            .collect()
        }
        "F07" | "F08" | "F10" => {
            let flags = float_flags(vector.expected);
            direction_rows(vector.expected)
                .into_iter()
                .map(|(mode, bits)| {
                    float_function(
                        width,
                        &mode,
                        &add(operands[0], operands[1]),
                        float_value(bits, &flags),
                    )
                })
                .collect()
        }
        "F09" => {
            // The first operation is F07's, under `exact`; the second is the
            // authored exact sum.
            let first = self::vector("F07");
            let first_operands = hexes(first.inputs);
            let result = hexes(vector.expected);
            let flags = float_flags(
                vector
                    .expected
                    .split_once("second")
                    .expect("both operations")
                    .1,
            );
            vec![
                float_function(
                    32,
                    "exact",
                    &add(first_operands[0], first_operands[1]),
                    refusal_expected(vector.expected),
                ),
                float_function(
                    32,
                    "exact",
                    &add(operands[0], operands[1]),
                    float_value(result[0], &flags),
                ),
            ]
        }
        "F11" => vec![float_function(
            32,
            "nearest-even",
            &format!(
                "{IEEE}fma({}, {}, {})",
                constant(operands[0]),
                constant(operands[1]),
                constant(operands[2])
            ),
            float_expected(vector.expected.split_once(';').expect("fused result").0),
        )],
        "F12" => {
            let sources = [
                format!("{} * {}", constant(operands[0]), constant(operands[1])),
                format!("{IEEE}sqrt({})", constant(operands[2])),
            ];
            let mut cases = Vec::new();
            for source in &sources {
                for mode in DIRECTIONS.iter().chain(std::iter::once(&"exact")) {
                    cases.push(float_function(
                        32,
                        mode,
                        source,
                        float_expected(vector.expected),
                    ));
                }
            }
            cases
        }
        "F13" => {
            let results = hexes(vector.expected);
            let flags = float_flags(vector.expected);
            let one = |sign: bool| if sign { 0xbf80_0000 } else { 0x3f80_0000 };
            let zero = |sign: bool| if sign { 0x8000_0000 } else { 0 };
            let mut cases = Vec::new();
            for (index, (negative_numerator, negative_zero)) in
                [(false, false), (true, false), (false, true), (true, true)]
                    .into_iter()
                    .enumerate()
            {
                let source = format!(
                    "{} / {}",
                    constant(one(negative_numerator)),
                    constant(zero(negative_zero))
                );
                for mode in DIRECTIONS.iter().chain(std::iter::once(&"exact")) {
                    cases.push(float_function(
                        32,
                        mode,
                        &source,
                        float_value(results[index], &flags),
                    ));
                }
            }
            cases
        }
        "F14" => {
            let flags = float_flags(vector.expected.split_once(';').expect("two rows").0);
            vec![
                float_function(
                    32,
                    "nearest-even",
                    &add(operands[0], operands[1]),
                    float_value(operands_result(vector.expected), &flags),
                ),
                float_function(
                    32,
                    "exact",
                    &add(operands[0], operands[1]),
                    refusal_expected(vector.expected),
                ),
            ]
        }
        "F15" | "F16" => vec![float_function(
            32,
            "nearest-even",
            &format!("{} * {}", constant(operands[0]), constant(operands[1])),
            float_expected(vector.expected),
        )],
        "F23" => {
            let rows: Vec<&str> = vector.expected.split(';').collect();
            vec![
                float_function(
                    32,
                    "nearest-even",
                    &add(operands[0], operands[1]),
                    float_value(operands_result(rows[0]), &float_flags(rows[0])),
                ),
                float_function(
                    32,
                    "toward-zero",
                    &add(operands[0], operands[1]),
                    float_value(operands_result(rows[1]), &float_flags(rows[1])),
                ),
                float_function(
                    32,
                    "exact",
                    &add(operands[0], operands[1]),
                    refusal_expected(self::vector("F14").expected),
                ),
            ]
        }
        _ => return None,
    };
    Some(cases)
}

fn operands_result(text: &str) -> u64 {
    *hexes(text).first().expect("authored result bits")
}

fn division_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    match vector.id {
        "DIV-01" => Some(
            ["euclidean", "floor", "truncating"]
                .into_iter()
                .flat_map(|law| {
                    ["div", "rem"].into_iter().map(move |operator| SourceCase {
                        declarations: format!(
                            "{KERNEL}profile d = \"filament.value.integer-division.{law}/v1\";\n\
                             function f using v(): Integer pure {{ 7 {operator} 0 }}"
                        ),
                        expected: Expected::Undefined,
                    })
                })
                .collect(),
        ),
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

fn quantity_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    const UNITS: &str = "dimension L;\nunit m : L = rational(1, 1);\nunit cm : L = rational(1, 100) * m;\ndimension T;\nunit s : T = rational(1, 1);\n";
    let rational_quantity = |text: &str| {
        let (amount, unit) = text.split_once(' ').expect("authored quantity");
        let (numerator, denominator) = amount.split_once('/').unwrap_or((amount, "1"));
        format!("rational({numerator}, {denominator}) {unit}")
    };
    let boolean = |body: String| SourceCase {
        declarations: format!("{UNITS}function f using v(): Boolean pure {{ {body} }}"),
        expected: Expected::Boolean(true),
    };
    match vector.id {
        "U01" => {
            let inputs = quoted(vector.inputs);
            let results = quoted(vector.expected);
            Some(vec![
                boolean(format!(
                    "convert<Quantity<{}>>({}) = {}",
                    inputs[1],
                    inputs[0],
                    rational_quantity(results[0])
                )),
                boolean(format!(
                    "convert<Quantity<{}>>({}) + {} = {}",
                    inputs[1],
                    inputs[0],
                    inputs[2],
                    rational_quantity(results[1])
                )),
            ])
        }
        "U03" => {
            let inputs = quoted(vector.inputs);
            Some(vec![SourceCase {
                declarations: format!(
                    "{UNITS}function f using v(): Boolean pure {{ {} + {} = {} }}",
                    inputs[0], inputs[1], inputs[0]
                ),
                expected: refusal_expected(vector.expected),
            }])
        }
        _ => None,
    }
}

/// The standard's mathematical unit scale is rendered with the native
/// grammar's exact-rational literal; dimension terms are left intact.
fn native_units(source: &str) -> String {
    source
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
        .collect()
}

fn unit_program(source: &str, expected: Expected) -> SourceCase {
    SourceCase {
        declarations: format!(
            "{}function f using v(): Boolean pure {{ true }}",
            native_units(source)
        ),
        expected,
    }
}

/// The `code`/`cause` pairs of an authored cell, in order: a backticked
/// package-level code followed by its backticked cause.
fn refusal_pairs(text: &str) -> Vec<Expected> {
    let tokens = quoted(text);
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| matches!(**token, "invalid_package" | "missing_declaration"))
        .map(|(index, token)| Expected::Refused {
            code: (*token).to_owned(),
            cause: Some(tokens[index + 1].to_owned()),
        })
        .collect()
}

fn unit_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let snippets = quoted(vector.inputs);
    let pairs = refusal_pairs(vector.expected);
    let first = || {
        pairs.first().map(|pair| match pair {
            Expected::Refused { code, cause } => Expected::Refused {
                code: code.clone(),
                cause: cause.clone(),
            },
            _ => unreachable!("refusal_pairs returns refusals"),
        })
    };
    let nth = |index: usize| match &pairs[index] {
        Expected::Refused { code, cause } => Expected::Refused {
            code: code.clone(),
            cause: cause.clone(),
        },
        _ => unreachable!("refusal_pairs returns refusals"),
    };
    let source = match vector.id {
        "U11a" => format!("dimension L; {} {}", snippets[1], snippets[0]),
        "U11b" => format!("dimension L; {} {}", snippets[0], snippets[1]),
        "U11c" | "U11f" | "U11h" => format!("dimension L; unit m : L = 1; {}", snippets[0]),
        "U11g1" | "U11g2" | "U11g3" | "U11g4" | "U11n" | "U11o" | "U11w" => snippets.join("\n"),
        "U11d" => format!("dimension L; dimension T; unit s : T = 1; {}", snippets[0]),
        "U11i" => format!("dimension {}; {} {}", snippets[0], snippets[1], snippets[2]),
        "U11j" => format!(
            "dimension {}; {} {} {}",
            snippets[0], snippets[3], snippets[1], snippets[2]
        ),
        "U11k" => format!(
            "dimension {}; dimension {}; {} {}",
            snippets[0], snippets[1], snippets[3], snippets[5]
        ),
        "U11l" => format!("dimension L; {} {}", snippets[0], snippets[1]),
        "U11p" => snippets.join("\n"),
        "U11m" => {
            return Some(vec![
                unit_program(
                    &format!(
                        "{} {} {} {}",
                        snippets[0], snippets[1], snippets[2], snippets[3]
                    ),
                    nth(0),
                ),
                unit_program(
                    &format!(
                        "{} {} {} {}",
                        snippets[4], snippets[5], snippets[2], snippets[3]
                    ),
                    nth(1),
                ),
            ]);
        }
        "U11q" => {
            let (t_dimension, t_unit) = ("dimension T;", "unit z : T = 0;");
            let (l_dimension, l_unit) = ("dimension L;", "unit u_self : L = 1 * u_self;");
            return Some(vec![
                unit_program(
                    &format!("{t_dimension} {t_unit} {l_dimension} {l_unit}"),
                    nth(0),
                ),
                unit_program(
                    &format!("{l_dimension} {l_unit} {t_dimension} {t_unit}"),
                    nth(1),
                ),
            ]);
        }
        "U11x" => {
            let base = format!(
                "{} {} {} {}",
                snippets[0], snippets[1], snippets[2], snippets[3]
            );
            return Some(vec![
                unit_program(&base, nth(0)),
                unit_program(&format!("{base} {}", quoted(vector.inputs)[4]), nth(1)),
            ]);
        }
        "U11z3" => {
            let declarations = format!("{} {} {}", snippets[0], snippets[2], snippets[3]);
            return Some(vec![
                unit_program(
                    &format!(
                        "{} {} {} {}",
                        snippets[0], snippets[1], snippets[2], snippets[3]
                    ),
                    nth(0),
                ),
                unit_program(&declarations, nth(1)),
            ]);
        }
        _ => return None,
    };
    let expected = if vector.expected.starts_with("admitted:") {
        Expected::Admitted
    } else {
        first().expect("authored refusal")
    };
    Some(vec![unit_program(&source, expected)])
}

fn case(declarations: impl Into<String>, expected: Expected) -> SourceCase {
    SourceCase {
        declarations: declarations.into(),
        expected,
    }
}

const TEMPERATURES: &str = "dimension Temp;\nunit K : Temp = rational(1, 1);\n\
    unit degC : Temp = rational(1, 1) * K + rational(5463, 20);\n\
    unit degF : Temp = rational(5, 9) * K + rational(45967, 180);\n";

/// Roles of F34's comparison table at one width.
fn role_bits(width: u32, role: &str) -> u64 {
    let (sign, exponent, quiet) = if width == 32 {
        (1_u64 << 31, 0xff_u64 << 23, 1_u64 << 22)
    } else {
        (1_u64 << 63, 0x7ff_u64 << 52, 1_u64 << 51)
    };
    let one = if width == 32 {
        0x3f80_0000
    } else {
        0x3ff0_0000_0000_0000
    };
    match role {
        "p0" => 0,
        "n0" => sign,
        "one" => one,
        "pinf" => exponent,
        "qnan" => exponent | quiet,
        "snan" => exponent | 1,
        other => panic!("authored role {other}"),
    }
}

fn truth_words(text: &str) -> Vec<bool> {
    text.split([',', ' '])
        .filter_map(|word| match word {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
        .collect()
}

/// Vectors that need only a different rendering of source.
fn later_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let tokens = quoted(vector.inputs);
    let cases = match vector.id {
        "D11" => {
            let types = decimal_types(vector.inputs);
            let groups: Vec<Vec<(Integer, u32)>> = vector.inputs.split(';').map(pairs).collect();
            // The authored result: `100` in `[0,9999]` and `0.001` in
            // `Decimal[-9,9;0,2;exact]` refuse, every other value is a member.
            let refused = [(1, 0), (2, 0)];
            let mut cases = Vec::new();
            for (group, ty) in groups.iter().zip(types) {
                let group_index = cases_group(&cases, vector.inputs, group);
                for (index, operand) in group.iter().enumerate() {
                    let declarations =
                        format!("function f using v(): {ty} pure {{ {} }}", decimal(operand));
                    let expected = if refused.contains(&(group_index, index)) {
                        Expected::RefusedAny
                    } else {
                        Expected::Decimal {
                            value: DecimalRepresentation::new(operand.0.clone(), operand.1)
                                .normalized(),
                            losses: Vec::new(),
                        }
                    };
                    cases.push(case(declarations, expected));
                }
            }
            cases
        }
        "D24" => {
            let (first, _) = vector
                .expected
                .split_once("incomplete")
                .expect("limit rows");
            let result = first.split_once("result `(").expect("authored result").1;
            let (coefficient, scale) = pairs(&format!("({result}"))
                .into_iter()
                .next()
                .expect("authored result");
            let loss = first
                .split_once("{ exact_numerator: ")
                .expect("authored loss")
                .1;
            let field = |name: &str| -> Integer {
                loss.split_once(name)
                    .map_or(loss, |(_, tail)| tail)
                    .split([',', '}'])
                    .next()
                    .expect("field")
                    .trim()
                    .parse()
                    .expect("authored integer")
            };
            let numerator: Integer = loss
                .split(',')
                .next()
                .expect("numerator")
                .trim()
                .parse()
                .expect("numerator");
            let (denominator, rounded) =
                (field("exact_denominator:"), field("rounded_coefficient:"));
            let source = |mode: &str| {
                format!(
                    "function q using v(): Rational[0,2;1,3] pure {{ rational(2, 3) }}\n\
                     function f using v(): Decimal[0,100;2,2;{mode}] pure {{ convert<Decimal[0,100;2,2;{mode}]>(q()) }}"
                )
            };
            vec![
                case(
                    source("nearest-even"),
                    Expected::Decimal {
                        value: DecimalRepresentation::new(coefficient, scale).normalized(),
                        losses: vec![DecimalLoss {
                            numerator,
                            denominator,
                            coefficient: rounded,
                            scale,
                            mode: "nearest-even".to_owned(),
                        }],
                    },
                ),
                case(source("exact"), Expected::RefusedAny),
            ]
        }
        "D25" => {
            let expected: Vec<Expected> = {
                let mut last = Expected::Admitted;
                vector
                    .expected
                    .split(';')
                    .map(|segment| {
                        if segment.contains("code:") {
                            last = refusal_expected(segment);
                        } else if !segment.trim().starts_with("the same") {
                            last = Expected::Admitted;
                        }
                        match &last {
                            Expected::Refused { code, cause } => Expected::Refused {
                                code: code.clone(),
                                cause: cause.clone(),
                            },
                            _ => Expected::Admitted,
                        }
                    })
                    .collect()
            };
            let (p, r) = (tokens[0], tokens[1]);
            let (pp, rp) = (tokens[2], tokens[3]);
            let narrow = format!("type R = {};", tokens[13]);
            let units = format!("{p}\n{r}\n");
            let bodies = [
                format!("function e using v(): P pure {{ {} }}", tokens[4]),
                format!("function e using v(): Boolean pure {{ {} }}", tokens[5]),
                format!("function e using v({pp}): P pure {{ {} }}", tokens[6]),
                format!("function e using v({pp}, {rp}): P pure {{ {} }}", tokens[7]),
                format!("function e using v({pp}, {rp}): P pure {{ {} }}", tokens[8]),
                format!("function e using v(): P pure {{ {} }}", tokens[10]),
            ];
            let mut cases: Vec<SourceCase> = bodies
                .iter()
                .zip(&expected)
                .map(|(body, expected)| case(format!("{units}{body}"), clone_expected(expected)))
                .collect();
            cases.push(case(
                format!(
                    "{p}\n{narrow}\nfunction e using v({pp}, {rp}): P pure {{ {} }}",
                    tokens[11]
                ),
                clone_expected(&expected[6]),
            ));
            cases
        }
        "U04" => {
            let result = quoted(vector.expected)[0];
            let (numerator, rest) = result.split_once('/').expect("authored ratio");
            let (denominator, unit) = rest.split_once(' ').expect("authored unit");
            tokens[..2]
                .iter()
                .map(|quantity| {
                    case(
                        format!(
                            "{TEMPERATURES}function f using v(): Boolean pure {{ convert<Quantity<{unit}>>({quantity}) = rational({numerator}, {denominator}) {unit} }}"
                        ),
                        Expected::Boolean(true),
                    )
                })
                .collect()
        }
        "U05" => {
            let refusal = refusal_expected(vector.expected);
            ["1 degC + 1 degC", "1 degC - 1 degC", "2 degC * 2 degC", "2 degC / 2 degC", "2 degC ^ 2"]
                .into_iter()
                .map(|expression| {
                    case(
                        format!(
                            "{TEMPERATURES}function f using v(): Boolean pure {{ {expression} = {expression} }}"
                        ),
                        clone_expected(&refusal),
                    )
                })
                .collect()
        }
        "U06" => vec![unit_program(
            &format!("dimension L; unit m : L = 1; {}", tokens[1]),
            refusal_pairs(vector.expected).remove(0),
        )],
        "U11e" => vec![unit_program(
            &format!("dimension L; {}", tokens[0]),
            refusal_pairs(vector.expected).remove(0),
        )],
        "U07" => {
            let inch = tokens[0].replace(" in", " inch");
            let rounded = decimal_expected(
                vector
                    .expected
                    .split_once("`nearest-even`:")
                    .expect("rounded row")
                    .1,
            );
            let Expected::Decimal { value, losses } = rounded else {
                unreachable!("decimal_expected returns a decimal")
            };
            let source = |mode: &str| {
                format!(
                    "dimension L;\nunit m : L = rational(1, 1);\nunit inch : L = rational(127, 5000) * m;\n\
                     function f using v(): Quantity<m, Decimal[-100,100;2,2;{mode}]> pure {{ convert<Quantity<m, Decimal[-100,100;2,2;{mode}]>>({inch}) }}"
                )
            };
            vec![
                case(source("exact"), Expected::RefusedAny),
                case(
                    source("nearest-even"),
                    Expected::Quantity {
                        magnitude: format!("{}/{}", value.coefficient(), 10_u64.pow(value.scale())),
                        losses,
                    },
                ),
            ]
        }
        "U08" => {
            let domain = tokens[tokens.len() - 1].trim_matches(['[', ']']);
            tokens[..4]
                .iter()
                .enumerate()
                .map(|(index, quantity)| {
                    let amount = quantity.split(' ').next().expect("authored amount");
                    case(
                        format!(
                            "dimension L;\nunit m : L = rational(1, 1);\n\
                             function f using v(): Quantity<m, Decimal[{domain};0,0;exact]> pure {{ convert<Quantity<m, Decimal[{domain};0,0;exact]>>({quantity}) }}"
                        ),
                        if index < 2 {
                            Expected::Quantity { magnitude: format!("{amount}/1"), losses: Vec::new() }
                        } else {
                            Expected::RefusedAny
                        },
                    )
                })
                .collect()
        }
        "DIV-16" => {
            let results: Vec<Integer> = vector
                .expected
                .split("returns `")
                .skip(1)
                .map(|tail| {
                    tail.split('`')
                        .next()
                        .expect("result")
                        .parse()
                        .expect("authored integer")
                })
                .collect();
            let rows = [
                (tokens[0], tokens[1], tokens[2], 0),
                (tokens[3], tokens[4], tokens[5], 1),
            ];
            let mut cases = Vec::new();
            for law in ["euclidean", "floor", "truncating"] {
                for (domain, expression, binding, result) in rows {
                    let (name, value) = binding.split_once(" = ").expect("authored binding");
                    let body = expression.replace(name, &format!("({value})"));
                    cases.push(case(
                        format!(
                            "{KERNEL}profile d = \"filament.value.integer-division.{law}/v1\";\n\
                             function f using v(): {domain} pure {{ {body} }}"
                        ),
                        Expected::Integer(results[result].clone()),
                    ));
                }
            }
            cases
        }
        "DIV-17" => {
            let code = tokens_after(vector.expected, "unsupported_construct");
            let two = vector
                .expected
                .split_once("Euclidean `")
                .expect("authored mod")
                .1
                .split('`')
                .next()
                .expect("value");
            let bodies = [
                "1 + 7 div 3 * 2 = 1 + ((7 div 3) * 2)".to_owned(),
                "7 rem 3 = 7 - 3 * (7 div 3)".to_owned(),
                format!("-7 mod 3 = {two}"),
            ];
            let mut cases = Vec::new();
            for body in &bodies {
                cases.push(case(
                    format!(
                        "profile d = \"filament.value.integer-division.truncating/v1\";\n\
                         function f using v(): Boolean pure {{ {body} }}"
                    ),
                    Expected::Boolean(true),
                ));
                cases.push(case(
                    format!("{STATE_ONLY}function f using v(): Boolean pure {{ {body} }}"),
                    Expected::Refused {
                        code: code.clone(),
                        cause: None,
                    },
                ));
            }
            cases
        }
        "DIV-18" => {
            let code = tokens_after(vector.expected, "unsupported_construct");
            let mut cases: Vec<SourceCase> = ["div", "rem", "mod"]
                .into_iter()
                .map(|operator| {
                    case(
                        format!(
                            "{STATE_ONLY}function f using v(): Boolean pure {{ (if true then 1 else (7 {operator} 3)) = 1 }}"
                        ),
                        Expected::Refused { code: code.clone(), cause: None },
                    )
                })
                .collect();
            cases.push(case(
                format!(
                    "{STATE_ONLY}function f using v(): Boolean pure {{ 7 / 3 = rational(7, 3) }}"
                ),
                Expected::RefusedAny,
            ));
            cases.push(case(
                "function f using v(): Boolean pure { 7 / 3 = 7 / 3 }",
                Expected::RefusedAny,
            ));
            cases.push(case(
                "function g using v(): Rational[0,7;1,3] pure { 7 / 3 }\n\
                 function f using v(): Boolean pure { g() = rational(7, 3) }",
                Expected::Boolean(true),
            ));
            cases
        }
        "F05" => {
            let ordered = [0xff80_0000_u64, 0xff7f_ffff, 0x7f7f_ffff, 0x7f80_0000];
            let constant = |bits: u64| float_constant(32, bits);
            let mut cases = Vec::new();
            for (i, a) in ordered.iter().enumerate() {
                cases.push(boolean_case(
                    32,
                    &format!("{IEEE}bitIdentical({}, {})", constant(*a), constant(*a)),
                    true,
                ));
                for b in &ordered[i + 1..] {
                    cases.push(boolean_case(
                        32,
                        &format!("{} < {}", constant(*a), constant(*b)),
                        true,
                    ));
                    cases.push(boolean_case(
                        32,
                        &format!("{IEEE}totalOrder({}, {})", constant(*a), constant(*b)),
                        true,
                    ));
                    cases.push(boolean_case(
                        32,
                        &format!("{IEEE}totalOrder({}, {})", constant(*b), constant(*a)),
                        false,
                    ));
                    cases.push(boolean_case(
                        32,
                        &format!("{IEEE}bitIdentical({}, {})", constant(*a), constant(*b)),
                        false,
                    ));
                }
            }
            cases
        }
        "F20" => {
            let code = refusal_expected(vector.expected);
            let (b32, b64) = (
                float_constant(32, 0x3f80_0000),
                float_constant(64, 0x3ff0_0000_0000_0000),
            );
            [
                ("Boolean", format!("{IEEE}numericEqual({b32}, {b64})")),
                ("Boolean", format!("{IEEE}totalOrder({b32}, {b64})")),
                ("Boolean", format!("{IEEE}bitIdentical({b32}, {b64})")),
                ("Float32[nearest-even]", format!("{b32} + {b64}")),
                ("Float32[nearest-even]", format!("{b32} + 1")),
                (
                    "Decimal[0,10;0,0;exact]",
                    format!("convert<Decimal[0,10;0,0;exact]>({b32})"),
                ),
                ("Int[0,1]", format!("convert<Int[0,1]>({b32})")),
            ]
            .into_iter()
            .map(|(result, body)| function(result, &body, clone_expected(&code)))
            .collect()
        }
        "T07" | "T09" | "T10" => {
            let word = vector
                .expected
                .split_whitespace()
                .find(|word| *word == "ill-typed")
                .map(|word| word.replace('-', "_"));
            let refusal = word.map_or_else(|| refusal_expected(vector.expected), |code| Expected::Refused { code, cause: None });
            let compare = |declarations: &str, body: &str| {
                case(
                    format!("{declarations}\nfunction f using v(): Boolean pure {{ {body} }}"),
                    clone_expected(&refusal),
                )
            };
            match vector.id {
                "T07" => vec![compare(
                    "enum A { READY, DONE }\nenum B { READY, DONE }",
                    "A::READY = B::READY",
                )],
                "T09" => vec![
                    compare("enum A { READY, DONE }\nenum B { READY, DONE }", "A::READY = B::READY"),
                    compare("enum A { READY, DONE }\nenum B { READY, FINISHED }", "A::READY = B::READY"),
                    compare(
                        "ordered enum A { READY, DONE }\nordered enum B { DONE, READY }",
                        "A::READY = B::READY",
                    ),
                ],
                _ => vec![compare(
                    "function left using v(): Text[0,8;nfc] pure { Text(\"a\") }\n\
                     function right using v(): Text[0,8;binary-utf8] pure { Text(\"a\") }",
                    "left() = right()",
                )],
            }
        }
        "T08" => vec![
            case(
                "ordered enum A { READY, DONE }\nfunction f using v(): Boolean pure { A::READY < A::DONE }",
                Expected::Boolean(true),
            ),
            case(
                "enum A { READY, DONE }\nfunction f using v(): Boolean pure { A::READY < A::DONE }",
                Expected::RefusedAny,
            ),
        ],
        "F34" => {
            let roles: Vec<&str> = tokens[..12].iter().step_by(2).copied().collect();
            let comparisons = &tokens[12..];
            let truths = truth_words(
                vector
                    .expected
                    .split(" at both")
                    .next()
                    .expect("authored truths"),
            );
            assert_eq!(comparisons.len(), truths.len());
            let mut cases = Vec::new();
            for width in [64, 32] {
                for (comparison, truth) in comparisons.iter().zip(&truths) {
                    let mut body = (*comparison).to_owned();
                    for role in &roles {
                        body = body
                            .split(' ')
                            .map(|word| {
                                if word == *role {
                                    float_constant(width, role_bits(width, role))
                                } else {
                                    word.to_owned()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                    }
                    cases.push(boolean_case(width, &body, *truth));
                }
            }
            cases
        }
        _ => return None,
    };
    Some(cases)
}

/// The `Expected` is not `Clone` (it holds loss records only in tests), so
/// the few places that reuse one rebuild it.
fn clone_expected(expected: &Expected) -> Expected {
    match expected {
        Expected::Refused { code, cause } => Expected::Refused {
            code: code.clone(),
            cause: cause.clone(),
        },
        Expected::Admitted => Expected::Admitted,
        Expected::RefusedAny => Expected::RefusedAny,
        other => panic!("only refusals and admissions are reused, got {other:?}"),
    }
}

/// The first backticked word of `text` that begins with `prefix`.
fn tokens_after(text: &str, prefix: &str) -> String {
    quoted(text)
        .into_iter()
        .find(|token| token.starts_with(prefix))
        .expect("authored code")
        .to_owned()
}

/// D11's group number: groups are the `;`-separated clauses of the inputs.
fn cases_group(_done: &[SourceCase], inputs: &str, group: &[(Integer, u32)]) -> usize {
    inputs
        .split(';')
        .position(|clause| pairs(clause) == group)
        .expect("authored group")
}

fn source_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    primary_cases(vector).or_else(|| later_cases(vector))
}

fn primary_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    match vector.test_case {
        TestCase::Tc185 => decimal_cases(vector),
        TestCase::Tc186 => text_cases(vector),
        TestCase::Tc187 => unit_cases(vector).or_else(|| quantity_cases(vector)),
        TestCase::Tc192 => division_cases(vector),
        TestCase::Tc193 => float_cases(vector),
    }
}

fn observed(case: &SourceCase) -> Expected {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"{}\";\n{}",
        if case.declarations.starts_with(STATE_ONLY) {
            "filament.state.core/v1"
        } else {
            "filament.value.complete/v1"
        },
        case.declarations
            .trim_start_matches(KERNEL)
            .trim_start_matches(STATE_ONLY)
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
        return Expected::Refused {
            code: diagnostic.code.as_str().to_owned(),
            cause: None,
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
            return Expected::Refused {
                code: error.cause.code().as_str().to_owned(),
                cause: Some(error.cause.catalog_code().cause().to_owned()),
            };
        }
    };
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("public IEEE definition selection"),
    );
    let checked = match declarations.check(
        if case.declarations.starts_with(KERNEL) {
            CheckMode::Kernel
        } else {
            CheckMode::Linked
        },
        CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    ) {
        Ok(checked) => checked,
        Err(errors) => {
            let error = errors.first().expect("checking refusal");
            return Expected::Refused {
                code: error.cause.code().as_str().to_owned(),
                cause: error.cause.cause().map(str::to_owned),
            };
        }
    };
    if case.expected == Expected::Admitted {
        return Expected::Admitted;
    }
    let package = CheckedPackage::link(checked);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("function name"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("checked function invocation");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("value outcome: {:?}", evaluation.outcome);
    };
    let decimal_losses: Vec<DecimalLoss> = evaluation
        .losses
        .iter()
        .filter_map(|loss| {
            let ValueLoss::Decimal(loss) = &loss.loss else {
                return None;
            };
            Some(DecimalLoss {
                numerator: loss.exact_numerator().clone(),
                denominator: loss.exact_denominator(),
                coefficient: loss.rounded_coefficient().clone(),
                scale: loss.rounded_scale(),
                mode: loss.mode().as_str().to_owned(),
            })
        })
        .collect();
    match outcome {
        Outcome::Completed(Value::Boolean(value)) => Expected::Boolean(value),
        Outcome::Completed(Value::Integer(value)) => Expected::Integer(value),
        Outcome::Completed(Value::Decimal(value)) => {
            let losses = decimal_losses;
            Expected::Decimal {
                value: value.normalized().clone(),
                losses,
            }
        }
        Outcome::Completed(Value::Quantity(quantity)) => Expected::Quantity {
            magnitude: quantity.magnitude().to_string(),
            losses: decimal_losses,
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
        Outcome::Undefined(_) => Expected::Undefined,
        Outcome::Refused(refusal) => {
            assert!(
                evaluation.losses.is_empty(),
                "refusal exposes no partial losses: {:?}",
                evaluation.losses
            );
            Expected::Refused {
                code: refusal.code().expect("refusal code").to_owned(),
                cause: refusal.cause().map(str::to_owned),
            }
        }
        other => panic!("qualification result has an unexpected shape: {other:?}"),
    }
}

/// Runs every source case of `vector` and reports each disagreement with the
/// standard, headed by the vector's test case and identity. A refusal whose
/// authored expectation names no cause is compared by code alone.
fn compare(vector: &QualificationVector) -> Result<usize, String> {
    let cases = source_cases(vector).expect("this vector has a source adapter");
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let actual = observed(case);
        if !agrees(&case.expected, &actual) {
            failures.push(format!(
                "{}:{}[{index}] expected {:?}, actual {actual:?}; source {}",
                vector.test_case.id(),
                vector.id,
                case.expected,
                case.declarations
            ));
        }
    }
    if failures.is_empty() {
        Ok(cases.len())
    } else {
        Err(failures.join("\n"))
    }
}

fn agrees(expected: &Expected, actual: &Expected) -> bool {
    match (expected, actual) {
        (Expected::RefusedAny, Expected::Refused { .. }) => true,
        (Expected::Refused { code, cause: None }, Expected::Refused { code: actual, .. }) => {
            code == actual
        }
        _ => expected == actual,
    }
}

fn vector(id: &str) -> &'static QualificationVector {
    QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.id == id)
        .expect("published vector identity")
}

/// The Linear tickets that own the unexpressible groups (QSL-2052 groups
/// V1 to V6). A recorded vector's key must be one of these.
const GROUP_KEYS: [&str; 6] = [
    "QSL-3057", "QSL-3058", "QSL-3059", "QSL-3060", "QSL-3061", "QSL-3062",
];
const V1: &str = "QSL-3057";
const V2: &str = "QSL-3058";
const V3: &str = "QSL-3059";
const V4: &str = "QSL-3060";
const V5: &str = "QSL-3061";

/// A published vector the source language cannot express today, with the
/// typed refusal its closest FSL source meets now (`refusal`), or none where
/// the vector's input is not source text at all (accounting tuples, wire
/// rows). `key` is the Linear ticket that owns the gap; an entry without one
/// fails `unexpressible_vectors_carry_ticket_keys`.
struct Unexpressible {
    id: &'static str,
    refusal: Option<Refusal>,
    key: Option<&'static str>,
}

struct Refusal {
    probe: &'static str,
    code: &'static str,
    cause: Option<&'static str>,
}

const fn refused(
    id: &'static str,
    key: Option<&'static str>,
    probe: &'static str,
    code: &'static str,
    cause: Option<&'static str>,
) -> Unexpressible {
    Unexpressible {
        id,
        refusal: Some(Refusal { probe, code, cause }),
        key,
    }
}

const fn not_source(id: &'static str, key: &'static str) -> Unexpressible {
    Unexpressible {
        id,
        refusal: None,
        key: Some(key),
    }
}

const KERNEL_TRUNCATING: &str =
    "#kernel\nprofile d = \"filament.value.integer-division.truncating/v1\";\n";

/// D13 and D18's authored text says their retained operands are not members
/// of the result type, so no FSL source form exists; the probes write those
/// operands anyway. The `Int[..]` consumer-domain rows meet
/// `integer_out_of_domain`, not the standard's division/modulo codes. The
/// out-of-bounds `Text` literal rows meet `ill_typed` at check time.
const UNEXPRESSIBLE: &[Unexpressible] = &[
    refused("D13", Some(V1), "function f using v(): Decimal[-9,9;0,0;toward-zero] pure { decimal(150, 2) * decimal(2, 0) }", "ill_typed", Some("type-mismatch")),
    refused("D18", Some(V1), "function f using v(): Decimal[-1000,1000;0,1;nearest-even] pure { decimal(1234, 3) / decimal(2, 0) }", "ill_typed", Some("type-mismatch")),
    refused("DIV-05", Some(V2), "#kernel\nprofile d = \"filament.value.integer-division.truncating/v1\";\nfunction f using v(): Int[-9223372036854775808, 9223372036854775807] pure { (-9223372036854775808) div (-1) }", "integer_out_of_domain", Some("outside-domain")),
    refused("DIV-06", Some(V2), "#kernel\nprofile d = \"filament.value.integer-division.truncating/v1\";\nfunction f using v(): Int[-9223372036854775808, 9223372036854775807] pure { (-9223372036854775808) div (-1) }", "integer_out_of_domain", Some("outside-domain")),
    refused("DIV-12", Some(V2), "#kernel\nfunction f using v(): Int[0, 1] pure { -7 mod 3 }", "integer_out_of_domain", Some("outside-domain")),
    refused("DIV-15", Some(V2), "#kernel\nprofile d = \"filament.value.integer-division.truncating/v1\";\nfunction f using v(): Int[1, 10] pure { 10 rem 5 }", "integer_out_of_domain", Some("outside-domain")),
    refused("T04", Some(V3), "function f using v(): Text[1,1;binary-utf8] pure { Text(\"\u{e9}\") }", "ill_typed", Some("type-mismatch")),
    refused("T05", Some(V3), "function f using v(): Text[0,0;nfc] pure { Text(\"a\") }", "ill_typed", Some("type-mismatch")),
    refused("T13", Some(V3), "function f using v(): Text[1,1;nfd] pure { Text(\"\u{e9}\") }", "ill_typed", Some("type-mismatch")),
    // No group ticket covers these two: negation is refused outside `exact`
    // and a derived dimension is not the dimension its map normalizes to.
    refused("F32", None, "function f using v(): Float32[nearest-even] pure { -float32(bits: 0x3f800000) }", "ill_typed", Some("type-mismatch")),
    refused("U02", None, "dimension L;\ndimension T;\nunit m : L = rational(1, 1);\nunit s : T = rational(1, 1);\ndimension X = L * T / T;\nunit u : X = rational(1, 1);\nfunction f using v(): Boolean pure { convert<Quantity<m>>(1 u) = 1 m }", "ill_typed", Some("type-mismatch")),
    // ScalarLimitsV1 accounting vectors (V4).
    not_source("D09", V4), not_source("D14", V4), not_source("D15", V4), not_source("D16", V4),
    not_source("D17", V4), not_source("D19", V4), not_source("D20", V4), not_source("D21", V4),
    not_source("D22", V4), not_source("D23", V4), not_source("T11", V4), not_source("T14", V4),
    not_source("T15", V4), not_source("T16", V4), not_source("U10", V4), not_source("U13", V4),
    not_source("U14", V4), not_source("U15", V4), not_source("U16", V4), not_source("U17", V4),
    not_source("U18", V4), not_source("U19", V4), not_source("U20", V4), not_source("U21", V4),
    not_source("U22", V4), not_source("U23", V4), not_source("U24", V4), not_source("U25", V4),
    not_source("U26", V4), not_source("U27", V4), not_source("U28", V4), not_source("U29", V4),
    not_source("DIV-08", V4), not_source("DIV-10", V4), not_source("DIV-11", V4),
    not_source("DIV-13", V4), not_source("F17", V4), not_source("F18", V4), not_source("F19", V4),
    not_source("F21", V4), not_source("F22", V4), not_source("F24", V4), not_source("F25", V4),
    not_source("F26", V4), not_source("F27", V4), not_source("F28", V4), not_source("F29", V4),
    not_source("F30", V4), not_source("F31", V4), not_source("F33", V4),
    // Multi-package, imported-model and wire rows (V5).
    not_source("U11r", V5), not_source("U11s", V5), not_source("U11t", V5), not_source("U11u", V5),
    not_source("U11y", V5), not_source("U11z1", V5), not_source("U11z2", V5),
    not_source("U30", V5), not_source("U31", V5), not_source("U32", V5), not_source("U33", V5),
    not_source("U34", V5), not_source("U35", V5), not_source("U36", V5), not_source("U37", V5),
    not_source("U38", V5), not_source("U39", V5), not_source("U40", V5),
    not_source("DIV-03", V5), not_source("DIV-07", V5), not_source("DIV-09", V5),
    not_source("DIV-14", V5), not_source("T06", V5),
    // A distinct dimension or unit node key needs a second package (V5).
    not_source("U09", V5), not_source("U09b", V5),
    // No group ticket covers compound-unit results (2 m * 3 s).
    Unexpressible { id: "U12", refusal: None, key: None },
];

fn unexpressible(id: &str) -> Option<&'static Unexpressible> {
    UNEXPRESSIBLE.iter().find(|entry| entry.id == id)
}

#[trace("FSpec-TC-185")]
#[test]
fn all_vectors_from_source() {
    let (mut executed, mut cases, mut recorded) = (0, 0, 0);
    let (mut failures, mut pending) = (Vec::new(), Vec::new());
    for vector in QUALIFICATION_VECTORS {
        if source_cases(vector).is_some() {
            executed += 1;
            match compare(vector) {
                Ok(count) => cases += count,
                Err(failure) => failures.push(failure),
            }
        } else if unexpressible(vector.id).is_some() {
            recorded += 1;
        } else {
            pending.push(format!("{}:{}", vector.test_case.id(), vector.id));
        }
    }
    assert!(
        failures.is_empty(),
        "{} vector(s) disagree with the standard:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        cases >= executed,
        "every executed vector runs a source case"
    );
    assert!(
        pending.is_empty(),
        "{} vector(s) have neither a source adapter nor a recorded refusal: {}",
        pending.len(),
        pending.join(", ")
    );
    assert_eq!(
        executed + recorded,
        QUALIFICATION_VECTORS.len(),
        "executed vectors plus recorded refusals cover QUALIFICATION_VECTORS"
    );
}

#[trace("FSpec-TC-185")]
#[test]
fn refusing_vectors_match_codes() {
    let mut refusals = Vec::new();
    for vector in QUALIFICATION_VECTORS {
        let Some(cases) = source_cases(vector) else {
            continue;
        };
        for case in cases {
            let (code, cause) = match &case.expected {
                Expected::Refused { code, cause } => (code.as_str(), cause.clone()),
                Expected::Undefined => ("undefined", None),
                _ => continue,
            };
            let actual = observed(&case);
            let matches = match (&actual, code) {
                (Expected::Undefined, "undefined") => true,
                (
                    Expected::Refused {
                        code: actual,
                        cause: actual_cause,
                    },
                    _,
                ) => {
                    actual == code
                        && cause
                            .as_ref()
                            .is_none_or(|cause| Some(cause) == actual_cause.as_ref())
                }
                _ => false,
            };
            assert!(
                matches,
                "{}:{} expected refusal {code} {cause:?}, actual {actual:?}; source {}",
                vector.test_case.id(),
                vector.id,
                case.declarations
            );
            refusals.push(code.to_owned());
        }
    }
    for code in [
        "inexact_decimal",
        "decimal_out_of_domain",
        "ieee_not_exact",
        "ill_typed",
        "undefined",
    ] {
        assert!(
            refusals.iter().any(|seen| seen == code),
            "no vector exercised the refusal {code}"
        );
    }
}

#[trace("FSpec-TC-185")]
#[test]
fn recorded_refusals_still_occur() {
    let with_probe: Vec<&Unexpressible> = UNEXPRESSIBLE
        .iter()
        .filter(|entry| entry.refusal.is_some())
        .collect();
    assert!(!with_probe.is_empty(), "an empty registry proves nothing");
    for entry in with_probe {
        let refusal = entry.refusal.as_ref().expect("filtered");
        let actual = observed(&SourceCase {
            declarations: refusal.probe.to_owned(),
            expected: Expected::Boolean(true),
        });
        assert_eq!(
            actual,
            Expected::Refused {
                code: refusal.code.to_owned(),
                cause: refusal.cause.map(str::to_owned),
            },
            "{}: the recorded refusal no longer occurs; if the vector became expressible, render it and remove the entry",
            entry.id
        );
    }
}

#[trace("FSpec-TC-185")]
#[test]
fn recorded_vectors_are_published_and_not_also_executed() {
    for entry in UNEXPRESSIBLE {
        let published = vector(entry.id);
        assert!(
            source_cases(published).is_none(),
            "{} has a source adapter; drop its registry entry",
            entry.id
        );
    }
    let mut ids: Vec<&str> = UNEXPRESSIBLE.iter().map(|entry| entry.id).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "a vector is recorded once");
}

#[trace("FSpec-TC-185")]
#[test]
fn unexpressible_vectors_carry_ticket_keys() {
    let unkeyed: Vec<&str> = UNEXPRESSIBLE
        .iter()
        .filter(|entry| entry.key.is_none())
        .map(|entry| entry.id)
        .collect();
    assert!(
        unkeyed.is_empty(),
        "unexpressible vectors without a ticket key: {}",
        unkeyed.join(", ")
    );
    let foreign: Vec<&str> = UNEXPRESSIBLE
        .iter()
        .filter(|entry| entry.key.is_some_and(|key| !GROUP_KEYS.contains(&key)))
        .map(|entry| entry.id)
        .collect();
    assert!(
        foreign.is_empty(),
        "keys outside {GROUP_KEYS:?}: {}",
        foreign.join(", ")
    );
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
