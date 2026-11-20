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
use fsl_package::{emit_checked, CheckedPackage, EmitLimits, ReadIdentityRows};
use fsl_semantics::{
    check::{CheckCause, CheckRefusal, PackageDeclarations, ProbabilisticCause},
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

const TIMED: &str = "object O { d: Clock; operation step() modifies self.d; }\n\
    object Other { n: Integer; operation poke() modifies self.n; }\n";

#[trace("TC-3012", "FR-740-AC-5")]
#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]
#[test]
#[ignore = "QSL-2463: S4 emission of a source-owned declaration waits on the contract-ir #1082 pin (PR 3157 carries the pin bump) and the identity-row join (QSL-2645)"]
fn a_delay_on_each_source_object_of_a_timed_source_model_is_emitted() {
    let package = check(&format!(
        "{TIMED}time O dense unit ms;\n\
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
    let body = discrete["body"].to_string();
    assert!(body.contains("\"1\"") && body.contains("\"3\""), "{body}");
}

#[trace("TC-3012", "FR-740-AC-5")]
#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]
#[test]
#[ignore = "QSL-2699: S3 refuses a delay on an untimed source model as invalid_timed_model, where FR-740-AC-5 states invalid_probabilistic_model/delay-on-untimed-model"]
fn a_delay_on_an_untimed_source_model_is_refused_at_its_span() {
    let Err(refusals) = check(&format!("{TIMED}delay O::step ~ uniform;\n")) else {
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
#[ignore = "QSL-2463: S4 emission of a source-owned declaration waits on the contract-ir #1082 pin (PR 3157 carries the pin bump) and the identity-row join (QSL-2645)"]
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
#[ignore = "QSL-2463: the strict read refuses the source clock place with MalformedClockPlace until the contract-ir #1082 pin lands (PR 3157)"]
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
#[trace("TC-3012", "FR-740-AC-6")]
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

#[trace("TC-3012", "FR-740-AC-6")]
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
#[trace("TC-3012", "FR-740-AC-6")]
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
#[trace("TC-3012", "FR-740-AC-6")]
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
