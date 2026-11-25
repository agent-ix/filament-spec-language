// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-398's admitted dimension-equivalence projection through the public
//! replay facade.

use filament_exact::{Integer, Meter, Quantity, Rational, ScalarLimits};
use filament_semantic_value::quantity::{convert_quantity, ConvertedValue, QuantityTarget};
use fsl_foundation::SourceIdentity;
use fsl_replay::{
    compile_admitted, AdmittedLookupRefusal, CompileUnit, DependencyInput, ReplayLimits,
    StageLimits,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SOURCE_PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.model.complete/v1\";\n\
dimension Time;\n\
dimension Period;\n\
unit s : Time = rational(1, 1);\n\
unit sec : Period = rational(1, 1);\n";

fn compile(source: &str) -> fsl_replay::AdmittedCompilation {
    compile_admitted(
        CompileUnit {
            source: SourceIdentity::new("agent-ix", "qsl-2094-fr398-equivalence"),
            package_name: "qsl-2094-fr398".to_owned(),
            path: "units.native",
            bytes: source.as_bytes(),
        },
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("the dimension-equivalence unit is admitted")
}

#[trace("TC-1092", "FR-398-AC-6", "FR-398-AC-7")]
#[test]
fn admitted_dimension_equivalence_preserves_exact_root_ratio_and_wrong_kind() {
    let source = format!("{SOURCE_PREFIX}dimension Period ~ Time by 1 sec = 1 s;\n");
    let admitted = compile(&source);
    let context = admitted.context();
    let equivalences = context.dimension_equivalences().collect::<Vec<_>>();
    let [equivalence] = equivalences.as_slice() else {
        panic!("the context lists the one declared equivalence");
    };
    let time = context
        .dimensions()
        .find(|dimension| {
            dimension
                .qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "Time")
        })
        .expect("the admitted Time dimension");
    let period = context
        .dimensions()
        .find(|dimension| {
            dimension
                .qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "Period")
        })
        .expect("the admitted Period dimension");
    assert_eq!(
        equivalence.dimensions(),
        &[period.node().clone(), time.node().clone()]
    );
    assert_eq!(equivalence.root_scale().to_string(), "1/1");
    let package: Value =
        serde_json::from_slice(admitted.package().bytes()).expect("the emitted package is JSON");
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes");
    let equivalence_node = nodes
        .iter()
        .find(|node| node["semantic_form"] == "dimension_equivalence")
        .unwrap_or_else(|| panic!("emitted equivalence node is absent: {package}"));
    let emitted_node_key = context
        .wire_graph()
        .key_of_value(&equivalence_node["node_id"])
        .expect("the emitted equivalence id resolves structurally");
    assert_eq!(&emitted_node_key, equivalence.node());
    let emitted_type_key = context
        .wire_graph()
        .key_of_value(&equivalence_node["semantic_type"])
        .expect("the self type resolves structurally");
    assert_eq!(&emitted_type_key, equivalence.node());

    let seconds = context
        .units()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "s")
        })
        .expect("the admitted s unit");
    let seconds_as_period = context
        .units()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "sec")
        })
        .expect("the admitted sec unit");
    assert!(seconds.converts_to(seconds_as_period));
    assert!(seconds_as_period.converts_to(seconds));
    let quantity = Quantity::new(Rational::from_integer(Integer::from(3_i64)), seconds.id());
    let resolved = context
        .unit_table()
        .resolve(&quantity)
        .expect("the context unit table resolves its admitted s id");
    let converted = convert_quantity(
        resolved,
        seconds_as_period.quantity_unit(),
        &QuantityTarget::Exact,
        &mut conversion_meter(),
    )
    .expect("equivalent dimensions permit quantity conversion");
    let filament_exact::Outcome::Completed(converted) = converted else {
        panic!("the exact conversion completes: {converted:?}");
    };
    assert_eq!(
        converted.value(),
        &ConvertedValue::Exact(Rational::from_integer(Integer::from(3_i64)))
    );

    assert_eq!(
        context.unit_declaration(equivalence.node()),
        Err(AdmittedLookupRefusal::WrongKind {
            declaration: equivalence.node().clone(),
        })
    );
    assert_eq!(
        context.dimension_declaration(equivalence.node()),
        Err(AdmittedLookupRefusal::WrongKind {
            declaration: equivalence.node().clone(),
        })
    );
    assert!(!context
        .dimension_equivalences()
        .any(|entry| entry.node() == time.node()));
}

/// The one `dimension_equivalence` node of the emitted package.
fn emitted_equivalence(compilation: &fsl_replay::AdmittedCompilation) -> Value {
    let package: Value =
        serde_json::from_slice(compilation.package().bytes()).expect("the emitted package is JSON");
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes");
    let equivalences: Vec<_> = nodes
        .iter()
        .filter(|node| node["semantic_form"] == "dimension_equivalence")
        .collect();
    let [equivalence] = equivalences.as_slice() else {
        panic!("one emitted equivalence node: {package}");
    };
    (*equivalence).clone()
}

#[trace("TC-1092", "FR-398-AC-6")]
#[test]
fn dimension_equivalence_ratio_changes_body_not_identity_and_absence_refuses_conversion() {
    let equal = format!("{SOURCE_PREFIX}dimension Period ~ Time by 1 sec = 1 s;\n");
    let scaled = format!("{SOURCE_PREFIX}dimension Period ~ Time by 1 sec = 2 s;\n");
    let equal_context = compile(&equal);
    let scaled_context = compile(&scaled);
    let equal_equivalences = equal_context
        .context()
        .dimension_equivalences()
        .collect::<Vec<_>>();
    let [equal_equivalence] = equal_equivalences.as_slice() else {
        panic!("one admitted equivalence");
    };
    let scaled_equivalences = scaled_context
        .context()
        .dimension_equivalences()
        .collect::<Vec<_>>();
    let [scaled_equivalence] = scaled_equivalences.as_slice() else {
        panic!("one admitted equivalence");
    };
    assert_eq!(equal_equivalence.root_scale().to_string(), "1/1");
    assert_eq!(scaled_equivalence.root_scale().to_string(), "2/1");
    assert_eq!(equal_equivalence.node(), scaled_equivalence.node());

    // Both packages carry one package name and differ only in the ratio: the
    // emitted node has one id, and its body and the package bytes differ.
    let equal_node = emitted_equivalence(&equal_context);
    let scaled_node = emitted_equivalence(&scaled_context);
    assert_eq!(equal_node["node_id"], scaled_node["node_id"]);
    assert_eq!(
        equal_node["body"]["root_scale"],
        json!({"numerator": "1", "denominator": "1"})
    );
    assert_eq!(
        scaled_node["body"]["root_scale"],
        json!({"numerator": "2", "denominator": "1"})
    );
    assert_ne!(equal_node["body"], scaled_node["body"]);
    assert_ne!(
        equal_context.package().bytes(),
        scaled_context.package().bytes()
    );

    let scaled_context = scaled_context.context();
    let scaled_seconds = scaled_context
        .units()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "s")
        })
        .expect("the admitted s unit");
    let scaled_period = scaled_context
        .units()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "sec")
        })
        .expect("the admitted sec unit");
    let quantity = Quantity::new(
        Rational::from_integer(Integer::from(3_i64)),
        scaled_seconds.id(),
    );
    let resolved = scaled_context
        .unit_table()
        .resolve(&quantity)
        .expect("s resolves in the scaled unit table");
    let converted = convert_quantity(
        resolved,
        scaled_period.quantity_unit(),
        &QuantityTarget::Exact,
        &mut conversion_meter(),
    )
    .expect("the declared equivalence permits exact conversion");
    let filament_exact::Outcome::Completed(converted) = converted else {
        panic!("the exact conversion completes: {converted:?}");
    };
    assert_eq!(
        converted.value(),
        &ConvertedValue::Exact(
            Rational::new(Integer::from(3_i64), Integer::from(2_i64))
                .expect("2 is a nonzero denominator")
        )
    );

    let without = compile(SOURCE_PREFIX);
    let no_equivalence_context = without.context();
    let no_units = no_equivalence_context.units().collect::<Vec<_>>();
    let second = no_units
        .iter()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "s")
        })
        .expect("s exists without the relation");
    let first = no_units
        .iter()
        .find(|unit| {
            unit.qualified_name()
                .last()
                .is_some_and(|name| name.as_str() == "sec")
        })
        .expect("sec exists without the relation");
    assert!(!second.converts_to(first));
    let quantity = Quantity::new(Rational::from_integer(Integer::from(3_i64)), second.id());
    let resolved = no_equivalence_context
        .unit_table()
        .resolve(&quantity)
        .expect("s resolves in the unit table");
    assert!(convert_quantity(
        resolved,
        first.quantity_unit(),
        &QuantityTarget::Exact,
        &mut conversion_meter(),
    )
    .is_err());
}

fn conversion_meter() -> Meter {
    Meter::new(ScalarLimits {
        integer_bits: 4096,
        decimal_digits: 4096,
        scale_expansion: 4096,
        text_input_bytes: 4096,
        text_scalars: 4096,
        normalized_scalars: 4096,
        unit_edges: 4096,
        value_occurrences: 4096,
        work_units: 4096,
        result_units: 4096,
    })
}
