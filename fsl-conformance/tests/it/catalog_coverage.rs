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

macro_rules! original {
    ($name:literal, $path:literal, $operations:expr, $forms:expr) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../../", $path)),
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
    copied_braking => original!("braking", "examples/hybrid-braking/braking.native", &["filament.op.rational.gt", "filament.op.rational.le", "filament.op.temporal.always", "filament.op.temporal.clause", "filament.op.temporal.holds"], &[("hybrid", "hybrid_mode"), ("hybrid", "hybrid_model"), ("hybrid", "hybrid_transition"), ("temporal", "formula"), ("temporal", "temporal_clause")]),
    copied_attempt_result => original!("attempt-result", "tests/corpus/protocol/original/attempt-result.native", &["filament.op.protocol.control"], &[("protocol", "control"), ("protocol", "obligation"), ("protocol", "protocol_clause"), ("protocol", "role"), ("state", "frame"), ("state", "operation_anchor")]),
    copied_r_exists_empty => original!("r-exists-empty", "tests/corpus/collections/r-exists-empty.native", &["filament.op.collection.exists", "filament.op.collection.sequence", "filament.op.integer.gt"], &[("expression", "quantify")]),
    copied_native_equality => original!("native-equality", "fsl-conformance/cases/type-operators/native-equality-accepted.native", &["filament.op.bytes.eq", "filament.op.json_object.eq", "filament.op.timestamp.eq", "filament.op.uuid.eq"], &[]),
    copied_source_object_workload => original!("source-object-workload", "tests/corpus/probability/staged/source-object-workload.native", &[], &[("probabilistic", "random_parameter"), ("probabilistic", "reward"), ("probabilistic", "workload")]),
    copied_operators => original!("operators", "tests/corpus/temporal/operators.native", &["filament.op.temporal.eventually", "filament.op.temporal.not", "filament.op.temporal.until"], &[]),
    copied_quant_range_fact => original!("quant-range-fact", "tests/corpus/collections/quant-range-fact.native", &["filament.op.collection.forall", "filament.op.integer.le"], &[]),
    copied_r_bag_eq => original!("r-bag-eq", "tests/corpus/collections/r-bag-eq.native", &["filament.op.collection.bag", "filament.op.structural.ne"], &[]),
    copied_float_ordering => original!("float-ordering", "fsl-conformance/cases/type-operators/float-ordering-accepted.native", &["filament.op.ieee.numeric_equal", "filament.op.ieee.total_order"], &[]),
    copied_exchange => original!("exchange", "tests/corpus/protocol/exchange.native", &[], &[("protocol", "channel"), ("protocol", "queue")]),
    copied_some => original!("some", "fsl-conformance/cases/some-accepted.native", &[], &[("value", "option_value")]),
    copied_coll_set_ok => original!("coll-set-ok", "tests/corpus/collections/coll-set-ok.native", &["filament.op.collection.set"], &[]),
    copied_coll_oset_ok => original!("coll-oset-ok", "tests/corpus/collections/coll-oset-ok.native", &["filament.op.collection.ordered_set"], &[]),
    copied_coll_removeone => original!("coll-removeone", "tests/corpus/collections/coll-removeone.native", &["filament.op.collection.remove_one"], &[]),
    copied_fn_recursion_ok => original!("fn-recursion-ok", "tests/corpus/collections/fn-recursion-ok.native", &[], &[("function", "recursive_function")]),
    copied_coll_dropfirst => original!("coll-dropfirst", "tests/corpus/collections/coll-dropfirst.native", &["filament.op.collection.sequence.drop_first"], &[]),
    copied_tup_ok => original!("tup-ok", "tests/corpus/values/tup-ok.native", &[], &[("value", "tuple_value")]),
    copied_coll_filter_ok => original!("coll-filter-ok", "tests/corpus/collections/coll-filter-ok.native", &["filament.op.collection.filter"], &[]),
    copied_rec_ok => original!("rec-ok", "tests/corpus/values/rec-ok.native", &[], &[("value", "record_value")]),
    copied_conv_coll_ok => original!("conv-coll-ok", "tests/corpus/collections/conv-coll-ok.native", &["filament.op.collection.convert"], &[]),
    copied_fn_predicate_ok => original!("fn-predicate-ok", "tests/corpus/collections/fn-predicate-ok.native", &[], &[("function", "predicate")]),
    copied_agg_sum_named => original!("agg-sum-named", "tests/corpus/collections/agg-sum-named.native", &["filament.op.collection.sum.integer"], &[]),
    copied_union_recursive_seq => original!("union-recursive-seq", "tests/corpus/collections/union-recursive-seq.native", &[], &[("value", "union_value")]),
    copied_agg_count_ok => original!("agg-count-ok", "tests/corpus/collections/agg-count-ok.native", &["filament.op.collection.count"], &[]),
    copied_coll_reduce_named => original!("coll-reduce-named", "tests/corpus/collections/coll-reduce-named.native", &["filament.op.collection.reduce"], &[]),
    copied_coll_fold_ok => original!("coll-fold-ok", "tests/corpus/collections/coll-fold-ok.native", &["filament.op.collection.fold"], &[]),
    copied_to_rational => original!("to_rational", "fsl-replay/tests/fixtures/ieee_float/to_rational.native", &["filament.op.ieee.to_rational"], &[]),
    copied_coll_flatmap_typed => original!("coll-flatmap-typed", "tests/corpus/collections/coll-flatmap-typed.native", &["filament.op.collection.flat_map"], &[]),
    object_contract => sourced!("object-contract", &["filament.op.state.clause", "filament.op.state.pre", "filament.op.model.deref"], &[("state", "state_clause"), ("expression", "pre_read"), ("expression", "deref")]),
    temporal_operators => sourced!("temporal-operators", &["filament.op.temporal.and", "filament.op.temporal.or", "filament.op.temporal.historically", "filament.op.temporal.implies", "filament.op.temporal.once", "filament.op.temporal.release", "filament.op.temporal.since", "filament.op.temporal.triggered", "filament.op.temporal.true", "filament.op.temporal.false"], &[]),
    population_only => sourced!("population-only", &["filament.op.collection.only"], &[]),
    domain_population_function => original!("population-function-explicit", "tests/corpus/collections/contexts/population-function-explicit.native", &["filament.op.model.all_instances"], &[("bounded_domain", "model_population")]),
    domain_population_relation => original!("population-count-clause-false", "tests/corpus/collections/contexts/population-count-clause-false.native", &[], &[("relation", "population")]),
    domain_hyperproperty => original!("hyper-single-impossible", "tests/corpus/refinement/hyper-single-impossible.native", &[], &[("claim", "hyperproperty")]),
    domain_probabilistic_claim => original!("link-unit", "fsl-conformance/cases/link-unit.native", &[], &[("claim", "probabilistic_claim"), ("temporal", "terminal")]),
    domain_schedulability => original!("counter-step-ctl", "fsl-conformance/cases/counter-step-ctl.native", &[], &[("claim", "schedulability")]),
    domain_step_relation => original!("relation-pre-and-post", "tests/corpus/refinement/remaining/relation-pre-and-post.native", &[], &[("claim", "step_relation")]),
    domain_reachability => original!("reaches-clause-self", "tests/corpus/collections/contexts/reaches-clause-self.native", &["filament.op.model.reaches_field"], &[("expression", "reachability")]),
    domain_reference_equality => original!("fs-distinct", "tests/corpus/operations/fs-distinct.native", &["filament.op.reference.eq"], &[]),
    domain_state_transition => original!("switch-adjust", "examples/switch-transition/adjust.native", &["filament.op.state.transition"], &[("state", "transition")]),
    domain_fairness => original!("counter-monitor", "fsl-conformance/cases/counter-monitor.native", &["filament.op.temporal.fair"], &[("temporal", "fairness")]),
    domain_model_root => original!("checked-handoff-library", "examples/checked-handoff/library.native", &[], &[("model", "model_root"), ("model", "record_value_type")]),
    domain_refinement => original!("refinement-constant-map", "tests/corpus/refinement/refinement-constant-map.native", &[], &[("relation", "refinement")]),
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
/// A refusal of a source on the way to an admitted package, with the stage
/// that refused it.
#[derive(Debug)]
struct Refusal {
    stage: &'static str,
    detail: String,
}

impl Refusal {
    fn new(stage: &'static str, detail: impl std::fmt::Debug) -> Self {
        Self {
            stage,
            detail: format!("{detail:?}"),
        }
    }
}

/// The domain packages the checked-in sources select by identity: the conformance
/// packages, the example packages and the test fixtures. A source that selects
/// none of them is unaffected.
fn model_pool() -> &'static PackageInput {
    static POOL: std::sync::OnceLock<PackageInput> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut pool = PackageInput::new();
        for dir in ["fsl-conformance/cases", "examples", "tests/fixtures"] {
            let mut stack = vec![root.join(dir)];
            while let Some(path) = stack.pop() {
                if path.is_dir() {
                    stack.extend(
                        std::fs::read_dir(&path)
                            .into_iter()
                            .flatten()
                            .flatten()
                            .map(|entry| entry.path()),
                    );
                } else if path
                    .extension()
                    .is_some_and(|extension| extension == "json")
                {
                    let Ok(bytes) = std::fs::read(&path) else {
                        continue;
                    };
                    if let Ok(one) = fsl_semantics::model::intake::package_input([bytes.as_slice()])
                    {
                        for (identity, document) in one {
                            pool.entry(identity).or_insert(document);
                        }
                    }
                }
            }
        }
        pool
    })
}

fn try_compile_and_read(name: &str, source: &str) -> Result<Value, Refusal> {
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
    .map_err(|error| Refusal::new("parse", error))?
    .into_value();
    let inputs = model_pool();
    let models = spine::select(&parsed, inputs, limits.model, &cancel)
        .map_err(|error| Refusal::new("select", error))?
        .into_value();
    let checked = spine::check(
        &parsed,
        "catalog-source".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| Refusal::new("check", error))?
    .into_value();
    let read = fsl_package::read_evaluation_package(
        checked.package(),
        LibraryName::new("catalog-source").expect("package name"),
        inputs,
        V2ReadLimits::bounded(16_777_216),
        EmitLimits::new(16_777_216),
    )
    .map_err(|error| Refusal::new("admit", error))?;
    if !read.emission().omitted().is_empty() {
        return Err(Refusal::new("admit", "omitted source rows"));
    }
    serde_json::from_slice(read.emission().package().bytes())
        .map_err(|error| Refusal::new("admit", error))
}

fn compile_and_read(case: &Case) -> Value {
    try_compile_and_read(case.name, case.source)
        .unwrap_or_else(|refusal| panic!("{} is refused: {refusal:?}", case.name))
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

/// How a listed entry's probe is judged.
enum Detect {
    /// The emitted package applies this catalog operation.
    Operation(&'static str),
    /// The emitted package holds a node of this `tag/form`.
    Form(&'static str),
    /// The source is admitted by the strict reader at all.
    Admitted,
}

/// What the probe source of an entry must do today.
enum Expect {
    /// The source is admitted and does not emit the item.
    Admitted,
    /// The source is refused at this stage with this cause. The refusal is
    /// pinned: the test fails when the stage or cause changes, and when the
    /// source is admitted and emits the item.
    Refused {
        /// `parse`, `select`, `check` or `admit`.
        stage: &'static str,
        /// A fragment of the refusal's debug text.
        cause: &'static str,
    },
    /// No probe is written. No admitted checked-in source emits the item, and
    /// this file makes no claim about whether the language has syntax for it.
    /// Only `no_source_emits_a_listed_id` guards the entry: it fails when any
    /// checked-in `.native` source emits the item.
    Unprobed,
}

/// A checked-in source that must not yet produce what its entry names.
struct Probe {
    source: &'static str,
    detect: Detect,
    expect: Expect,
}

/// One thing the language cannot yet produce, with the open tickets that
/// own it. Keys are ticket keys only; no hashes.
struct Unproduced {
    /// A catalog operation identity, a node form `tag/form`, or one of
    /// `FEATURES`.
    id: &'static str,
    keys: &'static [&'static str],
    probe: Probe,
}

/// The ids that name neither a catalog operation nor a node form.
const FEATURES: &[&str] = &[
    "source-declared-object-navigation",
    "float-record-equality",
    "float-tuple-equality",
    "float-union-equality",
    "float-option-equality",
    "float-sequence-equality",
];

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
                expect: Expect::Refused {
                    stage: "admit",
                    cause: "OperatorIneligible",
                },
            },
        }
    };
}

/// The unproduced list. A catalog operation or node form listed here needs no
/// source case. Entries with `Expect::Refused` have a probe whose refusal is
/// pinned; entries with `Expect::Unprobed` have none and are guarded
/// by the corpus sweep only.
const UNPRODUCED: &[Unproduced] = &[
    Unproduced {
        id: "filament.op.collection.index",
        keys: &["IR-1491"],
        probe: Probe {
            source: include_str!("../../cases/index-accepted.native"),
            detect: Detect::Operation("filament.op.collection.index"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "OperationMemberMismatch",
            },
        },
    },
    Unproduced {
        id: "filament.op.collection.seq",
        keys: &["IR-1492"],
        probe: Probe {
            source: probe_source!("probe-sequence-population-call"),
            detect: Detect::Operation("filament.op.collection.seq"),
            expect: Expect::Refused {
                stage: "check",
                cause: "PopulationFormOutsideRefinement",
            },
        },
    },
    Unproduced {
        id: "filament.op.reference.ne",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-reference-inequality"),
            detect: Detect::Operation("filament.op.reference.ne"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "OperatorIneligible",
            },
        },
    },
    Unproduced {
        id: "filament.op.model.dispatch_call",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-operation-call"),
            detect: Detect::Operation("filament.op.model.dispatch_call"),
            expect: Expect::Refused {
                stage: "check",
                cause: "OperatorIneligible",
            },
        },
    },
    Unproduced {
        id: "source-declared-object-navigation",
        keys: &["QSL-1880"],
        probe: Probe {
            source: probe_source!("probe-object-navigation"),
            detect: Detect::Admitted,
            expect: Expect::Refused {
                stage: "admit",
                cause: "InvalidSemanticGraph",
            },
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
    Unproduced {
        id: "filament.op.bytes.concat",
        keys: &["QSL-3043"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.bytes.concat"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.bytes.length",
        keys: &["QSL-3043"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.bytes.length"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.collection.sum.carried_quantity",
        keys: &["QSL-3044"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.collection.sum.carried_quantity"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.collection.sum.decimal",
        keys: &["QSL-3044"],
        probe: Probe {
            source: include_str!("../../cases/catalog-source/probe-sum-decimal.native"),
            detect: Detect::Operation("filament.op.collection.sum.decimal"),
            expect: Expect::Refused {
                stage: "check",
                cause: "Unproved",
            },
        },
    },
    Unproduced {
        id: "filament.op.collection.sum.rational",
        keys: &["QSL-3044"],
        probe: Probe {
            source: include_str!("../../cases/catalog-source/probe-sum-rational.native"),
            detect: Detect::Operation("filament.op.collection.sum.rational"),
            expect: Expect::Refused {
                stage: "check",
                cause: "Unproved",
            },
        },
    },
    Unproduced {
        id: "filament.op.rational.narrow",
        keys: &["QSL-3045"],
        probe: Probe {
            source: include_str!("../../cases/catalog-source/probe-rational-narrow.native"),
            detect: Detect::Operation("filament.op.rational.narrow"),
            expect: Expect::Refused {
                stage: "check",
                cause: "TypeMismatch",
            },
        },
    },
    Unproduced {
        id: "filament.op.clock_reference.eq",
        keys: &["QSL-3046"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.clock_reference.eq"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.clock_reference.ne",
        keys: &["QSL-3046"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.clock_reference.ne"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.clock_reference.ref",
        keys: &["QSL-3046"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.clock_reference.ref"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "scalar_type/clock_reference",
        keys: &["QSL-3046"],
        probe: Probe {
            source: "",
            detect: Detect::Form("scalar_type/clock_reference"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "bounded_domain/finite_trace_population",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("bounded_domain/finite_trace_population"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "expression/place_reference",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("expression/place_reference"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "expression/reference",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("expression/reference"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.model.lookup",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.model.lookup"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.model.navigate",
        keys: &["QSL-3047"],
        probe: Probe {
            source: include_str!("../../cases/catalog-source/probe-object-navigation.native"),
            detect: Detect::Operation("filament.op.model.navigate"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "InvalidSemanticGraph",
            },
        },
    },
    Unproduced {
        id: "filament.op.model.reaches",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.model.reaches"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.option.slots",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.option.slots"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.reference.upcast",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.reference.upcast"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "relation/causal_relation",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("relation/causal_relation"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "relation/membership",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("relation/membership"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "relation/relationship",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("relation/relationship"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.claim.clause",
        keys: &["QSL-3048"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.claim.clause"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.refinement.history",
        keys: &["QSL-3048"],
        probe: Probe {
            source: include_str!("../../../tests/corpus/refinement/refinement-constant-map.native"),
            detect: Detect::Operation("filament.op.refinement.history"),
            expect: Expect::Admitted,
        },
    },
    Unproduced {
        id: "state/snapshot",
        keys: &["QSL-3048"],
        probe: Probe {
            source: "",
            detect: Detect::Form("state/snapshot"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "filament.op.temporal.aggregate",
        keys: &["QSL-3049"],
        probe: Probe {
            source: "",
            detect: Detect::Operation("filament.op.temporal.aggregate"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "temporal/activation",
        keys: &["QSL-3049"],
        probe: Probe {
            source: "",
            detect: Detect::Form("temporal/activation"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "temporal/clock",
        keys: &["QSL-3049"],
        probe: Probe {
            source: include_str!("../../../tests/corpus/temporal/event-window.native"),
            detect: Detect::Form("temporal/clock"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "MissingTemporalClockSelection",
            },
        },
    },
    Unproduced {
        id: "temporal/deadline",
        keys: &["QSL-3049"],
        probe: Probe {
            source: include_str!("../../../tests/corpus/protocol/staged/await.native"),
            detect: Detect::Form("temporal/deadline"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "MissingTemporalClockSelection",
            },
        },
    },
    Unproduced {
        id: "temporal/monitor_plan",
        keys: &["QSL-3049"],
        probe: Probe {
            source: "",
            detect: Detect::Form("temporal/monitor_plan"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "temporal/window",
        keys: &["QSL-3049"],
        probe: Probe {
            source: include_str!("../../../tests/corpus/temporal/fixed-window.native"),
            detect: Detect::Form("temporal/window"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "MissingTemporalClockSelection",
            },
        },
    },
    Unproduced {
        id: "timed_model/clock_constraint",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/clock_constraint"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/clock_guard",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/clock_guard"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/clock_place",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/clock_place"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/clock_reset",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/clock_reset"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/timed_model",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/timed_model"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/time_invariant",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/time_invariant"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/time_source",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/time_source"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "timed_model/urgency",
        keys: &["QSL-3050"],
        probe: Probe {
            source: "",
            detect: Detect::Form("timed_model/urgency"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "claim/analysis_claim",
        keys: &["QSL-3051"],
        probe: Probe {
            source: "",
            detect: Detect::Form("claim/analysis_claim"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "claim/state_graph",
        keys: &["QSL-3051"],
        probe: Probe {
            source: "",
            detect: Detect::Form("claim/state_graph"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "claim/synthesis_request",
        keys: &["QSL-3051"],
        probe: Probe {
            source: "",
            detect: Detect::Form("claim/synthesis_request"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "claim/verification_claim",
        keys: &["QSL-3051"],
        probe: Probe {
            source: "",
            detect: Detect::Form("claim/verification_claim"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/abstraction_relation",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/abstraction_relation"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/binding_role",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/binding_role"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/component_composition",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/component_composition"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/implementation_guarantee",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/implementation_guarantee"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/model_correspondence",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/model_correspondence"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/profile_correspondence",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/profile_correspondence"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "correspondence/source_locus",
        keys: &["QSL-3052"],
        probe: Probe {
            source: "",
            detect: Detect::Form("correspondence/source_locus"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/event_type",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/event_type"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/model_import",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/model_import"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/namespace",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/namespace"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/persistence_interface",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/persistence_interface"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/process",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/process"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/state_machine",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/state_machine"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/systems_allocation",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/systems_allocation"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/systems_connection",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/systems_connection"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/systems_interface",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/systems_interface"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/systems_part",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/systems_part"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/systems_port",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/systems_port"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/tuple_value_type",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/tuple_value_type"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/value_type",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/value_type"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "model/variant_type",
        keys: &["QSL-3053"],
        probe: Probe {
            source: "",
            detect: Detect::Form("model/variant_type"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "composite_type/alias",
        keys: &["QSL-3054"],
        probe: Probe {
            source: "",
            detect: Detect::Form("composite_type/alias"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "composite_type/map",
        keys: &["QSL-3054"],
        probe: Probe {
            source: include_str!("../../cases/catalog-source/probe-map-contains.native"),
            detect: Detect::Form("composite_type/map"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "InvalidDerivedShape",
            },
        },
    },
    Unproduced {
        id: "scalar_type/bounded_quantity",
        keys: &["QSL-3055"],
        probe: Probe {
            source: "",
            detect: Detect::Form("scalar_type/bounded_quantity"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "scalar_type/carried_quantity",
        keys: &["QSL-3055"],
        probe: Probe {
            source: "",
            detect: Detect::Form("scalar_type/carried_quantity"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "scalar_type/dimension_equivalence",
        keys: &["QSL-3055"],
        probe: Probe {
            source: "",
            detect: Detect::Form("scalar_type/dimension_equivalence"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "value/collection_value",
        keys: &["QSL-3055"],
        probe: Probe {
            source: "",
            detect: Detect::Form("value/collection_value"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "extension/extension_form",
        keys: &["QSL-3056"],
        probe: Probe {
            source: "",
            detect: Detect::Form("extension/extension_form"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "probabilistic/delay_distribution",
        keys: &["QSL-3056"],
        probe: Probe {
            source: "",
            detect: Detect::Form("probabilistic/delay_distribution"),
            expect: Expect::Unprobed,
        },
    },
    Unproduced {
        id: "protocol/compensation",
        keys: &["QSL-3056"],
        probe: Probe {
            source: include_str!("../../../tests/corpus/protocol/original/compensation.native"),
            detect: Detect::Form("protocol/compensation"),
            expect: Expect::Refused {
                stage: "select",
                cause: "MissingImport",
            },
        },
    },
    Unproduced {
        id: "protocol/protocol_reference",
        keys: &["QSL-3056"],
        probe: Probe {
            source: "",
            detect: Detect::Form("protocol/protocol_reference"),
            expect: Expect::Unprobed,
        },
    },
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

/// Whether an emitted package holds what a probe's detector looks for.
fn detected(detect: &Detect, package: &Value) -> bool {
    match detect {
        Detect::Operation(identity) => emitted_operations(package).contains(*identity),
        Detect::Form(form) => emitted_forms(package).contains(*form),
        Detect::Admitted => true,
    }
}

/// `Ok` while the probe still behaves as its entry says; the error names how
/// it changed.
fn probe_verdict(entry: &Unproduced) -> Result<(), String> {
    let probe = &entry.probe;
    if matches!(probe.expect, Expect::Unprobed) {
        return Ok(());
    }
    let outcome = try_compile_and_read(entry.id, probe.source);
    match (&probe.expect, outcome) {
        (Expect::Unprobed, _) => Ok(()),
        (Expect::Admitted, Ok(package)) if !detected(&probe.detect, &package) => Ok(()),
        (Expect::Admitted, Ok(_)) => Err("the probe now emits the item".to_owned()),
        (Expect::Admitted, Err(refusal)) => Err(format!("the probe is now refused: {refusal:?}")),
        (Expect::Refused { .. }, Ok(package)) if detected(&probe.detect, &package) => {
            Err("the probe is now admitted and emits the item".to_owned())
        }
        (Expect::Refused { .. }, Ok(_)) => {
            Err("the pinned refusal is gone: the probe is now admitted".to_owned())
        }
        (Expect::Refused { stage, cause }, Err(refusal)) => {
            if refusal.stage == *stage && refusal.detail.contains(cause) {
                Ok(())
            } else {
                Err(format!(
                    "the refusal changed from {stage}/{cause} to {}/{}",
                    refusal.stage, refusal.detail
                ))
            }
        }
    }
}

/// Every checked-in case compiled once and shared by the coverage tests.
fn compiled_cases() -> &'static [(&'static Case, Value)] {
    static COMPILED: std::sync::OnceLock<Vec<(&'static Case, Value)>> = std::sync::OnceLock::new();
    COMPILED.get_or_init(|| {
        CASES
            .iter()
            .map(|case| (case, compile_and_read(case)))
            .collect()
    })
}

/// Every operation each checked-in case emits.
fn represented_operations() -> BTreeSet<String> {
    compiled_cases()
        .iter()
        .flat_map(|(_, package)| emitted_operations(package))
        .collect()
}

/// Every node form each checked-in case emits.
fn represented_forms() -> BTreeSet<String> {
    compiled_cases()
        .iter()
        .flat_map(|(_, package)| emitted_forms(package))
        .collect()
}

fn listed_ids() -> BTreeSet<String> {
    UNPRODUCED.iter().map(|entry| entry.id.to_owned()).collect()
}

/// Every FR-322 node form as `tag/form`: the strict reader's closed
/// vocabulary.
fn node_forms() -> BTreeSet<String> {
    filament_contract_model::CheckedNodeKind::all()
        .into_iter()
        .map(|kind| format!("{}/{}", kind.tag().as_wire(), kind.form_wire()))
        .collect()
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

/// The FR-322 node forms that no case emits and no list entry owns.
fn forms_without_source() -> BTreeSet<String> {
    let represented = represented_forms();
    let listed = listed_ids();
    node_forms()
        .into_iter()
        .filter(|form| !represented.contains(form) && !listed.contains(form))
        .collect()
}

/// Every `.native` source under the case, corpus and example directories that
/// the strict reader admits, with its emitted package.
fn corpus_packages() -> &'static [(String, Value)] {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "native")
            {
                out.push(path);
            }
        }
    }
    static CORPUS: std::sync::OnceLock<Vec<(String, Value)>> = std::sync::OnceLock::new();
    CORPUS.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut files = Vec::new();
        for dir in ["fsl-conformance/cases", "tests/corpus", "examples"] {
            walk(&root.join(dir), &mut files);
        }
        files.sort();
        files
            .into_iter()
            .filter_map(|file| {
                let source = std::fs::read_to_string(&file).ok()?;
                let name = file.strip_prefix(&root).ok()?.display().to_string();
                let package = std::panic::catch_unwind(|| try_compile_and_read(&name, &source))
                    .ok()?
                    .ok()?;
                Some((name, package))
            })
            .collect()
    })
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

/// The same, with every case admitted by the strict reader and holding
/// exactly the operations and forms it declares.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn every_catalog_operation_has_a_source_and_is_admitted() {
    let catalog = catalog_identities();
    for (case, package) in compiled_cases() {
        assert_producers(case, &catalog, package);
    }
    assert_catalog_has_sources(&catalog);
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
        try_compile_and_read(case.name, case.source).unwrap_or_else(|refusal| {
            panic!("the strict reader refuses {}: {refusal:?}", case.name)
        });
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

/// Operation bodies are emitted from source, so QSL-1899 is not an unproduced
/// entry.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn operation_bodies_are_emitted_from_source() {
    let (_, package) = compiled_cases()
        .iter()
        .find(|(case, _)| case.name == "object-contract")
        .expect("the object-contract case exists");
    assert!(
        package["operation_bodies"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "the object-contract case emits an operation body row"
    );
    assert!(
        UNPRODUCED
            .iter()
            .all(|entry| entry.id != "operation-bodies"),
        "operation bodies are produced and not listed"
    );
}

/// Each probe still behaves as its entry says: its pinned refusal is
/// unchanged, and it does not emit the item. A change fails the entry by id.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn listed_entries_are_still_unproduced() {
    let changed: Vec<_> = UNPRODUCED
        .iter()
        .filter_map(|entry| probe_verdict(entry).err().map(|why| (entry.id, why)))
        .collect();
    assert!(
        changed.is_empty(),
        "listed entries whose probe changed; re-key, remove or add a case: {changed:#?}"
    );
}

/// Pinned probes exist where the language has syntax; the rest are
/// corpus-guard-only. The count is stated so that dropping a probe is a
/// visible change.
#[test]
#[trace("FR-350")]
fn entries_without_a_probe_say_why() {
    for entry in UNPRODUCED {
        match &entry.probe.expect {
            Expect::Unprobed => assert!(
                entry.probe.source.is_empty(),
                "{} is unprobed and carries no source",
                entry.id
            ),
            _ => assert!(
                !entry.probe.source.is_empty(),
                "{} has a pinned probe without a source",
                entry.id
            ),
        }
    }
}

/// No checked-in case emits a listed operation or form.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn no_case_emits_a_listed_id() {
    let operations = represented_operations();
    let forms = represented_forms();
    let overlap: Vec<_> = UNPRODUCED
        .iter()
        .filter(|entry| operations.contains(entry.id) || forms.contains(entry.id))
        .map(|entry| entry.id)
        .collect();
    assert!(
        overlap.is_empty(),
        "listed ids that a case emits: {overlap:?}"
    );
}

/// The corpus-wide guard: no `.native` source under the case, corpus and
/// example directories emits a listed operation or form, whether or not it is
/// one of the cases.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn no_source_emits_a_listed_id() {
    let hits: Vec<_> = corpus_packages()
        .iter()
        .flat_map(|(name, package)| {
            let operations = emitted_operations(package);
            let forms = emitted_forms(package);
            UNPRODUCED
                .iter()
                .filter(move |entry| operations.contains(entry.id) || forms.contains(entry.id))
                .map(move |entry| (entry.id, name.clone()))
        })
        .collect();
    assert!(
        hits.is_empty(),
        "listed ids that checked-in sources now emit; remove them and add cases: {hits:#?}"
    );
    assert!(
        corpus_packages().len() > 100,
        "the corpus sweep admitted too few sources to guard anything"
    );
}

/// A listed id names a catalog operation, a node form or a known feature.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn listed_ids_name_something_real() {
    let catalog = catalog_identities();
    let forms = node_forms();
    let stale: Vec<_> = UNPRODUCED
        .iter()
        .map(|entry| entry.id)
        .filter(|id| !catalog.contains(*id) && !forms.contains(*id) && !FEATURES.contains(id))
        .collect();
    assert!(stale.is_empty(), "listed ids that name nothing: {stale:?}");
    let duplicated = UNPRODUCED.len() - listed_ids().len();
    assert_eq!(duplicated, 0, "an id is listed twice");
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

/// The named unproduced owners are all listed with their keys.
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
    assert_eq!(keyed("source-declared-object-navigation"), ["QSL-1880"]);
}

/// Only the filed tickets own an unproduced entry: QSL-3043..QSL-3056 (the
/// QSL-2051 groups) and the long-standing owners of the named non-producers.
#[test]
#[trace("FR-350")]
fn listed_keys_are_among_the_filed_set() {
    const FILED: &[&str] = &[
        "IR-1491", "IR-1492", "QSL-1880", "QSL-1904", "IR-1320", "STD-654",
    ];
    let outside: Vec<_> = UNPRODUCED
        .iter()
        .flat_map(|entry| entry.keys.iter().map(move |key| (entry.id, *key)))
        .filter(|(_, key)| {
            let group = key
                .strip_prefix("QSL-")
                .and_then(|number| number.parse::<u32>().ok())
                .is_some_and(|number| (3043..=3056).contains(&number));
            !group && !FILED.contains(key)
        })
        .collect();
    assert!(
        outside.is_empty(),
        "keys outside the filed set: {outside:?}"
    );
}
