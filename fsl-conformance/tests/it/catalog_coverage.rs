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
    forms: &'static [&'static str],
}

macro_rules! operators {
    ($name:literal, $family:literal, [$($operation:literal),+ $(,)?]) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../cases/type-operators/", $name, "-accepted.native")),
            operations: &[$(concat!("filament.op.", $family, ".", $operation)),+],
            forms: &["application"],
        }
    };
}

const CASES: &[Case] = &[
    operators!(
        "boolean-connectives",
        "boolean",
        ["not", "and", "or", "implies"]
    ),
    operators!("boolean-equality", "boolean", ["eq", "ne"]),
    operators!("integer-arithmetic", "integer", ["add", "sub", "mul"]),
    operators!("integer-negation", "integer", ["negate"]),
    operators!("integer-equality", "integer", ["eq", "ne"]),
    operators!("integer-ordering", "integer", ["lt", "le", "gt", "ge"]),
    operators!("integer-division", "integer", ["div", "rem"]),
    operators!("integer-modulo", "integer", ["mod"]),
    operators!("rational-arithmetic", "rational", ["add", "sub", "mul"]),
    operators!("rational-negation", "rational", ["negate"]),
    operators!("rational-equality", "rational", ["eq", "ne"]),
    operators!("rational-ordering", "rational", ["lt", "le", "gt", "ge"]),
    operators!("rational-division", "rational", ["div"]),
    operators!("decimal-arithmetic", "decimal", ["add", "sub", "mul"]),
    operators!("decimal-negation", "decimal", ["negate"]),
    operators!("decimal-equality", "decimal", ["eq", "ne"]),
    operators!("decimal-ordering", "decimal", ["lt", "le", "gt", "ge"]),
    operators!("decimal-division", "decimal", ["div"]),
    operators!("text-equality", "text", ["eq", "ne"]),
    operators!("text-ordering", "text", ["lt", "le", "gt", "ge"]),
    operators!("text-size", "text", ["size"]),
    operators!("collection-size", "collection", ["size"]),
    operators!("collection-contains", "collection", ["contains"]),
    Case {
        name: "option-presence",
        source: include_str!("../../cases/type-operators/option-presence-accepted.native"),
        operations: &[
            "filament.op.option.present",
            "filament.op.option.value",
            "filament.op.control.if",
        ],
        forms: &["application", "option"],
    },
    Case {
        name: "sequence-index",
        source: include_str!("../../cases/index-accepted.native"),
        operations: &["filament.op.collection.index"],
        forms: &["application", "sequence"],
    },
    Case {
        name: "tuple-position",
        source: include_str!("../../cases/tuple-position-accepted.native"),
        operations: &["filament.op.tuple.project"],
        forms: &["application", "tuple"],
    },
];

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

fn assert_producers(case: &Case, catalog: &BTreeSet<String>, package: &Value) {
    let nodes = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted graph nodes");
    let mut operations = BTreeSet::new();
    for node in nodes {
        if node["semantic_form"] == "application" {
            operation_identities(&node["body"], &mut operations);
        }
    }
    assert!(
        !operations.is_empty(),
        "{} emits application operations",
        case.name
    );
    for identity in &operations {
        assert!(
            catalog.contains(identity),
            "{} emits unpublished operation {identity}",
            case.name
        );
    }
    for expected in case.operations {
        assert!(
            operations.contains(*expected),
            "{} must produce {expected}; got {operations:?}",
            case.name
        );
    }
    for expected in case.forms {
        assert!(
            nodes.iter().any(|node| node["semantic_form"] == *expected),
            "{} must produce node form {expected}",
            case.name
        );
    }
}

#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn scalar_and_structural_operations_are_produced_from_source_and_admitted() {
    let catalog = catalog_identities();
    let mut failures = Vec::new();
    for case in CASES {
        let result = std::panic::catch_unwind(|| {
            let package = compile_and_read(case);
            assert_producers(case, &catalog, &package);
        });
        if result.is_err() {
            failures.push(case.name);
        }
    }
    assert!(
        failures.is_empty(),
        "source producer cases failed: {failures:?}"
    );
}
