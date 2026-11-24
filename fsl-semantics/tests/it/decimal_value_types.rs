// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056 "Decimal value types" (TC-3042): model intake reads a decimal
//! subject's type from the filament-core-data decimal policy and every bound
//! constraint on its resolution walk, as `Decimal[lo, hi; s, s; exact]`.
//! The golden package and its expected types live in `tests/fixtures/`.

use std::collections::BTreeMap;

use filament_exact::{DecimalType, Integer, RoundingMode, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{CheckCause, CheckedGraph, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{admit_unit, package_input, PackageInput, UnitIntakeCause};
use fsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, DocumentRef, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const GOLDEN: &[u8] =
    include_bytes!("../../../tests/fixtures/decimal-value-types.semantic-ir.json");
const EXPECTED: &[u8] = include_bytes!("../../../tests/fixtures/decimal-value-types.expected.json");
const O2: &[u8] = include_bytes!("../../../tests/fixtures/o2-battery-dispatch.semantic-ir.json");
const PACKAGE: &str = "test/decimal";

/// The unit text selecting `package` as `alias`, with `body` appended.
fn unit_text(package: &str, alias: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model {alias} = {package:?};\n\
         function keep using v(): Boolean pure {{ true }}\n{body}"
    )
}

/// The golden package, checked: its declarations, graph and package input.
struct Checked {
    declarations: PackageDeclarations,
    graph: CheckedGraph,
    packages: PackageInput,
}

/// Admits, assembles and checks `document` under the unit `unit`.
fn check(document: &[u8], unit: &str) -> Checked {
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
    Checked {
        declarations,
        graph,
        packages,
    }
}

fn check_golden(document: &Value) -> Checked {
    let bytes = serde_json::to_vec(document).expect("fixture encoding");
    check(&bytes, &unit_text(PACKAGE, "Config", ""))
}

impl Checked {
    fn meter(&self) -> filament_exact::EffectiveId {
        self.declarations
            .types
            .object_types()
            .find(|declared| declared.name() == "Config::Meter")
            .expect("Meter is declared")
            .key()
    }

    /// The checked type of `Meter.<field>`.
    fn field(&self, field: &str) -> ValueType {
        self.graph
            .scope()
            .types()
            .attributes(self.meter())
            .expect("Meter has attributes")
            .iter()
            .find(|attribute| attribute.field().name() == field)
            .map(|attribute| attribute.field().value_type().clone())
            .unwrap_or_else(|| panic!("{field} is declared"))
    }

    /// The checked parameter `by` and result types of `Meter.adjust`.
    fn adjust(&self) -> (ValueType, ValueType) {
        let operation = self
            .declarations
            .operations
            .declared_by(self.meter())
            .iter()
            .find(|operation| operation.name() == "adjust")
            .expect("adjust is declared")
            .clone();
        let by = operation
            .parameters()
            .iter()
            .find(|(name, _)| name == "by")
            .map(|(_, ty)| ty.clone())
            .expect("adjust has by");
        let result = operation.result().cloned().expect("adjust has a result");
        (by, result)
    }
}

fn decimal(lo: &str, hi: &str, scale: u64) -> ValueType {
    ValueType::Decimal(
        DecimalType::new(
            lo.parse::<Integer>().expect("lo"),
            hi.parse::<Integer>().expect("hi"),
            scale,
            scale,
            RoundingMode::Exact,
        )
        .expect("ordered bounds"),
    )
}

/// One intake refusal, as its typed parts.
#[derive(Debug)]
struct Refused {
    code: String,
    cause: String,
    detail: String,
}

/// The refusals intake raises for `document`; panics when it admits.
fn intake_refusals(document: &Value) -> Vec<Refused> {
    let bytes = serde_json::to_vec(document).expect("fixture encoding");
    let packages = package_input([bytes.as_slice()]).expect("byte admission");
    let unit = unit_text(PACKAGE, "Config", "");
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
    .expect_err("the defect refuses at intake and builds no declaration");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    refusals
        .iter()
        .map(|refusal| Refused {
            code: refusal.code.as_str().to_owned(),
            cause: refusal.cause.as_str().to_owned(),
            detail: refusal.detail.clone(),
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

/// Renames the alias `Rate` to `Amount`, and the `rate` field's reference to it,
/// so the document names no declaration it does not carry.
fn rename_rate_to_amount(document: &mut Value) -> &mut Value {
    let identity = "ix://test/decimal/Amount";
    field_mut(document, "rate")["typeRef"] = identity.into();
    let amount = type_mut(document, "Rate");
    amount["identity"] = identity.into();
    amount["displayName"] = "Amount".into();
    amount["origin"]["generated"]["generatorIdentity"] = identity.into();
    amount["origin"]["generated"]["inputIdentities"] = json!([identity]);
    amount
}

/// A bound constraint on the type `owner`, with an identity of its own.
fn bound(owner: &str, tag: &str, keyword: &str, value: Value) -> Value {
    let owner = format!("ix://test/decimal/{owner}");
    let identity = format!("{owner}/constraints/{tag}");
    json!({
        "identity": identity, "keyword": keyword, "operands": {"value": value},
        "appliesTo": owner, "diagnosticCode": format!("bound.{keyword}"),
        "origin": {"generated": {
            "generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }},
    })
}

/// FR-056-AC-66 (TC-3042 step 1): every field, the parameter `adjust.by` and
/// the result of `adjust` have the golden `Decimal[lo, hi; s, s; exact]`.
#[trace("TC-3042", "FR-056-AC-66")]
#[test]
fn decimal_members_have_the_golden_policy_types() {
    let checked = check(GOLDEN, &unit_text(PACKAGE, "Config", ""));
    let (by, result) = checked.adjust();
    let expected: Value = serde_json::from_slice(EXPECTED).expect("expected JSON");
    for entry in expected["members"].as_array().expect("members") {
        let member = entry["member"].as_str().expect("member");
        let declared = match member {
            "Meter.adjust.by" => by.clone(),
            "Meter.adjust.result" => result.clone(),
            other => checked.field(other.strip_prefix("Meter.").expect("a Meter member")),
        };
        let scale = entry["smax"].as_u64().expect("a scale");
        assert_eq!(entry["smin"].as_u64(), Some(scale), "{member}: smin = smax");
        assert_eq!(entry["mode"], "exact", "{member}");
        let want = decimal(
            entry["lo"].as_str().expect("lo"),
            entry["hi"].as_str().expect("hi"),
            scale,
        );
        assert_eq!(declared, want, "{member}");
    }
}

/// FR-056-AC-63 (TC-3031): the named Amount alias of native Decimal carries
/// its policy into a field and supports Decimal equality in a clause.
#[trace("TC-3031", "FR-056-AC-63")]
#[test]
fn amount_decimal_native_alias_has_its_policy_and_checks_equality() {
    let mut document = golden();
    let amount_identity = "ix://test/decimal/Amount";
    let amount = type_mut(&mut document, "Rate");
    amount["identity"] = amount_identity.into();
    amount["displayName"] = "Amount".into();
    amount["constraints"] = json!([]);
    amount["decimal"] = json!({"precision": 5, "scale": 2});
    amount["origin"]["generated"]["generatorIdentity"] = amount_identity.into();
    amount["origin"]["generated"]["inputIdentities"] = json!([amount_identity]);

    let field_identity = "ix://test/decimal/Meter/amount";
    let amount_field = field_mut(&mut document, "rate");
    amount_field["identity"] = field_identity.into();
    amount_field["name"] = "amount".into();
    amount_field["typeRef"] = amount_identity.into();
    amount_field["origin"]["generated"]["generatorIdentity"] = field_identity.into();
    amount_field["origin"]["generated"]["inputIdentities"] = json!([field_identity]);

    let bytes = serde_json::to_vec(&document).expect("Amount fixture encoding");
    let body = "invariant AmountEquality using v on Config::Meter at current { self.amount = self.amount }\n";
    let checked = check(&bytes, &unit_text(PACKAGE, "Config", body));
    assert_eq!(checked.field("amount"), decimal("-99999", "99999", 2));
}

/// The document with the alias-shaped construct a value type bound to a native
/// type by `target` is declared under.
fn with_value_alias_construct(document: &mut Value) {
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(json!({
            "kind": {"module": "test/decimal", "name": "value_alias"},
            "moduleVersion": "1.0.0",
            "construct": {
                "identity": "none", "shape": "alias", "members": {"target": "required"},
                "meaning": "filament.meaning.model.value-type/v1",
            },
        }));
}

/// A `Cash` value type bound to native Decimal by `target`, declared under the
/// alias-shaped construct, with no policy. It keeps every other member of the
/// golden `Price` value type it is derived from.
fn cash_value_type(document: &mut Value) -> Value {
    let identity = "ix://test/decimal/Cash";
    let mut cash = type_mut(document, "Price").clone();
    let object = cash.as_object_mut().expect("Cash");
    object.remove("scalar");
    object.remove("decimal");
    object.insert("identity".to_owned(), identity.into());
    object.insert("displayName".to_owned(), "Cash".into());
    object.insert(
        "kind".to_owned(),
        json!({"module": "test/decimal", "name": "value_alias"}),
    );
    object.insert("target".to_owned(), "ix://filament/native/Decimal".into());
    cash["origin"]["generated"]["generatorIdentity"] = identity.into();
    cash["origin"]["generated"]["inputIdentities"] = json!([identity]);
    cash
}

/// FR-056-AC-63 (TC-3031 step 2): the value type `Cash`, bound to native
/// Decimal by `target` with no policy, is admitted at intake, and a clause
/// comparing a `Cash` field refuses `ill_typed`/`operator-ineligible`.
#[trace("TC-3031", "FR-056-AC-63")]
#[test]
fn target_bound_cash_without_policy_is_operator_ineligible() {
    let mut document = golden();
    with_value_alias_construct(&mut document);
    let cash = cash_value_type(&mut document);
    document["types"].as_array_mut().expect("types").push(cash);
    let mut cash_field = field_mut(&mut document, "price").clone();
    cash_field["identity"] = "ix://test/decimal/Meter/cash".into();
    cash_field["name"] = "cash".into();
    cash_field["typeRef"] = "ix://test/decimal/Cash".into();
    type_mut(&mut document, "Meter")["fields"]
        .as_array_mut()
        .expect("Meter fields")
        .push(cash_field);

    let bytes = serde_json::to_vec(&document).expect("Cash fixture encoding");
    let packages = package_input([bytes.as_slice()]).expect("Cash package bytes");
    let body =
        "invariant CashEquality using v on Config::Meter at current { self.cash = self.cash }\n";
    let unit = unit_text(PACKAGE, "Config", body);
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "decimal-cash"),
        "cash.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = build_unit(&parsed).expect("S2 builds the unit");
    let selected = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 admits Cash: {refusal:?}"));
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("Cash unit assembles: {refusal:?}"));
    let refusals = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect_err("Cash has no Decimal policy and no checked value type");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let CheckCause::StateModel(state) = &*refusals[0].cause else {
        panic!("a state-model refusal: {refusals:?}");
    };
    let CheckCause::UntypedMember(member) = &state.cause else {
        panic!("an untyped-member refusal: {refusals:?}");
    };
    assert_eq!(
        (
            refusals[0].cause.code().as_str(),
            refusals[0].cause.cause(),
            member.owner.as_str(),
            member.member.as_str(),
            member.locus.as_str(),
        ),
        (
            "ill_typed",
            Some("operator-ineligible"),
            "Meter",
            "cash",
            "Field.typeRef"
        ),
        "{refusals:?}"
    );
}

/// FR-056-AC-63 (TC-3031): an alias of native Decimal that declares no policy,
/// typing a member that declares none either, refuses once at intake with the
/// validator's `DECIMAL_POLICY_MISSING`, so no declaration has an untyped Decimal.
#[trace("TC-3031", "FR-056-AC-63")]
#[test]
fn policy_less_cash_alias_member_refuses_once() {
    let mut document = golden();
    let cash_identity = "ix://test/decimal/Cash";
    let mut cash = type_mut(&mut document, "Rate").clone();
    cash["identity"] = cash_identity.into();
    cash["displayName"] = "Cash".into();
    cash["constraints"] = json!([]);
    cash.as_object_mut().expect("Cash").remove("decimal");
    cash["origin"]["generated"]["generatorIdentity"] = cash_identity.into();
    cash["origin"]["generated"]["inputIdentities"] = json!([cash_identity]);
    document["types"].as_array_mut().expect("types").push(cash);

    let mut cash_field = field_mut(&mut document, "price").clone();
    cash_field["identity"] = "ix://test/decimal/Meter/cash".into();
    cash_field["name"] = "cash".into();
    cash_field["typeRef"] = cash_identity.into();
    type_mut(&mut document, "Meter")["fields"]
        .as_array_mut()
        .expect("Meter fields")
        .push(cash_field);

    let refusals = intake_refusals(&document);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        (refusals[0].code.as_str(), refusals[0].cause.as_str()),
        ("invalid_model_binding", "malformed-declaration")
    );
    assert!(
        refusals[0].detail.contains("DECIMAL_POLICY_MISSING"),
        "{refusals:?}"
    );
}

/// FR-056-AC-72 (TC-3042 step 5): an alias chain that closes on itself refuses
/// once at intake with the validator's `ALIAS_CYCLE`, at its `target`.
#[trace("TC-3042", "FR-056-AC-72")]
#[test]
fn cyclic_decimal_alias_refuses_at_intake() {
    let mut document = golden();
    let amount = rename_rate_to_amount(&mut document);
    amount["target"] = "ix://test/decimal/Amount".into();
    amount["constraints"] = json!([]);
    amount["decimal"] = json!({"precision": 5, "scale": 2});

    let refusals = intake_refusals(&document);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        (refusals[0].code.as_str(), refusals[0].cause.as_str()),
        ("invalid_model_binding", "malformed-declaration")
    );
    assert!(refusals[0].detail.contains("ALIAS_CYCLE"), "{refusals:?}");
    assert!(refusals[0].detail.contains("target"), "{refusals:?}");
}

/// FR-056-AC-67 (TC-3042 step 1): every bound on every node of the walk
/// narrows the range and the tighter wins; `lo > hi` refuses.
#[trace("TC-3042", "FR-056-AC-67")]
#[test]
fn bounds_on_every_node_of_the_walk_narrow() {
    let golden_checked = check_golden(&golden());
    assert_eq!(golden_checked.field("half"), decimal("0", "50", 2));
    assert_eq!(golden_checked.field("soc"), decimal("0", "100", 2));
    for (keyword, value, lo, hi) in [
        ("max", "2", "0", "100"),
        ("exclusiveMin", "0.1", "11", "100"),
        ("min", "0.6", "60", "100"),
    ] {
        let mut document = golden();
        type_mut(&mut document, "Half")["constraints"] =
            json!([bound("Half", keyword, keyword, json!(value))]);
        let checked = check_golden(&document);
        assert_eq!(
            checked.field("half"),
            decimal(lo, hi, 2),
            "{keyword} {value}"
        );
        assert_eq!(
            checked.field("soc"),
            decimal("0", "100", 2),
            "{keyword} {value}"
        );
    }
    let mut document = golden();
    type_mut(&mut document, "Half")["constraints"] =
        json!([bound("Half", "min", "min", json!("1.5"))]);
    let refusals = intake_refusals(&document);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        (refusals[0].code.as_str(), refusals[0].cause.as_str()),
        ("invalid_model_binding", "malformed-declaration")
    );
    for named in ["constraints", "150", "100"] {
        assert!(refusals[0].detail.contains(named), "{refusals:?}: {named}");
    }
}

/// FR-056-AC-68 (TC-3042 steps 2-3): the Decimal members check in clauses,
/// and FSpec's o2 planning model admits and checks its `SocFloor` invariant.
#[trace("TC-3042", "FR-056-AC-68")]
#[test]
fn decimal_members_check_in_clauses() {
    for clause in [
        "self.soc >= 0.20",
        "self.soc = self.soc",
        "self.fee = self.fee",
        "self.rate <= 1.0",
    ] {
        let body = format!("invariant I using v on Config::Meter at current {{ {clause} }}\n");
        check(GOLDEN, &unit_text(PACKAGE, "Config", &body));
    }
    let body = "invariant SocFloor using v on M::Battery at current { self.soc >= 0.20 }\n";
    check(O2, &unit_text("example/plan-battery", "M", body));
}

fn coefficient(c: &str, s: &str) -> Value {
    json!({"kind": "decimal", "coefficient": c, "scale": s})
}

/// A `Meter` holding a member of every field's domain, with `put` replacing
/// one field's value.
fn meter_fields(put: (&str, Value)) -> Value {
    let mut fields = json!({
        "soc": coefficient("50", "2"),
        "price": coefficient("0", "0"),
        "margin": coefficient("0", "0"),
        "tax": coefficient("125", "3"),
        "whole": coefficient("0", "0"),
        "fee": coefficient("0", "0"),
        "rate": coefficient("1", "0"),
        "half": coefficient("0", "0"),
    });
    fields[put.0] = put.1;
    fields
}

/// Admits a current snapshot of the golden package holding one `Meter`.
fn admit_meter(
    checked: &Checked,
    fields: Value,
) -> Result<(), (String, BTreeMap<&'static str, String>)> {
    let reference = DocumentRef {
        authority: "test".to_owned(),
        identity: "decimal-current".to_owned(),
    };
    let document = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "decimal-current"},
        "observation": "current",
        "model": {"identity": PACKAGE},
        "populations": [{
            "population": "ix://test/decimal/meters", "complete": true,
            "objects": [{"key": "m1", "type": "ix://test/decimal/Meter", "fields": fields}],
        }],
    });
    let provision = BTreeMap::from([(
        reference.clone(),
        serde_json::to_vec(&document).expect("snapshot encoding"),
    )]);
    match admit_current_snapshot(
        checked.graph.model_selections(),
        checked.graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
        &provision,
        &reference,
        ObservationLimits::default(),
    ) {
        Ok(_) => Ok(()),
        Err(AdmissionFailure::Refused(record)) => {
            Err((record.catalog.cause().to_owned(), record.fields.clone()))
        }
        Err(other) => panic!("unexpected admission failure: {other:?}"),
    }
}

/// FR-056-AC-71 (TC-3042 step 4): snapshot values admit by FSpec FR-140
/// membership by value in the field's Decimal value type.
#[trace("TC-3042", "FR-056-AC-71")]
#[test]
fn snapshot_decimals_admit_by_membership() {
    let checked = check(GOLDEN, &unit_text(PACKAGE, "Config", ""));
    for (field, c, s) in [
        ("soc", "50", "2"),
        ("soc", "5", "1"),
        ("soc", "1", "0"),
        ("soc", "0", "0"),
        ("margin", "9", "1"),
        ("tax", "125", "3"),
    ] {
        assert_eq!(
            admit_meter(&checked, meter_fields((field, coefficient(c, s)))),
            Ok(()),
            "{field} {c}e-{s}"
        );
    }
    for (field, c, s, domain) in [
        ("soc", "101", "2", "Decimal[0, 100; 2, 2; exact]"),
        ("soc", "-1", "2", "Decimal[0, 100; 2, 2; exact]"),
        ("soc", "5", "3", "Decimal[0, 100; 2, 2; exact]"),
        ("margin", "1", "0", "Decimal[-9, 9; 1, 1; exact]"),
        ("margin", "-1", "0", "Decimal[-9, 9; 1, 1; exact]"),
        ("tax", "124", "3", "Decimal[125, 999; 3, 3; exact]"),
    ] {
        let refused = admit_meter(&checked, meter_fields((field, coefficient(c, s))))
            .expect_err("a non-member refuses");
        assert_eq!(refused.0, "invalid-value", "{field} {c}e-{s}");
        assert_eq!(
            refused.1.get("field").map(String::as_str),
            Some(field),
            "{field} {c}e-{s}"
        );
        assert_eq!(
            refused.1.get("expected").map(String::as_str),
            Some(domain),
            "{field} {c}e-{s}"
        );
        assert_eq!(
            refused.1.get("coefficient").map(String::as_str),
            Some(c),
            "{field} {c}e-{s}"
        );
        assert_eq!(
            refused.1.get("scale").map(String::as_str),
            Some(s),
            "{field} {c}e-{s}"
        );
    }
}

/// FR-056-AC-72 (TC-3042 step 5): each single defect refuses once at intake
/// with the code, cause, diagnostic and named item the criterion lists, and
/// builds no declaration.
#[trace("TC-3042", "FR-056-AC-72")]
#[test]
fn decimal_policy_defects_refuse_once() {
    const VALIDATOR: &str = "agent-ix.semantic-ir.";
    // (name, document, cause, text the detail holds, whether a validator
    // diagnostic is retained)
    let mut cases: Vec<(&str, Value, &str, &str, bool)> = Vec::new();
    let mut document = golden();
    type_mut(&mut document, "Soc")
        .as_object_mut()
        .expect("Soc")
        .remove("decimal");
    cases.push((
        "Soc without a policy",
        document,
        "malformed-declaration",
        "DECIMAL_POLICY_MISSING",
        true,
    ));
    let mut document = golden();
    field_mut(&mut document, "fee")
        .as_object_mut()
        .expect("fee")
        .remove("decimal");
    cases.push((
        "fee without a policy",
        document,
        "malformed-declaration",
        "DECIMAL_POLICY_MISSING",
        true,
    ));
    let mut document = golden();
    let amount = rename_rate_to_amount(&mut document);
    amount["constraints"] = json!([]);
    amount.as_object_mut().expect("Amount").remove("decimal");
    cases.push((
        "Amount without a policy",
        document,
        "malformed-declaration",
        "DECIMAL_POLICY_MISSING",
        true,
    ));
    let mut document = golden();
    field_mut(&mut document, "soc")["decimal"] = json!({"precision": 5, "scale": 2});
    cases.push((
        "two policies",
        document,
        "malformed-declaration",
        "DECIMAL_POLICY_CONFLICT",
        true,
    ));
    let mut document = golden();
    with_value_alias_construct(&mut document);
    let mut cash = cash_value_type(&mut document);
    cash["decimal"] = json!({"precision": 5, "scale": 2});
    document["types"].as_array_mut().expect("types").push(cash);
    cases.push((
        "policy on a target-bound value type",
        document,
        "malformed-declaration",
        "DECIMAL_POLICY_CONFLICT",
        false,
    ));
    let mut document = golden();
    type_mut(&mut document, "Soc")["decimal"] =
        json!({"precision": 5, "scale": 2, "rounding": "half_up"});
    cases.push((
        "policy with an extra member",
        document,
        "malformed-declaration",
        "rounding",
        true,
    ));
    let mut document = golden();
    type_mut(&mut document, "Half")["constraints"]
        .as_array_mut()
        .expect("constraints")
        .push(bound("Soc", "other-node", "min", json!("0.1")));
    cases.push((
        "bound held by another node",
        document,
        "malformed-declaration",
        "cannot be carried by another node",
        false,
    ));
    for (precision, scale) in [(39, 2), (0, 0), (2, 3)] {
        let mut document = golden();
        type_mut(&mut document, "Soc")["decimal"] = json!({"precision": precision, "scale": scale});
        cases.push((
            "policy schema",
            document,
            "malformed-declaration",
            "SCHEMA_VIOLATION",
            true,
        ));
    }
    for operand in [json!("0.001"), json!("1000"), json!("00"), json!(0)] {
        let mut document = golden();
        type_mut(&mut document, "Soc")["constraints"][0]["operands"]["value"] = operand;
        cases.push((
            "operand",
            document,
            "malformed-declaration",
            "INVALID_OPERAND",
            true,
        ));
    }
    let mut document = golden();
    type_mut(&mut document, "Soc")["unit"] = "ix://filament/unit/m".into();
    cases.push(("unit", document, "malformed-declaration", "unit", false));
    let mut document = golden();
    let enumerated = bound("Soc", "enumValues", "enumValues", json!(["0"]));
    let mut enumerated = enumerated;
    enumerated["operands"] = json!({"values": ["0"]});
    type_mut(&mut document, "Soc")["constraints"]
        .as_array_mut()
        .expect("constraints")
        .push(enumerated);
    cases.push((
        "enumValues",
        document.clone(),
        "declaration-form",
        "enumValues",
        false,
    ));
    type_mut(&mut document, "Soc")["unit"] = "ix://filament/unit/m".into();
    cases.push((
        "unit before enumValues",
        document,
        "malformed-declaration",
        "unit",
        false,
    ));
    let mut document = golden();
    type_mut(&mut document, "Soc")["constraints"]
        .as_array_mut()
        .expect("constraints")
        .push(bound("Soc", "min-again", "min", json!("0.5")));
    cases.push(("min twice", document, "malformed-declaration", "min", false));
    let mut document = golden();
    type_mut(&mut document, "Soc")["constraints"][0]["operands"]["value"] = "1.5".into();
    cases.push(("150 > 100", document, "malformed-declaration", "150", false));
    let mut document = golden();
    type_mut(&mut document, "Margin")["constraints"][1]["operands"]["value"] = "0.9".into();
    cases.push(("10 > 9", document, "malformed-declaration", "10", false));
    for (name, document, cause, named, validator) in cases {
        let refusals = intake_refusals(&document);
        assert_eq!(refusals.len(), 1, "{name}: {refusals:?}");
        let refusal = &refusals[0];
        let code = if cause == "declaration-form" {
            "unsupported_construct"
        } else {
            "invalid_model_binding"
        };
        assert_eq!(
            (refusal.code.as_str(), refusal.cause.as_str()),
            (code, cause),
            "{name}"
        );
        assert!(refusal.detail.contains(named), "{name}: {refusal:?}");
        assert_eq!(
            refusal.detail.contains(VALIDATOR) || refusal.detail.contains("SCHEMA_VIOLATION"),
            validator,
            "{name}: {refusal:?}"
        );
    }
}
