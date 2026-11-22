// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-740-AC-5, FR-740-AC-6, FR-569-AC-18 (TC-3012); FSpec FR-640-AC-6 and
//! FR-610-AC-9 (TC-2033): a delay, a random parameter and a workload declared
//! on an object type the unit declares in its own source are emitted on that
//! type's source `model`/`object_type` node, and the identity rows carry each
//! source operation's parameters.

#![cfg(feature = "std")]

use filament_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    NativeJoinBudget,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{emit_checked, CheckedPackage, EmitLimits, EmitRefusal, ReadIdentityRows};
use fsl_semantics::{
    check::{
        CheckCause, CheckRefusal, PackageDeclarations, ProbabilisticCause, ProbabilisticFeature,
    },
    model::{
        accounting::ModelNormalizationLimits,
        intake::{admit_unit, package_input, PackageInput},
    },
};
use ix_trace_rs::trace;
use serde_json::Value;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    profile t = \"filament.temporal.complete/v1\";\n\
    dimension Time; unit ms : Time = rational(1, 1);\n";

/// `body` after [`HEADER`], with no selected domain package, through S3.
fn check(body: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let source = format!("{HEADER}{body}");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:source-probabilistic"),
        "program.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert_eq!(parsed.diagnostics(), []);
    let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
    let inputs: PackageInput = package_input(std::iter::empty::<&[u8]>()).unwrap();
    let selected = admit_unit(
        &unit.selections().models,
        &inputs,
        ModelNormalizationLimits::default(),
    )
    .expect("empty selected models admit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "source-probabilistic".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("the package assembles");
    let _pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

/// The wire JSON of `package`, after the strict read and the identity-row
/// join have admitted it, with the identity file's JSON.
fn emitted_and_joined(package: &CheckedPackage) -> (Value, Value) {
    let emission = emit_checked(package, EmitLimits::new(10_000_000))
        .unwrap_or_else(|refusal| panic!("the package emits: {refusal:?}"));
    let wire: Value = serde_json::from_slice(emission.package().bytes()).expect("wire JSON");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let read = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    );
    let CheckedPackageV2ReadResult::Admitted(checked) = read else {
        panic!("the strict read admits the package: {read:?}");
    };
    let rows = emission
        .identity_rows(&checked, &mut NativeJoinBudget::new(10_000_000, 10_000_000))
        .expect("the identity-row join admits the package");
    let constructed = ReadIdentityRows::of_rows_with_source_rows(
        &checked,
        rows.declarations().to_vec(),
        rows.fields().to_vec(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
        rows.source_objects().to_vec(),
        rows.source_operations().to_vec(),
        emission.source_object_member_facts().to_vec(),
        emission.source_object_operations().to_vec(),
    );
    let identities = constructed
        .to_bytes(&checked)
        .expect("the identity rows serialize");
    (
        wire,
        serde_json::from_slice(&identities).expect("identity JSON"),
    )
}

/// The `probabilistic` nodes of `form` in `wire`.
fn probabilistic<'a>(wire: &'a Value, form: &str) -> Vec<&'a Value> {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["node_tag"] == "probabilistic" && node["semantic_form"] == form)
        .collect()
}

/// The node id of the source-declared `model`/`object_type` node `name`.
fn source_node<'a>(wire: &'a Value, name: &str) -> &'a Value {
    let node = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| {
            node["node_tag"] == "model"
                && node["semantic_form"] == "object_type"
                && node["declaration"]["qualified_name"] == serde_json::json!([name])
        })
        .unwrap_or_else(|| panic!("a source node for {name}"));
    &node["node_id"]
}

/// The `declaring` (or a workload's `model`) target of a probabilistic node.
fn first_reference(node: &Value) -> &Value {
    &node["body"]["members"][0]["value"]["target"]
}

const TIMED: &str = "object O { n: Integer; operation step() modifies self.n; }\n\
    object Other { n: Integer; operation poke() modifies self.n; }\n";

#[trace("TC-3012", "FR-740-AC-5")]
#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]
#[test]
fn a_delay_on_each_source_object_of_a_timed_source_model_is_emitted() {
    assert_delays_emitted(TIMED);
}

/// FR-740-AC-5's literal unit, whose `O` has a `Clock` field.
#[trace("TC-3012", "FR-740-AC-5")]
#[test]
fn a_delay_on_each_source_object_beside_a_source_clock_is_emitted() {
    assert_delays_emitted(&format!(
        "{TIMED_CLOCK}object Other {{ n: Integer; operation poke() modifies self.n; }}\n"
    ));
}

/// The delays of `objects` timed on `O` are emitted on their source object
/// nodes, the discrete one typed by the unit node of `ms`.
fn assert_delays_emitted(objects: &str) {
    let package = check(&format!(
        "{objects}time O dense unit ms;\n\
         delay O::step ~ uniform;\n\
         delay Other::poke ~ discrete {{ 1 ms: 9, 3 ms: 1 }};\n"
    ))
    .unwrap_or_else(|refusals| panic!("S3 admits the delays: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    let delays = probabilistic(&wire, "delay_distribution");
    assert_eq!(delays.len(), 2, "{delays:?}");
    let mut declaring: Vec<&Value> = delays.iter().map(|node| first_reference(node)).collect();
    declaring.sort_by_key(|id| id.to_string());
    let mut expected = vec![source_node(&wire, "O"), source_node(&wire, "Other")];
    expected.sort_by_key(|id| id.to_string());
    assert_eq!(declaring, expected);
    let discrete = delays
        .iter()
        .find(|node| node["body"].to_string().contains("\"discrete\""))
        .expect("the discrete delay");
    let unit = unit_node(&wire, "ms");
    let mut times = Vec::new();
    quantities(&discrete["body"], &mut times);
    assert_eq!(
        times,
        [("1".to_owned(), unit.clone()), ("3".to_owned(), unit)]
    );
}

#[trace("TC-3012", "FR-740-AC-5")]
#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]
#[test]
fn a_delay_on_an_untimed_source_model_is_refused_at_its_span() {
    let Err(refusals) = check(
        "object O { n: Integer; operation step() modifies self.n; }\n\
         delay O::step ~ uniform;\n",
    ) else {
        panic!("a delay on an untimed source model checks");
    };
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code().as_str(), "invalid_probabilistic_model");
    assert_eq!(refusal.cause.cause(), Some("delay-on-untimed-model"));
}

#[trace("TC-3012", "FR-740-AC-6")]
#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]
#[test]
fn a_random_parameter_and_a_workload_on_the_source_model_are_emitted() {
    let package = check(
        "object O { n: Integer; operation step(k: Int[1, 3]) modifies self.n; }\n\
         random O::step(k) ~ { 1: 1, 3: 3 };\n\
         workload W on O { weight O::step = 1; }\n",
    )
    .unwrap_or_else(|refusals| panic!("S3 admits the declarations: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    let object = source_node(&wire, "O");
    let random = probabilistic(&wire, "random_parameter");
    let workload = probabilistic(&wire, "workload");
    assert_eq!((random.len(), workload.len()), (1, 1));
    assert_eq!(first_reference(random[0]), object);
    assert_eq!(first_reference(workload[0]), object);
}

#[trace("TC-3012", "FR-569-AC-18")]
#[trace("FSpec-TC-2033", "FSpec-FR-610-AC-9")]
#[test]
fn a_source_operation_entry_lists_its_parameters() {
    let package = check(
        "object O { d: Clock; n: Integer; \
         operation arm(x: Clock, k: Int[1, 3]) modifies self.d; }\n\
         time O dense unit ms;\n",
    )
    .unwrap_or_else(|refusals| panic!("S3 admits the type: {refusals:?}"));
    let (_, identities) = emitted_and_joined(&package);
    let entry = &identities["source_objects"][0]["operations"][0];
    assert_eq!(entry["identity"], "O::arm");
    assert_eq!(entry["clock_parameters"], serde_json::json!(["x"]));
    let parameters = entry["parameters"].as_array().expect("parameters");
    assert_eq!(parameters.len(), 2);
    assert_eq!(
        parameters[0]["member_type"], "ix://filament/native/Clock",
        "{parameters:?}"
    );
    assert!(parameters[0]["type_node"].is_null());
    assert!(parameters[1]["type_node"].is_object(), "{parameters:?}");
}

const SWITCH: &str = "object Switch { n: Integer; \
    operation choose(k: Int[0, 1]) modifies self.n; operation clear() modifies self.n; }\n\
    object Other { n: Integer; operation poke() modifies self.n; }\n";

const LINEAGE: &str = "object Base { n: Integer; operation a() modifies self.n; }\n\
    object Sub extends Base { operation b() modifies self.n; }\n";

/// The one refusal of `body`, as its probabilistic cause.
fn refused(body: &str) -> ProbabilisticCause {
    let Err(refusals) = check(body) else {
        panic!("the unit is refused: {body}");
    };
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::Probabilistic(cause) = &refusal.cause else {
        panic!("a probabilistic cause: {refusal:?}");
    };
    (**cause).clone()
}

/// FR-185 "Workloads": the `on` name of a workload names a source object
/// type, whose operations the workload weighs under the rules of a selected
/// model. (No acceptance criterion states the source-model clause.)
#[trace("TC-620", "FR-185-AC-14")]
#[test]
fn a_workload_on_a_source_object_type_is_admitted_with_that_types_operations() {
    let package = check(&format!(
        "{SWITCH}workload Input on Switch {{ weight Switch::choose = 1; weight Switch::clear = 0; }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("S3 admits the workload: {refusals:?}"));
    let [workload] = package.graph().probabilistic_model().workloads() else {
        panic!("one checked workload");
    };
    assert_eq!(workload.name(), "Input");
    let weights: Vec<(&str, String)> = workload
        .weights()
        .iter()
        .map(|(operation, weight)| (operation.name(), weight.to_string()))
        .collect();
    assert_eq!(
        weights,
        [("choose", "1/1".to_owned()), ("clear", "0/1".to_owned())]
    );
    let switch = workload.weights()[0].0.declaring_key().expect("a key");
    assert_eq!(workload.model_key(), Some(switch));
}

#[trace("TC-620", "FR-185-AC-14")]
#[test]
fn a_workload_on_a_source_object_type_weighs_only_that_types_operations() {
    let cause = refused(&format!(
        "{SWITCH}workload Input on Switch {{ weight Switch::choose = 1; weight Switch::clear = 1; \
         weight Other::poke = 1; }}\n"
    ));
    assert!(
        matches!(&cause, ProbabilisticCause::UnknownOperation { operation, .. }
            if operation == "Other::poke"),
        "{cause:?}"
    );
    let cause = refused(&format!(
        "{SWITCH}workload Input on Switch {{ weight Switch::choose = 1; }}\n"
    ));
    assert!(
        matches!(&cause, ProbabilisticCause::IncompleteWorkload { operation, .. }
            if operation == "Switch::clear"),
        "{cause:?}"
    );
    let cause = refused(&format!(
        "{SWITCH}workload Input on Missing {{ weight Switch::choose = 1; }}\n"
    ));
    assert!(
        matches!(&cause, ProbabilisticCause::UnresolvedModel { model, .. } if model == "Missing"),
        "{cause:?}"
    );
}

/// A source object type's operations include those it inherits, as a
/// selected model's workload weighs an operation of an ancestor type.
#[trace("TC-620", "FR-185-AC-14")]
#[test]
fn a_workload_on_a_source_subtype_weighs_and_requires_its_inherited_operations() {
    for weights in [
        "weight Sub::b = 1; weight Sub::a = 1;",
        "weight Sub::b = 1; weight Base::a = 2;",
    ] {
        let package = check(&format!("{LINEAGE}workload W on Sub {{ {weights} }}\n"))
            .unwrap_or_else(|refusals| panic!("{weights}: S3 admits: {refusals:?}"));
        let [workload] = package.graph().probabilistic_model().workloads() else {
            panic!("one checked workload");
        };
        let mut names: Vec<&str> = workload.weights().iter().map(|(o, _)| o.name()).collect();
        names.sort_unstable();
        assert_eq!(names, ["a", "b"], "{weights}");
    }
    let cause = refused(&format!(
        "{LINEAGE}workload W on Sub {{ weight Sub::b = 1; }}\n"
    ));
    assert!(
        matches!(&cause, ProbabilisticCause::IncompleteWorkload { operation, .. }
            if operation == "Base::a"),
        "{cause:?}"
    );
    let cause = refused(&format!(
        "{LINEAGE}workload W on Base {{ weight Base::a = 1; weight Sub::b = 1; }}\n"
    ));
    assert!(
        matches!(&cause, ProbabilisticCause::UnknownOperation { operation, .. }
            if operation == "Sub::b"),
        "{cause:?}"
    );
}

/// A subtype's redefinition hides the ancestor's operation of that name, for a
/// pure operation and for one that modifies state.
#[trace("TC-620", "FR-185-AC-14")]
#[test]
fn a_workload_on_a_source_subtype_weighs_a_redefined_operation_once() {
    for (base, redefinition) in [
        (
            "operation f(p: Int[0, 9]): Integer;",
            "operation f(p: Int[0, 9]): Integer redefines Base::f;",
        ),
        (
            "operation f() modifies self.n;",
            "operation f() redefines Base::f modifies self.n;",
        ),
    ] {
        let units = format!(
            "object Base {{ n: Integer; {base} }}\n\
             object Sub extends Base {{ {redefinition} operation g() modifies self.n; }}\n"
        );
        let package = check(&format!(
            "{units}workload W on Sub {{ weight Sub::f = 1; weight Sub::g = 1; }}\n"
        ))
        .unwrap_or_else(|refusals| panic!("{base}: S3 admits: {refusals:?}"));
        let [workload] = package.graph().probabilistic_model().workloads() else {
            panic!("one checked workload");
        };
        assert_eq!(workload.weights().len(), 2, "{base}");
        let cause = refused(&format!(
            "{units}workload W on Sub {{ weight Sub::g = 1; }}\n"
        ));
        assert!(
            matches!(&cause, ProbabilisticCause::IncompleteWorkload { operation, .. }
                if operation == "Sub::f"),
            "{base}: {cause:?}"
        );
        let cause = refused(&format!(
            "{units}workload W on Sub {{ weight Sub::f = 1; weight Sub::g = 1; weight Base::f = 1; }}\n"
        ));
        assert!(
            matches!(&cause, ProbabilisticCause::UnknownOperation { operation, .. }
                if operation == "Base::f"),
            "{base}: {cause:?}"
        );
    }
}

const DIAMOND: &str = "object B { n: Integer; operation x() modifies self.n; }\n\
    object C { m: Integer; operation x() modifies self.m; }\n\
    object D extends B, C { operation y() modifies self.n; }\n";

/// A name two unrelated ancestors declare is carried by the subtype twice: a
/// workload weighs and requires each declaration by its own spelling, and
/// FR-104 refuses only the written `D::x`.
#[trace("TC-620", "FR-185-AC-14")]
#[test]
fn a_workload_on_a_source_subtype_requires_each_ambiguous_ancestor_operation() {
    let package = check(&format!(
        "{DIAMOND}workload W on D {{ weight D::y = 1; weight B::x = 1; weight C::x = 1; }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let [workload] = package.graph().probabilistic_model().workloads() else {
        panic!("one checked workload");
    };
    assert_eq!(workload.weights().len(), 3);

    let Err(refusals) = check(&format!(
        "{DIAMOND}workload W on D {{ weight D::y = 1; }}\n"
    )) else {
        panic!("a workload omitting both is refused");
    };
    let omitted: Vec<&str> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::Probabilistic(cause) => match &**cause {
                ProbabilisticCause::IncompleteWorkload { operation, .. } => {
                    Some(operation.as_str())
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(omitted, ["B::x", "C::x"], "{refusals:?}");

    let Err(refusals) = check(&format!(
        "{DIAMOND}workload W on D {{ weight D::y = 1; weight D::x = 1; }}\n"
    )) else {
        panic!("a written D::x is ambiguous");
    };
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::Probabilistic(cause)
                if matches!(&**cause, ProbabilisticCause::AmbiguousOperation { name, .. } if name == "D::x")
        )),
        "{refusals:?}"
    );
}

const TIMED_CLOCK: &str = "object O { d: Clock; operation step() modifies self.d; }\n";

/// The QSL-2045 shape: a source `Switch` with a Boolean random parameter, a
/// reward, and a workload on it emits and is admitted by the strict read and
/// the identity-row join.
#[trace("TC-3012", "FR-740-AC-6")]
#[test]
fn a_boolean_source_switch_with_a_workload_and_a_reward_emits() {
    let package = check(
        "object Switch { n: Integer; operation choose(bit: Boolean) modifies self.n; \
         operation clear() modifies self.n; }\n\
         random Switch::choose(bit) ~ { true: 1, false: 3 };\n\
         reward cost on Switch::choose = 1;\n\
         workload Input on Switch { weight Switch::choose = 1; weight Switch::clear = 0; }\n",
    )
    .unwrap_or_else(|refusals| panic!("S3 admits the unit: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    let object = source_node(&wire, "Switch");
    let random = probabilistic(&wire, "random_parameter");
    let workload = probabilistic(&wire, "workload");
    assert_eq!((random.len(), workload.len()), (1, 1));
    assert_eq!(first_reference(random[0]), object);
    assert_eq!(first_reference(workload[0]), object);
    assert_eq!(probabilistic(&wire, "reward").len(), 1);
    // The emitted bodies carry the Boolean support and the workload weights.
    let support: Vec<(Value, Value, Value)> = random[0]["body"]["members"][3]["value"]["members"]
        .as_array()
        .expect("support")
        .iter()
        .map(|pair| {
            let value = &pair["members"][0]["value"];
            (
                value["value_kind"].clone(),
                value["value"].clone(),
                pair["members"][1]["value"]["value"].clone(),
            )
        })
        .collect();
    let boolean = serde_json::json!("boolean");
    assert_eq!(
        support,
        [
            (
                boolean.clone(),
                serde_json::json!(true),
                serde_json::json!("1")
            ),
            (boolean, serde_json::json!(false), serde_json::json!("3")),
        ]
    );
    let weights: Vec<(&Value, &Value, &Value)> = workload[0]["body"]["members"][2]["value"]
        ["members"]
        .as_array()
        .expect("weights")
        .iter()
        .map(|entry| {
            (
                &entry["members"][0]["value"]["target"],
                &entry["members"][1]["value"]["value"],
                &entry["members"][2]["value"]["value"],
            )
        })
        .collect();
    assert_eq!(
        weights,
        [
            (
                object,
                &serde_json::json!("choose"),
                &serde_json::json!("1")
            ),
            (object, &serde_json::json!("clear"), &serde_json::json!("0")),
        ]
    );
}

/// The node id of the unit node named `name`.
fn unit_node(wire: &Value, name: &str) -> Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["declaration"]["qualified_name"] == serde_json::json!([name]))
        .unwrap_or_else(|| panic!("a node for {name}"))["node_id"]
        .clone()
}

/// Every quantity literal under `term`, as its magnitude and its type node.
fn quantities(term: &Value, found: &mut Vec<(String, Value)>) {
    match term {
        Value::Object(map) => {
            if map.get("value_kind").is_some_and(|kind| kind == "quantity") {
                found.push((
                    map["value"].as_str().expect("a magnitude").to_owned(),
                    map["type"].clone(),
                ));
            }
            map.values().for_each(|inner| quantities(inner, found));
        }
        Value::Array(items) => items.iter().for_each(|inner| quantities(inner, found)),
        _ => {}
    }
}

/// FSpec FR-640 Reader rule 3 reads a workload's operation names as distinct,
/// so a workload on a subtype of two parents that each declare `x` weighs two
/// operations of one name: S3 admits it (FR-185-AC-14) and S4 refuses it at
/// the workload's span rather than write rows the strict read refuses without
/// a locus (QSL-2707).
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_workload_weighing_two_operations_of_one_name_is_refused_at_its_span() {
    let source = "object B { n: Integer; operation x() modifies self.n; }\n\
        object C { m: Integer; operation x() modifies self.m; }\n\
        object D extends B, C { operation y() modifies self.n; }\n\
        workload W on D { weight D::y = 1; weight B::x = 1; weight C::x = 1; }\n";
    let package = check(source).unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let refusal = emit_checked(&package, EmitLimits::new(10_000_000))
        .expect_err("a repeated operation name is refused");
    let EmitRefusal::ProbabilisticModelNotEmitted {
        declaration,
        feature,
        region,
        ..
    } = &refusal
    else {
        panic!("a located probabilistic refusal: {refusal:?}");
    };
    assert_eq!(declaration, "W");
    assert_eq!(*feature, ProbabilisticFeature::Workload);
    let text = format!("{HEADER}{source}");
    let label = text
        .get(region.start() as usize..region.end() as usize)
        .expect("the region lies in the unit");
    assert!(label.contains('W'), "{label}");
    assert!(!refusal.to_string().contains("Span {"), "{refusal}");
}

/// `units`, each a source identity and its text, assembled as one package
/// with no selected domain package, through S3.
fn check_units(units: &[(&str, &str)]) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    check_units_named("source-probabilistic", units)
}

/// [`check_units`] for the package named `package`.
fn check_units_named(
    package: &str,
    units: &[(&str, &str)],
) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let units: Vec<_> = units
        .iter()
        .enumerate()
        .map(|(at, (identity, text))| {
            // The dimension and unit of `HEADER` are declared once, by the first unit.
            let header: String = HEADER
                .lines()
                .take(if at == 0 { usize::MAX } else { 3 })
                .map(|line| format!("{line}\n"))
                .collect();
            let parsed = fsl_cst::parse(
                fsl_foundation::SourceIdentity::new("agent-ix", *identity),
                "program.native",
                format!("{header}{text}").as_bytes(),
                fsl_cst::Limits::default(),
            )
            .expect("the source parses");
            assert_eq!(parsed.diagnostics(), []);
            let unit = fsl_forms::build_unit(&parsed).expect("the forms build");
            (parsed.source().reference().clone(), unit)
        })
        .collect();
    let inputs: PackageInput = package_input(std::iter::empty::<&[u8]>()).unwrap();
    let selections: Vec<_> = units
        .iter()
        .map(|(_, unit)| unit.selections().models.as_slice())
        .collect();
    let selected = fsl_semantics::model::intake::admit_units_with_cancel(
        &selections,
        &inputs,
        ModelNormalizationLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("empty selected models admit");
    let mut declarations = PackageDeclarations::assemble_units(
        package.to_owned(),
        units,
        selected,
        Vec::new(),
        fsl_semantics::check::AssemblyLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the package assembles");
    let _pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_semantics::check::test_support::ZeroRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

const FIRST_UNIT: &str = "object A { n: Integer; operation t(k: Int[1, 3]) modifies self.n; }\n\
    time A dense unit ms;\n\
    delay A::t ~ discrete { 2 ms: 1, 4 ms: 1 };\n\
    random A::t(k) ~ { 1: 1, 3: 3 };\n\
    workload WA on A { weight A::t = 1; }\n";

const SECOND_UNIT: &str = "unit s : Time = rational(1000, 1) * ms;\n\
    object B { n: Integer; operation u(k: Int[1, 3]) modifies self.n; }\n\
    time B dense unit s;\n\
    delay B::u ~ discrete { 1 s: 9, 3 s: 1 };\n\
    random B::u(k) ~ { 1: 3, 3: 1 };\n\
    workload WB on B { weight B::u = 1; }\n";

/// FR-740: each unit of a package emits its declarations on its own source
/// object types, whichever unit declares them.
#[trace("TC-3012", "FR-740-AC-5")]
#[test]
fn the_declarations_of_each_unit_of_a_package_are_emitted_on_its_own_types() {
    let package = check_units(&[("test:first", FIRST_UNIT), ("test:second", SECOND_UNIT)])
        .unwrap_or_else(|refusals| panic!("S3 admits the units: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    let delays = probabilistic(&wire, "delay_distribution");
    assert_eq!(delays.len(), 2);
    assert_eq!(probabilistic(&wire, "random_parameter").len(), 2);
    let workloads = probabilistic(&wire, "workload");
    assert_eq!(workloads.len(), 2);
    for (name, unit) in [("A", "ms"), ("B", "s")] {
        let object = source_node(&wire, name);
        let delay = delays
            .iter()
            .find(|node| first_reference(node) == object)
            .unwrap_or_else(|| panic!("a delay on {name}"));
        let mut times = Vec::new();
        quantities(&delay["body"], &mut times);
        assert_eq!(times.len(), 2, "{name}");
        assert!(
            times.iter().all(|(_, ty)| *ty == unit_node(&wire, unit)),
            "{name}: {times:?}"
        );
        assert!(
            workloads.iter().any(|node| first_reference(node) == object),
            "{name}"
        );
        assert!(
            probabilistic(&wire, "random_parameter")
                .iter()
                .any(|node| first_reference(node) == object),
            "{name}"
        );
    }
}

/// FR-185-AC-12: an S4 refusal of a declaration in a later unit of a package
/// carries that unit's source reference and the declaration's label span.
#[trace("TC-2062", "FR-185-AC-12")]
#[test]
fn a_refusal_in_a_later_unit_is_located_in_that_unit() {
    let second = "object B { n: Integer; operation x() modifies self.n; }\n\
        object C { m: Integer; operation x() modifies self.m; }\n\
        object D extends B, C { operation y() modifies self.n; }\n\
        workload W on D { weight D::y = 1; weight B::x = 1; weight C::x = 1; }\n";
    let package = check_units(&[("test:first", FIRST_UNIT), ("test:second", second)])
        .unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let refusal = emit_checked(&package, EmitLimits::new(10_000_000))
        .expect_err("a repeated operation name is refused");
    let EmitRefusal::ProbabilisticModelNotEmitted { region, .. } = &refusal else {
        panic!("a located probabilistic refusal: {refusal:?}");
    };
    assert_eq!(region.source().identity(), "test:second");
    let text = format!(
        "{}{second}",
        HEADER
            .lines()
            .take(3)
            .map(|l| format!("{l}\n"))
            .collect::<String>()
    );
    let label = text
        .get(region.start() as usize..region.end() as usize)
        .expect("the region lies in the second unit");
    assert!(label.contains('W'), "{label}");
}

const OWN_A: &str = "object A { n: Integer; operation t(k: Int[1, 3]) modifies self.n; }\n\
    time A dense unit ms;\n";
const OWN_B: &str = "unit s : Time = rational(1000, 1) * ms;\n\
    object B { n: Integer; operation u(k: Int[1, 3]) modifies self.n; }\n\
    time B dense unit s;\n";

/// The cause of the one refusal of `units`, which S3 must give.
fn refused_cause(units: &[(&str, &str)]) -> ProbabilisticCause {
    let refusals = match check_units(units) {
        Ok(_) => panic!("S3 admits a declaration on another unit's type"),
        Err(refusals) => refusals,
    };
    let [CheckRefusal {
        cause: CheckCause::Probabilistic(cause),
        ..
    }] = refusals.as_slice()
    else {
        panic!("one probabilistic refusal: {refusals:?}");
    };
    (**cause).clone()
}

/// FR-569: a source object type is visible only in the unit that declares it,
/// so a probabilistic declaration of one unit on a type of another is refused
/// as naming nothing, whichever unit declares the type.
#[trace("TC-1408", "FR-569-AC-2")]
#[test]
fn a_declaration_on_another_units_source_type_is_refused() {
    let on_a = [
        ("a reward", "reward cost on A::t = 1;\n"),
        ("a random parameter", "random A::t(k) ~ { 1: 1, 3: 3 };\n"),
        ("a delay", "delay A::t ~ discrete { 1 s: 1, 2 s: 1 };\n"),
    ];
    let on_b = [
        ("a reward", "reward cost on B::u = 1;\n"),
        ("a random parameter", "random B::u(k) ~ { 1: 1, 3: 3 };\n"),
        ("a delay", "delay B::u ~ discrete { 1 ms: 1, 2 ms: 1 };\n"),
    ];
    for (what, text) in on_a {
        let second = format!("{OWN_B}{text}");
        let cause = refused_cause(&[("test:first", OWN_A), ("test:second", &second)]);
        assert!(
            matches!(cause, ProbabilisticCause::UnresolvedOperation { .. }),
            "{what} of unit b on A: {cause:?}"
        );
    }
    for (what, text) in on_b {
        let first = format!("{OWN_A}{text}");
        let cause = refused_cause(&[("test:first", &first), ("test:second", OWN_B)]);
        assert!(
            matches!(cause, ProbabilisticCause::UnresolvedOperation { .. }),
            "{what} of unit a on B: {cause:?}"
        );
    }
    let second = format!("{OWN_B}workload W on A {{ weight A::t = 1; }}\n");
    let cause = refused_cause(&[("test:first", OWN_A), ("test:second", &second)]);
    assert!(
        matches!(cause, ProbabilisticCause::UnresolvedModel { .. }),
        "a workload of unit b on A: {cause:?}"
    );
    let first = format!("{OWN_A}workload W on B {{ weight B::u = 1; }}\n");
    let cause = refused_cause(&[("test:first", &first), ("test:second", OWN_B)]);
    assert!(
        matches!(cause, ProbabilisticCause::UnresolvedModel { .. }),
        "a workload of unit a on B: {cause:?}"
    );
}

/// FR-096-AC-1: each probabilistic node's source map region names the unit that
/// declares it, with that unit's offsets.
#[trace("TC-426", "FR-096-AC-1")]
#[test]
fn the_source_map_region_of_a_later_units_declaration_names_that_unit() {
    let package = check_units(&[("test:first", FIRST_UNIT), ("test:second", SECOND_UNIT)])
        .unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    assert_eq!(probabilistic(&wire, "delay_distribution").len(), 2);
    let text: String = HEADER
        .lines()
        .take(3)
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        + SECOND_UNIT;
    let entries: Vec<&Value> = wire["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .filter(|entry| {
            let path = &entry["node_id"]["path"];
            path[1] == "delay_distribution" && path[3] == "u"
        })
        .collect();
    assert!(!entries.is_empty(), "a source map entry for the delay");
    for region in entries
        .iter()
        .flat_map(|entry| entry["regions"].as_array().expect("regions"))
    {
        assert_eq!(region["source"]["identity"], "test:second", "{region}");
        let (start, end) = (
            region["start"].as_u64().expect("start") as usize,
            region["end"].as_u64().expect("end") as usize,
        );
        let label = text.get(start..end).expect("a region of the second unit");
        assert!(label.contains("delay"), "{label}");
    }
}

/// A package named like a later unit's identity (`acme/v/2`) keys its source
/// types under that name, whichever unit declares them.
#[trace("TC-3012", "FR-740-AC-5")]
#[test]
fn a_package_name_ending_in_a_number_keys_its_source_types_under_that_name() {
    let package = check_units_named(
        "acme/v/2",
        &[("test:first", FIRST_UNIT), ("test:second", SECOND_UNIT)],
    )
    .unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    for name in ["A", "B"] {
        assert_eq!(source_node(&wire, name)["package"], "acme/v/2", "{name}");
    }
    assert_eq!(probabilistic(&wire, "delay_distribution").len(), 2);
    assert_eq!(probabilistic(&wire, "workload").len(), 2);
}

const CTL: &str = "taskset Ctl { task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo; }\n";

/// FR-569: a `release` of one unit on another unit's source operation names
/// nothing and is refused, whichever unit declares the type; one on the
/// unit's own type is admitted.
#[trace("TC-1408", "FR-569-AC-2")]
#[test]
fn a_release_on_another_units_source_operation_is_refused() {
    let first = format!("{OWN_A}{CTL}");
    let release_on_b = format!("{first}release Ctl.a on B::u;\n");
    let release_on_a = format!("{OWN_B}release Ctl.a on A::t;\n");
    let own = format!("{first}release Ctl.a on A::t;\n");
    let admitted = check_units(&[("test:first", &own), ("test:second", OWN_B)]);
    assert!(
        admitted.is_ok(),
        "an own-unit release: {:?}",
        admitted.err()
    );
    for units in [
        [
            ("test:first", first.as_str()),
            ("test:second", release_on_a.as_str()),
        ],
        [
            ("test:first", release_on_b.as_str()),
            ("test:second", OWN_B),
        ],
    ] {
        let refusals = check_units(&units).expect_err("a cross-unit release is refused");
        assert!(
            refusals.iter().any(
                |refusal| format!("{:?}", refusal.cause).contains("UnresolvedReleaseOperation")
            ),
            "{refusals:?}"
        );
    }
}

/// FSpec FR-322: the source map region of each node of a later unit's `time`
/// declaration names that unit, with offsets inside its text.
#[trace("TC-426", "FR-096-AC-1")]
#[test]
fn the_source_map_regions_of_a_later_units_time_nodes_name_that_unit() {
    let package = check_units(&[("test:first", FIRST_UNIT), ("test:second", SECOND_UNIT)])
        .unwrap_or_else(|refusals| panic!("S3 admits: {refusals:?}"));
    let (wire, _) = emitted_and_joined(&package);
    let text: String = HEADER
        .lines()
        .take(3)
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        + SECOND_UNIT;
    let mut checked = 0;
    for entry in wire["source_map"].as_array().expect("source map") {
        let path = &entry["node_id"]["path"];
        if !(path[0] == "B" && path[1] == "Timed" || path[0] == "s") {
            continue;
        }
        for region in entry["regions"].as_array().expect("regions") {
            assert_eq!(region["source"]["identity"], "test:second", "{entry}");
            let (start, end) = (
                region["start"].as_u64().expect("start") as usize,
                region["end"].as_u64().expect("end") as usize,
            );
            let label = text.get(start..end).expect("a region of the second unit");
            assert!(label == "s" || label.starts_with("time B"), "{label}");
            checked += 1;
        }
    }
    assert!(checked >= 2, "the unit's time nodes were found");
}
