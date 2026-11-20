// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source producers of published checked-operation identities and node forms.

use std::collections::BTreeSet;

use filament_exact::Cancel;
use filament_verification_contracts::operation_catalog::CHECKED_OPERATION_CATALOG_V1;
use fsl_foundation::SourceIdentity;
use fsl_package::{EmitLimits, V2ReadLimits};
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::PackageInput;
use ix_trace_rs::trace;
use serde_json::Value;

struct Case {
    name: &'static str,
    source: &'static str,
    operations: &'static [&'static str],
    forms: &'static [(&'static str, &'static str)],
}

macro_rules! operators {
    ($name:literal, $family:literal, [$($operation:literal),+ $(,)?]) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../cases/type-operators/", $name, "-accepted.native")),
            operations: &[$(concat!("filament.op.", $family, ".", $operation)),+],
            forms: &[],
        }
    };
}

macro_rules! structural_equality {
    ($name:literal, $declarations:literal, $ty:literal) => {
        Case {
            name: $name,
            source: concat!(
                "language \"ix:native\" edition \"1-draft\";\n",
                "profile v = \"filament.value.complete/v1\";\n",
                $declarations,
                "\n",
                "function equal using v(a: ",
                $ty,
                ",b: ",
                $ty,
                "): Boolean pure { a=b }\n",
            ),
            operations: &["filament.op.structural.eq"],
            forms: &[],
        }
    };
}

macro_rules! source_cases {
    ($($test:ident => $case:expr),+ $(,)?) => {
        const CASES: &[Case] = &[$($case),+];
        $(
            #[test]
            #[trace("FR-350", "FSpec-FR-322")]
            fn $test() {
                let case = $case;
                let package = compile_and_read(&case);
                assert_producers(&case, &catalog_identities(), &package);
            }
        )+
    };
}

source_cases! {
    boolean_connectives => operators!("boolean-connectives", "boolean", ["not", "and", "or", "implies"]),
    boolean_equality => operators!("boolean-equality", "boolean", ["eq", "ne"]),
    integer_arithmetic => operators!("integer-arithmetic", "integer", ["add", "sub", "mul"]),
    integer_negation => operators!("integer-negation", "integer", ["negate"]),
    integer_equality => operators!("integer-equality", "integer", ["eq", "ne"]),
    integer_ordering => operators!("integer-ordering", "integer", ["lt", "ge"]),
    integer_division => operators!("integer-division", "integer", ["div", "rem"]),
    integer_modulo => operators!("integer-modulo", "integer", ["mod"]),
    rational_arithmetic => operators!("rational-arithmetic", "rational", ["add", "sub", "mul"]),
    rational_negation => operators!("rational-negation", "rational", ["negate"]),
    rational_equality => operators!("rational-equality", "rational", ["eq", "ne"]),
    rational_ordering => operators!("rational-ordering", "rational", ["lt", "ge"]),
    rational_division => operators!("rational-division", "rational", ["div"]),
    decimal_arithmetic => operators!("decimal-arithmetic", "decimal", ["add", "sub", "mul"]),
    decimal_negation => operators!("decimal-negation", "decimal", ["negate"]),
    decimal_equality => operators!("decimal-equality", "decimal", ["eq"]),
    decimal_ordering => operators!("decimal-ordering", "decimal", ["lt", "ge"]),
    decimal_division => operators!("decimal-division", "decimal", ["div"]),
    text_equality => operators!("text-equality", "text", ["eq"]),
    text_ordering => operators!("text-ordering", "text", ["lt"]),
    text_size => operators!("text-size", "text", ["size"]),
    collection_size => operators!("collection-size", "collection", ["size"]),
    collection_contains => operators!("collection-contains", "collection", ["contains"]),
    quantity_arithmetic => operators!("quantity-arithmetic", "quantity", ["add", "sub", "mul"]),
    quantity_negation => operators!("quantity-negation", "quantity", ["negate"]),
    quantity_division => operators!("quantity-division", "quantity", ["div"]),
    quantity_scaling => operators!("quantity-scaling", "quantity", ["scale"]),
    quantity_power => operators!("quantity-power", "quantity", ["pow"]),
    quantity_equality => operators!("quantity-equality", "quantity", ["eq"]),
    quantity_ordering => operators!("quantity-ordering", "quantity", ["lt"]),
    enum_equality => operators!("enum-equality", "enum", ["eq", "ne"]),
    enum_ordering => operators!("enum-ordering", "enum", ["lt"]),
    integer_record_equality => structural_equality!("integer-record-equality", "record R { n: Integer; }", "R"),
    integer_tuple_equality => structural_equality!("integer-tuple-equality", "tuple T(Integer,Integer);", "T"),
    integer_union_equality => structural_equality!("integer-union-equality", "union U { A(Integer),B }", "U"),
    float_record_equality => structural_equality!("float-record-equality", "type F = Float64[nearest-even]; record R { n: F; }", "R"),
    float_tuple_equality => structural_equality!("float-tuple-equality", "type F = Float64[nearest-even]; tuple T(F,F);", "T"),
    float_union_equality => structural_equality!("float-union-equality", "type F = Float64[nearest-even]; union U { A(F),B }", "U"),
    float_option_equality => structural_equality!("float-option-equality", "type F = Float64[nearest-even];", "Option<F>"),
    float_sequence_equality => structural_equality!("float-sequence-equality", "type F = Float64[nearest-even];", "Sequence<F>[0,3]"),
    option_presence => Case {
        name: "option-presence",
        source: include_str!("../../cases/type-operators/option-presence-accepted.native"),
        operations: &[
            "filament.op.option.present",
            "filament.op.option.value",
            "filament.op.control.if",
        ],
        forms: &[("composite_type", "option"), ("expression", "conditional"), ("expression", "presence_read"), ("expression", "value_read")],
    },
    tuple_position => Case {
        name: "tuple-position",
        source: include_str!("../../cases/tuple-position-accepted.native"),
        operations: &["filament.op.tuple.project"],
        forms: &[("composite_type", "tuple"), ("expression", "query")],
    },
    float_arithmetic => Case {
        name: "float-arithmetic",
        source: include_str!("../../cases/type-operators/float-arithmetic-accepted.native"),
        operations: &["filament.op.ieee.float32.add", "filament.op.ieee.float32.sub", "filament.op.ieee.float32.mul", "filament.op.ieee.float64.add", "filament.op.ieee.float64.sub", "filament.op.ieee.float64.mul"],
        forms: &[],
    },
    float_negation => Case {
        name: "float-negation",
        source: include_str!("../../cases/type-operators/float-negation-accepted.native"),
        operations: &["filament.op.ieee.float32.negate", "filament.op.ieee.float64.negate"],
        forms: &[],
    },
    float_division => Case {
        name: "float-division",
        source: include_str!("../../cases/type-operators/float-division-accepted.native"),
        operations: &["filament.op.ieee.float32.div", "filament.op.ieee.float64.div"],
        forms: &[],
    },
    structural_calls => Case {
        name: "structural-calls",
        source: include_str!("../../cases/catalog-source-structural.native"),
        operations: &["filament.op.record.project", "filament.op.function.call", "filament.op.control.let", "filament.op.integer.add"],
        forms: &[("composite_type", "record"), ("expression", "call"), ("expression", "let")],
    },
}

fn catalog_identities() -> BTreeSet<String> {
    let catalog: Value = serde_json::from_str(CHECKED_OPERATION_CATALOG_V1)
        .expect("the dependency publishes a JSON operation catalog");
    let rows = catalog["operations"]
        .as_array()
        .expect("catalog operation rows");
    assert!(!rows.is_empty(), "the published catalog is nonempty");
    rows.iter()
        .map(|row| {
            row["identity"]
                .as_str()
                .expect("operation identity")
                .to_owned()
        })
        .collect()
}

fn compile_and_read(case: &Case) -> Value {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("catalog-source", case.name);
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: case.name,
            bytes: case.source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap_or_else(|error| panic!("{} parses: {error:?}", case.name))
    .into_value();
    let inputs = PackageInput::default();
    let models = spine::select(&parsed, &inputs, limits.model, &cancel)
        .unwrap_or_else(|error| panic!("{} selects its models: {error:?}", case.name))
        .into_value();
    let checked = spine::check(
        &parsed,
        "catalog-source".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .unwrap_or_else(|error| {
        panic!(
            "{} checks through the public source path: {error:?}",
            case.name
        )
    })
    .into_value();
    let read = fsl_package::read_evaluation_package(
        checked.package(),
        LibraryName::new("catalog-source").expect("package name"),
        &inputs,
        V2ReadLimits::bounded(16_777_216),
        EmitLimits::new(16_777_216),
    )
    .unwrap_or_else(|error| panic!("{} emitted package is admitted: {error:?}", case.name));
    assert!(
        read.emission().omitted().is_empty(),
        "{} has no omitted source rows",
        case.name
    );
    serde_json::from_slice(read.emission().package().bytes()).expect("admitted package JSON")
}

fn operation_identities(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(identity) = fields.get("identity").and_then(Value::as_str) {
                if identity.starts_with("filament.op.") {
                    found.insert(identity.to_owned());
                }
            }
            for value in fields.values() {
                operation_identities(value, found);
            }
        }
        Value::Array(values) => {
            for value in values {
                operation_identities(value, found);
            }
        }
        _ => {}
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ProducerMismatch {
    UnpublishedOperation {
        case: &'static str,
        identity: String,
    },
    MissingOperation {
        case: &'static str,
        identity: String,
    },
    MissingForm {
        case: &'static str,
        form: String,
    },
}

fn check_producers(
    case: &Case,
    catalog: &BTreeSet<String>,
    package: &Value,
) -> Result<(), ProducerMismatch> {
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted graph nodes");
    let mut operations = BTreeSet::new();
    for node in nodes {
        if node["node_tag"] == "expression" && node["body"]["term"] == "application" {
            operation_identities(&node["body"], &mut operations);
        }
    }
    assert!(
        !operations.is_empty(),
        "{} emits application operations",
        case.name
    );
    for identity in &operations {
        if !catalog.contains(identity) {
            return Err(ProducerMismatch::UnpublishedOperation {
                case: case.name,
                identity: identity.clone(),
            });
        }
    }
    for expected in case.operations {
        if !operations.contains(*expected) {
            return Err(ProducerMismatch::MissingOperation {
                case: case.name,
                identity: (*expected).to_owned(),
            });
        }
    }
    for (tag, form) in case.forms {
        if !nodes
            .iter()
            .any(|node| node["node_tag"] == *tag && node["semantic_form"] == *form)
        {
            return Err(ProducerMismatch::MissingForm {
                case: case.name,
                form: format!("{tag}/{form}"),
            });
        }
    }
    Ok(())
}

fn assert_producers(case: &Case, catalog: &BTreeSet<String>, package: &Value) {
    check_producers(case, catalog, package)
        .unwrap_or_else(|mismatch| panic!("source producer mismatch: {mismatch:?}"));
}

fn unrepresented_operations(catalog: &BTreeSet<String>) -> BTreeSet<String> {
    let represented: BTreeSet<_> = CASES
        .iter()
        .flat_map(|case| case.operations.iter().copied())
        .collect();
    catalog
        .iter()
        .filter(|identity| !represented.contains(identity.as_str()))
        .cloned()
        .collect()
}

/// This control checks the runner's catalog dispatch, not language coverage.
#[test]
#[trace("FR-350")]
fn an_added_catalog_entry_is_reported_by_its_identity() {
    let mut catalog = catalog_identities();
    let original_gaps = unrepresented_operations(&catalog);
    let added = "filament.op.conformance.nonexistent".to_owned();
    assert!(catalog.insert(added.clone()), "the negative entry is new");
    let new_gaps = unrepresented_operations(&catalog);
    assert_eq!(
        new_gaps
            .difference(&original_gaps)
            .cloned()
            .collect::<Vec<_>>(),
        vec![added]
    );
}

/// An altered expected operation is rejected after compiling real source.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn a_wrong_expected_operation_names_the_source_case_and_identity() {
    let case = Case {
        name: "altered-boolean-expectation",
        source: CASES[0].source,
        operations: &["filament.op.integer.add"],
        forms: &[],
    };
    let package = compile_and_read(&case);
    assert_eq!(
        check_producers(&case, &catalog_identities(), &package),
        Err(ProducerMismatch::MissingOperation {
            case: case.name,
            identity: "filament.op.integer.add".to_owned(),
        })
    );
}

/// Full source qualification remains strict while additional adapters are authored.
#[test]
#[ignore = "QSL-2051: source adapters for the remaining catalog operations and node families are not complete"]
fn every_catalog_operation_has_a_source_and_is_admitted() {
    let catalog = catalog_identities();
    for case in CASES {
        let package = compile_and_read(case);
        assert_producers(case, &catalog, &package);
    }
    assert!(
        unrepresented_operations(&catalog).is_empty(),
        "catalog operations without source adapters: {:?}",
        unrepresented_operations(&catalog)
    );
}

#[test]
#[ignore = "IR-1491: emitted sequence indexing is refused OperationMemberMismatch at body/operation/member"]
fn sequence_index_is_emitted_and_admitted() {
    let case = Case {
        name: "sequence-index",
        source: include_str!("../../cases/index-accepted.native"),
        operations: &["filament.op.collection.index"],
        forms: &[("composite_type", "sequence"), ("expression", "query")],
    };
    let package = compile_and_read(&case);
    assert_producers(&case, &catalog_identities(), &package);
}
