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

macro_rules! sourced {
    ($name:literal, $operations:expr, $forms:expr) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../cases/catalog-source/", $name, ".native")),
            operations: $operations,
            forms: $forms,
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
    enum_equality => operators!("enum-equality", "enum", ["eq"]),
    enum_ordering => operators!("enum-ordering", "enum", ["lt"]),
    integer_record_equality => structural_equality!("integer-record-equality", "record R { n: Integer; }", "R"),
    integer_tuple_equality => structural_equality!("integer-tuple-equality", "tuple T(Integer,Integer);", "T"),
    integer_union_equality => structural_equality!("integer-union-equality", "union U { A(Integer),B }", "U"),
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
    text_comparison => sourced!("text-comparison", &["filament.op.text.ne", "filament.op.text.le", "filament.op.text.gt", "filament.op.text.ge"], &[]),
    decimal_comparison => sourced!("decimal-comparison", &["filament.op.decimal.ne", "filament.op.decimal.le", "filament.op.decimal.gt"], &[]),
    enum_comparison => sourced!("enum-comparison", &["filament.op.enum.ne", "filament.op.enum.le", "filament.op.enum.gt", "filament.op.enum.ge"], &[]),
    quantity_comparison => sourced!("quantity-comparison", &["filament.op.quantity.ne", "filament.op.quantity.le", "filament.op.quantity.gt", "filament.op.quantity.ge"], &[]),
    native_inequality => sourced!("native-inequality", &["filament.op.uuid.ne", "filament.op.timestamp.ne"], &[]),
    string_inequality => sourced!("string-inequality", &["filament.op.string.ne"], &[]),
    bytes_inequality => sourced!("bytes-inequality", &["filament.op.bytes.ne"], &[]),
    json_object_inequality => sourced!("json-object-inequality", &["filament.op.json_object.ne"], &[]),
    float_sqrt => sourced!("float-sqrt", &["filament.op.ieee.float64.sqrt", "filament.op.ieee.float32.sqrt"], &[]),
    float_fma => sourced!("float-fma", &["filament.op.ieee.float64.fma", "filament.op.ieee.float32.fma"], &[]),
    float_bit_identical => sourced!("float-bit-identical", &["filament.op.ieee.bit_identical"], &[]),
    float_conversion => sourced!("float-conversion", &["filament.op.ieee.to_float32", "filament.op.ieee.to_float64", "filament.op.ieee.from_exact"], &[]),
    ordered_set_drop_first => sourced!("ordered-set-drop-first", &["filament.op.collection.ordered_set.drop_first"], &[]),
    collection_map_flatten => sourced!("collection-map-flatten", &["filament.op.collection.map", "filament.op.collection.flatten"], &[]),
    float_sum => sourced!("float-sum", &["filament.op.collection.sum.float64", "filament.op.collection.sum.float32"], &[]),
    quantity_sum => sourced!("quantity-sum", &["filament.op.collection.sum.quantity"], &[]),
    union_case => sourced!("union-case", &["filament.op.control.case"], &[("composite_type", "union"), ("expression", "case")]),
    decimal_scale_reduction => sourced!("decimal-scale-reduction", &["filament.op.numeric.convert_rounding"], &[]),
    text_conversion => sourced!("text-conversion", &["filament.op.text.to_integer", "filament.op.text.to_decimal", "filament.op.text.to_rational", "filament.op.text.to_float64", "filament.op.text.to_float32"], &[]),
    string_conversion => sourced!("string-conversion", &["filament.op.string.to_integer", "filament.op.string.to_decimal", "filament.op.string.to_rational", "filament.op.string.to_float64", "filament.op.string.to_float32"], &[]),
    quantity_conversion => sourced!("quantity-conversion", &["filament.op.quantity.convert", "filament.op.text.to_quantity", "filament.op.string.to_quantity"], &[]),
    quantity_scale_division => sourced!("quantity-scale-division", &["filament.op.quantity.scale_div"], &[]),
    copied_braking => sourced!("braking", &["filament.op.rational.gt", "filament.op.rational.le", "filament.op.temporal.always", "filament.op.temporal.clause", "filament.op.temporal.holds"], &[("hybrid", "hybrid_mode"), ("hybrid", "hybrid_model"), ("hybrid", "hybrid_transition"), ("temporal", "formula"), ("temporal", "temporal_clause")]),
    copied_attempt_result => sourced!("attempt-result", &["filament.op.protocol.control"], &[("protocol", "control"), ("protocol", "obligation"), ("protocol", "protocol_clause"), ("protocol", "role"), ("state", "frame"), ("state", "operation_anchor")]),
    copied_r_exists_empty => sourced!("r-exists-empty", &["filament.op.collection.exists", "filament.op.collection.sequence", "filament.op.integer.gt"], &[("expression", "quantify")]),
    copied_native_equality => sourced!("native-equality", &["filament.op.bytes.eq", "filament.op.json_object.eq", "filament.op.timestamp.eq", "filament.op.uuid.eq"], &[]),
    copied_source_object_workload => sourced!("source-object-workload", &[], &[("probabilistic", "random_parameter"), ("probabilistic", "reward"), ("probabilistic", "workload")]),
    copied_operators => sourced!("operators", &["filament.op.temporal.eventually", "filament.op.temporal.not", "filament.op.temporal.until"], &[]),
    copied_quant_range_fact => sourced!("quant-range-fact", &["filament.op.collection.forall", "filament.op.integer.le"], &[]),
    copied_r_bag_eq => sourced!("r-bag-eq", &["filament.op.collection.bag", "filament.op.structural.ne"], &[]),
    copied_float_ordering => sourced!("float-ordering", &["filament.op.ieee.numeric_equal", "filament.op.ieee.total_order"], &[]),
    copied_exchange => sourced!("exchange", &[], &[("protocol", "channel"), ("protocol", "queue")]),
    copied_some => sourced!("some", &[], &[("value", "option_value")]),
    copied_coll_set_ok => sourced!("coll-set-ok", &["filament.op.collection.set"], &[]),
    copied_coll_oset_ok => sourced!("coll-oset-ok", &["filament.op.collection.ordered_set"], &[]),
    copied_coll_removeone => sourced!("coll-removeone", &["filament.op.collection.remove_one"], &[]),
    copied_fn_recursion_ok => sourced!("fn-recursion-ok", &[], &[("function", "recursive_function")]),
    copied_coll_dropfirst => sourced!("coll-dropfirst", &["filament.op.collection.sequence.drop_first"], &[]),
    copied_tup_ok => sourced!("tup-ok", &[], &[("value", "tuple_value")]),
    copied_coll_filter_ok => sourced!("coll-filter-ok", &["filament.op.collection.filter"], &[]),
    copied_rec_ok => sourced!("rec-ok", &[], &[("value", "record_value")]),
    copied_conv_coll_ok => sourced!("conv-coll-ok", &["filament.op.collection.convert"], &[]),
    copied_fn_predicate_ok => sourced!("fn-predicate-ok", &[], &[("function", "predicate")]),
    copied_agg_sum_named => sourced!("agg-sum-named", &["filament.op.collection.sum.integer"], &[]),
    copied_union_recursive_seq => sourced!("union-recursive-seq", &[], &[("value", "union_value")]),
    copied_agg_count_ok => sourced!("agg-count-ok", &["filament.op.collection.count"], &[]),
    copied_coll_reduce_named => sourced!("coll-reduce-named", &["filament.op.collection.reduce"], &[]),
    copied_coll_fold_ok => sourced!("coll-fold-ok", &["filament.op.collection.fold"], &[]),
    copied_to_rational => sourced!("to_rational", &["filament.op.ieee.to_rational"], &[]),
    copied_coll_flatmap_typed => sourced!("coll-flatmap-typed", &["filament.op.collection.flat_map"], &[]),
    source_object => sourced!("source-object", &[], &[("model", "object_type"), ("composite_type", "reference")]),
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

/// Compiles and reads a source through the public front end and the strict
/// reader; every refusal is returned, never raised.
fn try_compile_and_read(name: &str, source: &str) -> Result<Value, String> {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("catalog-source", name);
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: name,
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|error| format!("{name} parses: {error:?}"))?
    .into_value();
    let inputs = PackageInput::default();
    let models = spine::select(&parsed, &inputs, limits.model, &cancel)
        .map_err(|error| format!("{name} selects its models: {error:?}"))?
        .into_value();
    let checked = spine::check(
        &parsed,
        "catalog-source".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("{name} checks through the public source path: {error:?}"))?
    .into_value();
    let read = fsl_package::read_evaluation_package(
        checked.package(),
        LibraryName::new("catalog-source").expect("package name"),
        &inputs,
        V2ReadLimits::bounded(16_777_216),
        EmitLimits::new(16_777_216),
    )
    .map_err(|error| format!("{name} emitted package is admitted: {error:?}"))?;
    if !read.emission().omitted().is_empty() {
        return Err(format!("{name} has omitted source rows"));
    }
    serde_json::from_slice(read.emission().package().bytes())
        .map_err(|error| format!("{name} admitted package JSON: {error}"))
}

fn compile_and_read(case: &Case) -> Value {
    try_compile_and_read(case.name, case.source).unwrap_or_else(|error| panic!("{error}"))
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

fn graph_nodes(package: &Value) -> &Vec<Value> {
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted graph nodes")
}

/// Every operation identity an emitted package applies, in any node body.
fn emitted_operations(package: &Value) -> BTreeSet<String> {
    let mut operations = BTreeSet::new();
    for node in graph_nodes(package) {
        operation_identities(&node["body"], &mut operations);
    }
    operations
}

/// Every `tag/form` pair an emitted package holds as a node.
fn emitted_forms(package: &Value) -> BTreeSet<String> {
    graph_nodes(package)
        .iter()
        .map(|node| {
            format!(
                "{}/{}",
                node["node_tag"].as_str().expect("node tag"),
                node["semantic_form"].as_str().expect("node form")
            )
        })
        .collect()
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
    let operations = emitted_operations(package);
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
    let forms = emitted_forms(package);
    for (tag, form) in case.forms {
        let wanted = format!("{tag}/{form}");
        if !forms.contains(&wanted) {
            return Err(ProducerMismatch::MissingForm {
                case: case.name,
                form: wanted,
            });
        }
    }
    Ok(())
}

fn assert_producers(case: &Case, catalog: &BTreeSet<String>, package: &Value) {
    check_producers(case, catalog, package)
        .unwrap_or_else(|mismatch| panic!("source producer mismatch: {mismatch:?}"));
}

/// How a listed probe source is judged producible.
enum Detect {
    /// The emitted package applies this catalog operation.
    Operation(&'static str),
    /// The emitted package carries at least one `operation_bodies` row.
    OperationBodies,
    /// The source is admitted by the strict reader at all.
    Admitted,
}

/// A checked-in source that must not yet produce what its entry names.
struct Probe {
    source: &'static str,
    detect: Detect,
}

/// One thing the language cannot yet produce, with the open tickets that
/// own it. Keys are ticket keys only; no hashes.
struct Unproduced {
    /// A catalog operation identity, or a short name of a feature.
    id: &'static str,
    keys: &'static [&'static str],
    probe: Probe,
}

macro_rules! probe_source {
    ($name:literal) => {
        include_str!(concat!("../../cases/catalog-source/", $name, ".native"))
    };
}

macro_rules! float_composite_equality {
    ($id:literal, $declarations:literal, $ty:literal) => {
        Unproduced {
            id: $id,
            keys: &["IR-1320", "STD-654"],
            probe: Probe {
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
                detect: Detect::Operation("filament.op.structural.eq"),
            },
        }
    };
}

/// The unproduced list. A catalog operation listed here needs no source
/// case; an entry whose probe becomes producible fails
/// `listed_entries_are_still_unproduced`; an entry without a key fails
/// `listed_entries_carry_ticket_keys`.
const UNPRODUCED: &[Unproduced] = &[
    Unproduced {
        id: "filament.op.collection.index",
        keys: &["IR-1491"],
        probe: Probe {
            source: include_str!("../../cases/index-accepted.native"),
            detect: Detect::Operation("filament.op.collection.index"),
        },
    },
    Unproduced {
        id: "filament.op.collection.seq",
        keys: &["IR-1492"],
        probe: Probe {
            source: probe_source!("probe-sequence-population-call"),
            detect: Detect::Operation("filament.op.collection.seq"),
        },
    },
    Unproduced {
        id: "filament.op.collection.only",
        keys: &["IR-1492"],
        probe: Probe {
            source: probe_source!("probe-sequence-population-call"),
            detect: Detect::Operation("filament.op.collection.only"),
        },
    },
    Unproduced {
        id: "filament.op.reference.eq",
        keys: &["QSL-1880"],
        probe: Probe {
            source: include_str!("../../cases/type-operators/reference-equality-accepted.native"),
            detect: Detect::Operation("filament.op.reference.eq"),
        },
    },
    Unproduced {
        id: "operation-bodies",
        keys: &["QSL-1899"],
        probe: Probe {
            source: probe_source!("probe-operation-body"),
            detect: Detect::OperationBodies,
        },
    },
    Unproduced {
        id: "filament.op.model.reaches_field",
        keys: &["QSL-1904"],
        probe: Probe {
            source: probe_source!("probe-reaches-field"),
            detect: Detect::Operation("filament.op.model.reaches_field"),
        },
    },
    Unproduced {
        id: "source-declared-object-navigation",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-object-navigation"),
            detect: Detect::Admitted,
        },
    },
    Unproduced {
        id: "filament.op.reference.ne",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-reference-inequality"),
            detect: Detect::Operation("filament.op.reference.ne"),
        },
    },
    Unproduced {
        id: "filament.op.model.dispatch_call",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-operation-call"),
            detect: Detect::Operation("filament.op.model.dispatch_call"),
        },
    },
    float_composite_equality!(
        "float-record-equality",
        "type F = Float64[nearest-even]; record R { n: F; }",
        "R"
    ),
    float_composite_equality!(
        "float-tuple-equality",
        "type F = Float64[nearest-even]; tuple T(F,F);",
        "T"
    ),
    float_composite_equality!(
        "float-union-equality",
        "type F = Float64[nearest-even]; union U { A(F),B }",
        "U"
    ),
    float_composite_equality!(
        "float-option-equality",
        "type F = Float64[nearest-even];",
        "Option<F>"
    ),
    float_composite_equality!(
        "float-sequence-equality",
        "type F = Float64[nearest-even];",
        "Sequence<F>[0,3]"
    ),
];

fn is_ticket_key(key: &str) -> bool {
    let Some((team, number)) = key.split_once('-') else {
        return false;
    };
    !team.is_empty()
        && team.bytes().all(|byte| byte.is_ascii_uppercase())
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn probe_is_producible(probe: &Probe) -> bool {
    let Ok(package) = try_compile_and_read("probe", probe.source) else {
        return false;
    };
    match probe.detect {
        Detect::Operation(identity) => emitted_operations(&package).contains(identity),
        Detect::OperationBodies => package["operation_bodies"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        Detect::Admitted => true,
    }
}

/// Every operation each checked-in case emits, found by compiling it.
fn represented_operations() -> BTreeSet<String> {
    CASES
        .iter()
        .flat_map(|case| emitted_operations(&compile_and_read(case)))
        .collect()
}

/// Every node form each checked-in case emits, found by compiling it.
fn represented_forms() -> BTreeSet<String> {
    CASES
        .iter()
        .flat_map(|case| emitted_forms(&compile_and_read(case)))
        .collect()
}

fn listed_ids() -> BTreeSet<String> {
    UNPRODUCED.iter().map(|entry| entry.id.to_owned()).collect()
}

/// The catalog operations that no case emits and no list entry owns. This is
/// the logic of `every_catalog_operation_has_a_source`; the negative test
/// calls it too.
fn operations_without_source(catalog: &BTreeSet<String>) -> BTreeSet<String> {
    let represented = represented_operations();
    let listed = listed_ids();
    catalog
        .iter()
        .filter(|identity| !represented.contains(*identity) && !listed.contains(*identity))
        .cloned()
        .collect()
}

/// The FR-322 node forms (the strict reader's closed vocabulary) that no case
/// emits and no list entry owns.
fn forms_without_source() -> BTreeSet<String> {
    let represented = represented_forms();
    let listed = listed_ids();
    filament_contract_model::CheckedNodeKind::all()
        .into_iter()
        .map(|kind| format!("{}/{}", kind.tag().as_wire(), kind.form_wire()))
        .filter(|form| !represented.contains(form) && !listed.contains(form))
        .collect()
}

/// A wrong expected operation is rejected after compiling real source.
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

/// The assertion of `every_catalog_operation_has_a_source`, over any catalog.
fn assert_catalog_has_sources(catalog: &BTreeSet<String>) {
    let gaps = operations_without_source(catalog);
    assert!(
        gaps.is_empty(),
        "catalog operations with neither a source case nor an unproduced entry: {gaps:?}"
    );
}

/// Every catalog operation is emitted by a checked-in source case or is owned
/// by an unproduced entry with a ticket key.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn every_catalog_operation_has_a_source() {
    assert_catalog_has_sources(&catalog_identities());
}

/// The same, with every case compiled, admitted by the strict reader and
/// holding exactly the operations and forms it declares.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn every_catalog_operation_has_a_source_and_is_admitted() {
    let catalog = catalog_identities();
    for case in CASES {
        let package = compile_and_read(case);
        assert_producers(case, &catalog, &package);
    }
    let gaps = operations_without_source(&catalog);
    assert!(
        gaps.is_empty(),
        "catalog operations without a source case or unproduced entry: {gaps:?}"
    );
}

#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn every_node_form_has_a_source() {
    let gaps = forms_without_source();
    assert!(
        gaps.is_empty(),
        "FR-322 node forms with neither a source case nor an unproduced entry: {gaps:?}"
    );
}

#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn emitted_packages_are_admitted() {
    for case in CASES {
        try_compile_and_read(case.name, case.source)
            .unwrap_or_else(|error| panic!("the strict reader refuses a case: {error}"));
    }
}

/// A fake catalog entry in a test copy of the catalog fails the real check,
/// and the failure names the entry.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn a_fake_catalog_entry_makes_the_catalog_check_fail_naming_it() {
    let mut catalog = catalog_identities();
    let fake = "filament.op.conformance.nonexistent".to_owned();
    assert!(catalog.insert(fake.clone()), "the fake entry is new");
    let panic = std::panic::catch_unwind(|| assert_catalog_has_sources(&catalog))
        .expect_err("the check of every_catalog_operation_has_a_source fails");
    let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.contains(&fake),
        "the failure names the entry: {message}"
    );
}

/// A listed entry whose probe source is now emitted and admitted is no longer
/// unproduced: the entry must leave the list and a case must take its place.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn listed_entries_are_still_unproduced() {
    let produced: Vec<_> = UNPRODUCED
        .iter()
        .filter(|entry| probe_is_producible(&entry.probe))
        .map(|entry| entry.id)
        .collect();
    assert!(
        produced.is_empty(),
        "listed entries are now producible; remove them and add source cases: {produced:?}"
    );
}

/// No checked-in case emits a listed operation, so a listed operation that
/// some other case now produces is reported rather than hidden.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn no_case_emits_a_listed_operation() {
    let represented = represented_operations();
    let overlap: Vec<_> = UNPRODUCED
        .iter()
        .filter(|entry| represented.contains(entry.id))
        .map(|entry| entry.id)
        .collect();
    assert!(
        overlap.is_empty(),
        "listed operations that a case emits: {overlap:?}"
    );
}

#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn listed_entries_carry_ticket_keys() {
    let keyless: Vec<_> = UNPRODUCED
        .iter()
        .filter(|entry| entry.keys.is_empty() || !entry.keys.iter().all(|key| is_ticket_key(key)))
        .map(|entry| entry.id)
        .collect();
    assert!(
        keyless.is_empty(),
        "listed entries without a well-formed ticket key: {keyless:?}"
    );
}

/// The control for the key check: an entry without a key is rejected.
#[test]
#[trace("FR-350")]
fn an_entry_without_a_key_or_with_a_hash_is_rejected() {
    assert!(is_ticket_key("IR-1491"));
    assert!(!is_ticket_key(""));
    assert!(!is_ticket_key("IR-"));
    assert!(!is_ticket_key("cd0c8ef67e5d"));
    assert!(!is_ticket_key("ir-1491"));
}

/// The five owners of the ticket are all listed with their keys.
#[test]
#[trace("FR-350")]
fn the_named_unproduced_owners_are_listed_with_their_keys() {
    let keyed = |id: &str| {
        UNPRODUCED
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("{id} is listed"))
            .keys
    };
    assert_eq!(keyed("filament.op.collection.index"), ["IR-1491"]);
    assert_eq!(keyed("filament.op.collection.seq"), ["IR-1492"]);
    assert_eq!(keyed("operation-bodies"), ["QSL-1899"]);
    assert_eq!(keyed("filament.op.model.reaches_field"), ["QSL-1904"]);
    assert_eq!(keyed("source-declared-object-navigation"), ["QSL-1880"]);
}
