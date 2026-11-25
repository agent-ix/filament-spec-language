// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source producers of published checked-operation identities and node forms.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use filament_exact::Cancel;
use filament_verification_contracts::operation_catalog::CHECKED_OPERATION_CATALOG_V1;
use fsl_foundation::SourceIdentity;
use fsl_package::{EmitLimits, V2ReadLimits};
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::{package_input, PackageInput};
use ix_trace_rs::trace;
use serde_json::Value;

use crate::fspec_standard::is_ticket_key;

struct Case {
    name: &'static str,
    source: &'static str,
    /// The domain packages the source selects, each supplied under the identity
    /// it declares in place of any pooled document.
    documents: &'static [&'static [u8]],
    operations: &'static [&'static str],
    forms: &'static [(&'static str, &'static str)],
}

macro_rules! operators {
    ($name:literal, $family:literal, [$($operation:literal),+ $(,)?]) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../cases/type-operators/", $name, "-accepted.native")),
            documents: &[],
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
            documents: &[],
            operations: $operations,
            forms: $forms,
        }
    };
}

/// A catalog source that names the domain package it selects, one the pool does
/// not supply: it sits outside the pooled directories, or another checked-in
/// document declares its identity too.
macro_rules! selecting {
    ($name:literal, $document:literal, $operations:expr, $forms:expr) => {
        Case {
            name: $name,
            source: include_str!(concat!("../../cases/catalog-source/", $name, ".native")),
            documents: &[include_bytes!(concat!("../../../", $document))],
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
            documents: &[],
            operations: $operations,
            forms: $forms,
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
    integer_record_equality => sourced!("integer-record-equality", &["filament.op.structural.eq"], &[]),
    integer_tuple_equality => sourced!("integer-tuple-equality", &["filament.op.structural.eq"], &[]),
    integer_union_equality => sourced!("integer-union-equality", &["filament.op.structural.eq"], &[]),
    option_presence => Case {
        name: "option-presence",
        source: include_str!("../../cases/type-operators/option-presence-accepted.native"),
        documents: &[],
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
        documents: &[],
        operations: &["filament.op.tuple.project"],
        forms: &[("composite_type", "tuple"), ("expression", "query")],
    },
    float_arithmetic => Case {
        name: "float-arithmetic",
        source: include_str!("../../cases/type-operators/float-arithmetic-accepted.native"),
        documents: &[],
        operations: &["filament.op.ieee.float32.add", "filament.op.ieee.float32.sub", "filament.op.ieee.float32.mul", "filament.op.ieee.float64.add", "filament.op.ieee.float64.sub", "filament.op.ieee.float64.mul"],
        forms: &[],
    },
    float_negation => Case {
        name: "float-negation",
        source: include_str!("../../cases/type-operators/float-negation-accepted.native"),
        documents: &[],
        operations: &["filament.op.ieee.float32.negate", "filament.op.ieee.float64.negate"],
        forms: &[],
    },
    float_division => Case {
        name: "float-division",
        source: include_str!("../../cases/type-operators/float-division-accepted.native"),
        documents: &[],
        operations: &["filament.op.ieee.float32.div", "filament.op.ieee.float64.div"],
        forms: &[],
    },
    structural_calls => Case {
        name: "structural-calls",
        source: include_str!("../../cases/catalog-source-structural.native"),
        documents: &[],
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
    bytes_length_concat => sourced!("bytes-length-concat", &["filament.op.bytes.length", "filament.op.bytes.concat"], &[]),
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
    domain_population_function => original!("population-function-explicit", "tests/corpus/collections/contexts/population-function-explicit.native", &["filament.op.model.all_instances"], &[("bounded_domain", "model_population"), ("correspondence", "model_correspondence")]),
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
    state_graph_claims => sourced!("state-graph-claims", &[], &[("claim", "state_graph")]),
    domain_relationship => selecting!("domain-relationship", "tests/fixtures/spine-model-relationship.semantic-ir.json", &[], &[("relation", "relationship")]),
    domain_protocol_reference => selecting!("protocol-reference", "tests/fixtures/spine-model.semantic-ir.json", &[], &[("protocol", "protocol_reference")]),
    domain_systems_interface => sourced!("domain-systems-interface", &[], &[("model", "systems_interface")]),
    domain_lifecycle => sourced!("domain-lifecycle", &[], &[("model", "process"), ("model", "state_machine"), ("model", "variant_type")]),
    timed_urgency => sourced!("timed-urgency", &["filament.op.model.lookup"], &[("timed_model", "timed_model"), ("timed_model", "time_source"), ("timed_model", "clock_place"), ("timed_model", "urgency")]),
    timed_clauses => sourced!("timed-clauses", &[], &[("timed_model", "clock_constraint"), ("timed_model", "clock_guard"), ("timed_model", "clock_reset"), ("timed_model", "time_invariant"), ("expression", "reference")]),
    clock_reference => sourced!("clock-reference", &["filament.op.clock_reference.ref", "filament.op.clock_reference.eq", "filament.op.clock_reference.ne"], &[("scalar_type", "clock_reference"), ("expression", "place_reference")]),
    reference_literal_inequality => selecting!("reference-literal-inequality", "tests/corpus/models/shop.semantic-ir.json", &["filament.op.reference.ne"], &[]),
    delay_distribution => sourced!("delay-distribution", &[], &[("probabilistic", "delay_distribution")]),
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

/// Every distinct domain package among `documents`, by the identity it
/// declares. A document that is no domain package is skipped.
fn declared_packages<'a>(
    documents: impl IntoIterator<Item = &'a [u8]>,
) -> BTreeMap<String, BTreeSet<Vec<u8>>> {
    let mut declared = BTreeMap::<String, BTreeSet<Vec<u8>>>::new();
    for bytes in documents {
        let Ok(one) = package_input([bytes]) else {
            continue;
        };
        for (identity, document) in one {
            declared.entry(identity).or_default().insert(document);
        }
    }
    declared
}

/// Keys each domain package among `documents` by the identity it declares. An
/// identity that two different documents declare is supplied by neither, so a
/// source selecting it is refused whatever order the documents are read in.
fn pool_of<'a>(documents: impl IntoIterator<Item = &'a [u8]>) -> PackageInput {
    declared_packages(documents)
        .into_iter()
        .filter_map(|(identity, mut documents)| {
            let document = documents.pop_first()?;
            documents.is_empty().then_some((identity, document))
        })
        .collect()
}

/// The pooled packages, with each of `documents` supplied under the identity
/// it declares.
fn pool_with(documents: &[&[u8]]) -> PackageInput {
    let mut pool = model_pool().clone();
    for bytes in documents {
        pool.extend(package_input([*bytes]).expect("a case's domain package reads"));
    }
    pool
}

/// The inputs a swept `source` is compiled against: the pooled packages with
/// each identity the source names supplied by one of its `declared` documents,
/// one input for every such combination.
fn inputs_for(source: &str, declared: &BTreeMap<String, BTreeSet<Vec<u8>>>) -> Vec<PackageInput> {
    declared
        .iter()
        .filter(|(identity, _)| source.contains(&format!("\"{identity}\"")))
        .fold(
            vec![model_pool().clone()],
            |inputs, (identity, documents)| {
                inputs
                    .iter()
                    .flat_map(|input| {
                        documents.iter().map(|document| {
                            let mut input = input.clone();
                            input.insert(identity.clone(), document.clone());
                            input
                        })
                    })
                    .collect()
            },
        )
}

/// The domain packages a case or probe may select by identity alone: the
/// conformance packages, the example packages and the test fixtures. A source
/// that selects none of them is unaffected.
fn model_pool() -> &'static PackageInput {
    static POOL: std::sync::OnceLock<PackageInput> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        let documents: Vec<Vec<u8>> = checked_in_files(
            &["fsl-conformance/cases", "examples", "tests/fixtures"],
            "json",
        )
        .iter()
        .map(|path| std::fs::read(path).expect("a checked-in document reads"))
        .collect();
        pool_of(documents.iter().map(Vec::as_slice))
    })
}

/// The repository root.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every file with `extension` under `directories` of the repository, sorted.
/// Build output (`target`) and hidden directories are skipped.
fn checked_in_files(directories: &[&str], extension: &str) -> Vec<PathBuf> {
    let mut stack: Vec<PathBuf> = directories
        .iter()
        .map(|directory| repository_root().join(directory))
        .collect();
    let mut files = Vec::new();
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("a checked-in directory reads") {
            let entry = entry.expect("a checked-in directory entry reads");
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                if name != "target" && !name.to_string_lossy().starts_with('.') {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|found| found == extension) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Compiles and reads a source over the domain packages `inputs` through the
/// public front end and the strict reader; every refusal is returned, never
/// raised.
fn try_compile_and_read(name: &str, source: &str, inputs: &PackageInput) -> Result<Value, Refusal> {
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

/// Compiles and reads a case's source over the domain packages it selects.
fn try_compile_case(case: &Case) -> Result<Value, Refusal> {
    try_compile_and_read(case.name, case.source, &pool_with(case.documents))
}

fn compile_and_read(case: &Case) -> Value {
    try_compile_case(case).unwrap_or_else(|refusal| panic!("{} is refused: {refusal:?}", case.name))
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
#[derive(Clone, Copy)]
enum Detect {
    /// The emitted package applies this catalog operation.
    Operation(&'static str),
    /// The emitted package holds a node of this `tag/form`.
    Form(&'static str),
    /// The source is admitted by the strict reader at all.
    Admitted,
}

/// What the probe source of an entry must do today.
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
struct Probe {
    source: &'static str,
    detect: Detect,
    expect: Expect,
}

/// One item no checked-in case covers yet, with the tickets that own it. Keys
/// are ticket keys only; no hashes.
#[derive(Clone, Copy)]
struct Unproduced {
    /// A catalog operation identity, a node form `tag/form`, or one of
    /// `FEATURES`.
    id: &'static str,
    keys: &'static [&'static str],
    probe: Probe,
}

/// The ids that name neither a catalog operation nor a node form. The
/// unproduced list must account for each.
const FEATURES: &[&str] = &[
    "source-declared-object-navigation",
    "source-declared-object-reference-equality",
    "source-declared-object-reaches-field",
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
        id: "filament.op.model.dispatch_call",
        keys: &["QSL-3047"],
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
        keys: &["QSL-3047"],
        probe: Probe {
            source: probe_source!("probe-object-navigation"),
            detect: Detect::Admitted,
            expect: Expect::Refused {
                stage: "admit",
                cause: "InvalidSemanticGraph",
            },
        },
    },
    // `reference.eq` and `model.reaches_field` have cases over domain package
    // declarations; the same operations over a source-declared object do not
    // reach an admitted package.
    Unproduced {
        id: "source-declared-object-reference-equality",
        keys: &["QSL-3047"],
        probe: Probe {
            source: include_str!("../../cases/type-operators/reference-equality-accepted.native"),
            detect: Detect::Operation("filament.op.reference.eq"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "OperatorIneligible",
            },
        },
    },
    Unproduced {
        id: "source-declared-object-reaches-field",
        keys: &["QSL-3047"],
        probe: Probe {
            source: probe_source!("probe-reaches-field"),
            detect: Detect::Operation("filament.op.model.reaches_field"),
            expect: Expect::Refused {
                stage: "admit",
                cause: "OperatorIneligible",
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
        id: "bounded_domain/finite_trace_population",
        keys: &["QSL-3047"],
        probe: Probe {
            source: "",
            detect: Detect::Form("bounded_domain/finite_trace_population"),
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
            source: include_str!("../../../tests/corpus/protocol/await.native"),
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
        id: "claim/analysis_claim",
        keys: &["QSL-3051"],
        probe: Probe {
            source: "",
            detect: Detect::Form("claim/analysis_claim"),
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
];

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
    let outcome = try_compile_and_read(entry.id, probe.source, model_pool());
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

/// The operation identities and node forms an emitted package holds.
fn emitted_items(package: &Value) -> BTreeSet<String> {
    let mut items = emitted_operations(package);
    items.extend(emitted_forms(package));
    items
}

/// Every operation identity and node form the checked-in cases emit.
fn produced() -> BTreeSet<String> {
    compiled_cases()
        .iter()
        .flat_map(|(_, package)| emitted_items(package))
        .collect()
}

/// Every item the unproduced list accounts for: each catalog operation
/// identity, each node form and each of `FEATURES`.
fn published() -> BTreeSet<String> {
    let mut published = catalog_identities();
    published.extend(node_forms());
    published.extend(FEATURES.iter().map(|feature| (*feature).to_owned()));
    published
}

/// Every FR-322 node form as `tag/form`: the strict reader's closed
/// vocabulary.
fn node_forms() -> BTreeSet<String> {
    filament_contract_model::CheckedNodeKind::all()
        .into_iter()
        .map(|kind| format!("{}/{}", kind.tag().as_wire(), kind.form_wire()))
        .collect()
}

/// Why a run over the unproduced list fails.
#[derive(Debug, PartialEq, Eq)]
enum ListFailure {
    /// A published item that no case produces and no entry lists.
    Unlisted(String),
    /// A listed operation or node form that a case produces.
    Stale(&'static str),
    /// A listed entry with no key (`None`), or with a key that is empty or no
    /// ticket key.
    BadKey {
        id: &'static str,
        key: Option<&'static str>,
    },
    /// A listed id that names no published item.
    Unknown(&'static str),
    /// An id listed more than once.
    Duplicate(&'static str),
    /// Full qualification was requested while these ids remain listed.
    Unqualified(Vec<&'static str>),
}

/// The decision over one run of the unproduced list.
struct ListRun {
    /// One line for each listed entry, printed by every run.
    summary: Vec<String>,
    /// Everything that fails the run.
    failures: Vec<ListFailure>,
}

/// Checks `list` as a whole against what is `published` and what the cases
/// `produced`. A listed entry is summarised and passes; under
/// `full_qualification` any remaining entry fails the run.
fn run_list(
    list: &[Unproduced],
    published: &BTreeSet<String>,
    produced: &BTreeSet<String>,
    full_qualification: bool,
) -> ListRun {
    let mut listed = BTreeSet::new();
    let mut summary = Vec::new();
    let mut failures = Vec::new();
    for entry in list {
        let id = entry.id;
        summary.push(format!(
            "pending ({}): catalog-source: {id}",
            entry.keys.join(", ")
        ));
        if !listed.insert(id) {
            failures.push(ListFailure::Duplicate(id));
        }
        if !published.contains(id) {
            failures.push(ListFailure::Unknown(id));
        }
        if produced.contains(id) {
            failures.push(ListFailure::Stale(id));
        }
        if entry.keys.is_empty() {
            failures.push(ListFailure::BadKey { id, key: None });
        }
        failures.extend(
            entry
                .keys
                .iter()
                .filter(|key| !is_ticket_key(key))
                .map(|key| ListFailure::BadKey { id, key: Some(key) }),
        );
    }
    failures.extend(
        published
            .iter()
            .filter(|item| !produced.contains(*item) && !listed.contains(item.as_str()))
            .map(|item| ListFailure::Unlisted(item.clone())),
    );
    if full_qualification && !list.is_empty() {
        failures.push(ListFailure::Unqualified(
            list.iter().map(|entry| entry.id).collect(),
        ));
    }
    ListRun { summary, failures }
}

/// The members of `universe` that no case emits and no list entry owns.
fn without_source(universe: &BTreeSet<String>) -> BTreeSet<String> {
    run_list(UNPRODUCED, universe, &produced(), false)
        .failures
        .into_iter()
        .filter_map(|failure| match failure {
            ListFailure::Unlisted(item) => Some(item),
            ListFailure::Stale(_)
            | ListFailure::BadKey { .. }
            | ListFailure::Unknown(_)
            | ListFailure::Duplicate(_)
            | ListFailure::Unqualified(_) => None,
        })
        .collect()
}

/// Every checked-in `.native` source, wherever it sits in the repository, that
/// the strict reader admits, with its emitted package: one for each
/// combination of checked-in documents of the identities it names.
///
/// Not seen: source text written inline in Rust, a source whose domain package
/// or library dependency exists only as Rust values, and a source the front
/// end or the reader refuses.
fn corpus_packages() -> &'static [(String, Value)] {
    static CORPUS: std::sync::OnceLock<Vec<(String, Value)>> = std::sync::OnceLock::new();
    CORPUS.get_or_init(|| {
        let root = repository_root();
        let documents: Vec<Vec<u8>> = checked_in_files(&[""], "json")
            .iter()
            .map(|path| std::fs::read(path).expect("a checked-in document reads"))
            .collect();
        let declared = declared_packages(documents.iter().map(Vec::as_slice));
        let mut admitted = Vec::new();
        for file in checked_in_files(&[""], "native") {
            let source = std::fs::read_to_string(&file).expect("a checked-in source reads");
            let name = file
                .strip_prefix(&root)
                .expect("a swept source is under the root")
                .display()
                .to_string();
            for inputs in inputs_for(&source, &declared) {
                let outcome =
                    std::panic::catch_unwind(|| try_compile_and_read(&name, &source, &inputs));
                if let Ok(Ok(package)) = outcome {
                    admitted.push((name.clone(), package));
                }
            }
        }
        admitted
    })
}

/// A wrong expected operation is rejected after compiling real source.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn a_wrong_expected_operation_names_the_source_case_and_identity() {
    let case = Case {
        name: "altered-boolean-expectation",
        source: CASES[0].source,
        documents: &[],
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
    let gaps = without_source(catalog);
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
    let gaps = without_source(&node_forms());
    assert!(
        gaps.is_empty(),
        "FR-322 node forms with neither a source case nor an unproduced entry: {gaps:?}"
    );
}

#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn emitted_packages_are_admitted() {
    for case in CASES {
        try_compile_case(case).unwrap_or_else(|refusal| {
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

/// An unprobed entry carries no source, and every other entry has the source
/// its expectation is judged on.
#[test]
#[trace("FR-350")]
fn a_probe_has_a_source_exactly_when_it_is_judged() {
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

/// The repository-wide guard: no checked-in `.native` source that
/// `corpus_packages` admits emits a listed operation or form, whether or not it
/// is one of the cases.
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
    assert!(
        corpus_packages()
            .iter()
            .any(|(name, _)| name == "fsl-replay/tests/fixtures/ieee_float/to_rational.native"),
        "the corpus sweep reaches sources outside the case, corpus and example directories"
    );
}

/// FR-350-AC-24, FR-350-AC-25: every published catalog operation and node form
/// is emitted by a checked-in native case or is listed with a ticket key. The
/// listed entries are printed; `FSPEC_FULL_QUALIFICATION` fails the run while
/// any of them remains.
#[test]
#[trace("TC-3218", "FR-350-AC-24", "FR-350-AC-25")]
fn conformance_catalog_source_coverage() {
    let catalog = catalog_identities();
    for (case, package) in compiled_cases() {
        assert_producers(case, &catalog, package);
    }
    let run = run_list(
        UNPRODUCED,
        &published(),
        &produced(),
        std::env::var_os("FSPEC_FULL_QUALIFICATION").is_some(),
    );
    for line in &run.summary {
        println!("{line}");
    }
    assert!(
        run.failures.is_empty(),
        "the unproduced list fails the run: {:#?}",
        run.failures
    );
}

/// The entry of the checked-in list for `id`.
fn listed(id: &str) -> Unproduced {
    *UNPRODUCED
        .iter()
        .find(|entry| entry.id == id)
        .unwrap_or_else(|| panic!("{id} is listed"))
}

/// An entry with no probe, for altering a copy of the list.
fn unprobed(id: &'static str) -> Unproduced {
    Unproduced {
        id,
        keys: &["QSL-2051"],
        probe: Probe {
            source: "",
            detect: Detect::Admitted,
            expect: Expect::Unprobed,
        },
    }
}

/// A copy of the checked-in list with `entry` added.
fn list_with(entry: Unproduced) -> Vec<Unproduced> {
    let mut list = UNPRODUCED.to_vec();
    list.push(entry);
    list
}

/// A copy of the checked-in list without `id`.
fn list_without(id: &str) -> Vec<Unproduced> {
    assert_eq!(listed(id).id, id);
    UNPRODUCED
        .iter()
        .filter(|entry| entry.id != id)
        .copied()
        .collect()
}

/// A copy of the checked-in list with the keys of `id` replaced.
fn list_rekeyed(id: &str, keys: &'static [&'static str]) -> Vec<Unproduced> {
    assert_eq!(listed(id).id, id);
    UNPRODUCED
        .iter()
        .map(|entry| {
            if entry.id == id {
                Unproduced { keys, ..*entry }
            } else {
                *entry
            }
        })
        .collect()
}

/// FR-350-AC-24: an item leaves the list only when a case covers it. The
/// `ordered-set-drop-first` case is taken away and its operation listed in its
/// place; dropping the entry without the case then fails, and so does keeping
/// the entry once the case is back.
#[test]
#[trace("TC-3218", "FR-350-AC-24")]
fn the_unproduced_list_shrinks_only_by_a_covering_case() {
    const ITEM: &str = "filament.op.collection.ordered_set.drop_first";
    let without_case: BTreeSet<String> = compiled_cases()
        .iter()
        .filter(|(case, _)| case.name != "ordered-set-drop-first")
        .flat_map(|(_, package)| emitted_items(package))
        .collect();
    let with_case = produced();
    let listing_it = list_with(unprobed(ITEM));
    let failures = |list: &[Unproduced], produced: &BTreeSet<String>| {
        run_list(list, &published(), produced, false).failures
    };

    assert_eq!(failures(&listing_it, &without_case), []);
    assert_eq!(
        failures(UNPRODUCED, &without_case),
        [ListFailure::Unlisted(ITEM.to_owned())]
    );
    assert_eq!(
        failures(&listing_it, &with_case),
        [ListFailure::Stale(ITEM)]
    );
    assert_eq!(failures(UNPRODUCED, &with_case), []);
}

/// FR-350-AC-25: removing the entry of an unproduced item or feature, listing a
/// produced one, and blanking or garbling a key each fail the run, naming it.
#[test]
#[trace("TC-3218", "FR-350-AC-25")]
fn each_alteration_of_the_unproduced_list_fails_naming_the_entry() {
    const OPERATION: &str = "filament.op.collection.index";
    const FORM: &str = "extension/extension_form";
    let published = published();
    let produced = produced();
    let failures = |list: &[Unproduced]| run_list(list, &published, &produced, false).failures;

    assert_eq!(failures(UNPRODUCED), []);
    for removed in [OPERATION, FORM].iter().chain(FEATURES) {
        assert_eq!(
            failures(&list_without(removed)),
            [ListFailure::Unlisted((*removed).to_owned())]
        );
    }
    for stale in ["filament.op.boolean.not", "composite_type/option"] {
        assert_eq!(
            failures(&list_with(unprobed(stale))),
            [ListFailure::Stale(stale)]
        );
    }
    assert_eq!(
        failures(&list_rekeyed(OPERATION, &[])),
        [ListFailure::BadKey {
            id: OPERATION,
            key: None
        }]
    );
    let malformed: [&'static [&'static str]; 4] =
        [&[""], &["IR-"], &["ir-1491"], &["cd0c8ef67e5d"]];
    for keys in malformed {
        assert_eq!(
            failures(&list_rekeyed(FORM, keys)),
            [ListFailure::BadKey {
                id: FORM,
                key: Some(keys[0])
            }]
        );
    }
    assert_eq!(
        failures(&list_with(unprobed("filament.op.conformance.nonexistent"))),
        [ListFailure::Unknown("filament.op.conformance.nonexistent")]
    );
    assert_eq!(
        failures(&list_with(listed(OPERATION))),
        [ListFailure::Duplicate(OPERATION)]
    );
}

/// FR-350-AC-25: a listed entry passes the default run and is in its summary;
/// full qualification fails while any entry remains, and passes an empty list.
#[test]
#[trace("TC-3218", "FR-350-AC-25")]
fn a_listed_entry_is_summarised_and_fails_only_full_qualification() {
    let list = [
        listed("filament.op.collection.index"),
        listed("float-record-equality"),
    ];
    let published = BTreeSet::from([
        "filament.op.collection.index".to_owned(),
        "float-record-equality".to_owned(),
    ]);
    let produced = BTreeSet::new();
    let summary = [
        "pending (IR-1491): catalog-source: filament.op.collection.index",
        "pending (IR-1320, STD-654): catalog-source: float-record-equality",
    ];

    let default_run = run_list(&list, &published, &produced, false);
    assert_eq!(default_run.summary, summary);
    assert_eq!(default_run.failures, []);

    let full_run = run_list(&list, &published, &produced, true);
    assert_eq!(full_run.summary, summary);
    assert_eq!(
        full_run.failures,
        [ListFailure::Unqualified(vec![
            "filament.op.collection.index",
            "float-record-equality",
        ])]
    );

    let empty = run_list(&[], &BTreeSet::new(), &BTreeSet::new(), true);
    assert_eq!(empty.failures, []);
}

/// Two different documents declaring one identity leave it unsupplied, in
/// either order; a repeated document supplies it once.
#[test]
#[trace("FR-350")]
fn an_identity_two_documents_declare_is_not_pooled() {
    let plain: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");
    let related: &[u8] =
        include_bytes!("../../../tests/fixtures/spine-model-relationship.semantic-ir.json");
    assert_ne!(plain, related);

    for alone in [plain, related] {
        let pool = pool_of([alone, alone]);
        assert_eq!(pool.keys().collect::<Vec<_>>(), ["acme/orders"]);
        assert_eq!(pool["acme/orders"], alone);
    }
    assert!(pool_of([plain, related]).is_empty());
    assert!(pool_of([related, plain, plain]).is_empty());
    assert!(!model_pool().contains_key("acme/orders"));
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
    assert_eq!(keyed("source-declared-object-navigation"), ["QSL-3047"]);
}

/// Only the filed tickets own an unproduced entry: QSL-3044..QSL-3056 (the
/// QSL-2051 groups) and the long-standing owners of the named non-producers.
#[test]
#[trace("FR-350")]
fn listed_keys_are_among_the_filed_set() {
    const FILED: &[&str] = &["IR-1491", "IR-1492", "IR-1320", "STD-654"];
    let outside: Vec<_> = UNPRODUCED
        .iter()
        .flat_map(|entry| entry.keys.iter().map(move |key| (entry.id, *key)))
        .filter(|(_, key)| {
            let group = key
                .strip_prefix("QSL-")
                .and_then(|number| number.parse::<u32>().ok())
                .is_some_and(|number| (3044..=3056).contains(&number));
            !group && !FILED.contains(key)
        })
        .collect();
    assert!(
        outside.is_empty(),
        "keys outside the filed set: {outside:?}"
    );
}
