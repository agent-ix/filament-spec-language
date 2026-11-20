// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2052: every published qualification vector (`QUALIFICATION_VECTORS`),
//! rendered as FSL source, checked, linked and evaluated, and compared with
//! the vector's expected value or refusal read from the standard crate.
//! A vector with no source rendering, or whose rendering samples only part
//! of the row, is not a successful qualification: it is a listed known gap.

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

use super::{gap, Corpus, Difference, KnownGap};

/// Rows that do not currently conform, asserted exactly.
const KNOWN_GAPS: &[KnownGap] = &[gap("TC-185:D03", "placeholder")];

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
    Float {
        bits: u64,
        flags: Vec<String>,
    },
    Refused {
        code: String,
        cause: Option<String>,
    },
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

fn float_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    let operands = hexes(vector.inputs);
    let (width, expression, expected) = match vector.id {
        "F03" | "F04" => (32, format!("float32(bits: {:#010x}) + float32(bits: {:#010x})", operands[0], operands[1]), float_expected(vector.expected)),
        "F07" | "F08" | "F10" => {
            let width = if vector.id == "F10" { 64 } else { 32 };
            (width, format!("float{width}(bits: {:#x}) + float{width}(bits: {:#x})", operands[0], operands[1]), float_expected(vector.expected))
        }
        "F11" => (32, format!("filament::value::ieee::fma(float32(bits: {:#x}), float32(bits: {:#x}), float32(bits: {:#x}))", operands[0], operands[1], operands[2]), float_expected(vector.expected)),
        "F15" | "F16" => (32, format!("float32(bits: {:#010x}) * float32(bits: {:#010x})", operands[0], operands[1]), float_expected(vector.expected)),
        _ => return None,
    };
    Some(vec![function(
        &format!("Float{width}[nearest-even]"),
        &expression,
        expected,
    )])
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
    }])
}

fn source_cases(vector: &QualificationVector) -> Option<Vec<SourceCase>> {
    match vector.test_case {
        TestCase::Tc185 => decimal_cases(vector),
        TestCase::Tc186 => text_cases(vector),
        TestCase::Tc187 => unit_cases(vector),
        TestCase::Tc192 => division_cases(vector),
        TestCase::Tc193 => float_cases(vector),
    }
}

fn observed(case: &SourceCase) -> Expected {
    let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{}", case.declarations);
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
        CheckMode::Linked,
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
    match outcome {
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

fn compare(vector: &QualificationVector) -> Result<usize, String> {
    let cases = source_cases(vector).expect("this vector has a source adapter");
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let actual = observed(case);
        let matches = match (&case.expected, &actual) {
            (Expected::Refused { code, cause: None }, Expected::Refused { code: actual, .. }) => {
                code == actual
            }
            _ => case.expected == actual,
        };
        if !matches {
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
source_vectors!("FSpec-TC-187"; u11a=>"U11a", u11b=>"U11b", u11c=>"U11c", u11f=>"U11f", u11h=>"U11h", u11g1=>"U11g1", u11g2=>"U11g2", u11g3=>"U11g3", u11g4=>"U11g4", u11n=>"U11n", u11o=>"U11o", u11w=>"U11w");
source_vectors!("FSpec-TC-192"; div02=>"DIV-02", div04=>"DIV-04");
source_vectors!("FSpec-TC-193"; f03=>"F03", f04=>"F04", f07=>"F07", f08=>"F08", f10=>"F10", f11=>"F11", f15=>"F15", f16=>"F16");

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


/// Rows whose source rendering covers only part of the authored row.
const PARTIAL: &[&str] = &[
    "D01", "D26", "D27", "T02", "T06b", "F07", "F08", "F10", "F11", "U11a", "U11b", "U11c",
    "U11f", "U11h", "U11g1", "U11g2", "U11g3", "U11g4", "U11n", "U11o", "U11w",
];

/// Why a row has no source rendering.
fn unrendered(vector: &QualificationVector) -> &'static str {
    match (vector.test_case, vector.id) {
        (TestCase::Tc185, "D13" | "D18") => {
            "published evaluator-boundary operands explicitly have no source form"
        }
        (TestCase::Tc186, "T06") => "invalid UTF-8 reader-boundary input, not a source value",
        (TestCase::Tc192, "DIV-03" | "DIV-09") => {
            "mutation of an already checked wire definition closure"
        }
        (TestCase::Tc192, "DIV-07" | "DIV-14") => {
            "backend negotiation rather than source evaluation"
        }
        _ => "no source rendering of the row is authored",
    }
}

fn vector_outcome(vector: &QualificationVector) -> Result<(), Difference> {
    if source_cases(vector).is_none() {
        return Err(Difference::new(vector.expected, unrendered(vector)));
    }
    compare(vector).map_err(|mismatch| Difference::new(vector.expected, mismatch))?;
    if PARTIAL.contains(&vector.id) {
        return Err(Difference::new(
            vector.expected,
            "the sampled part matched; the rest of the row has no source rendering",
        ));
    }
    Ok(())
}

fn key(vector: &QualificationVector) -> String {
    format!("{}:{}", vector.test_case.id(), vector.id)
}

fn run_corpus(vectors: &[QualificationVector]) -> Corpus {
    let mut corpus = Corpus::new("qualification", vectors.len());
    for vector in vectors {
        corpus.record(key(vector), vector_outcome(vector));
    }
    corpus
}

/// Provenance: QSL-2052. Every published vector is evaluated from source or
/// is an exactly listed known gap.
#[trace("FSpec-TC-185", "FSpec-TC-186", "FSpec-TC-187", "FSpec-TC-192", "FSpec-TC-193")]
#[test]
fn conformance_qualification_vectors_from_source() {
    let summary = run_corpus(QUALIFICATION_VECTORS).assert_settled(KNOWN_GAPS);
    assert_eq!(summary.cases, QUALIFICATION_VECTORS.len());
}

/// Provenance: QSL-2052. Rendered vectors whose expected outcome is a
/// refusal are refused with the standard's code.
#[trace("FSpec-TC-185", "FSpec-TC-186", "FSpec-TC-187")]
#[test]
fn refusing_vectors_match_codes() {
    let mut refusals = 0;
    for vector in QUALIFICATION_VECTORS {
        let Some(cases) = source_cases(vector) else {
            continue;
        };
        if cases
            .iter()
            .any(|case| matches!(case.expected, Expected::Refused { .. }))
        {
            refusals += 1;
            compare(vector).unwrap_or_else(|mismatch| panic!("{mismatch}"));
        }
    }
    assert!(refusals > 0, "the rendered rows include refusals");
}

/// Provenance: QSL-2052. An altered vector in a copy of the table is an
/// unlisted mismatch named by its vector identity.
#[trace("FSpec-TC-185")]
#[test]
fn an_altered_vector_in_a_table_copy_is_reported_by_identity() {
    let mut vectors = QUALIFICATION_VECTORS.to_vec();
    let altered = vectors
        .iter_mut()
        .find(|vector| vector.id == "D02")
        .expect("D02");
    altered.expected = "exact `(999,0)` with no loss";
    let errors = run_corpus(&vectors)
        .settle(KNOWN_GAPS)
        .expect_err("the altered row mismatches");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].starts_with("qualification: TC-185:D02: "), "{errors:?}");
}
