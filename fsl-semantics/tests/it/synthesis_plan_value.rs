// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-800 to FR-802: a plan synthesis types the plan value its goal and
//! objective read (`Reach.length`, `.final`, `.initial`, `.steps`), refuses a
//! plan length that is not a non-negative integer range, and refuses a read
//! of a member the plan value does not have.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::{CheckedSynthesis, PackageDeclarations};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input_from_sources;
use ix_trace_rs::trace;

const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total {
    pre CanPay { self.total > 0 };
    post Paid { self.total = pre(self.total) - 1 };
  }
}
population orders: Order;
";

/// Checks a unit declaring `Lengths` as `lengths` and one plan synthesis with
/// `objective` and `goal`; the checked plan synthesis, or the refusals.
fn check(lengths: &str, objective: &str, goal: &str) -> Result<CheckedSynthesis, String> {
    check_model(SHOP, lengths, objective, goal)
}

fn check_model(
    model_text: &str,
    lengths: &str,
    objective: &str,
    goal: &str,
) -> Result<CheckedSynthesis, String> {
    let model = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        model_text.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the model source is admitted");
    let packages =
        package_input_from_sources([("example/shop", &model)]).expect("the model unit compiles");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.temporal.infinite-trace/v1\";\n\
         model M = \"example/shop\";\n\
         type Lengths = {lengths};\n\
         synthesis Reach using v grammar M::Order domain Lengths {objective} \
         satisfies {{ {goal} }};\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "plan-value"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .map_err(|refusal| format!("{refusal:?}"))?;
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .map_err(|refusals| format!("{refusals:?}"))?;
    Ok(graph
        .syntheses()
        .first()
        .expect("the unit declares one synthesis")
        .clone())
}

#[trace("FSpec-FR-800")]
#[test]
fn a_plan_goal_reads_the_typed_plan_value() {
    let checked = check(
        "Int[0, 3]",
        "",
        "Reach.length >= 1 and Reach.final.total >= 0 and Reach.initial.total >= 0",
    )
    .expect("the goal reads the length and the state records");
    let plan = checked
        .plan()
        .expect("a plan synthesis carries its plan value");
    assert_eq!(
        plan.shape()
            .operations
            .iter()
            .map(|operation| operation.name.as_str())
            .collect::<Vec<_>>(),
        ["pay"]
    );
}

#[trace("FSpec-FR-802")]
#[test]
fn a_plan_objective_is_typed_without_an_expected_domain_type() {
    let checked = check("Int[0, 3]", "minimize Reach.length", "Reach.length >= 1")
        .expect("`minimize Reach.length` checks");
    assert!(checked.objective().is_some());
    assert_eq!(checked.objective_direction(), Some("minimize"));
}

#[trace("FSpec-FR-806-AC-2", "FSpec-TC-1956")]
#[test]
fn a_plan_declaration_refuses_what_the_plan_value_does_not_have() {
    let refusals = check("Int[-1, 3]", "", "Reach.length >= 1")
        .expect_err("a negative plan length is refused");
    assert!(refusals.contains("TypeMismatch"), "{refusals}");
    let refusals = check("Int[0, 3]", "minimize Reach.elapsed", "Reach.length >= 1")
        .expect_err("an untimed plan has no `elapsed`");
    assert!(
        refusals.contains("PlanUnknownField") && refusals.contains("field: \"elapsed\""),
        "{refusals}"
    );
    let refusals =
        check("Int[0, 3]", "", "Reach.unknown = 0").expect_err("an unknown plan field is refused");
    assert!(
        refusals.contains("PlanUnknownField") && refusals.contains("field: \"unknown\""),
        "{refusals}"
    );
}

/// The source adapter names finite argument forms it does not yet carry;
/// it never emits an argument record that silently omits those fields.
#[trace("FR-1013", "FSpec-FR-800")]
#[test]
fn unsupported_operation_parameters_are_named_before_search() {
    for (parameter, declarations) in [
        ("Integer", ""),
        ("Order", ""),
        ("Choice", "enum Choice { First, Second }\n"),
    ] {
        let model = SHOP.replace(
            "operation pay()",
            &format!("operation pay(amount: {parameter})"),
        );
        let model = model.replace("object Order", &format!("{declarations}object Order"));
        let refusal = check_model(&model, "Int[0,1]", "", "true")
            .expect_err("an unsupported argument type cannot be dropped from a step");
        assert!(refusal.contains("PlanUnsupported"), "{refusal}");
        assert!(
            refusal.contains("operation pay parameter amount"),
            "{refusal}"
        );
    }
    let model = SHOP
        .replace("operation pay()", "operation pay(amount: Boolean)")
        .replace(
            "population orders:",
            "type OtherAmount = Int[0,3];\nobject Other { operation change(amount: OtherAmount): Boolean; }\npopulation orders:",
        );
    let refusal = check_model(&model, "Int[0,1]", "", "true")
        .expect_err("conflicting argument fields cannot be silently omitted");
    assert!(refusal.contains("PlanUnsupported"), "{refusal}");
    assert!(
        refusal.contains("parameter amount has conflicting types or declared presence in operations pay and change"),
        "{refusal}"
    );
}

/// The existing authored Semantic-IR fixture builder can express declared
/// optional presence even though the native model-unit syntax cannot yet.
/// Equal Boolean types must not hide a presence-only arguments collision.
#[test]
#[trace("FR-1013", "FSpec-FR-800")]
fn conflicting_declared_parameter_presence_names_both_operations() {
    use super::model_operations::{
        assemble_units, config_version_document_with_operations, empty_frame, operation,
        operation_parameter,
    };
    use serde_json::json;

    for optional_first in [false, true] {
        let required = operation(
            "acceptRequired",
            json!([operation_parameter(
                "acceptRequired",
                "amount",
                "ix://filament/native/Boolean"
            )]),
            None,
            empty_frame(),
        );
        let mut parameter =
            operation_parameter("acceptOptional", "amount", "ix://filament/native/Boolean");
        parameter["presence"] = json!("optional");
        let optional = operation("acceptOptional", json!([parameter]), None, empty_frame());
        let operations = if optional_first {
            vec![optional, required]
        } else {
            vec![required, optional]
        };
        let document = config_version_document_with_operations(operations);
        let mut declarations = assemble_units(
            &[(
                "presence-collision",
                r#"
profile v = "filament.temporal.infinite-trace/v1";
model Config = "example/config-version";
type Lengths = Int[0,1];
synthesis Reach using v grammar Config::ConfigVersion domain Lengths satisfies { true };
"#,
            )],
            &[document.as_slice()],
        );
        declarations
            .link_dispatch(ModelNormalizationLimits::default())
            .expect("the two operations link");
        let refusals = match declarations.check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        ) {
            Err(refusals) => refusals,
            Ok(_) => panic!("a presence-only arguments collision must refuse"),
        };
        assert!(
            refusals.iter().any(|refusal| matches!(&refusal.cause,
            fsl_semantics::check::CheckCause::PlanUnsupported { form, .. }
                if form.contains("amount") && form.contains("declared presence")
                    && form.contains("acceptRequired") && form.contains("acceptOptional"))),
            "{refusals:?}"
        );
    }
}

/// Mixed receiver declarations use an existing sealed source union whose
/// members retain their exact reference payload type and settled identity.
#[test]
#[trace("FSpec-FR-800")]
fn mixed_operation_receivers_use_a_sealed_typed_union() {
    use filament_exact::{Presence, ValueType};
    use filament_semantic_value::declaration::CompositeShape;
    let model = SHOP.replace(
        "population orders",
        "object Other { operation idle(): Boolean; }\npopulation orders",
    );
    let checked = check_model(
        &model,
        "Int[1,1]",
        "",
        "forall(s in Reach.steps: s.receiver = s.receiver and s.operation = M::Op::pay and present(s.outcome_ordinal))",
    )
    .expect("the receiver and optional ordinal are source-readable");
    let plan = checked.plan().unwrap();
    let shape = plan.shape();
    assert_eq!(
        shape
            .operations
            .iter()
            .map(|operation| operation.name.as_str())
            .collect::<Vec<_>>(),
        ["pay", "idle"],
        "native invocation order remains declaration order"
    );
    assert!(!shape.operation_enum.declaration.declaration().is_ordered());
    assert_eq!(
        shape
            .operation_enum
            .members
            .iter()
            .map(|member| member.case())
            .collect::<Vec<_>>(),
        ["idle", "pay"],
        "unordered nominal cases use FR141 canonical order"
    );
    let handle = shape
        .receiver_union
        .as_ref()
        .expect("distinct receiver types use a union");
    let CompositeShape::Union(members) = plan
        .scope()
        .types()
        .composite(handle.clone())
        .unwrap()
        .shape()
    else {
        panic!("receiver union");
    };
    assert_eq!(members.len(), 2);
    for (member, (expected_type, expected_case)) in members.iter().zip(&shape.receiver_cases) {
        assert_eq!(member.identifier(), expected_case.as_str());
        assert_eq!(
            member.positions(),
            &[ValueType::Reference(expected_type.clone())]
        );
        let sealed = member
            .member()
            .expect("a generated receiver case is sealed");
        assert_ne!(
            sealed.declaration(),
            handle,
            "private handles are not member identities"
        );
    }
    assert_eq!(
        members[0].member().unwrap().declaration(),
        members[1].member().unwrap().declaration()
    );
    let CompositeShape::Record(fields) = plan
        .scope()
        .types()
        .composite(shape.step.clone())
        .unwrap()
        .shape()
    else {
        panic!("step record");
    };
    let receiver = fields
        .iter()
        .find(|field| field.name() == "receiver")
        .unwrap();
    assert_eq!(receiver.value_type(), &ValueType::Composite(handle.clone()));
    assert_eq!(receiver.presence(), Presence::Required);
    let ordinal = fields
        .iter()
        .find(|field| field.name() == "outcome_ordinal")
        .unwrap();
    assert_eq!(ordinal.value_type(), &ValueType::Integer);
    assert_eq!(ordinal.presence(), Presence::Optional);
}

/// Optional X and required Option<X> normalize to one record value type;
/// their native invocation omission flags remain a separate concern.
#[test]
#[trace("FSpec-FR-800", "FR-1013")]
fn normalized_option_parameters_do_not_report_a_shared_record_collision() {
    use super::model_operations::{
        assemble_units, config_version_document_with_operations, empty_frame, operation,
        operation_parameter,
    };
    use serde_json::{json, Value};
    for optional_first in [false, true] {
        let required = operation(
            "requiredOption",
            json!([operation_parameter(
                "requiredOption",
                "amount",
                "ix://example/config-version/MaybeBool"
            )]),
            None,
            empty_frame(),
        );
        let mut parameter =
            operation_parameter("optionalBool", "amount", "ix://filament/native/Boolean");
        parameter["presence"] = json!("optional");
        let optional = operation("optionalBool", json!([parameter]), None, empty_frame());
        let operations = if optional_first {
            vec![optional, required]
        } else {
            vec![required, optional]
        };
        let mut document: Value =
            serde_json::from_slice(&config_version_document_with_operations(operations)).unwrap();
        let identity = "ix://example/config-version/MaybeBool";
        document["types"].as_array_mut().unwrap().push(json!({
            "identity":identity, "displayName":"MaybeBool", "kind":"option", "roles":[],
            "inner":"ix://filament/native/Boolean", "constraints":[], "extensions":[], "unknownPolicy":"reject",
            "origin":{"generated":{"generatorIdentity":identity,"generatorVersion":"1.0.0","inputIdentities":[identity]}}
        }));
        let bytes = serde_json::to_vec(&document).unwrap();
        let mut declarations = assemble_units(
            &[(
                "normalized-arguments",
                r#"
profile v = "filament.temporal.infinite-trace/v1";
model Config = "example/config-version";
type Lengths = Int[0,1];
synthesis Reach using v grammar Config::ConfigVersion domain Lengths satisfies { true };
"#,
            )],
            &[bytes.as_slice()],
        );
        declarations
            .link_dispatch(ModelNormalizationLimits::default())
            .expect("both normalized declarations admit and link");
        let refusals = declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &crate::reward_evaluator::ZeroRewardEvaluator,
            )
            .expect_err("the individual Option/optional form remains explicitly staged");
        assert!(
            refusals.iter().any(|refusal| matches!(&refusal.cause,
            fsl_semantics::check::CheckCause::PlanUnsupported { form, .. }
                if form.contains("amount") && !form.contains("conflicting")
                    && !(form.contains("requiredOption") && form.contains("optionalBool")))),
            "{refusals:?}"
        );
    }
}
