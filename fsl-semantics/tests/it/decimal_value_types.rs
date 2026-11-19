// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056 "Decimal value types" (TC-3042): model intake reads a decimal
//! subject's type from the filament-core-data decimal policy and the bound
//! constraints on its resolution walk, as `Decimal[lo, hi; s, s; exact]`.
//! The golden package and its expected types live in `tests/fixtures/`.

use filament_exact::{DecimalType, Integer, RoundingMode, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{CheckedGraph, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input, UnitIntakeCause};
use ix_trace_rs::trace;
use serde_json::Value;

const GOLDEN: &[u8] =
    include_bytes!("../../../tests/fixtures/decimal-value-types.semantic-ir.json");
const EXPECTED: &[u8] = include_bytes!("../../../tests/fixtures/decimal-value-types.expected.json");
const O2: &[u8] = include_bytes!("../../../tests/fixtures/o2-battery-dispatch.semantic-ir.json");

/// The unit text selecting `package` as `alias`, with `body` appended.
fn unit_text(package: &str, alias: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model {alias} = {package:?};\n\
         function keep using v(): Boolean pure {{ true }}\n{body}"
    )
}

/// Admits, assembles and checks `document` under the unit `unit`; the
/// assembled declarations come back with the checked graph.
fn check(document: &[u8], unit: &str) -> (PackageDeclarations, CheckedGraph) {
    let packages = package_input([document]).expect("byte admission");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "decimal-value-types"),
        "decimal.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let assemble = || {
        let built = build_unit(&parsed).expect("S2 builds the unit");
        let selected = admit_unit(
            &built.selections().models,
            &packages,
            ModelNormalizationLimits::default(),
        )
        .unwrap_or_else(|refusal| panic!("I1 admits the package: {refusal:?}"));
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            crate::TEST_PACKAGE_NAME.to_owned(),
            built,
            selected,
            Vec::new(),
        )
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    };
    let declarations = assemble();
    let graph = assemble()
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    (declarations, graph)
}

/// The `(code, cause)` pairs intake refuses `document` with.
fn intake_refusals(document: &Value) -> Vec<(String, String)> {
    let bytes = serde_json::to_vec(document).expect("fixture encoding");
    let packages = package_input([bytes.as_slice()]).expect("byte admission");
    let unit = unit_text("test/decimal", "Config", "");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "decimal-value-types"),
        "decimal.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let built = build_unit(&parsed).expect("S2 builds the unit");
    let refusal = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect_err("the defect refuses at intake");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    refusals
        .iter()
        .map(|refusal| {
            (
                refusal.code.as_str().to_owned(),
                refusal.cause.as_str().to_owned(),
            )
        })
        .collect()
}

fn golden() -> Value {
    serde_json::from_slice(GOLDEN).expect("golden JSON")
}

/// The type definition `name` of the golden package.
fn type_mut<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    let identity = format!("ix://test/decimal/{name}");
    document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["identity"] == identity.as_str())
        .expect("the type is declared")
}

/// The field `name` of `Meter`.
fn field_mut<'a>(document: &'a mut Value, name: &str) -> &'a mut Value {
    type_mut(document, "Meter")["fields"]
        .as_array_mut()
        .expect("fields")
        .iter_mut()
        .find(|field| field["name"] == name)
        .expect("the field is declared")
}

/// FR-056-AC-65 (TC-3042 step 1): every field of the golden package has the
/// golden `Decimal[lo, hi; s, s; exact]`.
#[trace("TC-3042", "FR-056-AC-65")]
#[test]
#[ignore = "QSL-2451: the Decimal value type reader is not implemented yet"]
fn decimal_members_have_the_golden_policy_types() {
    let (declarations, graph) = check(GOLDEN, &unit_text("test/decimal", "Config", ""));
    let meter = declarations
        .types
        .object_types()
        .find(|declared| declared.name() == "Config::Meter")
        .expect("Meter is declared")
        .key();
    let attributes = graph
        .scope()
        .types()
        .attributes(meter)
        .expect("Meter has attributes");
    let expected: Value = serde_json::from_slice(EXPECTED).expect("expected JSON");
    for entry in expected["members"].as_array().expect("members") {
        let member = entry["member"].as_str().expect("member");
        let Some(field) = member.strip_prefix("Meter.") else {
            continue;
        };
        if field.contains('.') {
            continue;
        }
        let declared = attributes
            .iter()
            .find(|attribute| attribute.field().name() == field)
            .map(|attribute| attribute.field().value_type().clone())
            .unwrap_or_else(|| panic!("{field} is declared"));
        let bound = |key: &str| {
            entry[key]
                .as_str()
                .expect("bound text")
                .parse::<Integer>()
                .expect("an integer bound")
        };
        let scale = |key: &str| entry[key].as_u64().expect("a scale");
        let want = ValueType::Decimal(
            DecimalType::new(
                bound("lo"),
                bound("hi"),
                scale("smin"),
                scale("smax"),
                RoundingMode::Exact,
            )
            .expect("golden bounds are ordered"),
        );
        assert_eq!(declared, want, "{member}");
    }
}

/// FR-056-AC-66 (TC-3042 steps 2-3): the Decimal members check in clauses,
/// and FSpec's o2 planning model admits and checks its `SocFloor` invariant.
#[trace("TC-3042", "FR-056-AC-66")]
#[test]
#[ignore = "QSL-2451: the Decimal value type reader is not implemented yet"]
fn decimal_members_check_in_clauses() {
    for clause in [
        "self.soc >= 0.20",
        "self.soc = self.soc",
        "self.fee = self.fee",
        "self.rate <= 1",
    ] {
        let body = format!("invariant I using v on Config::Meter at current {{ {clause} }}\n");
        check(GOLDEN, &unit_text("test/decimal", "Config", &body));
    }
    let body = "invariant SocFloor using v on M::Battery at current { self.soc >= 0.20 }\n";
    check(O2, &unit_text("example/plan-battery", "M", body));
}

/// FR-056-AC-68 (TC-3042 step 5): each single defect refuses once at intake
/// with the code and cause the criterion lists.
#[trace("TC-3042", "FR-056-AC-68")]
#[test]
#[ignore = "QSL-2451: the Decimal value type reader is not implemented yet"]
fn decimal_policy_defects_refuse_once() {
    let malformed = ("invalid_model_binding", "malformed-declaration");
    let form = ("unsupported_construct", "declaration-form");
    let mut cases: Vec<(&str, Value, (&str, &str))> = Vec::new();
    let mut document = golden();
    type_mut(&mut document, "Soc")
        .as_object_mut()
        .expect("Soc")
        .remove("decimal");
    cases.push(("Soc without a policy", document, malformed));
    let mut document = golden();
    field_mut(&mut document, "fee")
        .as_object_mut()
        .expect("fee")
        .remove("decimal");
    cases.push(("fee without a policy", document, malformed));
    let mut document = golden();
    field_mut(&mut document, "soc")["decimal"] = serde_json::json!({"precision": 5, "scale": 2});
    cases.push(("two policies", document, malformed));
    for (precision, scale) in [(39, 2), (0, 0), (2, 3)] {
        let mut document = golden();
        type_mut(&mut document, "Soc")["decimal"] =
            serde_json::json!({"precision": precision, "scale": scale});
        cases.push(("policy schema", document, malformed));
    }
    for operand in [
        serde_json::json!("0.001"),
        serde_json::json!("1000"),
        serde_json::json!("00"),
        serde_json::json!(0),
    ] {
        let mut document = golden();
        type_mut(&mut document, "Soc")["constraints"][0]["operands"]["value"] = operand;
        cases.push(("operand", document, malformed));
    }
    let mut document = golden();
    type_mut(&mut document, "Soc")["unit"] = "ix://filament/unit/m".into();
    cases.push(("unit", document, malformed));
    let mut document = golden();
    let mut enumerated = type_mut(&mut document, "Soc")["constraints"][0].clone();
    enumerated["keyword"] = "enumValues".into();
    enumerated["operands"] = serde_json::json!({"values": ["0"]});
    type_mut(&mut document, "Soc")["constraints"]
        .as_array_mut()
        .expect("constraints")
        .push(enumerated.clone());
    cases.push(("enumValues", document.clone(), form));
    type_mut(&mut document, "Soc")["unit"] = "ix://filament/unit/m".into();
    cases.push(("unit before enumValues", document, malformed));
    let mut document = golden();
    let repeated = type_mut(&mut document, "Soc")["constraints"][0].clone();
    type_mut(&mut document, "Soc")["constraints"]
        .as_array_mut()
        .expect("constraints")
        .push(repeated);
    cases.push(("min twice", document, malformed));
    let mut document = golden();
    type_mut(&mut document, "Soc")["constraints"][0]["operands"]["value"] = "1.5".into();
    cases.push(("150 > 100", document, malformed));
    let mut document = golden();
    type_mut(&mut document, "Margin")["constraints"][1]["operands"]["value"] = "0.9".into();
    cases.push(("10 > 9", document, malformed));
    for (name, document, (code, cause)) in cases {
        assert_eq!(
            intake_refusals(&document),
            vec![(code.to_owned(), cause.to_owned())],
            "{name}"
        );
    }
}
