// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2051: every checked-operation catalog v1 identity and every FR-322
//! node form, produced by a checked-in FSL source whose emitted package the
//! strict reader admits.
//!
//! The sources are every `*-accepted.native` and `catalog-source-*.native`
//! file under `fsl-conformance/cases/`. Each is compiled through public
//! parse, model selection and check, and its emitted package is read back
//! with `fsl_package::read_evaluation_package`. An operation or node form is
//! produced when an admitted package contains it. The node forms are the
//! `(node_tag, semantic_form)` pairs of the standard's checked-package/v2
//! schema `*Node` definitions.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use filament_exact::Cancel;
use filament_verification_contracts::operation_catalog::CHECKED_OPERATION_CATALOG_V1;
use fsl_foundation::SourceIdentity;
use fsl_package::{EmitLimits, V2ReadLimits};
use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::intake::PackageInput;
use ix_trace_rs::trace;
use serde_json::Value;

use super::{gap, Corpus, Difference, KnownGap};

const SCHEMA: &str = "proposals/checked-package-v2/schema.json";

/// Owner of a case with no authored source rendering.
const UNPRODUCED: &str = "QSL-2051: no checked-in source produces it yet";
/// Float leaves of a structural equality refuse at strict read.
const IR1320: &str = "IR-1320 (STD-654): composite IEEE leaves refuse OperatorIneligible at operation/leaves (fsl-package strict read)";
/// Composite equality refused at strict read.
const OPELIG: &str = "fsl-package strict read: OperatorIneligible for collection/composite/reference equality (same area as IR-1320)";
/// Map emission shape.
const MAPSHAPE: &str =
    "fsl-package emission: map type refused InvalidPackage/InvalidDerivedShape at strict read";
const Q2610: &str = "QSL-2610: native payload signatures leave Bytes unresolved";
const Q2371: &str = "QSL-2371: rational literal identity division is not proved in range";
const IR1491: &str = "IR-1491: emitted sequence indexing is refused OperationMemberMismatch at body/operation/member";
const Q1904: &str = "QSL-1904";

/// Cases that do not currently conform, asserted exactly.
const KNOWN_GAPS: &[KnownGap] = &[
    gap("operation/filament.op.collection.index", IR1491),
    gap("operation/filament.op.model.reaches_field", Q1904),
    gap("source/index-accepted.native", IR1491),
    gap("form/model/object_type", UNPRODUCED),
    gap("form/bounded_domain/model_population", UNPRODUCED),
    gap("form/claim/analysis_claim", UNPRODUCED),
    gap("form/claim/hyperproperty", UNPRODUCED),
    gap("form/claim/probabilistic_claim", UNPRODUCED),
    gap("form/claim/schedulability", UNPRODUCED),
    gap("form/claim/state_graph", UNPRODUCED),
    gap("form/claim/step_relation", UNPRODUCED),
    gap("form/claim/synthesis_request", UNPRODUCED),
    gap("form/claim/value_validity", UNPRODUCED),
    gap("form/claim/verification_claim", UNPRODUCED),
    gap("form/composite_type/alias", UNPRODUCED),
    gap("form/composite_type/map", UNPRODUCED),
    gap("form/composite_type/reference", UNPRODUCED),
    gap("form/correspondence/abstraction_relation", UNPRODUCED),
    gap("form/correspondence/binding_role", UNPRODUCED),
    gap("form/correspondence/component_composition", UNPRODUCED),
    gap("form/correspondence/implementation_guarantee", UNPRODUCED),
    gap("form/correspondence/model_correspondence", UNPRODUCED),
    gap("form/correspondence/profile_correspondence", UNPRODUCED),
    gap("form/correspondence/source_locus", UNPRODUCED),
    gap("form/expression/case", UNPRODUCED),
    gap("form/expression/deref", UNPRODUCED),
    gap("form/expression/place_reference", UNPRODUCED),
    gap("form/expression/pre_read", UNPRODUCED),
    gap("form/expression/reachability", UNPRODUCED),
    gap("form/expression/reference", UNPRODUCED),
    gap("form/extension/extension_form", UNPRODUCED),
    gap("form/function/predicate", UNPRODUCED),
    gap("form/function/recursive_function", UNPRODUCED),
    gap("form/hybrid/hybrid_mode", UNPRODUCED),
    gap("form/hybrid/hybrid_model", UNPRODUCED),
    gap("form/hybrid/hybrid_transition", UNPRODUCED),
    gap("form/model/event_type", UNPRODUCED),
    gap("form/model/model_import", UNPRODUCED),
    gap("form/model/model_root", UNPRODUCED),
    gap("form/model/namespace", UNPRODUCED),
    gap("form/model/persistence_interface", UNPRODUCED),
    gap("form/model/process", UNPRODUCED),
    gap("form/model/record_value_type", UNPRODUCED),
    gap("form/model/state_machine", UNPRODUCED),
    gap("form/model/systems_allocation", UNPRODUCED),
    gap("form/model/systems_connection", UNPRODUCED),
    gap("form/model/systems_interface", UNPRODUCED),
    gap("form/model/systems_part", UNPRODUCED),
    gap("form/model/systems_port", UNPRODUCED),
    gap("form/model/tuple_value_type", UNPRODUCED),
    gap("form/model/value_type", UNPRODUCED),
    gap("form/model/variant_type", UNPRODUCED),
    gap("form/protocol/channel", UNPRODUCED),
    gap("form/protocol/compensation", UNPRODUCED),
    gap("form/protocol/control", UNPRODUCED),
    gap("form/protocol/obligation", UNPRODUCED),
    gap("form/protocol/protocol_clause", UNPRODUCED),
    gap("form/protocol/protocol_reference", UNPRODUCED),
    gap("form/protocol/queue", UNPRODUCED),
    gap("form/protocol/role", UNPRODUCED),
    gap("form/relation/causal_relation", UNPRODUCED),
    gap("form/relation/membership", UNPRODUCED),
    gap("form/relation/population", UNPRODUCED),
    gap("form/relation/refinement", UNPRODUCED),
    gap("form/relation/relationship", UNPRODUCED),
    gap("form/scalar_type/bounded_quantity", UNPRODUCED),
    gap("form/scalar_type/bytes", UNPRODUCED),
    gap("form/scalar_type/carried_quantity", UNPRODUCED),
    gap("form/scalar_type/clock_reference", UNPRODUCED),
    gap("form/scalar_type/dimension_equivalence", UNPRODUCED),
    gap("form/scalar_type/json_object", UNPRODUCED),
    gap("form/state/frame", UNPRODUCED),
    gap("form/state/operation_anchor", UNPRODUCED),
    gap("form/state/snapshot", UNPRODUCED),
    gap("form/state/state_clause", UNPRODUCED),
    gap("form/state/transition", UNPRODUCED),
    gap("form/temporal/activation", UNPRODUCED),
    gap("form/temporal/clock", UNPRODUCED),
    gap("form/temporal/deadline", UNPRODUCED),
    gap("form/temporal/fairness", UNPRODUCED),
    gap("form/temporal/formula", UNPRODUCED),
    gap("form/temporal/monitor_plan", UNPRODUCED),
    gap("form/temporal/temporal_clause", UNPRODUCED),
    gap("form/temporal/terminal", UNPRODUCED),
    gap("form/temporal/window", UNPRODUCED),
    gap("form/timed_model/clock_constraint", UNPRODUCED),
    gap("form/timed_model/clock_guard", UNPRODUCED),
    gap("form/timed_model/clock_place", UNPRODUCED),
    gap("form/timed_model/clock_reset", UNPRODUCED),
    gap("form/timed_model/time_invariant", UNPRODUCED),
    gap("form/timed_model/time_source", UNPRODUCED),
    gap("form/timed_model/timed_model", UNPRODUCED),
    gap("form/timed_model/urgency", UNPRODUCED),
    gap("form/value/collection_value", UNPRODUCED),
    gap("form/value/record_value", UNPRODUCED),
    gap("form/value/tuple_value", UNPRODUCED),
    gap("form/value/union_value", UNPRODUCED),
    gap("operation/filament.op.bytes.concat", UNPRODUCED),
    gap("operation/filament.op.bytes.eq", UNPRODUCED),
    gap("operation/filament.op.bytes.length", UNPRODUCED),
    gap("operation/filament.op.bytes.ne", UNPRODUCED),
    gap("operation/filament.op.claim.clause", UNPRODUCED),
    gap("operation/filament.op.clock_reference.eq", UNPRODUCED),
    gap("operation/filament.op.clock_reference.ne", UNPRODUCED),
    gap("operation/filament.op.clock_reference.ref", UNPRODUCED),
    gap("operation/filament.op.collection.bag", UNPRODUCED),
    gap("operation/filament.op.collection.convert", UNPRODUCED),
    gap("operation/filament.op.collection.count", UNPRODUCED),
    gap("operation/filament.op.collection.filter", UNPRODUCED),
    gap("operation/filament.op.collection.flat_map", UNPRODUCED),
    gap("operation/filament.op.collection.flatten", UNPRODUCED),
    gap("operation/filament.op.collection.fold", UNPRODUCED),
    gap("operation/filament.op.collection.map", UNPRODUCED),
    gap("operation/filament.op.collection.only", UNPRODUCED),
    gap("operation/filament.op.collection.ordered_set", UNPRODUCED),
    gap(
        "operation/filament.op.collection.ordered_set.drop_first",
        UNPRODUCED,
    ),
    gap("operation/filament.op.collection.reduce", UNPRODUCED),
    gap("operation/filament.op.collection.remove_one", UNPRODUCED),
    gap("operation/filament.op.collection.seq", UNPRODUCED),
    gap("operation/filament.op.collection.sequence", UNPRODUCED),
    gap(
        "operation/filament.op.collection.sequence.drop_first",
        UNPRODUCED,
    ),
    gap("operation/filament.op.collection.set", UNPRODUCED),
    gap(
        "operation/filament.op.collection.sum.carried_quantity",
        UNPRODUCED,
    ),
    gap("operation/filament.op.collection.sum.decimal", UNPRODUCED),
    gap("operation/filament.op.collection.sum.float32", UNPRODUCED),
    gap("operation/filament.op.collection.sum.float64", UNPRODUCED),
    gap("operation/filament.op.collection.sum.integer", UNPRODUCED),
    gap("operation/filament.op.collection.sum.quantity", UNPRODUCED),
    gap("operation/filament.op.collection.sum.rational", UNPRODUCED),
    gap("operation/filament.op.control.case", UNPRODUCED),
    gap("operation/filament.op.ieee.bit_identical", UNPRODUCED),
    gap("operation/filament.op.ieee.float32.fma", UNPRODUCED),
    gap("operation/filament.op.ieee.float32.sqrt", UNPRODUCED),
    gap("operation/filament.op.ieee.float64.fma", UNPRODUCED),
    gap("operation/filament.op.ieee.float64.sqrt", UNPRODUCED),
    gap("operation/filament.op.ieee.from_exact", UNPRODUCED),
    gap("operation/filament.op.ieee.to_float32", UNPRODUCED),
    gap("operation/filament.op.ieee.to_float64", UNPRODUCED),
    gap("operation/filament.op.ieee.to_rational", UNPRODUCED),
    gap("operation/filament.op.json_object.eq", UNPRODUCED),
    gap("operation/filament.op.json_object.ne", UNPRODUCED),
    gap("operation/filament.op.model.all_instances", UNPRODUCED),
    gap("operation/filament.op.model.deref", UNPRODUCED),
    gap("operation/filament.op.model.dispatch_call", UNPRODUCED),
    gap("operation/filament.op.model.lookup", UNPRODUCED),
    gap("operation/filament.op.model.navigate", UNPRODUCED),
    gap("operation/filament.op.model.reaches", UNPRODUCED),
    gap("operation/filament.op.numeric.convert_rounding", UNPRODUCED),
    gap("operation/filament.op.option.slots", UNPRODUCED),
    gap("operation/filament.op.protocol.control", UNPRODUCED),
    gap("operation/filament.op.quantity.convert", UNPRODUCED),
    gap("operation/filament.op.quantity.scale_div", UNPRODUCED),
    gap("operation/filament.op.rational.narrow", UNPRODUCED),
    gap("operation/filament.op.reference.eq", UNPRODUCED),
    gap("operation/filament.op.reference.ne", UNPRODUCED),
    gap("operation/filament.op.reference.upcast", UNPRODUCED),
    gap("operation/filament.op.refinement.history", UNPRODUCED),
    gap("operation/filament.op.state.clause", UNPRODUCED),
    gap("operation/filament.op.state.pre", UNPRODUCED),
    gap("operation/filament.op.state.transition", UNPRODUCED),
    gap("operation/filament.op.string.to_decimal", UNPRODUCED),
    gap("operation/filament.op.string.to_float32", UNPRODUCED),
    gap("operation/filament.op.string.to_float64", UNPRODUCED),
    gap("operation/filament.op.string.to_integer", UNPRODUCED),
    gap("operation/filament.op.string.to_quantity", UNPRODUCED),
    gap("operation/filament.op.string.to_rational", UNPRODUCED),
    gap("operation/filament.op.temporal.aggregate", UNPRODUCED),
    gap("operation/filament.op.temporal.always", UNPRODUCED),
    gap("operation/filament.op.temporal.and", UNPRODUCED),
    gap("operation/filament.op.temporal.clause", UNPRODUCED),
    gap("operation/filament.op.temporal.eventually", UNPRODUCED),
    gap("operation/filament.op.temporal.fair", UNPRODUCED),
    gap("operation/filament.op.temporal.false", UNPRODUCED),
    gap("operation/filament.op.temporal.historically", UNPRODUCED),
    gap("operation/filament.op.temporal.holds", UNPRODUCED),
    gap("operation/filament.op.temporal.implies", UNPRODUCED),
    gap("operation/filament.op.temporal.not", UNPRODUCED),
    gap("operation/filament.op.temporal.once", UNPRODUCED),
    gap("operation/filament.op.temporal.or", UNPRODUCED),
    gap("operation/filament.op.temporal.release", UNPRODUCED),
    gap("operation/filament.op.temporal.since", UNPRODUCED),
    gap("operation/filament.op.temporal.triggered", UNPRODUCED),
    gap("operation/filament.op.temporal.true", UNPRODUCED),
    gap("operation/filament.op.temporal.until", UNPRODUCED),
    gap("operation/filament.op.text.to_decimal", UNPRODUCED),
    gap("operation/filament.op.text.to_float32", UNPRODUCED),
    gap("operation/filament.op.text.to_float64", UNPRODUCED),
    gap("operation/filament.op.text.to_integer", UNPRODUCED),
    gap("operation/filament.op.text.to_quantity", UNPRODUCED),
    gap("operation/filament.op.text.to_rational", UNPRODUCED),
    gap("source/catalog-source-float-option-equality.native", IR1320),
    gap("source/catalog-source-float-record-equality.native", IR1320),
    gap(
        "source/catalog-source-float-sequence-equality.native",
        IR1320,
    ),
    gap("source/catalog-source-float-tuple-equality.native", IR1320),
    gap("source/catalog-source-float-union-equality.native", IR1320),
    gap("source/collection-equality-accepted.native", OPELIG),
    gap("source/composite-equality-accepted.native", OPELIG),
    gap("source/map-accepted.native", MAPSHAPE),
    gap("source/native-payload-signatures-accepted.native", Q2610),
    gap(
        "source/rational-literal-identity-division-accepted.native",
        Q2371,
    ),
    gap("source/reference-equality-accepted.native", OPELIG),
];

/// One checked-in source.
struct Source {
    name: String,
    text: String,
}

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("cases")
}

fn is_producer(name: &str) -> bool {
    name.ends_with("-accepted.native") || name.starts_with("catalog-source-")
}

fn sources(root: &Path) -> Result<Vec<Source>, String> {
    let mut found = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if is_producer(name) {
                let text = std::fs::read_to_string(&path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
                found.push(Source {
                    name: name.to_owned(),
                    text,
                });
            }
        }
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    if found.is_empty() {
        return Err(format!("no producer sources under {}", root.display()));
    }
    Ok(found)
}

fn catalog_identities(catalog: &str) -> Result<BTreeSet<String>, String> {
    let catalog: Value = serde_json::from_str(catalog).map_err(|error| error.to_string())?;
    let rows = catalog["operations"]
        .as_array()
        .ok_or("the catalog has operation rows")?;
    let identities: BTreeSet<String> = rows
        .iter()
        .map(|row| {
            row["identity"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("operation row without identity: {row}"))
        })
        .collect::<Result<_, _>>()?;
    if identities.is_empty() {
        return Err("the published catalog is empty".to_owned());
    }
    Ok(identities)
}

/// The `(node_tag, semantic_form)` pairs the schema's `*Node` definitions
/// declare.
fn node_forms(schema: &[u8]) -> Result<BTreeSet<(String, String)>, String> {
    let schema: Value = serde_json::from_slice(schema).map_err(|error| error.to_string())?;
    let definitions = schema["$defs"].as_object().ok_or("the schema has $defs")?;
    let mut forms = BTreeSet::new();
    for (name, definition) in definitions {
        if !name.ends_with("Node") {
            continue;
        }
        for part in definition["allOf"].as_array().into_iter().flatten() {
            let properties = &part["properties"];
            let Some(tag) = properties["node_tag"]["const"].as_str() else {
                continue;
            };
            let form = &properties["semantic_form"];
            let listed: Vec<&Value> = match (form["enum"].as_array(), form.get("const")) {
                (Some(values), _) => values.iter().collect(),
                (None, Some(value)) => vec![value],
                (None, None) => return Err(format!("{name} declares no semantic_form")),
            };
            for value in listed {
                let value = value
                    .as_str()
                    .ok_or_else(|| format!("{name}: non-text semantic_form"))?;
                forms.insert((tag.to_owned(), value.to_owned()));
            }
        }
    }
    if forms.is_empty() {
        return Err("the schema declares no node forms".to_owned());
    }
    Ok(forms)
}

/// Compiles `source` and reads its emitted package back with the strict
/// reader, returning the admitted package JSON.
fn compile_and_read(source: &Source) -> Result<Value, String> {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("catalog-source", &source.name);
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: &source.name,
            bytes: source.text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|error| format!("parse: {error:?}"))?
    .into_value();
    let inputs = PackageInput::default();
    let models = spine::select(&parsed, &inputs, limits.model, &cancel)
        .map_err(|error| format!("select: {error:?}"))?
        .into_value();
    let checked = spine::check(
        &parsed,
        "catalog-source".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|error| format!("check: {error:?}"))?
    .into_value();
    let read = fsl_package::read_evaluation_package(
        checked.package(),
        LibraryName::new("catalog-source").map_err(|error| format!("{error:?}"))?,
        &inputs,
        V2ReadLimits::bounded(16_777_216),
        EmitLimits::new(16_777_216),
    )
    .map_err(|error| format!("strict read: {error:?}"))?;
    if !read.emission().omitted().is_empty() {
        return Err(format!(
            "emission omitted {} nodes",
            read.emission().omitted().len()
        ));
    }
    serde_json::from_slice(read.emission().package().bytes()).map_err(|error| error.to_string())
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

/// What an admitted package contains.
#[derive(Default)]
struct Produced {
    operations: BTreeSet<String>,
    forms: BTreeSet<(String, String)>,
}

impl Produced {
    fn add(&mut self, package: &Value) -> Result<(), String> {
        let nodes = package["semantic_graph"]["nodes"]
            .as_array()
            .ok_or("the admitted package has graph nodes")?;
        for node in nodes {
            if let (Some(tag), Some(form)) =
                (node["node_tag"].as_str(), node["semantic_form"].as_str())
            {
                self.forms.insert((tag.to_owned(), form.to_owned()));
            }
            if node["node_tag"] == "expression" && node["body"]["term"] == "application" {
                operation_identities(&node["body"], &mut self.operations);
            }
        }
        Ok(())
    }
}

fn run_corpus(catalog: &BTreeSet<String>, forms: &BTreeSet<(String, String)>) -> Corpus {
    let sources = sources(&cases_root()).unwrap_or_else(|error| panic!("{error}"));
    let mut corpus = Corpus::new("catalog", sources.len() + catalog.len() + forms.len());
    let mut produced = Produced::default();
    for source in &sources {
        let outcome = compile_and_read(source)
            .and_then(|package| produced.add(&package))
            .map_err(|observed| Difference::new("checked and strictly admitted", observed));
        corpus.record(format!("source/{}", source.name), outcome);
    }
    for identity in &produced.operations {
        if !catalog.contains(identity) {
            corpus.record(
                format!("unpublished/{identity}"),
                Err(Difference::new(
                    "a catalog v1 operation",
                    "not in the catalog",
                )),
            );
        }
    }
    for identity in catalog {
        let outcome = if produced.operations.contains(identity) {
            Ok(())
        } else {
            Err(Difference::new(
                "emitted by an admitted checked-in source",
                "no admitted source emits it",
            ))
        };
        corpus.record(format!("operation/{identity}"), outcome);
    }
    for (tag, form) in forms {
        let outcome = if produced.forms.contains(&(tag.clone(), form.clone())) {
            Ok(())
        } else {
            Err(Difference::new(
                "emitted by an admitted checked-in source",
                "no admitted source emits it",
            ))
        };
        corpus.record(format!("form/{tag}/{form}"), outcome);
    }
    corpus
}

fn standard_forms() -> BTreeSet<(String, String)> {
    node_forms(filament_specification::published_file(SCHEMA).expect("checked-package/v2 schema"))
        .unwrap_or_else(|error| panic!("{error}"))
}

/// Provenance: QSL-2051. Every catalog operation and node form is produced
/// by admitted checked-in source, or is an exactly listed known gap.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn conformance_catalog_operations_and_node_forms_from_source() {
    let catalog =
        catalog_identities(CHECKED_OPERATION_CATALOG_V1).unwrap_or_else(|error| panic!("{error}"));
    run_corpus(&catalog, &standard_forms()).assert_settled(KNOWN_GAPS);
}

/// Provenance: QSL-2051. A fake catalog entry is reported by its identity.
#[test]
#[trace("FR-350")]
fn an_added_catalog_entry_is_reported_by_its_identity() {
    let mut catalog =
        catalog_identities(CHECKED_OPERATION_CATALOG_V1).unwrap_or_else(|error| panic!("{error}"));
    assert!(catalog.insert("filament.op.conformance.nonexistent".to_owned()));
    let errors = run_corpus(&catalog, &standard_forms())
        .settle(KNOWN_GAPS)
        .expect_err("the added entry has no producer");
    assert_eq!(
        errors,
        vec![
            "catalog: operation/filament.op.conformance.nonexistent: expected emitted by an admitted checked-in source, observed no admitted source emits it (not a listed known gap)"
                .to_owned()
        ]
    );
}

/// Provenance: QSL-2051. A fake node form is reported by its tag and form.
#[test]
#[trace("FR-350", "FSpec-FR-322")]
fn an_added_node_form_is_reported_by_its_name() {
    let catalog =
        catalog_identities(CHECKED_OPERATION_CATALOG_V1).unwrap_or_else(|error| panic!("{error}"));
    let mut forms = standard_forms();
    assert!(forms.insert(("expression".to_owned(), "nonexistent".to_owned())));
    let errors = run_corpus(&catalog, &forms)
        .settle(KNOWN_GAPS)
        .expect_err("the added form has no producer");
    assert_eq!(
        errors,
        vec![
            "catalog: form/expression/nonexistent: expected emitted by an admitted checked-in source, observed no admitted source emits it (not a listed known gap)"
                .to_owned()
        ]
    );
}

/// Provenance: QSL-2051. An empty source tree or catalog cannot pass.
#[test]
#[trace("FR-350")]
fn an_empty_source_tree_or_catalog_is_an_error() {
    let empty = tempfile::tempdir().expect("temporary directory");
    assert_eq!(
        sources(empty.path()).err(),
        Some(format!(
            "no producer sources under {}",
            empty.path().display()
        ))
    );
    assert_eq!(
        catalog_identities(r#"{"operations":[]}"#).err(),
        Some("the published catalog is empty".to_owned())
    );
}
